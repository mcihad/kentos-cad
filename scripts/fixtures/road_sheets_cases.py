#!/usr/bin/env python3
"""The shared cases of `road.sheets.plan` (plan/road.md Y10.5, docs/adr/0908 §10; ROAD-SEC-11).

An alignment laid on plan–profile sheets by the references and no KentOS code: the alignment's points and km are
road_alignment_cases.py's (40 digits, through road_profile_command_cases.py's drawing), each profile view's frame and
objects road_profile_command_cases.py's `drawing` over the sheet's stations; the plan map's size is the template's
(`crates/shared/sheet/templates/yol-plan-boykesit.json`, and with KGM's bands `yol-plan-boykesit-kgm.json`, docs/adr/0981;
data), the rest the rule of docs/adr/0908 §10 written here.

    uv run --no-project --python 3.11 --with triangle --with numpy --with mpmath --with pyproj \\
        python3 scripts/fixtures/road_sheets_cases.py [--check]
"""

import copy
import json
import math
import sys
from pathlib import Path

import mpmath as mp

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_alignment_cases as aref  # noqa: E402
import road_profile_command_cases as pcmd  # noqa: E402
import road_grades  # noqa: E402
import road_profile_cases as pref  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/road.sheets.plan.json"
def template(name):
    """A road template's items, its plan and profile maps and its page (the template's file, data)."""
    t = json.loads((ROOT / f"crates/shared/sheet/templates/{name}.json").read_text(encoding="utf-8"))
    items = t["sheet"]["items"]
    return (items, next(i for i in items if i["id"] == "plan"), next(i for i in items if i["id"] == "boykesit"),
            t["sheet"]["page"]["size"])


ITEMS, PLAN, PROFILE, PAGE = template("yol-plan-boykesit")
# Docs/adr/0981: KGM's A1 plan–profile sheet, the default with KGM's bands.
KGM = template("yol-plan-boykesit-kgm")

completed, failed, step, case = pcmd.completed, pcmd.failed, pcmd.step, pcmd.case
UNCHANGED, STEPPED, CONFLICT, BAD_REVISION = pcmd.UNCHANGED, pcmd.STEPPED, pcmd.CONFLICT, pcmd.BAD_REVISION
NOT_ROAD = failed("not_a_road_project", "Aks yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.")
START = [pcmd.MAIN, pcmd.SIDE, pcmd.PART]
MOST = 500
PLOT = 1000.0
DEPTH = 5 * 2.4 * 2.5 * PLOT / 1000.0
GAP = 0.4 * DEPTH
# Docs/adr/0980: KGM's six bands, 7 + 7 + 9 + 6 + 7 + 7 text heights deep.
KGM_DEPTH = 43 * 2.5 * PLOT / 1000.0
STEP_NAME = "Yol paftaları"


def num(v):
    return str(int(v)) if float(v).is_integer() else repr(float(v))


def bad(path, words):
    return failed("invalid_road_sheets", words, path)


def point(aid, s):
    b = pcmd.ALIGNED[aid]
    x, y = pref.point_at(b.els, mp.mpf(s))
    return float(x), float(y)


def km_text(aid, s, digits=2):
    b = pcmd.ALIGNED[aid]
    return pcmd.acmd.km_text(aref.km_of(b.regions, mp.mpf(s)), digits)


class Refused(Exception):
    def __init__(self, result):
        super().__init__(result["error"]["message"])
        self.result = result


