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

import base64
import json
import math
import os
import threading
import time
from collections.abc import Iterable, Iterator, Sequence
from dataclasses import dataclass, field
from typing import Any, Callable, TypeVar, overload

__all__ = [
    "ArchitectureFacts",
    "AuditFinding",
    "CommandContext",
    "CommandResult",
    "DoorEscape",
    "EscapeWalk",
    "Feature",
    "FeatureInput",
    "Gap",
    "GisAuditFacts",
    "InvalidGeometry",
    "LayerFacts",
    "MapToolContext",
    "NewFeature",
    "NotAvailable",
    "Overlap",
    "PanelSession",
    "ProcessingContext",
    "ProjectAuditFacts",
    "RoomEscape",
    "RoomFacts",
    "SemanticFix",
    "Shape",
    "Stopped",
    "Storey",
    "UnitFacts",
    "UnitLevel",
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
    rows: list[dict[str, Any]] | None = None
    """None: a line only (a message). A list, even an empty one, is a report: what it found, none too. A search that
    found nothing went as a message, and a panel's table kept the rows of the search before under "0 yapı bulundu"
    (real window, 15 November 2026)."""
    cancelled: bool = False

    def to_json(self) -> dict[str, Any]:
        out: dict[str, Any] = {"kind": "report" if self.rows is not None else "message"}
        if self.title is not None:
            out["title"] = self.title
        if self.message is not None:
            out["message"] = self.message
        if self.rows is not None:
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
    """What a context needs of its process: its stop flag, a way to tell KentOS how far it is, and a way to ask it
    (a dialog's answer comes when the user gives it)."""

    def __init__(
        self,
        send: Callable[[dict[str, Any]], None],
        run: int,
        stop: threading.Event,
        call: Callable[[str, dict[str, Any]], Any] | None = None,
    ) -> None:
        self._send = send
        self.run = run
        self.stop = stop
        self._call = call
        self._last_progress = 0.0

    def ask(self, method: str, params: dict[str, Any]) -> Any:
        if self._call is None:
            raise NotAvailable("Bu çalıştırma kullanıcıya soramaz: KentOS masaüstünde bir komut sorabilir.")
        return self._call(method, params)

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


_KEEP: Any = object()


def _json_value(value: Any) -> Any:
    """A value as a panel takes it: a library's number as a plain one (``item()``: NumPy …), a date as ISO text
    (``2026-11-15``, ``14:30``, ``2026-11-15T14:30``), a NaN or an infinity as nothing."""
    import datetime

    if value is None or isinstance(value, (bool, str, int)):
        return value
    if isinstance(value, float):
        return value if math.isfinite(value) else None
    if isinstance(value, datetime.datetime):
        return value.strftime("%Y-%m-%dT%H:%M")
    if isinstance(value, datetime.date):
        return value.isoformat()
    if isinstance(value, datetime.time):
        return value.strftime("%H:%M")
    if isinstance(value, dict):
        return {str(k): _json_value(v) for k, v in value.items()}
    if isinstance(value, (list, tuple, set)):
        return [_json_value(v) for v in value]
    item = getattr(value, "item", None)
    if callable(item):
        return _json_value(item())
    raise TypeError(f"{type(value).__name__} panele verilemez: metin, sayı, tarih, liste ya da sözlük olmalı.")


def _png_bytes(image: Any) -> bytes:
    """A picture given to a panel: bytes as they are, else a file's (a path or a path-like)."""
    if isinstance(image, (bytes, bytearray, memoryview)):
        return bytes(image)
    with open(os.fspath(image), "rb") as f:
        return f.read()


class PanelSession:
    """One of the extension's panels, changed as the run goes (docs/adr/8057 §32): ``ctx.panel`` (the panel whose
    button or changed input started the run; None when it did not start from a panel) or what
    :meth:`CommandContext.open_panel` gives.

    Each change is checked as the panel's definition is: an id the panel does not have, a property its component does
    not take, a value of the wrong kind or out of its bounds, a choice not among the options raise
    :class:`kentos.HostError` (``unknown_component``, ``invalid_input``) and apply nothing. A change returns False when
    it was not applied: the user closed the panel, or changed that input while the command ran (the user's value
    stays; the other properties of the call are applied). The focus, the text's cursor and the panel's scroll stay
    where the user left them."""

    def __init__(self, link: _Link, local: str) -> None:
        self._link = link
        #: The panel's id in the manifest (``entrypoints.panels[].id``).
        self.id: str = local

    def _ask(self, method: str, params: dict[str, Any]) -> bool:
        answer = self._link.ask(method, {**params, "panel": self.id})
        return bool(isinstance(answer, dict) and answer.get("applied"))

    def set(
        self,
        id: str,
        *,
        value: Any = _KEEP,
        options: Iterable[Any] | None = None,
        enabled: bool | None = None,
        visible: bool | None = None,
        text: str | None = None,
        image: Any = _KEEP,
    ) -> bool:
        """Changes component ``id``: ``value`` (a field's text or number, a date as ``datetime.date`` or ISO text, a
        choice's value, a multiple choice's values, a check's True or False, a layer picker's layer id, a feature
        picker's persistent ids; None empties it), ``options`` (a choice's or a multiple choice's: values, or dicts
        with ``value`` and ``label``; a chosen value no longer among them is let go), ``enabled`` (buttons and typed
        fields), ``visible`` (any component with an id; a hidden required field is not asked of the user),
        ``text`` (a ``text`` component's words) and ``image`` (an ``image`` component's picture: a PNG's bytes, e.g.
        ``fig.savefig(buf, format="png")``, or the path of a PNG file; None takes it away; at most 4096 pixels a side)."""
        params: dict[str, Any] = {"id": str(id)}
        if value is not _KEEP:
            params["value"] = _json_value(value)
        if options is not None:
            params["options"] = _json_value(list(options))
        if enabled is not None:
            params["enabled"] = bool(enabled)
        if visible is not None:
            params["visible"] = bool(visible)
        if text is not None:
            params["text"] = str(text)
        if image is not _KEEP:
            params["image"] = None if image is None else base64.b64encode(_png_bytes(image)).decode("ascii")
        return self._ask("panel.set", params)

    def rows(self, id: str, rows: Iterable[dict[str, Any]], columns: Sequence[str] | None = None) -> bool:
        """Puts ``rows`` (each a dict of column → value) in table, tree, cards, figures or chart ``id`` now, as a
        command's report would at its end; ``columns`` in their order, else in the order the rows first give them."""
        given = [dict(r) for r in rows]
        # The columns in the order the rows first give them: a JSON object's keys have no order KentOS keeps ("no"
        # came after "alan (m²)", real window, 15 November 2026).
        order = list(columns) if columns is not None else list(dict.fromkeys(k for r in given for k in r))
        params: dict[str, Any] = {
            "id": str(id),
            "rows": [_json_value(r) for r in given],
            "columns": [str(c) for c in order],
        }
        return self._ask("panel.rows", params)

    def say(self, text: str, kind: str = "info") -> bool:
        """A line under the panel's form (``info``, ``success``, ``warning``, ``error``); kept when the run ends
        without words of its own."""
        return self._ask("panel.say", {"text": str(text), "kind": str(kind)})

    def focus(self, id: str) -> bool:
        """Puts the cursor in typed field ``id``; only in the panel shown (False otherwise: the code does not bring its
        tab in front of what the user looks at)."""
        return self._ask("panel.focus", {"id": str(id)})

    def close(self) -> bool:
        """Closes the panel (its values with it)."""
        return self._ask("panel.close", {})


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
        panel: str | None = None,
    ) -> None:
        self._link = link
        #: The drawing open in KentOS (kentos.cad.Document); none when no drawing is open.
        self.doc = doc
        self.settings: dict[str, Any] = dict(settings or {})
        self.params: dict[str, Any] = dict(params or {})
        #: The panel whose button (or changed input) started the run, to change as it goes (:class:`PanelSession`);
        #: None when it did not start from a panel.
        self.panel: PanelSession | None = PanelSession(link, panel) if panel else None
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
        """Rows to show the user, each a dict of column → value; none is an answer too (a panel's table empties)."""
        return CommandResult(title=title, message=message, rows=[dict(r) for r in rows])

    def message(self, text: str, title: str | None = None) -> CommandResult:
        """A line to show the user."""
        return CommandResult(title=title, message=text)

    def dialog(
        self, title: str, body: list[dict[str, Any]], ok: str = "Tamam", cancel: str = "İptal"
    ) -> dict[str, Any] | None:
        """Asks the user in a KentOS window over the drawing and waits for the answer (docs/adr/8057 §29): ``body`` is
        a panel's fields (``kentos.ui.panel`` components: ``text``, ``textField``, ``numberField``, ``dateField``,
        ``timeField``, ``dateTimeField``, ``select``, ``multiSelect``, ``checkbox``, ``layerPicker``, ``fieldPicker``),
        no buttons or commands. Tamam gives the values by id, as a panel's command gets them; İptal, Esc and Durdur give
        None (after Durdur the run stops: :class:`Stopped`)."""
        self.check()
        if not isinstance(body, list) or not all(isinstance(c, dict) for c in body):
            raise TypeError("dialog: body bir alan listesi olmalı (her biri bir dict, ör. {'type': 'textField', …}).")
        answer = self._link.ask(
            "dialog", {"title": str(title), "body": list(body), "ok": str(ok), "cancel": str(cancel)}
        )
        self.check()
        return answer if isinstance(answer, dict) else None

    def confirm(self, text: str, title: str = "Onay", ok: str = "Evet", cancel: str = "Hayır") -> bool:
        """Asks the user yes or no in a KentOS window (:meth:`dialog` with ``text``): True for ``ok``."""
        return self.dialog(title, [{"type": "text", "text": str(text)}], ok=ok, cancel=cancel) is not None

    def open_panel(self, id: str) -> PanelSession:
        """Opens the extension's panel ``id`` (``entrypoints.panels[].id``; brought in front when it is open) and gives
        it to change as the run goes (:class:`PanelSession`): a command run from the ribbon shows what it did in its
        panel. Until the run ends an input the user changes there stays the user's. A panel the manifest does not
        declare raises :class:`kentos.HostError` (``unknown_panel``)."""
        self._link.ask("panel.open", {"panel": str(id)})
        return PanelSession(self._link, str(id))

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
        rows = _install_schema(self.doc, schema, path, self._package_json)
        added = sum(1 for r in rows if r["Sonuç"] != "vardı")
        if not added:
            return CommandResult(title="Katman şeması", message="Şema zaten kurulu: değişiklik yok.", rows=rows)
        counts: dict[str, int] = {}
        for r in rows:
            if r["Sonuç"] != "vardı":
                counts[r["Tür"]] = counts.get(r["Tür"], 0) + 1
        # Written out, not lowered: Python's lower() makes “İlişki” “i̇lişki” (a dotted i and a combining dot).
        words = {"Grup": "grup", "Değer listesi": "değer listesi", "Katman": "katman", "Form": "form",
                 "Alt türler": "katmana alt tür", "Değer bileşimleri": "katmana değer bileşimi", "İlişki": "ilişki"}
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


