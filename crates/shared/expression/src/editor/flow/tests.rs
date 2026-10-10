//! The flow's nodes, places and changes (docs/adr/0101).

use super::*;
use crate::{FieldDef, FieldSource, FieldType};

fn schema() -> Schema {
    Schema {
        variables: Vec::new(),
        fields: vec![
            FieldDef {
                name: "Ada".into(),
                ty: FieldType::Text,
                source: FieldSource::Attribute,
                description: String::new(),
            },
            FieldDef {
                name: "Kat".into(),
                ty: FieldType::Number,
                source: FieldSource::User,
                description: String::new(),
            },
        ],
        world: false,
    }
}

fn one(text: &str) -> Vec<Tree> {
    vec![Tree::new(text, None)]
}

fn texts(trees: &[Tree]) -> Vec<&str> {
    trees.iter().map(|t| t.text.as_str()).collect()
}

fn node<'f>(f: &'f Flow, id: &str) -> &'f FlowNode {
    f.nodes
        .iter()
        .find(|n| n.id == id)
        .unwrap_or_else(|| panic!("no node {id}"))
}

#[test]
fn a_call_is_nodes_right_to_left_with_its_inputs() {
    let f = flow(&one("yuvarla($alan * 2, 1)"), &schema());
    let ids: Vec<&str> = f.nodes.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(ids, ["r", "0", "0.0", "0.0.0", "0.0.1", "0.1"]);
    let r = node(&f, "r");
    assert_eq!(
        (r.x, r.y, r.ports[0].from.as_deref()),
        (0.0, 0.0, Some("0"))
    );
    let call = node(&f, "0");
    assert_eq!((call.title.as_str(), call.x), ("yuvarla", -COLUMN));
    assert_eq!(call.ports.len(), 2);
    assert_eq!(
        (
            call.ports[0].name.as_str(),
            call.ports[0].from.as_deref(),
            call.ports[0].optional
        ),
        ("sayı", Some("0.0"), false)
    );
    assert_eq!(
        (
            call.ports[1].name.as_str(),
            call.ports[1].from.as_deref(),
            call.ports[1].optional
        ),
        ("basamak", Some("0.1"), true)
    );
    assert_eq!(call.ty, Type::Number);
    assert_eq!(node(&f, "0.0").x, -2.0 * COLUMN);
    assert_eq!(node(&f, "0.0.0").x, -3.0 * COLUMN);
    assert_eq!(node(&f, "0.0").text, "$alan * 2");
    // The root's output is level with the result's input.
    assert_eq!(call.y + HEAD / 2.0, r.y + r.ports[0].y);
    // Children stack in port order, the node centred on them.
    let (a, b) = (node(&f, "0.0"), node(&f, "0.1"));
    assert!(a.y + a.h <= b.y);
    assert!(f.error.is_none() && call.error.is_none());
}

#[test]
fn empty_inputs_show_the_editors_error_on_their_node() {
    let f = flow(&one("yuvarla(?) + Pafta"), &schema());
    let call = node(&f, "0.0");
    assert_eq!(call.ports[0].from, None);
    assert_eq!(
        call.error.as_deref(),
        Some(crate::editor::check::EMPTY_INPUT)
    );
    assert!(!call.whole && !node(&f, "0").whole);
    // A field the objects do not have: the field's warning.
    let pafta = node(&f, "0.1");
    assert_eq!(pafta.warnings.len(), 1, "{:?}", pafta.warnings);
    // A text that does not read: the error, and only the result.
    let broken = flow(&one("yuvarla(1"), &schema());
    assert!(broken.error.is_some());
    assert_eq!(broken.nodes.len(), 1);
}

#[test]
fn ports_say_how_another_type_is_read() {
    let f = flow(&one("yuvarla(Ada) || Kat"), &schema());
    let call = node(&f, "0.0");
    assert!(
        call.ports[0]
            .note
            .as_deref()
            .is_some_and(|n| n.contains("sayıya"))
    );
    let join = node(&f, "0");
    assert_eq!(join.ty, Type::Text);
}

