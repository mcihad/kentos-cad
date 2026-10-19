//! The atlas tests' drawing (docs/adr/0534): a hundred and five parcels on a TUREF / TM27 project, five of them
//! cancelled (`Durum` = “iptal”) and so outside the layer's filter, an owner each (`Malik`, which a form marks
//! `redact`) and an attribute no field declares (`Sır`); a road layer for the maps; and the system's atlas
//! sheet, its title, a note of the page's own fields and an attribute table of the parcels of the atlas object.
//! `scripts/fixtures/atlas_cases.py` writes the same drawing without KentOS code and the pages it must give.

use kentos_contracts::DocumentSnapshotV1;
use kentos_domain::Document;
use kentos_sheet::model::{Atlas, SheetBook, SortKey};
use serde_json::{Value, json};

pub const COLUMNS: usize = 15;
pub const ROWS: usize = 7;
/// Parcel numbers (1-based) that are cancelled.
pub const CANCELLED: [usize; 5] = [5, 20, 35, 50, 65];
/// The east and north of the first parcel's lower left corner; the parcels are 30 × 20 m, 2 m apart.
pub const ORIGIN: (f64, f64) = (400_000.0, 4_400_000.0);
pub const WIDTH: f64 = 30.0;
pub const HEIGHT: f64 = 20.0;
pub const GAP: f64 = 2.0;

/// The last parcel is a large one, 90 × 50 m: it needs a smaller scale than the rest.
pub const BIG: (f64, f64) = (90.0, 50.0);

/// The parcel `i` (0-based): its number, its ada, its lower left corner, its width and height.
pub fn parcel(i: usize) -> (usize, String, (f64, f64), (f64, f64)) {
    let (col, row) = (i % COLUMNS, i / COLUMNS);
    let size = if i + 1 == COLUMNS * ROWS {
        BIG
    } else {
        (WIDTH, HEIGHT)
    };
    (
        i + 1,
        format!("A{}", row + 1),
        (
            ORIGIN.0 + col as f64 * (WIDTH + GAP),
            ORIGIN.1 + row as f64 * (HEIGHT + GAP),
        ),
        size,
    )
}

/// The drawing, without a sheet book.
pub fn drawing() -> Value {
    drawing_with(true)
}

/// The drawing with the parcel layer's filter, or without it.
pub fn drawing_with(filter: bool) -> Value {
    let mut entities: Vec<Value> = Vec::new();
    for i in 0..COLUMNS * ROWS {
        let (n, ada, (x, y), (w, h)) = parcel(i);
        let durum = if CANCELLED.contains(&n) {
            "iptal"
        } else {
            "aktif"
        };
        entities.push(json!({
            "kind": "polygon", "id": entities.len() + 1, "layerId": "parsel",
            "pts": [{"x": x, "y": y}, {"x": x + w, "y": y}, {"x": x + w, "y": y + h}, {"x": x, "y": y + h}],
            "attrs": {"Ada": ada, "Parsel": n.to_string(), "Durum": durum, "Malik": format!("Gizli Kişi {n}"), "Sır": format!("gizli-not-{n}")}
        }));
    }
    for k in 0..4 {
        let y = ORIGIN.1 - 10.0 + f64::from(k) * 60.0;
        entities.push(json!({
            "kind": "line", "id": entities.len() + 1, "layerId": "yol",
            "a": {"x": ORIGIN.0 - 20.0, "y": y}, "b": {"x": ORIGIN.0 + 520.0, "y": y}, "attrs": {}
        }));
    }
    let field = |name: &str, kind: &str| json!({"name": name, "kind": kind});
    let layer = |id: &str, name: &str, color: &str| {
        json!({"id": id, "name": name, "type": "layer", "visible": true, "locked": false, "expanded": true,
               "style": {"color": color, "lineType": "continuous", "lineWeight": 0.35}, "children": []})
    };
    let mut parsel = layer("parsel", "Parsel", "#0090FF");
    parsel["fields"] = json!([
        field("Ada", "text"),
        field("Parsel", "integer"),
        field("Durum", "text"),
        field("Malik", "text")
    ]);
    if filter {
        parsel["filter"] = json!({"expression": "Durum = 'aktif'"});
    }
    let body = json!({"revision": 1, "items": [
        {"type": "field", "id": "malik", "field": "Malik", "redact": true}
    ]});
    let form =
        json!({"id": "parsel-formu", "name": "Parsel formu", "layer": "parsel", "draft": body});
    json!({
        "format": "kentos.document", "version": 1, "name": "Parsel atlası",
        "settings": {"srid": 5253, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                     "plotScale": 1000, "workspace": "gis", "forms": [form]},
        "origin": {"x": ORIGIN.0, "y": ORIGIN.1},
        "layers": [parsel, layer("yol", "Yol", "#555555")],
        "activeLayer": "parsel", "entities": entities, "styles": {"items": [], "categories": []}
    })
}

