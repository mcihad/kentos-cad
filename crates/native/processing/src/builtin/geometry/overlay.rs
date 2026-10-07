//! Kesişim, Fark, Simetrik fark and Birleşim (the web's
//! `builtin/geometry/overlay.ts`; docs/adr/0201 §5): two sets of objects
//! overlaid in the core's overlay, each piece written with the attributes of
//! the objects it is of: the first side's as they are (Alan oranıyla
//! paylaştır's fields times the piece's share), the second side's with Önek
//! before their names, “ad (2)” where the first side has the name already.

use std::collections::BTreeMap;

use kentos_contracts::Entity;
use kentos_geometry_core::ops::geoprocess::calls::{Mode, OverlayPiece, overlay_run};
use kentos_geometry_core::ops::statistics::apportion;
use serde_json::json;

use super::dissolve::field_names;
use super::{
    AREA_KINDS, GEO_KINDS, empty_note, features_param, geo_scopes, input_notes, layer_param,
    new_object, shapes,
};
use crate::types::{
    ChangeSet, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, RunContext,
    RunResult, Target, Tool,
};

/// The second side's attributes added to the first's: each name with the
/// prefix, “ad (2)”, “ad (3)” … where taken.
pub fn with_second(
    mut out: BTreeMap<String, String>,
    second: &BTreeMap<String, String>,
    prefix: &str,
) -> BTreeMap<String, String> {
    for (k, v) in second {
        let mut name = format!("{prefix}{k}");
        if out.contains_key(&name) {
            let mut n = 2;
            while out.contains_key(&format!("{name} ({n})")) {
                n += 1;
            }
            name = format!("{name} ({n})");
        }
        out.insert(name, v.clone());
    }
    out
}

/// A piece's attributes; `skipped` counts the values Alan oranıyla paylaştır could not read.
fn piece_attrs(
    p: &OverlayPiece,
    a: &[&Entity],
    b: &[&Entity],
    prefix: &str,
    fields: &[&str],
    skipped: &mut usize,
) -> BTreeMap<String, String> {
    let mut attrs = BTreeMap::new();
    if let Some(i) = p.a {
        let own = &a[i].base().attrs;
        attrs = own.clone();
        if let Some(share) = p.share {
            for f in fields {
                if let Some(value) = own.get(*f) {
                    match apportion(value, share) {
                        Some(s) => {
                            attrs.insert((*f).to_owned(), s);
                        }
                        None => *skipped += 1,
                    }
                }
            }
        }
    }
    if let Some(j) = p.b {
        attrs = with_second(attrs, &b[j].base().attrs, prefix);
    }
    attrs
}

/// How many pieces were of both sides, of the first only and of the second only.
struct Counts {
    both: usize,
    first: usize,
    second: usize,
}

/// An overlay's run: its pieces on the output layer, the notes, and the counts.
fn overlay(
    mode: Mode,
    v: &Resolved<'_>,
    prefix: &str,
    fields: &[&str],
    feedback: &mut dyn Feedback,
) -> (Vec<Entity>, Counts) {
    let a = &v.features("input").entities;
    let b = &v.features("overlay").entities;
    input_notes(feedback, a, b, false);
    feedback.progress(0.0, "Alanlar bindiriliyor");
    let pieces = overlay_run(&shapes(a), &shapes(b), mode);
    let mut skipped = 0;
    let layer = &v.layer("layer").id;
    let attrs: Vec<BTreeMap<String, String>> = pieces
        .iter()
        .map(|p| piece_attrs(p, a, b, prefix, fields, &mut skipped))
        .collect();
    if skipped > 0 {
        feedback.warn(format!(
            "{skipped} değer sayı olarak okunamadığı için paylaştırılmadı."
        ));
    }
    // Objects that gave no piece: the first side's, and in Simetrik fark and Birleşim the second's.
    let second = matches!(mode, Mode::SymDifference | Mode::Union);
    let none = (0..a.len())
        .filter(|i| !pieces.iter().any(|p| p.a == Some(*i)))
        .count()
        + if second {
            (0..b.len())
                .filter(|j| !pieces.iter().any(|p| p.b == Some(*j)))
                .count()
        } else {
            0
        };
    empty_note(none, feedback);
    let counts = Counts {
        both: pieces
            .iter()
            .filter(|p| p.a.is_some() && p.b.is_some())
            .count(),
        first: pieces
            .iter()
            .filter(|p| p.a.is_some() && p.b.is_none())
            .count(),
        second: pieces.iter().filter(|p| p.a.is_none()).count(),
    };
    let add = pieces
        .into_iter()
        .zip(attrs)
        .filter_map(|(p, attrs)| new_object(p.shape, layer, attrs))
        .collect();
    (add, counts)
}

