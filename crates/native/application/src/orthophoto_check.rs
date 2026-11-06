//! `cad.orthophoto.check` v1 (plan/map-cad.md MAP-12, docs/adr/1480): Ortofoto doğruluk denetimi, read only. The
//! ground's points and the image's points matched by name; a point the raster's georeference was fitted to (a control
//! point of `georef`, not a check point) is a fit point, the others check points. KOH_X,Y = √(Σ(dx² + dy²) / 2n), the
//! radial RMSE, the means and the largest error over each set; the limits from the rule package (BÖHHBÜY 2018 by
//! default, as data). Nothing is written. The shared cases are `fixtures/commands/v1/cad.orthophoto.check.json`.

use kentos_commands::{
    CommandResult, Entity, LayerNodeType, OrthoPoint, OrthoRules, OrthoStats, OrthophotoCheck, OrthophotoChecked,
    ortho_rules_default,
};
use kentos_domain::{Document, Uuid};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

const FIELD: &str = "Ad";

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// A layer's named points in the drawing's order (the first of a name): name, east, north, height.
fn named_points(doc: &Document, layer: &str, field: &str) -> Vec<(String, f64, f64, Option<f64>)> {
    let mut out: Vec<(String, f64, f64, Option<f64>)> = Vec::new();
    for e in doc.entities().filter(|e| e.base().layer_id == layer) {
        let Entity::Point(p) = e else { continue };
        let Some(name) = e.base().attrs.get(field).map(|v| v.trim().to_owned()).filter(|v| !v.is_empty()) else { continue };
        if !out.iter().any(|(n, ..)| *n == name) {
            out.push((name, p.p.x, p.p.y, p.z));
        }
    }
    out
}

fn stats(points: &[&OrthoPoint]) -> OrthoStats {
    let n = points.len();
    if n == 0 {
        return OrthoStats { n: 0, rmse_x: 0.0, rmse_y: 0.0, koh_xy: 0.0, rmse_r: 0.0, mean_x: 0.0, mean_y: 0.0, max_d: 0.0, n_z: 0, koh_z: None };
    }
    let k = n as f64;
    let sx: f64 = points.iter().map(|p| p.dx * p.dx).sum();
    let sy: f64 = points.iter().map(|p| p.dy * p.dy).sum();
    let zs: Vec<f64> = points.iter().filter_map(|p| p.dz).collect();
    OrthoStats {
        n: n as u32,
        rmse_x: (sx / k).sqrt(),
        rmse_y: (sy / k).sqrt(),
        koh_xy: ((sx + sy) / (2.0 * k)).sqrt(),
        rmse_r: ((sx + sy) / k).sqrt(),
        mean_x: points.iter().map(|p| p.dx).sum::<f64>() / k,
        mean_y: points.iter().map(|p| p.dy).sum::<f64>() / k,
        max_d: points.iter().map(|p| p.d).fold(0.0, f64::max),
        n_z: zs.len() as u32,
        koh_z: (!zs.is_empty()).then(|| (zs.iter().map(|z| z * z).sum::<f64>() / zs.len() as f64).sqrt()),
    }
}

