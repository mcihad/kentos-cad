#!/usr/bin/env python3
"""The shared cases of `file.export.landxml` (plan/road.md Y10.2, docs/adr/0908 §6; ROAD-IO-05), and the check of a
file KentOS wrote.

The drawing is road_corridor_command_cases.py's (its alignments road_alignment_cases.py's at 40 digits, its terrain
Triangle's Delaunay of 120 points, its profiles road_profile_command_cases.py's); no KentOS code. A case says what the
file holds (bytes and digest aside). `--written` reads the file KentOS wrote
(`KENTOS_WRITE_LANDXML=1 cargo test -p kentos-native-application --test all landxml`): xmllint checks it against the
LandXML 1.2 schema (landxml.org's XSD, fixtures/road/v1/landxml), xml.etree reads it, and every place, PI, direction,
radius, station and PVI is compared with the references.

    uv run --no-project --python 3.11 --with triangle --with numpy --with mpmath --with pyproj \\
        python3 scripts/fixtures/road_landxml_cases.py [--check | --written]
"""

import json
import subprocess
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

import mpmath as mp

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_corridor_command_cases as ccmd  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/file.export.landxml.json"
HERE = ROOT / "fixtures/road/v1/landxml"
XSD = HERE / "LandXML-1.2.xsd"
WRITTEN = HERE / "out/yol.xml"
NS = "{http://www.landxml.org/schema/LandXML-1.2}"
STAMP = "2026-11-03T08:30:00"

completed, failed, step, case = ccmd.completed, ccmd.failed, ccmd.step, ccmd.case
UNCHANGED = ccmd.UNCHANGED
pcmd = ccmd.pcmd


def setup(work="road"):
    base = ccmd.setup([], work=work)
    base["name"] = "Yol alışverişi"
    return base


SETUP = setup()
ROAD = SETUP["settings"]["road"]
ALIGNMENTS = ROAD["alignments"]
PROFILES = ROAD.get("profiles", [])
SURFACES = ROAD["surfaces"]


def surface_unbuilt(name):
    return ("“" + name + "” yüzeyi kurulamıyor: Kaynaklardan yüzey kurulamıyor: kotlu yer üçten az ya da hepsi bir "
            "doğru üstünde.")


def built(a):
    return pcmd.ALIGNED[a["id"]]


def written_alignment(a, profiles=True):
    b = built(a)
    kinds = [e.kind for e in b.els]
    names = [p["name"] for p in PROFILES if p["alignment"] == a["id"]] if profiles else []
    eqs = len(a.get("stationing", {}).get("equations", []))
    return {"id": a["id"], "name": a["name"], "length": float(b.length), "lines": kinds.count("tangent"),
            "curves": kinds.count("arc"), "spirals": kinds.count("spiral"), "equations": eqs, "profiles": names}


def written_surface(s):
    tin = pcmd.TIN
    return {"id": s["id"], "name": s["name"], "points": len(tin.xy), "triangles": len(tin.tris)}


def losses(message, path):
    return {"code": "export_losses", "message": message, "path": path}


