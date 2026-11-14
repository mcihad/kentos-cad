"""What an extension offers KentOS from its Python (plan/eklenti.md ADIM 19; docs/adr/8057 §6).

An extension's code marks each function with the contribution it answers to, by the local id its manifest gives::

    from kentos import ext

    @ext.command("check-setbacks")
    def check_setbacks(ctx: ext.CommandContext) -> ext.CommandResult:
        for record in ctx.doc.entities(layer="yapi"):
            ctx.check()
            ...
        return ctx.report(rows, title="Yaklaşma mesafesi denetimi")

KentOS knows the contributions from the manifest without importing anything; it imports the module only when one is
called, in the extension's own process and environment (``kentos.ext_host``), and refuses a function not marked with
the kind and id the manifest names (PY-RUN-08).

- ``ctx.doc`` is the drawing open in KentOS, as :class:`kentos.cad.Document` reads and writes it; every request goes to
  KentOS, which answers it on its own drawing within what the extension was granted (``document.read``,
  ``document.write``). A run is one undo step; a run that raises or is stopped is taken back.
- ``ctx.check()`` raises :class:`Stopped` once the user pressed Durdur; ``ctx.is_stopped()`` asks without raising.
  KentOS stops a run that does not stop on its own after a few seconds.
- ``ctx.progress()`` and ``ctx.log()`` say how far it is and what it found, as it goes.
- ``ctx.doc.select(uids)`` leaves what it found selected in the drawing when it ends (a session result, not a change).
- What this KentOS cannot do yet raises :class:`NotAvailable`: KentOS says so calmly, as a missing ability rather
  than a failure of the extension's code.

An İşlemler tool is a function of its parameters, named as its descriptor (``tools/<id>.tool.json``) names them; it
uses whatever its own environment holds (shapely, NumPy, pandas, SciPy, GeoPandas …: its lock decides)::

    @ext.processing("taks")
    def taks(ctx: ext.ProcessingContext, parcels: ext.FeatureInput, buildings: ext.FeatureInput, limit: float):
        shapes = [shapely.geometry.shape(f) for f in parcels]
        ...
        return {"parcels": [ext.NewFeature(source=f, attrs={"taks": r}) for f, r in zip(parcels, ratios)]}

- Its inputs come as data, as they were when the run began (:class:`Feature`): it does not read the drawing, and the
  drawing is not held while it computes (PY-TOOL-09). Each input speaks GeoJSON (``__geo_interface__``, RFC 7946):
  shapely, GeoPandas and the other Python libraries read it as it is.
- It returns its outputs by the descriptor's names; KentOS checks every one (PY-TOOL-10) and writes them in one undo
  step: new objects to the output layer the user chose (:class:`NewFeature`; a shape is any GeoJSON geometry or
  anything with ``__geo_interface__``, a GeoDataFrame whole), rows to a table (dicts, or a DataFrame), numbers and
  texts. A library's own numbers and arrays (anything with ``item()`` or ``tolist()``) come as plain ones.

A separate process is not a sandbox: an extension runs with the user's rights (PY-RUN-19, 20).
"""

from __future__ import annotations

import json
import math
import os
import threading
import time
from collections.abc import Iterable, Iterator, Sequence
from dataclasses import dataclass, field
from typing import Any, Callable, TypeVar, overload

__all__ = [
    "AuditFinding",
    "CommandContext",
    "CommandResult",
    "Feature",
    "FeatureInput",
    "GisAuditFacts",
    "MapToolContext",
    "NewFeature",
    "NotAvailable",
    "ProcessingContext",
    "SemanticFix",
    "Shape",
    "Stopped",
    "audit",
    "command",
    "expr",
    "map_tool",
    "marked",
    "point",
    "polygon",
    "polyline",
    "processing",
]

F = TypeVar("F", bound=Callable[..., Any])

#: The kinds a function can be marked with, as the manifest names its entry points.
KINDS = ("command", "processing", "audit", "mapTool", "expression")