fn run(doc: &Document, input: &OrthophotoCheck) -> Result<OrthophotoChecked, Stop> {
    let uid = input.raster.trim();
    if !checks::is_uid_text(uid) {
        return Err(fail(codes::INVALID_UID, format!("“{uid}” geçerli bir nesne kimliği değil."), "raster"));
    }
    let Some(e) = Uuid::parse_str(uid).ok().and_then(|u| doc.slot_of(u)).and_then(|s| doc.get(s)) else {
        return Err(fail(codes::ENTITY_NOT_FOUND, format!("“{uid}” kimlikli nesne çizimde yok."), "raster"));
    };
    let Entity::Raster(r) = e else {
        return Err(fail(codes::NOT_A_RASTER, format!("“{uid}” bir raster değil; ortofotoyu seçin."), "raster"));
    };
    let project = doc.settings().srid;
    if r.raster.srid != 0 && r.raster.srid != project {
        return Err(fail(
            codes::CRS_MISMATCH,
            format!(
                "Ortofotonun koordinat sistemi EPSG:{}, projeninki EPSG:{project}; farklı sistemdeki görüntü denetlenmez. Önce projeye çevirin (Raster › Yeniden izdüşür).",
                r.raster.srid
            ),
            "raster",
        ));
    }
    for (layer, path) in [(&input.known, "known"), (&input.observed, "observed")] {
        if !doc.layers().get(layer.trim()).is_some_and(|n| n.kind == LayerNodeType::Layer) {
            return Err(fail(codes::LAYER_NOT_FOUND, format!("“{}” katmanı çizimde yok.", layer.trim()), path));
        }
    }
    let scale = match input.scale {
        Some(0) => return Err(fail(codes::INVALID_INPUT, "Ölçek paydası 0 olamaz.".into(), "scale")),
        Some(s) => s,
        None => doc.settings().plot_scale.round().max(1.0) as u32,
    };
    let rules: OrthoRules = match &input.rules {
        Some(r) => r.clone(),
        None => ortho_rules_default().ok_or_else(|| fail(codes::INVALID_INPUT, "Yerleşik kural paketi okunamadı.".into(), "rules"))?,
    };
    if rules.id.trim().is_empty() || rules.source.trim().is_empty() {
        return Err(fail(codes::INVALID_INPUT, "Kural paketinin kimliği ya da kaynağı boş; sınırın dayanağı yazılmadan denetim yapılmaz.".into(), "rules"));
    }
    if rules.xy.iter().chain([&rules.z_factor, &rules.gsd_tolerance]).any(|v| !v.is_finite()) {
        return Err(fail(codes::INVALID_INPUT, "Kural paketinin bir sınırı sonlu bir sayı değil.".into(), "rules"));
    }
    let field = input.field.as_deref().map(str::trim).filter(|f| !f.is_empty()).unwrap_or(FIELD);
    let known = named_points(doc, input.known.trim(), field);
    let observed = named_points(doc, input.observed.trim(), field);
    // The names the image was fitted to: its georeference's control points.
    let fitted: Vec<String> = r
        .raster
        .georef
        .as_ref()
        .map(|g| g.points.iter().filter(|p| !p.check).filter_map(|p| p.name.as_ref().map(|n| n.trim().to_owned())).collect())
        .unwrap_or_default();
    let mut points = Vec::new();
    let mut unmatched = Vec::new();
    for (name, x, y, z) in &known {
        match observed.iter().find(|(n, ..)| n == name) {
            Some((_, ox, oy, oz)) => {
                let (dx, dy) = (ox - x, oy - y);
                points.push(OrthoPoint {
                    name: name.clone(),
                    role: if fitted.contains(name) { "fit".into() } else { "check".into() },
                    dx,
                    dy,
                    dz: oz.zip(*z).map(|(a, b)| a - b),
                    d: dx.hypot(dy),
                });
            }
            None => unmatched.push(name.clone()),
        }
    }
    for (name, ..) in &observed {
        if !known.iter().any(|(n, ..)| n == name) {
            unmatched.push(name.clone());
        }
    }
    let check = stats(&points.iter().filter(|p| p.role == "check").collect::<Vec<_>>());
    let fits: Vec<&OrthoPoint> = points.iter().filter(|p| p.role == "fit").collect();
    let fit = (!fits.is_empty()).then(|| stats(&fits));
    let s = scale as f64;
    let limit_xy = (rules.xy[0] * s * s + rules.xy[1] * s + rules.xy[2]) / 100.0;
    let limit_z = rules.z_factor * limit_xy;
    let a = r.raster.affine;
    let gsd = (a[1] * a[5] - a[2] * a[4]).abs().sqrt();
    let gsd_limit = rules.gsd.iter().find(|g| g.scale == scale).map(|g| g.gsd);
    let mut notes = vec![
        format!("KOH_X,Y = √(Σ(dx² + dy²) / 2n): X ve Y'nin ortak karesel ortalama hatası; sınırı {} (S = {scale}).", rules.id),
        "Uyum noktaları (görüntünün oturtulduğu kontrol noktaları) doğruluk göstermez; yalnız bağımsız denetim noktaları sayılır.".to_owned(),
        "Denetim noktalarının sayısı ve dağılımı (BÖHHBÜY m. 91(1): detay noktalarının en az %5'i) bu denetimde bilinmez; elle gözden geçirin.".to_owned(),
    ];
    if gsd_limit.is_none() {
        notes.push(format!("Kural paketi 1/{scale} ölçeği için yer örnekleme aralığı vermiyor."));
    }
    if !unmatched.is_empty() {
        notes.push(format!("Eşi bulunmayan noktalar: {}.", unmatched.join(", ")));
    }
    Ok(OrthophotoChecked {
        rules: rules.id.clone(),
        source: rules.source.clone(),
        scale,
        gsd,
        gsd_ok: gsd_limit.map(|l| gsd <= l * (1.0 + rules.gsd_tolerance) + 1e-12),
        gsd_limit,
        passed_xy: (check.n > 0).then(|| check.koh_xy <= limit_xy),
        passed_z: check.koh_z.map(|kz| kz <= limit_z),
        below_pixel: check.n > 0 && check.koh_xy < gsd / 2.0,
        points,
        unmatched,
        check,
        fit,
        limit_xy,
        limit_z,
        notes,
    })
}

pub fn validate(cx: &ExecutionContext<'_>, input: &OrthophotoCheck) -> CommandResult<()> {
    match run(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &OrthophotoCheck) -> CommandResult<OrthophotoChecked> {
    match run(cx.doc, input) {
        Ok(output) => CommandResult::Completed { output, warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: OrthophotoCheck) -> CommandResult<OrthophotoChecked> {
    plan(cx, &input)
}
