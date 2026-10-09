//! The eight tools of Yüzey analizi (docs/adr/0231 §1, §10): their
//! parameters, and their settings handed to the raster core's job.

use kentos_raster::insolation::MONTH_NAMES;
use serde_json::{Value, json};

use super::{
    CATEGORY, band_param, choice, method_param, number, raster_param, result_layer, run_lines,
    run_raster, whole, z_param,
};
use crate::builtin::pointcloud::{add_param, output_param};
use crate::types::{OutputDef, OutputKind, ParamDef, ParamKind, RunResult, Target, Tool, Values};

/// The layers' colour for the results (a raster's is not drawn).
const RASTER_LAYER: &str = "#7A6B5B";
/// The contours' layer: brown, as the topographic maps draw them.
const CONTOUR_LAYER: &str = "#A0522D";

fn text<'a>(v: &'a Values, name: &str, or: &'a str) -> &'a str {
    v.get(name).and_then(Value::as_str).unwrap_or(or)
}

fn num(v: &Values, name: &str) -> Option<f64> {
    v.get(name).and_then(Value::as_f64)
}

/// A tool of the category: run on the desktop's other thread, its result a file.
fn tool(
    (id, label, icon): (&str, &str, &str),
    description: &str,
    help: &[&str],
    keywords: &[&str],
    aliases: &[&str],
    parameters: Vec<ParamDef>,
    run: crate::types::RunFn,
) -> Tool {
    let lines = id == "surface.contours";
    Tool {
        id: id.into(),
        label: label.into(),
        category: CATEGORY.into(),
        description: description.into(),
        help: Some(help.join("\n\n")),
        keywords: keywords.iter().map(|k| (*k).to_owned()).collect(),
        aliases: aliases.iter().map(|a| (*a).to_owned()).collect(),
        icon: Some(icon.into()),
        parameters,
        outputs: vec![if lines {
            OutputDef::new("lines", "Eğri sayısı", OutputKind::Number)
        } else {
            OutputDef::new("file", "Sonuç dosyası", OutputKind::Text)
        }],
        targets: vec![Target::Worker],
        validate: None,
        preview: None,
        run: Some(run),
    }
}

/// The parameters every raster tool ends with: Çıktı dosyası, Çizime ekle, Çıktı katmanı.
fn ends(suffix: &str, layer: &str) -> [ParamDef; 3] {
    [
        output_param(suffix, &[".tif"]),
        add_param(),
        result_layer(layer, RASTER_LAYER),
    ]
}

const HELP_SOURCE: &str = "Raster tek bir yükseklik rasteridir (DEM); bandın nodata'sı ve NaN boş sayılır. Pencerenin dışında kalan komşu kenar hücresinin, boş komşu merkezin değerini alır; boş hücrenin sonucu boştur. Coğrafi (derece) rasterde türevler satırın enleminde metreye çevrilir.";
const HELP_OUTPUT: &str = "Sonuç karolu, Deflate'li ve önizleme katlı GeoTIFF'tir; çıktı dosyası boşsa kaynağın yanına adının sonuna ek konarak yazılır. Çizime ekle açıksa raster yeni katmana eklenir; görünüşü Raster stili ile değişir.";

pub fn slope() -> Tool {
    let mut p = vec![
        raster_param(),
        band_param(),
        method_param(),
        choice(
            "unit",
            "Birim",
            &[("degrees", "Derece"), ("percent", "Yüzde")],
        ),
        z_param(),
    ];
    p.extend(ends("-egim", "Eğim"));
    tool(
        ("surface.slope", "Eğim", "slope"),
        "Yükseklik rasterinin her hücresinin eğimini derece ya da yüzde olarak yazar.",
        &[
            "Eğim, hücrenin 3 × 3 komşuluğundan bulunan yüzey eğiminin büyüklüğüdür: derece atan|g|, yüzde 100·|g|.",
            HELP_SOURCE,
            HELP_OUTPUT,
        ],
        &["eğim", "slope", "dem", "yüzey", "gdaldem"],
        &["EGIMANALIZI", "EGIMHARITASI"],
        p,
        |r, cx, fb| {
            run_raster(
                r,
                cx,
                fb,
                json!({"kind": "slope", "method": r.text("method"), "unit": r.text("unit"), "zFactor": r.number("zFactor")}),
                ("-egim", "Eğim hesaplanıyor"),
            )
        },
    )
}

