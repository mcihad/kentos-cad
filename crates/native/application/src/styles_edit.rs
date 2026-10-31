//! `cad.styles.edit` v1 (docs/adr/0248 §4): one of the project's named text
//! or dimension styles (docs/adr/0183) created, updated, renamed or
//! removed; the kind's styles given out as a `kentos.annotation-styles`
//! file's text or taken in from one by name (docs/adr/0322 §4). The table that results is saved as the Yazı stilleri and Ölçü
//! stilleri windows save theirs ([`crate::style_tables`]): the table is a
//! project setting (an edit, no undo step), the objects that follow are
//! written in one undo step “Yazı stili” or “Ölçü stili”, those on locked
//! layers left and counted. The shared cases are in
//! `fixtures/commands/v1/cad.styles.edit.json`. A wall style's component bound to a project material takes the
//! material's look; one bound to a material the project has not is refused (docs/adr/0791;
//! `cad.styles.edit.materials.json`).

use kentos_commands::{
    ANNOTATION_STYLES_FORMAT, AnnotationStyleKind, CommandResult, DimensionStyleDef, LeaderStyleDef,
    StyleEditOperation, StylesEdit, StylesEdited, TextStyleDef, WallStyleDef, dimension_styles_problem,
    leader_styles_problem, text_styles_problem, wall_styles_problem,
};
use kentos_domain::{Document, Uuid};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::style_tables;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// One table, text or dimension, as the command changes it.
enum Table {
    Text(Vec<TextStyleDef>),
    Dimension(Vec<DimensionStyleDef>),
    Leader(Vec<LeaderStyleDef>),
    Wall(Vec<WallStyleDef>),
}

impl Table {
    fn ids(&self) -> Vec<&str> {
        match self {
            Table::Text(t) => t.iter().map(|s| s.id.as_str()).collect(),
            Table::Dimension(t) => t.iter().map(|s| s.id.as_str()).collect(),
            Table::Leader(t) => t.iter().map(|s| s.id.as_str()).collect(),
            Table::Wall(t) => t.iter().map(|s| s.id.as_str()).collect(),
        }
    }

    fn problem(&self) -> Option<String> {
        match self {
            Table::Text(t) => text_styles_problem(t),
            Table::Dimension(t) => dimension_styles_problem(t),
            Table::Leader(t) => leader_styles_problem(t),
            Table::Wall(t) => wall_styles_problem(t),
        }
    }
}

/// The table the input would leave, the style's id and the input's path of
/// its definition; or why not. A new style without an id is given `fresh`.
/// A wall style's components bound to the project's materials (docs/adr/0791): each bound one takes its material's
/// text, hatch and cut pen (`ArchMaterial::bind`); one bound to a material the project has not refuses the write
/// (`material_not_found`), so a style never names a material that is not there.
fn bind_materials(doc: &Document, s: &mut WallStyleDef, path: &str) -> Result<(), Stop> {
    let table = doc.materials();
    for (i, c) in s.components.iter_mut().enumerate() {
        let Some(id) = c.material_id.clone() else {
            continue;
        };
        match table.iter().find(|m| m.id == id) {
            Some(m) => *c = m.bind(c),
            None => {
                return Err(fail(
                    codes::MATERIAL_NOT_FOUND,
                    format!(
                        "“{}” duvar stilinin {}. bileşeni (“{}”) projede olmayan “{id}” malzemesine bağlı. Önce malzemeyi projeye alın (Malzemeler › Kitaplık) ya da bileşeni bağdan ayırın.",
                        s.name,
                        i + 1,
                        c.name
                    ),
                    &format!("{path}.components[{i}].materialId"),
                ));
            }
        }
    }
    Ok(())
}

