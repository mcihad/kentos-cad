//! `cad.materials.edit` v1 (docs/adr/0785): the project's building materials made, changed and removed, wall style
//! components, wall components and slabs bound to one (or left), and the text materials taken into the table, as one
//! undo step “Malzeme” with every style, wall and slab bound to a changed material. Its plan is the preview: the table
//! and what follows, nothing written. The shared cases are `fixtures/commands/v1/cad.materials.edit.json`
//! (`scripts/fixtures/materials_cases.py`).
//!
//! A bound component's `material` text, `hatch` and `edge` are its material's name, hatch and cut pen
//! ([`ArchMaterial::bind`]); a bound slab's `material` is the material's name. A style's change reaches the walls of the
//! style (their components are the style's); a wall's own components are rewritten too. Walls and slabs on locked
//! layers keep their older values (`material_locked`); a target named on a locked layer refuses the write
//! (`layer_locked`).

use kentos_commands::{
    ArchMaterial, CommandResult, CommandWarning, Entity, MaterialChange, MaterialReport, MaterialTarget,
    MaterialsEdit, MaterialsEdited, MaterialsOperation, WallComponent, WallStyleDef, material_id_of, material_id_ok,
    material_name_problem, materials_problem,
};
use kentos_domain::{Document, Slot, Uuid};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

pub const STEP: &str = "Malzeme";

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

/// What the command writes: the table, the wall styles after it, the walls and slabs rewritten, the report.
struct Work {
    materials: Vec<ArchMaterial>,
    styles: Vec<WallStyleDef>,
    updates: Vec<(Slot, Entity)>,
    report: MaterialReport,
    material: Option<String>,
}

/// The project's material `key` names (its id, else its name, case aside), by its place.
fn find(list: &[ArchMaterial], key: &str, path: &str) -> Result<usize, Stop> {
    let folded = key.to_lowercase();
    list.iter()
        .position(|m| m.id == key)
        .or_else(|| list.iter().position(|m| m.name.to_lowercase() == folded))
        .ok_or_else(|| {
            fail(
                codes::MATERIAL_NOT_FOUND,
                format!("Projede “{key}” kimlikli ya da adlı malzeme yok. Malzemeler penceresindeki bir malzemeyi verin."),
                path,
            )
        })
}

/// A free id for `name`: from its name, then with `-2`, `-3` ….
fn free_id(list: &[ArchMaterial], name: &str) -> String {
    let base = material_id_of(name);
    let mut id = base.clone();
    let mut n = 2;
    while list.iter().any(|m| m.id == id) {
        id = format!("{base}-{n}");
        n += 1;
    }
    id
}

/// Whether `name` (case aside) or `id` is another material's: the words; none when free.
fn taken(list: &[ArchMaterial], id: &str, name: &str, but: Option<usize>) -> Option<String> {
    let folded = name.to_lowercase();
    list.iter().enumerate().filter(|(i, _)| Some(*i) != but).find_map(|(_, m)| {
        if m.id == id {
            Some(format!("Projede “{id}” kimlikli bir malzeme (“{}”) zaten var.", m.name))
        } else if m.name.to_lowercase() == folded {
            Some(format!("Projede “{}” adında bir malzeme zaten var; başka bir ad verin.", m.name))
        } else {
            None
        }
    })
}

/// `m` changed by `c` (its name aside, which the caller checks).
fn changed(mut m: ArchMaterial, c: &MaterialChange) -> ArchMaterial {
    if let Some(n) = &c.name {
        m.name.clone_from(n);
    }
    if let Some(k) = c.category {
        m.category = k;
    }
    if let Some(q) = c.quantity {
        m.quantity = q;
    }
    if let Some(h) = &c.hatch {
        m.hatch.clone_from(h);
    }
    if let Some(e) = &c.cut {
        m.cut.clone_from(e);
    }
    m
}

fn invalid(field: &str, words: String) -> Stop {
    fail(codes::INVALID_MATERIAL, words, field)
}