pub fn aspect() -> Tool {
    let mut p = vec![raster_param(), band_param(), method_param(), z_param()];
    p.extend(ends("-baki", "Bakı"));
    tool(
        ("surface.aspect", "Bakı", "aspect"),
        "Her hücrenin baktığı yönü, en dik inişin kuzeyden saat yönündeki açısı olarak yazar; düzlük −1.",
        &[
            "Bakı 0–360 derecedir: 0 kuzey, 90 doğu, 180 güney, 270 batı. Eğimi sıfır olan hücre −1 alır.",
            HELP_SOURCE,
            HELP_OUTPUT,
        ],
        &["bakı", "aspect", "yön", "dem", "gdaldem"],
        &["BAKI"],
        p,
        |r, cx, fb| {
            run_raster(
                r,
                cx,
                fb,
                json!({"kind": "aspect", "method": r.text("method"), "zFactor": r.number("zFactor")}),
                ("-baki", "Bakı hesaplanıyor"),
            )
        },
    )
}

pub fn hillshade() -> Tool {
    let mut p = vec![
        raster_param(),
        band_param(),
        number("azimuth", "Işığın açısı", 315.0, 0.0, 360.0, "°")
            .describe("Işığın geldiği yön, kuzeyden saat yönünde (315: kuzeybatı)."),
        number("altitude", "Işığın yüksekliği", 45.0, 0.0, 90.0, "°")
            .describe("Işığın ufuktan yüksekliği."),
        z_param(),
    ];
    p.extend(ends("-golge", "Gölgeli kabartma"));
    tool(
        ("surface.hillshade", "Gölgeli kabartma", "hillshade"),
        "Arazinin verilen yönden ışıklandırılmış gri görüntüsünü yazar (1–255, boş hücre 0).",
        &[
            "Değer, yüzeyin ışığa dönüklüğüdür (gdaldem hillshade'in formülü): ışığa tam dönük yüzey 255, gölgede kalan 1.",
            HELP_SOURCE,
            HELP_OUTPUT,
        ],
        &["gölgeli kabartma", "hillshade", "rölyef", "kabartma", "dem"],
        &["GOLGEURET", "KABARTMAURET"],
        p,
        |r, cx, fb| {
            run_raster(
                r,
                cx,
                fb,
                json!({"kind": "hillshade", "azimuth": r.number("azimuth"), "altitude": r.number("altitude"), "zFactor": r.number("zFactor")}),
                ("-golge", "Gölgeli kabartma hesaplanıyor"),
            )
        },
    )
}

fn colors(v: &Values) -> &str {
    text(v, "colors", "ramp")
}

fn manual(v: &Values) -> bool {
    colors(v) == "ramp" && text(v, "range", "auto") == "manual"
}

fn relief_check(v: &Values) -> Option<String> {
    if colors(v) == "table" && text(v, "table", "").trim().is_empty() {
        return Some(
            "Renk tablosunu yazın: “değer #RRGGBB” satırları, ör. 100 #2E7D32; 500 #FFF59D.".into(),
        );
    }
    if manual(v) {
        match (num(v, "min"), num(v, "max")) {
            (Some(lo), Some(hi)) if lo < hi => {}
            _ => return Some("En küçük değer en büyük değerden küçük olmalı.".into()),
        }
    }
    None
}

