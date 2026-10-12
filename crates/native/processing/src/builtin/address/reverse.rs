//! Konumlara adres yaz (docs/adr/0229 §12): each object's address, from its
//! own place (a point's) or its inner point, looked for in the project's
//! address data within a reach or asked of an online provider, written
//! into its attributes in one step; an object whose address is not found
//! loses the fields it had.

use std::collections::BTreeMap;

use kentos_domain::Slot;
use kentos_geometry_core::ops::convert::points::interior_point;
use kentos_geometry_core::vec2::Vec2;
use kentos_services::geocode::{read, request};
use serde_json::{Value, json};

use super::source_param;
use crate::address::{Match, NO_SYSTEM, Plane, Source, local_place, online_place, resolve};
use crate::types::{
    ChangeSet, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Patch, Resolved, RunContext,
    RunResult, ScopeKind, Target, Tool,
};

/// The fields the tool writes, in order.
pub const FIELDS: [&str; 10] = [
    "Adres",
    "Mahalle",
    "Yol",
    "Kapı no",
    "İlçe",
    "İl",
    "Posta kodu",
    "Adres uzaklığı",
    "Adres türü",
    "Adres kaynağı",
];

pub fn tool() -> Tool {
    Tool {
        id: "address.reverse".into(),
        label: "Konumlara adres yaz".into(),
        category: "address".into(),
        description: "Nesnelerin yerlerinin adresini bulur ve özniteliklerine yazar.".into(),
        help: Some([
            "Örnekler: arıza ya da şikâyet noktalarına en yakın kapının adresini yazmak, yapı alanlarına bulundukları mahalle ve yolu yazmak.",
            "Noktanın kendi yeri, öbür nesnelerin iç noktası alınır. Projenin adres verisinde uzaklık içindeki en yakın kapı, yoksa en yakın yol, ayrıca içinde kaldığı mahalle, ilçe ve il alanı bulunur. Çevrimiçi sağlayıcıda yerin WGS 84 koordinatları onun sunucusuna gider.",
            "Her nesneye Adres, Mahalle, Yol, Kapı no, İlçe, İl, Posta kodu, Adres uzaklığı (m), Adres türü ve Adres kaynağı yazılır; adresi bulunamayan nesnenin bu alanları boşaltılır. Tek geri alma adımıdır.",
        ]
        .join("\n\n")),
        keywords: [
            "adres", "reverse geocode", "konumdan adres", "ters coğrafi kodlama", "mahalle bul", "kapı bul",
        ]
        .map(String::from)
        .to_vec(),
        aliases: ["ADRESYAZ", "REVERSEGEOCODE"].map(String::from).to_vec(),
        icon: Some("addressReverseBatch".into()),
        parameters: vec![
            ParamDef::new(
                "input",
                "Nesneler",
                ParamKind::Features {
                    kinds: None,
                    scopes: Some(vec![
                        ScopeKind::Selection,
                        ScopeKind::Layer,
                        ScopeKind::Visible,
                        ScopeKind::All,
                    ]),
                    writes: true,
                },
            )
            .describe("Adresleri yazılacak nesneler; nokta kendi yeriyle, öbürleri iç noktalarıyla."),
            source_param(),
            ParamDef::new(
                "reach",
                "En büyük uzaklık",
                ParamKind::Number {
                    min: Some(0.001),
                    max: Some(10_000.0),
                    integer: false,
                    unit: "m".into(),
                    placeholder: Some("Adres verisininki".into()),
                },
            )
            .optional()
            .default_value(Value::Null)
            .describe("Projenin adres verisinde kapının ya da yolun en çok bu kadar uzakta aranır; boşsa adres verisinin uzaklığı."),
        ],
        outputs: vec![
            OutputDef::new("count", "Adresi yazılan nesne sayısı", OutputKind::Number),
            OutputDef::new("missing", "Adresi bulunamayan", OutputKind::Number),
        ],
        targets: vec![Target::Worker],
        validate: None,
        preview: None,
        run: Some(run),
    }
}

/// An address's fields, in [`FIELDS`]' order.
type Fields = [String; 10];

/// A found place's fields from its source `source`.
fn fields_of(m: &Match, source: &str) -> Fields {
    let f = &m.fields;
    [
        m.label.clone(),
        f[0].clone(),
        f[1].clone(),
        f[2].clone(),
        f[3].clone(),
        f[4].clone(),
        f[5].clone(),
        m.distance.map(|d| format!("{d:.2}")).unwrap_or_default(),
        m.kind.clone(),
        source.to_owned(),
    ]
}

