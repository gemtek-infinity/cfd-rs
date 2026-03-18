use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use zip::CompressionMethod;
use zip::write::SimpleFileOptions;

use crate::logging::{HostLogConfiguration, HostLogError, LOG_FILENAME, collect_host_logs};

use super::http::normalize_base_url;
use super::network::collect_network_traces;
use super::{
    DiagnosticBundle, DiagnosticHttpClient, DiagnosticOptions, DiagnosticRunError, DiagnosticTaskResult,
    TunnelState, diagnostics_http_client, find_metrics_server_http,
};

const TUNNEL_STATE_JOB_NAME: &str = "tunnel state";
const SYSTEM_INFORMATION_JOB_NAME: &str = "system information";
const GOROUTINE_JOB_NAME: &str = "goroutine profile";
const HEAP_JOB_NAME: &str = "heap profile";
const METRICS_JOB_NAME: &str = "metrics";
const LOG_INFORMATION_JOB_NAME: &str = "log information";
const RAW_NETWORK_JOB_NAME: &str = "raw network information";
const NETWORK_JOB_NAME: &str = "network information";
const CLI_CONFIGURATION_JOB_NAME: &str = "cli configuration";
const CONFIGURATION_JOB_NAME: &str = "configuration";
const JOB_REPORT_NAME: &str = "job report";

const SYSTEM_INFORMATION_FILENAME: &str = "systeminformation.json";
const METRICS_FILENAME: &str = "metrics.txt";
const ZIP_NAME_PREFIX: &str = "cloudflared-diag";
const HEAP_FILENAME: &str = "heap.pprof";
const GOROUTINE_FILENAME: &str = "goroutine.pprof";
const NETWORK_FILENAME: &str = "network.json";
const RAW_NETWORK_FILENAME: &str = "raw-network.txt";
const TUNNEL_STATE_FILENAME: &str = "tunnelstate.json";
const CLI_CONFIGURATION_FILENAME: &str = "cli-configuration.json";
const CONFIGURATION_FILENAME: &str = "configuration.json";
const TASK_RESULT_FILENAME: &str = "task-result.json";
const TAIL_MAX_NUMBER_OF_LINES: &str = "10000";

pub fn run_diagnostic(options: DiagnosticOptions) -> Result<DiagnosticBundle, DiagnosticRunError> {
    let resolved = resolve_instance(&options)?;
    let mut client = diagnostics_http_client();
    client.set_base_url(resolved.base_url.clone());

    let (mut task_results, mut files) = run_jobs(&client, resolved.discovered_tunnel, &options);
    let task_report = write_task_report(&task_results)?;
    files.push(task_report);
    task_results.insert(JOB_REPORT_NAME.to_owned(), DiagnosticTaskResult::success());

    let zip_path = create_zip_file(options.output_dir.as_deref(), &files)?;
    cleanup_files(&files);

    Ok(DiagnosticBundle {
        selected_address: resolved.base_url,
        zip_path,
        task_results,
    })
}

struct ResolvedInstance {
    base_url: String,
    discovered_tunnel: Option<TunnelState>,
}

fn resolve_instance(options: &DiagnosticOptions) -> Result<ResolvedInstance, DiagnosticRunError> {
    if let Some(address) = options.address.as_deref() {
        return Ok(ResolvedInstance {
            base_url: normalize_base_url(address)?,
            discovered_tunnel: None,
        });
    }

    let discovered = find_metrics_server_http(&options.known_addresses)?;
    Ok(ResolvedInstance {
        base_url: normalize_base_url(&discovered.address)?,
        discovered_tunnel: Some(discovered.state),
    })
}

