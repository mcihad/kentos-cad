//! An object's room module (docs/adr/0775 §3): the module and the instance it is in, the instance's version against the
//! source's, whether the object keeps its own values (a local exception), and the instance's actions (Kaynağı bu
//! örnekten güncelle, Yerel istisna yap, Kaynağa döndür, Bağımsızlaştır), each through `cad.modules.edit` in one undo
//! step.
use kentos_commands::ModulesOperation;
use kentos_domain::Slot;

use super::{Choice, Editor, Event, Row, Section};
use crate::app::Message;
use crate::document::Document;

/// The module section of the object in `slot`; none when it is in no instance.
pub(super) fn section(doc: &Document, slot: Slot, locked: bool) -> Option<Section> {
    let uid = doc.model.uid(slot)?.to_string();
    let arch = doc.model.settings().arch.as_ref()?;
    let (m, inst, key) = arch.modules.iter().find_map(|m| {
        m.instances.iter().find_map(|i| {
            i.members
                .iter()
                .find(|o| o.uid == uid)
                .map(|o| (m, i, o.key.clone()))
        })
    })?;
    let action =
        |label: &'static str, icon: &'static str, op: Option<ModulesOperation>| Choice::Action {
            label,
            icon,
            message: op
                .filter(|_| !locked)
                .map(|op| Message::Properties(Event::Module(op))),
        };
    let kept = inst.exceptions.contains(&key);
    let mut rows = vec![Row::text("Örnek", inst.id.clone())];
    let version = if m.removed {
        "Kaynağı silinmiş (bağ kopuk)".to_owned()
    } else if inst.version < m.version {
        format!("{} · eski (kaynağın sürümü {})", inst.version, m.version)
    } else {
        format!("{} · güncel", inst.version)
    };
    rows.push(Row::text("Sürüm", version));
    let mut where_ = Vec::new();
    if let Some(l) = &inst.level {
        where_.push(
            arch.levels
                .iter()
                .find(|x| &x.id == l)
                .map_or_else(|| l.clone(), |x| x.name.clone()),
        );
    }
    if inst.mirror {
        where_.push("aynalı".into());
    }
    if inst.rotation != 0.0 {
        where_.push(format!("{:.0}°", inst.rotation.to_degrees()));
    }
    if !where_.is_empty() {
        rows.push(Row::text("Yerleşim", where_.join(" · ")));
    }
    let words = if kept {
        "Evet (yerel istisna)"
    } else {
        "Hayır"
    };
    let module = m.id.clone();
    let items = vec![
        action(
            "Kaynağı bu örnekten güncelle",
            "moduleUpdate",
            (!m.removed).then(|| ModulesOperation::Update {
                module: module.clone(),
                from: inst.id.clone(),
                instances: Vec::new(),
            }),
        ),
        Choice::Separator,
        action(
            "Yerel istisna yap",
            "moduleExcept",
            (!kept).then(|| ModulesOperation::Except {
                module: module.clone(),
                uids: vec![uid.clone()],
            }),
        ),
        action(
            "Kaynağa döndür",
            "moduleRestore",
            (!m.removed).then(|| ModulesOperation::Restore {
                module: module.clone(),
                uids: vec![uid.clone()],
            }),
        ),
        Choice::Separator,
        action(
            "Örneği bağımsızlaştır",
            "moduleDetach",
            Some(ModulesOperation::Detach {
                module: module.clone(),
                instance: inst.id.clone(),
            }),
        ),
        Choice::Command {
            label: "Oda modülleri…",
            icon: crate::icons::from_web(Some("roomModules")),
            id: "arch.modules",
        },
    ];
    rows.push(
        Row::text("Yerel istisna", words).editor(Some(Editor::Select {
            text: words.to_owned(),
            swatch: None,
            icon: None,
            items,
        })),
    );
    Some(Section {
        id: "module",
        title: format!("Modül: {}", m.name).into(),
        rows,
    })
}
