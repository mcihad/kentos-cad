//! A revision download cut short goes on from where it stopped (docs/adr/0045):
//! a server that breaks off the first answer halfway must be asked for the
//! rest with `Range`, and the pieces must make the file its entity tag names.
//! A small stand-in server on this computer plays the cut. A server job's artifact goes the same way (docs/adr/0547).

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

/// A one-answer-per-connection server: for each request head it gets, the response its function gives (as bytes).
async fn serve(
    listener: TcpListener,
    answers: Vec<Box<dyn Fn(&str) -> Vec<u8> + Send>>,
    heads: Arc<Mutex<Vec<String>>>,
) {
    for answer in answers {
        let (mut s, _) = listener.accept().await.unwrap();
        let h = head(&mut s).await;
        let bytes = answer(&h);
        heads.lock().unwrap().push(h);
        s.write_all(&bytes).await.unwrap();
        s.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn a_cut_job_artifact_goes_on_with_a_range_and_the_job_routes_are_the_servers() {
    let file: Vec<u8> = (0..120_000u32).map(|i| (i % 249) as u8).collect();
    let sha = hex_sha256(&file);
    let half = file.len() / 3;
    let (tenant, project, job) = (Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let heads = Arc::new(Mutex::new(Vec::new()));
    let (a, b, sha_a, sha_b) = (file.clone(), file.clone(), sha.clone(), sha.clone());
    let json = |body: &str| {
        format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
            body.len()
        )
        .into_bytes()
    };
    let job_json = format!(
        r#"{{"id":"{job}","project":"{project}","kind":"project.export","input":{{}},"inputSha256":"{}","baseRevision":"3","state":"running","progress":0.5,"attempts":1,"maxAttempts":3,"timeoutSeconds":600,"cancelRequested":false,"createdBy":"u","createdAt":"2026-10-20T10:00:00Z"}}"#,
        "0".repeat(64)
    );
    let list_json = format!(r#"{{"jobs":[{job_json}]}}"#);
    let (jj, lj) = (job_json.clone(), list_json.clone());
    tokio::spawn(serve(
        listener,
        vec![
            Box::new(move |_| json(&lj)),
            Box::new({
                let json = json;
                let jj = jj.clone();
                move |_| json(&jj)
            }),
            Box::new(move |_| {
                let mut out = format!(
                    "HTTP/1.1 200 OK\r\ncontent-length: {}\r\netag: \"{sha_a}\"\r\nx-kentos-revision: 3\r\n\r\n",
                    a.len()
                )
                .into_bytes();
                out.extend_from_slice(&a[..half]);
                out
            }),
            Box::new(move |_| {
                let mut out = format!(
                    "HTTP/1.1 206 Partial Content\r\ncontent-length: {}\r\ncontent-range: bytes {half}-{}/{}\r\netag: \"{sha_b}\"\r\n\r\n",
                    b.len() - half,
                    b.len() - 1,
                    b.len()
                )
                .into_bytes();
                out.extend_from_slice(&b[half..]);
                out
            }),
        ],
        heads.clone(),
    ));
    let cloud = Cloud::new(&format!("http://{addr}")).unwrap();
    let list = cloud.server_jobs(tenant, project).await.unwrap();
    assert_eq!(list.jobs.len(), 1);
    assert_eq!(list.jobs[0].id, job.to_string());
    let one = cloud.server_job(tenant, project, job).await.unwrap();
    assert!((one.progress - 0.5).abs() < f32::EPSILON);
    let got = cloud
        .server_job_artifact(tenant, project, job, None)
        .await
        .unwrap();
    assert_eq!((got.bytes, got.sha256), (file, sha));
    let heads = heads.lock().unwrap().clone();
    let base = format!("/v1/tenants/{tenant}/projects/{project}/jobs");
    assert!(
        heads[0].starts_with(&format!("get {base} ")),
        "{}",
        heads[0]
    );
    assert!(
        heads[1].starts_with(&format!("get {base}/{job} ")),
        "{}",
        heads[1]
    );
    assert!(
        heads[2].starts_with(&format!("get {base}/{job}/artifact ")),
        "{}",
        heads[2]
    );
    assert!(
        heads[3].contains(&format!("range: bytes={half}-")),
        "{}",
        heads[3]
    );
}
