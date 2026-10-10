//! The three tools of Çok boyutlu veri (docs/adr/0243 §8–§10): their
//! parameters as the web's (`builtin/multidim/tools.ts`).

use serde_json::{Value, json};

use super::{MULTIDIM, run_calc, run_profile, run_series};
use crate::builtin::pointcloud::{add_param, output_param};
use crate::builtin::surface::{choice, result_layer, whole};
use crate::types::{
    NewLayerStyle, OutputDef, OutputKind, ParamDef, ParamKind, Returns, ScopeKind, Target, Tool,
    Values,
};

/// The lines a Kesit walks.
pub const LINE_KINDS: [&str; 6] = ["line", "polyline", "arc", "circle", "ellipse", "spline"];

/// A tool of the category, run on the desktop's other thread.
#[allow(clippy::too_many_arguments)]
fn tool(
    (id, label, icon): (&str, &str, &str),
    description: &str,
    help: &[&str],
    keywords: &[&str],
    aliases: &[&str],
    parameters: Vec<ParamDef>,
    outputs: Vec<OutputDef>,
    run: crate::types::RunFn,
) -> Tool {
    Tool {
        id: id.into(),
        label: label.into(),
        category: MULTIDIM.into(),
        description: description.into(),
        help: Some(help.join("\n\n")),
        keywords: keywords.iter().map(|k| (*k).to_owned()).collect(),
        aliases: aliases.iter().map(|a| (*a).to_owned()).collect(),
        icon: Some(icon.into()),
        parameters,
        outputs,
        targets: vec![Target::Worker],
        validate: None,
        preview: None,
        run: Some(run),
    }
}

/// One raster.
fn raster(description: &str) -> ParamDef {
    ParamDef::new(
        "input",
        "Raster",
        ParamKind::Features {
            kinds: Some(vec!["raster".to_owned()]),
            scopes: Some(vec![ScopeKind::Selection, ScopeKind::Layer]),
            writes: false,
        },
    )
    .describe(description)
}

fn draws(v: &Values) -> bool {
    v.get("draw").and_then(Value::as_bool).unwrap_or(false)
}

pub fn profile() -> Tool {
    let mut layer = result_layer("Kesit", "#B4572E");
    if let ParamKind::Layer {
        new_layer_style, ..
    } = &mut layer.kind
    {
        *new_layer_style = NewLayerStyle {
            color: Some("#B4572E".to_owned()),
            line_weight: Some(0.35),
            ..NewLayerStyle::default()
        };
    }
    let p = vec![
        raster("Değerleri okunan raster: ızgara, NetCDF dilimi ya da mesh."),
        ParamDef::new(
            "lines",
            "Çizgiler",
            ParamKind::Features {
                kinds: Some(LINE_KINDS.iter().map(|k| (*k).to_owned()).collect()),
                scopes: Some(vec![ScopeKind::Selection, ScopeKind::Layer]),
                writes: false,
            },
        )
        .describe("Kesitin çizgileri: çizgi, çoklu çizgi, yay, daire, elips ya da eğri; her biri başından yürünür."),
        ParamDef::new(
            "step",
            "Adım",
            ParamKind::Number {
                min: Some(1e-6),
                max: Some(1e9),
                integer: false,
                unit: "m".into(),
                placeholder: Some("Hücre boyu".into()),
            },
        )
        .optional()
        .default_value(Value::Null)
        .describe("Çizginin başından bu aralıklarla ve sonunda nokta; boşsa rasterin hücre boyu."),
        whole("band", "Bant", 1, 1, 255).describe("Çok bantlı rasterde değerin okunduğu bant (1'den)."),
        ParamDef::new("draw", "Kotlu çizgi", ParamKind::Boolean)
            .default_value(json!(false))
            .describe("Değerler köşe kotu olan çoklu çizgiler yeni katmana eklenir; değersiz noktalar çizgiyi böler."),
        layer.shown_when(draws),
    ];
    tool(
        ("multidim.profile", "Kesit", "multidimProfile"),
        "Rasterin değerlerini çizgiler boyunca, Adım aralıklarla okur: tablo ve isteğe bağlı kotlu çizgiler.",
        &[
            "Her çizginin başından Adım aralıklarla ve sonunda bir nokta; yay, daire, elips ve eğri kendi eğrileriyle yürünür. Değer noktanın düştüğü hücrenin değeridir; mesh rasterde ağdan enterpolasyondur (pikselin değil). NetCDF ve mesh rasterde gösterilen dilim okunur.",
            "Tablo: Çizgi, Uzaklık (m), koordinatlar ve Değer; panoya kopyalanır ya da CSV olarak kaydedilir. Kotlu çizgi açıksa değerler köşe kotu olan çoklu çizgiler rasterin katmanının hemen üstündeki yeni katmana eklenir.",
        ],
        &[
            "kesit",
            "profil",
            "boyuna kesit",
            "profile",
            "cross section",
            "transect",
        ],
        &["KESIT", "PROFIL", "BOYKESIT"],
        p,
        vec![OutputDef::new("table", "Kesit", OutputKind::Table)],
        run_profile,
    )
}