def _install_schema(doc: Any, schema: Any, path: str, read: Callable[[str], Any]) -> list[dict[str, Any]]:
    """What ``apply_declarative_schema`` writes: the group, the value lists, the layers with their fields, their forms
    (a package's ``.kform`` read by ``read``, bound to the layer under its name as the form's id), the layers' subtypes
    and value groups, the relations, in that order (a field links a list, a form a layer's fields, a subtype the
    fields, a relation its layers); what is there is kept. A row each."""
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
    forms = [layer for layer in schema["layers"] if layer.get("form")]
    if forms:
        have = _schema_ids(doc, "forms")
        for layer in forms:
            lid = str(layer["name"])
            package = read(str(layer["form"]))
            made = lid not in have
            if made:
                cad.forms.import_(doc, layer=lid, package=cad.FormPackage.from_json(package), id=lid, add_fields=True)
            rows.append({"Tür": "Form", "Kimlik": lid, "Ad": str(package.get("name") or lid),
                         "Sonuç": "eklendi" if made else "vardı"})
    # A layer's subtypes and value groups (KentOS's, docs/adr/0422, 0423), after its fields and forms: a subtype
    # narrows the fields, a group's row may be a subtype's. A layer that has them keeps its own.
    for layer in schema["layers"]:
        lid = str(layer["name"])
        title = str(layer.get("title") or lid)
        node = nodes.get(lid)
        if layer.get("subtypes"):
            made = node is None or not isinstance(getattr(node, "subtypes", None), cad.LayerSubtypes)
            if made:
                cad.layers.subtypes(doc, layer=lid, subtypes=cad.LayerSubtypes.from_json(layer["subtypes"]))
            n = len(layer["subtypes"].get("types") or [])
            rows.append({"Tür": "Alt türler", "Kimlik": lid, "Ad": f"{title}: {n} alt tür",
                         "Sonuç": "eklendi" if made else "vardı"})
        if layer.get("contingents"):
            have = getattr(node, "contingents", None) if node is not None else None
            made = not (isinstance(have, list) and have)
            if made:
                cad.layers.contingents(doc, layer=lid,
                                       groups=[cad.FieldGroup.from_json(g) for g in layer["contingents"]])
            rows.append({"Tür": "Değer bileşimleri", "Kimlik": lid, "Ad": f"{title}: {len(layer['contingents'])} grup",
                         "Sonuç": "eklendi" if made else "vardı"})
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
    :class:`Feature` (persistent id, kind, layer, attributes, shape as GeoJSON), taken when the audit began, and what
    KentOS measured in it (:attr:`project`). A rule writes nothing: it answers :class:`AuditFinding` s, each with an
    optional fix the user chooses to apply."""

    __slots__ = ("_layers", "cut", "_project")

    def __init__(self, raw: dict[str, Any]) -> None:
        self._layers: list[tuple[str, str, list[Feature]]] = [
            (str(layer.get("id", "")), str(layer.get("name", "")), [Feature(f) for f in layer.get("features") or []])
            for layer in raw.get("layers") or []
            if isinstance(layer, dict)
        ]
        #: KentOS gave only the first objects of a very large drawing (its limit): a rule may say so.
        self.cut: bool = bool(raw.get("cut"))
        project = raw.get("project")
        self._project: ProjectAuditFacts | None = ProjectAuditFacts(project) if isinstance(project, dict) else None

    @property
    def project(self) -> ProjectAuditFacts:
        """What KentOS measured in the drawing, once and in Rust (:class:`ProjectAuditFacts`): read it rather than
        working the geometry out again. :class:`NotAvailable` when they are missing (a KentOS that does not measure
        them, or could not on this drawing)."""
        if self._project is None:
            raise NotAvailable("Çizimin ölçülmüş olguları (facts.project) bu denetimde yok: bu KentOS onları vermiyor "
                               "ya da bu çizimde ölçemedi.")
        return self._project

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


class ProjectAuditFacts:
    """What KentOS measured in the drawing for an audit (``kentos.auditFacts`` 1, the SDK's ``factsSchema``;
    docs/adr/8057 §36): each layer's objects by geometry, its invalid geometries, the overlaps of its areas and the gaps
    their union closes in, worked out once in Rust at the project's topology tolerance, the same on the desktop and in
    a cloud job. Areas in m² to 4 decimals, lengths and places in metres to 3; a place is ``(x, y)`` in the project's
    plane, as the features' GeoJSON gives it."""

    __slots__ = ("tolerance", "cut", "_layers", "architecture")

    def __init__(self, raw: dict[str, Any]) -> None:
        #: The project's topology tolerance (m): areas nearer than it touch, a piece no wider is no overlap or gap.
        self.tolerance: float = float(raw.get("tolerance") or 0.0)
        #: KentOS measured only the first objects of a very large drawing (by persistent id): a rule may say so.
        self.cut: bool = bool(raw.get("cut"))
        self._layers = [LayerFacts(layer) for layer in raw.get("layers") or [] if isinstance(layer, dict)]
        #: The drawing's storeys, rooms, program groups and escape routes (docs/adr/8057 §39); empty without rooms.
        self.architecture: ArchitectureFacts = ArchitectureFacts(raw.get("architecture") or {})

    def layers(self) -> list[LayerFacts]:
        """The layers holding objects, in the layer tree's order."""
        return list(self._layers)

    def layer(self, layer: str) -> LayerFacts:
        """A layer's facts, by its id or its name (``parsel``, ``Parsel``); an empty one when no layer holding objects
        has it."""
        for found in self._layers:
            if layer in (found.id, found.name):
                return found
        return LayerFacts({"id": layer, "name": layer})


