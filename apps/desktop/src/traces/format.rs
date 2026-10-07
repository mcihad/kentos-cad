//! The trace format (`kentos.interaction-trace` v1,
//! fixtures/interaction/README.md). Every field is named: a field the player
//! does not know stops it, so a new one cannot be skipped unnoticed.

use std::path::Path;

use serde::Deserialize;

use super::folder;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Trace {
    pub format: String,
    pub version: u32,
    pub id: String,
    /// What the trace shows; the runner's report names it.
    #[cfg_attr(not(test), allow(dead_code))]
    pub title: String,
    /// For the reader: where the trace comes from and the TODOS items it covers.
    #[allow(dead_code)]
    pub source: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    pub covers: Vec<String>,
    pub document: String,
    pub view: View,
    #[serde(default)]
    pub draft: DraftSpec,
    #[serde(default)]
    pub prefs: Prefs,
    pub click_tolerance: f64,
    /// A file of the traces' folder the open dialog answers with (Metin
    /// dosyası yerleştir's, docs/adr/0145 §6); none: the trace's own drawing file.
    pub open_file: Option<String>,
    pub steps: Vec<Step>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct View {
    pub center: [f64; 2],
    pub metres_per_pixel: f64,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DraftSpec {
    #[serde(default)]
    pub(super) snap: bool,
    #[serde(default)]
    pub(super) grid: bool,
    #[serde(default)]
    pub(super) ortho: bool,
    #[serde(default)]
    pub(super) polar: bool,
    #[serde(default)]
    pub(super) tracking: bool,
    /// Topological editing and its Noktalar da (docs/adr/0160).
    #[serde(default)]
    pub(super) topology: bool,
    #[serde(default)]
    pub(super) topology_points: bool,
    /// The overlap control's mode (`allow`, `layer`, `layers`) and Seçili
    /// katmanlarda önle's layers (docs/adr/0162); absent: Serbest, none.
    #[serde(default)]
    pub(super) overlap: Option<String>,
    #[serde(default)]
    pub(super) overlap_layers: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Prefs {
    pub(super) cursor_input: Option<bool>,
    /// The highlight's colour and width (Görünüm kipleri, docs/adr/0195).
    pub(super) highlight_color: Option<String>,
    pub(super) highlight_width: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Step {
    pub(super) run: Option<String>,
    /// Draws with the drawing's or a library's object template of this id,
    /// as choosing it does (docs/adr/0176 §3).
    pub(super) template: Option<String>,
    /// Applies the template of this id to the selected objects, as the
    /// Şablonlar panel's Seçili nesnelere uygula does (docs/adr/0176 §6).
    pub(super) apply_template: Option<String>,
    pub(super) key: Option<String>,
    pub(super) text: Option<String>,
    #[serde(rename = "move")]
    pub(super) move_to: Option<[f64; 2]>,
    /// The pointer comes to the point and rests past object tracking's
    /// dwell (docs/adr/0085): the player moves there, then lets the wait end.
    pub(super) rest: Option<[f64; 2]>,
    pub(super) click: Option<[f64; 2]>,
    /// The left button down at the first point, moved to the second, released there.
    pub(super) drag: Option<[[f64; 2]; 2]>,
    pub(super) double_click: Option<[f64; 2]>,
    pub(super) right_click: Option<[f64; 2]>,
    pub(super) focus: Option<String>,
    pub(super) save_and_reopen: Option<bool>,
    /// Shift held during the step's click or drag.
    pub(super) shift: Option<bool>,
    /// A named picture of the app as it is now (usage scenarios, `kentos-cad
    /// kullan`); a test run passes over it.
    pub(super) shot: Option<String>,
    /// The open window, by its title, answered with `fill`, `check` and
    /// `press`, in that order (answers.rs).
    pub(super) dialog: Option<String>,
    /// Its fields by label, each typed over with this text, in order.
    pub(super) fill: Option<InOrder<String>>,
    /// Its check boxes by their words, each set on or off, in order.
    pub(super) check: Option<InOrder<bool>>,
    /// Its button with these words, pressed last.
    pub(super) press: Option<String>,
    /// The bottom panel's open tab, by its title (`Arama`), answered with
    /// `fill`, `check`, `pick`, `sort`, `row`, `press` and `key`, in that
    /// order (docs/adr/0178; answers.rs).
    pub(super) panel: Option<String>,
    /// Its lists by name, each given the item with these words, in order.
    pub(super) pick: Option<InOrder<String>>,
    /// Its table's header, pressed.
    pub(super) sort: Option<String>,
    /// Its n-th result row, pressed (1 first); `shift` and `ctrl` held.
    pub(super) row: Option<usize>,
    /// Öznitelik tablosu's cell (docs/adr/0199 §4): the n-th row's value
    /// under the header, double-clicked, typed over and written with Enter.
    pub(super) edit: Option<CellEdit>,
    pub(super) ctrl: Option<bool>,
    /// Genel bakış's picture pressed where it shows this point of the
    /// drawing, east and north of `view.center` (docs/adr/0181).
    pub(super) overview: Option<[f64; 2]>,
    /// Büyüteç's zoom button, by its words (`8×`).
    pub(super) magnifier: Option<String>,
    /// The paragraph editor (docs/adr/0182 §4): its text typed, letters
    /// chosen and formatted, then kept or dropped.
    pub(super) paragraph: Option<ParagraphStep>,
    pub(super) expect: Option<Expect>,
    /// For the reader; not checked.
    #[allow(dead_code)]
    pub(super) note: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Expect {
    pub(super) tool: Option<String>,
    pub(super) points: Option<usize>,
    /// The prompt's whole text, exactly (docs/adr/0083).
    pub(super) prompt: Option<String>,
    /// Texts the step must have written, each whole and in this order, among
    /// all its messages at any level; others may come between.
    pub(super) logged: Option<Vec<String>>,
    pub(super) options: Option<Vec<String>>,
    /// `null` (closed) and absent (not compared) differ.
    #[serde(default, deserialize_with = "present")]
    pub(super) dynamic_input: Option<Option<String>>,
    pub(super) command_line: Option<String>,
    pub(super) entities: Option<usize>,
    pub(super) newest: Option<Newest>,
    pub(super) can_undo: Option<bool>,
    pub(super) can_redo: Option<bool>,
    pub(super) dirty: Option<bool>,
    pub(super) log: Option<String>,
    pub(super) metres_per_pixel: Option<f64>,
    /// The view's centre, east and north of `view.center`: where Kaydır and the
    /// zooms left it (docs/adr/0056), within `clickTolerance`.
    pub(super) view_center: Option<[f64; 2]>,
    /// The selected objects' ids, in the order they were selected.
    pub(super) selected: Option<Vec<u32>>,
    /// The hovered object's id; `null` (none) and absent differ.
    #[serde(default, deserialize_with = "present")]
    pub(super) hover: Option<Option<u32>>,
    /// Sıradakini seç's chip (docs/adr/0187 §1): which candidate is chosen
    /// (from 0) of how many; `null` (no chip) and absent differ.
    #[serde(default, deserialize_with = "present")]
    pub(super) cycle: Option<Option<CycleSeen>>,
    /// The object snap's kind the marker shows (`endpoint` …); `null` (none) and absent differ.
    #[serde(default, deserialize_with = "present")]
    pub(super) snap: Option<Option<String>>,
    /// Every object's id, in the drawing's order.
    pub(super) ids: Option<Vec<u32>>,
    /// Object tracking's acquired points, oldest first, east and north of
    /// `view.center`, within `clickTolerance` (docs/adr/0085).
    pub(super) track_points: Option<Vec<[f64; 2]>>,
    /// The alignment the cursor is locked to; `null` (none) and absent differ.
    #[serde(default, deserialize_with = "present")]
    pub(super) track: Option<Option<TrackExpect>>,
    /// Objects by their ids, each as `newest` is read (docs/adr/0037): a moved one in place.
    pub(super) objects: Option<Vec<Newest>>,
    /// The open window's title; `null` (none) and absent differ.
    #[serde(default, deserialize_with = "present")]
    pub(super) dialog: Option<Option<String>>,
    /// The bottom panel's open tab; `null` (closed) and absent differ.
    #[serde(default, deserialize_with = "present")]
    pub(super) panel: Option<Option<String>>,
    /// Arama's count line and rows (Katman, Tür, Alan, Değer joined by “ | ”),
    /// exact (docs/adr/0178).
    pub(super) search: Option<SearchExpect>,
    /// Öznitelik tablosu's count, headers (when given) and rows (the cells
    /// after Sıra joined by “ | ”), exact (docs/adr/0199 §4).
    #[serde(rename = "featureTable")]
    pub(super) feature_table: Option<FeatureTableExpect>,
    /// Topoloji's count line and rows (Katman, Kural, Sorun, Nesneler, Ölçü
    /// joined by “ | ”), exact (docs/adr/0202 §5).
    pub(super) topology: Option<SearchExpect>,
    /// The place the data search's Git marked, east and north of
    /// `view.center`, within `clickTolerance`; `null` none, and absent differ.
    #[serde(default, deserialize_with = "present")]
    pub(super) mark: Option<Option<[f64; 2]>>,
    /// The digitizing locks holding the next point, as their chips and
    /// the command line say them, in order (docs/adr/0166 §6); `[]` for none.
    pub(super) locks: Option<Vec<String>>,
    /// The active layer: the names of the groups above it and its own
    /// (docs/adr/0176 §3).
    pub(super) active_layer: Option<Vec<String>>,
    /// The layers and groups hidden or locked of their own, by their paths
    /// (“Yapılar / Bina”), in tree order (docs/adr/0177 §1).
    pub(super) hidden_layers: Option<Vec<String>>,
    pub(super) locked_layers: Option<Vec<String>>,
    /// Every layer and group by its path, in tree order (docs/adr/0177 §5).
    pub(super) layers: Option<Vec<String>>,
    /// The colour and line weight new objects take now (the ribbon's
    /// Renk and Kalınlık); `null` the layer's, and absent differ.
    #[serde(default, deserialize_with = "present")]
    pub(super) current_color: Option<Option<String>>,
    #[serde(default, deserialize_with = "present")]
    pub(super) current_weight: Option<Option<f64>>,
    /// Genel bakış's extent, east and north of `view.center`; `null` when it
    /// is closed, and absent differ (docs/adr/0181).
    #[serde(default, deserialize_with = "present")]
    pub(super) overview: Option<Option<OverviewExpect>>,
    /// Büyüteç's zoom and side, exact, and its centre within `clickTolerance`;
    /// `null` when it is closed.
    #[serde(default, deserialize_with = "present")]
    pub(super) magnifier: Option<Option<MagnifierExpect>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverviewExpect {
    pub(super) extent: [f64; 4],
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MagnifierExpect {
    pub(super) zoom: u32,
    pub(super) side: Option<String>,
    pub(super) center: Option<[f64; 2]>,
}

/// Arama as a step sees it: the count line and every row.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchExpect {
    pub(super) count: String,
    pub(super) rows: Vec<String>,
}

/// Öznitelik tablosu as a step sees it: the count, the headers after Sıra
/// (when the step names them) and every row.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeatureTableExpect {
    pub(super) count: String,
    pub(super) columns: Option<Vec<String>>,
    pub(super) rows: Vec<String>,
}

/// A cell of Öznitelik tablosu edited: its row (1 first), its column's
/// header and the text typed over its value.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CellEdit {
    pub(super) row: usize,
    pub(super) column: String,
    pub(super) text: String,
}

/// The expected lock: its point (within `clickTolerance`) and its lines in
/// the core's order, each with its acquired point and its angle (exact).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrackExpect {
    pub(super) point: [f64; 2],
    pub(super) lines: Vec<TrackLineExpect>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrackLineExpect {
    pub(super) origin: [f64; 2],
    pub(super) angle: f64,
}

/// What a `paragraph` step does in the open paragraph editor (docs/adr/0182 §4).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParagraphStep {
    /// The whole text, typed over what is there.
    pub(super) text: Option<String>,
    /// Letters `start..end` chosen, then a format's button pressed: `bold`,
    /// `italic`, `underline`, `super`, `sub`, or `{ "color": … }` (null: the text's).
    #[serde(default)]
    pub(super) formats: Vec<(u32, u32, serde_json::Value)>,
    /// `keep` (Tamam) or `drop` (Vazgeç).
    pub(super) close: Option<String>,
}

/// An object's expected shape: the newest one, or one of `objects` by its `id`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Newest {
    /// In `objects`: whose shape this is.
    pub(super) id: Option<u32>,
    pub(super) kind: String,
    pub(super) points: Option<Vec<[f64; 2]>>,
    pub(super) edges: Option<Vec<[f64; 2]>>,
    pub(super) arcs: Option<usize>,
    /// A circle's or an arc's centre, relative to the view's centre (docs/adr/0032).
    pub(super) center: Option<[f64; 2]>,
    pub(super) radius: Option<f64>,
    /// A text's content, exact (docs/adr/0144 §7).
    pub(super) text: Option<String>,
    /// A text's alignment by its name, `null` the left of the baseline, its
    /// width factor (1 without one) and mask (docs/adr/0145); absent, not compared.
    #[serde(default, deserialize_with = "present")]
    pub(super) align: Option<Option<String>>,
    #[serde(rename = "widthFactor")]
    pub(super) width_factor: Option<f64>,
    pub(super) mask: Option<bool>,
    /// A text's turn in degrees, exact (Okunur yap, docs/adr/0145 §6).
    pub(super) rotation: Option<f64>,
    /// A text's curve: its vertices past its point, `null` a straight text
    /// (docs/adr/0196); absent, not compared.
    #[serde(default, deserialize_with = "present")]
    pub(super) curve: Option<Option<usize>>,
    /// A text's height, within 1e-9 (a label's size at a scale, docs/adr/0175).
    pub(super) height: Option<f64>,
    /// A leader's arrowhead by its name, `null` the filled arrow (docs/adr/0146); absent, not compared.
    #[serde(default, deserialize_with = "present")]
    pub(super) arrow: Option<Option<String>>,
    /// A dimension's direction in degrees (a linear one's measured, an
    /// ordinate's axis; docs/adr/0147), within 1e-9: a typed angle in grads
    /// comes back through radians.
    pub(super) angle: Option<f64>,
    /// The text beside it (a survey point's name), `null` none; its
    /// attributes, all of them; a point's elevation, `null` none
    /// (docs/adr/0152). Exact; absent, not compared.
    #[serde(default, deserialize_with = "present")]
    pub(super) label: Option<Option<String>>,
    pub(super) attrs: Option<std::collections::BTreeMap<String, String>>,
    #[serde(default, deserialize_with = "present")]
    pub(super) z: Option<Option<f64>>,
    /// The outer path's bulges, each within 1e-9, one left out a straight
    /// edge; its vertex elevations (a line's two ends), each within 1e-9,
    /// `null` none (docs/adr/0160: a shared arc, an elevation along an edge).
    pub(super) bulges: Option<Vec<f64>>,
    pub(super) zs: Option<Vec<Option<f64>>>,
    /// An area's holes, in all its parts (docs/adr/0173 §5), exact.
    pub(super) holes: Option<usize>,
    /// A linked text's object by its slot, `null` a text of its own; its
    /// scale (docs/adr/0175 §4). Exact; absent, not compared.
    #[serde(rename = "labelOf", default, deserialize_with = "present")]
    pub(super) label_of: Option<Option<u32>>,
    #[serde(rename = "labelScale")]
    pub(super) label_scale: Option<f64>,
    /// A multi-line text's box width (from clicks: within the click
    /// tolerance), `null` none; its line spacing, `null` none, and runs,
    /// exact (docs/adr/0182).
    #[serde(rename = "boxWidth", default, deserialize_with = "present")]
    pub(super) box_width: Option<Option<f64>>,
    #[serde(rename = "lineSpacing", default, deserialize_with = "present")]
    pub(super) line_spacing: Option<Option<f64>>,
    pub(super) runs: Option<Vec<kentos_contracts::TextRun>>,
    /// Its own symbol, colour and line weight, `null` none; its layer's
    /// name (docs/adr/0176 §3). Exact; absent, not compared.
    #[serde(default, deserialize_with = "present")]
    pub(super) symbol: Option<Option<String>>,
    #[serde(default, deserialize_with = "present")]
    pub(super) color: Option<Option<String>>,
    #[serde(rename = "lineWeight", default, deserialize_with = "present")]
    pub(super) line_weight: Option<Option<f64>>,
    pub(super) layer: Option<String>,
    /// A text's style and face, a dimension's style and look, as the contract
    /// writes them (`{}` none; docs/adr/0183). Exact; absent, not compared.
    pub(super) face: Option<kentos_contracts::TextFace>,
    pub(super) look: Option<kentos_contracts::DimensionLook>,
    /// A table's cells and look (docs/adr/0184); absent, not compared.
    pub(super) table: Option<TableExpect>,
    /// A hatch's pattern (docs/adr/0186): its kind, a pattern's name, a
    /// gradient's shape; exact, an absent member none. Absent, not compared.
    pub(super) pattern: Option<HatchPatternSeen>,
    /// How many objects a hatch follows: its closed object, islands and
    /// cutouts, 0 none (docs/adr/0186 §6). Exact; absent, not compared.
    pub(super) assoc: Option<usize>,
}

/// A hatch's pattern as a trace sees it: its kind (`pattern`), a pattern's
/// name (`ANSI31`) and a gradient's shape (`spherical`), none for others.
#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
#[serde(deny_unknown_fields)]
pub struct HatchPatternSeen {
    #[serde(rename = "type")]
    pub kind: String,
    pub name: Option<String>,
    pub shape: Option<String>,
}

/// A table's expected cells (exact), rows' heights and columns' widths
/// (within 1e-6), merged ranges, columns' alignment (`null` all left), its
/// heading row, lines (`null` all), frame width (within 1e-9, `null` none)
/// and source's kind (`null` none). Each absent one is not compared.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TableExpect {
    pub(super) cells: Option<Vec<Vec<String>>>,
    pub(super) rows: Option<Vec<f64>>,
    pub(super) columns: Option<Vec<f64>>,
    pub(super) merges: Option<Vec<kentos_contracts::CellRange>>,
    #[serde(default, deserialize_with = "present")]
    pub(super) aligns: Option<Option<Vec<String>>>,
    pub(super) header: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    pub(super) grid: Option<Option<String>>,
    #[serde(default, deserialize_with = "present")]
    pub(super) frame: Option<Option<f64>>,
    #[serde(default, deserialize_with = "present")]
    pub(super) source: Option<Option<String>>,
}

/// A JSON object's members in the order they are written (a `dialog` step's
/// fields and boxes are used in that order, as on the web).
#[derive(Debug)]
pub struct InOrder<T>(pub(super) Vec<(String, T)>);

impl<'de, T: Deserialize<'de>> Deserialize<'de> for InOrder<T> {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Members<T>(std::marker::PhantomData<T>);
        impl<'de, T: Deserialize<'de>> serde::de::Visitor<'de> for Members<T> {
            type Value = InOrder<T>;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("an object")
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error> {
                let mut members = Vec::new();
                while let Some(member) = map.next_entry()? {
                    members.push(member);
                }
                Ok(InOrder(members))
            }
        }
        d.deserialize_map(Members(std::marker::PhantomData))
    }
}

