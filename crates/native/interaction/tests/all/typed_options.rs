//! A running command's options by their names typed (docs/adr/0281), and every tool's first prompt free of two
//! options on one letter or one name, over the selection traces' drawing (fixtures/interaction/v1/objects.kcad).

use crate::common;

use common::Bench;
use kentos_domain::Slot;
use kentos_interaction::{TOOLS, upper_tr};

const OBJECTS: &str = include_str!("../../../../../fixtures/interaction/v1/objects.kcad");

fn bench(slots: &[u32]) -> Bench {
    let mut b = Bench::on(OBJECTS);
    b.draft.snap = false;
    b.selection
        .set(slots.iter().map(|s| Slot(*s)).collect::<Vec<_>>());
    b
}

fn value(b: &Bench, label: &str) -> Option<String> {
    b.session
        .prompt()
        .options
        .iter()
        .find(|o| o.label == label)
        .and_then(|o| o.value.clone())
}

/// Döndür: “kopya” in full, Turkish marks and case aside, is Kopya (K); its start “kop” too.
#[test]
fn an_option_by_its_name_or_its_start() {
    let mut b = bench(&[1]);
    b.start("rotate");
    b.move_to(-24.0, -12.0);
    b.click(-24.0, -12.0);
    assert_eq!(value(&b, "Kopya").as_deref(), Some("kapalı"));
    assert!(b.type_text("KOPYA"));
    assert_eq!(value(&b, "Kopya").as_deref(), Some("açık"));
    assert!(b.type_text("kop"));
    assert_eq!(value(&b, "Kopya").as_deref(), Some("kapalı"));
    // A typed angle is still the tool's.
    assert!(b.type_text("90"));
    assert!(!b.session.is_running());
}

/// A hot grip's options by name: “döndür” turns the grip into Döndür.
#[test]
fn a_hot_grip_takes_its_options_by_name() {
    let mut b = bench(&[1]);
    b.move_to(-8.0, -12.0);
    b.click(-8.0, -12.0);
    assert!(b.type_text("döndür"));
    assert_eq!(b.last_text(), Some("Tutamaç: Döndür."));
    assert!(b.type_text("olc"));
    assert_eq!(b.last_text(), Some("Tutamaç: Ölçekle."));
}

/// A start several option names fit is said with them; nothing is chosen.
#[test]
fn a_start_several_names_fit_is_said() {
    let mut b = bench(&[1]);
    b.move_to(-8.0, -12.0);
    b.click(-8.0, -12.0);
    // Taşı and Taban noktası both start with “ta”.
    assert!(b.type_text("ta"));
    assert_eq!(
        b.last_text(),
        Some("“ta” birden çok seçeneğin başı: Taşı, Taban noktası. Seçeneğin adını tam yazın ya da harfine basın.")
    );
    assert!(b.session.prompt().text().starts_with("Tutamaç: "));
}

/// No tool's first prompt has two options on one letter or one name: a letter and a name mean one option.
#[test]
fn no_first_prompt_has_two_options_on_one_key_or_name() {
    let mut clashes = Vec::new();
    for &tool in TOOLS {
        for selected in [&[][..], &[1u32][..]] {
            let mut b = bench(selected);
            if !b.session.start(tool) {
                continue;
            }
            b.run(|s, cx| s.activate(cx));
            let prompt = b.session.prompt();
            let mut keys: Vec<String> = Vec::new();
            let mut names: Vec<String> = Vec::new();
            for o in &prompt.options {
                let key = upper_tr(o.key);
                if keys.contains(&key) {
                    clashes.push(format!("{tool}: harf {} iki kez", o.key));
                }
                keys.push(key);
                let name = upper_tr(o.label);
                if names.contains(&name) {
                    clashes.push(format!("{tool}: ad {} iki kez", o.label));
                }
                names.push(name);
            }
        }
    }
    clashes.sort();
    clashes.dedup();
    assert!(clashes.is_empty(), "{}", clashes.join("\n"));
}
