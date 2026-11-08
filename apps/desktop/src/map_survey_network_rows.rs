//! GNSS arşivi's network rows (plan/map-cad.md MAP-03, docs/adr/2920): the baselines `bz-N`, the known points `bn-N`
//! and the accepted network adjustments `da-N` that `map.survey.query` reads, under the panel's marks, as Trimble
//! Business Center's Project Explorer lists a project's GNSS vectors. A baseline's or a known point's row selects its
//! marks' accepted points; its button leaves it out with the toolbar's reason, or takes it back (`map.survey.edit`'s
//! exclude and include take `bz-` and `bn-` ids). An adjustment's row selects the points it made, its button opens
//! Ağ dengelemesi; it is out of date when one of its baselines or known points has been left out since.

use kentos_commands::{MapSurveyTree, SurveyAdjustment, SurveyBaselineRow, SurveyFileState as F, SurveyKnown};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::widget::{Status, StatusItem};

use crate::app::Message;
use crate::map_survey_panel::{Event, glyph, msg};

/// The accepted points' objects of the marks, by their lasting ids.
fn objects_of(tree: &MapSurveyTree, marks: &[&str]) -> Vec<String> {
    tree.points.iter().filter(|p| marks.contains(&p.mark.as_str())).map(|p| p.uid.clone()).collect()
}

/// The three groups, those with rows: Bazlar, Bilinen noktalar, Dengelemeler.
pub(crate) fn groups<'a>(tree: &MapSurveyTree) -> Vec<StatusItem<'a, Message>> {
    let mut out = Vec::new();
    if !tree.baselines.is_empty() {
        let mut group = StatusItem::new("baselines", "Bazlar")
            .icon(icon(Icon::Line).size(14.0))
            .meta(format!("{}", tree.baselines.len()));
        for b in &tree.baselines {
            group = group.push(baseline_item(tree, b));
        }
        out.push(group);
    }
    if !tree.known.is_empty() {
        let mut group = StatusItem::new("known", "Bilinen noktalar")
            .icon(icon(Icon::Target).size(14.0))
            .meta(format!("{}", tree.known.len()));
        for k in &tree.known {
            group = group.push(known_item(tree, k));
        }
        out.push(group);
    }
    if !tree.adjustments.is_empty() {
        let mut group = StatusItem::new("adjustments", "Dengelemeler")
            .icon(icon(glyph("surveyNetwork")).size(14.0))
            .meta(format!("{}", tree.adjustments.len()));
        for a in &tree.adjustments {
            group = group.push(adjustment_item(tree, a));
        }
        out.push(group);
    }
    out
}

/// Three standard deviations in millimetres.
fn mm(s: [f64; 3]) -> String {
    format!("{:.1} / {:.1} / {:.1} mm", s[0] * 1000.0, s[1] * 1000.0, s[2] * 1000.0)
}

/// The button that leaves a record out with the toolbar's reason, or takes it back.
fn out_or_back<'a>(item: StatusItem<'a, Message>, id: &str, excluded: bool) -> StatusItem<'a, Message> {
    if excluded {
        item.action(icon(Icon::Undo).size(13.0), "Geri al: yeniden dengelemeye girebilir", msg(Event::Include(id.to_owned())))
    } else {
        item.action(
            icon(Icon::Close).size(13.0),
            "Dışla: araç çubuğundaki nedenle (kayıt silinmez)",
            msg(Event::Exclude(id.to_owned())),
        )
    }
}

/// A baseline's row: its marks and id, its length faint, its state as a pill (left out, its file missing or changed,
/// in the adjustment, in the archive); its σ, source, file and adjustments in the tip.
fn baseline_item<'a>(tree: &MapSurveyTree, b: &SurveyBaselineRow) -> StatusItem<'a, Message> {
    let file = b.file.as_ref().map(|f| f.state);
    let (tone, words) = match (&b.excluded, file) {
        (Some(_), _) => (Status::Neutral, "Dışlandı"),
        (None, Some(F::Missing)) => (Status::Warning, "Dosyası yok"),
        (None, Some(F::Changed)) => (Status::Warning, "Dosyası değişti"),
        (None, _) if b.adjust => (Status::Info, "Dengelemede"),
        (None, _) => (Status::Neutral, "Arşivde"),
    };
    let mut detail = vec![format!("{} → {}, {:.3} m; σ X/Y/Z {}.", b.from, b.to, b.length, mm(b.sigma))];
    if let Some(why) = &b.excluded {
        detail.push(format!("Dışlama nedeni: {why}."));
    }
    if let Some(s) = &b.source {
        detail.push(format!("Kaynak: {s}."));
    }
    if let Some(f) = &b.file {
        detail.push(format!("Dosyası: {}.", f.path));
    }
    if !b.adjustments.is_empty() {
        detail.push(format!("Dengelemeler: {}.", b.adjustments.join(", ")));
    }
    let item = StatusItem::new(format!("bz:{}", b.id), format!("{} → {} · {}", b.from, b.to, b.id))
        .meta(format!("{:.3} m", b.length))
        .status(tone, words)
        .detail(detail.join(" "))
        .on_press(msg(Event::Objects(objects_of(tree, &[&b.from, &b.to]))));
    out_or_back(item, &b.id, b.excluded.is_some())
}