/// The components rebound to the table: each bound one the material's look; one bound to a material the table has not
/// (a removed one cannot be: it is refused) stays.
fn rebound(components: &[WallComponent], table: &[ArchMaterial]) -> Vec<WallComponent> {
    components
        .iter()
        .map(|c| match c.material_id.as_deref().and_then(|id| table.iter().find(|m| m.id == id)) {
            Some(m) => m.bind(c),
            None => c.clone(),
        })
        .collect()
}

/// The uid text of an object's slot.
fn uid_of(doc: &Document, slot: Slot) -> String {
    doc.uid(slot).map(|u| u.to_string()).unwrap_or_default()
}

/// Every wall style and every wall and slab brought to the table `table` (after the styles `styles` are written): the
/// styles, the updates, and the report's walls, slabs and locked. `assigned` stand in for the drawing's objects of their
/// slots (an Assign's or an Adopt's); each object is compared with the drawing's own.
fn follow(doc: &Document, styles: &[WallStyleDef], table: &[ArchMaterial], assigned: &[(Slot, Entity)], work: &mut Work) {
    let before = &doc.settings().wall_styles;
    work.styles = styles
        .iter()
        .map(|s| WallStyleDef {
            components: rebound(&s.components, table),
            ..s.clone()
        })
        .collect();
    for s in &work.styles {
        if before.iter().find(|b| b.id == s.id) != Some(s) {
            work.report.styles.push(s.id.clone());
        }
    }
    let mut updates = Vec::new();
    for original in doc.entities() {
        let slot = Slot(original.base().id);
        let e = assigned.iter().find(|(s, _)| *s == slot).map_or(original, |(_, e)| e);
        let next = match e {
            Entity::Wall(w) => {
                let mut next = w.clone();
                next.wall.components = match w.wall.style.as_deref().and_then(|id| work.styles.iter().find(|s| s.id == id)) {
                    // A wall of a style takes its components when they were the style's.
                    Some(s) if before.iter().any(|b| b.id == s.id && b.components == w.wall.components) => s.components.clone(),
                    _ => rebound(&w.wall.components, table),
                };
                Entity::Wall(next)
            }
            Entity::Slab(s) => {
                let mut next = s.clone();
                if let Some(m) = s.slab.material_id.as_deref().and_then(|id| table.iter().find(|m| m.id == id)) {
                    next.slab.material = Some(m.name.clone());
                }
                Entity::Slab(next)
            }
            _ => continue,
        };
        if next == *original {
            continue;
        }
        let uid = uid_of(doc, slot);
        if doc.layers().is_locked(&original.base().layer_id) {
            work.report.locked.push(uid);
            continue;
        }
        match next {
            Entity::Wall(_) => work.report.walls.push(uid),
            _ => work.report.slabs.push(uid),
        }
        updates.push((slot, next));
    }
    work.updates = updates;
}

/// The slot `uid` names at `path`, its entity; `invalid_uid`, `entity_not_found`.
fn object<'a>(doc: &'a Document, uid: &str, path: &str) -> Result<(Slot, &'a Entity), Stop> {
    if !checks::is_uid_text(uid) {
        return Err(fail(
            codes::INVALID_UID,
            format!("“{uid}” bir kalıcı kimlik (UUID) değil; duvarın ya da döşemenin kimliğini verin."),
            path,
        ));
    }
    let slot = Uuid::parse_str(uid).ok().and_then(|u| doc.slot_of(u)).ok_or_else(|| {
        fail(
            codes::ENTITY_NOT_FOUND,
            format!("Çizimde “{uid}” kimlikli nesne yok."),
            path,
        )
    })?;
    let e = doc.get(slot).ok_or_else(|| {
        fail(codes::ENTITY_NOT_FOUND, format!("Çizimde “{uid}” kimlikli nesne yok."), path)
    })?;
    Ok((slot, e))
}

