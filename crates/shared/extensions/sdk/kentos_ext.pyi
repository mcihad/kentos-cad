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
    rows: list[dict[str, Any]] = field(default_factory=list)
    cancelled: bool = False


class CommandContext:
    """What a command is given: the drawing, its settings and parameters, and its run's controls."""
    #: The drawing open in KentOS (kentos.cad.Document); none when no drawing is open.
    doc: Any
    settings: dict[str, Any]
    params: dict[str, Any]
    def is_stopped(self) -> bool:
        """The user pressed Durdur."""
    def check(self) -> None:
        """Raises :class:`Stopped` once the user pressed Durdur: call it in long loops."""
    def progress(self, done: float, total: float | None = None, text: str | None = None) -> None:
        """How far the run is; also a place where Durdur is answered."""
    def log(self, text: str, level: str = "info") -> None:
        """A line in the extension's log as the run goes (``info``, ``warning``, ``error``)."""
    def report(self, rows: list[dict[str, Any]], title: str | None = None, message: str | None = None) -> CommandResult:
        """Rows to show the user, each a dict of column → value."""
    def message(self, text: str, title: str | None = None) -> CommandResult:
        """A line to show the user."""
    def cancelled(self) -> CommandResult:
        """What a run that saw Durdur returns; its writes are taken back."""
    def apply_declarative_schema(self, path: str) -> CommandResult:
        """Sets up the layers, value lists and relations the package's ``schema.json`` declares, in one undo step."""


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


class Shape:
    """A shape the user drew with an extension's map tool (plan/eklenti.md ADIM 19), as GeoJSON (``__geo_interface__``, RFC 7946): a ``Point``, a ``LineString`` or a ``Polygon`` (a rectangle's four corners; a circle's ring of chords, with ``circle`` its centre and radius), in the project's plane: x east, y north, metres."""
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
    """A map tool's context (plan/eklenti.md ADIM 19): a command's, and the writes of what the user drew."""
    def add_feature(self, layer: str, shape: Any, attrs: dict[str, Any] | None = None) -> CommandResult:
        """Writes ``shape`` (a :class:`Shape`, any GeoJSON geometry or anything with ``__geo_interface__``) as a new object of ``layer`` (its id, or its name when only one layer has it) with ``attrs``, and says what it wrote: an area for a polygon, a line for a line string, a point for a point."""


class GisAuditFacts:
    """What an audit rule reads (docs/adr/8057 §17): the drawing's layers holding objects, each object a :class:`Feature` (persistent id, kind, layer, attributes, shape as GeoJSON), taken when the audit began."""
    #: KentOS gave only the first objects of a very large drawing (its limit): a rule may say so.
    cut: bool
    def layers(self) -> list[tuple[str, str]]:
        """The layers holding objects, ``(id, name)``, in the layer tree's order."""
    def features(self, layer: str) -> list[Feature]:
        """A layer's objects, by its id or its name (KentOS's GIS template: ``parsel``, ``yapi``); none when no layer has it."""
    def all_features(self) -> list[Feature]:
        """Every object given, layer by layer."""
    def group_related(self, layer: str, foreign_key: str) -> dict[str, list[Feature]]:
        """A layer's objects grouped by their ``foreign_key`` attribute's value (a related table's key); objects without it are left out."""


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