def _mark(kind: str, local: str) -> Callable[[F], F]:
    if not isinstance(local, str) or not local:
        raise ValueError(f"@ext.{kind}: the manifest's local id is needed, e.g. @ext.{kind}(\"my-id\")")

    def wrap(fn: F) -> F:
        setattr(fn, "__kentos_ext__", (kind, local))
        return fn

    return wrap


def command(local: str) -> Callable[[F], F]:
    """A command (``entrypoints.commands``): a button, a menu item, a context menu entry."""
    return _mark("command", local)


def processing(local: str) -> Callable[[F], F]:
    """An İşlemler tool (``entrypoints.processing``)."""
    return _mark("processing", local)


def audit(local: str) -> Callable[[F], F]:
    """An audit rule (``entrypoints.audits``)."""
    return _mark("audit", local)


def map_tool(local: str) -> Callable[[F], F]:
    """A map tool (``entrypoints.mapTools``)."""
    return _mark("mapTool", local)


def expr(local: str) -> Callable[[F], F]:
    """An expression function (``entrypoints.expressions``)."""
    return _mark("expression", local)


def marked(fn: Any) -> tuple[str, str] | None:
    """The kind and local id a function is marked with; none when it is not."""
    mark = getattr(fn, "__kentos_ext__", None)
    if isinstance(mark, tuple) and len(mark) == 2 and all(isinstance(p, str) for p in mark):
        return (mark[0], mark[1])
    return None


class Stopped(Exception):
    """The user stopped the run (Durdur). What the run wrote is taken back."""


class NotAvailable(Exception):
    """What this KentOS cannot do (yet, or in this runtime): the run ends, what it wrote is taken back, and KentOS says
    the ability is missing instead of reporting the extension's code as failed."""


@dataclass
class CommandResult:
    """What a command says when it ends: a line, and rows to show (a report)."""

    title: str | None = None
    message: str | None = None
    rows: list[dict[str, Any]] = field(default_factory=list)
    cancelled: bool = False

    def to_json(self) -> dict[str, Any]:
        out: dict[str, Any] = {"kind": "report" if self.rows else "message"}
        if self.title is not None:
            out["title"] = self.title
        if self.message is not None:
            out["message"] = self.message
        if self.rows:
            out["rows"] = self.rows
            # The columns in the order the rows first give them: a JSON object's keys have no order KentOS keeps.
            columns: dict[str, None] = {}
            for row in self.rows:
                columns.update(dict.fromkeys(row))
            out["columns"] = list(columns)
        if self.cancelled:
            out["cancelled"] = True
        return out


class _Link:
    """What a context needs of its process: its stop flag and a way to tell KentOS how far it is."""

    def __init__(self, send: Callable[[dict[str, Any]], None], run: int, stop: threading.Event) -> None:
        self._send = send
        self.run = run
        self.stop = stop
        self._last_progress = 0.0

    def progress(self, done: float, total: float | None, text: str | None) -> None:
        # At most ten a second: the user cannot read more, and the channel stays free.
        now = time.monotonic()
        if total is not None and done < total and now - self._last_progress < 0.1:
            return
        self._last_progress = now
        message: dict[str, Any] = {"type": "progress", "id": self.run, "done": done}
        if total is not None:
            message["total"] = total
        if text:
            message["text"] = str(text)[:500]
        self._send(message)

    def log(self, level: str, text: str) -> None:
        self._send({"type": "log", "id": self.run, "level": level, "text": str(text)[:4000]})


