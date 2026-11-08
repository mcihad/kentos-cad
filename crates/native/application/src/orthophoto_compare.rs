//! `cad.orthophoto.compare` v1 (plan/map-cad.md MAP-12, docs/adr/1780): two orthophotos compared, read only. The points
//! read on each image are matched by name; a point's shift is the second's place − the first's. The shift's statistics
//! are those of `cad.orthophoto.check` (its means are the systematic shift); a point over the limit, the package's
//! `mosaic` × GSD (BÖHHBÜY Article 67(6): ±2·GSD; the coarser image's, KentOS's reading for two pixels), is flagged.
//! With the ground's points each image is also checked on its own (`cad.orthophoto.check`'s answer). Asked, the shift is
//! also measured in the images themselves where they overlap (`orthophoto_seams`, docs/adr/2460). Nothing is written.
//! The shared cases are `fixtures/commands/v1/cad.orthophoto.compare.json`.

use kentos_commands::{
    CommandResult, Entity, LayerNodeType, OrthoPair, OrthoPoint, OrthoRules, OrthophotoCheck, OrthophotoChecked,
    OrthophotoCompare, OrthophotoCompared, ortho_rules_default,
};
use kentos_domain::{Document, Uuid};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::orthophoto_check::{named_points, run as check_one, stats};

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// A raster of the input, checked: its id and ground sampling distance (the square root of a pixel's area).
fn raster(doc: &Document, uid: &str, path: &str) -> Result<(Uuid, f64), Stop> {
    let uid = uid.trim();
    if !checks::is_uid_text(uid) {
        return Err(fail(codes::INVALID_UID, format!("“{uid}” geçerli bir nesne kimliği değil."), path));
    }
    let Some((id, e)) = Uuid::parse_str(uid).ok().and_then(|u| doc.slot_of(u).and_then(|s| doc.get(s)).map(|e| (u, e))) else {
        return Err(fail(codes::ENTITY_NOT_FOUND, format!("“{uid}” kimlikli nesne çizimde yok."), path));
    };
    let Entity::Raster(r) = e else {
        return Err(fail(codes::NOT_A_RASTER, format!("“{uid}” bir raster değil; ortofotoyu seçin."), path));
    };
    let project = doc.settings().srid;
    if r.raster.srid != 0 && r.raster.srid != project {
        return Err(fail(
            codes::CRS_MISMATCH,
            format!(
                "Ortofotonun koordinat sistemi EPSG:{}, projeninki EPSG:{project}; farklı sistemdeki görüntü karşılaştırılmaz. Önce projeye çevirin (Raster › Yeniden izdüşür).",
                r.raster.srid
            ),
            path,
        ));
    }
    let a = r.raster.affine;
    Ok((id, (a[1] * a[5] - a[2] * a[4]).abs().sqrt()))
}

/// A number as Rust writes it (2 for 2.0), for the notes.
fn number(v: f64) -> String {
    format!("{v}")
}

