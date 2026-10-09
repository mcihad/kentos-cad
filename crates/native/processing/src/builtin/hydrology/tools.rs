//! The eight tools of Hidroloji (docs/adr/0235 §3–§11): their parameters
//! as the web's (`builtin/hydrology/tools.ts`), and their settings handed to
//! the raster core's operation job.

use serde_json::{Value, json};

use super::{HYDROLOGY, run_objects, run_raster, trimmed, warn_skipped};
use crate::builtin::geometry::output_style;
use crate::builtin::pointcloud::{add_param, count_words, output_param};
use crate::builtin::surface::{band_param, choice, number, result_layer};
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
        category: HYDROLOGY.into(),
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

const HELP_EMPTY: &str = "Rasterin nodata'sı, NaN ve alfası 0 olan pikseller değersizdir; kenardaki ve değersiz hücreye komşu hücreler çıkıştır: su rasterin dışına ya da değersiz hücreye akabilir.";
const HELP_FILL: &str = "Çukurları doldur açıkken önce çukurlar taşma yüksekliğine doldurulur (en küçük eğim 0); düzlüklerde yön Barnes ve ark.'nın (2014) yöntemiyle verilir: alçak kenara yaklaşan ve yüksek kenardan uzaklaşan iki gradyan.";
const HELP_D8: &str = "D8: hücre, eğimi (yükseklik farkı / merkezler arası uzaklık) en büyük alçak komşusuna akar; eşitlerde doğudan saat yönünde ilki.";
const HELP_SIZE: &str = "Bütün raster bellekte çalışılır: en çok 2²⁵ hücre (5792 × 5792).";

fn file_output() -> Vec<OutputDef> {
    vec![OutputDef::new("file", "Sonuç dosyası", OutputKind::Text)]
}

fn objects_output(label: &str) -> Vec<OutputDef> {
    vec![
        OutputDef::new("objects", label, OutputKind::Features),
        OutputDef::new("count", "Nesne sayısı", OutputKind::Number),
    ]
}

/// The DEM.
fn dem() -> ParamDef {
    ParamDef::new(
        "input",
        "Yükseklik modeli",
        ParamKind::Features {
            kinds: Some(vec!["raster".to_owned()]),
            scopes: Some(vec![ScopeKind::Selection, ScopeKind::Layer]),
            writes: false,
        },
    )
    .describe("Yükseklik modeli (DEM) rasteri.")
}

fn fill_first() -> ParamDef {
    ParamDef::new("fill", "Çukurları doldur", ParamKind::Boolean)
        .default_value(json!(true))
        .describe("Önce çukurlar doldurulur; kapalıysa çukurlar akışı durdurur.")
}

/// Çıktı dosyası, Çizime ekle, Çıktı katmanı.
fn ends(suffix: &str, layer: &str) -> [ParamDef; 3] {
    [
        output_param(suffix, &[".tif"]),
        add_param(),
        result_layer(layer, RASTER_LAYER),
    ]
}

/// Çıktı katmanı of an object tool: a new one goes right above the DEM's.
fn objects_layer(name: &str, color: &str) -> ParamDef {
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
    .describe("Bu adda katman yoksa oluşturulur; yükseklik modelinin katmanının hemen üstünde.")
}

fn method(first: &str) -> ParamDef {
    let all = [
        ("d8", "D8"),
        ("dinf", "D∞ (Tarboton)"),
        ("mfd", "Çoklu yön (MFD)"),
    ];
    let mut opts: Vec<(&str, &str)> = all.iter().copied().filter(|(v, _)| *v == first).collect();
    opts.extend(all.iter().copied().filter(|(v, _)| *v != first));
    choice("method", "Yöntem", &opts)
}

fn exponent() -> ParamDef {
    number("exponent", "Çoklu yönün üssü", 0.0, 0.0, 100.0, "")
        .describe("0: uyarlanan (Qin ve ark. 2007, ArcGIS Pro'nunki); 0,1–100: sabit üs (1: Quinn ve ark. 1991).")
        .shown_when(|v| text(v, "method", "d8") == "mfd")
}

