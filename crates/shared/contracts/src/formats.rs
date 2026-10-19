//! File exchange (CLAUDE.md §9.7, §14): what the format readers and writers
//! of `crates/shared/formats` take and give. The browser runs them in a Web Worker
//! (the `kentos-formats-wasm` module); the server's import job will run the
//! same code natively. Coordinates are float64 exactly as the file wrote
//! them: nothing here rounds, rescales or reprojects (§5, §23).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::annotation::{DimensionStyleDef, TextStyleDef};
use crate::document::{Bounds, DrawingFont, DrawingUnit};
use crate::entity::{BlockDefinition, Entity, Vec2};
use crate::identity::EntityId;
use crate::layer::LineType;

/// Version of this boundary; the WASM module reports the one it was built with.
/// 2: `writeDxf` (`DxfWriteInput`), and the reader takes back KentOS's DXF data.
/// 3: dimensions are written as DXF dimensions (`dimensionValues`) and read back.
/// 4: `v1Identities`, the persistent ids of a v1 drawing's objects (`identity`, docs/adr/0014).
/// 5: `encodeKcad`, `decodeKcad`: the binary `.kcad` v2 (docs/specs/kcad-v2.md, docs/adr/0025).
/// 6: the drawing crosses as typed columns (`kentos_kcad::columns`) with progress, not as JSON (docs/adr/0030).
/// 7: GeoJSON read and write, Shapefile read (`readGeoJson`, `writeGeoJson`, `readShapefile`);
///    an import says the coordinate system its file declares (`declaredCrs`, docs/adr/0046).
/// 8: Netcad NCZ (`NczReadOptions`, `CrsSource::Ncz`, docs/adr/0138); DXF and NCZ read in modules
///    of their own, loaded when such a file is imported, reporting their progress, their objects
///    crossing as typed columns; a layer's objects by kind and their box (`ImportLayer.kinds`,
///    `ImportLayer.bounds`) and where the view shows an import (`ImportResult.view`).
/// 9: an object's own line weight (`EntityBase.line_weight`, docs/adr/0139): DXF's group 370 and
///    an NCZ's pen read into it, written back as 370.
/// 10: vertex elevations (docs/adr/0142): `.kcad` document schema 4 and the typed columns' layout
///    (`kentos_kcad::columns`): a line's `za` and `zb`, a path's and a hole's `zs`, NaN in the
///    columns for a vertex without an elevation.
/// 11: multi-part areas (docs/adr/0143): `.kcad` document schema 5 and the typed columns' layout,
///    a polygon's `parts` after its holes, each part's flags, ring, bulges, elevations and holes.
/// 12: blocks (docs/adr/0144): `.kcad` document schema 6, the definitions in the drawing's JSON
///    and the `insert` kind in the typed columns (kind 13: p, scale, rotation, the block's id as
///    text, mirror a flag).
/// 13: DXF blocks kept (docs/adr/0144 §5): `ImportResult.blocks`, the definitions an import's
///    inserts place, and `DxfReadOptions.explode_blocks` (Blokları patlat).
/// 14: DXF blocks written (docs/adr/0144 §5): `DxfWriteInput.blocks`, and the writers' objects
///    take inserts.
/// 15: GeoJSON writes an insert as its block's objects placed (docs/adr/0144 §5):
///    `GeoJsonWriteInput.blocks`.
/// 16: a text's alignment, width factor and mask (docs/adr/0145): `.kcad` document schema 7,
///    the text's optional fields in the typed columns (align an int, width factor a float, mask
///    a flag) and an attribute definition's alignment and width factor in the drawing's JSON.
/// 17: the leader (docs/adr/0146): `.kcad` document schema 8, a new kind in the typed columns
///    (its vertices, height and turn; the note a text, the arrowhead an int, the mask a flag).
/// 18: the new dimensions (docs/adr/0147): `.kcad` document schema 9, five more dimension styles
///    in the typed columns' numbering and the dimension's mask (a flag), `za` and `zb` (floats).
/// 19: the field book (docs/adr/0169): `readFieldCsv`, `FieldCsvOptions` → `FieldBookRead`.
/// 20: Leica GSI (`readFieldGsi`); the book's angle unit, a station's coordinates.
/// 21: one entry for field books (`readFieldBook`), the book's format.
/// 22: Sokkia SDR2x and SDR33 field books (`readFieldBook`, format `sdr`).
/// 23: Topcon GTS-7 field books (`gts7`).
/// 24: Nikon RAW field books (`nikon`).
/// 25: Trimble JobXML field books (`jobxml`), XML read by roxmltree.
/// 26: GNSS files, GPX 1.1 and NMEA 0183 (`readGnss` → `GnssRead`).
/// 27: instrument coordinate files written (`writeField`: `FieldPoint`s and
///    `FieldWriteOptions` → `FieldWrite`).
/// 28: multi-part polylines and points (docs/adr/0174): `.kcad` document schema 17 and the typed
///    columns' layout, a polyline's `parts` as an area's, a point's `parts` (each its flags, place
///    and elevation).
/// 29: linked texts (docs/adr/0175 §4): `.kcad` document schema 18 and the typed columns' layout,
///    a text's `labelOf` (as its text) and `labelScale` behind a fourth option flag.
/// 30: multi-line texts (docs/adr/0182): `.kcad` document schema 20 and the typed columns' layout,
///    a text's box width, line spacing and runs behind its fifth to seventh option flags; MTEXT read
///    as one text with its line breaks, box, spacing and letter formats, written back as MTEXT.
/// 31: text and dimension styles (docs/adr/0183): `.kcad` document schema 21 and the typed
///    columns' layout, a text's style, typeface, bold, italic and slant, a dimension's style and
///    look behind their next option flags; DXF STYLE and DIMSTYLE read as the project's styles,
///    the import result's `textStyles` and `dimensionStyles`, written back as records.
/// 32: tables (docs/adr/0184): `.kcad` document schema 22 and the typed columns' `table` kind;
///    a table written to DXF as an anonymous block and its INSERT, read back as the table; the
///    formats module's table files (`readTableFile`: CSV, TXT and XLSX rows).
/// 33: hatch patterns (docs/adr/0186): `.kcad` document schema 23 and the typed columns' hatch
///    name, scale, families, gradient and tie behind their next option flags; DXF HATCH's
///    pattern lines read whole and written turned and scaled, its gradient both ways.
/// 34: pictures (docs/adr/0192): `.kcad` document schema 24 and the typed columns' `image`
///    kind; DXF and GeoJSON leave pictures out and say so.
/// 35: texts along a curve (docs/adr/0196): `.kcad` document schema 25 and the typed columns'
///    text curve; DXF writes one as an anonymous block of its letters with its KENTOS data.
/// 36: a layer's fields (docs/adr/0199): `.kcad` document schema 26 and the layer node's
///    `fields`; Shapefile's DBF fields and GeoJSON's properties read as the new layer's fields,
///    a field layer's numbers and yes or no values written to GeoJSON as JSON numbers and booleans.
/// 37: topology rules (docs/adr/0202): `.kcad` document schema 27 and the settings' `topology`.
/// 38: network adjustment (docs/adr/0203): `.kcad` document schema 28, the survey settings' a
///    priori standard deviations.
/// 39: raster layers (docs/adr/0204): `.kcad` document schema 29 and the typed columns' `raster`
///    kind; DXF and GeoJSON leave rasters out and say so.
/// 40: annotation heights and lines (docs/adr/0205): `.kcad` document schema 30, the settings'
///    `annotation`, a dimension's and a dimension style's line fields, a leader's `arrowSize` and
///    AutoCAD's arrowheads; the columns' dimension line flags and leader arrowhead size.
/// 41: point clouds (docs/adr/0207): `.kcad` document schema 31, the typed columns'
///    `pointcloud` kind and a raster's `url`; DXF and GeoJSON leave point clouds out and say so.
/// 42: map services (docs/adr/0208): `.kcad` document schema 32, a layer's `service` and `feed`
///    and the settings' `connections`.
/// 43: networks (docs/adr/0209): `.kcad` document schema 33, the settings' `networks`.
/// 44: temporal layers and scenarios (docs/adr/0210): `.kcad` document schema 34, a layer's `time`,
///    a group's `scenario` and a scenario layer's `replaces`.
/// 45: layer filters (docs/adr/0211): `.kcad` document schema 35, a layer's `filter`.
/// 46: the label engine (docs/adr/0212): `.kcad` document schema 36, a label style's engine
///    fields, a layer style's `labels`, an object's `labelPins` (the typed columns' `PINS` flag).
/// 47: the project's variables (docs/adr/0214 §2.3): `.kcad` document schema 37, the settings'
///    `variables`.
/// 48: mesh and multidimensional data (docs/adr/0243): `.kcad` document schema 38, a raster's
///    `dataset` and its look's `edges` (the typed columns' `LOOK_EDGES` bit and raster `OPT[4]`).
/// 49: KML/KMZ and GPX (docs/adr/0222): `readKml` (`CrsSource::Kml`), `writeKml`
///    (`KmlWriteInput`), `writeGpx` (`GpxWriteInput`), an import moved into the project's system
///    (`reprojectImport`), GeoJSON written in WGS 84 (`writeGeoJsonWgs84`) and `GnssRead.lines`.
/// 50: table exchange (docs/adr/0223): `readTableFile` reads ODS and a workbook's dates, within
///    İşlemler's limits when asked; `writeWorkbook` (`WorkbookWriteInput`) writes XLSX, ODS and CSV.
/// 51: vertex measures (docs/adr/0224 §1): `.kcad` document schema 39, a line's `ma` and `mb`, a
///    polyline's and its parts' `ms` (the typed columns' line and path measure flags); Shapefile's
///    PolyLineM and PolyLineZ read with their measures; DXF, GeoJSON, KML and GPX leave them out
///    and say so.
/// 52: object attachments, layer actions and information cards (docs/adr/0225): `.kcad` document
///    schema 40, an object's `attachments` (the typed columns' `ATTACHMENTS` flag), a layer's
///    `actions` and `card`; `readExif` reads a photo's position, time and direction.
/// 53: layer compositions (docs/adr/0226 §3, §4): `.kcad` document schema 41, a layer's `compose`
///    (opacity, blending mode, the mask of its labels and markers).
/// 54: address data (docs/adr/0229 §2): `.kcad` document schema 42, the settings'
///    `addressLocators` (a project's doors', streets' and areas' layers and fields). The web's copy
///    (`apps/web/src/io/version.ts`) stays 53 while the web is paused (docs/adr/0228).
/// 55: associative dimensions (docs/adr/0249 §4): `.kcad` document schema 43, a dimension's `refs`
///    (the places on objects its points `a`, `b` and `c` follow; the typed columns' dimension
///    `REFS` flag). The web's copy stays 53 while the web is paused.
/// 56: the project's sheet book (docs/adr/0251): `.kcad` document schema 44, the document's
///    `sheets` (the sheet core's book, opaque, and its pictures' bytes under their SHA-256; in the
///    snapshot's JSON the bytes are base64). The web's copy stays 53 while the web is paused.
/// 57: the request ledger (docs/adr/0253 §6): `.kcad` document schema 45, the document's
///    `requests` (the requests a drawing executed: id, fingerprint, command, version, revision,
///    status and the answer then). The web's copy stays 53 while the web is paused.
/// 58: a DXF's unit (docs/adr/0257 §1): the reader's `fileUnit` (the unit its values are in, the
///    user's or the caller's) and the import's `unit` (`UnitDecision`: what the file declares, the
///    unit read in and why, the blocks in another unit). The web's copy stays 53.
/// 59: closed and wide polylines (docs/adr/0258): `.kcad` document schema 46, a polyline's
///    `closed` and `widths` (the typed columns' path bits 13 and 14); the DXF reader's
///    `closedPolylines` (a closed LWPOLYLINE or POLYLINE as an area or as a closed line). The
///    web's copy stays 53.
/// 60 (docs/adr/0259): the NURBS curve (`nurbs`), the typed columns' kind 19; the DXF reader keeps a
///    SPLINE with control points as one exactly, its fit points with it. The web's copy stays 53.
/// 61 (docs/adr/0260): an object's look in the typed columns (flag 64: line type, scale, transparency,
///    order, by-layer parts); DXF's 6, 48 and 440 of an object and 290 of a layer read and written. The
///    web's copy stays 53.
/// 62 (docs/adr/0263): a dimension reference's `along` place (kind 6, its t a float) and block path (the
///    kind plus 8, then the path) in the typed columns. The web's copy stays 53.
/// 63 (docs/adr/0300): associative arrays: `.kcad` document schema 53, the document's `arrays` (a
///    block, its first place, the array command's layout and one item per place: its insert's id,
///    none when removed, `kept` when changed by hand). The web's copy stays 53.
/// 64 (docs/adr/0310): an area's mask: `.kcad` document schema 54, the look's `wipeout` (`{ frame }`), in the
///    typed columns the look's bits 32 and 64. The web's copy stays 53.
/// 65 (docs/adr/0313): line types with patterns: `.kcad` document schema 56, the settings' `lineTypes` and
///    `lineTypeScale`, a layer style's and an object's `linePattern`; in the typed columns the look's bit 128 and
///    its name. The web's copy stays 53.
/// 66 (docs/adr/0315): an insert's own y factor and a block's unit: `.kcad` document schema 57, the insert's
///    `yScale` and the definition's `units`; in the typed columns the insert's option bit 1 and its float after the
///    turn. The web's copy stays 53.
/// 67 (docs/adr/0317): attribute options, moved attributes and block cards: `.kcad` document schema 58, an attribute
///    definition's `invisible`, `constant`, `verify`, `preset`, `movable`, `multiline`, `boxWidth` and `field`, an
///    insert's `attributePlaces` and a definition's `card`; in the typed columns the insert's option bit 2, a count
///    and each tag and point. The web's copy stays 53.
/// 68 (docs/adr/0318): dynamic blocks: `.kcad` document schema 59, a definition's `dynamic` (its parameters and
///    actions as their JSON) and an insert's `parameters`; in the typed columns the insert's option bit 3 and the
///    values' JSON text. The web's copy stays 53.
/// 69 (docs/adr/0320 §1): a dimension's references' `rule` (`curve`, `edges`): `.kcad` document schema 60; in the
///    typed columns the references' mask bits 3 and 4. The web's copy stays 53.
/// 70 (docs/adr/0321): a dimension's and a dimension style's `options` and a dimension's `textShift` and `breaks`:
///    `.kcad` document schema 61; in the typed columns the dimension's bit 29 and their JSON text. The web's copy
///    stays 53.
/// 71 (docs/adr/0322): a dimension's kinds `centerMark`, `centerLine` and `elevation` and its references' rule `pair`:
///    `.kcad` document schema 62; in the typed columns the kinds 10–12 and both of the mask's rule bits. The web's copy
///    stays 53.
/// 72 (docs/adr/0322 §5): a text's and a dimension's `overrides`: `.kcad` document schema 63; in the typed columns
///    the text's option 14 and its JSON text, the dimension's in its more JSON (bit 29). The web's copy stays 53.
/// 73 (docs/adr/0323): a text's, a dimension's and a leader's `annotative`: `.kcad` document schema 64; in the typed
///    columns the text's option 15, the dimension's in its more JSON (bit 29), the leader's bit 2, each its JSON text.
///    The web's copy stays 53.
/// 74 (docs/adr/0324): a multi-line text's `layout` (indents, tab stops, columns) and a run's `stack` script:
///    `.kcad` document schema 65; in the typed columns the text's option 16 (its JSON text) and a run's flags 8 and 16
///    together. The web's copy stays 53.
/// 75–77 (docs/adr/0325–0327): a text's `fields`, a leader's arms and blocks, a table's cells: `.kcad` document
///    schemas 66–68, each its JSON text in the typed columns. The web's copy stays 53.
/// 78 (docs/adr/0328): a hatch's exact `boundary` and a face's tie (`bounds`, no `outer`): `.kcad` document schema
///    69; in the typed columns the hatch's option bit 6 and one JSON text, a face's tie with an empty outer id. The
///    web's copy stays 53.
/// 79 (docs/adr/0329 §3): a face's tie's `gap`: `.kcad` document schema 70; in the typed columns inside the hatch's
///    option 6 JSON. The web's copy stays 53.
/// 80 (docs/adr/0331): a pattern's `origin` and a hatch's `annotative`: `.kcad` document schema 71; in the typed
///    columns inside the hatch's option 6 JSON. The web's copy stays 53.
/// 81 (docs/adr/0410): a record without a geometry: `.kcad` document schema 72's `record` kind; in the typed
///    columns kind 20, with no fields of its own. The web's copy stays 53.
/// 82 (docs/adr/0339): the wall: `.kcad` document schema 75's `wall` kind; in the typed columns kind 21, its path's
///    points and bulges, the rest of its source as one JSON text. The web's copy stays 53.
/// 83 (docs/adr/0340): a wall's `joins` and `cleanup` and a component's `priority`: `.kcad` document schema 76; in the
///    typed columns inside the wall's JSON text. The web's copy stays 53.
/// 84 (docs/adr/0341): the opening: `.kcad` document schema 77's `opening` kind; in the typed columns kind 22, its
///    jambs' points, the rest of its source as one JSON text. The web's copy stays 53.
/// 85 (docs/adr/0342): the space: `.kcad` document schema 78's `space` kind and the settings' `arch.spaceMethod`; in
///    the typed columns kind 23, its source as one JSON text. The web's copy stays 53.
/// 86 (docs/adr/0343): the grid, column, stair and ramp: `.kcad` document schema 79's `grid`, `column`, `stair` and
///    `ramp` kinds; in the typed columns kinds 24–27, each's source as one JSON text. The web's copy stays 53.
/// 87 (docs/adr/0344): the tag and the schedule: `.kcad` document schema 80's `tag` kind and a table's `schedule`
///    source; in the typed columns kind 28, its source as one JSON text. The web's copy stays 53.
/// 88 (docs/adr/0345): the view mark: `.kcad` document schema 81's `viewmark` kind and a named view's `id`; in the typed
///    columns kind 29, its fields as one JSON text. The web's copy stays 53.
/// 89 (docs/adr/0346): the settings' `arch.phaseView`: `.kcad` document schema 82. The web's copy stays 53.
/// 90 (docs/adr/0348): a block definition's `xref` (an external reference's source): `.kcad` document schema 83; not in
///    the typed columns (definitions are not). The web's copy stays 53.
/// 91 (docs/adr/0349 §1): the settings' `xrefPaths`: `.kcad` document schema 84. The web's copy stays 53.
pub const FORMATS_VERSION: u32 = 91;

