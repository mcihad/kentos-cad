//! The ten tools of Mekânsal istatistik (the web's `builtin/stats/tools.ts`;
//! docs/adr/0238): the same names, parameters, defaults and texts; Moran I and
//! Sıcak nokta's permutations and correction and Yerel Moran (docs/adr/0520)
//! are the desktop's only while the web pauses (ADR 0228).

use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::ops::sampling::random::seed_of;
use kentos_geometry_core::ops::spatial_stats::autocorrelation::{Neighbourhood, morans_i_with};
use kentos_geometry_core::ops::spatial_stats::centers::{CenterInput, CenterKind, centers};
use kentos_geometry_core::ops::spatial_stats::clusters::{dbscan, k_means};
use kentos_geometry_core::ops::spatial_stats::inference::{Correction, Inference};
use kentos_geometry_core::ops::spatial_stats::local::{hot_spots_with, local_morans};
use kentos_geometry_core::ops::spatial_stats::nearest::nearest;
use kentos_geometry_core::ops::spatial_stats::weights::Concept;
use serde_json::{Value, json};

use super::{GEOGRAPHIC, geographic, result_of, scope_of, tested_result_of};
use crate::builtin::geometry::{GEO_KINDS, features_param, geo_scopes, layer_param, shapes};
use crate::builtin::queries::attr;
use crate::types::{
    EnumOption, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, RunContext,
    RunResult, Target, Tool, Values,
};
use kentos_contracts::{PointStyle, PointSymbol};

fn input(description: &str) -> ParamDef {
    features_param("input", "Nesneler", &GEO_KINDS, geo_scopes(), description)
}

fn field(name: &str, label: &str, optional: bool, description: &str) -> ParamDef {
    let p = ParamDef::new(
        name,
        label,
        ParamKind::Field {
            of: vec!["input".into()],
            allow_new: false,
            multiple: false,
        },
    )
    .describe(description);
    if optional { p.optional() } else { p }
}

fn weight_field() -> ParamDef {
    field(
        "weightField",
        "Ağırlık alanı",
        true,
        "Seçilirse her nesne bu alanın değeriyle ağırlıklıdır; okunamayan ve eksi değerli nesne alınmaz.",
    )
}

fn group_field() -> ParamDef {
    field(
        "groupField",
        "Grup alanı",
        true,
        "Seçilirse her grubun sonucu ayrı yazılır; boş değerliler “(boş)” grubudur.",
    )
}

fn value_field() -> ParamDef {
    field(
        "valueField",
        "Değer alanı",
        false,
        "Değerleri sayı olarak okunan alan; okunamayan nesne alınmaz.",
    )
}

fn deviations() -> ParamDef {
    ParamDef::new(
        "deviations",
        "Standart sapma",
        ParamKind::Choice {
            options: vec![
                EnumOption::new("1", "1 kat").hint("Noktaların yaklaşık %63’ü (elipste)"),
                EnumOption::new("2", "2 kat").hint("Yaklaşık %98"),
                EnumOption::new("3", "3 kat").hint("Yaklaşık %99,9"),
            ],
        },
    )
    .default_value(json!("1"))
    .describe("Sonucun kaç standart sapma boyunda yazılacağı.")
}

fn concepts(inverse: bool) -> ParamDef {
    let mut options = vec![
        EnumOption::new("band", "Sabit bant").hint("Bant içindeki her nesne komşu, ağırlığı 1"),
    ];
    if inverse {
        options.push(
            EnumOption::new("inverse", "Ters uzaklık")
                .hint("Bant içinde ağırlık 1 / uzaklık; 1 m’den yakın 1 m sayılır"),
        );
    }
    options.push(
        EnumOption::new("nearest", "k en yakın")
            .hint("Her nesnenin en yakın k komşusu, ağırlığı 1"),
    );
    ParamDef::new("concept", "Komşuluk", ParamKind::Choice { options })
        .default_value(json!("band"))
        .describe("Nesnelerin komşularının nasıl bulunacağı.")
}

fn number(
    name: &str,
    label: &str,
    min: Option<f64>,
    max: Option<f64>,
    integer: bool,
    unit: &str,
) -> ParamDef {
    ParamDef::new(
        name,
        label,
        ParamKind::Number {
            min,
            max,
            integer,
            unit: unit.into(),
            placeholder: None,
        },
    )
}

fn band() -> ParamDef {
    number("band", "Bant", Some(0.001), None, false, "m")
        .optional()
        .default_value(Value::Null)
        .placeholder("Kendiliğinden")
        .shown_when(|v| v.get("concept").and_then(Value::as_str) != Some("nearest"))
        .describe("Boş bırakılırsa en büyük en yakın komşu uzaklığı: her nesnenin en az bir komşusu olur.")
}