fn points_param() -> ParamDef {
    ParamDef::new(
        "points",
        "Noktalar",
        ParamKind::Features {
            kinds: Some(vec!["point".to_owned()]),
            scopes: Some(vec![
                ScopeKind::Layer,
                ScopeKind::Selection,
                ScopeKind::Visible,
                ScopeKind::All,
            ]),
            writes: false,
        },
    )
    .describe("Döküm noktaları: nokta nesneleri (çok noktalının her noktası).")
}

fn snap() -> ParamDef {
    number("snap", "Yaklaştırma uzaklığı", 0.0, 0.0, 1e6, "m")
        .describe("0: noktanın hücresi; artı bir uzaklıkta bu uzaklıktaki hücrelerden D8 birikimi en büyük olanı.")
}

fn threshold() -> ParamDef {
    number("threshold", "Eşik alanı", 0.0, 0.0, 1e15, "m²")
        .describe("D8 birikimi (alan) bu değerden küçük olmayan hücreler deredir; 0: en büyük birikimin yüzde biri.")
}

pub fn fill() -> Tool {
    let mut p = vec![
        dem(),
        band_param(),
        number("slope", "En küçük eğim", 0.0, 0.0, 100.0, "")
            .describe("Yüzde; 0 çukurları düz doldurur, artı değerde doldurulan yüzey çıkışa en az bu eğimle alçalır."),
        choice(
            "result",
            "Sonuç",
            &[("filled", "Doldurulmuş yükseklik"), ("depth", "Dolgu derinliği")],
        ),
    ];
    p.extend(ends("-dolu", "Doldurulmuş DEM"));
    tool(
        ("hydrology.fill", "Çukur doldur", "fillSinks"),
        "Yükseklik modelinin çukurlarını taşma yüksekliğine doldurur: her hücreden rasterin dışına alçalan bir yol kalır.",
        &[
            "Doldurulmuş yüzey f ≥ z olan ve her iç hücrenin f(n) + ε·d ≤ f(c) olan bir komşusu bulunan en küçük yüzeydir (ε en küçük eğim / 100). En küçük eğim 0'da her hücre taşma yüksekliğine, kenara giden yolların en yüksek noktalarının en küçüğüne yükselir (Wang ve Liu 2006; Planchon ve Darboux 2002); artı eğimde çukurlar ve düzlükler bu eğimle çıkışa doğru alçalır.",
            "Hesap öncelik kuyruklu taşmadır (Barnes ve ark. 2014); birden çok iş parçacığında raster şeritlerde paralel doldurulur (Barnes 2016), sonuç aynıdır.",
            HELP_EMPTY,
            "Dolgu derinliği doldurulmuş yükseklik eksi DEM'dir. Sonuç 32 bit (DEM 64 bitse 64 bit) karolu GeoTIFF'tir; Çizime ekle açıksa DEM'in katmanının hemen üstüne eklenir.",
            HELP_SIZE,
        ],
        &[
            "çukur",
            "doldur",
            "fill",
            "sink",
            "depression",
            "wang liu",
            "priority flood",
        ],
        &["CUKURDOLDUR", "FILLSINKS"],
        p,
        file_output(),
        |r, cx, fb| {
            let depth = r.text("result") == "depth";
            let tool = json!({
                "kind": "fill",
                "band": r.number("band").unwrap_or(1.0) as u32,
                "slope": r.number("slope").unwrap_or(0.0),
                "result": if depth { "depth" } else { "filled" },
            });
            run_raster(r, cx, fb, tool, ("-dolu", "Çukurlar dolduruluyor"), &|n| {
                let h = &n.hydro;
                if h.cells == 0 {
                    return " Doldurulacak çukur yok.".to_owned();
                }
                let mut s = format!(" {} hücre dolduruldu.", count_words(h.cells));
                if depth {
                    s.push_str(&format!(" En derin dolgu {} m.", trimmed(h.most, 3)));
                }
                s
            })
        },
    )
}