// ── Every import ────────────────────────────────────────────────────────

/// A layer as the source file defines it. Objects name it in `layerId`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImportLayer {
    pub name: String,
    /// Hex colour or a theme token (DXF colour 7 → "ink").
    pub color: String,
    pub visible: bool,
    pub locked: bool,
    pub line_type: LineType,
    /// The file's line type with a pattern the layer draws with, by its name (docs/adr/0313 §6), one of the result's
    /// `line_types`; `line_type` its nearest built-in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub line_pattern: Option<String>,
    /// Plot line weight in mm, when the file gives one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub line_weight: Option<f64>,
    /// Yazdır (docs/adr/0260 §4): `false` when the file's layer is not printed (DXF's 290).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub plot: Option<bool>,
    /// Dondur (docs/adr/0311 §1): `true` when the file's layer is frozen (DXF's 70 bit 1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub frozen: Option<bool>,
    /// Objects read onto this layer.
    pub count: u32,
    /// Those objects by kind (`point`, `line`, …): what a choice of layers
    /// brings, counted without walking the objects (docs/adr/0138).
    #[serde(default)]
    pub kinds: BTreeMap<String, u32>,
    /// The box those objects' defining points span (a text's is its
    /// insertion point): where a choice of layers is, without walking them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub bounds: Option<Bounds>,
    /// The fields the file gives the layer's attributes (docs/adr/0199 §6): a
    /// Shapefile's DBF fields, the kinds of a GeoJSON's properties; a new
    /// layer takes them. Empty: none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(
        feature = "ts",
        ts(as = "Option<Vec<crate::fields::LayerField>>", optional)
    )]
    pub fields: Vec<crate::fields::LayerField>,
}