fn run_jobs(
    client: &DiagnosticHttpClient,
    discovered_tunnel: Option<TunnelState>,
    options: &DiagnosticOptions,
) -> (BTreeMap<String, DiagnosticTaskResult>, Vec<CollectedFile>) {
    let mut results = BTreeMap::new();
    let mut files = Vec::new();

    record_job(
        TUNNEL_STATE_JOB_NAME,
        collect_tunnel_state(client, discovered_tunnel),
        &mut results,
        &mut files,
    );
    record_optional_job(
        SYSTEM_INFORMATION_JOB_NAME,
        options.toggles.no_diag_system,
        collect_json_endpoint(client, "/diag/system", SYSTEM_INFORMATION_FILENAME),
        &mut results,
        &mut files,
    );
    record_optional_job(
        GOROUTINE_JOB_NAME,
        options.toggles.no_diag_runtime,
        collect_bytes_endpoint(client, "debug/pprof/goroutine", GOROUTINE_FILENAME),
        &mut results,
        &mut files,
    );
    record_optional_job(
        HEAP_JOB_NAME,
        options.toggles.no_diag_runtime,
        collect_bytes_endpoint(client, "debug/pprof/heap", HEAP_FILENAME),
        &mut results,
        &mut files,
    );
    record_optional_job(
        METRICS_JOB_NAME,
        options.toggles.no_diag_metrics,
        collect_bytes_endpoint(client, "metrics", METRICS_FILENAME),
        &mut results,
        &mut files,
    );
    record_optional_job(
        LOG_INFORMATION_JOB_NAME,
        options.toggles.no_diag_logs,
        collect_logs(client, options),
        &mut results,
        &mut files,
    );
    if !options.toggles.no_diag_network {
        let traces = collect_network_traces();
        record_job(
            RAW_NETWORK_JOB_NAME,
            write_raw_network_report(&traces),
            &mut results,
            &mut files,
        );
        record_job(
            NETWORK_JOB_NAME,
            write_network_json_report(&traces),
            &mut results,
            &mut files,
        );
    }
    record_job(
        CLI_CONFIGURATION_JOB_NAME,
        collect_json_endpoint(client, "/diag/configuration", CLI_CONFIGURATION_FILENAME),
        &mut results,
        &mut files,
    );
    record_job(
        CONFIGURATION_JOB_NAME,
        collect_json_endpoint(client, "/config", CONFIGURATION_FILENAME),
        &mut results,
        &mut files,
    );

    (results, files)
}

struct JobExecution {
    file: Option<CollectedFile>,
    error: Option<String>,
}

#[derive(Clone)]
struct CollectedFile {
    archive_name: &'static str,
    path: PathBuf,
    cleanup_required: bool,
}

fn record_optional_job(
    name: &'static str,
    bypass: bool,
    execution: JobExecution,
    results: &mut BTreeMap<String, DiagnosticTaskResult>,
    files: &mut Vec<CollectedFile>,
) {
    if bypass {
        return;
    }

    record_job(name, execution, results, files);
}

fn record_job(
    name: &'static str,
    execution: JobExecution,
    results: &mut BTreeMap<String, DiagnosticTaskResult>,
    files: &mut Vec<CollectedFile>,
) {
    if let Some(file) = execution.file {
        files.push(file);
    }

    let result = match execution.error {
        Some(error) => DiagnosticTaskResult::failure(error),
        None => DiagnosticTaskResult::success(),
    };
    results.insert(name.to_owned(), result);
}

fn collect_tunnel_state(
    client: &DiagnosticHttpClient,
    discovered_tunnel: Option<TunnelState>,
) -> JobExecution {
    match discovered_tunnel {
        Some(state) => write_pretty_json(TUNNEL_STATE_FILENAME, &state),
        None => collect_json_endpoint(client, "/diag/tunnel", TUNNEL_STATE_FILENAME),
    }
}

fn collect_json_endpoint(
    client: &DiagnosticHttpClient,
    endpoint: &str,
    archive_name: &'static str,
) -> JobExecution {
    let path = unique_temp_path(archive_name);
    let mut file = match std::fs::File::create(&path) {
        Ok(file) => file,
        Err(error) => {
            return JobExecution {
                file: None,
                error: Some(format!("temporary file creation failed: {error}")),
            };
        }
    };

    let error = client.copy_pretty_json(endpoint, &mut file).err();
    JobExecution {
        file: Some(CollectedFile {
            archive_name,
            path,
            cleanup_required: true,
        }),
        error,
    }
}

