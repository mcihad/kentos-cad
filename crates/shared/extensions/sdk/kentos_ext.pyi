# kentos.ext: what an extension's Python uses (`from kentos import ext`). Generated from python/kentos/ext.py by
# `KENTOS_WRITE_SDK=1 cargo test -p kentos-extensions --lib -- sdk_pack`; do not edit.
from dataclasses import dataclass, field
from typing import Any, Callable, Iterable, Iterator, Sequence, TypeVar

F = TypeVar("F", bound=Callable[..., Any])


def command(local: str) -> Callable[[F], F]:
    """A command (``entrypoints.commands``): a button, a menu item, a context menu entry."""


def processing(local: str) -> Callable[[F], F]:
    """An İşlemler tool (``entrypoints.processing``)."""


def audit(local: str) -> Callable[[F], F]:
    """An audit rule (``entrypoints.audits``)."""


def map_tool(local: str) -> Callable[[F], F]:
    """A map tool (``entrypoints.mapTools``)."""


def expr(local: str) -> Callable[[F], F]:
    """An expression function (``entrypoints.expressions``)."""


def marked(fn: Any) -> tuple[str, str] | None:
    """The kind and local id a function is marked with; none when it is not."""


class Stopped(Exception):
    """The user stopped the run (Durdur)."""


class NotAvailable(Exception):
    """What this KentOS cannot do (yet, or in this runtime): the run ends, what it wrote is taken back, and KentOS says the ability is missing instead of reporting the extension's code as failed."""


@dataclass
class CommandResult:
    """What a command says when it ends: a line, and rows to show (a report)."""
    title: str | None = None
    message: str | None = None
    rows: list[dict[str, Any]] | None = None
    cancelled: bool = False


class PanelSession:
    """One of the extension's panels, changed as the run goes: ``ctx.panel`` (the panel whose button or changed input started the run; None when it did not start from a panel) or what :meth:`CommandContext.open_panel` gives."""
    #: The panel's id in the manifest (``entrypoints.panels[].id``).
    id: str
    def set(self, id: str, *, value: Any = ..., options: Iterable[Any] | None = None, enabled: bool | None = None, visible: bool | None = None, text: str | None = None, image: Any = ...) -> bool:
        """Changes component ``id``: ``value`` (a field's text or number, a date as ``datetime.date`` or ISO text, a choice's value, a multiple choice's values, a check's True or False, a layer picker's layer id, a feature picker's persistent ids; None empties it), ``options`` (a choice's or a multiple choice's: values, or dicts with ``value`` and ``label``; a chosen value no longer among them is let go), ``…"""
    def rows(self, id: str, rows: Iterable[dict[str, Any]], columns: Sequence[str] | None = None) -> bool:
        """Puts ``rows`` (each a dict of column → value) in table, tree, cards, figures or chart ``id`` now, as a command's report would at its end; ``columns`` in their order, else in the order the rows first give them."""
    def say(self, text: str, kind: str = "info") -> bool:
        """A line under the panel's form (``info``, ``success``, ``warning``, ``error``); kept when the run ends without words of its own."""
    def focus(self, id: str) -> bool:
        """Puts the cursor in typed field ``id``; only in the panel shown (False otherwise: the code does not bring its tab in front of what the user looks at)."""
    def close(self) -> bool:
        """Closes the panel (its values with it)."""


def layer_input(layer: str, kinds: Sequence[str] | None = None) -> dict[str, Any]:
    """The objects of ``layer`` (its id) for ``ctx.run_tool``; ``kinds`` narrows them (``["polygon"]``)."""


def new_layer(name: str) -> dict[str, Any]:
    """A new layer ``name`` for ``ctx.run_tool``'s output."""


