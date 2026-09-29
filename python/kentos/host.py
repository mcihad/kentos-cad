"""The desktop's Python console, on the Python side (docs/adr/0132):
``python -m kentos.host``, which the KentOS desktop starts and stops.

The two talk in JSON lines: the desktop writes to this process's standard
input, this process answers on its standard output. What scripts print is
captured and sent as messages, so the channel stays clean; what is written
below Python (a C library) goes to standard error, which the desktop shows
as it comes.

Desktop → host::

    {"type": "exec", "id": 1, "code": "…"}
    {"type": "complete", "id": 2, "code": "…", "cursor": 14}
    {"type": "signature", "id": 3, "code": "…", "cursor": 20}
    {"type": "reply", "id": 7, "ok": true, "result": …}
    {"type": "reply", "id": 7, "ok": false, "code": "…", "message": "…"}

Host → desktop::

    {"type": "ready", "python": "3.14.4", "kentos": "0.1.0"}
    {"type": "out", "stream": "out" | "err", "text": "…"}
    {"type": "call", "id": 7, "method": "run", "params": {…}}
    {"type": "done", "id": 1, "ok": true}
    {"type": "done", "id": 1, "ok": false, "error": "Traceback …", "exception": "ValueError"}
    {"type": "completions", "id": 2, "start": 10, "items": [{"text": …, "kind": …, "detail": …}]}
    {"type": "signature", "id": 3, "label": "create(doc, /, *, layer_id: str, …)", "doc": "…", "argument": "…"}

Completion and signatures are asked only between runs (``_assist``), from
the console's names, without running anything written.

A run of code sees ``cad`` (``kentos.cad``) and ``doc``: the drawing open in
the desktop (``kentos.cad.current()``), whose requests the desktop answers on
its own drawing. The desktop makes every run one undo step and takes it back
when the run raises or is stopped. Names a run defines stay for the next
runs until the console restarts.
"""

from __future__ import annotations

import ast
import builtins
import io
import json
import linecache
import os
import platform
import sys
import traceback
from typing import IO, Any

import kentos

from . import _assist, _native
from . import cad
from .cad import document as _document
from .cad.errors import KentosError

# The package's own frames are left out of a refusal's traceback: the script's line says where.
PACKAGE = os.path.dirname(os.path.abspath(kentos.__file__)) + os.sep


class Channel:
    """The JSON lines to and from the desktop."""

    def __init__(self, incoming: IO[bytes], outgoing: IO[bytes]) -> None:
        self._in = incoming
        self._out = outgoing
        self._calls = 0

    def send(self, message: dict[str, Any]) -> None:
        line = json.dumps(message, ensure_ascii=False, separators=(",", ":")) + "\n"
        self._out.write(line.encode("utf-8"))
        self._out.flush()

    def receive(self) -> dict[str, Any] | None:
        line = self._in.readline()
        if not line:
            return None
        message: dict[str, Any] = json.loads(line)
        return message

    def call(self, method: str, params: dict[str, Any]) -> Any:
        """Asks the desktop and waits for its answer."""
        self._calls += 1
        call = self._calls
        self.send({"type": "call", "id": call, "method": method, "params": params})
        while True:
            message = self.receive()
            if message is None:
                # The desktop went away: nothing is left to answer to.
                os._exit(0)
            if message.get("type") == "reply" and message.get("id") == call:
                if message.get("ok"):
                    return message.get("result")
                raise _native.HostError(message.get("code", "host"), message.get("message", ""))


class Stream(io.TextIOBase):
    """``sys.stdout`` or ``sys.stderr``: whole lines go to the desktop."""

    encoding = "utf-8"

    def __init__(self, channel: Channel, name: str) -> None:
        super().__init__()
        self._channel = channel
        self._name = name
        self._pending = ""

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
        self._channel.send({"type": "out", "stream": self._name, "text": text})


