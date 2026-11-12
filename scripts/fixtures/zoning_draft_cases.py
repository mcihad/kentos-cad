#!/usr/bin/env python3
"""The shared cases of `cad.zoning.draft` (Plan değişikliği taslağı; plan/map-cad.md MAP-09, docs/adr/5040), without
KentOS code.

    python3 scripts/fixtures/zoning_draft_cases.py           # writes fixtures/commands/v1/cad.zoning.draft.json
    python3 scripts/fixtures/zoning_draft_cases.py --check   # writes nothing; compares

The rules, by hand, from Mekânsal Planlar Yapım Yönetmeliği (RG 14.06.2014/29030, as amended to 22.01.2026):

- a draft keeps its id, the plan (a plan boundary's PlanKimlik), the drawing's revision it is made from, its reason
  (m. 9(4): a plan change's report explains its justification; none is refused) and, for each decision area it
  changes, the field and the value before (the area's, trimmed; empty when it has none) and after; a draft of the id
  is replaced, `remove` takes one out; the decision areas are never written;
- a decision area belongs to the plan its `Plan` names, or to the only plan boundary when it names none;
- the answer: the plan's decision areas by use before and after (shoelace areas of the rectangles; the list's order,
  then areas without a use), each with after − before; then, in that order, each social or technical infrastructure
  use (m. 4: eğitim, sağlık, dini, sosyal, kültür, spor, resmî, teknik altyapı, otopark, park, yeşil alan, meydan;
  roads left out as m. 26(3)(b) leaves them) whose area gets smaller (`infrastructureLess`, by how much), and for each
  changed area in the drawing's order a rise of KAKS or KatAdedi (`densityUp`, m. 26(5)(a)) and a rise of KatAdedi
  or Hmax (`heightUp`, m. 26(4), (5)(b)), judged only when both values are numbers;
- refusals in the contract's order.
"""

import copy
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.zoning.draft.json"
PROFILE = json.loads((ROOT / "catalog/v1/profiles/imar-plani.json").read_text("utf-8"))
X0, Y0 = 512000.0, 4410000.0
STEP, REMOVE = "Plan değişikliği taslağı", "Plan değişikliği taslağını sil"
SAME = {"revision": "same", "canUndo": False}


def domain(d):
    return [v["code"] for v in next(x for x in PROFILE["domains"] if x["id"] == d)["values"]]


USES = domain("plan-kullanim")
NIZAM = domain("yapi-nizami")
LABEL = {v["code"]: v["label"] for v in next(x for x in PROFILE["domains"] if x["id"] == "plan-kullanim")["values"]}
INFRA = ["egitim", "saglik", "dini", "sosyal", "kultur", "spor", "resmi", "teknik", "otopark", "park", "yesil", "meydan"]
FIELDS = ["Kullanim", "Nizam", "TAKS", "KAKS", "KatAdedi", "Hmax", "OnCekme", "YanCekme", "ArkaCekme"]
NUMBERS = FIELDS[2:]
MPYY = "Mekânsal Planlar Yapım Yönetmeliği"


def uid(n):
    return f"0192f5c0-5040-7000-8000-{n:012x}"


def node(id_, name, children=None):
    return {"id": id_, "name": name, "type": "group" if children is not None else "layer", "visible": True,
            "locked": False, "expanded": True, "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25},
            "children": children or []}


def rect(x0, y0, x1, y1):
    return [{"x": X0 + x0, "y": Y0 + y0}, {"x": X0 + x1, "y": Y0 + y0}, {"x": X0 + x1, "y": Y0 + y1},
            {"x": X0 + x0, "y": Y0 + y1}]


