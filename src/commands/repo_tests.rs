use super::*;
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};

/// Local scripted HTTP peer: validates paths/auth, never uses stored credentials.
fn server(
    replies: Vec<(&'static str, u16, Value, &'static str)>,
) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let handle = thread::spawn(move || {
        for (path, status, body, headers) in replies {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut request = Vec::new();
            let mut buf = [0; 4096];
            loop {
                let n = socket.read(&mut buf).unwrap();
                if n == 0 {
                    break;
                }
                request.extend_from_slice(&buf[..n]);
                if let Some(end) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                    let head = String::from_utf8_lossy(&request[..end]).to_lowercase();
                    let size = head
                        .lines()
                        .find_map(|l| l.strip_prefix("content-length: "))
                        .and_then(|v| v.parse::<usize>().ok())
                        .unwrap_or(0);
                    if request.len() >= end + 4 + size {
                        break;
                    }
                }
            }
            let request = String::from_utf8(request).unwrap();
            assert!(request.lines().next().unwrap().contains(path), "{request}");
            assert!(request
                .to_lowercase()
                .contains("authorization: apikey test-only"));
            let text = body.to_string();
            write!(socket, "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n{headers}\r\n{text}", text.len()).unwrap();
        }
    });
    (url, handle)
}
fn logs(label: &str) -> Value {
    json!({"data":{"repo_status":label,"upload_response":[],"livelog":[],"queue_info":{"depth":0}}})
}
fn opts(timeout: u64) -> WaitOptions {
    WaitOptions {
        wait: true,
        timeout,
        poll_interval: 1,
    }
}

#[tokio::test]
async fn create_then_wait_tracks_empty_queue_as_processing() {
    let (url, peer) = server(vec![
        (
            "POST /v2/repo/create",
            202,
            json!({"data":{"id":42,"slug":"a/b","owner_username":"a b"}}),
            "",
        ),
        ("GET /v2/repo/logs/42", 200, logs("submitted"), ""),
        ("GET /v2/repo/logs/42", 200, logs("processing"), ""),
        ("GET /v2/repo/logs/42", 200, logs("approved"), ""),
        (
            "POST /v2/repo/deploy/42",
            200,
            json!({"status":"success","data":{"id":42}}),
            "",
        ),
    ]);
    let api = Api::new(&url, "test-only".into()).unwrap();
    let created = api
        .create(&json!({"url":"https://github.com/example/repo"}), 5)
        .await
        .unwrap();
    assert_eq!(created["repo_id"], "42");
    assert!(created["repository_url"]
        .as_str()
        .unwrap()
        .ends_with("/r/a%20b/a%2Fb"));
    assert_eq!(created["metadata_deployment"]["state"], "not_deployed");
    let ready = api
        .wait("42", &opts(5), Instant::now() + Duration::from_secs(5))
        .await
        .unwrap();
    assert_eq!(ready["state"], "ready");
    assert_eq!(ready["metadata_deployment"]["state"], "unknown");
    let deployed = api
        .deploy(
            Some("42"),
            &json!({"tree":[],"layouts":[]}),
            Instant::now() + Duration::from_secs(3),
        )
        .await
        .unwrap();
    assert_eq!(deployed.data.id, 42);
    peer.join().unwrap();
}

#[tokio::test]
async fn rejected_unknown_and_http_errors_stop_without_retry() {
    for (status, body, code) in [
        (200, logs("rejected"), "REPO_REJECTED"),
        (200, logs("unexpected"), "UNKNOWN_REPO_STATE"),
        (403, json!({"error":true}), "STATUS_REQUEST_FAILED"),
        (503, json!({"error":true}), "STATUS_REQUEST_FAILED"),
    ] {
        let (url, peer) = server(vec![("GET /v2/repo/logs/42", status, body, "")]);
        let api = Api::new(&url, "test-only".into()).unwrap();
        let error = api
            .wait("42", &opts(5), Instant::now() + Duration::from_secs(5))
            .await
            .unwrap_err();
        assert!(error.to_string().contains(code), "{error}");
        peer.join().unwrap();
    }
}

