#!/usr/bin/env python3
"""The shared cases of `cad.survey.adjust` (Yatay ağ and Kot ağı dengelemesi; docs/adr/0203 §7–§8, 0622 §9).

    python3 scripts/fixtures/survey_adjust_command_cases.py           # writes the file
    python3 scripts/fixtures/survey_adjust_command_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.survey.adjust.json with Python's standard library, no KentOS code. The adjustments are
the independent reference's (`scripts/fixtures/network_adjust_cases.py`, 50 digits with mpmath): the networks and their
answers are read from fixtures/network-adjust/v1/cases.json, its new points' places, heights and standard deviations,
degrees of freedom, m0 and model test, compared within 1e-6. What the command adds to them, from ADR 0203 §8 and the
command's contract:

- a known point given by its name alone is the drawing's point of the name (its label, else its `Ad`, compared the
  Turkish way); a known height so, its z; a new point the drawing has starts where it has it;
- only the new points are written: Yatay ağ moves every drawing point of the name to its adjusted place with the
  vertices of lines, polylines and areas on it (their heights kept), and adds the others on `layerId` as named
  points (label and `Ad` the name, `Tür` “Ağ noktası”); Kot ağı gives every drawing point of the name its height, the
  vertices on it the same, a `Z (m)` it has the height in three decimals, and says the names it has not;
- one undo step named after the window; nothing to write writes nothing;
- the refusals in the contract's order, the adjustment's in the reference's words.

ADR 5300 adds leap-frog setups (atlamalı trigonometrik nivelman): each a height difference from the back point to the
fore point, the fore sight's `S·cos z + (1 − k)·(S·sin z)²/2R − t` less the back sight's (the project's k, 0.13 when it
names none; R 6 371 000 m), its length the two horizontal distances' sum, worked out here with mpmath at 50 digits and
adjusted by the reference's own levelling (network_adjust_cases.levelling). So this script needs mpmath:

    uv run --no-project --python 3.11 --with mpmath python3 scripts/fixtures/survey_adjust_command_cases.py
"""
import importlib.util
import json
import sys
from pathlib import Path

import mpmath as mp

ROOT = Path(__file__).resolve().parents[2]


