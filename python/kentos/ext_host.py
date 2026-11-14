"""An extension's own process (plan/eklenti.md ADIM 19; docs/adr/8057 §6): ``python -m kentos.ext_host``, which KentOS
starts in the extension's own environment, one process for an extension version, and stops.

JSON lines, as the console's (docs/adr/0132), with a protocol version of their own.

KentOS → process::

    {"type": "hello", "protocol": 1, "extension": "tr.ornek.imar-denetim", "version": "1.0.0",
     "digest": "sha256:…", "source": "/…/packages/tr.ornek.imar-denetim/1.0.0", "paths": ["src"]}
    {"type": "invoke", "id": 1, "kind": "command", "local": "check-setbacks",
     "handler": "imar.logic:check_setbacks", "params": {}, "settings": {}, "document": true}
    {"type": "invoke", "id": 2, "kind": "processing", "local": "taks", "handler": "imar.logic:taks",
     "params": {"limit": 0.4}, "inputs": {"parcels": {"description": "…", "features": [{"uid": …, "layer": …,
     "entity": {…}}]}}, "settings": {}, "document": false}
    {"type": "cancel", "id": 1}
    {"type": "reply", "id": 7, "ok": true, "result": …}
    {"type": "reply", "id": 7, "ok": false, "code": "…", "message": "…"}
    {"type": "quit"}

Process → KentOS::

    {"type": "ready", "protocol": 1, "python": "3.12.13", "kentos": "0.1.0"}
    {"type": "refused", "message": "…"}                  (a hello it cannot answer)
    {"type": "out", "id": 1, "stream": "out" | "err", "text": "…"}
    {"type": "progress", "id": 1, "done": 3, "total": 10, "text": "…"}
    {"type": "log", "id": 1, "level": "info" | "warning" | "error", "text": "…"}
    {"type": "call", "id": 7, "method": "entities", "params": {…}}
    {"type": "done", "id": 1, "ok": true, "result": {"kind": "report", "title": …, "rows": […]}}
    {"type": "done", "id": 2, "ok": true, "result": {"kind": "outputs", "outputs": {"parcels": […]}, "summary": …}}
    {"type": "invoke", "id": 3, "kind": "expression", "local": "ada-parsel", "handler": "imar.logic:ada_parsel",
     "rows": [["101", "1"], ["102", "4"]]}
    {"type": "done", "id": 3, "ok": true, "result": {"kind": "values", "values": ["101/1", "102/4"]}}
    {"type": "done", "id": 1, "ok": false, "stopped": true}
    {"type": "done", "id": 1, "ok": false, "refused": true, "message": "…"}
    {"type": "done", "id": 1, "ok": false, "unavailable": true, "message": "…"}   (ext.NotAvailable)
    {"type": "done", "id": 1, "ok": false, "exception": "ValueError", "message": "…", "error": "Traceback …"}

- **Code is imported only when a contribution is invoked** (PY-RUN-05), from the package's own folders (``paths``,
  relative to ``source``), which come after the environment's own: a package's module cannot shadow an installed one,
  and a package carrying a ``kentos`` module is refused (PY-UV-31). A function not marked ``@ext.<kind>("<local>")``
  as the manifest says is refused (PY-RUN-08).
- **Durdur is heard at once:** a thread reads KentOS's lines while the extension computes; a cancel sets the run's stop
  flag (``ctx.check()``, ``ctx.is_stopped()``), a reply goes to the call waiting for it. Code that does not look is
  stopped by KentOS, which ends the process.
- **What it prints** comes as ``out`` messages, at most a megabyte a run, so the channel stays clean (PY-RUN-11).
- **The drawing** is KentOS's: ``ctx.doc`` asks KentOS for everything, which answers within the extension's grants.
- **An İşlemler tool** gets its inputs as data (``inputs``, features as they were when the run began, each with its
  GeoJSON shape) and its other parameters (``params``) as keyword arguments; it has no drawing. Its outputs come back
  plain whatever library made them (``plain``): GeoJSON for shapes, ``{columns, rows}`` for rows, plain numbers and
  lists; KentOS checks them against the tool's descriptor.
"""

