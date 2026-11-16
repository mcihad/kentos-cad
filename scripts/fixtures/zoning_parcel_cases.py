#!/usr/bin/env python3
"""The shared cases of `cad.zoning.parcel` (Parselin plan kararları; docs/adr/8072 §5, plan/bim-plan.md BIM-11),
without KentOS code.

    python3 scripts/fixtures/zoning_parcel_cases.py           # writes fixtures/commands/v1/cad.zoning.parcel.json
    python3 scripts/fixtures/zoning_parcel_cases.py --check   # writes nothing; compares

The rules, by hand, over the profile `catalog/v1/profiles/imar-plani.json` (its fields' order is read from it):
- the parcel: the one given, else the site plan's; a closed area (here axis-aligned rectangles);
- a decision's share: its common part with the parcel, counted when its mean width 2A/P passes the tolerance (0.01 m
  unless given); a rectangle's common part with a rectangle is a rectangle, so 2A/P = 2wh/(2w+2h);
- its values as written, in the profile's order, the empty ones left out; its plan the boundary whose PlanKimlik its
  Plan names; its notes the numbers its PlanNotlari names among its plan's notes (a missing one said), then its plan's
  general notes; a plan whose Durum is neither onayli nor yururlukte, or without OnayTarihi, said;
- uncovered: the parcel less the decisions' counted parts (the parts here never overlap each other's outside the
  conflict case, whose two decisions are one inside the other);
- a conflict: two decisions whose counted parts overlap in a counted piece, each field they write differently (numbers
  as numbers);
- against: the typed emsal (KAKS) and TAKS where a decision on the parcel writes another;
- notes: several decisions (a kademe hattı, PAİY 4/1-mm) first, then the plans' and the notes' words in their sorted
  order.
"""

import copy
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.zoning.parcel.json"
PROFILE = json.loads((ROOT / "catalog/v1/profiles/imar-plani.json").read_text("utf-8"))
X0, Y0 = 487000.0, 4420000.0
FIELDS = [f["name"] for c in PROFILE["classes"] if c["id"] == "karar-alani" for f in c["fields"]
          if f["name"] not in ("KararKimlik", "Plan", "PlanNotlari")]
NUMBERS = {"TAKS", "KAKS", "KatAdedi", "Hmax", "OnCekme", "YanCekme", "ArkaCekme"}
IN_FORCE = {"onayli", "yururlukte"}
TOLERANCE = 0.01


def uid(n):
    return f"0192f5c0-8072-7005-8000-{n:012x}"


def rect(x0, y0, x1, y1):
    return (x0, y0, x1, y1)


def pts(r):
    x0, y0, x1, y1 = r
    return [{"x": X0 + x0, "y": Y0 + y0}, {"x": X0 + x1, "y": Y0 + y0}, {"x": X0 + x1, "y": Y0 + y1},
            {"x": X0 + x0, "y": Y0 + y1}]


def inter(a, b):
    x0, y0 = max(a[0], b[0]), max(a[1], b[1])
    x1, y1 = min(a[2], b[2]), min(a[3], b[3])
    return (x0, y0, x1, y1) if x1 > x0 and y1 > y0 else None


def area(r):
    return (r[2] - r[0]) * (r[3] - r[1])


def counts(r, t):
    w, h = r[2] - r[0], r[3] - r[1]
    return 2 * w * h / (2 * w + 2 * h) > t


def number(v):
    try:
        return float(v.replace(",", "."))
    except ValueError:
        return None


def same(field, a, b):
    if field in NUMBERS and number(a) is not None and number(b) is not None:
        return number(a) == number(b)
    return a == b