/// One line of an import or export report: what, how many, and what happened to it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReportItem {
    /// The source's name for it ("IMAGE", "Genişlikli çoklu çizgi").
    pub what: String,
    pub count: u32,
    /// What happened and, where it helps, what to do (Turkish, for the user).
    pub reason: String,
    /// The first source lines where it occurs.
    pub lines: Vec<u32>,
}

/// A fact about the source file, shown before the import ("Sürüm": "AutoCAD 2000").
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SourceFact {
    pub label: String,
    pub value: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImportReport {
    /// Objects read, by kind (`point`, `line`, …).
    pub counts: BTreeMap<String, u32>,
    /// Not imported, with the reason.
    pub skipped: Vec<ReportItem>,
    /// Imported with a change the user should know about (a conversion, an approximation).
    pub notes: Vec<ReportItem>,
    pub source: Vec<SourceFact>,
}

/// Where a file's statement of its coordinate system comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CrsSource {
    /// A GeoJSON file without a `crs` member: RFC 7946 makes it WGS 84 longitude, latitude.
    Rfc7946,
    /// A GeoJSON file's legacy `crs` member (the 2008 specification).
    GeoJsonCrs,
    /// A Shapefile's `.prj` (WKT).
    Prj,
    /// A Netcad NCZ drawing's MPROJ block (datum, projection, zone) and the
    /// SRS its TILED_XML block names (docs/adr/0138).
    Ncz,
    /// A KML or KMZ file: OGC KML 2.2 makes it WGS 84 longitude, latitude (docs/adr/0222).
    Kml,
}