pub fn series() -> Tool {
    let p = vec![
        raster("Zaman boyutlu veri seti (NetCDF ya da mesh) ya da çok bantlı raster."),
        ParamDef::new(
            "points",
            "Noktalar",
            ParamKind::Features {
                kinds: Some(vec!["point".to_owned()]),
                scopes: Some(vec![ScopeKind::Selection, ScopeKind::Layer]),
                writes: false,
            },
        )
        .describe("Değerlerin okunduğu noktalar; çok noktalı nesnenin her noktası. Sütunun adı noktanın ad özniteliği."),
    ];
    tool(
        ("multidim.series", "Zaman serisi", "timeSeries"),
        "Noktalardaki değerleri veri setinin her zaman adımında (ya da rasterin her bandında) okur: tablo.",
        &[
            "Zaman boyutlu veri setinde öbür boyutlar rasterin gösterdiği değerlerde kalır; yalnız noktaların değerleri okunur. Zaman boyutu olmayan çok bantlı rasterde her bant bir satırdır. Değer hücrenin değeridir; mesh rasterde ağdan enterpolasyondur.",
            "Tablo: satırlar adımlar (Adım ve Zaman, ya da Bant), sütunlar noktalar; panoya kopyalanır ya da CSV olarak kaydedilir. Çizim değişmez.",
        ],
        &[
            "zaman serisi",
            "zaman",
            "seri",
            "time series",
            "netcdf",
            "mesh",
        ],
        &["ZAMANSERISI", "ZSERI"],
        p,
        vec![OutputDef::new("table", "Zaman serisi", OutputKind::Table)],
        run_series,
    )
}

pub fn calculator() -> Tool {
    let mut p = vec![
        raster("Mesh ekle ile eklenen mesh raster; ifadenin veri setleri onun ağındandır."),
        ParamDef::new(
            "expression",
            "İfade",
            ParamKind::Expression {
                returns: Returns::Value,
                of: None,
                placeholder: Some("[depth] * 2".into()),
            },
        )
        .describe("Veri setleri değişken adlarıyla ya da gösterilen adlarıyla ([Su derinliği]); vektörün adı büyüklüğü."),
        choice(
            "summary",
            "Zaman özeti",
            &[
                ("none", "Yok"),
                ("max", "En büyük"),
                ("min", "En küçük"),
                ("mean", "Ortalama"),
                ("sum", "Toplam"),
            ],
        )
        .describe("Yok: her zaman adımı yazılır; öbürleri adımları birleştirir (değersizler atlanır)."),
        ParamDef::new(
            "name",
            "Veri setinin adı",
            ParamKind::Text {
                placeholder: Some("Hesap".into()),
                max_length: Some(256),
                allow_empty: false,
            },
        )
        .default_value(json!("Hesap")),
        output_param("-hesap", &[".nc"]),
        add_param(),
        result_layer("Mesh hesabı", "#7A6B5B"),
    ];
    if let Some(o) = p.iter_mut().find(|x| x.name == "output") {
        o.description = Some("Boş bırakılırsa kaynağın yanına, adının sonuna -hesap eklenerek yazılır (UGRID NetCDF).".into());
    }
    tool(
        (
            "multidim.meshCalculator",
            "Mesh hesaplayıcı",
            "meshCalculator",
        ),
        "Mesh'in veri setlerinden ifadeyle yeni veri seti yazar: aynı ağda, yeni UGRID dosyasında.",
        &[
            "İfadenin andığı veri setleri aynı konumda (düğümlerde ya da yüzlerde) olmalı; zamanlı olanlar aynı zaman adımlarında, zamansızlar her adımda geçerlidir; katmanlı veri seti okunmaz. Değersiz girdi değersiz sonuç verir; etkin olmayan yüzün değeri yoktur.",
            "Örnekler: [depth] * 2; [Su derinliği] + bed; durum eğer depth > 0,5 ise 1 yoksa 0 son. Matematik işlevleri Raster hesaplayıcı'nınkiler.",
            "Sonuç kaynağın ağı, zamanı (özetle yok olur) ve yeni veri setiyle 32 bit ondalık UGRID NetCDF'tir; Çizime ekle açıksa onu gösteren mesh raster kaynağın katmanının hemen üstündeki yeni katmana eklenir.",
        ],
        &[
            "mesh",
            "hesap",
            "ifade",
            "veri seti",
            "mesh calculator",
            "ugrid",
        ],
        &["MESHHESAP", "AGHESAP"],
        p,
        vec![OutputDef::new("file", "Sonuç dosyası", OutputKind::Text)],
        run_calc,
    )
}