fn run(doc: &Document, input: &OrthophotoCompare) -> Result<OrthophotoCompared, Stop> {
    let (u1, g1) = raster(doc, &input.first, "first")?;
    let (u2, g2) = raster(doc, &input.second, "second")?;
    if u1 == u2 {
        return Err(fail(codes::INVALID_INPUT, "Aynı ortofoto iki kez verilmiş; iki ayrı görüntü seçin.".into(), "second"));
    }
    let mut layers = vec![(&input.first_points, "firstPoints"), (&input.second_points, "secondPoints")];
    if let Some(k) = &input.known {
        layers.push((k, "known"));
    }
    for (layer, path) in layers {
        if !doc.layers().get(layer.trim()).is_some_and(|n| n.kind == LayerNodeType::Layer) {
            return Err(fail(codes::LAYER_NOT_FOUND, format!("“{}” katmanı çizimde yok.", layer.trim()), path));
        }
    }
    if input.scale == Some(0) {
        return Err(fail(codes::INVALID_INPUT, "Ölçek paydası 0 olamaz.".into(), "scale"));
    }
    if let Some(seam) = &input.seam {
        crate::orthophoto_seams::asked(seam)?;
    }
    let rules: OrthoRules = match &input.rules {
        Some(r) => r.clone(),
        None => ortho_rules_default().ok_or_else(|| fail(codes::INVALID_INPUT, "Yerleşik kural paketi okunamadı.".into(), "rules"))?,
    };
    if rules.id.trim().is_empty() || rules.source.trim().is_empty() {
        return Err(fail(codes::INVALID_INPUT, "Kural paketinin kimliği ya da kaynağı boş; sınırın dayanağı yazılmadan karşılaştırma yapılmaz.".into(), "rules"));
    }
    let Some(mosaic) = rules.mosaic.filter(|m| m.is_finite() && *m > 0.0) else {
        return Err(fail(
            codes::INVALID_INPUT,
            "Kural paketi iki görüntü arasındaki kaymanın sınırını (mosaic: YÖA'nın pozitif katı) vermiyor; dayanağı yazılmadan karşılaştırma yapılmaz.".into(),
            "rules",
        ));
    };
    let field = input.field.as_deref().map(str::trim).filter(|f| !f.is_empty()).unwrap_or("Ad");
    let first = named_points(doc, input.first_points.trim(), field);
    let second = named_points(doc, input.second_points.trim(), field);
    // BÖHHBÜY 67(6) speaks of one mosaic's images; for two of different pixels the coarser one's GSD (KentOS's reading).
    let limit = mosaic * g1.max(g2);
    let mut pairs = Vec::new();
    let mut shifts = Vec::new();
    let mut unmatched = Vec::new();
    for (name, x, y, _) in &first {
        match second.iter().find(|(n, ..)| n == name) {
            Some((_, sx, sy, _)) => {
                let (dx, dy) = (sx - x, sy - y);
                let d = dx.hypot(dy);
                pairs.push(OrthoPair { name: name.clone(), dx, dy, d, over: d > limit });
                shifts.push(OrthoPoint { name: name.clone(), role: "check".into(), dx, dy, dz: None, d });
            }
            None => unmatched.push(name.clone()),
        }
    }
    for (name, ..) in &second {
        if !first.iter().any(|(n, ..)| n == name) {
            unmatched.push(name.clone());
        }
    }
    let shift = stats(&shifts.iter().collect::<Vec<_>>());
    // Each image against the ground's points: `cad.orthophoto.check`'s answer, with this input's scale and package.
    let each = |raster: &str, observed: &str| -> Result<Option<OrthophotoChecked>, Stop> {
        let Some(known) = &input.known else { return Ok(None) };
        let check = OrthophotoCheck {
            raster: raster.to_owned(),
            known: known.clone(),
            observed: observed.to_owned(),
            field: input.field.clone(),
            scale: input.scale,
            rules: Some(rules.clone()),
        };
        check_one(doc, &check).map(Some)
    };
    let first_check = each(&input.first, &input.first_points)?;
    let second_check = each(&input.second, &input.second_points)?;
    // The overlap measured in the images themselves (docs/adr/2460).
    let seam = match &input.seam {
        Some(ask) => {
            let entity = |u: Uuid| match doc.slot_of(u).and_then(|s| doc.get(s)) {
                Some(Entity::Raster(r)) => Some(r.clone()),
                _ => None,
            };
            let (Some(r1), Some(r2)) = (entity(u1), entity(u2)) else { return Err(Stop::Cancelled) };
            Some(crate::orthophoto_seams::measure(doc, &r1, &r2, ask, limit)?)
        }
        None => None,
    };
    let mut notes = vec![
        format!(
            "Kayma = ikinci − birinci görüntüde okunan yer; nokta başına sınır {}·YÖA = {limit:.3} m ({}), iki görüntüden YÖA'sı iri olanınki (KentOS'un okuması).",
            number(mosaic),
            rules.id
        ),
        "Ortalama kayma görüntüler arasındaki sistematik farktır; iki zamanın farkı kendiliğinden deformasyon ya da yer değiştirme değildir, görüntülerin kendi doğruluğuyla birlikte okunur.".to_owned(),
    ];
    if !unmatched.is_empty() {
        notes.push(format!("Eşi bulunmayan noktalar: {}.", unmatched.join(", ")));
    }
    if input.known.is_none() {
        notes.push("Arazi noktaları verilmedi: görüntülerin kendi doğruluğu denetlenmedi.".into());
    }
    if let Some(s) = &seam {
        notes.push(format!(
            "Dikiş ölçümü: örtüşmede {} yer; {} sayıldı, {} korelasyonu düşük, {} tepe arama sınırında, {} dokusuz. Yerler görüntülerin kendisinden normalize çapraz korelasyonla ölçüldü (piksel altı, parabolle).",
            s.patches.len(),
            s.ok,
            s.low,
            s.edge,
            s.flat
        ));
    }
    Ok(OrthophotoCompared {
        rules: rules.id.clone(),
        source: rules.source.clone(),
        gsd: [g1, g2],
        limit,
        passed: (!pairs.is_empty()).then(|| pairs.iter().all(|p| !p.over)),
        pairs,
        unmatched,
        shift,
        first_check,
        second_check,
        seam,
        notes,
    })
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &OrthophotoCompare) -> CommandResult<()> {
    match run(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &OrthophotoCompare) -> CommandResult<OrthophotoCompared> {
    match run(cx.doc, input) {
        Ok(output) => CommandResult::Completed { output, warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: OrthophotoCompare) -> CommandResult<OrthophotoCompared> {
    plan(cx, &input)
}
