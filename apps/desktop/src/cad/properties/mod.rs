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
pub(crate) mod wall_parts;

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
use kentos_ui::widget::{EditCell, Menu};
use kentos_ui::icon::{Icon, icon};

pub(crate) use rows::{Choice, Editor, Panel, Row, Summary};

use crate::app::{App, Message};
use crate::document::Document;

/// A change asked for in the panel.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// A rendered panel's actions belong to that exact drawing, revision and selection.
    Guarded(PanelKey, Box<Event>),
    /// Only narrow the property inspector; preserve the actual scene selection.
    Kind(Option<String>),
    Query,
    /// A section's header: closes or opens it.
    Toggle(&'static str),
    /// A user-defined heading has an owned, definition-scoped key.
    ToggleGroup(String),
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
    /// Reset only the named parameter (a lookup resets the values its rows control).
    ParameterReset(Slot, String),
    /// Kaynak ▾'s Göm of a linked picture: its file read and kept in the
    /// project's library, the picture embedded (docs/adr/0192 §2).
    EmbedImage(Slot),
    /// Kaynak ▾'s Göm of a linked raster (docs/adr/0204 §8).
    EmbedRaster(Slot),
    /// A wall's own field chosen (Hiza, Stil, Uçlar, Kat, Tadilat durumu): written through `cad.walls.edit` (docs/adr/0339 §6).
    Wall(Slot, kentos_commands::WallChange),
    /// A wall's joins (docs/adr/0340 §6).
    WallOp(Slot, kentos_commands::WallsOperation),
    /// Kalınlıkta sabit (docs/adr/0771 §2): the line a wall's width or style change keeps this session.
    WallKeep(kentos_commands::WallReference),
    /// An opening's own field chosen, its type replaced (docs/adr/0341 §5): written through `cad.openings.edit`.
    Opening(Slot, kentos_commands::OpeningsOperation),
    /// A component's or a slab's project material chosen (docs/adr/0785): written through `cad.materials.edit`.
    Material(kentos_commands::MaterialsOperation),
    /// A room module's action on the object's instance (docs/adr/0775 §3): written through `cad.modules.edit`.
    Module(kentos_commands::ModulesOperation),
    /// A derived interior elevation's action (docs/adr/0799 §5): written through `cad.elevations.edit`.
    Elevation(kentos_commands::ElevationsOperation),
    /// A section's Yeniden çiz (docs/adr/8042 §5): written through `cad.sections.edit`.
    Section(kentos_commands::BuildingSectionsOperation),
    /// A space's own field chosen (Yöntem, Kat, Tadilat durumu) or its boundary found again (docs/adr/0342 §5): written through
    /// `cad.spaces.edit`.
    Space(Slot, Box<kentos_commands::SpacesOperation>),
    /// A slab's own field chosen (Tür, Kenar, Kat, Tadilat durumu, Metraj; docs/adr/0722 §6): written through `cad.slabs.edit`.
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
    /// Preview an explicit layout choice in the existing in-place editor; never write immediately.
    TextLayoutPreview(Slot, Option<kentos_geometry_core::text::paragraph::Engine>),
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
    /// Tablo stili ▾ (docs/adr/1127 §3): the tables made to follow the table style; none, no style.
    TableStyle(Vec<Slot>, Option<String>),
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
    /// A balanced stair's winders either side of each turn (docs/adr/8045 §1); empty: a landing at each turn.
    Winders,
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
    /// A beam's height and its top's offset (empty 0; docs/adr/8041 §1); a lintel's or a bond beam's underside over its
    /// storey's floor (empty: it hangs; docs/adr/8041 §3).
    BeamHeight,
    BeamOffset,
    BeamBase,
    /// A railing's balusters' clear gap (docs/adr/8045 §3); empty: PAİY 38/4's 0.10 m.
    RailingGap,
    /// A column's or a beam's material (empty: Betonarme; docs/adr/8041 §2).
    Material,
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
    TextHeights(Vec<Slot>),
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
    /// A special door's construction length (docs/adr/0777 §5): a revolving door's drum, a sliding door's overlap, a
    /// pocket door's lip (empty: none).
    OpeningBuild(Slot, kentos_commands::OpeningTypeField),
    /// A space's number, name, usage, inside height (empty: up to its ceiling; with an asma tavan, its height) and
    /// finishes (empty: none; docs/adr/0342 §5, 3040 §3).
    SpaceNumber(Slot),
    SpaceName(Slot),
    SpaceUsage(Slot),
    SpaceHeight(Slot),
    /// Its Döşeme kot farkı (empty: the floor slab under it, else its storey's) and Asma tavan (empty or Yok: none;
    /// docs/adr/8040 §3).
    SpaceFloorOffset(Slot),
    SpaceSuspended(Slot),
    SpaceFloor(Slot),
    SpaceWall(Slot),
    SpaceCeiling(Slot),
    /// Its skirting's finish and the room list's (docs/adr/8038): sills and parapets, stair treads and risers, handrails
    /// and railings.
    SpaceBase(Slot),
    SpaceSill(Slot),
    SpaceStair(Slot),
    SpaceRailing(Slot),
    /// A slab's thickness, top's offset, edge's width (lengths) and material (empty: none; docs/adr/0722 §6).
    SlabThickness(Slot),
    SlabOffset(Slot),
    SlabEdgeWidth(Slot),
    SlabMaterial(Slot),
    /// A slab's layer's thickness (docs/adr/8040 §2): over its structure (true) or under it, by its place from the top;
    /// 0 takes the layer away.
    SlabLayer(Slot, bool, usize),
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
    /// Topoloji's Gerekçe (docs/adr/4600): the words a finding's mark is written with, the tab's own; no drawing change.
    TopologyReason,
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
pub type PanelKey = (u64, u64, u64, u64);

#[derive(Default)]
pub(crate) struct InspectScope {
    kind: Option<(u64, u64, String)>,
    epoch: u64,
}

/// Past this many objects of a selection Öznitelikler shows its size and kinds only (AutoCAD's PROPOBJLIMIT default,
/// docs/adr/1027).
pub(crate) const PROPERTIES_LIMIT: usize = 25_000;

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

impl App {
    /// Temporary object visibility uses the same product command as Python and MCP.
    pub(crate) fn object_isolation(&mut self, end: bool) {
        use kentos_commands::{EntitiesIsolate, IsolationAction, CommandResult};
        if self.session.is_running() {
            self.warn("Önce etkin işlemi bitirin veya Esc ile iptal edin; ardından nesneleri yalıtın.");
            return;
        }
        let Some(doc) = self.document.as_mut() else { return; };
        let input = EntitiesIsolate { action: if end { IsolationAction::End } else {
            IsolationAction::Isolate { uids: self.selection.ids().iter()
                .filter_map(|s| doc.model.uid(*s)).map(|u| u.to_string()).collect() }
        }};
        match kentos_native_application::entities_select::isolation::execute(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { output, .. } => {
                self.output(if output.active {
                    format!("Geçici nesne yalıtımı: {} nesne gizlendi. Katman ayarları korunuyor; Nesne yalıtımını bitir ile geri dönün.", output.hidden)
                } else { "Nesne yalıtımı bitti. Katman görünürlüğü ve süzgeçleri korunuyor.".into() });
            }
            CommandResult::Failed { error } | CommandResult::Conflict { error } => self.warn(error.message),
            CommandResult::Cancelled | CommandResult::Queued { .. } | CommandResult::NeedsInput { .. } => {}
        }
    }
}

/// Kaynağı (docs/adr/1200 §4): the object's role in the map's work, its item, the source it was derived from, the
/// source's revision then (its first 12 hex digits), the operation and whether it is up to date; none outside a map
/// project's work structure.
/// An accepted GNSS point's measurement (docs/adr/1340 §1, §6): its mark, record and revision, its state and why,
/// its method and solutions with their roles, the mark's height, σ, the epoch it is given in, the coordinate chain and
/// its older results; or the accepted points an object was made from. HDOP and a fix are not accuracies.
fn survey_section(doc: &Document, slot: Slot) -> Option<rows::Section> {
    let uid = doc.model.uid(slot)?.to_string();
    let found = kentos_native_application::map_survey::object_of(&doc.model, &uid)?;
    let mut out = Vec::new();
    if let Some(p) = &found.point {
        let r = &p.result;
        out.push(rows::Row::text("İşaret", p.mark.clone()));
        out.push(rows::Row::text("Kabul edilen nokta", format!("{} · {}. revizyon", p.id, r.revision)));
        out.push(rows::Row::text("Durum", crate::map_survey_panel::state_words(p.state)));
        for why in &p.why {
            out.push(rows::Row::note(why));
        }
        out.push(rows::Row::text("Yöntem", r.method.title()));
        for s in &found.solutions {
            let mut roles = Vec::new();
            if s.latest {
                roles.push("son ölçülen");
            }
            if s.accepted {
                roles.push("kabul edilen");
            }
            if s.adjusted {
                roles.push("dengelemede");
            }
            if s.excluded.is_some() {
                roles.push("dışlandı");
            }
            out.push(rows::Row::text(
                s.id.clone(),
                format!("{} · {}", crate::map_survey_panel::kind_words(s.kind), roles.join(", ")),
            ));
        }
        out.push(rows::Row::text("İşaret yüksekliği", format!("{:.3} m (elipsoit)", r.height)));
        if let Some(g) = r.sigma {
            out.push(rows::Row::text("σ K/D/Y", format!("{:.1} / {:.1} / {:.1} mm", g[0] * 1000.0, g[1] * 1000.0, g[2] * 1000.0)));
        }
        out.push(rows::Row::text(
            "Epok",
            r.target_epoch.map_or_else(|| "söylenmedi".to_owned(), |e| format!("{e:.2}")),
        ));
        out.push(rows::Row::text(
            "Koordinat zinciri",
            if r.chain == "complete" { "Tam" } else { "Sınırlı: hassas dönüşüm sayılmaz" },
        ));
        if p.history > 0 {
            out.push(rows::Row::text("Eski sonuç", format!("{} (raporları korunur)", p.history)));
        }
    }
    if let Some(u) = &found.used {
        out.push(rows::Row::text("Dayandığı kayıt", format!("{} · {}", u.id, u.operation)));
        out.push(rows::Row::text("Güncellik", crate::map_survey_panel::state_words(u.state)));
        for why in &u.why {
            out.push(rows::Row::note(why));
        }
    }
    Some(rows::Section { id: "mapSurvey".into(), title: "GNSS ölçümü".into(), rows: out })
}

fn map_work_section(doc: &Document, slot: Slot) -> Option<rows::Section> {
    let uid = doc.model.uid(slot)?.to_string();
    let found = kentos_native_application::map_work::source_of(&doc.model, &uid)?;
    let work = doc.model.map_work()?;
    let name = |id: &Option<String>| {
        id.as_deref().map(|i| work.item(i).map_or_else(|| i.to_owned(), |w| format!("{} ({i})", w.name)))
    };
    let role = match found.role {
        kentos_commands::WorkObjectRole::Source => "Kaynak veri",
        kentos_commands::WorkObjectRole::Derived => "Kaynaktan türetildi",
        kentos_commands::WorkObjectRole::Free => "Bağımsız çizim",
    };
    let mut out = vec![rows::Row::text("Rol", role)];
    if let Some(item) = name(&found.item) {
        out.push(rows::Row::text("İş öğesi", item));
    }
    if found.role == kentos_commands::WorkObjectRole::Derived
        && let Some(source) = name(&found.source)
    {
        out.push(rows::Row::text("Kaynağı", source));
    }
    if let Some(r) = &found.revision {
        out.push(rows::Row::text("Kaynak revizyonu", r.chars().take(12).collect::<String>()));
    }
    if let Some(op) = &found.operation {
        out.push(rows::Row::text("Üretim işlemi", op.clone()));
    }
    if let Some(current) = found.current {
        out.push(rows::Row::text("Güncellik", if current { "Güncel" } else { "Güncel değil: kaynağı değişti" }));
    }
    out.push(rows::Row::text("Kimlik", uid));
    Some(rows::Section { id: "mapWork".into(), title: "Kaynağı".into(), rows: out })
}

mod panel;

mod commit;
use commit::*;
