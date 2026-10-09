//! The nine tools of Raster işlemleri and Raster istatistiği (docs/adr/0233
//! §3–§14): their parameters, and their settings handed to the raster core's
//! operation job.

use serde_json::{Value, json};

use super::{OPS, STATS, run_histogram, run_raster_op, run_zonal};
use crate::builtin::pointcloud::{add_param, output_param};
use crate::builtin::queries::{AREA_KINDS, kinds};
use crate::builtin::surface::{band_param, choice, number, result_layer, whole};
use crate::types::{
    OutputDef, OutputKind, ParamDef, ParamKind, Returns, ScopeKind, Target, Tool, Values,
};

/// The results' layers' colour (a raster's is not drawn).
const RASTER_LAYER: &str = "#7A6B5B";

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

fn file_output() -> Vec<OutputDef> {
    vec![OutputDef::new("file", "Sonuç dosyası", OutputKind::Text)]
}

/// Çıktı dosyası, Çizime ekle, Çıktı katmanı.
fn ends(suffix: &str, layer: &str) -> [ParamDef; 3] {
    [
        output_param(suffix, &[".tif"]),
        add_param(),
        result_layer(layer, RASTER_LAYER),
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

/// Several rasters, the run's order theirs in the drawing (§2).
fn rasters(description: &str, first: ScopeKind) -> ParamDef {
    let mut scopes = vec![
        ScopeKind::Visible,
        ScopeKind::Selection,
        ScopeKind::Layer,
        ScopeKind::All,
    ];
    scopes.retain(|s| *s != first);
    scopes.insert(0, first);
    ParamDef::new(
        "input",
        "Rasterler",
        ParamKind::Features {
            kinds: Some(vec!["raster".to_owned()]),
            scopes: Some(scopes),
            writes: false,
        },
    )
    .describe(description)
}

/// Areas: a mask's or the zones' (closed objects).
fn areas(name: &str, label: &str, writes: bool, description: &str) -> ParamDef {
    ParamDef::new(
        name,
        label,
        ParamKind::Features {
            kinds: kinds(&AREA_KINDS),
            scopes: Some(vec![
                ScopeKind::Layer,
                ScopeKind::Selection,
                ScopeKind::Visible,
                ScopeKind::All,
            ]),
            writes,
        },
    )
    .describe(description)
}

/// The statistics a cell's window or stack gives (§9).
fn stats(with_count: bool) -> ParamDef {
    let mut opts = vec![
        ("mean", "Ortalama"),
        ("sum", "Toplam"),
        ("min", "En küçük"),
        ("max", "En büyük"),
        ("range", "Aralık"),
        ("std", "Standart sapma"),
        ("median", "Ortanca"),
        ("majority", "Çoğunluk"),
        ("minority", "Azınlık"),
        ("variety", "Çeşit"),
    ];
    if with_count {
        opts.push(("count", "Sayı"));
    }
    choice("stat", "İstatistik", &opts).describe(
        "Standart sapma örneklemindir (n − 1); Çoğunluk en sık, Azınlık en seyrek değer (eşitse küçüğü); Çeşit farklı değer sayısı.",
    )
}

fn ignore_param() -> ParamDef {
    ParamDef::new("ignore", "Değersizleri yok say", ParamKind::Boolean)
        .default_value(json!(true))
        .describe("Açıkken değeri olan hücrelerin istatistiği; kapalıyken değeri olmayan hücre varsa sonuç değersiz.")
}

const HELP_OUTPUT: &str = "Sonuç karolu, Deflate'li ve önizleme katlı GeoTIFF'tir; çıktı dosyası boşsa ilk rasterin yanına adının sonuna ek konarak yazılır. Çizime ekle açıksa raster ilk rasterin katmanının hemen üstündeki yeni katmana eklenir.";
const HELP_EMPTY: &str = "Rasterin nodata'sı, NaN ve alfası 0 olan pikseller değersizdir.";

pub fn calculator() -> Tool {
    let mut p = vec![
        rasters("İfadenin andığı rasterler; her biri katmanının adıyla anılır.", ScopeKind::Visible),
        ParamDef::new(
            "expression",
            "İfade",
            ParamKind::Expression {
                returns: Returns::Value,
                of: Some("input".into()),
                placeholder: Some("[DEM] * 2".into()),
            },
        )
        .describe("Rasterler katmanlarının adıyla, bant @ ile: [Ortofoto@3]; [DEM] 1. bant. $y ve $x hücrenin merkezi, $alan hücrenin alanı."),
        choice(
            "empty",
            "Değeri olmayan hücreler",
            &[("propagate", "Değersiz kalır"), ("expression", "İfade karar verir")],
        )
        .describe("İfade karar verirse değersiz hücre boş olarak görülür: aritmetik boş verir, boş ve eğer onu karşılar."),
        choice(
            "sample",
            "Sonuç türü",
            &[("f32", "Ondalık 32 bit"), ("f64", "Ondalık 64 bit")],
        ),
    ];
    p.extend(ends("-hesap", "Hesap"));
    tool(
        (
            "raster.calculator",
            "Raster hesaplayıcı",
            "rasterCalculator",
            OPS,
        ),
        "Rasterlerin hücrelerinden ifadeyle yeni bir raster yazar: aritmetik, koşul ve matematik işlevleri.",
        &[
            "Örnekler: ([Ortofoto@4] - [Ortofoto@1]) / ([Ortofoto@4] + [Ortofoto@1]) (NDVI); durum eğer [Eğim] > 15 ise 1 yoksa 0 son; [DEM] - [DEM (2)].",
            "Sonuç, ifadenin ilk andığı rasterin ızgarasında, adıyla ve katmanının hemen üstündedir; yalnız ifadenin andığı rasterler açılır, öbürleri hücre merkezlerinde en yakın hücreleriyle okunur. Koşul doğruysa 1, yanlışsa 0; boş ve sayı olmayan değer değersizdir.",
            "Matematik işlevleri: ln, log10, log, üstel, sin, cos, tan, asin, acos, atan, atan2, derece, radyan, kök, mutlak, yuvarla, min, max.",
            HELP_EMPTY,
            HELP_OUTPUT,
        ],
        &[
            "hesap",
            "cebir",
            "harita cebiri",
            "raster calculator",
            "map algebra",
            "band math",
            "ndvi",
            "ifade",
        ],
        &["RASTERHESAP", "HARITACEBIRI", "BANDARITMETIGI"],
        p,
        file_output(),
        |r, cx, fb| {
            run_raster_op(
                r,
                cx,
                fb,
                json!({"kind": "calculator", "expression": r.text("expression"), "empty": r.text("empty"), "sample": r.text("sample")}),
                ("-hesap", "Raster hesaplanıyor"),
                (false, None),
            )
        },
    )
}

pub fn reclassify() -> Tool {
    let mut p = vec![
        one_raster("Sınıflandırılacak raster."),
        band_param(),
        ParamDef::new(
            "table",
            "Tablo",
            ParamKind::Text {
                placeholder: Some("0 100 1; 100 200 2; 200 * 3".into()),
                max_length: Some(20_000),
                allow_empty: false,
            },
        )
        .describe("Satır başına bir kural (satırlar ; ile de): “alt üst yeni”, “değer yeni” ya da “boş yeni”; açık uç *; yeni değer boş olabilir."),
        choice(
            "bounds",
            "Sınırlar",
            &[("upperClosed", "alt < değer ≤ üst"), ("lowerClosed", "alt ≤ değer < üst")],
        ),
        choice(
            "unmatched",
            "Tabloda olmayanlar",
            &[("keep", "Olduğu gibi kalır"), ("empty", "Değersiz olur")],
        ),
        choice(
            "sample",
            "Sonuç türü",
            &[("f32", "Ondalık 32 bit"), ("i32", "Tam sayı 32 bit"), ("u8", "Bayt (0–254)")],
        )
        .describe("Tam sayıda değerler yarımlar sıfırdan uzağa yuvarlanır; değersiz −2 147 483 648, baytta 255."),
    ];
    p.extend(ends("-sinif", "Sınıflar"));
    tool(
        (
            "raster.reclassify",
            "Yeniden sınıflandır",
            "reclassify",
            OPS,
        ),
        "Rasterin değerlerini bir tabloyla yeni değerlere çevirir: aralıklar, tek değerler, değersiz hücreler.",
        &[
            "Kurallar tablonun sırasıyla denenir; ilk tutan yeni değeri verir. Örnek: “* 5 1; 5 15 2; 15 * 3” eğimi üç sınıfa ayırır.",
            HELP_EMPTY,
            HELP_OUTPUT,
        ],
        &["sınıflandır", "reclassify", "tablo", "sınıf", "aralık"],
        &["YENIDENSINIFLA", "RECLASS"],
        p,
        file_output(),
        |r, cx, fb| {
            run_raster_op(
                r,
                cx,
                fb,
                json!({"kind": "reclassify", "band": r.number("band").unwrap_or(1.0) as u32, "table": r.text("table"), "bounds": r.text("bounds"), "unmatched": r.text("unmatched"), "sample": r.text("sample")}),
                ("-sinif", "Sınıflandırılıyor"),
                (true, None),
            )
        },
    )
}

pub fn clip_by_mask() -> Tool {
    let mut p = vec![
        one_raster("Kırpılacak raster (bütün bantları)."),
        areas("mask", "Maske", false, "Kapalı alanlar: kapalı alan, daire, elips, kapalı eğri, tarama; delikleri dışarıdır."),
        ParamDef::new("crop", "Maskenin kutusuna kırp", ParamKind::Boolean)
            .default_value(json!(true))
            .describe("Açıkken sonuç maskenin içindeki hücrelerin satır ve sütunlarıdır; kapalıyken rasterin bütün ızgarası."),
    ];
    p.extend(ends("-kirpik", "Kırpılmış"));
    tool(
        ("raster.clipByMask", "Maskeyle kırp", "clipRaster", OPS),
        "Rasterin maskenin içindeki hücrelerini tutar, dışarıdakileri değersiz yapar; isterseniz maskenin kutusuna kırpar.",
        &[
            "Hücre, merkezi bir maske nesnesinin içindeyse tutulur (nesnelerin birleşimi; delik dışarıdır). Sınırın üstündeki merkez yarı açık kuralla karar verir: sol ve alt kenar içeride.",
            "Hücreler yeniden örneklenmez: sonuç kaynağın hücreleridir. Kaynağın nodata'sı yoksa dışarıdaki hücreler ondalıkta NaN, RGB'de alfa 0 olur.",
            HELP_OUTPUT,
        ],
        &["kırp", "maske", "clip", "mask", "extract by mask", "kes"],
        &["MASKEKIRP", "RASTERKIRP"],
        p,
        file_output(),
        |r, cx, fb| {
            run_raster_op(
                r,
                cx,
                fb,
                json!({"kind": "clipByMask", "crop": r.flag("crop")}),
                ("-kirpik", "Kırpılıyor"),
                (true, Some("mask")),
            )
        },
    )
}

pub fn mosaic() -> Tool {
    let mut p = vec![
        rasters("Birleştirilecek rasterler: aynı bant sayısı ve türde.", ScopeKind::Selection),
        choice(
            "overlap",
            "Çakışanlar",
            &[
                ("top", "Üstteki"),
                ("bottom", "Alttaki"),
                ("mean", "Ortalama"),
                ("min", "En küçük"),
                ("max", "En büyük"),
            ],
        )
        .describe("Üstteki: çizimde üstte görünen rasterin değeri (Katmanlar panelinde üstteki katman; aynı katmanda sonra eklenen)."),
        choice(
            "sampling",
            "Örnekleme",
            &[("nearest", "En yakın"), ("bilinear", "Çift doğrusal"), ("cubic", "Kübik")],
        ),
    ];
    p.extend(ends("-mozaik", "Mozaik"));
    tool(
        ("raster.mosaic", "Mozaik", "mosaic", OPS),
        "Rasterleri tek rasterde birleştirir; çakışan hücrelerde üstteki, alttaki, ortalama, en küçük ya da en büyük değer.",
        &[
            "Sonucun ızgarası en ince hücreli rasterin eksenleri ve hücre boyudur; kafesi bütün rasterleri kapsayacak kadar büyür. Öbür rasterler sonucun hücre merkezlerinde okunur.",
            "Rasterlerin bant sayısı ve türü aynı olmalıdır; nodata'sı olmayan RGB'de sonuç alfa bandı alır.",
            HELP_OUTPUT,
        ],
        &["mozaik", "mosaic", "birleştir", "merge", "pafta"],
        &["MOZAIK"],
        p,
        file_output(),
        |r, cx, fb| {
            run_raster_op(
                r,
                cx,
                fb,
                json!({"kind": "mosaic", "overlap": r.text("overlap"), "sampling": r.text("sampling")}),
                ("-mozaik", "Birleştiriliyor"),
                (false, None),
            )
        },
    )
}

pub fn resample() -> Tool {
    let mut p = vec![
        one_raster("Yeniden örneklenecek raster (bütün bantları)."),
        number("cell", "Hücre boyu", 0.0, 0.0, 1e6, "m").describe("Yeni hücrenin kenarı; rasterin eksenleri boyunca."),
        choice(
            "method",
            "Yöntem",
            &[
                ("nearest", "En yakın"),
                ("bilinear", "Çift doğrusal"),
                ("cubic", "Kübik"),
                ("mean", "Ortalama"),
                ("mode", "Çoğunluk"),
                ("min", "En küçük"),
                ("max", "En büyük"),
            ],
        )
        .describe("En yakın, Çift doğrusal ve Kübik yeni hücrenin merkezinde okur; Ortalama, Çoğunluk, En küçük ve En büyük örtüştüğü hücrelerden (büyütürken)."),
    ];
    p.extend(ends("-ornek", "Örneklenmiş"));
    tool(
        ("raster.resample", "Yeniden örnekle", "resample", OPS),
        "Rasteri yeni bir hücre boyuyla yeniden örnekler: en yakın, çift doğrusal, kübik ya da örtüşen hücrelerin ortalaması, çoğunluğu, en küçüğü, en büyüğü.",
        &[
            "Sonucun sol üst köşesi ve eksenleri rasterinkidir; rasteri bütünüyle kapsar. Kübik, Keys'in evrişimidir (a = −½, GDAL'ın cubic'i).",
            "Değeri olmayan komşular dışarıda kalır, kalanların ağırlıkları toplamlarına bölünür. Tam sayılı rasterde değerler yarımlar sıfırdan uzağa yuvarlanır.",
            HELP_OUTPUT,
        ],
        &[
            "örnekle",
            "resample",
            "hücre boyu",
            "çözünürlük",
            "warp",
            "aggregate",
        ],
        &["YENIDENORNEKLE", "RESAMPLE"],
        p,
        file_output(),
        |r, cx, fb| {
            let cell = r.number("cell").unwrap_or(0.0);
            if cell.is_nan() || cell <= 0.0 {
                return crate::types::RunResult::refused(
                    "Hücre boyunu yazın (0'dan büyük).".into(),
                );
            }
            run_raster_op(
                r,
                cx,
                fb,
                json!({"kind": "resample", "cell": r.number("cell"), "method": r.text("method")}),
                ("-ornek", "Yeniden örnekleniyor"),
                (true, None),
            )
        },
    )
}

pub fn zonal_statistics() -> Tool {
    let p = vec![
        one_raster("Değerleri okunan raster."),
        band_param(),
        areas("zones", "Bölgeler", true, "Kapalı alanlar; istatistik bunların alanına yazılır, kilitli katmandakiler alınmaz."),
        choice(
            "stat",
            "İstatistik",
            &[
                ("mean", "Ortalama"),
                ("count", "Sayı"),
                ("sum", "Toplam"),
                ("min", "En küçük"),
                ("max", "En büyük"),
                ("range", "Aralık"),
                ("std", "Standart sapma"),
                ("median", "Ortanca"),
                ("majority", "Çoğunluk"),
                ("minority", "Azınlık"),
                ("variety", "Çeşit"),
                ("area", "Alan"),
            ],
        )
        .describe("Alan: değeri olan hücrelerin alanlarının toplamı. Tabloda her bölgenin sayısı, toplamı, ortalaması, en küçüğü, en büyüğü ve standart sapması da var."),
        ParamDef::new(
            "output",
            "Yazılacak alan",
            ParamKind::Field {
                of: vec!["zones".into()],
                allow_new: true,
                multiple: false,
            },
        )
        .default_value(json!("Ortalama"))
        .optional()
        .describe("Listeden var olan bir alanı seçin ya da yeni bir ad yazın; boşsa yalnız tablo."),
        whole("decimals", "Basamak", 3, 0, 12).describe("Yazılan değerin ondalık basamağı (tipli ondalık alanda alanın kendi basamağı)."),
    ];
    tool(
        (
            "raster.zonalStatistics",
            "Bölgesel istatistik",
            "zonalStats",
            STATS,
        ),
        "Her bölgenin (kapalı alanın) içindeki hücrelerin istatistiğini alanına yazar ve tablo olarak verir.",
        &[
            "Örnekler: parsellerin ortalama kotu, eğim rasterinden ada başına en büyük eğim, arazi örtüsü rasterinden mahalle başına çoğunluk sınıfı.",
            "Bölgenin hücreleri merkezi alanın içinde olanlardır; örtüşen bölgeler aynı hücreyi ikisi de sayar. Değeri olmayan hücre sayılmaz; hiç hücresi olmayan bölgeye Sayı 0 yazılır, öbürleri alanı boşaltır.",
            "Toplam ve ortalama çift-çift toplamla, bir kez yuvarlanarak; standart sapma örneklemindir (n − 1).",
        ],
        &[
            "bölgesel",
            "zonal",
            "istatistik",
            "parsel",
            "ortalama kot",
            "zonal statistics",
        ],
        &["BOLGESELISTATISTIK", "ZONAL"],
        p,
        vec![
            OutputDef::new("table", "Bölgelerin istatistikleri", OutputKind::Table),
            OutputDef::new("changed", "Değişen nesneler", OutputKind::Features),
            OutputDef::new("count", "Yazılan nesne sayısı", OutputKind::Number),
        ],
        run_zonal,
    )
}

pub fn histogram() -> Tool {
    let p = vec![
        one_raster("Histogramı çıkarılacak raster."),
        band_param(),
        whole("bins", "Aralık sayısı", 20, 1, 1000),
        ParamDef::new(
            "min",
            "En küçük",
            ParamKind::Number {
                min: None,
                max: None,
                integer: false,
                unit: String::new(),
                placeholder: Some("Bandın".into()),
            },
        )
        .optional()
        .default_value(Value::Null)
        .describe("Boşsa bandın en küçük ve en büyük değeri; verilirse ikisi birlikte."),
        ParamDef::new(
            "max",
            "En büyük",
            ParamKind::Number {
                min: None,
                max: None,
                integer: false,
                unit: String::new(),
                placeholder: Some("Bandın".into()),
            },
        )
        .optional()
        .default_value(Value::Null),
    ];
    tool(
        ("raster.histogram", "Histogram", "histogram", STATS),
        "Bandın değerlerini eşit aralıklara bölüp her aralıktaki hücre sayısını tablo olarak verir.",
        &[
            "Aralık k = ⌊(x − alt) / (üst − alt) · n⌋ (son aralık üst sınırı da alır). Sınırların dışındaki değerler ayrıca sayılır.",
            "Tablo panoya kopyalanır ya da CSV olarak kaydedilir; çizim değişmez.",
        ],
        &["histogram", "dağılım", "frekans", "aralık"],
        &["HISTOGRAM"],
        p,
        vec![OutputDef::new("table", "Histogram", OutputKind::Table)],
        run_histogram,
    )
}

fn shown_rect(v: &Values) -> bool {
    text(v, "shape", "rect") == "rect"
}

fn shown_round(v: &Values) -> bool {
    text(v, "shape", "rect") != "rect"
}

fn shown_ring(v: &Values) -> bool {
    text(v, "shape", "rect") == "ring"
}

pub fn focal_statistics() -> Tool {
    let mut p = vec![
        one_raster("Değerleri okunan raster."),
        band_param(),
        choice(
            "shape",
            "Komşuluk",
            &[
                ("rect", "Dikdörtgen"),
                ("circle", "Daire"),
                ("ring", "Halka"),
            ],
        ),
        whole("width", "Genişlik", 3, 1, 255)
            .describe("Hücre; tek sayı.")
            .shown_when(shown_rect),
        whole("height", "Yükseklik", 3, 1, 255)
            .describe("Hücre; tek sayı.")
            .shown_when(shown_rect),
        whole("radius", "Yarıçap", 3, 1, 127)
            .describe("Hücre: merkezi bu uzaklıkta olan hücreler (halkada dış yarıçap).")
            .shown_when(shown_round),
        whole("inner", "İç yarıçap", 1, 1, 126)
            .describe("Hücre; dış yarıçaptan küçük.")
            .shown_when(shown_ring),
        stats(false),
        ignore_param(),
    ];
    p.extend(ends("-komsuluk", "Komşuluk"));
    tool(
        (
            "raster.focalStatistics",
            "Komşuluk istatistiği",
            "focalStats",
            STATS,
        ),
        "Her hücreye çevresindeki pencerenin istatistiğini yazar: ortalama, toplam, en küçük, en büyük, aralık, standart sapma, ortanca, çoğunluk, azınlık, çeşit.",
        &[
            "Pencere dikdörtgen (tek kenarlı), daire (merkezi r hücre içinde olanlar) ya da halkadır (iç ve dış yarıçap arası). Rasterin dışı pencerede sayılmaz.",
            "Değersizleri yok say açıkken değersiz merkez de çevresinde değer varsa değer alır (ArcGIS'in varsayılanı).",
            HELP_OUTPUT,
        ],
        &[
            "komşuluk",
            "odak",
            "focal",
            "filtre",
            "pencere",
            "neighborhood",
            "yumuşat",
        ],
        &["KOMSULUK", "ODAKISTATISTIK", "FOCAL"],
        p,
        file_output(),
        |r, cx, fb| {
            let shape = r.text("shape");
            run_raster_op(
                r,
                cx,
                fb,
                json!({
                    "kind": "focalStatistics",
                    "band": r.number("band").unwrap_or(1.0) as u32,
                    "shape": shape,
                    "width": r.number("width").unwrap_or(3.0) as u32,
                    "height": r.number("height").unwrap_or(3.0) as u32,
                    "radius": r.number("radius").unwrap_or(3.0) as u32,
                    "inner": r.number("inner").unwrap_or(1.0) as u32,
                    "stat": r.text("stat"),
                    "ignore": r.flag("ignore"),
                }),
                ("-komsuluk", "Komşuluk hesaplanıyor"),
                (true, None),
            )
        },
    )
}

pub fn cell_statistics() -> Tool {
    let mut p = vec![
        rasters(
            "İstatistiği alınacak rasterler (en az iki).",
            ScopeKind::Selection,
        ),
        band_param(),
        stats(true),
        ignore_param(),
    ];
    p.extend(ends("-hucre", "Hücre istatistiği"));
    tool(
        (
            "raster.cellStatistics",
            "Hücre istatistiği",
            "cellStats",
            STATS,
        ),
        "Rasterlerin her hücredeki değerlerinin istatistiğini yeni bir rastere yazar: yılların ortalaması, en büyüğü, sayısı.",
        &[
            "Sonucun ızgarası Mozaik'inki gibidir: en ince hücreli rasterinki, bütün rasterleri kapsar; rasterler hücre merkezlerinde en yakın hücreleriyle okunur.",
            HELP_EMPTY,
            HELP_OUTPUT,
        ],
        &[
            "hücre",
            "yığın",
            "cell statistics",
            "ortalama",
            "zaman serisi",
        ],
        &["HUCREISTATISTIK", "CELLSTATS"],
        p,
        file_output(),
        |r, cx, fb| {
            run_raster_op(
                r,
                cx,
                fb,
                json!({"kind": "cellStatistics", "band": r.number("band").unwrap_or(1.0) as u32, "stat": r.text("stat"), "ignore": r.flag("ignore")}),
                ("-hucre", "Hücre istatistiği hesaplanıyor"),
                (false, None),
            )
        },
    )
}
