//! What the wall commands share (docs/adr/0339 §2, §5): a wall style found by its id or name (the project's, else
//! KentOS's own, then taken into the project), a wall's fields made from what the user gave and checked as the file
//! checks them, and whether its faces can be drawn.

use kentos_contracts::{
    ArchInfo, DEFAULT_WALL_WIDTH, NewWall, Vec2, WallCaps, WallFields, WallJustify, WallStyleDef,
    builtin_wall_styles, fixed_width,
};
use kentos_domain::Document;
use kentos_geometry_core::Vec2 as CoreVec2;

use crate::checks::{self, Stop, error};
use crate::codes;

/// A wall style the input names: the project's (by its id, then its name, case aside), else KentOS's own (by its id,
/// then its name); whether it is KentOS's and not yet the project's.
pub fn find_style(doc: &Document, key: &str) -> Option<(WallStyleDef, bool)> {
    let folded = key.trim().to_lowercase();
    let by = |list: &[WallStyleDef]| {
        list.iter()
            .find(|s| s.id == key)
            .or_else(|| list.iter().find(|s| s.name.to_lowercase() == folded))
            .cloned()
    };
    if let Some(s) = by(&doc.settings().wall_styles) {
        return Some((s, false));
    }
    by(&builtin_wall_styles()).map(|s| (s, true))
}

/// `style_not_found` for `key` at `path`.
pub(crate) fn no_style(doc: &Document, key: &str, path: &str) -> Stop {
    let mut names: Vec<String> = doc
        .settings()
        .wall_styles
        .iter()
        .map(|s| s.name.clone())
        .collect();
    for s in builtin_wall_styles() {
        if !names.contains(&s.name) {
            names.push(s.name);
        }
    }
    Stop::failed(error(
        codes::STYLE_NOT_FOUND,
        format!(
            "“{key}” adında ya da kimliğinde duvar stili yok. Var olanlar: {}.",
            names.join(", ")
        ),
        Some(path.into()),
    ))
}

/// The path's points as the core takes them.
pub fn core_points(pts: &[Vec2]) -> Vec<CoreVec2> {
    pts.iter().map(|p| CoreVec2::new(p.x, p.y)).collect()
}

/// `wall_collapses` when a face of `w` cannot be drawn (an arc tighter than it), at `path`.
pub(crate) fn drawable(w: &WallFields, path: &str) -> Result<(), Stop> {
    let spec = kentos_formats::walls::spec(w);
    kentos_geometry_core::geom::wall::lines(&core_points(&w.pts), w.bulges.as_deref(), &spec)
        .map(|_| ())
        .map_err(|why| Stop::failed(error(codes::WALL_COLLAPSES, why, Some(path.into()))))
}

/// `level_not_found` when the wall names a level the project does not have.
pub(crate) fn level_known(doc: &Document, arch: &ArchInfo, path: &str) -> Result<(), Stop> {
    let levels = doc
        .settings()
        .arch
        .as_ref()
        .map(|a| a.levels.as_slice())
        .unwrap_or_default();
    match kentos_contracts::level_problem(arch.level.as_deref(), levels) {
        Some(why) => Err(Stop::failed(error(
            codes::LEVEL_NOT_FOUND,
            why,
            Some(path.into()),
        ))),
        None => Ok(()),
    }
}

/// `invalid_wall` for a wall's problem, its field under `prefix`.
pub(crate) fn valid(w: &WallFields, prefix: &str) -> Result<(), Stop> {
    match w.problem() {
        Some((field, why)) => Err(Stop::failed(error(
            codes::INVALID_WALL,
            why,
            Some(format!("{prefix}.{field}")),
        ))),
        None => Ok(()),
    }
}

/// The finite numbers and distinct vertices of a path as given (one bulge per edge), at `prefix`.
pub(crate) fn path_numbers(
    pts: &[Vec2],
    bulges: Option<&[f64]>,
    closed: bool,
    prefix: &str,
) -> Result<(), Stop> {
    for (i, p) in pts.iter().enumerate() {
        checks::point(
            *p,
            &format!("{}. noktanın", i + 1),
            &format!("{prefix}.pts[{i}]"),
        )?;
    }
    if let Some(b) = bulges {
        for (i, x) in b.iter().enumerate() {
            checks::finite(
                *x,
                &format!("{}. büküm", i + 1),
                "Sonlu bir sayı verin.",
                &format!("{prefix}.bulges[{i}]"),
            )?;
        }
        let edges = if closed {
            pts.len()
        } else {
            pts.len().saturating_sub(1)
        };
        if b.len() != edges {
            return Err(Stop::failed(error(
                codes::INVALID_WALL,
                format!(
                    "Her kenarın bir bükümü olur: {} kenar, {} büküm (açık yolda noktalardan bir eksik, kapalıda nokta kadar).",
                    edges,
                    b.len()
                ),
                Some(format!("{prefix}.bulges")),
            )));
        }
    }
    let n = pts.len();
    let edges = if closed { n } else { n.saturating_sub(1) };
    for i in 0..edges {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        if a.x == b.x && a.y == b.y && n > 1 {
            return Err(Stop::failed(error(
                codes::COINCIDENT_POINTS,
                format!(
                    "{}. ve {}. nokta aynı yerde: duvarın kenarı uzunluksuz olur. Yinelenen noktayı çıkarın.",
                    i + 1,
                    (i + 1) % n + 1
                ),
                Some(format!("{prefix}.pts[{}]", (i + 1) % n)),
            )));
        }
    }
    Ok(())
}

