//! `cad.views.edit` v1 (docs/adr/0285 §1): the project's named views. A
//! project setting (`Document::set_settings`: an edit, no undo step); no
//! object moves, and going to a view is the host's camera. The shared cases
//! are `fixtures/commands/v1/cad.views.edit.json`.

use kentos_commands::{
    CommandResult, CommandWarning, MOST_NAMED_VIEWS, MOST_VIEW_SIZE, MOST_VIEW_TEMPLATES, NamedView, ProjectSettings,
    VIEW_REACH, ViewRecipe, ViewTemplate, ViewsEdit, ViewsEdited, ViewsOp, normal_angle, view_name_ok,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

const NON_FINITE: &str =
    "Görünümün sayıları sonlu olmalı; NaN ya da sonsuz bir değer verildi. Sayıyı yeniden verin.";

/// The input's own checks: what needs no document.
fn check_input(input: &ViewsEdit) -> Result<(), Stop> {
    if let ViewsOp::Save {
        center,
        width,
        height,
        twist,
        ..
    } = &input.edit
    {
        for (path, v) in [
            ("edit.center", center.x),
            ("edit.center", center.y),
            ("edit.width", *width),
            ("edit.height", *height),
            ("edit.twist", twist.unwrap_or(0.0)),
        ] {
            if !v.is_finite() {
                return Err(fail(codes::NOT_FINITE, NON_FINITE.into(), path));
            }
        }
        if center.x.abs() > VIEW_REACH || center.y.abs() > VIEW_REACH {
            return Err(fail(
                codes::FAR_POINT,
                "Görünümün ortası çizimden çok uzak: koordinatlar en çok 10 000 km olur. Çizimdeki bir noktayı verin.".into(),
                "edit.center",
            ));
        }
        for (path, v) in [("edit.width", *width), ("edit.height", *height)] {
            if v <= 0.0 || v > MOST_VIEW_SIZE {
                return Err(fail(
                    codes::INVALID_VIEW_SIZE,
                    "Görünümün eni ve boyu 0'dan büyük ve en çok 20 000 km olmalı. Gösterilecek alanın boyunu verin.".into(),
                    path,
                ));
            }
        }
    }
    if let ViewsOp::Recipe {
        recipe: Some(r), ..
    } = &input.edit
    {
        // The version is the command's to write: the input's is ignored.
        let r = ViewRecipe { version: 1, ..r.clone() };
        if let Some(why) = kentos_commands::recipe_problem(&r) {
            return Err(fail(codes::INVALID_RECIPE, format!("{why} Tarifi düzeltip yeniden verin."), "edit.recipe"));
        }
    }
    if let ViewsOp::Template {
        recipe: Some(r), ..
    } = &input.edit
    {
        let r = ViewRecipe { version: 1, ..r.clone() };
        if let Some(why) = kentos_commands::template_problem(&r) {
            return Err(fail(codes::INVALID_RECIPE, format!("{why} Şablonu düzeltip yeniden verin."), "edit.recipe"));
        }
    }
    let names: Vec<(&str, &str)> = match &input.edit {
        ViewsOp::Save { name, .. }
        | ViewsOp::Delete { name }
        | ViewsOp::Recipe { name, .. }
        | ViewsOp::Template { name, .. } => {
            vec![("edit.name", name)]
        }
        ViewsOp::Rename { name, to } | ViewsOp::TemplateRename { name, to } => {
            vec![("edit.name", name), ("edit.to", to)]
        }
    };
    for (path, name) in names {
        if !view_name_ok(name) {
            return Err(fail(
                codes::INVALID_VIEW_NAME,
                "Görünüm adı boş olamaz, en çok 64 karakterdir ve denetim karakteri içermez. Başka bir ad yazın.".into(),
                path,
            ));
        }
    }
    Ok(())
}

fn unknown(name: &str) -> Stop {
    fail(
        codes::UNKNOWN_VIEW,
        format!(
            "“{name}” adlı bir görünüm yok. Adlı görünümlerden birini seçin ya da geçerli görünümü bu adla kaydedin."
        ),
        "edit.name",
    )
}

fn unknown_template(key: &str, path: &str) -> Stop {
    fail(
        codes::UNKNOWN_TEMPLATE,
        format!(
            "“{key}” adlı ya da kimlikli bir görünüş şablonu yok. Görünümler › Şablonlar'dan birini seçin ya da önce şablonu kaydedin."
        ),
        path,
    )
}

/// The settings afterwards, the view saved or renamed, the template kept and the warnings.
type Next = (ProjectSettings, Option<NamedView>, Option<ViewTemplate>, Vec<CommandWarning>);

fn next(doc: &Document, input: &ViewsEdit) -> Result<Next, Stop> {
    let mut s = doc.settings().clone();
    let mut warnings = Vec::new();
    let mut template = None;
    let view = match &input.edit {
        ViewsOp::Save {
            name,
            center,
            width,
            height,
            twist,
        } => {
            let name = name.trim();
            // A view saved over one of its name keeps that one's id (docs/adr/0345 §2).
            let id = s
                .named_views
                .iter()
                .find(|v| v.name == name)
                .map(|v| v.id.clone())
                .unwrap_or_default();
            // A new view whose name gives an id another view keeps (renamed from that name) takes the next free one.
            let id = if id.is_empty() && !s.named_views.iter().any(|v| v.name == name) {
                let own = kentos_commands::view_id_of_name(name);
                let taken = |k: &str| s.named_views.iter().any(|v| kentos_commands::view_key(v) == k);
                if taken(&own) {
                    (2..)
                        .map(|k| format!("{own}-{k}"))
                        .find(|k| !taken(k))
                        .unwrap_or_default()
                } else {
                    String::new()
                }
            } else {
                id
            };
            // Saved over one, the view keeps its recipe (docs/adr/0774 §2): the camera changes, not what it draws.
            let recipe = s
                .named_views
                .iter()
                .find(|v| v.name == name)
                .and_then(|v| v.recipe.clone());
            let entry = NamedView {
                id,
                name: name.to_owned(),
                center: *center,
                width: *width,
                height: *height,
                twist: normal_angle(twist.unwrap_or(0.0)),
                recipe,
            };
            match s.named_views.iter_mut().find(|v| v.name == name) {
                Some(v) => {
                    *v = entry.clone();
                    warnings.push(CommandWarning {
                        code: codes::VIEW_REPLACED.into(),
                        message: format!("“{name}” adlı görünümün yerine yenisi kaydedildi."),
                        path: Some("edit.name".into()),
                    });
                }
                None => {
                    if s.named_views.len() >= MOST_NAMED_VIEWS {
                        return Err(fail(
                            codes::TOO_MANY_VIEWS,
                            "Projede en çok 100 adlı görünüm olur. Kullanmadığınız birini silin ya da var olanın üzerine kaydedin.".into(),
                            "edit.name",
                        ));
                    }
                    s.named_views.push(entry.clone());
                }
            }
            Some(entry)
        }
        ViewsOp::Rename { name, to } => {
            let (name, to) = (name.trim(), to.trim());
            let Some(at) = s.named_views.iter().position(|v| v.name == name) else {
                return Err(unknown(name));
            };
            if to != name && s.named_views.iter().any(|v| v.name == to) {
                return Err(fail(
                    codes::VIEW_EXISTS,
                    format!(
                        "“{to}” adlı başka bir görünüm var. Başka bir ad yazın ya da önce onu silin."
                    ),
                    "edit.to",
                ));
            }
            // The id the old name gave stays the view's (docs/adr/0345 §2): marks pointing at it keep pointing.
            if s.named_views[at].id.is_empty() && to != name {
                s.named_views[at].id = kentos_commands::view_key(&s.named_views[at]);
            }
            s.named_views[at].name = to.to_owned();
            Some(s.named_views[at].clone())
        }
        ViewsOp::Delete { name } => {
            let name = name.trim();
            let before = s.named_views.len();
            s.named_views.retain(|v| v.name != name);
            if s.named_views.len() == before {
                return Err(unknown(name));
            }
            None
        }
        ViewsOp::Recipe { name, recipe } => {
            let name = name.trim();
            let Some(at) = s.named_views.iter().position(|v| v.name == name) else {
                return Err(unknown(name));
            };
            let mut recipe = recipe.clone();
            if let Some(r) = recipe.as_mut() {
                recipe_known(&s, r)?;
                // A template named by its id or its name is kept by its id (docs/adr/0783 §2).
                if let Some(key) = &r.template {
                    let key = key.trim();
                    let Some(t) = s
                        .view_templates
                        .iter()
                        .find(|t| t.id == key)
                        .or_else(|| s.view_templates.iter().find(|t| t.name == key))
                    else {
                        return Err(unknown_template(key, "edit.recipe.template"));
                    };
                    r.template = Some(t.id.clone());
                }
            }
            let was = s.named_views[at].recipe.as_ref().map_or(0, |r| r.version);
            let next = recipe.map(|r| ViewRecipe { version: was + 1, ..r });
            // The same recipe again is no change: its version stays.
            let same = match (&s.named_views[at].recipe, &next) {
                (Some(a), Some(b)) => ViewRecipe { version: 0, ..a.clone() } == ViewRecipe { version: 0, ..b.clone() },
                (None, None) => true,
                _ => false,
            };
            if !same {
                // The id the name gives stays the view's: the maps linked to it keep their link.
                if s.named_views[at].id.is_empty() {
                    s.named_views[at].id = kentos_commands::view_key(&s.named_views[at]);
                }
                s.named_views[at].recipe = next;
            }
            Some(s.named_views[at].clone())
        }
        ViewsOp::Template { name, recipe } => {
            let name = name.trim();
            let at = s.view_templates.iter().position(|t| t.name == name);
            match recipe {
                None => {
                    let Some(at) = at else {
                        return Err(unknown_template(name, "edit.name"));
                    };
                    let gone = s.view_templates.remove(at);
                    // The views following it keep its fields as their own recipe, their version one more.
                    let mut detached = Vec::new();
                    for v in &mut s.named_views {
                        if let Some(r) = v.recipe.as_ref()
                            && r.template.as_deref() == Some(gone.id.as_str())
                        {
                            let own = ViewRecipe {
                                version: r.version + 1,
                                level: r.level.clone(),
                                crop: r.crop,
                                template: None,
                                ..gone.recipe.clone()
                            };
                            v.recipe = Some(own);
                            detached.push(format!("“{}”", v.name));
                        }
                    }
                    if !detached.is_empty() {
                        warnings.push(CommandWarning {
                            code: codes::TEMPLATE_DETACHED.into(),
                            message: format!(
                                "Şablon kaldırıldı; {} görünümü tarifini kendi tarifi olarak tutar.",
                                detached.join(", ")
                            ),
                            path: Some("edit.name".into()),
                        });
                    }
                }
                Some(r) => {
                    recipe_known(&s, r)?;
                    match at {
                        None => {
                            if s.view_templates.len() >= MOST_VIEW_TEMPLATES {
                                return Err(fail(
                                    codes::TOO_MANY_TEMPLATES,
                                    "Projede en çok 50 görünüş şablonu olur. Kullanmadığınız birini kaldırın ya da var olanın üzerine kaydedin.".into(),
                                    "edit.name",
                                ));
                            }
                            let own = kentos_commands::template_id_of_name(name);
                            let taken = |k: &str| s.view_templates.iter().any(|t| t.id == k);
                            let id = if taken(&own) {
                                (2..).map(|k| format!("{own}-{k}")).find(|k| !taken(k)).unwrap_or(own)
                            } else {
                                own
                            };
                            let t = ViewTemplate {
                                id,
                                name: name.to_owned(),
                                recipe: ViewRecipe { version: 1, ..r.clone() },
                            };
                            s.view_templates.push(t.clone());
                            template = Some(t);
                        }
                        Some(at) => {
                            let was = &s.view_templates[at].recipe;
                            let next = ViewRecipe { version: was.version + 1, ..r.clone() };
                            if (ViewRecipe { version: 0, ..was.clone() }) != (ViewRecipe { version: 0, ..next.clone() }) {
                                s.view_templates[at].recipe = next;
                            }
                            template = Some(s.view_templates[at].clone());
                        }
                    }
                }
            }
            None
        }
        ViewsOp::TemplateRename { name, to } => {
            let (name, to) = (name.trim(), to.trim());
            let Some(at) = s.view_templates.iter().position(|t| t.name == name) else {
                return Err(unknown_template(name, "edit.name"));
            };
            if to != name && s.view_templates.iter().any(|t| t.name == to) {
                return Err(fail(
                    codes::TEMPLATE_EXISTS,
                    format!("“{to}” adlı başka bir görünüş şablonu var. Başka bir ad yazın ya da önce onu kaldırın."),
                    "edit.to",
                ));
            }
            s.view_templates[at].name = to.to_owned();
            template = Some(s.view_templates[at].clone());
            None
        }
    };
    Ok((s, view, template, warnings))
}

/// A recipe's level and layer state are the project's (docs/adr/0774 §2).
fn recipe_known(s: &ProjectSettings, r: &ViewRecipe) -> Result<(), Stop> {
    if let Some(l) = &r.level {
        let levels = s.arch.as_ref().map_or(&[][..], |a| a.levels.as_slice());
        if !levels.iter().any(|k| k.id == l.id) {
            return Err(fail(
                codes::LEVEL_NOT_FOUND,
                format!(
                    "Projede “{}” kimlikte kat yok. Proje ayarları › Mimari'deki katlardan birinin kimliğini verin.",
                    l.id
                ),
                "edit.recipe.level.id",
            ));
        }
    }
    if let Some(name) = &r.layer_state
        && !s.layer_states.iter().any(|st| st.name.trim() == name.trim())
    {
        return Err(fail(
            codes::STATE_NOT_FOUND,
            format!("Projede “{name}” adlı katman durumu yok. Katmanlar › Katman durumları'ndan birinin adını verin."),
            "edit.recipe.layerState",
        ));
    }
    Ok(())
}

fn check(doc: &Document, input: &ViewsEdit) -> Result<Next, Stop> {
    check_input(input)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    next(doc, input)
}

fn answer(
    s: &ProjectSettings,
    view: Option<NamedView>,
    template: Option<ViewTemplate>,
    changed: bool,
    revision: String,
) -> ViewsEdited {
    ViewsEdited {
        views: s.named_views.clone(),
        view,
        templates: s.view_templates.clone(),
        template,
        changed,
        revision,
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &ViewsEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would do; nothing is written.
pub fn plan(cx: &ExecutionContext<'_>, input: &ViewsEdit) -> CommandResult<ViewsEdited> {
    match check(cx.doc, input) {
        Ok((next, view, template, warnings)) => {
            let changed = next != *cx.doc.settings();
            CommandResult::Completed {
                output: answer(&next, view, template, changed, cx.doc.revision().to_string()),
                warnings,
            }
        }
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes the settings.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ViewsEdit) -> CommandResult<ViewsEdited> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_VIEWS_EDIT,
        kentos_commands::CAD_VIEWS_EDIT_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: ViewsEdit) -> CommandResult<ViewsEdited> {
    let (next, view, template, warnings) = match check(cx.doc, &input) {
        Ok(next) => next,
        Err(stop) => return stop.into(),
    };
    let changed = next != *cx.doc.settings();
    if changed {
        cx.doc.set_settings(next);
    }
    CommandResult::Completed {
        output: answer(
            cx.doc.settings(),
            view,
            template,
            changed,
            cx.doc.revision().to_string(),
        ),
        warnings,
    }
}
