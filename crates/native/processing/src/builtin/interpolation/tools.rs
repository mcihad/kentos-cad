//! The seven tools of İnterpolasyon and Yoğunluk (docs/adr/0232 §1, §13):
//! their parameters, and their settings handed to the raster core's point job.

use serde_json::{Value, json};

use super::{
    DENSITY, INTERPOLATION, Input, LINE_KINDS, POINT_KINDS, cross_param, field_param, grid_params,
    input_param, options, result_layer, run_points,
};
use crate::builtin::pointcloud::{add_param, output_param};
use crate::builtin::surface::{number, whole};
use crate::types::{OutputDef, OutputKind, ParamDef, Target, Tool, Values};

fn text<'a>(v: &'a Values, name: &str) -> &'a str {
    v.get(name).and_then(Value::as_str).unwrap_or("")
}

/// A tool of the categories: run on the desktop's other thread, its result a file.
#[allow(clippy::too_many_arguments)]
fn tool(
    (id, label, icon): (&str, &str, &str),
    category: &str,
    description: &str,
    help: &[&str],
    keywords: &[&str],
    aliases: &[&str],
    parameters: Vec<ParamDef>,
    run: crate::types::RunFn,
) -> Tool {
    let mut outputs = vec![OutputDef::new("file", "Sonuç dosyası", OutputKind::Text)];
    if category == INTERPOLATION {
        outputs.push(OutputDef::new(
            "table",
            "Çapraz doğrulama",
            OutputKind::Table,
        ));
    }
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

/// An interpolation's first parameters: Noktalar and Değer alanı.
fn points() -> [ParamDef; 2] {
    [
        input_param(
            "Noktalar",
            &POINT_KINDS,
            "Değerleri okunan noktalar; çizgi, çoklu çizgi ve alanların köşeleri de alınır.",
        ),
        field_param(
            "field",
            "Değer alanı",
            "Boş bırakılırsa köşelerin kotu okunur; seçilirse nesnenin bu alandaki sayısı bütün köşelerine verilir.",
        ),
    ]
}

/// The parameters every tool ends with: the grid, (Çapraz doğrulama,) Çıktı dosyası, Çizime ekle, Çıktı katmanı.
fn ends(suffix: &str, layer: &str, cross: bool) -> Vec<ParamDef> {
    let mut p: Vec<ParamDef> = grid_params().into();
    if cross {
        p.push(cross_param());
    }
    p.push(output_param(suffix, &[".tif"]));
    p.push(add_param());
    p.push(result_layer(layer, "layer"));
    p
}

const HELP_POINTS: &str = "Noktaların, çizgi ve alanların köşelerinin kotları (ya da Değer alanı'nın sayıları) alınır; aynı yerdeki köşeler değerlerinin ortalamasıyla tek nokta olur. Kotu olmayan köşe ve sayı olmayan değer alınmaz, söylenir.";
const HELP_GRID: &str = "Hücre boyu 0 ise noktaların kutusunun kısa kenarının 250'de biri, 1, 2, 2,5 ya da 5 × 10ᵏ'ye yuvarlanır; ızgara bu boyun katlarına oturur. Rasterin ızgarası seçilirse sonuç o rasterle hücre hücre üst üste gelir. Değer hücrenin merkezinde hesaplanır.";
const HELP_OUTPUT: &str = "Sonuç karolu, Deflate'li ve önizleme katlı 32 bit GeoTIFF'tir; çıktı dosyası boşsa çizimin klasörüne girdinin katmanının adıyla yazılır. Çizime ekle açıksa raster, girdinin katmanının hemen altındaki yeni katmana eklenir.";
const HELP_CROSS: &str = "Çapraz doğrulama her noktayı dışarıda bırakıp öbürlerinden tahmin eder: tabloda ölçülen, tahmin ve fark; özette ortalama fark, karesel ortalama hata ve ortalama mutlak fark.";

pub fn idw() -> Tool {
    let mut p: Vec<ParamDef> = points().into();
    p.extend([
        number("power", "Üs", 2.0, 0.1, 10.0, "")
            .describe("Ağırlık uzaklığın bu üssüyle azalır: büyük üs yakın noktaları öne çıkarır."),
        whole("points", "Nokta sayısı", 12, 1, 64).describe("Her hücrede en yakın bu kadar nokta."),
        number("radius", "Arama yarıçapı", 0.0, 0.0, 1e7, "m")
            .describe("0: sınırsız; verilirse yalnız bu uzaklıktaki noktalar."),
        whole("minPoints", "En az nokta", 1, 1, 64)
            .describe("Yarıçapta bundan az nokta olan hücre boş kalır."),
    ]);
    p.extend(ends("-idw", "IDW", true));
    tool(
        ("interpolation.idw", "Ters uzaklık (IDW)", "idw"),
        INTERPOLATION,
        "Noktalardan yüzey: her hücre en yakın noktaların uzaklıklarının ters üssüyle ağırlıklı ortalamasıdır.",
        &[
            "Hücrenin değeri Σ wᵢ zᵢ / Σ wᵢ, wᵢ = 1 / dᵢ^üs; en yakın Nokta sayısı kadar nokta (eşit uzaklıkta önce sırası küçük olan), Arama yarıçapı verilmişse onun içindekiler. Bir nokta hücrenin merkezindeyse değer onunkidir. GDAL'ın invdistnn'iyle aynı tanım.",
            HELP_POINTS,
            HELP_GRID,
            HELP_CROSS,
            HELP_OUTPUT,
        ],
        &[
            "idw",
            "ters uzaklık",
            "ters mesafe",
            "ağırlıklı ortalama",
            "interpolasyon",
            "enterpolasyon",
            "yüzey",
            "inverse distance",
        ],
        &["IDW", "TERSUZAKLIK", "TERSMESAFE"],
        p,
        |r, cx, fb| {
            run_points(
                r,
                cx,
                fb,
                json!({"kind": "idw", "power": r.number("power"), "points": r.number("points").unwrap_or(12.0) as u32,
                       "radius": r.number("radius").unwrap_or(0.0), "minPoints": r.number("minPoints").unwrap_or(1.0) as u32}),
                ("-idw", "Ters uzaklıkla hesaplanıyor"),
                Input::Points,
            )
        },
    )
}

pub fn natural_neighbor() -> Tool {
    let mut p: Vec<ParamDef> = points().into();
    p.extend(ends("-dogalkomsu", "Doğal komşu", true));
    tool(
        (
            "interpolation.naturalNeighbor",
            "Doğal komşu",
            "naturalNeighbor",
        ),
        INTERPOLATION,
        "Noktalardan yüzey: her hücre doğal komşularının, Voronoi hücrelerinden aldığı alanlarla ağırlıklı ortalamasıdır (Sibson).",
        &[
            "Hücre noktalara eklenseydi Voronoi hücresinin her komşudan alacağı alan o komşunun ağırlığıdır; yüzey noktalardan geçer ve yumuşaktır. Noktaların dışbükey kabuğunun dışı boş kalır.",
            HELP_POINTS,
            HELP_GRID,
            HELP_CROSS,
            HELP_OUTPUT,
        ],
        &[
            "doğal komşu",
            "natural neighbor",
            "sibson",
            "voronoi",
            "interpolasyon",
            "yüzey",
        ],
        &["DOGALKOMSU", "SIBSON"],
        p,
        |r, cx, fb| {
            run_points(
                r,
                cx,
                fb,
                json!({"kind": "naturalNeighbor"}),
                ("-dogalkomsu", "Doğal komşuyla hesaplanıyor"),
                Input::Points,
            )
        },
    )
}

pub fn tin() -> Tool {
    let mut p: Vec<ParamDef> = points().into();
    p.extend(ends("-tin", "TIN", true));
    tool(
        ("interpolation.tin", "TIN'den raster", "tinRaster"),
        INTERPOLATION,
        "Noktaların Delaunay üçgenlemesinden yüzey: her hücre içinde kaldığı üçgende doğrusal.",
        &[
            "Noktalar Delaunay üçgenlerine bölünür; hücrenin değeri içinde kaldığı üçgenin üç köşesinden doğrusal hesaplanır. Kabuğun dışı boş kalır. GDAL'ın linear'ıyla aynı tanım; eş çemberli noktalarda (düzgün ızgara) üçgenleme tek değildir.",
            HELP_POINTS,
            HELP_GRID,
            HELP_CROSS,
            HELP_OUTPUT,
        ],
        &[
            "tin",
            "üçgenleme",
            "delaunay",
            "doğrusal",
            "yüzey",
            "dem",
            "sayısal arazi modeli",
        ],
        &["TINRASTER", "TINDENRASTER"],
        p,
        |r, cx, fb| {
            run_points(
                r,
                cx,
                fb,
                json!({"kind": "tin"}),
                ("-tin", "Üçgenlemeyle hesaplanıyor"),
                Input::Points,
            )
        },
    )
}

pub fn spline() -> Tool {
    let mut p: Vec<ParamDef> = points().into();
    p.extend([
        options("splineType", "Tür", &[("regularized", "Düzenlemeli"), ("tension", "Gerilimli")])
            .describe("Düzenlemeli: yumuşak, ağırlık büyüdükçe daha yumuşak; Gerilimli: sert, ağırlık büyüdükçe daha gergin."),
        number("weight", "Ağırlık", 0.1, 0.0, 100.0, "").describe("Düzenlemelide τ², 0 ile 5 arası (0: ince plaka); gerilimlide φ², 0'dan büyük."),
        whole("points", "Nokta sayısı", 12, 3, 64).describe("Her hücrede en yakın bu kadar noktadan geçen eğri."),
    ]);
    p.extend(ends("-spline", "Spline", true));
    tool(
        ("interpolation.spline", "Spline", "splineSurface"),
        INTERPOLATION,
        "Noktalardan geçen en az eğrilikli yüzey (ArcGIS'in Spline'ı): düzenlemeli ya da gerilimli.",
        &[
            "Her hücrede en yakın Nokta sayısı kadar noktadan geçen eğri (Mitas ve Mitasova 1988, ArcGIS'in tanımı); komşu kümesi değiştikçe yüzeyde küçük basamaklar olabilir. Doğru üzerindeki komşularla düzenlemeli spline çözülmez, hücre boş kalır.",
            HELP_POINTS,
            HELP_GRID,
            HELP_CROSS,
            HELP_OUTPUT,
        ],
        &[
            "spline",
            "eğri",
            "ince plaka",
            "thin plate",
            "düzenlemeli",
            "gerilimli",
            "interpolasyon",
            "yüzey",
        ],
        &["SPLINEYUZEY", "SPLINEINTERPOLASYON"],
        p,
        |r, cx, fb| {
            run_points(
                r,
                cx,
                fb,
                json!({"kind": "spline", "spline": r.text("splineType"), "weight": r.number("weight"),
                       "points": r.number("points").unwrap_or(12.0) as u32}),
                ("-spline", "Spline'la hesaplanıyor"),
                Input::Points,
            )
        },
    )
}

pub fn kriging() -> Tool {
    let manual = |v: &Values| text(v, "variogram") == "manual";
    let mut p: Vec<ParamDef> = points().into();
    p.extend([
        options("model", "Model", &[("spherical", "Küresel"), ("exponential", "Üstel"), ("gaussian", "Gauss")])
            .describe("Variogramın biçimi: erime kadar artıp eşikte duran (pratik erim)."),
        options("variogram", "Variogram", &[("auto", "Otomatik"), ("manual", "Elle")])
            .describe("Otomatik: noktaların ampirik variogramına uydurulur; Elle: külçe, kısmi eşik ve erim yazılır."),
        whole("lags", "Aralık sayısı", 12, 3, 100)
            .describe("Ampirik variogramın eşit aralıkları (en büyük uzaklık kutunun köşegeninin yarısı).")
            .shown_when(|v: &Values| text(v, "variogram") != "manual"),
        number("nugget", "Külçe", 0.0, 0.0, 1e12, "").describe("γ'nın 0'dan hemen sonraki sıçraması.").shown_when(manual),
        number("sill", "Kısmi eşik", 1.0, 0.0, 1e12, "").describe("Külçenin üstüne erimde eklenen.").shown_when(manual),
        number("range", "Erim", 100.0, 1e-6, 1e9, "m").describe("Variogramın eşiğe ulaştığı uzaklık.").shown_when(manual),
        whole("points", "Nokta sayısı", 12, 1, 64).describe("Her hücrede en yakın bu kadar nokta."),
        number("radius", "Arama yarıçapı", 0.0, 0.0, 1e7, "m").describe("0: sınırsız."),
        ParamDef::new("errorSurface", "Hata yüzeyi", crate::types::ParamKind::Boolean)
            .default_value(json!(false))
            .describe("Tahminin standart hatası ikinci bant olarak yazılır ve kendi katmanında çizilir."),
    ]);
    let mut tail = ends("-kriging", "Kriging", true);
    let mut error = result_layer("Kriging standart hatası", "errorLayer");
    error.label = "Hata katmanı".into();
    error.visible_when = Some(crate::types::ShownWhen::Rule(|v: &Values| {
        v.get("errorSurface").and_then(Value::as_bool) == Some(true)
            && v.get("add").and_then(Value::as_bool) != Some(false)
    }));
    tail.push(error);
    p.extend(tail);
    tool(
        ("interpolation.kriging", "Kriging", "kriging"),
        INTERPOLATION,
        "Noktalardan yüzey: sıradan kriging, variogram otomatik uydurulur ya da elle verilir; isteğe bağlı standart hata.",
        &[
            "Her hücrede en yakın Nokta sayısı kadar noktanın sıradan kriging tahmini; variogram küresel, üstel ya da Gauss. Otomatik variogram ampirik variograma ağırlıklı en küçük karelerle uydurulur, değerleri özette söylenir. Hata yüzeyi tahminin standart hatasını ikinci bant olarak yazar.",
            HELP_POINTS,
            HELP_GRID,
            HELP_CROSS,
            HELP_OUTPUT,
        ],
        &[
            "kriging",
            "variogram",
            "jeoistatistik",
            "ordinary kriging",
            "standart hata",
            "interpolasyon",
            "yüzey",
        ],
        &["KRIGING"],
        p,
        |r, cx, fb| {
            let variogram = if r.text("variogram") == "manual" {
                json!({"fit": "manual", "nugget": r.number("nugget"), "sill": r.number("sill"), "range": r.number("range")})
            } else {
                json!({"fit": "auto", "lags": r.number("lags").unwrap_or(12.0) as u32})
            };
            run_points(
                r,
                cx,
                fb,
                json!({"kind": "kriging", "model": r.text("model"), "variogram": variogram,
                       "points": r.number("points").unwrap_or(12.0) as u32, "radius": r.number("radius").unwrap_or(0.0),
                       "error": r.flag("errorSurface")}),
                ("-kriging", "Kriging'le hesaplanıyor"),
                Input::Points,
            )
        },
    )
}

pub fn kernel_density() -> Tool {
    let mut p = vec![
        input_param("Noktalar", &["point"], "Yoğunluğu alınan noktalar."),
        field_param(
            "weightField",
            "Ağırlık alanı",
            "Boş bırakılırsa her nokta 1 sayılır; seçilirse nesnenin bu alandaki sayısı (eksi değer alınmaz).",
        ),
        number("radius", "Yarıçap", 0.0, 0.0, 1e7, "m")
            .describe("0: kendiliğinden (Silverman'ın kuralı, ArcGIS'in varsayılanı)."),
        options(
            "kernel",
            "Çekirdek",
            &[
                ("quartic", "Dörtlü"),
                ("triangular", "Üçgen"),
                ("uniform", "Düzgün"),
                ("epanechnikov", "Epanechnikov"),
                ("triweight", "Üçlü ağırlık"),
            ],
        )
        .describe("Noktanın etkisinin uzaklıkla azalışı; integrali 1."),
        options(
            "unit",
            "Birim",
            &[
                ("squareKilometre", "km² başına"),
                ("hectare", "Hektar başına"),
                ("decare", "Dönüm başına"),
                ("squareMetre", "m² başına"),
            ],
        ),
    ];
    p.extend(ends("-yogunluk", "Yoğunluk", false));
    tool(
        ("density.kernel", "Çekirdek yoğunluğu", "kernelDensity"),
        DENSITY,
        "Noktaların yoğunluğu: her nokta yarıçap içindeki hücrelere çekirdeğiyle yayılır (ısı haritası).",
        &[
            "Hücrenin değeri Σ ağırlık · K(d / r) / r², d < r olan noktalar üstünden; çekirdeğin düzlemdeki integrali 1'dir, yoğunluğun toplamı ağırlıkların toplamına yaklaşır. Yarıçap 0 ise ArcGIS'in Silverman kuralı: 0,9 · min(standart uzaklık, √(1/ln 2) · ortanca uzaklık) · n^−0,2.",
            "Kapsam noktaların kutusu yarıçap kadar büyütülerek; değeri 0 olan hücreler çizimde boştur.",
            HELP_OUTPUT,
        ],
        &[
            "yoğunluk",
            "çekirdek",
            "kernel density",
            "ısı haritası",
            "heatmap",
            "sıcak nokta",
        ],
        &["YOGUNLUK", "CEKIRDEKYOGUNLUGU", "ISIHARITASI", "HEATMAP"],
        p,
        |r, cx, fb| {
            run_points(
                r,
                cx,
                fb,
                json!({"kind": "kernel", "radius": r.number("radius").unwrap_or(0.0), "kernel": r.text("kernel"), "unit": r.text("unit")}),
                ("-yogunluk", "Yoğunluk hesaplanıyor"),
                Input::Weighted,
            )
        },
    )
}

pub fn line_density() -> Tool {
    let mut p = vec![
        input_param(
            "Çizgiler",
            &LINE_KINDS,
            "Yoğunluğu alınan çizgiler; alanların sınırları da.",
        ),
        field_param(
            "weightField",
            "Ağırlık alanı",
            "Boş bırakılırsa her çizgi 1 sayılır; seçilirse nesnenin bu alandaki sayısı.",
        ),
        number("radius", "Yarıçap", 0.0, 0.0, 1e7, "m")
            .describe("0: kendiliğinden (çizgilerin kutusunun kısa kenarının 30'da biri)."),
        options(
            "unit",
            "Birim",
            &[
                ("kilometrePerSquareKilometre", "km/km²"),
                ("metrePerHectare", "m/ha"),
                ("metrePerSquareMetre", "m/m²"),
            ],
        ),
    ];
    p.extend(ends("-cizgiyogunlugu", "Çizgi yoğunluğu", false));
    tool(
        ("density.line", "Çizgi yoğunluğu", "lineDensity"),
        DENSITY,
        "Çizgilerin yoğunluğu: her hücre yarıçap içindeki çizgi uzunluğunun dairenin alanına oranıdır.",
        &[
            "Hücrenin değeri Σ ağırlık · (çizginin dairenin içindeki uzunluğu) / (π r²). Doğru parçaları ve yaylar daireyle kesin kesilir; elips ve eğri 0,1 mm içinde doğru parçalarıyla girer.",
            "Kapsam çizgilerin kutusu yarıçap kadar büyütülerek; değeri 0 olan hücreler çizimde boştur.",
            HELP_OUTPUT,
        ],
        &[
            "çizgi yoğunluğu",
            "line density",
            "yol yoğunluğu",
            "dere yoğunluğu",
            "yoğunluk",
        ],
        &["CIZGIYOGUNLUGU"],
        p,
        |r, cx, fb| {
            run_points(
                r,
                cx,
                fb,
                json!({"kind": "lineDensity", "radius": r.number("radius").unwrap_or(0.0), "unit": r.text("unit")}),
                ("-cizgiyogunlugu", "Çizgi yoğunluğu hesaplanıyor"),
                Input::Lines,
            )
        },
    )
}
