//! Semt ve uzunluktan çizgi, XY'den çizgi and Tablodan elips (the web's
//! `tables.ts`; docs/adr/0218 §5–§7): a row each, from a file or from
//! objects' attributes; the cells are read and the shapes made in the core
//! (docs/adr/0180's number rule); a row that gives nothing is in the report
//! with its line or object number and why.

use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::ops::convert::table::{RowFault, bearing_rows, ellipse_rows, xy_rows};
use serde_json::json;

use super::super::geometry::{layer_param, new_object};
use super::{Rows, column, read_rows, reason, source_params, unit_param};
use crate::text::js_trim;
use crate::types::{
    ChangeSet, Feedback, OutputDef, OutputKind, Resolved, RunContext, RunResult, Target, Tool,
};

fn outputs() -> Vec<OutputDef> {
    vec![
        OutputDef::new("count", "Yazılan nesne sayısı", OutputKind::Number),
        OutputDef::new("table", "Atlanan satırlar", OutputKind::Table),
    ]
}

/// A table tool's run: its rows, the core's results, the objects, the report.
fn run(
    v: &Resolved<'_>,
    names: &[&str],
    make: impl Fn(&[Vec<String>]) -> Vec<Result<Shape, RowFault>>,
    what: &str,
    feedback: &mut dyn Feedback,
) -> RunResult {
    let columns: Vec<String> = names
        .iter()
        .map(|n| js_trim(v.text(n)).to_owned())
        .collect();
    let rows: Rows = match read_rows(v, &columns) {
        Ok(rows) => rows,
        Err(why) => return RunResult::refused(why),
    };
    feedback.progress(0.0, "Satırlar okunuyor");
    let layer = &v.layer("layer").id;
    let mut add = Vec::new();
    let mut report = Vec::new();
    for (i, r) in make(&rows.cells).into_iter().enumerate() {
        match r {
            Ok(s) => add.extend(new_object(s, layer, rows.attrs[i].clone())),
            Err(fault) => report.push(json!([
                rows.labels[i],
                reason(fault, &columns, &rows.cells[i])
            ])),
        }
    }
    if !report.is_empty() {
        feedback.warn(format!(
            "{} satır atlandı; nedenleri tabloda.",
            report.len()
        ));
    }
    let count = add.len();
    let mut out: std::collections::BTreeMap<String, serde_json::Value> =
        [("count".to_owned(), json!(count))].into_iter().collect();
    if !report.is_empty() {
        out.insert(
            "table".into(),
            json!({ "columns": ["Satır", "Neden"], "rows": report }),
        );
    }
    RunResult {
        changes: Some(ChangeSet {
            add,
            ..ChangeSet::default()
        }),
        outputs: out.into_iter().collect(),
        summary: Some(format!(
            "{} satırdan {count} {what} yazıldı.",
            rows.cells.len()
        )),
        ..RunResult::default()
    }
}

fn lines(
    rows: Vec<Result<[kentos_geometry_core::vec2::Vec2; 2], RowFault>>,
) -> Vec<Result<Shape, RowFault>> {
    rows.into_iter()
        .map(|r| r.map(|[a, b]| Shape::Line { a, b }))
        .collect()
}

pub fn bearing_line() -> Tool {
    Tool {
        id: "convert.bearingLine".into(),
        label: "Semt ve uzunluktan çizgi".into(),
        category: "conversion".into(),
        description: "Bir tablonun her satırındaki başlangıç noktası, semt ve uzunluktan çizgi yazar.".into(),
        help: Some([
            "Örnekler: bir aplikasyon karnesindeki doğrultu ve mesafelerden çizgiler, bir röper noktasından ölçülen semt ve uzunluklar, eski bir tapu krokisindeki sınırların semt ve uzunlukları.",
            "Semt kuzeyden saat yönünde, seçilen açı biriminde (grad ya da derece). Sayılar ondalık nokta ya da virgülle okunur. Boş ya da sayı olmayan hücreli, uzunluğu sıfır ya da eksi satır atlanır ve tabloda nedeniyle söylenir.",
            "Kaynak bir dosya (ilk satırı sütun adları) ya da nesneler olabilir; nesnelerde öznitelikler satırdır. Çizgi satırın dolu hücrelerini öznitelik olarak alır.",
        ]
        .join("\n\n")),
        keywords: [
            "semt", "uzunluk", "açı", "mesafe", "doğrultu", "çizgi", "bearing distance to line",
            "cogo",
        ]
        .map(String::from)
        .to_vec(),
        aliases: vec!["SEMTUZUNLUK".into()],
        icon: Some("bearingLine".into()),
        parameters: [
            source_params("çizgi"),
            vec![
                column("east", "Doğu sütunu", "Başlangıç noktasının doğusu (CBS projesinde Y).", false),
                column("north", "Kuzey sütunu", "Başlangıç noktasının kuzeyi (CBS projesinde X).", false),
                column("bearing", "Semt sütunu", "Kuzeyden saat yönünde doğrultu.", false),
                column("length", "Uzunluk sütunu", "Çizginin uzunluğu (m).", false),
                unit_param(),
                layer_param("Semt çizgileri", "#AB4ABA"),
            ],
        ]
        .concat(),
        outputs: outputs(),
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run_bearing),
    }
}

