"""A drawing open in this process, without a window: :class:`Document`
(docs/adr/0130, 0131).

It is the desktop's own document, run by ``kentos._native``. A command writes
it as the desktop's tool would, one undo step each; ``group`` makes a whole
script one step, and takes all of it back when the script fails. Objects are
named by their persistent ids (``uid``, UUID text), which files, the cloud,
the desktop and the web share; slots (``Entity.id``) mean nothing once the
drawing is closed.
"""

from __future__ import annotations

import json
import math
import os
from collections.abc import Callable, Iterable, Iterator, Mapping, Sequence
from contextlib import contextmanager
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Literal, TypeVar

from .. import _native
from ._runtime import Outcome, _enum_out, _Model, call_outcome, decode, dumps
from .errors import NotRunHere, UnknownObject, host_error
from .protocol import Expected, Job, ReplayReport, Stamps
from .types import (
    BlockDefinition,
    Bounds,
    CadImportScale,
    DrawingFont,
    DrawingFontName,
    DrawingUnit,
    DrawingUnitName,
    Entity,
    LayerNode,
    LayerNodeType,
    NamedView,
    LineageRecord,
    ProjectForm,
    ProjectSettings,
    SheetInfo,
    Workspace,
    WorkspaceName,
)

T = TypeVar("T")


class SessionBase:
    """What a Document works over besides a native session: the drawing
    open in the desktop, asked through its console (``kentos.host``).
    Internal; a Document is opened with ``new``, ``open``, ``from_bytes`` or
    :func:`current`. The methods are the native session's, JSON text in and out."""

    __slots__ = ()

    def run(self, command: str, op: str, input: str, version: int | None = None) -> str:
        raise NotImplementedError

    def save(self, path: str | None = None) -> str:
        raise NotImplementedError

    def to_bytes(self) -> bytes:
        raise NotImplementedError

    def export_dxf(self, path: str) -> str:
        raise NotImplementedError

    def export_pdf(self, path: str, sheets: list[str] | None = None) -> str:
        raise NotImplementedError

    def summary(self) -> str:
        raise NotImplementedError

    def layers(self) -> str:
        raise NotImplementedError

    def blocks(self) -> str:
        raise NotImplementedError

    def arrays(self) -> str:
        raise NotImplementedError

    def constraints(self) -> str:
        raise NotImplementedError

    def entities(
        self,
        layer: str | None = None,
        kinds: list[str] | None = None,
        bbox: tuple[float, float, float, float] | None = None,
        after: str | None = None,
        limit: int | None = None,
    ) -> str:
        raise NotImplementedError

    def entity(self, uid: str) -> str:
        raise NotImplementedError

    def measure(self, uid: str, method: str | None = None) -> str:
        raise NotImplementedError

    def measure_many(self, uids: str, method: str | None = None) -> str:
        raise NotImplementedError

    def relax(self, on: bool) -> None:
        raise NotImplementedError

    def undo(self) -> str | None:
        raise NotImplementedError

    def redo(self) -> str | None:
        raise NotImplementedError

    def begin_group(self, label: str) -> None:
        raise NotImplementedError

    def end_group(self) -> bool:
        raise NotImplementedError

    def cancel_group(self) -> bool:
        raise NotImplementedError

    def call(self, envelope: str) -> str:
        raise NotImplementedError

    def start(self, envelope: str) -> str:
        raise NotImplementedError

    def job(self, id: str) -> str:
        raise NotImplementedError

    def wait(self, id: str) -> str:
        raise NotImplementedError

    def cancel(self, id: str) -> str:
        raise NotImplementedError

    def journal(self) -> str:
        raise NotImplementedError

    def replay(self, journal: str) -> str:
        raise NotImplementedError

    def selection(self) -> str:
        raise NotImplementedError

    def select(self, uids: str) -> int:
        raise NotImplementedError


_CURRENT: Document | None = None


def current() -> Document:
    """The drawing open in the KentOS desktop, when this runs in its Python
    console (there ``doc`` is it too). Every run in the console is one undo
    step; the desktop takes it back when the run raises or is stopped."""
    if _CURRENT is None:
        raise NotRunHere(
            "no_desktop",
            "Bu betik masaüstünün Python konsolunda çalışmıyor: bir çizimi Document.new ya da Document.open ile açın.",
        )
    return _CURRENT


def _host(call: Callable[..., T], *args: Any) -> T:
    try:
        return call(*args)
    except _native.HostError as e:
        code, message = e.args
        raise host_error(code, message) from None


@dataclass(frozen=True, slots=True)
class Record:
    """An object of the drawing with its persistent id and its state stamp."""

    uid: str
    entity: Entity
    stamp: str | None = None
    """Its state stamp (docs/adr/0253 §2): ``Expected(objects={uid: stamp})`` writes only while it holds."""


@dataclass(frozen=True, slots=True)
class Page:
    """Objects in the drawing's order; ``next`` reads on (``after=``)."""

    items: list[Record]
    next: str | None