fn collect_bytes_endpoint(
    client: &DiagnosticHttpClient,
    endpoint: &str,
    archive_name: &'static str,
) -> JobExecution {
    let path = unique_temp_path(archive_name);
    let mut file = match std::fs::File::create(&path) {
        Ok(file) => file,
        Err(error) => {
            return JobExecution {
                file: None,
                error: Some(format!("temporary file creation failed: {error}")),
            };
        }
    };

    let error = client.copy_bytes(endpoint, &mut file).err();
    JobExecution {
        file: Some(CollectedFile {
            archive_name,
            path,
            cleanup_required: true,
        }),
        error,
    }
}

fn collect_logs(client: &DiagnosticHttpClient, options: &DiagnosticOptions) -> JobExecution {
    if let Some(pod_id) = options.pod_id.as_deref() {
        return run_command_log_collector(
            "kubectl",
            &kubernetes_args(options.container_id.as_deref(), pod_id),
        );
    }
    if let Some(container_id) = options.container_id.as_deref() {
        return run_command_log_collector(
            "docker",
            &["logs", "--tail", TAIL_MAX_NUMBER_OF_LINES, container_id],
        );
    }

    match client.get_log_configuration() {
        Ok(configuration) => {
            let host_config = HostLogConfiguration {
                uid: configuration.uid.parse::<u32>().unwrap_or(u32::MAX),
                log_file: configuration.log_file.map(PathBuf::from),
                log_directory: configuration.log_directory.map(PathBuf::from),
            };
            match collect_host_logs(&host_config) {
                Ok(collection) => JobExecution {
                    file: Some(CollectedFile {
                        archive_name: LOG_FILENAME,
                        path: collection.path,
                        cleanup_required: collection.cleanup_required,
                    }),
                    error: None,
                },
                Err(error) => match error {
                    HostLogError::InvalidConfiguration => JobExecution {
                        file: None,
                        error: Some(error.to_string()),
                    },
                    other => JobExecution {
                        file: None,
                        error: Some(format!("error collecting logs: {other}")),
                    },
                },
            }
        }
        Err(error) => JobExecution {
            file: None,
            error: Some(format!("error getting log configuration: {error}")),
        },
    }
}

fn kubernetes_args<'a>(container_id: Option<&'a str>, pod_id: &'a str) -> Vec<&'a str> {
    let mut args = vec!["logs", pod_id, "--tail", TAIL_MAX_NUMBER_OF_LINES];
    if let Some(container_id) = container_id {
        args.push("-c");
        args.push(container_id);
    }
    args
}

fn run_command_log_collector(command: &str, args: &[&str]) -> JobExecution {
    let path = unique_temp_path(LOG_FILENAME);
    let mut output = match std::fs::File::create(&path) {
        Ok(file) => file,
        Err(error) => {
            return JobExecution {
                file: None,
                error: Some(format!("temporary file creation failed: {error}")),
            };
        }
    };

    let error = pipe_command_output(command, args, &mut output).err();
    JobExecution {
        file: Some(CollectedFile {
            archive_name: LOG_FILENAME,
            path,
            cleanup_required: true,
        }),
        error,
    }
}

fn pipe_command_output(command: &str, args: &[&str], writer: &mut dyn Write) -> Result<(), String> {
    let mut child = Command::new(command)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("error running command '{command}': {error}"))?;

    if let Some(mut stdout) = child.stdout.take() {
        std::io::copy(&mut stdout, writer)
            .map_err(|error| format!("error copying stdout from {command}: {error}"))?;
    }
    if let Some(mut stderr) = child.stderr.take() {
        std::io::copy(&mut stderr, writer)
            .map_err(|error| format!("error copying stderr from {command}: {error}"))?;
    }

    let status = child
        .wait()
        .map_err(|error| format!("error waiting from command '{command}': {error}"))?;
    if !status.success() {
        return Err(format!("error waiting from command '{command}': exit {status}"));
    }

    Ok(())
}

