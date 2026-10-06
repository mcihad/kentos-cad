//! The tool session (ADR 0018, TODOS.md UX-01): the web's `ToolManager`
//! (`apps/web/src/tools/ToolManager.ts`) without the DOM.
//!
//! | State | Here |
//! |---|---|
//! | Boşta (idle) | no tool: the pointer selects ([`Select`], docs/adr/0029); the traces read the tool as `select` |
//! | Çalışıyor (running) | a tool waits for a point, a number or an option ([`Session::prompt`]) |
//! | Önizleme (preview) | pointer moves change only [`Session::preview`], never the drawing |
//! | Onay (confirm) | [`Session::confirm`]: the tool commits one undo step and stays for the next object |
//! | İptal (cancel) | [`Session::exit`]: the draft is dropped, nothing reaches the drawing or its history |
//! | Askıda (suspended) | [`Session::nest`]: the point calculator runs over a command waiting for a point (or a grip being moved); its point goes to the command as if clicked, Esc brings the command back as it was (docs/adr/0083) |
//!
//! “Repeat the last command” and what a key means are the host's: it asks
//! [`Session::last`] and routes keys by ADR 0018's order. So is the object
//! snap: the host asks [`Session::snap`] with the pointer and gives the tool
//! the snapped pointer, as the web's viewport does before its tools. Kaydır
//! is not remembered for repeat, nor is a tool that is not in the catalog
//! ([`Session::run`]: Yapıştır), as on the web (docs/adr/0056).

use kentos_geometry_core::store::snap::{SnapExtras, SnapHit, SnapKind};
use kentos_geometry_core::tools::locks::{LockText, parse_lock_text};
use kentos_geometry_core::tools::point_text::point_name;

use crate::Vec2;
use crate::align::{self, Align};
use crate::arc::{self, Arc};
use crate::array::{self, Array};
use crate::array_path;
use crate::breaking::{self, Break};
use crate::circle::{self, Circle};
use crate::corner::{self, CornerTool};
use crate::erase::{self, Erase};
use crate::format::Format;
use crate::lengthen::{self, Lengthen};
use crate::line::{self, Line};
use crate::locks::{LockPick, NO_LOCK_EDGE, Toward, picked_edge};
use crate::log::Level;
use crate::mirror::{self, Mirror};
use crate::move_copy::{self, Move};
use crate::navigate::{self, Pan, ZoomWindow};
use crate::object::{self, ObjectAction};
use crate::object_tracking::{Aids, ObjectTracking, TRACK_PX};
use crate::offset::{self, Offset};
use crate::path::{self, Path};
use crate::perpendicular::{self, Perpendicular};
use crate::point::{self, Point};
use crate::polar::{self, Polar};
use crate::prompt::Prompt;
use crate::rectangle::{self, Rectangle};
use crate::regular::{self, RegularPolygon};
use crate::reshape::{self, Reshape};
use crate::rotate::{self, Rotate};
use crate::rotated::{self, RotatedRectangle};
use crate::scale::{self, Scale};
use crate::select::{Select, SelectBox};
use crate::spatial::Spatial;
use crate::stretch::{self, Stretch};
use crate::tool::{Context, Cursor, Draft, Flow, Pointer, Preview, Tool, View, screen_scale};
use crate::trim::{self, Boundary};
use crate::vertex::{self, Vertex};
use crate::{
    adjoin, angle, area, between, block_define, block_insert, boundary, cleanup, construction,
    continuation, coordinate, dimension, dimension_chain, divide, donut, ellipse, hatch, holes,
    labels_to_text, layer_move, layer_tools, leader, match_properties, meeting, paragraph,
    parallel, polygonize, quick_dimension, reshape_by, revcloud, sector, select_circle,
    select_containing, select_fence, set_elevation, spline, split, station_offset, text, text_file,
    topology, vertex_points,
};

