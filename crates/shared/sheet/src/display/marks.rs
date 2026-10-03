//! A map's marks: the scale bar (ADR 0110's look and its 1-2-5 rule: the
//! longest 1, 2 or 5 times a power of ten that fits the frame) and the north
//! arrow (ADR 0110's “K”: half filled, half outlined; grid, true and magnetic
//! north, the meridian convergence from the projection when it is known, the
//! magnetic declination from the World Magnetic Model at the map's centre on
//! the sheet's date or typed by hand; and the Turkish topographic sheets'
//! north diagram, design §8a).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use super::{Ctx, Note, Pen};
use crate::geodesy;
use crate::kinds::*;
use crate::model::Item;
use crate::style::{Stroke, TextStyle};
use crate::text;
use crate::units::*;

/// Metres as a scale bar writes them: whole numbers bare, else to three decimals without trailing zeros.
fn metres_text(v: f64) -> String {
    let s = kentos_geometry_core::display::fixed(v, 3);
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_owned()
    } else {
        s
    }
}

struct Bar {
    /// One segment on the ground, metres.
    length: f64,
    /// One segment on the paper, µm.
    seg: f64,
    /// The labels: position along the bar from its left end (µm) and text.
    labels: Vec<(f64, String)>,
    left_pad: f64,
    right_pad: f64,
}

fn layout_bar(s: &ScaleBarItem, scale: u32, width: f64) -> Option<Bar> {
    let n = f64::from(s.segments) + f64::from(s.left_segments);
    let km = |total: f64| match s.unit {
        ScaleUnit::Km => true,
        ScaleUnit::M => false,
        ScaleUnit::Auto => total >= 1000.0,
    };
    let make = |length: f64| -> Bar {
        let seg = length * 1_000_000.0 / f64::from(scale.max(1));
        let total = length * f64::from(s.segments);
        let in_km = km(total);
        let div = if in_km { 1000.0 } else { 1.0 };
        let mut labels: Vec<(f64, String)> = Vec::new();
        let left = i32::from(s.left_segments);
        for i in 0..=(s.segments as i32 + left) {
            let v = f64::from(i - left) * length / div;
            let mut t = metres_text(v.abs());
            if i == s.segments as i32 + left {
                t.push_str(if in_km { " km" } else { " m" });
            }
            labels.push((f64::from(i) * seg, t));
        }
        let wf = |t: &str| text::text_width(t, &s.text) as f64;
        let left_pad = labels.first().map_or(0.0, |l| wf(&l.1) / 2.0);
        let right_pad = labels.last().map_or(0.0, |l| wf(&l.1) / 2.0);
        // Labels that do not fit side by side give way: the last (with the unit) stays, then zero, then the first, then the rest.
        let last = labels.len().saturating_sub(1);
        let zero = usize::from(s.left_segments);
        let mut order: Vec<usize> = vec![last, zero, 0];
        order.extend(1..last);
        order.dedup();
        let mut kept: Vec<usize> = Vec::new();
        for i in order {
            if kept.contains(&i) {
                continue;
            }
            let (pos, t) = &labels[i];
            let (a, b) = (pos - wf(t) / 2.0, pos + wf(t) / 2.0);
            let clear = kept.iter().all(|&k| {
                let (p2, t2) = &labels[k];
                let (c, d) = (p2 - wf(t2) / 2.0, p2 + wf(t2) / 2.0);
                b + 1_000.0 <= c || d + 1_000.0 <= a
            });
            if clear {
                kept.push(i);
            }
        }
        kept.sort_unstable();
        let labels: Vec<(f64, String)> = kept.into_iter().map(|i| labels[i].clone()).collect();
        Bar {
            length,
            seg,
            labels,
            left_pad,
            right_pad,
        }
    };
    match s.length {
        ScaleBarLength::Fixed(f) => Some(make(f.metres)),
        ScaleBarLength::Auto(_) => {
            for e in (-3..=7).rev() {
                let p = pow10(e);
                for m in [5.0, 2.0, 1.0] {
                    let b = make(m * p);
                    if n * b.seg + b.left_pad + b.right_pad <= width {
                        return Some(b);
                    }
                }
            }
            None
        }
    }
}

fn pow10(e: i32) -> f64 {
    let mut p = 1.0;
    for _ in 0..e.unsigned_abs() {
        p *= 10.0;
    }
    if e < 0 { 1.0 / p } else { p }
}

