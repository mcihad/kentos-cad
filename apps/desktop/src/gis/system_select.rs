//! A coordinate system chosen from the registry (docs/adr/9112 §3): a searchable list whose first row is the default
//! its window names (a data layer's own system, the project's), then the project's system when it is not the default,
//! then every other system of the registry by datum, as Harita sistemi lists them. Searching takes a part of the name,
//! the datum or the EPSG code.

use iced::Element;
use kentos_ui::widget::select::{Choice, Select};

/// The rows: the system each stands for (none: the default) and its words.
pub(crate) fn rows(default_label: String, default_srid: Option<u32>, project: u32) -> Vec<(Option<u32>, String)> {
    let mut rows = vec![(None, default_label)];
    if project > 0
        && default_srid != Some(project)
        && let Some(s) = kentos_project::crs::system(project)
    {
        rows.push((Some(project), format!("Projenin sistemi · {} (EPSG:{project})", s.name)));
    }
    for (datum, systems) in crate::second_crs::choices(project) {
        for s in systems.into_iter().filter(|s| Some(s.srid) != default_srid) {
            rows.push((Some(s.srid), format!("{datum} · {} (EPSG:{})", s.name, s.srid)));
        }
    }
    rows
}

/// The list, the chosen row shown (the default when none is chosen or the chosen is not listed).
pub(crate) fn select<'a, Message: Clone + 'a>(
    rows: Vec<(Option<u32>, String)>,
    chosen: Option<u32>,
    on_pick: impl Fn(Option<u32>) -> Message + 'a,
) -> Element<'a, Message> {
    let at = rows.iter().position(|(s, _)| *s == chosen).unwrap_or(0);
    let ids: Vec<Option<u32>> = rows.iter().map(|(s, _)| *s).collect();
    Select::new(rows.into_iter().map(|(_, n)| Choice::new(n)), Some(at), move |i| on_pick(ids.get(i).copied().flatten()))
        .placeholder("Sistem ara: ad, datum ya da EPSG kodu")
        .into()
}