fn next_table(doc: &Document, input: &StylesEdit, fresh: &str) -> Result<(Table, String), Stop> {
    use StyleEditOperation as Op;
    let settings = doc.settings();
    let (path, what) = match input.kind {
        AnnotationStyleKind::Text => ("text", "yazı stili"),
        AnnotationStyleKind::Dimension => ("dimension", "ölçü stili"),
        AnnotationStyleKind::Leader => ("leader", "kılavuz stili"),
        AnnotationStyleKind::Wall => ("wall", "duvar stili"),
    };
    let mut table = match input.kind {
        AnnotationStyleKind::Text => Table::Text(settings.text_styles.clone()),
        AnnotationStyleKind::Dimension => Table::Dimension(settings.dimension_styles.clone()),
        AnnotationStyleKind::Leader => Table::Leader(settings.leader_styles.clone()),
        AnnotationStyleKind::Wall => Table::Wall(settings.wall_styles.clone()),
    };
    let not_found = |id: &str, at: &str| {
        fail(
            codes::STYLE_NOT_FOUND,
            format!("“{id}” kimlikli {what} projede yok."),
            at,
        )
    };
    let given_id = match input.operation {
        Op::Create | Op::Update => match input.kind {
            AnnotationStyleKind::Text => input.text.as_ref().map(|s| s.id.clone()),
            AnnotationStyleKind::Dimension => input.dimension.as_ref().map(|s| s.id.clone()),
            AnnotationStyleKind::Leader => input.leader.as_ref().map(|s| s.id.clone()),
            AnnotationStyleKind::Wall => input.wall.as_ref().map(|s| s.id.clone()),
        }
        .unwrap_or_default(),
        Op::Rename | Op::Remove => input.style.clone().unwrap_or_default(),
        Op::Export | Op::Import => String::new(),
    };
    let known = table.ids().contains(&given_id.as_str());
    let id = match input.operation {
        Op::Create if given_id.is_empty() => fresh.to_owned(),
        Op::Create if known => {
            return Err(fail(
                codes::DUPLICATE_STYLE,
                format!(
                    "“{given_id}” kimlikli {what} zaten var; yeni stile başka bir kimlik verin ya da kimliği boş bırakın."
                ),
                &format!("{path}.id"),
            ));
        }
        Op::Update if !known => return Err(not_found(&given_id, &format!("{path}.id"))),
        Op::Rename | Op::Remove if !known => return Err(not_found(&given_id, "style")),
        _ => given_id,
    };
    let name = input.name.as_deref().unwrap_or_default().trim().to_owned();
    match (&mut table, input.operation) {
        (Table::Text(t), Op::Create) => {
            if let Some(mut s) = input.text.clone() {
                s.id.clone_from(&id);
                t.push(s);
            }
        }
        (Table::Dimension(t), Op::Create) => {
            if let Some(mut s) = input.dimension.clone() {
                s.id.clone_from(&id);
                t.push(s);
            }
        }
        (Table::Leader(t), Op::Create) => {
            if let Some(mut s) = input.leader.clone() {
                s.id.clone_from(&id);
                t.push(s);
            }
        }
        (Table::Leader(t), Op::Update) => {
            if let (Some(s), Some(at)) = (input.leader.clone(), t.iter().position(|s| s.id == id)) {
                t[at] = s;
            }
        }
        (Table::Leader(t), Op::Rename) => {
            if let Some(s) = t.iter_mut().find(|s| s.id == id) {
                s.name.clone_from(&name);
            }
        }
        (Table::Leader(t), Op::Remove) => t.retain(|s| s.id != id),
        (Table::Wall(t), Op::Create) => {
            if let Some(mut s) = input.wall.clone() {
                s.id.clone_from(&id);
                bind_materials(doc, &mut s, "wall")?;
                t.push(s);
            }
        }
        (Table::Wall(t), Op::Update) => {
            if let (Some(mut s), Some(at)) = (input.wall.clone(), t.iter().position(|s| s.id == id)) {
                bind_materials(doc, &mut s, "wall")?;
                t[at] = s;
            }
        }
        (Table::Wall(t), Op::Rename) => {
            if let Some(s) = t.iter_mut().find(|s| s.id == id) {
                s.name.clone_from(&name);
            }
        }
        (Table::Wall(t), Op::Remove) => t.retain(|s| s.id != id),
        (Table::Text(t), Op::Update) => {
            if let (Some(s), Some(at)) = (input.text.clone(), t.iter().position(|s| s.id == id)) {
                t[at] = s;
            }
        }
        (Table::Dimension(t), Op::Update) => {
            if let (Some(s), Some(at)) =
                (input.dimension.clone(), t.iter().position(|s| s.id == id))
            {
                t[at] = s;
            }
        }
        (Table::Text(t), Op::Rename) => {
            if let Some(s) = t.iter_mut().find(|s| s.id == id) {
                s.name.clone_from(&name);
            }
        }
        (Table::Dimension(t), Op::Rename) => {
            if let Some(s) = t.iter_mut().find(|s| s.id == id) {
                s.name.clone_from(&name);
            }
        }
        (Table::Text(t), Op::Remove) => t.retain(|s| s.id != id),
        (Table::Dimension(t), Op::Remove) => t.retain(|s| s.id != id),
        (_, Op::Export | Op::Import) => {}
    }
    if let Some(problem) = table.problem() {
        let at = match input.operation {
            Op::Create | Op::Update => path,
            Op::Rename => "name",
            Op::Remove => "style",
            Op::Export | Op::Import => "library",
        };
        let kind = match input.kind {
            AnnotationStyleKind::Text => "Yazı stilleri",
            AnnotationStyleKind::Dimension => "Ölçü stilleri",
            AnnotationStyleKind::Leader => "Kılavuz stilleri",
            AnnotationStyleKind::Wall => "Duvar stilleri",
        };
        return Err(fail(
            codes::INVALID_STYLE,
            format!("{kind}: {problem}."),
            at,
        ));
    }
    // A wall style's walls take it: one its components do not fit, or whose faces it makes undrawable, refuses it.
    if let Table::Wall(t) = &table
        && let Err((code, why)) = style_tables::wall_followers(doc, t)
    {
        return Err(fail(code, why, path));
    }
    Ok((table, id))
}