def lay(input_, sets=()):
    """The output, warnings, objects drawn and the set kept, or Refused."""
    alignments = {a["id"]: a for a in pcmd.ALIGNMENTS}
    by_name = {a["name"].lower(): a for a in pcmd.ALIGNMENTS}
    q = input_["alignment"]
    a = alignments.get(q) or by_name.get(q.lower())
    if a is None:
        raise Refused(failed("unknown_alignment", f"“{q}” kimlikli ya da adlı aks projede yok.", "alignment"))
    if "offsetOf" in a:
        raise Refused(bad("alignment", f"“{a['name']}” paralel bir aks; yol paftası PI'larıyla kurulan aksta olur."))
    length = float(pcmd.ALIGNED[a["id"]].length)
    profile = None
    if "profile" in input_:
        p = next((x for x in START if x["id"] == input_["profile"] or x["name"].lower() == input_["profile"].lower()),
                 None)
        if p is None:
            raise Refused(failed("unknown_road_profile", f"“{input_['profile']}” kimlikli ya da adlı boykesit projede yok.",
                                 "profile"))
        if p["alignment"] != a["id"]:
            raise Refused(bad("profile", f"“{p['name']}” boykesiti “{a['name']}” aksında değil."))
        if "layerId" not in input_:
            raise Refused(bad("layerId", "Boykesit görünümleri için bir katman (layerId) verin."))
        if "at" not in input_:
            raise Refused(bad("at", "İlk boykesit görünümünün yeri (at) verilmeli."))
        profile = p
    lo, hi = (0.0, length) if profile is None else (max(profile["pvis"][0]["station"], 0.0),
                                                     min(profile["pvis"][-1]["station"], length))
    frm, to = input_.get("from", lo), input_.get("to", hi)
    span = f"aks 0 ile {num(length)} m arası"
    if not -1e-9 <= frm <= length + 1e-9:
        raise Refused(bad("from", f"{num(frm)} m aksın üstünde değil: {span}."))
    if not -1e-9 <= to <= length + 1e-9:
        raise Refused(bad("to", f"{num(to)} m aksın üstünde değil: {span}."))
    if to <= frm:
        raise Refused(bad("to", "Aralığın sonu başından sonra olmalı."))
    if (profile is not None or input_.get("matchLines")) and input_.get("layerId") == "kilitli":
        raise Refused(failed("layer_locked", "“Kilitli” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka "
                                             "bir katmanı etkinleştirin.", "layerId"))
    if (profile is not None or input_.get("matchLines")) and input_.get("layerId") == "yok":
        raise Refused(failed("layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.",
                             "layerId"))
    if (profile is not None or input_.get("matchLines")) and input_.get("layerId") == "grup":
        raise Refused(failed("not_a_layer", "“Yol” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir "
                                            "katman seçin.", "layerId"))
    # Docs/adr/0998: match lines are drawn objects; they need a layer.
    if input_.get("matchLines") and "layerId" not in input_:
        raise Refused(bad("layerId", "Eşleşme çizgileri bir katmana çizilir; layerId verin."))
    default = "sys:yol-plan-boykesit-kgm" if input_.get("bands") == "kgm" else "sys:yol-plan-boykesit"
    source = input_.get("source", {"type": "system", "id": default})
    items, plan_item, profile_item, page = KGM if source["id"] == "sys:yol-plan-boykesit-kgm" else (ITEMS, PLAN, PROFILE, PAGE)
    if source["id"] not in ("sys:yol-plan-boykesit", "sys:yol-plan-boykesit-kgm", "sys:cad-teknik"):
        raise Refused(failed("unknown_template", f"“{source['id']}” sistem şablonu yok; cad.sheets.set(details=true) ile "
                                                 "şablonları listeleyin.", "source.id"))
    for field in ("name", "center", "scale"):
        if field in input_.get("options", {}):
            raise Refused(failed("invalid_options", "Yol paftalarının adı, ortası ve ölçeği aksından ve scale'den gelir; "
                                                    f"options.{field} verilmez.", f"options.{field}"))
    if source["id"] == "sys:cad-teknik" and profile is not None:
        raise Refused(failed("not_fixed_map", "Şablonun paftasında iki görünüm penceresi yok; plan–boykesit paftası "
                                              "planı birinciye, boykesiti ikinciye koyar.", "source"))
    scale = input_.get("scale", plan_item["kind"]["view"]["scale"])
    if not 1 <= scale <= 1_000_000:
        raise Refused(bad("scale", "Ölçek 1 ile 1 000 000 arası olmalı."))
    width = plan_item["frame"]["width"] / 1e6 * scale
    height = plan_item["frame"]["height"] / 1e6 * scale
    pheight = profile_item["frame"]["height"] / 1e6 * scale
    overlap = input_.get("overlap", 0.0)
    if not 0 <= overlap < 0.5 * width:
        raise Refused(bad("overlap", f"Bindirme 0 ile bir paftanın boyunun yarısı ({num(0.5 * width)} m) arasında "
                                     "olmalı."))
    stretches, s = [], frm
    while True:
        b = min(s + width, to)
        stretches.append((s, b))
        if len(stretches) > MOST:
            raise Refused(bad("scale", f"Bu ölçekle {MOST}'den çok pafta çıkar; ölçeği büyütün ya da aralığı daraltın."))
        if b >= to - 1e-9:
            break
        s = b - overlap
    name = (input_.get("name") or "").strip() or f"{a['name']} paftaları"
    if any(x["name"].lower() == name.lower() for x in sets):
        raise Refused(bad("name", f"“{name}” adında bir pafta takımı var."))
    first = input_.get("first", "Y-101")
    if not first[-1:].isdigit():
        raise Refused(failed("bad_drawing_info", f"“{first}” bir sayıyla bitmiyor; ilk pafta numarası Y-101 gibi olmalı.",
                             "first"))
    set_id = next(f"ps-{n}" for n in range(1, 1000) if all(x["id"] != f"ps-{n}" for x in sets))
    sheets, warnings, objects, outside = [], [], 0, []
    next_y = None
    for k, (sa, sb) in enumerate(stretches):
        pa, pb = point(a["id"], sa), point(a["id"], sb)
        theta = math.atan2(pb[1] - pa[1], pb[0] - pa[0])
        center = [0.5 * (pa[0] + pb[0]), 0.5 * (pa[1] + pb[1])]
        mdeg = round(math.degrees(theta) % 360.0 * 1000) % 360_000
        n = min(max(math.ceil(sb - sa), 2), 2000)
        off = False
        for i in range(n + 1):
            x, y = point(a["id"], sa + (sb - sa) * i / n)
            dx, dy = x - center[0], y - center[1]
            u, v = dx * math.cos(theta) + dy * math.sin(theta), -dx * math.sin(theta) + dy * math.cos(theta)
            if abs(u) > 0.5 * width + 1e-6 or abs(v) > 0.5 * height + 1e-6:
                off = True
                break
        before, digits = first[: len(first.rstrip("0123456789"))], first[len(first.rstrip("0123456789")):]
        row = {"sheet": f"yol-{set_id}-{k + 1}", "number": f"{before}{int(digits) + k:0{len(digits)}d}",
               "title": f"Km {km_text(a['id'], sa)} – {km_text(a['id'], sb)}", "from": sa, "to": sb,
               "center": center, "rotation": mdeg / 1000.0}
        if profile is not None:
            p0, p1 = profile["pvis"][0]["station"], profile["pvis"][-1]["station"]
            va, vb = max(sa, p0), min(sb, p1)
            if vb > va + 1e-9:
                ex = input_.get("exaggeration", 10.0)
                bands = input_.get("bands")
                dever = pcmd.DEVER if input_.get("superelevation") else None
                depth = KGM_DEPTH if bands == "kgm" else DEPTH
                sized, _, _ = pcmd.drawing(profile, [0.0, 0.0], ex=ex, frm=va, to=vb)
                h = sized["frame"][1][1] - sized["frame"][0][1]
                # Docs/adr/0990, KGM's kot taşıma: a view taller than the map is split to the whole metres of
                # elevation the map holds over the bands; one that cannot be stays whole and outside.
                span = None
                if h + depth > pheight + 1e-6:
                    room = math.floor((pheight - depth) / ex)
                    if room > 0:
                        tried, fit, _ = pcmd.drawing(profile, [0.0, 0.0], ex=ex, frm=va, to=vb, span=float(room))
                        if fit is not None:
                            span = float(room)
                            h = tried["frame"][1][1] - tried["frame"][0][1]
                at = input_["at"]
                y0 = at[1] if next_y is None else next_y - h
                drawn, count, _ = pcmd.drawing(profile, [at[0], y0], ex=ex, frm=va, to=vb, bands=bands, dever=dever,
                                               span=span)
                objects += count
                next_y = y0 - depth - (0.4 * depth)
                row["view"] = drawn["frame"]
                if h + depth > pheight + 1e-6:
                    off = True
        if off:
            row["outside"] = True
            outside.append(k + 1)
        sheets.append(row)
    if profile is not None and input_["layerId"] == "gizli":
        warnings.append({"code": "layer_hidden", "message": "“Gizli” katmanı gizli; çizilen nesne görünmeyecek.",
                         "path": "layerId"})
    if profile is not None and profile["id"] == "b-2":
        warnings.append(pcmd.stale_warning(profile))
    if outside:
        warnings.append({"code": "road_sheet_outside", "path": "scale",
                         "message": f"{len(outside)} paftada aks ya da boykesit görünümü haritanın dışına taşıyor "
                                    f"({', '.join(str(k) for k in outside)}. pafta); ölçeği büyütün ya da şablonun "
                                    "haritasını genişletin."})
    # Docs/adr/0998: a match line between each two sheets: its line and a text on each side.
    if input_.get("matchLines"):
        objects += 3 * (len(sheets) - 1)
    out = {"set": set_id, "name": name, "alignment": a["id"], "scale": scale, "length": width, "height": height,
           "sheets": sheets}
    if profile is not None:
        out["profile"] = profile["id"]
    kept = {"id": set_id, "name": name, "alignment": a["id"], "alignmentRevision": a["revision"], "from": frm,
            "to": to, "scale": scale,
            "sheets": [{k: r[k] for k in ("sheet", "from", "to", "view") if k in r} for r in sheets]}
    if profile is not None:
        kept["profile"] = profile["id"]
        kept["profileRevision"] = profile["revision"]
    if overlap:
        kept["overlap"] = overlap
    book = [{"drawingInfo": {"number": r["number"], "title": r["title"], "revision": "", "status": "",
                             "subset": ["Yol", a["name"]]},
             "id": r["sheet"], "name": f"{r['number']} {r['title']}", "width": page["width"] / 1000,
             "height": page["height"] / 1000, "items": len(items),
             "maps": [{"item": f"{r['sheet']}-item-1", "name": "Plan", "scale": float(scale)},
                      {"item": f"{r['sheet']}-item-2", "name": "Boykesit", "scale": float(scale)}]}
            for r in sheets]
    return out, warnings, objects, kept, book


