use super::*;

fn known(id: &str, name: &str) -> Known {
    Known {
        id: id.to_owned(),
        name: name.to_owned(),
        ..Known::default()
    }
}

fn codes(found: &[Resemblance]) -> Vec<(&'static str, &str)> {
    found
        .iter()
        .map(|r| (r.code(), r.other_id.as_str()))
        .collect()
}

/// What the eye cannot tell apart has one skeleton: Turkish letters and their bases, capitals, Cyrillic and Greek
/// letters that look Latin, digits and pairs that pass for letters, spaces, punctuation and invisible characters.
#[test]
fn a_skeleton_is_what_a_reader_sees() {
    let vitrin = skeleton("Vitrin");
    for same in [
        "VİTRİN",
        "VITRIN",
        "Vіtrіn",         // Cyrillic і
        "Vιtrin",         // Greek iota
        "V1trin",         // the digit one
        "Vltrin",         // a small L
        "Vit rin",        // a space
        "Vit\u{200b}rin", // an invisible character
        "Vitrin!",
    ] {
        assert_eq!(skeleton(same), vitrin, "{same}");
    }
    assert_eq!(skeleton("Mezarlık"), skeleton("MEZARLIK"));
    assert_eq!(skeleton("Çiçek Şehri"), skeleton("Cicek sehri"));
    assert_eq!(skeleton("tr.ornek.vitrin"), skeleton("tr.0rnek.vitrin"));
    assert_eq!(
        skeleton("tr.ornek.vitrin"),
        skeleton("tr.omek.vitrin"),
        "rn reads as m"
    );
    assert_eq!(skeleton("tr.ornek.vitrin"), skeleton("tr-ornek-vitrin"));
    assert_ne!(skeleton("Vitrin"), skeleton("Vitray"));
}

/// Another publisher's extension whose name and id read the same is an imitation, both said, the imitation first; a
/// gate's findings name the code and the other extension.
#[test]
fn a_name_and_an_id_that_read_the_same_are_imitations() {
    let installed = [
        known("tr.ornek.vitrin", "Vitrin"),
        known("tr.kentos.mevzuat", "Türkiye mevzuatı"),
    ];
    let copy = known("tr.0rnek.vitrin", "Vіtrin");
    let found = resemblances(&copy, &installed);
    assert_eq!(
        codes(&found),
        [
            ("imitates_name", "tr.ornek.vitrin"),
            ("imitates_id", "tr.ornek.vitrin")
        ]
    );
    assert_eq!(
        found[0].words(),
        "Adı, “Vitrin” (tr.ornek.vitrin) eklentisinin adıyla aynı okunuyor."
    );
    assert_eq!(
        found[1].words(),
        "Kimliği (tr.0rnek.vitrin), “Vitrin” (tr.ornek.vitrin) eklentisinin kimliğiyle aynı okunuyor."
    );
    let reasons = imitations(&copy, &installed);
    assert_eq!(reasons.len(), 2);
    assert_eq!(reasons[0].code, "imitates_name");
    assert_eq!(reasons[0].dependency.as_deref(), Some("tr.ornek.vitrin"));
}

/// A letter apart is a resemblance: said, never an imitation (two municipalities' Ula and Urla); short names and the
/// extension's own update are not compared, nor a publisher's own extensions.
#[test]
fn a_letter_apart_is_said_and_never_keeps_a_package_out() {
    let installed = [
        known("tr.bel.urla.mezarlik", "Urla Mezarlık"),
        known("tr.ornek.imar", "İmar"),
    ];
    let ula = known("tr.bel.ula.mezarlik", "Ula Mezarlık");
    let found = resemblances(&ula, &installed);
    assert_eq!(
        codes(&found),
        [
            ("resembles_name", "tr.bel.urla.mezarlik"),
            ("resembles_id", "tr.bel.urla.mezarlik")
        ]
    );
    assert!(imitations(&ula, &installed).is_empty());
    assert_eq!(
        found[0].words(),
        "Adı, “Urla Mezarlık” (tr.bel.urla.mezarlik) eklentisinin adına çok benziyor."
    );
    // Two swapped letters are one change.
    let swapped = known("tr.baska.segri", "Segri Pro");
    assert_eq!(
        codes(&resemblances(
            &swapped,
            &[known("tr.ornek.sergi", "Sergi Pro")]
        )),
        [("resembles_name", "tr.ornek.sergi")]
    );
    // A short name a letter apart is another word.
    assert!(resemblances(&known("tr.baska.imat", "İmat"), &installed).is_empty());
    // An update of the same extension, and the same publisher's other extension, are not compared.
    assert!(resemblances(&known("tr.ornek.imar", "İmar"), &installed).is_empty());
    let mut own = known("tr.ornek.imar-ek", "İmar");
    own.publisher = Some("ornek".into());
    let mut theirs = known("tr.ornek.imar", "İmar");
    theirs.publisher = Some("ornek".into());
    assert!(resemblances(&own, &[theirs.clone()]).is_empty());
    // Where nobody checked the publisher (a package on the computer), the same name is said whoever wrote it.
    own.publisher = None;
    assert_eq!(
        codes(&resemblances(&own, &[theirs])),
        [("imitates_name", "tr.ornek.imar")]
    );
}

/// An icon whose bytes are another's is an imitation; the blueprint's icon, every new extension's, is no one's.
#[test]
fn the_same_icon_is_an_imitation_unless_it_is_the_blueprints() {
    let theirs = Known {
        icon: Some(sha256_hex(b"<svg>vitrin</svg>")),
        ..known("tr.ornek.vitrin", "Vitrin")
    };
    let copy = Known {
        icon: theirs.icon.clone(),
        ..known("tr.baska.sergi", "Sergi")
    };
    let found = resemblances(&copy, std::slice::from_ref(&theirs));
    assert_eq!(codes(&found), [("imitates_icon", "tr.ornek.vitrin")]);
    assert_eq!(
        found[0].words(),
        "Simgesi, “Vitrin” (tr.ornek.vitrin) eklentisinin simgesiyle bayt bayt aynı."
    );
    let blueprint = Some(sha256_hex(crate::blueprint::ICON.as_bytes()));
    let a = Known {
        icon: blueprint.clone(),
        ..known("tr.baska.sergi", "Sergi")
    };
    let b = Known {
        icon: blueprint,
        ..known("tr.ornek.vitrin", "Vitrin")
    };
    assert!(resemblances(&a, &[b]).is_empty());
}

/// A person reads one sentence for each other extension: its parts joined, the imitation and the resemblance apart.
#[test]
fn the_findings_are_said_one_sentence_an_extension() {
    let theirs = Known {
        icon: Some(sha256_hex(b"<svg>vitrin</svg>")),
        ..known("tr.ornek.vitrin", "Eklenti Vitrini")
    };
    let copy = Known {
        icon: theirs.icon.clone(),
        ..known("tr.ornek.vltrin", "EKLENTİ VİTRİNİ")
    };
    let found = resemblances(&copy, &[theirs, known("tr.ornek.sergi", "Eklenti Vitrin")]);
    assert_eq!(
        sentences(&found),
        [
            "Adı, kimliği (tr.ornek.vltrin) ve simgesi “Eklenti Vitrini” (tr.ornek.vitrin) eklentisininkilerle aynı \
             okunuyor.",
            "Adı “Eklenti Vitrin” (tr.ornek.sergi) eklentisininkine çok benziyor."
        ]
    );
}
