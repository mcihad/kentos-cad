//! Alt bölümlere ayır (the web's `subdivide.ts`; docs/adr/0220 §6): areas
//! and polylines cut into pieces of at most so many vertices, PostGIS's
//! `ST_Subdivide` on curved geometry, each piece with its object's
//! attributes, its number and its elevations.

use kentos_geometry_core::ops::sampling::subdivide::{LEAST, subdivide};
use kentos_native_application::geometry::shape;
use serde_json::json;

use super::super::convert::{placed, zs_of};
use super::super::geometry::layer_param;
use super::{scopes, written};
use crate::types::{
    Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, RunContext, RunResult, Target,
    Tool,
};

pub fn tool() -> Tool {
    Tool {
        id: "sample.subdivide".into(),
        label: "Alt bölümlere ayır".into(),
        category: "sampling".into(),
        description: "Çok köşeli alanları ve çoklu çizgileri en çok verilen köşe sayısı olan parçalara ayırıp yeni katmana yazar.".into(),
        help: Some([
            "Örnekler: binlerce köşeli bir il ya da havza sınırını hızlı sorgulanan parçalara ayırmak, uzun eş yükselti eğrilerini bölmek.",
            "Parça köşe sayısını aşıyorsa kutusunun uzun kenarına dik ikiye kesilir; kesim halkanın kutunun ortasına en yakın köşesinden geçer (delikler köşelerin çoğunu tutuyorsa en büyük deliğinkinden). Parçalar yeniden ele alınır, en çok 50 kat. Yaylar kesin kesilir.",
            "Her parça nesnenin özniteliklerini ve Parça numarasını (kutusunun sol alt köşesine göre: önce batı, sonra güney) alır; köşelerin kotları taşınır. Köşe sayısını aşmayan nesne tek parça yazılır.",
        ]
        .join("\n\n")),
        keywords: [
            "alt bölüm", "parçala", "köşe", "subdivide", "dizin", "bölme", "parça",
        ]
        .map(String::from)
        .to_vec(),
        aliases: ["ALTBOLUM", "SUBDIVIDE"].map(String::from).to_vec(),
        icon: Some("subdivideParts".into()),
        parameters: vec![
            ParamDef::new(
                "input",
                "Nesneler",
                ParamKind::Features {
                    kinds: Some(vec!["polygon".into(), "polyline".into()]),
                    scopes: scopes(),
                    writes: false,
                },
            )
            .describe("Ayrılacak alanlar ve çoklu çizgiler."),
            ParamDef::new(
                "max",
                "En çok köşe",
                ParamKind::Number {
                    min: Some(LEAST as f64),
                    max: None,
                    integer: true,
                    unit: String::new(),
                    placeholder: None,
                },
            )
            .default_value(json!(256))
            .describe("Bir parçanın en çok köşe sayısı (halkaların ve deliklerin köşeleri birlikte)."),
            layer_param("Alt bölümler", "#30A46C"),
        ],
        outputs: vec![
            OutputDef::new("pieces", "Yazılan parçalar", OutputKind::Features),
            OutputDef::new("count", "Parça sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run),
    }
}

fn run(v: &Resolved<'_>, _ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let inputs = &v.features("input").entities;
    let max = v.number("max").unwrap_or(256.0);
    let max = if max >= 1.0 { max as usize } else { 1 };
    feedback.progress(0.0, "Parçalar kesiliyor");
    let layer = &v.layer("layer").id;
    let mut add = Vec::new();
    let mut whole = 0usize;
    for e in inputs {
        let pieces = subdivide(&shape(e), zs_of(e).as_ref(), max);
        if pieces.len() == 1 {
            whole += 1;
        }
        for (k, piece) in pieces.into_iter().enumerate() {
            let mut attrs = e.base().attrs.clone();
            attrs.insert("Parça".to_owned(), (k + 1).to_string());
            add.extend(placed(piece, layer, attrs));
        }
    }
    if whole > 0 {
        feedback.info(format!(
            "{whole} nesne en çok köşeyi aşmadığı için tek parça yazıldı."
        ));
    }
    let summary = format!("{} nesneden {} parça yazıldı.", inputs.len(), add.len());
    written(add, summary)
}