fn result(add: Vec<Entity>, summary: String) -> RunResult {
    let count = add.len();
    RunResult {
        changes: Some(ChangeSet {
            add,
            ..ChangeSet::default()
        }),
        outputs: [("count".to_owned(), json!(count))].into_iter().collect(),
        summary: Some(summary),
        ..RunResult::default()
    }
}

fn prefix_param() -> ParamDef {
    ParamDef::new(
        "prefix",
        "Önek",
        ParamKind::Text {
            placeholder: None,
            max_length: Some(24),
            allow_empty: true,
        },
    )
    .default_value(json!(""))
    .describe("İkinci tarafın öznitelik adlarının önüne yazılır; ad birinci tarafta da varsa “ad (2)” olur.")
}

fn apportion_param() -> ParamDef {
    ParamDef::new(
        "apportion",
        "Alan oranıyla paylaştır",
        ParamKind::Field {
            of: vec!["input".into()],
            allow_new: false,
            multiple: true,
        },
    )
    .optional()
    .advanced()
    .describe("Bu alanların sayı değerleri parçanın payıyla çarpılır: alanda alan, çizgide uzunluk oranı.")
}

const APPORTION_HELP: &str = "Alan oranıyla paylaştır: seçilen alanların sayı değerleri, parçanın birinci taraftaki nesnenin alanına (çizgide uzunluğuna, noktada sayısına) oranıyla çarpılır ve değerin basamağından iki fazla basamağa yarım çifte yuvarlanır; sayı olarak okunamayan değer olduğu gibi kalır ve söylenir.";

fn outputs(pieces: &str, count: &str) -> Vec<OutputDef> {
    vec![
        OutputDef::new("pieces", pieces, OutputKind::Features),
        OutputDef::new("count", count, OutputKind::Number),
    ]
}

pub fn intersection() -> Tool {
    Tool {
        id: "geometry.intersection".into(),
        label: "Kesişim".into(),
        category: "geometry".into(),
        description: "Her nesnenin kesen alanlarla ortak parçalarını, iki tarafın öznitelikleriyle yeni katmana yazar.".into(),
        help: Some([
            "Her nesne, kesiştiği her alanla ayrı bir parça verir: alanda ortak alan, çizgide içerideki parçalar, noktada içerideki noktalar. Parça nesnenin ve kesen alanın özniteliklerini taşır; kesen alanınkiler Önek ile yazılır.",
            APPORTION_HELP,
            "Örnek: imar adalarıyla kesişen parsellerin her adadaki parçası, parselin ve adanın bilgileriyle; parselin değeri parçanın alan oranıyla.",
        ]
        .join("\n\n")),
        keywords: ["kesişim", "kesiştir", "intersect", "bindirme", "overlay", "ortak alan", "paylaştır"]
            .map(String::from)
            .to_vec(),
        aliases: ["KESISIMAL", "GEOINTERSECT"].map(String::from).to_vec(),
        icon: Some("geoIntersection".into()),
        parameters: vec![
            features_param("input", "Nesneler", &GEO_KINDS, geo_scopes(), "Kesilecek alanlar, çizgiler ve noktalar."),
            features_param("overlay", "Kesen alanlar", &AREA_KINDS, geo_scopes(), "Her nesne kesiştiği her alanla ayrı parça verir."),
            prefix_param(),
            apportion_param(),
            layer_param("Kesişim", "#F76B15"),
        ],
        outputs: outputs("Parçalar", "Parça sayısı"),
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(|v: &Resolved<'_>, _ctx: &RunContext<'_>, feedback: &mut dyn Feedback| {
            let fields = field_names(v.text("apportion"));
            let (add, _) = overlay(Mode::Intersection, v, v.text("prefix"), &fields, feedback);
            let summary = format!("{} kesişim parçası yazıldı.", add.len());
            result(add, summary)
        }),
    }
}

pub fn difference() -> Tool {
    Tool {
        id: "geometry.difference".into(),
        label: "Fark".into(),
        category: "geometry".into(),
        description: "Her nesneden çıkarılacak alanların kapladığı kısmı atar, kalanı öznitelikleriyle yeni katmana yazar.".into(),
        help: Some([
            "Çıkarılacak alanlar önce birleşir; her nesnenin onların dışında kalan kısmı yazılır: alanın kalan parçaları, çizginin dışarıdaki parçaları, dışarıdaki noktalar. Sınır üstündeki çizgi parçası atılır.",
            "Bütünüyle örtülen nesne yazılmaz ve söylenir. Öznitelikler nesnenin kendisinindir.",
        ]
        .join("\n\n")),
        keywords: ["fark", "çıkar", "erase", "difference", "sil", "bindirme", "overlay"]
            .map(String::from)
            .to_vec(),
        aliases: ["FARKAL", "GEOERASE"].map(String::from).to_vec(),
        icon: Some("geoDifference".into()),
        parameters: vec![
            features_param("input", "Nesneler", &GEO_KINDS, geo_scopes(), "Kısımları çıkarılacak alanlar, çizgiler ve noktalar."),
            features_param("overlay", "Çıkarılacak alanlar", &AREA_KINDS, geo_scopes(), "Kapladıkları kısım nesnelerden atılır."),
            layer_param("Fark", "#E5484D"),
        ],
        outputs: outputs("Kalan nesneler", "Yazılan nesne sayısı"),
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(|v: &Resolved<'_>, _ctx: &RunContext<'_>, feedback: &mut dyn Feedback| {
            let (add, _) = overlay(Mode::Difference, v, "", &[], feedback);
            let summary = format!("{} nesnenin farkı yazıldı.", add.len());
            result(add, summary)
        }),
    }
}

