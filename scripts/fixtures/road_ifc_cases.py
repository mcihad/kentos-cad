#!/usr/bin/env python3
"""The shared cases of `file.export.ifc` (plan/road.md Y10.3, docs/adr/0908 §8; ROAD-IO-06), and the check of a file
KentOS wrote.

The drawing is road_landxml_cases.py's (its alignments road_alignment_cases.py's at 40 digits, its profiles
road_profile_command_cases.py's); no KentOS code. A case says what the file holds (bytes and digest aside).
`--written` reads the file KentOS wrote (`KENTOS_WRITE_IFC=1 cargo test -p kentos-native-application --test all
ifc_files`) with IfcOpenShell: the IFC4X3_ADD2 schema and its EXPRESS rules; the horizontal and vertical business logic
against the references; every curve segment against the one IfcOpenShell's alignment API maps from that business logic;
and the composite and gradient curves, as IfcOpenShell's geometry kernel evaluates them every 5 m, against the
references' places and heights.

    uv run --no-project --python 3.11 --with triangle --with numpy --with mpmath --with pyproj \\
        python3 scripts/fixtures/road_ifc_cases.py [--check]
    uv run --no-project --python 3.11 --with triangle --with numpy --with mpmath --with pyproj --with ifcopenshell \\
        --with pytest --with lark python3 scripts/fixtures/road_ifc_cases.py --written
"""

import json
import math
import sys
from pathlib import Path

import mpmath as mp

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_landxml_cases as lx  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/file.export.ifc.json"
WRITTEN = ROOT / "fixtures/road/v1/ifc/out/yol.ifc"
STAMP = "2026-11-03T10:00:00"

completed, failed, step, case, pcmd = lx.completed, lx.failed, lx.step, lx.case, lx.pcmd
UNCHANGED = lx.UNCHANGED
ALIGNMENTS, PROFILES, SETUP = lx.ALIGNMENTS, lx.PROFILES, lx.SETUP


def vertical_segments(p):
    """The profile's pieces of some length: the grades between curves and the curves (an asymmetric one two)."""
    curves = sorted(pcmd.report(p)["curves"], key=lambda c: c["start"][0])
    s, n = p["pvis"][0]["station"], 0
    for c in curves:
        if c["start"][0] > s + 1e-9:
            n += 1
        n += 2 if c["kind"] == "asymmetric" else 1
        s = c["end"][0]
    return n + (1 if p["pvis"][-1]["station"] > s + 1e-9 else 0)


def written(a, profiles=True):
    kinds = [e.kind for e in lx.built(a).els]
    mine = [p for p in PROFILES if p["alignment"] == a["id"]] if profiles else []
    return {"id": a["id"], "name": a["name"], "lines": kinds.count("tangent"), "arcs": kinds.count("arc"),
            "clothoids": kinds.count("spiral"), "referents": 1 + len(a.get("stationing", {}).get("equations", [])),
            "profiles": [p["name"] for p in mine], "verticalSegments": sum(vertical_segments(p) for p in mine)}


def losses(message):
    return {"code": "export_losses", "message": message, "path": "alignments"}


def answer(input_):
    names = input_.get("alignments")
    chosen = []
    if names is None:
        chosen = list(ALIGNMENTS)
    else:
        for i, n in enumerate(names):
            a = next((x for x in ALIGNMENTS if x["id"] == n), None) or \
                next((x for x in ALIGNMENTS if x["name"].lower() == n.lower()), None)
            if a is None:
                return failed("unknown_alignment", f"“{n}” kimlikli ya da adlı aks projede yok.", f"alignments[{i}]"), []
            if a not in chosen:
                chosen.append(a)
    stamp = input_.get("stamp")
    if stamp is not None and not lx.good_stamp(stamp):
        return failed("invalid_ifc", f"“{stamp}” bir tarih ve saat değil; YYYY-MM-DDThh:mm:ss yazın.", "stamp"), []
    warnings, out = [], []
    for a in chosen:
        if "offsetOf" in a:
            warnings.append(losses(f"“{a['name']}” paralel bir aks; IFC'ye yazılmadı."))
            continue
        out.append(written(a, not input_.get("withoutProfiles", False)))
    if not out:
        return failed("invalid_ifc", "Yazılacak bir şey yok: seçilen aks yok."), []
    return completed({"path": input_["path"], "schema": "IFC4X3_ADD2", "alignments": out, "crs": "EPSG:5254",
                      "bytes": "$any", "sha256": "$any"}), warnings


