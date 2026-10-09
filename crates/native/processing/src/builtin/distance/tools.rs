//! The four tools of Uzaklık ve maliyet (docs/adr/0236 §3–§7): their
//! parameters as the web's (`builtin/distance/tools.ts`), and their settings
//! handed to the raster core.

use serde_json::{Value, json};

use super::{DISTANCE, ends_of, run_from_objects, run_paths, run_raster, shapes_of, trimmed};
use crate::builtin::geometry::output_style;
use crate::builtin::pointcloud::{add_param, count_words, output_param, result_layer_param};
use crate::builtin::raster_vector::BURN_KINDS;
use crate::builtin::surface::{choice, number};
use crate::types::{OutputDef, OutputKind, ParamDef, ParamKind, ScopeKind, Target, Tool, Values};

/// The results' layers' colour (a raster's is not drawn).
const RASTER_LAYER: &str = "#7A6B5B";

fn text<'a>(v: &'a Values, name: &str, or: &'a str) -> &'a str {
    v.get(name).and_then(Value::as_str).unwrap_or(or)
}

fn from_raster(v: &Values) -> bool {
    text(v, "from", "objects") == "raster"
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
        category: DISTANCE.into(),
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

const HELP_SIZE: &str = "Bütün raster bellekte çalışılır: en çok 2²⁵ hücre (5792 × 5792).";
const HELP_BURN: &str = "Nesneler hücrelere Rasterleştir'in kuralıyla düşer: kapalı şekil merkezini içine alan hücrelere, açık şekil dokunduğu hücrelere, nokta içinde olduğu hücreye. Bir hücreye birden çok kaynak düşerse girdide önce gelenindir; hiçbir hücreye düşmeyen nesne söylenir.";
const HELP_COST: &str = "Maliyet metre başınadır; değersiz hücre (nodata, NaN, alfası 0) engeldir. 0 ya da eksi değer reddedilir: geçilmeyecek yeri değersiz yapın, çok ucuz yere küçük bir artı değer verin.";
const HELP_STEP: &str = "Hücre merkezleri 8 komşuya ya da at hamleleriyle 16 komşuya bağlanır. Dik ve çapraz adımın maliyeti iki hücrenin ortalaması çarpı adımın uzunluğu, at hamlesininki iki uçla geçtiği iki hücrenin ortalaması çarpı uzunluk (GRASS'ın r.cost'u gibi). Değersiz hücreye adım atılmaz; iki yanı da değersiz çapraz adım ve geçtiği hücrelerden biri değersiz at hamlesi atılmaz: köşeden bağlı engel geçilmez.";
const HELP_SURFACE: &str = "Yükseklik modeliyle açıkken yükseklik modeli maliyet rasterinin ızgarasına çift doğrusal okunur: Yüzey uzunluğu açıkken adımın uzunluğu yükseklik farkıyla üç boyutta alınır; En büyük boyuna eğimden dik adım (iki yönde) atılmaz, yol yamaçta kıvrılır. Yükseklik modelinin değersiz hücresine adım atılmaz.";
const HELP_NET: &str = "Ağın bozulması: sabit maliyette ağın maliyeti düz uzaklığın en çok %8,24 (8 komşu) ve %2,75 (16 komşu) fazlasıdır; her yolun maliyeti adımlarından yeniden hesaplanabilir.";

fn file_output() -> Vec<OutputDef> {
    vec![OutputDef::new("file", "Sonuç dosyası", OutputKind::Text)]
}

fn burn_kinds() -> Option<Vec<String>> {
    Some(BURN_KINDS.iter().map(|k| (*k).to_owned()).collect())
}

fn objects(name: &str, label: &str, description: &str) -> ParamDef {
    ParamDef::new(
        name,
        label,
        ParamKind::Features {
            kinds: burn_kinds(),
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

fn raster(name: &str, label: &str, description: &str) -> ParamDef {
    ParamDef::new(
        name,
        label,
        ParamKind::Features {
            kinds: Some(vec!["raster".to_owned()]),
            scopes: Some(vec![ScopeKind::Selection, ScopeKind::Layer]),
            writes: false,
        },
    )
    .describe(description)
}

fn band(description: &str) -> ParamDef {
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
    .describe(description)
}

/// Maliyet rasteri and its band.
fn cost_raster() -> [ParamDef; 2] {
    [
        raster(
            "input",
            "Maliyet rasteri",
            "Metre başına maliyet; değersiz hücre engeldir.",
        ),
        band("Maliyetin okunduğu bant (1'den)."),
    ]
}

fn with_surface(v: &Values) -> bool {
    v.get("useSurface")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

/// Komşuluk; Yükseklik modeliyle and, while it is on, Yükseklik modeli, Yüzey uzunluğu and En büyük boyuna eğim.
fn network() -> [ParamDef; 5] {
    [
        choice(
            "neighbours",
            "Komşuluk",
            &[("16", "16 (at hamleleriyle)"), ("8", "8")],
        )
        .describe("16 komşuda ağın düz uzaklıktan sapması en çok %2,75, 8 komşuda %8,24."),
        ParamDef::new("useSurface", "Yükseklik modeliyle", ParamKind::Boolean)
            .default_value(json!(false))
            .describe("Yüzey uzunluğu ve En büyük boyuna eğim için bir yükseklik modeli okunur."),
        raster(
            "surface",
            "Yükseklik modeli",
            "Maliyet rasterinin ızgarasına çift doğrusal okunur.",
        )
        .optional()
        .shown_when(with_surface),
        ParamDef::new("surfaceLength", "Yüzey uzunluğu", ParamKind::Boolean)
            .default_value(json!(false))
            .describe("Adımın uzunluğu yükseklik farkıyla, üç boyutta.")
            .shown_when(with_surface),
        number("slope", "En büyük boyuna eğim", 0.0, 0.0, 1000.0, "")
            .describe("Yüzde; bundan dik adım atılmaz (iki yönde); 0: sınırsız.")
            .shown_when(with_surface),
    ]
}

fn sample() -> ParamDef {
    choice(
        "sample",
        "Sonuç türü",
        &[("f32", "Ondalık 32 bit"), ("f64", "Ondalık 64 bit")],
    )
}

/// Çıktı katmanı of a raster result (shown while Çizime ekle is on): a new
/// one goes right above the input raster's layer, or right below the
/// sources' (`below`, Uzaklık yüzeyi from objects).
fn raster_layer(name: &str, below: Option<&str>, description: &str) -> ParamDef {
    let mut p = result_layer_param(name, RASTER_LAYER).describe(description);
    if let ParamKind::Layer {
        above, below: b, ..
    } = &mut p.kind
    {
        *above = Some("input".to_owned());
        *b = below.map(str::to_owned);
    }
    p
}

const ABOVE_COST: &str =
    "Bu adda katman yoksa oluşturulur; maliyet rasterinin katmanının hemen üstünde.";

/// The network's settings as the core reads them (the surface's only while Yükseklik modeliyle is on).
fn network_of(r: &crate::types::Resolved<'_>) -> (&'static str, bool, f64) {
    let surface = r.flag("useSurface");
    (
        if r.text("neighbours") == "8" {
            "8"
        } else {
            "16"
        },
        surface && r.flag("surfaceLength"),
        if surface {
            r.number("slope").unwrap_or(0.0)
        } else {
            0.0
        },
    )
}

fn band_of(r: &crate::types::Resolved<'_>) -> u32 {
    r.number("band").unwrap_or(1.0) as u32
}

pub fn euclidean() -> Tool {
    tool(
        ("distance.euclidean", "Uzaklık yüzeyi", "distanceSurface"),
        "Her hücreye en yakın kaynağın düz uzaklığını (m) ya da en yakın kaynağın numarasını yazar.",
        &[
            "Kaynaklar nesneler (nokta, çizgi, alan) ya da bir rasterin bandının değerli hücreleridir. Hücrenin uzaklığı merkezinin en yakın kaynak hücrenin merkezine düz uzaklığıdır; kaynak hücrede 0. En yakın kaynak, eşitse sütunu, o da eşitse satırı küçük olan hücredir; En yakın kaynak sonucu nesnelerde nesnenin girdideki sırası, rasterde hücrenin değeridir.",
            "Hesap Felzenszwalb ve Huttenlocher'in kesin uzaklık dönüşümüdür: önce sütunlar, sonra satırlar, iş parçacıklarında; kare hücrede karşılaştırmalar tam sayılarla kesindir.",
            HELP_BURN,
            "Nesnelerde ızgara kaynakların kutusudur, her yanda Kenar payı kadar geniş; Hücre boyu 0 ise kutunun kısa kenarının 250'de biri, 1, 2, 2,5 ya da 5 × 10ᵏ'ye yuvarlanır. Rasterin ızgarası seçilirse sonuç o rasterle hücre hücre üst üste gelir. Rasterde ızgara rasterin kendisidir.",
            "Düz uzaklık engelin üstünden ölçülür; engelin çevresinden uzaklık için Birikimli maliyet'i 1 maliyetle kullanın. Izgaranın eksenleri dik olmalıdır; coğrafi ızgarada uzaklık metre olmadığından reddedilir.",
            "Sonuç 32 bit; Uzaklık'ta Viridis, En yakın kaynak'ta Spektral ve en yakın örnekleme. Çizime ekle açıksa nesnelerin sonucu kaynakların katmanının hemen altına, rasterinki rasterin katmanının hemen üstüne eklenir.",
            HELP_SIZE,
        ],
        &[
            "uzaklık",
            "mesafe",
            "distance",
            "proximity",
            "euclidean",
            "tampon",
            "en yakın",
            "allocation",
        ],
        &["UZAKLIK", "PROXIMITY", "EUCDIST"],
        vec![
            choice("from", "Kaynak", &[("objects", "Nesneler"), ("raster", "Raster")]),
            objects(
                "sources",
                "Kaynaklar",
                "Uzaklığın ölçüldüğü noktalar, çizgiler ve alanlar.",
            )
            .shown_when(|v| !from_raster(v)),
            raster(
                "input",
                "Kaynak raster",
                "Bandının değerli hücreleri kaynaktır.",
            )
            .shown_when(from_raster),
            band("Kaynak hücrelerin okunduğu bant (1'den).").shown_when(from_raster),
            number("max", "En büyük uzaklık", 0.0, 0.0, 1e9, "m")
                .describe("Bundan uzak hücreler değersiz; 0: sınırsız."),
            choice(
                "result",
                "Sonuç",
                &[("distance", "Uzaklık"), ("allocation", "En yakın kaynak")],
            ),
            number("margin", "Kenar payı", 100.0, 0.0, 1e7, "m")
                .describe("Kaynakların kutusu her yanda bu kadar genişletilir.")
                .shown_when(|v| !from_raster(v) && text(v, "extent", "objects") != "raster"),
            number("cellSize", "Hücre boyu", 0.0, 0.0, 1e6, "m")
                .describe("0: kendiliğinden, kutunun kısa kenarının 250'de biri yuvarlanarak (1, 2, 2,5, 5 × 10ᵏ).")
                .shown_when(|v| !from_raster(v) && text(v, "extent", "objects") != "raster"),
            choice(
                "extent",
                "Kapsam",
                &[("objects", "Kaynakların kutusu"), ("raster", "Rasterin ızgarası")],
            )
            .describe("Rasterin ızgarası: sonuç seçilen rasterle hücre hücre üst üste gelir.")
            .shown_when(|v| !from_raster(v)),
            raster("grid", "Izgara rasteri", "Izgarası (yeri, hücre boyu, boyu) alınan raster.")
                .optional()
                .shown_when(|v| !from_raster(v) && text(v, "extent", "objects") == "raster"),
            output_param("-uzaklik", &[".tif"]),
            add_param(),
            raster_layer(
                "Uzaklık",
                Some("sources"),
                "Bu adda katman yoksa oluşturulur: nesnelerin sonucu kaynakların katmanının hemen altında, rasterinki rasterin katmanının hemen üstünde.",
            ),
        ],
        file_output(),
        |r, cx, fb| {
            let allocation = r.text("result") == "allocation";
            let result = if allocation { "allocation" } else { "distance" };
            let max = r.number("max").unwrap_or(0.0);
            if r.text("from") != "raster" {
                let tool = json!({
                    "kind": "distance",
                    "max": max,
                    "result": result,
                    "margin": r.number("margin").unwrap_or(100.0),
                });
                return run_from_objects(r, cx, fb, tool);
            }
            let tool = json!({ "kind": "distance", "band": band_of(r), "max": max, "result": result });
            run_raster(
                r,
                cx,
                fb,
                tool,
                (Vec::new(), "kaynak"),
                ("-uzaklik", "Uzaklık yüzeyi hesaplanıyor"),
                &|n, _, _| {
                    let d = &n.distance;
                    let mut s = format!(" {} kaynak hücre", count_words(d.sources));
                    if !allocation && d.cells > 0 {
                        s.push_str(&format!("; en uzak hücre {} m", trimmed(d.most, 3)));
                    }
                    s.push('.');
                    s
                },
            )
        },
    )
}

pub fn cost() -> Tool {
    let mut p = cost_raster().to_vec();
    p.push(objects(
        "sources",
        "Kaynaklar",
        "Maliyetin başladığı noktalar, çizgiler ve alanlar.",
    ));
    p.extend(network());
    p.extend([
        number("max", "En büyük maliyet", 0.0, 0.0, 1e300, "")
            .describe("Bundan pahalı hücreler değersiz; 0: sınırsız."),
        choice(
            "result",
            "Sonuç",
            &[
                ("cost", "Birikimli maliyet"),
                ("allocation", "En ucuz kaynak"),
            ],
        ),
        sample().shown_when(|v| text(v, "result", "cost") != "allocation"),
        output_param("-maliyet", &[".tif"]),
        add_param(),
        raster_layer("Birikimli maliyet", None, ABOVE_COST),
    ]);
    tool(
        ("distance.cost", "Birikimli maliyet", "costDistance"),
        "Her hücreye en ucuz kaynaktan varmanın birikimli maliyetini ya da o kaynağın numarasını yazar.",
        &[
            HELP_COST,
            HELP_STEP,
            HELP_SURFACE,
            "Birikimli maliyet kaynak hücrelerde 0, öbürlerinde komşunun maliyeti artı adımın maliyetinin en küçüğüdür (Dijkstra); toplamlar float64'te yapılır, hesabın sırası sonucu değiştirmez. Erişilemeyen ya da En büyük maliyeti aşan hücre değersizdir. En ucuz kaynak hücrenin geldiği komşunun kaynağıdır: girdideki sırası.",
            HELP_BURN,
            HELP_NET,
            "Sonuç Birikimli maliyet'te Viridis ve yüzde gerdirme, En ucuz kaynak'ta Spektral ve en yakın örnekleme; Çizime ekle açıksa maliyet rasterinin katmanının hemen üstüne eklenir.",
            HELP_SIZE,
        ],
        &[
            "maliyet",
            "cost distance",
            "birikimli",
            "accumulation",
            "erişim",
            "güzergâh",
            "r.cost",
            "hizmet alanı",
        ],
        &["MALIYET", "COSTDIST"],
        p,
        file_output(),
        |r, cx, fb| {
            let (neighbours, surface_length, slope) = network_of(r);
            let allocation = r.text("result") == "allocation";
            let tool = json!({
                "kind": "costDistance",
                "band": band_of(r),
                "neighbours": neighbours,
                "surfaceLength": surface_length,
                "slope": slope,
                "max": r.number("max").unwrap_or(0.0),
                "result": if allocation { "allocation" } else { "cost" },
                "sample": if r.text("sample") == "f64" { "f64" } else { "f32" },
            });
            let shapes = shapes_of(r, "sources");
            run_raster(
                r,
                cx,
                fb,
                tool,
                (shapes, "kaynak nesnesi"),
                ("-maliyet", "Birikimli maliyet hesaplanıyor"),
                &|n, cells, _| {
                    let d = &n.distance;
                    let mut s = format!(" {} kaynak hücre", count_words(d.sources));
                    if !allocation && d.cells > 0 {
                        s.push_str(&format!(
                            "; en büyük birikimli maliyet {}",
                            trimmed(d.most, 3)
                        ));
                    }
                    s.push('.');
                    if d.cells < cells {
                        s.push_str(&format!(
                            " Değersiz {} hücre (engel, erişilemeyen ya da sınırın ötesi).",
                            count_words(cells - d.cells)
                        ));
                    }
                    s
                },
            )
        },
    )
}

pub fn path() -> Tool {
    let mut p = cost_raster().to_vec();
    p.push(objects(
        "sources",
        "Başlangıç",
        "Yolların başladığı noktalar, çizgiler ve alanlar: her varış en ucuzuna bağlanır.",
    ));
    p.push(objects(
        "targets",
        "Varış",
        "Her varış nesnesinden bir yol: hücrelerinden birikimli maliyeti en küçük olanından.",
    ));
    p.extend(network());
    p.extend([
        number("simplify", "Sadeleştirme", 0.0, 0.0, 1000.0, "").describe(
            "Douglas–Peucker toleransı, hücre cinsinden; 0 yalnız doğrultudaki köşeleri atar.",
        ),
        ParamDef::new(
            "layer",
            "Çıktı katmanı",
            ParamKind::Layer {
                new_layer_style: output_style("#E5484D"),
                above: Some("input".to_owned()),
                below: None,
            },
        )
        .default_value(json!({ "newName": "En düşük maliyetli yol" }))
        .describe("Bu adda katman yoksa oluşturulur; maliyet rasterinin katmanının hemen üstünde."),
    ]);
    tool(
        ("distance.path", "En düşük maliyetli yol", "costPath"),
        "Her varış nesnesinden en ucuz başlangıca giden en düşük maliyetli yolu çoklu çizgi olarak yazar.",
        &[
            "Varış hücresi, varış nesnesinin hücrelerinden birikimli maliyeti en küçük olanıdır (eşitse numarası küçük olan). Yol, hücrenin geldiği komşular boyunca başlangıca iner: geldiği komşu, maliyeti ve adımı hücrenin maliyetini veren komşulardan maliyeti, o da eşitse numarası küçük olanıdır. Çizgi başlangıçtan varışa doğru hücre merkezlerindendir; Sadeleştirme hücre cinsinden Douglas–Peucker toleransıdır.",
            "Öznitelikler: Yol (varış nesnesinin girdideki sırası), Kaynak (başlangıç nesnesinin sırası), Maliyet (varış hücresinin birikimli maliyeti), Uzunluk (m, adımların plan uzunlukları); yükseklik modeli verilince Yüzey uzunluğu (m) ve En büyük eğim (%). Erişilemeyen ya da rasterin dışındaki varış atlanır, söylenir.",
            HELP_COST,
            HELP_STEP,
            HELP_SURFACE,
            "Dijkstra varışların hücreleri kesinleşince durur. Yollar maliyet rasterinin katmanının hemen üstündeki yeni katmana yazılır.",
            HELP_SIZE,
        ],
        &[
            "yol",
            "güzergâh",
            "güzergah",
            "least cost path",
            "en ucuz",
            "optimal path",
            "r.path",
            "boru hattı",
        ],
        &["GUZERGAHBUL", "COSTPATH"],
        p,
        vec![
            OutputDef::new("objects", "Yollar", OutputKind::Features),
            OutputDef::new("count", "Nesne sayısı", OutputKind::Number),
        ],
        |r, cx, fb| {
            let (neighbours, surface_length, slope) = network_of(r);
            let (shapes, first) = ends_of(r, "targets");
            let tool = json!({
                "kind": "costPath",
                "band": band_of(r),
                "neighbours": neighbours,
                "surfaceLength": surface_length,
                "slope": slope,
                "simplify": r.number("simplify").unwrap_or(0.0),
                "first": first,
            });
            run_paths(r, cx, fb, tool, shapes)
        },
    )
}

pub fn corridor() -> Tool {
    let mut p = cost_raster().to_vec();
    p.push(objects(
        "sources",
        "Birinci uçlar",
        "Koridorun bir ucu: noktalar, çizgiler ya da alanlar.",
    ));
    p.push(objects("targets", "İkinci uçlar", "Koridorun öbür ucu."));
    p.extend(network());
    p.extend([
        choice(
            "threshold",
            "Eşik",
            &[
                ("none", "Yok"),
                ("percent", "En küçük toplamın yüzdesi"),
                ("value", "Birikimli maliyet"),
            ],
        )
        .describe("Eşiği aşan hücre değersiz: koridor en ucuz yolun çevresinde kalır."),
        number("percent", "Yüzde", 10.0, 0.0, 1e6, "")
            .describe("En küçük toplamın bu kadar yüzde fazlasına kadar.")
            .shown_when(|v| text(v, "threshold", "none") == "percent"),
        number("value", "Eşik değeri", 0.0, 0.0, 1e300, "")
            .describe("Toplamı bundan büyük olmayan hücreler.")
            .shown_when(|v| text(v, "threshold", "none") == "value"),
        sample(),
        output_param("-koridor", &[".tif"]),
        add_param(),
        raster_layer("Maliyet koridoru", None, ABOVE_COST),
    ]);
    tool(
        ("distance.corridor", "Maliyet koridoru", "costCorridor"),
        "İki uçtan birikimli maliyetlerin toplamını yazar: en küçük toplam en ucuz yolun maliyetidir, koridor onun çevresidir.",
        &[
            "Koridor K = A'dan birikimli maliyet + B'den birikimli maliyettir; ikisinden biri tanımlı değilse hücre değersizdir. En küçük K, en ucuz A–B yolunun maliyetidir ve yol bu hücrelerden geçer. Eşik: yok, en küçük toplamın yüzdesi (K ≤ en küçük · (1 + yüzde/100)) ya da bir birikimli maliyet; eşiği aşan hücre değersizdir (ArcGIS'in Least Cost Corridor'ı).",
            HELP_COST,
            HELP_STEP,
            HELP_SURFACE,
            "İki uçtan aramalar iki iş parçacığında çalışır. Sonuç Viridis; Çizime ekle açıksa maliyet rasterinin katmanının hemen üstüne eklenir.",
            HELP_SIZE,
        ],
        &[
            "koridor",
            "corridor",
            "güzergâh",
            "least cost corridor",
            "maliyet",
            "bant",
        ],
        &["KORIDOR", "CORRIDOR"],
        p,
        file_output(),
        |r, cx, fb| {
            let (neighbours, surface_length, slope) = network_of(r);
            let (shapes, first) = ends_of(r, "targets");
            let threshold = match r.text("threshold") {
                "percent" => "percent",
                "value" => "value",
                _ => "none",
            };
            let value = match threshold {
                "percent" => r.number("percent").unwrap_or(10.0),
                "value" => r.number("value").unwrap_or(0.0),
                _ => 0.0,
            };
            let tool = json!({
                "kind": "costCorridor",
                "band": band_of(r),
                "neighbours": neighbours,
                "surfaceLength": surface_length,
                "slope": slope,
                "first": first,
                "threshold": threshold,
                "value": value,
                "sample": if r.text("sample") == "f64" { "f64" } else { "f32" },
            });
            run_raster(
                r,
                cx,
                fb,
                tool,
                (shapes, "uç nesnesi"),
                ("-koridor", "Maliyet koridoru hesaplanıyor"),
                &|n, _, _| {
                    let d = &n.distance;
                    let least = trimmed(d.least, 3);
                    if d.cells == 0 {
                        return format!(
                            " En ucuz yolun maliyeti {least}; eşiğin içinde hücre yok."
                        );
                    }
                    format!(
                        " En ucuz yolun maliyeti {least}; koridorda {} hücre.",
                        count_words(d.cells)
                    )
                },
            )
        },
    )
}