/// The coordinate system a file declares: what the file says, never a
/// guess (CLAUDE.md §5). The import window shows it as the answer to its
/// question, and the user confirms or changes it; nothing is transformed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DeclaredCrs {
    /// The EPSG code the statement names, when it names one the reader recognises.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub srid: Option<u32>,
    /// The statement as the window shows it ("urn:ogc:def:crs:EPSG::5256", "TUREF_TM36").
    pub text: String,
    pub source: CrsSource,
}

/// What a reader produced. Objects have id 0 (the app numbers them when it
/// adds them) and `layerId` holds the name of their layer in `layers`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImportResult {
    pub entities: Vec<Entity>,
    pub layers: Vec<ImportLayer>,
    pub report: ImportReport,
    /// Extent of the objects' defining points (shown before the import).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub bounds: Option<Bounds>,
    /// The coordinate system the file declares (GeoJSON, Shapefile; docs/adr/0046).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub declared_crs: Option<DeclaredCrs>,
    /// Where the view shows the objects once they are in: their extent without
    /// the far strays a file can hold (a slip drawn at 0, 0 beside a city,
    /// `kentos_formats::import::view_bounds`; docs/adr/0138).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub view: Option<Bounds>,
    /// The block definitions the objects' inserts place (a DXF's BLOCKs,
    /// docs/adr/0144 §5), in the file's order: ids the reader numbered (the
    /// app gives them new ones and a name the drawing does not have yet),
    /// their objects on layer "0". None when blocks were exploded.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<BlockDefinition>>", optional))]
    pub blocks: Vec<BlockDefinition>,
    /// The text and dimension styles the objects follow (a DXF's STYLE and
    /// DIMSTYLE records, docs/adr/0183 §7), in the file's order: ids the
    /// reader numbered, their sizes in paper mm at the scale the options
    /// gave. The app takes a style of a name the project has as that one,
    /// and gives the others new ids and names the project does not have yet.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<TextStyleDef>>", optional))]
    pub text_styles: Vec<TextStyleDef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<DimensionStyleDef>>", optional))]
    pub dimension_styles: Vec<DimensionStyleDef>,
    /// The unit the file's lengths were read in and why (a DXF's,
    /// docs/adr/0257 §1); none for a format without units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub unit: Option<UnitDecision>,
    /// The file's line types with patterns (a DXF's LTYPE table, docs/adr/0313 §6), lengths in the file's unit as read.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<crate::LineTypeDef>>", optional))]
    pub line_types: Vec<crate::LineTypeDef>,
    /// The file's line type scale ($LTSCALE), when other than 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub line_type_scale: Option<f64>,
}

// ── Coordinate lists (Netcad NCN, TXT, CSV) ─────────────────────────────