class CommandContext:
    """What a command is given: the drawing, its settings and parameters, and its run's controls."""

    def __init__(
        self,
        link: _Link,
        doc: Any,
        settings: dict[str, Any] | None = None,
        params: dict[str, Any] | None = None,
        *,
        source: str | None = None,
    ) -> None:
        self._link = link
        #: The drawing open in KentOS (kentos.cad.Document); none when no drawing is open.
        self.doc = doc
        self.settings: dict[str, Any] = dict(settings or {})
        self.params: dict[str, Any] = dict(params or {})
        # The package's folder, as installed: where its own files (schema.json) are read from.
        self._source = source

    def is_stopped(self) -> bool:
        """The user pressed Durdur."""
        return self._link.stop.is_set()

    def check(self) -> None:
        """Raises :class:`Stopped` once the user pressed Durdur: call it in long loops."""
        if self._link.stop.is_set():
            raise Stopped()

    def progress(self, done: float, total: float | None = None, text: str | None = None) -> None:
        """How far the run is; also a place where Durdur is answered."""
        self.check()
        self._link.progress(done, total, text)

    def log(self, text: str, level: str = "info") -> None:
        """A line in the extension's log as the run goes (``info``, ``warning``, ``error``)."""
        self._link.log(level if level in ("info", "warning", "error") else "info", text)

    def report(self, rows: list[dict[str, Any]], title: str | None = None, message: str | None = None) -> CommandResult:
        """Rows to show the user, each a dict of column → value."""
        return CommandResult(title=title, message=message, rows=[dict(r) for r in rows])

    def message(self, text: str, title: str | None = None) -> CommandResult:
        """A line to show the user."""
        return CommandResult(title=title, message=text)

    def cancelled(self) -> CommandResult:
        """What a run that saw Durdur returns; its writes are taken back."""
        return CommandResult(cancelled=True)

    def apply_declarative_schema(self, path: str = "schema.json") -> CommandResult:
        """Sets up the layers, value lists and relations the package's ``schema.json`` declares (docs/adr/8057 §23), in
        the run's one undo step, and says what it did, a row each.

        Each layer's id is its schema name (``mezar_parsel``): a run finds it again. What is there is left as it is and
        only what is missing is added: a layer's missing fields, a missing value list or relation; a second run changes
        nothing. Needs the drawing and ``document.write``."""
        if self.doc is None:
            raise NotAvailable(
                "Açık çizim yok ya da eklentiye çizimi okuma ve yazma izni verilmedi: katman şeması kurulamaz."
            )
        schema = self._package_json(path)
        rows = _install_schema(self.doc, schema, path)
        added = sum(1 for r in rows if r["Sonuç"] != "vardı")
        if not added:
            return CommandResult(title="Katman şeması", message="Şema zaten kurulu: değişiklik yok.", rows=rows)
        counts: dict[str, int] = {}
        for r in rows:
            if r["Sonuç"] != "vardı":
                counts[r["Tür"]] = counts.get(r["Tür"], 0) + 1
        # Written out, not lowered: Python's lower() makes “İlişki” “i̇lişki” (a dotted i and a combining dot).
        words = {"Grup": "grup", "Değer listesi": "değer listesi", "Katman": "katman", "İlişki": "ilişki"}
        said = ", ".join(f"{n} {words.get(kind, kind)}" for kind, n in counts.items())
        return CommandResult(title="Katman şeması", message=f"Şema kuruldu: {said}.", rows=rows)

    def _package_json(self, path: str) -> Any:
        """A JSON file of the package, by its path in it; never a file outside it."""
        if not self._source:
            raise NotAvailable("Eklentinin paket klasörü bilinmiyor: paketin dosyaları okunamaz.")
        parts = path.replace("\\", "/").split("/")
        if path.startswith("/") or any(p in ("", ".", "..") for p in parts):
            raise ValueError(f"“{path}” paketin içinde bir dosya yolu değil (ör. schema.json).")
        full = os.path.join(self._source, *parts)
        try:
            with open(full, encoding="utf-8") as f:
                return json.load(f)
        except FileNotFoundError:
            raise ValueError(f"Pakette “{path}” yok.") from None
        except json.JSONDecodeError as e:
            raise ValueError(f"“{path}” JSON değil: satır {e.lineno}, sütun {e.colno}: {e.msg}.") from None


#: The layer schema this KentOS installs (docs/adr/8057 §23): ``schema.json``'s ``format`` and ``version``, both
#: optional.
SCHEMA_FORMAT = "kentos.layerSchema"
SCHEMA_VERSION = 1


