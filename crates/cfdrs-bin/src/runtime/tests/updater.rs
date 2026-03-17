use std::collections::VecDeque;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use cfdrs_his::updater::AutoUpdateSettings;
use sha2::{Digest, Sha256};

use crate::runtime::{HarnessBuilder, RuntimeAutoUpdate, RuntimeExit, run_with_source};

use super::fixtures::runtime_config;
use super::harness::{TestBehavior, test_source};

struct MockUpdateServer {
    address: String,
    responses: Arc<Mutex<VecDeque<(String, String, String)>>>,
    join: Option<thread::JoinHandle<()>>,
}

impl MockUpdateServer {
    fn start(responses: Vec<(String, String, String)>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
        let address = listener.local_addr().expect("addr").to_string();
        let responses = Arc::new(Mutex::new(VecDeque::from(responses)));
        let server_responses = Arc::clone(&responses);

        let join = thread::spawn(move || {
            while let Ok((mut stream, _)) = listener.accept() {
                let mut buffer = [0_u8; 8192];
                let _ = stream.read(&mut buffer).expect("read request");
                let Some((status_line, content_type, body)) =
                    server_responses.lock().expect("responses").pop_front()
                else {
                    break;
                };
                let response = format!(
                    "HTTP/1.1 {status_line}\r\nContent-Length: {}\r\nContent-Type: \
                     {content_type}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).expect("write response");
            }
        });

        Self {
            address,
            responses,
            join: Some(join),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("http://{}{}", self.address, path)
    }

    fn push_response(&self, status_line: &str, content_type: &str, body: String) {
        self.responses.lock().expect("responses").push_back((
            status_line.to_owned(),
            content_type.to_owned(),
            body,
        ));
    }
}

impl Drop for MockUpdateServer {
    fn drop(&mut self) {
        let _ = std::net::TcpStream::connect(&self.address);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
        let _ = self.responses.lock().expect("responses").len();
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(bytes);
    format!("{:x}", digest.finalize())
}

#[test]
fn auto_updater_applies_update_and_requests_restart() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let target = tempdir.path().join("cloudflared");
    let current_contents = b"old-binary";
    let next_contents = b"new-binary";
    fs::write(&target, current_contents).expect("write current binary");

    let checksum = sha256_hex(next_contents);
    let server = MockUpdateServer::start(Vec::new());
    let check_body = format!(
        "{{\"url\":\"{}\",\"version\":\"2026.2.1\",\"checksum\":\"{checksum}\",\"compressed\":false,\"\
         userMessage\":\"\",\"shouldUpdate\":true,\"error\":\"\"}}",
        server.url("/artifact")
    );
    server.push_response("200 OK", "application/json", check_body);
    server.push_response(
        "200 OK",
        "application/octet-stream",
        String::from_utf8_lossy(next_contents).into_owned(),
    );

    let config = runtime_config().with_auto_update(
        RuntimeAutoUpdate::new(AutoUpdateSettings::new(true, Duration::from_millis(10), None))
            .with_target_path_override(target.clone())
            .with_base_url_override(server.url("/check")),
    );

    let execution = run_with_source(
        config,
        test_source([TestBehavior::WaitForShutdown]),
        HarnessBuilder::for_tests()
            .with_shutdown_after(Duration::from_millis(250))
            .build(),
        None,
        None,
    );

    assert!(
        matches!(execution.exit, RuntimeExit::Updated { .. }),
        "auto-update should request restart, summary: {:?}",
        execution.summary_lines
    );
    assert_eq!(fs::read(&target).expect("updated bytes"), next_contents);
}
