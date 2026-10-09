//! The seven tools of Raster ve vektör and Taranmış harita (docs/adr/0234
//! §3–§10): their parameters as the web's (`builtin/rasterVector/tools.ts`),
//! and their settings handed to the raster core's jobs.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use super::{
    BURN_KINDS, RASTER_VECTOR, SCANNED, objects_of, run_contour_elevations, run_rasterize,
    run_vector,
};
use crate::builtin::geometry::output_style;
use crate::builtin::interpolation::{grid_params, result_layer};
use crate::builtin::pointcloud::{add_param, count_words, output_param};
use crate::builtin::surface::{band_param, choice, number, whole};
use crate::types::{OutputDef, OutputKind, ParamDef, ParamKind, ScopeKind, Target, Tool, Values};

fn text<'a>(v: &'a Values, name: &str, or: &'a str) -> &'a str {
    v.get(name).and_then(Value::as_str).unwrap_or(or)
}

/// A tool of the categories, run on the desktop's other thread.
#[allow(clippy::too_many_arguments)]
fn tool(
    (id, label, icon, category): (&str, &str, &str, &str),
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
        category: category.into(),
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
const HELP_PLACE: &str = "Köşeler hücre uzayında (hücre köşeleri ya da merkezleri) bulunur, rasterin yeriyle (afin) çizime geçer.";
const HELP_THIN: &str = "Çizgi hücreleri Zhang–Suen inceltmesiyle (Lü–Wang düzeltmesi: iki piksel kalınlığındaki çapraz çizgi kalır) tek piksele indirilir; iskeletin kavşaktan kavşağa yolları birer çoklu çizgidir. Kısa parçaları at: ucu boşta, kavşağa bağlı ve bu kadar adımdan kısa dallar, sonra bütün kısa yollar atılır. Sadeleştirme hücre cinsinden Douglas–Peucker toleransıdır.";

fn features_outputs(label: &str) -> Vec<OutputDef> {
    vec![
        OutputDef::new("objects", label, OutputKind::Features),
        OutputDef::new("count", "Nesne sayısı", OutputKind::Number),
    ]
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

/// Çıktı katmanı of a vectorizing tool: a new one goes right above the raster's.
fn vector_layer(name: &str, color: &str) -> ParamDef {
    ParamDef::new(
        "layer",
        "Çıktı katmanı",
        ParamKind::Layer {
            new_layer_style: output_style(color),
            above: Some("input".to_owned()),
            below: None,
        },
    )
    .default_value(json!({ "newName": name }))
    .describe("Bu adda katman yoksa oluşturulur; rasterin katmanının hemen üstünde.")
}

fn spur(default: u32) -> ParamDef {
    whole("spur", "Kısa parçaları at", default, 0, 10_000)
        .describe("Hücre cinsinden; 0 hiçbirini atmaz.")
}

fn simplify() -> ParamDef {
    number("simplify", "Sadeleştirme", 1.0, 0.0, 1000.0, "").describe(
        "Douglas–Peucker toleransı, hücre cinsinden; 0 yalnız doğrultudaki köşeleri atar.",
    )
}

fn tolerance() -> ParamDef {
    number("tolerance", "Renk toleransı", 60.0, 0.0, 442.0, "")
}

fn point(name: &str, label: &str, description: &str) -> ParamDef {
    ParamDef::new(name, label, ParamKind::Point).describe(description)
}

pub fn rasterize() -> Tool {
    let mut p = vec![
        ParamDef::new(
            "input",
            "Nesneler",
            ParamKind::Features {
                kinds: Some(BURN_KINDS.iter().map(|k| (*k).to_owned()).collect()),
                scopes: Some(vec![
                    ScopeKind::Layer,
                    ScopeKind::Selection,
                    ScopeKind::Visible,
                    ScopeKind::All,
                ]),
                writes: false,
            },
        )
        .describe("Yakılacak alanlar, çizgiler ve noktalar."),
        choice(
            "valueFrom",
            "Değer",
            &[("constant", "Sabit"), ("field", "Alandan")],
        ),
        number("value", "Sabit değer", 1.0, -1e15, 1e15, "")
            .shown_when(|v| text(v, "valueFrom", "constant") != "field"),
        ParamDef::new(
            "field",
            "Değer alanı",
            ParamKind::Field {
                of: vec!["input".into()],
                allow_new: false,
                multiple: false,
            },
        )
        .optional()
        .describe("Nesnenin bu alandaki sayısı; sayı olmayan nesne alınmaz.")
        .shown_when(|v| text(v, "valueFrom", "constant") == "field"),
        choice(
            "overlap",
            "Çakışanlar",
            &[
                ("last", "Son çizilen"),
                ("first", "İlk çizilen"),
                ("max", "En büyük"),
                ("min", "En küçük"),
                ("sum", "Toplam"),
                ("count", "Sayı"),
            ],
        )
        .describe("Bir hücreyi birden çok nesne yakarsa kalan değer; Sayı nesne sayısıdır."),
        choice(
            "sample",
            "Sonuç türü",
            &[
                ("f32", "Ondalık 32 bit"),
                ("f64", "Ondalık 64 bit"),
                ("i32", "Tam sayı 32 bit"),
                ("u8", "Bayt (0–254)"),
            ],
        ),
    ];
    p.extend(grid_params());
    p.extend([
        output_param("-raster", &[".tif"]),
        add_param(),
        result_layer("Rasterleştirilmiş", "layer"),
    ]);
    tool(
        (
            "raster.rasterize",
            "Rasterleştir",
            "rasterize",
            RASTER_VECTOR,
        ),
        "Nesneleri rasterin hücrelerine yakar: alanlar merkezlerini, çizgiler geçtikleri hücreleri, noktalar içinde oldukları hücreyi.",
        &[
            "Kapalı şekil (kapalı alan, daire, tam elips, kapalı eğri, tarama) merkezi içinde olan hücreleri, açık şekil (çizgi, çoklu çizgi, yay, elips yayı, açık eğri) bir noktası yarı açık karesine düşen bütün hücreleri yakar; yaylar 0,1 mm kirişlerle izlenir. Nokta içinde olduğu hücreyi yakar. Bir nesne bir hücreyi bir kez sayar.",
            "Değer: Sabit değer ya da seçilen alandaki sayı (ondalık nokta ya da virgül); sayı olmayan nesne alınmaz, söylenir. Çakışanlar: girdinin sırasında sonraki (Son çizilen), önceki, en büyük, en küçük, toplam ya da nesne sayısı.",
            "Tam sayıda yarımlar sıfırdan uzağa yuvarlanır; türe sığmayan değer reddedilir (Tam sayı 32 bit'te −2 147 483 648, Bayt'ta 255 değersizdir). Yanmayan hücre değersizdir.",
            "Hücre boyu 0 ise nesnelerin kutusunun kısa kenarının 250'de biri, 1, 2, 2,5 ya da 5 × 10ᵏ'ye yuvarlanır; ızgara bu boyun katlarına oturur. Rasterin ızgarası seçilirse sonuç o rasterle hücre hücre üst üste gelir.",
            "Sonuç karolu, Deflate'li GeoTIFF'tir; Çizime ekle açıksa girdinin katmanının hemen altındaki yeni katmana eklenir (nesneler üstte kalır).",
        ],
        &[
            "rasterleştir",
            "rasterize",
            "feature to raster",
            "polygon to raster",
            "yak",
            "burn",
            "maske",
        ],
        &["RASTERIZE", "VEKTORRASTER"],
        p,
        vec![OutputDef::new("file", "Sonuç dosyası", OutputKind::Text)],
        run_rasterize,
    )
}

pub fn to_polygons() -> Tool {
    tool(
        (
            "raster.toPolygons",
            "Rasterden alan",
            "toPolygons",
            RASTER_VECTOR,
        ),
        "Rasterin değeri eşit komşu hücrelerini alan yapar; her alanın Değer özniteliği bölgenin değeridir.",
        &[
            "Bölge, değeri tam eşit ve komşu (4 komşu: kenarla; 8 komşu: kenar ya da köşeyle) hücrelerin bağlı kümesidir; değersiz hücre bölge değildir.",
            "Halkalar hücre kenarlarındandır: dış halka saat yönünün tersine, delikler saat yönünde; bölgeyi bir noktadan dokunarak saran boşluk o noktada dış halkaya değen delik olur. Alanlar bölgelerin ilk hücresinin sırasıyla.",
            "En çok 8192 × 8192 hücre, 1 000 000 alan: çok değerli raster için önce Yeniden sınıflandır'la sınıflara ayırın.",
            HELP_EMPTY,
            HELP_PLACE,
        ],
        &[
            "poligon",
            "polygonize",
            "raster to polygon",
            "sınıf",
            "bölge",
            "vektörleştir",
        ],
        &["RASTERDENALAN", "RASTERALAN"],
        vec![
            one_raster("Bölgeleri alan yapılacak raster (genellikle sınıflandırılmış)."),
            band_param(),
            choice(
                "connect",
                "Komşuluk",
                &[
                    ("four", "4 komşu (kenar)"),
                    ("eight", "8 komşu (kenar ve köşe)"),
                ],
            ),
            vector_layer("Bölgeler", "#3E63DD"),
        ],
        features_outputs("Alanlar"),
        |r, cx, fb| {
            let tool = json!({ "kind": "toPolygons", "band": r.number("band").unwrap_or(1.0) as u32, "connect": r.text("connect") });
            run_vector(
                r,
                cx,
                fb,
                tool,
                "Bölgeler alan yapılıyor",
                &|f, layer, _| {
                    let add = objects_of(
                        f,
                        layer,
                        &|k| BTreeMap::from([("Değer".to_owned(), f.texts[k].clone())]),
                        None,
                    );
                    let values = f
                        .texts
                        .iter()
                        .collect::<std::collections::BTreeSet<_>>()
                        .len();
                    let summary = if add.is_empty() {
                        "Rasterde değeri olan hücre yok.".to_owned()
                    } else {
                        format!(
                            "{} değerden {} alan yazıldı.",
                            count_words(values as u64),
                            count_words(add.len() as u64)
                        )
                    };
                    (add, summary)
                },
            )
        },
    )
}

pub fn to_lines() -> Tool {
    tool(
        (
            "raster.toLines",
            "Rasterden çizgi",
            "toLines",
            RASTER_VECTOR,
        ),
        "Rasterdeki ince çizgileri inceltip çoklu çizgi yapar: sıfır dışındaki hücreler, bir değer aralığı ya da bir renk.",
        &[
            "Çizgi hücreleri: 0 ve değersiz dışındakiler, en küçük ile en büyük arasındaki değerler ya da üç bantlı rasterde #RRGGBB'ye Renk toleransı içindeki renkler ((ΔR² + ΔG² + ΔB²) ≤ tolerans²).",
            HELP_THIN,
            "Kapalı halka çoklu çizgisi ilk noktasını sonda yineler. En çok 8192 × 8192 hücre.",
            HELP_EMPTY,
            HELP_PLACE,
        ],
        &[
            "çizgi",
            "polyline",
            "raster to polyline",
            "iskelet",
            "skeleton",
            "inceltme",
            "vektörleştir",
        ],
        &["RASTERDENCIZGI"],
        vec![
            one_raster("Çizgileri alınacak raster."),
            band_param(),
            choice(
                "select",
                "Çizgi hücreleri",
                &[
                    ("nonZero", "0 ve değersiz dışındakiler"),
                    ("range", "Değer aralığı"),
                    ("color", "Renk"),
                ],
            ),
            number("min", "En küçük", 1.0, -1e15, 1e15, "")
                .shown_when(|v| text(v, "select", "nonZero") == "range"),
            number("max", "En büyük", 1.0, -1e15, 1e15, "")
                .shown_when(|v| text(v, "select", "nonZero") == "range"),
            ParamDef::new(
                "color",
                "Renk",
                ParamKind::Text {
                    placeholder: Some("#RRGGBB".into()),
                    max_length: None,
                    allow_empty: false,
                },
            )
            .default_value(json!("#000000"))
            .shown_when(|v| text(v, "select", "nonZero") == "color"),
            tolerance().shown_when(|v| text(v, "select", "nonZero") == "color"),
            spur(0),
            simplify(),
            vector_layer("Çizgiler", "#E5484D"),
        ],
        features_outputs("Çizgiler"),
        |r, cx, fb| {
            let tool = json!({
                "kind": "toLines",
                "band": r.number("band").unwrap_or(1.0) as u32,
                "select": r.text("select"),
                "min": r.number("min").unwrap_or(0.0),
                "max": r.number("max").unwrap_or(0.0),
                "color": r.text("color"),
                "tolerance": r.number("tolerance").unwrap_or(60.0),
                "spur": r.number("spur").unwrap_or(0.0) as u32,
                "simplify": r.number("simplify").unwrap_or(1.0),
            });
            run_vector(
                r,
                cx,
                fb,
                tool,
                "Çizgiler çıkarılıyor",
                &|f, layer, _| {
                    let add = objects_of(f, layer, &|_| BTreeMap::new(), None);
                    let summary = if add.is_empty() {
                        "Çizgi hücresi yok ya da hepsi kısa parça olarak atıldı.".to_owned()
                    } else {
                        format!("{} çizgi yazıldı.", count_words(add.len() as u64))
                    };
                    (add, summary)
                },
            )
        },
    )
}

pub fn to_points() -> Tool {
    tool(
        (
            "raster.toPoints",
            "Rasterden nokta",
            "toPoints",
            RASTER_VECTOR,
        ),
        "Hücre merkezlerinde nokta yazar: adım adım, her hücrede ya da yalnız tepe ve çukurlarda; kotu hücrenin değeri.",
        &[
            "Adımla: k hücrede bir (i mod k = ⌊k/2⌋ ve j mod k = ⌊k/2⌋ olan hücreler). Tepeler ve çukurlar: (2r + 1)² pencerede, değeri olan bütün komşularından kesin büyük olan tepe, kesin küçük olan çukur.",
            "Noktalar satır satır; öznitelikleri Değer (ve Tür: Tepe ya da Çukur). En çok 2 000 000 nokta.",
            HELP_EMPTY,
        ],
        &[
            "nokta",
            "point",
            "raster to point",
            "yükseklik noktaları",
            "spot",
            "tepe",
            "çukur",
            "dem",
        ],
        &["RASTERDENNOKTA", "YUKSEKLIKNOKTALARI"],
        vec![
            one_raster("Değerleri nokta yapılacak raster."),
            band_param(),
            choice(
                "mode",
                "Biçim",
                &[
                    ("step", "Adımla"),
                    ("all", "Her hücre"),
                    ("extrema", "Tepeler ve çukurlar"),
                ],
            ),
            whole("step", "Adım", 10, 1, 100_000)
                .describe("Hücre cinsinden: k hücrede bir nokta.")
                .shown_when(|v| text(v, "mode", "step") == "step"),
            whole("radius", "Pencere yarıçapı", 1, 1, 50)
                .describe("Hücre cinsinden: (2r + 1)² pencere.")
                .shown_when(|v| text(v, "mode", "step") == "extrema"),
            ParamDef::new("elevation", "Kot olarak yaz", ParamKind::Boolean)
                .default_value(json!(true))
                .describe("Noktanın kotu hücrenin değeri."),
            vector_layer("Noktalar", "#30A46C"),
        ],
        features_outputs("Noktalar"),
        |r, cx, fb| {
            let mode = r.text("mode").to_owned();
            let tool = json!({
                "kind": "toPoints",
                "band": r.number("band").unwrap_or(1.0) as u32,
                "mode": mode,
                "step": r.number("step").unwrap_or(10.0) as u32,
                "radius": r.number("radius").unwrap_or(1.0) as u32,
            });
            let z = r.values.get("elevation").and_then(Value::as_bool) != Some(false);
            run_vector(
                r,
                cx,
                fb,
                tool,
                "Noktalar çıkarılıyor",
                &|f, layer, _| {
                    let extrema = mode == "extrema";
                    let attrs = |k: usize| {
                        let mut a = BTreeMap::from([("Değer".to_owned(), f.texts[k].clone())]);
                        if extrema {
                            a.insert(
                                "Tür".to_owned(),
                                if f.tags[k] == 1 { "Tepe" } else { "Çukur" }.to_owned(),
                            );
                        }
                        a
                    };
                    let value = |k: usize| f.values[k];
                    let add = objects_of(
                        f,
                        layer,
                        &attrs,
                        z.then_some(&value as &dyn Fn(usize) -> f64),
                    );
                    let summary = if add.is_empty() {
                        "Nokta çıkmadı: değeri olan hücre yok.".to_owned()
                    } else if extrema {
                        let peaks = f.tags.iter().filter(|&&t| t == 1).count();
                        format!(
                            "{} tepe, {} çukur yazıldı.",
                            count_words(peaks as u64),
                            count_words((add.len() - peaks) as u64)
                        )
                    } else {
                        format!("{} nokta yazıldı.", count_words(add.len() as u64))
                    };
                    (add, summary)
                },
            )
        },
    )
}

pub fn capture_line() -> Tool {
    tool(
        ("scan.captureLine", "Çizgi yakala", "captureLine", SCANNED),
        "Taranmış paftada tıklanan çizgiyi (ve ona bağlı aynı renkteki çizgileri) yakalayıp çoklu çizgi yapar; isterseniz kotuyla.",
        &[
            "Noktanın çevresindeki 7 × 7 hücrenin en koyusu tohumdur; hedef renk onun rengidir. Hedefe Renk toleransı içindeki, tohuma köşe ya da kenarla bağlı hücreler çizgidir. Tıklanan çizgiye bağlı aynı renkteki bütün çizgiler yakalanır: yalnız bir eğri için toleransı daraltın.",
            HELP_THIN,
            "Okuma tohumun çevresindeki 1024 × 1024 hücreyle başlar, çizgi pencerenin kenarına değdikçe genişler (en çok 8192 × 8192). 4 194 304 hücreden büyük yakalama reddedilir. Kot verilirse çizgilerin bütün köşelerinin kotu odur (eş yükselti eğrisi).",
        ],
        &[
            "yakala",
            "hat",
            "çizgi",
            "taranmış",
            "pafta",
            "trace",
            "vectorization",
            "eş yükselti",
            "kontur",
        ],
        &["RASTERDENHATYAKALA", "HATYAKALA", "RASTERDENEGRIYAKALA"],
        vec![
            one_raster("Taranmış pafta ya da harita rasteri."),
            point(
                "at",
                "Çizgi üzerinde nokta",
                "Çizginin üstüne ya da yanına tıklayın.",
            ),
            tolerance(),
            spur(5),
            simplify(),
            ParamDef::new(
                "z",
                "Kot",
                ParamKind::Number {
                    min: Some(-1e6),
                    max: Some(1e6),
                    integer: false,
                    unit: "m".into(),
                    placeholder: Some("Yok".into()),
                },
            )
            .optional()
            .default_value(Value::Null)
            .describe("Verilirse çizgilerin bütün köşelerinin kotu."),
            vector_layer("Yakalanan çizgiler", "#D6409F"),
        ],
        features_outputs("Çizgiler"),
        |r, cx, fb| {
            let Some(at) = r.point("at") else {
                return crate::types::RunResult::refused(
                    "Çizgi üzerinde bir nokta seçin.".to_owned(),
                );
            };
            let tool = json!({
                "kind": "captureLine",
                "x": at.x,
                "y": at.y,
                "tolerance": r.number("tolerance").unwrap_or(60.0),
                "spur": r.number("spur").unwrap_or(5.0) as u32,
                "simplify": r.number("simplify").unwrap_or(1.0),
            });
            let z = r.number("z");
            run_vector(r, cx, fb, tool, "Çizgi yakalanıyor", &|f, layer, _| {
                let level = |_: usize| z.unwrap_or(f64::NAN);
                let add = objects_of(
                    f,
                    layer,
                    &|_| BTreeMap::new(),
                    z.is_some().then_some(&level as &dyn Fn(usize) -> f64),
                );
                let summary = if add.is_empty() {
                    "Yakalanan hücreler çizgi vermedi: hepsi kısa parça olarak atıldı.".to_owned()
                } else {
                    format!("{} çizgi yakalandı.", count_words(add.len() as u64))
                };
                (add, summary)
            })
        },
    )
}

pub fn close_area() -> Tool {
    tool(
        ("scan.closeArea", "Alan kapat", "closeArea", SCANNED),
        "Taranmış paftada tıklanan noktanın çevresindeki çizgilerin kapattığı alanı bulup alan yapar.",
        &[
            "Alan, noktanın hücresinin rengine Renk toleransı içindeki hücrelerin o hücreye kenarla bağlı kümesidir (çizgiler köşeyle bağlı olduğu için çapraz boşluktan sızmaz). Küme rasterin kenarına ulaşırsa alan kapanmıyordur: reddedilir.",
            "Halkalar hücre kenarlarındandır; Doldur'da delikler atılır. Halkalar Douglas–Peucker'la sadeleşir; sadeleşen halkalar kendine ya da birbirine değerse alan sadeleştirilmeden yazılır ve söylenir.",
        ],
        &[
            "alan kapat",
            "kapalı alan",
            "flood",
            "doldur",
            "parsel",
            "taranmış",
            "pafta",
        ],
        &["RASTERDENALANKAPAT", "PAFTAALANKAPAT"],
        vec![
            one_raster("Taranmış pafta ya da harita rasteri."),
            point(
                "at",
                "Alanın içinde nokta",
                "Kapatılacak alanın içine tıklayın.",
            ),
            tolerance(),
            choice("holes", "Delikler", &[("fill", "Doldur"), ("keep", "Koru")]),
            simplify(),
            vector_layer("Kapatılan alanlar", "#6E56CF"),
        ],
        features_outputs("Alanlar"),
        |r, cx, fb| {
            let Some(at) = r.point("at") else {
                return crate::types::RunResult::refused(
                    "Alanın içinde bir nokta seçin.".to_owned(),
                );
            };
            let tool = json!({
                "kind": "closeArea",
                "x": at.x,
                "y": at.y,
                "tolerance": r.number("tolerance").unwrap_or(60.0),
                "holes": r.text("holes"),
                "simplify": r.number("simplify").unwrap_or(1.0),
            });
            run_vector(r, cx, fb, tool, "Alan kapatılıyor", &|f, layer, fb| {
                let add = objects_of(f, layer, &|_| BTreeMap::new(), None);
                if f.tags.first() == Some(&1) {
                    fb.warn(
                        "Sadeleşen halkalar birbirine değdiği için alan sadeleştirilmeden yazıldı."
                            .to_owned(),
                    );
                }
                let holes = f.rings.first().map_or(0, |&n| n.saturating_sub(1));
                let corners: u32 = f.sizes.iter().sum();
                let mut summary =
                    format!("Alan kapatıldı: {} köşe", count_words(u64::from(corners)));
                if holes > 0 {
                    summary.push_str(&format!(", {} delik", count_words(u64::from(holes))));
                }
                summary.push('.');
                (add, summary)
            })
        },
    )
}

pub fn contour_elevations_tool() -> Tool {
    tool(
        (
            "scan.contourElevations",
            "Eğrilere kot ver",
            "contourElevations",
            SCANNED,
        ),
        "Kesen bir çizginin geçtiği eğrilere sırayla kot verir: ilk kot, sonra her eğride bir aralık.",
        &[
            "Her eğrinin yeri, kesen çizgiyle kesişimlerinden başlangıca en yakınıdır; eğriler bu yerlere göre (eşitse girdinin sırasıyla) sıralanır ve k. eğrinin kotu İlk kot + k · Aralık olur. Kesmeyen eğri değişmez, söylenir.",
            "Kot eğrinin bütün köşelerine (deliklerin ve parçaların dahil) yazılır; tek adımda. Eksi aralık kotları azaltır.",
        ],
        &[
            "kot",
            "eğri",
            "eş yükselti",
            "kontur",
            "contour",
            "elevation",
            "taranmış",
        ],
        &["EGRILEREKOTVER"],
        vec![
            ParamDef::new(
                "curves",
                "Eğriler",
                ParamKind::Features {
                    kinds: Some(vec!["line".into(), "polyline".into(), "polygon".into()]),
                    scopes: Some(vec![
                        ScopeKind::Selection,
                        ScopeKind::Layer,
                        ScopeKind::Visible,
                        ScopeKind::All,
                    ]),
                    writes: true,
                },
            )
            .describe("Kot verilecek eğriler; kilitli katmandakiler alınmaz."),
            point(
                "start",
                "Başlangıç",
                "Kesen çizginin ilk eğriden önceki ucu.",
            ),
            point("end", "Bitiş", "Kesen çizginin öbür ucu."),
            number("first", "İlk kot", 0.0, -1e6, 1e6, "m"),
            number("step", "Aralık", 5.0, -1e6, 1e6, "m")
                .describe("0 olamaz; eksi aralık kotları azaltır."),
        ],
        vec![
            OutputDef::new("changed", "Değişen eğriler", OutputKind::Features),
            OutputDef::new("count", "Kot verilen eğri sayısı", OutputKind::Number),
        ],
        run_contour_elevations,
    )
}