# The plan: two plans side by side (UIP-1 in force, UIP-2 suspended without a date), their decisions and notes.
PLANS = [
    {"id": 1, "r": rect(0, 0, 100, 50), "attrs": {"PlanKimlik": "UIP-1", "PlanAdi": "Merkez uygulama imar planı",
                                                   "Seviye": "uip", "Durum": "yururlukte", "Surum": "3",
                                                   "OnayTarihi": "2024-05-14", "Dayanak": "Meclis kararı 2024/112"}},
    {"id": 2, "r": rect(100, 0, 140, 50), "attrs": {"PlanKimlik": "UIP-2", "Seviye": "uip", "Durum": "askida"}},
    # A plan whose status nobody wrote.
    {"id": 16, "r": rect(200, 0, 240, 50), "attrs": {"PlanKimlik": "UIP-3", "Seviye": "uip"}},
]
DECISIONS = [
    {"id": 3, "r": rect(0, 0, 40, 50), "attrs": {"KararKimlik": "K1", "Plan": "UIP-1", "Kullanim": "konut",
                                                 "Nizam": "A", "TAKS": "0.40", "KAKS": "1.20", "KatAdedi": "3",
                                                 "OnCekme": "5", "YanCekme": "3", "PlanNotlari": "1; 2"}},
    {"id": 4, "r": rect(40, 0, 100, 50), "attrs": {"KararKimlik": "K2", "Plan": "UIP-1", "Kullanim": "ticaret-konut",
                                                  "Nizam": "BL", "TAKS": "0.50", "KAKS": "2,00", "Hmax": "15.50",
                                                  "PlanNotlari": "2, 3"}},
    # Inside K1's north: a decision drawn over another (a plan error).
    {"id": 5, "r": rect(0, 40, 20, 50), "attrs": {"KararKimlik": "K3", "Plan": "UIP-1", "Kullanim": "konut",
                                                 "Nizam": "A", "TAKS": "0.4", "KAKS": "1.50", "KatAdedi": "4"}},
    {"id": 6, "r": rect(100, 0, 140, 50), "attrs": {"KararKimlik": "K4", "Plan": "UIP-2", "Kullanim": "egitim",
                                                   "Belirtilmeyen": "TAKS"}},
    # A decision naming no plan, one naming a plan not drawn, one of the plan without a status.
    {"id": 17, "r": rect(160, 0, 180, 50), "attrs": {"KararKimlik": "K5", "Kullanim": "park"}},
    {"id": 18, "r": rect(180, 0, 200, 50), "attrs": {"KararKimlik": "K6", "Plan": "UIP-9", "Kullanim": "spor"}},
    {"id": 19, "r": rect(200, 0, 240, 50), "attrs": {"KararKimlik": "K7", "Plan": "UIP-3", "Kullanim": "konut",
                                                    "TAKS": "0.35", "KAKS": "1.05"}},
]
NOTES = [
    {"id": 7, "at": (10, 60), "attrs": {"NotNo": "1", "Plan": "UIP-1", "Metin": "Bodrum katlar emsale dahil değildir.",
                                        "Kapsam": "genel"}},
    {"id": 8, "at": (20, 60), "attrs": {"NotNo": "2", "Plan": "UIP-1", "Metin": "Çatı katı yapılamaz.",
                                        "Kapsam": "alan"}},
]
# The parcels (on layer 0): across K1 and K2; inside K1; over K1 and K3; across K2 and K4 and past the plans; K1's
# with a 5 mm sliver over K2's edge.
PARCELS = {
    "P1": (9, rect(30, 10, 60, 40)),
    "P2": (10, rect(5, 5, 35, 30)),
    "P3": (11, rect(10, 35, 30, 50)),
    "P4": (12, rect(90, 10, 150, 20)),
    "P6": (14, rect(10, 10, 40.005, 30)),
    "P7": (20, rect(165, 10, 175, 20)),
    "P8": (21, rect(185, 10, 195, 20)),
    "P9": (22, rect(205, 10, 235, 40)),
    "P11": (24, rect(300, 0, 320, 10)),
}
# P2's corners again, as a closed line (Parsel ve bahçe mesafeleri takes either).
CLOSED = 23
LINE = 15


def layers(with_plan=True):
    style = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}

    def layer(i, name):
        return {"id": i, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
                "style": style, "children": []}
    group = {"id": "imar", "name": "İmar planı", "type": "group", "visible": True, "locked": False, "expanded": True,
             "style": style, "children": [layer("plan", "Plan sınırı"), layer("karar", "Karar alanları"),
                                          layer("notlar", "Plan notları")]}
    return [layer("0", "0"), group] if with_plan else [layer("0", "0")]