fn neighbors() -> ParamDef {
    number(
        "neighbors",
        "Komşu sayısı (k)",
        Some(1.0),
        Some(100.0),
        true,
        "",
    )
    .default_value(json!(8))
    .shown_when(|v| v.get("concept").and_then(Value::as_str) == Some("nearest"))
    .describe("Her nesnenin en yakın kaç komşusu; eşit uzaklıkta önce gelen.")
}

fn permuting(v: &Values) -> bool {
    v.get("inference").and_then(Value::as_str) == Some("permutation")
}

/// Çıkarım (docs/adr/0520 §2): the normal distribution's p, or the permutations'.
fn inference_param(conditional: bool) -> ParamDef {
    ParamDef::new(
        "inference",
        "Çıkarım",
        ParamKind::Choice {
            options: vec![
                EnumOption::new("analytic", "Normal dağılım")
                    .hint("z ve p rastgele dağıtım varsayımıyla, formülden"),
                EnumOption::new("permutation", "Permütasyon").hint(if conditional {
                    "Her nesnenin değeri yerinde kalır, komşularınınkiler rastgele dağıtılır; p dağılımdan"
                } else {
                    "Değerler yerlere rastgele dağıtılır; p dağılımdan, tohumla tekrarlanır"
                }),
            ],
        },
    )
    .default_value(json!("analytic"))
    .describe("p değerinin nasıl bulunacağı; permütasyon dağılım varsayımı istemez.")
}

fn permutations_param(always: bool) -> ParamDef {
    let p = number(
        "permutations",
        "Permütasyon sayısı",
        Some(9.0),
        Some(99_999.0),
        true,
        "",
    )
    .default_value(json!(999))
    .describe("p'nin en küçüğü 1 / (sayı + 1): 999 permütasyonla 0,001.");
    if always { p } else { p.shown_when(permuting) }
}

fn seed_param(always: bool) -> ParamDef {
    let p = number("seed", "Tohum", Some(0.0), None, true, "")
        .default_value(json!(1))
        .describe("Aynı tohum aynı permütasyonları, dolayısıyla aynı p değerlerini verir.");
    if always { p } else { p.shown_when(permuting) }
}

/// Çoklu test düzeltmesi (docs/adr/0520 §4): a local statistic tests every object.
fn correction_param() -> ParamDef {
    ParamDef::new(
        "correction",
        "Çoklu test düzeltmesi",
        ParamKind::Choice {
            options: vec![
                EnumOption::new("none", "Yok").hint("Her nesnenin p'si kendi başına"),
                EnumOption::new("fdr", "FDR")
                    .hint("Benjamini–Hochberg: anlamlı sayılanlarda yanlışların beklenen oranı sınırlı"),
                EnumOption::new("bonferroni", "Bonferroni")
                    .hint("Her p sınama sayısıyla çarpılır; en tutucu"),
            ],
        },
    )
    .default_value(json!("none"))
    .describe("Çok sayıda nesne sınandığında rastlantıyla anlamlı çıkanları ayıklar; sınıf düzeltilmiş p'den.")
}

/// The inference the values ask for; `always`: the permutations whatever Çıkarım says (Yerel Moran).
fn inference_of(v: &Resolved<'_>, always: bool) -> Inference {
    let permuted = always || v.text("inference") == "permutation";
    Inference {
        permutations: if permuted {
            v.number("permutations").unwrap_or(999.0).max(0.0) as usize
        } else {
            0
        },
        seed: seed_of(v.number("seed").unwrap_or(1.0)),
        correction: Correction::from_key(v.text("correction")).unwrap_or(Correction::None),
    }
}

/// An output layer, right above the input's layer: the results over the objects they come from.
fn output_layer(name: &str, color: &str) -> ParamDef {
    let mut p = layer_param(name, color);
    if let ParamKind::Layer { above, .. } = &mut p.kind {
        *above = Some("input".to_owned());
    }
    p
}

/// A centre's output layer: its points drawn as crosses, apart from the places it sums.
fn center_layer(name: &str, color: &str) -> ParamDef {
    let mut p = output_layer(name, color);
    if let ParamKind::Layer {
        new_layer_style, ..
    } = &mut p.kind
    {
        new_layer_style.point = Some(PointStyle {
            symbol: PointSymbol::Cross,
            size: 12.0,
        });
    }
    p
}

/// Sıcak noktalar' output layer: its classes' look (`hot_renderer`).
fn hot_layer() -> ParamDef {
    let mut p = output_layer("Sıcak noktalar", "#E03131");
    if let ParamKind::Layer {
        new_layer_style, ..
    } = &mut p.kind
    {
        new_layer_style.renderer = Some(super::hot_renderer());
    }
    p
}