from __future__ import annotations

import importlib
import io
import itertools
import json
import math
import numbers
import os
import platform
import queue
import sys
import threading
import traceback
from typing import IO, Any

import kentos

from . import _native, ext
from . import cad
from .host import DesktopSession

PROTOCOL = 1

#: What a run may print before the rest is dropped.
MAX_OUTPUT = 1024 * 1024

#: How much of a traceback is sent.
MAX_TRACEBACK = 20_000


class Refused(Exception):
    """A contribution this process will not run: not found, or not marked as the manifest says."""


class Channel:
    """The JSON lines to and from KentOS, read by a thread of their own."""

    def __init__(self, incoming: IO[bytes], outgoing: IO[bytes]) -> None:
        self._out = outgoing
        self._write = threading.Lock()
        self._calls = itertools.count(1)
        self._replies: dict[int, dict[str, Any]] = {}
        self._arrived = threading.Condition()
        self._closed = False
        #: Messages for the main loop: hello, invoke, quit; ``None`` once KentOS went away.
        self.work: queue.Queue[dict[str, Any] | None] = queue.Queue()
        #: The running runs' stop flags, by run id.
        self.stops: dict[int, threading.Event] = {}
        threading.Thread(target=self._read, args=(incoming,), name="kentos-ext-oku", daemon=True).start()

    def send(self, message: dict[str, Any]) -> None:
        line = json.dumps(message, ensure_ascii=False, separators=(",", ":")) + "\n"
        with self._write:
            self._out.write(line.encode("utf-8"))
            self._out.flush()

    def _read(self, incoming: IO[bytes]) -> None:
        for line in iter(incoming.readline, b""):
            try:
                message = json.loads(line)
            except ValueError:
                continue
            if not isinstance(message, dict):
                continue
            kind = message.get("type")
            if kind == "reply":
                with self._arrived:
                    self._replies[int(message.get("id", 0))] = message
                    self._arrived.notify_all()
            elif kind == "cancel":
                stop = self.stops.get(message.get("id"))  # type: ignore[arg-type]
                if stop is not None:
                    stop.set()
            else:
                self.work.put(message)
        with self._arrived:
            self._closed = True
            self._arrived.notify_all()
        for stop in list(self.stops.values()):
            stop.set()
        self.work.put(None)

    def call(self, method: str, params: dict[str, Any]) -> Any:
        """Asks KentOS and waits for its answer."""
        call = next(self._calls)
        self.send({"type": "call", "id": call, "method": method, "params": params})
        with self._arrived:
            while call not in self._replies and not self._closed:
                self._arrived.wait()
            reply = self._replies.pop(call, None)
        if reply is None:
            # KentOS went away: nothing is left to answer to.
            os._exit(0)
        if reply.get("ok"):
            return reply.get("result")
        raise _native.HostError(reply.get("code", "host"), reply.get("message", ""))


class Stream(io.TextIOBase):
    """``sys.stdout`` or ``sys.stderr`` during a run: whole lines go to KentOS, a megabyte a run at most."""

    encoding = "utf-8"

    def __init__(self, channel: Channel, name: str) -> None:
        super().__init__()
        self._channel = channel
        self._name = name
        self._pending = ""
        self.run: int | None = None
        self.sent = 0

    def writable(self) -> bool:
        return True

    def isatty(self) -> bool:
        return False

    def write(self, text: str) -> int:
        self._pending += text
        cut = self._pending.rfind("\n")
        if cut >= 0:
            self._send(self._pending[: cut + 1])
            self._pending = self._pending[cut + 1 :]
        return len(text)

    def flush(self) -> None:
        if self._pending:
            self._send(self._pending)
            self._pending = ""

    def _send(self, text: str) -> None:
        if self.sent >= MAX_OUTPUT:
            return
        room = MAX_OUTPUT - self.sent
        if len(text) > room:
            text = text[:room] + "\n… (çıktının geri kalanı kesildi)\n"
        self.sent += len(text)
        message: dict[str, Any] = {"type": "out", "stream": self._name, "text": text}
        if self.run is not None:
            message["id"] = self.run
        self._channel.send(message)


