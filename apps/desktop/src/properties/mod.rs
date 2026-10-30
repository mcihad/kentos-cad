//! Öznitelikler: the web's properties panel (`ui/properties/PropertiesPanel.ts`
//! on `ui/widgets/PropertyGrid.ts`), editable as the web's (docs/adr/0063).
//!
//! - Nothing selected: “Seçili nesne yok”, how to select, and the drawing's
//!   summary. One object: its kind, label and layer over the grid, “#id” in
//!   the header; Genel, Geometri and its attributes. Several: how many of
//!   which kinds, “N nesne”; what they share and their totals.
//! - A text or number cell looks like text until clicked: Enter or leaving
//!   it commits, Esc keeps the value (KentOS UI's `EditCell`). A value the
//!   panel does not take (an empty text, not a number, a height not above
//!   zero) leaves the drawing as it was, and the cell shows its value again.
//! - Katman ▾, Renk ▾ and Sembol ▾ are drop-downs; an object on a locked
//!   layer is not edited, its values still named. Several objects holding
//!   locked ones: the others are edited, the locked skipped and counted, a
//!   note first says how many (docs/adr/0278, 0286); Ortak öznitelikler
//!   shows every key, shared or Çeşitli, and writes one value to them all.
//! - Every edit goes through a product command, as the web's panel writes
//!   (docs/adr/0066): the layer, colour and attributes through
//!   `cad.entities.set`, a geometry value through `cad.entities.edit`'s
//!   `properties`. One undo step with the web's name: “Katman değiştir”,
//!   “Renk değiştir”, else “Değiştir”. Nothing is said but the command's
//!   refusal or warning (objects moved to a hidden layer).
//! - Sections close on their header and stay closed while the app runs.
//!
//! The rows are data (`rows`), drawn by KentOS UI's `PropertySheet`.

pub(crate) mod rows;

pub(crate) use crate::annotation_styles::Part;
pub(crate) use rows::layer_path;
#[cfg(test)]
mod tests;

use iced::widget::{Column, column, container, row, space};
use iced::{Center, Color, Element, Fill, Theme};
use std::cell::RefCell;
use std::collections::BTreeMap;

use kentos_commands::{
    BlockId, DimensionEntity, EntitiesSetProperties, Entity, LeaderEntity, PropertiesOperation,
};
use kentos_domain::Slot;
use kentos_interaction::elevation::{self, Change};
use kentos_interaction::properties;
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::widget::property_grid::{self, PropertySheet};
use kentos_ui::widget::{EditCell, Menu, swatch};

pub(crate) use rows::{Choice, Editor, Panel, Row, Summary};

use crate::app::{App, Message};
use crate::document::Document;