/// What the command writes: the table that results (none for an export), the style it names, and an export's or an
/// import's lists.
struct Planned {
    table: Option<Table>,
    style: String,
    added: Vec<String>,
    replaced: Vec<String>,
    skipped: Vec<String>,
    library: Option<String>,
}

impl Planned {
    fn of((table, style): (Table, String)) -> Planned {
        Planned {
            table: Some(table),
            style,
            added: Vec::new(),
            replaced: Vec::new(),
            skipped: Vec::new(),
            library: None,
        }
    }
}

fn kind_name(kind: AnnotationStyleKind) -> &'static str {
    match kind {
        AnnotationStyleKind::Text => "text",
        AnnotationStyleKind::Dimension => "dimension",
        AnnotationStyleKind::Leader => "leader",
        AnnotationStyleKind::Wall => "wall",
    }
}

fn invalid_library(why: String) -> Stop {
    fail(
        codes::INVALID_LIBRARY,
        format!(
            "Stil dosyası okunamadı: {why}. KentOS'un stilleri dışa aktarmasının yazdığı bir {ANNOTATION_STYLES_FORMAT} dosyası verin."
        ),
        "library",
    )
}

/// Export: the kind's styles named (all of them when none is), in the table's order, as the file's canonical text.
fn export(doc: &Document, input: &StylesEdit) -> Result<Planned, Stop> {
    let settings = doc.settings();
    let (ids, styles): (Vec<String>, serde_json::Value) = match input.kind {
        AnnotationStyleKind::Text => {
            let all = &settings.text_styles;
            let chosen: Vec<&TextStyleDef> =
                picked(all.iter().map(|s| s.id.as_str()), input, "yazı stili")?
                    .into_iter()
                    .map(|i| &all[i])
                    .collect();
            (
                chosen.iter().map(|s| s.id.clone()).collect(),
                serde_json::json!(chosen),
            )
        }
        AnnotationStyleKind::Dimension => {
            let all = &settings.dimension_styles;
            let chosen: Vec<&DimensionStyleDef> =
                picked(all.iter().map(|s| s.id.as_str()), input, "ölçü stili")?
                    .into_iter()
                    .map(|i| &all[i])
                    .collect();
            (
                chosen.iter().map(|s| s.id.clone()).collect(),
                serde_json::json!(chosen),
            )
        }
        AnnotationStyleKind::Leader => {
            let all = &settings.leader_styles;
            let chosen: Vec<&LeaderStyleDef> =
                picked(all.iter().map(|s| s.id.as_str()), input, "kılavuz stili")?
                    .into_iter()
                    .map(|i| &all[i])
                    .collect();
            (chosen.iter().map(|s| s.id.clone()).collect(), serde_json::json!(chosen))
        }
        AnnotationStyleKind::Wall => {
            let all = &settings.wall_styles;
            let chosen: Vec<&WallStyleDef> = picked(all.iter().map(|s| s.id.as_str()), input, "duvar stili")?
                .into_iter()
                .map(|i| &all[i])
                .collect();
            (chosen.iter().map(|s| s.id.clone()).collect(), serde_json::json!(chosen))
        }
    };
    let file = serde_json::json!({
        "format": ANNOTATION_STYLES_FORMAT,
        "version": 1,
        "kind": kind_name(input.kind),
        "styles": styles,
    });
    Ok(Planned {
        table: None,
        style: ids.first().cloned().unwrap_or_default(),
        added: Vec::new(),
        replaced: Vec::new(),
        skipped: Vec::new(),
        library: Some(kentos_commands::canonical::canonical_json(&file)),
    })
}

