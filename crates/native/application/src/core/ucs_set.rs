//! `cad.ucs.set` v1 (docs/adr/0261 §2): the drawing's 2D user coordinate
//! system. A project setting (`Document::set_settings`: an edit, no undo
//! step); no object moves. The frame an object gives is the shared core's
//! (`tools::ucs::object_frame`). The shared cases are
//! `fixtures/commands/v1/cad.ucs.set.json`.

use kentos_commands::{
    CommandResult, CommandWarning, MOST_NAMED_UCS, NamedUcs, ProjectSettings, UCS_REACH, UCS_SAME,
    Ucs, UcsChanged, UcsNow, UcsSet, UcsTo, Vec2, ucs_name_ok,
};
use kentos_domain::{Document, Uuid};
use kentos_geometry_core::tools::ucs::object_frame;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// A point and a number of the input, with its path.
type Named<T> = Vec<(&'static str, T)>;

/// The input's points and numbers with their paths, in the order they are checked.
fn numbers(to: &UcsTo) -> (Named<Vec2>, Named<f64>) {
    match to {
        UcsTo::Frame { origin, angle } => {
            (vec![("to.origin", *origin)], vec![("to.angle", *angle)])
        }
        UcsTo::Origin { point } => (vec![("to.point", *point)], vec![]),
        UcsTo::Rotate { angle } => (vec![], vec![("to.angle", *angle)]),
        UcsTo::TwoPoints { origin, x_point } => (
            vec![("to.origin", *origin), ("to.xPoint", *x_point)],
            vec![],
        ),
        UcsTo::Object { at, .. } => (vec![("to.at", *at)], vec![]),
        _ => (vec![], vec![]),
    }
}

const NON_FINITE: &str =
    "UCS'in sayıları sonlu olmalı; NaN ya da sonsuz bir değer verildi. Sayıyı yeniden verin.";

/// The input's own checks: what needs no document.
fn check_input(input: &UcsSet) -> Result<(), Stop> {
    let (points, values) = numbers(&input.to);
    for (path, p) in &points {
        if !p.x.is_finite() || !p.y.is_finite() {
            return Err(fail(codes::NOT_FINITE, NON_FINITE.into(), path));
        }
    }
    for (path, v) in &values {
        if !v.is_finite() {
            return Err(fail(codes::NOT_FINITE, NON_FINITE.into(), path));
        }
    }
    for (path, p) in &points {
        if p.x.abs() > UCS_REACH || p.y.abs() > UCS_REACH {
            return Err(fail(
                codes::FAR_POINT,
                "UCS'in noktası çizimden çok uzak: koordinatlar en çok 10 000 km olur. Çizimdeki bir noktayı verin.".into(),
                path,
            ));
        }
    }
    if let UcsTo::Named { name } | UcsTo::Save { name } | UcsTo::Delete { name } = &input.to
        && !ucs_name_ok(name)
    {
        return Err(fail(
                codes::INVALID_UCS_NAME,
                "UCS adı boş olamaz, en çok 64 karakterdir ve denetim karakteri içermez. Başka bir ad yazın.".into(),
                "to.name",
            ));
    }
    if let UcsTo::TwoPoints { origin, x_point } = &input.to
        && kentos_geometry_core::jsmath::js_hypot(x_point.x - origin.x, x_point.y - origin.y)
            < UCS_SAME
    {
        return Err(fail(
                codes::SAME_POINTS,
                "İki nokta aynı yerde; x ekseninin yönü bulunamadı. İkinci noktayı başlangıçtan uzakta verin.".into(),
                "to.xPoint",
            ));
    }
    if let UcsTo::Object { uid, .. } = &input.to
        && !checks::is_uid_text(uid)
    {
        return Err(fail(
            codes::INVALID_UID,
            format!(
                "“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın."
            ),
            "to.uid",
        ));
    }
    Ok(())
}

fn unknown(name: &str) -> Stop {
    fail(
        codes::UNKNOWN_UCS,
        format!(
            "“{name}” adlı bir UCS yok. Adlı UCS'lerden birini seçin ya da geçerli UCS'i bu adla kaydedin."
        ),
        "to.name",
    )
}

/// What an object's kind is called in a refusal.
fn kind_word(e: &kentos_commands::Entity) -> &'static str {
    use kentos_commands::Entity as E;
    match e {
        E::Dimension(_) => "Ölçü",
        E::Leader(_) => "Kılavuz",
        E::Spline(_) => "Eğri",
        E::Nurbs(_) => "NURBS eğrisi",
        E::Table(_) => "Tablo",
        E::Image(_) => "Resim",
        E::Raster(_) => "Raster",
        E::PointCloud(_) => "Nokta bulutu",
        E::Xline(_) => "Yardımcı çizgi",
        E::Ray(_) => "Işın",
        _ => "Bu",
    }
}

