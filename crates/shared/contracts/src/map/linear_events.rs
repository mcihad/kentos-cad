//! Olayları rotalarına bağla (docs/adr/0569): a project's links of an event layer to its routes. An event object
//! carries its route's id and its measures in its own attributes (docs/adr/0224, 0526); a link names where the routes
//! are and which attributes hold the id, the measures and the offset, so that every write that changes a route (its
//! geometry, its M, its id) or an event (its attributes) places the layer's events again, in the same undo step.
//! A project setting (`.kcad` schema 97, `linearEvents`): what follows what, never the places.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// The most links a project holds, and the most route layers a link names.
pub const MOST_EVENT_LINKS: usize = 50;
pub const MOST_EVENT_ROUTE_LAYERS: usize = 16;
/// How far (m) an event may lie from the place its route gives it before a write moves it.
pub const DEFAULT_EVENT_TOLERANCE: f64 = 0.001;

/// What an event object is: a point at a measure, or a line between two.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum EventLinkKind {
    /// A point at one measure (`measure`).
    Point,
    /// A line or a polyline from one measure to another (`from`, `to`).
    Line,
}

/// Which side of the route a positive offset lies on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum EventSide {
    /// The right of the measure's direction.
    #[default]
    Right,
    /// The left of it.
    Left,
}

impl EventSide {
    pub fn is_default(&self) -> bool {
        *self == EventSide::Right
    }
}

/// A link: the events of one layer on the routes of some others.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EventLink {
    /// Letters, digits, `-`, `_`, `.`; 1–64; unique in the project.
    pub id: String,
    /// What the windows show; trimmed, 1–64 characters.
    pub name: String,
    /// The event layer's id: the objects it holds are the events (points, or lines and polylines).
    pub events: String,
    pub kind: EventLinkKind,
    /// The route layers' ids (1–16, none twice, none the event layer).
    pub routes: Vec<String>,
    /// The routes' id field: a route is the first object of an id.
    pub route_field: String,
    /// The events' attribute naming their route.
    pub event_route: String,
    /// A point event's measure attribute (`point` only, and then required).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub measure: Option<String>,
    /// A line event's from and to measure attributes (`line` only, and then both required).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub to: Option<String>,
    /// The attribute of the offset from the route (m, perpendicular); none: the event is on the route.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub offset: Option<String>,
    /// The side a positive offset lies on; written only when not `right`.
    #[serde(default, skip_serializing_if = "EventSide::is_default")]
    #[cfg_attr(feature = "ts", ts(as = "Option<EventSide>", optional))]
    pub side: EventSide,
    /// How far (m) an event may lie from its route's place before a write moves it; none: 0,001.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub tolerance: Option<f64>,
}

impl EventLink {
    /// The link's tolerance in metres.
    pub fn tolerance_m(&self) -> f64 {
        self.tolerance.unwrap_or(DEFAULT_EVENT_TOLERANCE)
    }

    /// Every layer the link reads or writes: the event layer, then the routes'.
    pub fn layers(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.events.as_str()).chain(self.routes.iter().map(String::as_str))
    }
}

/// What is wrong with a link by its shape (no drawing needed); none when it holds.
pub fn event_link_problem(l: &EventLink) -> Option<String> {
    if !crate::form_id_ok(&l.id) {
        return Some(format!(
            "“{}” bir bağ kimliği olamaz: 1–64 harf, rakam, -, _ ya da . olmalı.",
            l.id
        ));
    }
    if !crate::form_name_ok(&l.name) {
        return Some(format!(
            "“{}” bağının adı boş, baş ya da sonda boşluklu, 64 karakterden uzun ya da denetim karakterli.",
            l.id
        ));
    }
    if l.events.is_empty() {
        return Some(format!("“{}” bağının olay katmanı yazılmamış.", l.name));
    }
    if l.routes.is_empty() || l.routes.len() > MOST_EVENT_ROUTE_LAYERS {
        return Some(format!(
            "“{}” bağının 1–{MOST_EVENT_ROUTE_LAYERS} rota katmanı olmalı; {} verildi.",
            l.name,
            l.routes.len()
        ));
    }
    for (i, r) in l.routes.iter().enumerate() {
        if r.is_empty() {
            return Some(format!(
                "“{}” bağının rota katmanlarından biri boş.",
                l.name
            ));
        }
        if r == &l.events {
            return Some(format!(
                "“{}” bağında olay katmanı rota katmanı da olamaz.",
                l.name
            ));
        }
        if l.routes[..i].contains(r) {
            return Some(format!(
                "“{}” bağında “{r}” rota katmanı iki kez yazılmış.",
                l.name
            ));
        }
    }
    for (what, value) in [
        ("rota kimliği alanı", Some(&l.route_field)),
        ("olayın rota alanı", Some(&l.event_route)),
        ("ölçü alanı", l.measure.as_ref()),
        ("başlangıç ölçüsü alanı", l.from.as_ref()),
        ("bitiş ölçüsü alanı", l.to.as_ref()),
        ("öteleme alanı", l.offset.as_ref()),
    ] {
        if value.is_some_and(String::is_empty) {
            return Some(format!("“{}” bağının {what} boş.", l.name));
        }
    }
    match l.kind {
        EventLinkKind::Point => {
            if l.measure.is_none() {
                return Some(format!(
                    "“{}” nokta bağının ölçü alanı (measure) yazılmamış.",
                    l.name
                ));
            }
            if l.from.is_some() || l.to.is_some() {
                return Some(format!(
                    "“{}” nokta bağının başlangıç ve bitiş alanı olmaz; ölçü alanı verin.",
                    l.name
                ));
            }
        }
        EventLinkKind::Line => {
            if l.from.is_none() || l.to.is_none() {
                return Some(format!(
                    "“{}” çizgi bağının başlangıç (from) ve bitiş (to) ölçüsü alanları yazılmalı.",
                    l.name
                ));
            }
            if l.measure.is_some() {
                return Some(format!(
                    "“{}” çizgi bağının tek ölçü alanı olmaz; başlangıç ve bitiş alanlarını verin.",
                    l.name
                ));
            }
        }
    }
    if let Some(t) = l.tolerance
        && !(t.is_finite() && t >= 0.0)
    {
        return Some(format!(
            "“{}” bağının toleransı sıfır ya da daha büyük bir sayı olmalı.",
            l.name
        ));
    }
    None
}