class CommandContext:
    """What a command is given: the drawing, its settings and parameters, and its run's controls."""
    #: The drawing open in KentOS (kentos.cad.Document); none when no drawing is open.
    doc: Any
    settings: dict[str, Any]
    params: dict[str, Any]
    #: The panel whose button (or changed input) started the run, to change as it goes (:class:`PanelSession`); None when it did not start from a panel.
    panel: PanelSession | None
    def is_stopped(self) -> bool:
        """The user pressed Durdur."""
    def check(self) -> None:
        """Raises :class:`Stopped` once the user pressed Durdur: call it in long loops."""
    def progress(self, done: float, total: float | None = None, text: str | None = None) -> None:
        """How far the run is; also a place where Durdur is answered."""
    def log(self, text: str, level: str = "info") -> None:
        """A line in the extension's log as the run goes (``info``, ``warning``, ``error``)."""
    def report(self, rows: list[dict[str, Any]], title: str | None = None, message: str | None = None) -> CommandResult:
        """Rows to show the user, each a dict of column → value; none is an answer too (a panel's table empties)."""
    def message(self, text: str, title: str | None = None) -> CommandResult:
        """A line to show the user."""
    def dialog(self, title: str, body: list[dict[str, Any]], ok: str = "Tamam", cancel: str = "İptal", steps: bool = False) -> dict[str, Any] | None:
        """Asks the user in a KentOS window over the drawing and waits for the answer: ``body`` is a panel's fields (``kentos.ui.panel`` components: ``text``, ``textField``, ``numberField``, ``dateField``, ``timeField``, ``dateTimeField``, ``select``, ``multiSelect``, ``checkbox``, ``layerPicker``, ``fieldPicker``), no buttons or commands."""
    def confirm(self, text: str, title: str = "Onay", ok: str = "Evet", cancel: str = "Hayır") -> bool:
        """Asks the user yes or no in a KentOS window (:meth:`dialog` with ``text``): True for ``ok``."""
    def open_panel(self, id: str) -> PanelSession:
        """Opens the extension's panel ``id`` (``entrypoints.panels[].id``; brought in front when it is open) and gives it to change as the run goes (:class:`PanelSession`): a command run from the ribbon shows what it did in its panel."""
    def cancelled(self) -> CommandResult:
        """What a run that saw Durdur returns; its writes are taken back."""
    def run_tool(self, tool: str, /, **parameters: Any) -> Any:
        """Runs a KentOS tool with this command's rights, in its undo step."""
    def apply_declarative_schema(self, path: str = "schema.json") -> CommandResult:
        """Sets up the layers, value lists and relations the package's ``schema.json`` declares, in the run's one undo step, and says what it did, a row each."""


class Feature:
    """An object a tool was given, as it was when the run began: its persistent id (``uid``), its kind, its layer's name, its attributes (texts, as KentOS keeps them) and its shape as GeoJSON (``geometry``; arcs tessellated as the drawing draws them)."""
    uid: str
    kind: str
    layer: str
    attrs: dict[str, str]
    #: Its shape, a GeoJSON geometry (Point, LineString, Polygon, their multi-part kinds); none for a kind GeoJSON has no shape for (a text, a dimension).
    geometry: dict[str, Any] | None
    @property
    def __geo_interface__(self) -> dict[str, Any]:
        """The object as a GeoJSON feature: its id, its shape and its attributes as properties."""
    @property
    def id(self) -> str:
        """Its persistent id (``uid``): what a finding names (``AuditFinding.feature_id``) and a fix writes to."""
    @property
    def entity(self) -> Any:
        """Its whole object as :mod:`kentos.cad` types it (what ``ctx.doc.entities`` gives a command)."""
    @property
    def pts(self) -> list[tuple[float, float]]:
        """Its vertices, ``(x, y)`` in metres, arcs tessellated: a point's one, a line's, an area's outline (without repeating the first); its first part's for a multi-part one; none for a kind without a shape."""
    def number(self, name: str, default: float | None = None) -> float | None:
        """An attribute as a number (``12,5`` read as 12.5); ``default`` when it is empty or not a number."""


class FeatureInput(Sequence[Feature]):
    """A features parameter's objects, in the order KentOS resolved them, and how it described them (``"12 kapalı alan; seçili nesneler"``)."""
    description: str
    @property
    def __geo_interface__(self) -> dict[str, Any]:
        ...