/// Yerel Moran's output layer: its cluster types' look (`lisa_renderer`).
fn lisa_layer() -> ParamDef {
    let mut p = output_layer("Yerel Moran", "#D7191C");
    if let ParamKind::Layer {
        new_layer_style, ..
    } = &mut p.kind
    {
        new_layer_style.renderer = Some(super::lisa_renderer());
    }
    p
}

/// A clustering's output layer: its clusters' look (`cluster_renderer`).
fn cluster_layer(name: &str) -> ParamDef {
    let mut p = output_layer(name, "#F08C00");
    if let ParamKind::Layer {
        new_layer_style, ..
    } = &mut p.kind
    {
        new_layer_style.renderer = Some(super::cluster_renderer());
    }
    p
}

fn table_outputs(name: &str, label: &str) -> Vec<OutputDef> {
    vec![
        OutputDef::new("table", "Sonuç tablosu", OutputKind::Table),
        OutputDef::new(name, label, OutputKind::Number),
        OutputDef::new("z", "z", OutputKind::Number),
        OutputDef::new("p", "p", OutputKind::Number),
    ]
}

const CENTER_HELP: [&str; 3] = [
    "Nesnenin yeri ağırlık merkezidir: noktanın kendisi (çok noktalının ortalaması), alanın, dairenin ve bütün elipsin ağırlık merkezi, çizginin ortası, çoklu çizginin ortadaki köşesi; ifadelerin $merkez_y ve $merkez_x’i.",
    "Ağırlık alanı seçilirse değerler sayı olarak okunur (ondalık nokta ya da virgül); okunamayan, boş ve eksi değerli nesneler alınmaz ve söylenir. Grup alanı seçilirse gruplar adlarının doğal sırasıyla, boş değerliler “(boş)” grubu olarak en sonda yazılır.",
    "Coğrafi koordinatlı projede çalışmaz: uzaklıklar metre olmalıdır.",
];

fn center_help(first: &[&str]) -> String {
    first
        .iter()
        .copied()
        .chain(CENTER_HELP)
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn words(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_owned()).collect()
}

#[allow(clippy::too_many_arguments)]
fn tool(
    id: &str,
    label: &str,
    icon: &str,
    description: &str,
    help: String,
    keywords: &[&str],
    aliases: &[&str],
    parameters: Vec<ParamDef>,
    outputs: Vec<OutputDef>,
    run: fn(&Resolved<'_>, &RunContext<'_>, &mut dyn Feedback) -> RunResult,
) -> Tool {
    Tool {
        id: id.into(),
        label: label.into(),
        category: "spatialStats".into(),
        description: description.into(),
        help: Some(help),
        keywords: words(keywords),
        aliases: words(aliases),
        icon: Some(icon.into()),
        parameters,
        outputs,
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run),
    }
}

fn shape_list(v: &Resolved<'_>) -> Vec<Shape> {
    shapes(&v.features("input").entities)
}

fn texts(v: &Resolved<'_>, name: &str) -> Vec<Option<String>> {
    v.features("input")
        .entities
        .iter()
        .map(|e| attr(e, name).map(str::to_owned))
        .collect()
}

/// A centres tool's run: the weights and groups when their fields are chosen.
fn run_centers(
    kind: CenterKind,
    v: &Resolved<'_>,
    ctx: &RunContext<'_>,
    feedback: &mut dyn Feedback,
) -> RunResult {
    if geographic(ctx) {
        return RunResult::refused(GEOGRAPHIC.into());
    }
    let w = v.text("weightField");
    let g = v.text("groupField");
    let weights = (!w.is_empty()).then(|| texts(v, w));
    let groups = (!g.is_empty()).then(|| texts(v, g));
    let k = match kind {
        CenterKind::Distance | CenterKind::Ellipse => v.text("deviations").parse().unwrap_or(1.0),
        _ => 1.0,
    };
    let input = CenterInput {
        weights: weights.as_deref(),
        groups: groups.as_deref(),
        weight_field: w,
        k,
    };
    match centers(&shape_list(v), kind, &input) {
        Ok(run) => result_of(
            run,
            feedback,
            &v.features("input").entities,
            Some(&v.layer("layer").id),
        ),
        Err(why) => RunResult::refused(why),
    }
}

fn hood(v: &Resolved<'_>) -> Neighbourhood {
    Neighbourhood {
        concept: Concept::from_key(v.text("concept")).unwrap_or(Concept::Band),
        band: v.number("band"),
        k: v.number("neighbors").unwrap_or(8.0).max(1.0) as usize,
    }
}

