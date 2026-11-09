//! Tadilat lejantı (docs/adr/3039 §5): Mimarlar Odası's example legend for a renovation project ("Projede mevcut, iptal
//! ve ilaveler üç farklı teknikle ifade edilmelidir"): its title, then three boxes and their words, mevcut a plain
//! outline, ilave hatched at 45°, iptal dashed. One block of the drawing, named by the digital project regulation's
//! EK-2 Tablo 4.4 (`MM-XXXX-TADILATLEJANTI`); its sizes are paper millimetres at the drawing's plot scale: texts 2.5 mm,
//! the title 3.5 mm, boxes 15 × 6 mm, the hatch's lines 3.175 mm apart (ANSI31's, as the Tadilat projesi view hatches
//! an ilave wall).

use kentos_contracts::{BlockDefinition, BlockId};
use serde_json::{Value, json};

use crate::Document;

/// The legend's block: one in a drawing, KentOS's.
pub const LEGEND_ID: BlockId = BlockId([
    0x01, 0x93, 0xa5, 0xe0, 0x30, 0x39, 0x70, 0x00, 0x80, 0x00, 0x74, 0x61, 0x64, 0x69, 0x6c, 0x74,
]);
/// Its name by EK-2 Tablo 4.4: discipline, element (XXXX: none), what it shows.
pub const LEGEND_NAME: &str = "MM-XXXX-TADILATLEJANTI";
/// Its title and its three words, top to bottom (Mimarlar Odası's legend).
pub const TITLE: &str = "Tadilat lejantı";
pub const WORDS: [&str; 3] = ["Mevcut", "İlave", "İptal"];

impl Document {
    /// The legend's block at the drawing's plot scale (docs/adr/3039 §5): its top left at its base point.
    pub fn renovation_legend_block(&self) -> BlockDefinition {
        // Model metres per paper millimetre.
        let mm = self.settings().plot_scale.max(1.0) / 1000.0;
        let mut out: Vec<Value> = Vec::new();
        let mut id = 0_u64;
        let mut next = || {
            id += 1;
            id
        };
        let rect = |x0: f64, y0: f64, x1: f64, y1: f64| {
            vec![
                json!({"x": x0, "y": y0}),
                json!({"x": x1, "y": y0}),
                json!({"x": x1, "y": y1}),
                json!({"x": x0, "y": y1}),
            ]
        };
        out.push(
            json!({"kind": "text", "id": next(), "layerId": "", "attrs": {},
            "p": {"x": 0.0, "y": -4.0 * mm}, "text": TITLE, "height": 3.5 * mm, "rotation": 0.0,
            "align": "bottomLeft"}),
        );
        for (k, word) in WORDS.iter().enumerate() {
            let top = -(8.0 + 9.0 * k as f64) * mm;
            let (x0, y0, x1, y1) = (0.0, top - 6.0 * mm, 15.0 * mm, top);
            let mut outline = json!({"kind": "polyline", "id": next(), "layerId": "", "attrs": {},
                "pts": rect(x0, y0, x1, y1), "closed": true});
            match k {
                // İlave: hatched at 45°.
                1 => out.push(json!({"kind": "hatch", "id": next(), "layerId": "", "attrs": {},
                    "ring": [{"x": x0, "y": y0}, {"x": x1, "y": y0}, {"x": x1, "y": y1}, {"x": x0, "y": y1}],
                    "pattern": {"type": "lines", "angle": 45.0, "spacing": 3.175 * mm}})),
                // İptal: dashed.
                2 => outline["lineType"] = json!("dashed"),
                _ => {}
            }
            out.push(outline);
            out.push(json!({"kind": "text", "id": next(), "layerId": "", "attrs": {},
                "p": {"x": 19.0 * mm, "y": top - 3.0 * mm}, "text": word, "height": 2.5 * mm, "rotation": 0.0,
                "align": "middleLeft"}));
        }
        BlockDefinition {
            id: LEGEND_ID,
            name: LEGEND_NAME.to_owned(),
            base: kentos_contracts::Vec2 { x: 0.0, y: 0.0 },
            entities: out.into_iter().filter_map(|v| serde_json::from_value(v).ok()).collect(),
            attributes: Vec::new(),
            description: Some(
                "Tadilat lejantı (Mimarlar Odası): mevcut düz kontur, ilave taralı, iptal kesikli. KentOS yazar.".into(),
            ),
            units: Some(kentos_contracts::BlockUnits::M),
            card: None,
            dynamic: None,
            xref: None,
        }
    }
}