fn locked(doc: &Document, layer: &str, path: &str) -> Result<(), Stop> {
    let layers = doc.layers();
    if layers.is_locked(layer) {
        let name = layers.get(layer).map_or(layer, |n| n.name.as_str());
        return Err(fail(
            codes::LAYER_LOCKED,
            format!("“{name}” katmanı kilitli; üzerindeki nesne malzemeye bağlanamaz. Kilidi Katmanlar panelinden açın."),
            path,
        ));
    }
    Ok(())
}

/// The component `index` of `components` at `path` (`component_not_found`).
fn component_at(len: usize, index: u32, whose: &str, path: &str) -> Result<usize, Stop> {
    let i = index as usize;
    if i >= len {
        return Err(fail(
            codes::COMPONENT_NOT_FOUND,
            format!("{whose} {len} bileşeni var; {i}. sıra yok (sıra soldan 0'dan sayılır)."),
            path,
        ));
    }
    Ok(i)
}

/// The component left: its text and look stay, its binding goes.
fn left(c: &WallComponent) -> WallComponent {
    WallComponent {
        material_id: None,
        ..c.clone()
    }
}

/// Assign: the styles and objects the targets name, bound to `m` (none: left), written straight into `styles` and
/// `entities` (by slot).
fn assign(
    doc: &Document,
    m: Option<&ArchMaterial>,
    targets: &[MaterialTarget],
    styles: &mut [WallStyleDef],
    objects: &mut Vec<(Slot, Entity)>,
) -> Result<(), Stop> {
    let set = |c: &WallComponent| match m {
        Some(m) => m.bind(c),
        None => left(c),
    };
    for (i, t) in targets.iter().enumerate() {
        let path = format!("operation.targets[{i}]");
        match t {
            MaterialTarget::WallStyle { style, component } => {
                let folded = style.to_lowercase();
                let at = styles
                    .iter()
                    .position(|s| s.id == *style)
                    .or_else(|| styles.iter().position(|s| s.name.to_lowercase() == folded))
                    .ok_or_else(|| {
                        fail(
                            codes::STYLE_NOT_FOUND,
                            format!("Projede “{style}” kimlikli ya da adlı duvar stili yok."),
                            &format!("{path}.style"),
                        )
                    })?;
                let s = &mut styles[at];
                let k = component_at(
                    s.components.len(),
                    *component,
                    &format!("“{}” stilinin", s.name),
                    &format!("{path}.component"),
                )?;
                s.components[k] = set(&s.components[k]);
            }
            MaterialTarget::Wall { uid, component } => {
                let (slot, e) = object(doc, uid, &format!("{path}.uid"))?;
                let at = match objects.iter().position(|(s, _)| *s == slot) {
                    Some(at) => at,
                    None => {
                        objects.push((slot, e.clone()));
                        objects.len() - 1
                    }
                };
                let Entity::Wall(w) = &mut objects[at].1 else {
                    return Err(fail(codes::NOT_A_WALL, "Nesne duvar değil; bileşen yalnız duvarda olur.".into(), &format!("{path}.uid")));
                };
                locked(doc, &w.base.layer_id, &format!("{path}.uid"))?;
                let k = component_at(w.wall.components.len(), *component, "Duvarın", &format!("{path}.component"))?;
                w.wall.components[k] = set(&w.wall.components[k]);
            }
            MaterialTarget::Slab { uid } => {
                let (slot, e) = object(doc, uid, &format!("{path}.uid"))?;
                let at = match objects.iter().position(|(s, _)| *s == slot) {
                    Some(at) => at,
                    None => {
                        objects.push((slot, e.clone()));
                        objects.len() - 1
                    }
                };
                let Entity::Slab(s) = &mut objects[at].1 else {
                    return Err(fail(codes::NOT_A_SLAB, "Nesne döşeme değil.".into(), &format!("{path}.uid")));
                };
                locked(doc, &s.base.layer_id, &format!("{path}.uid"))?;
                match m {
                    Some(m) => {
                        s.slab.material = Some(m.name.clone());
                        s.slab.material_id = Some(m.id.clone());
                    }
                    None => s.slab.material_id = None,
                }
            }
        }
    }
    Ok(())
}

