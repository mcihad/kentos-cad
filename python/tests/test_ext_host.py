"""An extension's own process (kentos.ext_host, plan/eklenti.md ADIM 19) against a stand-in KentOS: this test starts
`python -m kentos.ext_host` as KentOS does, says hello with a package of its own, invokes its contributions and answers
their requests on a headless drawing, as KentOS answers them on its open one."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import tempfile
import textwrap
import time
import unittest
from typing import Any

import kentos.cad as cad

LOGIC = '''
"""A stand-in extension's functions."""
import os
import pathlib

from kentos import ext

# Imported only when a contribution is invoked: this file says when.
pathlib.Path(os.environ["KENTOS_TEST_IMPORTED"]).write_text("imported")


@ext.command("count-layers")
def count_layers(ctx):
    print("katmanlar okunuyor")
    names = [layer.name for layer in ctx.doc.layers()]
    ctx.log(f"{len(names)} katman")
    return ctx.report([{"katman": n} for n in names], title="Katmanlar")


@ext.command("loop")
def loop(ctx):
    n = 0
    while True:
        n += 1
        ctx.check()


@ext.command("raises")
def raises(ctx):
    return 1 / 0


@ext.command("ordered")
def ordered(ctx):
    return ctx.report([{"yapı": "B1", "mesafe_m": 2.0}, {"yapı": "B2", "durum": "parsel dışında"}])


@ext.command("schema")
def schema(ctx):
    return ctx.apply_declarative_schema("schema.json")


@ext.map_tool("place")
def place(ctx, shape):
    print(repr(shape))
    return ctx.add_feature(ctx.doc.active_layer, shape, {"durum": "BOS", "boyut": 1.1})


@ext.command("another-id")
def wrong_id(ctx):
    return ctx.message("çalışmamalıydı")


def unmarked(ctx):
    return ctx.message("çalışmamalıydı")


class Scalar:
    """Shaped as a NumPy scalar: what a tool's NumPy gives, without NumPy here."""

    def __init__(self, value):
        self.value = value

    def item(self):
        return self.value


class Array:
    """Shaped as a NumPy array."""

    def __init__(self, values):
        self.values = values

    def tolist(self):
        return list(self.values)


def shoelace(pts):
    return abs(sum(x0 * y1 - x1 * y0 for (x0, y0), (x1, y1) in zip(pts, pts[1:] + pts[:1]))) / 2


@ext.processing("areas")
def areas(ctx, parcels, limit, prefix="P"):
    rows, out, big = [], [], []
    for n, parcel in enumerate(ctx.read_features(parcels), start=1):
        area = shoelace(parcel.pts)
        rows.append({"parsel": f"{prefix}{n}", "alan": Scalar(area), "katman": parcel.layer})
        out.append(ext.NewFeature(source=parcel, attrs={"alan": Scalar(area), "no": n}))
        if area > limit:
            big.append(parcel)
    out.append(ext.NewFeature(geometry=ext.polygon(Array([(0, 0), (1, 0), (1, 1)])), attrs={"yeni": True}))
    ctx.select(big)
    ctx.summary(f"{len(rows)} parsel ölçüldü; {parcels.description}")
    return {"parcels": out, "table": rows, "total": Array([sum(r["alan"].item() for r in rows)]),
            "nan": float("nan"), "numbered": parcels[0].number("no")}


class Shape:
    """Shaped as a shapely geometry: what speaks GeoJSON through __geo_interface__."""

    __geo_interface__ = {"type": "Point", "coordinates": (3.0, 4.0)}


class Frame:
    """Shaped as a pandas DataFrame: its columns in their order, its rows as records."""

    columns = ["b", "a"]

    def to_dict(self, orient):
        assert orient == "records"
        return [{"a": 1, "b": Scalar(2.5)}, {"a": 3, "b": None}]


@ext.processing("libraries")
def libraries(ctx, parcels, limit, prefix="P"):
    import datetime
    import decimal

    first = parcels[0]
    return {
        "parcels": [ext.NewFeature(geometry=Shape(), attrs={"tarih": datetime.date(2026, 11, 14)}),
                    ext.NewFeature(geometry=ext.polygon([(0, 0), (4, 0), (4, 4)], holes=[[(1, 1), (2, 1), (2, 2)]]))],
        "table": Frame(),
        "rows": [{"z": 1, "a": decimal.Decimal("2.5")}, {"a": 3}],
        "geo": first.__geo_interface__,
        "collection": parcels.__geo_interface__["type"],
        "pts": first.pts,
    }