def _load(name):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(f"{name}.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


NA = _load("network_adjust_cases")
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.survey.adjust.json"
REFERENCE = json.loads((ROOT / "fixtures" / "network-adjust" / "v1" / "cases.json").read_text(encoding="utf-8"))
WITHIN = 1e-6
POINT_KIND = "Ağ noktası"
NETWORK_STEP = "Yatay ağ dengelemesi"
LEVEL_STEP = "Kot ağı dengelemesi"

SETTINGS = {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
            "plotScale": 1000, "workspace": "gis"}


def layer(i, name, locked=False, visible=True, kind="layer"):
    return {"id": i, "name": name, "type": kind, "visible": visible, "locked": locked, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def pt(x, y):
    return {"x": float(x), "y": float(y)}


def named(i, name, x, y, z=None, z_text=None):
    e = {"kind": "point", "id": i, "layerId": "nokta", "attrs": {"Ad": name}, "p": pt(x, y), "label": name}
    if z is not None:
        e["z"] = z
    if z_text is not None:
        e["attrs"]["Z (m)"] = z_text
    return e


# The Hesap windows' scene (fixtures/interaction/v1/network-adjust.kcad) with three layers more for the refusals.
LAYERS = [layer("nokta", "Nokta"), layer("poligon", "Poligon"), layer("hat", "Hat"),
          layer("kilitli", "Kilitli", locked=True), layer("gizli", "Gizli", visible=False),
          layer("grup", "Grup", kind="group")]
ENTITIES = [
    named(1, "K1", 487000.0, 4420000.0),
    named(2, "K2", 487400.0, 4420020.0),
    named(3, "Y1", 487030.02, 4420349.97),
    {"kind": "polyline", "id": 4, "layerId": "hat", "attrs": {},
     "pts": [pt(487000.0, 4420000.0), pt(487030.02, 4420349.97), pt(487400.0, 4420020.0)]},
    named(5, "R1", 487100.0, 4420100.0, z=102.4567),
    named(6, "R2", 487300.0, 4420120.0, z=98.1234),
    named(7, "N1", 487150.0, 4420200.0, z=105.0, z_text="105.000"),
    named(8, "N2", 487250.0, 4420215.0, z=101.78, z_text="101.780"),
    {"kind": "polyline", "id": 9, "layerId": "hat", "attrs": {},
     "pts": [pt(487100.0, 4420100.0), pt(487150.0, 4420200.0)], "zs": [102.4567, 105.0]},
]
BY_ID = {e["id"]: e for e in ENTITIES}
IDS = [e["id"] for e in ENTITIES]
SETUP = {"format": "kentos.document", "version": 1, "name": "Ağ dengelemesi", "settings": SETTINGS,
         "origin": {"x": 487000.0, "y": 4420000.0}, "layers": LAYERS, "activeLayer": "hat",
         "styles": {"items": [], "categories": []}, "entities": ENTITIES}
UNCHANGED = {"ids": IDS, "dirty": False, "revision": "same"}


def reference(start):
    case = next(c for c in REFERENCE["cases"] if c["name"].startswith(start))
    defaults = REFERENCE["defaults"]
    # The project's a priori standard deviations are the defaults: only the reference's cases with them are taken.
    assert case.get("sigma", defaults) == defaults, start
    return case


def upper(s):
    return "".join("İ" if c == "i" else "I" if c == "ı" else c.upper() for c in s)


def key(name):
    return upper(name.strip())


def drawn(name):
    """The drawing's points of the name, in its order."""
    k = key(name)
    return [e for e in ENTITIES if e["kind"] == "point" and key(e.get("label") or e["attrs"].get("Ad", "")) == k]


def wire_known(k, by_name):
    out = {"name": k["name"]}
    if not by_name:
        out["at"] = pt(k["y"], k["x"])
    if k.get("sigma") is not None:
        out["sigma"] = k["sigma"]
    return out


def horizontal_work(case, by_name=False, names=None):
    known = [wire_known(k, by_name) for k in case["known"]]
    if names:
        for k, n in zip(known, names):
            k["name"] = n
    rows = []
    for r in case["rows"]:
        row = {"station": r["station"], "target": r["target"]}
        if r.get("direction") is not None:
            row["direction"] = r["direction"]
        if r.get("distance") is not None:
            row["distance"] = r["distance"]
        rows.append(row)
    return {"kind": "horizontal", "known": known, "rows": rows}


def level_work(case, by_name=False):
    known = []
    for k in case["known"]:
        out = {"name": k["name"]}
        if not by_name:
            out["h"] = k["h"]
        if k.get("sigma") is not None:
            out["sigma"] = k["sigma"]
        known.append(out)
    work = {"kind": "level", "known": known,
            "rows": [{"from": r["from"], "to": r["to"], "dh": r["dh"], "length": r["length"]} for r in case["rows"]]}
    if case.get("levelKind") == "trigonometric":
        work["method"] = "trigonometric"
    return work


def follow(entity, frm, to, z=None, set_z=False):
    """A line's vertices on `frm` moved to `to` (the point editor's rule), their heights kept or given."""
    pts = [dict(p) for p in entity["pts"]]
    zs = list(entity.get("zs") or [None] * len(pts))
    hit = False
    for i, p in enumerate(pts):
        if p["x"] == frm["x"] and p["y"] == frm["y"]:
            pts[i] = pt(to["x"], to["y"])
            if set_z:
                zs[i] = z
            hit = True
    return hit, pts, zs


def kept_record(name, kind, points, expect):
    """ADR 5200: the adjustment as the project keeps it: its name, kind, the revision it was made on (captured before
    the write), its new points as the answer gives them, f, m0 and the test."""
    # The numbers are the answer's (compared within 1e-9 there); here their places are held and the values any, the
    # settings being compared exactly.
    kept_points = [{k: ("$any" if k in ("at", "h", "sigma") else v) for k, v in p_.items()} for p_ in points]
    r = {"name": name, "kind": kind, "revision": "$before", "points": kept_points, "f": expect["f"]}
    if expect.get("m0") is not None:
        r["m0"] = "$any"
    if expect.get("passed") is not None:
        r["passed"] = expect["passed"]
    return r


def keeping(c, keep, kind, before_kept=None):
    """`c` with its write keeping the adjustment under `keep` (ADR 5200): the input's `keep`, the answer's `kept`, the
    project's kept adjustments after (`before_kept` first, one of the name replaced) and before the undo."""
    step = c["steps"][0]
    step["input"]["keep"] = keep
    out = step["result"]["output"]
    out["kept"] = keep.strip()
    record = kept_record(keep.strip(), kind, out["points"], out)
    at = next((i for i, r in enumerate(before_kept or []) if r["name"] == keep.strip()), None)
    after = list(before_kept or [])
    if at is None:
        after.append(record)
    else:
        after[at] = record
    step["expect"]["settings"] = {"surveyAdjustments": after}
    c["steps"].insert(0, {"op": "captureRevision", "as": "before"})
    if len(c["steps"]) > 2:
        c["steps"][2]["expect"]["settings"] = {"surveyAdjustments": before_kept}
    return c


def horizontal_case(name, case, by_name=False, names=None, layer_id="poligon", warnings=(), note=None):
    known_keys = {key(k["name"]) for k in case["known"]}
    expect = case["expect"]
    points, moved, adds = [], [], []
    fields_after = {}
    slot = max(IDS)
    for p in expect["points"]:
        if key(p["name"]) in known_keys:
            continue
        to = pt(p["y"], p["x"])
        points.append({"name": p["name"], "at": to, "sigma": p["sp"]})
        there = drawn(p["name"])
        if not there:
            adds.append((p["name"], to))
        for d in there:
            moved.append(d["id"])
            fields_after[str(d["id"])] = {"p": to}
            for other in ENTITIES:
                if other["kind"] != "polyline":
                    continue
                hit, pts, _ = follow(other, d["p"], to)
                if hit:
                    fields_after[str(other["id"])] = {"pts": pts}
    created = []
    for n, to in adds:
        slot += 1
        created.append(slot)
        fields_after[str(slot)] = {"kind": "point", "layerId": layer_id, "label": n, "p": to,
                                   "attrs": {"Ad": n, "Tür": POINT_KIND}}
    output = {"points": points, "moved": [f"$uidOf:{i}" for i in moved], "created": [f"$uidOf:{i}" for i in created],
              "ids": created, "absent": [], "f": expect["f"], "m0": expect["m0"], "passed": expect["passed"],
              "revision": "$current"}
    step_input = {"work": horizontal_work(case, by_name, names)}
    if layer_id:
        step_input["layerId"] = layer_id
    step = {"op": "execute", "input": step_input, "within": WITHIN,
            "result": {"status": "completed", "output": output, "warnings": list(warnings)},
            "expect": {"ids": IDS + created, "canUndo": True, "dirty": True, "revision": "changed",
                       "fields": fields_after}}
    before = {k: ({"p": BY_ID[int(k)]["p"]} if BY_ID[int(k)]["kind"] == "point" else {"pts": BY_ID[int(k)]["pts"]})
              for k in fields_after if int(k) in BY_ID}
    undo = {"op": "undo", "returns": NETWORK_STEP, "expect": {"ids": IDS, "canUndo": False, "canRedo": True,
                                                              "fields": before}}
    c = {"name": name, "steps": [step, undo]}
    if note:
        c["note"] = note
    return c


def level_case(name, case, by_name=False, note=None):
    known_keys = {key(k["name"]) for k in case["known"]}
    expect = case["expect"]
    points, moved, absent = [], [], []
    fields_after = {}
    for p in expect["points"]:
        if key(p["name"]) in known_keys:
            continue
        points.append({"name": p["name"], "h": p["h"], "sigma": p["sh"]})
        there = drawn(p["name"])
        if not there:
            absent.append(p["name"])
        for d in there:
            moved.append(d["id"])
            attrs = dict(d["attrs"])
            if "Z (m)" in attrs:
                attrs["Z (m)"] = f"{p['h']:.3f}"
            fields_after[str(d["id"])] = {"z": p["h"], "attrs": attrs}
            for other in ENTITIES:
                if other["kind"] != "polyline":
                    continue
                hit, _, zs = follow(other, d["p"], d["p"], z=p["h"], set_z=True)
                if hit:
                    fields_after[str(other["id"])] = {"zs": zs}
    warnings = []
    if absent:
        warnings.append({"code": "points_not_drawn", "message": f"{', '.join(absent)} çizimde yok; yazılmadı.",
                         "path": "work"})
    output = {"points": points, "moved": [f"$uidOf:{i}" for i in moved], "created": [], "ids": [], "absent": absent,
              "f": expect["f"], "m0": expect["m0"], "passed": expect["passed"], "revision": "$current"}
    step = {"op": "execute", "input": {"work": level_work(case, by_name)}, "within": WITHIN,
            "result": {"status": "completed", "output": output, "warnings": warnings}}
    if moved:
        step["expect"] = {"ids": IDS, "canUndo": True, "dirty": True, "revision": "changed", "fields": fields_after}
        before = {}
        for k in fields_after:
            e = BY_ID[int(k)]
            before[k] = {"z": e["z"], "attrs": e["attrs"]} if e["kind"] == "point" else {"zs": e["zs"]}
        steps = [step, {"op": "undo", "returns": LEVEL_STEP,
                        "expect": {"ids": IDS, "canUndo": False, "canRedo": True, "fields": before}}]
    else:
        step["expect"] = UNCHANGED | {"canUndo": False}
        steps = [step]
    c = {"name": name, "steps": steps}
    if note:
        c["note"] = note
    return c


def refused(code, message, path, status="failed"):
    return {"status": status, "error": {"code": code, "message": message, "path": path}}


def refusal(name, step_input, result, non_finite=None, before=None, note=None):
    step = {"op": "execute", "input": step_input}
    if non_finite:
        step["nonFinite"] = non_finite
    step["result"] = result
    step["expect"] = UNCHANGED if before is None else {"ids": IDS, "revision": "same"}
    c = {"name": name, "steps": (before or []) + [step]}
    if note:
        c["note"] = note
    return c


# ── Leap-frog setups (ADR 5300) ──────────────────────────────────────────────────────────────────────────────────

mp.mp.dps = 50
K = mp.mpf("0.13")
R = mp.mpf(6371000)
# Three setups from R1 to R2 through N1 and N2: each (from, to, back (S, z, t), fore (S, z, t)), zeniths in grad.
SETUPS = [
    ("R1", "N1", (62.415, 101.2345, 1.5), (58.902, 97.1234, 1.5)),
    ("N1", "N2", (71.31, 98.7654, 1.7), (69.88, 102.4321, 1.7)),
    ("N2", "R2", (55.12, 100.9876, 1.5), (61.045, 105.1191, 1.6)),
]


def num_words(x):
    t = repr(float(x))
    return t[:-2] if t.endswith(".0") else t


def leap(back, fore, full=400):
    """A setup's (dh, length) with mpmath: each sight S·cos z + (1 − k)·D²/2R − t, D = S·sin z."""
    parts = []
    for slope, zenith, target in (back, fore):
        z = mp.mpf(zenith) * 2 * mp.pi / full
        d = mp.mpf(slope) * mp.sin(z)
        parts.append((mp.mpf(slope) * mp.cos(z) + (1 - K) * d * d / (2 * R) - mp.mpf(target), d))
    return parts[1][0] - parts[0][0], parts[0][1] + parts[1][1]


def sight(s):
    return {"distance": s[0], "zenith": s[1], "target": s[2]}


def wire_setup(f, t, b, fo, scale=1.0):
    return {"from": f, "to": t, "back": sight((b[0], b[1] * scale, b[2])), "fore": sight((fo[0], fo[1] * scale, fo[2]))}


def leap_case(name, setups, rows=(), unit=None, note=None):
    """R1 and R2 known by their names (the drawing's heights); the setups' differences after `rows`."""
    full = 360 if unit == "deg" else 400
    scale = 0.9 if unit == "deg" else 1.0
    made = []
    for f, t, b, fo in setups:
        dh, length = leap((b[0], b[1] * scale, b[2]), (fo[0], fo[1] * scale, fo[2]), full)
        made.append({"from": f, "to": t, "dh": float(dh), "length": float(length)})
    known = [{"name": n, "h": next(e["z"] for e in ENTITIES if e.get("label") == n)} for n in ("R1", "R2")]
    case = {"kind": "level", "levelKind": "trigonometric", "known": known, "rows": list(rows) + made,
            "sigma": REFERENCE["defaults"]}
    case["expect"] = NA.levelling(case)
    case["expect"] = json.loads(json.dumps(case["expect"], default=float))
    case["rows"] = list(rows)
    c = level_case(name, case, by_name=True, note=note)
    work = c["steps"][0]["input"]["work"]
    work["leapfrog"] = [wire_setup(f, t, b, fo, scale) for f, t, b, fo in setups]
    if not rows:
        work["rows"] = []
    if unit:
        c["steps"][0]["input"]["unit"] = unit
    c["steps"][0]["result"]["output"]["leapfrog"] = made
    return c


def leap_refusal(name, setups, result, non_finite=None, method="trigonometric", unit=None):
    work = {"kind": "level", "method": method, "known": [{"name": "R1"}, {"name": "R2"}], "rows": [],
            "leapfrog": setups}
    step_input = {"work": work}
    if unit:
        step_input["unit"] = unit
    return refusal(name, step_input, result, non_finite)


def leap_cases():
    wired = [wire_setup(*x) for x in SETUPS]
    bent = lambda i, side, field, v: [  # noqa: E731
        {**w, side: {**w[side], field: v}} if k == i else w for k, w in enumerate(wired)]
    # A direct difference from R1 to N2 beside the first two setups (the reference's own row).
    direct = {"from": "R1", "to": "N2", "dh": -0.6789, "length": 260.0}
    return [
        leap_case("Atlamalı nivelman: iki reper arasında üç kurulum; N1 ve N2 kotlarını alır", SETUPS,
                  note="Alet noktaların üstünde değil: her kurulumun kot farkı ileri nişanınkinden geri nişanınki "
                       "çıkarılarak, alet yüksekliği düşer; uzunluk iki yatay uzunluğun toplamı."),
        leap_case("Atlamalı kurulumlar kot farklarıyla birlikte dengelenir", SETUPS[:2], rows=[direct]),
        leap_case("Atlamalı nivelman derece birimiyle (unit deg): aynı kurulumlar, aynı kotlar", SETUPS, unit="deg"),
        leap_refusal("Atlamalı kurulumun geri eğik uzunluğu sonlu değil", wired,
                     refused("not_finite", "Eğik uzunluk sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir sayı "
                             "verin.", "work.leapfrog[0].back.distance"), {"work.leapfrog[0].back.distance": "NaN"}),
        leap_refusal("Atlamalı kurulumun ileri başucu açısı sonlu değil", wired,
                     refused("not_finite", "Başucu açısı sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir sayı "
                             "verin.", "work.leapfrog[1].fore.zenith"), {"work.leapfrog[1].fore.zenith": "Infinity"}),
        leap_refusal("Atlamalı kurulumun işaret yüksekliği sonlu değil", wired,
                     refused("not_finite", "İşaret yüksekliği sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir "
                             "sayı verin.", "work.leapfrog[2].back.target"), {"work.leapfrog[2].back.target": "NaN"}),
        leap_refusal("Atlamalı kurulumun geri eğik uzunluğu sıfır", bent(0, "back", "distance", 0.0),
                     refused("survey_refused", "Geri nişanın eğik uzunluğu 0: sıfırdan büyük olmalı.",
                             "work.leapfrog[0]")),
        leap_refusal("Atlamalı kurulumun ileri başucu açısı tam aşağı (200 grad)", bent(1, "fore", "zenith", 200.0),
                     refused("survey_refused", "İleri nişanın başucu açısı 200: 0 ile 200 arasında olmalı (0 tam "
                             "yukarı).", "work.leapfrog[1]")),
        leap_refusal("Atlamalı kurulumun geri başucu açısı eksi", bent(2, "back", "zenith", -5.0),
                     refused("survey_refused", "Geri nişanın başucu açısı -5: 0 ile 200 arasında olmalı (0 tam "
                             "yukarı).", "work.leapfrog[2]")),
        leap_refusal("Derece biriminde başucu açısı 180", bent(0, "fore", "zenith", 180.0),
                     refused("survey_refused", "İleri nişanın başucu açısı 180: 0 ile 180 arasında olmalı (0 tam "
                             "yukarı).", "work.leapfrog[0]"), unit="deg"),
        leap_refusal("Geometrik nivelmanda atlamalı kurulum", wired,
                     refused("invalid_input", "Atlamalı kurulumların kot farkı trigonometriktir; method trigonometric "
                             "verin.", "work.method"), method="geometric"),
    ]


def build_cases():
    quad = reference("Çaprazlı dörtgen")
    weighted = reference("Ağırlıklı kontrol")
    free = reference("Serbest istasyon")
    loops = reference("Nivelman halkaları")
    trig = reference("Trigonometrik kot farkları")
    nodraw = reference("Trilaterasyon çizimsiz")
    same = reference("Durulan ve bakılan aynı")
    loose = reference("Bağlı olmayan nokta")
    hidden = {"code": "layer_hidden", "message": "“Gizli” katmanı gizli; çizilen nesne görünmeyecek.", "path": "layerId"}
    lock = [{"op": "execute", "command": "cad.layers.set", "input": {"operation": "locked", "layer": "nokta",
                                                                      "value": True},
             "result": {"status": "completed", "output": "$any", "warnings": []}}]
    # ADR 5200: the adjustment kept in the project under a name, in the same undo step.
    kept_cases = [
        keeping(horizontal_case("Yatay ağ sonucu projede saklanır: noktaların yeri ve σ'sı, f, m0, test", quad,
                                by_name=True), "Poligon 2026/1", "horizontal"),
        keeping(level_case("Kot ağı sonucu projede saklanır: noktaların kotu ve σ'sı", loops, by_name=True),
                "  Nivelman A ", "height"),
        keeping(horizontal_case("Ağırlıklı bilinenlerle saklanır", weighted, layer_id=None), "Kontrol", "horizontal"),
    ]
    cases = [
        horizontal_case("Yatay ağ: bilinenler çizimden adıyla; Y1 taşınır, çizgisinin köşesi onunla; Y2 eklenir", quad,
                        by_name=True,
                        note="Y1 çizimde 4 cm ötede; dengelenmiş yerine taşınır, Hat'taki çoklu çizginin köşesi onunla. "
                             "Y2 çizimde yok: Poligon'a Ağ noktası olarak eklenir."),
        horizontal_case("Yatay ağ: bilinenler koordinatlarıyla", quad),
        horizontal_case("Yatay ağ: bilinenin adı küçük harfle ve boşlukla, Türkçe karşılaştırılır", quad, by_name=True,
                        names=[" k1 ", "k2"]),
        horizontal_case("Ağırlıklı bilinenler: yalnız yeni Y1 yazılır, katman gerekmez", weighted, layer_id=None,
                        note="K2 ve Y2 ağırlıklı bilinen noktalardır: dengelenirler ama yazılmazlar."),
        horizontal_case("Serbest istasyon: S çizimde yok, eklenir", free,
                        note="Bilinen R1, R2, R3 koordinatlarıyla verilir; çizimin R1 ve R2 adlı noktaları kullanılmaz."),
        horizontal_case("Gizli katmana eklenen yeni nokta: söylenir", quad, by_name=True, layer_id="gizli",
                        warnings=[hidden]),
        level_case("Kot ağı: reperler çizimden kotlarıyla; N1 ve N2 kotlarını, Z (m)'lerini ve köşelerini alır; N3 "
                   "söylenir", loops, by_name=True),
        level_case("Kot ağı: reperler kotlarıyla", loops),
        level_case("Trigonometrik kot ağı: noktaların hiçbiri çizimde yok; yazılmaz, söylenir", trig),
        {"name": "Plan: yazılmaz",
         "steps": [{"op": "plan", "input": {"work": horizontal_work(quad, True), "layerId": "poligon"},
                    "result": {"status": "completed",
                               "output": {"changed": "$any", "created": "$any", "removed": [], "revision": "$current"},
                               "warnings": []},
                    "expect": UNCHANGED}]},
        {"name": "Doğrulama: yazılmaz",
         "steps": [{"op": "validate", "input": {"work": horizontal_work(quad, True), "layerId": "poligon"},
                    "result": {"status": "completed", "output": None, "warnings": []}, "expect": UNCHANGED}]},
        refusal("Doğrultu sonlu değil", {"work": horizontal_work(quad, True), "layerId": "poligon"},
                refused("not_finite", "Doğrultu sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir sayı verin.",
                        "work.rows[1].direction"), {"work.rows[1].direction": "NaN"}),
        refusal("Bilinen noktanın koordinatı sonlu değil", {"work": horizontal_work(quad), "layerId": "poligon"},
                refused("not_finite", "Bilinen noktanın doğu (Y) değeri sonlu bir sayı değil (NaN ya da sonsuz). "
                        "Koordinatı sonlu bir sayıyla verin.", "work.known[0].at.x"), {"work.known[0].at.x": "Infinity"}),
        refusal("Kot farkı sonlu değil", {"work": level_work(loops)},
                refused("not_finite", "Kot farkı sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir sayı verin.",
                        "work.rows[0].dh"), {"work.rows[0].dh": "NaN"}),
        refusal("Beklenen sürüm geçersiz", {"work": horizontal_work(quad, True), "layerId": "poligon",
                                            "expectedRevision": "dün"},
                refused("invalid_revision", "Beklenen sürüm “dün” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı "
                        "yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision")),
        {"name": "Beklenen sürüm eski: çakışma",
         "steps": [{"op": "execute", "input": {"work": horizontal_work(quad, True), "layerId": "poligon",
                                              "expectedRevision": "999"},
                    "result": {"status": "conflict",
                               "error": {"code": "revision_conflict",
                                         "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. "
                                                    "Komutu çizimin şimdiki hâline göre yeniden hazırlayın.",
                                         "path": "expectedRevision", "revision": "$current"}},
                    "expect": UNCHANGED}]},
        refusal("Adıyla verilen bilinen nokta çizimde yok",
                {"work": horizontal_work(quad, True, names=["K9", "K2"]), "layerId": "poligon"},
                refused("point_not_found", "“K9” adlı nokta çizimde yok. Adını denetleyin ya da koordinatlarını verin.",
                        "work.known[0].name")),
        refusal("Adıyla verilen reperin kotu yok",
                {"work": {"kind": "level", "known": [{"name": "K1"}], "rows": [{"from": "K1", "to": "N1", "dh": 1.0,
                                                                                "length": 100.0}]}},
                refused("point_not_found", "“K1” adlı kotlu nokta çizimde yok. Adını denetleyin ya da kotunu verin.",
                        "work.known[0].name")),
        refusal("Yaklaşık yeri bulunamayan noktalar: dengelemenin reddi", {"work": horizontal_work(nodraw),
                                                                           "layerId": "poligon"},
                refused("survey_refused", nodraw["error"], "work")),
        refusal("Durulan ve bakılan aynı: dengelemenin reddi, satırıyla", {"work": horizontal_work(same),
                                                                          "layerId": "poligon"},
                refused("survey_refused", same["error"], "work")),
        refusal("Kot ağında bağlı olmayan noktalar: dengelemenin reddi", {"work": level_work(loose)},
                refused("survey_refused", loose["error"], "work")),
        refusal("Taşınacak nokta kilitli katmanda", {"work": horizontal_work(quad, True), "layerId": "poligon"},
                refused("layer_locked", "“Nokta” katmanı kilitli; dengelenen noktalar ve onlara bağlı çizgiler "
                        "yazılamaz. Kilidi Katmanlar panelinden açın.", "work"), before=lock,
                note="Nokta katmanı önce kilitlenir; Y1 onun üzerinde."),
        refusal("Yeni nokta var, katman verilmedi", {"work": horizontal_work(quad, True)},
                refused("no_layer", "Çizimde olmayan 1 yeni nokta var; eklenecekleri katmanı kimliğiyle verin "
                        "(layerId).", "layerId")),
        refusal("Yeni noktaların katmanı yok", {"work": horizontal_work(quad, True), "layerId": "yok"},
                refused("layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.",
                        "layerId")),
        refusal("Yeni noktaların katmanı bir grup", {"work": horizontal_work(quad, True), "layerId": "grup"},
                refused("not_a_layer", "“Grup” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir "
                        "katman seçin.", "layerId")),
        refusal("Yeni noktaların katmanı kilitli", {"work": horizontal_work(quad, True), "layerId": "kilitli"},
                refused("layer_locked", "“Kilitli” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir "
                        "katmanı etkinleştirin.", "layerId")),
    ]
    # ADR 5200: a kept adjustment's name empty is refused before anything.
    cases.append(refusal("Saklanacak kaydın adı boş", {"work": horizontal_work(quad, True), "layerId": "poligon",
                                                       "keep": "  "},
                         refused("invalid_input", "Dengeleme kaydının adı boş olmaz; en çok 120 karakterdir, denetim "
                                 "karakteri taşımaz.", "keep")))
    return cases + kept_cases + leap_cases()


def compact(v):
    return json.dumps(v, ensure_ascii=False, separators=(", ", ": "))


def build(cases):
    out = ["{", '  "format": "kentos.command-cases",', '  "version": 1,', '  "command": "cad.survey.adjust",',
           '  "commandVersion": 1,', f'  "title": {compact("Ağ dengelemesi")},',
           f'  "note": {compact("Written by scripts/fixtures/survey_adjust_command_cases.py from docs/adr/0203 §8, 0622 §9 and fixtures/network-adjust/v1; do not edit by hand.")},',
           f'  "setup": {compact(SETUP)},', '  "cases": [']
    blocks = []
    for c in cases:
        lines = ["    {", f'      "name": {compact(c["name"])},']
        if "note" in c:
            lines.append(f'      "note": {compact(c["note"])},')
        lines.append('      "steps": [')
        lines.append(",\n".join(f"        {compact(s)}" for s in c["steps"]))
        lines.append("      ]")
        lines.append("    }")
        blocks.append("\n".join(lines))
    out.append(",\n".join(blocks))
    out.append("  ]")
    out.append("}")
    return "\n".join(out) + "\n"


def main():
    cases = build_cases()
    text = build(cases)
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            sys.exit(f"{OUT.relative_to(ROOT)} güncel değil")
        print(f"{len(cases)} durum tutarlı")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{len(cases)} durum yazıldı")


if __name__ == "__main__":
    main()