/// The fields a wall of `style` (or none) takes from it: its components, ends and fill, and its width (`width`, else
/// the components' sum when they fix it, else the style's new width; a width they do not fit is the wall's problem).
pub fn styled(
    style: Option<&WallStyleDef>,
    width: Option<f64>,
) -> (
    f64,
    Vec<kentos_contracts::WallComponent>,
    WallCaps,
    Option<String>,
) {
    match style {
        Some(s) => (
            width
                .or_else(|| fixed_width(&s.components))
                .unwrap_or_else(|| s.new_width()),
            s.components.clone(),
            s.caps,
            s.fill.clone(),
        ),
        None => (
            width.unwrap_or(DEFAULT_WALL_WIDTH),
            Vec::new(),
            WallCaps::Closed,
            None,
        ),
    }
}

/// A wall as the user gave it, checked: its numbers, its style, its fields as the file checks them, its faces, its
/// level; KentOS's styles it takes are put in `added` (once each).
pub(crate) fn build(
    doc: &Document,
    w: &NewWall,
    prefix: &str,
    added: &mut Vec<WallStyleDef>,
) -> Result<WallFields, Stop> {
    path_numbers(&w.pts, w.bulges.as_deref(), w.closed, prefix)?;
    for (v, what, name) in [
        (w.width, "width", "Genişlik"),
        (w.offset, "offset", "Öteleme"),
        (w.height, "height", "Yükseklik"),
    ] {
        if let Some(x) = v {
            checks::finite(
                x,
                name,
                "Sonlu bir sayı verin.",
                &format!("{prefix}.{what}"),
            )?;
        }
    }
    let style = match &w.style {
        Some(key) => {
            let (s, builtin) = find_style(doc, key)
                .ok_or_else(|| no_style(doc, key, &format!("{prefix}.style")))?;
            if builtin && !added.iter().any(|a| a.id == s.id) {
                added.push(s.clone());
            }
            Some(s)
        }
        None => None,
    };
    let (width, components, caps, fill) = styled(style.as_ref(), w.width);
    // The document keeps one bulge per vertex: an open path's last is 0.
    let bulges = w
        .bulges
        .clone()
        .filter(|b| b.iter().any(|x| *x != 0.0))
        .map(|mut b| {
            if !w.closed {
                b.push(0.0);
            }
            b
        });
    let wall = WallFields {
        pts: w.pts.clone(),
        bulges,
        closed: w.closed,
        width,
        justify: w.justify.unwrap_or(WallJustify::Center),
        offset: w.offset.unwrap_or(0.0),
        style: style.as_ref().map(|s| s.id.clone()),
        components,
        caps: w.caps.unwrap_or(caps),
        fill,
        height: w.height,
        arch: w.arch.clone().unwrap_or_default(),
    };
    valid(&wall, prefix)?;
    drawable(&wall, &format!("{prefix}.pts"))?;
    level_known(doc, &wall.arch, &format!("{prefix}.arch.level"))?;
    Ok(wall)
}

/// The project's settings with KentOS's styles `added` taken in (none: unchanged).
pub fn take_styles(doc: &mut Document, added: &[WallStyleDef]) {
    if added.is_empty() {
        return;
    }
    let mut settings = doc.settings().clone();
    for s in added {
        if settings.wall_styles.iter().any(|w| w.id == s.id) {
            continue;
        }
        // A project style of the same name and another id keeps its name: KentOS's comes in marked.
        let mut s = s.clone();
        if settings
            .wall_styles
            .iter()
            .any(|w| w.name.to_lowercase() == s.name.to_lowercase())
        {
            s.name = format!("{} (KentOS)", s.name);
        }
        settings.wall_styles.push(s);
    }
    doc.set_settings(settings);
}

/// The view the project's walls are drawn at (docs/adr/0339 §4): the detail Proje ayarları › Mimari says, else the
/// drawing scale's (`WallDetail::at_scale`), and that scale for the hatches.
pub fn view_of(
    settings: &kentos_contracts::ProjectSettings,
) -> kentos_geometry_core::geom::wall::View {
    view_at(settings, settings.plot_scale)
}

/// The view a sheet's map at 1:`scale` draws the walls at: the settings' detail, else that scale's.
pub fn view_at(
    settings: &kentos_contracts::ProjectSettings,
    scale: f64,
) -> kentos_geometry_core::geom::wall::View {
    let detail = match &settings.arch {
        Some(arch) => arch.detail_at(scale),
        None => kentos_contracts::WallDetail::at_scale(scale),
    };
    kentos_geometry_core::geom::wall::View {
        detail: kentos_formats::walls::detail(detail),
        scale,
    }
}