pub fn flow_direction() -> Tool {
    let mut p = vec![
        dem(),
        band_param(),
        fill_first(),
        choice(
            "coding",
            "Kodlama",
            &[("esri", "ESRI (1–128)"), ("taudem", "1–8 (TauDEM)")],
        ),
    ];
    p.extend(ends("-yon", "Akış yönü"));
    tool(
        ("hydrology.flowDirection", "Akış yönü", "flowDirection"),
        "Her hücrenin suyunu verdiği komşuyu D8 kodu olarak yazar.",
        &[
            HELP_D8,
            "ESRI kodları doğudan saat yönünde 1, 2, 4, 8, 16, 32, 64, 128 (güneydoğu 2, kuzey 64); 1–8 TauDEM'inki: doğudan saat yönünün tersine. Çıkış hücresinin alçak komşusu yoksa dışarı akar. Yönsüz hücre 0, değersiz 255.",
            HELP_FILL,
            HELP_EMPTY,
            HELP_SIZE,
        ],
        &["akış yönü", "flow direction", "d8", "drenaj", "yön"],
        &["AKISYONU", "FLOWDIR"],
        p,
        file_output(),
        |r, cx, fb| {
            let tool = json!({
                "kind": "flowDirection",
                "band": r.number("band").unwrap_or(1.0) as u32,
                "fill": r.flag("fill"),
                "coding": if r.text("coding") == "taudem" { "taudem" } else { "esri" },
            });
            run_raster(
                r,
                cx,
                fb,
                tool,
                ("-yon", "Akış yönleri bulunuyor"),
                &|n| {
                    let h = &n.hydro;
                    let mut s = String::new();
                    if h.cells > 0 {
                        s.push_str(&format!(
                            " {} düzlük hücresinin yönü verildi.",
                            count_words(h.cells)
                        ));
                    }
                    if h.empty > 0 {
                        s.push_str(&format!(
                        " {} hücre yönsüz kaldı: çukurlar ve çıkışsız düzlükler (Çukurları doldur açıkken kalmaz).",
                        count_words(h.empty)
                    ));
                    }
                    s
                },
            )
        },
    )
}

pub fn accumulation() -> Tool {
    let mut p = vec![
        dem(),
        band_param(),
        fill_first(),
        method("d8"),
        exponent(),
        choice(
            "unit",
            "Birim",
            &[
                ("cells", "Hücre sayısı"),
                ("area", "Alan (m²)"),
                ("sca", "Özgül havza alanı (m)"),
            ],
        ),
    ];
    p.extend(ends("-birikim", "Akış birikimi"));
    tool(
        (
            "hydrology.flowAccumulation",
            "Akış birikimi",
            "flowAccumulation",
        ),
        "Her hücreden geçen suyun geldiği alanı yazar: hücre sayısı, alan ya da özgül havza alanı.",
        &[
            "Birikim hücrenin kendisi ve yukarısındaki hücrelerin paylarıdır (hücre kendini sayar; ArcGIS'in Flow Accumulation'ı saymaz, onunki bir eksiktir). Özgül havza alanı alan / hücrenin genişliğidir.",
            "D8: bütün pay yönüne. D∞ (Tarboton 1997): sekiz üçgen yüzün en dik alçalanı, pay iki komşu arasında açıyla. Çoklu yön: pay L·tᵖ ile orantılı (t eğim, L Quinn ve ark.'nın kontur uzunluğu: dikte 0,5, çaprazda 0,354); üs sabit ya da uyarlanan p = 8,9·min(e, 1) + 1,1 (Qin ve ark. 2007, e hücrenin en dik eğimi). Alçak komşusu olmayan hücrenin payı D8 yönüne.",
            HELP_FILL,
            HELP_EMPTY,
            HELP_SIZE,
        ],
        &[
            "birikim",
            "flow accumulation",
            "akış",
            "havza alanı",
            "sca",
            "d8",
            "mfd",
            "d-infinity",
        ],
        &["AKISBIRIKIMI", "FLOWACC"],
        p,
        file_output(),
        |r, cx, fb| {
            let unit = match r.text("unit") {
                "area" => "area",
                "sca" => "sca",
                _ => "cells",
            };
            let tool = json!({
                "kind": "flowAccumulation",
                "band": r.number("band").unwrap_or(1.0) as u32,
                "fill": r.flag("fill"),
                "method": match r.text("method") { "" => "d8", m => m },
                "exponent": r.number("exponent").unwrap_or(0.0),
                "unit": unit,
            });
            let name = match unit {
                "area" => "m²",
                "sca" => "m",
                _ => "hücre",
            };
            run_raster(
                r,
                cx,
                fb,
                tool,
                ("-birikim", "Akış birikimi hesaplanıyor"),
                &|n| format!(" En büyük birikim {} {name}.", trimmed(n.hydro.most, 3)),
            )
        },
    )
}

