//! Uçtan uca HTTP testi — gerçek soket, gerçek yönlendirici, gerçek WebSocket (docs/adr/9002 §4).
//!
//! Zarfı sunucunun yaptığı gibi imzalar (`kentos-imza-security`'nin `issue` özelliği: yalnız sunucu ve testler açar).
//! Kart gerektirmez: imzalama testleri yazılım kartıyla (`mock-token`) koşar.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test içinde başarısızlık zaten çökmeli"
)]

use std::sync::Arc;

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD as B64URL};
use ed25519_dalek::{Signer as _, SigningKey};
use kentos_imza_api::{AppState, OriginAllowlist, PinEntryMode};
use kentos_imza_domain::{BatchResult, DigestAlgorithm, SignedRequestPayload};
use kentos_imza_security::{TrustConfig, digest};
use kentos_imza_signature::delivery::verify_delivery;
use kentos_imza_signature::mock_token::{MOCK_PIN, MockToken};
use kentos_imza_signature::{MockElectronicSignatureProvider, SignatureProviderRegistry, Token};

/// Sabit tohumdan Ed25519 çifti — test deterministik olsun. Bu bir ÜRETİM anahtarı değil.
fn anahtar() -> SigningKey {
    SigningKey::from_bytes(&[42u8; 32])
}