def answer(input_):
    """The output and warnings, or a refusal: (result, warnings)."""
    path = input_["path"]
    warnings = []
    surfaces = input_.get("surfaces")
    picked = []
    if surfaces is None:
        picked = [(s, False) for s in SURFACES]
    else:
        for i, n in enumerate(surfaces):
            s = next((x for x in SURFACES if x["id"] == n), None) or \
                next((x for x in SURFACES if x["name"].lower() == n.lower()), None)
            if s is None:
                return failed("unknown_terrain", f"“{n}” kimlikli ya da adlı yüzey projede yok.", f"surfaces[{i}]"), []
            if all(s is not p for p, _ in picked):
                picked.append((s, True))
    names = input_.get("alignments")
    chosen = []
    if names is None:
        chosen = [(a, False) for a in ALIGNMENTS]
    else:
        for i, n in enumerate(names):
            a = next((x for x in ALIGNMENTS if x["id"] == n), None) or \
                next((x for x in ALIGNMENTS if x["name"].lower() == n.lower()), None)
            if a is None:
                return failed("unknown_alignment", f"“{n}” kimlikli ya da adlı aks projede yok.", f"alignments[{i}]"), []
            if all(a is not p for p, _ in chosen):
                chosen.append((a, True))
    stamp = input_.get("stamp")
    if stamp is not None and not good_stamp(stamp):
        return failed("invalid_landxml", f"“{stamp}” bir tarih ve saat değil; YYYY-MM-DDThh:mm:ss yazın.", "stamp"), []
    out = {"path": path, "surfaces": [], "alignments": [], "bytes": "$any", "sha256": "$any"}
    for s, asked in picked:
        if s["id"] != "t-1":
            if asked:
                return failed("invalid_landxml", surface_unbuilt(s["name"]), "surfaces"), []
            warnings.append(losses(f"“{s['name']}” yüzeyi kurulamıyor; yazılmadı.", "surfaces"))
            continue
        out["surfaces"].append(written_surface(s))
    with_profiles = not input_.get("withoutProfiles", False)
    for a, _asked in chosen:
        if "offsetOf" in a:
            warnings.append(losses(f"“{a['name']}” paralel bir aks; LandXML'in paralel aks türü yok, yazılmadı.",
                                   "alignments"))
            continue
        out["alignments"].append(written_alignment(a, with_profiles))
    if with_profiles and names is None:
        ids = [a["id"] for a, _ in chosen if "offsetOf" not in a]
        for p in PROFILES:
            if p["alignment"] not in ids:
                warnings.append(losses(f"“{p['name']}” boykesitinin aksı yazılmadı; boykesit de yazılmadı.", "alignments"))
    if not out["surfaces"] and not out["alignments"]:
        return failed("invalid_landxml", "Yazılacak bir şey yok: seçilen yüzey ya da aks yok."), []
    return completed(out), warnings


def good_stamp(t):
    if "T" not in t:
        return False
    d, h = t.split("T", 1)
    ok = len(d) == 10 and d[4] == "-" and d[7] == "-" and len(h) == 8 and h[2] == ":" and h[5] == ":"
    return ok and all(c.isdigit() for c in (d + h).replace("-", "").replace(":", ""))


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
        return f"$tmp:yol-{next(names)}.xml"
    there = "$fixture:road/v1/landxml/var.xml"
    return [
        ok("Hepsi: yüzey, akslar ve boykesitleri; paralel aks ve kurulamayan yüzey yazılmaz", {"path": tmp(), "stamp": STAMP}),
        ok("Planı: dosya yazılmadan", {"path": tmp(), "stamp": STAMP}, op="plan"),
        ok("Yalnız doğrulama", {"path": tmp()}, op="validate"),
        ok("Yalnız zemin yüzeyi", {"path": tmp(), "surfaces": ["Mevcut zemin"], "alignments": [], "stamp": STAMP}),
        ok("Yüzey kimlikle", {"path": tmp(), "surfaces": ["t-1"], "alignments": [], "stamp": STAMP}),
        ok("Ana aks boykesitsiz", {"path": tmp(), "surfaces": [], "alignments": ["Ana aks"], "withoutProfiles": True,
                                    "stamp": STAMP}),
        ok("Ana aks ve boykesitleri, kimlik ve adla bir kez", {"path": tmp(), "surfaces": [], "alignments": ["g-1", "ana AKS"],
                                                               "stamp": STAMP}),
        ok("Yan aks ve dar kurp", {"path": tmp(), "surfaces": [], "alignments": ["Yan aks", "g-4"], "stamp": STAMP}),
        ok("Seçilen paralel aks uyarıyla, yüzeyle", {"path": tmp(), "surfaces": ["t-1"], "alignments": ["Sağ kenar"],
                                                      "stamp": STAMP}),
        ok("Var olan dosyanın üstüne, planla", {"path": there, "overwrite": True, "stamp": STAMP}, op="plan"),
        ok("Saatsiz: şimdi", {"path": tmp(), "alignments": ["g-2"], "surfaces": []}),
        no("Yol projesi değil", {"path": tmp()},
           failed("not_a_road_project", "Aks yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil."),
           setup("map")),
        no("Boş yol", {"path": " "},
           failed("invalid_file_path", "LandXML'in yolu boş ya da göreli; mutlak bir .xml yolu verin.", "path")),
        no("Göreli yol", {"path": "yol.xml"},
           failed("invalid_file_path", "LandXML'in yolu boş ya da göreli; mutlak bir .xml yolu verin.", "path")),
        no("XML değil", {"path": "$tmp:yol.txt"}, failed("invalid_file_path", "“$tmp:yol.txt” bir .xml dosyası değil.",
                                                         "path")),
        no("Klasörü yok", {"path": "$tmp:yok/yol.xml"},
           failed("invalid_file_path", "“$tmp:yok” klasörü yok; dosya var olan bir klasöre yazılır.", "path")),
        no("Dosya var", {"path": there}, failed("file_exists", f"{there} var; üzerine yazmak için overwrite verin.", "path")),
        no("Olmayan yüzey", {"path": tmp(), "surfaces": ["Yok"]},
           failed("unknown_terrain", "“Yok” kimlikli ya da adlı yüzey projede yok.", "surfaces[0]")),
        no("Olmayan aks", {"path": tmp(), "alignments": ["g-1", "yok"]},
           failed("unknown_alignment", "“yok” kimlikli ya da adlı aks projede yok.", "alignments[1]")),
        no("Okunmayan saat", {"path": tmp(), "stamp": "3 Kasım"},
           failed("invalid_landxml", "“3 Kasım” bir tarih ve saat değil; YYYY-MM-DDThh:mm:ss yazın.", "stamp")),
        no("Yazılacak bir şey yok", {"path": tmp(), "surfaces": [], "alignments": []},
           failed("invalid_landxml", "Yazılacak bir şey yok: seçilen yüzey ya da aks yok.")),
        no("Seçilen yüzey kurulamıyor", {"path": tmp(), "surfaces": ["Kurulamaz"]},
           failed("invalid_landxml", surface_unbuilt("Kurulamaz"), "surfaces")),
    ]


