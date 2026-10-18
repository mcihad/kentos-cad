//! `cad.walls.edit` v1 (docs/adr/0339 §5): walls named by their persistent ids changed as one undo step “Duvarı
//! düzenle”, all or none: their width, justification, offset, style, ends, fill, height and architectural
//! information. Öznitelikler writes through it. The shared cases are `fixtures/commands/v1/cad.walls.edit.json`.
//!
//! A style given takes its components, ends and fill (and its width when its components fix it); a style taken away
//! leaves one plain body. A width the components do not fit, a face a new width or offset cannot draw, a level the
//! project does not have: nothing is written.

use kentos_contracts::{
    CommandResult, Entity, WallChange, WallEntity, WallStyleDef, WallsEdit, WallsEdited,
    WallsOperation, fixed_width,
};
use kentos_domain::{Document, Slot};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::walls;

pub const STEP: &str = "Duvarı düzenle";

/// The walls as they would be, each its slot, and KentOS's styles taken.
type Work = (Vec<(Slot, WallEntity)>, Vec<WallStyleDef>);

/// `w` changed as `c` says, `style` the style a change names (found), `at` where the input names it.
pub(crate) fn changed(
    doc: &Document,
    w: &WallEntity,
    c: &WallChange,
    style: Option<&WallStyleDef>,
    at: &str,
) -> Result<WallEntity, Stop> {
    let mut out = w.clone();
    let f = &mut out.wall;
    match (&c.style, style) {
        (Some(Some(_)), Some(s)) => {
            f.style = Some(s.id.clone());
            f.components = s.components.clone();
            f.caps = s.caps;
            f.fill = s.fill.clone();
            if let Some(width) = fixed_width(&s.components) {
                f.width = width;
            }
        }
        (Some(None), _) => {
            f.style = None;
            f.components.clear();
        }
        _ => {}
    }
    if let Some(width) = c.width {
        f.width = width;
    }
    if let Some(j) = c.justify {
        f.justify = j;
    }
    if let Some(o) = c.offset {
        f.offset = o;
    }
    if let Some(caps) = c.caps {
        f.caps = caps;
    }
    if let Some(fill) = &c.fill {
        f.fill = fill.clone();
    }
    if let Some(h) = c.height {
        f.height = h;
    }
    if let Some(l) = &c.level {
        f.arch.level = l.clone();
    }
    if let Some(p) = c.phase {
        f.arch.phase = p;
    }
    if let Some(s) = c.status {
        f.arch.status = s;
    }
    if let Some(d) = c.discipline {
        f.arch.discipline = d;
    }
    walls::valid(&out.wall, "operation")?;
    walls::drawable(&out.wall, at)?;
    walls::level_known(doc, &out.wall.arch, "operation.level")?;
    Ok(out)
}

fn check(doc: &Document, input: &WallsEdit) -> Result<Work, Stop> {
    checks::uids(&input.uids, "Düzenlenecek duvar verilmedi.")?;
    let found = checks::named(doc, &input.uids)?;
    let WallsOperation::Set(c) = &input.operation;
    for (at, _, e, _) in &found {
        if !matches!(e, Entity::Wall(_)) {
            return Err(Stop::failed(error(
                codes::NOT_A_WALL,
                "Nesne duvar değil; Duvarı düzenle yalnız duvarları değiştirir.".into(),
                Some(format!("uids[{at}]")),
            )));
        }
    }
    if c == &WallChange::default() {
        return Err(Stop::failed(error(
            codes::NOTHING_TO_SET,
            "Değiştirilecek bir alan verilmedi: genişlik, hiza, öteleme, stil, uçlar, dolgu, yükseklik, kat, evre, durum ya da disiplin verin.".into(),
            Some("operation".into()),
        )));
    }
    for (v, what, name) in [
        (c.width, "width", "Genişlik"),
        (c.offset, "offset", "Öteleme"),
        (c.height.flatten(), "height", "Yükseklik"),
    ] {
        if let Some(x) = v {
            checks::finite(
                x,
                name,
                "Sonlu bir sayı verin.",
                &format!("operation.{what}"),
            )?;
        }
    }
    let mut added = Vec::new();
    let style = match &c.style {
        Some(Some(key)) => {
            let (s, builtin) = walls::find_style(doc, key)
                .ok_or_else(|| walls::no_style(doc, key, "operation.style"))?;
            if builtin {
                added.push(s.clone());
            }
            Some(s)
        }
        _ => None,
    };
    let mut out = Vec::with_capacity(found.len());
    for (at, slot, e, _) in &found {
        let Entity::Wall(w) = e else { continue };
        out.push((
            *slot,
            changed(doc, w, c, style.as_ref(), &format!("uids[{at}]"))?,
        ));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    // All or none: a wall on a locked layer refuses the whole change.
    for (at, _, e, _) in &found {
        let layer = &e.base().layer_id;
        if doc.layers().is_locked(layer) {
            let name = doc
                .layers()
                .get(layer)
                .map_or(layer.as_str(), |n| n.name.as_str());
            return Err(Stop::failed(error(
                codes::LAYER_LOCKED,
                format!(
                    "“{name}” katmanı kilitli; üzerindeki duvar düzenlenemez. Kilidi Katmanlar panelinden açın."
                ),
                Some(format!("uids[{at}]")),
            )));
        }
    }
    Ok((out, added))
}

pub fn validate(cx: &ExecutionContext<'_>, input: &WallsEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &WallsEdit) -> CommandResult<WallsEdited> {
    match check(cx.doc, input) {
        Ok((walls, added)) => CommandResult::Completed {
            output: WallsEdited {
                walls: walls.into_iter().map(|(_, w)| Entity::Wall(w)).collect(),
                styles_added: added.into_iter().map(|s| s.id).collect(),
                revision: cx.doc.revision().to_string(),
            },
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: WallsEdit) -> CommandResult<WallsEdited> {
    crate::call::recorded(
        cx,
        kentos_contracts::CAD_WALLS_EDIT,
        kentos_contracts::CAD_WALLS_EDIT_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: WallsEdit) -> CommandResult<WallsEdited> {
    let (walls, added) = match check(cx.doc, &input) {
        Ok(work) => work,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    walls::take_styles(doc, &added);
    let entities: Vec<Entity> = walls.iter().map(|(_, w)| Entity::Wall(w.clone())).collect();
    doc.update_many(
        walls
            .into_iter()
            .map(|(s, w)| (s, Entity::Wall(w)))
            .collect(),
        STEP,
    );
    CommandResult::Completed {
        output: WallsEdited {
            walls: entities,
            styles_added: added.into_iter().map(|s| s.id).collect(),
            revision: doc.revision().to_string(),
        },
        warnings: Vec::new(),
    }
}