fn run(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let host = feedback.addresses();
    let source = match resolve(v.text("via"), ctx.doc, host.as_deref()) {
        Ok(s) => s,
        Err(why) => return RunResult::refused(why),
    };
    let objects = &v.features("input").entities;
    let total = objects.len();
    let places: Vec<Option<Vec2>> = objects
        .iter()
        .map(|e| interior_point(&kentos_native_application::geometry::shape(e)))
        .collect();
    let mut found: Vec<Option<Fields>> = Vec::with_capacity(total);
    match &source {
        Source::Local(l) => {
            feedback.progress(0.0, "Adres dizini kuruluyor");
            let loc = kentos_native_application::addresses::build(ctx.doc, l);
            let reach = v.number("reach").unwrap_or(l.reach);
            for (i, p) in places.iter().enumerate() {
                found.push(
                    p.and_then(|p| loc.reverse(p, reach))
                        .map(|place| fields_of(&local_place(&place), &l.name)),
                );
                if i % 1000 == 999 {
                    feedback.progress(
                        i as f64 / total as f64,
                        &format!("{} / {total} nesne", i + 1),
                    );
                    if feedback.canceled() {
                        return RunResult::default();
                    }
                }
            }
        }
        Source::Online(p) => {
            let Some(plane) = Plane::of(ctx.doc) else {
                return RunResult::refused(NO_SYSTEM.into());
            };
            let Some(host) = host else {
                return RunResult::refused("Bu ortamda çevrimiçi adres sağlayıcısı yok.".into());
            };
            if let Some(why) = p.batch_refusal(places.iter().flatten().count()) {
                return RunResult::refused(why);
            }
            let mut failures = 0usize;
            let mut last = String::new();
            for (i, at) in places.iter().enumerate() {
                feedback.progress(
                    i as f64 / total.max(1) as f64,
                    &format!("{} / {total} nesne", i + 1),
                );
                if feedback.canceled() {
                    return RunResult::default();
                }
                let Some((lon, lat)) = at.and_then(|q| plane.to_wgs84(q)) else {
                    found.push(None);
                    continue;
                };
                let req = request::reverse(p, lon, lat);
                let stop = || feedback.canceled();
                let answer = host
                    .ask(p, &req, &stop)
                    .and_then(|body| read::reverse(p.kind, &body));
                match answer {
                    Ok(Some(c)) => {
                        let per = kentos_native_application::addresses::per_metre(ctx.doc);
                        let place = at.and_then(|o| online_place(&c, &plane, o, per));
                        found.push(place.map(|m| fields_of(&m, &p.name)));
                    }
                    Ok(None) => found.push(None),
                    Err(why) => {
                        failures += 1;
                        last = why;
                        found.push(None);
                    }
                }
            }
            if failures > 0 {
                feedback.warn(format!(
                    "{failures} yer için sağlayıcı cevap vermedi; son neden: {last}"
                ));
            }
        }
    }
    let mut update = Vec::with_capacity(total);
    let (mut count, mut missing) = (0usize, 0usize);
    for (e, f) in objects.iter().zip(found) {
        let mut attrs: BTreeMap<String, String> = e.base().attrs.clone();
        match f {
            Some(values) => {
                count += 1;
                for (name, value) in FIELDS.iter().zip(values) {
                    if value.is_empty() {
                        attrs.remove(*name);
                    } else {
                        attrs.insert((*name).into(), value);
                    }
                }
            }
            None => {
                missing += 1;
                for name in FIELDS {
                    attrs.remove(name);
                }
            }
        }
        if attrs != e.base().attrs {
            update.push(Patch {
                id: Slot(e.base().id),
                attrs: Some(attrs),
                label: None,
                zs: None,
                geometry: None,
                layer: None,
            });
        }
    }
    RunResult {
        changes: Some(ChangeSet {
            update,
            ..ChangeSet::default()
        }),
        outputs: [
            ("count".to_owned(), json!(count)),
            ("missing".to_owned(), json!(missing)),
        ]
        .into_iter()
        .collect(),
        summary: Some(format!(
            "{total} nesneden {count} nesnenin adresi yazıldı; {missing} nesnenin adresi bulunamadı."
        )),
        ..RunResult::default()
    }
}
