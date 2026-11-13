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

A separate process is not a sandbox: an extension runs with the user's rights (PY-RUN-19, 20).
"""

from __future__ import annotations

import threading
import time
from dataclasses import dataclass, field
from typing import Any, Callable, TypeVar

__all__ = [
    "AuditFinding",
    "CommandContext",
    "CommandResult",
    "GisAuditFacts",
    "MapToolContext",
    "NotAvailable",
    "ProcessingContext",
    "SemanticFix",
    "Stopped",
    "audit",
    "command",
    "expr",
    "map_tool",
    "marked",
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
    ) -> None:
        self._link = link
        #: The drawing open in KentOS (kentos.cad.Document); none when no drawing is open.
        self.doc = doc
        self.settings: dict[str, Any] = dict(settings or {})
        self.params: dict[str, Any] = dict(params or {})

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

    def apply_declarative_schema(self, path: str) -> CommandResult:
        """Sets up the layers, value lists and relations the package's ``schema.json`` declares, in one undo step.

        Its format and KentOS's own command for it are fixed with the panels and forms (plan/eklenti.md ADIM 22); until
        then this KentOS raises :class:`NotAvailable`."""
        raise NotAvailable(
            f"Katman şeması kurulumu ({path}) bu KentOS sürümünde yok: eklentinin katmanları, değer listeleri ve "
            "ilişkileri henüz kurulamaz."
        )


class ProcessingContext(CommandContext):
    """An İşlemler tool's context; its inputs and outputs are ADIM 19's next slice."""


class MapToolContext(CommandContext):
    """A map tool's context; ADIM 19's next slice."""


class GisAuditFacts:
    """An audit rule's facts; ADIM 19's next slice."""


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