pub fn wetness() -> Tool {
    let mut p = vec![
        dem(),
        band_param(),
        fill_first(),
        method("mfd"),
        exponent(),
        number("slope", "En küçük eğim", 0.1, 0.0001, 100.0, "").describe(
            "Yüzde; tan β bunun altındaysa bu kullanılır (CSIRO'nun DEM-H TWI'ında %0,1).",
        ),
    ];
    p.extend(ends("-twi", "Nemlilik indisi"));
    tool(
        ("hydrology.wetness", "Topografik nemlilik indisi", "wetness"),
        "Topografik nemlilik indisi TWI = ln(a / tan β): suyun toplandığı düz yerler yüksek, sırtlar düşük.",
        &[
            "a özgül havza alanıdır (m; Akış birikimi'nin), tan β yüzeyin Horn yöntemiyle eğimi (ADR 0231); tan β en küçük eğimin altındaysa en küçük eğim alınır. Beven ve Kirkby'nin (1979) indisidir.",
            "Yöntem birikimin yöntemidir; Çoklu yön varsayılandır. Sonuç 32 bit; görünüş kuruda kırmızı, ıslakta mavi.",
            HELP_FILL,
            HELP_EMPTY,
            HELP_SIZE,
        ],
        &[
            "twi",
            "nemlilik",
            "wetness",
            "topographic wetness index",
            "ıslak",
            "beven kirkby",
        ],
        &["NEMLILIK", "TWI"],
        p,
        file_output(),
        |r, cx, fb| {
            let tool = json!({
                "kind": "wetness",
                "band": r.number("band").unwrap_or(1.0) as u32,
                "fill": r.flag("fill"),
                "method": match r.text("method") { "" => "mfd", m => m },
                "exponent": r.number("exponent").unwrap_or(0.0),
                "slope": r.number("slope").unwrap_or(0.1),
            });
            run_raster(
                r,
                cx,
                fb,
                tool,
                ("-twi", "Nemlilik indisi hesaplanıyor"),
                &|n| {
                    let h = &n.hydro;
                    if h.cells > 0 {
                        format!(
                            " {} hücrede en küçük eğim kullanıldı.",
                            count_words(h.cells)
                        )
                    } else {
                        String::new()
                    }
                },
            )
        },
    )
}

pub fn pour_point() -> Tool {
    tool(
        ("hydrology.pourPoint", "Döküm noktası", "pourPoint"),
        "Noktaları yakındaki en büyük akış birikimli hücreye taşır: havzanın çıkışı derenin üstüne gelir.",
        &[
            "Yaklaştırma 0'da noktanın hücresi; artı bir uzaklıkta merkezi bu uzaklıkta olan hücrelerden D8 birikimi en büyük olanı (eşitse noktaya en yakın, o da eşitse satır satır ilki; ArcGIS'in Snap Pour Point'i).",
            "Her nokta hücresinin merkezine yazılır; öznitelikleri Nokta (girdideki sırası), Birikim (hücre), Alan (m²) ve Uzaklık (m). Rasterin dışındaki ya da değersiz hücreye düşen nokta atlanır.",
            HELP_FILL,
            HELP_SIZE,
        ],
        &[
            "döküm noktası",
            "pour point",
            "snap",
            "çıkış",
            "outlet",
            "havza",
        ],
        &["DOKUMNOKTASI", "SNAPPOUR"],
        vec![
            dem(),
            band_param(),
            points_param(),
            fill_first(),
            snap(),
            objects_layer("Döküm noktaları", "#E5484D"),
        ],
        objects_output("Noktalar"),
        |r, cx, fb| {
            let tool = json!({
                "kind": "pourPoint",
                "band": r.number("band").unwrap_or(1.0) as u32,
                "fill": r.flag("fill"),
                "snap": r.number("snap").unwrap_or(0.0),
            });
            run_objects(
                r,
                cx,
                fb,
                tool,
                Some("points"),
                "Döküm noktaları bulunuyor",
                &|f, n, fb| {
                    warn_skipped(n, fb);
                    format!("{} döküm noktası yazıldı.", count_words(f.len() as u64))
                },
            )
        },
    )
}