pub(crate) fn scale_bar(
    ctx: &Ctx,
    it: &Item,
    s: &ScaleBarItem,
    pen: &mut Pen,
    notes: &mut Vec<Note>,
) {
    let c = it.content_rect();
    let map = s.map.as_ref().and_then(|id| ctx.maps.get(id));
    if map.is_none() {
        notes.push(Note {
            item: it.id.clone(),
            code: "broken_link",
            detail: s.map.clone().unwrap_or_default(),
        });
    }
    let scale = map.map(|m| m.scale);
    let ts = &s.text;
    let line_h = (text::ascent(ts) + text::descent(ts)) as f64;
    let cap = text::cap_height(ts) as f64;
    let ratio = scale.map_or_else(
        || crate::expr::missing_mark("harita"),
        |sc| format!("1/{sc}"),
    );
    if s.style == ScaleBarStyle::Numeric {
        let t = if s.show_scale {
            format!("Ölçek {ratio}")
        } else {
            ratio
        };
        let w = text::text_width(&t, ts);
        if w as f64 > f64::from(c.width) + 0.5 || line_h > f64::from(c.height) + 0.5 {
            notes.push(written_overflow(&it.id, "ölçeğin yazısı"));
        }
        let x = f64::from(c.left) + (f64::from(c.width) - w as f64) / 2.0;
        let y = f64::from(c.top) + f64::from(c.height) / 2.0 + cap / 2.0;
        pen.text(&t, [x, y], ts, ts.size, w, 0);
        return;
    }
    let h = f64::from(s.height);
    let gap = 800.0;
    let scale_row = if s.show_scale { gap + line_h } else { 0.0 };
    let total = line_h + gap + h + scale_row;
    let top = f64::from(c.top) + (f64::from(c.height) - total) / 2.0;
    let bar_top = top + line_h + gap;
    let stroke = Stroke::solid(&s.color, s.line_width);
    let Some(scale) = scale else {
        // No map: the bar's frame and a mark that says so, never another map's scale.
        let x0 = f64::from(c.left);
        let w = f64::from(c.width);
        pen.rect(
            RectUm::new(round_um(x0), round_um(bar_top), round_um(w), round_um(h)),
            None,
            Some(&stroke),
            0,
        );
        let t = crate::expr::missing_mark("harita");
        let tw = text::text_width(&t, ts);
        pen.text(
            &t,
            [
                x0 + (w - tw as f64) / 2.0,
                top + line_h - text::descent(ts) as f64,
            ],
            ts,
            ts.size,
            tw,
            0,
        );
        return;
    };
    let Some(bar) = layout_bar(s, scale, f64::from(c.width)) else {
        notes.push(Note {
            item: it.id.clone(),
            code: "scale_bar_fit",
            detail: String::new(),
        });
        return;
    };
    let n = usize::from(s.segments) + usize::from(s.left_segments);
    let bar_w = n as f64 * bar.seg;
    let used = bar_w + bar.left_pad + bar.right_pad;
    let x0 = f64::from(c.left) + (f64::from(c.width) - used) / 2.0 + bar.left_pad;
    let primary = s.color.as_str();
    let secondary = s.secondary.as_str();
    // The segments: the left ones subdivided.
    let mut parts: Vec<(f64, f64)> = Vec::new();
    let left = usize::from(s.left_segments);
    for i in 0..n {
        let a = x0 + i as f64 * bar.seg;
        if i < left && s.subdivisions > 1 {
            let k = f64::from(s.subdivisions);
            for j in 0..s.subdivisions {
                let w = bar.seg / k;
                parts.push((a + f64::from(j) * w, w));
            }
        } else {
            parts.push((a, bar.seg));
        }
    }
    let rect = |x: f64, y: f64, w: f64, hh: f64| {
        RectUm::new(
            round_um(x),
            round_um(y),
            round_um(x + w) - round_um(x),
            round_um(y + hh) - round_um(y),
        )
    };
    match s.style {
        ScaleBarStyle::SingleBox => {
            for (k, (a, w)) in parts.iter().enumerate() {
                let fill = if k % 2 == 0 { primary } else { secondary };
                pen.rect(rect(*a, bar_top, *w, h), Some(fill), None, 0);
            }
            pen.rect(rect(x0, bar_top, bar_w, h), None, Some(&stroke), 0);
        }
        ScaleBarStyle::DoubleBox => {
            let hh = h / 2.0;
            for (k, (a, w)) in parts.iter().enumerate() {
                let (f1, f2) = if k % 2 == 0 {
                    (primary, secondary)
                } else {
                    (secondary, primary)
                };
                pen.rect(rect(*a, bar_top, *w, hh), Some(f1), None, 0);
                pen.rect(rect(*a, bar_top + hh, *w, h - hh), Some(f2), None, 0);
            }
            pen.rect(rect(x0, bar_top, bar_w, h), None, Some(&stroke), 0);
        }
        ScaleBarStyle::Hollow => {
            let mid = bar_top + h / 2.0;
            let mut segs = Vec::new();
            for (k, (a, w)) in parts.iter().enumerate() {
                if k % 2 == 0 {
                    segs.push([[*a, mid], [*a + *w, mid]]);
                }
                segs.push([[*a, bar_top], [*a, bar_top + h]]);
            }
            pen.rect(
                rect(x0, bar_top, bar_w, h),
                Some(secondary),
                Some(&stroke),
                0,
            );
            pen.lines(&segs, &Stroke::solid(primary, s.line_width.max(1) * 2));
        }
        ScaleBarStyle::Ticks => {
            let base = bar_top + h;
            let mut segs = vec![[[x0, base], [x0 + bar_w, base]]];
            for (k, (a, _)) in parts.iter().enumerate() {
                let tall = k == 0 || k == left;
                segs.push([
                    [*a, base],
                    [*a, if tall { bar_top } else { bar_top + h / 2.0 }],
                ]);
            }
            segs.push([[x0 + bar_w, base], [x0 + bar_w, bar_top]]);
            pen.lines(&segs, &stroke);
        }
        ScaleBarStyle::Stepped => {
            let mut pts = Vec::new();
            for (k, (a, w)) in parts.iter().enumerate() {
                let y = if k % 2 == 0 { bar_top } else { bar_top + h };
                pts.push([*a, y]);
                pts.push([*a + *w, y]);
            }
            pen.path(
                &pts,
                false,
                None,
                Some(&Stroke::solid(primary, s.line_width.max(1) * 2)),
            );
        }
        ScaleBarStyle::Numeric => {}
    }
    let base = bar_top - gap - text::descent(ts) as f64;
    for (pos, t) in &bar.labels {
        let w = text::text_width(t, ts);
        pen.text(t, [x0 + pos - w as f64 / 2.0, base], ts, ts.size, w, 0);
    }
    // Taller than its frame, or its scale's line wider: said (the bar's own fit is `scale_bar_fit`).
    let mut over = total > f64::from(c.height) + 0.5;
    if s.show_scale {
        let t = format!("Ölçek 1/{scale}");
        let w = text::text_width(&t, ts);
        over |= w as f64 > f64::from(c.width) + 0.5;
        let y = bar_top + h + gap + text::ascent(ts) as f64;
        pen.text(&t, [x0 + (bar_w - w as f64) / 2.0, y], ts, ts.size, w, 0);
    }
    if over {
        notes.push(written_overflow(&it.id, "ölçek çubuğunun yazıları"));
    }
    let _ = bar.length;
}

