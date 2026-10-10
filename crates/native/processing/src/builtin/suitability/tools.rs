//! The six tools of Uygunluk analizi (docs/adr/0237 §3–§8): their
//! parameters as the web's (`builtin/suitability/tools.ts`), and their
//! settings handed to the raster core.

use serde_json::{Value, json};

use super::{SUITABILITY, joined, run_pairwise, run_raster, run_roc, say_invalid, say_overlay};
use crate::builtin::pointcloud::{add_param, output_param, result_layer_param};
use crate::builtin::queries::AREA_KINDS;
use crate::builtin::surface::{choice, number};
use crate::types::{OutputDef, OutputKind, ParamDef, ParamKind, ScopeKind, Target, Tool, Values};

/// The results' layers' colour (a raster's is not drawn).
const RASTER_LAYER: &str = "#7A6B5B";

fn text<'a>(v: &'a Values, name: &str, or: &'a str) -> &'a str {
    v.get(name).and_then(Value::as_str).unwrap_or(or)
}

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
        category: SUITABILITY.into(),
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

const HELP_EMPTY: &str = "Rasterin nodata'sı, NaN ve alfası 0 olan pikseller değersizdir.";
const HELP_GRID: &str = "Rasterler ortak alanlarında, hücresi en küçük olanın ızgarasında birleşir: sonucun hücreleri merkezi bütün rasterlerin içinde olanlardır; her raster hücre merkezinde en yakın hücresiyle okunur. Bir rasterin değersiz olduğu hücre sonuçta değersizdir.";
const HELP_ORDER: &str = "Rasterler Katmanlar panelinde üstten aşağı sırayla (aynı katmanda sonra eklenen önce) ve katmanlarının adlarıyla anılır; aynı katmandaki ikinci raster “Ad (2)”.";
const HELP_OUTPUT: &str = "Sonuç karolu, Deflate'li ve önizleme katlı GeoTIFF'tir; çıktı dosyası boşsa ilk rasterin yanına adının sonuna ek konarak yazılır. Çizime ekle açıksa raster ilk rasterin katmanının hemen üstündeki yeni katmana eklenir.";

fn file_output() -> Vec<OutputDef> {
    vec![OutputDef::new("file", "Sonuç dosyası", OutputKind::Text)]
}

fn table_output() -> OutputDef {
    OutputDef::new("table", "Tablo", OutputKind::Table)
}

