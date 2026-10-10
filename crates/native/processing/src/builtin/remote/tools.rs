//! The eight tools of Uzaktan algılama (docs/adr/0242 §3–§10): their
//! parameters as the web's (`builtin/remote/tools.ts`), and their settings
//! handed to the raster core.

use serde_json::{Value, json};

use super::{REMOTE, objects_with, run_accuracy, run_raster, run_split};
use crate::builtin::pointcloud::{add_param, output_param, result_layer_param};
use crate::builtin::queries::AREA_KINDS;
use crate::builtin::surface::{choice, number, whole};
use crate::types::{OutputDef, OutputKind, ParamDef, ParamKind, ScopeKind, Target, Tool, Values};

/// The results' layers' colour (a raster's is not drawn).
const RASTER_LAYER: &str = "#7A6B5B";

const HELP_EMPTY: &str = "Rasterin nodata'sı, NaN ve alfası 0 olan pikseller değersizdir.";
const HELP_OUTPUT: &str = "Sonuç karolu, Deflate'li ve önizleme katlı GeoTIFF'tir; çıktı dosyası boşsa ilk rasterin yanına adının sonuna ek konarak yazılır. Çizime ekle açıksa raster ilk rasterin katmanının hemen üstündeki yeni katmana eklenir.";

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
        category: REMOTE.into(),
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

fn file() -> OutputDef {
    OutputDef::new("file", "Sonuç dosyası", OutputKind::Text)
}

fn table() -> OutputDef {
    OutputDef::new("table", "Tablo", OutputKind::Table)
}

/// One raster, the parameter `name`.
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

/// A band number (from 1).
fn band(name: &str, label: &str, default: u32, description: &str) -> ParamDef {
    whole(name, label, default, 1, 255).describe(description)
}

/// Çıktı dosyası, Çizime ekle, Çıktı katmanı (right above the first raster's layer).
fn ends(suffix: &str, layer: &str) -> [ParamDef; 3] {
    let mut l = result_layer_param(layer, RASTER_LAYER);
    if let ParamKind::Layer { above, .. } = &mut l.kind {
        *above = Some("input".to_owned());
    }
    [output_param(suffix, &[".tif"]), add_param(), l]
}