#[tokio::test]
async fn retry_after_cannot_extend_deadline_and_timeout_keeps_identity() {
    let (url, peer) = server(vec![(
        "GET /v2/repo/logs/42",
        200,
        logs("processing"),
        "Retry-After: 3600\r\n",
    )]);
    let api = Api::new(&url, "test-only".into()).unwrap();
    let start = Instant::now();
    let err = api
        .wait("42", &opts(1), start + Duration::from_millis(80))
        .await
        .unwrap_err();
    let data: Value = serde_json::from_str(&err.to_string()).unwrap();
    assert_eq!(data["code"], "WAIT_TIMEOUT");
    assert_eq!(data["repo_id"], "42");
    assert_eq!(data["last_status"]["state"], "processing");
    assert!(start.elapsed() < Duration::from_secs(1));
    peer.join().unwrap();
}

#[tokio::test]
async fn stalled_http_is_bounded_by_wait_deadline() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let api = Api::new(
        &format!("http://{}", listener.local_addr().unwrap()),
        "test-only".into(),
    )
    .unwrap();
    let err = api
        .wait("42", &opts(1), Instant::now() + Duration::from_millis(40))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("WAIT_TIMEOUT"));
}

#[tokio::test]
async fn create_legacy_id_and_error_envelopes() {
    let (url, peer) = server(vec![
        ("POST /v2/repo/create", 200, json!({"data":{"id":"42"}}), ""),
        (
            "POST /v2/repo/create",
            422,
            json!({"error":true,"data":{"message":"invalid metadata"}}),
            "",
        ),
        ("POST /v2/repo/create", 200, json!({"data":{}}), ""),
    ]);
    let api = Api::new(&url, "test-only".into()).unwrap();
    assert_eq!(
        api.create(&json!({}), 5).await.unwrap()["repository_url"],
        browser_url(&url, "42")
    );
    assert!(api
        .create(&json!({}), 5)
        .await
        .unwrap_err()
        .to_string()
        .contains("CREATE_FAILED"));
    assert!(api
        .create(&json!({}), 5)
        .await
        .unwrap_err()
        .to_string()
        .contains("outcome unknown"));
    peer.join().unwrap();
}

#[tokio::test]
async fn deploy_distinguishes_processing_service_failure_and_accepted() {
    let (url, peer) = server(vec![
        (
            "POST /v2/repo/deploy/42",
            503,
            json!({"error":true,"data":{"code":503,"message":"Atomization in progress"}}),
            "",
        ),
        (
            "POST /v2/repo/deploy/42",
            503,
            json!({"error":true,"data":{"message":"Unavailable"}}),
            "",
        ),
        (
            "POST /v2/repo/deploy/42",
            202,
            json!({"data":{"id":42}}),
            "",
        ),
        (
            "POST /v2/repo/deploy/42",
            200,
            json!({"status":"success","data":{"id":42}}),
            "",
        ),
    ]);
    let api = Api::new(&url, "test-only".into()).unwrap();
    for code in ["REPO_NOT_READY", "DEPLOY_FAILED", "DEPLOY_PENDING"] {
        let err = api
            .deploy(
                Some("42"),
                &json!({"tree":[]}),
                Instant::now() + Duration::from_secs(3),
            )
            .await
            .unwrap_err();
        assert!(err.to_string().contains(code), "{err}");
    }
    assert_eq!(
        api.deploy(
            Some("42"),
            &json!({"tree":[]}),
            Instant::now() + Duration::from_secs(3)
        )
        .await
        .unwrap()
        .data
        .id,
        42
    );
    peer.join().unwrap();
}

#[test]
fn ids_and_base_urls_cannot_inject_paths_or_credentials() {
    for id in ["0", "", "../1", "1?x=2", "-1", "18446744073709551616"] {
        assert!(validate_id(id).is_err());
    }
    assert!(Api::new("https://user:pass@example.com", "test".into()).is_err());
    assert!(Api::new("file:///tmp", "test".into()).is_err());
    assert!(Api::new("https://example.com?next=evil", "test".into()).is_err());
}