def resolve(handler: str, kind: str, local: str) -> Any:
    """The function ``module:function`` names, imported now, if it is marked as the manifest says."""
    module_name, _, path = handler.partition(":")
    if not module_name or not path:
        raise Refused(f"“{handler}” bir işlev adı değil: “modül:işlev” biçiminde olmalı.")
    module = importlib.import_module(module_name)
    target: Any = module
    for part in path.split("."):
        target = getattr(target, part, None)
        if target is None:
            raise Refused(f"“{handler}”: {module_name} içinde “{path}” yok.")
    if not callable(target):
        raise Refused(f"“{handler}” çağrılabilir bir işlev değil.")
    mark = ext.marked(target)
    if mark != (kind, local):
        said = "işaretsiz" if mark is None else f"@ext.{mark[0]}(\"{mark[1]}\") ile işaretli"
        raise Refused(
            f"“{handler}” {said}; manifest onu @ext.{kind}(\"{local}\") diye gösteriyor. KentOS işaretiyle "
            "uyuşmayan işlevi çalıştırmaz."
        )
    return target


def result_of(value: Any) -> dict[str, Any]:
    """What a contribution returned, as KentOS reads it."""
    if isinstance(value, ext.CommandResult):
        return value.to_json()
    if value is None:
        return {"kind": "none"}
    if isinstance(value, str):
        return {"kind": "message", "message": value}
    try:
        json.dumps(value)
    except (TypeError, ValueError):
        return {"kind": "message", "message": repr(value)[:2000]}
    return {"kind": "value", "value": value}


def table(rows: list[dict[str, Any]], where: str, columns: list[Any] | None = None) -> dict[str, Any]:
    """Rows as KentOS's table, ``{columns, rows}``: the columns in the order the rows first give them (KentOS's JSON
    objects keep no order), each cell plain."""
    if columns is None:
        seen: dict[str, None] = {}
        for row in rows:
            seen.update(dict.fromkeys(str(k) for k in row))
        names = list(seen)
    else:
        names = [str(c) for c in columns]
    return {
        "columns": names,
        "rows": [
            [plain(row.get(c), f"{where}[{i}].{c}") for c in names]
            for i, row in enumerate({str(k): v for k, v in r.items()} for r in rows)
        ],
    }


def _is_rows(value: Any) -> bool:
    """A list of plain dicts that are not objects to write (a GeoJSON feature, an ext.NewFeature's form): rows."""
    return (
        isinstance(value, list)
        and bool(value)
        and all(isinstance(r, dict) for r in value)
        and not any(r.get("type") == "Feature" or "geometry" in r or "source" in r for r in value)
    )


