//! A wall's components' and a slab's project materials (docs/adr/0785): one row per wall component (its text material,
//! and whether it is bound to a project material), a slab's one row; each a chooser of Bağsız and the project's
//! materials, written through `cad.materials.edit`'s Assign in one undo step; Malzemeler… opens the window.

use kentos_commands::{ArchMaterial, MaterialTarget, MaterialsOperation, WallComponent};
use kentos_domain::Slot;

use super::{Choice, Editor, Event, Row, Section};
use crate::app::Message;
use crate::document::Document;

/// The words a component's or a slab's material is shown with: the material's name, bound or not.
fn words(text: Option<&str>, id: Option<&str>, table: &[ArchMaterial]) -> String {
    match id.and_then(|id| table.iter().find(|m| m.id == id)) {
        Some(m) => format!("{} · bağlı", m.name),
        None => match text.filter(|t| !t.trim().is_empty()) {
            Some(t) => format!("{t} · metin"),
            None => "Yok".to_owned(),
        },
    }
}

/// The chooser of a target: Bağsız, each project material, then Malzemeler….
fn chooser(id: Option<&str>, table: &[ArchMaterial], target: MaterialTarget, locked: bool, text: String) -> Option<Editor> {
    let assign = |material: Option<String>| {
        Message::Properties(Event::Material(MaterialsOperation::Assign {
            material,
            targets: vec![target.clone()],
        }))
    };
    let mut items = vec![Choice::Pick {
        label: "Bağsız (metin malzeme)".into(),
        swatch: None,
        icon: None,
        wide: false,
        chosen: id.is_none(),
        enabled: id.is_some() && !locked,
        message: assign(None),
    }];
    for m in table {
        let chosen = id == Some(m.id.as_str());
        items.push(Choice::Pick {
            label: format!("{} · {}", m.name, m.category.label()),
            swatch: None,
            icon: Some("materials"),
            wide: false,
            chosen,
            enabled: !chosen && !locked,
            message: assign(Some(m.id.clone())),
        });
    }
    items.push(Choice::Separator);
    items.push(Choice::Command {
        label: "Malzemeler…",
        icon: crate::icons::from_web(Some("materials")),
        id: "arch.materials",
    });
    Some(Editor::Select {
        text,
        swatch: None,
        icon: None,
        items,
    })
}

/// The materials section of the wall or slab in `slot`; none for other objects or a project without materials.
pub(super) fn section(doc: &Document, slot: Slot, locked: bool) -> Option<Section> {
    let table = doc.model.materials();
    if table.is_empty() {
        return None;
    }
    let uid = doc.model.uid(slot)?.to_string();
    let mut rows = Vec::new();
    match doc.model.get(slot)? {
        kentos_commands::Entity::Wall(w) => {
            for (i, c) in w.wall.components.iter().enumerate() {
                rows.push(component_row(i, c, table, &uid, locked));
            }
        }
        kentos_commands::Entity::Slab(s) => {
            let text = words(s.slab.material.as_deref(), s.slab.material_id.as_deref(), table);
            let target = MaterialTarget::Slab { uid: uid.clone() };
            rows.push(
                Row::text("Proje malzemesi", text.clone())
                    .editor(chooser(s.slab.material_id.as_deref(), table, target, locked, text)),
            );
        }
        _ => return None,
    }
    if rows.is_empty() {
        return None;
    }
    Some(Section {
        id: "materials",
        title: "Malzemeler".into(),
        rows,
    })
}

fn component_row(i: usize, c: &WallComponent, table: &[ArchMaterial], uid: &str, locked: bool) -> Row {
    let text = words(c.material.as_deref(), c.material_id.as_deref(), table);
    let target = MaterialTarget::Wall {
        uid: uid.to_owned(),
        component: i as u32,
    };
    Row::text(format!("{}. {}", i + 1, c.name), text.clone())
        .editor(chooser(c.material_id.as_deref(), table, target, locked, text))
}
