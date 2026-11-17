//! Uçtan uca, kartsız: sunucu zarfı imzalar → istemci doğrular, açar, yazılım kartıyla imzalar ve yerelde doğrular →
//! sunucu teslimi kendi isteğine ve belgenin bugünkü baytlarına karşı sınar (docs/adr/9002 §5, §6).
//!
//! Masaüstünün süreç içi yolu tam olarak budur; yerel servis aynı motoru HTTP arkasında yürütür.

#![cfg(feature = "mock-token")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test içinde başarısızlık zaten çökmeli"
)]

use std::collections::HashMap;

use kentos_imza_domain::{
    BatchRecord, BatchStatus, DigestAlgorithm, FileStatus, PackageStatus, PayloadFile,
    PayloadPackage, SignedRequestPayload,
};
use kentos_imza_security::issue::{ServerKey, issue};
use kentos_imza_security::{ReplayGuard, TrustConfig, digest, verify};
use kentos_imza_signature::delivery::verify_delivery;
use kentos_imza_signature::mock_token::{MOCK_PIN, MockToken};
use kentos_imza_signature::{Choice, RecordSink, sign_batch};

const BELGE_1: &[u8] = b"Vaziyet plani, 3. revizyon";
const BELGE_2: &[u8] = b"Statik hesap raporu";
const BILGI: &[u8] = b"Bilgi notu";

fn dosya(id: &str, ad: &str, icerik: &[u8], gerekli: bool) -> PayloadFile {
    PayloadFile {
        id: id.into(),
        name: ad.into(),
        role: if gerekli {
            "MAIN_DOCUMENT"
        } else {
            "ATTACHMENT"
        }
        .into(),
        mime_type: "application/pdf".into(),
        size: icerik.len() as u64,
        digest_algorithm: "SHA-384".into(),
        digest: digest::compute(DigestAlgorithm::Sha384, icerik),
        signing_required: gerekli,
    }
}

fn istek() -> SignedRequestPayload {
    SignedRequestPayload {
        transaction_id: "tx-masaustu-1".into(),
        batch_id: "batch-1".into(),
        user_id: "kullanici-7".into(),
        session_id: "oturum".into(),
        signature_method: "ELECTRONIC_SIGNATURE".into(),
        client_type: "DESKTOP".into(),
        packages: vec![PayloadPackage {
            id: "dosya-2026-0007".into(),
            title: "Yapı ruhsatı başvurusu".into(),
            reference_number: Some("2026/0007".into()),
            document_type: Some("basvuru".into()),
            metadata: serde_json::json!({ "side": "professional" }),
            files: vec![
                dosya("f1", "vaziyet.pdf", BELGE_1, true),
                dosya("f2", "statik.pdf", BELGE_2, true),
                dosya("f3", "bilgi.pdf", BILGI, false),
            ],
        }],
        created_at: "2026-11-18T10:00:00Z".into(),
        expires_at: "2099-01-01T00:00:00Z".into(),
        nonce: "0123456789abcdef0123456789abcdef".into(),
        issuer: "kentos-server".into(),
        audience: "kentos-imza".into(),
    }
}

/// Sunucunun deposundaki baytların bugünkü özeti.
fn bugunku(icerikler: &HashMap<&str, &[u8]>) -> impl Fn(&str) -> Option<String> {
    move |id| {
        icerikler
            .get(id)
            .map(|b| digest::compute(DigestAlgorithm::Sha384, b))
    }
}