/// The convergence at a map's centre: the core's own from the projection, else the host's, else none.
pub(crate) fn convergence(ctx: &Ctx, map: &str) -> Option<f64> {
    let mf = ctx.maps.get(map)?;
    if let (Some(tm), Some(c)) = (ctx.inputs.crs.as_ref().and_then(|c| c.tm), mf.center)
        && let Some(g) = geodesy::tm_inverse(&tm, c.x, c.y)
    {
        return Some(g.convergence);
    }
    ctx.inputs
        .maps
        .iter()
        .find(|m| m.item == map)
        .and_then(|m| m.convergence)
}

/// The magnetic declination a north arrow turns by (design §8a).
pub(crate) enum Declination {
    /// Typed by hand (“elle”), for its year if one is given.
    Hand {
        deg: f64,
        year: Option<u16>,
    },
    /// The magnetic model's at the map's centre on the sheet's date: its name, the date's year
    /// and month (“2026-10”), the decimal year.
    Model {
        deg: f64,
        model: String,
        month: String,
        year: f64,
    },
    /// Not known: the map has no place on the ground (no coordinate system the core inverts), or
    /// the sheet no date.
    NoPlace,
    NoDate,
}

impl Declination {
    fn deg(&self) -> Option<f64> {
        match self {
            Declination::Hand { deg, .. } | Declination::Model { deg, .. } => Some(*deg),
            Declination::NoPlace | Declination::NoDate => None,
        }
    }

    /// “5°41' D (WMM2025, 2026-10)”, “2°10' B (elle, 2024)”, or why it is not known.
    fn text(&self) -> String {
        let side = |d: f64| if d < 0.0 { "B" } else { "D" };
        match self {
            Declination::Hand { deg, year } => format!(
                "{} {} ({})",
                dm(*deg),
                side(*deg),
                year.map_or_else(|| "elle".to_owned(), |y| format!("elle, {y}"))
            ),
            Declination::Model {
                deg, model, month, ..
            } => format!("{} {} ({model}, {month})", dm(*deg), side(*deg)),
            Declination::NoPlace => "bilinmiyor (haritanın konumu yok)".to_owned(),
            Declination::NoDate => "bilinmiyor (paftanın tarihi yok)".to_owned(),
        }
    }
}

/// An angle's size in degrees and minutes, “5°41'” (the sign is the caller's). The minute mark
/// is the apostrophe, as `geodesy::dms` writes it: every drawing face draws it as a narrow tick,
/// where Barlow's prime (U+2032) is 0.6 em wide, nearly half of it empty (“6°19′  D”).
fn dm(deg: f64) -> String {
    let total = libm::round(deg.abs() * 60.0) as i64;
    format!("{}°{:02}'", total / 60, total % 60)
}

/// The declination of a north arrow: typed by hand when it says so (a book of before the model:
/// when one was typed), else the World Magnetic Model's at its map's centre on the sheet's
/// date (`@tarih`: a sheet's or the project's variable of that name, else today's).
pub(crate) fn declination(ctx: &Ctx, n: &NorthArrowItem) -> Declination {
    if n.declination_hand.unwrap_or(n.declination != 0) {
        return Declination::Hand {
            deg: f64::from(n.declination) / 1000.0,
            year: n.declination_year,
        };
    }
    let place = n
        .map
        .as_deref()
        .and_then(|m| ctx.maps.get(m))
        .and_then(|mf| {
            let tm = ctx.inputs.crs.as_ref()?.tm?;
            let c = mf.center?;
            geodesy::tm_inverse(&tm, c.x, c.y)
        });
    let Some(g) = place else {
        return Declination::NoPlace;
    };
    let date = match ctx.scope.lookup("@tarih") {
        Some(crate::model::VarValue::Text(t)) => t.clone(),
        _ => String::new(),
    };
    let (Some(year), Some(info)) = (crate::wmm::decimal_year(&date), crate::wmm::info()) else {
        return Declination::NoDate;
    };
    match crate::wmm::declination(g.lat, g.lon, year) {
        Some(deg) => Declination::Model {
            deg,
            model: info.model,
            month: date.get(..7).unwrap_or_default().to_owned(),
            year,
        },
        None => Declination::NoPlace,
    }
}

/// Where a north arrow's date came from (`@tarih`): a sheet's variable, the project's, or the
/// host's today.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum DateSource {
    Sheet,
    Project,
    Today,
}

