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
import os
from collections.abc import Callable, Iterable, Iterator, Mapping, Sequence
from contextlib import contextmanager
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Literal, TypeVar

from .. import _native
from ._runtime import Outcome, _enum_out, _Model, call_outcome, decode, dumps
from .errors import NotRunHere, UnknownObject, host_error
from .protocol import Expected, Job, ReplayReport, Stamps
from .types import (
    BlockDefinition,
    Bounds,
    DrawingFont,
    DrawingFontName,
    DrawingUnit,
    DrawingUnitName,
    Entity,
    LayerNode,
    LayerNodeType,
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

    def measure(self, uid: str) -> str:
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
        """A length in metres in the drawing unit (4.0 m is 4000.0 in mm)."""
        return value * _PER_METRE[self.drawing]

    def to_metres(self, value: float) -> float:
        """A length in the drawing unit in metres (4000.0 mm is 4.0 m)."""
        return value / _PER_METRE[self.drawing]


_PER_METRE = {DrawingUnit.MM: 1000.0, DrawingUnit.CM: 100.0, DrawingUnit.M: 1.0}


def _units(raw: Mapping[str, Any] | None) -> Units:
    # A desktop older than docs/adr/0245 says none: its values are metres.
    raw = raw or {}
    return Units(
        length=str(raw.get("length", "m")),
        area=str(raw.get("area", "m2")),
        drawing=DrawingUnit(raw.get("drawing", "m")),
    )


@dataclass(frozen=True, slots=True)
class Measure:
    """An object's measures from its source geometry (not a drawn
    approximation), in the units ``units`` names: metres and m² in the
    project's plane, never the drawing unit."""

    uid: str
    kind: str
    area: float | None
    """Plane area of a closed shape (``units.area``, m²); None for an open one."""
    length: float | None
    """Length of a line or a path, perimeter of a closed shape (``units.length``, m); None for a point or a text."""
    bounds: Bounds
    units: Units


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
    sheet core's findings, and what a headless host leaves out (``notes``)."""

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
        - ``drawing_unit``: a local project's unit (``mm``, ``cm``, ``m``);
          one with a system is in metres.
        - ``plot_scale``: 1000 for 1:1000; by default 1 for a local project,
          1000 for any other.

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
        """A ``.kcad`` file: KCAD v2, or an old v1 JSON one (saved to a new path only)."""
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

    def export_pdf(self, path: str | os.PathLike[str], sheets: Sequence[str] | None = None) -> Printed:
        """The project's sheet book as a PDF: the sheets named (by id) or all, a page each, as the
        desktop's Dışa aktar › PDF writes them (each map at its scale, GeoPDF where the system is a
        transverse Mercator one). Sheets are made with ``kentos.cad.sheets.edit``. A headless
        host writes the maps' lines, areas and hatches but not their texts yet (``notes``)."""
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

    def measure(self, uid: str) -> Measure:
        """An object's area, length and box from its source geometry."""
        raw = json.loads(_host(self._session.measure, uid))
        return Measure(
            uid=raw["uid"],
            kind=raw["kind"],
            area=raw.get("area"),
            length=raw.get("length"),
            bounds=decode(Bounds, raw["bounds"], "Document.measure"),
            units=_units(raw.get("units")),
        )

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


__all__ = ["BoundsLike", "Document", "DocumentInfo", "Exported", "Measure", "Page", "Printed", "Record", "Saved", "Units", "current"]
