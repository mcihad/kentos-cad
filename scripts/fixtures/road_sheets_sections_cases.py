#!/usr/bin/env python3
"""The shared cases of `road.sheets.sections` (plan/road.md Y10.5b, docs/adr/0908 §11; ROAD-SEC-11).

A sample line set's section views laid on sheets by the references and no KentOS code: each view and its objects are
road_section_command_cases.py's `drawing` (the sections by the corridor and section references at 40 digits), a view's
pitch its rule (docs/adr/0903 §7), the map's size the template's file (`crates/shared/sheet/templates/cad-teknik.json`,
data), the packing the rule of docs/adr/0908 §11 written here.

    uv run --no-project --python 3.11 --with triangle --with numpy --with mpmath --with pyproj \\
        python3 scripts/fixtures/road_sheets_sections_cases.py [--check]
"""

import copy
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_grades  # noqa: E402
import road_profile_command_cases as pcmd  # noqa: E402
import road_section_command_cases as scmd  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/road.sheets.sections.json"
TEMPLATE = json.loads((ROOT / "crates/shared/sheet/templates/cad-teknik.json").read_text(encoding="utf-8"))
ITEMS = TEMPLATE["sheet"]["items"]
MAP = next(i for i in ITEMS if i["kind"]["type"] == "map")
PAGE = TEMPLATE["sheet"]["page"]["size"]

completed, failed, step, case = pcmd.completed, pcmd.failed, pcmd.step, pcmd.case
UNCHANGED, STEPPED, CONFLICT, BAD_REVISION = pcmd.UNCHANGED, pcmd.STEPPED, pcmd.CONFLICT, pcmd.BAD_REVISION
TEMPLATES, START = [scmd.TK1, scmd.TK2], [scmd.S1, scmd.S2]
LAYERS = pcmd.DRAW_LAYERS
STEP_NAME = "Enkesit paftaları"
FIRST = len(pcmd.ENTITIES) + len(scmd.LINES) + 1
HIDDEN = {"code": "layer_hidden", "message": "“Gizli” katmanı gizli; çizilen nesne görünmeyecek.", "path": "layerId"}


def bad(path, words):
    return failed("invalid_road_sheets", words, path)


class Refused(Exception):
    def __init__(self, result):
        super().__init__(result["error"]["message"])
        self.result = result