#[test]
fn nodes_are_built_and_connected_from_the_palette() {
    let s = schema();
    let step = |trees: &[Tree], e: Edit| edit(trees, &e, &s).expect("the change applies");
    let (t, focus) = step(
        &one(""),
        Edit::Add {
            key: "func:yuvarla".into(),
            at: (-500.0, 200.0),
        },
    );
    assert_eq!(texts(&t), ["", "yuvarla(?)"]);
    assert_eq!(
        (t[1].at, focus.as_deref()),
        (Some((-500.0, 200.0)), Some("1"))
    );
    let (t, focus) = step(
        &t,
        Edit::Connect {
            from: "1".into(),
            to: "r".into(),
            port: 0,
        },
    );
    assert_eq!(
        (texts(&t), focus.as_deref()),
        (vec!["yuvarla(?)"], Some("0"))
    );
    let (t, _) = step(
        &t,
        Edit::Add {
            key: "var:alan".into(),
            at: (-800.0, 0.0),
        },
    );
    let (t, focus) = step(
        &t,
        Edit::Connect {
            from: "1".into(),
            to: "0".into(),
            port: 0,
        },
    );
    assert_eq!(
        (texts(&t), focus.as_deref()),
        (vec!["yuvarla($alan)"], Some("0.0"))
    );
    let (t, _) = step(
        &t,
        Edit::Add {
            key: "lit:number".into(),
            at: (-800.0, 100.0),
        },
    );
    let (t, _) = step(
        &t,
        Edit::SetNumber {
            node: "1".into(),
            value: 2.0,
        },
    );
    assert_eq!(texts(&t), ["yuvarla($alan)", "2"]);
    // The optional input is written when something is connected to it.
    let (t, _) = step(
        &t,
        Edit::Connect {
            from: "1".into(),
            to: "0".into(),
            port: 1,
        },
    );
    assert_eq!(texts(&t), ["yuvarla($alan, 2)"]);
    // Taken off, the optional last input goes; the required one stays, empty.
    let (u, focus) = step(
        &t,
        Edit::Disconnect {
            to: "0".into(),
            port: 1,
        },
    );
    assert_eq!(
        (texts(&u), focus.as_deref()),
        (vec!["yuvarla($alan)", "2"], Some("0"))
    );
    let (u, _) = step(
        &t,
        Edit::Disconnect {
            to: "0".into(),
            port: 0,
        },
    );
    assert_eq!(texts(&u), ["yuvarla(?, 2)", "$alan"]);
    // What went apart stands where it stood.
    let before = flow(&t, &s);
    assert_eq!(
        u[1].at,
        Some((node(&before, "0.0").x, node(&before, "0.0").y))
    );
}

#[test]
fn connecting_where_something_is_puts_that_apart() {
    let s = schema();
    let (t, _) = edit(
        &one("Kat + 1"),
        &Edit::Add {
            key: "field:Ada".into(),
            at: (0.0, 300.0),
        },
        &s,
    )
    .expect("adds");
    let (t, _) = edit(
        &t,
        &Edit::Connect {
            from: "1".into(),
            to: "0".into(),
            port: 1,
        },
        &s,
    )
    .expect("connects");
    assert_eq!(texts(&t), ["Kat + Ada", "1"]);
    // A node inside the result's tree moves with its subtree.
    let (t, _) = edit(
        &one("eğer(Kat > 1, Ada, 'x')"),
        &Edit::Connect {
            from: "0.0".into(),
            to: "r".into(),
            port: 0,
        },
        &s,
    )
    .expect("connects");
    assert_eq!(texts(&t), ["Kat > 1", "eğer(?, Ada, 'x')"]);
    // Nothing goes into its own input.
    let e = edit(
        &one("Kat + 1"),
        &Edit::Connect {
            from: "0".into(),
            to: "0".into(),
            port: 0,
        },
        &s,
    );
    assert!(e.is_err());
}

#[test]
fn removing_a_node_keeps_its_inputs_apart() {
    let s = schema();
    let (t, focus) = edit(
        &one("yuvarla(Kat * 2, 1)"),
        &Edit::Remove { node: "0.0".into() },
        &s,
    )
    .expect("removes");
    assert_eq!(texts(&t), ["yuvarla(?, 1)", "Kat", "2"]);
    assert_eq!(focus, None);
    let (t, _) = edit(&t, &Edit::Remove { node: "1".into() }, &s).expect("removes");
    assert_eq!(texts(&t), ["yuvarla(?, 1)", "2"]);
}