/// A change asked for in the panel.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// A section's header: closes or opens it.
    Toggle(&'static str),
    /// What a text or number cell committed.
    Commit(Field, String),
    /// Katman ▾: the objects to the layer.
    Layer(Vec<Slot>, String),
    /// Renk ▾: the objects' colour; none, the layer's.
    Color(Vec<Slot>, Option<String>),
    /// Kalınlık ▾: the objects' own line weight in mm; none, the layer's (docs/adr/0139).
    Weight(Vec<Slot>, Option<f64>),
    /// Çizgi tipi ▾, Çizgi tipi ölçeği ▾, Saydamlık ▾ (docs/adr/0260 §2, §3):
    /// the objects' own; none, their source's (a scale of 1).
    LineType(Vec<Slot>, Option<kentos_commands::LineType>),
    /// A project line type with a pattern by its name (docs/adr/0313 §7); none: none.
    LinePattern(Vec<Slot>, Option<String>),
    LineTypeScale(Vec<Slot>, Option<f64>),
    Transparency(Vec<Slot>, Option<f64>),
    /// Maske ▾ of areas (docs/adr/0310 §5): a mask with its frame or without, or an ordinary area.
    Wipeout(Vec<Slot>, Option<kentos_commands::Wipeout>),
    /// Desen ▾ of a hatch: one of Tarama's choices (docs/adr/0186 §7), its
    /// Ölçek and Açı carried over.
    Pattern(Slot, usize),
    /// Degrade biçimi ▾, Ters ▾ and İkinci renk ▾ of a gradient hatch.
    GradientShape(Slot, kentos_commands::GradientShape),
    GradientInverted(Slot, bool),
    GradientColour(Slot, String),
    /// Blok ▾ of an insert: the block it places (docs/adr/0144).
    InsertBlock(Slot, BlockId),
    /// Aynalı ▾ of an insert.
    InsertMirror(Slot, bool),
    /// A dynamic block's insert's value chosen (docs/adr/0319 §3): a flip, a visibility state, a lookup row.
    Parameter(Slot, String, kentos_commands::ParamValue),
    /// Değerler ▾ › Hepsini varsayılana döndür.
    ParametersReset(Slot),
    /// Kaynak ▾'s Göm of a linked picture: its file read and kept in the
    /// project's library, the picture embedded (docs/adr/0192 §2).
    EmbedImage(Slot),
    /// Kaynak ▾'s Göm of a linked raster (docs/adr/0204 §8).
    EmbedRaster(Slot),
    /// A wall's own field chosen (Hiza, Stil, Uçlar, Kat, Evre): written through `cad.walls.edit` (docs/adr/0339 §6).
    Wall(Slot, kentos_commands::WallChange),
    /// A wall's joins (docs/adr/0340 §6).
    WallOp(Slot, kentos_commands::WallsOperation),
    /// An opening's own field chosen, its type replaced (docs/adr/0341 §5): written through `cad.openings.edit`.
    Opening(Slot, kentos_commands::OpeningsOperation),
    /// A space's own field chosen (Yöntem, Kat, Evre) or its boundary found again (docs/adr/0342 §5): written through
    /// `cad.spaces.edit`.
    Space(Slot, kentos_commands::SpacesOperation),
    /// A slab's own field chosen (Tür, Kenar, Kat, Evre, Metraj; docs/adr/0722 §6): written through `cad.slabs.edit`.
    Slab(Slot, kentos_commands::SlabsOperation),
    SpaceSeparator(Slot, bool),
    /// Kat ▾ of several architectural objects (docs/adr/0717 §2): through `cad.levels.assign`.
    Levels(Vec<Slot>, String),
    /// A grid's, column's, stair's or ramp's own field chosen, a column freed (docs/adr/0343 §5): written through
    /// `cad.arch.edit`.
    Arch(Slot, kentos_commands::ArchOperation),
    /// A tag's frame or leader (docs/adr/0344 §5), through `cad.tags.edit`.
    Tag(Slot, kentos_commands::TagChange),
    /// A view mark's side or target (docs/adr/0345 §5), through `cad.viewmarks.edit`.
    ViewMark(Slot, kentos_commands::ViewMarksOperation),
    /// Hiza ▾ of texts: each keeps where it is (docs/adr/0145 §6); none, the left of the baseline.
    TextAlign(Vec<Slot>, Option<kentos_commands::TextAlign>),
    /// Zemin ▾ of texts.
    TextMask(Vec<Slot>, bool),
    /// Kapalı ▾ of a polyline: a closed line, no area (docs/adr/0258 §1).
    PolylineClosed(Vec<Slot>, bool),
    /// Ok ▾ of leaders (none: the filled arrow) and their Zemin ▾ (docs/adr/0146 §7).
    LeaderArrow(Vec<Slot>, Option<kentos_commands::LeaderArrow>),
    LeaderMask(Vec<Slot>, bool),
    /// Zemin ▾ of dimensions, and Koordinat ▾ of ordinates: the axis's
    /// angle, 0 for Y and 90 for X (docs/adr/0147 §7).
    DimensionMask(Vec<Slot>, bool),
    DimensionAxis(Vec<Slot>, f64),
    /// İlişki ▾'s Serbest bırak: the dimensions' points let go (docs/adr/0249 §3).
    DimensionFree(Vec<Slot>),
    /// A live wall dimension chain's selection, layout or recipe action.
    DimensionSet(Slot, &'static str),
    DimensionSetValue(Slot, &'static str, String),
    /// A kitchen run's action (docs/adr/0727 §6): Dizinin tamamını seç, Diziyi bırak, Dizinin tamamını sil.
    Casework(Slot, &'static str),
    /// A kitchen run's value: the object's module's width, door or kind, or the run's corner, wall cabinets, oven or
    /// fridge, through `cad.casework.edit`.
    CaseworkValue(Slot, &'static str, String),
    /// A dimension edit through `cad.dimensions.edit` (docs/adr/0321 §4): its options, its value home, its breaks off.
    DimensionEdit(Vec<Slot>, DimensionOp),
    /// Stile döndür (docs/adr/0322 §5): the objects take their style's values again.
    RestoreStyle(Vec<Slot>),
    /// Ölçekli (docs/adr/0323 §4): the annotations made annotative (true) or one height again.
    Annotative(Vec<Slot>, bool),
    /// Ayrı değerler › Sıfırla: nothing set apart at their other scales.
    AnnotativeReset(Vec<Slot>),
    /// A dimension's line's colour (`#RRGGBB`; none: the object's), weight
    /// on paper (mm; none: a hairline) and type (none: continuous): the
    /// dimension line's and its arrowheads', the extension lines', the
    /// value's colour (docs/adr/0205 §6).
    DimensionLineColor(Vec<Slot>, Part, Option<String>),
    DimensionLineWeight(Vec<Slot>, Part, Option<f64>),
    DimensionLineType(Vec<Slot>, Part, Option<kentos_commands::LineType>),
    /// Bağlı nesne ▾ of a linked text: Nesneyi seç selects its object,
    /// Bağı kopar breaks the texts' links (docs/adr/0175 §4).
    SelectObject(Slot),
    Unlink(Vec<Slot>),
    /// A hatch's Sınır ▾: its boundary written again as an area or polylines, the hatch then following it
    /// (docs/adr/0329 §4).
    RecreateBoundary(Slot, kentos_commands::BoundaryForm),
    /// Yazı stili ▾ and Ölçü stili ▾ (docs/adr/0183 §6): the style applied
    /// to the objects; none, Standart.
    TextStyle(Vec<Slot>, Option<String>),
    DimensionStyle(Vec<Slot>, Option<String>),
}

/// A grid's, column's, stair's or ramp's field a cell edits (docs/adr/0343 §5): words, a length in the project's
/// unit, degrees, a count, a per cent or paper millimetres; empty takes a nullable one away.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchField {
    Label,
    Tag,
    Type,
    BubbleSize,
    Width,
    Depth,
    Turn,
    Risers,
    RiserHeight,
    Tread,
    Landing,
    Rise,
    Slope,
    Base,
    /// A ramp's start over its level's (docs/adr/0723 §2); empty frees it from its level.
    BaseOffset,
    /// A curved stair's centre post, its diameter (docs/adr/0724 §3); empty removes it.
    Post,
    /// A gapped grid's ends on paper, mm (docs/adr/0725 §5); empty or 0 the whole grid.
    Ends,
    /// An L column's legs (docs/adr/0725 §3): the one along its width's thickness, the one along its depth's.
    LegA,
    LegB,
    /// A tag's template (docs/adr/0344 §5): `\n` its line breaks; empty its kind's.
    TagContent,
    /// A tag's words' height on paper, mm; empty 2.5.
    TagSize,
    /// A view mark's number (docs/adr/0345 §5); empty: its sheet frame's order.
    MarkNumber,
    /// A view mark's words' height on paper, mm; empty 2.5.
    MarkSize,
}

/// The value a cell edits.
#[derive(Debug, Clone, PartialEq)]
pub enum Field {
    /// Ölçekler (docs/adr/0323 §4): the annotations' other scales, typed.
    AnnotativeScales(Vec<Slot>),
    /// A point's Y (east) and X (north).
    PointX(Slot),
    PointY(Slot),
    DimensionOffset(Slot),
    DimensionSetGap(Slot),
    /// A dimension recipe's sheet scales (docs/adr/0730 §1), typed as Diğer ölçekler.
    DimensionSetScales(Slot),
    /// A kitchen run's countertop depth (docs/adr/0727 §6), in the drawing's unit.
    CaseworkDepth(Slot),
    DimensionHeight(Slot),
    /// A dimension's own text; empty, the measured value again.
    DimensionText(Slot),
    HatchAngle(Slot),
    HatchSpacing(Slot),
    /// A pattern's scale on the paper (1: the library's) (docs/adr/0186 §7).
    HatchScale(Slot),
    /// A text object's text, height and angle.
    Text(Slot),
    TextHeight(Slot),
    TextAngle(Slot),
    /// An attribute, by its key.
    Attribute(Slot, String),
    /// A dynamic block's insert's typed value, by its parameter's name (docs/adr/0319 §3).
    Parameter(Slot, String),
    /// An attribute of several objects, by its key (Ortak öznitelikler, docs/adr/0286).
    Attributes(Vec<Slot>, String),
    /// A line's start or end elevation, or every vertex's of a polyline or an
    /// area: a number sets it, an empty text clears it (docs/adr/0142).
    Elevation(Slot, Spot),
    /// A polyline's width, every edge's, in the project's length; 0 or an
    /// empty text: none (docs/adr/0258 §2).
    PolylineWidth(Slot),
    /// Every vertex's elevation of these lines, polylines, areas and points.
    Elevations(Vec<Slot>),
    /// Texts' width factor: out of its range the command says why (docs/adr/0145).
    TextWidth(Vec<Slot>),
    /// Texts' box width (emptied or “Kutusuz”: none) and line spacing (docs/adr/0182 §4).
    TextBox(Vec<Slot>),
    TextSpacing(Vec<Slot>),
    /// A multi-line text's indents, tab stops and columns (docs/adr/0324 §3).
    TextIndent(Vec<Slot>),
    TextLeftIndent(Vec<Slot>),
    TextTabs(Vec<Slot>),
    TextColumns(Vec<Slot>),
    TextColumnHeight(Vec<Slot>),
    TextColumnGap(Vec<Slot>),
    /// An insert's place (Y, X), scale (above zero) and turn (degrees typed).
    InsertX(Slot),
    InsertY(Slot),
    InsertScale(Slot),
    /// An insert's own y factor (docs/adr/0315 §3).
    InsertYScale(Slot),
    InsertTurn(Slot),
    /// A picture's place (Y, X), width and height (each keeping its shape),
    /// turn (degrees typed) and see-through share (0 to 90 %; docs/adr/0192 §4).
    ImageX(Slot),
    ImageY(Slot),
    ImageWidth(Slot),
    ImageHeight(Slot),
    ImageTurn(Slot),
    ImageClear(Slot),
    /// A raster's see-through share (0 to 90 %; docs/adr/0204 §8).
    RasterClear(Slot),
    /// A wall's width, offset and height (docs/adr/0339 §6); an empty height, the level's.
    WallWidth(Slot),
    WallOffset(Slot),
    WallHeight(Slot),
    /// An opening's width, place along its wall, swing (degrees), frame, sill, head (empty: none) and number
    /// (docs/adr/0341 §5).
    OpeningWidth(Slot),
    OpeningAt(Slot),
    OpeningSwing(Slot),
    OpeningFrame(Slot),
    OpeningSill(Slot),
    OpeningHead(Slot),
    /// A niche's or flue's depth and a flue's distance from its face (empty: centred; docs/adr/0719).
    OpeningDepth(Slot),
    OpeningInset(Slot),
    OpeningNumber(Slot),
    /// A space's number, name, usage, height (empty: the level's) and finishes (empty: none; docs/adr/0342 §5).
    SpaceNumber(Slot),
    SpaceName(Slot),
    SpaceUsage(Slot),
    SpaceHeight(Slot),
    SpaceFloor(Slot),
    SpaceWall(Slot),
    SpaceCeiling(Slot),
    /// A slab's thickness, top's offset, edge's width (lengths) and material (empty: none; docs/adr/0722 §6).
    SlabThickness(Slot),
    SlabOffset(Slot),
    SlabEdgeWidth(Slot),
    SlabMaterial(Slot),
    /// A grid's, column's, stair's or ramp's typed field (docs/adr/0343 §5).
    Arch(Slot, ArchField),
    /// A point cloud's see-through share (0 to 90 %; docs/adr/0207 §9).
    CloudClear(Slot),
    /// Leaders' note (trimmed; emptied, the arrow alone), height (above zero)
    /// and turn (degrees) (docs/adr/0146 §7).
    LeaderNote(Vec<Slot>),
    LeaderHeight(Vec<Slot>),
    LeaderTurn(Vec<Slot>),
    /// Leaders' arrowhead size in the note's height, 0.1 to 10; 1 is no
    /// field (docs/adr/0205 §7).
    LeaderArrowSize(Vec<Slot>),
    /// Slopes' first and second elevation, metres (docs/adr/0147 §7).
    DimensionZa(Vec<Slot>),
    DimensionZb(Vec<Slot>),
    /// Dimensions' increment, and their tolerance's upper (true) or lower value, metres (docs/adr/0321 §2).
    DimensionRound(Vec<Slot>),
    DimensionTolerance(Vec<Slot>, bool),
    /// Kots' offset from their level, metres (docs/adr/0721 §2): Fark.
    DimensionLevelOffset(Vec<Slot>),
}

/// What Öznitelikler's dimension rows ask of `cad.dimensions.edit` (docs/adr/0321 §4).
#[derive(Debug, Clone, PartialEq)]
pub enum DimensionOp {
    /// The options as they become (none: none).
    Options(Option<kentos_commands::DimensionOptions>),
    /// The value back in its place.
    Home,
    /// The lines whole again.
    Unbreak,
    /// Kots bound to a level, or set free (none) keeping their value (docs/adr/0721 §2).
    Level(Option<kentos_commands::DimensionLevel>),
}

/// Which vertices of an object a Kot cell sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Spot {
    /// A line's start.
    Start,
    /// A line's end.
    End,
    /// Every vertex, a polyline's or an area's (its holes' too).
    All,
}

/// JavaScript's `parseFloat(text.replace(',', '.'))`, as the web reads the
/// panel's numbers: the first comma is the decimal point, the longest number
/// at the start is taken (“12abc” is 12), none is NaN.
pub(crate) fn web_number(text: &str) -> f64 {
    js_parse_float(&text.replacen(',', ".", 1))
}

/// JavaScript's `parseFloat`: white space at the start skipped, the longest
/// number at the start taken (“12abc” is 12, “12,5” is 12), none is NaN.
pub(crate) fn js_parse_float(text: &str) -> f64 {
    let text = kentos_interaction::js_trim(text);
    let bytes = text.as_bytes();
    let mut end = 0;
    if matches!(bytes.first(), Some(b'+' | b'-')) {
        end = 1;
    }
    if text[end..].starts_with("Infinity") {
        return text[..end + "Infinity".len()]
            .replace("Infinity", "inf")
            .parse()
            .unwrap_or(f64::NAN);
    }
    let digits = |from: usize| {
        bytes[from..]
            .iter()
            .take_while(|b| b.is_ascii_digit())
            .count()
    };
    let whole = digits(end);
    end += whole;
    let mut fraction = 0;
    if bytes.get(end) == Some(&b'.') {
        fraction = digits(end + 1);
        if whole > 0 || fraction > 0 {
            end += 1 + fraction;
        }
    }
    if whole == 0 && fraction == 0 {
        return f64::NAN;
    }
    if matches!(bytes.get(end), Some(b'e' | b'E')) {
        let sign = usize::from(matches!(bytes.get(end + 1), Some(b'+' | b'-')));
        let exponent = digits(end + 1 + sign);
        if exponent > 0 {
            end += 1 + sign + exponent;
        }
    }
    text[..end].parse().unwrap_or(f64::NAN)
}

/// The panel last worked out for several selected objects, and what it was
/// worked out from: the drawing, its changes and the selection. A large
/// selection's kinds, shared values and totals are gathered once, not in
/// every frame the pointer moves (docs/perf, the desktop's frame).
#[derive(Default)]
pub(crate) struct PanelCache(RefCell<Option<(PanelKey, Panel)>>);

/// The open drawing (its session), its changes and the selection's version.
type PanelKey = (u64, u64, u64);

impl App {
    /// The panel's content for the selection now.
    pub(crate) fn properties_panel(&self, doc: &Document) -> Panel {
        // An empty panel includes live source/file state. A single complex polyline/block is
        // expensive too: cache its geometric rows just like a large multi-selection.
        if self.selection.is_empty() {
            return self.work_out_panel(doc);
        }
        let key = (
            doc.session,
            doc.model.generation(),
            self.selection.version(),
        );
        let mut cache = self.properties_cache.0.borrow_mut();
        if let Some((known, panel)) = cache.as_ref()
            && *known == key
        {
            let mut panel = panel.clone();
            self.add_live_sections(&mut panel);
            return panel;
        }
        let panel = self.work_out_panel(doc);
        *cache = Some((key, panel.clone()));
        let mut panel = panel;
        self.add_live_sections(&mut panel);
        panel
    }

    fn work_out_panel(&self, doc: &Document) -> Panel {
        let selected: Vec<Slot> = self.selection.ids().to_vec();
        // The program's own rows (module_surfaces.rs).
        let shown = rows::Shown {
            crs: self.shows_property_rows(crate::module_surfaces::PropertyRows::Crs),
            widths: self.offers_tool_option(kentos_interaction::ToolOption::Widths),
            mask: self.shows_property_rows(crate::module_surfaces::PropertyRows::Mask),
            separators: self.shows_property_rows(crate::module_surfaces::PropertyRows::Separators),
        };
        let mut panel = rows::panel(
            doc,
            &selected,
            |ids| {
                // Summed by the geometry store, as the web sums them: a selection can be large.
                let ids: Vec<f64> = ids.iter().map(|s| f64::from(s.0)).collect();
                self.spatial.store().measure(&ids)
            },
            shown,
        );
        // Nothing selected: the layer chosen in the tree (else the active one), when a service draws or fed it; a
        // map's programs' (module_surfaces.rs).
        if selected.is_empty()
            && self.shows_property_rows(crate::module_surfaces::PropertyRows::Services)
        {
            let layers = doc.model.layers();
            let id = self.selected_layer.as_deref().unwrap_or(layers.active());
            if let Some(node) = layers.get(id)
                && let Some(section) = rows::service_section(
                    doc,
                    node,
                    node.service.as_ref().and_then(|s| self.service_failure(s)),
                )
            {
                // The layer chosen is what the user is looking at: its section first.
                panel.sections.insert(0, section);
            }
        }
        panel
    }

    fn add_live_sections(&self, panel: &mut Panel) {
        // Attachment tasks may finish without editing geometry. Never freeze their actions in the cache.
        if let [slot] = self.selection.ids() {
            panel
                .sections
                .extend(rows::attachments::sections(self, *slot));
        }
    }

    /// The header's meta: “#12”, “3 nesne”; none with nothing selected.
    pub(crate) fn properties_meta(&self, doc: &Document) -> Option<String> {
        self.properties_panel(doc).meta
    }

    /// The panel's body: the summary or the empty state, then the grid.
    pub(crate) fn properties_view<'a>(&'a self, doc: &'a Document) -> Element<'a, Message> {
        let panel = self.properties_panel(doc);
        let head: Element<'a, Message> = match panel.summary {
            Summary::Empty => boxed(
                column![
                    label::strong("Seçili nesne yok"),
                    label::muted(
                        "Özelliklerini görmek için çizimde bir nesneye tıklayın. Birden fazla nesne için sürükleyerek seçin."
                    ),
                ]
                .spacing(4),
                [14, 14],
            ),
            Summary::One { kind, label, layer } => {
                let mut title = row![label::strong(kind)].spacing(8).align_y(Center);
                if let Some(text) = label {
                    title = title.push(label::body(text).style(accent_text));
                }
                let mut lines = column![title].spacing(3);
                if let Some((color, path)) = layer {
                    lines = lines.push(
                        row![swatch(self.drawing_color(&color)), label::caption(path)]
                            .spacing(6)
                            .align_y(Center),
                    );
                }
                boxed(lines, [9, 12])
            }
            Summary::Many { count, kinds } => boxed(
                column![
                    label::strong(format!("{count} nesne seçili")),
                    label::caption(kinds),
                ]
                .spacing(3),
                [9, 12],
            ),
        };
        let mut sheet = PropertySheet::new();
        for section in panel.sections {
            let open = !self.props_closed.contains(section.id);
            sheet = sheet.section(
                section.title,
                open,
                Message::Properties(Event::Toggle(section.id)),
            );
            if !open {
                continue;
            }
            for mut row in section.rows {
                if row.note && row.warn {
                    sheet = sheet.note(self.cell(row));
                    continue;
                }
                if doc.settings().workspace == Some(kentos_commands::Workspace::Cad)
                    && row.numeric
                    && row.editor.is_none()
                    && row.thumb.is_none()
                    && let Some(unit) = &row.unit
                    && !unit.is_empty()
                {
                    sheet = sheet.measurement(row.label.into_owned(), row.value, unit.to_string());
                    continue;
                }
                let label = row.label.clone();
                // An attachment's picture before its cell (docs/adr/0225 §2).
                let thumb = row.thumb.take();
                let cell = self.cell(row);
                let cell = match thumb {
                    Some(t) => kentos_ui::widget::Cell::flat(
                        iced::widget::row![
                            container(self.thumb(t)).padding(iced::Padding {
                                left: kentos_ui::theme::metrics::cell_end(),
                                right: 2.0,
                                ..iced::Padding::ZERO
                            }),
                            cell,
                        ]
                        .align_y(iced::Center),
                    )
                    .into(),
                    None => cell,
                };
                sheet = sheet.row(label, cell);
            }
        }
        Column::new().push(head).push(sheet).width(Fill).into()
    }

    /// An attachment's picture: its photo once decoded, else its kind's icon;
    /// pressed, it opens it; under the pointer, where it is.
    fn thumb<'a>(&'a self, t: rows::Thumb) -> Element<'a, Message> {
        const W: f32 = 36.0;
        const H: f32 = 27.0;
        let face: Element<'a, Message> = match self.photo_picture(t.key.as_deref()) {
            Some(picture) => {
                crate::photo_view::photo(Some(picture), W, H, crate::photo_view::Fit::Cover)
            }
            None => {
                container(kentos_ui::icon::icon(crate::icons::from_web(Some(t.icon))).size(16.0))
                    .width(W)
                    .height(H)
                    .center_x(W)
                    .center_y(H)
                    .style(kentos_ui::style::container::field)
                    .into()
            }
        };
        kentos_ui::widget::tip(
            iced::widget::button(face)
                .on_press(t.open)
                .padding(0)
                .style(kentos_ui::style::button::ghost),
            kentos_ui::widget::Tip::new(t.title),
            iced::widget::tooltip::Position::Top,
        )
    }

    /// A row's value: text, an editable cell or a drop-down.
    fn cell<'a>(&'a self, row: Row) -> Element<'a, Message> {
        let unit = row.unit.clone();
        if row.note {
            let tone = if row.warn {
                style::text::warning
            } else {
                style::text::muted
            };
            return label::caption(row.value).style(tone).into();
        }
        match row.editor {
            None => property_grid::value(row.value, row.numeric, unit.map(|u| u.into_owned())),
            Some(Editor::Text(field) | Editor::Number(field)) => {
                EditCell::new(row.value, move |text| {
                    Message::Properties(Event::Commit(field.clone(), text))
                })
                .numeric(row.numeric)
                .unit(unit.map(|u| u.into_owned()).unwrap_or_default())
                .into()
            }
            Some(Editor::Select {
                text,
                swatch,
                icon,
                items,
            }) => {
                // Colours resolved now: the menu is built again at every opening.
                let items: Vec<(Choice, Option<Color>)> = items
                    .into_iter()
                    .map(|choice| {
                        let color = match &choice {
                            Choice::Pick {
                                swatch: Some(value),
                                ..
                            } => Some(self.drawing_color(value)),
                            _ => None,
                        };
                        (choice, color)
                    })
                    .collect();
                match icon {
                    // A picture before the value (a text's alignment, docs/adr/0145).
                    Some(name) => property_grid::icon_choice(
                        text,
                        crate::icons::from_web(Some(name)),
                        move || menu(&items),
                    ),
                    None => property_grid::choice(
                        text,
                        swatch.map(|value| self.drawing_color(&value)),
                        move || menu(&items),
                    ),
                }
            }
        }
    }

    /// A drawing colour as the panel shows it (a layer's, one of the
    /// colours): the theme's ink colours by the drawing's palette, else the hex.
    pub(crate) fn drawing_color(&self, value: &str) -> Color {
        let palette = crate::viewport::palette(self.canvas());
        palette
            .resolve(value)
            .map_or_else(|| crate::view::hex_color(value), crate::view::rgba_color)
    }

    /// What the panel asked for: an edit of the drawing, through the
    /// product commands as the web's panel writes (docs/adr/0066), or a
    /// section toggled. The command's refusal and warnings are said.
    pub(crate) fn properties_event(&mut self, event: Event) {
        if let Event::Toggle(id) = event {
            if !self.props_closed.remove(id) {
                self.props_closed.insert(id);
            }
            return;
        }
        if let Event::SelectObject(slot) = event {
            self.selection.set([slot]);
            return;
        }
        if let Event::Casework(slot, "select") = event {
            let Some(model) = self.document.as_ref().map(|doc| &doc.model) else {
                return;
            };
            let Some((run, _)) = model.uid(slot).and_then(|uid| model.casework_member(uid)) else {
                return;
            };
            let slots: Vec<_> = run
                .members
                .iter()
                .filter_map(|m| model.slot_of(kentos_domain::Uuid::from_bytes(m.uid.0)))
                .collect();
            self.selection.set(slots);
            return;
        }
        if let Event::DimensionSet(slot, action @ ("select" | "row" | "source")) = event {
            let Some(model) = self.document.as_ref().map(|doc| &doc.model) else {
                return;
            };
            let Some((set, index)) = model
                .uid(slot)
                .and_then(|uid| model.dimension_set_of_item(uid))
            else {
                return;
            };
            let row = set.members[index].row;
            let slots: Vec<_> = if action == "source" {
                set.sources()
                    .iter()
                    .filter_map(|id| id.to_text().parse().ok())
                    .filter_map(|uid| model.slot_of(uid))
                    .collect()
            } else {
                set.members
                    .iter()
                    .filter(|member| action == "select" || member.row == row)
                    .filter_map(|member| {
                        member
                            .uid
                            .to_text()
                            .parse()
                            .ok()
                            .and_then(|uid| model.slot_of(uid))
                    })
                    .collect()
            };
            self.selection.set(slots);
            if action == "source" {
                self.zoom_selection();
            }
            return;
        }
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        // A linked picture's file is relative to the drawing's folder (docs/adr/0192 §2).
        let folder = doc
            .path
            .as_deref()
            .and_then(std::path::Path::parent)
            .map(std::path::Path::to_path_buf);
        let drawing = doc.path.clone();
        let model = &mut doc.model;
        let said = match event {
            Event::Toggle(_) | Event::SelectObject(_) => Vec::new(),
            Event::Unlink(ids) => {
                let mut input = set_input(model, &ids, PropertiesOperation::Unlink);
                input.unlink = true;
                properties::set_properties(model, input)
            }
            Event::RecreateBoundary(slot, form) => properties::recreate_boundary(model, slot, form),
            // On a hidden layer they vanish from the drawing, still selected: the command's warning says so.
            Event::Layer(ids, layer) => {
                let mut input = set_input(model, &ids, PropertiesOperation::Layer);
                input.layer_id = Some(layer);
                properties::set_properties(model, input)
            }
            Event::Color(ids, color) => {
                let mut input = set_input(model, &ids, PropertiesOperation::Color);
                input.color = Some(color);
                properties::set_properties(model, input)
            }
            Event::Weight(ids, weight) => {
                let mut input = set_input(model, &ids, PropertiesOperation::LineWeight);
                input.line_weight = Some(weight);
                properties::set_properties(model, input)
            }
            Event::LineType(ids, t) => {
                let mut input = set_input(model, &ids, PropertiesOperation::LineType);
                input.line_type = Some(t);
                properties::set_properties(model, input)
            }
            Event::LinePattern(ids, p) => {
                let mut input = set_input(model, &ids, PropertiesOperation::LineType);
                input.line_pattern = Some(p);
                properties::set_properties(model, input)
            }
            Event::LineTypeScale(ids, s) => {
                let mut input = set_input(model, &ids, PropertiesOperation::LineTypeScale);
                input.line_type_scale = Some(s);
                properties::set_properties(model, input)
            }
            Event::Transparency(ids, t) => {
                let mut input = set_input(model, &ids, PropertiesOperation::Transparency);
                input.transparency = Some(t);
                properties::set_properties(model, input)
            }
            Event::Wipeout(ids, w) => {
                let mut input = set_input(model, &ids, PropertiesOperation::Wipeout);
                input.wipeout = Some(w);
                properties::set_properties(model, input)
            }
            Event::Pattern(slot, choice) => match model.get(slot) {
                Some(Entity::Hatch(h)) => {
                    let plot = model.settings().plot_scale;
                    match kentos_interaction::hatch_options::rechosen(&h.pattern, choice, plot) {
                        Some(pattern) => {
                            let mut h = h.clone();
                            h.pattern = pattern;
                            properties::set_geometry(model, slot, &Entity::Hatch(h))
                        }
                        None => Vec::new(),
                    }
                }
                _ => Vec::new(),
            },
            Event::GradientShape(slot, shape) => match model.get(slot) {
                Some(Entity::Hatch(h)) if h.pattern.gradient.is_some() => {
                    let mut h = h.clone();
                    if let Some(g) = &mut h.pattern.gradient {
                        g.shape = shape;
                    }
                    properties::set_geometry(model, slot, &Entity::Hatch(h))
                }
                _ => Vec::new(),
            },
            Event::GradientColour(slot, colour) => match model.get(slot) {
                Some(Entity::Hatch(h)) if h.pattern.gradient.is_some() => {
                    let mut h = h.clone();
                    if let Some(g) = &mut h.pattern.gradient {
                        g.color2 = colour;
                    }
                    properties::set_geometry(model, slot, &Entity::Hatch(h))
                }
                _ => Vec::new(),
            },
            Event::GradientInverted(slot, inverted) => match model.get(slot) {
                Some(Entity::Hatch(h)) if h.pattern.gradient.is_some() => {
                    let mut h = h.clone();
                    if let Some(g) = &mut h.pattern.gradient {
                        g.inverted = inverted;
                    }
                    properties::set_geometry(model, slot, &Entity::Hatch(h))
                }
                _ => Vec::new(),
            },
            Event::InsertBlock(slot, block) => match model.get(slot) {
                Some(Entity::Insert(i)) => {
                    let mut i = i.clone();
                    i.block = block;
                    properties::set_geometry(model, slot, &Entity::Insert(i))
                }
                _ => Vec::new(),
            },
            Event::EmbedImage(slot) => crate::pictures::embed(model, slot, drawing.as_deref()),
            Event::EmbedRaster(slot) => crate::rasters::embed(model, slot, folder.as_deref()),
            Event::Wall(slot, change) => properties::edit_wall(model, slot, change),
            Event::WallOp(slot, op) => properties::edit_wall_op(model, slot, op),
            Event::Opening(slot, op) => properties::edit_opening(model, slot, op),
            Event::Space(slot, op) => properties::edit_space(model, slot, op),
            Event::Slab(slot, op) => properties::edit_slab(model, slot, op),
            Event::SpaceSeparator(slot, on) => {
                let arch = on.then(|| {
                    model
                        .get(slot)
                        .and_then(kentos_domain::space_follow::separator_path)
                        .and_then(|p| p.space_separator.clone())
                        .unwrap_or_default()
                });
                properties::set_space_separator(model, slot, arch)
            }
            Event::Arch(slot, op) => properties::edit_arch(model, slot, op),
            Event::Levels(slots, level) => properties::assign_level(model, &slots, &level),
            Event::Tag(slot, change) => properties::edit_tag(model, slot, change),
            Event::ViewMark(slot, op) => properties::edit_viewmark(model, slot, op),
            Event::Parameter(slot, name, value) => {
                properties::set_parameters(model, slot, Some(BTreeMap::from([(name, value)])), None)
            }
            Event::ParametersReset(slot) => properties::set_parameters(model, slot, None, None),
            Event::InsertMirror(slot, mirror) => match model.get(slot) {
                Some(Entity::Insert(i)) => {
                    let mut i = i.clone();
                    i.mirror = mirror;
                    properties::set_geometry(model, slot, &Entity::Insert(i))
                }
                _ => Vec::new(),
            },
            Event::TextAlign(slots, to) => properties::realign_texts(model, &slots, to),
            Event::TextMask(slots, on) => properties::set_text_mask(model, &slots, on),
            Event::PolylineClosed(slots, on) => properties::set_polyline_closed(model, &slots, on),
            Event::TextStyle(slots, id) => {
                properties::apply_text_style(model, &slots, id.as_deref())
            }
            Event::RestoreStyle(slots) => properties::restore_styles(model, &slots),
            Event::Annotative(slots, on) => kentos_interaction::annotative::run(
                model,
                &slots,
                if on {
                    kentos_commands::AnnotativeOperation::Make
                } else {
                    kentos_commands::AnnotativeOperation::Unmake
                },
                Vec::new(),
            ),
            Event::AnnotativeReset(slots) => kentos_interaction::annotative::run(
                model,
                &slots,
                kentos_commands::AnnotativeOperation::Reset,
                Vec::new(),
            ),
            Event::DimensionStyle(slots, id) => {
                properties::apply_dimension_style(model, &slots, id.as_deref())
            }
            Event::LeaderArrow(slots, arrow) => properties::change_leaders(model, &slots, |l| {
                (l.arrow != arrow).then(|| LeaderEntity { arrow, ..l.clone() })
            }),
            Event::LeaderMask(slots, mask) => properties::change_leaders(model, &slots, |l| {
                (l.mask != mask).then(|| LeaderEntity { mask, ..l.clone() })
            }),
            Event::DimensionMask(slots, mask) => {
                properties::change_dimensions(model, &slots, |d| {
                    (d.mask != mask).then(|| DimensionEntity { mask, ..d.clone() })
                })
            }
            Event::DimensionFree(slots) => properties::free_dimensions(model, &slots),
            Event::DimensionSet(slot, action) => dimension_set_action(model, slot, action),
            Event::Casework(slot, action) => casework_action(model, slot, action),
            Event::CaseworkValue(slot, field, value) => casework_value(model, slot, field, &value),
            Event::DimensionSetValue(slot, field, value) => {
                if matches!(field, "dimensionStyle" | "layerId" | "tight") {
                    let mut operation = serde_json::json!({"kind":"set"});
                    operation[field] = serde_json::json!(value);
                    dimension_set_edit(model, slot, operation)
                } else {
                    Vec::new()
                }
            }
            Event::DimensionEdit(slots, op) => {
                use kentos_commands::DimensionEdit as E;
                properties::edit_dimensions(model, &slots, |uids| match op {
                    DimensionOp::Options(options) => E::Options { uids, options },
                    DimensionOp::Unbreak => E::Unbreak { uids },
                    DimensionOp::Level(level) => E::Level { uids, level },
                    DimensionOp::Home => E::MoveText {
                        uid: uids[0].clone(),
                        along: 0.0,
                        across: 0.0,
                    },
                })
            }
            Event::DimensionAxis(slots, angle) => {
                let y = angle == 0.0;
                properties::change_dimensions(model, &slots, |d| {
                    (rows::is_y_axis(d.angle) != y).then(|| DimensionEntity {
                        angle: Some(angle),
                        ..d.clone()
                    })
                })
            }
            // The same colour whatever its case is left out.
            Event::DimensionLineColor(slots, part, color) => {
                properties::change_dimensions(model, &slots, |d| {
                    let mut look = d.look.clone();
                    let field = match part {
                        Part::Line => &mut look.dim_line_color,
                        Part::Ext => &mut look.ext_color,
                        Part::Value => &mut look.text_color,
                    };
                    let same = match (field.as_deref(), color.as_deref()) {
                        (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
                        (a, b) => a == b,
                    };
                    if same {
                        return None;
                    }
                    *field = color.clone();
                    Some(DimensionEntity { look, ..d.clone() })
                })
            }
            Event::DimensionLineWeight(slots, part, weight) => {
                properties::change_dimensions(model, &slots, |d| {
                    let mut look = d.look.clone();
                    let field = match part {
                        Part::Line => &mut look.dim_line_weight,
                        Part::Ext => &mut look.ext_weight,
                        Part::Value => return None,
                    };
                    if *field == weight {
                        return None;
                    }
                    *field = weight;
                    Some(DimensionEntity { look, ..d.clone() })
                })
            }
            Event::DimensionLineType(slots, part, kind) => {
                properties::change_dimensions(model, &slots, |d| {
                    let mut look = d.look.clone();
                    let field = match part {
                        Part::Line => &mut look.dim_line_type,
                        Part::Ext => &mut look.ext_line_type,
                        Part::Value => return None,
                    };
                    if *field == kind {
                        return None;
                    }
                    *field = kind;
                    Some(DimensionEntity { look, ..d.clone() })
                })
            }
            Event::Commit(field, text) => commit(model, &field, &text),
        };
        for text in said {
            self.warn(text);
        }
    }
}