/// A field that is there, even as `null`.
fn present<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(d).map(Some)
}

/// Traces of web features the desktop does not have yet, by id, with why.
/// They are left out of [`Trace::all`] and said as skipped, never passed;
/// the entry goes with the port (a test fails once one would play).
#[cfg(test)]
pub const PENDING: &[(&str, &str)] = &[];

impl Trace {
    pub fn read(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("{} okunamadı: {e}", path.display()))?;
        let trace: Trace =
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        if trace.format != "kentos.interaction-trace" || trace.version != 1 {
            return Err(format!("{}: v1 etkileşim izi değil", path.display()));
        }
        Ok(trace)
    }

    /// Every trace of the folder, by file name, but the [`PENDING`] ones.
    #[cfg(test)]
    pub fn all() -> Result<Vec<Self>, String> {
        let dir = folder();
        let mut paths: Vec<std::path::PathBuf> = std::fs::read_dir(&dir)
            .map_err(|e| format!("{} okunamadı: {e}", dir.display()))?
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|e| e == "json"))
            .filter(|p| {
                !PENDING
                    .iter()
                    .any(|(id, _)| p.file_stem().is_some_and(|s| s == *id))
            })
            .collect();
        paths.sort();
        paths.iter().map(|p| Self::read(p)).collect()
    }

    pub fn by_id(id: &str) -> Result<Self, String> {
        Self::read(&folder().join(format!("{id}.json")))
    }
}

/// Sıradakini seç's chip as a trace sees it: the candidate chosen, from 0, of how many.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CycleSeen {
    pub(crate) index: usize,
    pub(crate) count: usize,
}