def _walk_nodes(nodes: Iterable[Any]) -> Iterator[Any]:
    for n in nodes:
        yield n
        yield from _walk_nodes(getattr(n, "children", None) or [])


def _schema_ids(doc: Any, kind: str) -> set[str]:
    """The ids of the project's value lists or relations, every page."""
    from kentos import cad

    ids: set[str] = set()
    cursor = None
    while True:
        page = cad.schema.describe(doc, kinds=[kind], limit=100, cursor=cursor)
        ids.update(item.id for item in page.items)
        cursor = page.next
        if not cursor:
            return ids


def _install_schema(doc: Any, schema: Any, path: str) -> list[dict[str, Any]]:
    """What ``apply_declarative_schema`` writes: the group, the value lists, the layers with their fields, the relations,
    in that order (a field links a list, a relation its layers); what is there is kept. A row each."""
    from kentos import cad

    if not isinstance(schema, dict) or not isinstance(schema.get("layers"), list) or not schema["layers"]:
        raise ValueError(f"“{path}” bir katman şeması değil: en az bir katmanlı bir layers listesi olmalı.")
    if schema.get("format", SCHEMA_FORMAT) != SCHEMA_FORMAT or schema.get("version", SCHEMA_VERSION) > SCHEMA_VERSION:
        raise NotAvailable(
            f"“{path}” {schema.get('format')} {schema.get('version')} biçiminde; bu KentOS {SCHEMA_FORMAT} "
            f"{SCHEMA_VERSION} kurar."
        )
    rows: list[dict[str, Any]] = []
    nodes = {n.id: n for n in _walk_nodes(doc.layers())}
    parent = None
    group = schema.get("group")
    if isinstance(group, dict) and group.get("id"):
        parent = str(group["id"])
        made = parent not in nodes
        if made:
            cad.layers.edit(doc, operation="add", kind="group", id=parent, name=str(group.get("name") or parent))
        rows.append({"Tür": "Grup", "Kimlik": parent, "Ad": str(group.get("name") or parent),
                     "Sonuç": "eklendi" if made else "vardı"})
    if schema.get("domains"):
        have = _schema_ids(doc, "domains")
        for d in schema["domains"]:
            made = d["id"] not in have
            if made:
                op: dict[str, Any] = {"kind": "create", "id": d["id"], "name": d.get("name") or d["id"],
                                      "domainKind": d.get("kind", "text")}
                for key in ("values", "min", "max", "minExclusive", "maxExclusive", "unit", "description"):
                    if key in d:
                        op[key] = d[key]
                cad.domains.edit(doc, edit=cad.DomainsOp.from_json(op))
            rows.append({"Tür": "Değer listesi", "Kimlik": d["id"], "Ad": d.get("name") or d["id"],
                         "Sonuç": "eklendi" if made else "vardı"})
    for layer in schema["layers"]:
        lid = str(layer["name"])
        title = str(layer.get("title") or lid)
        wanted = list(layer.get("fields") or [])
        if lid not in nodes:
            given: dict[str, Any] = {"operation": "add", "id": lid, "name": title}
            if layer.get("geometry"):
                given["geometry"] = layer["geometry"]
            if parent:
                given["parent"] = parent
            cad.layers.edit(doc, **given)
            if wanted:
                cad.layers.edit(doc, operation="fields", layer=lid,
                                fields=[cad.LayerField.from_json(f) for f in wanted])
            result = "eklendi"
        else:
            present = list(getattr(nodes[lid], "fields", None) or [])
            names = {f.name for f in present}
            missing = [f for f in wanted if f.get("name") not in names]
            if missing:
                cad.layers.edit(doc, operation="fields", layer=lid,
                                fields=present + [cad.LayerField.from_json(f) for f in missing])
                result = f"vardı; {len(missing)} alan eklendi"
            else:
                result = "vardı"
        rows.append({"Tür": "Katman", "Kimlik": lid, "Ad": title, "Sonuç": result})
    if schema.get("relations"):
        have = _schema_ids(doc, "relations")
        for r in schema["relations"]:
            made = r["id"] not in have
            if made:
                relation = {key: r[key] for key in ("id", "name", "parent", "child", "field", "onDelete") if key in r}
                relation.setdefault("name", r["id"])
                cad.relations.edit(doc, edit=cad.RelationsOp.from_json({"kind": "create", "relation": relation}))
            rows.append({"Tür": "İlişki", "Kimlik": r["id"], "Ad": r.get("name") or r["id"],
                         "Sonuç": "eklendi" if made else "vardı"})
    return rows