/// The places in the table of export's `styles` (every style's when none is given), in the table's order.
fn picked<'a>(
    table: impl Iterator<Item = &'a str>,
    input: &StylesEdit,
    what: &str,
) -> Result<Vec<usize>, Stop> {
    let ids: Vec<&str> = table.collect();
    let Some(wanted) = &input.styles else {
        return Ok((0..ids.len()).collect());
    };
    for (i, id) in wanted.iter().enumerate() {
        if !ids.contains(&id.as_str()) {
            return Err(fail(
                codes::STYLE_NOT_FOUND,
                format!("“{id}” kimlikli {what} projede yok."),
                &format!("styles[{i}]"),
            ));
        }
    }
    Ok((0..ids.len())
        .filter(|i| wanted.iter().any(|w| w == ids[*i]))
        .collect())
}

/// A style the file brings bound to the project's materials: a wall style's ([`bind_materials`]); others have none.
trait Incoming {
    fn bind(&mut self, doc: &Document) -> Result<(), Stop>;
}

impl Incoming for TextStyleDef {
    fn bind(&mut self, _: &Document) -> Result<(), Stop> {
        Ok(())
    }
}

impl Incoming for DimensionStyleDef {
    fn bind(&mut self, _: &Document) -> Result<(), Stop> {
        Ok(())
    }
}

impl Incoming for LeaderStyleDef {
    fn bind(&mut self, _: &Document) -> Result<(), Stop> {
        Ok(())
    }
}

impl Incoming for WallStyleDef {
    fn bind(&mut self, doc: &Document) -> Result<(), Stop> {
        bind_materials(doc, self, "library")
    }
}

fn bind_incoming(doc: &Document, s: &mut impl Incoming) -> Result<(), Stop> {
    s.bind(doc)
}