/// A kitchen run changed through `cad.casework.edit` (docs/adr/0727 §6): its refusal or warnings, said.
fn casework_edit(
    model: &mut kentos_domain::Document,
    slot: Slot,
    edit: serde_json::Value,
) -> Vec<String> {
    use kentos_commands::CallOp;
    use kentos_native_application::{ExecutionContext, registry};
    let Some(uid) = model.uid(slot) else {
        return Vec::new();
    };
    let input = serde_json::json!({
        "run": uid.to_string(),
        "edit": edit,
        "expectedRevision": model.revision().to_string(),
    });
    said(registry::run(
        &mut ExecutionContext::new(model),
        "cad.casework.edit",
        CallOp::Execute,
        input,
    ))
}

/// A command's answer as what the panel says: its warnings, or why it wrote nothing.
fn said(
    answer: Result<serde_json::Value, kentos_native_application::registry::Unrun>,
) -> Vec<String> {
    match answer {
        Ok(value) if value["status"] == "completed" => value["warnings"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|w| w["message"].as_str().map(str::to_owned))
            .collect(),
        Ok(value) => vec![
            value["error"]["message"]
                .as_str()
                .unwrap_or("Mutfak dizisi değiştirilemedi; katman kilitlerini kontrol edin.")
                .into(),
        ],
        Err(error) => vec![error.message],
    }
}

