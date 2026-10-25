#!/usr/bin/env python3
"""The shared cases of the CAD → GIS derivation (docs/adr/0530, plan/gis.md CBS-16), written without KentOS code:

  fixtures/gis-derivation/v1/chords.json        the chords of arcs, circles and bulged edges: how many, how far from the arc
  fixtures/commands/v1/cad.gis.derivation.json  `cad.gis.derivation` against the derived dataset `ada.gpkg` and its edited copies

    uv run --no-project --with mpmath python3 scripts/fixtures/cad_gis_derivation_cases.py           # writes both
    uv run --no-project --with mpmath python3 scripts/fixtures/cad_gis_derivation_cases.py --check   # compares

The chord rule is ADR 0530 §2: an arc of radius r and sweep θ is cut into n chords of one angle, the least n whose
sagitta 2·r·sin²(θ/(4n)) is within the tolerance and whose angle is at most a quarter turn; a chord's sagitta is the
farthest it comes from the arc. Here n is found by a search upward from the quarter-turn count with mpmath at 50
digits (the code solves it in closed form and steps up), the deviation is the sagitta at 50 digits, rounded once.
A bulged edge's radius is L(1 + b²)/(4|b|) and its sweep 4·atan(|b|).

The command cases are the statuses of rows (current, stale, edited, source gone, not exported) the ADR defines, each
built from what a case does to the drawing (`gis_derivation_drawing.py`) or from the edited copy of the dataset
(`gis_derivation_files.py` makes them from `ada.gpkg` with OGR: an attribute, a geometry and a text changed in the
GIS, a row deleted, a row's id changed). The file's size and SHA-256 are hashlib's; the sagitta of each table's
largest arc is the one above. A table's digest and the rows' `kentos_digest` are KentOS's: what the cases check is
what they mean (a changed source is stale, a changed look is not), not their value.
"""
import hashlib
import json
import math
import sqlite3
import sys
from pathlib import Path

from mpmath import atan, ceil, mp, mpf, pi, sin

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import gis_derivation_drawing as dd  # noqa: E402

mp.dps = 50
ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/gis-derivation/v1"
CHORDS = DIR / "chords.json"
CASES = ROOT / "fixtures/commands/v1/cad.gis.derivation.json"
TOL = 1e-4
ANY = "$any"


# ── The chord rule ──────────────────────────────────────────────────────────

def sagitta(r, step):
    return 2 * r * sin(abs(step) / 4) ** 2


def chord_count(r, theta, tol):
    """The least n with sagitta(r, θ/n) ≤ tol and θ/n ≤ π/2, searched upward from the quarter-turn count."""
    r, theta, tol = mpf(r), abs(mpf(theta)), mpf(tol)
    lo = max(1, int(ceil(theta / (pi / 2))))
    ok = lambda n: sagitta(r, theta / n) <= tol  # noqa: E731
    if ok(lo):
        return lo
    hi = lo * 2
    while not ok(hi):
        hi *= 2
    while hi - lo > 1:  # sagitta falls as n grows
        mid = (lo + hi) // 2
        if ok(mid):
            hi = mid
        else:
            lo = mid
    return hi


CAP = 1 << 16  # ADR 0530 §2: no arc is cut into more chords than this; its deviation then says it is over the tolerance


def arc_row(name, r, theta, tol):
    needed = chord_count(r, theta, tol)
    n = min(needed, CAP)
    return {"name": name, "r": r, "sweep": theta, "tolerance": tol, "chords": n, "capped": needed > CAP,
            "deviation": float(sagitta(mpf(r), mpf(theta) / n))}


def bulge_arc(a, b, bulge):
    chord = mp.sqrt((mpf(b[0]) - a[0]) ** 2 + (mpf(b[1]) - a[1]) ** 2)
    bl = mpf(bulge)
    r = chord * (1 + bl * bl) / (4 * abs(bl))
    k = (1 - bl * bl) / (4 * bl)
    c = ((mpf(a[0]) + b[0]) / 2 - (mpf(b[1]) - a[1]) * k, (mpf(a[1]) + b[1]) / 2 + (mpf(b[0]) - a[0]) * k)
    return r, 4 * atan(bl), c


