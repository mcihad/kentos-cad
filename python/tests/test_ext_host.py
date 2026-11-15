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


@ext.audit("no-number")
def no_number(facts):
    findings = []
    for f in facts.features("Yapı"):
        if not f.attrs.get("no"):
            findings.append(ext.AuditFinding(
                message=f"{f.id} numarasız", feature_id=f.id,
                fix=ext.SemanticFix(title="Numara ver", command="cad.entities.set",
                                    args={"uids": [f.id], "operation": "attributes", "attrs": {"no": "Y-99"}})))
    if facts.cut:
        findings.append(ext.AuditFinding(message="çizimin yalnız bir kısmı denetlendi", severity="info"))
    return findings


@ext.audit("not-findings")
def not_findings(facts):
    return ["bir bulgu değil"]


@ext.command("ordered")
def ordered(ctx):
    return ctx.report([{"yapı": "B1", "mesafe_m": 2.0}, {"yapı": "B2", "durum": "parsel dışında"}])


@ext.command("ask")
def ask(ctx):
    values = ctx.dialog(
        "Numarala",
        [{"type": "textField", "id": "onek", "label": "Önek", "value": "Y-"},
         {"type": "numberField", "id": "ilk", "label": "İlk numara", "value": 1}],
        ok="Numarala",
    )
    if values is None:
        return ctx.message("Vazgeçildi.")
    if not ctx.confirm(f"{values['onek']}{int(values['ilk'])} ile başlansın mı?"):
        return ctx.message("Onaylanmadı.")
    return ctx.report([values])


@ext.command("fill")
def fill(ctx):
    # docs/adr/8057 §32: a panel's command changes its panel as it runs.
    if ctx.panel is None:
        return ctx.message("Panel yok.")
    import datetime
    kept = ctx.panel.set("mahalle", options=[{"value": "m1", "label": "Merkez"}, "Yeni"], value="Yeni")
    ctx.panel.set("ara", enabled=False)
    ctx.panel.set("gun", value=datetime.date(2026, 11, 15))
    ctx.panel.rows("liste", [{"ad": "A", "alan": 12.5}], columns=["ad", "alan"])
    ctx.panel.say("Yüklendi.", "success")
    ctx.panel.focus("ad")
    shown = ctx.open_panel("sonuc-paneli")
    shown.rows("tablo", [{"no": 1}])
    return ctx.report([{"panel": ctx.panel.id, "kept": kept}])


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


