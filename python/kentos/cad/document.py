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
from typing import Any, TypeVar

from .. import _native
from ._runtime import _enum_out, _Model, decode, dumps
from .errors import NotRunHere, UnknownObject, host_error
from .types import (
    BlockDefinition,
    Bounds,
    DrawingFont,
    DrawingFontName,
    Entity,
    LayerNode,
    LayerNodeType,
    ProjectSettings,
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
    """An object of the drawing with its persistent id."""

    uid: str
    entity: Entity


@dataclass(frozen=True, slots=True)
class Page:
    """Objects in the drawing's order; ``next`` reads on (``after=``)."""

    items: list[Record]
    next: str | None


@dataclass(frozen=True, slots=True)
class Measure:
    """An object's measures in the project's units, from its source
    geometry (not a drawn approximation)."""

    uid: str
    kind: str
    area: float | None
    """Plane area of a closed shape (m²); None for an open one."""
    length: float | None
    """Length of a line or a path, perimeter of a closed shape (m); None for a point or a text."""
    bounds: Bounds


@dataclass(frozen=True, slots=True)
class Saved:
    """Where a save went, how many bytes, at which revision."""

    path: Path
    bytes: int
    revision: str


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
        name: str = "Adsız proje",
        *,
        srid: int,
        plot_scale: float = 1000.0,
        workspace: Workspace | WorkspaceName = Workspace.HYBRID,
        drawing_font: DrawingFont | DrawingFontName = DrawingFont.BARLOW,
    ) -> Document:
        """A new project, as Yeni proje makes it: the standard layer tree,
        the zone's work area, the default units. ``srid`` is the project's
        coordinate system (5254…5259 TUREF/TM, 32635…32638 UTM …); it is
        asked, never guessed."""
        return cls(
            _host(
                _native.Session.new_project,
                name,
                int(srid),
                float(plot_scale),
                _enum_out(workspace),
                _enum_out(drawing_font),
            )
        )

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
        )

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
        items = [Record(i["uid"], decode(Entity, i["entity"], "Document.page")) for i in raw["items"]]
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
        return Record(raw["uid"], decode(Entity, raw["entity"], "Document.entity"))

    def measure(self, uid: str) -> Measure:
        """An object's area, length and box from its source geometry."""
        raw = json.loads(_host(self._session.measure, uid))
        return Measure(
            uid=raw["uid"],
            kind=raw["kind"],
            area=raw.get("area"),
            length=raw.get("length"),
            bounds=decode(Bounds, raw["bounds"], "Document.measure"),
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
    ) -> dict[str, Any]:
        """Any command of this host by its id: the input as the catalog's
        JSON (or a typed input), the whole ``CommandResult`` as JSON. The
        host refuses an unknown id, another version or input not of the
        command's type (TODOS.md PY-11). The typed wrappers
        (``kentos.cad.polygon.create``) are the usual way."""
        payload = input.to_json() if isinstance(input, _Model) else dict(input)
        return self._command(command, version, op, payload)

    def _command(self, command: str, version: int | None, op: str, payload: Any) -> dict[str, Any]:
        text = _host(self._session.run, command, op, dumps(payload), version)
        answer: dict[str, Any] = json.loads(text)
        return answer


def _walk(nodes: Iterable[LayerNode]) -> Iterator[LayerNode]:
    for n in nodes:
        yield n
        yield from _walk(n.children)


__all__ = ["BoundsLike", "Document", "DocumentInfo", "Measure", "Page", "Record", "Saved", "current"]