def ok(name, input_, op="execute"):
    result, warnings = answer(input_)
    if op == "validate" and result["status"] == "completed":
        result = completed(None)
    result["warnings"] = warnings
    return case(name, [step(op, input_, result, UNCHANGED)])


def no(name, input_, result, setup_=None):
    return case(name, [step("execute", input_, result, UNCHANGED)], setup_)


def cases():
    names = iter(range(1, 100))

    def tmp():
        return f"$tmp:yol-{next(names)}.ifc"
    there = "$fixture:road/v1/ifc/var.ifc"
    return [
        ok("Bütün akslar boykesitleriyle; paralel aks yazılmaz", {"path": tmp(), "stamp": STAMP}),
        ok("Planı", {"path": tmp(), "stamp": STAMP}, op="plan"),
        ok("Doğrulama", {"path": tmp()}, op="validate"),
        ok("Ana aks: iki boykesit, biri alt aksta", {"path": tmp(), "alignments": ["Ana aks"], "stamp": STAMP}),
        ok("Ana aks boykesitsiz", {"path": tmp(), "alignments": ["g-1"], "withoutProfiles": True, "stamp": STAMP}),
        ok("Yan aks ve dar kurp", {"path": tmp(), "alignments": ["Yan aks", "Dar kurp"], "stamp": STAMP}),
        ok("Kimlik ve adla bir kez", {"path": tmp(), "alignments": ["g-2", "yan AKS"], "stamp": STAMP}),
        ok("Seçilen paralel aks uyarıyla", {"path": tmp(), "alignments": ["Sağ kenar", "Dar kurp"], "stamp": STAMP}),
        ok("Var olan dosyanın üstüne, planla", {"path": there, "overwrite": True, "stamp": STAMP}, op="plan"),
        ok("Saatsiz: şimdi", {"path": tmp(), "alignments": ["Dar kurp"]}),
        no("Yol projesi değil", {"path": tmp()},
           failed("not_a_road_project", "Aks yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil."),
           lx.setup("map")),
        no("Boş yol", {"path": ""},
           failed("invalid_file_path", "IFC'nin yolu boş ya da göreli; mutlak bir .ifc yolu verin.", "path")),
        no("Göreli yol", {"path": "yol.ifc"},
           failed("invalid_file_path", "IFC'nin yolu boş ya da göreli; mutlak bir .ifc yolu verin.", "path")),
        no("IFC değil", {"path": "$tmp:yol.xml"}, failed("invalid_file_path", "“$tmp:yol.xml” bir .ifc dosyası değil.", "path")),
        no("Klasörü yok", {"path": "$tmp:yok/yol.ifc"},
           failed("invalid_file_path", "“$tmp:yok” klasörü yok; dosya var olan bir klasöre yazılır.", "path")),
        no("Dosya var", {"path": there}, failed("file_exists", f"{there} var; üzerine yazmak için overwrite verin.", "path")),
        no("Olmayan aks", {"path": tmp(), "alignments": ["Ana aks", "yok"]},
           failed("unknown_alignment", "“yok” kimlikli ya da adlı aks projede yok.", "alignments[1]")),
        no("Okunmayan saat", {"path": tmp(), "stamp": "2026-11-03 10:00"},
           failed("invalid_ifc", "“2026-11-03 10:00” bir tarih ve saat değil; YYYY-MM-DDThh:mm:ss yazın.", "stamp")),
        no("Yalnız paralel aks: yazılacak bir şey yok", {"path": tmp(), "alignments": ["Sağ kenar"]},
           failed("invalid_ifc", "Yazılacak bir şey yok: seçilen aks yok.")),
        no("Boş liste", {"path": tmp(), "alignments": []}, failed("invalid_ifc", "Yazılacak bir şey yok: seçilen aks yok.")),
    ]


# ── The written file ──────────────────────────────────────────────────────────────────────────────────────────────


def close(a, b, tol, what):
    if abs(a - b) > tol:
        raise SystemExit(f"{what}: {a} ≠ {b} (±{tol})")


def angle_gap(a, b):
    d = (a - b) % (2 * math.pi)
    return min(d, 2 * math.pi - d)


