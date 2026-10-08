//! Alan sorgusu (docs/adr/0207 §7; Netcad's Nokta Bulutu Alan Sor, ArcGIS
//! LAS Point Statistics By Area): for each selected area, the points of the
//! clouds in it: how many, how dense, their heights' least, most, mean and
//! sample standard deviation, and each class's count (`ops::stats`). A
//! table, with Panoya kopyala and CSV; the drawing does not change.

use kentos_contracts::Entity;
use kentos_geometry_core::ops::areas::areas_of_entity;
use kentos_native_application::geometry::shape;
use kentos_pointcloud::ops::region::Region;
use kentos_pointcloud::ops::stats::AreaQuery;
use serde_json::json;

use super::{CATEGORY, broke, clouds, count_words, files_of, targets};
use crate::builtin::geometry::{AREA_KINDS, features_param, geo_scopes};
use crate::files::Files;
use crate::text::js_number;
use crate::types::{
    Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, RunContext, RunResult,
    ScopeKind, Tool,
};

pub fn tool() -> Tool {
    Tool {
        id: "pointcloud.areaStats".into(),
        label: "Nokta bulutu alan sorgusu".into(),
        category: CATEGORY.into(),
        description: "Seçili her alanın içindeki bulut noktalarını sayar: yoğunluk, kotların en küçüğü, en büyüğü, ortalaması ve standart sapması, sınıfların sayıları; tablo olarak verir.".into(),
        help: Some([
            "Alanın delikleri ve parçaları hesaba girer, yaylı kenarlar kesindir; düz kenarın üstündeki nokta içeride sayılır.",
            "Yoğunluk noktaların alanın net alanına (delikler düşülmüş) bölümüdür. Standart sapma örneklemindir (n − 1).",
            "Tablo Panoya kopyala ve CSV olarak kaydet ile alınır; çizim değişmez.",
        ]
        .join("\n\n")),
        keywords: ["alan sorgusu", "yoğunluk", "istatistik", "nokta sayısı", "density", "statistics", "lidar"]
            .map(String::from)
            .to_vec(),
        aliases: ["BULUTALANSOR", "ALANSORBULUT"].map(String::from).to_vec(),
        icon: Some("pointCloudArea".into()),
        parameters: vec![
            ParamDef::new(
                "input",
                "Nokta bulutları",
                ParamKind::Features {
                    kinds: Some(vec!["pointcloud".to_owned()]),
                    scopes: Some(vec![ScopeKind::All, ScopeKind::Selection, ScopeKind::Layer]),
                    writes: false,
                },
            )
            .describe("Noktaları sayılacak bulutlar (birlikte)."),
            features_param(
                "areas",
                "Alanlar",
                &AREA_KINDS,
                geo_scopes(),
                "Her biri tablonun bir satırı olan alanlar.",
            ),
        ],
        outputs: vec![OutputDef::new("table", "Alan sorgusu tablosu", OutputKind::Table)],
        targets: targets(),
        validate: None,
        preview: None,
        run: Some(run),
    }
}

/// An area's name in the table: its label, else its first attribute's value, else its number.
fn name_of(e: &Entity) -> String {
    let b = e.base();
    if let Some(l) = b.label.as_deref().filter(|l| !l.trim().is_empty()) {
        return l.to_owned();
    }
    b.attrs
        .values()
        .find(|v| !v.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| format!("#{}", b.id))
}

/// A number with `d` decimals, a comma for the point.
fn fixed(v: Option<f64>, d: usize) -> String {
    v.map_or_else(String::new, |v| format!("{v:.d$}").replace('.', ","))
}

fn run(r: &Resolved<'_>, _cx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let files = match files_of(feedback) {
        Ok(f) => f,
        Err(e) => return *e,
    };
    let list = clouds(r, "input");
    if list.is_empty() {
        return RunResult::refused("Sayılacak nokta bulutu yok: bir bulut seçin.".to_owned());
    }
    let areas: Vec<(&Entity, Region)> = r
        .features("areas")
        .entities
        .iter()
        .filter_map(|e| Region::new(&areas_of_entity(&shape(e))).map(|g| (*e, g)))
        .collect();
    if areas.is_empty() {
        return RunResult::refused(
            "Sorgulanacak alan yok: kapalı alan, daire ya da kapalı eğri seçin.".to_owned(),
        );
    }
    match go(&*files, &list, areas, feedback) {
        Ok(result) => result,
        Err(why) => broke(why),
    }
}

fn go(
    files: &dyn Files,
    list: &[&kentos_contracts::PointCloudEntity],
    areas: Vec<(&Entity, Region)>,
    feedback: &mut dyn Feedback,
) -> Result<RunResult, String> {
    let sources: Vec<_> = list
        .iter()
        .flat_map(|c| c.cloud.sources.iter().cloned())
        .collect();
    let names: Vec<String> = areas.iter().map(|(e, _)| name_of(e)).collect();
    let ids: Vec<u32> = areas.iter().map(|(e, _)| e.base().id).collect();
    let mut q = AreaQuery::new(areas.into_iter().map(|(_, g)| g).collect());
    let total: u64 = sources.iter().map(|s| s.count).sum::<u64>().max(1);
    let mut seen = 0u64;
    let mut raw = Vec::new();
    for s in &sources {
        let mut read = files.open_cloud(s)?;
        let input = read.input().clone();
        q.file(input.head.scale, input.head.offset);
        loop {
            if feedback.canceled() {
                return Err(super::STOPPED.to_owned());
            }
            raw.clear();
            if !read.next(&mut raw)? {
                break;
            }
            q.feed(&input.layout, &raw, input.head.scale, input.head.offset);
            seen += (raw.len() / input.layout.len.max(1)) as u64;
            feedback.progress((seen as f64 / total as f64).min(1.0), "Noktalar sayılıyor");
        }
    }
    let rows = q.rows();
    let columns = [
        "Nesne",
        "Ad",
        "Nokta",
        "Alan (m²)",
        "Yoğunluk (nokta/m²)",
        "Z en küçük",
        "Z en büyük",
        "Z ortalama",
        "Z std. sapma",
        "Sınıflar",
    ];
    let table_rows: Vec<Vec<String>> = rows
        .iter()
        .zip(&names)
        .zip(&ids)
        .map(|((row, name), id)| {
            vec![
                format!("#{id}"),
                name.clone(),
                js_number(row.count as f64),
                fixed(Some(row.area), 2),
                fixed(row.density, 2),
                fixed(row.z_min, 3),
                fixed(row.z_max, 3),
                fixed(row.z_mean, 3),
                fixed(row.z_std, 3),
                row.classes
                    .iter()
                    .map(|(c, n)| format!("{c}: {n}"))
                    .collect::<Vec<_>>()
                    .join("; "),
            ]
        })
        .collect();
    let inside: u64 = rows.iter().map(|r| r.count).sum();
    Ok(RunResult {
        outputs: [(
            "table".to_owned(),
            json!({ "columns": columns, "rows": table_rows }),
        )]
        .into_iter()
        .collect(),
        summary: Some(format!(
            "{} alanın içinde {} nokta sayıldı.",
            rows.len(),
            count_words(inside)
        )),
        ..RunResult::default()
    })
}
