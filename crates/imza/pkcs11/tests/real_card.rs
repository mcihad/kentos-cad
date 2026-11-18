//! GERÇEK KART testleri: takılı bir e-imza kartıyla çalışır (docs/adr/9002 §6).
//!
//! **Varsayılan olarak ATLANIR** (`#[ignore]`). Kartla koşmak için:
//!
//! ```bash
//! # PIN istemeyenler: cihaz, sertifika, mekanizma listesi.
//! KENTOS_IMZA_PKCS11=/usr/lib/libakisp11.so \
//!   scripts/dev/svc.sh heavy cargo test -p kentos-imza-pkcs11 --test real_card -- --ignored --test-threads=1 okunur
//! # TEK imza: tek C_Login, iki belge, yerel ve sunucu doğrulaması. PIN ortamdan okunur.
//! read -rs KENTOS_IMZA_TEST_PIN && export KENTOS_IMZA_TEST_PIN
//! KENTOS_IMZA_PKCS11=/usr/lib/libakisp11.so \
//!   scripts/dev/svc.sh heavy cargo test -p kentos-imza-pkcs11 --test real_card -- --ignored --exact \
//!   gercek_kartla_uctan_uca_imza
//! ```
//!
//! **PIN kaynak koda, loga, çıktıya YAZILMAZ** — ortamdan okunur ve yoksa imzalama testi atlanır. **PIN'i gereksiz yere
//! denemeyin**: yanlış deneme kartın sayacını düşürür ve sayaç bitince kart PUK ister. İmzalama testi PIN'i karta
//! **tek kez** verir (tek oturum); PIN'i ayrıca sınamaz.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "gerçek kart testinde başarısızlık zaten çökmeli — sessizce geçen bir kart testi hiçbir şey kanıtlamaz"
)]

use std::collections::HashMap;

use kentos_imza_domain::{
    BatchRecord, BatchStatus, DigestAlgorithm, PayloadFile, PayloadPackage, QualifiedStatus,
    SignedRequestPayload,
};
use kentos_imza_pkcs11::{Pkcs11Module, ProviderConfig};
use kentos_imza_security::issue::{ServerKey, issue};
use kentos_imza_security::{TrustConfig, digest, verify};
use kentos_imza_signature::delivery::verify_delivery;
use kentos_imza_signature::{Choice, RecordSink, Token as _, sign_batch};

/// Süreç ömrü boyunca **TEK** modül.
///
/// İki ölçülmüş kusuru kapatır: `C_Initialize` süreç başına BİR KEZ çağrılır (ikinci çağrı
/// `CKR_CRYPTOKI_ALREADY_INITIALIZED` döner); ve **sessizce geçen test, test DEĞİLDİR** — ortam değişkeni yoksa test
/// AÇIKÇA atlanır (mesajla), varsa ve yükleme başarısızsa panikler.
fn modul() -> Option<&'static Pkcs11Module> {
    use std::sync::OnceLock;
    static MODUL: OnceLock<Option<Pkcs11Module>> = OnceLock::new();
    MODUL
        .get_or_init(|| {
            let Ok(yol) = std::env::var("KENTOS_IMZA_PKCS11") else {
                return None;
            };
            let cfg = ProviderConfig::new("test", yol).expect("PKCS#11 yolu geçersiz");
            Some(Pkcs11Module::load(cfg).expect("PKCS#11 modülü yüklenemedi"))
        })
        .as_ref()
}

#[test]
#[ignore = "gerçek kart gerekir — KENTOS_IMZA_PKCS11 ile koşun"]
fn cihaz_ve_token_okunur() {
    let Some(m) = modul() else {
        eprintln!("ATLANDI: KENTOS_IMZA_PKCS11 ayarlı değil");
        return;
    };
    let cihazlar = m.devices().expect("cihaz listesi");
    assert!(!cihazlar.is_empty(), "takılı token bulunamadı");
    let c = &cihazlar[0];
    // Seri numarası ve etiket kişisel değil ama kurulumu tanıtır: yalnız uzunluklar basılır.
    println!(
        "cihaz sayısı={} etiket uzunluğu={} pinKilitliBayrak={}",
        cihazlar.len(),
        c.token_label.chars().count(),
        c.pin_locked_flag
    );
    assert!(!c.token_label.is_empty(), "cihazın bir etiketi olmalı");
    // PIN sınırları ZORUNLU DEĞİL: ölçülen kartta `utcTime` alanı bozuk olduğu için `C_GetTokenInfo` düşüyor ve
    //    `devices()` yuva bilgisine geri düşüyor; o yolda sınırlar `0` gelir.
    if !c.serial_number.is_empty() {
        assert!(c.pin_min >= 4, "PIN alt sınırı beklenenden küçük");
    }
}