pub fn sym_difference() -> Tool {
    Tool {
        id: "geometry.symDifference".into(),
        label: "Simetrik fark".into(),
        category: "geometry".into(),
        description: "İki alan kümesinin yalnız birinin kapladığı parçaları, kendi taraflarının öznitelikleriyle yeni katmana yazar.".into(),
        help: Some([
            "Önce birinci alanların ikincilerin dışında kalan parçaları, sonra ikincilerin birincilerin dışında kalanları yazılır; ortak kısımlar atılır.",
            "Birinci taraftan gelen parça kendi özniteliklerini, ikinciden gelen kendininkileri Önek ile taşır.",
        ]
        .join("\n\n")),
        keywords: ["simetrik fark", "symmetrical difference", "xor", "bindirme", "overlay", "değişen alanlar"]
            .map(String::from)
            .to_vec(),
        aliases: ["SIMFARK", "SYMDIFF"].map(String::from).to_vec(),
        icon: Some("geoSymDifference".into()),
        parameters: vec![
            features_param("input", "Birinci alanlar", &AREA_KINDS, geo_scopes(), "Kapalı alanlar, daireler, tam elipsler ve kapalı eğriler."),
            features_param("overlay", "İkinci alanlar", &AREA_KINDS, geo_scopes(), "Birincilerle karşılaştırılan alanlar."),
            prefix_param(),
            layer_param("Simetrik fark", "#E93D82"),
        ],
        outputs: outputs("Parçalar", "Parça sayısı"),
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(|v: &Resolved<'_>, _ctx: &RunContext<'_>, feedback: &mut dyn Feedback| {
            let (add, c) = overlay(Mode::SymDifference, v, v.text("prefix"), &[], feedback);
            let summary = format!(
                "{} parça yazıldı: {} birinci, {} ikinci alanlardan.",
                add.len(),
                c.first,
                c.second
            );
            result(add, summary)
        }),
    }
}

pub fn union() -> Tool {
    Tool {
        id: "geometry.union".into(),
        label: "Birleşim".into(),
        category: "geometry".into(),
        description: "İki alan kümesini bindirir: ortak parçalar iki tarafın, kalan parçalar kendi taraflarının öznitelikleriyle yeni katmana yazılır.".into(),
        help: Some([
            "Önce her birinci alanın her ikinciyle ortak parçası (iki tarafın öznitelikleriyle), sonra birincilerin ikincilerin dışında kalan, en son ikincilerin birincilerin dışında kalan parçaları yazılır. İkinci tarafın öznitelikleri Önek ile yazılır.",
            APPORTION_HELP,
        ]
        .join("\n\n")),
        keywords: ["birleşim", "union", "bindirme", "overlay", "paylaştır"]
            .map(String::from)
            .to_vec(),
        aliases: ["BIRLESIM", "GEOUNION"].map(String::from).to_vec(),
        icon: Some("geoUnion".into()),
        parameters: vec![
            features_param("input", "Birinci alanlar", &AREA_KINDS, geo_scopes(), "Kapalı alanlar, daireler, tam elipsler ve kapalı eğriler."),
            features_param("overlay", "İkinci alanlar", &AREA_KINDS, geo_scopes(), "Birincilerle bindirilen alanlar."),
            prefix_param(),
            apportion_param(),
            layer_param("Birleşim", "#3E63DD"),
        ],
        outputs: outputs("Parçalar", "Parça sayısı"),
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(|v: &Resolved<'_>, _ctx: &RunContext<'_>, feedback: &mut dyn Feedback| {
            let fields = field_names(v.text("apportion"));
            let (add, c) = overlay(Mode::Union, v, v.text("prefix"), &fields, feedback);
            let summary = format!(
                "{} parça yazıldı: {} ortak, {} yalnız birinci, {} yalnız ikinci alanlarda.",
                add.len(),
                c.both,
                c.first,
                c.second
            );
            result(add, summary)
        }),
    }
}