class LayerFacts:
    """One layer's measured facts (:class:`ProjectAuditFacts`); a list holds at most 10 000 items, the first in its
    order (invalid by object, overlaps by their pair, gaps north to south), and :attr:`totals` counts all of them."""

    __slots__ = ("id", "name", "points", "lines", "areas", "other", "area", "length", "invalid", "overlaps", "gaps",
                 "totals")

    def __init__(self, raw: dict[str, Any]) -> None:
        counts = raw.get("objects") or {}
        totals = raw.get("totals") or {}
        self.id: str = str(raw.get("id", ""))
        self.name: str = str(raw.get("name", ""))
        #: Its objects by geometry: points, lines (a closed polyline too), areas and the rest (texts, blocks …).
        self.points: int = int(counts.get("points") or 0)
        self.lines: int = int(counts.get("lines") or 0)
        self.areas: int = int(counts.get("areas") or 0)
        self.other: int = int(counts.get("other") or 0)
        #: Its areas' area summed, m²: holes out, arcs exact (not the tessellated GeoJSON's).
        self.area: float = float(raw.get("area") or 0.0)
        #: Its lines' length summed, m, arcs exact.
        self.length: float = float(raw.get("length") or 0.0)
        self.invalid: list[InvalidGeometry] = [
            InvalidGeometry(str(x.get("uid", "")), str(x.get("problem", "")), _place(x.get("at")))
            for x in raw.get("invalid") or []
        ]
        self.overlaps: list[Overlap] = [
            Overlap(str(x.get("a", "")), str(x.get("b", "")), float(x.get("area") or 0.0), _place(x.get("at")))
            for x in raw.get("overlaps") or []
        ]
        self.gaps: list[Gap] = [
            Gap(float(x.get("area") or 0.0), _place(x.get("at")), tuple(str(u) for u in x.get("around") or []))
            for x in raw.get("gaps") or []
        ]
        #: How many it has in all: ``{"invalid": …, "overlaps": …, "gaps": …}``.
        self.totals: dict[str, int] = {k: int(totals.get(k) or 0) for k in ("invalid", "overlaps", "gaps")}

    def __repr__(self) -> str:
        return (f"LayerFacts({self.name!r}: {self.areas} alan, {self.totals['invalid']} geçersiz, "
                f"{self.totals['overlaps']} çakışma, {self.totals['gaps']} boşluk)")


