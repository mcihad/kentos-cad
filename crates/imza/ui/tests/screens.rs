//! İmza penceresinin resimleri, pencere açmadan (KentOS UI'ın ekransız çizicisi; docs/adr/9002 §6).
//!
//! ```bash
//! RUST_MIN_STACK=67108864 scripts/dev/svc.sh heavy cargo test -p kentos-imza-ui --test screens -- --ignored --nocapture
//! ```
//!
//! Resimler `.run/gui-evidence/imza/` altına yazılır (Git'e girmez); `KENTOS_IMZA_SHOT` başka bir klasör verir. Veriler
//! deneme kartınındır: kişi adı, kimlik numarası ya da gerçek bir sertifika yoktur.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test içinde başarısızlık zaten çökmeli"
)]

use std::path::PathBuf;

use iced::Size;
use kentos_imza_domain::QualifiedStatus;
use kentos_imza_ui::{
    CertificateOption, DocState, Document, Link, Msg, Outcome, Request, SigningDialog,
};
use kentos_ui::snapshot::Snapshot;
use kentos_ui::theme::{Accent, Mode};

/// Ajan penceresinin ilk boyu (apps/imza-agent ile aynı).
const PENCERE: Size = Size::new(820.0, 720.0);

fn klasor() -> PathBuf {
    let k = std::env::var_os("KENTOS_IMZA_SHOT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../.run/gui-evidence/imza"),
        PathBuf::from,
    );
    std::fs::create_dir_all(&k).unwrap();
    k
}

fn belge(id: &str, ad: &str, boy: u64, gerekli: bool, ozet: &str) -> Document {
    Document {
        id: id.into(),
        name: ad.into(),
        size: boy,
        digest_algorithm: "SHA-384".into(),
        digest: ozet.repeat(96 / ozet.len()),
        signing_required: gerekli,
        state: if gerekli {
            DocState::Waiting
        } else {
            DocState::Informative
        },
    }
}

fn istek() -> Request {
    Request {
        id: "b-1".into(),
        title: "Konut yapı ruhsatı başvurusu, 1244 ada 7 parsel".into(),
        reference: Some("2026/0007".into()),
        documents: vec![
            belge("f1", "Mimari proje paftaları.pdf", 4_812_133, true, "a5ad"),
            belge("f2", "Statik hesap raporu.pdf", 932_480, true, "3c91"),
            belge("f3", "Zemin etüdü özeti.pdf", 288_004, true, "e07b"),
            belge("f4", "Bilgi notu.txt", 1_204, false, "77d0"),
        ],
    }
}

fn sertifikalar() -> Vec<CertificateOption> {
    vec![
        CertificateOption {
            slot_id: 1,
            device: "KentOS deneme kartı 1".into(),
            certificate_id: "01".into(),
            holder: "Deneme İmzacı (gerçek değil)".into(),
            issuer: "KentOS deneme makamı (gerçek değil)".into(),
            valid_until: "01.01.2036".into(),
            qualified: QualifiedStatus::No,
            currently_valid: true,
            key: "EC P-384".into(),
        },
        CertificateOption {
            slot_id: 2,
            device: "KentOS deneme kartı 2".into(),
            certificate_id: "02".into(),
            holder: "İkinci Deneme İmzacı (gerçek değil)".into(),
            issuer: "KentOS deneme makamı (gerçek değil)".into(),
            valid_until: "01.01.2036".into(),
            qualified: QualifiedStatus::No,
            currently_valid: true,
            key: "EC P-256".into(),
        },
    ]
}

fn hazir() -> SigningDialog {
    let mut d = SigningDialog::default();
    d.mock = true;
    d.link = Link::Connected;
    d.show_request(istek());
    d.set_certificates(sertifikalar(), None);
    d
}

