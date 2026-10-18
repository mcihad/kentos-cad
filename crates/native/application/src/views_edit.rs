//! `cad.views.edit` v1 (docs/adr/0285 §1): the project's named views. A
//! project setting (`Document::set_settings`: an edit, no undo step); no
//! object moves, and going to a view is the host's camera. The shared cases
//! are `fixtures/commands/v1/cad.views.edit.json`.

use kentos_contracts::{
    CommandResult, CommandWarning, MOST_NAMED_VIEWS, MOST_VIEW_SIZE, NamedView, ProjectSettings,
    VIEW_REACH, ViewsEdit, ViewsEdited, ViewsOp, normal_angle, view_name_ok,
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
    let names: Vec<(&str, &str)> = match &input.edit {
        ViewsOp::Save { name, .. } | ViewsOp::Delete { name } => vec![("edit.name", name)],
        ViewsOp::Rename { name, to } => vec![("edit.name", name), ("edit.to", to)],
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

/// The settings afterwards, the view saved or renamed and the warnings.
type Next = (ProjectSettings, Option<NamedView>, Vec<CommandWarning>);

fn next(doc: &Document, input: &ViewsEdit) -> Result<Next, Stop> {
    let mut s = doc.settings().clone();
    let mut warnings = Vec::new();
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
                let own = kentos_contracts::view_id_of_name(name);
                let taken = |k: &str| s.named_views.iter().any(|v| kentos_contracts::view_key(v) == k);
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
            let entry = NamedView {
                id,
                name: name.to_owned(),
                center: *center,
                width: *width,
                height: *height,
                twist: normal_angle(twist.unwrap_or(0.0)),
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
                s.named_views[at].id = kentos_contracts::view_key(&s.named_views[at]);
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
    };
    Ok((s, view, warnings))
}

fn check(doc: &Document, input: &ViewsEdit) -> Result<Next, Stop> {
    check_input(input)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    next(doc, input)
}

fn answer(
    s: &ProjectSettings,
    view: Option<NamedView>,
    changed: bool,
    revision: String,
) -> ViewsEdited {
    ViewsEdited {
        views: s.named_views.clone(),
        view,
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
        Ok((next, view, warnings)) => {
            let changed = next != *cx.doc.settings();
            CommandResult::Completed {
                output: answer(&next, view, changed, cx.doc.revision().to_string()),
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
        kentos_contracts::CAD_VIEWS_EDIT,
        kentos_contracts::CAD_VIEWS_EDIT_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: ViewsEdit) -> CommandResult<ViewsEdited> {
    let (next, view, warnings) = match check(cx.doc, &input) {
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
            changed,
            cx.doc.revision().to_string(),
        ),
        warnings,
    }
}
