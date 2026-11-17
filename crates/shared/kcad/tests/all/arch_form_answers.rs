//! Architecture data 10 (docs/adr/8077): the author's answers to the model quality control form are written with
//! `modules.arch = 10` and read back as they were; a drawing without them stays as an older KentOS reads it, and a file
//! that holds them under an older architecture version is refused.

use kentos_contracts::FormAnswer;

fn sample() -> kentos_contracts::DocumentSnapshotV2 {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/arch/v1/section-building.kcad");
    kentos_kcad::decode(&std::fs::read(path).expect("the building")).expect("decodes")
}

#[test]
fn the_form_s_answers_round_trip_under_arch_data_10() {
    let plain = sample();
    assert!(!kentos_kcad::uses_form_answers(&plain));
    assert!(kentos_kcad::arch_version(&plain) < kentos_kcad::ARCH_WITH_FORM_ANSWERS);
    let again = kentos_kcad::decode(&kentos_kcad::encode(&plain).expect("encodes")).expect("decodes");
    assert_eq!(again, plain, "a drawing without answers is as it was");

    let mut doc = plain.clone();
    doc.settings.arch.get_or_insert_with(Default::default).form_answers = vec![
        FormAnswer { no: 1, yes: true, note: "Bakanlığın sınırı yayımlanmadı".into() },
        FormAnswer { no: 7, yes: false, note: String::new() },
    ];
    assert!(kentos_kcad::uses_form_answers(&doc));
    assert_eq!(kentos_kcad::arch_version(&doc), kentos_kcad::ARCH_WITH_FORM_ANSWERS);
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(kentos_kcad::decode(&bytes).expect("decodes"), doc, "the answers read back");
}