/// Ids of the tools the session runs; each is the web command `tool.<id>`.
pub const TOOLS: &[&str] = &[
    path::POLYGON_ID,
    line::ID,
    path::POLYLINE_ID,
    // Mesafe ölç, Alan hesapla and Parsel oluştur: the path tool's other shapes (docs/adr/0067).
    path::MEASURE_ID,
    path::AREA_ID,
    path::PARCEL_ID,
    // Bitişik alan: the path tool's shape that closes on the neighbouring areas (docs/adr/0162 §3).
    adjoin::ID,
    erase::ID,
    point::ID,
    circle::ID,
    arc::ID,
    rectangle::ID,
    rotated::ID,
    regular::ID,
    move_copy::MOVE_ID,
    move_copy::COPY_ID,
    rotate::ID,
    scale::ID,
    mirror::ID,
    stretch::ID,
    array::ID,
    polar::ID,
    align::ID,
    offset::ID,
    trim::TRIM_ID,
    trim::EXTEND_ID,
    corner::FILLET_ID,
    corner::CHAMFER_ID,
    breaking::ID,
    object::JOIN_ID,
    object::EXPLODE_ID,
    object::READABLE_ID,
    lengthen::ID,
    vertex::ID,
    navigate::PAN_ID,
    navigate::ZOOM_WINDOW_ID,
    // Drawing tools, round 3 (docs/adr/0057).
    ellipse::ID,
    spline::ID,
    construction::XLINE_ID,
    construction::RAY_ID,
    parallel::ID,
    perpendicular::IN_ID,
    perpendicular::OUT_ID,
    donut::ID,
    revcloud::ID,
    point::SPOT_ID,
    divide::ID,
    text::ID,
    paragraph::ID,
    text_file::ID,
    leader::ID,
    dimension::ID,
    hatch::ID,
    // Blocks (docs/adr/0144).
    block_define::ID,
    block_insert::ID,
    // Alan işlemleri (docs/adr/0065).
    area::UNION_ID,
    area::INTERSECT_ID,
    area::SUBTRACT_ID,
    area::SPLIT_ID,
    area::TO_AREA_ID,
    area::TO_POLYLINE_ID,
    area::PARTS_JOIN_ID,
    area::PARTS_SPLIT_ID,
    boundary::ID,
    // Drawing and editing tools of docs/adr/0140, phase 1.
    reshape::FILLET_ALL_ID,
    reshape::CHAMFER_ALL_ID,
    reshape::REVERSE_ID,
    reshape::SIMPLIFY_ID,
    split::ID,
    cleanup::ID,
    match_properties::ID,
    // Drawing tools of docs/adr/0140, phase 2.
    sector::ID,
    between::ID,
    meeting::ID,
    angle::ID,
    coordinate::ID,
    dimension_chain::CONTINUE_ID,
    dimension_chain::BASELINE_ID,
    // docs/adr/0147 §7: Hızlı ölçü.
    quick_dimension::ID,
    // Phase 3: the array along a path (Çit, İki yana and Kaynağı sil are options of existing tools).
    array_path::ID,
    // docs/adr/0141: Dik ayak ölç, then the selecting tools that go back to Seç.
    station_offset::ID,
    select_fence::ID,
    select_circle::ID,
    select_containing::ID,
    // docs/adr/0142: Kot ver.
    set_elevation::ID,
    // docs/adr/0148: Topolojik temizlik.
    topology::ID,
    // docs/adr/0151: Toplu alan.
    polygonize::ID,
    // docs/adr/0152: Köşelere nokta.
    vertex_points::ID,
    // docs/adr/0175 §3: Etiketleri yazıya çevir.
    labels_to_text::ID,
    // docs/adr/0173 §5: Delik ekle, Deliği sil, Deliği doldur.
    holes::ADD_ID,
    holes::REMOVE_ID,
    holes::FILL_ID,
    // docs/adr/0173 §4: Sürdür, the path tool's shape that continues a line or a polyline.
    continuation::ID,
    // docs/adr/0173 §2–§3: Biçim değiştir, the path tool's shape that reshapes an area or a path.
    reshape_by::ID,
    // docs/adr/0177 §1: the layer actions by an object.
    layer_tools::OFF_ID,
    layer_tools::ISOLATE_ID,
    layer_tools::LOCK_ID,
    layer_tools::ACTIVE_ID,
    // docs/adr/0177 §2: Katmanı eşle and Katmana kopyala.
    layer_move::MATCH_ID,
    layer_move::COPY_ID,
];

/// What a tool running over another one suspended (docs/adr/0083).
enum Suspended {
    /// A command waiting for a point.
    Tool(Box<dyn Tool>),
    /// No command: the select tool with a grip being moved.
    Grip,
}

/// The running tool, if any, the last one started, and the select tool that
/// has the pointer while none runs.
#[derive(Default)]
pub struct Session {
    tool: Option<Box<dyn Tool>>,
    /// The command suspended under the running tool (the point calculator's).
    parent: Option<Suspended>,
    last: Option<&'static str>,
    select: Select,
    /// Nesneye paralel or dik waiting for its edge (docs/adr/0166 §3).
    lock_pick: Option<LockPick>,
    /// The press that picked the edge, or set the reference: its release
    /// does not reach the tool.
    pick_pressed: bool,
    /// Referans noktası's or Yapım kipi's point (docs/adr/0166 §5).
    reference: Option<Vec2>,
    /// The tool's own last point when the reference was set: once it
    /// moves (a corner was placed), a one-shot reference is done.
    reference_from: Option<Vec2>,
    /// Referans noktası asked: the next press or typed point is the reference.
    reference_wait: bool,
    /// Yapım kipi: every press and typed point renews the reference.
    construction: bool,
    /// The pointer's last place, for a distance typed as a reference.
    cursor: Option<Vec2>,
    /// How many tools have started ([`Session::runs`]).
    runs: u64,
}

/// What the web says when the command takes no point at its step.
pub const NO_POINT_NOW: &str =
    "Çalışan araç şu adımda nokta beklemiyor; hesaplanan nokta kullanılmadı.";

/// A lock asked for with no point to measure it from (docs/adr/0166 §1; the web's words).
pub const NO_LOCK_REFERENCE: &str =
    "Kilit için önce bir nokta verin: uzunluk ve doğrultu son noktadan ölçülür.";

/// Sapma asked for with no edge to turn from.
pub const NO_TRAVEL: &str =
    "Sapma için önce bir kenar çizin: sapma önceki kenarın doğrultusundan ölçülür.";

/// What Esc and Kilitleri kaldır say.
pub const LOCKS_GONE: &str = "Kilitler kaldırıldı.";

/// Referans noktası or Yapım kipi asked where no command takes points.
pub const NO_REFERENCE_TOOL: &str =
    "Referans noktası, nokta bekleyen bir komut çalışırken verilir.";

impl Session {
    pub fn new() -> Self {
        Self::default()
    }