/// JWS-compact zarf üretir: elle, sunucunun kitaplığını kullanmadan — biçimin kendisi de sınansın.
fn zarf(payload: &serde_json::Value) -> String {
    let sk = anahtar();
    let h = B64URL.encode(br#"{"alg":"EdDSA","typ":"KSI-SR"}"#);
    let p = B64URL.encode(serde_json::to_vec(payload).unwrap());
    let imzalanan = format!("{h}.{p}");
    let s = B64URL.encode(sk.sign(imzalanan.as_bytes()).to_bytes());
    format!("{imzalanan}.{s}")
}

fn payload(tx: &str, batch: &str, nonce: &str) -> serde_json::Value {
    serde_json::json!({
        "transactionId": tx,
        "batchId": batch,
        "userId": "u-1",
        "sessionId": "s-1",
        "signatureMethod": "ELECTRONIC_SIGNATURE",
        "clientType": "WEB",
        "packages": [{
            "id": "pkg-1",
            "title": "2026/1 sayılı olur",
            "referenceNumber": "E-12345",
            "documentType": "OLUR",
            "metadata": {},
            "files": [
                {
                    "id": "f-1",
                    "name": "olur.pdf",
                    "role": "MAIN_DOCUMENT",
                    "mimeType": "application/pdf",
                    "size": 1024,
                    "digestAlgorithm": "SHA-256",
                    "digest": "a".repeat(64),
                    "signingRequired": true
                },
                {
                    "id": "f-2",
                    "name": "ek.pdf",
                    "role": "ATTACHMENT",
                    "mimeType": "application/pdf",
                    "size": 64,
                    "digestAlgorithm": "SHA-256",
                    "digest": "b".repeat(64),
                    "signingRequired": false
                }
            ]
        }],
        "createdAt": "2026-01-01T00:00:00Z",
        "expiresAt": "2099-01-01T00:00:00Z",
        "nonce": nonce,
        "issuer": "kentos-server",
        "audience": "kentos-imza"
    })
}

/// Servisi rastgele bir geri döngü portunda ayağa kaldırır.
async fn servis(pin_entry: PinEntryMode) -> String {
    servis_kartla(pin_entry, None).await
}

/// Servisi verilen kartla ayağa kaldırır.
async fn servis_kartla(pin_entry: PinEntryMode, kart: Option<Arc<dyn Token>>) -> String {
    let trust = TrustConfig {
        verifying_key: anahtar().verifying_key(),
        expected_issuer: "kentos-server".into(),
        expected_audience: "kentos-imza".into(),
    };
    let state = AppState::new(
        SignatureProviderRegistry::new().with(Box::new(MockElectronicSignatureProvider)),
        pin_entry,
        Some(trust),
        kart,
    );
    // Port 0: işletim sistemi boş bir port seçsin; dinleyici sunucuya olduğu gibi verilir (kapatıp yeniden açmak,
    // arada başka bir sürecin portu kapması demekti).
    let dinleyici = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let adres = dinleyici.local_addr().unwrap();
    let origins = OriginAllowlist::parse("http://localhost:5173");
    tokio::spawn(async move {
        let _ = kentos_imza_api::serve_on(dinleyici, state, origins).await;
    });
    // Dinlemeye başlamasını bekle — bağlanabilene kadar dene.
    let taban = format!("http://127.0.0.1:{}", adres.port());
    for _ in 0..100 {
        if reqwest::get(format!("{taban}/api/v1/health")).await.is_ok() {
            return taban;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    panic!("servis ayağa kalkmadı");
}

#[tokio::test]
async fn batch_olustur_ac_ve_iptal_et() {
    let taban = servis(PinEntryMode::Native).await;
    let c = reqwest::Client::new();

    // ── 1. Oluştur ────────────────────────────────────────────────────────────
    let y = c
        .post(format!("{taban}/api/v1/signing/batches"))
        .json(&serde_json::json!({ "signedRequest": zarf(&payload("tx-1", "b-1", "n-1")) }))
        .send()
        .await
        .unwrap();
    assert_eq!(y.status(), 201, "batch oluşturulmalı");
    let j: serde_json::Value = y.json().await.unwrap();
    assert_eq!(j["status"], "CREATED");
    // İki dosyanın yalnız biri imzalanacak.
    assert_eq!(j["signingRequiredCount"], 1);

    // ── 2. Aynı işlem İKİNCİ kez → replay ──────────────────────────────
    let y = c
        .post(format!("{taban}/api/v1/signing/batches"))
        .json(&serde_json::json!({ "signedRequest": zarf(&payload("tx-1", "b-2", "n-2")) }))
        .send()
        .await
        .unwrap();
    assert_eq!(y.status(), 401, "aynı transactionId tekrar geçmemeli");
    assert_eq!(
        y.json::<serde_json::Value>().await.unwrap()["code"],
        "KSI-1503"
    );

    // ── 3. Aç ─────────────────────────────────────────────────────────────────
    let y = c
        .post(format!("{taban}/api/v1/signing/batches/b-1/open"))
        .send()
        .await
        .unwrap();
    assert_eq!(y.status(), 200);
    assert_eq!(
        y.json::<serde_json::Value>().await.unwrap()["status"],
        "READY"
    );

    // ── 4. İkinci `open` **idempotent** ─────────────────────────────────
    let y = c
        .post(format!("{taban}/api/v1/signing/batches/b-1/open"))
        .send()
        .await
        .unwrap();
    assert_eq!(y.status(), 200, "ikinci open hata vermemeli");

    // ── 5. `native` kipte AJAN YOKSA imza BAŞLAMAZ ───────────────────────────
    //
    // Başlatıp beklemek, kullanıcıyı hiç açılmayacak bir pencereyi beklemeye
    //    bırakırdı. Kontrol PKCS#11'den ÖNCE: kapalı bir pencere kullanıcının
    //    çözebileceği bir durum, eksik sürücü ise yöneticinin işi.
    let y = c
        .post(format!("{taban}/api/v1/signing/batches/b-1/sign"))
        .json(&serde_json::json!({ "slotId": 0, "certificateId": "aa", "pin": "123456" }))
        .send()
        .await
        .unwrap();
    assert_eq!(y.status(), 422);
    let e: serde_json::Value = y.json().await.unwrap();
    assert_eq!(e["code"], "KSI-1300");
    assert!(
        e["technicalMessage"]
            .as_str()
            .unwrap_or("")
            .contains("ajan"),
        "hata ajanı işaret etmeli: {e}"
    );
    // Hata metni PIN'i **taşımaz**.
    assert!(!e.to_string().contains("123456"), "PIN yanıta sızdı!");

    // ── 6. İptal ──────────────────────────────────────────────────────────────
    let y = c
        .post(format!("{taban}/api/v1/signing/batches/b-1/cancel"))
        .send()
        .await
        .unwrap();
    assert_eq!(
        y.json::<serde_json::Value>().await.unwrap()["status"],
        "CANCELLED"
    );

    // ── 7. Denetim zinciri sağlam ──────────────────────────────────────
    let a: serde_json::Value = reqwest::get(format!("{taban}/api/v1/audit"))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(a["valid"], true, "denetim zinciri kırık");
    let tipler: Vec<&str> = a["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|e| e["eventType"].as_str())
        .collect();
    assert!(tipler.contains(&"batch_created"), "olaylar: {tipler:?}");
    assert!(tipler.contains(&"batch_opened"), "olaylar: {tipler:?}");
    assert!(tipler.contains(&"batch_cancelled"), "olaylar: {tipler:?}");
}

#[tokio::test]
/// **PIN ucu TARAYICIDAN çağrılamaz** — `native` kipin güvenlik sınırı.
///
/// Tarayıcı `Origin`i **zorla** ekler ve saldırgan sayfa onu silemez; eksik `Origin`
/// isteğin tarayıcıdan gelmediğinin işaretidir. Bu kapı olmasaydı `native` kip,
/// `client` kipin daha dolambaçlı bir hâli olurdu.
async fn pin_ucu_tarayicidan_cagrilamaz() {
    let taban = servis(PinEntryMode::Native).await;
    let c = reqwest::Client::new();

    // Tarayıcıdan (Origin ile) → 422 `KSI-1300`.
    let y = c
        .post(format!("{taban}/api/v1/signing/batches/b-x/pin"))
        .header("Origin", "http://localhost:5173")
        .json(&serde_json::json!({ "pin": "123456" }))
        .send()
        .await
        .unwrap();
    assert_eq!(y.status(), 422);
    let e: serde_json::Value = y.json().await.unwrap();
    assert_eq!(e["code"], "KSI-1300");
    assert!(!e.to_string().contains("123456"), "PIN yanıta sızdı!");

    // Agent'tan (Origin'siz) → bekleyen yok, ama KAPIDAN GEÇER (`KSI-1500`).
    let y = c
        .post(format!("{taban}/api/v1/signing/batches/b-x/pin"))
        .json(&serde_json::json!({ "pin": "123456" }))
        .send()
        .await
        .unwrap();
    assert_eq!(y.status(), 400);
    assert_eq!(
        y.json::<serde_json::Value>().await.unwrap()["code"],
        "KSI-1500"
    );
}

#[tokio::test]
/// Yerli agent kalp atışı `interactiveAgent`i AÇAR ve uç tarayıcıya KAPALIDIR.
async fn agent_kalp_atisi_interactive_agent_i_acar() {
    let taban = servis(PinEntryMode::Native).await;
    let c = reqwest::Client::new();

    let h: serde_json::Value = c
        .get(format!("{taban}/api/v1/health"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(h["interactiveAgent"], false, "agent yokken FALSE olmalı");

    // Tarayıcıdan çağrılamaz.
    assert_eq!(
        c.post(format!("{taban}/api/v1/agent/heartbeat"))
            .header("Origin", "http://localhost:5173")
            .send()
            .await
            .unwrap()
            .status(),
        500
    );

    // Agent'tan (Origin'siz) geçer.
    assert_eq!(
        c.post(format!("{taban}/api/v1/agent/heartbeat"))
            .send()
            .await
            .unwrap()
            .status(),
        204
    );
    let h: serde_json::Value = c
        .get(format!("{taban}/api/v1/health"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        h["interactiveAgent"], true,
        "kalp atışından sonra TRUE olmalı"
    );
}

#[tokio::test]
/// **Kurcalanmış zarf reddedilir** — imza gövdeyi çiviler.
async fn kurcalanmis_zarf_reddedilir() {
    let taban = servis(PinEntryMode::Native).await;
    let z = zarf(&payload("tx-9", "b-9", "n-9"));
    let mut parcalar: Vec<&str> = z.split('.').collect();
    // Gövdeyi değiştir, imzayı olduğu gibi bırak — klasik saldırı.
    let sahte = B64URL.encode(serde_json::to_vec(&payload("tx-9", "b-KOTU", "n-9")).unwrap());
    parcalar[1] = &sahte;
    let bozuk = parcalar.join(".");

    let y = reqwest::Client::new()
        .post(format!("{taban}/api/v1/signing/batches"))
        .json(&serde_json::json!({ "signedRequest": bozuk }))
        .send()
        .await
        .unwrap();
    assert_eq!(y.status(), 401);
    assert_eq!(
        y.json::<serde_json::Value>().await.unwrap()["code"],
        "KSI-1502"
    );
}

#[tokio::test]
/// **Beyaz listede olmayan `Origin` REDDEDİLİR** — CORS başlığıyla gizlenmez.
async fn yabanci_origin_reddedilir() {
    let taban = servis(PinEntryMode::Native).await;
    let c = reqwest::Client::new();

    let y = c
        .get(format!("{taban}/api/v1/health"))
        .header("Origin", "https://saldirgan.example")
        .send()
        .await
        .unwrap();
    assert_eq!(y.status(), 403, "yabancı kaynak geçmemeli");

    // İzinli kaynak geçer.
    let y = c
        .get(format!("{taban}/api/v1/health"))
        .header("Origin", "http://localhost:5173")
        .send()
        .await
        .unwrap();
    assert_eq!(y.status(), 200);

    // `Origin`siz istek (curl, yerli agent) geçer — tarayıcı onu zorla ekler.
    assert_eq!(
        c.get(format!("{taban}/api/v1/health"))
            .send()
            .await
            .unwrap()
            .status(),
        200
    );
}

#[tokio::test]
/// **DNS rebinding**: `Host` geri döngü değilse `421`.
async fn yabanci_host_reddedilir() {
    let taban = servis(PinEntryMode::Native).await;
    let y = reqwest::Client::new()
        .get(format!("{taban}/api/v1/health"))
        .header("Host", "kotu.example.com")
        .send()
        .await
        .unwrap();
    assert_eq!(y.status(), 421);
}

#[tokio::test]
/// WebSocket olayları **gerçekten akar**.
async fn websocket_olaylari_akar() {
    use futures_util::StreamExt as _;

    let taban = servis(PinEntryMode::Native).await;
    let ws_url = taban.replacen("http://", "ws://", 1) + "/api/v1/events";
    let (soket, _) = tokio_tungstenite::connect_async(&ws_url).await.unwrap();
    let (_, mut oku) = soket.split();

    // İlk mesaj: `service_started` (bağlantı gerçekten açık).
    let ilk = oku.next().await.unwrap().unwrap();
    let j: serde_json::Value = serde_json::from_str(ilk.to_text().unwrap()).unwrap();
    assert_eq!(j["eventType"], "service_started");

    // Batch oluştur → `batch_created` soketten gelmeli.
    reqwest::Client::new()
        .post(format!("{taban}/api/v1/signing/batches"))
        .json(&serde_json::json!({ "signedRequest": zarf(&payload("tx-ws", "b-ws", "n-ws")) }))
        .send()
        .await
        .unwrap();

    let olay = tokio::time::timeout(std::time::Duration::from_secs(5), oku.next())
        .await
        .expect("olay 5 sn içinde gelmeli")
        .unwrap()
        .unwrap();
    let j: serde_json::Value = serde_json::from_str(olay.to_text().unwrap()).unwrap();
    assert_eq!(j["eventType"], "batch_created");
    assert_eq!(j["batchId"], "b-ws");
    assert_eq!(j["signatureMethod"], "ElectronicSignature");
    // Olay zarfı: `eventId` ve `timestamp` her olayda var.
    assert!(j["eventId"].as_str().is_some_and(|s| !s.is_empty()));
    assert!(j["timestamp"].as_str().is_some_and(|s| s.contains('T')));
}

/// İçeriği bilinen iki belgeyle gövde: biri imzalanacak, biri bilgi amaçlı.
fn gercek_govde(tx: &str, batch: &str, nonce: &str) -> serde_json::Value {
    let mut p = payload(tx, batch, nonce);
    p["packages"][0]["files"][0]["digest"] =
        serde_json::Value::String(digest::compute(DigestAlgorithm::Sha256, b"olur metni"));
    p["packages"][0]["files"][1]["digest"] =
        serde_json::Value::String(digest::compute(DigestAlgorithm::Sha256, b"ek"));
    p
}

async fn durum_bekle(
    c: &reqwest::Client,
    taban: &str,
    batch: &str,
    hedef: &str,
) -> serde_json::Value {
    for _ in 0..200 {
        let j: serde_json::Value = c
            .get(format!("{taban}/api/v1/signing/batches/{batch}"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        if j["status"] == hedef {
            return j;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    panic!("batch {batch} {hedef} durumuna gelmedi");
}

#[tokio::test]
/// `native` kipte bütün yol, deneme kartıyla: oluştur → aç → imzala → ajan PIN'i ve seçimi getirir → kart imzalar →
/// DELIVERING → sonuç sunucunun kurallarıyla doğrulanır → teslim cevabı → COMPLETED.
async fn native_akis_deneme_kartiyla_uctan_uca() {
    let taban = servis_kartla(PinEntryMode::Native, Some(Arc::new(MockToken::new()))).await;
    let c = reqwest::Client::new();
    let govde = gercek_govde("tx-n", "b-n", "n-n");

    // Ajan "yaşıyorum" der (Origin'siz).
    assert_eq!(
        c.post(format!("{taban}/api/v1/agent/heartbeat"))
            .send()
            .await
            .unwrap()
            .status(),
        204
    );
    let info: serde_json::Value = c
        .get(format!("{taban}/api/v1/info"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(info["mockToken"], true, "deneme kartı kendini söylemeli");
    assert_eq!(info["insecurePinEntry"], false);

    let y = c
        .post(format!("{taban}/api/v1/signing/batches"))
        .header("Origin", "http://localhost:5173")
        .json(&serde_json::json!({ "signedRequest": zarf(&govde) }))
        .send()
        .await
        .unwrap();
    assert_eq!(y.status(), 201);
    // İzinli kaynağa CORS cevabı.
    assert_eq!(
        y.headers()
            .get("access-control-allow-origin")
            .and_then(|v| v.to_str().ok()),
        Some("http://localhost:5173")
    );
    assert_eq!(
        c.post(format!("{taban}/api/v1/signing/batches/b-n/open"))
            .send()
            .await
            .unwrap()
            .status(),
        200
    );
    // Tarayıcı seçim göndermeden imzalatır: seçimi ajan penceresi yapacak.
    let y = c
        .post(format!("{taban}/api/v1/signing/batches/b-n/sign"))
        .header("Origin", "http://localhost:5173")
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(y.status(), 202);

    // Ajan: önce PIN'i imzalamadan sınar (yanlış PIN batch'e dokunmaz), sonra doğru PIN'i ve seçimi getirir.
    let y = c
        .post(format!("{taban}/api/v1/devices/pin-check"))
        .json(&serde_json::json!({ "slotId": 1, "pin": "0000" }))
        .send()
        .await
        .unwrap();
    assert_eq!(y.status(), 422);
    let e: serde_json::Value = y.json().await.unwrap();
    assert_eq!(e["code"], "KSI-1302");
    assert!(!e.to_string().contains("0000"), "PIN yanıta sızdı");
    let j: serde_json::Value = c
        .get(format!("{taban}/api/v1/signing/batches/b-n"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        j["status"], "READY",
        "yanlış PIN denemesi batch'i düşürmemeli"
    );

    let y = c
        .post(format!("{taban}/api/v1/signing/batches/b-n/pin"))
        .json(&serde_json::json!({ "pin": MOCK_PIN, "slotId": 1, "certificateId": "01" }))
        .send()
        .await
        .unwrap();
    assert_eq!(y.status(), 204);
    durum_bekle(&c, &taban, "b-n", "DELIVERING").await;

    // Sonuç: sunucuya gidecek gövde.
    let sonuc: BatchResult = c
        .get(format!("{taban}/api/v1/signing/batches/b-n/result"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!((sonuc.signed, sonuc.total), (1, 1));
    assert!(
        sonuc
            .signer
            .as_ref()
            .is_some_and(|s| s.key_parameters == "P-384")
    );
    let istek: SignedRequestPayload = serde_json::from_value(govde).unwrap();
    let hukum = verify_delivery(&istek, &sonuc, |id| match id {
        "f-1" => Some(digest::compute(DigestAlgorithm::Sha256, b"olur metni")),
        "f-2" => Some(digest::compute(DigestAlgorithm::Sha256, b"ek")),
        _ => None,
    });
    assert!(hukum.accepted, "{:?}", hukum.reason);
    assert_eq!(hukum.status(), "completed");

    let j: serde_json::Value = c
        .post(format!("{taban}/api/v1/signing/batches/b-n/acknowledge"))
        .json(&serde_json::json!({ "accepted": true, "reason": null }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(j["status"], "COMPLETED");
    assert_eq!(j["packages"][0]["status"], "SIGNED");

    let a: serde_json::Value = c
        .get(format!("{taban}/api/v1/audit"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(a["valid"], true);
    let tipler: Vec<&str> = a["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|e| e["eventType"].as_str())
        .collect();
    for beklenen in [
        "pin_required",
        "signing_started",
        "file_signed",
        "delivery_started",
        "delivery_completed",
        "batch_completed",
    ] {
        assert!(tipler.contains(&beklenen), "{beklenen} yok: {tipler:?}");
    }
}

#[tokio::test]
/// Ajan penceresinde vazgeçmek bir CEVAPTIR: batch hemen CANCELLED olur, 180 saniye beklenmez.
async fn ajanda_vazgecmek_batchi_hemen_iptal_eder() {
    let taban = servis_kartla(PinEntryMode::Native, Some(Arc::new(MockToken::new()))).await;
    let c = reqwest::Client::new();
    c.post(format!("{taban}/api/v1/agent/heartbeat"))
        .send()
        .await
        .unwrap();
    c.post(format!("{taban}/api/v1/signing/batches"))
        .json(&serde_json::json!({ "signedRequest": zarf(&gercek_govde("tx-v", "b-v", "n-v")) }))
        .send()
        .await
        .unwrap();
    c.post(format!("{taban}/api/v1/signing/batches/b-v/open"))
        .send()
        .await
        .unwrap();
    c.post(format!("{taban}/api/v1/signing/batches/b-v/sign"))
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(
        c.post(format!("{taban}/api/v1/signing/batches/b-v/pin"))
            .json(&serde_json::json!({ "cancelled": true }))
            .send()
            .await
            .unwrap()
            .status(),
        204
    );
    let j = durum_bekle(&c, &taban, "b-v", "CANCELLED").await;
    assert_eq!(j["lastError"]["code"], "KSI-1301");
}

#[tokio::test]
/// `client` kipinde (geliştirme sapması) PIN gövdede gelir ve aynı motor imzalar.
async fn client_kipinde_pin_govdede() {
    let taban = servis_kartla(PinEntryMode::Client, Some(Arc::new(MockToken::new()))).await;
    let c = reqwest::Client::new();
    c.post(format!("{taban}/api/v1/signing/batches"))
        .json(&serde_json::json!({ "signedRequest": zarf(&gercek_govde("tx-c", "b-c", "n-c")) }))
        .send()
        .await
        .unwrap();
    c.post(format!("{taban}/api/v1/signing/batches/b-c/open"))
        .send()
        .await
        .unwrap();
    let y = c
        .post(format!("{taban}/api/v1/signing/batches/b-c/sign"))
        .json(&serde_json::json!({ "slotId": 2, "certificateId": "02", "pin": MOCK_PIN }))
        .send()
        .await
        .unwrap();
    assert_eq!(y.status(), 202);
    durum_bekle(&c, &taban, "b-c", "DELIVERING").await;
    let s: BatchResult = c
        .get(format!("{taban}/api/v1/signing/batches/b-c/result"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(s.signer.unwrap().key_parameters, "P-256");
}