class Feature:
    """An object a tool was given, as it was when the run began: its persistent id (``uid``), its kind, its layer's
    name, its attributes (texts, as KentOS keeps them) and its shape as GeoJSON (``geometry``; arcs tessellated as the
    drawing draws them). It is a GeoJSON feature itself (``__geo_interface__``): ``shapely.geometry.shape(f)`` and
    ``geopandas.GeoDataFrame.from_features(features)`` read it. A field the drawing hides from third parties comes
    with its value cut, as an agent reads it (docs/adr/0738)."""

    __slots__ = ("uid", "kind", "layer", "attrs", "geometry", "_raw", "_entity")

    def __init__(self, raw: dict[str, Any]) -> None:
        entity = raw.get("entity") if isinstance(raw.get("entity"), dict) else {}
        self.uid: str = str(raw.get("uid", ""))
        self.kind: str = str(entity.get("kind", ""))
        self.layer: str = str(raw.get("layer", ""))
        self.attrs: dict[str, str] = {str(k): str(v) for k, v in (entity.get("attrs") or {}).items()}
        #: Its shape, a GeoJSON geometry (Point, LineString, Polygon, their multi-part kinds); none for a kind GeoJSON
        #: has no shape for (a text, a dimension).
        self.geometry: dict[str, Any] | None = raw.get("geometry") if isinstance(raw.get("geometry"), dict) else None
        self._raw: dict[str, Any] = entity
        self._entity: Any = None

    @property
    def __geo_interface__(self) -> dict[str, Any]:
        """The object as a GeoJSON feature: its id, its shape and its attributes as properties."""
        return {"type": "Feature", "id": self.uid, "geometry": self.geometry, "properties": dict(self.attrs)}

    @property
    def id(self) -> str:
        """Its persistent id (``uid``): what a finding names (``AuditFinding.feature_id``) and a fix writes to."""
        return self.uid

    @property
    def entity(self) -> Any:
        """Its whole object as :mod:`kentos.cad` types it (what ``ctx.doc.entities`` gives a command)."""
        if self._entity is None:
            from .cad import Entity
            from .cad._runtime import decode

            self._entity = decode(Entity, self._raw, "Feature.entity")
        return self._entity

    @property
    def pts(self) -> list[tuple[float, float]]:
        """Its vertices, ``(x, y)`` in metres, arcs tessellated: a point's one, a line's, an area's outline (without
        repeating the first); its first part's for a multi-part one; none for a kind without a shape."""
        g = self.geometry or {}
        kind, c = g.get("type"), g.get("coordinates")
        if not c:
            return []
        if kind == "Point":
            return [(float(c[0]), float(c[1]))]
        if kind in ("MultiPoint", "LineString"):
            line = c
        elif kind in ("Polygon", "MultiLineString"):
            line = c[0]
        elif kind == "MultiPolygon":
            line = c[0][0]
        else:
            return []
        out = [(float(p[0]), float(p[1])) for p in line]
        if kind in ("Polygon", "MultiPolygon") and len(out) > 1 and out[0] == out[-1]:
            out.pop()
        return out

    def number(self, name: str, default: float | None = None) -> float | None:
        """An attribute as a number (``12,5`` read as 12.5); ``default`` when it is empty or not a number."""
        text = self.attrs.get(name, "").strip().replace(" ", "")
        if "," in text and "." not in text:
            text = text.replace(",", ".")
        try:
            value = float(text)
        except ValueError:
            return default
        return value if math.isfinite(value) else default

    def __repr__(self) -> str:
        return f"Feature({self.kind} {self.uid} on {self.layer!r})"