def plain(value: Any, where: str) -> Any:
    """A tool's output as JSON carries it, whatever library made it: GeoJSON for a shape (``__geo_interface__``:
    shapely, GeoPandas …), a table for rows (dicts, a DataFrame), a plain number or list for a library's own
    (``item()``, ``tolist()``: NumPy, pandas …), a date as ISO text, a NaN or an infinity as nothing; anything else is
    the extension's mistake, said with where it is."""
    if value is None or isinstance(value, (bool, str)):
        return value
    if isinstance(value, int):
        return value
    if isinstance(value, float):
        return value if math.isfinite(value) else None
    if isinstance(value, ext.NewFeature):
        return plain(value.to_json(), where)
    if isinstance(value, ext.Feature):
        return {"source": value.uid, "attrs": {}}
    if isinstance(value, ext.FeatureInput):
        return [{"source": f.uid, "attrs": {}} for f in value]
    if isinstance(value, dict):
        return {str(k): plain(v, f"{where}.{k}") for k, v in value.items()}
    if isinstance(value, (list, tuple)):
        if _is_rows(list(value)):
            return table(list(value), where)
        return [plain(v, f"{where}[{i}]") for i, v in enumerate(value)]
    # A shape, a GeoDataFrame, a feature of any library: GeoJSON (RFC 7946), the libraries' shared interface.
    geo = getattr(value, "__geo_interface__", None)
    if isinstance(geo, dict):
        return plain(geo, where)
    # A DataFrame (pandas and the like): its columns in their order, its rows.
    columns = getattr(value, "columns", None)
    to_dict = getattr(value, "to_dict", None)
    if columns is not None and callable(to_dict):
        try:
            rows = to_dict(orient="records")
        except TypeError:
            rows = None
        if isinstance(rows, list):
            return table(rows, where, list(columns))
    # A library's arrays and numbers (NumPy, pandas, …) without importing them here.
    tolist = getattr(value, "tolist", None)
    if callable(tolist):
        return plain(tolist(), where)
    item = getattr(value, "item", None)
    if callable(item):
        return plain(item(), where)
    isoformat = getattr(value, "isoformat", None)
    if callable(isoformat):
        return str(isoformat())
    # A number of another kind (a Decimal, a Fraction): as a float; a complex one has none.
    if isinstance(value, numbers.Number) and not isinstance(value, complex):
        try:
            return plain(float(value), where)  # type: ignore[arg-type]
        except (TypeError, ValueError):
            pass
    raise TypeError(
        f"{where}: {type(value).__name__} türünde bir değer çıktı olarak verilemez (sayı, metin, liste, sözlük, "
        "ext.NewFeature, bir GeoJSON şekli ya da tablo olmalı)."
    )


def outputs_of(value: Any, ctx: ext.ProcessingContext) -> dict[str, Any]:
    """What an İşlemler tool returned, as KentOS reads it: its outputs by name, its summary and its selection."""
    out: dict[str, Any] = {"kind": "outputs", "outputs": {}}
    if isinstance(value, ext.CommandResult):
        if value.cancelled:
            return {"kind": "outputs", "cancelled": True}
        if value.message:
            out["summary"] = value.message
    elif isinstance(value, dict):
        out["outputs"] = {str(k): plain(v, str(k)) for k, v in value.items()}
    elif value is not None:
        raise TypeError(
            f"Araç {type(value).__name__} döndürdü: çıktılarını adlarıyla bir sözlükte vermeli ({{\"çıktı\": değer}})."
        )
    if ctx._summary is not None:
        out["summary"] = ctx._summary
    if ctx._select is not None:
        out["select"] = ctx._select
    return out