/// İstemci: zarfı doğrular, replay kapısından geçirir, batch'i açar ve imzalar.
fn imzala(zarf: &str, guven: &TrustConfig, cert: &str, pin: &str) -> (BatchRecord, Vec<String>) {
    let payload = verify(zarf, guven, "2026-11-18T10:05:00Z").unwrap();
    let mut replay = ReplayGuard::new();
    replay
        .consume(
            &payload.transaction_id,
            &payload.nonce,
            "2026-11-18T10:05:00Z",
        )
        .unwrap();
    let mut kayit = BatchRecord::new(payload.to_batch().unwrap());
    kayit.open().unwrap();
    let batch = kayit.batch.clone();
    let mut olaylar = Vec::new();
    let mut topla = |ad: &str, _: &serde_json::Value| olaylar.push(ad.to_owned());
    let mut sink = RecordSink {
        record: &mut kayit,
        on_event: &mut topla,
    };
    let kart = MockToken::new();
    let secim = Choice {
        slot_id: 1,
        certificate_id: cert.into(),
    };
    if let Err(e) = sign_batch(&batch, &kart, &secim, pin, &mut sink) {
        kayit.last_error = Some(e);
    }
    (kayit, olaylar)
}

#[test]
fn zarf_imza_ve_sunucu_dogrulamasi_uctan_uca() {
    let sunucu = ServerKey::generate().unwrap();
    let zarf = issue(&istek(), &sunucu, None).unwrap();
    let guven = TrustConfig::from_base64(
        &sunucu.public_base64(),
        "kentos-server".into(),
        "kentos-imza".into(),
    )
    .unwrap();

    let (kayit, olaylar) = imzala(&zarf, &guven, "01", MOCK_PIN);
    assert_eq!(
        kayit.batch.status,
        BatchStatus::Delivering,
        "{:?}",
        kayit.last_error
    );
    assert_eq!(kayit.results.len(), 2);
    // Paket VERIFYING üzerinden SIGNED'a vardı; bilgi amaçlı dosya SKIPPED kaldı.
    assert_eq!(kayit.batch.packages[0].status, PackageStatus::Signed);
    assert_eq!(kayit.batch.packages[0].files[2].status, FileStatus::Skipped);
    assert_eq!(
        olaylar,
        [
            "signing_started",
            "package_started",
            "file_started",
            "file_signed",
            "file_started",
            "file_signed",
            "package_completed",
            "delivery_started"
        ]
    );

    // Teslim gövdesi telden geçer: JSON'a yazılıp sunucuda okunur.
    let govde = serde_json::to_string(&kayit.result()).unwrap();
    let teslim: kentos_imza_domain::BatchResult = serde_json::from_str(&govde).unwrap();
    assert_eq!(teslim.signed, 2);
    assert!(teslim.signer.is_some());

    let depo: HashMap<&str, &[u8]> = [("f1", BELGE_1), ("f2", BELGE_2), ("f3", BILGI)].into();
    let hukum = verify_delivery(&istek(), &teslim, bugunku(&depo));
    assert!(hukum.accepted, "{:?}", hukum.reason);
    assert_eq!(hukum.status(), "completed");
    assert_eq!((hukum.signed, hukum.required), (2, 2));
    assert!(hukum.files.iter().all(|f| f.verified));
    let imzalayan = hukum.signer.unwrap();
    assert!(imzalayan.subject.contains("gerçek değil"));
    assert!(
        !hukum.legal_check_done,
        "zincir ve iptal denetimi yapılmadı"
    );

    // Sonra istemci sunucunun cevabını teslim eder: ancak şimdi COMPLETED.
    let mut kayit = kayit;
    assert_eq!(kayit.acknowledge(true).unwrap(), BatchStatus::Completed);
}