/// Objects of the kinds `kinds` the user picks.
fn objects(name: &str, label: &str, kinds: Vec<String>, description: &str) -> ParamDef {
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

fn field(name: &str, label: &str, of: &str, description: &str) -> ParamDef {
    ParamDef::new(
        name,
        label,
        ParamKind::Field {
            of: vec![of.to_owned()],
            allow_new: false,
            multiple: false,
        },
    )
    .describe(description)
}

/// A choice whose options carry hints.
fn hinted(name: &str, label: &str, options: &[(&str, &str, &str)]) -> ParamDef {
    let plain: Vec<(&str, &str)> = options.iter().map(|(v, l, _)| (*v, *l)).collect();
    let mut p = choice(name, label, &plain);
    if let ParamKind::Choice { options: o } = &mut p.kind {
        for (opt, (_, _, hint)) in o.iter_mut().zip(options) {
            opt.hint = Some((*hint).to_owned());
        }
    }
    p
}

pub fn composite() -> Tool {
    let mut p = vec![
        ParamDef::new(
            "input",
            "Rasterler",
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
        .describe("Birleştirilecek rasterler (en az iki)."),
        choice(
            "sampling",
            "Örnekleme",
            &[("nearest", "En yakın"), ("bilinear", "Çift doğrusal")],
        )
        .describe("Rasterler sonucun ızgarasına böyle okunur; en yakın değerleri değiştirmez."),
    ];
    p.extend(ends("-birlesik", "Bant birleştir"));
    tool(
        ("remote.composite", "Bant birleştir", "bandComposite"),
        "Rasterlerin bantlarını tek çok bantlı rasterde birleştirir: her rasterin bütün bantları sırayla sonucun bantları olur.",
        &[
            "Rasterler Katmanlar panelinde üstten aşağı sırayla (aynı katmanda sonra eklenen önce) birleşir; tabloda her bandın kaynağı yazılır.",
            "Rasterler ortak alanlarında, hücresi en küçük olanın ızgarasında birleşir: sonucun hücreleri merkezi bütün rasterlerin içinde olanlardır. Örnekleme en yakın hücre ya da çift doğrusal (dört komşu hücre merkezi).",
            "Rasterlerin hepsi aynı türdeyse sonuç o türde (ve aynı nodata ile), değilse ondalık 32 bit. Üç ve daha çok bantta görünüş renkli (1, 2, 3).",
            HELP_EMPTY,
            HELP_OUTPUT,
        ],
        &[
            "bant",
            "birleştir",
            "composite",
            "kompozit",
            "çok bantlı",
            "multiband",
            "stack",
        ],
        &["BANTBIRLESTIR", "COMPOSITEBANDS"],
        p,
        vec![table(), file()],
        |r, cx, fb| {
            let tool = json!({ "kind": "composite", "sampling": r.text("sampling") });
            run_raster(
                r,
                cx,
                fb,
                tool,
                (false, None),
                Vec::new(),
                ("-birlesik", "Bantlar birleştiriliyor"),
            )
        },
    )
}

pub fn split() -> Tool {
    let [output, add, layer] = ends("-b", "Bantlar");
    let p = vec![
        raster("input", "Raster", "Bantlarına ayrılacak raster."),
        output,
        add,
        layer,
    ];
    tool(
        ("remote.split", "Bantlara ayır", "bandSplit"),
        "Çok bantlı rasterin her bandını ayrı bir rastere yazar.",
        &[
            "Her bant kendi GeoTIFF'ine yazılır: çıktı dosyası boşsa rasterin yanına adının sonuna “-b1”, “-b2” … eklenerek; bir ad yazılırsa o adın sonuna. Örnek türü ve nodata rasterinkidir; alfa bandı ayrılmaz.",
            "Çizime ekle açıksa bantlar sırayla rasterin katmanının hemen üstündeki yeni katmana eklenir; görünüşleri gri, %2–98 gerdirmeyle.",
        ],
        &["bant", "ayır", "split", "band", "tek bant"],
        &["BANTAYIR", "SPLITBANDS"],
        p,
        vec![table()],
        run_split,
    )
}

/// The indices whose formula reads `band`.
fn reads(v: &Values, uses: &[&str]) -> bool {
    uses.contains(&text(v, "index", "ndvi"))
}

pub fn index() -> Tool {
    let evi = |v: &Values| text(v, "index", "ndvi") == "evi";
    let mut p = vec![
        raster(
            "input",
            "Raster",
            "Çok bantlı görüntü (Landsat, Sentinel-2, ortofoto).",
        ),
        hinted(
            "index",
            "İndis",
            &[
                ("ndvi", "NDVI", "bitki örtüsü"),
                ("gndvi", "GNDVI", "yeşil bantla bitki"),
                ("savi", "SAVI", "toprak etkisi azaltılmış bitki"),
                ("evi", "EVI", "yoğun bitki örtüsü; yansıma ister"),
                ("ndwi", "NDWI", "açık su"),
                ("mndwi", "MNDWI", "su; yapı gölgesini ayırır"),
                ("ndbi", "NDBI", "yapılaşma"),
                ("ratio", "Oran", "A / B"),
                ("normalized", "Normalize fark", "(A − B) / (A + B)"),
            ],
        )
        .describe("Hesaplanacak indis."),
        band("blue", "Mavi bant", 1, "Mavi bandın numarası (1'den).").shown_when(|v| reads(v, &["evi"])),
        band("green", "Yeşil bant", 2, "Yeşil bandın numarası (1'den).")
            .shown_when(|v| reads(v, &["gndvi", "ndwi", "mndwi"])),
        band("red", "Kırmızı bant", 3, "Kırmızı bandın numarası (1'den).")
            .shown_when(|v| reads(v, &["ndvi", "savi", "evi"])),
        band("nir", "Yakın kızılötesi bandı", 4, "Yakın kızılötesi bandın numarası (1'den).")
            .shown_when(|v| reads(v, &["ndvi", "gndvi", "savi", "evi", "ndwi", "ndbi"])),
        band("swir", "Kısa dalga kızılötesi bandı", 5, "Kısa dalga kızılötesi bandın numarası (1'den).")
            .shown_when(|v| reads(v, &["mndwi", "ndbi"])),
        band("a", "A bandı", 4, "Oranın payı, normalize farkın ilk bandı (1'den).")
            .shown_when(|v| reads(v, &["ratio", "normalized"])),
        band("b", "B bandı", 3, "Oranın paydası, normalize farkın ikinci bandı (1'den).")
            .shown_when(|v| reads(v, &["ratio", "normalized"])),
        number("saviL", "L", 0.5, 0.0, 10.0, "")
            .describe("SAVI'nin toprak düzeltmesi: yoğun bitkide 0, seyrekte 1.")
            .shown_when(|v| reads(v, &["savi"])),
        number("eviG", "G", 2.5, 0.0, 100.0, "")
            .describe("EVI'nin kazancı.")
            .shown_when(evi)
            .advanced(),
        number("eviC1", "C₁", 6.0, 0.0, 100.0, "")
            .describe("EVI'de kırmızının aerosol katsayısı.")
            .shown_when(evi)
            .advanced(),
        number("eviC2", "C₂", 7.5, 0.0, 100.0, "")
            .describe("EVI'de mavinin aerosol katsayısı.")
            .shown_when(evi)
            .advanced(),
        number("eviL", "EVI'nin L'si", 1.0, 0.0, 100.0, "")
            .describe("EVI'nin örtü düzeltmesi.")
            .shown_when(evi)
            .advanced(),
        number("scale", "Ölçek", 1.0, -1e9, 1e9, "")
            .describe("Yansıma = sayı × ölçek + öteleme (Sentinel-2 L2A: 0,0001; Landsat C2 L2: 0,0000275).")
            .advanced(),
        number("offset", "Öteleme", 0.0, -1e9, 1e9, "")
            .describe("Yansıma = sayı × ölçek + öteleme (Landsat C2 L2: −0,2).")
            .advanced(),
    ];
    p.extend(ends("-indis", "Spektral indis"));
    tool(
        ("remote.index", "Spektral indis", "spectralIndex"),
        "Bantlardan bitki, su ve yapılaşma indisi hesaplar: NDVI, GNDVI, SAVI, EVI, NDWI, MNDWI, NDBI, oran ya da normalize fark.",
        &[
            "NDVI = (YKÖ − K) / (YKÖ + K); GNDVI = (YKÖ − Y) / (YKÖ + Y); SAVI = (1 + L)(YKÖ − K) / (YKÖ + K + L); EVI = G(YKÖ − K) / (YKÖ + C₁K − C₂M + L); NDWI = (Y − YKÖ) / (Y + YKÖ); MNDWI = (Y − KDK) / (Y + KDK); NDBI = (KDK − YKÖ) / (KDK + YKÖ); oran A / B; normalize fark (A − B) / (A + B). M mavi, Y yeşil, K kırmızı, YKÖ yakın kızılötesi, KDK kısa dalga kızılötesi.",
            "Her bant önce yansımaya çevrilir: ρ = sayı × ölçek + öteleme (Sentinel-2 L2A'da ölçek 0,0001; Landsat Collection 2 L2'de 0,0000275 ve −0,2). EVI yansıma ister.",
            "Hesap 64 bitte, sonuç 32 bit ondalıktır; bölenin sıfır olduğu hücre değersiz kalır, söylenir.",
            HELP_EMPTY,
            HELP_OUTPUT,
        ],
        &[
            "ndvi",
            "indis",
            "index",
            "bitki",
            "vegetation",
            "ndwi",
            "su",
            "ndbi",
            "yapılaşma",
            "savi",
            "evi",
            "bant aritmetiği",
        ],
        &["INDIS", "NDVI"],
        p,
        vec![file()],
        |r, cx, fb| {
            let b = |name: &str, or: f64| r.number(name).unwrap_or(or) as u32;
            let tool = json!({
                "kind": "index",
                "index": r.text("index"),
                "bands": {
                    "blue": b("blue", 1.0), "green": b("green", 2.0), "red": b("red", 3.0), "nir": b("nir", 4.0),
                    "swir": b("swir", 5.0), "a": b("a", 4.0), "b": b("b", 3.0),
                },
                "scale": r.number("scale").unwrap_or(1.0),
                "offset": r.number("offset").unwrap_or(0.0),
                "saviL": r.number("saviL").unwrap_or(0.5),
                "g": r.number("eviG").unwrap_or(2.5),
                "c1": r.number("eviC1").unwrap_or(6.0),
                "c2": r.number("eviC2").unwrap_or(7.5),
                "eviL": r.number("eviL").unwrap_or(1.0),
            });
            run_raster(
                r,
                cx,
                fb,
                tool,
                (true, None),
                Vec::new(),
                ("-indis", "İndis hesaplanıyor"),
            )
        },
    )
}

pub fn supervised() -> Tool {
    let mut p = vec![
        raster("input", "Raster", "Sınıflandırılacak çok bantlı görüntü."),
        objects(
            "training",
            "Eğitim alanları",
            AREA_KINDS.iter().map(|k| (*k).to_owned()).collect(),
            "Sınıfları örnekleyen alanlar.",
        ),
        field(
            "classField",
            "Sınıf alanı",
            "training",
            "Eğitim alanının sınıfının adı (metin).",
        ),
        hinted(
            "method",
            "Yöntem",
            &[
                (
                    "likelihood",
                    "En büyük olabilirlik",
                    "sınıfın ortalaması ve yayılımıyla (kovaryans)",
                ),
                (
                    "distance",
                    "En yakın ortalama",
                    "sınıfın ortalamasına en yakın",
                ),
            ],
        )
        .describe("Hücrenin hangi sınıfa verileceği."),
    ];
    p.extend(ends("-siniflar", "Denetimli sınıflandırma"));
    tool(
        (
            "remote.supervised",
            "Denetimli sınıflandırma",
            "classifySupervised",
        ),
        "Eğitim alanlarından öğrenip görüntünün her hücresini bir sınıfa atar: en büyük olabilirlik ya da en yakın ortalama.",
        &[
            "Sınıflar eğitim alanlarının Sınıf alanındaki metinlerdir (kırpılır; boş olanlar alınmaz), doğal sırayla 1, 2, … değerlerini alır. Bir sınıfın eğitim hücreleri merkezi o sınıfın alanlarından birinin içinde olan ve hiçbir bandı değersiz olmayan hücrelerdir.",
            "En büyük olabilirlik her sınıfı ortalaması ve kovaryansıyla çok değişkenli normal dağılım sayar (eşit önsel olasılık): bir sınıfın en az bant sayısı + 1 eğitim hücresi olmalı, kovaryansı tekil olmamalı. En yakın ortalama hücreyi ortalamasına en yakın sınıfa verir.",
            "Sonuç 8 bit tam sayı (255 sınıftan çoğunda 16 bit), 0 değersiz; tabloda her sınıfın eğitim hücresi, hücre sayısı ve alanı.",
            HELP_EMPTY,
            HELP_OUTPUT,
        ],
        &[
            "sınıflandırma",
            "classification",
            "denetimli",
            "supervised",
            "maximum likelihood",
            "en büyük olabilirlik",
            "arazi örtüsü",
            "eğitim",
        ],
        &["DENETIMLI", "MAXLIKELIHOOD"],
        p,
        vec![table(), file()],
        |r, cx, fb| {
            let (shapes, texts) = objects_with(r, "training", r.text("classField"));
            let tool = json!({ "kind": "supervised", "method": r.text("method"), "texts": texts });
            run_raster(
                r,
                cx,
                fb,
                tool,
                (true, None),
                shapes,
                ("-siniflar", "Sınıflandırılıyor"),
            )
        },
    )
}

pub fn unsupervised() -> Tool {
    let mut p = vec![
        raster("input", "Raster", "Kümelere ayrılacak çok bantlı görüntü."),
        whole("clusters", "Küme sayısı", 8, 2, 50).describe("Bulunacak küme sayısı."),
        whole("iterations", "En çok yineleme", 20, 1, 100)
            .describe("Atamalar değişmeyince ya da bu kadar yinelemeden sonra durur."),
    ];
    p.extend(ends("-kumeler", "Denetimsiz sınıflandırma"));
    tool(
        (
            "remote.unsupervised",
            "Denetimsiz sınıflandırma",
            "classifyUnsupervised",
        ),
        "Görüntünün hücrelerini benzer spektral değerlerine göre kümelere ayırır (k-ortalamalar).",
        &[
            "Kümeler hücrelerin bir örneğinden bulunur: hiçbir bandı değersiz olmayan, sütunu ve satırı s'nin katı olan hücreler (s, yaklaşık 250 000 hücre kalacak biçimde). Başlangıç merkezleri bantların ortalaması ± standart sapması boyunca dizilir; her yinelemede hücreler en yakın merkeze, merkezler üyelerinin ortalamasına gider; atamalar değişmeyince ya da en çok yinelemede durur.",
            "Kümeler merkezlerinin bant toplamına göre küçükten büyüğe 1 … k değerlerini alır; bütün hücreler en yakın merkeze atanır. Sonuç 8 bit, 0 değersiz; tabloda her kümenin hücre sayısı, alanı ve merkezi.",
            HELP_EMPTY,
            HELP_OUTPUT,
        ],
        &[
            "kümeleme",
            "clustering",
            "denetimsiz",
            "unsupervised",
            "k-means",
            "isodata",
            "iso cluster",
            "sınıflandırma",
        ],
        &["DENETIMSIZ", "ISOCLUSTER"],
        p,
        vec![table(), file()],
        |r, cx, fb| {
            let tool = json!({
                "kind": "unsupervised",
                "clusters": r.number("clusters").unwrap_or(8.0) as u32,
                "iterations": r.number("iterations").unwrap_or(20.0) as u32,
            });
            run_raster(
                r,
                cx,
                fb,
                tool,
                (true, None),
                Vec::new(),
                ("-kumeler", "Kümeleniyor"),
            )
        },
    )
}

pub fn accuracy() -> Tool {
    let mut kinds = vec!["point".to_owned()];
    kinds.extend(AREA_KINDS.iter().map(|k| (*k).to_owned()));
    let p = vec![
        raster("input", "Raster", "Sınıflandırılmış raster."),
        band("band", "Bant", 1, "Sınıfların okunduğu bant (1'den)."),
        objects(
            "reference",
            "Referans nesneleri",
            kinds,
            "Sınıfı bilinen noktalar ya da alanlar.",
        ),
        field(
            "referenceField",
            "Referans alanı",
            "reference",
            "Nesnenin gerçek sınıfının değeri (tam sayı).",
        ),
    ];
    tool(
        ("remote.accuracy", "Doğruluk analizi", "accuracyMatrix"),
        "Sınıflandırılmış rasteri referans nesneleriyle karşılaştırır: karışıklık matrisi, genel doğruluk, üretici ve kullanıcı doğruluğu, kappa.",
        &[
            "Nokta düştüğü hücreyi, alan merkezini içine aldığı hücreleri verir; her nesnenin hücreleri ayrı sayılır. Referans alanının değeri tam sayı olarak okunur (sınıfın değeri); okunamayan nesne ve değersiz ya da rasterin dışındaki hücre alınmaz, söylenir.",
            "Matrisin satırları sınıflandırılan, sütunları referans değerlerdir. Kullanıcı doğruluğu satırın, üretici doğruluğu sütunun köşegendeki payıdır; genel doğruluk köşegenin toplamdaki payı; kappa (Cohen) şansla uyuşmanın ötesindeki uyum: (pₒ − pₑ) / (1 − pₑ).",
        ],
        &[
            "doğruluk",
            "accuracy",
            "karışıklık matrisi",
            "confusion matrix",
            "kappa",
            "hata matrisi",
            "doğrulama",
        ],
        &["DOGRULUK", "CONFUSIONMATRIX"],
        p,
        vec![
            table(),
            OutputDef::new("overall", "Genel doğruluk (%)", OutputKind::Number),
            OutputDef::new("kappa", "Kappa", OutputKind::Number),
        ],
        run_accuracy,
    )
}

pub fn change() -> Tool {
    let mut p = vec![
        raster("input", "Önceki raster", "Önceki tarihin rasteri."),
        raster("after", "Sonraki raster", "Sonraki tarihin rasteri."),
        band(
            "band",
            "Bant",
            1,
            "Karşılaştırılan bant (1'den); iki rasterde de aynı bant.",
        ),
        hinted(
            "method",
            "Yöntem",
            &[
                ("difference", "Fark", "sonraki − önceki"),
                ("ratio", "Oran", "sonraki / önceki"),
                (
                    "normalized",
                    "Normalize fark",
                    "(sonraki − önceki) / (sonraki + önceki)",
                ),
                (
                    "classes",
                    "Sınıf değişimi",
                    "sınıf rasterleri: önceki × 1000 + sonraki",
                ),
            ],
        )
        .describe("İki tarihin nasıl karşılaştırılacağı."),
    ];
    p.extend(ends("-degisim", "Değişim"));
    tool(
        ("remote.change", "Değişim tespiti", "changeDetect"),
        "İki tarihin rasterlerini karşılaştırır: fark, oran, normalize fark ya da sınıf değişimi.",
        &[
            "Fark sonraki − önceki, oran sonraki / önceki, normalize fark (sonraki − önceki) / (sonraki + önceki); sonuç 32 bit ondalık, bölenin sıfır olduğu hücre değersiz. Özette artan, azalan ve değişmeyen hücreler sayılır.",
            "Sınıf değişimi sınıf rasterleri içindir: değer önceki × 1000 + sonraki (örneğin 2003: 2'den 3'e), 0 değerli ya da değersiz hücre değersiz; tablo “neden neye” matrisidir, özet değişen hücrelerin payı.",
            "Rasterler ortak alanlarında, hücresi en küçük olanın ızgarasında, en yakın hücreleriyle okunur.",
            HELP_EMPTY,
            HELP_OUTPUT,
        ],
        &[
            "değişim",
            "change detection",
            "fark",
            "difference",
            "zaman",
            "karşılaştırma",
            "sınıf değişimi",
        ],
        &["DEGISIM", "CHANGEDETECTION"],
        p,
        vec![table(), file()],
        |r, cx, fb| {
            let tool = json!({
                "kind": "change",
                "band": r.number("band").unwrap_or(1.0) as u32,
                "method": r.text("method"),
            });
            run_raster(
                r,
                cx,
                fb,
                tool,
                (true, Some(("after", "Sonraki raster"))),
                Vec::new(),
                ("-degisim", "Değişim hesaplanıyor"),
            )
        },
    )
}

pub fn pansharpen() -> Tool {
    let brovey = |v: &Values| text(v, "method", "brovey") == "brovey";
    let mut p = vec![
        raster(
            "input",
            "Çok bantlı raster",
            "Renkli ya da çok bantlı görüntü (kaba çözünürlük).",
        ),
        raster(
            "pan",
            "Pankromatik raster",
            "Tek bantlı, ince çözünürlüklü görüntü.",
        ),
        hinted(
            "method",
            "Yöntem",
            &[
                (
                    "brovey",
                    "Brovey",
                    "ağırlıklı; renkleri pankromatiğin parlaklığıyla ölçekler",
                ),
                (
                    "mean",
                    "Basit ortalama",
                    "bant ile pankromatiğin ortalaması",
                ),
            ],
        )
        .describe("Bantların pankromatikle nasıl birleşeceği."),
        ParamDef::new(
            "weights",
            "Ağırlıklar",
            ParamKind::Text {
                placeholder: Some("eşit".to_owned()),
                max_length: None,
                allow_empty: true,
            },
        )
        .optional()
        .default_value(json!(""))
        .describe(
            "Bantların ağırlıkları, noktalı virgülle ayrılmış (0,1; 0,3; 0,3; 0,3); boşsa eşit.",
        )
        .shown_when(brovey),
        choice(
            "sampling",
            "Örnekleme",
            &[
                ("cubic", "Kübik"),
                ("bilinear", "Çift doğrusal"),
                ("nearest", "En yakın"),
            ],
        )
        .describe("Çok bantlı görüntü pankromatiğin ızgarasına böyle okunur."),
    ];
    p.extend(ends("-birlesim", "Görüntü birleştirme"));
    tool(
        ("remote.pansharpen", "Görüntü birleştirme", "pansharpen"),
        "Çok bantlı görüntüyü pankromatik görüntünün ince çözünürlüğüne taşır (pansharpening): ağırlıklı Brovey ya da basit ortalama.",
        &[
            "Çok bantlı görüntü pankromatiğin ızgarasına seçilen örneklemeyle okunur. Brovey: her bant × PAN / Σ wⱼ·bantⱼ (toplam sıfır ya da eksiyse 0); ağırlıklar boşsa eşittir (1/n). Basit ortalama: (bant + PAN) / 2.",
            "Sonuç pankromatiğin ızgarasında, çok bantlının örnek türündedir (tam sayıda en yakın tam sayıya yuvarlanır, türün aralığında tutulur); görünüşü çok bantlınınki.",
            "Ağırlıkları noktalı virgül ya da boşlukla ayırarak yazın (0,1; 0,3; 0,3; 0,3); sayısı çok bantlının bant sayısı olmalı.",
            HELP_EMPTY,
            HELP_OUTPUT,
        ],
        &[
            "pansharpening",
            "pankromatik",
            "görüntü birleştirme",
            "fusion",
            "brovey",
            "çözünürlük",
            "keskinleştirme",
        ],
        &["PANSHARPEN", "FUSION"],
        p,
        vec![file()],
        |r, cx, fb| {
            let tool = json!({
                "kind": "pansharpen",
                "method": r.text("method"),
                "weights": r.text("weights"),
                "sampling": r.text("sampling"),
            });
            run_raster(
                r,
                cx,
                fb,
                tool,
                (true, Some(("pan", "Pankromatik raster"))),
                Vec::new(),
                ("-birlesim", "Görüntüler birleştiriliyor"),
            )
        },
    )
}