/// What a column of a coordinate list holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CoordColumn {
    /// Point name (becomes the label and the `Ad` attribute).
    Name,
    /// Y, to the right (east): the model's x.
    Y,
    /// X, up (north): the model's y.
    X,
    /// Elevation.
    Z,
    /// Point code or description (the `Kod` attribute).
    Code,
    /// Not read.
    Skip,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CoordDelimiter {
    #[default]
    Auto,
    Tab,
    Semicolon,
    Comma,
    /// One or more spaces (Netcad NCN).
    Space,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DecimalMark {
    #[default]
    Auto,
    Point,
    /// A decimal comma (Turkish spreadsheets); only with a delimiter other than the comma.
    Comma,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum HeaderMode {
    #[default]
    Auto,
    Yes,
    No,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CoordReadOptions {
    pub delimiter: CoordDelimiter,
    pub decimal: DecimalMark,
    pub header: HeaderMode,
    /// What each column holds, in file order; empty lets the reader suggest.
    pub columns: Vec<CoordColumn>,
    /// Rows for the preview table.
    pub preview_rows: u32,
    /// Whether to build the objects (the dialog asks for them only when importing).
    pub entities: bool,
}

/// A row of the preview: the fields as written and, if the row is not a point, why.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CoordRow {
    pub line: u32,
    pub fields: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LineError {
    pub line: u32,
    pub message: String,
}

/// A plain-text field book's columns as the user maps them (0-based; the
/// station, its instrument height and the optional values may be left out)
/// and whether its first line names them (docs/adr/0169 §1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FieldCsvOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub station: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub instrument_height: Option<u32>,
    pub target: u32,
    pub hz: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub zenith: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub slope: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub target_height: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub code: Option<u32>,
    pub header: bool,
}

/// An observation as the instrument wrote it (docs/adr/0169 §2): the
/// target, the horizontal reading, the zenith angle (none: a direction
/// only), the slope distance, the target height, a code, the file's line.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FieldObservation {
    pub target: String,
    pub hz: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub zenith: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub slope: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub target_height: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub code: Option<String>,
    pub line: u32,
}

/// A station of the field book: its name (empty when the file names none),
/// the instrument's height above it, its coordinates when the file gives
/// them (east, north, height; Leica GSI's WI 84–86), its observations in order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FieldStation {
    pub station: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub instrument_height: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub east: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub north: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub height: Option<f64>,
    pub observations: Vec<FieldObservation>,
}

/// A field book read: its format (`gsi`, told by the content; else `csv`, a
/// text book), the text's encoding, the book's angle unit when the file says
/// it (`grad`, `deg`; a CSV book's is the user's), the first line's cells
/// (the columns' names to map), its stations, and the lines not read with why.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FieldBookRead {
    pub format: String,
    pub encoding: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub unit: Option<String>,
    pub first_line: Vec<String>,
    pub stations: Vec<FieldStation>,
    pub problems: Vec<LineError>,
}

/// A GNSS position as the file wrote it (docs/adr/0169 §1, §6): its kind
/// (`wpt`, `rtept`, `trkpt` of GPX; `gga` of NMEA), its name, its WGS 84
/// latitude and longitude (degrees), its height as written (GPX's `ele`;
/// GGA's above mean sea level), the geoid's height above the WGS 84
/// ellipsoid, the ellipsoidal height (the two summed, when both are
/// given), its time as the file gives it, its fix (named: `3B`, `RTK
/// sabit` …), satellites in use, HDOP, and the file's line.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GnssPoint {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    pub lat: f64,
    pub lon: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub height: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub geoid: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub ellipsoidal: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub time: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub satellites: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub hdop: Option<f64>,
    pub line: u32,
}

/// A GNSS file read (docs/adr/0169 §1, §6): its format (`gpx`, `nmea`),
/// the text's encoding, its positions in order, the points or sentences
/// not read with why.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GnssRead {
    pub format: String,
    pub encoding: String,
    pub points: Vec<GnssPoint>,
    pub problems: Vec<LineError>,
    /// The file's routes and tracks (docs/adr/0222 §6): their points' places in `points`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<GnssLine>>", optional))]
    pub lines: Vec<GnssLine>,
}

/// A route or a track of a GNSS file (docs/adr/0222 §6): GPX's `rte` (one
/// part) or `trk` (a part per `trkseg`), or an NMEA log's positions in order
/// (`nmea`, one part); its name when the file gives one; each part its points
/// as their places in `GnssRead.points` (a point not read is not there); the
/// file's line.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GnssLine {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    pub parts: Vec<Vec<u32>>,
    pub line: u32,
}

/// A point sent to an instrument (docs/adr/0169 §4): its name, east, north
/// (metres), elevation and code when it has them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FieldPoint {
    pub name: String,
    pub east: f64,
    pub north: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub elevation: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub code: Option<String>,
}

/// An instrument's coordinate file (docs/adr/0169 §4): Leica GSI-16 and
/// GSI-8, Topcon GTS-7 points, Trimble JobXML, Nikon RAW, CSV.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum FieldWriteFormat {
    Gsi16,
    Gsi8,
    Gts7,
    Jobxml,
    Nikon,
    Csv,
}

/// How the points are written: the format, the job's name (JobXML's
/// jobName, Nikon RAW's comment) and the time stamp JobXML carries
/// (`2026-10-04T10:15:00`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FieldWriteOptions {
    pub format: FieldWriteFormat,
    pub job: String,
    pub stamp: String,
}

/// A point the format cannot carry: its place among the points (from 1),
/// its name and why it is not written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FieldSkip {
    pub index: u32,
    pub name: String,
    pub problem: String,
}

/// An instrument's coordinate file written: its text, how many points went
/// in, and those that did not with why; or why nothing was written (the
/// job's name).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FieldWrite {
    pub text: String,
    pub written: u32,
    pub skipped: Vec<FieldSkip>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub error: Option<String>,
}

/// A coordinate list read with the given options, and what the reader found.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CoordRead {
    /// Text encoding of the file ("UTF-8", "Windows-1254 (Türkçe)").
    pub encoding: String,
    /// The delimiter and decimal mark in use (never `auto`).
    pub delimiter: CoordDelimiter,
    pub decimal: DecimalMark,
    /// Whether the first data line is a header, and its fields.
    pub header: bool,
    pub header_fields: Vec<String>,
    /// What each column holds, in use (as given, or suggested).
    pub columns: Vec<CoordColumn>,
    pub preview: Vec<CoordRow>,
    /// Lines that are neither blank nor comments (header included).
    pub data_lines: u32,
    pub points: u32,
    /// The first rows that are not points, with the reason.
    pub errors: Vec<LineError>,
    pub error_count: u32,
    /// Points whose name another point already has.
    pub duplicate_names: u32,
    /// Things worth checking before the import (Turkish).
    pub hints: Vec<String>,
    /// Extent of the points (Y as x, X as y), to show where the data lies before the import.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub bounds: Option<Bounds>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub result: Option<ImportResult>,
}