fn run_bearing(v: &Resolved<'_>, _ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let unit = v.text("unit").to_owned();
    run(
        v,
        &["east", "north", "bearing", "length"],
        |cells| lines(bearing_rows(cells, &unit)),
        "çizgi",
        feedback,
    )
}

pub fn xy_line() -> Tool {
    Tool {
        id: "convert.xyLine".into(),
        label: "XY'den çizgi".into(),
        category: "conversion".into(),
        description: "Bir tablonun her satırındaki başlangıç ve bitiş koordinatlarından çizgi yazar.".into(),
        help: Some([
            "Örnekler: bir hesap tablosundaki kenarları, bir aplikasyon listesindeki bağlantıları, iki nokta listesinin eşleşmelerini çizgi olarak çizmek.",
            "Sayılar ondalık nokta ya da virgülle okunur. Boş ya da sayı olmayan hücreli satır ve başlangıcı ile bitişi aynı yerde olan satır atlanır ve tabloda nedeniyle söylenir. Çizgi satırın dolu hücrelerini öznitelik olarak alır.",
        ]
        .join("\n\n")),
        keywords: ["xy", "koordinat", "çizgi", "başlangıç", "bitiş", "xy to line"]
            .map(String::from)
            .to_vec(),
        aliases: vec!["XYCIZGI".into()],
        icon: Some("xyLine".into()),
        parameters: [
            source_params("çizgi"),
            vec![
                column("startEast", "Başlangıç doğu", "Başlangıcın doğusu (CBS projesinde Y).", false),
                column("startNorth", "Başlangıç kuzey", "Başlangıcın kuzeyi (CBS projesinde X).", false),
                column("endEast", "Bitiş doğu", "Bitişin doğusu.", false),
                column("endNorth", "Bitiş kuzey", "Bitişin kuzeyi.", false),
                layer_param("Çizgiler", "#12A594"),
            ],
        ]
        .concat(),
        outputs: outputs(),
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run_xy),
    }
}

fn run_xy(v: &Resolved<'_>, _ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    run(
        v,
        &["startEast", "startNorth", "endEast", "endNorth"],
        |cells| lines(xy_rows(cells)),
        "çizgi",
        feedback,
    )
}

pub fn table_ellipse() -> Tool {
    Tool {
        id: "convert.tableEllipse".into(),
        label: "Tablodan elips".into(),
        category: "conversion".into(),
        description: "Bir tablonun her satırındaki merkez, iki yarı eksen ve semtten elips yazar.".into(),
        help: Some([
            "Örnekler: ağ dengelemesinin hata elipslerini (a, b ve büyük eksenin semti) çizmek, ölçü belirsizliklerini ya da etki alanlarını göstermek.",
            "Büyük eksen semt doğrultusundadır (kuzeyden saat yönünde, seçilen açı biriminde; hücre boşsa kuzey). Küçük yarı eksen büyükten büyükse ikisi yer değiştirir, doğrultu dik açı döner. Yarı ekseni sıfır ya da eksi satır atlanır.",
        ]
        .join("\n\n")),
        keywords: [
            "elips", "hata elipsi", "yarı eksen", "tablo", "table to ellipse", "error ellipse",
        ]
        .map(String::from)
        .to_vec(),
        aliases: vec!["TABLODANELIPS".into()],
        icon: Some("tableEllipse".into()),
        parameters: [
            source_params("elips"),
            vec![
                column("east", "Doğu sütunu", "Merkezin doğusu (CBS projesinde Y).", false),
                column("north", "Kuzey sütunu", "Merkezin kuzeyi (CBS projesinde X).", false),
                column("major", "Büyük yarı eksen", "Büyük yarı eksen (m).", false),
                column("minor", "Küçük yarı eksen", "Küçük yarı eksen (m).", false),
                column("bearing", "Semt sütunu", "Büyük eksenin doğrultusu; boşsa kuzey.", true),
                unit_param(),
                layer_param("Elipsler", "#F76B15"),
            ],
        ]
        .concat(),
        outputs: outputs(),
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run_ellipse),
    }
}

fn run_ellipse(v: &Resolved<'_>, _ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let unit = v.text("unit").to_owned();
    run(
        v,
        &["east", "north", "major", "minor", "bearing"],
        |cells| ellipse_rows(cells, &unit),
        "elips",
        feedback,
    )
}