# The drawing: two plans; UİP-1's five decision areas (one without a use) and UİP-2's park.
OBJECTS = [
    ("PU", "plan", (0, 0, 120, 40), {"PlanKimlik": "UİP-1", "Seviye": "uip", "Durum": "yururlukte"}),
    ("PV", "plan", (200, 0, 240, 40), {"PlanKimlik": "UİP-2", "Seviye": "uip", "Durum": "yururlukte"}),
    ("PARK", "karar", (0, 0, 20, 20), {"KararKimlik": "K1", "Plan": "UİP-1", "Kullanim": "park"}),
    ("TICARET", "karar", (20, 0, 50, 20), {"KararKimlik": "K2", "Plan": "UİP-1", "Kullanim": "ticaret", "Nizam": "A",
                                          "TAKS": "0.40", "KAKS": "1.50", "KatAdedi": "4", "Hmax": "12.50"}),
    ("KONUT", "karar", (50, 0, 70, 20), {"KararKimlik": "K3", "Plan": "UİP-1", "Kullanim": "konut", "TAKS": "0.30",
                                        "KAKS": "1.20", "KatAdedi": "3", "Hmax": "9.50"}),
    ("OKUL", "karar", (70, 0, 95, 20), {"KararKimlik": "K4", "Plan": "UİP-1", "Kullanim": "egitim"}),
    ("BOS", "karar", (95, 0, 105, 20), {"KararKimlik": "K5", "Plan": "UİP-1"}),
    ("OBUR", "karar", (200, 0, 220, 20), {"KararKimlik": "K6", "Plan": "UİP-2", "Kullanim": "park"}),
]
SLOT = {name: i + 1 for i, (name, *_) in enumerate(OBJECTS)}


def U(name):
    return uid(SLOT[name])


def entities(objects):
    return [{"kind": "polygon", "id": i + 1, "layerId": layer, "attrs": dict(attrs), "pts": rect(*r)}
            for i, (_, layer, r, attrs) in enumerate(objects)]


def setup(objects=None, zoning=None, decisions=True):
    objects = objects if objects is not None else OBJECTS
    ents = entities(objects)
    kids = [node("plan", "Plan sınırı")] + ([node("karar", "Karar alanları")] if decisions else [])
    settings = {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                "plotScale": 1000.0, "workspace": "cad", "cadWork": "map"}
    if zoning:
        settings["zoningPlan"] = copy.deepcopy(zoning)
    return {"format": "kentos.document", "version": 2, "name": "Plan değişikliği taslağı", "settings": settings,
            "origin": {"x": X0, "y": Y0}, "layers": [node("0", "0"), node("imar", "İmar planı", kids)],
            "activeLayer": "plan", "entities": ents, "uids": [uid(i + 1) for i in range(len(ents))],
            "styles": {"items": [], "categories": []}}


def number(t):
    try:
        v = float(t.strip().replace(",", "."))
    except ValueError:
        return None
    return v if v == v and v not in (float("inf"), float("-inf")) else None


def area(r):
    x0, y0, x1, y1 = r
    pts = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
    return abs(sum(a[0] * b[1] - b[0] * a[1] for a, b in zip(pts, pts[1:] + pts[:1]))) / 2


def answer(objects, plan, changes):
    """The plan's areas by use before and after, and what m. 26 asks, by hand."""
    plans = [a["PlanKimlik"] for _, layer, _, a in objects if layer == "plan"]
    after = {(c["area"], c["field"]): c["after"] for c in changes}
    totals, per_area = {}, []
    for i, (_, layer, r, a) in enumerate(objects):
        if layer != "karar":
            continue
        own = a.get("Plan", "").strip() or (plans[0] if len(plans) == 1 else "")
        if own != plan:
            continue
        u = uid(i + 1)
        before_use = a.get("Kullanim", "").strip()
        after_use = after.get((u, "Kullanim"), before_use)
        totals.setdefault(before_use, [0.0, 0.0])[0] += area(r)
        totals.setdefault(after_use, [0.0, 0.0])[1] += area(r)

        def rise(field, word):
            now, nxt = a.get(field, "").strip(), after.get((u, field))
            if nxt is None or number(now) is None or number(nxt) is None or not number(nxt) > number(now):
                return None
            return f"{word} {now} → {nxt}"
        name = a.get("KararKimlik", "").strip() or u
        density = [x for x in (rise("KAKS", "KAKS"), rise("KatAdedi", "kat adedi")) if x]
        if density:
            per_area.append({"kind": "densityUp", "uid": u, "message": f"“{name}” karar alanında yoğunluk artıyor "
                             f"({', '.join(density)}): {MPYY} m. 26(5)(a) artan nüfusun ihtiyacı olan sosyal ve teknik "
                             "altyapı alanlarının standartlara uygun ayrılmasını ister."})
        height = [x for x in (rise("KatAdedi", "kat adedi"), rise("Hmax", "Hmax")) if x]
        if height:
            per_area.append({"kind": "heightUp", "uid": u, "message": f"“{name}” karar alanında yükseklik artıyor "
                             f"({', '.join(height)}): {MPYY} m. 26(4) yörenin dokusunu ve siluetini, m. 26(5)(b) "
                             "karşılıklı bina cepheleri arasında en az K = (Y1 + Y2)/2 + 7,00 m uzaklığı ister; KentOS "
                             "bu uzaklığı ölçmez."})
    order = sorted(totals, key=lambda c: (c == "", USES.index(c) if c in USES else len(USES), c))
    uses = [{"use": c, "before": totals[c][0], "after": totals[c][1], "difference": totals[c][1] - totals[c][0]}
            for c in order]
    findings = [{"kind": "infrastructureLess", "use": u["use"], "area": u["before"] - u["after"],
                 "message": f"“{LABEL[u['use']]}” küçülüyor: {MPYY} m. 26(3) sosyal ve teknik altyapı alanının "
                            "kaldırılmasını ya da küçültülmesini zorunluluğa, ilgili kurumun görüşüne ve hizmet etki "
                            "alanında eşdeğer yeni alana bağlar; KentOS eşdeğerliği değerlendirmez."}
                for u in uses if u["use"] in INFRA and u["after"] < u["before"]]
    return uses, findings + per_area


