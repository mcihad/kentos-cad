//! docs/adr/0776 §3: the command line offers the prompt's options as its suggestions and Enter takes the first, so
//! a letter the tool takes but the prompt leaves out is not an option there: Döşeme's “B” (Kaynağa bağlı,
//! docs/adr/0771) became a command's start and ended the tool in the 2+1 acceptance trace. The prompt lists it, the
//! letter turns it on and off, and the setting shows it.
use crate::common::Bench;

fn bound(b: &Bench) -> String {
    b.session
        .settings(&Default::default())
        .into_iter()
        .find(|r| r.key == "B")
        .expect("Kaynağa bağlı is a setting")
        .value
}

#[test]
fn slab_says_kaynaga_bagli_in_its_prompt_and_its_letter_turns_it() {
    let mut b = Bench::new("slab");
    let option = b.session.prompt().options.into_iter().find(|o| o.key == "B");
    assert_eq!(option.map(|o| o.label), Some("Kaynağa bağlı"), "the prompt offers B");
    let before = bound(&b);
    assert!(b.type_text("B"), "the letter is the slab's");
    assert_ne!(bound(&b), before, "B turned Kaynağa bağlı");
    assert!(b.type_text("b"), "lower case too");
    assert_eq!(bound(&b), before, "and back");
}

/// The other architectural tools' settings the prompt does not list (docs/adr/0776 §3, kept as a measure, not a
/// rule: their panels hold more than a prompt can say). `--ignored --nocapture` prints them.
#[test]
#[ignore = "a measure for docs/adr/0776, not a rule"]
fn architectural_settings_out_of_the_prompts() {
    let tools = [
        "wall", "wallRectangle", "wallPartition", "interiorDimensions", "wallDimensions", "door", "window", "opening",
        "space", "slab", "spaceNumber", "spaceDiscover", "grid", "column", "stair", "ramp", "gridSystem", "casework",
        "tag", "tagAll", "spaceSplit",
    ];
    let mut out = 0;
    for tool in tools {
        let b = Bench::new(tool);
        let options: Vec<&str> = b.session.prompt().options.iter().map(|o| o.key).collect();
        for row in b.session.settings(&Default::default()) {
            if !options.contains(&row.key) {
                out += 1;
                println!("{tool}: {} ({})", row.key, row.label);
            }
        }
    }
    println!("{out} settings of {} tools out of the prompts", tools.len());
}
