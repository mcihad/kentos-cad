//! Ağ analizi's İşlemler tools (docs/adr/0209 §6, §7, §10; the web's
//! `builtin/network/`): En yakın tesis, Maliyet matrisi and Hizmet alanları.
//! A run builds the network in a store of its own from its layers' objects as
//! the run sees them (`crate::network`); what the tools share is here: the
//! points of a features input found on the network, the names the results
//! say, and costs written as numbers.

pub mod closest_facility;
pub mod od_matrix;
pub mod service_areas;

use kentos_commands::{Entity, NetworkDef};
use kentos_geometry_core::display::fixed;
use kentos_geometry_core::ops::network::NetworkSession;
use kentos_geometry_core::vec2::Vec2;
use serde_json::Value;

use crate::network::{FromDocument, from_document};
use crate::text::js_trim;
use crate::types::{Feedback, ParamDef, ParamKind, Resolved, RunContext, ScopeKind};

/// A network built for a run: its definition, the cost asked with (its place), its costs' names and the session.
pub struct RunNetwork {
    pub def: NetworkDef,
    pub cost: usize,
    pub cost_names: Vec<String>,
    pub session: NetworkSession,
}

/// The network parameter of the tools.
pub fn network_param(description: &str) -> ParamDef {
    ParamDef::new(
        "network",
        "Ağ",
        ParamKind::Network {
            prefers: kentos_commands::NetworkKind::Road,
        },
    )
    .describe(description)
}

/// A points input of the tools.
pub fn points_param(name: &str, label: &str, description: Option<&str>) -> ParamDef {
    let p = ParamDef::new(
        name,
        label,
        ParamKind::Features {
            kinds: Some(vec!["point".to_owned()]),
            scopes: Some(vec![
                ScopeKind::Layer,
                ScopeKind::Selection,
                ScopeKind::Visible,
                ScopeKind::All,
            ]),
            writes: false,
        },
    );
    match description {
        Some(d) => p.describe(d),
        None => p,
    }
}

/// Arama uzaklığı.
pub fn reach_param(what: &str) -> ParamDef {
    ParamDef::new(
        "reach",
        "Arama uzaklığı",
        ParamKind::Number {
            min: Some(0.001),
            max: Some(100_000.0),
            integer: false,
            unit: "m".into(),
            placeholder: None,
        },
    )
    .default_value(serde_json::json!(100))
    .advanced()
    .describe(what)
}

/// Kalkış saati (docs/adr/0521 §4): the time of day the questions set off at, for a network with a time profile.
pub fn time_param() -> ParamDef {
    ParamDef::new(
        "time",
        "Kalkış saati",
        ParamKind::Text {
            placeholder: Some("ör. 07:30".into()),
            max_length: Some(5),
            allow_empty: true,
        },
    )
    .optional()
    .advanced()
    .describe(
        "Zaman profili olan ağda maliyet bu saatin hızlarıyla hesaplanır; boş: her saat aynı.",
    )
}

/// Araç profili (docs/adr/0567 §1): the vehicle the questions are for, by its name in the network's definition.
pub fn profile_param() -> ParamDef {
    ParamDef::new(
        "profile",
        "Araç profili",
        ParamKind::Text {
            placeholder: Some("ör. Kamyon".into()),
            max_length: Some(40),
            allow_empty: true,
        },
    )
    .optional()
    .advanced()
    .describe(
        "Ağın Araç profillerinden birinin adı: sınırlarına uymayan ve yasakladığı kenarlardan geçilmez, hızı en yüksek hızıyla sınırlanır; boş: profil yok.",
    )
}

/// Tarih (docs/adr/0567 §2): the day the questions are for; the edges closed that day are not travelled.
pub fn date_param() -> ParamDef {
    ParamDef::new(
        "date",
        "Tarih",
        ParamKind::Text {
            placeholder: Some("ör. 2026-11-03".into()),
            max_length: Some(10),
            allow_empty: true,
        },
    )
    .optional()
    .advanced()
    .describe("Tarihli kapanmaları olan ağda o gün kapalı kenarlardan geçilmez; boş: kapanmalar uygulanmaz.")
}

