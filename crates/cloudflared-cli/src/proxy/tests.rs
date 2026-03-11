use super::*;
use cloudflared_config::{IngressMatch, OriginRequestConfig};

fn expect_proxy_state(msg: RuntimeCommand, expected: ProxySeamState) -> String {
    match msg {
        RuntimeCommand::ProxyState { state, detail } => {
            assert_eq!(state, expected);
            detail
        }
        other => panic!("expected ProxyState({expected:?}), got: {other:?}"),
    }
}

fn expect_service_status(msg: RuntimeCommand) -> String {
    match msg {
        RuntimeCommand::ServiceStatus { service, detail } => {
            assert_eq!(service, PROXY_SEAM_NAME);
            detail
        }
        other => panic!("expected ServiceStatus for {PROXY_SEAM_NAME}, got: {other:?}"),
    }
}

fn expect_protocol_state(msg: RuntimeCommand, expected: ProtocolBridgeState) -> String {
    match msg {
        RuntimeCommand::ProtocolState { state, detail } => {
            assert_eq!(state, expected);
            detail
        }
        other => panic!("expected ProtocolState({expected:?}), got: {other:?}"),
    }
}

fn http_status_rule(hostname: Option<&str>, code: u16) -> IngressRule {
    IngressRule {
        matcher: IngressMatch {
            hostname: hostname.map(String::from),
            punycode_hostname: None,
            path: None,
        },
        service: IngressService::HttpStatus(code),
        origin_request: OriginRequestConfig::default(),
    }
}

fn catch_all_rule(code: u16) -> IngressRule {
    self::http_status_rule(None, code)
}

fn build_request(method: &str, path: &[u8], host: Option<&str>) -> RequestHeader {
    let mut request = RequestHeader::build(method, path, None).expect("test request should build");
    if let Some(host_value) = host {
        request
            .insert_header("host", host_value)
            .expect("test host header should insert");
    }
    request
}

// -- Dependency admission (preserved from 3.4a) --

#[test]
fn pingora_http_request_type_admitted() {
    let header = RequestHeader::build("GET", b"/", None);
    assert!(
        header.is_ok(),
        "Pingora HTTP request type should build at the admitted seam"
    );
}

// -- Origin/proxy path tests (3.4c) --

#[test]
fn handle_request_returns_http_status_from_catch_all() {
    let seam = PingoraProxySeam::new(vec![catch_all_rule(503)]);
    let request = build_request("GET", b"/", None);
    let response = seam.handle_request(&request);
    assert_eq!(response.status.as_u16(), 503);
}

#[test]
fn handle_request_matches_hostname_to_origin() {
    let seam = PingoraProxySeam::new(vec![
        http_status_rule(Some("example.com"), 200),
        catch_all_rule(404),
    ]);
    let request = build_request("GET", b"/", Some("example.com"));
    let response = seam.handle_request(&request);
    assert_eq!(response.status.as_u16(), 200);
}

#[test]
fn handle_request_falls_through_to_catch_all() {
    let seam = PingoraProxySeam::new(vec![
        http_status_rule(Some("example.com"), 200),
        catch_all_rule(404),
    ]);
    let request = build_request("GET", b"/", Some("other.com"));
    let response = seam.handle_request(&request);
    assert_eq!(response.status.as_u16(), 404);
}

#[test]
fn handle_request_returns_502_for_empty_ingress() {
    let seam = PingoraProxySeam::new(vec![]);
    let request = build_request("GET", b"/", None);
    let response = seam.handle_request(&request);
    assert_eq!(response.status.as_u16(), 502);
}

#[test]
fn handle_request_returns_502_for_unimplemented_origin() {
    let seam = PingoraProxySeam::new(vec![IngressRule {
        matcher: IngressMatch::default(),
        service: IngressService::HelloWorld,
        origin_request: OriginRequestConfig::default(),
    }]);
    let request = build_request("GET", b"/", None);
    let response = seam.handle_request(&request);
    assert_eq!(response.status.as_u16(), 502);
}

#[test]
fn ingress_count_reflects_handoff_rules() {
    let seam = PingoraProxySeam::new(vec![
        http_status_rule(Some("a.example.com"), 200),
        http_status_rule(Some("b.example.com"), 201),
        catch_all_rule(503),
    ]);
    assert_eq!(seam.ingress_count(), 3);
}

// -- Lifecycle participation (evolved from 3.4b) --

#[tokio::test]
async fn proxy_seam_reports_origin_path_and_shuts_down() {
    let (command_tx, mut command_rx) = mpsc::channel(16);
    let shutdown = CancellationToken::new();
    let mut child_tasks = JoinSet::new();

    let seam = PingoraProxySeam::new(vec![catch_all_rule(503)]);
    seam.spawn(command_tx, None, shutdown.clone(), &mut child_tasks);

    let msg = command_rx.recv().await.expect("should receive proxy state");
    let detail = expect_proxy_state(msg, ProxySeamState::Admitted);
    assert!(detail.contains("ingress-rules=1"));

    // Seam should report the admitted origin/proxy path on startup.
    let msg = command_rx.recv().await.expect("should receive origin status");
    let detail = expect_service_status(msg);
    assert!(
        detail.contains("origin-proxy-admitted"),
        "startup status should report admitted origin path, got: {detail}"
    );
    assert!(
        detail.contains("ingress-rules=1"),
        "startup status should report ingress rule count, got: {detail}"
    );

    shutdown.cancel();

    let msg = command_rx
        .recv()
        .await
        .expect("should receive shutdown proxy state");
    let detail = expect_proxy_state(msg, ProxySeamState::ShutdownAcknowledged);
    assert!(detail.contains("shutdown"));

    let msg = command_rx.recv().await.expect("should receive shutdown status");
    let detail = expect_service_status(msg);
    assert!(detail.contains("shutdown acknowledged"));

    let result = child_tasks.join_next().await;
    assert!(result.is_some(), "proxy seam child task should complete");
    match result
        .expect("join should succeed")
        .expect("task should not panic")
    {
        ChildTask::ProxySeam => {}
        other => panic!("expected ChildTask::ProxySeam, got: {other:?}"),
    }
}