@dataclass(frozen=True, slots=True)
class Units:
    """The units of the values a drawing gives and takes (docs/adr/0245).

    Coordinates, lengths and areas are in the project's plane, metres (or
    degrees in a geographic system), whatever the drawing unit: a 4000 mm
    wall in a millimetre project is written 4.0. The drawing unit is only
    how the desktop's windows read and write lengths (docs/adr/0165 §2)."""

    length: str
    """Coordinates and lengths: ``m``, or ``degree`` in a geographic system."""
    area: str
    """Areas: ``m2``, or ``degree2``."""
    drawing: DrawingUnit
    """The project's drawing unit: a local project's own, else metres."""

    def from_metres(self, value: float) -> float:
        """A length in metres in the drawing unit (4.0 m is 4000.0 in mm); by the unit's ratio of whole numbers, as
        the desktop turns them (an inch is 254/10000 m, docs/adr/0262 §1)."""
        mul, div = _METRES[self.drawing]
        return value * div if mul == 1.0 else value * div / mul

    def to_metres(self, value: float) -> float:
        """A length in the drawing unit in metres (4000.0 mm is 4.0 m; 150.5 in is 3.8227 m)."""
        mul, div = _METRES[self.drawing]
        return value / div if mul == 1.0 else value * mul / div


# A unit's metres as a ratio of whole numbers (kentos_contracts::DrawingUnit::metres).
_METRES = {
    DrawingUnit.MM: (1.0, 1000.0),
    DrawingUnit.CM: (1.0, 100.0),
    DrawingUnit.M: (1.0, 1.0),
    DrawingUnit.IN: (254.0, 10000.0),
    DrawingUnit.FT: (3048.0, 10000.0),
}


def _units(raw: Mapping[str, Any] | None) -> Units:
    # A desktop older than docs/adr/0245 says none: its values are metres.
    raw = raw or {}
    return Units(
        length=str(raw.get("length", "m")),
        area=str(raw.get("area", "m2")),
        drawing=DrawingUnit(raw.get("drawing", "m")),
    )


@dataclass(frozen=True, slots=True)
class Basis:
    """What a measure was taken in and how (docs/adr/0256 §4)."""

    srid: int
    """The project's system's EPSG code; 0 for none (a local project) or a definition of its own."""
    system: str
    """Its name: ``TUREF / TM33``, a definition's, ``Yerel (koordinat sistemi yok)``."""
    kind: str
    """``local``, ``projected`` or ``geographic``."""
    frame: str
    """The frame the coordinates and the box are in: ``world``, the drawing's own."""
    method: str
    """``plane`` (the project's plane), ``ellipsoid`` (geodesics on its ellipsoid) or ``ground``."""
    height: float | None = None
    """The ground's mean ellipsoidal height, m (``ground``)."""


def _basis(raw: Mapping[str, Any] | None) -> Basis:
    # A desktop older than docs/adr/0256 says none: its measures are the plane's.
    raw = raw or {}
    return Basis(
        srid=int(raw.get("srid", 0)),
        system=str(raw.get("system", "")),
        kind=str(raw.get("kind", "")),
        frame=str(raw.get("frame", "world")),
        method=str(raw.get("method", "plane")),
        height=raw.get("height"),
    )


@dataclass(frozen=True, slots=True)
class Measure:
    """An object's measures from its source geometry (not a drawn
    approximation), in the units ``units`` names, taken as ``basis`` says:
    by default metres and m² in the project's plane, never the drawing unit."""

    uid: str
    kind: str
    area: float | None
    """Area of a closed shape (``units.area``); None for an open one."""
    length: float | None
    """Length of a line or a path, perimeter of a closed shape (``units.length``); None for a point or a text."""
    bounds: Bounds
    units: Units
    basis: Basis = field(default_factory=lambda: _basis(None))


@dataclass(frozen=True, slots=True)
class MeasureItem:
    """One object of a batch."""

    uid: str
    kind: str
    area: float | None
    length: float | None
    bounds: Bounds


@dataclass(frozen=True, slots=True)
class Measures:
    """Many objects' measures taken one way (docs/adr/0256 §4): each object,
    the sums (``count``, ``area`` of those with one, ``length``), and the
    units and basis once."""

    items: list[MeasureItem]
    count: int
    area: float
    length: float
    units: Units
    basis: Basis


@dataclass(frozen=True, slots=True)
class Conventions:
    """The axes and angles the drawing's values and windows speak
    (docs/adr/0165 §4, docs/adr/0256 §5). On the wire ``x`` is east and
    ``y`` north, always; the windows call them X, Y in CAD and Y, X in CBS;
    polar angles start east and turn counter-clockwise in CAD, start north
    and turn clockwise (azimuth) in CBS."""

    workspace: str
    """``cad`` or ``gis``."""
    axes: tuple[str, str]
    """What the wire's x and y are: ``("east", "north")``."""
    labels: tuple[str, str]
    """What the windows call them: ``("X", "Y")`` in CAD, ``("Y", "X")`` in CBS."""
    polar_zero: str
    """Where polar angles start: ``east`` or ``north``."""
    polar_turn: str
    """Which way they turn: ``ccw`` or ``cw``."""
    angle_unit: str
    """The angle unit the windows show and take: ``deg`` or ``grad``. On the wire each angle field is in its own unit (the catalog's ``x-unit``)."""
    ucs: UcsFrame | None = None
    """The drawing's user coordinate system (docs/adr/0261 §5): how its windows take and show points; ``None``, the
    world's. The wire stays the world: turn points with :meth:`Document.to_world` before giving them to a command."""