    /// The ids of the tools this session can run.
    pub fn tools() -> &'static [&'static str] {
        TOOLS
    }

    /// Starts a tool by id (`polygon`), dropping whatever ran. False for an
    /// id the session does not know; nothing changes then. The host calls
    /// [`Session::activate`] right after.
    pub fn start(&mut self, id: &str) -> bool {
        let tool: Box<dyn Tool> = match id {
            path::POLYGON_ID => Box::new(Path::polygon()),
            path::POLYLINE_ID => Box::new(Path::polyline()),
            path::MEASURE_ID => Box::new(Path::new(path::Shape::MeasureLength)),
            path::AREA_ID => Box::new(Path::new(path::Shape::MeasureArea)),
            path::PARCEL_ID => Box::new(Path::new(path::Shape::Parcel)),
            adjoin::ID => Box::new(Path::new(path::Shape::Adjoin)),
            line::ID => Box::new(Line::new()),
            erase::ID => Box::new(Erase::new()),
            point::ID => Box::new(Point::new()),
            circle::ID => Box::new(Circle::new()),
            arc::ID => Box::new(Arc::new()),
            rectangle::ID => Box::new(Rectangle::new()),
            rotated::ID => Box::new(RotatedRectangle::new()),
            regular::ID => Box::new(RegularPolygon::new()),
            move_copy::MOVE_ID => Box::new(Move::tool()),
            move_copy::COPY_ID => Box::new(Move::copy_tool()),
            rotate::ID => Box::new(Rotate::tool()),
            scale::ID => Box::new(Scale::tool()),
            mirror::ID => Box::new(Mirror::tool()),
            stretch::ID => Box::new(Stretch::new()),
            array::ID => Box::new(Array::tool()),
            polar::ID => Box::new(Polar::tool()),
            align::ID => Box::new(Align::tool()),
            offset::ID => Box::new(Offset::new()),
            trim::TRIM_ID => Box::new(Boundary::trim()),
            trim::EXTEND_ID => Box::new(Boundary::extend()),
            corner::FILLET_ID => Box::new(CornerTool::fillet()),
            corner::CHAMFER_ID => Box::new(CornerTool::chamfer()),
            breaking::ID => Box::new(Break::new()),
            object::JOIN_ID => Box::new(ObjectAction::join()),
            object::EXPLODE_ID => Box::new(ObjectAction::explode()),
            object::READABLE_ID => Box::new(ObjectAction::readable()),
            lengthen::ID => Box::new(Lengthen::new()),
            vertex::ID => Box::new(Vertex::new()),
            navigate::PAN_ID => Box::new(Pan::new()),
            navigate::ZOOM_WINDOW_ID => Box::new(ZoomWindow::new()),
            ellipse::ID => Box::new(crate::ellipse::Ellipse::new()),
            spline::ID => Box::new(crate::spline::Spline::new()),
            construction::XLINE_ID => Box::new(crate::construction::Xline::new()),
            construction::RAY_ID => Box::new(crate::construction::Ray::new()),
            parallel::ID => Box::new(crate::parallel::Parallel::new()),
            perpendicular::IN_ID => Box::new(Perpendicular::perpendicular_in()),
            perpendicular::OUT_ID => Box::new(Perpendicular::perpendicular_out()),
            donut::ID => Box::new(crate::donut::Donut::new()),
            revcloud::ID => Box::new(crate::revcloud::RevCloud::new()),
            point::SPOT_ID => Box::new(Point::spot()),
            divide::ID => Box::new(crate::divide::Divide::new()),
            text::ID => Box::new(crate::text::Text::new()),
            paragraph::ID => Box::new(crate::paragraph::ParagraphText::new()),
            text_file::ID => Box::new(crate::text_file::PlaceTextFile::new()),
            leader::ID => Box::new(crate::leader::Leader::new()),
            dimension::ID => Box::new(crate::dimension::Dimension::new()),
            hatch::ID => Box::new(crate::hatch::Hatch::new()),
            block_define::ID => Box::new(crate::block_define::BlockDefine::tool()),
            block_insert::ID => Box::new(crate::block_insert::BlockInsert::new()),
            area::UNION_ID => Box::new(crate::area::AreaAction::union()),
            area::INTERSECT_ID => Box::new(crate::area::AreaAction::intersect()),
            area::SUBTRACT_ID => Box::new(crate::area::AreaSubtract::tool()),
            area::SPLIT_ID => Box::new(crate::area::AreaSplit::tool()),
            area::TO_AREA_ID => Box::new(crate::area::AreaAction::to_area()),
            area::TO_POLYLINE_ID => Box::new(crate::area::AreaAction::to_polyline()),
            area::PARTS_JOIN_ID => Box::new(crate::area::AreaAction::parts_join()),
            area::PARTS_SPLIT_ID => Box::new(crate::area::AreaAction::parts_split()),
            boundary::ID => Box::new(crate::boundary::Boundary::new()),
            holes::ADD_ID => Box::new(Path::new(path::Shape::Hole)),
            continuation::ID => Box::new(Path::new(path::Shape::Continue)),
            reshape_by::ID => Box::new(Path::new(path::Shape::Reshape)),
            holes::REMOVE_ID => Box::new(crate::holes::HoleClick::remove()),
            holes::FILL_ID => Box::new(crate::holes::HoleClick::fill()),
            reshape::FILLET_ALL_ID => Box::new(Reshape::fillet_all()),
            reshape::CHAMFER_ALL_ID => Box::new(Reshape::chamfer_all()),
            reshape::REVERSE_ID => Box::new(Reshape::reverse()),
            reshape::SIMPLIFY_ID => Box::new(Reshape::simplify()),
            split::ID => Box::new(crate::split::Split::new()),
            cleanup::ID => Box::new(crate::cleanup::Cleanup::new()),
            match_properties::ID => Box::new(crate::match_properties::MatchProperties::new()),
            layer_tools::OFF_ID => Box::new(layer_tools::LayerTool::off()),
            layer_tools::ISOLATE_ID => Box::new(layer_tools::LayerTool::isolate()),
            layer_tools::LOCK_ID => Box::new(layer_tools::LayerTool::lock()),
            layer_tools::ACTIVE_ID => Box::new(layer_tools::LayerTool::make_active()),
            layer_move::MATCH_ID => Box::new(layer_move::LayerMove::layer_match()),
            layer_move::COPY_ID => Box::new(layer_move::LayerMove::copy_to_layer()),
            sector::ID => Box::new(crate::sector::Sector::new()),
            between::ID => Box::new(crate::between::PointsBetween::new()),
            meeting::ID => Box::new(crate::meeting::IntersectPoint::new()),
            angle::ID => Box::new(crate::angle::MeasureAngle::new()),
            coordinate::ID => Box::new(crate::coordinate::CrsQuery::new()),
            dimension_chain::CONTINUE_ID => {
                Box::new(crate::dimension_chain::DimensionChain::continued())
            }
            dimension_chain::BASELINE_ID => {
                Box::new(crate::dimension_chain::DimensionChain::baseline())
            }
            quick_dimension::ID => Box::new(crate::quick_dimension::QuickDimension::tool()),
            array_path::ID => Box::new(crate::array_path::ArrayPath::tool()),
            station_offset::ID => Box::new(station_offset::StationOffset::new()),
            select_fence::ID => Box::new(select_fence::SelectFence::new()),
            select_circle::ID => Box::new(select_circle::SelectCircle::new()),
            select_containing::ID => Box::new(select_containing::SelectContaining::new()),
            set_elevation::ID => Box::new(set_elevation::SetElevation::tool()),
            topology::ID => Box::new(crate::topology::Topology::new()),
            labels_to_text::ID => Box::new(crate::labels_to_text::LabelsToText::new()),
            polygonize::ID => Box::new(crate::polygonize::Polygonize::new()),
            vertex_points::ID => Box::new(crate::vertex_points::VertexPoints::tool()),
            _ => return false,
        };
        // Kaydır is not repeated: Enter while panning repeats the command
        // before it (the web's `lastRepeatable`).
        if tool.id() != navigate::PAN_ID {
            self.last = Some(tool.id());
        }
        self.run(tool);
        true
    }

    /// Runs a tool that is not in the catalog (Yapıştır with the clipboard's
    /// objects), dropping whatever ran; it is not remembered for repeat (the
    /// web's `ToolManager.run`). The host calls [`Session::activate`] right after.
    pub fn run(&mut self, tool: Box<dyn Tool>) {
        self.runs += 1;
        self.tool = Some(tool);
        self.parent = None;
        self.select.reset();
    }

    /// Runs `child` over the running command without ending it (the point
    /// calculator; the web's `ToolManager.nest`, ADR 0018 “Askıda”). The
    /// command keeps its state. When the child finishes, its point goes to
    /// the command as if clicked; Esc brings the command back as it was.
    /// With no command, it runs over a grip being moved. `label` is what the
    /// command line says (“Nokta hesabı: Kenar kesişimi”).
    pub fn nest(&mut self, child: Box<dyn Tool>, label: String, cx: &mut Context<'_>) {
        if self.parent.is_some() {
            return;
        }
        let parent = match self.tool.take() {
            Some(tool) => Suspended::Tool(tool),
            None if self.select.grip_active() => Suspended::Grip,
            None => return,
        };
        self.parent = Some(parent);
        cx.say(Level::Command, label);
        self.tool = Some(child);
        if self
            .tool
            .as_mut()
            .is_some_and(|t| t.activate(cx) == Flow::Exit)
        {
            self.unnest(None, cx);
        }
    }

    /// Whether a tool runs over a suspended command.
    pub fn nested(&self) -> bool {
        self.parent.is_some()
    }

    /// Whether the point calculator may run now (the web's `canCalcPoint`):
    /// a command that takes points and snaps, or a grip being moved; not
    /// over another calculator.
    pub fn can_calc_point(&self) -> bool {
        if self.parent.is_some() {
            return false;
        }
        match &self.tool {
            Some(tool) => tool.accepts_points() && tool.snaps(),
            None => self.select.grip_active(),
        }
    }

    /// Ends the tool run over the suspended command, which goes on; `point`
    /// reaches it as if clicked (the web's `unnest`).
    fn unnest(&mut self, point: Option<Vec2>, cx: &mut Context<'_>) {
        let Some(parent) = self.parent.take() else {
            self.tool = None;
            return;
        };
        self.tool = match parent {
            Suspended::Tool(tool) => Some(tool),
            Suspended::Grip => None,
        };
        let Some(p) = point else {
            return;
        };
        let taken = match &mut self.tool {
            Some(tool) => tool.accept_point(p, cx),
            None => self.select.accept_point(p, cx),
        };
        if !taken {
            cx.say(Level::Warn, NO_POINT_NOW);
        }
    }

    /// Right after a start: the select tool lets go of the hovered object,
    /// and the new tool may act at once (the erase tool deletes a selection
    /// and leaves; the web's `activate`).
    pub fn activate(&mut self, cx: &mut Context<'_>) {
        // A new command starts with nothing locked (docs/adr/0166 §1).
        cx.locks.reset();
        self.lock_pick = None;
        self.drop_reference();
        cx.selection.set_hover(None);
        if let Some(tool) = &mut self.tool
            && tool.activate(cx) == Flow::Exit
        {
            self.tool = None;
        }
    }

    /// Leaves the running tool; its draft is dropped (ADR 0018, “İptal”),
    /// and so is a command suspended under it.
    pub fn exit(&mut self) {
        self.tool = None;
        self.parent = None;
        self.lock_pick = None;
        self.drop_reference();
    }

    /// No reference, none asked, Yapım kipi off: a new command's start.
    fn drop_reference(&mut self) {
        self.reference = None;
        self.reference_from = None;
        self.reference_wait = false;
        self.construction = false;
    }

    /// Esc: the running tool steps back when it can (the web's `cancel`: an
    /// edge tool drops the object it picked); otherwise it is left. Whether it stays.
    pub fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        // Esc leaves an edge being picked for a lock, then lets the locks go
        // (docs/adr/0166 §1, §3): one step back each.
        if self.lock_pick.take().is_some() {
            return true;
        }
        // Then a reference asked for, Yapım kipi, and the reference with the locks (§5).
        if std::mem::take(&mut self.reference_wait) {
            return true;
        }
        if self.construction {
            self.set_construction(false, cx);
            return true;
        }
        if (cx.locks.any() || self.reference.is_some()) && self.lock_reference().is_some() {
            cx.locks.clear();
            self.reference = None;
            cx.locks.reference = None;
            cx.say(Level::Info, LOCKS_GONE.to_owned());
            return true;
        }
        // No command: Esc leaves a grip being moved where it was.
        if self.tool.is_none() {
            return self.select.cancel();
        }
        let stays = self.tool.as_mut().is_some_and(|t| t.cancel(cx));
        if stays {
            return true;
        }
        // Over a suspended command: it comes back as it was.
        if self.parent.is_some() {
            self.unnest(None, cx);
            return true;
        }
        self.tool = None;
        false
    }

    pub fn is_running(&self) -> bool {
        self.tool.is_some()
    }

    /// The running command's id; `select` when none runs, as the traces
    /// read it. Under the point calculator it is the suspended command's.
    pub fn tool_id(&self) -> &'static str {
        match &self.parent {
            Some(Suspended::Tool(tool)) => tool.id(),
            Some(Suspended::Grip) => "select",
            None => self.tool.as_ref().map_or("select", |t| t.id()),
        }
    }

    /// The running tool's name (`Kapalı alan`).
    pub fn label(&self) -> Option<&'static str> {
        self.tool.as_ref().map(|t| t.label())
    }

    /// The last tool started, for Enter or Space with no command (repeat).
    pub fn last(&self) -> Option<&'static str> {
        self.last
    }

    /// How many tools have started in this session (a nested one not): what a
    /// host keeps for one run (an object template's, docs/adr/0176 §3) ends
    /// when this moves on or no tool runs.
    pub fn runs(&self) -> u64 {
        self.runs
    }

    /// Points the running command has taken; 0 when none runs.
    pub fn point_count(&self) -> usize {
        self.tool.as_ref().map_or(0, |t| t.point_count())
    }

    /// Whether a running tool takes Enter, Space and a quick right click as
    /// its confirm. Kaydır and Pencere yakınlaştır do not: Enter repeats the
    /// last command instead (the web's `tool.confirm`, docs/adr/0056). With no
    /// command, a grip being moved takes them (docs/adr/0068).
    pub fn confirms(&self) -> bool {
        self.tool
            .as_ref()
            .map_or(self.select.grip_active(), |t| t.confirms())
    }

    /// Whether the running command's step asks for words: Space is a letter
    /// in the command line, not a second Enter (docs/adr/0183 §4).
    pub fn takes_words(&self) -> bool {
        self.tool.as_ref().is_some_and(|t| t.takes_words())
    }

    /// Whether no command runs and a grip of the selection is being moved
    /// (docs/adr/0068): typed points, Enter and Esc are the grip's then.
    pub fn grip_active(&self) -> bool {
        self.tool.is_none() && self.select.grip_active()
    }

    /// The grip being moved, drawn larger (the web's `activeGrip`).
    pub fn active_grip(&self) -> Option<(kentos_domain::Slot, usize)> {
        if self.tool.is_some() {
            return None;
        }
        self.select.active_grip()
    }

    /// The pointer's look over the drawing: the running tool's, else the
    /// select tool's pick (the web's `SelectTool.cursor`).
    pub fn cursor(&self) -> Cursor {
        // An edge awaited for a lock is picked, whatever the tool's look (docs/adr/0166 §3).
        if self.lock_pick.is_some() {
            return Cursor::Pick;
        }
        self.tool.as_ref().map_or(Cursor::Pick, |t| t.cursor())
    }

    pub fn prompt(&self) -> Prompt {
        let prompt = self
            .tool
            .as_ref()
            .map_or_else(|| self.select.prompt(), |t| t.prompt());
        // An edge awaited for a lock, a reference asked for and Yapım kipi
        // speak for the step (docs/adr/0166 §3, §5).
        let step = match self.lock_pick {
            Some(pick) => pick.step(),
            None if self.reference_wait => "referans noktasını belirtin",
            None if self.construction && self.tool.is_some() => {
                "yapım noktasını belirtin; köşe olmaz"
            }
            None => return prompt,
        };
        match prompt.tool {
            Some(tool) => Prompt::new(tool, step),
            None => Prompt::untitled(step),
        }
        .option("Vazgeç", "Esc")
    }

    /// Referans noktası's or Yapım kipi's point, if one is set (docs/adr/0166 §5).
    pub fn reference(&self) -> Option<Vec2> {
        self.reference
    }

    /// Whether Yapım kipi is on.
    pub fn construction(&self) -> bool {
        self.construction
    }

    /// Whether a command runs that takes points: Referans noktası and
    /// Yapım kipi work then, before its first corner too.
    pub fn takes_points(&self) -> bool {
        self.tool
            .as_ref()
            .is_some_and(|t| t.accepts_points() && t.snaps())
    }

    /// Referans noktası (docs/adr/0166 §5): the next press or typed point
    /// is the reference the locks and relative input are measured from,
    /// not a corner; false (and why) when no command takes points.
    pub fn ask_reference(&mut self, cx: &mut Context<'_>) -> bool {
        if !self.takes_points() {
            cx.say(Level::Warn, NO_REFERENCE_TOOL.to_owned());
            return false;
        }
        self.reference_wait = true;
        true
    }

    /// Yapım kipi on or off: while on, every press and typed point renews
    /// the reference; off, the next point is a corner, measured from it.
    pub fn set_construction(&mut self, on: bool, cx: &mut Context<'_>) -> bool {
        if on && !self.takes_points() {
            cx.say(Level::Warn, NO_REFERENCE_TOOL.to_owned());
            return false;
        }
        self.construction = on;
        self.reference_wait = false;
        // The reference stays for the next corner, then goes.
        self.reference_from = self.snap_from();
        cx.say(
            Level::Info,
            if on {
                "Yapım kipi açık: tıklanan ve yazılan noktalar köşe olmaz, referansı yeniler."
            } else {
                "Yapım kipi kapalı: sonraki nokta köşedir."
            }
            .to_owned(),
        );
        true
    }

    /// Whether the next press or typed point is a reference rather than a corner.
    fn takes_reference(&self) -> bool {
        self.reference_wait || (self.construction && self.tool.is_some())
    }

    /// The reference is now `p`: the locks follow it, said in the log.
    fn set_reference(&mut self, p: Vec2, cx: &mut Context<'_>) {
        self.reference = Some(p);
        self.reference_from = self.snap_from();
        self.reference_wait = false;
        cx.locks.reference = Some(p);
        let line = format!("Referans noktası: {}", cx.format().point(p));
        cx.say(Level::Info, line);
        self.follow_locks(cx);
    }

    /// What the cursor rule reads of the session before an event: the
    /// tool's direction (Sapma, Dik açı) and the reference point.
    fn prime(&self, cx: &mut Context<'_>) {
        cx.locks.travel = self.travel();
        cx.locks.reference = self.reference;
    }

    /// Nesneye paralel or dik waiting for its edge, if one is.
    pub fn lock_pick(&self) -> Option<LockPick> {
        self.lock_pick
    }

    /// Nesneye paralel or dik (docs/adr/0166 §3): the next press on the
    /// drawing picks the edge the direction is taken from; false (and why)
    /// with no point to measure the lock from.
    pub fn pick_lock_edge(&mut self, pick: LockPick, cx: &mut Context<'_>) -> bool {
        if self.lock_reference().is_none() {
            cx.say(Level::Warn, NO_LOCK_REFERENCE.to_owned());
            return false;
        }
        self.lock_pick = Some(pick);
        true
    }

    /// The press while an edge is awaited: the edge under it gives the
    /// direction lock, or the wait goes on and says why.
    fn pick_edge_now(&mut self, pick: LockPick, p: &Pointer, cx: &mut Context<'_>) {
        let Some((edge, u)) = picked_edge(p.raw, cx) else {
            cx.say(Level::Warn, NO_LOCK_EDGE.to_owned());
            return;
        };
        self.lock_pick = None;
        if self.lock_toward(pick.toward(u), cx) {
            cx.locks.edge = Some(edge);
        }
    }

    /// The object snap for the pointer at `at` (the web's `updateSnap`):
    /// while a tool that snaps runs and snapping is on, the store's snap
    /// point among the drafting kinds within the aperture, perpendicular and
    /// tangent from the tool's last point. None otherwise: the erase tool
    /// does not snap, the select tool only while a grip moves (from where it was).
    /// Whether object tracking follows the cursor: a command runs and snaps
    /// (the web's: not the select tool, not a tool that takes no snap).
    pub fn tracks(&self) -> bool {
        self.tool.as_ref().is_some_and(|t| t.snaps())
    }

    /// The running command's last point, for object tracking's crossings.
    pub fn snap_from(&self) -> Option<Vec2> {
        self.tool
            .as_ref()
            .filter(|t| t.snaps())
            .and_then(|t| t.snap_from())
    }

    pub fn snap(
        &self,
        spatial: &Spatial,
        at: Vec2,
        view: &dyn View,
        draft: &Draft,
        tracking: &ObjectTracking,
    ) -> Option<SnapHit> {
        let (from, path) = match &self.tool {
            Some(tool) if tool.snaps() => (tool.snap_from(), tool.draft_path()),
            None if self.select.grip_active() => (self.select.snap_from(), None),
            _ => return None,
        };
        // Out of the snap's scale range no snap applies (docs/adr/0163 §5).
        if !draft.snap || !draft.snap_in_range(screen_scale(view.world_length(1.0))) {
            return None;
        }
        let tol = view.world_length(draft.snap_aperture);
        let extras = SnapExtras {
            // What rests acquired: ends' extensions and edges' directions (§2).
            extensions: tracking.extensions(),
            parallels: tracking.parallels(),
            // The object being drawn, with `snap.self` (§3).
            draft: path.filter(|_| draft.snap_self).into_iter().collect(),
            grid: draft
                .snap_grid
                .iter()
                .all(|&g| g > 0.0)
                .then_some(draft.snap_grid),
        };
        spatial.snap_ex(at, tol, draft.snap_kinds, from, &extras)
    }

    /// After the snap for the cursor at `at`: object tracking and the snap
    /// additions' rests follow it (docs/adr/0085, 0163 §2), as the desktop's
    /// pointer does. `draft` is the one the snap was taken with (a one-shot
    /// snap's kinds); Paralel rests on the straight edge under the cursor
    /// when no point is snapped there.
    pub fn follow(
        &self,
        tracking: &mut ObjectTracking,
        spatial: &Spatial,
        at: Vec2,
        view: &dyn View,
        draft: &Draft,
        snap: Option<&SnapHit>,
    ) {
        let aids = if self.tracks() {
            draft.aids(screen_scale(view.world_length(1.0)))
        } else {
            Aids::default()
        };
        let edge = (aids.parallel && snap.is_none_or(|s| s.kind == SnapKind::Nearest))
            .then(|| spatial.direction_at(at, view.world_length(draft.snap_aperture)))
            .flatten();
        tracking.update(
            aids,
            snap,
            at,
            self.snap_from(),
            draft.polar,
            view.world_length(TRACK_PX),
            edge,
        );
    }

    pub fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.prime(cx);
        self.cursor = Some(p.world);
        match &mut self.tool {
            Some(tool) => tool.pointer_move(p, cx),
            None => self.select.pointer_move(p, cx),
        }
        // Akış leaves corners as the cursor moves: the locks follow them too.
        self.follow_locks(cx);
    }

    /// The left button went down on the drawing.
    pub fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.prime(cx);
        if let Some(pick) = self.lock_pick {
            self.pick_pressed = true;
            self.pick_edge_now(pick, p, cx);
            return;
        }
        // Referans noktası and Yapım kipi: the press is the reference, not a corner (§5).
        if self.takes_reference() {
            self.pick_pressed = true;
            self.set_reference(p.world, cx);
            return;
        }
        match &mut self.tool {
            Some(tool) => tool.pointer_down(p, cx),
            None => self.select.pointer_down(p, cx),
        }
        self.settle(cx);
    }

    /// The left button came up (on the drawing, or wherever a press on it ended).
    pub fn pointer_up(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.prime(cx);
        // The release of the press that picked a lock's edge is the pick's.
        if std::mem::take(&mut self.pick_pressed) {
            return;
        }
        match &mut self.tool {
            Some(tool) => tool.pointer_up(p, cx),
            None => self.select.pointer_up(p, cx),
        }
        self.settle(cx);
    }

    /// The point the locks are measured from (docs/adr/0166 §1): the running
    /// command's last point, or where a grip being moved was; none when no
    /// point is expected after another.
    pub fn lock_reference(&self) -> Option<Vec2> {
        match &self.tool {
            Some(_) => self.reference.or_else(|| self.snap_from()),
            None if self.select.grip_active() => self.select.snap_from(),
            None => None,
        }
    }

    /// The unit direction the object being drawn travels at its last point:
    /// Sapma turns from it (docs/adr/0166 §1).
    pub fn travel(&self) -> Option<Vec2> {
        self.tool.as_ref().and_then(|t| t.travel())
    }

    /// Locks the next point's length (metres) at the lock reference, said
    /// on the command line; false (and why) without a reference.
    pub fn lock_length(&self, metres: f64, cx: &mut Context<'_>) -> bool {
        let Some(at) = self.lock_reference() else {
            cx.say(Level::Warn, NO_LOCK_REFERENCE.to_owned());
            return false;
        };
        cx.locks.lock_length(metres, at);
        self.say_locks(cx);
        true
    }

    /// Locks the next point's direction at the lock reference; a deflection
    /// needs an edge to turn from. False (and why) when it cannot.
    pub fn lock_toward(&self, toward: Toward, cx: &mut Context<'_>) -> bool {
        let Some(at) = self.lock_reference() else {
            cx.say(Level::Warn, NO_LOCK_REFERENCE.to_owned());
            return false;
        };
        if matches!(toward, Toward::Deflection(_)) && self.travel().is_none() {
            cx.say(Level::Warn, NO_TRAVEL.to_owned());
            return false;
        }
        self.prime(cx);
        cx.locks.lock_toward(toward, at);
        self.say_locks(cx);
        true
    }

    /// A point computed elsewhere (the locks' Enter in the value field,
    /// docs/adr/0166 §6), given to the running tool as if clicked, or to a
    /// grip being moved; false when the step takes none.
    pub fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        self.prime(cx);
        let taken = match &mut self.tool {
            Some(tool) => tool.accepts_points() && tool.accept_point(p, cx),
            None => self.select.accept_point(p, cx),
        };
        if !taken {
            cx.say(Level::Warn, NO_POINT_NOW.to_owned());
        }
        self.settle(cx);
        taken
    }

    /// Kalıcı on or off for the running command (docs/adr/0166 §1).
    pub fn keep_locks(&self, keep: bool, cx: &mut Context<'_>) {
        cx.locks.keep = keep;
        cx.say(
            Level::Info,
            if keep {
                "Kilitler kalıcı: sonraki noktalarda da durur."
            } else {
                "Kilitler tek seferlik: nokta konunca kalkar."
            }
            .to_owned(),
        );
    }

    /// Kilitleri kaldır.
    pub fn clear_locks(&self, cx: &mut Context<'_>) {
        if cx.locks.any() {
            cx.locks.clear();
            cx.say(Level::Info, LOCKS_GONE.to_owned());
        }
    }

    /// What is locked now, on the command line: “Kilit: Uzunluk 12.500 m · Semt 100.0000 g.”
    fn say_locks(&self, cx: &mut Context<'_>) {
        let words = cx.locks.words(&cx.format());
        cx.say(Level::Info, format!("Kilit: {}.", words.join(" · ")));
    }

    /// After an event: the locks follow the reference (one-shot ones go when
    /// a point was placed); with no command and no grip, nothing is locked.
    fn follow_locks(&mut self, cx: &mut Context<'_>) {
        if self.tool.is_none() && !self.select.grip_active() {
            cx.locks.reset();
            return;
        }
        // A one-shot reference is done once the tool's own last point moved:
        // a corner was placed from it, or taken back (docs/adr/0166 §5).
        if self.reference.is_some()
            && !self.takes_reference()
            && self.snap_from() != self.reference_from
        {
            self.reference = None;
        }
        self.prime(cx);
        cx.locks.follow(self.lock_reference());
    }

    /// The selection box being drawn: the select tool's while no command
    /// runs, a modify tool's while it picks its objects.
    pub fn select_box(&self) -> Option<SelectBox> {
        match &self.tool {
            Some(tool) => tool.select_box(),
            None => self.select.select_box(),
        }
    }

    /// Typed text for the running tool; false when it does not understand it
    /// or no tool runs.
    /// A text field's answer for the running tool (Yazı).
    pub fn text_typed(&mut self, text: Option<&str>, cx: &mut Context<'_>) {
        if let Some(tool) = self.tool.as_mut() {
            tool.text_typed(text, cx);
        }
        self.settle(cx);
    }

    /// The paragraph editor's answer for the running tool (Çok satırlı yazı, docs/adr/0182 §4).
    pub fn paragraph_typed(
        &mut self,
        typed: Option<(&str, &[kentos_contracts::TextRun])>,
        cx: &mut Context<'_>,
    ) {
        if let Some(tool) = self.tool.as_mut() {
            tool.paragraph_typed(typed, cx);
        }
        self.settle(cx);
    }

    /// The values window's answer for the running tool (Blok ekle).
    pub fn values_given(
        &mut self,
        values: Option<&std::collections::BTreeMap<String, String>>,
        cx: &mut Context<'_>,
    ) {
        if let Some(tool) = self.tool.as_mut() {
            tool.values_given(values, cx);
        }
        self.settle(cx);
    }

    pub fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        // #ad: a point's place by its name (docs/adr/0152 §4).
        if let Some(name) = point_name(text) {
            self.named_point(name, cx);
            self.settle(cx);
            return true;
        }
        // `<45`: the next point's direction locked at that angle (docs/adr/0166 §6).
        if let Some(LockText::Angle(a)) = parse_lock_text(text)
            && (self.tool.is_some() || self.select.grip_active())
        {
            self.lock_toward(Toward::Angle(a), cx);
            return true;
        }
        self.prime(cx);
        // Referans noktası and Yapım kipi: a typed point is the reference (§5).
        if self.takes_reference()
            && let Some(p) = cx.typed_point(text, self.lock_reference(), self.cursor)
        {
            self.set_reference(p, cx);
            return true;
        }
        let taken = match self.tool.as_mut() {
            Some(tool) => tool.input(text, cx),
            // A typed point places a grip being moved.
            None => self.select.input(text, cx),
        };
        self.settle(cx);
        taken
    }

    /// `#ad` typed where a point is asked (docs/adr/0152 §4; the web's
    /// `namedPoint`): the place of the point named so, given to the running
    /// tool as if clicked (as the point calculator gives one), or to a grip
    /// being moved. The name is matched with the points' labels, the spaces
    /// round them dropped; hidden layers count too: a name is an identity.
    /// Why no point was taken is said.
    fn named_point(&mut self, name: &str, cx: &mut Context<'_>) {
        let found: Vec<Vec2> = cx
            .doc
            .entities()
            .filter_map(|e| match e {
                kentos_contracts::Entity::Point(q)
                    if q.base.label.as_deref().map(crate::js_trim) == Some(name) =>
                {
                    Some(Vec2::new(q.p.x, q.p.y))
                }
                _ => None,
            })
            .collect();
        let line = match found.as_slice() {
            [] => format!("#{name}: bu adda nokta yok."),
            [p] => {
                // A tool that takes computed points (the web's `acceptPoint` is there).
                let taken = match &mut self.tool {
                    Some(tool) => tool.accepts_points() && tool.accept_point(*p, cx),
                    None => self.select.accept_point(*p, cx),
                };
                if taken {
                    return;
                }
                format!("#{name}: bu adımda nokta istenmiyor.")
            }
            more => format!(
                "#{name}: bu adda {} nokta var; koordinatı yazın.",
                more.len()
            ),
        };
        cx.say(Level::Warn, line);
    }

    /// The file the running tool asked for (Metin dosyası yerleştir): its
    /// name and bytes, or none (the picker cancelled).
    pub fn file_given(&mut self, file: Option<(&str, &[u8])>, cx: &mut Context<'_>) {
        if let Some(tool) = self.tool.as_mut() {
            tool.file_given(file, cx);
        }
        self.settle(cx);
    }

    /// The values the running tool's option `key` chooses between (Yazı's
    /// Hiza, docs/adr/0145 §6); none without a tool or such an option.
    pub fn option_choices(&self, key: &str) -> Vec<crate::tool::OptionChoice> {
        self.tool
            .as_ref()
            .map_or_else(Vec::new, |tool| tool.option_choices(key))
    }

    /// One of the values of the running tool's option `key` chosen, as
    /// typing the key and then `typed`; false when this step takes none.
    pub fn choose_option(&mut self, key: &str, typed: &str, cx: &mut Context<'_>) -> bool {
        let taken = self
            .tool
            .as_mut()
            .is_some_and(|tool| tool.choose_option(key, typed, cx));
        self.settle(cx);
        taken
    }

    /// A tool that finished with that call leaves: the web's modify tools
    /// call `ctx.tools.exit()` once their transform is written. One run over
    /// a suspended command hands it its point, and the command may finish
    /// with that point in turn.
    fn settle(&mut self, cx: &mut Context<'_>) {
        while self.tool.as_ref().is_some_and(|t| t.finished()) {
            if self.parent.is_some() {
                let point = self.tool.as_ref().and_then(|t| t.computed());
                self.unnest(point, cx);
            } else {
                self.tool = None;
            }
        }
        self.follow_locks(cx);
    }

    /// Enter, Space or a quick right click while a tool runs: it commits what
    /// it has, or leaves when it has nothing (the web's `PointInputTool.confirm`).
    pub fn confirm(&mut self, cx: &mut Context<'_>) {
        match &mut self.tool {
            Some(tool) => {
                if tool.confirm(cx) == Flow::Exit {
                    if self.parent.is_some() {
                        let point = self.tool.as_ref().and_then(|t| t.computed());
                        self.unnest(point, cx);
                        self.settle(cx);
                    } else {
                        self.tool = None;
                    }
                }
            }
            None => {
                self.select.confirm(cx);
            }
        }
        self.follow_locks(cx);
    }

    /// Ctrl+Z while a tool runs: its newest step goes first. False when it
    /// had none (or no tool runs): the drawing is undone then (ADR 0018).
    pub fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        self.tool.as_mut().is_some_and(|t| t.undo_step(cx))
    }

    /// What to draw over the drawing while a tool runs, or a grip moves.
    pub fn preview(&self, format: &Format) -> Option<Preview> {
        match &self.tool {
            Some(tool) => Some(tool.preview(format)),
            None => self.select.preview(format),
        }
    }
}