def lay(input_, sheet_sets=()):
    d = next((x for x in START if x["id"] == input_["set"] or x["name"].lower() == input_["set"].lower()), None)
    if d is None:
        raise Refused(failed("unknown_section_set", f"“{input_['set']}” kimlikli ya da adlı kesit takımı projede yok.",
                             "set"))
    if input_["layerId"] == "yok":
        raise Refused(failed("layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini "
                                                "verin.", "layerId"))
    stations = input_.get("stations") or scmd.stations_of(d)
    ex = input_.get("exaggeration", 1.0)
    height = input_.get("height", 2.5)
    whole, _ = scmd.drawing(d, [0.0, 0.0], stations=stations, columns=1, ex=ex, height=height)
    v0 = whole["views"][0]
    tall = (v0["top"] - v0["datum"]) * ex
    h = height
    row = 2.4 * h
    across = 8 * h + d["left"] + d["right"] + 4 * h
    down = row + tall + 3 * row + 4 * h
    warns = []
    if input_["layerId"] == "gizli":
        warns.append(HIDDEN)
    if d["id"] == "k-2":
        warns.append(scmd.stale_warning(d))
    warns += [w for w in scmd.sections(d, stations, TEMPLATES)[1] if w["code"] in ("section_open", "section_ground_missing")]
    source = input_.get("source", {"type": "system", "id": "sys:cad-teknik"})
    if source["id"] not in ("sys:cad-teknik", "sys:gis-atlas"):
        raise Refused(failed("unknown_template", f"“{source['id']}” sistem şablonu yok; cad.sheets.set(details=true) ile "
                                                 "şablonları listeleyin.", "source.id"))
    for field in ("name", "center", "scale"):
        if field in input_.get("options", {}):
            raise Refused(failed("invalid_options", "Enkesit paftalarının adı, ortası ve ölçeği kesitlerinden ve "
                                                    f"scale'den gelir; options.{field} verilmez.", f"options.{field}"))
    if source["id"] == "sys:gis-atlas":
        raise Refused(failed("not_fixed_map", "Şablonun görünüm penceresi sabit görünümlü değil; enkesit paftası "
                                              "haritayı kendisi yerleştirir.", "source"))
    scale = input_.get("scale", MAP["kind"]["view"]["scale"])
    if not 1 <= scale <= 1_000_000:
        raise Refused(bad("scale", "Ölçek 1 ile 1 000 000 arası olmalı."))
    width = MAP["frame"]["width"] / 1e6 * scale
    hgt = MAP["frame"]["height"] / 1e6 * scale
    columns, rows = math.floor(width / across), math.floor(hgt / down)
    if columns == 0 or rows == 0:
        raise Refused(bad("scale", f"Bir enkesit görünümü ({across:.1f} × {down:.1f} m) 1:{scale} ölçekte haritaya "
                                   f"({width:.1f} × {hgt:.1f} m) sığmıyor; ölçeğin paydasını büyütün."))
    per = columns * rows
    groups = [stations[i:i + per] for i in range(0, len(stations), per)]
    name = (input_.get("name") or "").strip() or f"{d['name']} paftaları"
    if any(x["name"].lower() == name.lower() for x in sheet_sets):
        raise Refused(bad("name", f"“{name}” adında bir pafta takımı var."))
    first = input_.get("first", "E-101")
    if not first[-1:].isdigit():
        raise Refused(failed("bad_drawing_info", f"“{first}” bir sayıyla bitmiyor; ilk pafta numarası Y-101 gibi olmalı.",
                             "first"))
    set_id = next(f"ps-{n}" for n in range(1, 1000) if all(x["id"] != f"ps-{n}" for x in sheet_sets))
    gap = 0.25 * down
    at = input_["at"]
    sheets, objects = [], 0
    before, digits = first[: len(first.rstrip("0123456789"))], first[len(first.rstrip("0123456789")):]
    for k, g in enumerate(groups):
        top = at[1] - k * (hgt + gap)
        drawn, n = scmd.drawing(d, [at[0], top], stations=g, columns=columns, ex=ex, height=height)
        objects += n
        a, b = drawn["views"][0], drawn["views"][-1]
        title = f"Enkesitler km {a['kmText']} – {b['kmText']}" if a["station"] != b["station"] else \
            f"Enkesit km {a['kmText']}"
        sheets.append({"sheet": f"yol-{set_id}-{k + 1}", "number": f"{before}{int(digits) + k:0{len(digits)}d}",
                       "title": title, "from": min(g), "to": max(g), "center": [at[0] + 0.5 * width, top - 0.5 * hgt],
                       "rotation": 0.0, "view": [[at[0], top - hgt], [at[0] + width, top]]})
    out = {"set": set_id, "name": name, "alignment": d["alignment"], "sections": d["id"], "scale": scale,
           "length": width, "height": hgt, "sheets": sheets}
    kept = {"id": set_id, "name": name, "alignment": d["alignment"], "alignmentRevision": d["alignmentRevision"],
            "sections": d["id"], "sectionsRevision": d["revision"], "from": min(stations), "to": max(stations),
            "scale": scale, "sheets": [{k: r[k] for k in ("sheet", "from", "to", "view")} for r in sheets]}
    book = [{"drawingInfo": {"number": r["number"], "title": r["title"], "revision": "", "status": "",
                             "subset": ["Yol", d["name"]]},
             "id": r["sheet"], "name": f"{r['number']} {r['title']}", "width": PAGE["width"] / 1000,
             "height": PAGE["height"] / 1000, "items": len(ITEMS),
             "maps": [{"item": f"{r['sheet']}-item-1", "name": MAP["name"], "scale": float(scale)}]}
            for r in sheets]
    return out, warns, objects, kept, book


def road_with(sets):
    road = scmd.road_of(TEMPLATES, START)
    if sets:
        road["sheetSets"] = copy.deepcopy(list(sets))
    return road


def setup(sets=()):
    base = scmd.setup(layers=LAYERS)
    if sets:
        base["settings"]["road"]["sheetSets"] = copy.deepcopy(list(sets))
    return base


def ok(name, input_, op="execute", sets=(), undo=False):
    out, warns, n, kept, book = lay(input_, sets)
    out = {**out, "uids": "$any" if op == "execute" else [], "revision": "$current"}
    if op == "validate":
        result, expect = completed(None, warns), {**UNCHANGED, "road": road_with(sets)}
    elif op == "plan":
        result, expect = completed(out, warns), {**UNCHANGED, "road": road_with(sets)}
    else:
        result = completed(out, warns)
        expect = {**STEPPED, "road": road_with(list(sets) + [kept]), "ids": list(range(1, FIRST + n)), "sheets": book}
    s = step(op, input_, result, expect)
    s["within"] = 1e-6
    steps = [s]
    if undo:
        steps.append({"op": "undo", "returns": STEP_NAME,
                      "expect": {"ids": list(range(1, FIRST)), "canRedo": True, "road": road_with(sets)}})
    return case(name, steps, setup(sets) if sets else None)