def bulge_row(name, a, b, bulge, tol):
    r, theta, c = bulge_arc(a, b, bulge)
    n = chord_count(r, abs(theta), tol)
    return {"name": name, "a": list(a), "b": list(b), "bulge": bulge, "tolerance": tol, "chords": n,
            "deviation": float(sagitta(r, abs(theta) / n)), "r": float(r), "center": [float(c[0]), float(c[1])],
            "sweep": float(theta)}


def path_row(name, pts, bulges, closed, tol):
    """A path: the points the chords make (an arc's n - 1 inside points, every vertex), and the farthest deviation."""
    n = len(pts)
    edges = n if closed else n - 1
    count, dev = 0, mpf(0)
    for i in range(edges):
        bl = bulges[i] if bulges and i < len(bulges) else 0
        count += 1
        if abs(bl) > 1e-12:
            r, theta, _ = bulge_arc(pts[i], pts[(i + 1) % n], bl)
            m = chord_count(r, abs(theta), tol)
            count += m - 1
            dev = max(dev, sagitta(r, abs(theta) / m))
    if not closed:
        count += 1
    return {"name": name, "pts": [list(p) for p in pts], "bulges": bulges, "closed": closed, "tolerance": tol,
            "count": count, "deviation": float(dev)}


def chords():
    arcs = []
    for r in (0.01, 0.5, 2.5, 18.75, 60.0, 1000.0):
        for tol in (1e-6, 1e-4, 1e-3, 0.05):
            arcs.append(arc_row(f"daire r={r} tolerans={tol}", r, 2 * math.pi, tol))
    for r, theta in ((60.0, 1.2), (5.0, math.pi), (100.0, 0.001), (3.0, 3 * math.pi / 2)):
        arcs.append(arc_row(f"yay r={r} süpürme={theta}", r, theta, 1e-4))
    arcs.append(arc_row("tolerans yarıçaptan büyük: dörde kesilir", 0.5, 2 * math.pi, 1.0))
    bulges = []
    for bl in (0.5, -0.5, 1.0, 0.3, -0.4, 0.2, 2.0):
        bulges.append(bulge_row(f"kenar kabarık={bl}", (487000.0, 4420000.0), (487030.0, 4420000.0), bl, 1e-4))
    bulges.append(bulge_row("küçük kenar ince tolerans", (0.0, 0.0), (0.4, 0.3), 0.7, 1e-6))
    paths = [
        path_row("düz yol", [(0.0, 0.0), (4.0, 0.0), (4.0, 4.0)], None, False, 1e-3),
        path_row("düz halka", [(0.0, 0.0), (4.0, 0.0), (4.0, 4.0)], None, True, 1e-3),
        path_row("iki yaylı açık yol", [(0.0, 0.0), (20.0, 0.0), (40.0, 10.0), (60.0, 10.0)], [0.3, 0, -0.4], False, 1e-4),
        path_row("yaylı kapalı halka", [(0.0, 0.0), (20.0, 0.0), (20.0, 20.0)], [0.2, 0, 0], True, 1e-4),
        path_row("kapanış kenarı yaylı", [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)], [0, 0, 0, 0.5], True, 1e-4),
    ]
    return {"format": "kentos.gis-derivation-chords", "version": 1,
            "note": "ADR 0530 §2. Zinciri sayısı ve sapma mpmath ile 50 basamakta (scripts/fixtures/cad_gis_derivation_cases.py), "
                    "KentOS kodu olmadan.",
            "arcs": arcs, "bulges": bulges, "paths": paths}


# ── The command cases ───────────────────────────────────────────────────────

DRAWING = dd.drawing()
ENTS = {e["id"]: e for e in DRAWING["entities"]}
LAYER_OF = {e["id"]: e["layerId"] for e in DRAWING["entities"]}

# The dataset's tables in the order the file has them, and each row's source (a row's fid is its place).
TABLES = [
    {"table": "Parsel", "role": "layer", "layer": "parsel", "slots": [1, 2, 3, 4]},
    {"table": "blok_Bina", "role": "blocks", "slots": [15, 16]},
    {"table": "Bina", "role": "layer", "layer": "bina", "slots": [17]},
    {"table": "Yol", "role": "layer", "layer": "yol", "slots": [5, 6, 7, 8]},
    {"table": "kentos_annotations", "role": "annotations", "slots": [14]},
    {"table": "Çizim", "role": "layer", "layer": "cizim", "slots": [9, 10, 11, 12, 13]},
]