@ext.processing("into-layer")
def into_layer(ctx, parcels, hedef):
    """Names its output layer: KentOS writes the outputs, the function is told where."""
    return {"target": hedef.get("newName") or hedef.get("layerId")}


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
        #: What the user answers each dialog, in turn (None: İptal); the dialogs asked, as they came.
        self.dialog_answers: list[Any] = []
        self.dialogs: list[dict[str, Any]] = []
        self.panel_calls: list[tuple[str, dict[str, Any]]] = []
        self.touched: set[str] = set()

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
        document: bool | None = None,
        layers: dict[str, Any] | None = None,
        panel: str | None = None,
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
            "document": kind in ("command", "mapTool") if document is None else document,
        }
        if inputs is not None:
            message["inputs"] = inputs
        if layers is not None:
            message["layers"] = layers
        if panel is not None:
            message["panel"] = panel
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
        elif method == "entities":
            # A page of objects, as KentOS reads its open drawing for doc.entities.
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
            self.send({"type": "reply", "id": call["id"], "ok": True, "result": result})
        elif method == "dialog":
            # The user answers in KentOS's window: here, the next answer given.
            self.dialogs.append(params)
            answer = self.dialog_answers.pop(0) if self.dialog_answers else None
            self.send({"type": "reply", "id": call["id"], "ok": True, "result": answer})
        elif method.startswith("panel."):
            # The panel the run started from, changed as KentOS's panel session would: applied, else not (the user
            # changed that input).
            self.panel_calls.append((method, params))
            applied = not (method == "panel.set" and params.get("id") in self.touched)
            self.send({"type": "reply", "id": call["id"], "ok": True, "result": {"applied": applied}})
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

    def test_a_command_asks_the_user_in_a_dialog_and_goes_on_with_the_answer(self) -> None:
        # docs/adr/8057 §29: ctx.dialog sends a panel's fields and waits; Tamam's values come back by id, İptal's None;
        # ctx.confirm is a dialog of words, True for its yes.
        self.kentos.hello()
        self.kentos.dialog_answers = [{"onek": "A-", "ilk": 7}, {}]
        done, _ = self.kentos.invoke("ask", "deneme.logic:ask")
        self.assertTrue(done["ok"], done)
        self.assertEqual(done["result"]["rows"], [{"onek": "A-", "ilk": 7}])
        first, second = self.kentos.dialogs
        self.assertEqual((first["title"], first["ok"], first["cancel"]), ("Numarala", "Numarala", "İptal"))
        self.assertEqual([c["id"] for c in first["body"]], ["onek", "ilk"])
        self.assertEqual(second["body"], [{"type": "text", "text": "A-7 ile başlansın mı?"}])
        self.assertEqual((second["ok"], second["cancel"]), ("Evet", "Hayır"))
        # İptal: None, the code says so.
        self.kentos.dialog_answers = [None]
        done, _ = self.kentos.invoke("ask", "deneme.logic:ask")
        self.assertEqual(done["result"]["message"], "Vazgeçildi.")
        # Onaylanmadı: the second dialog's İptal.
        self.kentos.dialog_answers = [{"onek": "Y-", "ilk": 1}, None]
        done, _ = self.kentos.invoke("ask", "deneme.logic:ask")
        self.assertEqual(done["result"]["message"], "Onaylanmadı.")

    def test_a_panels_command_changes_its_panel_as_it_runs(self) -> None:
        # docs/adr/8057 §32: ctx.panel is the panel the run started from; its changes go to KentOS as calls, values
        # made plain (a date as ISO text); a value the user typed meanwhile is kept (False). Without a panel, None.
        self.kentos.hello()
        self.kentos.touched = {"mahalle"}
        done, _ = self.kentos.invoke("fill", "deneme.logic:fill", panel="arama")
        self.assertTrue(done["ok"], done)
        self.assertEqual(done["result"]["rows"], [{"panel": "arama", "kept": False}])
        calls = self.kentos.panel_calls
        self.assertEqual(
            [m for m, _ in calls],
            ["panel.set", "panel.set", "panel.set", "panel.rows", "panel.say", "panel.focus", "panel.open", "panel.rows"],
        )
        self.assertEqual(
            calls[0][1],
            {"id": "mahalle", "value": "Yeni", "options": [{"value": "m1", "label": "Merkez"}, "Yeni"], "panel": "arama"},
        )
        self.assertEqual(calls[1][1], {"id": "ara", "enabled": False, "panel": "arama"})
        self.assertEqual(calls[2][1], {"id": "gun", "value": "2026-11-15", "panel": "arama"})
        self.assertEqual(
            calls[3][1], {"id": "liste", "rows": [{"ad": "A", "alan": 12.5}], "columns": ["ad", "alan"], "panel": "arama"}
        )
        self.assertEqual(calls[4][1], {"text": "Yüklendi.", "kind": "success", "panel": "arama"})
        # Another of its panels, opened by the code: its requests name it.
        self.assertEqual(calls[6][1], {"panel": "sonuc-paneli"})
        self.assertEqual(calls[7][1], {"id": "tablo", "rows": [{"no": 1}], "panel": "sonuc-paneli"})
        done, _ = self.kentos.invoke("fill", "deneme.logic:fill")
        self.assertEqual(done["result"]["message"], "Panel yok.")

    def test_what_this_kentos_cannot_do_is_said_as_unavailable(self) -> None:
        # A schema without a drawing it may write to: a missing ability, not the extension's failure.
        self.kentos.hello()
        done, _ = self.kentos.invoke("schema", "deneme.logic:schema", document=False)
        self.assertFalse(done["ok"])
        self.assertTrue(done.get("unavailable"), done)
        self.assertIn("katman şeması", done["message"])
        self.assertNotIn("error", done, "no traceback: the extension's code did not fail")

    def test_a_packages_schema_is_set_up_through_the_broker(self) -> None:
        # docs/adr/8057 §23: schema.json read from the package's own folder, its layer written through KentOS's broker
        # with its schema name as its id.
        with open(os.path.join(self.folder, "schema.json"), "w", encoding="utf-8") as f:
            json.dump({"layers": [{"name": "mezar_yeri", "title": "Mezar yeri", "geometry": "area",
                                   "fields": [{"name": "mezar_no", "kind": "text"}]}]}, f)
        self.kentos.hello()
        done, _ = self.kentos.invoke("schema", "deneme.logic:schema")
        self.assertTrue(done["ok"], done)
        self.assertEqual(done["result"]["message"], "Şema kuruldu: 1 katman.")
        self.assertEqual(done["result"]["rows"], [{"Tür": "Katman", "Kimlik": "mezar_yeri", "Ad": "Mezar yeri",
                                                   "Sonuç": "eklendi"}])
        layers = json.loads(self.kentos.session.layers())
        found = [n for n in layers if n["id"] == "mezar_yeri"]
        self.assertEqual([n["name"] for n in found], ["Mezar yeri"])
        self.assertEqual([f["name"] for f in found[0]["fields"]], ["mezar_no"])

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

    def test_an_output_layer_is_given_only_to_a_function_that_names_it(self) -> None:
        # 15 November 2026: an agent's tool named its output layer and failed with a missing argument; a tool that does
        # not name it (as Vitrin's TAKS) is not given it.
        self.kentos.hello()
        parcels = {"description": "1 kapalı alan", "features": [polygon("u-1", [(0, 0), (1, 0), (1, 1)])]}
        layers = {"hedef": {"newName": "Boş mezarlar"}}
        done, _ = self.kentos.invoke(
            "into-layer", "deneme.logic:into_layer", kind="processing", inputs={"parcels": parcels}, layers=layers
        )
        self.assertTrue(done["ok"], done)
        self.assertEqual(done["result"]["outputs"]["target"], "Boş mezarlar")
        done, _ = self.kentos.invoke(
            "areas", "deneme.logic:areas", kind="processing", params={"limit": 50}, inputs={"parcels": parcels},
            layers={"cikti": {"newName": "Alanlar"}},
        )
        self.assertTrue(done["ok"], done)

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

    def audit(self, local: str, handler: str, facts: dict[str, Any]) -> dict[str, Any]:
        """An audit rule run on `facts` (docs/adr/8057 §17): its done."""
        self.kentos.runs += 1
        self.kentos.send({"type": "invoke", "id": self.kentos.runs, "kind": "audit", "local": local,
                          "handler": handler, "facts": facts, "document": False})
        while True:
            done = self.kentos.receive()
            if done["type"] == "done":
                return done

    def test_an_audit_rule_reads_the_layers_as_data_and_answers_findings(self) -> None:
        self.kentos.hello()
        building = lambda uid, attrs: {"uid": uid, "layer": "Yapı", "geometry": None,  # noqa: E731
                                       "entity": {"kind": "polygon", "attrs": attrs}}
        facts = {"layers": [{"id": "yapi", "name": "Yapı",
                             "features": [building("u1", {"no": "Y-01"}), building("u2", {})]}], "cut": True}
        done = self.audit("no-number", "deneme.logic:no_number", facts)
        self.assertTrue(done["ok"], done)
        self.assertEqual(done["result"]["kind"], "findings")
        findings = done["result"]["findings"]
        self.assertEqual([f.get("featureId") for f in findings], ["u2", None])
        self.assertEqual(findings[0]["severity"], "warning")
        self.assertEqual(findings[0]["fix"], {"title": "Numara ver", "command": "cad.entities.set",
                                              "args": {"uids": ["u2"], "operation": "attributes",
                                                       "attrs": {"no": "Y-99"}}})
        self.assertEqual(findings[1]["severity"], "info", "the rule knows the facts were cut")
        # What is not a finding is the rule's mistake, said with what to return.
        done = self.audit("not-findings", "deneme.logic:not_findings", facts)
        self.assertFalse(done["ok"])
        self.assertEqual(done["exception"], "TypeError")
        self.assertIn("ext.AuditFinding listesi", done["message"])

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