def _place(at: Any) -> tuple[float, float]:
    x, y = (list(at) + [0.0, 0.0])[:2] if isinstance(at, (list, tuple)) else (0.0, 0.0)
    return (float(x), float(y))


@dataclass(frozen=True)
class InvalidGeometry:
    """An object whose shape is not valid: its id, the problem (``repeated``, ``zeroArea``, ``ringCrossing``,
    ``pathCrossing``, ``holeOutside``, ``holesOverlap``) and where it first shows."""

    uid: str
    problem: str
    at: tuple[float, float]


@dataclass(frozen=True)
class Overlap:
    """Two areas of one layer covering each other: their ids (``a`` the smaller), the area both cover (m²) and the
    centroid of its largest piece."""

    a: str
    b: str
    area: float
    at: tuple[float, float]


@dataclass(frozen=True)
class Gap:
    """What a layer's areas close in and none covers (a gap between parcels, an area's hole): its area (m²), its
    centroid and the areas around it, the one sharing the longest boundary first."""

    area: float
    at: tuple[float, float]
    around: tuple[str, ...]


class ArchitectureFacts:
    """The drawing's architecture as KentOS Mimari measured it (docs/adr/8057 §39): its storeys, every room with its
    net area and its outline to the walls' axes, the program's groups with PAİY 4/ğ's areas and the escape routes
    (BYKHY); empty when the drawing has no room. PAİY is Planlı Alanlar İmar Yönetmeliği, BYKHY Binaların Yangından
    Korunması Hakkında Yönetmelik; areas in m² to 4 decimals, lengths in metres to 3."""

    __slots__ = ("levels", "rooms", "units", "doors", "_exits")

    def __init__(self, raw: dict[str, Any]) -> None:
        #: The project's storeys, lowest first.
        self.levels: list[Storey] = [
            Storey(str(s.get("id", "")), str(s.get("name", "")), float(s.get("elevation") or 0.0),
                   _maybe_float(s.get("height")))
            for s in raw.get("levels") or []
        ]
        #: Every room, a void and an outdoor one too, by persistent id.
        self.rooms: list[RoomFacts] = [_room(r) for r in raw.get("rooms") or []]
        #: The program's groups that have rooms, in the program's order.
        self.units: list[UnitFacts] = [_unit(u) for u in raw.get("units") or []]
        #: Each dwelling unit's door and its walk to the nearest storey exit (BYKHY 48/4).
        self.doors: list[DoorEscape] = [
            DoorEscape(str(d.get("uid", "")), str(d.get("unit", "")), _maybe_str(d.get("level")),
                       _place(d.get("at")), _walk(d.get("exit")))
            for d in raw.get("doors") or []
        ]
        self._exits = {_maybe_str(e.get("level")): int(e.get("count") or 0) for e in raw.get("exits") or []}

    def exits(self, level: str | None) -> int:
        """How many storey exits a storey has (BYKHY 4/u: an escape stair's room, an open stair, a door out on the
        ground storey), by its id; 0 for a storey without rooms."""
        return self._exits.get(level, 0)

    def room(self, uid: str) -> RoomFacts | None:
        """A room by its persistent id; none when the drawing has no such room."""
        return next((r for r in self.rooms if r.uid == uid), None)

    def unit(self, unit: str) -> UnitFacts | None:
        """A program group by its id or its number; none when no group with rooms has it."""
        return next((u for u in self.units if unit in (u.id, u.number)), None)