/// Diziyi bırak and Dizinin tamamını sil.
fn casework_action(model: &mut kentos_domain::Document, slot: Slot, action: &str) -> Vec<String> {
    use kentos_commands::CallOp;
    use kentos_native_application::{ExecutionContext, registry};
    match action {
        "release" => casework_edit(model, slot, serde_json::json!({"kind": "release"})),
        "delete" => {
            let Some((run, _)) = model.uid(slot).and_then(|uid| model.casework_member(uid)) else {
                return Vec::new();
            };
            let uids: Vec<String> = run.members.iter().map(|m| m.uid.to_text()).collect();
            let input =
                serde_json::json!({"uids": uids, "expectedRevision": model.revision().to_string()});
            said(registry::run(
                &mut ExecutionContext::new(model),
                "cad.entities.delete",
                CallOp::Execute,
                input,
            ))
        }
        _ => Vec::new(),
    }
}

/// A run's value picked: the object's module's width, door or kind (its other own value kept), or the recipe's.
fn casework_value(
    model: &mut kentos_domain::Document,
    slot: Slot,
    field: &str,
    value: &str,
) -> Vec<String> {
    use serde_json::json;
    let Some((run, member)) = model.uid(slot).and_then(|uid| model.casework_member(uid)) else {
        return Vec::new();
    };
    let (recipe, module) = (run.recipe.clone(), member.index);
    let (own_width, own_hand) = match module {
        Some(k) => (
            rows::casework::own_width(&recipe, k),
            rows::casework::own_hand(&recipe, k),
        ),
        None => (None, None),
    };
    let edit = match (field, module) {
        ("width", Some(k)) => {
            let Ok(width) = value.parse::<f64>() else {
                return Vec::new();
            };
            // A cabinet wider than 0.60 m takes a double door: its hinge goes.
            let hand = own_hand.filter(|_| width <= kentos_commands::CASEWORK_APPLIANCE + 1e-9);
            json!({"kind": "module", "module": k, "width": width, "hand": hand})
        }
        ("hand", Some(k)) => {
            json!({"kind": "module", "module": k, "hand": value, "width": own_width})
        }
        ("holds", Some(k)) => {
            // An appliance (a dishwasher, an oven's cooktop) has no width or door of its own.
            let appliance =
                value == "dishwasher" || (value == "cooktop" && recipe.oven == Some(true));
            if appliance {
                json!({"kind": "module", "module": k, "holds": value})
            } else {
                json!({"kind": "module", "module": k, "holds": value, "width": own_width, "hand": own_hand})
            }
        }
        ("corner" | "fridge", _) => {
            let mut r = serde_json::to_value(&recipe).unwrap_or_default();
            r[field] = if value == "none" {
                serde_json::Value::Null
            } else {
                json!(value)
            };
            json!({"kind": "recipe", "recipe": r})
        }
        ("upper" | "oven", _) => {
            let mut r = serde_json::to_value(&recipe).unwrap_or_default();
            r[field] = json!(value == "true");
            json!({"kind": "recipe", "recipe": r})
        }
        _ => return Vec::new(),
    };
    casework_edit(model, slot, edit)
}