pub fn color_relief() -> Tool {
    let ramps: Vec<(&str, &str)> = kentos_contracts::RASTER_RAMPS
        .iter()
        .map(|r| (*r, *r))
        .collect();
    let mut p = vec![
        raster_param(),
        band_param(),
        choice("colors", "Renkler", &[("ramp", "Rampa"), ("table", "Renk tablosu")]),
        choice("ramp", "Rampa", &ramps)
            .default_value(json!("Arazi"))
            .shown_when(|v| colors(v) == "ramp"),
        ParamDef::new("invert", "Ters", ParamKind::Boolean)
            .default_value(json!(false))
            .shown_when(|v| colors(v) == "ramp"),
        choice("range", "Değer aralığı", &[("auto", "Bandın en küçüğü–en büyüğü"), ("manual", "Elle")])
            .shown_when(|v| colors(v) == "ramp"),
        number("min", "En küçük", 0.0, -1e9, 1e9, "").shown_when(manual),
        number("max", "En büyük", 1000.0, -1e9, 1e9, "").shown_when(manual),
        ParamDef::new(
            "table",
            "Renk tablosu",
            ParamKind::Text {
                placeholder: Some("100 #2E7D32; 500 #FFF59D; 1000 #FAFAFA".into()),
                max_length: Some(20_000),
                allow_empty: true,
            },
        )
        .default_value(json!(""))
        .describe("Satırlar “değer #RRGGBB” ya da “değer #RRGGBBAA”, satır başına ya da noktalı virgülle ayrılarak (gdaldem color-relief'in metni gibi).")
        .shown_when(|v| colors(v) == "table"),
        choice("interp", "Ara renkler", &[("linear", "Doğrusal"), ("nearest", "En yakın")]),
    ];
    p.extend(ends("-renkli", "Renkli kabartma"));
    let mut t = tool(
        ("surface.colorRelief", "Renkli kabartma", "colorRelief"),
        "Yükseklikleri bir rampayla ya da renk tablosuyla renklendirip RGBA raster yazar.",
        &[
            "Rampa bandın en küçüğünden en büyüğüne (ya da elle verilen aralığa) yayılır. Renk tablosu “değer #RRGGBB” satırlarıdır; tablonun ilk değerinden küçük hücre ilk rengi, son değerinden büyüğü son rengi alır. Boş hücre saydamdır.",
            HELP_SOURCE,
            HELP_OUTPUT,
        ],
        &[
            "renkli kabartma",
            "color relief",
            "hipsometri",
            "renk",
            "dem",
        ],
        &["RENKLIKABARTMA", "HIPSOMETRI"],
        p,
        |r, cx, fb| {
            let table = r.text("colors") == "table";
            let manual = !table && r.text("range") == "manual";
            run_raster(
                r,
                cx,
                fb,
                json!({
                    "kind": "colorRelief",
                    "ramp": (!table).then(|| r.text("ramp")),
                    "invert": !table && r.flag("invert"),
                    "min": if manual { r.number("min") } else { None },
                    "max": if manual { r.number("max") } else { None },
                    "table": table.then(|| r.text("table")),
                    "interp": r.text("interp"),
                }),
                ("-renkli", "Renkli kabartma hesaplanıyor"),
            )
        },
    );
    t.validate = Some(relief_check);
    t
}

pub fn curvature() -> Tool {
    let mut p = vec![
        raster_param(),
        band_param(),
        choice(
            "curvature",
            "Eğrilik",
            &[("total", "Toplam"), ("profile", "Profil"), ("plan", "Plan")],
        )
        .describe("Toplam: yüzeyin eğriliği; profil: eğim yönünde; plan: eğriler boyunca."),
        z_param(),
    ];
    p.extend(ends("-egrilik", "Eğrilik"));
    tool(
        ("surface.curvature", "Eğrilik", "curvature"),
        "Yüzeyin toplam, profil ya da plan eğriliğini (ArcGIS'teki gibi 1/100 m⁻¹) yazar.",
        &[
            "Zevenbergen ve Thorne'un yüzeyinden: toplamda artı tepe, eksi çukur; profilde eksi dışbükey (akış yavaşlar), artı içbükey; planda artı sırt, eksi dere.",
            "Eğrilik dik pikselli raster ister (afinin eksenleri dik).",
            HELP_SOURCE,
            HELP_OUTPUT,
        ],
        &[
            "eğrilik",
            "eğrisellik",
            "curvature",
            "profil",
            "plan",
            "dem",
        ],
        &["EGRILIK", "EGRISELLIK"],
        p,
        |r, cx, fb| {
            run_raster(
                r,
                cx,
                fb,
                json!({"kind": "curvature", "curvature": r.text("curvature"), "zFactor": r.number("zFactor")}),
                ("-egrilik", "Eğrilik hesaplanıyor"),
            )
        },
    )
}