@ext.expr("ada-parsel")
def ada_parsel(ada, parsel):
    if parsel == "x":
        raise ValueError("geçersiz parsel")
    return f"{ada}/{parsel}" if ada and parsel else None


@ext.processing("slow")
def slow(ctx, parcels):
    for parcel in ctx.read_features(parcels):
        pass
    while True:
        ctx.check()


@ext.processing("not-a-dict")
def not_a_dict(ctx, parcels):
    return [1, 2]


@ext.processing("bad-value")
def bad_value(ctx, parcels):
    return {"parcels": object()}
'''


def polygon(uid, pts, layer="parsel", attrs=None):
    """A feature as KentOS sends it: its persistent id, its layer's name, its GeoJSON shape and its object (the slot id
    zeroed)."""
    ring = [list(p) for p in pts] + [list(pts[0])]
    return {
        "uid": uid,
        "layer": layer,
        "geometry": {"type": "Polygon", "coordinates": [ring]},
        "entity": {"kind": "polygon", "id": 0, "layerId": "p", "attrs": attrs or {},
                   "pts": [{"x": x, "y": y} for x, y in pts]},
    }


class Kentos:
    """Runs an extension's process as KentOS starts it (extensions/runner.rs `interpreter_args`) and answers it on a
    headless drawing."""

    def __init__(self, package: str, imported: str, pycache: str) -> None:
        self.drawing = cad.Document.new("Eklenti", srid=5256)
        self.session = self.drawing._session
        env = dict(os.environ, KENTOS_TEST_IMPORTED=imported)
        self.process = subprocess.Popen(
            [sys.executable, "-I", "-X", f"pycache_prefix={pycache}", "-m", "kentos.ext_host"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=env,
        )
        self.package = package
        self.runs = 0

    def receive(self) -> dict[str, Any]:
        assert self.process.stdout is not None
        line = self.process.stdout.readline()
        assert line, "the process ended"
        message: dict[str, Any] = json.loads(line)
        return message

    def send(self, message: dict[str, Any]) -> None:
        assert self.process.stdin is not None
        self.process.stdin.write((json.dumps(message) + "\n").encode())
        self.process.stdin.flush()

    def hello(self, protocol: int = 1) -> dict[str, Any]:
        self.send(
            {
                "type": "hello",
                "protocol": protocol,
                "extension": "tr.ornek.deneme",
                "version": "1.0.0",
                "digest": "sha256:0",
                "source": self.package,
                "paths": ["src"],
            }
        )
        return self.receive()

    def invoke(
        self,
        local: str,
        handler: str,
        cancel_after: float | None = None,
        kind: str = "command",
        params: dict[str, Any] | None = None,
        inputs: dict[str, Any] | None = None,
    ) -> tuple[dict[str, Any], list[dict[str, Any]]]:
        """(done, what came before it)."""
        self.runs += 1
        message: dict[str, Any] = {
            "type": "invoke",
            "id": self.runs,
            "kind": kind,
            "local": local,
            "handler": handler,
            "params": params or {},
            "settings": {},
            "document": kind in ("command", "mapTool"),
        }
        if inputs is not None:
            message["inputs"] = inputs
        self.send(message)
        if cancel_after is not None:
            time.sleep(cancel_after)
            self.send({"type": "cancel", "id": self.runs})
        said: list[dict[str, Any]] = []
        while True:
            m = self.receive()
            if m["type"] == "call":
                self.answer(m)
            elif m["type"] == "done":
                assert m["id"] == self.runs
                return m, said
            else:
                said.append(m)

    def answer(self, call: dict[str, Any]) -> None:
        method, params = call["method"], call["params"]
        if method in ("summary", "layers"):
            result: Any = json.loads(getattr(self.session, method)())
            self.send({"type": "reply", "id": call["id"], "ok": True, "result": result})
        elif method == "run":
            # A write, as KentOS's broker runs it on its drawing (the map tool's add_feature).
            result = json.loads(
                self.session.run(params["command"], params["op"], json.dumps(params["input"]), params.get("version"))
            )
            self.send({"type": "reply", "id": call["id"], "ok": True, "result": result})
        else:
            self.send({"type": "reply", "id": call["id"], "ok": False, "code": "capability", "message": method})

    def close(self) -> None:
        if self.process.poll() is None:
            self.send({"type": "quit"})
            try:
                self.process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                self.process.kill()
        for stream in (self.process.stdin, self.process.stdout, self.process.stderr):
            if stream is not None:
                stream.close()


class ExtHostTest(unittest.TestCase):
    def setUp(self) -> None:
        self.folder = tempfile.mkdtemp(prefix="kentos-ext-host-")
        source = os.path.join(self.folder, "src", "deneme")
        os.makedirs(source)
        with open(os.path.join(source, "__init__.py"), "w", encoding="utf-8") as f:
            f.write("")
        with open(os.path.join(source, "logic.py"), "w", encoding="utf-8") as f:
            f.write(textwrap.dedent(LOGIC))
        self.imported = os.path.join(self.folder, "imported.txt")
        self.pycache = tempfile.mkdtemp(prefix="kentos-ext-pyc-")
        self.kentos = Kentos(self.folder, self.imported, self.pycache)

    def tearDown(self) -> None:
        self.kentos.close()
        shutil.rmtree(self.folder, ignore_errors=True)
        shutil.rmtree(self.pycache, ignore_errors=True)

    def test_code_is_imported_only_when_invoked_and_answers_with_its_report(self) -> None:
        ready = self.kentos.hello()
        self.assertEqual(ready["type"], "ready")
        self.assertEqual(ready["protocol"], 1)
        self.assertFalse(os.path.exists(self.imported), "nothing imported at hello")
        done, said = self.kentos.invoke("count-layers", "deneme.logic:count_layers")
        self.assertTrue(done["ok"], done)
        self.assertTrue(os.path.exists(self.imported))
        result = done["result"]
        self.assertEqual(result["kind"], "report")
        self.assertEqual(result["title"], "Katmanlar")
        self.assertGreaterEqual(len(result["rows"]), 1)
        printed = [m for m in said if m["type"] == "out"]
        self.assertEqual(printed[0]["text"], "katmanlar okunuyor\n")
        self.assertEqual(printed[0]["id"], 1, "what a run prints carries its id")
        logged = [m for m in said if m["type"] == "log"]
        self.assertTrue(logged and logged[0]["text"].endswith("katman"))

    def test_a_reports_columns_keep_the_order_its_rows_give(self) -> None:
        self.kentos.hello()
        done, _ = self.kentos.invoke("ordered", "deneme.logic:ordered")
        self.assertEqual(done["result"]["columns"], ["yapı", "mesafe_m", "durum"])

    def test_what_this_kentos_cannot_do_is_said_as_unavailable(self) -> None:
        self.kentos.hello()
        done, _ = self.kentos.invoke("schema", "deneme.logic:schema")
        self.assertFalse(done["ok"])
        self.assertTrue(done.get("unavailable"), done)
        self.assertIn("schema.json", done["message"])
        self.assertNotIn("error", done, "no traceback: the extension's code did not fail")

    def test_the_package_stays_as_installed(self) -> None:
        # Its integrity check refuses files added to it: the bytecode goes to the environment's folder (14 November 2026:
        # an extension stopped at the next start after its first run).
        self.kentos.hello()
        done, _ = self.kentos.invoke("count-layers", "deneme.logic:count_layers")
        self.assertTrue(done["ok"], done)
        added = [
            os.path.join(root, name)
            for root, dirs, files in os.walk(os.path.join(self.folder, "src"))
            for name in dirs + files
            if name == "__pycache__" or name.endswith(".pyc")
        ]
        self.assertEqual(added, [])
        compiled = [name for _, _, files in os.walk(self.pycache) for name in files if name.startswith("logic.")]
        self.assertTrue(compiled, "its bytecode is kept in the environment's folder")

    def test_a_function_not_marked_as_the_manifest_says_is_refused(self) -> None:
        self.kentos.hello()
        done, _ = self.kentos.invoke("count-layers", "deneme.logic:wrong_id")
        self.assertTrue(done.get("refused"), done)
        self.assertIn("@ext.command(\"another-id\")", done["message"])
        done, _ = self.kentos.invoke("count-layers", "deneme.logic:unmarked")
        self.assertTrue(done.get("refused"), done)
        self.assertIn("işaretsiz", done["message"])
        done, _ = self.kentos.invoke("count-layers", "deneme.logic:missing")
        self.assertTrue(done.get("refused"), done)
        done, _ = self.kentos.invoke("count-layers", "no-colon")
        self.assertTrue(done.get("refused"), done)

    def test_an_exception_is_reported_with_its_traceback(self) -> None:
        self.kentos.hello()
        done, _ = self.kentos.invoke("raises", "deneme.logic:raises")
        self.assertFalse(done["ok"])
        self.assertEqual(done["exception"], "ZeroDivisionError")
        self.assertIn("1 / 0", done["error"])

    def test_durdur_is_heard_while_the_code_computes(self) -> None:
        self.kentos.hello()
        started = time.monotonic()
        done, _ = self.kentos.invoke("loop", "deneme.logic:loop", cancel_after=0.3)
        self.assertTrue(done.get("stopped"), done)
        self.assertLess(time.monotonic() - started, 3.0, "stopped within a second of Durdur")
        # The process goes on: the next run works.
        done, _ = self.kentos.invoke("count-layers", "deneme.logic:count_layers")
        self.assertTrue(done["ok"], done)

    def test_a_package_carrying_kentos_is_refused(self) -> None:
        # KentOS's library comes from the environment (PY-UV-31): a package's own `kentos` would shadow it.
        os.makedirs(os.path.join(self.folder, "src", "kentos"))
        with open(os.path.join(self.folder, "src", "kentos", "__init__.py"), "w", encoding="utf-8") as f:
            f.write("")
        refused = self.kentos.hello()
        self.assertEqual(refused["type"], "refused")
        self.assertIn("kentos", refused["message"])

    def test_a_processing_tool_gets_its_inputs_as_data_and_answers_with_typed_outputs(self) -> None:
        self.kentos.hello()
        parcels = {
            "description": "2 kapalı alan; tüm nesneler",
            "features": [
                polygon("u-1", [(0, 0), (10, 0), (10, 10), (0, 10)], attrs={"no": "4,5"}),
                polygon("u-2", [(0, 0), (2, 0), (2, 2), (0, 2)]),
            ],
        }
        done, said = self.kentos.invoke(
            "areas", "deneme.logic:areas", kind="processing", params={"limit": 50}, inputs={"parcels": parcels}
        )
        self.assertTrue(done["ok"], done)
        result = done["result"]
        self.assertEqual(result["kind"], "outputs")
        outputs = result["outputs"]
        self.assertEqual(outputs["parcels"][0], {"attrs": {"alan": 100.0, "no": 1}, "source": "u-1"})
        self.assertEqual(
            outputs["parcels"][2]["geometry"],
            {"type": "Polygon", "coordinates": [[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 0.0]]]},
        )
        self.assertEqual(outputs["table"]["columns"], ["parsel", "alan", "katman"], "rows keep their columns' order")
        self.assertEqual(outputs["table"]["rows"][1], ["P2", 4.0, "parsel"])
        self.assertEqual(outputs["total"], [104.0], "a NumPy-shaped array comes as a list")
        self.assertIsNone(outputs["nan"], "a NaN comes as nothing: JSON does not carry it")
        self.assertEqual(outputs["numbered"], 4.5, "a Turkish decimal comma is read")
        self.assertEqual(result["select"], ["u-1"])
        self.assertEqual(result["summary"], "2 parsel ölçüldü; 2 kapalı alan; tüm nesneler")
        self.assertFalse([m for m in said if m["type"] == "call"], "a tool does not ask for the drawing")

    def test_whatever_library_made_an_output_it_comes_plain(self) -> None:
        # Not NumPy alone: a shapely-shaped shape, a pandas-shaped table, dates and decimals (the extension's own
        # environment decides its libraries).
        self.kentos.hello()
        parcels = {"features": [polygon("u-1", [(0, 0), (10, 0), (10, 10), (0, 10)])]}
        done, _ = self.kentos.invoke(
            "libraries", "deneme.logic:libraries", kind="processing", params={"limit": 1}, inputs={"parcels": parcels}
        )
        self.assertTrue(done["ok"], done)
        out = done["result"]["outputs"]
        self.assertEqual(out["parcels"][0]["geometry"], {"type": "Point", "coordinates": [3.0, 4.0]})
        self.assertEqual(out["parcels"][0]["attrs"], {"tarih": "2026-11-14"})
        hole = out["parcels"][1]["geometry"]["coordinates"][1]
        self.assertEqual(hole[0], hole[-1], "a GeoJSON ring is closed")
        self.assertEqual(out["table"], {"columns": ["b", "a"], "rows": [[2.5, 1], [None, 3]]})
        self.assertEqual(out["rows"], {"columns": ["z", "a"], "rows": [[1, 2.5], [None, 3]]})
        self.assertEqual(out["geo"]["type"], "Feature")
        self.assertEqual(out["geo"]["id"], "u-1")
        self.assertEqual(out["geo"]["geometry"]["type"], "Polygon")
        self.assertEqual(out["collection"], "FeatureCollection")
        self.assertEqual(out["pts"], [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]], "the ring not closed again")

    def test_an_expression_function_answers_a_batch_of_rows(self) -> None:
        self.kentos.hello()
        self.kentos.runs += 1
        self.kentos.send({"type": "invoke", "id": self.kentos.runs, "kind": "expression", "local": "ada-parsel",
                          "handler": "deneme.logic:ada_parsel", "rows": [["101", "1"], ["102", None], ["1", "x"]]})
        while True:
            done = self.kentos.receive()
            if done["type"] == "done":
                break
        self.assertTrue(done["ok"], done)
        result = done["result"]
        self.assertEqual(result["values"], ["101/1", None, None])
        self.assertEqual(result["failed"], 1, "a row's failure empties that row alone")
        self.assertIn("ValueError: geçersiz parsel", result["error"])

    def test_a_processing_tool_is_stopped_while_it_computes(self) -> None:
        self.kentos.hello()
        parcels = {"features": [polygon("u-1", [(0, 0), (1, 0), (1, 1)])]}
        started = time.monotonic()
        done, _ = self.kentos.invoke(
            "slow", "deneme.logic:slow", cancel_after=0.3, kind="processing", inputs={"parcels": parcels}
        )
        self.assertTrue(done.get("stopped"), done)
        self.assertLess(time.monotonic() - started, 3.0)

    def test_what_a_processing_tool_cannot_return_is_said_with_where(self) -> None:
        self.kentos.hello()
        parcels = {"features": []}
        done, _ = self.kentos.invoke("not-a-dict", "deneme.logic:not_a_dict", kind="processing", inputs={"parcels": parcels})
        self.assertEqual(done["exception"], "TypeError")
        self.assertIn("sözlükte", done["message"])
        done, _ = self.kentos.invoke("bad-value", "deneme.logic:bad_value", kind="processing", inputs={"parcels": parcels})
        self.assertEqual(done["exception"], "TypeError")
        self.assertIn("parcels", done["message"])

    def test_a_map_tool_writes_the_shape_the_user_drew(self) -> None:
        # plan/eklenti.md ADIM 19: KentOS gives the drawn shape (GeoJSON) to the marked function with a command's
        # context; add_feature writes it to the layer through KentOS's commands (one undo step there).
        self.kentos.hello()
        ring = [[0, 0], [1.1, 0], [1.1, 2.2], [0, 2.2], [0, 0]]
        done, said = self.kentos.invoke(
            "place",
            "deneme.logic:place",
            kind="mapTool",
            params={"shape": {"type": "Polygon", "coordinates": [ring]}},
        )
        self.assertTrue(done["ok"], done)
        self.assertIn("bir alan yazıldı", done["result"]["message"])
        self.assertIn("Shape(Polygon, 4 köşe)", "".join(m.get("text", "") for m in said))
        entities = json.loads(self.kentos.session.entities(None, None, None, None, None))["items"]
        self.assertEqual(len(entities), 1)
        made = entities[0]["entity"]
        self.assertEqual((made["kind"], made["attrs"]), ("polygon", {"durum": "BOS", "boyut": "1.1"}))
        self.assertEqual(len(made["pts"]), 4)

    def test_a_processing_tool_is_not_run_as_a_command(self) -> None:
        self.kentos.hello()
        done, _ = self.kentos.invoke("areas", "deneme.logic:areas")
        self.assertTrue(done.get("refused"), done)

    def test_another_protocol_is_refused(self) -> None:
        refused = self.kentos.hello(protocol=99)
        self.assertEqual(refused["type"], "refused")
        self.assertIn("protokol", refused["message"])


if __name__ == "__main__":
    unittest.main()