/// Import: the file's styles by name into the table (docs/adr/0322 §4).
fn import(
    doc: &Document,
    input: &StylesEdit,
    mut fresh: impl FnMut() -> String,
) -> Result<Planned, Stop> {
    let text = input.library.as_deref().unwrap_or_default();
    let file: serde_json::Value =
        serde_json::from_str(text).map_err(|e| invalid_library(format!("JSON değil ({e})")))?;
    if file.get("format").and_then(serde_json::Value::as_str) != Some(ANNOTATION_STYLES_FORMAT) {
        return Err(invalid_library(format!(
            "biçimi {ANNOTATION_STYLES_FORMAT} değil"
        )));
    }
    match file.get("version").and_then(serde_json::Value::as_u64) {
        Some(1) => {}
        Some(v) => {
            return Err(invalid_library(format!(
                "sürümü {v}; bu KentOS 1. sürümü okur"
            )));
        }
        None => return Err(invalid_library("sürümü yok".into())),
    }
    let kind = kind_name(input.kind);
    match file.get("kind").and_then(serde_json::Value::as_str) {
        Some(k) if k == kind => {}
        Some(k) => {
            return Err(invalid_library(format!(
                "dosyanın stilleri “{k}”; istenen “{kind}”"
            )));
        }
        None => return Err(invalid_library("stillerin türü (kind) yok".into())),
    }
    let list = file
        .get("styles")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let replace = input.replace == Some(true);
    let key = |name: &str| name.trim().to_lowercase();
    let mut planned = Planned {
        table: None,
        style: String::new(),
        added: Vec::new(),
        replaced: Vec::new(),
        skipped: Vec::new(),
        library: None,
    };
    // The table with the file's styles merged in: the name's style updated in its place (its id kept) or skipped,
    // a new name added (the file's id when it is free).
    macro_rules! merge {
        ($table:expr, $ty:ty) => {{
            let incoming: Vec<$ty> = serde_json::from_value(list)
                .map_err(|e| invalid_library(format!("stilleri okunmuyor ({e})")))?;
            let mut table = $table.clone();
            for mut s in incoming {
                bind_incoming(doc, &mut s)?;
                match table.iter().position(|t| key(&t.name) == key(&s.name)) {
                    Some(at) if replace => {
                        // Its id and name stay the table's: the file gives the values.
                        s.id.clone_from(&table[at].id);
                        s.name.clone_from(&table[at].name);
                        planned.replaced.push(s.id.clone());
                        table[at] = s;
                    }
                    Some(_) => planned.skipped.push(s.name.trim().to_owned()),
                    None => {
                        if s.id.is_empty() || table.iter().any(|t| t.id == s.id) {
                            s.id = fresh();
                        }
                        planned.added.push(s.id.clone());
                        table.push(s);
                    }
                }
            }
            table
        }};
    }
    let settings = doc.settings();
    let table = match input.kind {
        AnnotationStyleKind::Text => Table::Text(merge!(settings.text_styles, TextStyleDef)),
        AnnotationStyleKind::Dimension => {
            Table::Dimension(merge!(settings.dimension_styles, DimensionStyleDef))
        }
        AnnotationStyleKind::Leader => Table::Leader(merge!(settings.leader_styles, LeaderStyleDef)),
        AnnotationStyleKind::Wall => Table::Wall(merge!(settings.wall_styles, WallStyleDef)),
    };
    if let Some(problem) = table.problem() {
        let what = match input.kind {
            AnnotationStyleKind::Text => "Yazı stilleri",
            AnnotationStyleKind::Dimension => "Ölçü stilleri",
            AnnotationStyleKind::Leader => "Kılavuz stilleri",
            AnnotationStyleKind::Wall => "Duvar stilleri",
        };
        return Err(fail(
            codes::INVALID_STYLE,
            format!("{what}: {problem}."),
            "library",
        ));
    }
    if let Table::Wall(t) = &table
        && let Err((code, why)) = style_tables::wall_followers(doc, t)
    {
        return Err(fail(code, why, "library"));
    }
    planned.style = planned
        .added
        .first()
        .or(planned.replaced.first())
        .cloned()
        .unwrap_or_default();
    planned.table = Some(table);
    Ok(planned)
}

fn check_input(input: &StylesEdit) -> Result<(), Stop> {
    use StyleEditOperation as Op;
    let op = input.operation;
    let has = match input.kind {
        AnnotationStyleKind::Text => input.text.is_some(),
        AnnotationStyleKind::Dimension => input.dimension.is_some(),
        AnnotationStyleKind::Leader => input.leader.is_some(),
        AnnotationStyleKind::Wall => input.wall.is_some(),
    };
    if matches!(op, Op::Create | Op::Update) && !has {
        let path = match input.kind {
            AnnotationStyleKind::Text => "text",
            AnnotationStyleKind::Dimension => "dimension",
            AnnotationStyleKind::Leader => "leader",
            AnnotationStyleKind::Wall => "wall",
        };
        return Err(fail(
            codes::NO_DEFINITION,
            format!("Stilin tanımını {path} ile verin."),
            path,
        ));
    }
    if matches!(op, Op::Rename | Op::Remove) && input.style.is_none() {
        return Err(fail(
            codes::NO_STYLE,
            "Stili kimliğiyle verin (style).".into(),
            "style",
        ));
    }
    if op == Op::Rename && input.name.as_deref().is_none_or(checks::is_blank) {
        return Err(fail(
            codes::EMPTY_NAME,
            "Stilin adı boş olamaz; bir ad yazın.".into(),
            "name",
        ));
    }
    if op == Op::Import && input.library.as_deref().is_none_or(checks::is_blank) {
        return Err(fail(
            codes::NO_LIBRARY,
            "Alınacak stil dosyasının metnini (library) verin.".into(),
            "library",
        ));
    }
    Ok(())
}