fn write_raw_network_report(traces: &BTreeMap<String, super::NetworkTrace>) -> JobExecution {
    let path = unique_temp_path(RAW_NETWORK_FILENAME);
    let mut file = match std::fs::File::create(&path) {
        Ok(file) => file,
        Err(error) => {
            return JobExecution {
                file: None,
                error: Some(format!("temporary file creation failed: {error}")),
            };
        }
    };

    let mut first_error = None;
    for (name, trace) in traces {
        let body = if trace.raw.is_empty() {
            "no content".to_owned()
        } else {
            trace.raw.clone()
        };
        if let Err(error) = writeln!(file, "{name}\n{body}") {
            first_error = Some(format!("error writing raw network information: {error}"));
            break;
        }
        if first_error.is_none() {
            first_error = trace.error.clone();
        }
    }

    JobExecution {
        file: Some(CollectedFile {
            archive_name: RAW_NETWORK_FILENAME,
            path,
            cleanup_required: true,
        }),
        error: first_error,
    }
}

fn write_network_json_report(traces: &BTreeMap<String, super::NetworkTrace>) -> JobExecution {
    let path = unique_temp_path(NETWORK_FILENAME);
    let mut payload = BTreeMap::new();
    let mut first_error = None;

    for (name, trace) in traces {
        payload.insert(name.clone(), trace.hops.clone());
        if first_error.is_none() {
            first_error = trace.error.clone();
        }
    }

    let execution = write_pretty_json_with_name(NETWORK_FILENAME, &payload, path);
    if execution.error.is_some() {
        return execution;
    }

    JobExecution {
        file: execution.file,
        error: first_error,
    }
}

fn write_pretty_json<T>(archive_name: &'static str, value: &T) -> JobExecution
where
    T: serde::Serialize,
{
    write_pretty_json_with_name(archive_name, value, unique_temp_path(archive_name))
}

fn write_pretty_json_with_name<T>(archive_name: &'static str, value: &T, path: PathBuf) -> JobExecution
where
    T: serde::Serialize,
{
    match std::fs::File::create(&path) {
        Ok(mut file) => {
            let error = serde_json::to_writer_pretty(&mut file, value)
                .map_err(|error| error.to_string())
                .and_then(|_| file.write_all(b"\n").map_err(|error| error.to_string()))
                .err();

            JobExecution {
                file: Some(CollectedFile {
                    archive_name,
                    path,
                    cleanup_required: true,
                }),
                error,
            }
        }
        Err(error) => JobExecution {
            file: None,
            error: Some(format!("temporary file creation failed: {error}")),
        },
    }
}

fn write_task_report(
    task_results: &BTreeMap<String, DiagnosticTaskResult>,
) -> Result<CollectedFile, DiagnosticRunError> {
    let path = unique_temp_path(TASK_RESULT_FILENAME);
    let mut file = std::fs::File::create(&path)
        .map_err(|error| DiagnosticRunError::Fatal(format!("temporary file creation failed: {error}")))?;
    serde_json::to_writer_pretty(&mut file, task_results)
        .map_err(|error| DiagnosticRunError::Fatal(format!("error encoding task results: {error}")))?;
    file.write_all(b"\n")
        .map_err(|error| DiagnosticRunError::Fatal(format!("error encoding task results: {error}")))?;

    Ok(CollectedFile {
        archive_name: TASK_RESULT_FILENAME,
        path,
        cleanup_required: true,
    })
}

fn create_zip_file(
    output_dir: Option<&Path>,
    files: &[CollectedFile],
) -> Result<PathBuf, DiagnosticRunError> {
    let directory = match output_dir {
        Some(path) => path.to_path_buf(),
        None => std::env::current_dir().map_err(|error| {
            DiagnosticRunError::Fatal(format!("failed to determine current directory: {error}"))
        })?,
    };
    let zip_path = directory.join(format!("{}-{}.zip", ZIP_NAME_PREFIX, diagnostic_timestamp()));
    let file = std::fs::File::create(&zip_path).map_err(|error| {
        DiagnosticRunError::Fatal(format!("error creating file {}: {error}", zip_path.display()))
    })?;

    let mut archive = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    for collected in files {
        let mut input = std::fs::File::open(&collected.path).map_err(|error| {
            DiagnosticRunError::Fatal(format!(
                "error opening file {}: {error}",
                collected.path.display()
            ))
        })?;
        archive
            .start_file(collected.archive_name, options)
            .map_err(|error| DiagnosticRunError::Fatal(format!("error creating archive writer: {error}")))?;
        std::io::copy(&mut input, &mut archive).map_err(|error| {
            DiagnosticRunError::Fatal(format!(
                "error copying file {}: {error}",
                collected.path.display()
            ))
        })?;
    }

    archive
        .finish()
        .map_err(|error| DiagnosticRunError::Fatal(format!("error finalizing archive: {error}")))?;

    Ok(zip_path)
}

fn cleanup_files(files: &[CollectedFile]) {
    for file in files {
        if file.cleanup_required {
            let _ = std::fs::remove_file(&file.path);
        }
    }
}

fn unique_temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("{name}.{}", uuid::Uuid::new_v4()))
}