/// A known point's row: its mark and id, its epoch faint, fixed or weighted (or left out) as a pill; its place, σ,
/// velocity and source in the tip.
fn known_item<'a>(tree: &MapSurveyTree, k: &SurveyKnown) -> StatusItem<'a, Message> {
    let (tone, words) = match (&k.excluded, k.sigma) {
        (Some(_), _) => (Status::Neutral, "Dışlandı"),
        (None, None) => (Status::Info, "Sabit"),
        (None, Some(_)) => (Status::Info, "Ağırlıklı"),
    };
    let mut detail = vec![format!(
        "Enlem {:.9}°, boylam {:.9}°, elipsoit yüksekliği {:.4} m (EPSG:{}).",
        k.lat, k.lon, k.height, k.srid
    )];
    if let Some(s) = k.sigma {
        detail.push(format!("σ kuzey/doğu/yukarı {}.", mm(s)));
    }
    if let Some(v) = k.velocity {
        detail.push(format!("Hız VX/VY/VZ {:.1} / {:.1} / {:.1} mm/yıl.", v[0] * 1000.0, v[1] * 1000.0, v[2] * 1000.0));
    }
    if let Some(s) = &k.source {
        detail.push(format!("Kaynak: {s}."));
    }
    if let Some(why) = &k.excluded {
        detail.push(format!("Dışlama nedeni: {why}."));
    }
    let meta = k.epoch.map_or_else(|| "epoksuz".to_owned(), |e| format!("{e:.4}"));
    let item = StatusItem::new(format!("bn:{}", k.id), format!("{} · {}", k.mark, k.id))
        .meta(meta)
        .status(tone, words)
        .detail(detail.join(" "))
        .on_press(msg(Event::Objects(objects_of(tree, &[&k.mark]))));
    out_or_back(item, &k.id, k.excluded.is_some())
}

/// How an accepted adjustment stands: out of date when a baseline or a known point it used is left out since (those
/// named), the rules failing, else accepted.
pub(crate) fn adjustment_state<'t>(tree: &'t MapSurveyTree, a: &'t SurveyAdjustment) -> (Status, &'static str, Vec<&'t str>) {
    let out_since: Vec<&str> = a
        .baselines
        .iter()
        .filter(|id| tree.baselines.iter().any(|b| &b.id == *id && b.excluded.is_some()))
        .chain(a.known.iter().filter(|id| tree.known.iter().any(|k| &k.id == *id && k.excluded.is_some())))
        .map(String::as_str)
        .collect();
    let (tone, words) = match (out_since.is_empty(), a.rules_passed) {
        (false, _) => (Status::Warning, "Güncel değil"),
        (true, Some(false)) => (Status::Warning, "Sınıf tutmadı"),
        (true, _) => (Status::Info, "Kabul edildi"),
    };
    (tone, words, out_since)
}

/// The objects of the points an adjustment made, by their lasting ids.
pub(crate) fn made_by(tree: &MapSurveyTree, a: &SurveyAdjustment) -> Vec<String> {
    tree.points.iter().filter(|p| p.result.adjustment.as_deref() == Some(a.id.as_str())).map(|p| p.uid.clone()).collect()
}

/// An accepted adjustment's row: its id and class, how many baselines faint; out of date when a baseline or a known
/// point it used is left out since, the rules failing, or accepted; both stages' m0 and degrees of freedom, the frame,
/// the epoch and what the free stage held in the tip. Its row selects the points it made; its button opens Ağ
/// dengelemesi.
fn adjustment_item<'a>(tree: &MapSurveyTree, a: &SurveyAdjustment) -> StatusItem<'a, Message> {
    let (tone, words, out_since) = adjustment_state(tree, a);
    let m0 = |m: Option<f64>| m.map_or_else(|| "—".to_owned(), |v| format!("{v:.3}"));
    let mut detail = vec![format!(
        "Serbest: m0 {}, serbestlik {}; dayalı: m0 {}, serbestlik {}. Tutulan {}; çarpan {}.",
        m0(a.free_m0),
        a.free_dof,
        m0(a.m0),
        a.dof,
        a.hold,
        a.scale
    )];
    detail.push(match a.epoch {
        Some(e) => format!("Çerçeve EPSG:{}, epok {e:.4}.", a.srid),
        None => format!("Çerçeve EPSG:{}.", a.srid),
    });
    if let Some(c) = &a.class {
        detail.push(format!("Sınıf {c}{}.", a.rules.as_ref().map(|r| format!(" ({r})")).unwrap_or_default()));
    }
    if !out_since.is_empty() {
        detail.push(format!("Sonra dışlananlar: {}; noktaları güncel değil, ağı yeniden dengeleyin.", out_since.join(", ")));
    }
    let made = made_by(tree, a);
    let title = match &a.class {
        Some(c) => format!("{} · {c}", a.id),
        None => a.id.clone(),
    };
    StatusItem::new(format!("da:{}", a.id), title)
        .meta(format!("{} baz, {} nokta", a.baselines.len(), made.len()))
        .status(tone, words)
        .detail(detail.join(" "))
        .on_press(msg(Event::Objects(made)))
        .action(icon(glyph("surveyNetwork")).size(13.0), "Ağ dengelemesi…", msg(Event::Network))
}