def deviation_of(e):
    """The deviation a row's source gets at the default tolerance (m): an arc's sagitta exactly; an ellipse, a curve and
    a NURBS the bound the algorithm keeps; its own points 0."""
    k = e["kind"]
    if k in ("ellipse", "spline", "nurbs"):
        return TOL
    arcs = []
    if k == "circle":
        arcs.append(sagitta(mpf(e["r"]), 2 * pi / chord_count(e["r"], 2 * math.pi, TOL)))
    elif k == "arc":
        th = e["a1"] - e["a0"]
        arcs.append(sagitta(mpf(e["r"]), mpf(th) / chord_count(e["r"], th, TOL)))
    elif k in ("polygon", "polyline"):
        rings = [(e["pts"], e.get("bulges"), k == "polygon" or e.get("closed", False))]
        for h in e.get("holes", []):
            rings.append((h["pts"], h.get("bulges"), True))
        for pts, bulges, closed in rings:
            n = len(pts)
            for i in range(n if closed else n - 1):
                bl = bulges[i] if bulges and i < len(bulges) else 0
                if abs(bl) > 1e-12:
                    a, b = pts[i], pts[(i + 1) % n]
                    r, theta, _ = bulge_arc((a["x"], a["y"]), (b["x"], b["y"]), bl)
                    arcs.append(sagitta(r, abs(theta) / chord_count(r, abs(theta), TOL)))
    return float(max(arcs)) if arcs else 0.0


def max_deviation(t):
    return max(deviation_of(ENTS[s]) for s in t["slots"])


SOURCE = {"name": DRAWING["name"], "project": ANY, "revision": "0", "srid": 5256}


def fx(name):
    return f"$fixture:gis-derivation/v1/{name}"