/// Where a north arrow's declination comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum DeclinationSource {
    /// Typed by hand (“elle”).
    Hand,
    /// The World Magnetic Model at the map's centre on the date.
    Model,
}

/// Why a north arrow's declination is not known.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum NorthMissing {
    /// The map has no place on the ground the core can work out (no map, no centre, a system that is not TM or UTM).
    NoPlace,
    /// No date (`@tarih`) that reads as an ISO date.
    NoDate,
}

/// What a north arrow shows and from what (design §8a; WASM `northInfo`): the centre of its map
/// on the ground, the convergence there, the declination with its source, and the date it was
/// worked out for with where that date came from. The inspector's lines; a host needs no lat/lon
/// of its own.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct NorthInfo {
    /// The map it points on, when it names one that is on the sheet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub map: Option<String>,
    /// The map's centre, geodetic latitude and longitude in degrees (GRS80/WGS84), when the
    /// project's system is one the core turns back (TM, UTM).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub lat: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub lon: Option<f64>,
    /// From grid north to true north at the centre, degrees (the sign the arrow writes).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub convergence: Option<f64>,
    /// The declination the arrow uses, degrees, east positive; none: not known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub declination: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub source: Option<DeclinationSource>,
    /// The year a typed value is for, when one was given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub hand_year: Option<u16>,
    /// Why the declination is not known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub missing: Option<NorthMissing>,
    /// The model (“WMM2025”) and the decimal years it holds for.
    pub model: String,
    pub valid_from: f64,
    pub valid_until: f64,
    /// The date used (`@tarih`, ISO, as written), and where it came from; none: no date at all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub date_source: Option<DateSource>,
    /// The date as a decimal year (NOAA's count), and whether it is inside the model's years.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub year: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub in_model: Option<bool>,
}

/// [`NorthInfo`] of a north arrow in its sheet's context: what `north_arrow` draws, as data.
pub(crate) fn north_info(ctx: &Ctx, n: &NorthArrowItem) -> NorthInfo {
    let mf = n
        .map
        .as_deref()
        .and_then(|m| ctx.maps.get(m).map(|f| (m, f)));
    let place = mf.and_then(|(_, f)| {
        let tm = ctx.inputs.crs.as_ref()?.tm?;
        let c = f.center?;
        geodesy::tm_inverse(&tm, c.x, c.y)
    });
    let convergence = mf.and_then(|(m, _)| convergence(ctx, m));
    // `@tarih` as `Scope::lookup` finds it: a sheet's variable, then the project's, then today.
    let named = |vars: &[crate::model::Variable]| {
        vars.iter()
            .any(|v| crate::expr::fold(&v.name) == crate::expr::fold("tarih"))
    };
    let date_source = if named(&ctx.scope.sheet) {
        Some(DateSource::Sheet)
    } else if named(&ctx.scope.project) {
        Some(DateSource::Project)
    } else if !ctx.inputs.project.date.trim().is_empty() {
        Some(DateSource::Today)
    } else {
        None
    };
    let date = match ctx.scope.lookup("@tarih") {
        Some(crate::model::VarValue::Text(t)) => Some(t.clone()),
        _ => None,
    };
    let year = date.as_deref().and_then(crate::wmm::decimal_year);
    let info = crate::wmm::info();
    let (declination, source, hand_year, missing) = match declination(ctx, n) {
        Declination::Hand { deg, year } => (Some(deg), Some(DeclinationSource::Hand), year, None),
        Declination::Model { deg, .. } => (Some(deg), Some(DeclinationSource::Model), None, None),
        Declination::NoPlace => (None, None, None, Some(NorthMissing::NoPlace)),
        Declination::NoDate => (None, None, None, Some(NorthMissing::NoDate)),
    };
    NorthInfo {
        map: mf.map(|(m, _)| m.to_owned()),
        lat: place.map(|g| g.lat),
        lon: place.map(|g| g.lon),
        convergence,
        declination,
        source,
        hand_year,
        missing,
        model: info.as_ref().map_or_else(String::new, |i| i.model.clone()),
        valid_from: info.as_ref().map_or(0.0, |i| i.valid_from),
        valid_until: info.as_ref().map_or(0.0, |i| i.valid_until),
        date,
        date_source,
        year,
        in_model: year.map(crate::wmm::valid),
    }
}

fn signed(deg: f64) -> String {
    let s = geodesy::dms(deg, 0);
    if deg < 0.0 && s != "0°00'00\"" {
        format!("−{s}")
    } else {
        format!("+{s}")
    }
}