/// What is wrong with a project's links (KCAD, the commands): at most 50, each holding, ids unique, a layer the
/// events of one link only and never a route layer of another (a link's write would feed the next).
pub fn event_links_problem(links: &[EventLink]) -> Option<String> {
    if links.len() > MOST_EVENT_LINKS {
        return Some(format!(
            "Projede en çok {MOST_EVENT_LINKS} olay bağı olur; {} var.",
            links.len()
        ));
    }
    for (i, l) in links.iter().enumerate() {
        if let Some(p) = event_link_problem(l) {
            return Some(p);
        }
        if links[..i].iter().any(|m| m.id == l.id) {
            return Some(format!(
                "“{}” kimlikli iki olay bağı var; kimlik bir kez olur.",
                l.id
            ));
        }
        if links[..i].iter().any(|m| m.events == l.events) {
            return Some(format!(
                "“{}” katmanının olayları iki bağda; bir katman bir bağın olay katmanıdır.",
                l.events
            ));
        }
    }
    for l in links {
        if let Some(m) = links.iter().find(|m| m.routes.contains(&l.events)) {
            return Some(format!(
                "“{}” katmanı “{}” bağının olay katmanı ve “{}” bağının rota katmanı; ikisi birden olamaz.",
                l.events, l.name, m.name
            ));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link() -> EventLink {
        EventLink {
            id: "kazalar".into(),
            name: "Kazalar".into(),
            events: "kaza".into(),
            kind: EventLinkKind::Point,
            routes: vec!["yol".into()],
            route_field: "ad".into(),
            event_route: "yol".into(),
            measure: Some("km".into()),
            from: None,
            to: None,
            offset: None,
            side: EventSide::Right,
            tolerance: None,
        }
    }

    #[test]
    fn a_link_holds_by_its_shape() {
        assert_eq!(event_link_problem(&link()), None);
        let json = serde_json::to_value(link()).expect("writes");
        assert!(json.get("side").is_none() && json.get("tolerance").is_none());
        let mut l = link();
        l.measure = None;
        assert!(event_link_problem(&l).unwrap().contains("ölçü alanı"));
        let mut l = link();
        l.kind = EventLinkKind::Line;
        assert!(event_link_problem(&l).unwrap().contains("başlangıç"));
        l.measure = None;
        l.from = Some("a".into());
        l.to = Some("b".into());
        assert_eq!(event_link_problem(&l), None);
        let mut l = link();
        l.routes = vec!["kaza".into()];
        assert!(event_link_problem(&l).unwrap().contains("rota katmanı da"));
        let mut l = link();
        l.tolerance = Some(-1.0);
        assert!(event_link_problem(&l).unwrap().contains("tolerans"));
    }

    #[test]
    fn a_list_keeps_layers_apart() {
        let mut b = link();
        b.id = "ikinci".into();
        b.name = "İkinci".into();
        b.events = "levha".into();
        assert_eq!(event_links_problem(&[link(), b.clone()]), None);
        assert!(
            event_links_problem(&[link(), link()])
                .unwrap()
                .contains("iki")
        );
        b.routes = vec!["kaza".into()];
        assert!(
            event_links_problem(&[link(), b])
                .unwrap()
                .contains("ikisi birden")
        );
    }
}