pub fn ruggedness() -> Tool {
    let mut p = vec![
        raster_param(),
        band_param(),
        choice(
            "index",
            "Ölçü",
            &[
                ("triRiley", "TRI (Riley)"),
                ("triWilson", "TRI (Wilson)"),
                ("tpi", "TPI"),
                ("roughness", "Engebe"),
            ],
        )
        .describe("TRI: komşuların merkezden farkı; TPI: merkezin komşuların ortalamasından farkı; Engebe: penceredeki en büyük ile en küçüğün farkı."),
    ];
    p.extend(ends("-puruzluluk", "Pürüzlülük"));
    tool(
        ("surface.ruggedness", "Pürüzlülük", "ruggedness"),
        "Arazinin pürüzlülüğünü TRI (Riley ya da Wilson), TPI ya da engebe olarak yazar.",
        &[
            "TRI (Riley): komşuların merkezden farklarının karelerinin toplamının karekökü; TRI (Wilson): farkların mutlak değerlerinin ortalaması; TPI: merkez eksi komşuların ortalaması; Engebe: 3 × 3 pencerenin en büyüğü eksi en küçüğü. Yükseklikler z çarpanıyla çarpılmaz (gdaldem gibi).",
            HELP_SOURCE,
            HELP_OUTPUT,
        ],
        &[
            "pürüzlülük",
            "tri",
            "tpi",
            "engebe",
            "ruggedness",
            "roughness",
            "dem",
        ],
        &["PURUZLULUK", "TRI", "TPI"],
        p,
        |r, cx, fb| {
            run_raster(
                r,
                cx,
                fb,
                json!({"kind": "ruggedness", "index": r.text("index")}),
                ("-puruzluluk", "Pürüzlülük hesaplanıyor"),
            )
        },
    )
}

const MONTHS: [(&str, &str); 12] = [
    ("1", MONTH_NAMES[0]),
    ("2", MONTH_NAMES[1]),
    ("3", MONTH_NAMES[2]),
    ("4", MONTH_NAMES[3]),
    ("5", MONTH_NAMES[4]),
    ("6", MONTH_NAMES[5]),
    ("7", MONTH_NAMES[6]),
    ("8", MONTH_NAMES[7]),
    ("9", MONTH_NAMES[8]),
    ("10", MONTH_NAMES[9]),
    ("11", MONTH_NAMES[10]),
    ("12", MONTH_NAMES[11]),
];

fn period(v: &Values) -> &str {
    text(v, "period", "year")
}

/// The period's first and last day of the year; why not.
pub fn period_days(v: &Values) -> Result<(u32, u32), String> {
    let day = |d: &str, m: &str| -> Result<u32, String> {
        let month = text(v, m, "1").parse::<u32>().unwrap_or(0);
        let day = num(v, d).unwrap_or(1.0);
        if day.fract() != 0.0 || !(1.0..=31.0).contains(&day) {
            return Err("Gün 1 ile 31 arasında bir tam sayı olmalı.".into());
        }
        kentos_raster::insolation::day_of_year(month, day as u32)
    };
    match period(v) {
        "year" => Ok((1, 365)),
        "day" => day("startDay", "startMonth").map(|d| (d, d)),
        _ => {
            let (first, last) = (day("startDay", "startMonth")?, day("endDay", "endMonth")?);
            if first > last {
                return Err("Bitiş başlangıçtan önce; aynı yıl içinde bir aralık verin.".into());
            }
            Ok((first, last))
        }
    }
}

fn insolation_check(v: &Values) -> Option<String> {
    period_days(v).err()
}