/// Resmin adı ve penceresi.
fn durumlar() -> Vec<(&'static str, SigningDialog)> {
    let mut v = Vec::new();

    let mut bekliyor = SigningDialog::default();
    bekliyor.link = Link::Connected;
    v.push(("01-bekliyor", bekliyor));

    let mut pin = hazir();
    pin.update(Msg::Pin("123456".into()));
    v.push(("02-pin", pin));

    let mut yanlis = hazir();
    yanlis.update(Msg::Pin("0000".into()));
    yanlis.update(Msg::Sign);
    yanlis.pin_refused("PIN kodu yanlış. Kalan deneme hakkınıza dikkat edin.");
    v.push(("03-yanlis-pin", yanlis));

    let mut imzaliyor = hazir();
    imzaliyor.update(Msg::Pin("1234".into()));
    imzaliyor.update(Msg::Sign);
    imzaliyor.signing();
    let olaylar = [
        ("signing_started", serde_json::json!({})),
        (
            "file_started",
            serde_json::json!({ "fileId": "f1", "fileName": "Mimari proje paftaları.pdf" }),
        ),
        (
            "file_signed",
            serde_json::json!({ "fileId": "f1", "fileName": "Mimari proje paftaları.pdf" }),
        ),
        (
            "file_started",
            serde_json::json!({ "fileId": "f2", "fileName": "Statik hesap raporu.pdf" }),
        ),
    ];
    for (i, (ad, yuk)) in olaylar.iter().enumerate() {
        imzaliyor.event(ad, &format!("2026-11-18T10:15:0{i}Z"), yuk);
    }
    v.push(("04-imzaliyor", imzaliyor));

    let mut bitti = hazir();
    bitti.update(Msg::Pin("1234".into()));
    bitti.update(Msg::Sign);
    bitti.signing();
    let adlar = [
        ("f1", "Mimari proje paftaları.pdf"),
        ("f2", "Statik hesap raporu.pdf"),
        ("f3", "Zemin etüdü özeti.pdf"),
    ];
    for (i, (f, ad)) in adlar.iter().enumerate() {
        bitti.event(
            "file_signed",
            &format!("2026-11-18T10:15:1{i}Z"),
            &serde_json::json!({ "fileId": f, "fileName": ad }),
        );
    }
    bitti.event(
        "delivery_started",
        "2026-11-18T10:15:14Z",
        &serde_json::json!({}),
    );
    bitti.event(
        "delivery_completed",
        "2026-11-18T10:15:15Z",
        &serde_json::json!({}),
    );
    bitti.finish(Outcome::Completed { signed: 3 });
    v.push(("05-tamamlandi", bitti));

    let mut kartsiz = SigningDialog::default();
    kartsiz.link = Link::Connected;
    kartsiz.show_request(istek());
    kartsiz.card_failed("Elektronik imza cihazı bulunamadı. Kartı takıp yeniden tarayın.");
    v.push(("06-kart-yok", kartsiz));
    v
}

#[test]
#[ignore = "resim üretir: --ignored ile koşun"]
fn screens() {
    let k = klasor();
    for (tema, kip) in [("acik", Mode::Light), ("koyu", Mode::Dark)] {
        let theme = kentos_ui::theme::theme(kip, Accent::default());
        for (ad, d) in durumlar() {
            let mut s = Snapshot::new(PENCERE).unwrap();
            let yol = k.join(format!("{ad}-{tema}.png"));
            s.render(d.view_window(), &theme).save(&yol).unwrap();
            println!("{}", yol.display());
        }
    }
}

#[test]
/// Her durum çizilebiliyor (resim yazmadan): görünüm bir durumda düşerse bu test kırılır.
fn her_durum_cizilir() {
    let theme = kentos_ui::theme::theme(Mode::Light, Accent::default());
    for (ad, d) in durumlar() {
        let mut s = Snapshot::software(PENCERE).unwrap();
        let resim = s.render(d.view_window(), &theme);
        assert_eq!(resim.width as f32, PENCERE.width, "{ad}");
        assert!(
            resim.rgba.chunks(4).any(|p| p[0] != resim.rgba[0]),
            "{ad}: boş resim"
        );
    }
}