@dataclass(frozen=True, slots=True)
class UcsFrame:
    """A user coordinate system (docs/adr/0261): its origin in the world and the angle of its x axis from east,
    counter-clockwise, in radians; ``name`` when a named UCS is this frame. The world's is origin (0, 0), angle 0.

    Points are turned in Python's own floating point: within a float's rounding of the desktop's."""

    origin: tuple[float, float]
    angle: float
    name: str | None = None

    @property
    def is_world(self) -> bool:
        return self.origin == (0.0, 0.0) and self.angle == 0.0

    def to_world(self, points: Iterable[Sequence[float]]) -> list[tuple[float, float]]:
        """Points of this frame in the world: origin + R(angle)·p."""
        c, s = math.cos(self.angle), math.sin(self.angle)
        ox, oy = self.origin
        return [(ox + p[0] * c - p[1] * s, oy + p[0] * s + p[1] * c) for p in points]

    def to_frame(self, points: Iterable[Sequence[float]]) -> list[tuple[float, float]]:
        """World points in this frame: R(−angle)·(w − origin)."""
        c, s = math.cos(self.angle), math.sin(self.angle)
        ox, oy = self.origin
        return [((p[0] - ox) * c + (p[1] - oy) * s, -(p[0] - ox) * s + (p[1] - oy) * c) for p in points]


WORLD_UCS = UcsFrame((0.0, 0.0), 0.0)


def _frame(raw: Mapping[str, Any] | None, name: str | None = None) -> UcsFrame:
    if not raw:
        return WORLD_UCS
    o = raw["origin"]
    return UcsFrame((float(o["x"]), float(o["y"])), float(raw["angle"]), raw.get("name", name))


def _conventions(raw: Mapping[str, Any] | None) -> Conventions:
    raw = raw or {}
    axes = raw.get("axes") or {"x": "east", "y": "north"}
    labels = raw.get("labels") or {"x": "Y", "y": "X"}
    polar = raw.get("polar") or {"zero": "north", "turn": "cw"}
    return Conventions(
        workspace=str(raw.get("workspace", "gis")),
        axes=(str(axes["x"]), str(axes["y"])),
        labels=(str(labels["x"]), str(labels["y"])),
        polar_zero=str(polar["zero"]),
        polar_turn=str(polar["turn"]),
        angle_unit=str(raw.get("angleUnit", "grad")),
        ucs=_frame(raw["ucs"]) if raw.get("ucs") else None,
    )


@dataclass(frozen=True, slots=True)
class Saved:
    """Where a save went, how many bytes, at which revision."""

    path: Path
    bytes: int
    revision: str


@dataclass(frozen=True, slots=True)
class Exported:
    """Where an export went, how many bytes, how many objects and layers, and
    what changed on the way (the writer's report: ``counts``, ``notes``,
    ``skipped``); the file's SHA-256."""

    path: Path
    bytes: int
    objects: int
    layers: int
    report: dict[str, Any]
    sha256: str = ""


@dataclass(frozen=True, slots=True)
class Printed:
    """Where a sheet PDF went, how many bytes, the sheets it holds (by id, a page each), the
    sheet core's findings, and native rasterization choices and limits (``notes``)."""

    path: Path
    bytes: int
    sheets: list[str]
    findings: list[dict[str, Any]]
    notes: list[str]
    sha256: str = ""


@dataclass(frozen=True, slots=True)
class DocumentInfo:
    """The drawing as a whole."""

    name: str
    revision: str
    dirty: bool
    objects: int
    settings: ProjectSettings
    active_layer: str
    path: Path | None
    legacy: bool
    """Read from an old v1 JSON file: a save needs a new path."""
    can_undo: bool
    can_redo: bool
    units: Units
    """What the values are in: metres in the project's plane; ``units.drawing`` is the drawing unit."""
    stamps: Stamps | None = None
    """The project's parts' state stamps (``layers``, ``settings``, ``styles``, ``sheets``; docs/adr/0253 §2)."""
    conventions: Conventions | None = None
    """The axes and angles the values and the windows speak (docs/adr/0256 §5)."""
    home_view: Bounds | None = None
    """Where the view opens, in the project's plane; None: every object."""
    opened_cad: Mapping[str, Any] | None = None
    """External CAD source path, unit decision and read report. Its source is never the save target."""


BoundsLike = Bounds | Sequence[float]
"""A box: :class:`Bounds`, or ``(min_x, min_y, max_x, max_y)``."""


def _bbox(value: BoundsLike | None) -> tuple[float, float, float, float] | None:
    if value is None:
        return None
    if isinstance(value, Bounds):
        return (value.min_x, value.min_y, value.max_x, value.max_y)
    a, b, c, d = value
    return (float(a), float(b), float(c), float(d))