def point(x: float, y: float) -> dict[str, Any]:
    """A point, GeoJSON, in metres."""


def polyline(pts: Iterable[Any]) -> dict[str, Any]:
    """A line, GeoJSON: its vertices, ``(x, y)`` pairs (an array of them too)."""


def polygon(pts: Iterable[Any], holes: Iterable[Iterable[Any]] = ()) -> dict[str, Any]:
    """An area, GeoJSON: its outline's vertices and its holes', each ring closed as GeoJSON writes it."""


@dataclass
class NewFeature:
    """An object a tool writes to an output layer: a shape of its own (``geometry``: a GeoJSON geometry, from :func:`point`, :func:`polyline`, :func:`polygon`, or anything with ``__geo_interface__``, a shapely shape) or the shape of one of its inputs (``source``: the :class:`Feature`, or its ``uid``), with its attributes; an input's attributes come with it unless ``keep_attrs`` is false."""
    attrs: dict[str, Any] = field(default_factory=dict)
    geometry: Any = None
    source: Feature | str | None = None
    keep_attrs: bool = True
    label: str | None = None


class ProcessingContext(CommandContext):
    """What an İşlemler tool is given besides its parameters: its run's controls (``check``, ``progress``, ``log``) and its settings."""
    def read_features(self, features: FeatureInput) -> Iterator[Feature]:
        """A features parameter's objects one by one, Durdur answered between them."""
    def summary(self, text: str) -> None:
        """The run's one line in İşlemler' history, in place of KentOS's own ("12 nesne eklendi")."""
    def select(self, features: Iterable[Feature | str]) -> None:
        """Leaves these inputs selected in the drawing when the run ends (a session result, not a change)."""
    def update(self, feature: Feature | str, /, **attrs: Any) -> None:
        """Writes ``attrs`` into an input object of this tool, one that ``writes: true``."""
    def run_tool(self, tool: str, /, **parameters: Any) -> ToolAnswer:
        """Runs a KentOS built-in tool on objects this tool holds, never on the drawing."""


class ToolAnswer:
    """What a tool's ``ctx.run_tool`` gave back: ``summary``, ``warnings``, ``layers``."""
    summary: str
    warnings: list[Any]
    layers: dict[str, list[Feature]]
    def features(self) -> list[Feature]:
        """Every object it made, its layers one after another."""


class Shape:
    """A shape the user drew with an extension's map tool, as GeoJSON (``__geo_interface__``, RFC 7946): a ``Point``, a ``LineString`` or a ``Polygon`` (a rectangle's four corners; a circle's ring of chords, with ``circle`` its centre and radius), in the project's plane: x east, y north, metres."""
    type: str
    coordinates: Any
    #: A circle's centre ``(x, y)`` and radius, when the tool drew a circle; none otherwise.
    circle: tuple[tuple[float, float], float] | None
    @property
    def __geo_interface__(self) -> dict[str, Any]:
        ...
    @property
    def points(self) -> list[tuple[float, float]]:
        """Its corners in order: a point's one, a line's, an area's outer ring without its closing repeat."""


class MapToolContext(CommandContext):
    """A map tool's context: a command's, and the writes of what the user drew."""
    def add_feature(self, layer: str, shape: Any, attrs: dict[str, Any] | None = None) -> CommandResult:
        """Writes ``shape`` (a :class:`Shape`, any GeoJSON geometry or anything with ``__geo_interface__``) as a new object of ``layer`` (its id, or its name when only one layer has it) with ``attrs``, and says what it wrote: an area for a polygon, a line for a line string, a point for a point."""