#[test]
#[ignore = "gerçek kart gerekir"]
fn sertifika_okunur_ve_nitelikli_belirlenir() {
    let Some(m) = modul() else {
        eprintln!("ATLANDI: KENTOS_IMZA_PKCS11 ayarlı değil");
        return;
    };
    let slot = m
        .devices()
        .expect("cihaz")
        .first()
        .expect("token yok")
        .slot_id;
    // Sertifika okuma PIN GEREKTİRMEZ — açık nesnedir.
    let sertifikalar = m.certificates(slot).expect("sertifika listesi");
    assert!(!sertifikalar.is_empty(), "kartta sertifika yok");
    for s in &sertifikalar {
        // Konu kişisel veridir (ad, kimlik numarası): basılmaz; yalnız teknik alanlar.
        println!(
            "alg={} | anahtar={} {} | nitelikli={:?} | geçerli={} | kullanım={:?}",
            s.signature_algorithm,
            s.key_algorithm,
            s.key_parameters,
            s.qualified,
            s.currently_valid,
            s.key_usage
        );
        assert_eq!(s.fingerprint_sha256.len(), 64, "SHA-256 parmak izi 64 hex");
        assert!(matches!(
            s.qualified,
            QualifiedStatus::Yes | QualifiedStatus::No | QualifiedStatus::Unknown
        ));
    }
}

#[test]
#[ignore = "gerçek kart gerekir"]
fn token_mekanizmalari_okunur() {
    let Some(m) = modul() else {
        eprintln!("ATLANDI: KENTOS_IMZA_PKCS11 ayarlı değil");
        return;
    };
    let slot = m.devices().expect("cihaz").first().expect("token").slot_id;
    let mekanizmalar = m.mechanisms(slot).expect("mekanizma listesi");
    println!("mekanizmalar: {mekanizmalar:?}");
    assert!(!mekanizmalar.is_empty());
}