class Document:
    """A drawing open in this process.

    Open one with :meth:`new`, :meth:`open` or :meth:`from_bytes`; change it
    with the commands (``kentos.cad.polygon.create(doc, …)``); read it with
    :meth:`layers`, :meth:`entities`, :meth:`measure`; keep it with
    :meth:`save`. Not for two threads at once: a second call while one runs
    raises.
    """

    __slots__ = ("__weakref__", "_session")

    def __init__(self, session: Any) -> None:
        if not isinstance(session, (_native.Session, SessionBase)):
            raise TypeError("Bir çizim Document.new, Document.open ya da Document.from_bytes ile açılır.")
        self._session = session

    # ------------------------------------------------------------ opening

    @classmethod
    def new(
        cls,
        name: str | None = None,
        *,
        workspace: Workspace | WorkspaceName = Workspace.GIS,
        coords: Literal["local", "real"] | None = None,
        srid: int | None = None,
        province: int | None = None,
        drawing_unit: DrawingUnit | DrawingUnitName | None = None,
        plot_scale: float | None = None,
        drawing_font: DrawingFont | DrawingFontName = DrawingFont.BARLOW,
    ) -> Document:
        """A new project, as the Yeni proje wizard makes it (docs/adr/0165,
        0245): its type's layer tree, start view and units.

        - ``workspace``: CBS (``gis``) or CAD (``cad``).
        - ``coords``: a CAD project's coordinates, ``local`` (no system,
          SRID 0, from 0,0) or ``real``; by default local for CAD without a
          ``srid``, real otherwise. A CBS project always has a system.
        - ``srid``: the coordinate system (5254…5259 TUREF/TM, 32635…32638
          UTM …), 0 for local. A project with a system needs it or a
          ``province`` (plate code), whose TUREF zone is then taken; it is
          never guessed.
        - ``drawing_unit``: a local project's unit (``mm``, ``cm``, ``m``, ``in``, ``ft``); defaults to ``mm``;
          one with a system is in metres.
        - ``plot_scale``: 1000 for 1:1000; metres default to 1000, centimetres
          to 50, millimetres/inches/feet to 1.

        Units: geometry is always metres in the project's plane, whatever
        the drawing unit. A 4000 × 3000 mm room in a ``mm`` project is
        written ``(0, 0), (4, 0), (4, 3), (0, 3)`` and measures 12 m²;
        :attr:`DocumentInfo.units` converts. A refusal raises
        :class:`InvalidInput` (or :class:`UnknownSystem`) in the wizard's words."""
        choices: dict[str, Any] = {
            "workspace": _enum_out(workspace),
            "drawingFont": _enum_out(drawing_font),
        }
        optional = {
            "name": name,
            "coords": coords,
            "srid": None if srid is None else int(srid),
            "province": None if province is None else int(province),
            "drawingUnit": None if drawing_unit is None else _enum_out(drawing_unit),
            "plotScale": None if plot_scale is None else float(plot_scale),
        }
        choices.update({k: v for k, v in optional.items() if v is not None})
        return cls(_host(_native.Session.new_project, json.dumps(choices)))

    @classmethod
    def open(cls, path: str | os.PathLike[str]) -> Document:
        """Open KCAD (v2/v1), DWG, DXF or NCZ. External CAD opens as its own
        drawing; save to a new ``.kcad`` path to preserve the source file."""
        return cls(_host(_native.Session.open, os.fspath(path)))

    @classmethod
    def from_bytes(cls, data: bytes | bytearray | memoryview) -> Document:
        """A drawing from a ``.kcad`` file's bytes (a download, a database cell)."""
        return cls(_host(_native.Session.from_bytes, bytes(data)))

    # ------------------------------------------------------------ keeping

    def save(self, path: str | os.PathLike[str] | None = None) -> Saved:
        """Saves as ``.kcad`` v2 to ``path``, or where it was read from. The
        bytes are read back before they replace the file; a failed save
        leaves the old file as it was."""
        raw = json.loads(_host(self._session.save, None if path is None else os.fspath(path)))
        return Saved(Path(raw["path"]), int(raw["bytes"]), str(raw["revision"]))

    def to_bytes(self) -> bytes:
        """The ``.kcad`` v2 bytes of the drawing as it is now, verified."""
        return _host(self._session.to_bytes)

    def export_dxf(self, path: str | os.PathLike[str]) -> Exported:
        """The whole drawing as an AutoCAD 2007 DXF file, as the desktop's DXF
        dışa aktar writes it (the same writer): every object with its layers,
        what DXF cannot hold carried as KentOS data. An export is not a save."""
        raw = json.loads(_host(self._session.export_dxf, os.fspath(path)))
        return Exported(
            Path(raw["path"]), int(raw["bytes"]), int(raw["objects"]), int(raw["layers"]), raw["report"], str(raw.get("sha256", ""))
        )

    def import_dxf(
        self,
        path: str | os.PathLike[str],
        *,
        file_unit: str | None = None,
        scale_reference: CadImportScale | None = None,
        explode_blocks: bool = False,
        skip_layers: Sequence[str] = (),
        layers: Mapping[str, str] | None = None,
        group: str | None = None,
    ) -> Any:
        """A DXF file into the drawing as the desktop's DXF içe aktar takes it, as one undo step
        (``file.import.dxf``, docs/adr/0257): each of its layers onto the drawing's layer of the
        same name, else a new one in a group named after the file (``group``; ``""`` for the top of
        the tree), ``layers`` routing a file's layer to a layer id, ``skip_layers`` leaving some out.
        Its unit: ``file_unit`` (``"mm"``, ``"in"``, ``"usFt"`` …) when given, else what the file
        declares in every project; a file declaring none uses the project's unit (metres with a
        coordinate system). ``scale_reference`` instead calibrates a raw source distance to metres;
        it cannot be combined with ``file_unit``. Neither changes project display units.
        The answer is ``DxfImported``: the unit decided, the layers made, the objects taken."""
        from .file import import_

        return import_.dxf(
            self,
            path=os.fspath(path),
            file_unit=file_unit,
            scale_reference=scale_reference,
            explode_blocks=explode_blocks,
            skip_layers=list(skip_layers),
            layers=dict(layers or {}),
            group=group,
        )

    def export_pdf(self, path: str | os.PathLike[str], sheets: Sequence[str] | None = None) -> Printed:
        """The project's sheet book as a PDF: the sheets named (by id) or all, a page each, as the
        desktop's Dışa aktar › PDF writes them (each map at its scale, GeoPDF where the system is a
        transverse Mercator one). Sheets are made with ``kentos.cad.sheets.edit``. The native
        renderer runs without a window or printer and preserves searchable Unicode text.
        ``notes`` reports rasterization choices and limits; PDF metadata identifies the source snapshot."""
        raw = json.loads(_host(self._session.export_pdf, os.fspath(path), None if sheets is None else list(sheets)))
        return Printed(
            Path(raw["path"]),
            int(raw["bytes"]),
            list(raw["sheets"]),
            list(raw["findings"]),
            list(raw["notes"]),
            str(raw.get("sha256", "")),
        )

    # ------------------------------------------------------------ the whole

    def info(self) -> DocumentInfo:
        s = json.loads(self._session.summary())
        return DocumentInfo(
            name=s["name"],
            revision=s["revision"],
            dirty=s["dirty"],
            objects=s["objects"],
            settings=decode(ProjectSettings, s["settings"], "Document.info"),
            active_layer=s["activeLayer"],
            path=Path(s["path"]) if s.get("path") else None,
            legacy=s["legacy"],
            can_undo=s["canUndo"],
            can_redo=s["canRedo"],
            units=_units(s.get("units")),
            stamps=None if s.get("stamps") is None else Stamps.from_json(s["stamps"]),
            conventions=_conventions(s.get("conventions")),
            home_view=None if s.get("homeView") is None else decode(Bounds, s["homeView"], "Document.info"),
            opened_cad=s.get("openedCad"),
        )

    def sheets(self) -> list[SheetInfo]:
        """The project's sheet book's sheets, in its order: page size in millimetres, map frames
        with their scales (``kentos.cad.sheets.edit`` changes the book). Empty without a book."""
        raw = json.loads(self._session.summary()).get("sheets")
        if raw is None:
            raise host_error("book_unreadable", "Bu projenin pafta kitabı okunamadı; daha yeni bir KentOS'la açın.")
        return [decode(SheetInfo, x, "Document.sheets") for x in raw]

    @property
    def name(self) -> str:
        return self.info().name

    @property
    def revision(self) -> str:
        """The drawing's revision (decimal text); a command's ``expected_revision``."""
        return self.info().revision

    @property
    def dirty(self) -> bool:
        """Changed since it was opened or saved."""
        return self.info().dirty

    @property
    def path(self) -> Path | None:
        return self.info().path

    @property
    def settings(self) -> ProjectSettings:
        """The project's settings: its coordinate system (``srid``), units, scale."""
        return self.info().settings

    @property
    def units(self) -> Units:
        """What the drawing's values are in: metres in the project's plane;
        ``units.drawing`` is the drawing unit (docs/adr/0245)."""
        return self.info().units

    @property
    def conventions(self) -> Conventions:
        """The axes and angles the values and the windows speak: on the wire
        ``x`` is east, ``y`` north; the windows' names and polar angles by the
        project's type (docs/adr/0256 §5)."""
        return self.info().conventions or _conventions(None)

    @property
    def ucs(self) -> UcsFrame:
        """The drawing's user coordinate system (docs/adr/0261): the world's when it has none. Set it with
        ``kentos.cad.ucs.set``."""
        return self.conventions.ucs or WORLD_UCS

    @property
    def named_ucs(self) -> list[UcsFrame]:
        """The project's named UCSs, in the order they were kept."""
        raw = json.loads(self._session.summary())["settings"].get("namedUcs") or []
        return [_frame(n, n["name"]) for n in raw]

    @property
    def named_views(self) -> list[NamedView]:
        """The project's named views, in the order they were kept (docs/adr/0285): each its middle, the world width
        and height it shows and its twist. Keep, rename and remove them with ``kentos.cad.views.edit``; going to one is
        the desktop's camera, not the drawing's."""
        raw = json.loads(self._session.summary())["settings"].get("namedViews") or []
        return [NamedView.from_json(v) for v in raw]

    @property
    def forms(self) -> list[ProjectForm]:
        """The project's forms, in the order they were made (docs/adr/0401): each its layer, draft, releases and
        whether its latest release is the layer's write profile. Make, save, publish and enforce them with
        ``kentos.cad.forms.edit``; ask about a record with ``kentos.cad.forms.validate`` and write one with
        ``kentos.cad.forms.submit``."""
        raw = json.loads(self._session.summary())["settings"].get("forms") or []
        return [ProjectForm.from_json(f) for f in raw]

    @property
    def lineage(self) -> list[LineageRecord]:
        """What the split and merged objects came from (docs/adr/0598), in the order the steps made them: each record
        an object, its parents, ``split`` or ``merge`` and the step's name. Undo takes a step's records back; a bound
        layer's write puts its objects' records beside its rows (``kentos_lineage``)."""
        raw = json.loads(self._session.summary())["settings"].get("lineage") or []
        return [LineageRecord.from_json(r) for r in raw]

    def ancestors(self, uid: str) -> list[str]:
        """The objects ``uid`` came from and theirs, nearest first, each once (docs/adr/0598)."""
        out: list[str] = []
        queue = [uid]
        records = self.lineage
        while queue:
            u = queue.pop(0)
            for r in records:
                if r.child == u:
                    for p in r.parents:
                        if p != uid and p not in out:
                            out.append(p)
                            queue.append(p)
        return out

    def descendants(self, uid: str) -> list[str]:
        """The objects that came from ``uid`` and theirs, nearest first, each once (docs/adr/0598)."""
        out: list[str] = []
        queue = [uid]
        records = self.lineage
        while queue:
            u = queue.pop(0)
            for r in records:
                if u in r.parents and r.child != uid and r.child not in out:
                    out.append(r.child)
                    queue.append(r.child)
        return out

    def _ucs_of(self, ucs: UcsFrame | str | None) -> UcsFrame:
        if ucs is None:
            return self.ucs
        if isinstance(ucs, str):
            found = next((n for n in self.named_ucs if n.name == ucs), None)
            if found is None:
                raise UnknownObject("unknown_ucs", f"“{ucs}” adlı bir UCS yok.")
            return found
        return ucs

    def to_world(self, points: Iterable[Sequence[float]], ucs: UcsFrame | str | None = None) -> list[tuple[float, float]]:
        """Points given in a UCS turned into the world, where commands take them (docs/adr/0261 §5): the
        drawing's current UCS, or ``ucs`` (a :class:`UcsFrame` or a named UCS's name)."""
        return self._ucs_of(ucs).to_world(points)

    def to_ucs(self, points: Iterable[Sequence[float]], ucs: UcsFrame | str | None = None) -> list[tuple[float, float]]:
        """World points in a UCS, as the windows show them: the drawing's current UCS, or ``ucs``."""
        return self._ucs_of(ucs).to_frame(points)

    @property
    def active_layer(self) -> str:
        return self.info().active_layer

    def __len__(self) -> int:
        return self.info().objects

    def __repr__(self) -> str:
        i = self.info()
        return f"<kentos.cad.Document {i.name!r}: {i.objects} nesne, revizyon {i.revision}>"

    # ------------------------------------------------------------ layers

    def layers(self) -> list[LayerNode]:
        """The layer tree: groups and layers, with their styles and flags."""
        return [decode(LayerNode, n, "Document.layers") for n in json.loads(self._session.layers())]

    def blocks(self) -> list[BlockDefinition]:
        """The block definitions in the drawing's order (docs/adr/0144): each its
        id, name, base point and objects (their ids local to it)."""
        return [decode(BlockDefinition, b, "Document.blocks") for b in json.loads(self._session.blocks())]

    def arrays(self) -> list[dict[str, Any]]:
        """The associative arrays in the order they were made (docs/adr/0300): each its id, source block, first
        place (`p`, `scale`, `rotation`, `mirror`), layout (`cad.entities.array`'s) and items, one per place: the
        insert there (`uid`; none when removed by hand) and `kept` when it was changed by hand. Made by
        `cad.entities.array` with `associative=True`."""
        return list(json.loads(self._session.arrays()))

    def constraints(self) -> dict[str, Any]:
        """The drawing's geometric and dimensional constraints and its parameter table (docs/adr/0334): `constraints`,
        each its id, rule (`kind` and the parts it names: an object's uid and its point, segment or circle) and
        `reference` when it only measures, and `parameters`, each its name, kind, expression and range. Both are
        absent when empty. Changed by `cad.constraints.edit`, read with the diagnosis by `cad.constraints.query`."""
        return dict(json.loads(self._session.constraints()))

    @contextmanager
    def relaxed(self) -> Iterator[None]:
        """The writes inside relax the constraints (docs/adr/0337 §1, AutoCAD's Shift while editing): no solve follows
        them; the constraints a write leaves unmet go with it, the warning ``constraints_relaxed`` says how many.
        Outside, every write that changes constrained objects is followed by a solve that keeps them, or writes
        nothing (``constraint_conflict``)."""
        _host(self._session.relax, True)
        try:
            yield
        finally:
            _host(self._session.relax, False)

    def all_layers(self) -> Iterator[LayerNode]:
        """Every layer of the tree (not the groups), depth first."""
        for n in _walk(self.layers()):
            if n.type == LayerNodeType.LAYER:
                yield n

    def layer(self, key: str) -> LayerNode:
        """A layer or group by its id, else by its name when only one has it."""
        nodes = list(_walk(self.layers()))
        for n in nodes:
            if n.id == key:
                return n
        named = [n for n in nodes if n.name == key]
        if len(named) == 1:
            return named[0]
        if named:
            ids = ", ".join(n.id for n in named)
            raise UnknownObject("unknown_object", f"“{key}” adlı {len(named)} katman var; kimliğiyle anın: {ids}.")
        raise UnknownObject("unknown_object", f"“{key}” kimlikli ya da adlı katman yok.")

    # ------------------------------------------------------------ objects

    def page(
        self,
        *,
        layer: str | LayerNode | None = None,
        kinds: str | Iterable[str] | None = None,
        bbox: BoundsLike | None = None,
        after: str | None = None,
        limit: int | None = None,
    ) -> Page:
        """One page of objects in the drawing's order: on ``layer``, of
        ``kinds`` (``"polygon"``, ``["line", "polyline"]``), meeting ``bbox``,
        after the object ``after``; ``limit`` 1000 by default, 10 000 at most."""
        if isinstance(layer, LayerNode):
            layer = layer.id
        if isinstance(kinds, str):
            kinds = [kinds]
        raw = json.loads(
            _host(
                self._session.entities,
                layer,
                None if kinds is None else list(kinds),
                _bbox(bbox),
                after,
                limit,
            )
        )
        items = [Record(i["uid"], decode(Entity, i["entity"], "Document.page"), i.get("stamp")) for i in raw["items"]]
        return Page(items, raw.get("next"))

    def entities(
        self,
        *,
        layer: str | LayerNode | None = None,
        kinds: str | Iterable[str] | None = None,
        bbox: BoundsLike | None = None,
        page_size: int = 1000,
    ) -> Iterator[Record]:
        """Every object that fits, page by page. Deleting the last object of
        a page while iterating ends the reading with UnknownObject: collect
        the ids first, then change."""
        after: str | None = None
        while True:
            p = self.page(layer=layer, kinds=kinds, bbox=bbox, after=after, limit=page_size)
            yield from p.items
            if p.next is None:
                return
            after = p.next

    def entity(self, uid: str) -> Record:
        """One object by its persistent id."""
        raw = json.loads(_host(self._session.entity, uid))
        return Record(raw["uid"], decode(Entity, raw["entity"], "Document.entity"), raw.get("stamp"))

    def measure(self, uid: str, method: str = "plane") -> Measure:
        """An object's area, length and box from its source geometry.

        ``method``: ``plane`` (the project's plane, the default), ``ellipsoid``
        (on its coordinate system's ellipsoid, metres) or ``ground`` (at the
        project's mean ellipsoidal height); ``basis`` says which
        (docs/adr/0256 §4)."""
        raw = json.loads(_host(self._session.measure, uid, method))
        return Measure(
            uid=raw["uid"],
            kind=raw["kind"],
            area=raw.get("area"),
            length=raw.get("length"),
            bounds=decode(Bounds, raw["bounds"], "Document.measure"),
            units=_units(raw.get("units")),
            basis=_basis(raw.get("basis")),
        )

    def measure_many(self, uids: Sequence[str], method: str = "plane") -> Measures:
        """Many objects' measures taken one way, at most 1000: each object,
        the sums, the units and the basis once (docs/adr/0256 §4)."""
        raw = json.loads(_host(self._session.measure_many, json.dumps(list(uids)), method))
        return Measures(
            items=[
                MeasureItem(
                    uid=i["uid"],
                    kind=i["kind"],
                    area=i.get("area"),
                    length=i.get("length"),
                    bounds=decode(Bounds, i["bounds"], "Document.measure_many"),
                )
                for i in raw["items"]
            ],
            count=int(raw["totals"]["count"]),
            area=float(raw["totals"]["area"]),
            length=float(raw["totals"]["length"]),
            units=_units(raw.get("units")),
            basis=_basis(raw.get("basis")),
        )

    # ------------------------------------------------------------ selection

    def select(self, uids: Iterable[str]) -> int:
        """Selects these objects by their persistent ids (docs/adr/0622 §7).

        In the desktop's Python console it is the window's selection, what its
        tools act on next (the console writes the selections made by hand as
        this call too); elsewhere the drawing's own, which :meth:`selection`
        gives back. An unknown id raises :class:`UnknownObject` and changes
        nothing. ``select([])`` clears it. Returns how many are selected."""
        return int(_host(self._session.select, json.dumps(list(uids))))

    def selection(self) -> list[str]:
        """Selected persistent ids, omitting objects deleted since; shared session state with MCP."""
        return [str(u) for u in json.loads(_host(self._session.selection))]

    # ------------------------------------------------------------ history

    def undo(self) -> str | None:
        """Undoes the last step; its name, or None when there is none."""
        step: str | None = _host(self._session.undo)
        return step

    def redo(self) -> str | None:
        """Redoes the last undone step; its name, or None."""
        step: str | None = _host(self._session.redo)
        return step

    @contextmanager
    def group(self, label: str) -> Iterator[Document]:
        """Every command inside is one undo step named ``label``; an
        exception takes all of it back and goes on. No group inside a group,
        no save while one is open. In the desktop's console a run is one step
        already: a group names it, and a group that fails takes the whole run
        back at its end, even when the script goes on.

        >>> with doc.group("Parselleri böl"):
        ...     cad.entities.edit(doc, operation="areaSplit", changes=[...])
        """
        _host(self._session.begin_group, label)
        try:
            yield self
        except BaseException:
            _host(self._session.cancel_group)
            raise
        _host(self._session.end_group)

    # ------------------------------------------------------------ any command

    def run(
        self,
        command: str,
        input: Mapping[str, Any] | _Model,
        *,
        op: str = "execute",
        version: int | None = None,
        request_id: str | None = None,
        expected: Expected | None = None,
    ) -> dict[str, Any]:
        """Any command of this host by its id: the input as the catalog's
        JSON (or a typed input), the whole ``CommandResult`` as JSON. The
        host refuses an unknown id, another version or input not of the
        command's type (TODOS.md PY-11). With ``request_id`` or ``expected``
        the call goes by the protocol (docs/adr/0253) and the answer is the
        whole ``CallAnswer`` (the summary, the plan's id, the journal row).
        The typed wrappers (``kentos.cad.polygon.create``) are the usual way."""
        payload = input.to_json() if isinstance(input, _Model) else dict(input)
        if request_id is not None or expected is not None:
            return self._call(command, version, op, payload, request_id, expected)
        return self._command(command, version, op, payload)

    def call(
        self,
        command: str,
        input: Mapping[str, Any] | _Model,
        *,
        op: str = "execute",
        version: int | None = None,
        request_id: str | None = None,
        expected: Expected | None = None,
    ) -> Outcome[Any]:
        """A command by the protocol (docs/adr/0253): validates, plans or
        writes, and says what it touched (``summary``), the plan's id
        (``plan_id``; ``expected=Expected(plan=…)`` makes the write equal
        it), whether a request sent again was answered from before
        (``replayed``), its journal row (``call``). A refusal is in the
        answer (``error``, with ``uids`` and ``retryable``), not raised."""
        payload = input.to_json() if isinstance(input, _Model) else dict(input)
        return call_outcome(command, self._call(command, version, op, payload, request_id, expected), None)

    def start(
        self,
        command: str,
        input: Mapping[str, Any] | _Model,
        *,
        version: int | None = None,
        request_id: str | None = None,
        expected: Expected | None = None,
    ) -> Job:
        """Starts a command as a job (docs/adr/0253 §9): computed on a copy
        of the drawing on another thread, the drawing free meanwhile; its
        write comes in when the job is looked at (``job.wait()``)."""
        payload = input.to_json() if isinstance(input, _Model) else dict(input)
        envelope = _envelope(command, version, "execute", payload, request_id, expected)
        raw = json.loads(_host(self._session.start, dumps(envelope)))
        return Job(self, str(raw["id"]))

    def journal(self) -> dict[str, Any]:
        """The drawing's journal (``kentos.replay/1``, docs/adr/0253 §3):
        every write in order, a command's with its input, its algorithm and
        what it made; an edit made without a command too. JSON, to keep or
        to :meth:`replay` on another drawing."""
        raw: dict[str, Any] = json.loads(_host(self._session.journal))
        return raw

    def replay(self, journal: Mapping[str, Any]) -> ReplayReport:
        """Runs a journal's commands again on this drawing, in order, the ids
        earlier rows made mapped to the ones made here; what became of each."""
        return ReplayReport.from_json(json.loads(_host(self._session.replay, json.dumps(dict(journal)))))

    def _command(self, command: str, version: int | None, op: str, payload: Any) -> dict[str, Any]:
        text = _host(self._session.run, command, op, dumps(payload), version)
        answer: dict[str, Any] = json.loads(text)
        return answer

    def _call(
        self,
        command: str,
        version: int | None,
        op: str,
        payload: Any,
        request_id: str | None,
        expected: Expected | None,
    ) -> dict[str, Any]:
        envelope = _envelope(command, version, op, payload, request_id, expected)
        answer: dict[str, Any] = json.loads(_host(self._session.call, dumps(envelope)))
        return answer

    def _job(self, method: str, id: str) -> dict[str, Any]:
        raw: dict[str, Any] = json.loads(_host(getattr(self._session, method), id))
        return raw


def _envelope(
    command: str, version: int | None, op: str, payload: Any, request_id: str | None, expected: Expected | None
) -> dict[str, Any]:
    out: dict[str, Any] = {"command": command, "op": op, "input": payload}
    if version is not None:
        out["version"] = version
    if request_id is not None:
        out["requestId"] = request_id
    if expected is not None:
        out["expected"] = expected.to_json()
    return out


def _walk(nodes: Iterable[LayerNode]) -> Iterator[LayerNode]:
    for n in nodes:
        yield n
        yield from _walk(n.children)


__all__ = ["Basis", "BoundsLike", "Conventions", "Document", "UcsFrame", "WORLD_UCS", "DocumentInfo", "Exported", "Measure", "MeasureItem", "Measures", "Page", "Printed", "Record", "Saved", "Units", "current"]