// -- Wire/protocol bridge (3.5) --

#[tokio::test]
async fn proxy_seam_receives_protocol_registration() {
    let (command_tx, mut command_rx) = mpsc::channel(16);
    let (protocol_sender, protocol_receiver) = crate::protocol::protocol_bridge();
    let shutdown = CancellationToken::new();
    let mut child_tasks = JoinSet::new();

    let seam = PingoraProxySeam::new(vec![catch_all_rule(503)]);
    seam.spawn(
        command_tx,
        Some(protocol_receiver),
        shutdown.clone(),
        &mut child_tasks,
    );

    // Startup status.
    let msg = command_rx.recv().await.expect("should receive startup status");
    assert!(matches!(
        msg,
        RuntimeCommand::ProxyState {
            state: ProxySeamState::Admitted,
            ..
        }
    ));

    let msg = command_rx.recv().await.expect("should receive origin status");
    assert!(matches!(msg, RuntimeCommand::ServiceStatus { .. }));

    // Simulate transport sending registration event.
    protocol_sender
        .send(ProtocolEvent::Registered {
            peer: "127.0.0.1:7844".to_owned(),
        })
        .await
        .expect("protocol bridge should stay available during registration test");

    let msg = command_rx
        .recv()
        .await
        .expect("should receive protocol state update");
    let detail = expect_protocol_state(msg, ProtocolBridgeState::RegistrationObserved);
    assert!(detail.contains("127.0.0.1:7844"));

    let msg = command_rx
        .recv()
        .await
        .expect("should receive proxy registration state");
    let detail = expect_proxy_state(msg, ProxySeamState::RegistrationObserved);
    assert!(detail.contains("127.0.0.1:7844"));

    // Proxy should report the protocol bridge registration.
    let msg = command_rx
        .recv()
        .await
        .expect("should receive protocol bridge status");
    let detail = expect_service_status(msg);
    assert!(
        detail.contains("protocol-bridge: session registered"),
        "expected protocol bridge registration, got: {detail}"
    );
    assert!(
        detail.contains("peer=127.0.0.1:7844"),
        "expected peer address, got: {detail}"
    );

    shutdown.cancel();

    let msg = command_rx
        .recv()
        .await
        .expect("should receive shutdown proxy state");
    assert!(matches!(
        msg,
        RuntimeCommand::ProxyState {
            state: ProxySeamState::ShutdownAcknowledged,
            ..
        }
    ));

    let msg = command_rx.recv().await.expect("should receive shutdown status");
    let detail = expect_service_status(msg);
    assert!(detail.contains("shutdown acknowledged"));
}

#[tokio::test]
async fn proxy_seam_handles_bridge_closure_without_registration() {
    let (command_tx, mut command_rx) = mpsc::channel(16);
    let (protocol_sender, protocol_receiver) = crate::protocol::protocol_bridge();
    let shutdown = CancellationToken::new();
    let mut child_tasks = JoinSet::new();

    let seam = PingoraProxySeam::new(vec![catch_all_rule(503)]);
    seam.spawn(
        command_tx,
        Some(protocol_receiver),
        shutdown.clone(),
        &mut child_tasks,
    );

    // Startup status.
    let _ = command_rx.recv().await;
    let _ = command_rx.recv().await;

    // Drop sender without sending registration — simulates
    // transport failure before reaching the protocol boundary.
    drop(protocol_sender);

    let msg = command_rx
        .recv()
        .await
        .expect("should receive bridge-closed state");
    let detail = expect_protocol_state(msg, ProtocolBridgeState::BridgeClosed);
    assert!(detail.contains("before transport registration"));

    let msg = command_rx
        .recv()
        .await
        .expect("should receive bridge-closure status after closure");
    let detail = expect_service_status(msg);
    assert!(detail.contains("proxy bridge closed before transport registration"));

    shutdown.cancel();

    let msg = command_rx
        .recv()
        .await
        .expect("should receive shutdown proxy state after bridge closure");
    assert!(matches!(
        msg,
        RuntimeCommand::ProxyState {
            state: ProxySeamState::ShutdownAcknowledged,
            ..
        }
    ));

    let msg = command_rx
        .recv()
        .await
        .expect("should receive shutdown status after bridge closure");
    let detail = expect_service_status(msg);
    assert!(
        detail.contains("shutdown acknowledged"),
        "expected shutdown ack after bridge closure, got: {detail}"
    );
}