pub fn watershed() -> Tool {
    tool(
        ("hydrology.watershed", "Noktadan havza", "watershed"),
        "Her döküm noktasının havzasını alan olarak yazar: suyu o noktadan geçen bütün hücreler.",
        &[
            "Hücre D8 yolundaki ilk döküm noktasının havzasındadır: yukarıdaki nokta aşağıdakinin havzasından kendi havzasını ayırır (ArcGIS'in Watershed'i). İki nokta aynı hücreye düşerse hücre girdide sonra gelenindir; öbürünün havzası boş kalır, söylenir.",
            "Yaklaştırma Döküm noktası'nınkidir. Alanlar hücre kenarlarındandır (8 komşu); öznitelikleri Havza (noktanın girdideki sırası) ve Alan (m²).",
            HELP_FILL,
            HELP_SIZE,
        ],
        &[
            "havza",
            "watershed",
            "catchment",
            "su toplama",
            "menfez",
            "noktaya göre havza",
        ],
        &["HAVZABUL", "NOKTAHAVZA", "WATERSHED"],
        vec![
            dem(),
            band_param(),
            points_param(),
            fill_first(),
            snap(),
            objects_layer("Havzalar", "#30A46C"),
        ],
        objects_output("Havzalar"),
        |r, cx, fb| {
            let tool = json!({
                "kind": "watershed",
                "band": r.number("band").unwrap_or(1.0) as u32,
                "fill": r.flag("fill"),
                "snap": r.number("snap").unwrap_or(0.0),
            });
            run_objects(
                r,
                cx,
                fb,
                tool,
                Some("points"),
                "Havzalar bulunuyor",
                &|f, n, fb| {
                    warn_skipped(n, fb);
                    if !n.hydro.empty_points.is_empty() {
                        fb.warn(format!(
                            "{} noktanın hücresini sonraki bir nokta aldığı için havzası boş.",
                            count_words(n.hydro.empty_points.len() as u64)
                        ));
                    }
                    format!("{} havza yazıldı.", count_words(f.len() as u64))
                },
            )
        },
    )
}

pub fn basins() -> Tool {
    tool(
        ("hydrology.basins", "Havzalar", "basins"),
        "Bütün havzaları, dere kollarının alt havzalarını ya da bir güzergâhı kesen derelerin havzalarını alan olarak yazar.",
        &[
            "Ana havzalar: her hücre D8 yolunun bittiği hücreye göre (dışarı akan çıkış ya da yönsüz hücre); tek hücrelik havza da havzadır (ArcGIS'in Basin'i). Alt havzalar: Dere ağı'nın her kolunun kendi havzası (Bağ, Sıra). Güzergâhı kesen dereler: güzergâhın geçtiği dere hücrelerinden dere boyunca en aşağıdakiler geçiştir; her geçişin bütün havzası (menfez ve köprünün beslenme alanı, örtüşebilir), Km güzergâhtaki yeri, Sıra kolun Strahler sırası.",
            "En küçük alandan küçük havza yazılmaz, sayısı söylenir. Alanlar hücre kenarlarındandır.",
            HELP_FILL,
            HELP_SIZE,
        ],
        &[
            "havzalar",
            "basin",
            "alt havza",
            "subbasin",
            "güzergâh",
            "menfez",
            "köprü",
            "dere",
        ],
        &["HAVZALAR", "ALTHAVZA", "BASINS"],
        vec![
            dem(),
            band_param(),
            fill_first(),
            choice(
                "mode",
                "Biçim",
                &[
                    ("main", "Ana havzalar"),
                    ("sub", "Alt havzalar"),
                    ("route", "Güzergâhı kesen dereler"),
                ],
            ),
            threshold().shown_when(|v| text(v, "mode", "main") != "main"),
            ParamDef::new(
                "routes",
                "Güzergâh",
                ParamKind::Features {
                    kinds: Some(vec![
                        "line".to_owned(),
                        "polyline".to_owned(),
                        "arc".to_owned(),
                    ]),
                    scopes: Some(vec![ScopeKind::Selection, ScopeKind::Layer]),
                    writes: false,
                },
            )
            .optional()
            .describe("Yol ya da kanal ekseni: çizgi, çoklu çizgi ya da yay.")
            .shown_when(|v| text(v, "mode", "main") == "route"),
            number("least", "En küçük alan", 0.0, 0.0, 1e15, "m²")
                .describe("Bundan küçük havza yazılmaz; 0 hepsi."),
            objects_layer("Havzalar", "#30A46C"),
        ],
        objects_output("Havzalar"),
        |r, cx, fb| {
            let mode = match r.text("mode") {
                "sub" => "sub",
                "route" => "route",
                _ => "main",
            };
            let tool = json!({
                "kind": "basins",
                "band": r.number("band").unwrap_or(1.0) as u32,
                "fill": r.flag("fill"),
                "mode": mode,
                "threshold": r.number("threshold").unwrap_or(0.0),
                "least": r.number("least").unwrap_or(0.0),
            });
            let shapes = (mode == "route").then_some("routes");
            run_objects(r, cx, fb, tool, shapes, "Havzalar bulunuyor", &|f, n, _| {
                let mut s = format!("{} havza yazıldı.", count_words(f.len() as u64));
                if n.hydro.dropped > 0 {
                    s.push_str(&format!(
                        " En küçük alandan küçük {} havza yazılmadı.",
                        count_words(n.hydro.dropped)
                    ));
                }
                s
            })
        },
    )
}