pub fn insolation() -> Tool {
    let mut p = vec![
        raster_param(),
        band_param(),
        choice("period", "Dönem", &[("year", "Yıl"), ("range", "Tarih aralığı"), ("day", "Tek gün")]),
        whole("startDay", "Başlangıç günü", 21, 1, 31).shown_when(|v| period(v) != "year"),
        choice("startMonth", "Başlangıç ayı", &MONTHS)
            .default_value(json!("6"))
            .shown_when(|v| period(v) != "year"),
        whole("endDay", "Bitiş günü", 21, 1, 31).shown_when(|v| period(v) == "range"),
        choice("endMonth", "Bitiş ayı", &MONTHS)
            .default_value(json!("9"))
            .shown_when(|v| period(v) == "range"),
        whole("dayStep", "Gün aralığı", 14, 1, 365)
            .describe("Dönem bu kadar günlük parçalara bölünür; her parçayı ortasındaki gün temsil eder.")
            .shown_when(|v| period(v) != "day"),
        choice(
            "hourStep",
            "Saat aralığı",
            &[("0.25", "15 dakika"), ("0.5", "30 dakika"), ("1", "1 saat"), ("2", "2 saat")],
        )
        .default_value(json!("0.5")),
        number("transmissivity", "Geçirgenlik", 0.5, 0.01, 1.0, "")
            .describe("Atmosferin dik gelen ışığı geçirme oranı (açık gök 0,5–0,7)."),
        ParamDef::new(
            "latitude",
            "Enlem",
            ParamKind::Number {
                min: Some(-90.0),
                max: Some(90.0),
                integer: false,
                unit: "°".into(),
                placeholder: Some("Projenin sisteminden".into()),
            },
        )
        .optional()
        .default_value(Value::Null)
        .describe("Yalnız koordinat sistemi olmayan (yerel) projede gerekir; sistemi olan projede her satırın kendi enlemi kullanılır."),
        z_param(),
    ];
    p.extend(ends("-gunes", "Güneşlenme"));
    let mut t = tool(
        ("surface.insolation", "Güneşlenme", "insolation"),
        "Dönemde eğimli yüzeye gelen doğrudan güneş enerjisini (açık gök, kWh/m²) yazar.",
        &[
            "Güneşin yeri Spencer'ın formülleriyle, hava kütlesi Kasten ve Young'ınkiyle bulunur; ışınım 1367 W/m²·E₀·τ^m·max(0, n·s). Yalnız yüzeyin kendi gölgesi vardır; arazinin gölgesi ve dağınık ışınım hesaba girmez.",
            "Dönem 365 günlük yıldır; Gün aralığı ve Saat aralığı hesabın sıklığıdır.",
            HELP_SOURCE,
            HELP_OUTPUT,
        ],
        &[
            "güneşlenme",
            "güneş",
            "radyasyon",
            "ışınım",
            "insolation",
            "solar",
            "dem",
        ],
        &["GUNESLENME", "GUNESRADYASYONU"],
        p,
        |r, cx, fb| {
            let (first, last) = match period_days(&r.values) {
                Ok(days) => days,
                Err(why) => return RunResult::refused(why),
            };
            let step = if r.text("period") == "day" {
                1.0
            } else {
                r.number("dayStep").unwrap_or(14.0)
            };
            run_raster(
                r,
                cx,
                fb,
                json!({
                    "kind": "insolation",
                    "firstDay": first,
                    "lastDay": last,
                    "dayStep": step as u32,
                    "hourStep": r.text("hourStep").parse::<f64>().unwrap_or(0.5),
                    "transmissivity": r.number("transmissivity"),
                    "latitude": r.number("latitude"),
                    "zFactor": r.number("zFactor"),
                }),
                ("-gunes", "Güneşlenme hesaplanıyor"),
            )
        },
    );
    t.validate = Some(insolation_check);
    t
}

pub fn contours() -> Tool {
    let p = vec![
        raster_param(),
        band_param(),
        number("interval", "Aralık", 5.0, 0.001, 1e6, "m")
            .describe("İki eğri arasındaki yükseklik farkı."),
        number("base", "Taban", 0.0, -1e6, 1e6, "m")
            .describe("Düzeyler tabandan aralık aralık sayılır."),
        whole("indexEvery", "Ana eğri her", 5, 1, 1000)
            .describe("Bu kadar eğride bir ana eğri (kalın çizilir)."),
        number("simplify", "Sadeleştir", 0.0, 0.0, 1e4, "m")
            .describe("Douglas-Peucker toleransı; 0 sadeleştirmez."),
        result_layer("Eş yükselti eğrileri", CONTOUR_LAYER),
    ];
    tool(
        ("surface.contours", "Eş yükselti eğrileri", "contours"),
        "Yükseklik rasterinden kotlu eş yükselti eğrileri (ana ve ara) çıkarır.",
        &[
            "Eğriler hücre merkezlerinin karelerinden (marching squares) çıkar; eyerde karenin ortası karar verir. Yüksek taraf eğrinin solundadır; boş hücrede eğri kesilir.",
            "Her eğri çoklu çizgidir: köşelerinin kotu düzeydir, öznitelikleri Kot ve Tür (Ana ya da Ara); ana eğri 0,35 mm kalınlıktadır.",
            "Raster tek bir yükseklik rasteridir (DEM); bandın nodata'sı ve NaN boş sayılır. 10 000'den çok düzey ya da 5 milyondan çok köşe reddedilir: aralığı büyütün.",
        ],
        &[
            "eş yükselti",
            "eşyükselti",
            "kontur",
            "contour",
            "eğri",
            "dem",
        ],
        &["ESYUKSELTI", "KONTUR", "EGRIURET"],
        p,
        |r, cx, fb| {
            run_lines(
                r,
                cx,
                fb,
                json!({
                    "kind": "contours",
                    "interval": r.number("interval"),
                    "base": r.number("base"),
                    "indexEvery": r.number("indexEvery").unwrap_or(5.0) as u32,
                    "simplify": r.number("simplify").unwrap_or(0.0),
                }),
            )
        },
    )
}
