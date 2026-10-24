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
"""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
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
    return cases


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
