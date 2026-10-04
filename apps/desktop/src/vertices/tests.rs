//! Köşe tablosu's writes against `fixtures/vertex-table/v1/edits.json`
//! (`scripts/fixtures/vertex_edit_cases.py`, worked out from the rules with
//! no KentOS code), as the web's `vertexEdit.test.ts`.

// Test harness code: the std float methods are fine here.
#![allow(clippy::disallowed_methods)]

use kentos_domain::Slot;
use serde_json::{Value, json};

use super::edit::{At, Column, Draft, cell_text, remove, rows_of, write_cell, write_draft};

/// Bulges within 1e-12 (relative), every other number exactly; a missing field is null.
fn same(a: &Value, e: &Value, path: &str) -> Result<(), String> {
    match (a, e) {
        (Value::Number(x), Value::Number(y)) => {
            let (x, y) = (
                x.as_f64().unwrap_or(f64::NAN),
                y.as_f64().unwrap_or(f64::NAN),
            );
            let close = if path.contains(".bulges") {
                (x - y).abs() <= 1e-12 * y.abs().max(1.0)
            } else {
                x == y
            };
            if close {
                Ok(())
            } else {
                Err(format!("{path}: {x} ≠ {y}"))
            }
        }
        (Value::Array(x), Value::Array(y)) => {
            if x.len() != y.len() {
                return Err(format!("{path}: {} öğe ≠ {} öğe", x.len(), y.len()));
            }
            for (i, (p, q)) in x.iter().zip(y).enumerate() {
                same(p, q, &format!("{path}[{i}]"))?;
            }
            Ok(())
        }
        (Value::Object(x), Value::Object(y)) => {
            let mut keys: Vec<&String> = x.keys().chain(y.keys()).collect();
            keys.sort();
            keys.dedup();
            for k in keys {
                same(
                    x.get(k).unwrap_or(&Value::Null),
                    y.get(k).unwrap_or(&Value::Null),
                    &format!("{path}.{k}"),
                )?;
            }
            Ok(())
        }
        _ if a == e => Ok(()),
        _ => Err(format!("{path}: {a} ≠ {e}")),
    }
}

/// An object as the cases compare it: line work's paths with their
/// elevations and bulges (null when every edge is straight).
fn view(e: &kentos_contracts::Entity) -> Value {
    use kentos_contracts::Entity;
    if !matches!(
        e,
        Entity::Line(_) | Entity::Polyline(_) | Entity::Polygon(_)
    ) {
        return json!({ "kind": e.kind() });
    }
    let paths: Vec<Value> = kentos_native_application::elevation::paths(e)
        .iter()
        .map(|p| {
            let b: Vec<f64> = (0..p.pts.len())
                .map(|i| {
                    p.bulges
                        .as_ref()
                        .and_then(|b| b.get(i).copied())
                        .unwrap_or(0.0)
                })
                .collect();
            let arcs = b.iter().any(|x| x.abs() > 1e-12);
            json!({
                "pts": p.pts.iter().map(|q| [q.x, q.y]).collect::<Vec<_>>(),
                "zs": p.zs,
                "bulges": if arcs { json!(b) } else { Value::Null },
            })
        })
        .collect();
    json!({ "kind": e.kind(), "paths": paths })
}