pub(crate) fn north_arrow(
    ctx: &Ctx,
    it: &Item,
    n: &NorthArrowItem,
    pen: &mut Pen,
    notes: &mut Vec<Note>,
) {
    let c = it.content_rect();
    let mf = n.map.as_ref().and_then(|id| ctx.maps.get(id));
    if mf.is_none() {
        notes.push(Note {
            item: it.id.clone(),
            code: "broken_link",
            detail: n.map.clone().unwrap_or_default(),
        });
    }
    let gamma = n.map.as_deref().and_then(|m| convergence(ctx, m));
    if n.north != NorthKind::Grid && gamma.is_none() && mf.is_some() {
        notes.push(Note {
            item: it.id.clone(),
            code: "convergence_unknown",
            detail: String::new(),
        });
    }
    // The declination, where the arrow or the diagram shows magnetic north.
    let magnetic = n.north == NorthKind::Magnetic || n.style == NorthArrowStyle::Diagram;
    let decl = magnetic.then(|| declination(ctx, n));
    match &decl {
        // A map with no place says so itself (`map_unplaced`).
        Some(Declination::NoPlace) if mf.is_some_and(|m| m.center.is_some()) => notes.push(Note {
            item: it.id.clone(),
            code: "magnetic_no_place",
            detail: String::new(),
        }),
        Some(Declination::NoDate) => notes.push(Note {
            item: it.id.clone(),
            code: "magnetic_no_date",
            detail: String::new(),
        }),
        // With the model's value to the minute as the paper writes it (mdeg): “Sapmayı elle gir”
        // starts from it, as the inspector's switch does.
        Some(Declination::Model {
            year, model, deg, ..
        }) if !crate::wmm::valid(*year) => notes.push(Note {
            item: it.id.clone(),
            code: "magnetic_out_of_model",
            detail: format!(
                "{model}\u{1f}{year:.2}\u{1f}{}",
                round_i64((deg * 60.0).round() / 60.0 * 1000.0)
            ),
        }),
        _ => {}
    }
    // Where the arrow points: the map's grid north, turned to true north by the convergence, to magnetic by the declination.
    let grid = mf.map_or(0, |m| m.grid_north());
    let gamma_m = gamma.map_or(0, |g| round_i64(g * 1000.0));
    let decl_m = decl
        .as_ref()
        .and_then(Declination::deg)
        .map_or(0, |d| round_i64(d * 1000.0));
    if n.style == NorthArrowStyle::Diagram {
        return diagram(it, n, pen, notes, grid, gamma, decl.as_ref());
    }
    let angle = match n.north {
        NorthKind::Grid => i64::from(grid),
        NorthKind::True => i64::from(grid) - gamma_m,
        NorthKind::Magnetic => i64::from(grid) - gamma_m + decl_m,
    };
    // The note's lines under the arrow.
    let note_style = TextStyle {
        size: (n.text.size / 2).max(1_800),
        weight: 400,
        ..n.text.clone()
    };
    let mut note_lines: Vec<String> = Vec::new();
    if n.note {
        note_lines.push(
            match n.north {
                NorthKind::Grid => "Grid kuzeyi",
                NorthKind::True => "Coğrafi kuzey",
                NorthKind::Magnetic => "Manyetik kuzey",
            }
            .to_owned(),
        );
        if let Some(g) = gamma {
            note_lines.push(format!("Yakınsama {}", signed(g)));
        }
        if let Some(d) = &decl {
            note_lines.push(format!("Manyetik sapma {}", d.text()));
        }
    }
    let ts = &n.text;
    let cap = text::cap_height(ts) as f64;
    let gap = cap * 0.35;
    // The note under the arrow: one block, wrapped at spaces, smaller only if it must be (to the
    // legible size); the arrow keeps at least its letter and a short arrow above it (in a roomy
    // frame the note takes only its own height and the arrow the rest).
    let note = (!note_lines.is_empty()).then(|| {
        let arrow_min = cap * 3.0;
        written_block(
            &with_sources_apart(&note_lines, &note_style, c.width),
            &note_style,
            c,
            f64::from(c.height) - arrow_min - gap,
        )
    });
    if note.as_ref().is_some_and(|b| b.overflow) {
        notes.push(written_overflow(&it.id, "kuzey okunun notu"));
    }
    let note_h = note.as_ref().map_or(0.0, |b| b.height as f64);
    let avail_h = f64::from(c.height) - note_h - if note.is_some() { gap } else { 0.0 };
    let avail_w = f64::from(c.width);
    let color = n.color.as_str();
    let cx = f64::from(c.left) + avail_w / 2.0;
    let dir = |d: f64| {
        let (s, k) = sin_cos(sat(angle));
        [d * s, -d * k]
    };
    match n.style {
        NorthArrowStyle::KentosK | NorthArrowStyle::Simple => {
            // ADR 0110's proportions: 12 wide, 22 tall, the notch at 17 from the tip.
            let ah = (avail_h - cap - gap)
                .min(avail_w * 22.0 / 12.0)
                .max(1_000.0);
            let aw = ah * 12.0 / 22.0;
            let total = cap + gap + ah;
            let top = f64::from(c.top) + (avail_h - total) / 2.0;
            let center = [cx, top + cap + gap + ah / 2.0];
            let turn = |p: [f64; 2]| rotate(p, center, sat(angle));
            let tip = turn([center[0], center[1] - ah / 2.0]);
            if n.style == NorthArrowStyle::KentosK {
                let notch = turn([center[0], center[1] - ah / 2.0 + ah * 17.0 / 22.0]);
                let right = turn([center[0] + aw / 2.0, center[1] + ah / 2.0]);
                let left = turn([center[0] - aw / 2.0, center[1] + ah / 2.0]);
                let line = Stroke::solid(color, round_um((aw / 30.0).max(250.0)));
                pen.path(&[tip, right, notch], true, Some(color), Some(&line));
                pen.path(&[tip, left, notch], true, None, Some(&line));
            } else {
                let head = ah * 0.38;
                let base = turn([center[0], center[1] + ah / 2.0]);
                let neck = turn([center[0], center[1] - ah / 2.0 + head]);
                let l = turn([center[0] - aw / 2.0, center[1] - ah / 2.0 + head]);
                let r = turn([center[0] + aw / 2.0, center[1] - ah / 2.0 + head]);
                pen.path(
                    &[base, neck],
                    false,
                    None,
                    Some(&Stroke::solid(color, round_um((aw / 12.0).max(250.0)))),
                );
                pen.path(&[tip, r, l], true, Some(color), None);
            }
            // “K” beyond the tip, upright.
            let d = dir(ah / 2.0 + gap + cap / 2.0);
            let k = "K";
            let w = text::text_width(k, ts);
            pen.text(
                k,
                [
                    center[0] + d[0] - w as f64 / 2.0,
                    center[1] + d[1] + cap / 2.0,
                ],
                ts,
                ts.size,
                w,
                0,
            );
        }
        // Drawn by `diagram` (returned above).
        NorthArrowStyle::Diagram => {}
        NorthArrowStyle::Compass => {
            let letter = cap;
            let radius = ((avail_h.min(avail_w)) / 2.0 - letter * 1.6).max(1_000.0);
            let center = [cx, f64::from(c.top) + avail_h / 2.0];
            let line = Stroke::solid(color, round_um((radius / 40.0).max(180.0)));
            pen.ellipse(center, radius * 0.72, radius * 0.72, None, Some(&line));
            let point = |a: i64, len: f64| {
                let (s, k) = sin_cos(norm_mdeg(i64::from(sat(angle)) + a));
                [center[0] + len * s, center[1] - len * k]
            };
            let narrow = radius * 0.16;
            for (q, filled) in [
                (0, true),
                (90_000, false),
                (180_000, false),
                (270_000, false),
            ] {
                let p = point(q, radius);
                let l = point(q - 45_000, narrow);
                let r = point(q + 45_000, narrow);
                if filled {
                    pen.path(&[p, r, center], true, Some(color), Some(&line));
                    pen.path(&[p, l, center], true, None, Some(&line));
                } else {
                    pen.path(&[p, r, center, l], true, None, Some(&line));
                }
            }
            for (q, t) in [(0, "K"), (90_000, "D"), (180_000, "G"), (270_000, "B")] {
                let p = point(q, radius + letter * 0.9);
                let w = text::text_width(t, ts);
                let small = TextStyle {
                    weight: if q == 0 { ts.weight } else { 400 },
                    ..ts.clone()
                };
                pen.text(
                    t,
                    [p[0] - w as f64 / 2.0, p[1] + cap / 2.0],
                    &small,
                    ts.size,
                    w,
                    0,
                );
            }
        }
    }
    if let Some(b) = &note {
        pen.block(b, &note_style);
    }
}