def run(channel: Channel, streams: tuple[Stream, Stream], message: dict[str, Any]) -> None:
    """One invocation, answered with its ``done``."""
    run_id = int(message.get("id", 0))
    stop = threading.Event()
    channel.stops[run_id] = stop
    for stream in streams:
        stream.run = run_id
        stream.sent = 0
    done: dict[str, Any] = {"type": "done", "id": run_id}
    try:
        kind = str(message.get("kind", ""))
        fn = resolve(str(message.get("handler", "")), kind, str(message.get("local", "")))
        link = ext._Link(channel.send, run_id, stop)
        if kind == "expression":
            # A batch of rows, each a call with its arguments: one row's failure empties that row alone; how many
            # failed and the first's words come back (KentOS writes them to the log once).
            values: list[Any] = []
            failed = 0
            first = ""
            for i, row in enumerate(message.get("rows") or []):
                if stop.is_set():
                    raise ext.Stopped()
                try:
                    values.append(plain(fn(*row), f"[{i}]"))
                except Exception as e:  # noqa: BLE001 - a row's failure is counted, the others go on
                    values.append(None)
                    failed += 1
                    if not first:
                        first = f"{type(e).__name__}: {e}"[:500]
            result: dict[str, Any] = {"kind": "values", "values": values}
            if failed:
                result.update(failed=failed, error=first)
        elif kind == "processing":
            # Its parameters as keyword arguments, its features as data: it has no drawing (PY-TOOL-09).
            params = dict(message.get("params") or {})
            pctx = ext.ProcessingContext(link, message.get("settings"), params)
            arguments = dict(params)
            for name, given in (message.get("inputs") or {}).items():
                arguments[str(name)] = ext.FeatureInput(given)
            if stop.is_set():
                raise ext.Stopped()
            result = outputs_of(fn(pctx, **arguments), pctx)
        elif kind == "mapTool":
            # What the user drew, given to the tool with a command's context: its writes are one undo step.
            doc = cad.Document(DesktopSession(channel)) if message.get("document") else None  # type: ignore[arg-type]
            params = dict(message.get("params") or {})
            shape = ext.Shape(dict(params.pop("shape", None) or {}))
            mctx = ext.MapToolContext(link, doc, message.get("settings"), params)
            if stop.is_set():
                raise ext.Stopped()
            result = result_of(fn(mctx, shape))
        else:
            doc = cad.Document(DesktopSession(channel)) if message.get("document") else None  # type: ignore[arg-type]
            ctx = ext.CommandContext(link, doc, message.get("settings"), message.get("params"))
            if stop.is_set():
                raise ext.Stopped()
            result = result_of(fn(ctx))
        if result.get("cancelled") or stop.is_set():
            done.update(ok=False, stopped=True)
        else:
            done.update(ok=True, result=result)
    except ext.Stopped:
        done.update(ok=False, stopped=True)
    except Refused as e:
        done.update(ok=False, refused=True, message=str(e))
    except ext.NotAvailable as e:
        done.update(ok=False, unavailable=True, message=str(e)[:2000])
    except BaseException as e:  # noqa: BLE001 - an extension's failure is reported, never let through
        if stop.is_set() and isinstance(e, _native.HostError):
            done.update(ok=False, stopped=True)
        else:
            done.update(
                ok=False,
                exception=type(e).__name__,
                message=str(e)[:2000],
                error=traceback.format_exc()[-MAX_TRACEBACK:],
            )
    finally:
        for stream in streams:
            stream.flush()
            stream.run = None
        channel.stops.pop(run_id, None)
    channel.send(done)


def main() -> int:
    channel = Channel(sys.stdin.buffer, sys.stdout.buffer)
    streams = (Stream(channel, "out"), Stream(channel, "err"))
    sys.stdout, sys.stderr = streams  # type: ignore[assignment]
    hello = channel.work.get()
    if hello is None:
        return 0
    if hello.get("type") != "hello" or hello.get("protocol") != PROTOCOL:
        channel.send(
            {
                "type": "refused",
                "message": f"Bu KentOS kitaplığı eklenti protokolünün {PROTOCOL}. sürümünü konuşur.",
            }
        )
        return 1
    source = str(hello.get("source", ""))
    folders = [os.path.normpath(os.path.join(source, str(path))) for path in hello.get("paths") or []]
    # KentOS's library comes from the environment, never from an extension's own folders (PY-UV-31): a package
    # carrying a `kentos` module is refused, and its folders come after the environment's, so it cannot shadow an
    # installed package either.
    for folder in folders:
        if os.path.isdir(os.path.join(folder, "kentos")) or os.path.isfile(os.path.join(folder, "kentos.py")):
            channel.send(
                {
                    "type": "refused",
                    "message": "Eklenti KentOS'un kendi kitaplığının adını (kentos) taşıyan bir modül içeriyor; çalıştırılmaz.",
                }
            )
            return 1
        if folder not in sys.path:
            sys.path.append(folder)
    channel.send(
        {
            "type": "ready",
            "protocol": PROTOCOL,
            "python": platform.python_version(),
            "kentos": getattr(kentos, "__version__", ""),
        }
    )
    while True:
        message = channel.work.get()
        if message is None or message.get("type") == "quit":
            return 0
        if message.get("type") == "invoke":
            run(channel, streams, message)


if __name__ == "__main__":
    code = main()
    for stream in (sys.stdout, sys.stderr):
        try:
            stream.flush()
        except (OSError, ValueError):
            pass
    # Not through the interpreter's shutdown: the reading thread holds standard input, and closing it then is a
    # fatal error ("could not acquire lock … at interpreter shutdown", measured). Everything was sent and flushed.
    os._exit(code)