def change(name, field, value):
    return {"uid": U(name), "field": field, "value": value}


def draft_of(objects, id_, plan, reason, changes, revision="0"):
    attrs = {uid(i + 1): a for i, (_, _, _, a) in enumerate(objects)}
    return {"id": id_, "plan": plan, "revision": revision, "reason": reason.strip(),
            "changes": [{"area": c["uid"], "field": c["field"], "before": attrs[c["uid"]].get(c["field"], "").strip(),
                         "after": c["value"].strip()} for c in changes]}


REASON = "Okul alanının ihtiyaç analizine göre yeri değişiyor (plan raporu ekte)."


def saved(name, changes, id_="T-1", plan="UİP-1", reason=REASON, zoning=None, op="execute", note=None):
    """A draft kept: its answer, the project's data after and, for execute, the undo."""
    d = draft_of(OBJECTS, id_, plan, reason, changes)
    uses, findings = answer(OBJECTS, plan, d["changes"])
    before = copy.deepcopy(zoning) if zoning else None
    data = copy.deepcopy(zoning) if zoning else {}
    drafts = data.setdefault("drafts", [])
    at = next((k for k, x in enumerate(drafts) if x["id"] == id_), None)
    if at is None:
        drafts.append(d)
    else:
        drafts[at] = d
    out = {"draft": d, "level": "uip", "uses": uses, "findings": findings, "drafts": len(drafts),
           "revision": "$current"}
    input_ = {"id": id_, "plan": plan, "reason": reason, "changes": changes}
    case = {"name": name}
    if note:
        case["note"] = note
    if zoning:
        case["setup"] = setup(zoning=zoning)
    if op != "execute":
        case["steps"] = [{"op": op, "input": input_, "result": {"status": "completed", "output": None if op ==
                         "validate" else out, "warnings": []}, "expect": SAME}]
        return case
    changed = {c["uid"] for c in changes}
    untouched = {str(SLOT[n]): {"attrs": a} for n, _, _, a in OBJECTS if U(n) in changed}
    case["steps"] = [
        {"op": "execute", "input": input_, "result": {"status": "completed", "output": out, "warnings": []},
         "within": 1e-9, "expect": {"revision": "changed", "canUndo": True, "settings": {"zoningPlan": data},
                                    "fields": untouched}},
        {"op": "undo", "returns": STEP, "expect": {"settings": {"zoningPlan": before}, "fields": untouched}},
    ]
    return case


def refused(name, input_, code, message, path=None, d=None):
    case = {"name": name}
    if d is not None:
        case["setup"] = d
    error = {"code": code, "message": message}
    if path:
        error["path"] = path
    case["steps"] = [{"op": "execute", "input": input_, "result": {"status": "failed", "error": error},
                      "expect": SAME}]
    return case


def ok(changes, **more):
    return {"id": "T-1", "plan": "UİP-1", "reason": REASON, "changes": changes, **more}