/// The settings afterwards and the warnings.
fn next(doc: &Document, input: &UcsSet) -> Result<(ProjectSettings, Vec<CommandWarning>), Stop> {
    let mut s = doc.settings().clone();
    let now = s.ucs.unwrap_or(Ucs::WORLD);
    let mut warnings = Vec::new();
    let new = match &input.to {
        UcsTo::World => Some(Ucs::WORLD),
        UcsTo::Frame { origin, angle } => Some(Ucs::new(*origin, *angle)),
        UcsTo::Origin { point } => Some(Ucs::new(*point, now.angle)),
        UcsTo::Rotate { angle } => Some(Ucs::new(now.origin, now.angle + angle)),
        UcsTo::TwoPoints { origin, x_point } => Some(Ucs::new(
            *origin,
            kentos_geometry_core::jsmath::atan2(x_point.y - origin.y, x_point.x - origin.x),
        )),
        UcsTo::Object { uid, at } => {
            let found = Uuid::parse_str(uid)
                .ok()
                .and_then(|u| doc.slot_of(u))
                .and_then(|slot| doc.get(slot));
            let Some(e) = found else {
                return Err(fail(
                    codes::ENTITY_NOT_FOUND,
                    format!(
                        "“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."
                    ),
                    "to.uid",
                ));
            };
            let shape = crate::geometry::shape(e);
            let at = kentos_geometry_core::vec2::Vec2::new(at.x, at.y);
            let Some((o, angle)) = object_frame(&shape, at, now.angle) else {
                return Err(fail(
                    codes::OBJECT_WITHOUT_AXIS,
                    format!(
                        "{} nesnesinden UCS kurulamaz. Çizgi, çoklu çizgi, alan, tarama, yay, daire, elips, yazı, blok ya da nokta seçin.",
                        kind_word(e)
                    ),
                    "to.uid",
                ));
            };
            Some(Ucs::new(Vec2 { x: o.x, y: o.y }, angle))
        }
        UcsTo::Named { name } => {
            let name = name.trim();
            let Some(n) = s.named_ucs.iter().find(|n| n.name == name) else {
                return Err(unknown(name));
            };
            Some(n.ucs())
        }
        UcsTo::Save { name } => {
            let name = name.trim();
            let entry = NamedUcs {
                name: name.to_owned(),
                origin: now.origin,
                angle: now.angle,
            };
            match s.named_ucs.iter_mut().find(|n| n.name == name) {
                Some(n) => {
                    *n = entry;
                    warnings.push(CommandWarning {
                        code: codes::UCS_REPLACED.into(),
                        message: format!("“{name}” adlı UCS'in yerine geçerli UCS kaydedildi."),
                        path: Some("to.name".into()),
                    });
                }
                None => {
                    if s.named_ucs.len() >= MOST_NAMED_UCS {
                        return Err(fail(
                            codes::TOO_MANY_UCS,
                            "Projede en çok 100 adlı UCS olur. Kullanmadığınız birini silin ya da var olanın üzerine kaydedin.".into(),
                            "to.name",
                        ));
                    }
                    s.named_ucs.push(entry);
                }
            }
            None
        }
        UcsTo::Delete { name } => {
            let name = name.trim();
            let before = s.named_ucs.len();
            s.named_ucs.retain(|n| n.name != name);
            if s.named_ucs.len() == before {
                return Err(unknown(name));
            }
            None
        }
    };
    if let Some(u) = new {
        s.ucs = (!u.is_world()).then_some(u);
    }
    Ok((s, warnings))
}

fn check(doc: &Document, input: &UcsSet) -> Result<(ProjectSettings, Vec<CommandWarning>), Stop> {
    check_input(input)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    next(doc, input)
}

/// The answer from the settings afterwards.
fn answer(s: &ProjectSettings, changed: bool, revision: String) -> UcsChanged {
    let u = s.ucs.unwrap_or(Ucs::WORLD);
    UcsChanged {
        ucs: UcsNow {
            origin: u.origin,
            angle: u.angle,
            name: s
                .named_ucs
                .iter()
                .find(|n| n.origin == u.origin && n.angle == u.angle)
                .map(|n| n.name.clone()),
        },
        named: s.named_ucs.iter().map(|n| n.name.clone()).collect(),
        changed,
        revision,
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &UcsSet) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would do; nothing is written.
pub fn plan(cx: &ExecutionContext<'_>, input: &UcsSet) -> CommandResult<UcsChanged> {
    match check(cx.doc, input) {
        Ok((next, warnings)) => {
            let changed = next != *cx.doc.settings();
            CommandResult::Completed {
                output: answer(&next, changed, cx.doc.revision().to_string()),
                warnings,
            }
        }
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes the settings.
pub fn execute(cx: &mut ExecutionContext<'_>, input: UcsSet) -> CommandResult<UcsChanged> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_UCS_SET,
        kentos_commands::CAD_UCS_SET_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: UcsSet) -> CommandResult<UcsChanged> {
    let (next, warnings) = match check(cx.doc, &input) {
        Ok(next) => next,
        Err(stop) => return stop.into(),
    };
    let changed = next != *cx.doc.settings();
    if changed {
        cx.doc.set_settings(next);
    }
    CommandResult::Completed {
        output: answer(cx.doc.settings(), changed, cx.doc.revision().to_string()),
        warnings,
    }
}