def road_with(sets, supers=None):
    road = pcmd.road_of(START, supers=supers)
    if sets:
        road["sheetSets"] = copy.deepcopy(list(sets))
    return road


def setup(sets=(), supers=None):
    base = pcmd.setup(START, layers=pcmd.DRAW_LAYERS, supers=supers)
    if sets:
        base["settings"]["road"]["sheetSets"] = copy.deepcopy(list(sets))
    return base


FIRST = len(pcmd.ENTITIES) + 1


def ok(name, input_, op="execute", sets=(), undo=False, supers=None):
    out, warnings, n, kept, book = lay(input_, sets)
    out = {**out, "uids": "$any" if (op == "execute" and n) else [], "revision": "$current"}
    if op == "validate":
        result, expect = completed(None, warnings), {**UNCHANGED, "road": road_with(sets, supers)}
    elif op == "plan":
        result, expect = completed(out, warnings), {**UNCHANGED, "road": road_with(sets, supers)}
    else:
        result = completed(out, warnings)
        expect = {**STEPPED, "road": road_with(list(sets) + [kept], supers), "ids": list(range(1, FIRST + n)),
                  "sheets": book}
    s = step(op, input_, result, expect)
    s["within"] = 1e-6
    steps = [s]
    if undo:
        steps.append({"op": "undo", "returns": STEP_NAME,
                      "expect": {"ids": list(range(1, FIRST)), "canRedo": True, "road": road_with(sets)}})
    return case(name, steps, setup(sets, supers) if (sets or supers) else None)