# ── The written file ──────────────────────────────────────────────────────────────────────────────────────────────


def ne(text):
    n, e = [float(v) for v in text.split()]
    return [e, n]


def north_deg(theta):
    d = (90 - mp.degrees(theta)) % 360
    return float(d)


def close(a, b, tol, what):
    if abs(a - b) > tol:
        raise SystemExit(f"{what}: {a} ≠ {b} (±{tol})")


def check_written():
    subprocess.run(["xmllint", "--noout", "--schema", str(XSD), str(WRITTEN)], check=True)
    root = ET.parse(WRITTEN).getroot()
    metric = root.find(f"{NS}Units/{NS}Metric")
    assert metric.get("linearUnit") == "meter" and metric.get("directionUnit") == "decimal degrees"
    # The surface: the same points, and the same triangles as Triangle's Delaunay.
    surfs = root.findall(f"{NS}Surfaces/{NS}Surface")
    assert [s.get("name") for s in surfs] == ["Mevcut zemin"], [s.get("name") for s in surfs]
    pts = {}
    for p in surfs[0].iter(f"{NS}P"):
        n, e, z = [float(v) for v in p.text.split()]
        pts[int(p.get("id"))] = (e, n, z)
    want = {(x, y, z) for x, y, z in pcmd.POINTS}
    assert set(pts.values()) == want, "the points"
    index = {(x, y): i for i, (x, y) in enumerate(pcmd.TIN.xy)}
    faces = {frozenset(index[pts[int(k)][:2]] for k in f.text.split()) for f in surfs[0].iter(f"{NS}F")}
    assert faces == {frozenset(t) for t in pcmd.TIN.tris}, "the triangles"
    # The alignments: g-1, g-2 and g-4; every element against the reference.
    als = root.findall(f"{NS}Alignments/{NS}Alignment")
    names = [a.get("name") for a in als]
    want_names = [a["name"] for a in ALIGNMENTS if "offsetOf" not in a]
    assert names == want_names, names
    checked = 0
    for node, a in zip(als, [a for a in ALIGNMENTS if "offsetOf" not in a]):
        b = built(a)
        km0 = float(pcmd.acmd.ref.km_of(b.regions, 0))
        close(float(node.get("staStart")), km0, 1e-9, f"{a['name']} staStart")
        close(float(node.get("length")), float(b.length), 1e-7, f"{a['name']} length")
        geo = [c for c in node.find(f"{NS}CoordGeom")]
        assert len(geo) == len(b.els), (a["name"], len(geo), len(b.els))
        for g, e in zip(geo, b.els):
            tag = g.tag[len(NS):]
            assert tag == {"tangent": "Line", "arc": "Curve", "spiral": "Spiral"}[e.kind], (tag, e.kind)
            close(float(g.get("staStart")), km0 + float(e.station), 1e-7, f"{e.id} staStart")
            close(float(g.get("length")), float(e.length), 1e-7, f"{e.id} length")
            s0, s1 = ne(g.find(f"{NS}Start").text), ne(g.find(f"{NS}End").text)
            p0, p1 = e.point(0), e.end()
            for u, v, w in ((s0, p0, "Start"), (s1, p1, "End")):
                close(u[0], float(v[0]), 1e-6, f"{e.id} {w} x")
                close(u[1], float(v[1]), 1e-6, f"{e.id} {w} y")
            th0, th1 = e.theta(0), e.theta(e.length)
            d = float(g.get("dir") if tag == "Line" else g.get("dirStart"))
            close(min(abs(d - north_deg(th0)), 360 - abs(d - north_deg(th0))), 0, 1e-7, f"{e.id} direction")
            if tag == "Curve":
                k = e.k0
                close(float(g.get("radius")), float(1 / abs(k)), 1e-7, f"{e.id} radius")
                assert g.get("rot") == ("cw" if k < 0 else "ccw")
                c = ne(g.find(f"{NS}Center").text)
                cx, cy = p0[0] - mp.sin(e.theta0) / k, p0[1] + mp.cos(e.theta0) / k
                close(c[0], float(cx), 1e-6, f"{e.id} centre x")
                close(c[1], float(cy), 1e-6, f"{e.id} centre y")
            if tag in ("Curve", "Spiral") and g.find(f"{NS}PI") is not None:
                pi = ne(g.find(f"{NS}PI").text)
                # Where the end tangents meet, at 40 digits.
                c0, sn0, c1, sn1 = mp.cos(th0), mp.sin(th0), mp.cos(th1), mp.sin(th1)
                cross = c0 * sn1 - sn0 * c1
                t = ((p1[0] - p0[0]) * sn1 - (p1[1] - p0[1]) * c1) / cross
                close(pi[0], float(p0[0] + t * c0), 1e-6, f"{e.id} PI x")
                close(pi[1], float(p0[1] + t * sn0), 1e-6, f"{e.id} PI y")
            if tag == "Spiral":
                for attr, k in (("radiusStart", e.k0), ("radiusEnd", e.k1)):
                    v = g.get(attr)
                    if k == 0:
                        assert v == "INF", (e.id, attr, v)
                    else:
                        close(float(v), float(1 / abs(k)), 1e-7, f"{e.id} {attr}")
            checked += 1
        # The profiles: the PVIs at their km, and the curves as the profile reference makes them.
        for prof, p in zip(node.findall(f"{NS}Profile"), [p for p in PROFILES if p["alignment"] == a["id"]]):
            assert prof.get("name") == p["name"]
            rows = list(prof.find(f"{NS}ProfAlign"))
            assert len(rows) == len(p["pvis"]), (p["name"], len(rows))
            curves = {c["pvi"]: c for c in pcmd.report(p)["curves"]}
            for r, q in zip(rows, p["pvis"]):
                s, z = [float(v) for v in r.text.split()]
                close(s, km0 + q["station"], 1e-9, f"{p['name']} {q['id']} km")
                close(z, q["elevation"], 0, f"{p['name']} {q['id']} elevation")
                tag = r.tag[len(NS):]
                if q.get("radius"):
                    assert tag == "CircCurve"
                    close(float(r.get("length")), curves[q["id"]]["length"], 1e-7, f"{q['id']} circle length")
                elif q.get("length"):
                    assert tag == "ParaCurve" and float(r.get("length")) == q["length"]
                elif q.get("lengthIn") or q.get("lengthOut"):
                    assert tag == "UnsymParaCurve"
                else:
                    assert tag == "PVI", (q["id"], tag)
    print(f"{WRITTEN.name}: şemaya uygun; {len(pts)} nokta, {len(faces)} üçgen, {checked} aks elemanı, "
          f"{len(PROFILES)} boykesit başvuruyla aynı")


def main():
    if "--written" in sys.argv:
        check_written()
        return
    data = {"format": "kentos.command-cases", "version": 1, "command": "file.export.landxml", "commandVersion": 1,
            "title": "LandXML dışa aktar",
            "note": "scripts/fixtures/road_landxml_cases.py yazar; akslar road_alignment_cases.py'nin, yüzey "
                    "Triangle'ın, boykesitler road_profile_command_cases.py'nin başvurusundandır; KentOS kodu yok. "
                    "Dosyanın içeriği --written ile xmllint (LandXML 1.2 şeması) ve başvurularla denetlenir.",
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