#[test]
/// Belge imzadan sonra değişmişse, imza bozulmuşsa ya da istekte olmayan bir dosyaya imza geldiyse sunucu kabul etmez.
fn sunucu_degismis_belgeyi_ve_bozuk_imzayi_reddeder() {
    let sunucu = ServerKey::generate().unwrap();
    let zarf = issue(&istek(), &sunucu, None).unwrap();
    let guven = TrustConfig::from_base64(
        &sunucu.public_base64(),
        "kentos-server".into(),
        "kentos-imza".into(),
    )
    .unwrap();
    let (kayit, _) = imzala(&zarf, &guven, "01", MOCK_PIN);
    let teslim = kayit.result();

    // 1) f1 değişti.
    let degismis: HashMap<&str, &[u8]> = [("f1", &b"degisti"[..]), ("f2", BELGE_2)].into();
    let h = verify_delivery(&istek(), &teslim, bugunku(&degismis));
    assert_eq!(h.status(), "partially");
    let f1 = h.files.iter().find(|f| f.file_id == "f1").unwrap();
    assert!(!f1.verified);
    assert!(f1.error.as_deref().unwrap().contains("değişmiş"));

    // 2) İmzanın tek biti bozuldu.
    let mut bozuk = teslim.clone();
    let mut b = base64_coz(&bozuk.results[0].signature.signature_value);
    b[10] ^= 0x01;
    bozuk.results[0].signature.signature_value = base64_yaz(&b);
    let depo: HashMap<&str, &[u8]> = [("f1", BELGE_1), ("f2", BELGE_2)].into();
    let h = verify_delivery(&istek(), &bozuk, bugunku(&depo));
    assert!(!h.files[0].verified);
    assert_eq!(h.signed, 1);

    // 3) İstekte olmayan bir dosyaya imza.
    let mut yabanci = teslim.clone();
    yabanci.results[1].file_id = "f9".into();
    let h = verify_delivery(&istek(), &yabanci, bugunku(&depo));
    assert_eq!(h.signed, 1);
    assert!(h.reason.unwrap().contains("olmayan"));

    // 4) Başka bir işlemin sonucu.
    let mut baska = teslim;
    baska.transaction_id = "tx-baska".into();
    let h = verify_delivery(&istek(), &baska, bugunku(&depo));
    assert!(!h.accepted);
    assert_eq!(h.status(), "failed");
}

#[test]
/// Yanlış PIN imzalamayı BAŞLATMAZ: batch READY kalır, kullanıcı aynı pencerede yeniden dener.
fn yanlis_pin_batchi_baslatmaz() {
    let sunucu = ServerKey::generate().unwrap();
    let zarf = issue(&istek(), &sunucu, None).unwrap();
    let guven = TrustConfig::from_base64(
        &sunucu.public_base64(),
        "kentos-server".into(),
        "kentos-imza".into(),
    )
    .unwrap();
    let (kayit, olaylar) = imzala(&zarf, &guven, "01", "9999");
    assert_eq!(kayit.batch.status, BatchStatus::Ready);
    assert_eq!(kayit.last_error.unwrap().code, "KSI-1302");
    assert!(
        olaylar.is_empty(),
        "karta dokunulmadan hiçbir olay yayılmamalı"
    );
}

#[test]
/// Süresi geçmiş sertifika seçilemez; karta PIN de gitmez.
fn suresi_gecmis_sertifika_secilemez() {
    let kart = MockToken::new();
    let batch = istek().to_batch().unwrap();
    let mut kayit = BatchRecord::new(batch.clone());
    kayit.open().unwrap();
    let mut hic = |_: &str, _: &serde_json::Value| {};
    let mut sink = RecordSink {
        record: &mut kayit,
        on_event: &mut hic,
    };
    let e = sign_batch(
        &batch,
        &kart,
        &Choice {
            slot_id: 2,
            certificate_id: "03".into(),
        },
        MOCK_PIN,
        &mut sink,
    )
    .unwrap_err();
    assert_eq!(e.code, "KSI-1202");
    let e = sign_batch(
        &batch,
        &kart,
        &Choice {
            slot_id: 1,
            certificate_id: "ff".into(),
        },
        MOCK_PIN,
        &mut sink,
    )
    .unwrap_err();
    assert_eq!(e.code, "KSI-1201");
}

fn base64_coz(s: &str) -> Vec<u8> {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD.decode(s).unwrap()
}

fn base64_yaz(b: &[u8]) -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD.encode(b)
}