def setup(site="P1", zoning=(1.20, 0.40), with_plan=True):
    ents = []
    if with_plan:
        ents += [{"kind": "polygon", "id": p["id"], "layerId": "plan", "attrs": p["attrs"], "pts": pts(p["r"])}
                 for p in PLANS]
        ents += [{"kind": "polygon", "id": d["id"], "layerId": "karar", "attrs": d["attrs"], "pts": pts(d["r"])}
                 for d in DECISIONS]
        ents += [{"kind": "point", "id": n["id"], "layerId": "notlar", "attrs": n["attrs"],
                  "p": {"x": X0 + n["at"][0], "y": Y0 + n["at"][1]}} for n in NOTES]
    ents += [{"kind": "polygon", "id": i, "layerId": "0", "attrs": {}, "pts": pts(r)} for _, (i, r) in sorted(PARCELS.items())]
    ents.append({"kind": "polyline", "id": CLOSED, "layerId": "0", "attrs": {}, "pts": pts(PARCELS["P2"][1]),
                 "closed": True})
    ents.append({"kind": "line", "id": LINE, "layerId": "0", "attrs": {}, "a": {"x": X0, "y": Y0 - 5},
                 "b": {"x": X0 + 10, "y": Y0 - 5}})
    ents.sort(key=lambda e: e["id"])
    arch = {"levels": [{"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0}]}
    if site:
        arch["site"] = {"parcel": uid(PARCELS[site][0])}
    if zoning:
        arch["zoning"] = {k: v for k, v in (("parcelArea", 900.0), ("far", zoning[0]), ("coverage", zoning[1]))
                          if v is not None}
    return {"format": "kentos.document", "version": 2, "name": "Parselin plan kararları",
            "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                         "plotScale": 500.0, "workspace": "cad", "arch": arch},
            "origin": {"x": X0, "y": Y0}, "layers": layers(with_plan), "activeLayer": "0", "entities": ents,
            "uids": [uid(e["id"]) for e in ents], "styles": {"items": [], "categories": []}}


def refusal(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def answer(parcel_key, t=TOLERANCE, zoning=(1.20, 0.40), with_plan=True, closed=False):
    i, pr = PARCELS[parcel_key]
    if closed:
        i = CLOSED
    notes, said, out, kept = [], set(), [], []
    if not with_plan:
        return {"parcel": uid(i), "area": area(pr), "decisions": [], "uncovered": area(pr), "conflicts": [],
                "against": [], "notes": ["Çizimde imar planının karar alanı katmanı yok; plan kararları belediyenin "
                                         "CBS'inden imar-plani profiliyle gelir (cad.profiles.apply)."]}
    for d in DECISIONS:
        r = inter(pr, d["r"])
        if r is None or not counts(r, t):
            continue
        kept.append((d, r))
        a = d["attrs"]
        values = [{"field": f, "value": a[f]} for f in FIELDS if a.get(f, "").strip()]
        plan = next((p for p in PLANS if a.get("Plan") and p["attrs"]["PlanKimlik"] == a.get("Plan")), None)
        source = None
        if not plan:
            if not a.get("Plan"):
                said.add(f"“{a['KararKimlik']}” kararının planı yazılmamış.")
            else:
                said.add(f"“{a['KararKimlik']}” kararının planı {a['Plan']} çizimde yok.")
        if plan:
            pa = plan["attrs"]
            source = {"uid": uid(plan["id"]), "id": pa["PlanKimlik"]}
            for key, name in (("name", "PlanAdi"), ("level", "Seviye"), ("status", "Durum"), ("version", "Surum"),
                              ("approved", "OnayTarihi"), ("basis", "Dayanak")):
                if pa.get(name):
                    source[key] = pa[name]
            if not pa.get("Durum"):
                said.add(f"{pa['PlanKimlik']} planının durumu yazılmamış.")
            elif pa["Durum"] not in IN_FORCE:
                said.add(f"{pa['PlanKimlik']} planının durumu “{pa['Durum']}”; yürürlükte değil.")
            if not pa.get("OnayTarihi"):
                said.add(f"{pa['PlanKimlik']} planının onay tarihi yok.")
        texts = []
        for no in [n.strip() for n in a.get("PlanNotlari", "").replace(";", ",").replace(" ", ",").split(",") if n.strip()]:
            n = next((n for n in NOTES if n["attrs"]["NotNo"] == no and n["attrs"].get("Plan", a.get("Plan")) == a.get("Plan")), None)
            if n:
                texts.append({"number": no, "text": n["attrs"]["Metin"], "scope": n["attrs"]["Kapsam"]})
            else:
                said.add(f"“{a['KararKimlik']}” kararının andığı {no} numaralı plan notu çizimde yok.")
        for n in NOTES:
            na = n["attrs"]
            if na.get("Plan", a.get("Plan")) == a.get("Plan") and na["Kapsam"] == "genel" \
                    and not any(x["number"] == na["NotNo"] for x in texts):
                texts.append({"number": na["NotNo"], "text": na["Metin"], "scope": na["Kapsam"]})
        dec = {"uid": uid(d["id"]), "id": a["KararKimlik"], "share": area(r), "values": values, "notes": texts}
        if source:
            dec["plan"] = source
        out.append(dec)
    # Uncovered: the parcel less the kept parts (their union: K3's part lies in K1's).
    xs = sorted({pr[0], pr[2]} | {v for _, r in kept for v in (r[0], r[2])})
    ys = sorted({pr[1], pr[3]} | {v for _, r in kept for v in (r[1], r[3])})
    uncovered_cells = []
    for x0, x1 in zip(xs, xs[1:]):
        for y0, y1 in zip(ys, ys[1:]):
            cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
            if not any(r[0] < cx < r[2] and r[1] < cy < r[3] for _, r in kept):
                uncovered_cells.append((x0, y0, x1, y1))
    uncovered = sum(area(c) for c in uncovered_cells if counts(c, t)) if uncovered_cells else 0.0
    conflicts = []
    for k in range(len(kept)):
        for m in range(k + 1, len(kept)):
            (da, ra), (db, rb) = kept[k], kept[m]
            both = inter(ra, rb)
            if both is None or not counts(both, t):
                continue
            for f in FIELDS:
                va, vb = da["attrs"].get(f, "").strip(), db["attrs"].get(f, "").strip()
                if not same(f, va, vb):
                    conflicts.append({"field": f, "area": area(both),
                                      "decisions": [da["attrs"]["KararKimlik"], db["attrs"]["KararKimlik"]],
                                      "values": [va, vb]})
    if len(out) > 1:
        notes.append(f"Parsel {len(out)} karar alanına isabet ediyor ({', '.join(d['id'] for d in out)}); kademe hattı "
                     "ve hangi kararın nerede geçtiği plandan okunur (PAİY 4/1-mm).")
    notes += sorted(said)
    against = []
    if zoning:
        for field, typed in (("KAKS", zoning[0]), ("TAKS", zoning[1])):
            if typed is None:
                continue
            plan_values = []
            for d in out:
                v = next((v["value"] for v in d["values"] if v["field"] == field), None)
                if v is not None and v not in plan_values:
                    plan_values.append(v)
            if any(number(v) is None or number(v) != typed for v in plan_values):
                against.append({"field": field, "typed": typed, "plan": plan_values})
    return {"parcel": uid(i), "area": area(pr), "decisions": out, "uncovered": uncovered, "conflicts": conflicts,
            "against": against, "notes": notes}


def step(inp, result, op="execute"):
    return {"op": op, "input": inp, "result": result, "within": 1e-6, "expect": {"revision": "same"}}


def completed(output):
    return {"status": "completed", "output": output, "warnings": []}


def cases():
    out = []

    def case(name, steps, s=None):
        c = {"name": name, "steps": steps}
        if s is not None:
            c["setup"] = s
        out.append(c)
    case("Vaziyet planının parseli iki karara isabet ediyor: kaynakları, notları, imar durumuyla fark",
         [step({}, completed(answer("P1")))])
    case("Planı da aynı cevabı verir", [step({}, completed(answer("P1")), op="plan")])
    case("Tek karar, imar durumu planla aynı: fark yok", [step({"parcel": uid(PARCELS["P2"][0])}, completed(answer("P2")))])
    case("Üst üste çizilmiş iki karar: çelişkileri ikisiyle söylenir",
         [step({"parcel": uid(PARCELS["P3"][0])}, completed(answer("P3")))])
    case("Askıdaki, onay tarihsiz plan ve planın dışında kalan alan",
         [step({"parcel": uid(PARCELS["P4"][0])}, completed(answer("P4")))])
    case("Karar sınırında 5 mm'lik şerit sayılmaz",
         [step({"parcel": uid(PARCELS["P6"][0])}, completed(answer("P6")))])
    case("Daha dar toleransla şerit de sayılır", [step({"parcel": uid(PARCELS["P6"][0]), "tolerance": 0.001},
                                                        completed(answer("P6", 0.001)))])
    case("Karar katmanı yoksa söylenir", [step({}, completed(answer("P1", with_plan=False)))], setup(with_plan=False))
    case("İmar durumu yazılmamışsa karşılaştırma yok", [step({}, completed(answer("P1", zoning=None)))],
         setup(zoning=None))
    case("Vaziyet planında parsel yoksa reddedilir",
         [step({}, refusal("parcel_missing", "Parsel verilmedi ve vaziyet planında parsel yok. Parselin kalıcı "
                                             "kimliğini verin ya da parseli Mimari › Proje ▾ › Parsel ve bahçe "
                                             "mesafeleri'nde seçin.", "parcel"))], setup(site=None))
    case("Çizimde olmayan parsel reddedilir",
         [step({"parcel": uid(999)}, refusal("entity_not_found", f"“{uid(999)}” kimlikli nesne çizimde yok. Parselin "
                                                                 "kalıcı kimliğini verin.", "parcel"))])
    case("Kapalı olmayan nesne parsel değildir",
         [step({"parcel": uid(LINE)}, refusal("not_a_parcel", "Parsel kapalı bir çizgi ya da alan değil; parselin "
                                                              "sınırını kapalı çizin.", "parcel"))])
    case("Planı yazılmamış karar söylenir", [step({"parcel": uid(PARCELS["P7"][0])}, completed(answer("P7")))])
    case("Planı çizimde olmayan karar söylenir", [step({"parcel": uid(PARCELS["P8"][0])}, completed(answer("P8")))])
    case("Durumu ve onay tarihi yazılmamış plan söylenir; imar durumuyla fark",
         [step({"parcel": uid(PARCELS["P9"][0])}, completed(answer("P9")))])
    case("Parsel kapalı çizgi de olur", [step({"parcel": uid(CLOSED)}, completed(answer("P2", closed=True)))])
    case("Planın dışındaki parsel: karar yok, bütün alanı kararsız",
         [step({"parcel": uid(PARCELS["P11"][0])}, completed(answer("P11")))])
    case("İmar durumunda yalnız emsal yazılıysa yalnız o karşılaştırılır",
         [step({}, completed(answer("P1", zoning=(1.20, None))))], setup(zoning=(1.20, None)))
    case("Doğrulama: yazmaz, bir şey döndürmez", [step({}, {"status": "completed", "output": None, "warnings": []},
                                                      op="validate")])
    case("Doğrulama da reddeder", [step({"parcel": uid(LINE)}, refusal("not_a_parcel", "Parsel kapalı bir çizgi ya da "
                                                                          "alan değil; parselin sınırını kapalı çizin.",
                                                                          "parcel"), op="validate")])
    case("Sıfır tolerans reddedilir",
         [step({"tolerance": 0}, refusal("invalid_input", "Tolerans sonlu ve en az 0.000001 m olmalı (yazılan 0).",
                                         "tolerance"))])
    return out


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.zoning.parcel", "commandVersion": 1,
            "title": "Parselin plan kararları: payları, kaynak planları, notları, çelişkiler, imar durumuyla fark; retler",
            "note": "ADR 8072 §5. Beklenenler scripts/fixtures/zoning_parcel_cases.py'nin kurallarından, KentOS kodu "
                    "olmadan; parsel ve kararlar eksenlere paralel dikdörtgenler, ortak parçalar tam.",
            "setup": setup(), "cases": cases()}


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} başvurudan farklı; --check'siz yeniden yazın ve farkı okuyun.")
            sys.exit(1)
        print(f"{OUT.relative_to(ROOT)}: {len(build()['cases'])} durum başvuruyla aynı.")
        return
    OUT.write_text(text, "utf-8")
    print(f"{OUT.relative_to(ROOT)}: {len(build()['cases'])} durum yazıldı.")


if __name__ == "__main__":
    main()