pub fn mean_center() -> Tool {
    tool(
        "stats.meanCenter",
        "Ortalama merkez",
        "statsMeanCenter",
        "Nesnelerin (isterseniz ağırlıklı) ortalama merkezini, gruplara göre de, nokta olarak yazar.",
        center_help(&[]),
        &[
            "ortalama merkez",
            "merkez",
            "ağırlık merkezi",
            "mean center",
            "mean coordinates",
            "centroid",
            "istatistik",
        ],
        &["ORTALAMAMERKEZ", "MEANCENTER"],
        vec![
            input("Merkezi bulunacak noktalar, çizgiler ve alanlar."),
            weight_field(),
            group_field(),
            center_layer("Ortalama merkez", "#D6336C"),
        ],
        vec![
            OutputDef::new("centers", "Merkezler", OutputKind::Features),
            OutputDef::new("count", "Merkez sayısı", OutputKind::Number),
        ],
        |v, ctx, f| run_centers(CenterKind::Mean, v, ctx, f),
    )
}

pub fn median_center() -> Tool {
    tool(
        "stats.medianCenter",
        "Ortanca merkez",
        "statsMedianCenter",
        "Nesnelere uzaklıklarının (isterseniz ağırlıklı) toplamını en küçük yapan noktayı, gruplara göre de, yazar.",
        center_help(&[
            "Ortanca merkez (geometrik ortanca) aykırı nesnelerden ortalama merkez kadar etkilenmez. Weiszfeld’in yinelemesiyle ortalama merkezden başlanarak bulunur; ortanca bir nesnenin yerindeyse o yer yazılır.",
        ]),
        &[
            "ortanca merkez",
            "medyan merkez",
            "geometrik ortanca",
            "median center",
            "weber",
            "istatistik",
        ],
        &["ORTANCAMERKEZ", "MEDIANCENTER"],
        vec![
            input("Merkezi bulunacak noktalar, çizgiler ve alanlar."),
            weight_field(),
            group_field(),
            center_layer("Ortanca merkez", "#7048E8"),
        ],
        vec![
            OutputDef::new("centers", "Merkezler", OutputKind::Features),
            OutputDef::new("count", "Merkez sayısı", OutputKind::Number),
        ],
        |v, ctx, f| run_centers(CenterKind::Median, v, ctx, f),
    )
}

pub fn standard_distance() -> Tool {
    tool(
        "stats.standardDistance",
        "Standart uzaklık",
        "statsStandardDistance",
        "Nesnelerin ortalama merkezin çevresinde ne kadar yayıldığını, standart uzaklık yarıçaplı daire olarak yazar.",
        center_help(&[
            "Standart uzaklık, nesnelerin ortalama merkeze uzaklıklarının karelerinin (ağırlıklı) ortalamasının köküdür. Daire Standart sapma kadar katıyla çizilir; tek nesneli ya da çakışık nesneli grubunki sıfırdır, yazılmaz.",
        ]),
        &[
            "standart uzaklık",
            "yayılım",
            "dağılım",
            "standard distance",
            "dispersion",
            "istatistik",
        ],
        &["STANDARTUZAKLIK", "STANDARDDISTANCE"],
        vec![
            input("Yayılımı ölçülecek noktalar, çizgiler ve alanlar."),
            weight_field(),
            group_field(),
            deviations(),
            output_layer("Standart uzaklık", "#1C7ED6"),
        ],
        vec![
            OutputDef::new("circles", "Daireler", OutputKind::Features),
            OutputDef::new("count", "Daire sayısı", OutputKind::Number),
        ],
        |v, ctx, f| run_centers(CenterKind::Distance, v, ctx, f),
    )
}

pub fn directional_distribution() -> Tool {
    tool(
        "stats.directionalDistribution",
        "Yön dağılımı",
        "statsEllipse",
        "Nesnelerin hangi doğrultuda yayıldığını standart sapma elipsi olarak yazar: büyük ve küçük yarı eksenleri, doğrultusu.",
        center_help(&[
            "Elipsin eksenleri koordinatların (ağırlıklı) kovaryansının özvektörleridir; yarı eksenler √2 düzeltmesiyle yazılır, bir standart sapmalık elips noktaların yaklaşık %63’ünü alır. Doğrultu büyük eksenin kuzeyden saat yönünde açısıdır (0–180°).",
            "Nesneleri bir doğru üzerinde ya da çakışık olan grubun elipsi yazılmaz.",
        ]),
        &[
            "yön dağılımı",
            "standart sapma elipsi",
            "elips",
            "directional distribution",
            "standard deviational ellipse",
            "istatistik",
        ],
        &["YONDAGILIMI", "DIRECTIONALDISTRIBUTION"],
        vec![
            input("Doğrultusu bulunacak noktalar, çizgiler ve alanlar."),
            weight_field(),
            group_field(),
            deviations(),
            output_layer("Yön dağılımı", "#0CA678"),
        ],
        vec![
            OutputDef::new("ellipses", "Elipsler", OutputKind::Features),
            OutputDef::new("count", "Elips sayısı", OutputKind::Number),
        ],
        |v, ctx, f| run_centers(CenterKind::Ellipse, v, ctx, f),
    )
}

