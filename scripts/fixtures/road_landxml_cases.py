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

import copy
import json
import subprocess
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

import mpmath as mp

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_corridor_command_cases as ccmd  # noqa: E402
import road_grades  # noqa: E402

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


# Docs/adr/1031: the project says its heights' system, TUDKA99's orthometric heights (Antalya height) by TG-20.
HEIGHTS = {"kind": "orthometric", "verticalSrid": 5775, "geoid": {"name": "TG-20"}}
REGISTRY = ROOT / "fixtures/crs/v1/registry.json"


def setup(work="road", heights=HEIGHTS):
    base = ccmd.setup([], work=work)
    base["name"] = "Yol alışverişi"
    if work == "road" and heights is not None:
        base["settings"]["heights"] = copy.deepcopy(heights)
    return base


def heights_title(h):
    """What the heights are, in the words of the project's settings (docs/adr/0519)."""
    if h["kind"] == "ellipsoidal":
        return "elipsoit yüksekliği"
    if h["kind"] != "orthometric":
        return "bilinmiyor"
    from pyproj import CRS
    t = "ortometrik"
    t += (f", {CRS.from_epsg(h['verticalSrid']).name} (EPSG:{h['verticalSrid']})" if h.get("verticalSrid")
          else ", düşey sistemi belirtilmemiş")
    if h.get("geoid"):
        t += f", geoit {h['geoid']['name']}"
    return t


def coordinate_system(settings):
    """The `CoordinateSystem` a file for the field says (docs/adr/1031), its attributes: the horizontal system's name
    and datum as PROJ's database and the registry name them, the vertical system's as PROJ's database does."""
    from pyproj import CRS
    out = {}
    srid = settings.get("srid", 0)
    if srid:
        out["name"] = CRS.from_epsg(srid).name
        out["epsgCode"] = str(srid)
        reg = json.loads(REGISTRY.read_text(encoding="utf-8"))
        out["horizontalDatum"] = next(x["datum"] for x in reg["systems"] if x["srid"] == srid)
    h = settings.get("heights")
    if h and h["kind"] == "orthometric" and h.get("verticalSrid"):
        v = CRS.from_epsg(h["verticalSrid"])
        out["verticalDatum"] = v.datum.name
        out["verticalCoordinateSystemName"] = v.name
    elif h and h["kind"] == "ellipsoidal":
        out["verticalCoordinateSystemName"] = "ellipsoidal height"
    if h and h["kind"] != "unknown":
        out["desc"] = "Kotlar: " + heights_title(h)
    return out


def unsaid(what, why="projenin kotlarının düşey sistemi söylenmemiş"):
    return (f"{what} kotları bir düşey sisteme bağlı olmalı: {why}. Proje ayarları › Kotlar'da sistemi seçin (BÖHHBÜY: "
            "TUDKA99'a dayalı Helmert ortometrik yükseklik, Antalya height EPSG:5775). Kotlar bir varsayımla sahaya "
            "gönderilmez.")


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
    ] + machine_cases(tmp)


