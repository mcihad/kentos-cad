"""The console's Python side (kentos.host, docs/adr/0132) against a stand-in
desktop: this test starts `python -m kentos.host` as the desktop does, sends
it code, and answers its requests on a headless drawing, as the desktop
answers them on its open one."""

from __future__ import annotations

import json
import subprocess
import sys
import unittest
from typing import Any

import kentos.cad as cad
from kentos import _native

SQUARE = "[(423500, 4512300), (423520, 4512300), (423520, 4512312.5), (423500, 4512312.5)]"


class Desktop:
    """Runs the console process and answers it on a headless drawing."""

    def __init__(self) -> None:
        self.drawing = cad.Document.new("Konsol", srid=5256)
        self.session = self.drawing._session
        self.groups: list[str] = []
        self.process = subprocess.Popen(
            [sys.executable, "-m", "kentos.host"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        self.ready = self.receive()
        self.runs = 0

    def receive(self) -> dict[str, Any]:
        assert self.process.stdout is not None
        line = self.process.stdout.readline()
        assert line, "the console ended"
        message: dict[str, Any] = json.loads(line)
        return message

    def send(self, message: dict[str, Any]) -> None:
        assert self.process.stdin is not None
        self.process.stdin.write((json.dumps(message) + "\n").encode())
        self.process.stdin.flush()

    def run(self, code: str) -> tuple[dict[str, Any], str, str]:
        """(done, what it printed, what it wrote to its errors)."""
        self.runs += 1
        self.send({"type": "exec", "id": self.runs, "code": code})
        out, err = "", ""
        while True:
            m = self.receive()
            if m["type"] == "out":
                if m["stream"] == "out":
                    out += m["text"]
                else:
                    err += m["text"]
            elif m["type"] == "call":
                self.answer(m)
            elif m["type"] == "done":
                assert m["id"] == self.runs
                return m, out, err

    def answer(self, call: dict[str, Any]) -> None:
        method, params = call["method"], call["params"]
        try:
            if method == "run":
                result: Any = json.loads(
                    self.session.run(params["command"], params["op"], json.dumps(params["input"]), params.get("version"))
                )
            elif method in ("summary", "layers"):
                result = json.loads(getattr(self.session, method)())
            elif method == "entities":
                bbox = params.get("bbox")
                result = json.loads(
                    self.session.entities(
                        params.get("layer"),
                        params.get("kinds"),
                        None if bbox is None else tuple(bbox),
                        params.get("after"),
                        params.get("limit"),
                    )
                )
            elif method in ("entity", "measure"):
                result = json.loads(getattr(self.session, method)(params["uid"]))
            elif method in ("begin_group", "end_group", "cancel_group"):
                self.groups.append(method if method != "begin_group" else f"begin:{params['label']}")
                result = True
            else:
                raise _native.HostError("unknown_method", method)
        except _native.HostError as e:
            code, message = e.args
            self.send({"type": "reply", "id": call["id"], "ok": False, "code": code, "message": message})
            return
        self.send({"type": "reply", "id": call["id"], "ok": True, "result": result})

    def close(self) -> str:
        assert self.process.stdin is not None and self.process.stderr is not None
        self.process.stdin.close()
        self.process.wait(timeout=30)
        rest: str = self.process.stderr.read().decode()
        assert self.process.stdout is not None
        self.process.stdout.close()
        self.process.stderr.close()
        return rest


class Host(unittest.TestCase):
    def setUp(self) -> None:
        self.desktop = Desktop()

    def tearDown(self) -> None:
        self.desktop.close()

    def test_it_says_it_is_ready_with_its_versions(self) -> None:
        ready = self.desktop.ready
        self.assertEqual(ready["type"], "ready")
        self.assertEqual(ready["kentos"], _native.__version__)
        self.assertEqual(ready["python"], ".".join(map(str, sys.version_info[:3])))

    def test_code_writes_the_desktops_drawing_and_reads_it_back(self) -> None:
        done, out, _ = self.desktop.run(
            f"made = cad.polygon.create(doc, layer_id=doc.active_layer, pts={SQUARE})\n"
            "print(doc.measure(made.uid).area)\n"
            "cad.current() is doc"
        )
        self.assertTrue(done["ok"], done)
        self.assertEqual(out, "250.0\nTrue\n")
        self.assertEqual(len(self.desktop.drawing), 1)

    def test_names_stay_and_a_last_expression_shows_its_value(self) -> None:
        self.desktop.run("a = 21")
        done, out, _ = self.desktop.run("a * 2")
        self.assertTrue(done["ok"])
        self.assertEqual(out, "42\n")
        self.assertEqual(self.desktop.run("None")[1], "")
        self.assertEqual(self.desktop.run("_")[1], "42\n")

    def test_an_error_is_the_scripts_traceback_and_the_console_goes_on(self) -> None:
        done, _, _ = self.desktop.run("def f():\n    raise ValueError('bozuk değer')\n\nf()")
        self.assertFalse(done["ok"])
        self.assertEqual(done["exception"], "ValueError")
        self.assertIn('File "<konsol 1>", line 2, in f', done["error"])
        self.assertIn("raise ValueError('bozuk değer')", done["error"])
        self.assertNotIn("host.py", done["error"])
        syntax, _, _ = self.desktop.run("x = (")
        self.assertEqual(syntax["exception"], "SyntaxError")
        self.assertTrue(self.desktop.run("1")[0]["ok"])

    def test_a_commands_refusal_raises_in_the_script_with_the_scripts_line_only(self) -> None:
        done, _, _ = self.desktop.run(f"cad.polygon.create(doc, layer_id='yok', pts={SQUARE})")
        self.assertFalse(done["ok"])
        self.assertEqual(done["exception"], "CommandFailed")
        self.assertIn("layer_not_found", done["error"])
        self.assertIn('File "<konsol 1>", line 1', done["error"])
        self.assertNotIn("kentos/cad", done["error"], "the package's own frames are left out")
        # A fault that is not a refusal keeps every frame.
        fault, _, _ = self.desktop.run("cad.Vec2.of(object())")
        self.assertIn("kentos/cad", fault["error"])

    def test_what_the_desktop_does_itself_is_refused_with_its_reason(self) -> None:
        done, _, _ = self.desktop.run("doc.undo()")
        self.assertEqual(done["exception"], "NotRunHere")
        self.assertIn("not_in_console", done["error"])
        self.assertEqual(self.desktop.run("doc.save('x.kcad')")[0]["exception"], "NotRunHere")
        self.assertEqual(self.desktop.run("input('?')")[0]["exception"], "RuntimeError")

    def test_a_group_is_asked_of_the_desktop(self) -> None:
        done, _, _ = self.desktop.run(
            f"with doc.group('Parseller'):\n    cad.polygon.create(doc, layer_id=doc.active_layer, pts={SQUARE})"
        )
        self.assertTrue(done["ok"], done)
        self.assertEqual(self.desktop.groups, ["begin:Parseller", "end_group"])
        self.desktop.run("with doc.group('Bozuk'):\n    raise KeyError('x')")
        self.assertEqual(self.desktop.groups[-2:], ["begin:Bozuk", "cancel_group"])

    def test_warnings_and_errors_go_to_the_error_stream_and_output_below_python_to_stderr(self) -> None:
        _, _, err = self.desktop.run("import warnings\nwarnings.warn('dikkat')")
        self.assertIn("UserWarning: dikkat", err)
        done, out, _ = self.desktop.run("import os\nos.write(1, b'alttan\\n')\nprint('üstten')")
        self.assertTrue(done["ok"])
        self.assertEqual(out, "üstten\n")
        # The channel stayed whole; what went under Python came out on standard error.
        self.assertTrue(self.desktop.run("2")[0]["ok"])
        self.assertIn("alttan", self.desktop.close())
        self.desktop = Desktop()

    def test_exit_does_not_end_the_console(self) -> None:
        done, _, err = self.desktop.run("exit()")
        self.assertTrue(done["ok"])
        self.assertIn("exit() konsolu kapatmaz", err)
        self.assertEqual(self.desktop.run("3")[1], "3\n")


class Outside(unittest.TestCase):
    def test_outside_the_console_there_is_no_desktop_drawing(self) -> None:
        with self.assertRaises(cad.NotRunHere) as caught:
            cad.current()
        self.assertEqual(caught.exception.code, "no_desktop")


if __name__ == "__main__":
    unittest.main()
