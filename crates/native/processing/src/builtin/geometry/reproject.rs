//! Koordinat sistemine dönüştür (the web's `builtin/geometry/reproject.ts`;
//! docs/adr/0201 §8): objects whose coordinates are in another system moved
//! vertex by vertex into the project's, the project's datum choices taken
//! (`crs::transform_in`, docs/adr/0168); arcs and circles first become chords
//! within 1 mm. A point keeps its elevation. An object with a vertex that has
//! no value in the project's system is not written, and the reason is said.

use kentos_geometry_core::crs::Unreached;
use kentos_geometry_core::ops::geoprocess::reproject::reproject;
use kentos_project::crs::{code, system, systems, title};
use kentos_project::systems::{choices, own};
use serde_json::json;

use super::{
    GEO_KINDS, attrs_of, features_param, geo_scopes, input_notes, layer_param, new_object, shapes,
};
use crate::types::{
    ChangeSet, EnumOption, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved,
    RunContext, RunResult, Target, Tool,
};

/// Why an object was not written, as the note says it; in the order the notes come.
const REASONS: [(Unreached, &str); 4] = [
    (Unreached::Outside, "izdüşümün dışında"),
    (Unreached::NoLink, "datumlar arasında yol yok"),
    (Unreached::NoGrid, "datum seçiminin ızgarası bu cihazda yok"),
    (Unreached::OutsideGrid, "ızgaranın dışında"),
];

pub fn tool() -> Tool {
    // The source systems: the registry's but the local one, as a sentence names them.
    let sources = systems()
        .iter()
        .filter(|s| !s.is_local())
        .map(|s| {
            let option = EnumOption::new(&s.srid.to_string(), &title(s));
            match &s.area {
                Some(area) => option.hint(area),
                None => option,
            }
        })
        .collect();
    Tool {
        id: "geometry.reproject".into(),
        label: "Koordinat sistemine dönüştür".into(),
        category: "geometry".into(),
        description: "Koordinatları başka bir sistemde olan nesneleri projenin koordinat sistemine dönüştürüp yeni katmana yazar.".into(),
        help: Some([
            "Kaynak sistem, nesnelerin koordinatlarının bugün hangi sistemde olduğudur (örneğin ED50 / TM30 ile alınmış eski bir pafta); hedef her zaman projenin sistemidir. Projenin datum seçimleri (Proje ayarları › Datum dönüşümleri) uygulanır.",
            "Köşeler tek tek dönüştürülür. Yay ve daire kenarları önce 1 mm içinde doğru parçalarına çevrilir, çünkü izdüşüm daireyi daire olarak korumaz. Noktanın kotu korunur; çizgi ve alanların köşe kotları taşınmaz.",
            "Bir köşesi projenin sisteminde değeri olmayan nesne (dilimin çok dışında, datumlar arasında yol ya da ızgara yok) yazılmaz ve nedeniyle söylenir. Projenin koordinat sistemi yoksa araç çalışmaz.",
        ]
        .join("\n\n")),
        keywords: [
            "koordinat sistemi", "dönüştür", "izdüşüm", "datum", "reproject", "project", "ED50",
            "TUREF", "WGS 84", "transform",
        ]
        .map(String::from)
        .to_vec(),
        aliases: ["IZDUSUMDEGISTIR", "REPROJECT"].map(String::from).to_vec(),
        icon: Some("geoReproject".into()),
        parameters: vec![
            features_param(
                "input",
                "Nesneler",
                &GEO_KINDS,
                geo_scopes(),
                "Koordinatları kaynak sistemde olan alanlar, çizgiler ve noktalar.",
            ),
            ParamDef::new("source", "Kaynak sistem", ParamKind::Choice { options: sources })
                .default_value(json!("4326"))
                .describe("Nesnelerin koordinatlarının sistemi; hedef projenin sistemidir."),
            layer_param("Dönüştürülen", "#AB4ABA"),
        ],
        outputs: vec![
            OutputDef::new("moved", "Dönüştürülen nesneler", OutputKind::Features),
            OutputDef::new("count", "Yazılan nesne sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run),
    }
}

fn run(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let settings = ctx.doc.settings();
    let Some(project) = own(settings).filter(|n| n.system.is_some()) else {
        return RunResult::refused(
            "Projenin koordinat sistemi yok; önce Proje ayarları'nda sistem seçin.".into(),
        );
    };
    let source = v.text("source");
    let Some((src, from)) = source
        .parse::<u32>()
        .ok()
        .and_then(system)
        .and_then(|s| Some((s, s.transform_system()?)))
    else {
        return RunResult::refused(format!("Kaynak sistem bilinmiyor: {source}."));
    };
    if src.srid == settings.srid {
        return RunResult::refused(
            "Kaynak sistem projenin sistemiyle aynı; dönüştürülecek bir şey yok.".into(),
        );
    }
    let Some(to) = project.system.as_ref() else {
        return RunResult::refused(
            "Projenin koordinat sistemi yok; önce Proje ayarları'nda sistem seçin.".into(),
        );
    };
    let input = &v.features("input").entities;
    input_notes(feedback, input, &[], true);
    feedback.progress(0.0, "Köşeler dönüştürülüyor");
    let picks = choices(settings);
    let layer = &v.layer("layer").id;
    let mut add = Vec::new();
    let mut arcs = 0usize;
    let mut errors: Vec<Unreached> = Vec::new();
    for (e, s) in input.iter().zip(shapes(input)) {
        let r = reproject(&s, &from, to, &picks);
        match r.shape {
            Some(shape) => {
                if r.chorded > 0 {
                    arcs += 1;
                }
                if let Some(object) = new_object(shape, layer, attrs_of(e)) {
                    add.push(object);
                }
            }
            None => errors.extend(r.error),
        }
    }
    if arcs > 0 {
        feedback.warn(format!(
            "{arcs} nesnenin yayları 1 mm içinde doğru parçalarına çevrildi."
        ));
    }
    for (why, text) in REASONS {
        let n = errors.iter().filter(|e| **e == why).count();
        if n > 0 {
            feedback.warn(format!(
                "{n} nesnenin bir köşesi dönüştürülemedi ({text}); yazılmadı."
            ));
        }
    }
    let count = add.len();
    RunResult {
        changes: Some(ChangeSet {
            add,
            ..ChangeSet::default()
        }),
        outputs: [("count".to_owned(), json!(count))].into_iter().collect(),
        summary: Some(format!(
            "{count} nesne dönüştürüldü: {} → {}.",
            code(src),
            project.code
        )),
        ..RunResult::default()
    }
}