def cases():
    to_trade = [change("PARK", "Kullanim", "ticaret")]
    kept_draft = draft_of(OBJECTS, "T-1", "UİP-1", "Eski gerekçe.", [change("KONUT", "KAKS", "1.00")])
    symbols = [{"level": "uip", "use": "konut", "symbol": "mpyy.uip.konut.gelisme-konut-alani"}]
    removal_before = {"drafts": [kept_draft]}
    out_removed = {"level": "", "uses": [], "findings": [], "drafts": 0, "revision": "$current"}
    return [
        saved("Park alanı ticarete dönüyor: alanlar önce ve sonra, m. 26(3) bulgusu; taslak projede, karar alanı "
              "değişmez; tek adımda geri alınır", to_trade,
              note="Yürürlükteki karar alanlarının öznitelikleri olduğu gibi kalır."),
        saved("Yoğunluk ve yükseklik artışı: KAKS, kat adedi ve Hmax", [change("TICARET", "KAKS", "2.00"),
                                                                       change("TICARET", "KatAdedi", "6"),
                                                                       change("TICARET", "Hmax", "18.50")]),
        saved("Eğitim alanı parka dönüyor: eğitim küçülür, eşdeğerlik değerlendirilmez",
              [change("OKUL", "Kullanim", "park")]),
        saved("Azalan yoğunluk ve nizam değişikliği bulgu vermez", [change("KONUT", "KAKS", "1.00"),
                                                                     change("TICARET", "Nizam", "BL")]),
        saved("Kullanımı ve KAKS'ı olmayan alan: önceki değer boş, artış söylenmez",
              [change("BOS", "Kullanim", "konut"), change("BOS", "KAKS", "1.00")]),
        saved("Virgüllü sayı olduğu gibi saklanır", [change("KONUT", "Hmax", "12,50")]),
        saved("Aynı kimlikli taslak yenisiyle değişir; saklı sembol seçimi kalır", to_trade,
              zoning={"symbols": symbols, "drafts": [kept_draft]}),
        saved("Başka kimlikli taslak eklenir", to_trade, id_="T-2", zoning={"drafts": [kept_draft]}),
        saved("Plan yazmaz", to_trade, op="plan"),
        saved("Doğrulama", to_trade, op="validate"),
        {"name": "Taslağı sil: proje verisi boşalır; geri alma geri getirir", "setup": setup(zoning=removal_before),
         "steps": [{"op": "execute", "input": {"id": "T-1", "remove": True},
                    "result": {"status": "completed", "output": out_removed, "warnings": []},
                    "expect": {"revision": "changed", "canUndo": True, "settings": {"zoningPlan": None}}},
                   {"op": "undo", "returns": REMOVE, "expect": {"settings": {"zoningPlan": removal_before}}}]},
        refused("Sürüm yazısı geçersiz", {**ok(to_trade), "expectedRevision": "abc"}, "invalid_revision",
                "Beklenen sürüm “abc” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü "
                "belgeden ya da komutun planından alın.", "expectedRevision"),
        {"name": "Beklenen sürüm eski: çatışma", "steps": [{"op": "execute", "input": {**ok(to_trade),
                                                                                       "expectedRevision": "7"},
                                                           "result": {"status": "conflict", "error": {
                                                               "code": "revision_conflict", "message":
                                                               "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey "
                                                               "yazılmadı. Komutu çizimin şimdiki hâline göre yeniden "
                                                               "hazırlayın.", "path": "expectedRevision",
                                                               "revision": "$current"}}, "expect": SAME}]},
        refused("Kimlik boş", ok(to_trade, id=""), "invalid_input",
                "Taslağın kimliği boşluksuz başlayıp biten 1–64 karakterdir (yazılan “”).", "id"),
        refused("Kimliğin sonunda boşluk", ok(to_trade, id="T-1 "), "invalid_input",
                "Taslağın kimliği boşluksuz başlayıp biten 1–64 karakterdir (yazılan “T-1 ”).", "id"),
        refused("Olmayan taslak silinemez", {"id": "T-9", "remove": True}, "unknown_draft",
                "“T-9” kimlikli plan değişikliği taslağı projede yok.", "id"),
        refused("Karar katmanı yok", ok(to_trade), "layer_not_found",
                "“İmar planı › Karar alanları” katmanı yok; imar-plani profilini uygulayın (cad.profiles.apply).",
                None, d=setup([o for o in OBJECTS if o[1] == "plan"], decisions=False)),
        refused("Plan yazılmamış", ok(to_trade, plan=" "), "invalid_input",
                "Değişecek planı yazın: plan sınırının PlanKimlik'i.", "plan"),
        refused("Çizimde olmayan plan", ok(to_trade, plan="UİP-9"), "unknown_plan",
                "“UİP-9” kimlikli plan sınırı çizimde yok.", "plan"),
        refused("Gerekçesiz taslak (m. 9(4))", ok(to_trade, reason="  "), "invalid_input",
                f"Değişikliğin gerekçesini yazın: {MPYY} m. 9(4) plan değişikliğinde gerekçeyi ve gereklilik "
                "analizlerini açıklayan plan raporunu ister.", "reason"),
        refused("Değişikliksiz taslak", ok([]), "invalid_input",
                "Taslakta en az bir değişiklik olmalı: karar alanı, alanı ve yeni değeri.", "changes"),
        refused("Aynı alanın aynı alanı iki kez", ok([change("PARK", "Kullanim", "ticaret"),
                                                      change("PARK", "Kullanim", "konut")]), "invalid_input",
                f"“{U('PARK')}” karar alanının Kullanim alanı iki kez değişiyor.", "changes[1]"),
        refused("Kimlik UUID değil", ok([{"uid": "x", "field": "Kullanim", "value": "ticaret"}]), "invalid_uid",
                "“x” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir.", "changes[0].uid"),
        refused("Çizimde olmayan nesne", ok([{"uid": uid(99), "field": "Kullanim", "value": "ticaret"}]),
                "entity_not_found", f"“{uid(99)}” kimlikli nesne çizimde yok.", "changes[0].uid"),
        refused("Plan sınırı karar alanı değil", ok([change("PU", "Kullanim", "ticaret")]), "not_a_decision",
                f"“{U('PU')}” imar planının karar katmanında bir alan değil; taslak karar alanlarını değiştirir.",
                "changes[0].uid"),
        refused("Başka planın karar alanı", ok([change("OBUR", "Kullanim", "ticaret")]), "invalid_input",
                f"“{U('OBUR')}” karar alanı “UİP-1” planının değil; taslak bir planın karar alanlarını değiştirir.",
                "changes[0].uid"),
        refused("Taslağın değiştirmediği alan", ok([change("PARK", "Plan", "UİP-2")]), "invalid_input",
                f"Taslak “Plan” alanını değiştirmez; değiştirdikleri: {', '.join(FIELDS)}.", "changes[0].field"),
        refused("Boş değer", ok([change("PARK", "Kullanim", " ")]), "invalid_input",
                "Kullanim için yeni değeri yazın; taslak bir değeri silmez.", "changes[0].value"),
        refused("Listede olmayan kullanım", ok([change("PARK", "Kullanim", "otel")]), "invalid_input",
                "“otel” kullanım listesinde yok (plan-kullanim).", "changes[0].value"),
        refused("Listede olmayan nizam", ok([change("TICARET", "Nizam", "X")]), "invalid_input",
                "“X” yapı nizamı listesinde yok (yapi-nizami).", "changes[0].value"),
        refused("Eksi sayı", ok([change("TICARET", "KAKS", "-1")]), "invalid_input",
                "KAKS için “-1” 0 ya da daha büyük bir sayı değil.", "changes[0].value"),
        refused("Sayı değil", ok([change("TICARET", "Hmax", "on iki")]), "invalid_input",
                "Hmax için “on iki” 0 ya da daha büyük bir sayı değil.", "changes[0].value"),
        refused("Değer zaten öyle (virgüllü yazılmış aynı sayı)", ok([change("TICARET", "KAKS", "1,5")]),
                "invalid_input", f"“{U('TICARET')}” karar alanının KAKS değeri zaten “1.50”; değişiklik yok.",
                "changes[0].value"),
    ]


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.zoning.draft", "commandVersion": 1,
           "title": "Plan değişikliği taslağı",
           "note": "ADR 5040. Bağımsız başvuru scripts/fixtures/zoning_draft_cases.py: Mekânsal Planlar Yapım "
                   "Yönetmeliği m. 4, 9(4) ve 26'nın kuralları elle, alanlar dikdörtgenlerin shoelace alanı; KentOS "
                   "kodu yok.", "setup": setup(), "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        same = OUT.exists() and OUT.read_text("utf-8") == text
        print("Plan değişikliği taslağı ortak durumları güncel." if same else f"{OUT.relative_to(ROOT)} güncel değil.")
        return 0 if same else 1
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