/// A point to write into a coordinate list.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CoordPoint {
    pub name: String,
    pub p: Vec2,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub z: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub code: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum TextEncoding {
    #[default]
    Utf8,
    /// Windows-1254 (Turkish); other characters are left out and reported.
    Windows1254,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CoordWriteInput {
    pub points: Vec<CoordPoint>,
    /// Not `auto`.
    pub delimiter: CoordDelimiter,
    /// Column order (`skip` is not written).
    pub columns: Vec<CoordColumn>,
    /// A first line with the column names.
    pub header: bool,
    pub encoding: TextEncoding,
}

// ── DXF ─────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DxfReadOptions {
    /// Stop after this many objects (0: one million); the rest is counted and reported.
    pub max_entities: u32,
    /// Blokları patlat: every insert opened into its objects, no definitions
    /// kept (docs/adr/0144 §5). Off, a block is a definition and an insert
    /// places it; the inserts a block cannot hold are opened all the same.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub explode_blocks: bool,
    /// A local project's drawing unit (docs/adr/0165 §2): the file's values
    /// are taken in the unit its $INSUNITS names and turned into metres, a
    /// file that names none is in this unit. None: a project with a
    /// coordinate system, whose files are read in metres as they are.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub unit: Option<DrawingUnit>,
    /// The unit the file's values are in, chosen by the user or the caller
    /// (docs/adr/0257 §1): whatever the file declares, its values are taken
    /// in it and turned into metres. None: the rule of `unit`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub file_unit: Option<FileUnit>,
    /// The project's plot scale denominator (docs/adr/0183 §7): a style's
    /// heights and sizes, in drawing units in the file, are paper mm at it;
    /// none: 1000.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub scale: Option<f64>,
    /// The project's typeface: a style whose typeface KentOS does not have
    /// takes it (said); none: Barlow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing_font: Option<DrawingFont>,
    /// What a closed LWPOLYLINE or POLYLINE becomes unless KentOS marked it a
    /// closed line (docs/adr/0258 §1): an area (the default) or a closed
    /// line. One with too few vertices for an area is a closed line either way.
    #[serde(default, skip_serializing_if = "ClosedPolylines::is_area")]
    #[cfg_attr(feature = "ts", ts(as = "Option<ClosedPolylines>", optional))]
    pub closed_polylines: ClosedPolylines,
}

/// What a file's closed polyline becomes (docs/adr/0258 §1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ClosedPolylines {
    /// An area: what a GIS project wants of a parcel's boundary.
    #[default]
    Area,
    /// A closed line: a CAD drawing's polyline stays a line.
    Line,
}

impl ClosedPolylines {
    pub fn is_area(&self) -> bool {
        *self == ClosedPolylines::Area
    }
}

/// A unit a file's lengths may be read in, whatever the file says
/// (docs/adr/0257 §1): DXF's `$INSUNITS` most used.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum FileUnit {
    Mm,
    Cm,
    Dm,
    M,
    Km,
    /// The international inch (25.4 mm).
    In,
    /// The international foot (0.3048 m).
    Ft,
    Yd,
    Mi,
    /// The US survey foot (1200/3937 m).
    UsFt,
}

impl FileUnit {
    pub const ALL: [FileUnit; 10] = [
        FileUnit::Mm,
        FileUnit::Cm,
        FileUnit::Dm,
        FileUnit::M,
        FileUnit::Km,
        FileUnit::In,
        FileUnit::Ft,
        FileUnit::Yd,
        FileUnit::Mi,
        FileUnit::UsFt,
    ];

    /// Its `$INSUNITS` code.
    pub fn code(self) -> i64 {
        match self {
            FileUnit::In => 1,
            FileUnit::Ft => 2,
            FileUnit::Mi => 3,
            FileUnit::Mm => 4,
            FileUnit::Cm => 5,
            FileUnit::M => 6,
            FileUnit::Km => 7,
            FileUnit::Yd => 10,
            FileUnit::Dm => 14,
            FileUnit::UsFt => 21,
        }
    }

    /// The unit of a `$INSUNITS` code; none for another.
    pub fn of_code(code: i64) -> Option<FileUnit> {
        Self::ALL.into_iter().find(|u| u.code() == code)
    }

    /// A local project's drawing unit as a file unit.
    pub fn of_drawing(u: DrawingUnit) -> FileUnit {
        match u {
            DrawingUnit::Mm => FileUnit::Mm,
            DrawingUnit::Cm => FileUnit::Cm,
            DrawingUnit::M => FileUnit::M,
            DrawingUnit::In => FileUnit::In,
            DrawingUnit::Ft => FileUnit::Ft,
        }
    }

    /// Its mark: `mm`, `in`, `ft (ABD)`.
    pub fn mark(self) -> &'static str {
        match self {
            FileUnit::Mm => "mm",
            FileUnit::Cm => "cm",
            FileUnit::Dm => "dm",
            FileUnit::M => "m",
            FileUnit::Km => "km",
            FileUnit::In => "in",
            FileUnit::Ft => "ft",
            FileUnit::Yd => "yd",
            FileUnit::Mi => "mi",
            FileUnit::UsFt => "ft (ABD)",
        }
    }
}

/// Why a file's lengths were read in the unit they were (docs/adr/0257 §1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum UnitSource {
    /// The user or the caller chose it (`fileUnit`).
    Chosen,
    /// The file declares it, and a local project takes it.
    Declared,
    /// A project with a coordinate system reads metres.
    Metres,
    /// The file declares none, or a code no one defines: a local project's
    /// own unit, the numbers kept.
    Project,
}

/// A block whose record names another unit than the file's (DXF
/// BLOCK_RECORD's group 70): its inserts' scale holds the ratio
/// (docs/adr/0257 §2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BlockUnit {
    pub name: String,
    /// Its `$INSUNITS` code.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "insunits")))]
    pub code: i64,
}