pub fn nearest_neighbor() -> Tool {
    tool(
        "stats.nearestNeighbor",
        "En yakın komşu",
        "statsNearest",
        "Nesnelerin en yakın komşularına ortalama uzaklığını rastgele dağılımınkiyle karşılaştırır: kümelenmiş mi, dağınık mı?",
        [
            "Gözlenen ortalama en yakın komşu uzaklığı aynı alanda rastgele dağılmış aynı sayıda nesnenin beklenen uzaklığıyla (0,5 / √(n / A)) karşılaştırılır (Clark ve Evans 1954). Oran 1’den küçük ve z eksiyse kümelenme, büyük ve artıysa dağınıklık vardır; p 0,05’ten küçükse desen anlamlıdır.",
            "Alan boş bırakılırsa nesnelerin yerlerini çevreleyen kutunun alanıdır; sonuç alana duyarlıdır, çalışma alanını biliyorsanız yazın. Çizim değişmez.",
        ]
        .join("\n\n"),
        &["en yakın komşu", "kümelenme", "nearest neighbor", "average nearest neighbor", "clark evans", "desen", "istatistik"],
        &["ENYAKINKOMSU", "NEARESTNEIGHBOR"],
        vec![
            input("Deseni sınanacak noktalar, çizgiler ve alanlar."),
            number("area", "Alan", Some(0.001), None, false, "m²")
                .optional()
                .default_value(Value::Null)
                .placeholder("Kutudan")
                .describe("Çalışma alanı; boş bırakılırsa yerlerin kutusunun alanı."),
        ],
        table_outputs("ratio", "En yakın komşu oranı"),
        |v, ctx, f| {
            if geographic(ctx) {
                return RunResult::refused(GEOGRAPHIC.into());
            }
            match nearest(&shape_list(v), v.number("area")) {
                Ok(run) => result_of(run, f, &v.features("input").entities, None),
                Err(why) => RunResult::refused(why),
            }
        },
    )
}

pub fn morans() -> Tool {
    tool(
        "stats.moransI",
        "Moran I",
        "statsMoran",
        "Bir değerin komşu nesnelerde benzer olup olmadığını Moran I ile sınar: kümelenmiş mi, dağınık mı, rastgele mi?",
        [
            "Moran I artıysa benzer değerler yan yana (kümelenme), eksiyse farklılar yan yana (dağınıklık); beklenen değer −1 / (n − 1). z ve p, rastgele dağıtım varsayımıyla bulunur (Cliff ve Ord); p 0,05’ten küçükse desen anlamlıdır.",
            "Komşuluk: sabit uzaklık bandı, bant içinde ters uzaklık ya da k en yakın komşu. Bant boş bırakılırsa her nesnenin en az bir komşusu olacak en küçük uzaklık alınır. Satır standartlaştırma her nesnenin komşularının ağırlıklarını toplamına böler. Çizim değişmez.",
            "Çıkarım Permütasyon ise değerler yerlere tohumdan gelen sırayla rastgele dağıtılır ve her dağıtımın I'sı bulunur; p, gözlenen I kadar uç olanların payıdır ((sayı + 1) / (permütasyon + 1)), z gözlenenin dağılımın ortalamasından standart sapma cinsinden uzaklığıdır. Desen permütasyonun z ve p'sinden; aynı tohum ve permütasyon sayısı aynı sonucu verir. Deney profili tohumu, permütasyon sayısını, komşuluğu ve girdinin SHA-256 özetini yazar; seçili ya da görünen nesnelerle yapılan sınama alt küme diye söylenir.",
        ]
        .join("\n\n"),
        &["moran", "otokorelasyon", "mekânsal otokorelasyon", "spatial autocorrelation", "global moran", "kümelenme", "istatistik"],
        &["MORAN", "MORANI"],
        vec![
            input("Değeri sınanacak noktalar, çizgiler ve alanlar."),
            value_field(),
            concepts(true),
            band(),
            neighbors(),
            ParamDef::new("standardize", "Satır standartlaştırma", ParamKind::Boolean)
                .default_value(json!(true))
                .describe("Her nesnenin komşu ağırlıkları toplamına bölünür (önerilen)."),
            inference_param(false),
            permutations_param(false),
            seed_param(false),
        ],
        {
            let mut out = table_outputs("moransI", "Moran I");
            out.push(OutputDef::new("zPermutation", "Permütasyon z", OutputKind::Number));
            out.push(OutputDef::new("pPermutation", "Permütasyon p", OutputKind::Number));
            out.push(OutputDef::new("profile", "Deney profili", OutputKind::Table));
            out.push(OutputDef::new("experiment", "Deney profili (JSON)", OutputKind::Text));
            out
        },
        |v, ctx, f| {
            if geographic(ctx) {
                return RunResult::refused(GEOGRAPHIC.into());
            }
            let field = v.text("valueField");
            let scope = scope_of(v, ctx);
            match morans_i_with(
                &shape_list(v),
                &texts(v, field),
                field,
                hood(v),
                v.flag("standardize"),
                inference_of(v, false),
            ) {
                Ok(run) => tested_result_of(run, f, &v.features("input").entities, None, &scope),
                Err(why) => RunResult::refused(why),
            }
        },
    )
}