def _maybe_str(v: Any) -> str | None:
    return None if v is None else str(v)


def _maybe_float(v: Any) -> float | None:
    return None if v is None else float(v)


def _walk(raw: Any) -> EscapeWalk | None:
    if not isinstance(raw, dict):
        return None
    exits = raw.get("exits")
    return EscapeWalk(float(raw.get("length") or 0.0), tuple(_place(p) for p in raw.get("path") or []),
                      _maybe_float(raw.get("oneWay")), None if exits is None else int(exits))


def _room(raw: dict[str, Any]) -> RoomFacts:
    e = raw.get("escape")
    escape = None if not isinstance(e, dict) else RoomEscape(
        _walk(e.get("exit")), bool(e.get("storeyExits")), _walk(e.get("entrance")), bool(e.get("hasEntrance")),
        int(e.get("passages") or 0))
    text = {k: _maybe_str(raw.get(k)) for k in ("number", "name", "usage", "level", "unit")}
    return RoomFacts(str(raw.get("uid", "")), *text.values(), bool(raw.get("void")), bool(raw.get("outdoor")),
                     bool(raw.get("broken")), bool(raw.get("stair")), float(raw.get("net") or 0.0),
                     float(raw.get("axis") or 0.0), escape)


def _unit(raw: dict[str, Any]) -> UnitFacts:
    area = {k: float(raw.get(k) or 0.0) for k in ("gross", "net", "annexGross", "annexNet", "total")}
    return UnitFacts(
        str(raw.get("id", "")), str(raw.get("number", "")), _maybe_str(raw.get("name")), str(raw.get("kind", "")),
        _maybe_str(raw.get("nature")), _maybe_str(raw.get("annexOf")), bool(raw.get("dwelling")), *area.values(),
        _maybe_float(raw.get("share")), _maybe_float(raw.get("general")),
        tuple(UnitLevel(_maybe_str(x.get("level")), float(x.get("gross") or 0.0), float(x.get("net") or 0.0))
              for x in raw.get("levels") or []))