#[test]
#[ignore = "gerçek kart + PIN gerekir — KENTOS_IMZA_TEST_PIN"]
/// Sunucunun zarfı → doğrulama → gerçek kartla TEK oturumda iki belge → yerel doğrulama → sunucunun teslim doğrulaması.
/// Masaüstünün süreç içi yolu ve yerel servisin motoru tam olarak budur.
fn gercek_kartla_uctan_uca_imza() {
    let Some(m) = modul() else {
        eprintln!("ATLANDI: KENTOS_IMZA_PKCS11 ayarlı değil");
        return;
    };
    let Ok(pin) = std::env::var("KENTOS_IMZA_TEST_PIN") else {
        eprintln!("ATLANDI: KENTOS_IMZA_TEST_PIN ayarlı değil — karta hiç dokunulmadı");
        return;
    };

    // Kart ve sertifika: imzalamaya uygun, geçerli ilki. Seçim PIN'siz yapılır.
    let slot = m.devices().expect("cihaz").first().expect("token").slot_id;
    let sertifika = m
        .certificates(slot)
        .expect("sertifika")
        .into_iter()
        .find(|c| {
            c.currently_valid
                && c.key_usage
                    .iter()
                    .any(|k| k == "nonRepudiation" || k == "digitalSignature")
        })
        .expect("imzalamaya uygun geçerli sertifika yok");
    println!(
        "seçilen: {} {} ({})",
        sertifika.key_algorithm, sertifika.key_parameters, sertifika.signature_algorithm
    );

    // Sunucu tarafı: belgeler, özetleri ve imzalı zarf.
    let belgeler: [(&str, &[u8]); 2] = [
        ("f1", b"KentOS e-imza gercek kart testi, birinci belge"),
        ("f2", b"KentOS e-imza gercek kart testi, ikinci belge"),
    ];
    let istek = SignedRequestPayload {
        transaction_id: format!("gercek-kart-{}", std::process::id()),
        batch_id: "gercek-kart-batch".into(),
        user_id: "test".into(),
        session_id: "test".into(),
        signature_method: "ELECTRONIC_SIGNATURE".into(),
        client_type: "DESKTOP".into(),
        packages: vec![PayloadPackage {
            id: "p".into(),
            title: "Gerçek kart testi".into(),
            reference_number: None,
            document_type: None,
            metadata: serde_json::Value::Null,
            files: belgeler
                .iter()
                .map(|(id, b)| PayloadFile {
                    id: (*id).into(),
                    name: format!("{id}.txt"),
                    role: "MAIN_DOCUMENT".into(),
                    mime_type: "text/plain".into(),
                    size: b.len() as u64,
                    digest_algorithm: "SHA-384".into(),
                    digest: digest::compute(DigestAlgorithm::Sha384, b),
                    signing_required: true,
                })
                .collect(),
        }],
        created_at: "2026-01-01T00:00:00Z".into(),
        expires_at: "2099-01-01T00:00:00Z".into(),
        nonce: format!("{:032x}", std::process::id()),
        issuer: "kentos-server".into(),
        audience: "kentos-imza".into(),
    };
    let sunucu = ServerKey::generate().expect("anahtar");
    let zarf = issue(&istek, &sunucu, None).expect("zarf");

    // İstemci tarafı: doğrula, aç, imzala (TEK C_Login).
    let guven = TrustConfig::from_base64(
        &sunucu.public_base64(),
        "kentos-server".into(),
        "kentos-imza".into(),
    )
    .expect("güven");
    let simdi = kentos_imza_signature::verify::now_rfc3339();
    let govde = verify(&zarf, &guven, &simdi).expect("zarf doğrulanmalı");
    let mut kayit = BatchRecord::new(govde.to_batch().expect("batch"));
    kayit.open().expect("açılış");
    let batch = kayit.batch.clone();
    let mut olay = |ad: &str, _: &serde_json::Value| println!("olay: {ad}");
    let mut sink = RecordSink {
        record: &mut kayit,
        on_event: &mut olay,
    };
    sign_batch(
        &batch,
        m as &dyn kentos_imza_signature::Token,
        &Choice {
            slot_id: slot,
            certificate_id: sertifika.id.clone(),
        },
        &pin,
        &mut sink,
    )
    .expect("imzalama başlamalı (PIN doğru mu?)");
    drop(pin);
    assert_eq!(
        kayit.batch.status,
        BatchStatus::Delivering,
        "{:?}",
        kayit.last_error
    );
    assert_eq!(kayit.results.len(), 2, "iki belge de imzalanmalı");
    for r in kayit.results.values() {
        println!(
            "imza {} bayt, {}, yerel doğrulama {:?}",
            r.signature_value.len() * 3 / 4,
            r.signature_algorithm,
            r.verification.status
        );
    }

    // Sunucu tarafı: teslim gövdesi telden geçer ve sunucunun kurallarıyla sınanır.
    let teslim: kentos_imza_domain::BatchResult =
        serde_json::from_str(&serde_json::to_string(&kayit.result()).expect("json")).expect("json");
    let depo: HashMap<&str, &[u8]> = belgeler.into_iter().collect();
    let hukum = verify_delivery(&istek, &teslim, |id| {
        depo.get(id)
            .map(|b| digest::compute(DigestAlgorithm::Sha384, b))
    });
    assert!(hukum.accepted, "{:?}", hukum.reason);
    assert_eq!((hukum.signed, hukum.required), (2, 2));
    println!(
        "sunucu: {} / {} doğrulandı ({})",
        hukum.signed,
        hukum.required,
        hukum.status()
    );
    assert_eq!(
        kayit.acknowledge(true).expect("teslim"),
        BatchStatus::Completed
    );
}

#[test]
#[ignore = "yalnız kartın mekanizmalarını yazdırır"]
fn token_trait_ile_ayni_liste() {
    let Some(m) = modul() else {
        eprintln!("ATLANDI: KENTOS_IMZA_PKCS11 ayarlı değil");
        return;
    };
    let slot = m.devices().expect("cihaz").first().expect("token").slot_id;
    assert_eq!(
        m.mechanisms(slot).expect("liste"),
        kentos_imza_signature::Token::mechanisms(m, slot).expect("liste")
    );
    let _ = m.is_mock();
}