/// How a file's lengths were read (docs/adr/0257 §1): what the file
/// declares, the unit the values were taken in and why, and the blocks
/// whose unit is another.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct UnitDecision {
    /// The `$INSUNITS` code the file declares; none for 0 or no header.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "insunits")))]
    pub declared: Option<i64>,
    /// The `$INSUNITS` code of the unit the values were read in.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "insunits")))]
    pub used: i64,
    /// Its name: “milimetre”, “inç”.
    pub name: String,
    pub source: UnitSource,
    /// Metres in one of `used`: the values were multiplied by it.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub factor: f64,
    /// Neither declared nor chosen: the project's unit (metres with a
    /// coordinate system) taken, the numbers kept.
    pub assumed: bool,
    /// The blocks whose unit is another than the file's, in the file's order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<BlockUnit>>", optional))]
    pub blocks: Vec<BlockUnit>,
}

impl UnitDecision {
    /// What a command says of the decision (docs/adr/0257 §1, §2), as
    /// `(code, message)`: `unit_assumed`, `unit_ignored`,
    /// `block_units_differ`.
    pub fn warnings(&self) -> Vec<(&'static str, String)> {
        let mut out = Vec::new();
        let named = |code: i64| crate::units::insunits(code).map(|(_, name)| name);
        if self.assumed {
            let why = match self.declared {
                None => "Dosya birim bildirmiyor".to_owned(),
                Some(code) => format!("Dosyanın bildirdiği birim kodu ({code}) tanınmıyor"),
            };
            out.push((
                crate::codes::UNIT_ASSUMED,
                format!(
                    "{why}; değerler ölçeklenmeden {} sayıldı. Dosyanın birimi başkaysa onu seçin (fileUnit).",
                    if self.source == UnitSource::Metres {
                        "metre".to_owned()
                    } else {
                        format!("çizimin biriminde ({})", self.name)
                    }
                ),
            ));
        }
        if self.source == UnitSource::Metres
            && let Some(code) = self.declared
            && code != 6
            && let Some(name) = named(code)
        {
            out.push((
                crate::codes::UNIT_IGNORED,
                format!(
                    "Dosya birimini {name} olarak bildiriyor; koordinat sistemi olan proje dosyayı metre okur, değerler ölçeklenmedi. Ölçeklemek için dosyanın birimini seçin (fileUnit)."
                ),
            ));
        }
        if !self.blocks.is_empty() {
            let listed: Vec<String> = self
                .blocks
                .iter()
                .map(|b| {
                    format!(
                        "{} ({})",
                        b.name,
                        named(b.code).unwrap_or("bilinmeyen birim")
                    )
                })
                .collect();
            out.push((
                crate::codes::BLOCK_UNITS_DIFFER,
                format!(
                    "Bazı blokların birimi dosyanınkinden başka: {}. Yerleştirmelerin ölçeği bu oranı taşır; bloklar ayrıca ölçeklenmedi.",
                    listed.join(", ")
                ),
            ));
        }
        out
    }
}

/// A layer as the DXF writer receives it. DXF layers are flat and their
/// names ignore case: the writer makes each name valid and unique (the
/// group path tells layers of the same name apart) and reports what it changed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DxfWriteLayer {
    /// What the objects' `layerId` holds.
    pub id: String,
    pub name: String,
    /// Names of the groups above the layer, outermost first.
    pub path: Vec<String>,
    /// Hex colour or a theme token ("ink" is DXF colour 7).
    pub color: String,
    /// As the drawing shows it (inherited from the groups): a hidden layer is written off.
    pub visible: bool,
    pub locked: bool,
    pub line_type: LineType,
    /// The project line type it draws with (docs/adr/0313 §6), one of the input's `line_types`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub line_pattern: Option<String>,
    /// Plot line weight in mm (written as AutoCAD's nearest weight).
    pub line_weight: f64,
    /// Yazdır (docs/adr/0260 §4): absent, printed; `false`, written with
    /// DXF's plot switch off (290).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub plot: Option<bool>,
    /// Dondur (docs/adr/0311 §1): `true`, written frozen (DXF's 70 bit 1);
    /// `visible` is then its eye alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub frozen: Option<bool>,
}

/// What `file.export.dxf` writes (an AutoCAD 2007 DXF).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DxfWriteInput {
    pub entities: Vec<Entity>,
    pub layers: Vec<DxfWriteLayer>,
    /// Plot scale denominator (1000 for 1:1000): line type patterns and point marks are sized for paper at it.
    pub scale: f64,
    /// Decimals the project shows lengths with ($LUPREC).
    pub length_decimals: u32,
    /// Angles are shown in grads, else in degrees ($AUNITS).
    pub grads: bool,
    /// What each dimension without a text of its own shows, by object id:
    /// the measured value as the app formats it (project units). The
    /// dimension's block draws it; the DXF dimension keeps no text, so a
    /// program that redraws it measures again.
    #[serde(default)]
    pub dimension_values: BTreeMap<u32, String>,
    /// The drawing's block definitions (docs/adr/0144 §5): those the
    /// objects' inserts place, and those nested in them, are written as
    /// BLOCKs, inner ones first; the others are left out.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<BlockDefinition>>", optional))]
    pub blocks: Vec<BlockDefinition>,
    /// A local project's drawing unit (docs/adr/0165 §2): coordinates and
    /// lengths are written in it and $INSUNITS names it. None: metres.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub unit: Option<DrawingUnit>,
    /// The project's text and dimension styles (docs/adr/0183 §7): each is a
    /// STYLE or DIMSTYLE record, the objects that follow one name it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<TextStyleDef>>", optional))]
    pub text_styles: Vec<TextStyleDef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<DimensionStyleDef>>", optional))]
    pub dimension_styles: Vec<DimensionStyleDef>,
    /// The project's line types (docs/adr/0313 §6): each an LTYPE record, the layers and objects that draw with one
    /// name it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<crate::LineTypeDef>>", optional))]
    pub line_types: Vec<crate::LineTypeDef>,
    /// The project's line type scale ($LTSCALE); absent: 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub line_type_scale: Option<f64>,
    /// How tables, dimensions and leaders are written (docs/adr/0327 §6); absent: native entities with KentOS's data.
    #[serde(default, skip_serializing_if = "DxfAnnotations::is_default")]
    #[cfg_attr(feature = "ts", ts(as = "Option<DxfAnnotations>", optional))]
    pub annotations: DxfAnnotations,
    /// What the caller made of the drawing for DXF, said in the report beside the writer's own notes: a dynamic
    /// block's inserts written as the plain blocks they draw (docs/adr/0319 §5).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ReportItem>>", optional))]
    pub notes: Vec<ReportItem>,
    /// The objects' persistent ids by object id (docs/adr/0333 §3): an associative hatch is written with its boundary
    /// objects' handles (71 1, 97 and 330s) and they with its reactor, when they are written too. Absent: hatches
    /// are written follows nothing (71 0).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<BTreeMap<u32, EntityId>>", optional))]
    pub uids: BTreeMap<u32, EntityId>,
    /// The detail walls are drawn at (docs/adr/0339 §4): the project's own; absent: by the scale.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub wall_detail: Option<crate::WallDetail>,
}