def file_facts(name):
    data = (DIR / name).read_bytes()
    return {"file": name, "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}


STALE = "kaynak nesne dışa aktarmadan sonra değişti"
GONE = "kaynak nesne çizimde yok"
NOT_EXPORTED = "nesne dışa aktarmadan sonra yapıldı ya da bu katmana taşındı; türevde satırı yok"


def edited_reason(fields):
    return f"satır GIS'te değiştirilmiş: {', '.join(fields)}; kaynak nesne değişmedi"


def uid_ref(slot, names=None):
    return (names or {}).get(slot, f"$uidOf:{slot}")


class Scenario:
    """What a case did: the rows by source slot that are not current, and the objects with no row."""

    def __init__(self):
        self.status = {}  # slot -> (status, fields, reason)
        self.missing = []  # (table, slot, kind)
        self.uids = {}  # slot -> a captured uid name, for objects that are gone
        self.absent = set()  # slots whose row is not in the file (a row deleted in the GIS)
        self.row_uids = {}  # slot -> the id the file's row holds, when it is not the source's

    def stale(self, *slots):
        for s in slots:
            self.status[s] = ("stale", [], STALE)
        return self

    def gone(self, *slots):
        for s in slots:
            self.status[s] = ("source_gone", [], GONE)
        return self

    def edited(self, slot, fields):
        self.status[slot] = ("edited", fields, edited_reason(fields))
        return self

    def without_row(self, *slots):
        self.absent.update(slots)
        return self

    def not_exported(self, table, slot, kind):
        self.missing.append((table, slot, kind))
        return self

    def output(self, name="ada.gpkg", limit=100, same=True, source=None):
        tables, entries = [], []
        for t in TABLES:
            row = {"table": t["table"], "role": t["role"]}
            if "layer" in t:
                row["layer"] = t["layer"]
            counts = {"current": 0, "stale": 0, "edited": 0, "sourceGone": 0, "notExported": 0}
            rows = 0
            for fid, slot in enumerate(t["slots"], start=1):
                if slot in self.absent:
                    continue
                rows += 1
                st, fields, reason = self.status.get(slot, ("current", [], ""))
                key = {"current": "current", "stale": "stale", "edited": "edited", "source_gone": "sourceGone"}[st]
                counts[key] += 1
                if st != "current":
                    uid = self.row_uids.get(slot) or uid_ref(slot, self.uids)
                    e = {"table": t["table"], "fid": fid, "uid": uid, "status": st, "kind": ENTS[slot]["kind"]}
                    if fields:
                        e["fields"] = fields
                    e["reason"] = reason
                    entries.append(e)
            for table, slot, kind in self.missing:
                if table == t["table"]:
                    counts["notExported"] += 1
                    entries.append({"table": table, "uid": uid_ref(slot), "status": "not_exported", "kind": kind,
                                    "reason": NOT_EXPORTED})
            row.update({"rows": rows, **counts, "overTolerance": 0, "tolerance": TOL,
                        "maxDeviation": max_deviation(t)})
            tables.append(row)
        rank = {"stale": 0, "edited": 1, "source_gone": 2, "not_exported": 3}
        entries.sort(key=lambda e: rank[e["status"]])
        total = len(entries)
        listed = entries[:limit]
        sums = {k: sum(t[k] for t in tables) for k in ("current", "stale", "edited", "sourceGone", "notExported")}
        out = {**file_facts(name), "algorithm": "kentos.chords/1", "tolerance": TOL, "source": source or SOURCE,
               "sameProject": same, "tables": tables, **sums, "entries": listed, "unlisted": total - len(listed),
               "revision": "$current"}
        return out

    def warnings(self):
        stale = sum(1 for st, _, _ in self.status.values() if st == "stale")
        gone = sum(1 for st, _, _ in self.status.values() if st == "source_gone")
        edited = sum(1 for st, _, _ in self.status.values() if st == "edited")
        missing = len(self.missing)
        out = []
        if stale + gone + missing > 0:
            out.append({"code": "derivation_stale",
                        "message": f"Türev eski: {stale} satırın kaynağı dışa aktarmadan sonra değişti, "
                                   f"{gone} satırın kaynağı silindi, {missing} nesnenin satırı yok. "
                                   "file.export.gis ile yeniden dışa aktarın.", "path": "path"})
        if edited > 0:
            out.append({"code": "derivation_edited",
                        "message": f"{edited} satır GIS'te değiştirilmiş. Kaynak olan çizimdeki nesneler "
                                   "değişmedi: değişikliği cad.gis.takeback ile kaynağa alın ya da cad.gis.detach ile tablonun bağını koparın.",
                        "path": "path"})
        return out


def step(op, input, result, command=None, expect=None, within=None, note=None):
    s = {"op": op}
    if command:
        s["command"] = command
    if input is not None:
        s["input"] = input
    s["result"] = result
    if within is not None:
        s["within"] = within
    if expect is not None:
        s["expect"] = expect
    if note:
        s["note"] = note
    return s


def done():
    return {"status": "completed", "output": ANY, "warnings": ANY}


def run(command, input):
    return step("execute", input, done(), command=command)


def check_step(sc, name="ada.gpkg", limit=None, **kw):
    inp = {"path": fx(name)}
    if limit is not None:
        inp["limit"] = limit
    return step("execute", inp,
                {"status": "completed", "output": sc.output(name, limit if limit is not None else 100, **kw),
                 "warnings": sc.warnings()}, within=1e-12, expect={"revision": "same"})


def refused(inp, code, path="path", name_op="execute"):
    return step(name_op, inp, {"status": "failed", "error": {"code": code, "message": ANY, "path": path}},
                expect={"revision": "same"})


def case(name, steps, setup=None):
    c = {"name": name, "steps": steps}
    if setup is not None:
        c["setup"] = setup
    return c


def commands():
    set_attrs = lambda slot, attrs: run("cad.entities.set", {"uids": [f"$uidOf:{slot}"], "attrs": attrs, "operation": "attributes"})  # noqa: E731
    cases = []
    cases.append(case("güncel türev: bütün satırlar kaynağıyla aynı, listelenecek bir şey yok", [check_step(Scenario())]))
    cases.append(case("doğrulama aynı denetimi yazmadan yapar", [
        step("validate", {"path": fx("ada.gpkg")}, {"status": "completed", "output": None, "warnings": []},
             expect={"revision": "same"})]))
    cases.append(case("plan ile yürütme aynı cevabı verir, çizim değişmez", [
        step("plan", {"path": fx("ada.gpkg")},
             {"status": "completed", "output": Scenario().output(), "warnings": []}, within=1e-12,
             expect={"revision": "same", "canUndo": False})]))
    cases.append(case("öznitelik değişen kaynak: satırı eski", [
        set_attrs(2, {"parsel_no": "9"}), check_step(Scenario().stale(2))]))
    cases.append(case("taşınan kaynak: satırı eski", [
        run("cad.entities.transform", {"uids": ["$uidOf:2"], "transform": {"kind": "move", "dx": 5.0, "dy": 0.0}}),
        check_step(Scenario().stale(2))]))
    cases.append(case("yarıçapı değişen daire: satırı eski, öbürleri güncel", [
        run("cad.entities.edit", {"operation": "properties", "changes": [
            {"kind": "update", "uid": "$uidOf:4", "geometry": {"kind": "circle", "c": {"x": dd.E + 20, "y": dd.N + 40}, "r": 3.0}}]}),
        check_step(Scenario().stale(4))]))
    cases.append(case("silinen kaynak: satırının kaynağı çizimde yok", [
        {"op": "captureUid", "id": 17, "as": "bina"},
        run("cad.entities.delete", {"uids": ["$uid:bina"]}),
        check_step(_with_uid(Scenario().gone(17), 17, "$uid:bina"))]))
    cases.append(case("silinen yazı: ek tablosunun satırının kaynağı yok", [
        {"op": "captureUid", "id": 14, "as": "yazi"},
        run("cad.entities.delete", {"uids": ["$uid:yazi"]}),
        check_step(_with_uid(Scenario().gone(14), 14, "$uid:yazi"))]))
    cases.append(case("dışa aktarmadan sonra yapılan alan: türevde satırı yok", [
        run("cad.polygon.create", {"layerId": "parsel", "pts": [
            {"x": dd.E + 90, "y": dd.N}, {"x": dd.E + 100, "y": dd.N}, {"x": dd.E + 100, "y": dd.N + 10}]}),
        check_step(Scenario().not_exported("Parsel", 20, "polygon"))]))
    cases.append(case("dışa aktarmadan sonra yazılan yazı: yazı tablosunda satırı yok", [
        run("cad.entities.create", {"layerId": "yazi", "objects": [
            {"geometry": {"kind": "text", "p": {"x": dd.E + 6, "y": dd.N + 38}, "text": "Yeni", "height": 2.0, "rotation": 0.0}}]}),
        check_step(Scenario().not_exported("kentos_annotations", 20, "text"))]))
    cases.append(case("dışa aktarmadan sonra yerleştirilen blok: blok tablosunda satırı yok", [
        run("cad.blocks.insert", {"name": "Bina", "at": {"x": dd.E + 70, "y": dd.N + 5}, "layerId": "bina",
                                   "attributes": {"NO": "B-3"}}),
        check_step(Scenario().not_exported("blok_Bina", 20, "insert"))]))
    cases.append(case("bloğun öznitelik tanımı değişince yerleştirmeleri eski", [
        run("cad.blocks.edit", {"operation": "attributes", "block": dd.BLOCK, "attributes": [
            {"tag": "NO", "prompt": "Bina no", "value": "B-?", "p": {"x": 1, "y": 1}, "height": 0.5, "rotation": 0},
            {"tag": "KAT", "prompt": "Kat", "value": "2", "p": {"x": 1, "y": 2}, "height": 0.5, "rotation": 0}]}),
        check_step(Scenario().stale(15, 16))]))
    cases.append(case("yerleştirmenin özniteliği değişince yalnız o yerleştirme eski", [
        set_attrs(16, {"KAT": "5"}), check_step(Scenario().stale(16))]))
    cases.append(case("bakışı değişen nesne güncel kalır: renk, kalınlık, etiket, çizgi tipi", [
        run("cad.entities.set", {"uids": ["$uidOf:1"], "color": "#E5484D", "operation": "color"}),
        run("cad.entities.set", {"uids": ["$uidOf:5"], "lineWeight": 0.35, "operation": "lineWeight"}),
        run("cad.entities.set", {"uids": ["$uidOf:2"], "label": "P8", "operation": "label"}),
        run("cad.entities.set", {"uids": ["$uidOf:6"], "lineType": "dashed", "operation": "lineType"}),
        check_step(Scenario())]))
    cases.append(case("başka katmana taşınan nesne: eski tablosunda eski, yeni katmanın türevinde satırı yok", [
        run("cad.entities.set", {"uids": ["$uidOf:17"], "layerId": "parsel", "operation": "layer"}),
        check_step(Scenario().stale(17).not_exported("Parsel", 17, "polygon"))]))
    cases.append(case("limit: listelenmeyenler sayılır", [
        set_attrs(1, {"parsel_no": "70"}), set_attrs(2, {"parsel_no": "80"}),
        check_step(Scenario().stale(1, 2), limit=1)]))
    cases.append(case("limit 0: hiçbir satır listelenmez", [
        set_attrs(1, {"parsel_no": "70"}), check_step(Scenario().stale(1), limit=0)]))
    edited = Scenario().edited(2, ["ada"]).edited(5, ["geometry"]).edited(14, ["text"])
    cases.append(case("GIS'te değişen satırlar: öznitelik, geometri ve yazı; kaynak değişmedi", [
        check_step(edited, name="ada-duzenli.gpkg")]))
    cases.append(case("GIS'te değişen satırın kaynağı da değişmişse satır eski sayılır", [
        set_attrs(2, {"parsel_no": "9"}),
        check_step(Scenario().stale(2).edited(5, ["geometry"]).edited(14, ["text"]), name="ada-duzenli.gpkg")]))
    cases.append(case("GIS'te silinen satır: kaynağın satırı yok", [
        check_step(Scenario().without_row(3).not_exported("Parsel", 3, "record"), name="ada-eksik.gpkg")]))
    cases.append(case("kimliği değişen satır: kaynağı yok, nesnenin satırı yok", [
        check_step(_foreign(), name="ada-yabanci.gpkg")]))
    cases.append(case("başka çizim: hiçbir satırın kaynağı yok", [
        check_step(_everything_gone(), same=False)], setup=_other_drawing()))
    cases.append(case("yol mutlak değil", [refused({"path": "ada.gpkg"}, "invalid_file_path")]))
    cases.append(case("yol klasör", [refused({"path": fx("")}, "invalid_file_path")]))
    cases.append(case("dosya yok", [refused({"path": fx("yok.gpkg")}, "file_not_found")]))
    cases.append(case("GeoPackage olmayan dosya okunamaz", [refused({"path": fx("drawing.json")}, "import_unreadable")]))
    cases.append(case("türetim kaydı olmayan GeoPackage", [
        refused({"path": "$fixture:geopackage/v1/qgis.gpkg"}, "not_a_derivation")]))
    # A database's derivative (ADR 0634 §3): an address carries no password; with no PostGIS provider nothing is read.
    cases.append(case("veritabanı adresi parolayla verilmez", [
        refused({"path": "postgresql://kentos:gizli@127.0.0.1/turev"}, "invalid_address")]))
    cases.append(case("PostGIS sağlayıcısı olmayan ortamda veritabanı okunamaz", [
        refused({"path": "postgresql://kentos@127.0.0.1:1/turev"}, "source_unreachable")]))
    # The policy: a derivative's table is not bound, read into a layer or written back; taken in, it says what it is.
    cases.append(case("türevin tablosu yeni katmana bağlanmaz", [
        step("execute", {"edit": {"kind": "link", "path": fx("ada.gpkg"), "table": "Parsel"}},
             {"status": "failed", "error": {"code": "derived_dataset", "message": ANY, "path": "edit.table"}},
             command="cad.sources.edit", expect={"revision": "same"})]))
    cases.append(case("türevin tablosu var olan katmana bağlanmaz", [
        step("execute", {"edit": {"kind": "link", "path": fx("ada.gpkg"), "table": "Parsel", "layer": "parsel"}},
             {"status": "failed", "error": {"code": "derived_dataset", "message": ANY, "path": "edit.table"}},
             command="cad.sources.edit", expect={"revision": "same"})]))
    link = {"id": "kaynak-1", "kind": "geopackage", "path": fx("ada.gpkg"), "table": "Parsel", "layer": "parsel",
            "revision": "0" * 64, "digest": "0" * 64, "srid": 5256, "writable": True}
    linked = _linked_drawing(link)
    for direction in ("write", "read"):
        cases.append(case(f"önceden bağlı türev tablosuna {'yazılmaz' if direction == 'write' else 'okunmaz'}", [
            step("execute", {"layer": "parsel", "direction": direction},
                 {"status": "failed", "error": {"code": "derived_dataset", "message": ANY, "path": "layer"}},
                 command="cad.sources.sync", expect={"revision": "same"})], setup=linked))
    cases.append(case("türev aynı çizime alınamaz: nesnelerin kimlikleri çizimde", [
        step("execute", {"path": fx("ada.gpkg")},
             {"status": "failed", "error": {"code": "entity_exists", "message": ANY, "path": "path"}},
             command="file.import.gpkg", expect={"revision": "same"})]))
    cases.append(case("türev başka çizime alınınca ne olduğunu söyler: yaylar kirişlerdir", [
        step("execute", {"path": fx("ada.gpkg"), "tables": ["Yol"]},
             {"status": "completed", "output": ANY,
              "warnings": [{"code": "derived_import", "message": (
                  "Alınan tablolar çizimden türetilmiş GIS verisi: “Yol” (kaynak “Ada 101 türevi”, 0 sürümü, tolerans 0.0001 m). "
                  "Geometrileri kaynak yayların kirişleridir; yay olarak değil çoklu çizgi ve alan olarak alındı. "
                  "Kaynak nesneler hâlâ onların çizimindedir."), "path": "path"}]},
             command="file.import.gpkg", expect={"revision": "changed"})], setup=_without_entities()))
    return cases


def _foreign():
    sc = Scenario().gone(8).not_exported("Yol", 8, "polyline")
    sc.row_uids[8] = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d9999"
    return sc


def file_uids():
    """The ids ada.gpkg's rows hold, by table and fid: read from the file's kentos_uids with sqlite3."""
    c = sqlite3.connect(f"file:{DIR / 'ada.gpkg'}?mode=ro", uri=True)
    out = {(t, fid): uid for t, fid, uid in c.execute("SELECT table_name, fid, uid FROM kentos_uids")}
    c.close()
    return out


def _with_uid(sc, slot, name):
    sc.uids[slot] = name
    return sc


def _everything_gone():
    """Every row of a drawing that has none of the objects: their ids are the file's (no object to ask)."""
    sc = Scenario()
    ids = file_uids()
    for t in TABLES:
        for fid, s in enumerate(t["slots"], start=1):
            sc.gone(s)
            sc.row_uids[s] = ids[(t["table"], fid)]
    return sc


def _other_drawing():
    d = json.loads(json.dumps(DRAWING))
    d["name"] = "Başka çizim"
    d["entities"] = []
    return d


def _without_entities():
    d = json.loads(json.dumps(DRAWING))
    d["entities"] = []
    return d


def _linked_drawing(link):
    d = json.loads(json.dumps(DRAWING))
    d["settings"]["sources"] = [link]
    return d


def cases_file():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.gis.derivation", "commandVersion": 1,
            "title": "GIS türevini denetle: dışa aktarılan GeoPackage'ın satırları kaynağı olan nesnelerle",
            "note": "ADR 0530. Çizim fixtures/gis-derivation/v1/drawing.json; türev ada.gpkg ve GIS'te değiştirilmiş kopyaları "
                    "(ada-duzenli, ada-eksik, ada-yabanci: scripts/fixtures/gis_derivation_files.py OGR'la yapar ve denetler). "
                    "Dosyanın boyu ve SHA-256'sı hashlib'in; her tablonun en büyük sapması yayların kesin sagitta'sı "
                    "(mpmath, 50 basamak); satırın ve tablonun özetleri KentOS'un, durumlar anlamıyla denetlenir. "
                    "Bağımsız başvuru: scripts/fixtures/cad_gis_derivation_cases.py.",
            "setup": DRAWING, "cases": commands()}


def texts():
    return {CHORDS: json.dumps(chords(), ensure_ascii=False, indent=1) + "\n",
            CASES: json.dumps(cases_file(), ensure_ascii=False, indent=1) + "\n"}


def main():
    out = texts()
    if "--check" in sys.argv:
        for path, text in out.items():
            if not path.exists() or path.read_text() != text:
                sys.exit(f"{path} is not what the script writes; run it without --check and read the difference")
        print(f"{len(commands())} command cases, {len(chords()['arcs'])} arcs, {len(chords()['bulges'])} bulges, "
              f"{len(chords()['paths'])} paths ok")
        return
    for path, text in out.items():
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)
        print(f"wrote {path}")


if __name__ == "__main__":
    main()