class FeatureInput(Sequence[Feature]):
    """A features parameter's objects, in the order KentOS resolved them, and how it described them
    (``"12 kapalı alan; seçili nesneler"``). A GeoJSON FeatureCollection itself (``__geo_interface__``):
    ``geopandas.GeoDataFrame.from_features(parcels)`` reads it whole."""

    def __init__(self, raw: dict[str, Any] | None) -> None:
        raw = raw or {}
        self.description: str = str(raw.get("description", ""))
        self._raw: list[Any] = list(raw.get("features") or [])
        self._items: list[Feature | None] = [None] * len(self._raw)

    @property
    def __geo_interface__(self) -> dict[str, Any]:
        return {"type": "FeatureCollection", "features": [f.__geo_interface__ for f in self]}

    def __len__(self) -> int:
        return len(self._raw)

    @overload
    def __getitem__(self, index: int) -> Feature: ...

    @overload
    def __getitem__(self, index: slice) -> list[Feature]: ...

    def __getitem__(self, index: int | slice) -> Feature | list[Feature]:
        if isinstance(index, slice):
            return [self[i] for i in range(*index.indices(len(self)))]
        item = self._items[index]
        if item is None:
            item = Feature(self._raw[index])
            self._items[index] = item
        return item

    def __iter__(self) -> Iterator[Feature]:
        for i in range(len(self)):
            yield self[i]

    def __repr__(self) -> str:
        return f"FeatureInput({len(self)} nesne: {self.description!r})"


def _xy(value: Any) -> list[float]:
    if hasattr(value, "x") and hasattr(value, "y"):
        return [float(value.x), float(value.y)]
    x, y = value[0], value[1]
    return [float(x), float(y)]


def _pairs(pts: Iterable[Any]) -> list[list[float]]:
    # An array of pairs (NumPy's and the like) reads fastest through its tolist, and its rows are arrays, not pairs.
    tolist = getattr(pts, "tolist", None)
    if callable(tolist):
        pts = tolist()
    return [_xy(p) for p in pts]


def _ring(pts: Iterable[Any]) -> list[list[float]]:
    ring = _pairs(pts)
    if ring and ring[0] != ring[-1]:
        ring.append(list(ring[0]))
    return ring


def point(x: float, y: float) -> dict[str, Any]:
    """A point, GeoJSON, in metres."""
    return {"type": "Point", "coordinates": _xy((x, y))}


def polyline(pts: Iterable[Any]) -> dict[str, Any]:
    """A line, GeoJSON: its vertices, ``(x, y)`` pairs (an array of them too)."""
    return {"type": "LineString", "coordinates": _pairs(pts)}


def polygon(pts: Iterable[Any], holes: Iterable[Iterable[Any]] = ()) -> dict[str, Any]:
    """An area, GeoJSON: its outline's vertices and its holes', each ring closed as GeoJSON writes it."""
    return {"type": "Polygon", "coordinates": [_ring(pts), *(_ring(h) for h in holes)]}


@dataclass
class NewFeature:
    """An object a tool writes to an output layer: a shape of its own (``geometry``: a GeoJSON geometry, from
    :func:`point`, :func:`polyline`, :func:`polygon`, or anything with ``__geo_interface__``, a shapely shape) or the
    shape of one of its inputs (``source``: the :class:`Feature`, or its ``uid``), with its attributes; an input's
    attributes come with it unless ``keep_attrs`` is false. KentOS checks every value before anything is written."""

    attrs: dict[str, Any] = field(default_factory=dict)
    geometry: Any = None
    source: Feature | str | None = None
    keep_attrs: bool = True
    label: str | None = None

    def to_json(self) -> dict[str, Any]:
        out: dict[str, Any] = {"attrs": {str(k): v for k, v in self.attrs.items()}}
        if self.geometry is not None:
            out["geometry"] = self.geometry
        if self.source is not None:
            out["source"] = self.source.uid if isinstance(self.source, Feature) else str(self.source)
            if not self.keep_attrs:
                out["keepAttrs"] = False
        if self.label is not None:
            out["label"] = str(self.label)
        return out