class GisAuditFacts:
    """What an audit rule reads: the drawing's layers holding objects, each object a :class:`Feature` (persistent id, kind, layer, attributes, shape as GeoJSON), taken when the audit began, and what KentOS measured in it (:attr:`project`)."""
    #: KentOS gave only the first objects of a very large drawing (its limit): a rule may say so.
    cut: bool
    @property
    def project(self) -> ProjectAuditFacts:
        """What KentOS measured in the drawing, once and in Rust (:class:`ProjectAuditFacts`): read it rather than working the geometry out again."""
    def layers(self) -> list[tuple[str, str]]:
        """The layers holding objects, ``(id, name)``, in the layer tree's order."""
    def features(self, layer: str) -> list[Feature]:
        """A layer's objects, by its id or its name (KentOS's GIS template: ``parsel``, ``yapi``); none when no layer has it."""
    def all_features(self) -> list[Feature]:
        """Every object given, layer by layer."""
    def group_related(self, layer: str, foreign_key: str) -> dict[str, list[Feature]]:
        """A layer's objects grouped by their ``foreign_key`` attribute's value (a related table's key); objects without it are left out."""


class ProjectAuditFacts:
    """What KentOS measured in the drawing for an audit (``kentos.auditFacts`` 1, the SDK's ``factsSchema``): each layer's objects by geometry, its invalid geometries, the overlaps of its areas and the gaps their union closes in, worked out once in Rust at the project's topology tolerance, the same on the desktop and in a cloud job."""
    #: The project's topology tolerance (m): areas nearer than it touch, a piece no wider is no overlap or gap.
    tolerance: float
    #: KentOS measured only the first objects of a very large drawing (by persistent id): a rule may say so.
    cut: bool
    #: The drawing's storeys, rooms, program groups and escape routes; empty without rooms.
    architecture: ArchitectureFacts
    def layers(self) -> list[LayerFacts]:
        """The layers holding objects, in the layer tree's order."""
    def layer(self, layer: str) -> LayerFacts:
        """A layer's facts, by its id or its name (``parsel``, ``Parsel``); an empty one when no layer holding objects has it."""


class LayerFacts:
    """One layer's measured facts (:class:`ProjectAuditFacts`); a list holds at most 10 000 items, the first in its order (invalid by object, overlaps by their pair, gaps north to south), and :attr:`totals` counts all of them."""
    id: str
    name: str
    #: Its objects by geometry: points, lines (a closed polyline too), areas and the rest (texts, blocks …).
    points: int
    lines: int
    areas: int
    other: int
    #: Its areas' area summed, m²: holes out, arcs exact (not the tessellated GeoJSON's).
    area: float
    #: Its lines' length summed, m, arcs exact.
    length: float
    invalid: list[InvalidGeometry]
    overlaps: list[Overlap]
    gaps: list[Gap]
    #: How many it has in all: ``{"invalid": …, "overlaps": …, "gaps": …}``.
    totals: dict[str, int]


@dataclass(frozen=True)
class InvalidGeometry:
    """An object whose shape is not valid: its id, the problem (``repeated``, ``zeroArea``, ``ringCrossing``, ``pathCrossing``, ``holeOutside``, ``holesOverlap``) and where it first shows."""
    uid: str
    problem: str
    at: tuple[float, float]


@dataclass(frozen=True)
class Overlap:
    """Two areas of one layer covering each other: their ids (``a`` the smaller), the area both cover (m²) and the centroid of its largest piece."""
    a: str
    b: str
    area: float
    at: tuple[float, float]


@dataclass(frozen=True)
class Gap:
    """What a layer's areas close in and none covers (a gap between parcels, an area's hole): its area (m²), its centroid and the areas around it, the one sharing the longest boundary first."""
    area: float
    at: tuple[float, float]
    around: tuple[str, ...]


