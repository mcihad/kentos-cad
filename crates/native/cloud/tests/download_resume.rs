//! A revision download cut short goes on from where it stopped (docs/adr/0045):
//! a server that breaks off the first answer halfway must be asked for the
//! rest with `Range`, and the pieces must make the file its entity tag names.
//! A small stand-in server on this computer plays the cut.

use std::sync::{Arc, Mutex};

use kentos_cloud::Cloud;
use kentos_cloud::api::hex_sha256;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use uuid::Uuid;

/// Reads one request's head and returns it, lowercase.
async fn head(stream: &mut tokio::net::TcpStream) -> String {
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    while !buf.ends_with(b"\r\n\r\n") {
        if stream.read(&mut byte).await.unwrap() == 0 {
            break;
        }
        buf.push(byte[0]);
    }
    String::from_utf8_lossy(&buf).to_lowercase()
}

#[tokio::test]
async fn a_cut_revision_download_goes_on_with_a_range() {
    let file: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();
    let sha = hex_sha256(&file);
    let half = file.len() / 2;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let heads = Arc::new(Mutex::new(Vec::new()));
    let seen = heads.clone();
    let (served, sha_served) = (file.clone(), sha.clone());
    tokio::spawn(async move {
        // First: the whole file declared, half of it sent, then the connection drops.
        let (mut s, _) = listener.accept().await.unwrap();
        let h = head(&mut s).await;
        seen.lock().unwrap().push(h);
        let first = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/octet-stream\r\ncontent-length: {}\r\netag: \"{sha_served}\"\r\nx-kentos-revision: 1\r\n\r\n",
            served.len()
        );
        s.write_all(first.as_bytes()).await.unwrap();
        s.write_all(&served[..half]).await.unwrap();
        drop(s);
        // Then: the rest, as a range.
        let (mut s, _) = listener.accept().await.unwrap();
        let h = head(&mut s).await;
        seen.lock().unwrap().push(h);
        let rest = format!(
            "HTTP/1.1 206 Partial Content\r\ncontent-type: application/octet-stream\r\ncontent-length: {}\r\ncontent-range: bytes {half}-{}/{}\r\netag: \"{sha_served}\"\r\n\r\n",
            served.len() - half,
            served.len() - 1,
            served.len()
        );
        s.write_all(rest.as_bytes()).await.unwrap();
        s.write_all(&served[half..]).await.unwrap();
        s.shutdown().await.unwrap();
    });
    let cloud = Cloud::new(&format!("http://{addr}")).unwrap();
    let got = cloud
        .download_revision(Uuid::now_v7(), Uuid::now_v7(), 1, None)
        .await
        .unwrap();
    assert_eq!(got.bytes, file);
    assert_eq!(got.sha256, sha);
    let heads = heads.lock().unwrap().clone();
    assert_eq!(heads.len(), 2);
    assert!(!heads[0].contains("range:"), "{}", heads[0]);
    assert!(
        heads[1].contains(&format!("range: bytes={half}-")),
        "{}",
        heads[1]
    );
}