#[test]
fn values_operators_and_words_change_in_place() {
    let s = schema();
    let change = |src: &str, e: Edit| {
        edit(&one(src), &e, &s)
            .map(|(t, _)| t[0].text.clone())
            .unwrap_or_else(|e| e)
    };
    assert_eq!(
        change(
            "Kat + 1",
            Edit::SetOperator {
                node: "0".into(),
                symbol: "*".into()
            }
        ),
        "Kat * 1"
    );
    assert_eq!(
        change(
            "Kat + 1",
            Edit::SetText {
                node: "0.1".into(),
                value: "it's".into()
            }
        ),
        "Kat + 'it''s'"
    );
    assert_eq!(
        change(
            "Kat + 1",
            Edit::SetField {
                node: "0.1".into(),
                name: "Tapu alanı".into()
            }
        ),
        "Kat + [Tapu alanı]"
    );
    assert_eq!(
        change(
            "Kat + 1",
            Edit::SetVariable {
                node: "0.1".into(),
                name: "alan".into()
            }
        ),
        "Kat + $alan"
    );
    assert_eq!(
        change(
            "Kat içinde (1)",
            Edit::SetNegated {
                node: "0".into(),
                value: true
            }
        ),
        "Kat değil içinde (1)"
    );
    assert_eq!(
        change(
            "Ada gibi 'a%'",
            Edit::SetFold {
                node: "0".into(),
                value: true
            }
        ),
        "Ada benzer 'a%'"
    );
    assert_eq!(
        change(
            "tavan(Kat)",
            Edit::SetFunction {
                node: "0".into(),
                name: "yuvarla".into()
            }
        ),
        "yuvarla(Kat)"
    );
    // A node with inputs is not a value.
    assert!(
        change(
            "Kat + 1",
            Edit::SetNumber {
                node: "0".into(),
                value: 1.0
            }
        )
        .contains("Girişleri")
    );
}

#[test]
fn lists_grow_and_shrink() {
    let s = schema();
    let change = |src: &str, e: Edit| {
        edit(&one(src), &e, &s)
            .map(|(t, _)| texts(&t).join(" | "))
            .unwrap_or_else(|e| e)
    };
    assert_eq!(
        change("min(1, 2)", Edit::AddPort { node: "0".into() }),
        "min(1, 2, ?)"
    );
    assert_eq!(
        change(
            "min(1, 2)",
            Edit::RemovePort {
                node: "0".into(),
                port: 0
            }
        ),
        "min(2) | 1"
    );
    assert_eq!(
        change("Kat içinde (1)", Edit::AddPort { node: "0".into() }),
        "Kat içinde (1, ?)"
    );
    assert_eq!(
        change(
            "durum eğer Kat > 1 ise 'a' son",
            Edit::AddPort { node: "0".into() }
        ),
        "durum eğer Kat > 1 ise 'a' eğer ? ise ? son"
    );
    assert_eq!(
        change(
            "durum eğer Kat > 1 ise 'a' yoksa 'b' son",
            Edit::RemovePort {
                node: "0".into(),
                port: 2
            }
        ),
        "durum eğer Kat > 1 ise 'a' son | 'b'"
    );
    // The last condition stays.
    assert!(
        change(
            "durum eğer Kat ise 1 son",
            Edit::RemovePort {
                node: "0".into(),
                port: 0
            }
        )
        .contains("kaldırılamaz")
    );
}

#[test]
fn every_palette_entry_makes_a_node() {
    let s = schema();
    for section in crate::editor::catalog(&s, "") {
        for item in section.items {
            let (t, _) = edit(
                &one(""),
                &Edit::Add {
                    key: item.key.clone(),
                    at: (0.0, 0.0),
                },
                &s,
            )
            .unwrap_or_else(|e| panic!("{}: {e}", item.key));
            assert!(read(&t[1].text).is_ok(), "{}: {}", item.key, t[1].text);
        }
    }
}