/// Lines of a north arrow's text as one block: a line too wide for the frame breaks first before
/// its source in brackets, “… 6°19' D” over “(WMM2025, 2026-10)”, so the value stays with its
/// name and the source stays whole; the block's wrapping does the rest.
fn with_sources_apart(lines: &[String], style: &TextStyle, width: Um) -> String {
    let mut out: Vec<&str> = Vec::with_capacity(lines.len() + 1);
    for l in lines {
        match l.rfind(" (") {
            Some(at) if text::text_width(l, style) > i64::from(width) => {
                out.push(&l[..at]);
                out.push(&l[at + 1..]);
            }
            _ => out.push(l),
        }
    }
    out.join("\n")
}

/// A block of text the core writes under a figure (a north arrow's note): centred, at the bottom
/// of the frame, at most `room` tall (see [`written_block_in`]).
fn written_block(text: &str, style: &TextStyle, c: RectUm, room: f64) -> text::Block {
    let room = room.max(0.0);
    written_block_in(
        text,
        style,
        RectUm::new(
            c.left,
            round_um(c.bottom() as f64 - room),
            c.width,
            round_um(room),
        ),
        crate::style::HAlign::Center,
        crate::style::VAlign::Bottom,
    )
}

/// A block of text the core writes in `rect`: each line wrapped at spaces to its width, the size
/// going down only when that is not enough, and not below the legible size (`overflow` then
/// says it still does not fit).
fn written_block_in(
    text: &str,
    style: &TextStyle,
    rect: RectUm,
    align: crate::style::HAlign,
    valign: crate::style::VAlign,
) -> text::Block {
    text::layout_down_to(
        text,
        style,
        &rect,
        &text::Layout {
            align,
            valign,
            line_height: 120,
            wrap: true,
            fit: TextFit::ShrinkToFit,
        },
        text::LEGIBLE_MIN.min(style.size),
    )
}

/// A text the core writes that does not fit its item even at the legible size (`what`: whose).
pub(crate) fn written_overflow(item: &str, what: &str) -> Note {
    Note {
        item: item.to_owned(),
        code: "text_overflow",
        detail: format!("written\u{1f}{what}"),
    }
}