/// A member points at its recipe; the shared command owns all edits and undo.
fn dimension_set_edit(
    model: &mut kentos_domain::Document,
    slot: Slot,
    operation: serde_json::Value,
) -> Vec<String> {
    use kentos_commands::CallOp;
    use kentos_native_application::{ExecutionContext, registry};
    let Some((set, _)) = model
        .uid(slot)
        .and_then(|uid| model.dimension_set_of_item(uid))
    else {
        return Vec::new();
    };
    let input = serde_json::json!({"id":set.id.to_string(),"operation":operation,"expectedRevision":model.revision().to_string()});
    match registry::run(
        &mut ExecutionContext::new(model),
        "cad.dimensionsets.edit",
        CallOp::Execute,
        input,
    ) {
        Ok(value) if value["status"] == "completed" => value["warnings"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|w| w["message"].as_str().map(str::to_owned))
            .collect(),
        Ok(value) => vec![
            value["error"]["message"]
                .as_str()
                .unwrap_or(
                    "Ölçü dizisi değiştirilemedi; kaynağı ve katman kilitlerini kontrol edin.",
                )
                .into(),
        ],
        Err(error) => vec![error.message],
    }
}

fn dimension_set_action(
    model: &mut kentos_domain::Document,
    slot: Slot,
    action: &str,
) -> Vec<String> {
    use serde_json::json;
    let found = model
        .uid(slot)
        .and_then(|uid| model.dimension_set_of_item(uid));
    let row = found.map(|(set, index)| set.members[index].row.name());
    // A façade's or a space's row on or off, the rows kept in their order (docs/adr/0729 §7).
    let toggled = action.strip_prefix("toggle:").and_then(|name| {
        let (set, _) = found?;
        let order = ["openings", "walls", "grids", "overall"];
        let mut rows: Vec<&str> = order
            .iter()
            .copied()
            .filter(|r| set.rows.iter().any(|x| x.name() == *r))
            .collect();
        if rows.contains(&name) {
            rows.retain(|r| *r != name);
        } else {
            rows.push(name);
            rows.sort_by_key(|r| order.iter().position(|o| o == r));
        }
        Some(rows)
    });
    let operation = match action {
        _ if toggled.is_some() => json!({"kind":"set","rows":toggled}),
        "faces" | "core" => json!({"kind":"set","reference":action}),
        "left" | "right" => json!({"kind":"set","face":action}),
        "ends" | "centers" => json!({"kind":"set","openings":action}),
        "both" => json!({"kind":"set","rows":["openings","overall"]}),
        "openings" | "overall" => json!({"kind":"set","rows":[action]}),
        "rowText" => json!({"kind":"resetText","row":row}),
        "resetRows" | "restoreHidden" | "resetText" | "detach" | "delete" => json!({"kind":action}),
        _ => return Vec::new(),
    };
    dimension_set_edit(model, slot, operation)
}

/// `cad.entities.set`'s input for the objects in `slots`, nothing set yet.
fn set_input(
    model: &kentos_domain::Document,
    slots: &[Slot],
    operation: PropertiesOperation,
) -> EntitiesSetProperties {
    EntitiesSetProperties {
        line_pattern: None,
        line_type: None,
        line_type_scale: None,
        transparency: None,
        wipeout: None,
        uids: properties::uids_of(model, slots),
        layer_id: None,
        color: None,
        line_weight: None,
        symbol: None,
        attrs: None,
        label: None,
        operation,
        expected_revision: None,
        unlink: false,
        attachments: None,
    }
}

/// A Kot cell's text into the drawing (docs/adr/0142): a number sets the
/// elevation of the vertices it stands for, an empty text clears it, and
/// anything else changes nothing (the web's `parseElevation`). What to say:
/// the command's refusal.
fn commit_elevation(
    model: &mut kentos_domain::Document,
    slots: &[Slot],
    spot: Spot,
    text: &str,
) -> Vec<String> {
    // Typed in the project's unit (docs/adr/0165 §2).
    let f = kentos_interaction::Format::of(model.settings());
    let Some(value) = elevation::parse_typed_in(text, f.length_unit_label()) else {
        return Vec::new();
    };
    let value = value.map(|z| f.to_metres(z));
    let change = match spot {
        Spot::Start => Change::End(0, value),
        Spot::End => Change::End(1, value),
        Spot::All => Change::Set(value),
    };
    properties::set_elevations(model, slots, change)
}