def nested(entity):
    return [o for rel in entity.IsNestedBy for o in rel.RelatedObjects]


def same_segment(ours, theirs, what):
    """Our curve segment against the one IfcOpenShell maps from the same design parameters."""
    assert ours.Transition == "DISCONTINUOUS" or ours.Transition.startswith("CONT"), what
    for k in ("SegmentStart", "SegmentLength"):
        close(getattr(ours, k).wrappedValue, getattr(theirs, k).wrappedValue, 1e-9, f"{what} {k}")
    for i in (0, 1):
        close(ours.Placement.Location.Coordinates[i], theirs.Placement.Location.Coordinates[i], 1e-9, f"{what} place")
        close(ours.Placement.RefDirection.DirectionRatios[i], theirs.Placement.RefDirection.DirectionRatios[i], 1e-12,
              f"{what} direction")
    p, q = ours.ParentCurve, theirs.ParentCurve
    assert p.is_a() == q.is_a(), (what, p.is_a(), q.is_a())
    if p.is_a("IfcCircle"):
        close(p.Radius, q.Radius, 1e-9, f"{what} radius")
        for i in (0, 1):
            close(p.Position.Location.Coordinates[i], q.Position.Location.Coordinates[i], 1e-9, f"{what} centre")
    if p.is_a("IfcClothoid"):
        close(p.ClothoidConstant, q.ClothoidConstant, 1e-9, f"{what} A")
    if p.is_a("IfcPolynomialCurve"):
        for x, y in zip(p.CoefficientsY, q.CoefficientsY):
            close(x, y, 1e-12, f"{what} coefficient")


