//! Uygunluk analizi (docs/adr/0237): Bulanık üyelik, Bulanık çakıştırma,
//! Ağırlıklı toplam and Ağırlıklı çakıştırma (a raster beside the first
//! input, its object right above the first input's layer), İkili
//! karşılaştırma (the weights' table, and the weighted sum's raster) and ROC
//! ile doğrulama (the curve's table), each a run of the raster core's
//! operation job in the run's one step.

mod tools;

use std::collections::BTreeMap;

use kentos_contracts::{Entity, RasterEntity, RasterFields};
use kentos_geometry_core::display::fixed;
use kentos_native_application::geometry::shape;
use kentos_raster::ops::Notes;
use kentos_raster::suitability::pairwise::{CONSISTENT, Pairwise};
use kentos_raster::suitability::roc::Roc;
use serde_json::{Value, json};

use super::hydrology::trimmed;
use super::pointcloud::{broke, count_words};
use super::raster_ops::{drive, names_of, rasters_for, spec_of};
use super::surface::{base, files_of};
use crate::files::Beside;
use crate::types::{ChangeSet, Feedback, Resolved, RunContext, RunResult};

pub use tools::{fuzzy_membership, fuzzy_overlay, pairwise, roc, weighted_overlay, weighted_sum};

pub const SUITABILITY: &str = "suitability";

/// What a raster tool says of its notes: the summary's tail; the warnings through the feedback.
type Said<'a> = dyn Fn(&Notes, usize, &mut dyn Feedback) -> String + 'a;