pub fn hot_spot() -> Tool {
    tool(
        "stats.hotSpot",
        "Sıcak nokta (Gi*)",
        "statsHotSpot",
        "Yüksek ve düşük değerlerin kümelendiği yerleri Getis-Ord Gi* ile bulur; her nesnenin kopyasını z, p ve güven sınıfıyla renkli yazar.",
        [
            "Her nesnenin ve komşularının değer toplamı, bütün nesnelerden beklenenle karşılaştırılır (Ord ve Getis 1995); sonuç bir z puanıdır. Büyük artı z sıcak nokta (yüksek değerler kümesi), büyük eksi z soğuk noktadır.",
            "Güven sınıfı z’nin işaretiyle 3 (%99), 2 (%95), 1 (%90) ya da 0’dır; kopyalar sınıflarının rengini alır (kırmızılar sıcak, maviler soğuk). Komşuluk sabit uzaklık bandı ya da k en yakın komşu; nesne kendi komşusudur. Bant boş bırakılırsa her nesnenin en az bir komşusu olacak en küçük uzaklık alınır.",
            "Çıkarım Permütasyon ise her nesnenin değeri yerinde kalır, komşularının değerleri öbür nesnelerinkilerden tohumdan gelen sırayla rastgele çekilir (koşullu permütasyon); Permütasyon p ve z yazılır, sınıf onlardan. Her nesne ayrı sınandığından rastlantıyla anlamlı çıkanlar olur: Çoklu test düzeltmesi FDR (Benjamini–Hochberg) ya da Bonferroni ile düzeltilmiş p yazılır ve sınıf ondandır.",
            "Isı haritası noktaların yoğunluğunu gösterir, hiçbir şeyi sınamaz; sıcak nokta bir değerin komşuluğunda beklenenden yüksek olup olmadığını istatistikle sınar. Deney profili çıkarımı, tohumu, düzeltmeyi, komşuluğu ve girdinin SHA-256 özetini yazar; seçili ya da görünen nesnelerle yapılan sınama alt küme diye söylenir.",
        ]
        .join("\n\n"),
        &["sıcak nokta", "soğuk nokta", "getis", "gi*", "hot spot", "hotspot analysis", "kümelenme", "istatistik"],
        &["SICAKNOKTA", "HOTSPOT", "GISTAR"],
        vec![
            input("Değeri sınanacak noktalar, çizgiler ve alanlar."),
            value_field(),
            concepts(false),
            band(),
            neighbors(),
            inference_param(true),
            permutations_param(false),
            seed_param(false),
            correction_param(),
            hot_layer(),
        ],
        vec![
            OutputDef::new("spots", "Nesneler", OutputKind::Features),
            OutputDef::new("count", "Nesne sayısı", OutputKind::Number),
            OutputDef::new("hot", "Sıcak nokta", OutputKind::Number),
            OutputDef::new("cold", "Soğuk nokta", OutputKind::Number),
            OutputDef::new("profile", "Deney profili", OutputKind::Table),
            OutputDef::new("experiment", "Deney profili (JSON)", OutputKind::Text),
        ],
        |v, ctx, f| {
            if geographic(ctx) {
                return RunResult::refused(GEOGRAPHIC.into());
            }
            let field = v.text("valueField");
            let scope = scope_of(v, ctx);
            match hot_spots_with(&shape_list(v), &texts(v, field), field, hood(v), inference_of(v, false)) {
                Ok(run) => tested_result_of(
                    run,
                    f,
                    &v.features("input").entities,
                    Some(&v.layer("layer").id),
                    &scope,
                ),
                Err(why) => RunResult::refused(why),
            }
        },
    )
}