/// A cell's text into the drawing, as the web's editors take it; what they
/// do not take changes nothing. What to say: the command's refusal.
fn commit(model: &mut kentos_domain::Document, field: &Field, text: &str) -> Vec<String> {
    // Lengths and coordinates are typed in the project's unit (docs/adr/0165 §2).
    let f = kentos_interaction::Format::of(model.settings());
    // Only physical architectural length cells opt in. Angles, paper sizes,
    // coordinates and the legacy GIS readers keep their existing grammar.
    let cad = model.settings().workspace == Some(kentos_commands::Workspace::Cad);
    let length = |text: &str| {
        if cad {
            kentos_geometry_core::tools::point_text::parse_length(text)
                .map(|value| value.metres(|v| f.to_metres(v)))
        } else {
            Some(f.to_metres(web_number(text)))
        }
    };
    if let Field::CaseworkDepth(slot) = field {
        let Some(recipe) = model
            .uid(*slot)
            .and_then(|uid| model.casework_member(uid))
            .map(|(run, _)| run.recipe.clone())
        else {
            return Vec::new();
        };
        return match length(text) {
            Some(depth) if depth.is_finite() => {
                let mut r = serde_json::to_value(&recipe).unwrap_or_default();
                r["depth"] = serde_json::json!(depth);
                casework_edit(
                    model,
                    *slot,
                    serde_json::json!({"kind": "recipe", "recipe": r}),
                )
            }
            _ => vec!["Tezgâhın derinliğini girin; örneğin 65 cm.".into()],
        };
    }
    if let Field::DimensionSetScales(slot) = field {
        return match kentos_interaction::wall_dimensions::sheet_scales(text) {
            Some(scales) => dimension_set_edit(model, *slot, serde_json::json!({"kind":"set","scales":scales})),
            None => vec!["Pafta ölçeklerini 1:100, 1:200 ya da 100 200 gibi yazın (1 ile 1000000 arası, en çok 32); kaldırmak için boş bırakın ya da Yok yazın.".into()],
        };
    }
    if let Field::DimensionSetGap(slot) = field {
        let typed = text.trim().strip_suffix("mm").unwrap_or(text.trim()).trim();
        return match kentos_geometry_core::tools::point_text::parse_number(typed) {
            Some(mm) if mm.is_finite() && mm > 0.0 && mm <= 1000.0 =>
                dimension_set_edit(model, *slot, serde_json::json!({"kind":"set","gapMm":mm})),
            _ => vec!["Kağıt üzerinde 0'dan büyük, en çok 1000 mm bir sıra aralığı girin; örneğin 7.5 mm.".into()],
        };
    }
    if let Field::DimensionOffset(slot) = field
        && let Some((set, index)) = model
            .uid(*slot)
            .and_then(|uid| model.dimension_set_of_item(uid))
    {
        let row = set.members[index].row.name();
        return match length(text) {
            Some(offset) if offset.is_finite() => dimension_set_edit(
                model,
                *slot,
                serde_json::json!({"kind":"rowOffset","row":row,"offset":offset}),
            ),
            _ => vec!["Bu sıranın duvar yüzüne uzaklığını girin; örneğin 40 cm.".into()],
        };
    }
    match field {
        Field::Elevation(slot, spot) => return commit_elevation(model, &[*slot], *spot, text),
        // Typed in the project's unit; emptied or 0, none (docs/adr/0258 §2).
        Field::PolylineWidth(slot) => {
            let t = kentos_interaction::js_trim(text);
            let n = if t.is_empty() { 0.0 } else { web_number(t) };
            return if n.is_finite() {
                properties::set_polyline_width(model, &[*slot], f.to_metres(n))
            } else {
                Vec::new()
            };
        }
        Field::Elevations(slots) => return commit_elevation(model, slots, Spot::All, text),
        // Any finite number: the command refuses one out of range and says why.
        Field::TextWidth(slots) => {
            let n = web_number(text);
            return if n.is_finite() {
                properties::set_text_width(model, slots, n)
            } else {
                Vec::new()
            };
        }
        // A box: emptied, none; a number, the command refuses one not over 0 (docs/adr/0182 §4).
        Field::AnnotativeScales(slots) => {
            return match kentos_interaction::annotative::parse_scales(text) {
                Ok(wanted) => kentos_interaction::annotative::set_scales(model, slots, &wanted),
                Err(words) => vec![words],
            };
        }
        Field::TextBox(slots) => {
            let t = kentos_interaction::js_trim(text);
            if t.is_empty() || t == "Kutusuz" {
                return properties::set_text_box(model, slots, None);
            }
            let n = web_number(t);
            return if n.is_finite() {
                properties::set_text_box(model, slots, Some(n))
            } else {
                Vec::new()
            };
        }
        Field::TextSpacing(slots) => {
            let n = web_number(text);
            return if n.is_finite() {
                properties::set_text_spacing(model, slots, n)
            } else {
                Vec::new()
            };
        }
        // The layout's numbers (docs/adr/0324 §3): the command refuses one out of range and says why.
        Field::TextIndent(slots)
        | Field::TextLeftIndent(slots)
        | Field::TextColumns(slots)
        | Field::TextColumnHeight(slots)
        | Field::TextColumnGap(slots) => {
            use kentos_interaction::text_layout as layout;
            let n = web_number(text);
            if !n.is_finite() {
                return Vec::new();
            }
            return match field {
                Field::TextIndent(_) => layout::set_indent(model, slots, n),
                Field::TextLeftIndent(_) => layout::set_left_indent(model, slots, n),
                Field::TextColumns(_) => {
                    layout::set_columns(model, slots, n.max(0.0).min(f64::from(u32::MAX)) as u32)
                }
                Field::TextColumnHeight(_) => layout::set_column_height(model, slots, n),
                _ => layout::set_column_gap(model, slots, n),
            };
        }
        Field::TextTabs(slots) => {
            return match kentos_interaction::text_layout::parse_tabs(text) {
                Ok(stops) => kentos_interaction::text_layout::set_tabs(model, slots, &stops),
                Err(words) => vec![words],
            };
        }
        Field::LeaderNote(slots) => {
            let note = kentos_interaction::js_trim(text);
            let to = (!note.is_empty()).then(|| note.to_owned());
            return properties::change_leaders(model, slots, |l| {
                (l.text != to).then(|| LeaderEntity {
                    text: to.clone(),
                    ..l.clone()
                })
            });
        }
        Field::LeaderHeight(slots) => {
            let n = f.to_metres(web_number(text));
            if !(n.is_finite() && n > 0.0) {
                return Vec::new();
            }
            return properties::change_leaders(model, slots, |l| {
                (l.height != n).then(|| LeaderEntity {
                    height: n,
                    ..l.clone()
                })
            });
        }
        Field::DimensionRound(slots) | Field::DimensionTolerance(slots, _) => {
            let n = f.to_metres(web_number(text));
            let empty = kentos_interaction::js_trim(text).is_empty();
            if !(empty || n.is_finite()) {
                return Vec::new();
            }
            let base = model
                .get(slots[0])
                .and_then(|e| match e {
                    Entity::Dimension(d) => d.look.options.as_deref().cloned(),
                    _ => None,
                })
                .unwrap_or_default();
            let mut options = base;
            match field {
                Field::DimensionRound(_) => options.round = (!empty && n > 0.0).then_some(n),
                Field::DimensionTolerance(_, upper) => {
                    let Some(t) = options.tolerance.as_mut() else {
                        return Vec::new();
                    };
                    let v = if empty { 0.0 } else { n };
                    if *upper {
                        t.upper = v;
                    } else {
                        t.lower = Some(v);
                    }
                }
                _ => {}
            }
            let options = (!options.is_empty()).then_some(options);
            return properties::edit_dimensions(model, slots, |uids| {
                kentos_commands::DimensionEdit::Options { uids, options }
            });
        }
        // Fark (docs/adr/0721 §2): the Kots' offset from the level they share; emptied or 0, none.
        Field::DimensionLevelOffset(slots) => {
            let id = slots.iter().find_map(|s| match model.get(*s) {
                Some(Entity::Dimension(d)) => d.level.as_ref().map(|l| l.id.clone()),
                _ => None,
            });
            let Some(id) = id else {
                return Vec::new();
            };
            // A length with its unit kept (−2 cm), else in the drawing's unit (docs/adr/0646).
            let typed = kentos_interaction::js_trim(text);
            let n = if typed.is_empty() {
                Some(0.0)
            } else {
                length(typed)
            };
            let Some(n) =
                n.filter(|n| n.is_finite() && n.abs() <= kentos_commands::MAX_LEVEL_OFFSET)
            else {
                return vec![
                    "Farkı metre olarak bir sayıyla yazın (−0.02 gibi); en çok 1000 m.".into(),
                ];
            };
            let level = kentos_commands::DimensionLevel {
                id,
                offset: (n != 0.0).then_some(n),
            };
            return properties::edit_dimensions(model, slots, |uids| {
                kentos_commands::DimensionEdit::Level {
                    uids,
                    level: Some(level),
                }
            });
        }
        // A slope's and a Kot's elevations: a length with its unit kept in CAD (docs/adr/0646, 0721 §2).
        Field::DimensionZa(slots) | Field::DimensionZb(slots) => {
            let Some(n) = length(kentos_interaction::js_trim(text)).filter(|n| n.is_finite())
            else {
                return Vec::new();
            };
            let second = matches!(field, Field::DimensionZb(_));
            return properties::change_dimensions(model, slots, |d| {
                let z = if second { d.zb } else { d.za };
                (z != Some(n)).then(|| {
                    let mut d = d.clone();
                    *(if second { &mut d.zb } else { &mut d.za }) = Some(n);
                    d
                })
            });
        }
        Field::LeaderArrowSize(slots) => {
            let n = web_number(text);
            if !kentos_commands::leader_arrow_holds(n) {
                return Vec::new();
            }
            let size = (n != 1.0).then_some(n);
            return properties::change_leaders(model, slots, |l| {
                (l.arrow_size.unwrap_or(1.0) != n).then(|| LeaderEntity {
                    arrow_size: size,
                    ..l.clone()
                })
            });
        }
        Field::OpeningWidth(slot)
        | Field::OpeningAt(slot)
        | Field::OpeningSwing(slot)
        | Field::OpeningFrame(slot)
        | Field::OpeningSill(slot)
        | Field::OpeningHead(slot)
        | Field::OpeningDepth(slot)
        | Field::OpeningInset(slot)
        | Field::OpeningNumber(slot) => {
            use kentos_commands::{OpeningChange, OpeningsOperation};
            let blank = text.trim().is_empty();
            let op = match field {
                Field::OpeningNumber(_) => {
                    let n = kentos_interaction::js_trim(text);
                    OpeningsOperation::Set(OpeningChange {
                        number: Some((!n.is_empty()).then(|| n.to_owned())),
                        ..Default::default()
                    })
                }
                Field::OpeningFrame(_)
                | Field::OpeningSill(_)
                | Field::OpeningHead(_)
                | Field::OpeningSwing(_)
                | Field::OpeningInset(_)
                    if blank =>
                {
                    OpeningsOperation::Set(match field {
                        Field::OpeningFrame(_) => OpeningChange {
                            frame: Some(None),
                            ..Default::default()
                        },
                        Field::OpeningSill(_) => OpeningChange {
                            sill: Some(None),
                            ..Default::default()
                        },
                        Field::OpeningHead(_) => OpeningChange {
                            head: Some(None),
                            ..Default::default()
                        },
                        Field::OpeningInset(_) => OpeningChange {
                            inset: Some(None),
                            ..Default::default()
                        },
                        _ => OpeningChange {
                            swing: Some(None),
                            ..Default::default()
                        },
                    })
                }
                _ => {
                    let n = if matches!(field, Field::OpeningSwing(_)) {
                        web_number(text)
                    } else {
                        let Some(n) = length(text) else {
                            return vec![
                                "Açıklık ölçüsü için bir uzunluk girin; örneğin 900 mm veya 90 cm."
                                    .into(),
                            ];
                        };
                        n
                    };
                    if !n.is_finite() {
                        return Vec::new();
                    }
                    match field {
                        Field::OpeningWidth(_) => OpeningsOperation::Set(OpeningChange {
                            width: Some(n),
                            ..Default::default()
                        }),
                        Field::OpeningAt(_) => OpeningsOperation::Move {
                            by: None,
                            at: Some(n),
                        },
                        Field::OpeningSwing(_) => OpeningsOperation::Set(OpeningChange {
                            swing: Some(Some(n)),
                            ..Default::default()
                        }),
                        Field::OpeningFrame(_) => OpeningsOperation::Set(OpeningChange {
                            frame: Some(Some(n)),
                            ..Default::default()
                        }),
                        Field::OpeningSill(_) => OpeningsOperation::Set(OpeningChange {
                            sill: Some(Some(n)),
                            ..Default::default()
                        }),
                        Field::OpeningDepth(_) => OpeningsOperation::Set(OpeningChange {
                            depth: Some(Some(n)),
                            ..Default::default()
                        }),
                        Field::OpeningInset(_) => OpeningsOperation::Set(OpeningChange {
                            inset: Some(Some(n)),
                            ..Default::default()
                        }),
                        _ => OpeningsOperation::Set(OpeningChange {
                            head: Some(Some(n)),
                            ..Default::default()
                        }),
                    }
                }
            };
            return properties::edit_opening(model, *slot, op);
        }
        Field::Arch(slot, which @ (ArchField::MarkNumber | ArchField::MarkSize)) => {
            use kentos_commands::{ViewMarkChange, ViewMarksOperation};
            let t = kentos_interaction::js_trim(text);
            let c = if *which == ArchField::MarkNumber {
                ViewMarkChange {
                    mark: Some(t.to_owned()),
                    ..Default::default()
                }
            } else {
                let n = web_number(text);
                if !t.is_empty() && !n.is_finite() {
                    return Vec::new();
                }
                ViewMarkChange {
                    size: Some((!t.is_empty()).then_some(n)),
                    ..Default::default()
                }
            };
            return properties::edit_viewmark(model, *slot, ViewMarksOperation::Set(Box::new(c)));
        }
        Field::Arch(slot, which @ (ArchField::TagContent | ArchField::TagSize)) => {
            use kentos_commands::TagChange;
            let t = kentos_interaction::js_trim(text);
            let c = if *which == ArchField::TagContent {
                TagChange {
                    content: Some((!t.is_empty()).then(|| t.replace("\\n", "\n"))),
                    ..Default::default()
                }
            } else {
                let n = web_number(text);
                if !t.is_empty() && !n.is_finite() {
                    return Vec::new();
                }
                TagChange {
                    size: Some((!t.is_empty()).then_some(n)),
                    ..Default::default()
                }
            };
            return properties::edit_tag(model, *slot, c);
        }
        Field::Arch(slot, which) => {
            use kentos_commands::{ArchChange, ArchOperation};
            let t = kentos_interaction::js_trim(text);
            let words = (!t.is_empty()).then(|| t.to_owned());
            let physical = matches!(
                which,
                ArchField::Width
                    | ArchField::Depth
                    | ArchField::RiserHeight
                    | ArchField::Tread
                    | ArchField::Landing
                    | ArchField::Rise
                    | ArchField::Base
                    | ArchField::BaseOffset
                    | ArchField::Post
                    | ArchField::LegA
                    | ArchField::LegB
            );
            let n = if physical && !t.is_empty() {
                let Some(n) = length(text) else {
                    return vec![
                        "Mimari ölçü için bir uzunluk girin; örneğin 400 mm veya 40 cm.".into(),
                    ];
                };
                n
            } else {
                web_number(text)
            };
            if !t.is_empty()
                && !n.is_finite()
                && !matches!(which, ArchField::Label | ArchField::Tag | ArchField::Type)
            {
                return Vec::new();
            }
            let c = match which {
                ArchField::Label => match words {
                    Some(w) => ArchChange {
                        label: Some(w),
                        ..Default::default()
                    },
                    None => return Vec::new(),
                },
                ArchField::Tag => ArchChange {
                    tag: Some(words),
                    ..Default::default()
                },
                ArchField::Type => ArchChange {
                    kind: Some(words),
                    ..Default::default()
                },
                ArchField::BubbleSize => ArchChange {
                    bubble_size: Some(words.map(|_| n)),
                    ..Default::default()
                },
                ArchField::Depth => ArchChange {
                    depth: Some(words.map(|_| n)),
                    ..Default::default()
                },
                ArchField::Landing => ArchChange {
                    landing: Some(words.map(|_| n)),
                    ..Default::default()
                },
                ArchField::Slope => ArchChange {
                    slope: Some(words.map(|_| n)),
                    ..Default::default()
                },
                ArchField::Base => ArchChange {
                    base: Some(words.map(|_| n)),
                    ..Default::default()
                },
                ArchField::BaseOffset => ArchChange {
                    base_offset: Some(words.map(|_| n)),
                    ..Default::default()
                },
                ArchField::Post => ArchChange {
                    post: Some(words.map(|_| n)),
                    ..Default::default()
                },
                ArchField::Ends => ArchChange {
                    ends: Some(words.map(|_| n).filter(|v| *v != 0.0)),
                    ..Default::default()
                },
                _ if words.is_none() => return Vec::new(),
                // One leg typed, the other kept (docs/adr/0725 §3).
                ArchField::LegA | ArchField::LegB => {
                    let Some(kentos_commands::Entity::Column(c)) = model.get(*slot) else {
                        return Vec::new();
                    };
                    let [a, b] = c
                        .column
                        .legs
                        .unwrap_or([c.column.depth() / 2.0, c.column.width / 2.0]);
                    ArchChange {
                        legs: Some(if *which == ArchField::LegA {
                            [n, b]
                        } else {
                            [a, n]
                        }),
                        ..Default::default()
                    }
                }
                ArchField::Width => ArchChange {
                    width: Some(n),
                    ..Default::default()
                },
                ArchField::Turn => ArchChange {
                    rotation: Some(n.to_radians()),
                    ..Default::default()
                },
                ArchField::Risers if n >= 0.0 && n.fract() == 0.0 => ArchChange {
                    risers: Some(n as u32),
                    ..Default::default()
                },
                ArchField::Risers => return Vec::new(),
                ArchField::RiserHeight => ArchChange {
                    riser_height: Some(n),
                    ..Default::default()
                },
                ArchField::Tread => ArchChange {
                    tread: Some(n),
                    ..Default::default()
                },
                ArchField::Rise => ArchChange {
                    rise: Some(n),
                    ..Default::default()
                },
                ArchField::TagContent
                | ArchField::TagSize
                | ArchField::MarkNumber
                | ArchField::MarkSize => {
                    return Vec::new();
                }
            };
            return properties::edit_arch(model, *slot, ArchOperation::Set(Box::new(c)));
        }
        Field::SpaceNumber(slot)
        | Field::SpaceName(slot)
        | Field::SpaceUsage(slot)
        | Field::SpaceHeight(slot)
        | Field::SpaceFloor(slot)
        | Field::SpaceWall(slot)
        | Field::SpaceCeiling(slot) => {
            use kentos_commands::{SpaceChange, SpacesOperation};
            let t = kentos_interaction::js_trim(text);
            let words = (!t.is_empty()).then(|| t.to_owned());
            let change = match field {
                Field::SpaceNumber(_) => SpaceChange {
                    number: Some(words),
                    ..Default::default()
                },
                Field::SpaceName(_) => SpaceChange {
                    name: Some(words),
                    ..Default::default()
                },
                Field::SpaceUsage(_) => SpaceChange {
                    usage: Some(words),
                    ..Default::default()
                },
                Field::SpaceFloor(_) => SpaceChange {
                    floor: Some(words),
                    ..Default::default()
                },
                Field::SpaceWall(_) => SpaceChange {
                    wall: Some(words),
                    ..Default::default()
                },
                Field::SpaceCeiling(_) => SpaceChange {
                    ceiling: Some(words),
                    ..Default::default()
                },
                _ if t.is_empty() => SpaceChange {
                    height: Some(None),
                    ..Default::default()
                },
                _ => {
                    let Some(n) = length(text) else {
                        return vec![
                            "Mahal yüksekliği için bir uzunluk girin; örneğin 3000 mm veya 3 m."
                                .into(),
                        ];
                    };
                    if !n.is_finite() {
                        return Vec::new();
                    }
                    SpaceChange {
                        height: Some(Some(n)),
                        ..Default::default()
                    }
                }
            };
            return properties::edit_space(model, *slot, SpacesOperation::Set(change));
        }
        Field::SlabThickness(slot)
        | Field::SlabOffset(slot)
        | Field::SlabEdgeWidth(slot)
        | Field::SlabMaterial(slot) => {
            use kentos_commands::{SlabChange, SlabsOperation};
            let change = if let Field::SlabMaterial(_) = field {
                let t = kentos_interaction::js_trim(text);
                SlabChange {
                    material: Some((!t.is_empty()).then(|| t.to_owned())),
                    ..Default::default()
                }
            } else {
                let Some(n) = length(text) else {
                    return vec!["Bir uzunluk girin; örneğin 15 cm ya da 0.15 m.".into()];
                };
                if !n.is_finite() {
                    return Vec::new();
                }
                match field {
                    Field::SlabThickness(_) => SlabChange {
                        thickness: Some(n),
                        ..Default::default()
                    },
                    Field::SlabOffset(_) => SlabChange {
                        offset: Some(n),
                        ..Default::default()
                    },
                    _ => {
                        let Some(Entity::Slab(s)) = model.get(*slot) else {
                            return Vec::new();
                        };
                        let Some(e) = &s.slab.edge else {
                            return Vec::new();
                        };
                        SlabChange {
                            edge: Some(Some(kentos_commands::NewSlabEdge {
                                kind: e.kind,
                                width: Some(n),
                            })),
                            ..Default::default()
                        }
                    }
                }
            };
            return properties::edit_slab(model, *slot, SlabsOperation::Set(change));
        }
        Field::WallWidth(slot) | Field::WallOffset(slot) | Field::WallHeight(slot) => {
            let height = matches!(field, Field::WallHeight(_));
            if height && text.trim().is_empty() {
                return properties::edit_wall(
                    model,
                    *slot,
                    kentos_commands::WallChange {
                        height: Some(None),
                        ..Default::default()
                    },
                );
            }
            let Some(n) = length(text) else {
                return vec![
                    "Duvar ölçüsü için bir uzunluk girin; örneğin 250 mm veya 25 cm.".into(),
                ];
            };
            if !n.is_finite() {
                return Vec::new();
            }
            let change = match field {
                Field::WallWidth(_) => kentos_commands::WallChange {
                    width: Some(n),
                    ..Default::default()
                },
                Field::WallOffset(_) => kentos_commands::WallChange {
                    offset: Some(n),
                    ..Default::default()
                },
                _ => kentos_commands::WallChange {
                    height: Some(Some(n)),
                    ..Default::default()
                },
            };
            return properties::edit_wall(model, *slot, change);
        }
        Field::LeaderTurn(slots) => {
            let n = web_number(text);
            if !n.is_finite() {
                return Vec::new();
            }
            let turn = ((n % 360.0) + 360.0) % 360.0;
            return properties::change_leaders(model, slots, |l| {
                (l.rotation != turn).then(|| LeaderEntity {
                    rotation: turn,
                    ..l.clone()
                })
            });
        }
        _ => {}
    }
    if let Field::Attributes(slots, key) = field {
        let mut input = set_input(model, slots, PropertiesOperation::Attributes);
        input.attrs = Some(BTreeMap::from([(key.clone(), Some(text.to_owned()))]));
        return properties::set_properties(model, input);
    }
    if let Field::Parameter(slot, name) = field {
        let Some(Entity::Insert(i)) = model.get(*slot) else {
            return Vec::new();
        };
        let Some(p) = model
            .block(i.block)
            .and_then(|b| b.dynamic.as_ref())
            .and_then(|d| d.parameters.iter().find(|p| p.name() == name))
        else {
            return Vec::new();
        };
        let Some(value) = rows::params::typed(p, text) else {
            return vec![format!(
                "“{name}” için {} yazın.",
                match p {
                    kentos_commands::BlockParameter::Angle { .. } => "derece olarak bir açı",
                    kentos_commands::BlockParameter::Linear { .. } => "bir sayı",
                    _ => "“doğu, kuzey” olarak iki sayı",
                }
            )];
        };
        return properties::set_parameters(
            model,
            *slot,
            Some(BTreeMap::from([(name.clone(), value)])),
            None,
        );
    }
    if let Field::Attribute(slot, key) = field {
        let Some(base) = model.get(*slot).map(Entity::base) else {
            return Vec::new();
        };
        // Keep the drawn number in step with the cadastral attribute, in the same step.
        let follows = (key == "Parsel" || key == "Ada")
            && base.label.is_some()
            && base.label.as_ref() == base.attrs.get(key);
        let mut input = set_input(model, &[*slot], PropertiesOperation::Attributes);
        input.attrs = Some(BTreeMap::from([(key.clone(), Some(text.to_owned()))]));
        input.label = follows.then(|| Some(text.to_owned()));
        return properties::set_properties(model, input);
    }
    let n = web_number(text);
    let finite = n.is_finite();
    let slot = match field {
        Field::PointX(s)
        | Field::PointY(s)
        | Field::DimensionOffset(s)
        | Field::DimensionHeight(s)
        | Field::DimensionText(s)
        | Field::HatchAngle(s)
        | Field::HatchSpacing(s)
        | Field::HatchScale(s)
        | Field::Text(s)
        | Field::TextHeight(s)
        | Field::TextAngle(s)
        | Field::InsertX(s)
        | Field::InsertY(s)
        | Field::InsertScale(s)
        | Field::InsertYScale(s)
        | Field::InsertTurn(s)
        | Field::ImageX(s)
        | Field::ImageY(s)
        | Field::ImageWidth(s)
        | Field::ImageHeight(s)
        | Field::ImageTurn(s)
        | Field::ImageClear(s)
        | Field::RasterClear(s)
        | Field::CloudClear(s)
        | Field::Attribute(s, _) => *s,
        // Taken above.
        Field::WallWidth(_)
        | Field::WallOffset(_)
        | Field::WallHeight(_)
        | Field::OpeningWidth(_)
        | Field::OpeningAt(_)
        | Field::OpeningSwing(_)
        | Field::OpeningFrame(_)
        | Field::OpeningSill(_)
        | Field::OpeningHead(_)
        | Field::OpeningDepth(_)
        | Field::OpeningInset(_)
        | Field::OpeningNumber(_)
        | Field::SpaceNumber(_)
        | Field::SpaceName(_)
        | Field::SpaceUsage(_)
        | Field::SpaceHeight(_)
        | Field::SpaceFloor(_)
        | Field::SpaceWall(_)
        | Field::SpaceCeiling(_)
        | Field::SlabThickness(_)
        | Field::SlabOffset(_)
        | Field::SlabEdgeWidth(_)
        | Field::SlabMaterial(_)
        | Field::Arch(..)
        | Field::Elevation(..)
        | Field::PolylineWidth(_)
        | Field::Elevations(_)
        | Field::TextWidth(_)
        | Field::TextBox(_)
        | Field::AnnotativeScales(_)
        | Field::TextSpacing(_)
        | Field::TextIndent(_)
        | Field::TextLeftIndent(_)
        | Field::TextTabs(_)
        | Field::TextColumns(_)
        | Field::TextColumnHeight(_)
        | Field::TextColumnGap(_)
        | Field::LeaderNote(_)
        | Field::LeaderHeight(_)
        | Field::LeaderTurn(_)
        | Field::LeaderArrowSize(_)
        | Field::Attributes(..)
        | Field::Parameter(..)
        | Field::DimensionZa(_)
        | Field::DimensionZb(_)
        | Field::DimensionRound(_)
        | Field::DimensionTolerance(..)
        | Field::DimensionLevelOffset(_) => return Vec::new(),
        Field::DimensionSetGap(_) | Field::DimensionSetScales(_) | Field::CaseworkDepth(_) => {
            return Vec::new();
        }
    };
    let Some(mut e) = model.get(slot).cloned() else {
        return Vec::new();
    };
    let taken = match (field, &mut e) {
        (Field::PointX(_), Entity::Point(p)) if finite => {
            p.p.x = f.to_metres(n);
            true
        }
        (Field::PointY(_), Entity::Point(p)) if finite => {
            p.p.y = f.to_metres(n);
            true
        }
        (Field::DimensionOffset(_), Entity::Dimension(d)) if finite => {
            d.offset = f.to_metres(n);
            true
        }
        (Field::DimensionHeight(_), Entity::Dimension(d)) if finite && n > 0.0 => {
            d.height = f.to_metres(n);
            true
        }
        (Field::DimensionText(_), Entity::Dimension(d)) => {
            let own = kentos_interaction::js_trim(text);
            d.text = (!own.is_empty()).then(|| own.to_owned());
            true
        }
        (Field::HatchAngle(_), Entity::Hatch(h)) if finite => {
            h.pattern.angle = n;
            true
        }
        (Field::HatchSpacing(_), Entity::Hatch(h)) if finite && n > 0.0 => {
            h.pattern.spacing = f.to_metres(n);
            true
        }
        // The pattern's paper scale: its millimetres n × N / 1000 metres each (docs/adr/0186 §7).
        (Field::HatchScale(_), Entity::Hatch(h))
            if finite && n > 0.0 && h.pattern.scale.is_some() =>
        {
            h.pattern.scale = Some(n * model.settings().plot_scale / 1000.0);
            true
        }

        // Trimmed, as the in-place editor stores it; an empty text is not taken.
        (Field::Text(_), Entity::Text(t)) => {
            let body = kentos_interaction::js_trim(text);
            if body.is_empty() {
                false
            } else {
                t.text = body.to_owned();
                true
            }
        }
        (Field::TextHeight(_), Entity::Text(t)) if finite && n > 0.0 => {
            t.height = f.to_metres(n);
            true
        }
        (Field::TextAngle(_), Entity::Text(t)) if finite => {
            t.rotation = ((n % 360.0) + 360.0) % 360.0;
            true
        }
        // A table's text height and turn (docs/adr/0184 §6): its cells keep their sizes.
        (Field::TextHeight(_), Entity::Table(t)) if finite && n > 0.0 => {
            t.height = f.to_metres(n);
            true
        }
        (Field::TextAngle(_), Entity::Table(t)) if finite => {
            t.rotation = ((n % 360.0) + 360.0) % 360.0;
            true
        }
        (Field::InsertX(_), Entity::Insert(i)) if finite => {
            i.p.x = f.to_metres(n);
            true
        }
        (Field::InsertY(_), Entity::Insert(i)) if finite => {
            i.p.y = f.to_metres(n);
            true
        }
        // Ölçek sets both factors of a uniform insert; X ölçeği the x one alone (docs/adr/0315 §3).
        (Field::InsertScale(_), Entity::Insert(i)) if finite && n > 0.0 => {
            i.scale = n;
            i.y_scale = i.y_scale.filter(|y| *y != n);
            true
        }
        (Field::InsertYScale(_), Entity::Insert(i)) if finite && n > 0.0 => {
            i.y_scale = (n != i.scale).then_some(n);
            true
        }
        (Field::InsertTurn(_), Entity::Insert(i)) if finite => {
            i.rotation = kentos_commands::blocks::turn_of(n);
            true
        }
        // A picture keeps its shape: its width sets its height and the other way round (docs/adr/0192 §4).
        (Field::ImageX(_), Entity::Image(i)) if finite => {
            i.image.p.x = f.to_metres(n);
            true
        }
        (Field::ImageY(_), Entity::Image(i)) if finite => {
            i.image.p.y = f.to_metres(n);
            true
        }
        (Field::ImageWidth(_), Entity::Image(i)) if finite && n > 0.0 => {
            let k = f.to_metres(n) / i.image.width;
            i.image.width *= k;
            i.image.height *= k;
            true
        }
        (Field::ImageHeight(_), Entity::Image(i)) if finite && n > 0.0 => {
            let k = f.to_metres(n) / i.image.height;
            i.image.width *= k;
            i.image.height *= k;
            true
        }
        (Field::ImageTurn(_), Entity::Image(i)) if finite => {
            i.image.rotation = kentos_commands::blocks::turn_of(n);
            true
        }
        (Field::ImageClear(_), Entity::Image(i)) if finite && (0.0..=90.0).contains(&n) => {
            i.image.opacity = (n > 0.0).then(|| 1.0 - n / 100.0);
            true
        }
        (Field::RasterClear(_), Entity::Raster(r)) if finite && (0.0..=90.0).contains(&n) => {
            r.raster.opacity = (n > 0.0).then(|| 1.0 - n / 100.0);
            true
        }
        (Field::CloudClear(_), Entity::PointCloud(c)) if finite && (0.0..=90.0).contains(&n) => {
            c.cloud.opacity = (n > 0.0).then(|| 1.0 - n / 100.0);
            true
        }
        _ => false,
    };
    if !taken {
        return Vec::new();
    }
    properties::set_geometry(model, slot, &e)
}