class ProcessingContext(CommandContext):
    """What an İşlemler tool is given besides its parameters: its run's controls (``check``, ``progress``, ``log``)
    and its settings. Its inputs come as its parameters, as data; it has no drawing (``doc`` is none)."""

    def __init__(
        self,
        link: _Link,
        settings: dict[str, Any] | None = None,
        params: dict[str, Any] | None = None,
    ) -> None:
        super().__init__(link, None, settings, params)
        self._summary: str | None = None
        self._select: list[str] | None = None

    def read_features(self, features: FeatureInput) -> Iterator[Feature]:
        """A features parameter's objects one by one, Durdur answered between them."""
        for feature in features:
            self.check()
            yield feature

    def summary(self, text: str) -> None:
        """The run's one line in İşlemler' history, in place of KentOS's own ("12 nesne eklendi"). It says what was
        computed, not where it went: the user may send the result to a file instead of the drawing, and KentOS adds
        where it was written ("Sonuç … dosyasına yazıldı")."""
        self._summary = str(text)[:500]

    def select(self, features: Iterable[Feature | str]) -> None:
        """Leaves these inputs selected in the drawing when the run ends (a session result, not a change)."""
        self._select = [f.uid if isinstance(f, Feature) else str(f) for f in features]


class Shape:
    """A shape the user drew with an extension's map tool (plan/eklenti.md ADIM 19), as GeoJSON (``__geo_interface__``,
    RFC 7946): a ``Point``, a ``LineString`` or a ``Polygon`` (a rectangle's four corners; a circle's ring of chords,
    with ``circle`` its centre and radius), in the project's plane: x east, y north, metres. shapely reads it as it is:
    ``shapely.geometry.shape(shape)``."""

    __slots__ = ("type", "coordinates", "circle")

    def __init__(self, raw: dict[str, Any]) -> None:
        self.type: str = str(raw.get("type", ""))
        self.coordinates: Any = raw.get("coordinates")
        circle = raw.get("circle")
        #: A circle's centre ``(x, y)`` and radius, when the tool drew a circle; none otherwise.
        self.circle: tuple[tuple[float, float], float] | None = (
            ((float(circle["center"][0]), float(circle["center"][1])), float(circle["radius"]))
            if isinstance(circle, dict)
            else None
        )

    @property
    def __geo_interface__(self) -> dict[str, Any]:
        return {"type": self.type, "coordinates": self.coordinates}

    @property
    def points(self) -> list[tuple[float, float]]:
        """Its corners in order: a point's one, a line's, an area's outer ring without its closing repeat."""
        if self.type == "Point":
            return [(float(self.coordinates[0]), float(self.coordinates[1]))]
        ring = self.coordinates[0] if self.type == "Polygon" else self.coordinates
        pts = [(float(p[0]), float(p[1])) for p in ring]
        if self.type == "Polygon" and len(pts) > 1 and pts[0] == pts[-1]:
            pts.pop()
        return pts

    def __repr__(self) -> str:
        return f"Shape({self.type}, {len(self.points)} köşe)"