fn diagnostic_timestamp() -> String {
    match Command::new("date").arg("--iso-8601=seconds").output() {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().replace(':', "-")
        }
        _ => format!(
            "fallback-{}",
            std::time::SystemTime::now()
                .elapsed()
                .unwrap_or_default()
                .as_secs()
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use zip::ZipArchive;

    use super::*;

    struct MockDiagnosticServer {
        address: String,
        running: Arc<AtomicBool>,
        join: Option<thread::JoinHandle<()>>,
    }

    impl MockDiagnosticServer {
        fn start(routes: BTreeMap<String, (u16, String, &'static str)>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
            listener.set_nonblocking(true).expect("set nonblocking");
            let address = listener.local_addr().expect("addr").to_string();
            let running = Arc::new(AtomicBool::new(true));
            let keep_running = Arc::clone(&running);

            let join = thread::spawn(move || {
                while keep_running.load(Ordering::Relaxed) {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            let mut buffer = [0_u8; 4096];
                            let read = stream.read(&mut buffer).unwrap_or(0);
                            let request = String::from_utf8_lossy(&buffer[..read]);
                            let path = request
                                .lines()
                                .next()
                                .and_then(|line| line.split_whitespace().nth(1))
                                .unwrap_or("/");
                            let (status, body, content_type) = routes.get(path).cloned().unwrap_or((
                                404,
                                "not found".to_owned(),
                                "text/plain",
                            ));
                            let response = format!(
                                "HTTP/1.1 {status} OK\r\nContent-Length: {}\r\nContent-Type: \
                                 {content_type}\r\nConnection: close\r\n\r\n{body}",
                                body.len()
                            );
                            let _ = stream.write_all(response.as_bytes());
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(5));
                        }
                        Err(_) => break,
                    }
                }
            });

            Self {
                address,
                running,
                join: Some(join),
            }
        }
    }

    impl Drop for MockDiagnosticServer {
        fn drop(&mut self) {
            self.running.store(false, Ordering::Relaxed);
            let _ = std::net::TcpStream::connect(&self.address);
            if let Some(join) = self.join.take() {
                let _ = join.join();
            }
        }
    }

    fn temp_dir(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("cfdrs-diagnostic-{name}-{suffix}"));
        std::fs::create_dir_all(&path).expect("mkdir");
        path
    }

    fn base_routes(diag_configuration: &str) -> BTreeMap<String, (u16, String, &'static str)> {
        BTreeMap::from([
            (
                "/diag/tunnel".to_owned(),
                (
                    200,
                    "{\"tunnelID\":\"00000000-0000-0000-0000-000000000000\",\"connectorID\":\"\
                     11111111-1111-1111-1111-111111111111\"}"
                        .to_owned(),
                    "application/json",
                ),
            ),
            (
                "/diag/system".to_owned(),
                (
                    200,
                    "{\"info\":{\"osSystem\":\"Linux\"},\"errors\":{}}".to_owned(),
                    "application/json",
                ),
            ),
            (
                "/debug/pprof/goroutine".to_owned(),
                (501, "pprof deferred\n".to_owned(), "text/plain"),
            ),
            (
                "/debug/pprof/heap".to_owned(),
                (501, "pprof deferred\n".to_owned(), "text/plain"),
            ),
            (
                "//debug/pprof/goroutine".to_owned(),
                (501, "pprof deferred\n".to_owned(), "text/plain"),
            ),
            (
                "//debug/pprof/heap".to_owned(),
                (501, "pprof deferred\n".to_owned(), "text/plain"),
            ),
            (
                "//metrics".to_owned(),
                (200, "build_info 1\n".to_owned(), "text/plain"),
            ),
            (
                "/metrics".to_owned(),
                (200, "build_info 1\n".to_owned(), "text/plain"),
            ),
            (
                "/diag/configuration".to_owned(),
                (200, diag_configuration.to_owned(), "application/json"),
            ),
            (
                "/config".to_owned(),
                (
                    200,
                    "{\"version\":1,\"config\":{}}".to_owned(),
                    "application/json",
                ),
            ),
        ])
    }

    #[test]
    fn run_diagnostic_writes_expected_zip_members() {
        let server = MockDiagnosticServer::start(base_routes(
            "{\"uid\":\"1000\",\"logfile\":\"/tmp/cloudflared.log\"}",
        ));
        let output_dir = temp_dir("zip");
        std::fs::write("/tmp/cloudflared.log", "logs\n").expect("write log");

        let mut options = DiagnosticOptions::new(Vec::new());
        options.address = Some(server.address.clone());
        options.output_dir = Some(output_dir.clone());

        let bundle = run_diagnostic(options).expect("bundle");
        assert!(bundle.zip_path.exists());

        let archive_file = std::fs::File::open(&bundle.zip_path).expect("open zip");
        let mut archive = ZipArchive::new(archive_file).expect("archive");
        let names = (0..archive.len())
            .map(|index| archive.by_index(index).expect("file").name().to_owned())
            .collect::<Vec<_>>();

        assert!(names.contains(&TUNNEL_STATE_FILENAME.to_owned()));
        assert!(names.contains(&SYSTEM_INFORMATION_FILENAME.to_owned()));
        assert!(names.contains(&GOROUTINE_FILENAME.to_owned()));
        assert!(names.contains(&HEAP_FILENAME.to_owned()));
        assert!(names.contains(&METRICS_FILENAME.to_owned()));
        assert!(names.contains(&LOG_FILENAME.to_owned()));
        assert!(names.contains(&RAW_NETWORK_FILENAME.to_owned()));
        assert!(names.contains(&NETWORK_FILENAME.to_owned()));
        assert!(names.contains(&CLI_CONFIGURATION_FILENAME.to_owned()));
        assert!(names.contains(&CONFIGURATION_FILENAME.to_owned()));
        assert!(names.contains(&TASK_RESULT_FILENAME.to_owned()));

        let _ = std::fs::remove_file("/tmp/cloudflared.log");
        let _ = std::fs::remove_file(bundle.zip_path);
        let _ = std::fs::remove_dir_all(output_dir);
    }

    #[test]
    fn run_diagnostic_records_invalid_log_configuration_in_task_report() {
        let server = MockDiagnosticServer::start(base_routes("{\"uid\":\"1000\"}"));
        let output_dir = temp_dir("invalid-log-config");

        let mut options = DiagnosticOptions::new(Vec::new());
        options.address = Some(server.address.clone());
        options.output_dir = Some(output_dir.clone());
        options.toggles.no_diag_network = true;

        let bundle = run_diagnostic(options).expect("bundle");
        assert!(bundle.had_errors());
        assert!(bundle.contains_error_text("provided log configuration is invalid"));

        let archive_file = std::fs::File::open(&bundle.zip_path).expect("open zip");
        let mut archive = ZipArchive::new(archive_file).expect("archive");
        let mut task_report = String::new();
        archive
            .by_name(TASK_RESULT_FILENAME)
            .expect("task report")
            .read_to_string(&mut task_report)
            .expect("read task report");
        assert!(task_report.contains("\"log information\""));
        assert!(task_report.contains("\"result\": \"failure\""));

        let _ = std::fs::remove_file(bundle.zip_path);
        let _ = std::fs::remove_dir_all(output_dir);
    }
}