/// The drop-down's menu: its choices, their swatches, the commands.
fn menu(items: &[(Choice, Option<Color>)]) -> Menu<Message> {
    items
        .iter()
        .fold(Menu::new(), |menu, (choice, color)| match choice {
            Choice::Pick {
                label,
                chosen,
                enabled,
                message,
                icon,
                wide,
                ..
            } => {
                let menu = menu.radio(label.clone(), *chosen, enabled.then(|| message.clone()));
                // A colour's swatch, a hatch pattern's wide sample (docs/adr/0186 §11)
                // or a picture (a text's alignment, docs/adr/0145).
                match (color, icon) {
                    (Some(color), _) => menu.swatch(*color),
                    (None, Some(name)) if *wide => menu.preview(crate::icons::from_web(Some(name))),
                    (None, Some(name)) => menu.icon(crate::icons::from_web(Some(name))),
                    (None, None) => menu,
                }
            }
            Choice::Separator => menu.separator(),
            Choice::Command { label, icon, id } => menu.item(*label, Message::Run(id)).icon(*icon),
            Choice::Action {
                label,
                icon,
                message,
            } => menu
                .item(*label, message.clone())
                .icon(crate::icons::from_web(Some(icon))),
        })
}

/// The summary's box: its padding, a line under it.
fn boxed<'a>(content: impl Into<Element<'a, Message>>, padding: [u16; 2]) -> Element<'a, Message> {
    column![
        container(content).padding(padding).width(Fill),
        container(space::horizontal())
            .width(Fill)
            .height(1)
            .style(|theme: &Theme| container::Style {
                background: Some(kentos_ui::theme::Tokens::of(theme).border.into()),
                ..container::Style::default()
            }),
    ]
    .into()
}

/// The object's label in the summary: the accent's text colour (a parcel's number).
fn accent_text(theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(kentos_ui::theme::Tokens::of(theme).accent),
    }
}