pub fn local_moran() -> Tool {
    tool(
        "stats.localMoran",
        "Yerel Moran (LISA)",
        "statsLocalMoran",
        "Her nesnenin değerinin komşularınınkine benzeyip benzemediğini yerel Moran ile sınar: yüksek ve düşük değer kümeleri ile aykırılar. Her nesnenin kopyasını küme türünün rengiyle yazar.",
        [
            "Bir nesnenin yerel Moran I'sı z / m₂ ile komşularının ağırlıklı z toplamının çarpımıdır; z değerin ortalamadan farkı, m₂ = Σ z² / n (Anselin 1995). Artı I, nesnenin komşularına benzediğini (Yüksek-Yüksek ya da Düşük-Düşük küme), eksi I farklı olduğunu (Yüksek-Düşük ya da Düşük-Yüksek aykırı) söyler; Çeyrek nesnenin ve komşularının ortalamaya göre yeridir.",
            "Anlamlılık koşullu permütasyonla sınanır: nesnenin değeri yerinde kalır, komşularınınkiler öbür nesnelerinkilerden tohumdan gelen sırayla rastgele çekilir; p, gözlenen I kadar uç olanların payıdır. Küme türü, p (ya da düzeltilmiş p) anlamlılık düzeyinden küçükse çeyrektir, değilse “Anlamlı değil”; komşusu olmayan ya da permütasyonları yayılmayan nesne “Sınanamadı”dır.",
            "Her nesne ayrı sınandığından Çoklu test düzeltmesi önerilir (FDR: Benjamini–Hochberg). Aynı tohum ve permütasyon sayısı aynı sonucu verir; Deney profili onları, komşuluğu ve girdinin SHA-256 özetini yazar. Seçili ya da görünen nesnelerle yapılan sınama alt küme diye söylenir.",
        ]
        .join("\n\n"),
        &[
            "yerel moran",
            "lisa",
            "local moran",
            "küme ve aykırı",
            "cluster and outlier",
            "anselin",
            "permütasyon",
            "istatistik",
        ],
        &["YERELMORAN", "LISA", "LOCALMORAN"],
        vec![
            input("Değeri sınanacak noktalar, çizgiler ve alanlar."),
            value_field(),
            concepts(true),
            band(),
            neighbors(),
            ParamDef::new("standardize", "Satır standartlaştırma", ParamKind::Boolean)
                .default_value(json!(true))
                .describe("Her nesnenin komşu ağırlıkları toplamına bölünür: gecikme komşuların ortalaması olur."),
            permutations_param(true),
            seed_param(true),
            correction_param(),
            ParamDef::new(
                "significance",
                "Anlamlılık düzeyi",
                ParamKind::Choice {
                    options: vec![
                        EnumOption::new("0.10", "%10").hint("p < 0,10"),
                        EnumOption::new("0.05", "%5").hint("p < 0,05"),
                        EnumOption::new("0.01", "%1").hint("p < 0,01"),
                    ],
                },
            )
            .default_value(json!("0.05"))
            .describe("Küme türünün yazılacağı en büyük p."),
            lisa_layer(),
        ],
        vec![
            OutputDef::new("spots", "Nesneler", OutputKind::Features),
            OutputDef::new("count", "Nesne sayısı", OutputKind::Number),
            OutputDef::new("profile", "Deney profili", OutputKind::Table),
            OutputDef::new("experiment", "Deney profili (JSON)", OutputKind::Text),
            OutputDef::new("highHigh", "Yüksek-Yüksek", OutputKind::Number),
            OutputDef::new("lowLow", "Düşük-Düşük", OutputKind::Number),
            OutputDef::new("highLow", "Yüksek-Düşük", OutputKind::Number),
            OutputDef::new("lowHigh", "Düşük-Yüksek", OutputKind::Number),
        ],
        |v, ctx, f| {
            if geographic(ctx) {
                return RunResult::refused(GEOGRAPHIC.into());
            }
            let field = v.text("valueField");
            let scope = scope_of(v, ctx);
            let alpha = v.text("significance").parse().unwrap_or(0.05);
            match local_morans(
                &shape_list(v),
                &texts(v, field),
                field,
                hood(v),
                v.flag("standardize"),
                inference_of(v, true),
                alpha,
            ) {
                Ok(run) => tested_result_of(
                    run,
                    f,
                    &v.features("input").entities,
                    Some(&v.layer("layer").id),
                    &scope,
                ),
                Err(why) => RunResult::refused(why),
            }
        },
    )
}