/// The north diagram of the Turkish topographic sheets (design §8a): from one point at the
/// bottom, grid north (GK), true north (CK, a star at its tip) and magnetic north (MK, a half
/// arrowhead), as they lie on the paper (the map's grid north up the map); arcs between them
/// near the point; under it the angles: the convergence (GK–CK), the declination (CK–MK, with
/// its source) and grid to magnetic (GK–MK). Angles of a few degrees are drawn wider, the
/// smallest gap 8°, and the diagram says so.
fn diagram(
    it: &Item,
    n: &NorthArrowItem,
    pen: &mut Pen,
    notes: &mut Vec<Note>,
    grid: Mdeg,
    gamma: Option<f64>,
    decl: Option<&Declination>,
) {
    let c = it.content_rect();
    let color = n.color.as_str();
    let ts = &n.text;
    let small = TextStyle {
        size: (ts.size / 2).max(1_800),
        weight: 400,
        ..ts.clone()
    };
    let gamma_deg = gamma.unwrap_or(0.0);
    let d = decl.and_then(Declination::deg);
    // The three directions from grid north, degrees clockwise on the paper.
    let true_rel = -gamma_deg;
    let mag_rel = d.map(|d| true_rel + d);
    // As drawn: in their order, each at least MIN_GAP from the one before (equal ones stay
    // together), grid north up; the diagram then says its angles are not to scale.
    const MIN_GAP: f64 = 12.0;
    let mut order: Vec<(usize, f64)> = vec![(0, 0.0)];
    if gamma.is_some() {
        order.push((1, true_rel));
    }
    if let Some(m) = mag_rel {
        order.push((2, m));
    }
    order.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    let mut drawn = [0.0_f64; 3];
    let mut schematic = false;
    let mut prev: Option<(f64, f64)> = None;
    for (i, v) in &order {
        let at = match prev {
            None => *v,
            Some((pv, pd)) if v - pv < 1e-9 => pd,
            Some((pv, pd)) => {
                schematic |= v - pv < MIN_GAP;
                pd + (v - pv).max(MIN_GAP)
            }
        };
        drawn[*i] = at;
        prev = Some((*v, at));
    }
    let shift = drawn[0];
    for a in &mut drawn {
        *a -= shift;
    }
    // The text under the diagram.
    let mut lines: Vec<String> = Vec::new();
    if let Some(g) = gamma {
        lines.push(format!(
            "GK–CK yakınsama {}{}",
            if g < 0.0 { "−" } else { "+" },
            dm(g)
        ));
    } else {
        lines.push("GK–CK yakınsama bilinmiyor".to_owned());
    }
    if let Some(d) = decl {
        lines.push(format!("CK–MK manyetik sapma {}", d.text()));
    }
    if let Some(m) = mag_rel {
        lines.push(format!(
            "GK–MK açısı {} {}",
            dm(m),
            if m < 0.0 { "B" } else { "D" }
        ));
    }
    if schematic {
        lines.push("Açılar ölçekli değildir.".to_owned());
    }
    let cap = text::cap_height(ts) as f64;
    let gap = cap * 0.4;
    // The tips' names: small, upright, beyond each tip.
    let tag = TextStyle {
        size: ((f64::from(ts.size) * 0.6) as Um).max(1_800),
        ..ts.clone()
    };
    let tag_cap = text::cap_height(&tag) as f64;
    let tag_w = text::text_width("MK", &tag) as f64;
    // How far the fan opens to each side for a line of length 1 (the map's grid north may turn it).
    let shown = [
        Some(drawn[0]),
        gamma.map(|_| drawn[1]),
        mag_rel.map(|_| drawn[2]),
    ];
    let sines: Vec<f64> = shown
        .iter()
        .flatten()
        .map(|a| (f64::from(grid) / 1000.0 + a).to_radians().sin())
        .collect();
    let left = sines.iter().fold(0.0_f64, |m, s| m.max(-s));
    let right = sines.iter().fold(0.0_f64, |m, s| m.max(*s));
    let reach = |len: f64| len + tag_cap * 1.4;
    // The width the fan takes for a line of `len`, its names included.
    let fan_w = |len: f64| reach(len) * (left + right) + tag_w * 1.2;
    // The longest line that fits a box `w` × `h` (the names above the tips at its top).
    let len_in = |w: f64, h: f64| -> f64 {
        let by_h = h - cap * 1.6;
        let by_w = if left + right > 1e-9 {
            (w - tag_w * 1.2) / (left + right) - tag_cap * 1.4
        } else {
            f64::MAX
        };
        by_h.min(by_w).max(1_000.0)
    };
    // The angles: one block, wrapped at spaces, smaller only as far as it must be (to the legible
    // size). Under the figure in a frame about as tall as it is wide, beside it in a wide one;
    // the other way when the first does not fit; what fits neither way is said.
    let (cw, ch) = (f64::from(c.width), f64::from(c.height));
    let under = || {
        let figure_min = cap * 1.6 + (tag_cap * 5.0).max(5_000.0);
        let room = (ch - figure_min - gap).max(0.0);
        let rect = RectUm::new(
            c.left,
            round_um(c.bottom() as f64 - room),
            c.width,
            round_um(room),
        );
        let block = written_block_in(
            &with_sources_apart(&lines, &small, c.width),
            &small,
            rect,
            crate::style::HAlign::Center,
            crate::style::VAlign::Bottom,
        );
        let fh = (ch - block.height as f64 - gap).max(1_000.0);
        (block, [f64::from(c.left), f64::from(c.top), cw, fh])
    };
    let beside = || {
        let fw = fan_w(ch - cap * 1.6).min(cw * 0.5);
        let tw = round_um((cw - fw - gap).max(1_000.0));
        let rect = RectUm::new(round_um(f64::from(c.left) + fw + gap), c.top, tw, c.height);
        let block = written_block_in(
            &with_sources_apart(&lines, &small, tw),
            &small,
            rect,
            crate::style::HAlign::Left,
            crate::style::VAlign::Middle,
        );
        (block, [f64::from(c.left), f64::from(c.top), fw, ch])
    };
    let wide = cw > ch * 1.2;
    let (mut block, mut fig) = if wide { beside() } else { under() };
    if block.overflow {
        let other = if wide { under() } else { beside() };
        if !other.0.overflow {
            (block, fig) = other;
        }
    }
    if block.overflow {
        notes.push(written_overflow(
            &it.id,
            "kuzey çizelgesinin açıları en küçük okunur boyda da",
        ));
    }
    // The figure in its box: the longest line that fits, the fan centred on what it takes.
    let [fx, fy, fw, fh] = fig;
    let len = len_in(fw, fh);
    let taken = fan_w(len);
    let base = [
        fx + (fw - taken).max(0.0) / 2.0 + tag_w * 0.6 + reach(len) * left,
        fy + fh,
    ];
    let line = Stroke::solid(color, round_um((len / 90.0).max(180.0)));
    let tip_of = |drawn_deg: f64, at: f64| -> [f64; 2] {
        let a = (f64::from(grid) / 1000.0 + drawn_deg).to_radians();
        [base[0] + at * a.sin(), base[1] - at * a.cos()]
    };
    let label = |pen: &mut Pen, t: &str, drawn_deg: f64| {
        let p = tip_of(drawn_deg, len + tag_cap * 1.4);
        let w = text::text_width(t, &tag);
        pen.text(
            t,
            [p[0] - w as f64 / 2.0, p[1] + tag_cap / 2.0],
            &tag,
            tag.size,
            w,
            0,
        );
    };
    // Grid north.
    pen.path(&[base, tip_of(drawn[0], len)], false, None, Some(&line));
    label(pen, "GK", drawn[0]);
    // True north, a five-pointed star at its tip.
    if gamma.is_some() {
        let tip = tip_of(drawn[1], len);
        pen.path(&[base, tip], false, None, Some(&line));
        let r = tag_cap * 0.75;
        let star: Vec<[f64; 2]> = (0..10)
            .map(|i| {
                let a = (f64::from(grid) / 1000.0 + drawn[1] + f64::from(i) * 36.0).to_radians();
                let rr = if i % 2 == 0 { r } else { r * 0.4 };
                [tip[0] + rr * a.sin(), tip[1] - rr * a.cos()]
            })
            .collect();
        pen.path(&star, true, Some(color), None);
        label(pen, "CK", drawn[1]);
    }
    // Magnetic north, half an arrowhead at its tip.
    if mag_rel.is_some() {
        let tip = tip_of(drawn[2], len);
        pen.path(&[base, tip], false, None, Some(&line));
        let back = tip_of(drawn[2], len - tag_cap * 1.6);
        let a = (f64::from(grid) / 1000.0 + drawn[2]).to_radians();
        let side = [
            back[0] + tag_cap * 0.6 * a.cos(),
            back[1] + tag_cap * 0.6 * a.sin(),
        ];
        pen.path(&[tip, side, back], true, Some(color), None);
        label(pen, "MK", drawn[2]);
    }
    // The arcs between them near the point.
    let thin = Stroke::solid(color, round_um((len / 160.0).max(120.0)));
    let arc = |pen: &mut Pen, from: f64, to: f64, r: f64| {
        let steps = 16;
        let pts: Vec<[f64; 2]> = (0..=steps)
            .map(|i| tip_of(from + (to - from) * f64::from(i) / f64::from(steps), r))
            .collect();
        pen.path(&pts, false, None, Some(&thin));
    };
    if gamma.is_some() && (drawn[1] - drawn[0]).abs() > 1e-6 {
        arc(pen, drawn[0], drawn[1], len * 0.32);
    }
    if mag_rel.is_some() && (drawn[2] - drawn[1]).abs() > 1e-6 {
        arc(pen, drawn[1], drawn[2], len * 0.46);
    }
    // The angles under it.
    pen.block(&block, &small);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn angles_are_written_in_degrees_and_minutes() {
        assert_eq!(dm(5.6833), "5°41'");
        assert_eq!(dm(-0.099), "0°06'");
        assert_eq!(dm(2.99999), "3°00'");
    }

    #[test]
    fn scale_bar_labels_are_plain_numbers() {
        assert_eq!(metres_text(20.0), "20");
        assert_eq!(metres_text(0.5), "0.5");
        assert_eq!(metres_text(2.5), "2.5");
        assert_eq!(metres_text(0.25), "0.25");
    }

    #[test]
    fn the_longest_one_two_five_length_that_fits() {
        let mut s = match crate::kinds::default_kind("scaleBar") {
            Some(ItemKind::ScaleBar(s)) => s,
            _ => unreachable!(),
        };
        s.segments = 4;
        // 1/1000, 80 mm: 4 × 20 m (80 mm) leaves no room for the end labels; 4 × 10 m (40 mm) fits.
        let b = layout_bar(&s, 1000, 80_000.0).unwrap();
        assert_eq!(b.length, 10.0);
        // 1/500, 80 mm: 4 × 5 m is 40 mm; 4 × 10 m is 80 mm and does not fit with its labels.
        let b = layout_bar(&s, 500, 80_000.0).unwrap();
        assert_eq!(b.length, 5.0);
        // 1/50 000, 100 mm: 4 × 2 000 m is 160 mm; 4 × 1 000 m is 80 mm and fits, labels in km.
        let b = layout_bar(&s, 50_000, 100_000.0).unwrap();
        assert_eq!(b.length, 1000.0);
        assert_eq!(b.labels.last().unwrap().1, "4 km");
    }
}