/// Adopt: one material per text of the unbound components and slabs (styles first, then the walls, then the slabs, in
/// order), its look the first component's; components of the text with another look are left (`left`).
fn adopt(doc: &Document, list: &mut Vec<ArchMaterial>, styles: &mut [WallStyleDef], objects: &mut Vec<(Slot, Entity)>, left_texts: &mut Vec<String>) {
    let take = |text: &str, c: Option<&WallComponent>, list: &mut Vec<ArchMaterial>, left_texts: &mut Vec<String>| -> Option<ArchMaterial> {
        let folded = text.to_lowercase();
        if let Some(m) = list.iter().find(|m| m.name.to_lowercase() == folded) {
            let same = c.is_none_or(|c| c.hatch == m.hatch && c.edge == m.cut);
            if same {
                return Some(m.clone());
            }
            if !left_texts.iter().any(|t| t == text) {
                left_texts.push(text.to_owned());
            }
            return None;
        }
        if list.len() >= kentos_commands::MAX_MATERIALS || material_name_problem(text).is_some() {
            return None;
        }
        let m = ArchMaterial {
            id: free_id(list, text),
            name: text.to_owned(),
            category: Default::default(),
            quantity: Default::default(),
            hatch: c.and_then(|c| c.hatch.clone()),
            cut: c.and_then(|c| c.edge.clone()),
        };
        list.push(m.clone());
        Some(m)
    };
    for s in styles.iter_mut() {
        for c in s.components.iter_mut() {
            if c.material_id.is_some() {
                continue;
            }
            let Some(text) = c.material.clone().filter(|t| !t.trim().is_empty()) else { continue };
            if let Some(m) = take(&text, Some(c), list, left_texts) {
                *c = m.bind(c);
            }
        }
    }
    for e in doc.entities() {
        let slot = Slot(e.base().id);
        match e {
            Entity::Wall(w) => {
                if doc.layers().is_locked(&w.base.layer_id) {
                    continue;
                }
                let mut next = w.clone();
                for c in next.wall.components.iter_mut() {
                    if c.material_id.is_some() {
                        continue;
                    }
                    let Some(text) = c.material.clone().filter(|t| !t.trim().is_empty()) else { continue };
                    if let Some(m) = take(&text, Some(c), list, left_texts) {
                        *c = m.bind(c);
                    }
                }
                if next != *w {
                    objects.push((slot, Entity::Wall(next)));
                }
            }
            Entity::Slab(s) => {
                if s.slab.material_id.is_some() || doc.layers().is_locked(&s.base.layer_id) {
                    continue;
                }
                let Some(text) = s.slab.material.clone() else { continue };
                if let Some(m) = take(&text, None, list, left_texts) {
                    let mut next = s.clone();
                    next.slab.material_id = Some(m.id.clone());
                    objects.push((slot, Entity::Slab(next)));
                }
            }
            _ => {}
        }
    }
}

/// The objects bound to material `id` in the drawing and the styles: the wall styles' names, the walls' and slabs' ids.
fn bound_to(doc: &Document, id: &str) -> (Vec<String>, Vec<String>) {
    let styles = doc
        .settings()
        .wall_styles
        .iter()
        .filter(|s| s.components.iter().any(|c| c.material_id.as_deref() == Some(id)))
        .map(|s| s.name.clone())
        .collect();
    let mut uids = Vec::new();
    for e in doc.entities() {
        let hit = match e {
            Entity::Wall(w) => w.wall.components.iter().any(|c| c.material_id.as_deref() == Some(id)),
            Entity::Slab(s) => s.slab.material_id.as_deref() == Some(id),
            _ => false,
        };
        if hit {
            uids.push(uid_of(doc, Slot(e.base().id)));
        }
    }
    (styles, uids)
}