/// Runs a tool whose result is a raster (§2): the file beside the first
/// input (or where asked), its object right above the first input's layer.
pub fn run_raster(
    r: &Resolved<'_>,
    ctx: &RunContext<'_>,
    feedback: &mut dyn Feedback,
    tool: Value,
    (suffix, label): (&str, &str),
    (one, said): (bool, &Said<'_>),
) -> RunResult {
    let files = match files_of(feedback) {
        Ok(f) => f,
        Err(e) => return *e,
    };
    let go = || -> Result<RunResult, String> {
        let rasters = rasters_for(r, ctx, one)?;
        let names = names_of(&rasters, ctx);
        let spec = spec_of(ctx, &rasters, &names, tool)?;
        let first = &rasters[0].raster;
        let path =
            files.output_path(r.text("output"), Some(Beside::from(first)), suffix, ".tif")?;
        let ran = drive(
            &*files,
            &rasters,
            &spec,
            Vec::new(),
            Some(&path),
            feedback,
            label,
        )?;
        feedback.progress(1.0, label);
        let Some((bands, sample, style, grid)) = ran.raster else {
            return Err("Çözümleme raster vermedi.".into());
        };
        if ran.notes.empty_cells == u64::from(grid.width) * u64::from(grid.height) {
            feedback.warn("Sonucun hiçbir hücresinde değer yok.".to_owned());
        }
        let add: Vec<Entity> = r
            .flag("add")
            .then(|| {
                Entity::Raster(RasterEntity {
                    base: base(&r.layer("layer").id, BTreeMap::new(), None),
                    raster: RasterFields {
                        affine: grid.affine,
                        width: grid.width,
                        height: grid.height,
                        bands,
                        sample,
                        asset: None,
                        file: Some(path.clone()),
                        url: None,
                        srid: first.srid,
                        style,
                        opacity: None,
                    },
                })
            })
            .into_iter()
            .collect();
        let summary = format!(
            "{} × {} hücrelik raster; “{path}” yazıldı.{}",
            count_words(u64::from(grid.width)),
            count_words(u64::from(grid.height)),
            said(&ran.notes, rasters.len(), feedback)
        );
        let mut result = RunResult {
            changes: (!add.is_empty()).then(|| ChangeSet {
                add,
                ..ChangeSet::default()
            }),
            summary: Some(summary),
            above: Some(rasters[0].base.layer_id.clone()),
            ..RunResult::default()
        };
        result.outputs.insert("file".to_owned(), json!(path));
        Ok(result)
    };
    go().unwrap_or_else(broke)
}

/// “n raster birleşti.”
pub fn joined(n: usize) -> String {
    format!(" {} raster birleşti.", count_words(n as u64))
}

/// Bulanık çakıştırma's note: the cells with a membership outside 0–1.
pub fn say_invalid(n: &Notes, feedback: &mut dyn Feedback) {
    if n.suit.invalid > 0 {
        feedback.warn(format!(
            "{} hücrede üyelik 0–1 aralığının dışında; o hücreler değersiz bırakıldı.",
            count_words(n.suit.invalid)
        ));
    }
}

/// Ağırlıklı çakıştırma's notes: the cells no rule held, off the scale, restricted.
pub fn say_overlay(n: &Notes, feedback: &mut dyn Feedback) -> String {
    let s = &n.suit;
    if s.unmatched > 0 {
        feedback.warn(format!(
            "Sınıf tablosunun hiçbir kuralının tutmadığı {} hücre değersiz bırakıldı.",
            count_words(s.unmatched)
        ));
    }
    if s.outside > 0 {
        feedback.warn(format!(
            "Ölçeğin dışında (ya da tam sayı olmayan) değerli {} hücre değersiz bırakıldı.",
            count_words(s.outside)
        ));
    }
    if s.restricted > 0 {
        format!(" Kısıtlı {} hücre.", count_words(s.restricted))
    } else {
        String::new()
    }
}

/// İkili karşılaştırma's figures as the summary writes them.
pub fn consistency(p: &Pairwise) -> String {
    let verdict = if p.cr > CONSISTENT {
        "tutarsız"
    } else {
        "tutarlı"
    };
    format!(
        "λ {}; CI {}; RI {}; CR {} ({verdict}).",
        fixed(p.lambda, 4),
        fixed(p.ci, 4),
        fixed(p.ri, 2),
        fixed(p.cr, 4)
    )
}

/// The weights' table: Ölçüt, Ağırlık, Yüzde.
pub fn weights_table(names: &[String], p: &Pairwise) -> Value {
    let rows: Vec<Vec<String>> = names
        .iter()
        .zip(&p.weights)
        .map(|(n, w)| vec![n.clone(), fixed(*w, 6), fixed(100.0 * w, 2)])
        .collect();
    json!({ "columns": ["Ölçüt", "Ağırlık", "Yüzde (%)"], "rows": rows })
}

/// The warning of inconsistent comparisons.
pub fn warn_inconsistent(p: &Pairwise, feedback: &mut dyn Feedback) {
    if p.cr > CONSISTENT {
        feedback.warn(format!(
            "Karşılaştırmalar tutarsız (CR {} > 0.10); en çelişkili çiftleri yeniden gözden geçirin.",
            fixed(p.cr, 2)
        ));
    }
}

/// Runs İkili karşılaştırma: the weights' table and, when asked, the weighted sum's raster.
pub fn run_pairwise(
    r: &Resolved<'_>,
    ctx: &RunContext<'_>,
    feedback: &mut dyn Feedback,
) -> RunResult {
    let write = r.flag("write");
    let pairs = r.values.get("comparisons").cloned().unwrap_or(json!([]));
    let tool = json!({
        "kind": "pairwise",
        "band": r.number("band").unwrap_or(1.0) as u32,
        "pairs": pairs,
        "write": write,
        "sample": r.text("sample"),
    });
    let names = names_of(&rasters_in(r, ctx), ctx);
    let table = std::cell::RefCell::new(None);
    let said = |n: &Notes, _: usize, feedback: &mut dyn Feedback| -> String {
        match &n.suit.pairwise {
            Some(p) => {
                warn_inconsistent(p, feedback);
                *table.borrow_mut() = Some(weights_table(&names, p));
                format!(" Ağırlıklar tabloda; {}", consistency(p))
            }
            None => String::new(),
        }
    };
    if write {
        let mut result = run_raster(
            r,
            ctx,
            feedback,
            tool,
            ("-ahp", "Ağırlıklı toplam hesaplanıyor"),
            (false, &said),
        );
        if let Some(t) = table.into_inner() {
            result.outputs.insert("table".to_owned(), t);
        }
        return result;
    }
    let files = match files_of(feedback) {
        Ok(f) => f,
        Err(e) => return *e,
    };
    let go = || -> Result<RunResult, String> {
        let rasters = rasters_for(r, ctx, false)?;
        let names = names_of(&rasters, ctx);
        let spec = spec_of(ctx, &rasters, &names, tool)?;
        let label = "Ağırlıklar hesaplanıyor";
        let ran = drive(&*files, &rasters, &spec, Vec::new(), None, feedback, label)?;
        feedback.progress(1.0, label);
        let p = ran
            .notes
            .suit
            .pairwise
            .as_ref()
            .ok_or("Ağırlıklar hesaplanamadı.")?;
        warn_inconsistent(p, feedback);
        let mut result = RunResult {
            summary: Some(format!(
                "{} ölçütün ağırlıkları tabloda; {}",
                count_words(names.len() as u64),
                consistency(p)
            )),
            ..RunResult::default()
        };
        result
            .outputs
            .insert("table".to_owned(), weights_table(&names, p));
        Ok(result)
    };
    go().unwrap_or_else(broke)
}

/// The input's rasters, for their names (the run checks them again).
fn rasters_in<'a>(r: &Resolved<'a>, ctx: &RunContext<'_>) -> Vec<&'a RasterEntity> {
    super::raster_ops::rasters_in_order(r, ctx)
}

/// The curve's table (§8).
pub fn roc_table(c: &Roc) -> Value {
    let (fp_label, fp_share) = if c.all_cells {
        ("Hücre", "Alan oranı (%)")
    } else {
        ("Yanlış pozitif", "Yanlış pozitif oranı (%)")
    };
    let rows: Vec<Vec<String>> = c
        .rows
        .iter()
        .map(|x| {
            vec![
                trimmed(x.threshold, 6),
                x.tp.to_string(),
                fixed(100.0 * x.tp as f64 / c.presence as f64, 2),
                x.fp.to_string(),
                fixed(100.0 * x.fp as f64 / c.background as f64, 2),
            ]
        })
        .collect();
    json!({
        "columns": ["Eşik", "Doğru pozitif", "Doğru pozitif oranı (%)", fp_label, fp_share],
        "rows": rows,
    })
}

/// ROC's summary and warnings (§8).
pub fn roc_summary(c: &Roc, feedback: &mut dyn Feedback) -> String {
    if c.skipped > 0 {
        feedback.warn(format!(
            "Değersiz hücreye düşen {} örnek atlandı.",
            count_words(c.skipped)
        ));
    }
    if c.outside > 0 {
        feedback.warn(format!(
            "Rasterin dışında kalan {} nokta atlandı.",
            count_words(c.outside)
        ));
    }
    if c.both > 0 {
        feedback.warn(format!(
            "{} hücre hem varlık hem yokluk; ikisinde de sayıldı.",
            count_words(c.both)
        ));
    }
    let other = if c.all_cells {
        "değerli hücre"
    } else {
        "yokluk hücresi"
    };
    let mut s = format!(
        "AUC {} ({} varlık hücresi, {} {other}).",
        fixed(c.auc, 4),
        count_words(c.presence),
        count_words(c.background)
    );
    if let Some(b) = c.best.and_then(|k| c.rows.get(k)) {
        let share = if c.all_cells {
            "alan"
        } else {
            "yanlış pozitif"
        };
        s.push_str(&format!(
            " En iyi eşik {} (doğru pozitif %{}, {share} %{}).",
            trimmed(b.threshold, 6),
            fixed(100.0 * b.tp as f64 / c.presence as f64, 1),
            fixed(100.0 * b.fp as f64 / c.background as f64, 1)
        ));
    }
    s
}

/// Runs ROC ile doğrulama: the curve's table.
pub fn run_roc(r: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let files = match files_of(feedback) {
        Ok(f) => f,
        Err(e) => return *e,
    };
    let mut go = || -> Result<RunResult, String> {
        let rasters = rasters_for(r, ctx, true)?;
        let names = names_of(&rasters, ctx);
        let absence = r.text("background") == "absence";
        let mut shapes: Vec<_> = r
            .features("presence")
            .entities
            .iter()
            .map(|e| shape(e))
            .collect();
        let first = shapes.len();
        if absence {
            shapes.extend(r.features("absence").entities.iter().map(|e| shape(e)));
        }
        let tool = json!({
            "kind": "roc",
            "band": r.number("band").unwrap_or(1.0) as u32,
            "first": first,
            "absence": absence,
            "higher": r.flag("higher"),
        });
        let spec = spec_of(ctx, &rasters, &names, tool)?;
        let label = "Doğrulanıyor";
        let ran = drive(&*files, &rasters, &spec, shapes, None, feedback, label)?;
        feedback.progress(1.0, label);
        let c = ran.roc.ok_or("Doğrulama sonuç vermedi.")?;
        let mut result = RunResult {
            summary: Some(roc_summary(&c, feedback)),
            ..RunResult::default()
        };
        result.outputs.insert("table".to_owned(), roc_table(&c));
        result.outputs.insert("auc".to_owned(), json!(c.auc));
        Ok(result)
    };
    go().unwrap_or_else(broke)
}