@dataclass(frozen=True)
class Storey:
    """A storey of the project: its id, its name, its floor's elevation (m) and its height (m; None when not given)."""

    id: str
    name: str
    elevation: float
    height: float | None


@dataclass(frozen=True)
class EscapeWalk:
    """A walk along the points it turns at, its length in metres; a walk to an exit also says the part it shares with
    the walk to the nearest other exit (``one_way``, tek yön: a dead end's) and how many exits its start reaches."""

    length: float
    path: tuple[tuple[float, float], ...]
    one_way: float | None = None
    exits: int | None = None


@dataclass(frozen=True)
class RoomEscape:
    """A room's walks from its farthest point, 40 cm in front of its walls (BYKHY 32/6): to the nearest storey exit
    (32/4; None when it reaches none) and, a dwelling unit's room, to its unit's door (48/2)."""

    exit: EscapeWalk | None
    storey_exits: bool
    entrance: EscapeWalk | None
    has_entrance: bool
    passages: int


@dataclass(frozen=True)
class RoomFacts:
    """A room: its persistent id, number, name, usage, storey and group; whether it is a void, outdoor, broken (its
    boundary not found) or an escape stair's room; its net area and its outline to the walls' axes (m²); its walks
    (None for a void, an outdoor, a broken or a stair's room)."""

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


@dataclass(frozen=True)
class UnitLevel:
    """A group's gross and net area on one storey (m²)."""

    level: str | None
    gross: float
    net: float


@dataclass(frozen=True)
class UnitFacts:
    """A program group with PAİY 4/ğ's areas (m²): ``kind`` (``unit``, ``common``, ``outdoor``, ``annex``), ``gross``
    (BB brüt), ``net``, ``total`` (with its annexes'), ``share`` of the common areas and ``general`` (genel brüt; None
    without a share)."""

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
    """A dwelling unit's door: its persistent id, unit, storey and middle, and its walk from its outer side to the
    nearest storey exit (BYKHY 48/4; None when it opens into an escape stair's room)."""

    uid: str
    unit: str
    level: str | None
    at: tuple[float, float]
    exit: EscapeWalk | None


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