fn check(doc: &Document, input: &MaterialsEdit) -> Result<Work, Stop> {
    let mut list = doc.materials().to_vec();
    let mut styles = doc.settings().wall_styles.clone();
    let mut work = Work {
        materials: Vec::new(),
        styles: Vec::new(),
        updates: Vec::new(),
        report: MaterialReport::default(),
        material: None,
    };
    let mut assigned: Vec<(Slot, Entity)> = Vec::new();
    match &input.operation {
        MaterialsOperation::Define { name, id, change } => {
            if let Some(words) = material_name_problem(name) {
                return Err(invalid("operation.name", words));
            }
            if change.name.is_some() {
                return Err(invalid(
                    "operation.change.name",
                    "Yeni malzemenin adı “name” alanındadır; change.name verilmez.".into(),
                ));
            }
            let id = match id {
                Some(id) if !material_id_ok(id) => {
                    return Err(invalid(
                        "operation.id",
                        format!("“{id}” malzeme kimliği olamaz: küçük harf, rakam ve “-” ile en çok 64 harf."),
                    ));
                }
                Some(id) => id.clone(),
                None => free_id(&list, name),
            };
            if let Some(why) = taken(&list, &id, name, None) {
                return Err(fail(codes::MATERIAL_EXISTS, why, "operation.name"));
            }
            let m = changed(
                ArchMaterial {
                    id: id.clone(),
                    name: name.clone(),
                    category: Default::default(),
                    quantity: Default::default(),
                    hatch: None,
                    cut: None,
                },
                change,
            );
            list.push(m);
            work.material = Some(id);
        }
        MaterialsOperation::Update { material, change } => {
            let at = find(&list, material, "operation.material")?;
            if change.is_empty() {
                return Err(fail(
                    codes::NOTHING_TO_SET,
                    "Değiştirilecek bir alan verilmedi: ad, kategori, metraj, tarama ya da kesit kalemi verin.".into(),
                    "operation.change",
                ));
            }
            if let Some(name) = &change.name {
                if let Some(words) = material_name_problem(name) {
                    return Err(invalid("operation.change.name", words));
                }
                if let Some(why) = taken(&list, "", name, Some(at)) {
                    return Err(fail(codes::MATERIAL_EXISTS, why, "operation.change.name"));
                }
            }
            list[at] = changed(list[at].clone(), change);
            work.material = Some(list[at].id.clone());
        }
        MaterialsOperation::Remove { material } => {
            let at = find(&list, material, "operation.material")?;
            let id = list[at].id.clone();
            let (in_styles, uids) = bound_to(doc, &id);
            if !in_styles.is_empty() || !uids.is_empty() {
                let mut said = Vec::new();
                if !in_styles.is_empty() {
                    said.push(format!("{} duvar stili ({})", in_styles.len(), in_styles.join(", ")));
                }
                if !uids.is_empty() {
                    said.push(format!("{} duvar ya da döşeme", uids.len()));
                }
                let mut e = error(
                    codes::MATERIAL_IN_USE,
                    format!(
                        "“{}” malzemesine bağlı {} var; silinmedi. Önce onları başka malzemeye bağlayın ya da bağdan ayırın.",
                        list[at].name,
                        said.join(" ve ")
                    ),
                    Some("operation.material".into()),
                );
                e.uids = uids;
                return Err(Stop::failed(e));
            }
            list.remove(at);
            work.material = Some(id);
        }
        MaterialsOperation::Assign { material, targets } => {
            if targets.is_empty() {
                return Err(fail(
                    codes::NOTHING_TO_SET,
                    "Bağlanacak bir şey verilmedi: duvar stili bileşeni, duvar bileşeni ya da döşeme verin.".into(),
                    "operation.targets",
                ));
            }
            let m = match material {
                Some(key) => Some(list[find(&list, key, "operation.material")?].clone()),
                None => None,
            };
            assign(doc, m.as_ref(), targets, &mut styles, &mut assigned)?;
            work.material = m.map(|m| m.id);
        }
        MaterialsOperation::Adopt {} => {
            let mut left_texts = Vec::new();
            adopt(doc, &mut list, &mut styles, &mut assigned, &mut left_texts);
            work.report.left = left_texts;
        }
    }
    if let Some((field, words)) = materials_problem(&list) {
        return Err(invalid(&format!("operation.{field}"), words));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    // The assigned objects first, then every style, wall and slab brought to the table.
    follow(doc, &styles, &list, &assigned, &mut work);
    if matches!(input.operation, MaterialsOperation::Assign { .. } | MaterialsOperation::Adopt {})
        && work.updates.is_empty()
        && work.report.styles.is_empty()
        && list == doc.materials()
    {
        return Err(fail(
            codes::NOTHING_TO_SET,
            "Bağlanacak bir şey değişmiyor: hedefler zaten bu malzemede ya da bağlanacak metin malzeme yok.".into(),
            "operation",
        ));
    }
    work.materials = list;
    Ok(work)
}

fn warnings(report: &MaterialReport) -> Vec<CommandWarning> {
    let mut out = Vec::new();
    if !report.locked.is_empty() {
        out.push(CommandWarning {
            code: codes::MATERIAL_LOCKED.into(),
            message: format!(
                "Malzemeye bağlı {} nesne kilitli katmanda; eski değerleriyle kaldı. Kilidi açıp yeniden kaydedin.",
                report.locked.len()
            ),
            path: Some("report.locked".into()),
        });
    }
    if !report.left.is_empty() {
        out.push(CommandWarning {
            code: codes::MATERIAL_ADOPT_LEFT.into(),
            message: format!(
                "{} malzemesinin bazı bileşenlerinin taraması ya da kalemi farklı; görünüşü değişmesin diye bağlanmadı.",
                report.left.iter().map(|t| format!("“{t}”")).collect::<Vec<_>>().join(", ")
            ),
            path: Some("report.left".into()),
        });
    }
    out
}

pub fn validate(cx: &ExecutionContext<'_>, input: &MaterialsEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(work) => CommandResult::Completed {
            output: (),
            warnings: warnings(&work.report),
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &MaterialsEdit) -> CommandResult<MaterialsEdited> {
    plan_in(cx.doc, input)
}

/// [`plan`] on a document: the Malzemeler window's preview.
pub fn plan_in(doc: &Document, input: &MaterialsEdit) -> CommandResult<MaterialsEdited> {
    match check(doc, input) {
        Ok(work) => CommandResult::Completed {
            warnings: warnings(&work.report),
            output: MaterialsEdited {
                materials: work.materials,
                material: work.material,
                report: work.report,
                revision: doc.revision().to_string(),
            },
        },
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: MaterialsEdit) -> CommandResult<MaterialsEdited> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_MATERIALS_EDIT,
        kentos_commands::CAD_MATERIALS_EDIT_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: MaterialsEdit) -> CommandResult<MaterialsEdited> {
    let work = match check(cx.doc, &input) {
        Ok(work) => work,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    let result = doc.with_style_tables(STEP, |d| {
        d.transact(STEP, |d| {
            d.set_materials(work.materials.clone(), STEP)?;
            if d.settings().wall_styles != work.styles {
                let mut settings = d.settings().clone();
                settings.wall_styles = work.styles.clone();
                d.set_settings(settings);
            }
            if !work.updates.is_empty() {
                d.update_many(work.updates.clone(), STEP);
            }
            Ok::<(), kentos_domain::Refusal>(())
        })
    });
    if let Err(e) = result {
        return CommandResult::Failed {
            error: error(codes::INVALID_MATERIAL, e.0, Some("operation".into())),
        };
    }
    CommandResult::Completed {
        warnings: warnings(&work.report),
        output: MaterialsEdited {
            materials: work.materials,
            material: work.material,
            report: work.report,
            revision: doc.revision().to_string(),
        },
    }
}