def machine_cases(tmp):
    """Docs/adr/1031: a design for machine control rests on the project's heights, said; a plain file needs it not."""
    mc = {"stamp": STAMP, "machineControl": True}
    what = "Makine kontrolü dosyasının"

    def clouded(vertical):
        s = setup()
        record = {"layer": "zemin", "clouds": ["0192f5a0-7c3e-7d4a-9b1e-000000000001"], "classes": [2],
                  "returns": "all", "cell": 1.0, "read": 120, "kept": 120, "coverage": 1.0, "density": 1.0}
        if vertical is not None:
            record["vertical"] = vertical
        next(t for t in s["settings"]["road"]["surfaces"] if t["id"] == "t-1")["clouds"] = [record]
        return s

    def done(input_):
        result, warnings = answer(input_)
        result["warnings"] = warnings
        return result

    ground = {"surfaces": ["Mevcut zemin"], "alignments": []}
    on_project = {**mc, "path": tmp(), **ground}
    cloud_ok = case("Makine kontrolü: bulut zemini projenin sisteminde",
                    [step("execute", on_project, done(on_project), UNCHANGED)], clouded("EPSG:5775"))
    plain = {"path": tmp(), "stamp": STAMP}
    return [
        ok("Makine kontrolü için: projenin kotları söylenmiş", {**mc, "path": tmp()}),
        case("Kotları söylenmemiş proje: makine kontrolü olmadan yazılır", [step("execute", plain, done(plain), UNCHANGED)],
             setup(heights=None)),
        no("Makine kontrolü: projenin kotları söylenmemiş", {**mc, "path": tmp()},
           failed("field_heights_unknown", unsaid(what)), setup(heights=None)),
        no("Makine kontrolü: ortometrik ama düşey sistemi yok", {**mc, "path": tmp()},
           failed("field_heights_unknown", unsaid(what, "projenin kotları ortometrik ama düşey sistemi belirtilmemiş")),
           setup(heights={"kind": "orthometric"})),
        cloud_ok,
        no("Makine kontrolü: bulut zemininin sistemi bilinmiyor", {**mc, "path": tmp(), **ground},
           failed("unproven_heights", "“Mevcut zemin” yüzeyinin “zemin” katmanındaki bulut noktaları: dosyası düşey "
                  "sistemini söylemiyor ya da alındığında kaydedilmedi. Kotları projenin sisteminde gösterilmeyen yüzey "
                  "makine kontrolüne gönderilmez.", "surfaces"), clouded(None)),
        no("Makine kontrolü: bulut zemini başka sistemde", {**mc, "path": tmp(), **ground},
           failed("unproven_heights", "“Mevcut zemin” yüzeyinin “zemin” katmanındaki bulut noktaları: EGM2008 height "
                  "(EPSG:3855), projenin kotları Antalya height (EPSG:5775). Kotları projenin sisteminde gösterilmeyen "
                  "yüzey makine kontrolüne gönderilmez.", "surfaces"), clouded("EGM2008 height")),
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
    # Docs/adr/1031: the systems, as PROJ's database names them.
    cs = root.find(f"{NS}CoordinateSystem")
    assert cs is not None and dict(cs.attrib) == coordinate_system(SETUP["settings"]), cs.attrib if cs is not None else None
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


# ── road.alignments.import ────────────────────────────────────────────────────────────────────────────────────────

IMPORT_OUT = ROOT / "fixtures/commands/v1/road.alignments.import.json"
AKS = HERE / "aks.xml"
AKS_HIZ = HERE / "aks-hiz.xml"
STEPPED = {"canUndo": True, "canRedo": False, "dirty": True, "revision": "changed"}


def fmt(v):
    """A number as the shortest text that reads back, without an exponent."""
    from decimal import Decimal
    v = float(v)
    if v == 0:
        return "0"
    t = format(Decimal(repr(v)), "f")
    return t.rstrip("0").rstrip(".") if "." in t else t


def pl(p):
    return f"{fmt(p[1])} {fmt(p[0])}"


def meet(p0, th0, p1, th1):
    c0, s0, c1, s1 = mp.cos(th0), mp.sin(th0), mp.cos(th1), mp.sin(th1)
    t = ((p1[0] - p0[0]) * s1 - (p1[1] - p0[1]) * c1) / (c0 * s1 - s0 * c1)
    return [p0[0] + t * c0, p0[1] + t * s0]


def element_xml(e, km0):
    """A reference element (road_alignment_cases.El, 40 digits) as LandXML, written here without KentOS."""
    p0, p1 = e.point(0), e.end()
    th0, th1 = e.theta(0), e.theta(e.length)
    head = f'staStart="{fmt(km0 + e.station)}" length="{fmt(e.length)}"'
    if e.kind == "tangent":
        return f'<Line {head}><Start>{pl(p0)}</Start><End>{pl(p1)}</End></Line>'
    rot = "cw" if (e.k0 + e.k1) < 0 else "ccw"
    if e.kind == "arc":
        k = e.k0
        c = [p0[0] - mp.sin(th0) / k, p0[1] + mp.cos(th0) / k]
        pi = meet(p0, th0, p1, th1)
        return (f'<Curve rot="{rot}" crvType="arc" radius="{fmt(1 / abs(k))}" {head}><Start>{pl(p0)}</Start>'
                f'<Center>{pl(c)}</Center><End>{pl(p1)}</End><PI>{pl(pi)}</PI></Curve>')
    r = lambda k: "INF" if k == 0 else fmt(1 / abs(k))  # noqa: E731
    return (f'<Spiral rot="{rot}" spiType="clothoid" radiusStart="{r(e.k0)}" radiusEnd="{r(e.k1)}" {head}>'
            f'<Start>{pl(p0)}</Start><PI>{pl(meet(p0, th0, p1, th1))}</PI><End>{pl(p1)}</End></Spiral>')


def alignment_xml(name, b, km0, extra="", profile=None):
    els = "".join(element_xml(e, km0) for e in b.els)
    prof = ""
    if profile:
        rows = []
        for q in profile["pvis"]:
            at = f"{fmt(km0 + q['station'])} {fmt(q['elevation'])}"
            if q.get("radius"):
                c = next(c for c in pcmd.report(profile)["curves"] if c["pvi"] == q["id"])
                rows.append(f'<CircCurve length="{fmt(c["length"])}" radius="{fmt(q["radius"])}">{at}</CircCurve>')
            elif q.get("length"):
                rows.append(f'<ParaCurve length="{fmt(q["length"])}">{at}</ParaCurve>')
            elif q.get("lengthIn"):
                rows.append(f'<UnsymParaCurve lengthIn="{fmt(q["lengthIn"])}" lengthOut="{fmt(q["lengthOut"])}">{at}</UnsymParaCurve>')
            else:
                rows.append(f"<PVI>{at}</PVI>")
        prof = f'<Profile name="{profile["name"]}"><ProfAlign name="{profile["name"]}">{"".join(rows)}</ProfAlign></Profile>'
    return (f'<Alignment name="{name}" length="{fmt(b.length)}" staStart="{fmt(km0)}"><CoordGeom>{els}</CoordGeom>'
            f'{extra}{prof}</Alignment>')


def raw(name, body, extra=""):
    return f'<Alignment name="{name}" length="1" staStart="0"><CoordGeom>{body}</CoordGeom>{extra}</Alignment>'


def line(a, b):
    return f"<Line><Start>{pl(a)}</Start><End>{pl(b)}</End></Line>"


def aks_xml():
    g1, g4 = pcmd.ALIGNED["g-1"], pcmd.ALIGNED["g-4"]
    main = next(p for p in PROFILES if p["id"] == "b-1")
    parts = [
        alignment_xml("Ana aks", g1, 1000, profile=main),
        alignment_xml("Dar kurp", g4, 0),
        raw("Kırık aks", line([0, 0], [100, 0]) + line([100, 0], [150, 80])),
        raw("Denklemli aks", line([0, 300], [300, 300]),
            '<StaEquation staInternal="120" staAhead="500" staIncrement="increasing"/>'),
        raw("Bileşik kurp", line([0, 0], [100, 0])
            + '<Curve rot="ccw" radius="100"><Start>0 100</Start><Center>100 100</Center><End>29.289 170.711</End></Curve>'
            + '<Curve rot="ccw" radius="200"><Start>29.289 170.711</Start><Center>170.711 29.289</Center><End>100 200</End></Curve>'),
        raw("Ove klotoid", line([0, 0], [100, 0])
            + '<Spiral rot="ccw" spiType="clothoid" radiusStart="300" radiusEnd="150" length="20"><Start>0 100</Start><PI>0 110</PI><End>1 120</End></Spiral>'
            + '<Curve rot="ccw" radius="150"><Start>1 120</Start><Center>151 120</Center><End>5 150</End></Curve>'),
        raw("Kübik geçiş", line([0, 0], [100, 0])
            + '<Spiral rot="ccw" spiType="cubic" radiusStart="INF" radiusEnd="150" length="20"><Start>0 100</Start><PI>0 110</PI><End>1 120</End></Spiral>'),
        raw("Düzensiz", '<IrregularLine><Start>0 0</Start><End>10 10</End><PntList2D>0 0 10 10</PntList2D></IrregularLine>'),
    ]
    return ('<?xml version="1.0" encoding="UTF-8"?>\n<LandXML xmlns="http://www.landxml.org/schema/LandXML-1.2" '
            'date="2026-11-03" time="09:00:00" version="1.2"><Units><Metric areaUnit="squareMeter" linearUnit="meter" '
            'volumeUnit="cubicMeter" temperatureUnit="celsius" pressureUnit="milliBars"/></Units><Alignments>'
            + "".join(parts) + "</Alignments></LandXML>\n")


def aks_hiz_xml():
    """Docs/adr/0988: Ana aks with a roadway's design speeds, 80 km/h but 50 over its curve's middle, and one of
    250 km/h that is left."""
    g1 = pcmd.ALIGNED["g-1"]
    st = [float(x) for x in g1.reports[0]["stations"]]
    mid = (st[1] + st[2]) / 2
    assert 250 < mid < 350, mid
    speeds = ('<DesignSpeed staStart="1000" staEnd="1250" speed="80"/><DesignSpeed staStart="1250" staEnd="1350" speed="50"/>'
              f'<DesignSpeed staStart="1350" staEnd="{fmt(1000 + float(g1.length))}" speed="80"/>'
              '<DesignSpeed staStart="1000" staEnd="1010" speed="250"/>')
    return ('<?xml version="1.0" encoding="UTF-8"?>\n<LandXML xmlns="http://www.landxml.org/schema/LandXML-1.2" '
            'date="2026-11-06" time="09:00:00" version="1.2"><Units><Metric areaUnit="squareMeter" linearUnit="meter" '
            'volumeUnit="cubicMeter" temperatureUnit="celsius" pressureUnit="milliBars" velocityUnit="kilometersPerHour"/>'
            '</Units><Alignments>' + alignment_xml("Ana aks", g1, 1000) + '</Alignments>'
            '<Roadways><Roadway name="Ana aks" alignmentRefs="Ana aks"><Speeds>' + speeds
            + '</Speeds></Roadway></Roadways></LandXML>\n')


def import_setup(with_alignments=False, work="road"):
    base = ccmd.setup([], work=work)
    base["name"] = "Yol alışverişi"
    if work == "road" and not with_alignments:
        base["settings"]["road"] = {"surfaces": copy.deepcopy(SURFACES)}
    return base


def pi(id_, x, y, radius=0, s_in=0, s_out=0):
    out = {"id": id_, "x": float(x), "y": float(y)}
    if radius:
        out["radius"] = float(radius)
    if s_in:
        out["spiralIn"] = float(s_in)
    if s_out:
        out["spiralOut"] = float(s_out)
    return out


def defined(id_):
    a = next(a for a in ALIGNMENTS if a["id"] == id_)
    return [pi(p["id"], p["x"], p["y"], p.get("radius", 0), p.get("spiralIn", 0), p.get("spiralOut", 0)) for p in a["pis"]]


TAKEN = {
    "Ana aks": (defined("g-1"), 5, ["Proje kotu"]),
    "Dar kurp": (defined("g-4"), 3, []),
    "Kırık aks": ([pi("a", 0, 0), pi("p1", 100, 0), pi("b", 150, 80)], 2, []),
    "Denklemli aks": ([pi("a", 0, 300), pi("b", 300, 300)], 1, []),
}
WHY = {
    "Bileşik kurp": "3. elemanda aralarında aliyman olmayan bileşik ya da ters kurp var; KentOS'un PI'larında her kurp aliymanlar arasındadır",
    "Ove klotoid": "2. eleman aliymandan kurba giden bir klotoid değil (ove klotoid ya da yarıçapı kurbunkinden farklı)",
    "Kübik geçiş": "2. eleman klotoid olmayan bir geçiş eğrisi",
    "Düzensiz": "KentOS'un almadığı elemanlar var (1. eleman (IrregularLine))",
}
ORDER = ["Ana aks", "Dar kurp", "Kırık aks", "Denklemli aks", "Bileşik kurp", "Ove klotoid", "Kübik geçiş", "Düzensiz"]


OURS = {
    "Ana aks": (defined("g-1"), 5, ["Proje kotu", "Kısmi kot"]),
    "Yan aks": (defined("g-2"), 1, ["Yan kot"]),
    "Dar kurp": (defined("g-4"), 3, ["Dar kurbun kotu"]),
}


def taken(names, without_profiles=False, table=None):
    table = table or TAKEN
    out, warnings, n = [], [], 0
    for name in names:
        if name in WHY:
            warnings.append({"code": "landxml_left", "message": f"“{name}” aksı alınmadı: {WHY[name]}.", "path": "alignments"})
            continue
        n += 1
        pis, elements, profiles = table[name]
        out.append({"id": f"g-{n}", "name": name, "pis": pis, "elements": elements, "deviation": 0.0,
                    "profiles": [] if without_profiles else profiles})
    return out, warnings


def import_cases():
    path = f"$fixture:road/v1/landxml/{AKS.name}"
    cases = []

    def ok(name, input_, names, op="execute", without=False, table=None):
        out, warnings = taken(names, without, table)
        result = completed(None if op == "validate" else {"alignments": out, "revision": "$current"})
        result["warnings"] = warnings
        expect = STEPPED if op == "execute" else UNCHANGED
        steps = [{**step(op, input_, result, expect), "within": 1e-6}]
        if op == "execute":
            steps.append({"op": "undo", "returns": "LandXML aks al", "expect": {"canRedo": True}})
        cases.append(case(name, steps))

    ok("Bütün akslar: dördü alınır, dördü nedeniyle kalır", {"path": path}, ORDER)
    ok("Planı", {"path": path}, ORDER, op="plan")
    ok("Doğrulama", {"path": path}, ORDER, op="validate")
    ok("Ana aks boykesitiyle", {"path": path, "alignments": ["Ana aks"]}, ["Ana aks"])
    ok("Ana aks boykesitsiz", {"path": path, "alignments": ["Ana aks"], "withoutProfiles": True}, ["Ana aks"], without=True)
    ok("Dar kurp ve kırık aks", {"path": path, "alignments": ["Dar kurp", "Kırık aks"]}, ["Dar kurp", "Kırık aks"])
    ok("Km denklemi: geri km'si yazılmamış", {"path": path, "alignments": ["Denklemli aks"]}, ["Denklemli aks"])
    ok("Bir kez: aynı ad iki kez", {"path": path, "alignments": ["Kırık aks", "Kırık aks"]}, ["Kırık aks"])
    ok("Alınamayanla birlikte", {"path": path, "alignments": ["Ove klotoid", "Kırık aks"]}, ["Ove klotoid", "Kırık aks"])
    yol = "$fixture:road/v1/landxml/out/yol.xml"
    ok("KentOS'un yazdığı LandXML geri alınır", {"path": yol, "alignments": ["Ana aks", "Dar kurp"]},
       ["Ana aks", "Dar kurp"], table=OURS)
    ok("KentOS'un yazdığı LandXML'in bütün aksları boykesitleriyle", {"path": yol}, ["Ana aks", "Yan aks", "Dar kurp"],
       table=OURS)

    # Docs/adr/0988: a roadway's design speeds: the alignment's the longest stretch's (80), p1's the 50 over its middle;
    # the 250 is left and said.
    hiz = f"$fixture:road/v1/landxml/{AKS_HIZ.name}"
    fast = [{**p, "speed": 50.0} if p["id"] == "p1" else p for p in defined("g-1")]
    ok("Yolun proje hızları: aksın 80, kurbun 50", {"path": hiz}, ["Ana aks"], table={"Ana aks": (fast, 5, [])})
    cases[-1]["steps"][0]["result"]["warnings"].insert(0, {
        "code": "landxml_left", "path": "alignments",
        "message": "“Ana aks” aksının 250 km/sa'lik proje hızı alınmadı: 0'dan büyük, en çok 200 km/sa olmalı."})

    def no(name, input_, result, setup_=None):
        cases.append(case(name, [step("execute", input_, result, UNCHANGED)], setup_))

    no("Yol projesi değil", {"path": path},
       failed("not_a_road_project", "Aks yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil."),
       import_setup(work="map"))
    no("Dosya yok", {"path": "$tmp:yok.xml"}, failed("file_not_found", "$tmp:yok.xml yok; dosyanın yolunu denetleyin.", "path"))
    no("LandXML değil", {"path": "$fixture:road/v1/landxml/LandXML-1.2.xsd"},
       failed("import_unreadable", "$fixture:road/v1/landxml/LandXML-1.2.xsd LandXML olarak okunamadı: kökü LandXML değil.",
              "path"))
    no("Akssız dosya", {"path": "$fixture:road/v1/landxml/var.xml"}, failed("landxml_no_alignment", "Dosyada aks yok.", "path"))
    no("Dosyada olmayan ad", {"path": path, "alignments": ["Ana aks", "Yok"]},
       failed("landxml_no_alignment", "“Yok” adlı aks dosyada yok; dosyanın aksları: "
              + ", ".join(f"“{n}”" for n in ORDER) + ".", "alignments[1]"))
    no("Hiçbiri alınamıyor", {"path": path, "alignments": ["Bileşik kurp", "Düzensiz"]},
       failed("landxml_no_alignment", f"“Bileşik kurp” aksı alınmadı: {WHY['Bileşik kurp']}.", "alignments"))
    no("Yalnız kübik geçiş", {"path": path, "alignments": ["Kübik geçiş"]},
       failed("landxml_no_alignment", f"“Kübik geçiş” aksı alınmadı: {WHY['Kübik geçiş']}.", "alignments"))
    no("Projede aynı adlı aks", {"path": path, "alignments": ["Ana aks"]},
       failed("invalid_alignment", "Projede “Ana aks” adlı bir aks var; dosyanınkini almadan önce birini yeniden adlandırın.",
              "alignments"), import_setup(with_alignments=True))
    no("Okunmayan revizyon", {"path": path, "alignments": ["Kırık aks"], "expectedRevision": "r-0"},
       failed("invalid_revision", "Beklenen sürüm “r-0” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. "
              "Sürümü belgeden ya da komutun planından alın.", "expectedRevision"))
    no("Eski revizyon", {"path": path, "alignments": ["Kırık aks"], "expectedRevision": "99"},
       pcmd.CONFLICT)
    return cases


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
    data = road_grades.fixture(data)
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if (OUT.read_text(encoding="utf-8") != text or AKS.read_text(encoding="utf-8") != aks_xml()
                or not AKS_HIZ.exists() or AKS_HIZ.read_text(encoding="utf-8") != aks_hiz_xml()):
            raise SystemExit(f"{OUT} ya da {AKS} güncel değil: python3 {Path(__file__).name}")
        print(f"{OUT.name}: {len(data['cases'])} durum güncel")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.name}: {len(data['cases'])} durum")
    AKS.write_text(aks_xml(), encoding="utf-8")
    AKS_HIZ.write_text(aks_hiz_xml(), encoding="utf-8")
    data = {"format": "kentos.command-cases", "version": 1, "command": "road.alignments.import", "commandVersion": 1,
            "title": "LandXML'den aks al",
            "note": "scripts/fixtures/road_landxml_cases.py yazar; aks.xml'i başvurunun 40 basamaklı elemanlarından KentOS "
                    "kodu olmadan yazar, beklenen PI'lar aksların tanımındakilerdir (±1e-6).",
            "setup": import_setup(), "cases": import_cases()}
    data = road_grades.fixture(data)
    IMPORT_OUT.write_text(json.dumps(data, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    print(f"{IMPORT_OUT.name}: {len(data['cases'])} durum")


if __name__ == "__main__":
    main()