/// The atlas sheet `atlas-paftasi`: the system template's A4 page with the coverage layer, a sort by the parcel
/// number, a page name of ada and number, a note of the object's own fields and a table of the parcels of the
/// atlas object.
pub fn sheet_book() -> SheetBook {
    use kentos_sheet::template::{InstanceIds, InstanceOptions, instantiate, system_template};
    let t = system_template("sys:gis-atlas").expect("the atlas template");
    let ids = InstanceIds {
        sheet: "atlas-paftasi".into(),
        master: None,
        items: t.sheet.items.iter().map(|i| i.id.clone()).collect(),
        master_items: Vec::new(),
    };
    let options = InstanceOptions {
        name: Some("Parsel atlası".into()),
        ..InstanceOptions::default()
    };
    let inst = instantiate(t, &ids, &options).expect("an instance");
    let mut sheet = inst.sheet;
    sheet.atlas = Some(Atlas {
        layer: "parsel".into(),
        filter: String::new(),
        sort: vec![SortKey {
            expression: "Parsel".into(),
            descending: false,
        }],
        page_name: "Ada || '/' || Parsel".into(),
    });
    let mut value = serde_json::to_value(&sheet).expect("json");
    let items = value["items"].as_array_mut().expect("items");
    items.push(json!({
        "id": "not", "name": "Nesne notu", "frame": {"left": 19000, "top": 22000, "width": 177000, "height": 7000},
        "kind": {"type": "text", "content": "Ada [% Ada %] parsel [% Parsel %] durum [% Durum %] malik [% Malik %] sır [% Sır %]",
                 "style": {"size": 2600, "weight": 500}, "wrap": false}
    }));
    items.push(json!({
        "id": "parseller", "name": "Atlas nesnesinin parselleri", "frame": {"left": 96000, "top": 130000, "width": 100000, "height": 40000},
        "kind": {"type": "table", "title": "PARSELLER", "header": true,
                 "columns": [{"heading": "Ada", "value": "Ada", "width": 25000}, {"heading": "Parsel", "value": "Parsel", "width": 25000},
                             {"heading": "Malik", "value": "Malik"}],
                 "source": {"type": "layer", "layer": "parsel", "atlasFilter": true}}
    }));
    sheet = serde_json::from_value(value).expect("a sheet");
    SheetBook {
        sheets: vec![sheet],
        assets: inst.assets,
        ..SheetBook::default()
    }
}

/// The project: the drawing with the atlas sheet in its book.
pub fn document() -> Document {
    document_of(drawing())
}

/// The project of `drawing` (with or without the layer's filter) and the atlas sheet.
pub fn document_of(drawing: Value) -> Document {
    let snapshot = DocumentSnapshotV1::from_json(&drawing.to_string()).expect("reads");
    let mut doc = Document::from_snapshot(snapshot).expect("opens");
    let kept = kentos_sheet::project::project_sheets(&sheet_book(), |_| None).expect("kept");
    doc.set_sheets(kept).expect("the book");
    doc
}