pub fn streams() -> Tool {
    tool(
        ("hydrology.streams", "Dere ağı", "streams"),
        "Akış birikimi eşiği aşan hücrelerden dere ağını çoklu çizgi olarak yazar; her kolun Strahler ve Shreve sırasıyla.",
        &[
            "Dere hücresi D8 birikimi (alan) eşikten küçük olmayan hücredir. Kol kaynakta ya da kavşakta başlar, bir sonraki kavşakta ya da yolun sonunda biter; kavşakta biten kol aktığı kolun ilk hücresine uzanır.",
            "Strahler: kaynak 1; aynı en büyük sıradan iki ya da daha çok kol birleşince bir artar. Shreve: gelenlerin toplamı. Öznitelikler: Bağ, Sıra, Shreve, Uzunluk (m), Düşü (m), Eğim (düşü / uzunluk), Alan (kolun sonundaki birikim, m²), Aşağı (aktığı kol).",
            "Sadeleştirme hücre cinsinden Douglas–Peucker toleransıdır; 0 yalnız doğrultudaki köşeleri atar.",
            HELP_FILL,
            HELP_SIZE,
        ],
        &[
            "dere",
            "stream",
            "akarsu",
            "drenaj ağı",
            "strahler",
            "shreve",
            "kol",
            "network",
        ],
        &["DEREAGI", "STREAMORDER"],
        vec![
            dem(),
            band_param(),
            fill_first(),
            threshold(),
            number("simplify", "Sadeleştirme", 0.0, 0.0, 1000.0, "").describe(
                "Douglas–Peucker toleransı, hücre cinsinden; 0 yalnız doğrultudaki köşeleri atar.",
            ),
            objects_layer("Dere ağı", "#0090FF"),
        ],
        objects_output("Dereler"),
        |r, cx, fb| {
            let tool = json!({
                "kind": "streams",
                "band": r.number("band").unwrap_or(1.0) as u32,
                "fill": r.flag("fill"),
                "threshold": r.number("threshold").unwrap_or(0.0),
                "simplify": r.number("simplify").unwrap_or(0.0),
            });
            run_objects(
                r,
                cx,
                fb,
                tool,
                None,
                "Dere ağı çıkarılıyor",
                &|f, n, _| {
                    let h = &n.hydro;
                    if f.is_empty() {
                        return "Eşiği aşan dere hücresi yok.".to_owned();
                    }
                    format!(
                        "{} kol yazıldı; en büyük Strahler sırası {}, eşik {} m².",
                        count_words(f.len() as u64),
                        trimmed(h.most, 0),
                        trimmed(h.threshold, 3)
                    )
                },
            )
        },
    )
}
