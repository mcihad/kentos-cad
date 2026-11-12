//! Komut geçmişi's Mimari summaries (docs/adr/8004), read from the command fixtures' inputs: each Mimari command says
//! what it did in Turkish, without an id; the row's details (Ayrıntı) say the fields, without an id either.

use serde_json::{Value, json};

use super::*;
use crate::history::text::inputs_for;

/// A project in metres, three decimals; no levels, types or styles of its own.
fn names() -> Names<'static> {
    Names { levels: &[], opening_types: &[], wall_styles: &[], format: kentos_interaction::Format::default() }
}

/// The commands a Mimari tool runs, and every input their fixtures execute.
fn fixture_inputs() -> Vec<(String, Value)> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/commands/v1");
    let mut out = Vec::new();
    let mut files: Vec<_> = std::fs::read_dir(&dir).expect("fixtures").filter_map(|e| e.ok()).map(|e| e.path()).collect();
    files.sort();
    for path in files {
        let Ok(v) = serde_json::from_str::<Value>(&std::fs::read_to_string(&path).unwrap_or_default()) else { continue };
        let Some(command) = v["command"].as_str() else { continue };
        if summary(command, &json!({}), &names()).is_none() && !KNOWN.contains(&command) {
            continue;
        }
        for case in v["cases"].as_array().into_iter().flatten() {
            for step in case["steps"].as_array().into_iter().flatten() {
                if step["op"] == "execute"
                    && step.get("command").is_none_or(|c| c == command)
                    && let Some(input) = step.get("input")
                {
                    out.push((command.to_owned(), input.clone()));
                }
            }
        }
    }
    out
}

/// The commands the summary knows.
const KNOWN: [&str; 18] = COMMANDS;

/// An id as the journal keeps it, or a field's English name or word left in a Turkish line.
fn foreign(text: &str) -> Option<&'static str> {
    const WORDS: [&str; 19] = [
        "$uid", "walls", "frame", "adopt", "corners", "keepSources", "justify", "width", "left", "right", "clear",
        "inner", "outer", "shift", "distance", "length", "source", "extent", "operation",
    ];
    if text.split(|c: char| !c.is_ascii_hexdigit() && c != '-').any(|w| w.len() == 36 && w.matches('-').count() == 4) {
        return Some("uuid");
    }
    WORDS.into_iter().find(|w| {
        text.split(|c: char| !c.is_alphanumeric() && c != '$').any(|part| part == *w || (w.starts_with('$') && part.starts_with(w)))
    })
}

#[test]
fn every_mimari_command_of_the_fixtures_is_summed_up_in_turkish_without_an_id() {
    let inputs = fixture_inputs();
    let commands: std::collections::BTreeSet<&str> = inputs.iter().map(|(c, _)| c.as_str()).collect();
    for c in KNOWN {
        assert!(commands.contains(c), "{c} has a fixture");
    }
    for (command, input) in &inputs {
        let said = summary(command, input, &names()).unwrap_or_else(|| panic!("{command}: no summary for {input}"));
        assert!(foreign(&said).is_none(), "{command}: “{said}” says {:?}", foreign(&said));
        assert!(!said.is_empty());
    }
}

/// Ayrıntı's fields say no id and no English word either.
#[test]
fn the_details_say_the_fields_in_turkish_without_an_id() {
    for (command, input) in fixture_inputs() {
        let fields = inputs_for(Some(&command), &input, &|_| None);
        assert!(foreign(&fields).is_none(), "{command}: “{fields}” says {:?}", foreign(&fields));
    }
}

#[test]
fn the_summaries_read_as_a_designer_says_them() {
    let s = |c: &str, v: Value| summary(c, &v, &names()).expect("a summary");
    assert_eq!(
        s(
            "cad.walls.rectangle",
            json!({"layerId": "duvar", "frame": {"kind": "corners", "a": {"x": 4.2, "y": 0.0}, "b": {"x": 7.2, "y": 3.0}}, "adopt": true})
        ),
        "3.000 × 3.000 m · iç net · ortak duvar: var olan kullanılır"
    );
    assert_eq!(
        s(
            "cad.walls.rectangle",
            json!({"layerId": "d", "frame": {"kind": "corners", "a": {"x": 0, "y": 0}, "b": {"x": 11, "y": 8}}, "reference": "outer", "width": 0.25})
        ),
        "11.000 × 8.000 m · dış ölçü · kalınlık 0.250 m"
    );
    assert_eq!(
        s("cad.walls.create", json!({"layerId": "d", "walls": [{"pts": [{"x": 0, "y": 0}, {"x": 5, "y": 0}, {"x": 5, "y": 4}], "width": 0.2}]})),
        "9.000 m · kalınlık 0.200 m"
    );
    assert_eq!(s("cad.walls.edit", json!({"uids": ["$uidOf:6"], "operation": {"kind": "shift", "by": 0.3}})), "0.300 m kaydırıldı");
    assert_eq!(
        s("cad.walls.edit", json!({"uids": ["a"], "operation": {"kind": "length", "length": 2, "anchor": "end"}})),
        "boy 2.000 m · son uç sabit"
    );
    assert_eq!(
        s(
            "cad.walls.partition",
            json!({"source": {"wall": "$uid:source", "edge": 0}, "side": "left", "measure": "clear", "distance": 1.5,
                   "extent": {"kind": "source"}, "wall": {"layerId": "duvar", "walls": [{"pts": [], "closed": false, "width": 0.1}]}})
        ),
        "kaynaktan 1.500 m net · sol yan · kalınlık 0.100 m"
    );
    assert_eq!(
        s("cad.stairs.create", json!({"layerId": "m", "stairs": [{"pts": [], "width": 1.0, "height": 3.0, "tread": 0.28}]})),
        "genişlik 1.000 m · yükseklik 3.000 m"
    );
    assert_eq!(s("cad.spaces.number", json!({"uids": ["a", "b", "c", "d"], "first": "Z-009"})), "4 mahal · ilk numara Z-009");
    assert_eq!(s("cad.casework.run", json!({"layerId": "m", "from": {"x": 0, "y": 0}, "to": {"x": 3.2, "y": 0}, "sink": 2, "cooktop": 4})),
        "3.200 m · evye 2. modül · ocak 4. modül");
}

/// A door is said by its type's name, the drawing's or KentOS's own.
#[test]
fn an_opening_is_said_by_its_types_name() {
    let builtin = kentos_commands::builtin_opening_types().into_iter().next().expect("a type of KentOS's own");
    let said = summary(
        "cad.openings.create",
        &json!({"layerId": "kapi", "openings": [{"host": "$uidOf:1", "at": 3.4, "type": builtin.id, "distance": 0.5}]}),
        &names(),
    );
    assert_eq!(said.as_deref(), Some(format!("{} · uçtan 0.500 m", builtin.name).as_str()));
}

/// The Kapı tool sends no type for the form's first one of KentOS's own: the row says the form (docs/adr/8004).
#[test]
fn an_opening_without_a_type_is_said_by_its_form() {
    for (form, word) in [("door", "kapı"), ("window", "pencere"), ("niche", "niş")] {
        let said = summary("cad.openings.create", &json!({"layerId": "k", "openings": [{"host": "$uidOf:1", "form": form}]}), &names());
        assert_eq!(said.as_deref(), Some(word));
    }
}
