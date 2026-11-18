//! Notifications that leave the portal (docs/adr/9000 §17): nothing leaves until the administrator turns a channel on;
//! a person's choice and a verified address decide the rest; the sender writes a proper e-mail and a short SMS, signs
//! what it posts to the institution's gateway, tries again with pauses and gives up in the end.

use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use kentos_application::admin;
use kentos_contracts::{NotificationSettings, TokenIssued};
use kentos_postgres::testing::TestDb;

use super::tests::{app, send};
use crate::delivery::{Delivery, hex, hmac_sha256};

fn json(method: &str, uri: &str, token: &str, value: serde_json::Value) -> Request<Body> {
    let mut b = Request::builder().method(method).uri(uri).header(header::CONTENT_TYPE, "application/json");
    if !token.is_empty() {
        b = b.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    b.body(Body::from(value.to_string())).unwrap()
}

async fn token(app: &axum::Router, login: &str) -> String {
    let (status, _, body) = send(
        app,
        json("POST", "/v1/auth/token", "", serde_json::json!({ "login": login, "password": "dogru-parola-1" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{}", String::from_utf8_lossy(&body));
    serde_json::from_slice::<TokenIssued>(&body).unwrap().token
}

/// A notification for `login`, as kentos.notify_dossier writes one.
async fn notify(db: &TestDb, login: &str, title: &str) {
    sqlx::query(
        "insert into kentos.notification (user_id, kind, title, body)
         select u.id, 'dossier.changes_requested', $2, 'Otopark hesabı yeniden yapılmalı.'
           from kentos.app_user u join kentos.local_credential c on c.user_id = u.id where c.login = $1",
    )
    .bind(login)
    .bind(title)
    .execute(&db.owner)
    .await
    .unwrap();
}

async fn deliveries(db: &TestDb) -> Vec<(String, String, String, i32)> {
    sqlx::query_as("select channel, address, state, attempts from kentos.notification_delivery order by channel, created_at")
        .fetch_all(&db.owner)
        .await
        .unwrap()
}

/// Two accounts with e-mail, the platform's administrator; the router; their tokens.
async fn people(db: &TestDb) -> (axum::Router, String, String) {
    admin::create_local_user(&db.owner, "sistem", "Sistem Yöneticisi", Some("sistem@ornek.test"), "dogru-parola-1")
        .await
        .unwrap();
    admin::set_platform_admin(&db.owner, "sistem", true).await.unwrap();
    admin::create_local_user(&db.owner, "ayse", "Ayşe Yılmaz", Some("ayse@ornek.test"), "dogru-parola-1")
        .await
        .unwrap();
    let router = app(Some(db.app.clone()));
    let root = token(&router, "sistem").await;
    let ayse = token(&router, "ayse").await;
    (router, root, ayse)
}

#[tokio::test]
async fn nothing_leaves_until_a_channel_is_on_and_then_as_each_person_chose() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let (router, root, ayse) = people(&db).await;
    notify(&db, "ayse", "Kapalıyken gelen bildirim").await;
    assert!(deliveries(&db).await.is_empty(), "channels are off by default");

    // The administrator turns both on; the platform's other settings stay as they are.
    let (status, _, body) = send(
        &router,
        json("PUT", "/v1/platform/settings", &root, serde_json::json!({ "emailNotifications": true, "smsNotifications": true })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let s: kentos_contracts::PlatformSettings = serde_json::from_slice(&body).unwrap();
    assert!(s.email_notifications && s.sms_notifications && !s.signup_open);
    // E-mail by default; SMS needs a number and the person's own yes.
    notify(&db, "ayse", "Düzeltme istendi").await;
    assert_eq!(deliveries(&db).await, vec![("email".into(), "ayse@ornek.test".into(), "pending".into(), 0)]);
    let (status, _, _) = send(
        &router,
        json("PUT", "/v1/me/notification-settings", &ayse, serde_json::json!({ "email": false, "sms": true })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "SMS without a number");
    let (status, _, body) = send(
        &router,
        json("PUT", "/v1/me/notification-settings", &ayse, serde_json::json!({ "email": false, "sms": true, "phone": "0532 123 45 67" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let mine: NotificationSettings = serde_json::from_slice(&body).unwrap();
    assert_eq!((mine.email, mine.sms, mine.phone.as_deref(), mine.email_verified), (false, true, Some("05321234567"), true));
    notify(&db, "ayse", "Başvurunuz onaylandı").await;
    let all = deliveries(&db).await;
    assert_eq!(all.len(), 2);
    assert_eq!((all[1].0.as_str(), all[1].1.as_str()), ("sms", "05321234567"));

    // The sender writes them out: a proper e-mail and a short SMS, both pointing at the portal.
    let dir = std::env::temp_dir().join(format!("kentos-delivery-{}", uuid::Uuid::now_v7()));
    let sender = Delivery::to_folder(&dir, "https://bulut.ornek.test");
    assert_eq!(sender.once(&db.app, &reqwest::Client::new()).await.unwrap(), 2);
    let mut names: Vec<String> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    let eml = std::fs::read_to_string(dir.join(names.iter().find(|n| n.ends_with(".eml")).unwrap())).unwrap();
    assert!(eml.contains("<ayse@ornek.test>\r\n") && eml.contains("Subject: =?UTF-8?B?"), "{eml}");
    assert!(eml.contains("Content-Transfer-Encoding: base64") && eml.contains("Auto-Submitted: auto-generated"));
    let sms = std::fs::read_to_string(dir.join(names.iter().find(|n| n.ends_with(".sms.txt")).unwrap())).unwrap();
    assert!(sms.contains("05321234567") && sms.contains("KentOS: Başvurunuz onaylandı https://bulut.ornek.test/#/bildirimler"), "{sms}");
    assert!(deliveries(&db).await.iter().all(|d| d.2 == "sent"));
    let (_, _, body) = send(&router, Request::get("/v1/me/notification-settings").header(header::AUTHORIZATION, format!("Bearer {ayse}")).body(Body::empty()).unwrap()).await;
    let mine: NotificationSettings = serde_json::from_slice(&body).unwrap();
    let mut seen: Vec<(&str, &str)> = mine.recent.iter().map(|d| (d.channel.as_str(), d.state.as_str())).collect();
    seen.sort_unstable();
    assert_eq!(seen, vec![("email", "sent"), ("sms", "sent")], "what reached her, and how");
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn the_gateway_gets_a_signed_post_and_a_failing_one_is_tried_again_then_given_up() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let (router, root, _) = people(&db).await;
    send(&router, json("PUT", "/v1/platform/settings", &root, serde_json::json!({ "emailNotifications": true }))).await;
    // A gateway that refuses at first, then takes the posts, keeping what it got.
    let got: Arc<Mutex<Vec<(String, String)>>> = Arc::default();
    let answer = Arc::new(std::sync::atomic::AtomicU16::new(503));
    let gateway = {
        let (got, answer) = (got.clone(), answer.clone());
        axum::Router::new().route(
            "/bildirim",
            axum::routing::post(move |headers: axum::http::HeaderMap, body: String| {
                let (got, answer) = (got.clone(), answer.clone());
                async move {
                    let sig = headers.get("x-kentos-signature").and_then(|v| v.to_str().ok()).unwrap_or_default().to_owned();
                    got.lock().unwrap().push((sig, body));
                    StatusCode::from_u16(answer.load(std::sync::atomic::Ordering::SeqCst)).unwrap()
                }
            }),
        )
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/bildirim", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, gateway).await.unwrap() });
    let secret = "kurumun-paylasilan-gizli-anahtari";
    let sender = Delivery::to_webhook(&url, secret, "https://bulut.ornek.test");
    let client = reqwest::Client::new();

    notify(&db, "ayse", "Yeni aktarım").await;
    assert_eq!(sender.once(&db.app, &client).await.unwrap(), 1);
    assert_eq!(deliveries(&db).await[0].2, "pending", "refused: tried again later");
    let (signature, body) = got.lock().unwrap()[0].clone();
    assert_eq!(signature, format!("sha256={}", hex(&hmac_sha256(secret.as_bytes(), body.as_bytes()))));
    let posted: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!((posted["channel"].as_str(), posted["to"].as_str()), (Some("email"), Some("ayse@ornek.test")));
    // Not before its pause; then the gateway takes it.
    assert_eq!(sender.once(&db.app, &client).await.unwrap(), 0);
    sqlx::query("update kentos.notification_delivery set next_at = now()").execute(&db.owner).await.unwrap();
    answer.store(202, std::sync::atomic::Ordering::SeqCst);
    assert_eq!(sender.once(&db.app, &client).await.unwrap(), 1);
    assert_eq!(deliveries(&db).await[0].2, "sent");

    // A gateway that never takes it: given up after the last try, with the reason.
    answer.store(500, std::sync::atomic::Ordering::SeqCst);
    notify(&db, "ayse", "Süre doluyor").await;
    sqlx::query("update kentos.notification_delivery set attempts = 5 where state = 'pending'").execute(&db.owner).await.unwrap();
    sender.once(&db.app, &client).await.unwrap();
    let (state, error): (String, Option<String>) = sqlx::query_as(
        "select state, last_error from kentos.notification_delivery where state <> 'sent'",
    )
    .fetch_one(&db.owner)
    .await
    .unwrap();
    assert_eq!(state, "failed");
    assert!(error.unwrap().contains("500"));
}

#[cfg(unix)]
#[tokio::test]
async fn sendmail_gets_the_whole_message_on_its_input() {
    use std::os::unix::fs::PermissionsExt as _;
    let Some(db) = TestDb::create().await else {
        return;
    };
    let (router, root, _) = people(&db).await;
    send(&router, json("PUT", "/v1/platform/settings", &root, serde_json::json!({ "emailNotifications": true }))).await;
    notify(&db, "ayse", "Dosyanız teslim edildi").await;
    // The machine's mail program, as a script that keeps what it was given and how it was called.
    let dir = std::env::temp_dir().join(format!("kentos-sendmail-{}", uuid::Uuid::now_v7()));
    std::fs::create_dir_all(&dir).unwrap();
    let script = dir.join("sendmail");
    let kept = dir.join("ileti.eml");
    std::fs::write(&script, format!("#!/bin/sh\necho \"$@\" > '{0}.args'\ncat > '{0}'\n", kept.display())).unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    let sender = Delivery::to_sendmail(&script, "https://bulut.ornek.test");
    assert_eq!(sender.once(&db.app, &reqwest::Client::new()).await.unwrap(), 1);
    let message = std::fs::read_to_string(&kept).unwrap();
    assert!(
        message.starts_with("From: \"KentOS Bulut\" <bildirim@ornek.test>\r\nTo: =?UTF-8?B?QXnFn2UgWcSxbG1heg==?= <ayse@ornek.test>\r\n"),
        "{message}"
    );
    assert_eq!(std::fs::read_to_string(format!("{}.args", kept.display())).unwrap().trim(), "-t -i");
    assert_eq!(deliveries(&db).await[0].2, "sent");
    let _ = std::fs::remove_dir_all(dir);
}
