"""An extension's own process (plan/eklenti.md ADIM 19; docs/adr/8057 §6): ``python -m kentos.ext_host``, which KentOS
starts in the extension's own environment, one process for an extension version, and stops.

JSON lines, as the console's (docs/adr/0132), with a protocol version of their own.

KentOS → process::

    {"type": "hello", "protocol": 1, "extension": "tr.ornek.imar-denetim", "version": "1.0.0",
     "digest": "sha256:…", "source": "/…/packages/tr.ornek.imar-denetim/1.0.0", "paths": ["src"]}
    {"type": "invoke", "id": 1, "kind": "command", "local": "check-setbacks",
     "handler": "imar.logic:check_setbacks", "params": {}, "settings": {}, "document": true}
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
"""

from __future__ import annotations

import importlib
import io
import itertools
import json
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
        fn = resolve(str(message.get("handler", "")), str(message.get("kind", "")), str(message.get("local", "")))
        link = ext._Link(channel.send, run_id, stop)
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