/// One raster.
fn one_raster(description: &str) -> ParamDef {
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

/// Several rasters, the visible ones first.
fn rasters(label: &str, description: &str) -> ParamDef {
    ParamDef::new(
        "input",
        label,
        ParamKind::Features {
            kinds: Some(vec!["raster".to_owned()]),
            scopes: Some(vec![
                ScopeKind::Visible,
                ScopeKind::Selection,
                ScopeKind::Layer,
                ScopeKind::All,
            ]),
            writes: false,
        },
    )
    .describe(description)
}

fn band() -> ParamDef {
    ParamDef::new(
        "band",
        "Bant",
        ParamKind::Number {
            min: Some(1.0),
            max: Some(255.0),
            integer: true,
            unit: String::new(),
            placeholder: None,
        },
    )
    .default_value(json!(1))
    .describe("Değerlerin okunduğu bant (1'den); bütün rasterlerde aynı bant.")
}

fn sample() -> ParamDef {
    choice(
        "sample",
        "Sonuç türü",
        &[("f32", "Ondalık 32 bit"), ("f64", "Ondalık 64 bit")],
    )
}

/// Çıktı dosyası, Çizime ekle, Çıktı katmanı (right above the first raster's layer).
fn ends(suffix: &str, layer: &str) -> [ParamDef; 3] {
    let mut l = result_layer_param(layer, RASTER_LAYER);
    if let ParamKind::Layer { above, .. } = &mut l.kind {
        *above = Some("input".to_owned());
    }
    [output_param(suffix, &[".tif"]), add_param(), l]
}

/// A number the user types for each raster (docs/adr/0237 §9).
fn raster_numbers(
    name: &str,
    label: &str,
    (min, max): (f64, f64),
    placeholder: &str,
    description: &str,
) -> ParamDef {
    ParamDef::new(
        name,
        label,
        ParamKind::RasterValues {
            of: "input".to_owned(),
            number: true,
            min: Some(min),
            max: Some(max),
            placeholder: Some(placeholder.to_owned()),
        },
    )
    .default_value(json!({}))
    .describe(description)
}

fn number_shown(
    (name, label): (&str, &str),
    (default, min, max): (f64, f64, f64),
    description: &str,
    when: fn(&Values) -> bool,
) -> ParamDef {
    number(name, label, default, min, max, "")
        .describe(description)
        .shown_when(when)
}

pub fn fuzzy_membership() -> Tool {
    let ends_of = |v: &Values| matches!(text(v, "function", "linear"), "linear" | "power");
    let mut p = vec![
        one_raster("Üyeliğe çevrilecek raster (eğim, uzaklık …)."),
        band(),
        {
            let mut f = choice(
                "function",
                "İşlev",
                &[
                    ("linear", "Doğrusal"),
                    ("power", "Üslü"),
                    ("gaussian", "Gauss"),
                    ("large", "Büyük"),
                    ("small", "Küçük"),
                    ("near", "Yakın"),
                ],
            );
            if let ParamKind::Choice { options } = &mut f.kind {
                let hints = [
                    "alt değerden üst değere doğru 0'dan 1'e",
                    "doğrusalın üssü: yavaş ya da hızlı yükselen",
                    "orta noktada 1, iki yana çan eğrisiyle azalan",
                    "orta noktadan büyük değerler 1'e yakın",
                    "orta noktadan küçük değerler 1'e yakın",
                    "orta noktaya yakın değerler 1'e yakın",
                ];
                for (o, h) in options.iter_mut().zip(hints) {
                    o.hint = Some(h.to_owned());
                }
            }
            f.describe("Değer 0–1 üyeliğe çevrilir: 1 bütünüyle uygun, 0 hiç uygun değil.")
        },
        number("low", "Alt değer", 0.0, -1e12, 1e12, "")
            .describe(
                "Doğrusal ve Üslü'de üyeliğin 0 olduğu değer; üst değerden büyükse üyelik azalır.",
            )
            .shown_when(ends_of),
        number("high", "Üst değer", 100.0, -1e12, 1e12, "")
            .describe("Doğrusal ve Üslü'de üyeliğin 1 olduğu değer.")
            .shown_when(ends_of),
        number_shown(
            ("exponent", "Üs"),
            (2.0, 0.01, 100.0),
            "Doğrusal üyelik bu üsse yükseltilir: 1'den büyükse yavaş, küçükse hızlı yükselir.",
            |v| text(v, "function", "linear") == "power",
        ),
        number_shown(
            ("midpoint", "Orta nokta"),
            (1.0, -1e12, 1e12),
            "Gauss ve Yakın'da üyeliğin 1, Büyük ve Küçük'te 0,5 olduğu değer; Büyük ve Küçük'te 0'dan büyük.",
            |v| {
                matches!(
                    text(v, "function", "linear"),
                    "gaussian" | "large" | "small" | "near"
                )
            },
        ),
        number_shown(
            ("spread", "Yayılım"),
            (0.1, 1e-9, 1e6),
            "Büyüdükçe üyelik orta noktadan uzaklaştıkça daha hızlı düşer.",
            |v| matches!(text(v, "function", "linear"), "gaussian" | "near"),
        ),
        number_shown(
            ("steep", "Diklik"),
            (5.0, 1e-9, 1e6),
            "Büyüdükçe üyelik orta noktada daha keskin değişir.",
            |v| matches!(text(v, "function", "linear"), "large" | "small"),
        ),
        sample(),
    ];
    p.extend(ends("-uyelik", "Üyelik"));
    tool(
        (
            "suitability.fuzzyMembership",
            "Bulanık üyelik",
            "fuzzyMembership",
        ),
        "Rasterin değerlerini bir işlevle 0–1 arası üyeliğe çevirir: ölçütü ortak ölçeğe getirmenin bulanık yolu.",
        &[
            "Doğrusal: alt değerde 0, üst değerde 1, arada doğrusal (alt değer büyükse azalan). Üslü: doğrusal üyelik üssüne yükseltilir. Gauss: e^(−yayılım·(x − orta)²). Büyük: 1 / (1 + (x / orta)^(−diklik)), 0 ve altı 0. Küçük: 1 / (1 + (x / orta)^diklik), 0 ve altı 1. Yakın: 1 / (1 + yayılım·(x − orta)²).",
            "Varsayılanlar ArcGIS'in Fuzzy Membership'iyle aynıdır: Gauss ve Yakın'da yayılım 0,1, Büyük ve Küçük'te diklik 5.",
            HELP_EMPTY,
            HELP_OUTPUT,
        ],
        &[
            "bulanık",
            "fuzzy",
            "üyelik",
            "membership",
            "fuzzify",
            "uygunluk",
            "standartlaştır",
        ],
        &["BULANIKUYELIK", "FUZZYMEMBERSHIP"],
        p,
        file_output(),
        |r, cx, fb| {
            let tool = json!({
                "kind": "fuzzyMembership",
                "band": r.number("band").unwrap_or(1.0) as u32,
                "function": r.text("function"),
                "low": r.number("low").unwrap_or(0.0),
                "high": r.number("high").unwrap_or(100.0),
                "exponent": r.number("exponent").unwrap_or(2.0),
                "midpoint": r.number("midpoint").unwrap_or(1.0),
                "spread": r.number("spread").unwrap_or(0.1),
                "steep": r.number("steep").unwrap_or(5.0),
                "sample": r.text("sample"),
            });
            run_raster(
                r,
                cx,
                fb,
                tool,
                ("-uyelik", "Üyelik hesaplanıyor"),
                (true, &|_, _, _| String::new()),
            )
        },
    )
}

pub fn fuzzy_overlay() -> Tool {
    let mut p = vec![
        rasters(
            "Rasterler",
            "Üyelik rasterleri (0–1), örneğin Bulanık üyelik'in sonuçları.",
        ),
        band(),
        choice(
            "op",
            "İşleç",
            &[
                ("and", "Ve (en küçük)"),
                ("or", "Veya (en büyük)"),
                ("product", "Çarpım"),
                ("sum", "Toplam"),
                ("gamma", "Gamma"),
            ],
        )
        .describe("Ve: bütün ölçütler birlikte; Veya: en az biri; Çarpım ve Toplam ölçütlerin hepsini katar; Gamma ikisinin arası."),
        number("gamma", "Gamma", 0.9, 0.0, 1.0, "")
            .describe("Toplam^γ · Çarpım^(1 − γ): 1'de Toplam, 0'da Çarpım.")
            .shown_when(|v| text(v, "op", "and") == "gamma"),
        sample(),
    ];
    p.extend(ends("-bulanik", "Bulanık çakıştırma"));
    tool(
        (
            "suitability.fuzzyOverlay",
            "Bulanık çakıştırma",
            "fuzzyOverlay",
        ),
        "Üyelik rasterlerini bulanık mantığın işleciyle birleştirir: Ve, Veya, Çarpım, Toplam, Gamma.",
        &[
            "Ve en küçük üyeliği, Veya en büyüğünü alır. Çarpım üyeliklerin çarpımıdır (her ölçüt sonucu düşürür); Toplam 1 − Π(1 − μ)'dür (her ölçüt artırır). Gamma Toplam^γ · Çarpım^(1 − γ) (Bonham-Carter).",
            "Üyelik 0 ile 1 arasında olmalıdır: dışındaki değerin hücresi değersiz bırakılır ve söylenir.",
            HELP_GRID,
            HELP_OUTPUT,
        ],
        &[
            "bulanık",
            "fuzzy",
            "çakıştırma",
            "overlay",
            "gamma",
            "uygunluk",
        ],
        &["BULANIKCAKISTIR", "FUZZYOVERLAY"],
        p,
        file_output(),
        |r, cx, fb| {
            let tool = json!({
                "kind": "fuzzyOverlay",
                "band": r.number("band").unwrap_or(1.0) as u32,
                "op": r.text("op"),
                "gamma": r.number("gamma").unwrap_or(0.9),
                "sample": r.text("sample"),
            });
            run_raster(
                r,
                cx,
                fb,
                tool,
                ("-bulanik", "Bulanık çakıştırılıyor"),
                (false, &|n, k, fb| {
                    say_invalid(n, fb);
                    joined(k)
                }),
            )
        },
    )
}

pub fn weighted_sum() -> Tool {
    let mut p = vec![
        rasters("Rasterler", "Toplanacak ölçüt rasterleri."),
        band(),
        raster_numbers(
            "weights",
            "Ağırlıklar",
            (-1e9, 1e9),
            "1",
            "Her rasterin çarpanı; boş bırakılanınki 1. Eksi ağırlık ölçütü tersine çevirir.",
        ),
        sample(),
    ];
    p.extend(ends("-agirlikli", "Ağırlıklı toplam"));
    tool(
        ("suitability.weightedSum", "Ağırlıklı toplam", "weightedSum"),
        "Rasterleri ağırlıklarıyla çarpıp toplar: Σ ağırlık · değer, ondalık sonuç.",
        &[
            "Değerler yeniden ölçeklenmez: ölçütleri önce ortak bir ölçeğe getirin (Yeniden sınıflandır ya da Bulanık üyelik). Ağırlıklar İkili karşılaştırma'dan alınabilir.",
            HELP_ORDER,
            HELP_GRID,
            HELP_OUTPUT,
        ],
        &[
            "ağırlıklı",
            "toplam",
            "weighted sum",
            "uygunluk",
            "çok ölçütlü",
            "mcda",
        ],
        &["AGIRLIKLITOPLAM", "WEIGHTEDSUM"],
        p,
        file_output(),
        |r, cx, fb| {
            let tool = json!({
                "kind": "weightedSum",
                "band": r.number("band").unwrap_or(1.0) as u32,
                "weights": r.values.get("weights").cloned().unwrap_or(json!({})),
                "sample": r.text("sample"),
            });
            run_raster(
                r,
                cx,
                fb,
                tool,
                ("-agirlikli", "Ağırlıklı toplam hesaplanıyor"),
                (false, &|_, k, _| joined(k)),
            )
        },
    )
}

fn scale_end(name: &str, label: &str, default: f64, description: &str) -> ParamDef {
    ParamDef::new(
        name,
        label,
        ParamKind::Number {
            min: Some(-1e6),
            max: Some(1e6),
            integer: true,
            unit: String::new(),
            placeholder: None,
        },
    )
    .default_value(json!(default))
    .describe(description)
}

pub fn weighted_overlay() -> Tool {
    let mut p = vec![
        rasters("Rasterler", "Ölçüt rasterleri: sınıfları ya da değerleri."),
        band(),
        scale_end("low", "Ölçeğin alt ucu", 1.0, "Ortak ölçeğin en küçük sınıfı; kısıtlı hücreler bunun bir eksiğini alır."),
        scale_end("high", "Ölçeğin üst ucu", 9.0, "Ortak ölçeğin en büyük, en uygun sınıfı."),
        raster_numbers(
            "influence",
            "Etki (%)",
            (0.0, 100.0),
            "%",
            "Her rasterin etkisi, yüzde; en çok dört ondalık, toplamları tam 100.",
        ),
        ParamDef::new(
            "classes",
            "Sınıflar",
            ParamKind::RasterValues {
                of: "input".to_owned(),
                number: false,
                min: None,
                max: None,
                placeholder: Some("* 5 9; 5 15 6; 15 * kısıt".to_owned()),
            },
        )
        .default_value(json!({}))
        .optional()
        .describe("Her rasterin tablosu: “alt üst yeni”, “değer yeni” ya da “boş yeni”; yeni değer ölçekte bir tam sayı, boş ya da kısıt. Tablosu boş rasterin değerleri ölçek değeridir."),
        choice(
            "bounds",
            "Sınırlar",
            &[("upperClosed", "alt < değer ≤ üst"), ("lowerClosed", "alt ≤ değer < üst")],
        ),
    ];
    p.extend(ends("-cakistirma", "Ağırlıklı çakıştırma"));
    tool(
        (
            "suitability.weightedOverlay",
            "Ağırlıklı çakıştırma",
            "weightedOverlay",
        ),
        "Rasterleri ortak bir ölçeğe sınıflayıp etkileriyle birleştirir; sonuç ölçeğin tam sayı sınıfıdır.",
        &[
            "Her raster tablosuyla ölçeğin bir sınıfına çevrilir (tablosu yoksa değeri sınıftır); sonuç sınıfların etkileriyle ağırlıklı toplamıdır, tam sayılarla kesin hesaplanıp yarımlar sıfırdan uzağa yuvarlanır. Kısıt sınıflı bir raster hücreyi kısıtlı yapar (ölçeğin alt ucunun bir eksiği).",
            "Kuralı tutmayan, ölçeğin dışında ya da tam sayı olmayan değer hücreyi değersiz bırakır ve söylenir; değersiz kısıtlıdan önce gelir.",
            HELP_ORDER,
            HELP_GRID,
            HELP_OUTPUT,
        ],
        &[
            "ağırlıklı",
            "çakıştırma",
            "weighted overlay",
            "uygunluk",
            "arazi sentezi",
            "yerleşilebilirlik",
            "çok ölçütlü",
        ],
        &["AGIRLIKLICAKISTIR", "WEIGHTEDOVERLAY"],
        p,
        file_output(),
        |r, cx, fb| {
            let tool = json!({
                "kind": "weightedOverlay",
                "band": r.number("band").unwrap_or(1.0) as u32,
                "low": r.number("low").unwrap_or(1.0),
                "high": r.number("high").unwrap_or(9.0),
                "influence": r.values.get("influence").cloned().unwrap_or(json!({})),
                "classes": r.values.get("classes").filter(|v| v.is_object()).cloned().unwrap_or(json!({})),
                "bounds": r.text("bounds"),
            });
            run_raster(
                r,
                cx,
                fb,
                tool,
                ("-cakistirma", "Ağırlıklı çakıştırılıyor"),
                (false, &|n, k, fb| {
                    format!("{}{}", joined(k), say_overlay(n, fb))
                }),
            )
        },
    )
}

pub fn pairwise() -> Tool {
    let writes = |v: &Values| v.get("write").and_then(Value::as_bool).unwrap_or(true);
    let [output, add, mut layer] = ends("-ahp", "Ağırlıklı toplam (AHP)");
    layer.visible_when = Some(crate::types::ShownWhen::Rule(|v| {
        v.get("write").and_then(Value::as_bool).unwrap_or(true)
            && v.get("add").and_then(Value::as_bool).unwrap_or(true)
    }));
    let p = vec![
        rasters("Ölçütler", "Karşılaştırılan ölçütler: 2 ile 15 raster."),
        ParamDef::new(
            "comparisons",
            "Karşılaştırmalar",
            ParamKind::RasterPairs {
                of: "input".to_owned(),
            },
        )
        .default_value(json!([]))
        .describe("Her çift için hangisinin kaç kat önemli olduğu (Saaty'nin 1–9 ölçeği); seçilmeyen çift eşittir."),
        ParamDef::new("write", "Ağırlıklı toplamı yaz", ParamKind::Boolean)
            .default_value(json!(true))
            .describe("Açıkken ölçütler bu ağırlıklarla toplanıp raster yazılır; kapalıyken yalnız ağırlıklar."),
        band().shown_when(writes),
        sample().shown_when(writes),
        output.shown_when(writes),
        add.shown_when(writes),
        layer,
    ];
    tool(
        (
            "suitability.pairwise",
            "İkili karşılaştırma (AHP)",
            "pairwise",
        ),
        "Ölçütleri ikişer ikişer karşılaştırıp ağırlıklarını ve tutarlılığını bulur (Saaty'nin AHP'si); isterseniz ağırlıklı toplamı yazar.",
        &[
            "Her çift için önemli olan ve kaç kat önemli olduğu seçilir: 1 eşit, 3 biraz, 5 açıkça, 7 çok, 9 son derece önemli (2, 4, 6, 8 arası). Ağırlıklar karşılaştırma matrisinin baş özvektörüdür, toplamları 1.",
            "Tutarlılık oranı CR = CI / RI; CI = (λ − n) / (n − 1), RI Saaty'nin rastgele indeksi. CR 0,10'dan büyükse karşılaştırmalar tutarsızdır: en çelişkili çiftleri yeniden gözden geçirin.",
            HELP_ORDER,
            "Ağırlıklı toplam Ağırlıklı toplam aracınınkiyle aynıdır: ölçütleri önce ortak bir ölçeğe getirin.",
            HELP_OUTPUT,
        ],
        &[
            "ahp",
            "ikili karşılaştırma",
            "pairwise",
            "analitik hiyerarşi",
            "ağırlık",
            "saaty",
            "tutarlılık",
        ],
        &["AHP", "IKILIKARSILASTIRMA"],
        p,
        vec![
            table_output(),
            OutputDef::new("file", "Sonuç dosyası", OutputKind::Text),
        ],
        run_pairwise,
    )
}

fn samples(name: &str, label: &str, description: &str) -> ParamDef {
    let mut kinds = vec!["point".to_owned()];
    kinds.extend(AREA_KINDS.iter().map(|k| (*k).to_owned()));
    ParamDef::new(
        name,
        label,
        ParamKind::Features {
            kinds: Some(kinds),
            scopes: Some(vec![
                ScopeKind::Layer,
                ScopeKind::Selection,
                ScopeKind::Visible,
                ScopeKind::All,
            ]),
            writes: false,
        },
    )
    .describe(description)
}

pub fn roc() -> Tool {
    let p = vec![
        one_raster("Doğrulanacak raster: duyarlılık ya da uygunluk."),
        band(),
        samples("presence", "Varlık", "Bilinen olaylar: noktalar ya da alanlar (heyelan, sel …)."),
        choice(
            "background",
            "Karşılaştırma",
            &[("cells", "Bütün hücreler"), ("absence", "Yokluk nesneleri")],
        )
        .describe("Bütün hücreler başarı eğrisini verir (alan oranı); yokluk nesneleri olayın olmadığı bilinen yerlerdir."),
        samples("absence", "Yokluk", "Olayın olmadığı bilinen noktalar ya da alanlar.")
            .shown_when(|v| text(v, "background", "cells") == "absence"),
        ParamDef::new("higher", "Yüksek değer daha olası", ParamKind::Boolean)
            .default_value(json!(true))
            .describe("Kapalıyken düşük değer olayı daha olası gösterir."),
    ];
    tool(
        ("suitability.roc", "ROC ile doğrulama", "rocCurve"),
        "Rasterin bilinen olayları ne kadar iyi ayırdığını ölçer: ROC eğrisinin tablosu, eğrinin altındaki alan (AUC) ve en iyi eşik.",
        &[
            "Örnekler hücrelerdir: nokta içinde olduğu hücreyi, alan merkezini içine aldığı hücreleri verir; bir hücre bir kez sayılır. Değersiz hücreye ya da rasterin dışına düşen örnek atlanır, söylenir.",
            "Eşik büyükten küçüğe inerken doğru pozitif oranı yanlış pozitif oranına (bütün hücrelerde alan oranına) karşı çıkar; tablo varlık değerlerinin eşiklerindedir (en çok 200). AUC bir varlık hücresinin değerinin karşılaştırma hücresininkinden büyük olma olasılığıdır (eşitler yarım): 0,5 rastgele, 1 kusursuz. En iyi eşik Youden'in J'sidir (doğru pozitif oranı eksi yanlış pozitif oranı en büyük).",
            HELP_EMPTY,
        ],
        &[
            "roc",
            "auc",
            "doğrulama",
            "validation",
            "başarı eğrisi",
            "success rate",
            "duyarlılık",
        ],
        &["ROC", "AUC"],
        p,
        vec![
            table_output(),
            OutputDef::new("auc", "AUC", OutputKind::Number),
        ],
        run_roc,
    )
}