/// Builds the run's network; says what building said (expressions that did not compile, objects not taken).
pub fn run_network(
    v: &Resolved<'_>,
    ctx: &RunContext<'_>,
    feedback: &mut dyn Feedback,
) -> Result<RunNetwork, String> {
    let value = v.values.get("network").cloned().unwrap_or(Value::Null);
    let id = value.get("network").and_then(Value::as_str).unwrap_or("");
    let cost = value.get("cost").and_then(Value::as_str).unwrap_or("");
    let time = match v.values.get("time").and_then(Value::as_str).map(js_trim) {
        None | Some("") => None,
        Some(t) => Some(
            kentos_commands::clock_of(t)
                .ok_or_else(|| format!("Kalkış saati “{t}” okunamadı; 07:30 gibi yazın."))?,
        ),
    };
    let FromDocument {
        def,
        mut session,
        problems,
    } = from_document(ctx.doc, id)?;
    session.time = time.map(f64::from);
    let text_of = |key: &str| v.values.get(key).and_then(Value::as_str).map(js_trim).filter(|t| !t.is_empty());
    let (profile, date) = (text_of("profile"), text_of("date"));
    if date.is_some() && def.closures.is_none() {
        feedback.warn(format!(
            "{}: ağın tarihli kapanması yok; tarih maliyeti değiştirmez.",
            def.name
        ));
    }
    let varied = session.set_access(profile, date)?;
    if varied.by_profile > 0 {
        feedback.info(format!(
            "{}: “{}” profili {} kenara giremez.",
            def.name,
            profile.unwrap_or_default(),
            varied.by_profile
        ));
    }
    if varied.by_date > 0 {
        feedback.info(format!(
            "{}: {} kenar {} tarihinde kapalı.",
            def.name,
            varied.by_date,
            date.unwrap_or_default()
        ));
    }
    if varied.hours_unasked {
        feedback.warn(format!(
            "{}: {} kenarın saatli kapanması var; kalkış saati verilmedikçe uygulanmaz.",
            def.name, varied.by_hours
        ));
    } else if varied.by_hours > 0 {
        feedback.info(format!(
            "{}: {} kenar {} haftasının bazı saatlerinde kapalı; süre bekleyerek ya da dolaşarak hesaplandı.",
            def.name,
            varied.by_hours,
            date.unwrap_or_default()
        ));
    }
    if !varied.unread.is_empty() {
        feedback.warn(format!(
            "{}: {} kenarın sınır ya da kapanma değeri okunamadı; o değer kenarı kapatmadı.",
            def.name,
            varied.unread.len()
        ));
    }
    if time.is_some() && !def.timed() {
        feedback.warn(format!(
            "{}: ağın zaman profili yok; kalkış saati maliyeti değiştirmez.",
            def.name
        ));
    }
    for p in &problems {
        feedback.warn(format!("{}: {p}", def.name));
    }
    if !session.skipped.is_empty() {
        feedback.warn(format!(
            "{}: ağın katmanlarında {} nesne çizgi, çoklu çizgi, yay ya da nokta değil; ağa alınmadı.",
            def.name,
            session.skipped.len()
        ));
    }
    let cost_names: Vec<String> = def.cost_names().into_iter().map(str::to_owned).collect();
    let cost = cost_names.iter().position(|c| c == cost).unwrap_or(0);
    Ok(RunNetwork {
        def,
        cost,
        cost_names,
        session,
    })
}

/// A way as a polyline's geometry: its points, and its bulges when it has an arc (the web's `lineGeometry`).
pub fn line_geometry(
    l: &kentos_geometry_core::ops::network::Line,
) -> kentos_commands::EntityGeometry {
    let arcs = l.bulges.iter().any(|b| *b != 0.0);
    kentos_commands::EntityGeometry::Polyline {
        ms: None,
        pts: l
            .pts
            .iter()
            .map(|p| kentos_commands::Vec2 { x: p[0], y: p[1] })
            .collect(),
        bulges: arcs.then(|| l.bulges.clone()),
        zs: None,
        parts: None,
        closed: false,
        widths: None,
    }
}

/// A point object's place (its first point).
pub fn point_of(e: &Entity) -> Option<Vec2> {
    match e {
        Entity::Point(p) => Some(Vec2::new(p.p.x, p.p.y)),
        _ => None,
    }
}

/// The objects found on the network within `reach` (in their order, with their points) and those that are not.
/// Why pairs of points have no way between them with cost `cost` (docs/adr/0570): the first three as
/// “A → B: …”, and how many more.
pub fn unreached_words(
    s: &NetworkSession,
    pairs: &[(String, Vec2, String, Vec2)],
    reach: f64,
    cost: usize,
) -> String {
    let said: Vec<String> = pairs
        .iter()
        .take(3)
        .filter_map(|(a, pa, b, pb)| {
            let why = s.blocked_between(*pa, *pb, reach, cost)?;
            Some(format!("{a} → {b}: {}", why.words()))
        })
        .collect();
    let more = pairs.len().saturating_sub(3);
    let tail = if more > 0 { format!("; {more} tane daha") } else { String::new() };
    format!("{}{tail}", said.join("; "))
}

pub fn places_of<'e>(
    s: &NetworkSession,
    objects: &[&'e Entity],
    reach: f64,
) -> (Vec<(&'e Entity, Vec2)>, Vec<&'e Entity>) {
    let mut found = Vec::new();
    let mut missing = Vec::new();
    for e in objects {
        match point_of(e).filter(|p| s.locate(*p, reach).is_some()) {
            Some(p) => found.push((*e, p)),
            None => missing.push(*e),
        }
    }
    (found, missing)
}

/// A point's name in the results: its label, its `Ad`, else its number.
pub fn name_of(e: &Entity) -> String {
    let b = e.base();
    b.label
        .as_deref()
        .map(js_trim)
        .filter(|l| !l.is_empty())
        .or_else(|| {
            b.attrs
                .get("Ad")
                .map(|a| js_trim(a))
                .filter(|a| !a.is_empty())
        })
        .map_or_else(|| format!("#{}", b.id), str::to_owned)
}

/// A cost's value as an attribute: the length with the project's decimals, the others with two; empty for none.
pub fn cost_value(v: Option<f64>, c: usize, length_decimals: u32) -> String {
    match v.filter(|v| v.is_finite()) {
        Some(v) => fixed(v, if c == 0 { length_decimals as usize } else { 2 }),
        None => String::new(),
    }
}

/// The note about points not found on the network.
pub fn missing_note(missing: usize, what: &str, reach: f64, feedback: &mut dyn Feedback) {
    if missing > 0 {
        feedback.warn(format!(
            "{missing} {what} ağa {} m içinde değil (ya da nokta değil); alınmadı.",
            fixed(reach, 2)
        ));
    }
}