def no(name, input_, result=None, setup_=None, sets=()):
    if result is None:
        try:
            lay(input_, sets)
        except Refused as r:
            result = r.result
        else:
            raise AssertionError(f"{name}: refuses")
    road = None if setup_ is not None else road_with(sets)
    return case(name, [step("execute", input_, result, {**UNCHANGED, "road": road})],
                setup_ or (setup(sets) if sets else None))


BARE = {k: v for k, v in scmd.S1.items() if k != "surface"}
OTHER = {"id": "ps-1", "name": "Eski enkesitler", "alignment": "g-1", "alignmentRevision": 1, "sections": "k-1",
         "sectionsRevision": 1, "from": 100.0, "to": 300.0, "scale": 1000,
         "sheets": [{"sheet": "eski-1", "from": 100.0, "to": 300.0}]}


def cases():
    base = {"set": "k-1", "layerId": "0", "at": [0.0, -500.0]}
    return [
        ok("Takımın bütün enkesitleri 1:500", base),
        ok("1:1000: tek paftaya sığar", {**base, "scale": 1000}),
        ok("Planı: hiçbir şey yazılmaz", base, op="plan"),
        ok("Doğrulama", base, op="validate"),
        ok("İki kat abartma, büyük yazı", {**base, "exaggeration": 2.0, "height": 3.5}),
        ok("Yalnız üç km, takım adıyla", {**base, "set": "enkesitler", "stations": [250.0, 150.0, 200.0]}),
        ok("Eskiyen takım, açık yanlarla", {**base, "set": "k-2", "scale": 400}),
        ok("Gizli katmana, adıyla, K-11'den", {**base, "layerId": "gizli", "name": "Ana yol enkesitleri",
                                              "first": "K-11"}),
        ok("Başka takımın yanına", {**base, "scale": 2000}, sets=[OTHER]),
        ok("Tek geri alma adımı", {**base, "scale": 1000}, undo=True),
        no("Yol projesi değil", base, scmd.NOT_ROAD, setup_=scmd.setup(work="map", layers=LAYERS)),
        no("Olmayan takım", {**base, "set": "yok"}),
        no("Takımın km'si olmayan", {**base, "stations": [150.0, 160.0]},
           failed("invalid_places", "160 m “Enkesitler” kesit takımının km'lerinden biri değil.", "stations[1]")),
        no("Sıfır abartma", {**base, "exaggeration": 0.0},
           failed("invalid_label", "Düşey abartma (exaggeration) 0'dan büyük olmalı.", "exaggeration")),
        no("Olmayan katman", {**base, "layerId": "yok"}),
        no("Katman grubu", {**base, "layerId": "grup"},
           failed("not_a_layer", "“Yol” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin.",
                  "layerId")),
        no("Kilitli katman", {**base, "layerId": "kilitli"},
           failed("layer_locked", "“Kilitli” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı "
                                  "etkinleştirin.", "layerId")),
        no("Görünüm haritaya sığmaz", {**base, "scale": 100}),
        no("Sıfır ölçek", {**base, "scale": 0}),
        no("Atlas haritalı şablon", {**base, "source": {"type": "system", "id": "sys:gis-atlas"}}),
        case("Yüzeysiz takım", [step("execute", base, failed("no_terrain", "“Enkesitler” kesit takımının yüzeyi yok: "
                                                              "enkesit çizmek için önce bir yüzey verin.", "set"),
                                     {**UNCHANGED, "road": scmd.road_of(TEMPLATES, [BARE, scmd.S2])})],
             scmd.setup(sets=(BARE, scmd.S2), layers=LAYERS)),
        no("Olmayan şablon", {**base, "source": {"type": "system", "id": "sys:yok"}}),
        no("Seçeneklerde ölçek", {**base, "options": {"scale": 200}}),
        no("Sayıyla bitmeyen ilk numara", {**base, "first": "E-A"}),
        no("Adı alınmış takım", {**base, "name": "eski ENKESİTLER".replace("İ", "i")}, sets=[OTHER]),
        no("Geçersiz beklenen sürüm", {**base, "expectedRevision": "on"}, BAD_REVISION),
        no("Eski sürüme göre hazırlanmış", {**base, "expectedRevision": "41"}, CONFLICT),
    ]


def main():
    data = {"format": "kentos.command-cases", "version": 1, "command": "road.sheets.sections", "commandVersion": 1,
            "title": "Enkesit paftaları",
            "note": "scripts/fixtures/road_sheets_sections_cases.py yazar; görünümler road_section_command_cases.py'nin "
                    "başvurusundan, haritanın boyu şablonun dosyasından, dizilim ADR 0908 §11'den; KentOS kodu yok.",
            "setup": setup(), "cases": cases()}
    data = road_grades.fixture(data)
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