class DesktopSession(_document.SessionBase):
    """The drawing open in the desktop, as a Document works over it: every
    request goes to the desktop, which answers on its own drawing."""

    def __init__(self, channel: Channel) -> None:
        self._channel = channel

    def _ask(self, method: str, **params: Any) -> str:
        return json.dumps(self._channel.call(method, params), ensure_ascii=False)

    def run(self, command: str, op: str, input: str, version: int | None = None) -> str:
        return self._ask("run", command=command, op=op, input=json.loads(input), version=version)

    def summary(self) -> str:
        return self._ask("summary")

    def layers(self) -> str:
        return self._ask("layers")

    def blocks(self) -> str:
        return self._ask("blocks")

    def entities(
        self,
        layer: str | None = None,
        kinds: list[str] | None = None,
        bbox: tuple[float, float, float, float] | None = None,
        after: str | None = None,
        limit: int | None = None,
    ) -> str:
        return self._ask(
            "entities",
            layer=layer,
            kinds=kinds,
            bbox=None if bbox is None else list(bbox),
            after=after,
            limit=limit,
        )

    def entity(self, uid: str) -> str:
        return self._ask("entity", uid=uid)

    def measure(self, uid: str) -> str:
        return self._ask("measure", uid=uid)

    def begin_group(self, label: str) -> None:
        self._channel.call("begin_group", {"label": label})

    def end_group(self) -> bool:
        return bool(self._channel.call("end_group", {}))

    def cancel_group(self) -> bool:
        return bool(self._channel.call("cancel_group", {}))

    def undo(self) -> str | None:
        raise _native.HostError(
            "not_in_console",
            "Konsolda geri alma yok: bir çalıştırma tek adımdır, bitince masaüstünün Geri al'ı onu bütünüyle geri alır.",
        )

    def redo(self) -> str | None:
        raise _native.HostError("not_in_console", "Konsolda yineleme yok: masaüstünün Yinele'si yineler.")

    def save(self, path: str | None = None) -> str:
        raise _native.HostError(
            "not_in_console",
            "Açık çizimi masaüstü kaydeder (Kaydet, Ctrl+S); başka bir çizim için Document.new ya da Document.open.",
        )

    def to_bytes(self) -> bytes:
        raise _native.HostError(
            "not_in_console",
            "Açık çizimin dosyasını masaüstü yazar; Farklı kaydet ile bir .kcad olarak kaydedin.",
        )


def _no_input(prompt: object = "") -> str:
    raise RuntimeError("input() konsolda yok: değerleri betiğe yazın.")


class Console:
    """Runs code as the console does: statements, then the value of a last
    expression, in names that stay from run to run."""

    def __init__(self, channel: Channel) -> None:
        self.channel = channel
        self.doc = cad.Document(DesktopSession(channel))
        self.names: dict[str, Any] = {
            "__name__": "__console__",
            "__builtins__": builtins,
            "cad": cad,
            "doc": self.doc,
        }

    def run(self, code: str, name: str) -> tuple[bool, str, str]:
        """(ok, the traceback, the exception's name)."""
        # The source is kept for the tracebacks of later runs too (a function defined here).
        linecache.cache[name] = (len(code), None, code.splitlines(True), name)
        try:
            tree = ast.parse(code, filename=name, mode="exec")
            last = None
            if tree.body and isinstance(tree.body[-1], ast.Expr):
                last = ast.Expression(tree.body[-1].value)
                tree.body.pop()
            exec(compile(tree, name, "exec"), self.names)
            if last is not None:
                value = eval(compile(last, name, "eval"), self.names)
                if value is not None:
                    self.names["_"] = value
                    print(repr(value))
            return True, "", ""
        except SystemExit:
            print("exit() konsolu kapatmaz; Yeniden başlat konsolu yeniden başlatır.", file=sys.stderr)
            return True, "", ""
        except BaseException as e:
            tb = e.__traceback__
            # The console's own frames are not the script's.
            while tb is not None and tb.tb_frame.f_code.co_filename == __file__:
                tb = tb.tb_next
            report = traceback.TracebackException(type(e), e, tb)
            if isinstance(e, KentosError):
                # A refusal (a command's, the host's): the script's lines, not the package's.
                report.stack = traceback.StackSummary.from_list(
                    [f for f in report.stack if not os.path.abspath(f.filename).startswith(PACKAGE)]
                )
            return False, "".join(report.format()), type(e).__name__


def main() -> int:
    incoming = sys.stdin.buffer
    # The channel keeps the real standard output; below Python it becomes standard error.
    outgoing = os.fdopen(os.dup(1), "wb", buffering=0)
    os.dup2(2, 1)
    channel = Channel(incoming, outgoing)
    out, err = Stream(channel, "out"), Stream(channel, "err")
    sys.stdout, sys.stderr = out, err
    sys.stdin = io.StringIO("")
    builtins.input = _no_input
    console = Console(channel)
    _document._CURRENT = console.doc
    channel.send({"type": "ready", "python": platform.python_version(), "kentos": kentos.__version__})
    while True:
        message = channel.receive()
        if message is None:
            return 0
        kind = message.get("type")
        if kind in ("complete", "signature"):
            code, cursor = str(message.get("code", "")), int(message.get("cursor", 0))
            try:
                if kind == "complete":
                    answer = {"type": "completions", **_assist.complete(console.names, code, cursor)}
                else:
                    answer = {"type": "signature", **(_assist.signature(console.names, code, cursor) or {})}
            except Exception:  # noqa: BLE001 - help that fails is no help, never an error
                answer = {"type": "completions" if kind == "complete" else "signature"}
            channel.send({**answer, "id": message.get("id")})
            continue
        if kind != "exec":
            continue
        run = message.get("id")
        name = str(message.get("name") or f"<konsol {run}>")
        ok, error, exception = console.run(str(message.get("code", "")), name)
        out.flush()
        err.flush()
        done: dict[str, Any] = {"type": "done", "id": run, "ok": ok}
        if not ok:
            done["error"] = error
            done["exception"] = exception
        channel.send(done)


if __name__ == "__main__":
    sys.exit(main())