fn check(doc: &Document, input: &StylesEdit, fresh: &str) -> Result<Planned, Stop> {
    check_input(input)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    match input.operation {
        StyleEditOperation::Export => export(doc, input),
        StyleEditOperation::Import => import(doc, input, || Uuid::now_v7().to_string()),
        _ => next_table(doc, input, fresh).map(Planned::of),
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &StylesEdit) -> CommandResult<()> {
    match check(cx.doc, input, &Uuid::now_v7().to_string()) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would do, on a reading copy; the drawing is left as it is.
/// A new style without an id is shown with an id of its own that execute
/// will not give again.
pub fn plan(cx: &ExecutionContext<'_>, input: &StylesEdit) -> CommandResult<StylesEdited> {
    let mut copy = cx.doc.reading_copy();
    match check(&copy, input, &Uuid::now_v7().to_string()) {
        Ok(planned) => {
            let mut output = write(&mut copy, planned);
            output.revision = cx.doc.revision().to_string();
            CommandResult::Completed {
                output,
                warnings: Vec::new(),
            }
        }
        Err(stop) => stop.into(),
    }
}

/// Saves the table, then the objects that follow, in one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: StylesEdit) -> CommandResult<StylesEdited> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_STYLES_EDIT,
        kentos_commands::CAD_STYLES_EDIT_VERSION,
        input,
        write_execute,
    )
}

/// [`execute`]'s write; `call::recorded` runs it as one step of the
/// document, all or nothing, and journals it (docs/adr/0253 §1, §3).
fn write_execute(cx: &mut ExecutionContext<'_>, input: StylesEdit) -> CommandResult<StylesEdited> {
    let fresh = Uuid::now_v7().to_string();
    match check(cx.doc, &input, &fresh) {
        Ok(planned) => CommandResult::Completed {
            output: write(cx.doc, planned),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The table and the objects that follow it, in one undo step: the table's change is recorded with them, so
/// undo gives back a renamed style too (docs/adr/0699).
fn write(doc: &mut Document, planned: Planned) -> StylesEdited {
    let before = doc.revision();
    let table = planned.table;
    let label = match &table {
        Some(Table::Text(_)) => "Yazı stili",
        Some(Table::Dimension(_)) => "Ölçü stili",
        Some(Table::Leader(_)) => "Kılavuz stili",
        Some(Table::Wall(_)) => "Duvar stili",
        None => "",
    };
    let (changed, locked) = doc.with_style_tables(label, |doc| match table {
        Some(Table::Text(t)) => {
            let saved = style_tables::save_text_styles(doc, t);
            (saved.changed, saved.locked)
        }
        Some(Table::Dimension(t)) => {
            let saved = style_tables::save_dimension_styles(doc, t);
            (saved.changed, saved.locked)
        }
        Some(Table::Leader(t)) => {
            let saved = style_tables::save_leader_styles(doc, t);
            (saved.changed, saved.locked)
        }
        Some(Table::Wall(t)) => {
            let saved = style_tables::save_wall_styles(doc, t);
            (saved.changed, saved.locked)
        }
        None => (0, 0),
    });
    StylesEdited {
        style: planned.style,
        changed: doc.revision() != before,
        followed: count(changed),
        locked: count(locked),
        added: planned.added,
        replaced: planned.replaced,
        skipped: planned.skipped,
        library: planned.library,
        revision: doc.revision().to_string(),
    }
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}