def no(name, input_, setup_=None, sets=(), result=None):
    if result is None:
        try:
            lay(input_, sets)
        except Refused as r:
            result = r.result
        else:
            raise AssertionError(f"{name}: refuses")
    return case(name, [step("execute", input_, result, {**UNCHANGED, "road": road_with(sets) if setup_ is None else
                                                         None})], setup_ or (setup(sets) if sets else None))


OTHER = {"id": "ps-1", "name": "Eski paftalar", "alignment": "g-2", "alignmentRevision": 3, "from": 0.0,
         "to": 200.0, "scale": 1000, "sheets": [{"sheet": "eski-1", "from": 0.0, "to": 200.0}]}


def cases():
    pro = {"alignment": "Ana aks", "profile": "b-1", "layerId": "0", "at": [0.0, -400.0]}
    return [
        ok("Ana aks 1:500, yalnız plan", {"alignment": "Ana aks"}),
        ok("Ana aks 1:1000, boykesitiyle, 5 kat abartma", {**pro, "scale": 1000, "exaggeration": 5.0}),
        ok("Planı: hiçbir şey yazılmaz", {**pro, "scale": 1000, "exaggeration": 5.0}, op="plan"),
        ok("Doğrulama", {**pro, "scale": 1000}, op="validate"),
        # Docs/adr/0990: 1:1000 with 10 times exaggeration: a view taller than the map is split (kot taşıma).
        ok("1:1000, 10 kat abartma: boykesit kot taşımayla haritaya sığar", {**pro, "scale": 1000}),
        ok("Boykesitle 1:500: görünümler haritaya sığmaz, uyarıyla", pro),
        ok("Km 100–400, 20 m bindirme", {"alignment": "g-1", "from": 100.0, "to": 400.0, "overlap": 20.0}),
        # Docs/adr/0998: match lines, at the middle of each overlap.
        ok("Eşleşme çizgileri, 20 m bindirme", {**pro, "scale": 1000, "exaggeration": 5.0, "overlap": 20.0,
                                                "matchLines": True}),
        ok("Yalnız plan, eşleşme çizgileri katmana", {"alignment": "g-1", "layerId": "0", "matchLines": True}),
        ok("Kısmi boykesit: yalnız onun aralığı", {"alignment": "g-1", "profile": "Kısmi kot", "layerId": "yol-boykesit",
                                                    "at": [800.0, 0.0], "scale": 1000, "exaggeration": 2.0}),
        ok("Yan aks, eskiyen boykesit uyarıyla", {"alignment": "Yan aks", "profile": "b-2", "layerId": "0",
                                                   "at": [0.0, -900.0], "scale": 2000, "exaggeration": 2.0}),
        ok("Adıyla takım, başka takımın yanına", {"alignment": "Ana aks", "name": "Ana yol 1:2000", "scale": 2000},
           sets=[OTHER]),
        ok("Tek geri alma adımı", {**pro, "scale": 1000, "exaggeration": 5.0}, undo=True),
        # Docs/adr/0980: the profile views with KGM's bands, the second with the Dever band's schedule.
        ok("KGM bantlarıyla, 1:1000", {**pro, "scale": 1000, "exaggeration": 5.0, "bands": "kgm"}),
        ok("KGM bantları ve dever çizelgesiyle", {**pro, "scale": 1000, "exaggeration": 5.0, "bands": "kgm",
                                                 "superelevation": "Ana dever"}, supers=[pcmd.DEVER]),
        ok("Gizli katmana uyarıyla, Y-201'den", {**pro, "layerId": "gizli", "scale": 1000, "exaggeration": 5.0,
                                                 "first": "Y-201"}),
        no("Olmayan katman", {**pro, "layerId": "yok"}),
        no("Katman grubu", {**pro, "layerId": "grup"}),
        no("Eşleşme çizgileri katmansız", {"alignment": "g-1", "matchLines": True}),
        no("Olmayan şablon", {"alignment": "Ana aks", "source": {"type": "system", "id": "sys:yok"}}),
        no("Tek haritalı şablonda boykesit", {**pro, "source": {"type": "system", "id": "sys:cad-teknik"}}),
        no("Seçeneklerde ad", {"alignment": "Ana aks", "options": {"name": "Pafta"}}),
        no("Sayıyla bitmeyen ilk numara", {"alignment": "Ana aks", "first": "Yol-A"}),
        no("Yol projesi değil", {"alignment": "Ana aks"}, setup_=pcmd.setup(work="map", layers=pcmd.DRAW_LAYERS),
           result=NOT_ROAD),
        no("Olmayan aks", {"alignment": "yok"}),
        no("Paralel aks", {"alignment": "Sağ kenar"}),
        no("Olmayan boykesit", {"alignment": "Ana aks", "profile": "yok", "layerId": "0", "at": [0.0, 0.0]}),
        no("Başka aksın boykesiti", {"alignment": "Ana aks", "profile": "b-2", "layerId": "0", "at": [0.0, 0.0]}),
        no("Boykesit katmansız", {"alignment": "Ana aks", "profile": "b-1", "at": [0.0, 0.0]}),
        no("Boykesit yersiz", {"alignment": "Ana aks", "profile": "b-1", "layerId": "0"}),
        no("Aralığın başı aksın önünde", {"alignment": "Ana aks", "from": -5.0}),
        no("Aralık ters", {"alignment": "Ana aks", "from": 300.0, "to": 200.0}),
        no("Kilitli katman", {**pro, "layerId": "kilitli"}),
        no("Sıfır ölçek", {"alignment": "Ana aks", "scale": 0}),
        no("Bindirme paftanın yarısı", {"alignment": "Ana aks", "overlap": 92.5}),
        no("Çok pafta", {"alignment": "Ana aks", "scale": 1}),
        no("Adı alınmış takım", {"alignment": "Ana aks", "name": "eski PAFTALAR"}, sets=[OTHER]),
        no("Geçersiz beklenen sürüm", {"alignment": "Ana aks", "expectedRevision": "on"}, result=BAD_REVISION),
        no("Eski sürüme göre hazırlanmış", {"alignment": "Ana aks", "expectedRevision": "41"}, result=CONFLICT),
    ]


def main():
    data = {"format": "kentos.command-cases", "version": 1, "command": "road.sheets.plan", "commandVersion": 1,
            "title": "Yol paftaları",
            "note": "scripts/fixtures/road_sheets_cases.py yazar; aksın noktaları ve km'si road_alignment_cases.py'nin, "
                    "boykesit görünümleri road_profile_command_cases.py'nin başvurusundan, plan haritasının boyu "
                    "şablonun dosyasından, kural ADR 0908 §10'dan; KentOS kodu yok.",
            "setup": setup(), "cases": cases()}
    data = road_grades.fixture(data)  # docs/adr/0977: the schedule's slopes as grades
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
