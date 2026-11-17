//! A package uploaded to an extension catalog as a draft and sent to its review, as the desktop sends them
//! (docs/adr/8059 §10; docs/adr/8057 §107): the catalog's id a path segment, the bytes as they are, the move as the
//! server reads it.

use std::sync::{Arc, Mutex};

use kentos_cloud::Cloud;
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// A server answering `answers` in turn; what each request said (its head and body).
async fn server(answers: Vec<(u16, Value)>) -> (Cloud, Arc<Mutex<Vec<(String, Vec<u8>)>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let cloud = Cloud::new(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let seen = requests.clone();
    tokio::spawn(async move {
        for (status, body) in answers {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            let mut byte = [0];
            while !bytes.ends_with(b"\r\n\r\n") {
                if stream.read(&mut byte).await.unwrap() == 0 {
                    break;
                }
                bytes.push(byte[0]);
            }
            let head = String::from_utf8(bytes).unwrap();
            let length = head
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().unwrap())
                })
                .unwrap_or(0);
            let mut data = vec![0; length];
            stream.read_exact(&mut data).await.unwrap();
            seen.lock().unwrap().push((head, data));
            let body = serde_json::to_vec(&body).unwrap();
            let answer = format!(
                "HTTP/1.1 {status} Durum\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n",
                body.len()
            );
            stream.write_all(answer.as_bytes()).await.unwrap();
            stream.write_all(&body).await.unwrap();
        }
    });
    (cloud, requests)
}

#[tokio::test]
async fn a_package_is_uploaded_as_a_draft_and_sent_to_review() {
    let draft = json!({ "view": "insider", "release": { "state": "draft" }, "moves": ["submit"] });
    let submitted = json!({ "view": "insider", "release": { "state": "submitted" }, "moves": [] });
    let (cloud, seen) = server(vec![(201, draft.clone()), (200, submitted.clone())]).await;
    let package = b"PK\x03\x04 bir paket".to_vec();
    let answer = cloud
        .upload_release("tenant:ornek-belediyesi", package.clone())
        .await
        .expect("uploaded");
    assert_eq!(answer, draft);
    let answer = cloud
        .submit_release(
            "tenant:ornek-belediyesi",
            "tr.ornek.ceviri",
            "0.1.0",
            Some("İlk sürüm.".into()),
        )
        .await
        .expect("submitted");
    assert_eq!(answer, submitted);
    let seen = seen.lock().unwrap();
    let (head, body) = &seen[0];
    assert!(
        head.starts_with("POST /v1/extension-catalogs/tenant:ornek-belediyesi/releases HTTP/1.1"),
        "{head}"
    );
    assert!(
        head.to_lowercase()
            .contains("content-type: application/octet-stream"),
        "{head}"
    );
    assert!(
        head.to_lowercase().contains("x-kentos-client: desktop"),
        "a program: {head}"
    );
    assert_eq!(body, &package, "the bytes as they are");
    let (head, body) = &seen[1];
    assert!(
        head.starts_with(
            "POST /v1/extension-catalogs/tenant:ornek-belediyesi/releases/tr.ornek.ceviri/0.1.0/moves HTTP/1.1"
        ),
        "{head}"
    );
    let moved: Value = serde_json::from_slice(body).unwrap();
    assert_eq!(moved, json!({ "action": "submit", "note": "İlk sürüm." }));
}

/// The server's refusal comes back as its failure: its status and its words.
#[tokio::test]
async fn a_refused_upload_says_the_servers_words() {
    // The API's error body: the flow's code as `error`, its reason and remedy as `message`.
    let refused = json!({ "error": "version_exists", "message": "Bu sürüm katalogda var." });
    let (cloud, _) = server(vec![(409, refused)]).await;
    let failure = cloud
        .upload_release("personal:ayse", b"paket".to_vec())
        .await
        .expect_err("refused");
    assert_eq!(failure.status, 409);
    assert!(
        failure.to_string().contains("Bu sürüm katalogda var."),
        "{failure}"
    );
}