pub fn dbscan_tool() -> Tool {
    tool(
        "stats.dbscan",
        "DBSCAN kümeleme",
        "statsDbscan",
        "Yakın nesneleri yoğunluklarına göre kümelere ayırır; seyrek kalanlar gürültüdür. Her nesnenin kopyası küme numarası ve kümenin rengiyle yazılır.",
        [
            "Yarıçap içinde (kendisi dahil) en az nokta kadar nesnesi olan nesne çekirdektir; birbirine yarıçaptan yakın çekirdekler ve onlara yakın nesneler bir kümedir (Ester ve arkadaşları 1996). Nesneler çizimdeki sırasıyla gezilir; iki kümeye yakın sınır nesnesi ilk ulaşan kümeye girer.",
            "Sınır noktaları gürültü açıkken (DBSCAN*) yalnız çekirdekler kümelenir. Gürültünün küme numarası 0’dır.",
        ]
        .join("\n\n"),
        &["dbscan", "kümeleme", "yoğunluk", "density based clustering", "cluster", "gürültü", "istatistik"],
        &["DBSCAN"],
        vec![
            input("Kümelenecek noktalar, çizgiler ve alanlar."),
            number("radius", "Yarıçap", Some(0.001), None, false, "m")
                .default_value(json!(50))
                .describe("Komşuluğun yarıçapı (ε)."),
            number("minPoints", "En az nokta", Some(1.0), Some(1000.0), true, "")
                .default_value(json!(5))
                .describe("Çekirdek olmak için yarıçap içinde, kendisi dahil, gereken nesne sayısı."),
            ParamDef::new("borderNoise", "Sınır noktaları gürültü", ParamKind::Boolean)
                .default_value(json!(false))
                .describe("Açıkken yalnız çekirdekler kümelenir (DBSCAN*)."),
            cluster_layer("Kümeler (DBSCAN)"),
        ],
        vec![
            OutputDef::new("members", "Nesneler", OutputKind::Features),
            OutputDef::new("count", "Nesne sayısı", OutputKind::Number),
            OutputDef::new("clusters", "Küme sayısı", OutputKind::Number),
        ],
        |v, ctx, f| {
            if geographic(ctx) {
                return RunResult::refused(GEOGRAPHIC.into());
            }
            let radius = v.number("radius").unwrap_or(50.0);
            let min_points = v.number("minPoints").unwrap_or(5.0).max(1.0) as usize;
            match dbscan(&shape_list(v), radius, min_points, v.flag("borderNoise")) {
                Ok(run) => result_of(run, f, &v.features("input").entities, Some(&v.layer("layer").id)),
                Err(why) => RunResult::refused(why),
            }
        },
    )
}

pub fn k_means_tool() -> Tool {
    tool(
        "stats.kMeans",
        "k-ortalamalar kümeleme",
        "statsKMeans",
        "Nesneleri yerlerine göre k kümeye ayırır: her nesne en yakın küme merkezine. Her nesnenin kopyası küme numarası ve kümenin rengiyle yazılır.",
        [
            "Başlangıç merkezleri kendiliğinden ve hep aynı seçilir: ilki ortalama merkeze en yakın nesne, sonrakiler seçilmiş merkezlere en uzak nesneler. Ardından her nesne en yakın merkeze atanır ve merkezler üyelerinin ortalaması olur; atamalar değişmeyene dek (en çok 500 kez) sürer (Lloyd).",
            "Farklı yer sayısı küme sayısından azsa çalışmaz.",
        ]
        .join("\n\n"),
        &["k-ortalamalar", "k means", "kmeans", "kümeleme", "cluster", "gruplama", "istatistik"],
        &["KORTALAMA", "KMEANS"],
        vec![
            input("Kümelenecek noktalar, çizgiler ve alanlar."),
            number("clusters", "Küme sayısı", Some(2.0), Some(100.0), true, "")
                .default_value(json!(5))
                .describe("Kaç küme (k)."),
            cluster_layer("Kümeler (k-ortalamalar)"),
        ],
        vec![
            OutputDef::new("members", "Nesneler", OutputKind::Features),
            OutputDef::new("count", "Nesne sayısı", OutputKind::Number),
            OutputDef::new("clusters", "Küme sayısı", OutputKind::Number),
        ],
        |v, ctx, f| {
            if geographic(ctx) {
                return RunResult::refused(GEOGRAPHIC.into());
            }
            let k = v.number("clusters").unwrap_or(5.0).max(2.0) as usize;
            match k_means(&shape_list(v), k) {
                Ok(run) => result_of(run, f, &v.features("input").entities, Some(&v.layer("layer").id)),
                Err(why) => RunResult::refused(why),
            }
        },
    )
}
