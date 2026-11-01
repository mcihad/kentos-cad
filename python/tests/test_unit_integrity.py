"""CAD-ADV-02: unit correction preserves live recipes, not just their drawn output."""
from pathlib import Path
import unittest
from kentos import cad

ROOT = Path(__file__).resolve().parents[2]


class UnitIntegrity(unittest.TestCase):
    def test_hatch_model_spacing_and_paper_book_keep_their_distinct_units(self):
        import json
        import warnings
        raw = json.loads((ROOT / "fixtures/kcad/v2/hatches.json").read_text())
        raw["version"] = 1
        raw.pop("uids", None)
        raw["settings"]["srid"] = 0
        raw["settings"]["drawingUnit"] = "m"
        import copy
        lines = copy.deepcopy(next(e for e in raw["entities"] if e["kind"] == "hatch"))
        lines["id"] = 99
        lines["pattern"] = {"type": "lines", "angle": 45., "spacing": .2}
        raw["entities"].append(lines)
        doc = cad.Document.from_bytes(json.dumps(raw).encode())
        sheet = json.loads((ROOT / "fixtures/kcad/v2/sheets.json").read_text())["sheets"]["book"]["sheets"][0]
        cad.sheets.edit(doc, ops=[dict(op="addSheet", sheet=sheet)])
        book = cad.sheets.set(doc, details=True).book.to_json()
        before = doc.to_bytes()
        hatches = {r.uid: r.entity.to_json() for r in doc.entities() if r.entity.kind == "hatch"}
        with warnings.catch_warnings():
            warnings.simplefilter("ignore", cad.CommandWarning)
            result = cad.units.change.run(doc, cad.UnitsChange(from_="m", to="mm", geometry="scale"))
        self.assertIn("sheets_kept", [w.code for w in result.warnings])
        self.assertEqual(cad.sheets.set(doc, details=True).book.to_json(), book)
        self.assertEqual(doc.settings.plot_scale, 500.)
        for uid, old in hatches.items():
            new = doc.entity(uid).entity.to_json()
            self.assertAlmostEqual(new["ring"][1]["x"], old["ring"][1]["x"] / 1000)
            self.assertEqual(new["pattern"]["angle"], old["pattern"]["angle"])
            if old["pattern"]["type"] in ("lines", "cross"):
                self.assertAlmostEqual(new["pattern"]["spacing"], old["pattern"]["spacing"] / 1000)
            if "scale" in old["pattern"]:
                self.assertAlmostEqual(new["pattern"]["scale"], old["pattern"]["scale"] / 1000)
        after = doc.to_bytes()
        self.assertEqual(cad.Document.from_bytes(after).to_bytes(), after)
        doc.undo(); self.assertEqual(doc.to_bytes(), before)
        doc.redo(); self.assertEqual(doc.to_bytes(), after)

    def test_corrected_array_remains_editable_and_round_trips(self):
        for layout in [cad.GridArrayLayout(rows=2, cols=2, dx=5, dy=4),
                       cad.PolarArrayLayout(center=(0, 0), count=4, fill=360, rotate=True)]:
            with self.subTest(layout=layout):
                doc = cad.Document.new("Dizi", workspace="cad", drawing_unit="m")
                line = cad.line.create(doc, layer_id=doc.active_layer, a=(0, 0), b=(1, 0))
                cad.entities.array(doc, uids=[line.uid], layout=layout, associative=True)
                before = doc.to_bytes()
                cad.units.change(doc, from_="m", to="mm", geometry="scale")
                array = doc.arrays()[0]
                self.assertFalse(any(i.get("kept", False) for i in array["items"]),
                                 "a unit correction is not a manual item move")
                if array["layout"]["kind"] == "grid":
                    self.assertEqual((array["layout"]["dx"], array["layout"]["dy"]), (.005, .004))
                scaled = doc.to_bytes()
                self.assertEqual(cad.Document.from_bytes(scaled).arrays(), doc.arrays())
                doc.undo(); self.assertEqual(doc.to_bytes(), before)
                doc.redo(); self.assertEqual(doc.to_bytes(), scaled)

    def test_corrected_wall_recipe_keeps_members_and_follows_later_edits(self):
        doc = cad.Document.new("Duvar", workspace="cad", drawing_unit="m", plot_scale=50)
        wall = cad.walls.create(doc, layer_id=doc.active_layer,
            walls=[cad.NewWall(pts=[(10, 20), (20, 20)], width=.2, closed=False)]).uids[0]
        cad.dimensionsets.create(doc, layer_id=doc.active_layer, wall=wall, at=(15, 19),
                                rows=["overall"])
        before = doc.to_bytes()
        group = cad.dimensionsets.query(doc).sets[0]
        ids = [m.uid for m in group.members]
        height = group.height
        cad.units.change(doc, from_="m", to="mm", geometry="scale")
        group = cad.dimensionsets.query(doc).sets[0]
        self.assertEqual([m.uid for m in group.members], ids)
        self.assertEqual((group.at.x, group.at.y, group.height, group.gap_mm), (.015, .019, height / 1000, 7.5))
        scaled = doc.to_bytes()
        reopened = cad.Document.from_bytes(scaled)
        self.assertEqual(reopened.to_bytes(), scaled)
        doc.undo(); self.assertEqual(doc.to_bytes(), before)
        doc.redo(); self.assertEqual(doc.to_bytes(), scaled)
        # A later host edit must still regenerate the same members, without a protection refusal.
        cad.entities.transform(reopened, uids=[wall], transform=cad.MoveTransform(dx=.1, dy=.2))
        moved = cad.dimensionsets.query(reopened).sets[0]
        self.assertEqual([m.uid for m in moved.members], ids)
        self.assertFalse(any(m.a.broken is not cad.UNSET for m in moved.members))

    def test_dynamic_lengths_rules_lookup_expressions_and_overrides_scale_together(self):
        import copy
        import json
        raw = json.loads((ROOT / "fixtures/kcad/v2/dynamic-blocks.json").read_text())
        raw["version"] = 1
        raw.pop("uids", None)
        doc = cad.Document.from_bytes(json.dumps(raw).encode())
        block = doc.blocks()[0]
        dynamic = copy.deepcopy(raw["blocks"][0]["dynamic"])
        dynamic["parameters"].extend([
            {"kind": "linear", "name": "İkinci", "from": {"x": 0, "y": 0},
             "to": {"x": 2.05, "y": 0}, "rule": {"expression": "2 * @Genişlik + 0.25"}},
            {"kind": "angle", "name": "Açı", "center": {"x": 1, "y": 2}, "base": .5},
        ])
        cad.blocks.edit(doc, operation="dynamic", block=block.id, dynamic=cad.DynamicBlock.from_json(dynamic))
        inserts = [r.uid for r in doc.entities() if r.entity.kind == "insert"]
        cad.blocks.parameters(doc, operation="set", uids=inserts, values={"Genişlik": 1., "Açı": .75})
        before = doc.to_bytes()
        cad.units.change(doc, from_="m", to="mm", geometry="scale")
        out = cad.blocks.parameters(doc, operation="get", uids=inserts)
        for item in out.inserts:
            self.assertAlmostEqual(item.values["Genişlik"], .001)
            self.assertAlmostEqual(item.values["İkinci"], .00225)
            self.assertEqual(item.values["Açı"], .75)
        definition = doc.blocks()[0].to_json()
        width = definition["dynamic"]["parameters"][0]
        self.assertAlmostEqual(width["to"]["x"], .0009)
        self.assertAlmostEqual(width["rule"]["min"], .0006)
        self.assertAlmostEqual(width["rule"]["step"], .00005)
        scaled = doc.to_bytes()
        doc.undo(); self.assertEqual(doc.to_bytes(), before)
        doc.redo(); self.assertEqual(doc.to_bytes(), scaled)
        reopened = cad.Document.from_bytes(scaled)
        # Changing the lookup after conversion exercises the expression, not just the old stored output.
        cad.blocks.parameters(reopened, operation="set", uids=inserts, values={"Tip": "Dar"})
        for item in cad.blocks.parameters(reopened, operation="get", uids=inserts).inserts:
            self.assertAlmostEqual(item.values["Genişlik"], .0007)
            self.assertAlmostEqual(item.values["İkinci"], .00165)
            self.assertEqual(item.values["Açı"], .75)

    def test_physical_room_and_one_metre_survive_unit_input_clipboard_and_reopen(self):
        import warnings
        for source_unit, width, depth in [("mm", 4000, 3000), ("m", 4, 3), ("in", 4/.0254, 3/.0254)]:
            for target_unit in ["mm", "m", "in"]:
                with self.subTest(source=source_unit, target=target_unit):
                    source = cad.Document.new("Kaynak", workspace="cad", drawing_unit=source_unit)
                    w, d = source.units.to_metres(width), source.units.to_metres(depth)
                    room = cad.polygon.create(source, layer_id=source.active_layer,
                                              pts=[(0, 0), (w, 0), (w, d), (0, d)])
                    line = cad.line.create(source, layer_id=source.active_layer, a=(0, 0),
                                           b=(source.units.to_metres(1 / source.units.to_metres(1)), 0))
                    clip = cad.entities.copy(source, uids=[room.uid, line.uid], base=(0, 0))
                    target = cad.Document.new("Hedef", workspace="cad", drawing_unit=target_unit)
                    with warnings.catch_warnings():
                        warnings.simplefilter("ignore", cad.CommandWarning)
                        cad.entities.paste(target, clip=clip, at=(500000, 4400000))
                    measures = [target.measure(r.uid) for r in target.entities()]
                    self.assertAlmostEqual(next(m.area for m in measures if m.area), 12.)
                    self.assertAlmostEqual(next(m.length for m in measures if m.area is None), 1.)
                    saved = target.to_bytes()
                    reopened = cad.Document.from_bytes(saved)
                    self.assertEqual(reopened.to_bytes(), saved)
                    target.undo(); self.assertEqual(target.info().objects, 0)
                    target.redo(); self.assertEqual(target.to_bytes(), saved)

    def test_ambiguous_open_requires_source_unit_and_explicit_override_is_applied_once(self):
        source = ROOT / "fixtures/formats/v1/dxf-units/unitless.dxf"
        with self.assertRaises(cad.HostError) as unknown:
            cad.Document.open(source)
        self.assertEqual(unknown.exception.code, "source_unit_required")
        chosen = cad.Document.open(source, file_unit="cm")
        self.assertEqual(chosen.units.drawing.value, "cm")
        wall = next(r.entity for r in chosen.entities() if r.entity.kind == "line")
        self.assertEqual(wall.b.x - wall.a.x, 40.)
        # Declared mm is respected on ordinary open; an explicit override has the same result as import.
        mm = ROOT / "fixtures/formats/v1/dxf-units/mm.dxf"
        direct = cad.Document.open(mm, file_unit="cm")
        imported = cad.Document.new("Hedef", workspace="cad", drawing_unit="in")
        imported.import_dxf(mm, file_unit="cm")
        direct_line = next(r.entity for r in direct.entities() if r.entity.kind == "line")
        imported_line = next(r.entity for r in imported.entities() if r.entity.kind == "line")
        self.assertEqual((direct_line.a, direct_line.b), (imported_line.a, imported_line.b))

    def test_annotation_representations_and_moved_values_follow_unit_correction(self):
        import json
        raw = json.loads((ROOT / "fixtures/kcad/v2/annotative.json").read_text())
        raw["version"] = 1
        raw.pop("uids", None)
        raw["settings"]["srid"] = 0
        raw["settings"]["workspace"] = "cad"
        raw["settings"].pop("drawingUnit", None)
        text_entity = next(e for e in raw["entities"] if e["kind"] == "text")
        text_entity["boxWidth"] = .5
        text_entity["layout"] = {"columns": {"count": 2, "gap": 1., "height": 1.}}
        leader = next(e for e in raw["entities"] if e["kind"] == "leader")
        leader["arms"] = [{"pts": [{"x": 500001., "y": 4400001.}]}]
        dimension = next(e for e in raw["entities"] if e["kind"] == "dimension")
        dimension["textShift"] = {"along": .8, "across": -.2}
        dimension["options"] = {"round": .001, "tolerance": {"kind": "deviation", "upper": .01, "lower": .005}}
        doc = cad.Document.from_bytes(json.dumps(raw).encode())
        before = doc.to_bytes()
        cad.units.change(doc, from_="m", to="mm", geometry="scale")
        entities = {r.entity.kind: r.entity.to_json() for r in doc.entities()}
        self.assertEqual(entities["text"]["layout"]["columns"], {"count": 2, "gap": 1., "height": .001})
        self.assertEqual(entities["leader"]["arms"][0]["pts"][0], {"x": 500.001, "y": 4400.001})
        text = entities["text"]["annotative"]
        self.assertEqual(text["at"], 100.)
        self.assertEqual(text["scales"][0]["scale"], 50.)
        self.assertEqual(text["scales"][0]["point"], {"x": 500.0005, "y": 4400.00025})
        dimension = entities["dimension"]
        self.assertEqual(dimension["textShift"], {"along": .0008, "across": -.0002})
        self.assertEqual(dimension["annotative"]["scales"][0]["offset"], .0003)
        self.assertEqual(dimension["annotative"]["scales"][0]["textShift"]["along"], .0004)
        self.assertEqual(dimension["options"]["round"], .000001)
        self.assertEqual(dimension["options"]["tolerance"]["upper"], .00001)
        self.assertEqual(entities["leader"]["annotative"]["scales"][0]["rotation"], 15.)
        scaled = doc.to_bytes()
        self.assertEqual(cad.Document.from_bytes(scaled).to_bytes(), scaled)
        doc.undo(); self.assertEqual(doc.to_bytes(), before)
        doc.redo(); self.assertEqual(doc.to_bytes(), scaled)

    def source_drawing(self, unit):
        doc = cad.Document.new("4x3 oda " + unit, workspace="cad", drawing_unit=unit, plot_scale=50)
        cad.polygon.create(doc, layer_id=doc.active_layer, pts=[(0, 0), (4, 0), (4, 3), (0, 3)])
        line = cad.line.create(doc, layer_id=doc.active_layer, a=(0, 4), b=(1, 4))
        block = cad.blocks.define(doc, name="1m-" + unit, base=(0, 4), uids=[line.uid],
                                  units={"mm": "in", "m": "mm", "in": "m"}[unit])
        import warnings
        with warnings.catch_warnings():
            warnings.simplefilter("ignore", cad.CommandWarning)
            cad.blocks.insert(doc, layer_id=doc.active_layer, block=block.block, at=(5, 0))
        return doc

    def test_source_open_import_block_placement_and_reopen_share_physical_units(self):
        import tempfile
        import warnings
        with tempfile.TemporaryDirectory() as folder, warnings.catch_warnings():
            warnings.simplefilter("ignore", cad.CommandWarning)
            for source_unit in ["mm", "m", "in"]:
                path = Path(folder) / (source_unit + ".dxf")
                cad.file.export.dxf(self.source_drawing(source_unit), path=str(path))
                opened = cad.Document.open(path)
                self.assertEqual(opened.units.drawing.value, source_unit)
                # Direct CAD open retains closed polylines as lines. Check their physical
                # outline independently rather than silently converting their entity kind.
                room = next(r.entity for r in opened.entities() if r.entity.kind in ("polyline", "polygon"))
                points = list(room.pts)
                area = abs(sum(a.x*b.y - a.y*b.x for a, b in zip(points, points[1:] + points[:1]))) / 2
                self.assertAlmostEqual(area, 12.)
                for target_unit in ["mm", "m", "in"]:
                    with self.subTest(source=source_unit, target=target_unit):
                        doc = cad.Document.new("Aktarım", workspace="cad", drawing_unit=target_unit)
                        doc.import_dxf(path)
                        block = next(b for b in doc.blocks() if b.name == "1m-" + source_unit)
                        line = next(e for e in block.entities if e.kind == "line")
                        self.assertAlmostEqual(line.b.x - line.a.x, 1.)
                        made = cad.blocks.insert(doc, layer_id=doc.active_layer, block=block.id, at=(10, 5))
                        self.assertAlmostEqual(doc.entity(made.uid).entity.scale, 1.)
                        before = doc.to_bytes()
                        doc.undo(); doc.redo(); self.assertEqual(doc.to_bytes(), before)
                        self.assertEqual(cad.Document.from_bytes(before).to_bytes(), before)

    def test_unit_correction_preserves_a_recipe_on_a_multiedge_wall(self):
        doc = cad.Document.new("Çok kenarlı duvar", workspace="cad", drawing_unit="m", plot_scale=50)
        wall = cad.walls.create(doc, layer_id=doc.active_layer, walls=[cad.NewWall(
            pts=[(10, 20), (20, 20), (20, 25), (10, 25)], width=.2, closed=True)]).uids[0]
        cad.dimensionsets.create(doc, layer_id=doc.active_layer, wall=wall, edge=1,
                                at=(21, 22), rows=["overall"])
        ids = [m.uid for m in cad.dimensionsets.query(doc).sets[0].members]
        cad.units.change(doc, from_="m", to="mm", geometry="scale")
        group = cad.dimensionsets.query(doc).sets[0]
        self.assertEqual([m.uid for m in group.members], ids)
        self.assertEqual(group.edge, 1)
        self.assertIs(group.broken, cad.UNSET)