class ReferenceExtensionTest(unittest.TestCase):
    """MEBİS, the reference extension (fixtures/extensions/v1/mebis), run from its own folder as KentOS runs it."""

    def setUp(self) -> None:
        here = os.path.dirname(os.path.abspath(__file__))
        self.package = os.path.normpath(os.path.join(here, "..", "..", "fixtures", "extensions", "v1", "mebis"))
        self.pycache = tempfile.mkdtemp(prefix="kentos-ext-pyc-")
        self.kentos = Kentos(self.package, os.path.join(self.pycache, "imported.txt"), self.pycache)

    def tearDown(self) -> None:
        self.kentos.close()
        shutil.rmtree(self.pycache, ignore_errors=True)

    def test_its_search_lists_the_graves_whose_number_or_status_has_the_query(self) -> None:
        # Real window, 14 November 2026: the panel's search read the record instead of its object and found no grave.
        self.kentos.hello()
        done, _ = self.kentos.invoke("search", "mebis.logic:search", params={"query": "BOS"})
        self.assertTrue(done["ok"], done)
        self.assertIn("MEBİS katmanlarını kur", done["result"]["message"], "no layer yet: said, not a traceback")
        doc = self.kentos.drawing
        cad.layers.edit(doc, operation="add", id="mezar_yeri", name="Mezar yeri", geometry="area")
        ring = [(0.0, 0.0), (1.1, 0.0), (1.1, 2.2), (0.0, 2.2)]
        empty = cad.polygon.create(doc, layer_id="mezar_yeri", pts=ring, attrs={"mezar_no": "M-12", "durum": "BOS"})
        cad.polygon.create(doc, layer_id="mezar_yeri", pts=[(x + 2, y) for x, y in ring],
                           attrs={"mezar_no": "M-13", "durum": "DOLU"})
        for query in ("bos", "Boş"):
            done, _ = self.kentos.invoke("search", "mebis.logic:search", params={"query": query})
            self.assertTrue(done["ok"], done)
            result = done["result"]
            self.assertEqual(result["message"], "1 mezar yeri bulundu.", query)
            # The row's uid shows the grave when clicked; its columns are the user's words.
            self.assertEqual(result["rows"], [{"uid": empty.uid, "Mezar no": "M-12", "Ada no": "—", "Durum": "Boş"}])
        done, _ = self.kentos.invoke("search", "mebis.logic:search", params={"query": ""})
        self.assertEqual([r["Mezar no"] for r in done["result"]["rows"]], ["M-12", "M-13"])
        # Nothing found is a report of no rows, not a message: the panel's table empties (it kept the rows before).
        done, _ = self.kentos.invoke("search", "mebis.logic:search", params={"query": "yok"})
        self.assertEqual((done["result"]["kind"], done["result"]["rows"]), ("report", []))

    def test_its_second_tool_writes_the_empty_graves_to_a_layer(self) -> None:
        # The reference extension's two İşlemler tools: the first answers a table, this one a layer and a number.
        self.kentos.hello()
        ring = [(0, 0), (1.1, 0), (1.1, 2.2), (0, 2.2)]
        graves = {
            "description": "3 kapalı alan; katman",
            "features": [
                polygon("g-1", ring, layer="mezar_yeri", attrs={"mezar_no": "M-1", "ada_no": "A", "durum": "BOS"}),
                polygon("g-2", ring, layer="mezar_yeri", attrs={"mezar_no": "M-2", "ada_no": "A", "durum": "DOLU"}),
                polygon("g-3", ring, layer="mezar_yeri", attrs={"mezar_no": "M-3", "durum": "BOS"}),
            ],
        }
        done, _ = self.kentos.invoke(
            "empty-graves", "mebis.logic:empty_graves", kind="processing", inputs={"graves": graves}
        )
        self.assertTrue(done["ok"], done)
        outputs = done["result"]["outputs"]
        self.assertEqual(
            outputs["empty"],
            [{"source": "g-1", "attrs": {"mezar_no": "M-1", "ada_no": "A"}},
             {"source": "g-3", "attrs": {"mezar_no": "M-3", "ada_no": ""}}],
        )
        self.assertEqual(outputs["count"], 2)
        self.assertEqual(done["result"]["summary"], "2 boş mezar yeri bulundu.")


if __name__ == "__main__":
    unittest.main()