class MapToolContext(CommandContext):
    """A map tool's context (plan/eklenti.md ADIM 19): a command's, and the writes of what the user drew. A placement
    is one undo step; one that raises is taken back."""

    def add_feature(self, layer: str, shape: Any, attrs: dict[str, Any] | None = None) -> CommandResult:
        """Writes ``shape`` (a :class:`Shape`, any GeoJSON geometry or anything with ``__geo_interface__``) as a new
        object of ``layer`` (its id, or its name when only one layer has it) with ``attrs``, and says what it wrote: an
        area for a polygon, a line for a line string, a point for a point."""
        if self.doc is None:
            raise NotAvailable("Açık çizim yok: harita aracının yazacağı yer yok.")
        from kentos import cad

        node = self.doc.layer(layer)
        geo = getattr(shape, "__geo_interface__", shape)
        if not isinstance(geo, dict):
            raise TypeError(f"{type(shape).__name__} bir şekil değil: GeoJSON ya da __geo_interface__ olmalı.")
        values = {str(k): "" if v is None else str(v) for k, v in (attrs or {}).items()}
        given = Shape(geo)
        if given.type == "Polygon":
            made = cad.polygon.create(self.doc, layer_id=node.id, pts=given.points, attrs=values)
            what = "alan"
        elif given.type == "LineString":
            made = cad.polyline.create(self.doc, layer_id=node.id, pts=given.points, attrs=values)
            what = "çizgi"
        elif given.type == "Point":
            made = cad.point.create(self.doc, layer_id=node.id, p=given.points[0], attrs=values)
            what = "nokta"
        else:
            raise TypeError(f"“{given.type}” şekli yazılmaz: Point, LineString ya da Polygon olmalı.")
        del made
        return CommandResult(message=f"“{node.name}” katmanına bir {what} yazıldı.")


class GisAuditFacts:
    """What an audit rule reads (docs/adr/8057 §17): the drawing's layers holding objects, each object a
    :class:`Feature` (persistent id, kind, layer, attributes, shape as GeoJSON), taken when the audit began. A rule
    writes nothing: it answers :class:`AuditFinding` s, each with an optional fix the user chooses to apply."""

    __slots__ = ("_layers", "cut")

    def __init__(self, raw: dict[str, Any]) -> None:
        self._layers: list[tuple[str, str, list[Feature]]] = [
            (str(layer.get("id", "")), str(layer.get("name", "")), [Feature(f) for f in layer.get("features") or []])
            for layer in raw.get("layers") or []
            if isinstance(layer, dict)
        ]
        #: KentOS gave only the first objects of a very large drawing (its limit): a rule may say so.
        self.cut: bool = bool(raw.get("cut"))

    def layers(self) -> list[tuple[str, str]]:
        """The layers holding objects, ``(id, name)``, in the layer tree's order."""
        return [(i, n) for i, n, _ in self._layers]

    def features(self, layer: str) -> list[Feature]:
        """A layer's objects, by its id or its name (KentOS's GIS template: ``parsel``, ``yapi``); none when no layer
        has it."""
        for i, n, found in self._layers:
            if layer in (i, n):
                return list(found)
        return []

    def all_features(self) -> list[Feature]:
        """Every object given, layer by layer."""
        return [f for _, _, found in self._layers for f in found]

    def group_related(self, layer: str, foreign_key: str) -> dict[str, list[Feature]]:
        """A layer's objects grouped by their ``foreign_key`` attribute's value (a related table's key); objects
        without it are left out."""
        out: dict[str, list[Feature]] = {}
        for f in self.features(layer):
            key = f.attrs.get(foreign_key)
            if key:
                out.setdefault(key, []).append(f)
        return out


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

    def to_json(self) -> dict[str, Any]:
        """As KentOS reads it; a severity other than error, warning or info and an empty message are the rule's
        mistakes."""
        if self.severity not in ("error", "warning", "info"):
            raise ValueError(f"Bulgunun önemi “{self.severity}” olamaz: error, warning ya da info.")
        if not str(self.message).strip():
            raise ValueError("Bulgunun iletisi boş olamaz.")
        out: dict[str, Any] = {"severity": self.severity, "message": str(self.message)[:2000]}
        if self.feature_id:
            out["featureId"] = str(self.feature_id)
        if self.fix is not None:
            out["fix"] = {"title": str(self.fix.title), "command": str(self.fix.command), "args": dict(self.fix.args)}
        return out
