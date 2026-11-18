//! İmza penceresinin kararları (docs/adr/9002 §6): İmzala'nın şartları, PIN'in taşınmaması, yanlış PIN, Vazgeç ve
//! olayların belgeleri ilerletmesi.

#![allow(clippy::unwrap_used, reason = "test içinde başarısızlık zaten çökmeli")]

use kentos_imza_domain::QualifiedStatus;

use crate::dialog::{Action, Msg, SigningDialog};
use crate::model::{CertificateOption, DocState, Document, Outcome, Phase, Request};

fn istek() -> Request {
    Request {
        id: "b".into(),
        title: "Yapı ruhsatı başvurusu".into(),
        reference: Some("2026/0007".into()),
        documents: vec![
            Document {
                id: "f1".into(),
                name: "vaziyet.pdf".into(),
                size: 482_133,
                digest_algorithm: "SHA-384".into(),
                digest: "ab".repeat(48),
                signing_required: true,
                state: DocState::Waiting,
            },
            Document {
                id: "f2".into(),
                name: "bilgi.pdf".into(),
                size: 1200,
                digest_algorithm: "SHA-384".into(),
                digest: "cd".repeat(48),
                signing_required: false,
                state: DocState::Informative,
            },
        ],
    }
}

fn sertifika(slot: u64, id: &str, gecerli: bool) -> CertificateOption {
    CertificateOption {
        slot_id: slot,
        device: format!("Kart {slot}"),
        certificate_id: id.into(),
        holder: "Deneme İmzacı".into(),
        issuer: "Deneme makamı".into(),
        valid_until: "2036".into(),
        qualified: QualifiedStatus::No,
        currently_valid: gecerli,
        key: "EC P-384".into(),
    }
}

#[test]
/// İmzala ancak geçerli sertifika ve en az dört haneli PIN ile açılır; PIN eylemle çıkar ve pencerede kalmaz.
fn imzala_kurallari_ve_pin_tasinmaz() {
    let mut d = SigningDialog::default();
    d.show_request(istek());
    d.set_certificates(
        vec![sertifika(1, "03", false), sertifika(2, "01", true)],
        None,
    );
    assert_eq!(d.chosen, Some(1), "süresi geçmiş olan seçilmez");
    assert!(d.update(Msg::Choose(0)).is_none());
    assert_eq!(d.chosen, Some(1), "süresi geçmiş olana geçilmez");
    d.update(Msg::Pin("12".into()));
    assert!(!d.can_sign());
    d.update(Msg::Pin("1234".into()));
    assert!(d.can_sign());
    let a = d.update(Msg::Sign).unwrap();
    assert!(
        matches!(&a, Action::Sign { slot_id: 2, certificate_id, pin } if certificate_id == "01" && pin == "1234")
    );
    assert!(!format!("{a:?}").contains("1234"), "PIN Debug'a girdi");
    assert!(!format!("{:?}", Msg::Pin("1234".into())).contains("1234"));
    assert!(!format!("{d:?}").contains("1234"));
    assert_eq!(d.pin(), "", "PIN pencerede kalmadı");
    assert_eq!(d.phase, Phase::Checking);
    // Sınanırken ikinci basış yok sayılır.
    assert!(d.update(Msg::Sign).is_none());
}

#[test]
/// Yanlış PIN aynı pencerede yeniden sorulur.
fn yanlis_pin_ayni_pencerede() {
    let mut d = SigningDialog::default();
    d.show_request(istek());
    d.set_certificates(vec![sertifika(1, "01", true)], None);
    d.update(Msg::Pin("9999".into()));
    d.update(Msg::Sign);
    d.pin_refused("PIN kodu yanlış.");
    assert_eq!(d.phase, Phase::Ready);
    assert!(d.notice.as_ref().is_some_and(|n| n.error));
    d.update(Msg::Pin("1".into()));
    assert!(d.notice.is_none(), "yazmaya başlayınca uyarı kalkar");
}

#[test]
/// Vazgeç bir CEVAPTIR; kart imzalarken vazgeçilmez ve pencere kapanmaz.
fn vazgec_ve_kapat() {
    let mut d = SigningDialog::default();
    d.show_request(istek());
    assert!(matches!(d.update(Msg::Cancel), Some(Action::Cancel)));
    assert_eq!(d.phase, Phase::Done(Outcome::Cancelled));

    let mut d = SigningDialog::default();
    d.show_request(istek());
    d.signing();
    assert!(d.update(Msg::Cancel).is_none());
    assert!(d.update(Msg::Close).is_none());
}

#[test]
/// Olaylar belgelerin hâlini ve aşamayı ilerletir.
fn olaylar_belgeleri_ilerletir() {
    let mut d = SigningDialog::default();
    d.show_request(istek());
    d.event(
        "signing_started",
        "2026-11-18T10:00:00Z",
        &serde_json::json!({}),
    );
    assert_eq!(d.phase, Phase::Signing);
    d.event(
        "file_signed",
        "2026-11-18T10:00:01Z",
        &serde_json::json!({ "fileId": "f1", "fileName": "vaziyet.pdf" }),
    );
    assert_eq!(d.signed_count(), 1);
    d.event(
        "delivery_started",
        "2026-11-18T10:00:02Z",
        &serde_json::json!({}),
    );
    assert_eq!(d.phase, Phase::Delivering);
    assert_eq!(d.lines.len(), 3);
    assert_eq!(d.lines[0], "+0.0 sn  İmzalama başladı");
    assert_eq!(d.lines[1], "+1.0 sn  İmzalandı: vaziyet.pdf");
}