/// What a dimension or a leader becomes in DXF (docs/adr/0327 §6).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DxfForm {
    /// Its own DXF entity (DIMENSION; LEADER and MTEXT): another program edits it as one.
    #[default]
    Native,
    /// Its look as an anonymous block's lines, arrowheads and texts, placed by an INSERT: every program shows it as
    /// KentOS draws it, none edits it as a dimension or a leader.
    Block,
}

/// How tables, dimensions and leaders are written in DXF (docs/adr/0327 §6). A table is always its look as a block
/// (DXF's ACAD_TABLE is not written); KentOS's data, when written, gives each back to KentOS whole.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DxfAnnotations {
    #[serde(default)]
    pub dimensions: DxfForm,
    #[serde(default)]
    pub leaders: DxfForm,
    /// KentOS's data on them (the table's cells and styles, the dimension's and the leader's own fields).
    #[serde(default = "yes")]
    pub metadata: bool,
}

fn yes() -> bool {
    true
}

impl Default for DxfAnnotations {
    fn default() -> Self {
        DxfAnnotations {
            dimensions: DxfForm::Native,
            leaders: DxfForm::Native,
            metadata: true,
        }
    }
}

impl DxfAnnotations {
    pub fn is_default(&self) -> bool {
        *self == DxfAnnotations::default()
    }
}

/// What a writer did besides writing: counts, and anything it could not write as it was.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExportReport {
    pub counts: BTreeMap<String, u32>,
    pub notes: Vec<ReportItem>,
    pub skipped: Vec<ReportItem>,
}

// ── Netcad NCZ (docs/adr/0138) ──────────────────────────────────────────

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NczReadOptions {
    /// Stop after this many objects (0: one million); the rest is counted and reported.
    pub max_entities: u32,
    /// The drawing's typeface (`ProjectSettings.drawingFont`; empty: the
    /// default). A smart object's centred text is placed by its width in the
    /// face the drawing shows it in: the app's texts sit on their lower left.
    #[serde(default)]
    pub drawing_font: String,
}

// ── GeoJSON (RFC 7946) and Shapefile (docs/adr/0046) ───────────────────

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GeoJsonReadOptions {
    /// The layer of the objects the file does not place: the document's
    /// `name` member when it has one, else this (the file's name).
    pub layer: String,
    /// Stop after this many objects (0: one million); the rest is counted and reported.
    pub max_entities: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ShapefileReadOptions {
    /// The layer of the objects (the .shp file's name).
    pub layer: String,
    /// Stop after this many objects (0: one million); the rest is counted and reported.
    pub max_entities: u32,
}

/// A layer the GeoJSON writer names in each feature's `kentos.layer`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GeoJsonLayer {
    /// What the objects' `layerId` holds.
    pub id: String,
    pub name: String,
    /// The layer's fields (docs/adr/0199 §6): a number's and a yes or no
    /// value's attribute is written as a JSON number or boolean.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(
        feature = "ts",
        ts(as = "Option<Vec<crate::fields::LayerField>>", optional)
    )]
    pub fields: Vec<crate::fields::LayerField>,
}

/// What `file.export.geojson` writes: a FeatureCollection. A WGS 84
/// project (EPSG:4326) is written as RFC 7946 has it; any other in its own
/// coordinate system with the legacy `crs` member naming it, which RFC
/// 7946 does not allow (the export window says so). Nothing is transformed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GeoJsonWriteInput {
    pub entities: Vec<Entity>,
    pub layers: Vec<GeoJsonLayer>,
    /// The project's coordinate system (EPSG code).
    pub srid: u32,
    /// The collection's `name` (the drawing's).
    pub name: String,
    /// The drawing's block definitions (docs/adr/0144 §5): an insert is
    /// written as its block's objects placed, one GeometryCollection.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<BlockDefinition>>", optional))]
    pub blocks: Vec<BlockDefinition>,
}

// ── KML/KMZ and GPX (docs/adr/0222) ─────────────────────────────────────

/// A layer as KML writes it (docs/adr/0222 §4): what the objects' `layerId`
/// holds, its name, the groups it sits in (outermost first; a Folder each),
/// its colour (hex) and plot line weight (mm).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct KmlLayer {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub path: Vec<String>,
    pub color: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub line_weight: Option<f64>,
}

/// What `file.export.kml` writes (docs/adr/0222 §4): the objects in WGS 84
/// (the writer is given the project's system and datum choices), a Folder
/// per layer, a Placemark per object; a KMZ archive (`doc.kml` in it) when
/// `kmz`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct KmlWriteInput {
    pub entities: Vec<Entity>,
    pub layers: Vec<KmlLayer>,
    /// The Document's name (the drawing's).
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<BlockDefinition>>", optional))]
    pub blocks: Vec<BlockDefinition>,
    #[serde(default)]
    pub kmz: bool,
}

/// What `file.export.gpx` writes (docs/adr/0222 §5): points as waypoints,
/// lines as tracks or (`routes`) routes, areas' outlines as closed tracks, in
/// WGS 84 (the writer is given the project's system and datum choices).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GpxWriteInput {
    pub entities: Vec<Entity>,
    /// The metadata's name (the drawing's).
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<BlockDefinition>>", optional))]
    pub blocks: Vec<BlockDefinition>,
    #[serde(default)]
    pub routes: bool,
}