class ArchitectureFacts:
    """The drawing's architecture as KentOS Mimari measured it: its storeys, every room with its net area and its outline to the walls' axes, the program's groups with PAİY 4/ğ's areas and the escape routes (BYKHY); empty when the drawing has no room."""
    #: The project's storeys, lowest first.
    levels: list[Storey]
    #: Every room, a void and an outdoor one too, by persistent id.
    rooms: list[RoomFacts]
    #: The program's groups that have rooms, in the program's order.
    units: list[UnitFacts]
    #: Each dwelling unit's door and its walk to the nearest storey exit (BYKHY 48/4).
    doors: list[DoorEscape]
    #: The project says the building has sprinklers: BYKHY's sprinklered distances hold.
    sprinklers: bool
    def exits(self, level: str | None) -> int:
        """How many storey exits a storey has (BYKHY 4/u: an escape stair's room, an open stair, a door out on the ground storey), by its id; 0 for a storey without rooms."""
    def room(self, uid: str) -> RoomFacts | None:
        """A room by its persistent id; none when the drawing has no such room."""
    def unit(self, unit: str) -> UnitFacts | None:
        """A program group by its id or its number; none when no group with rooms has it."""


@dataclass(frozen=True)
class Storey:
    """A storey of the project: its id, its name, its floor's elevation (m) and its height (m; None when not given)."""
    id: str
    name: str
    elevation: float
    height: float | None


@dataclass(frozen=True)
class EscapeWalk:
    """A walk along the points it turns at, its length in metres; a walk to an exit also says the part it shares with the walk to the nearest other exit (``one_way``, tek yön: a dead end's) and how many exits its start reaches."""
    length: float
    path: tuple[tuple[float, float], ...]
    one_way: float | None = None
    exits: int | None = None


@dataclass(frozen=True)
class RoomEscape:
    """A room's walks from its farthest point, 40 cm in front of its walls (BYKHY 32/6): to the nearest storey exit (32/4; None when it reaches none) and, a dwelling unit's room, to its unit's door (48/2)."""
    exit: EscapeWalk | None
    storey_exits: bool
    entrance: EscapeWalk | None
    has_entrance: bool
    passages: int


@dataclass(frozen=True)
class RoomFacts:
    """A room: its persistent id, number, name, usage, storey and group; void, outdoor, broken (boundary not found) or an escape stair's room; net area and outline to the walls' axes (m²); walks (None for a void, outdoor, broken or stair room); ``kind`` by name or use (``living``, ``bedroom``, ``kitchen``, ``bath``, ``toilet``, ``corridor``; None for another); ``narrow``, its net width (m, dar kenar)."""
    uid: str
    number: str | None
    name: str | None
    usage: str | None
    level: str | None
    unit: str | None
    void: bool
    outdoor: bool
    broken: bool
    stair: bool
    net: float
    axis: float
    escape: RoomEscape | None
    kind: str | None = None
    narrow: float | None = None


@dataclass(frozen=True)
class UnitLevel:
    """A group's gross and net area on one storey (m²)."""
    level: str | None
    gross: float
    net: float


@dataclass(frozen=True)
class UnitFacts:
    """A program group with PAİY 4/ğ's areas (m²): ``kind`` (``unit``, ``common``, ``outdoor``, ``annex``), ``gross`` (BB brüt), ``net``, ``total`` (with its annexes'), ``share`` of the common areas and ``general`` (genel brüt; None without a share)."""
    id: str
    number: str
    name: str | None
    kind: str
    nature: str | None
    annex_of: str | None
    dwelling: bool
    gross: float
    net: float
    annex_gross: float
    annex_net: float
    total: float
    share: float | None
    general: float | None
    levels: tuple[UnitLevel, ...]


@dataclass(frozen=True)
class DoorEscape:
    """A dwelling unit's door: its persistent id, unit, storey and middle, and its walk from its outer side to the nearest storey exit (BYKHY 48/4; None when it opens into an escape stair's room, ``into_stair``, or reaches none)."""
    uid: str
    unit: str
    level: str | None
    at: tuple[float, float]
    exit: EscapeWalk | None
    into_stair: bool = False


@dataclass
class SemanticFix:
    """A finding's fix: a KentOS command and its arguments, run by the user's choice as one undo step."""
    title: str
    command: str
    args: dict[str, Any] = field(default_factory=dict)


@dataclass
class AuditFinding:
    """One finding of an audit rule: ``error``, ``warning`` or ``info``; the object it is about; how to fix it."""
    message: str
    severity: str = "warning"
    feature_id: str | None = None
    fix: SemanticFix | None = None