/// Every case of `fixtures/vertex-table/v1/edits.json` on its own drawing,
/// through the table's writes: the drawing after, the messages said, the
/// undo step, whether the cell stays open and where the next draft goes.
#[test]
fn every_write_is_the_references() {
    let file: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/vertex-table/v1/edits.json"
    ))
    .expect("edits.json reads");
    assert_eq!(file["format"], "kentos.vertex-edits");
    let layers: Vec<Value> = file["layers"]
        .as_array()
        .expect("layers")
        .iter()
        .map(|l| {
            json!({ "id": l["id"], "name": l["name"], "type": "layer", "visible": true, "locked": l["locked"], "expanded": true,
                "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 }, "children": [] })
        })
        .collect();
    let cases = file["cases"].as_array().expect("cases");
    assert!(cases.len() >= 50, "{} cases", cases.len());
    let mut off = Vec::new();
    for c in cases {
        let name = c["name"].as_str().unwrap_or_default();
        let snapshot = json!({
            "format": "kentos.document", "version": 1, "name": name,
            "settings": { "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                "plotScale": 1000, "workspace": c["workspace"], "drawingFont": "barlow" },
            "origin": { "x": 0, "y": 0 }, "layers": layers, "activeLayer": "cizim", "entities": c["objects"],
            "styles": { "items": [], "categories": [] }
        });
        let snapshot = kentos_contracts::DocumentSnapshotV1::from_json(&snapshot.to_string())
            .expect("the case's drawing");
        let mut doc = kentos_domain::Document::from_snapshot(snapshot).expect("opens");
        let text = |v: &Value| v.as_str().unwrap_or_default().to_owned();
        let index = |v: &Value| v.as_u64().unwrap_or_default() as usize;
        let (said, step, stay, next) = if let Some(a) = c.get("cell") {
            let col = Column::from_key(a["column"].as_str().unwrap_or_default()).expect("a column");
            let at = At {
                path: index(&a["path"]),
                index: index(&a["index"]),
            };
            let slot = Slot(a["id"].as_u64().unwrap_or_default() as u32);
            let out = write_cell(&mut doc, slot, at, col, &text(&a["text"]));
            (out.said, out.step, out.stay, None)
        } else if let Some(a) = c.get("draft") {
            let at = At {
                path: index(&a["path"]),
                index: index(&a["after"]),
            };
            let d = Draft {
                east: text(&a["east"]),
                north: text(&a["north"]),
                z: text(&a["z"]),
            };
            let slot = Slot(a["id"].as_u64().unwrap_or_default() as u32);
            let (out, next) = write_draft(&mut doc, slot, at, &d);
            (
                out.said,
                out.step,
                out.stay,
                Some(next.map_or(Value::Null, |n| json!([n.path, n.index]))),
            )
        } else {
            let a = &c["remove"];
            let at: Vec<At> = a["at"]
                .as_array()
                .expect("at")
                .iter()
                .map(|x| At {
                    path: index(&x[0]),
                    index: index(&x[1]),
                })
                .collect();
            let slot = Slot(a["id"].as_u64().unwrap_or_default() as u32);
            let out = remove(&mut doc, slot, &at);
            (out.said, out.step, out.stay, None)
        };
        let objects: Vec<Value> = doc.entities().map(view).collect();
        let undone = match step {
            Some(_) => doc.undo(),
            None => doc.can_undo().then(|| "yazıldı".to_owned()),
        };
        let mut seen = json!({ "said": said, "step": undone, "stay": stay, "objects": objects });
        if let Some(next) = next {
            seen["next"] = next;
        }
        if let Err(e) = same(&seen, &c["expected"], name) {
            off.push(e);
        }
    }
    assert!(
        off.is_empty(),
        "{} durum farklı:\n{}",
        off.len(),
        off.join("\n")
    );
}

/// A cell opens with its value whole, in a local project's unit.
#[test]
fn a_cell_opens_with_the_value_whole() {
    let e: kentos_contracts::Entity = serde_json::from_value(json!({
        "kind": "polyline", "id": 1, "layerId": "0", "attrs": {},
        "pts": [{ "x": 1.25, "y": 2.0 }, { "x": 3.0, "y": 4.5 }, { "x": 6.0, "y": 4.5 }],
        "bulges": [0.5, 0.0, 0.0], "zs": [10.0, null, null]
    }))
    .expect("a polyline");
    let rows = rows_of(&e);
    let (first, second) = (&rows[0], &rows[1]);
    assert_eq!(
        [
            cell_text(first, Column::East, 1.0),
            cell_text(first, Column::North, 1.0),
            cell_text(first, Column::Z, 1.0),
            cell_text(second, Column::Z, 1.0),
        ],
        ["1.25", "2", "10", ""].map(str::to_owned)
    );
    let radius: f64 = cell_text(first, Column::Radius, 1.0)
        .parse()
        .expect("a number");
    assert_eq!(Some(radius), first.radius);
    assert_eq!(cell_text(second, Column::Radius, 1.0), "");
    assert_eq!(cell_text(first, Column::East, 100.0), "125");
}

fn app_with_vertices() -> crate::app::App {
    let (mut app, _) = crate::app::App::boot(None);
    let snapshot = kentos_contracts::DocumentSnapshotV1::from_json(include_str!(
        "../../../../fixtures/interaction/v1/vertex-table.kcad"
    ))
    .expect("the drawing reads");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let _ = app.update(crate::app::Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn ev(app: &mut crate::app::App, e: super::Event) {
    let _ = app.update(crate::app::Message::Vertices(e));
}

fn parcel_x(app: &crate::app::App, i: usize) -> f64 {
    match app.document.as_ref().and_then(|d| d.model.get(Slot(1))) {
        Some(kentos_contracts::Entity::Polygon(a)) => a.pts[i].x,
        _ => panic!("the parcel"),
    }
}

/// The table in the app: the parcel's rows, a row selected and ringed, a Y
/// typed in place (Enter writes it and opens the row below), a radius
/// refused with the cell open and said; two objects or a locked layer show
/// the table without writing it.
#[test]
fn the_table_selects_rows_and_edits_cells_in_place() {
    use super::{Cell, Event, Walk, texts};
    use crate::app::Message;
    let mut app = app_with_vertices();
    app.selection.set([Slot(1)]);
    let _ = app.update(Message::Run("view.coords"));
    let target = app.vertex_target().expect("the parcel in the table");
    assert!(target.writes);
    assert_eq!(app.vertex_rows(Slot(1)).len(), 13);
    // A press selects a row and rings its vertex.
    ev(&mut app, Event::Press(2));
    assert_eq!(app.vertex_marks().len(), 1);
    // Y of the second row: its whole value, then written with Enter, the row below open.
    ev(&mut app, Event::Edit(1, Column::East));
    assert_eq!(app.vertices.text, "486760");
    ev(&mut app, Event::Input("486761.5".into()));
    ev(&mut app, Event::Finish(Some(Walk::Down)));
    assert_eq!(parcel_x(&app, 1), 486761.5);
    assert_eq!(
        app.vertices.editing,
        Some((Cell::Vertex(At { path: 0, index: 2 }), Column::East))
    );
    assert_eq!(app.vertices.text, "486780");
    // The row selection survives the write: the vertex count is the same.
    assert_eq!(app.vertex_marks().len(), 1);
    ev(&mut app, Event::Cancel);
    // A radius shorter than half the chord (the first edge, its end moved above): refused, said,
    // the cell kept open with what was typed.
    ev(&mut app, Event::Edit(0, Column::Radius));
    ev(&mut app, Event::Input("10".into()));
    ev(&mut app, Event::Finish(Some(Walk::Down)));
    assert_eq!(
        app.vertices.editing,
        Some((Cell::Vertex(At { path: 0, index: 0 }), Column::Radius))
    );
    assert_eq!(app.vertices.text, "10");
    let said = app.log.last().map(|l| l.text.clone()).unwrap_or_default();
    assert_eq!(
        said,
        "Köşe tablosu: Yarıçap kirişin yarısından küçük olamaz: en az 30.851 m."
    );
    ev(&mut app, Event::Cancel);
    // Tab walks right, over the cells that are edited: Z to Yarıçap, Yarıçap to the next row's Y.
    ev(&mut app, Event::Edit(0, Column::Z));
    ev(&mut app, Event::Finish(Some(Walk::Right)));
    assert_eq!(
        app.vertices.editing,
        Some((Cell::Vertex(At { path: 0, index: 0 }), Column::Radius))
    );
    ev(&mut app, Event::Finish(Some(Walk::Right)));
    assert_eq!(
        app.vertices.editing,
        Some((Cell::Vertex(At { path: 0, index: 1 }), Column::East))
    );
    ev(&mut app, Event::Cancel);
    // Satır ekle under the first row: Y, Tab, X, Enter adds the vertex after the first and opens
    // the next draft under it; Esc leaves it.
    ev(&mut app, Event::Press(0));
    assert!(app.vertices.keyboard);
    ev(&mut app, Event::AddRow);
    assert_eq!(app.vertices.editing, Some((Cell::Draft, Column::East)));
    ev(&mut app, Event::Input("486730".into()));
    ev(&mut app, Event::Finish(Some(Walk::Right)));
    assert_eq!(app.vertices.editing, Some((Cell::Draft, Column::North)));
    ev(&mut app, Event::Input("4420190".into()));
    ev(&mut app, Event::Finish(Some(Walk::Down)));
    let outer = |app: &crate::app::App| match app.document.as_ref().and_then(|d| d.model.get(Slot(1))) {
        Some(kentos_contracts::Entity::Polygon(a)) => a.pts.clone(),
        _ => panic!("the parcel"),
    };
    assert_eq!(outer(&app).len(), 6);
    assert_eq!((outer(&app)[1].x, outer(&app)[1].y), (486730.0, 4420190.0));
    assert_eq!(app.vertices.editing, Some((Cell::Draft, Column::East)));
    assert_eq!(app.vertex_rows(Slot(1)).len(), 14);
    ev(&mut app, Event::Cancel);
    assert!(app.vertices.draft.is_none());
    // Sil: the new vertex's row removed in one step.
    ev(&mut app, Event::Press(1));
    ev(&mut app, Event::Remove);
    assert_eq!(outer(&app).len(), 5);
    let undone = app.document.as_mut().and_then(|d| d.model.undo());
    assert_eq!(undone.as_deref(), Some("Köşe sil"));
    // Two objects: the first shown, not written.
    app.selection.set([Slot(1), Slot(2)]);
    let target = app.vertex_target().expect("the first in the table");
    assert_eq!(
        (target.slot, target.writes, target.note),
        (Slot(1), false, texts::MANY_NOTE)
    );
    ev(&mut app, Event::Edit(0, Column::East));
    assert!(!app.vertices.editing());
    // A locked layer's polyline: shown, not written.
    app.selection.set([Slot(4)]);
    let target = app.vertex_target().expect("the locked polyline");
    assert_eq!((target.writes, target.note), (false, texts::LOCKED_NOTE));
    // A point is the point list's.
    app.selection.set(Vec::<Slot>::new());
    assert!(app.vertex_target().is_none());
}

/// Köşe tablosu's pictures (docs/adr/0172), the web's `shots.mjs
/// vertextable`: the parcel's table with a row selected and its vertex
/// ringed, a Y written with Enter (the row below open), a radius refused,
/// a vertex added with Satır ekle, the road's arcs, two objects selected.
#[test]
#[ignore = "writes pictures: cargo test -p kentos-desktop vertices::tests::screens -- --ignored --nocapture"]
fn screens() {
    use super::{Event, Walk};
    use crate::app::{App, Message};
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for name in [
                "kose-tablosu",
                "kose-tablosu-duzenle",
                "kose-tablosu-yaricap",
                "kose-tablosu-satir-ekle",
                "kose-tablosu-yol",
                "kose-tablosu-coklu",
            ] {
                let mut app = app_with_vertices();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let ids = match name {
                    "kose-tablosu-yol" => vec![Slot(2)],
                    "kose-tablosu-coklu" => vec![Slot(1), Slot(2)],
                    _ => vec![Slot(1)],
                };
                app.selection.set(ids);
                let _ = app.update(Message::Run("view.coords"));
                let _ = app.update(Message::BottomResized(300.0));
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                match name {
                    "kose-tablosu" => ev(&mut app, Event::Press(2)),
                    "kose-tablosu-duzenle" => {
                        ev(&mut app, Event::Press(1));
                        ev(&mut app, Event::Edit(1, Column::East));
                        ev(&mut app, Event::Input("486761.5".into()));
                        ev(&mut app, Event::Finish(Some(Walk::Down)));
                    }
                    "kose-tablosu-yaricap" => {
                        ev(&mut app, Event::Press(0));
                        ev(&mut app, Event::Edit(0, Column::Radius));
                        ev(&mut app, Event::Input("10".into()));
                        ev(&mut app, Event::Finish(Some(Walk::Down)));
                    }
                    // Satır ekle under the first row: a vertex written, the next draft open under it.
                    "kose-tablosu-satir-ekle" => {
                        ev(&mut app, Event::Press(0));
                        ev(&mut app, Event::AddRow);
                        ev(&mut app, Event::Input("486730".into()));
                        ev(&mut app, Event::Finish(Some(Walk::Right)));
                        ev(&mut app, Event::Input("4420190".into()));
                        ev(&mut app, Event::Finish(Some(Walk::Down)));
                    }
                    _ => {}
                }
                snapshot.settle(&mut app, App::view, &mut update);
                if app.vertices.editing() {
                    // The field takes the keyboard with its text selected, as the app's task
                    // gives it when the edit opens.
                    let task = crate::points::focus_field();
                    use iced::futures::StreamExt as _;
                    if let Some(mut stream) = iced_runtime::task::into_stream(task) {
                        while let Some(action) = iced::futures::executor::block_on(stream.next()) {
                            if let iced_runtime::Action::Widget(operation) = action {
                                snapshot.operate(app.view(), operation);
                            }
                        }
                    }
                    snapshot.settle(&mut app, App::view, &mut update);
                }
                let file = out.join(format!("{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