def check_written():
    import ifcopenshell
    import ifcopenshell.geom
    import ifcopenshell.validate
    import numpy as np
    from ifcopenshell import ifcopenshell_wrapper as w
    from ifcopenshell.api.alignment._map_alignment_horizontal_segment import _map_alignment_horizontal_segment
    from ifcopenshell.api.alignment._map_alignment_vertical_segment import _map_alignment_vertical_segment

    f = ifcopenshell.open(str(WRITTEN))
    assert f.schema_identifier == "IFC4X3_ADD2", f.schema_identifier
    log = ifcopenshell.validate.json_logger()
    ifcopenshell.validate.validate(f, log, express_rules=True)
    assert not log.statements, log.statements[:5]
    crs = f.by_type("IfcProjectedCRS")
    assert [c.Name for c in crs] == ["EPSG:5254"], crs
    settings = ifcopenshell.geom.settings()
    tops = [a for a in f.by_type("IfcAlignment") if not a.Decomposes or a.Decomposes[0].RelatingObject.is_a("IfcProject")]
    want = [a for a in ALIGNMENTS if "offsetOf" not in a]
    assert [a.Name for a in tops] == [a["name"] for a in want], [a.Name for a in tops]
    checked = places = heights = 0
    for node, a in zip(tops, want):
        b = lx.built(a)
        layouts = nested(node)
        horizontal = next(x for x in layouts if x.is_a("IfcAlignmentHorizontal"))
        segs = nested(horizontal)
        assert len(segs) == len(b.els) + 1, (a["name"], len(segs))
        foot = next(r for r in node.Representation.Representations if r.RepresentationIdentifier == "FootPrint")
        composite = foot.Items[0]
        assert len(composite.Segments) == len(segs)
        for k, (seg, e) in enumerate(zip(segs, b.els)):
            p = seg.DesignParameters
            assert p.PredefinedType == {"tangent": "LINE", "arc": "CIRCULARARC", "spiral": "CLOTHOID"}[e.kind]
            start = e.point(0)
            close(p.StartPoint.Coordinates[0], float(start[0]), 1e-9, f"{e.id} x")
            close(p.StartPoint.Coordinates[1], float(start[1]), 1e-9, f"{e.id} y")
            close(angle_gap(p.StartDirection, float(e.theta0)), 0, 1e-12, f"{e.id} direction")
            close(p.SegmentLength, float(e.length), 1e-9, f"{e.id} length")
            for attr, kk in (("StartRadiusOfCurvature", e.k0), ("EndRadiusOfCurvature", e.k1)):
                close(getattr(p, attr), 0.0 if kk == 0 else float(1 / kk), 1e-6, f"{e.id} {attr}")
            theirs, _ = _map_alignment_horizontal_segment(f, seg)
            same_segment(composite.Segments[k], theirs, e.id)
            checked += 1
        last = segs[-1].DesignParameters
        assert last.SegmentLength == 0 and last.PredefinedType == "LINE"
        # The composite curve as IfcOpenShell evaluates it, every 5 m, against the alignment.
        fn = w.map_shape(settings, composite)
        ev = w.function_item_evaluator(settings, fn)
        close(fn.end(), float(b.length), 1e-6, f"{a['name']} curve length")
        s = 0.0
        while s <= float(b.length):
            m = np.array(ev.evaluate(s))
            e = next(x for x in reversed(b.els) if x.station <= s + 1e-12)
            q = e.point(mp.mpf(s) - e.station)
            close(math.hypot(m[0, 3] - float(q[0]), m[1, 3] - float(q[1])), 0, 1e-6, f"{a['name']} km {s}")
            places += 1
            s += 5.0
        # The verticals: the first on the alignment, the others on child alignments; their curves' heights.
        mine = [p for p in PROFILES if p["alignment"] == a["id"]]
        verticals = [x for x in layouts if x.is_a("IfcAlignmentVertical")]
        children = [c for rel in node.IsDecomposedBy for c in rel.RelatedObjects]
        for c in children:
            verticals += [x for x in nested(c) if x.is_a("IfcAlignmentVertical")]
        assert [v.Name for v in verticals] == [p["name"] for p in mine], ([v.Name for v in verticals], mine)
        gradients = [r.Items[0] for r in node.Representation.Representations if r.RepresentationIdentifier == "Axis"]
        for c in children:
            gradients += [r.Items[0] for r in c.Representation.Representations]
        for v, p, g in zip(verticals, mine, gradients):
            vsegs = nested(v)
            assert len(vsegs) == vertical_segments(p) + 1, (p["name"], len(vsegs))
            for k, seg in enumerate(vsegs[:-1]):
                theirs, _ = _map_alignment_vertical_segment(f, seg)
                same_segment(g.Segments[k], theirs, f"{p['name']} {k}")
            fn = w.map_shape(settings, g)
            ev = w.function_item_evaluator(settings, fn)
            height = pcmd.solve(p)[1]
            s = p["pvis"][0]["station"]
            while s <= p["pvis"][-1]["station"]:
                z = float(np.array(ev.evaluate(s))[2, 3])
                close(z, float(height(s)[0]), 1e-6, f"{p['name']} km {s}")
                heights += 1
                s += 5.0
        # The km at the start.
        referents = [r for r in nested(node) if r.is_a("IfcReferent")]
        stations = [p.NominalValue.wrappedValue for r in referents for d in r.IsDefinedBy
                    for p in d.RelatingPropertyDefinition.HasProperties if p.Name == "Station"]
        close(stations[0], float(pcmd.acmd.ref.km_of(b.regions, 0)), 1e-9, f"{a['name']} km")
    print(f"{WRITTEN.name}: IFC4X3_ADD2 şeması ve EXPRESS kuralları temiz; {checked} yatay parça iş mantığı ve IfcOpenShell'in "
          f"eşlemesiyle aynı; {places} yer ve {heights} kot IfcOpenShell'in değerlendirmesiyle başvurudan en çok 1e-6 m")


def main():
    if "--written" in sys.argv:
        check_written()
        return
    data = {"format": "kentos.command-cases", "version": 1, "command": "file.export.ifc", "commandVersion": 1,
            "title": "IFC dışa aktar",
            "note": "scripts/fixtures/road_ifc_cases.py yazar; akslar road_alignment_cases.py'nin, boykesitler "
                    "road_profile_command_cases.py'nin başvurusundandır; KentOS kodu yok. Dosyanın içeriği --written ile "
                    "IfcOpenShell'le (şema, EXPRESS kuralları, eşleme, eğrilerin değerlendirmesi) denetlenir.",
            "setup": SETUP, "cases": cases()}
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text(encoding="utf-8") != text:
            raise SystemExit(f"{OUT} güncel değil: python3 {Path(__file__).name}")
        print(f"{OUT.name}: {len(data['cases'])} durum güncel")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.name}: {len(data['cases'])} durum")


if __name__ == "__main__":
    main()
