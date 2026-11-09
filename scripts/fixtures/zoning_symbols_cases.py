#!/usr/bin/env python3
"""The shared cases of `cad.zoning.symbolize` (Sembolü karardan ver; plan/map-cad.md MAP-09, docs/adr/4240), without
KentOS code.

    python3 scripts/fixtures/zoning_symbols_cases.py           # writes fixtures/commands/v1/cad.zoning.symbolize.json
    python3 scripts/fixtures/zoning_symbols_cases.py --check   # writes nothing; compares

The table, worked out here again from the system library's items and the profile's `plan-kullanim` list: at a level
(uip: MPYY › Uygulama imar planı, nip: MPYY › Nazım imar planı) a use takes the level's fill symbol whose name is the
use's own, written alike (Turkish lower case, î as i, a trailing “ (…)” left out), when exactly one is; the script
checks that catalog/v1/profiles/imar-plani-semboller.json says the same. The rules, by hand:

- a decision area (an area on İmar planı › Karar alanları) belongs to the plan boundary its `Plan` names, or to the
  only one when it names none; the plan's `Seviye` is its level;
- states in this order: `noPlan`, `noLevel`, `noUse` (no `Kullanim`), `unknownUse` (not in the list); then the user's
  pick for the level and use, else the table's: none is `unmapped`; the area's symbol already that one is `same`,
  else `set` (the symbol changes; `was` the one it had);
- only `symbol` is written; the attributes stay; refusals as the contract lists them.
"""

import copy
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.zoning.symbolize.json"
LIBRARY = json.loads((ROOT / "crates/native/style/assets/system-library.kstil").read_text("utf-8"))["items"]
PROFILE = json.loads((ROOT / "catalog/v1/profiles/imar-plani.json").read_text("utf-8"))
TABLE = json.loads((ROOT / "catalog/v1/profiles/imar-plani-semboller.json").read_text("utf-8"))
X0, Y0 = 500000.0, 4400000.0
STEP = "Sembolü karardan ver"
SAME = {"revision": "same", "canUndo": False}
BRANCHES = {"uip": "Uygulama imar planı", "nip": "Nazım imar planı"}
USES = {v["code"]: v["label"] for v in next(d for d in PROFILE["domains"] if d["id"] == "plan-kullanim")["values"]}


def alike(name):
    name = name.replace("İ", "i").replace("I", "ı").lower().replace("î", "i")
    return name[: -len(" (…)")].strip() if name.endswith(" (…)") else name.strip()


def fills(level):
    return [i for i in LIBRARY if i["kind"] == "symbol" and i["path"][:2] == ["MPYY", BRANCHES[level]]
            and (i.get("symbol") or {}).get("type") == "fill"]


def own_table():
    """The table by the rule, on its own: {level: {use: symbol id}}."""
    out = {}
    for level in BRANCHES:
        out[level] = {}
        for code, label in USES.items():
            same = [i["id"] for i in fills(level) if alike(i["name"]) == alike(label)]
            if len(same) == 1:
                out[level][code] = same[0]
    return out


KNOWN = own_table()
shipped = {lv["level"]: {u["code"]: u["symbol"] for u in lv["uses"]} for lv in TABLE["levels"]}
assert shipped == KNOWN, f"the shipped table is not the rule's: {shipped} != {KNOWN}"
assert {lv["level"]: [c["id"] for c in lv["choices"]] for lv in TABLE["levels"]} == {
    level: [i["id"] for i in fills(level)] for level in BRANCHES}, "the choices are not the level's fill symbols"


def uid(n):
    return f"0192f5c0-4240-7000-8000-{n:012x}"


def node(id_, name, children=None, locked=False):
    return {"id": id_, "name": name, "type": "group" if children is not None else "layer", "visible": True,
            "locked": locked, "expanded": True, "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25},
            "children": children or []}


def rect(x0, y0, x1, y1):
    return [{"x": X0 + x0, "y": Y0 + y0}, {"x": X0 + x1, "y": Y0 + y0}, {"x": X0 + x1, "y": Y0 + y1},
            {"x": X0 + x0, "y": Y0 + y1}]


def plan(kimlik, x, seviye):
    attrs = {"PlanKimlik": kimlik, "Durum": "yururlukte"}
    if seviye:
        attrs["Seviye"] = seviye
    return ("plan", rect(x, 0, x + 90, 100), attrs, None)


def karar(kimlik, x, y, plan_=None, kullanim=None, symbol=None):
    attrs = {"KararKimlik": kimlik}
    if plan_ is not None:
        attrs["Plan"] = plan_
    if kullanim is not None:
        attrs["Kullanim"] = kullanim
    return ("karar", rect(x, y, x + 20, y + 20), attrs, symbol)


OBJECTS = [
    ("PU", plan("UİP-1", 0, "uip")),
    ("PN", plan("NİP-1", 200, "nip")),
    ("PX", plan("PX", 400, None)),
    ("PC", plan("ÇDP-1", 600, "cdp")),
    ("PARK", karar("K1", 0, 0, "UİP-1", "park")),
    ("TICARET", karar("K2", 30, 0, "UİP-1", "ticaret", symbol="mpyy.uip.calisma.ticaret-alani")),
    ("KONUT", karar("K3", 60, 0, "UİP-1", "konut")),
    ("YANLIS", karar("K4", 0, 30, "UİP-1", "ticaret", symbol="mpyy.uip.acik-yesil.park")),
    ("SPOR", karar("K5", 200, 0, "NİP-1", "spor")),
    ("NPARK", karar("K6", 230, 0, "NİP-1", "park")),
    ("YOKPLAN", karar("K7", 30, 30, "YOK", "park")),
    ("YAZILMAMIS", karar("K8", 60, 30, None, "park")),
    ("KULLANIMSIZ", karar("K9", 0, 60, "UİP-1")),
    ("OTEL", karar("K10", 30, 60, "UİP-1", "otel")),
    ("KADEMESIZ", karar("K11", 400, 0, "PX", "park")),
    ("CDP", karar("K12", 600, 0, "ÇDP-1", "park")),
]
SLOT = {name: i + 1 for i, (name, _) in enumerate(OBJECTS)}


def U(name):
    return uid(SLOT[name])


def entity(i, layer, pts, attrs, symbol):
    e = {"kind": "polygon", "id": i, "layerId": layer, "attrs": attrs, "pts": pts}
    if symbol:
        e["symbol"] = symbol
    return e


ENTS = [entity(i + 1, *o) for i, (_, o) in enumerate(OBJECTS)]


def setup(ents=None, locked=False, decisions=True):
    ents = copy.deepcopy(ents if ents is not None else ENTS)
    kids = [node("plan", "Plan sınırı")] + ([node("karar", "Karar alanları", locked=locked)] if decisions else [])
    return {"format": "kentos.document", "version": 2, "name": "Sembolü karardan ver",
            "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                         "plotScale": 1000.0, "workspace": "cad", "cadWork": "map"},
            "origin": {"x": X0, "y": Y0}, "layers": [node("0", "0"), node("imar", "İmar planı", kids)],
            "activeLayer": "plan", "entities": ents, "uids": [uid(i + 1) for i in range(len(ents))],
            "styles": {"items": [], "categories": []}}


def symbolized(ents, names=None, picks=()):
    """The answer and the symbols written, by hand."""
    picks = {(p["level"], p["use"]): p["symbol"] for p in picks}
    plans = [(e["attrs"].get("PlanKimlik", ""), e["attrs"].get("Seviye", "")) for e in ents if e["layerId"] == "plan"]
    chosen = [e for e in ents if e["layerId"] == "karar"]
    if names is not None:
        wanted = {SLOT[n] for n in names}
        chosen = [e for e in chosen if e["id"] in wanted]
    rows, written = [], {}
    for e in chosen:
        a = e["attrs"]
        written_plan = a.get("Plan", "").strip()
        if written_plan:
            found = next((p for p in plans if p[0] == written_plan), None)
        else:
            found = plans[0] if len(plans) == 1 else None
        usage = a.get("Kullanim", "").strip()
        row = {"uid": uid(e["id"]), "plan": found[0] if found else written_plan,
               "level": found[1] if found else "", "use": usage}
        if "symbol" in e:
            row["was"] = e["symbol"]
        if found is None:
            row["state"] = "noPlan"
        elif not row["level"]:
            row["state"] = "noLevel"
        elif not usage:
            row["state"] = "noUse"
        elif usage not in USES:
            row["state"] = "unknownUse"
        else:
            key = (row["level"], usage)
            symbol, source = (picks[key], "user") if key in picks else (KNOWN.get(row["level"], {}).get(usage), "kentos")
            if symbol is None:
                row["state"] = "unmapped"
            else:
                row["symbol"], row["from"] = symbol, source
                if e.get("symbol") == symbol:
                    row["state"] = "same"
                else:
                    row["state"] = "set"
                    written[str(e["id"])] = symbol
        rows.append(row)
    count = lambda *states: sum(1 for r in rows if r["state"] in states)  # noqa: E731
    out = {"areas": rows, "set": count("set"), "same": count("same"), "unmapped": count("unmapped"),
           "skipped": count("noPlan", "noLevel", "noUse", "unknownUse"), "revision": "$current"}
    return out, written


def done(name, names=None, picks=(), op="execute", ents=None, note=None):
    ents = ents if ents is not None else ENTS
    out, written = symbolized(ents, names, picks)
    input_ = {}
    if names is not None:
        input_["uids"] = [U(n) for n in names]
    if picks:
        input_["symbols"] = list(picks)
    case = {"name": name}
    if note:
        case["note"] = note
    if ents is not ENTS:
        case["setup"] = setup(ents)
    if op != "execute" or not written:
        result = {"status": "completed", "output": None if op == "validate" else out, "warnings": []}
        case["steps"] = [{"op": op, "input": input_, "result": result, "expect": SAME}]
        return case
    after = {k: {"symbol": v, "attrs": ents[int(k) - 1]["attrs"]} for k, v in written.items()}
    before = {k: {"symbol": ents[int(k) - 1].get("symbol"), "attrs": ents[int(k) - 1]["attrs"]} for k in written}
    case["steps"] = [
        {"op": "execute", "input": input_, "result": {"status": "completed", "output": out, "warnings": []},
         "expect": {"revision": "changed", "canUndo": True, "fields": after}},
        {"op": "undo", "returns": STEP, "expect": {"fields": before}},
    ]
    return case


def refused(name, input_, code, message, path_=None, d=None):
    case = {"name": name}
    if d is not None:
        case["setup"] = d
    error = {"code": code, "message": message}
    if path_:
        error["path"] = path_
    case["steps"] = [{"op": "execute", "input": input_, "result": {"status": "failed", "error": error},
                      "expect": SAME}]
    return case


def pick(level, use, symbol):
    return {"level": level, "use": use, "symbol": symbol}


def cases():
    gelisme = "mpyy.uip.konut.gelisme-konut-alani"
    arboretum = "mpyy.uip.acik-yesil.arboretum-botanik-parki"
    one_plan = [e for e in ENTS if e["id"] in (SLOT["PU"], SLOT["PARK"], SLOT["YAZILMAMIS"])]
    one_plan = [dict(e, id=i + 1) for i, e in enumerate(one_plan)]
    every = None
    return [
        done("Bütün karar alanları KentOS'un tablosuyla: park ve spor sembolünü alır, yanlış sembollü ticaret "
             "düzelir, konut ve NİP parkı açık kalır, planı, kademesi ya da kullanımı olmayan atlanır; geri alma",
             every, note="Yalnız symbol yazılır; öznitelikler (karar) olduğu gibi kalır."),
        done("Kullanıcının seçtiği sembol: UİP konut alanı Gelişme konut alanı", every,
             picks=(pick("uip", "konut", gelisme),)),
        done("Kullanıcının seçimi tablodan önce gelir: UİP parkı Arboretum - botanik parkı",
             ["PARK", "TICARET"], picks=(pick("uip", "park", arboretum),)),
        done("Seçili alanlar: yalnız park ve spor", ["PARK", "SPOR"]),
        done("Verilen sıra değil, çizimin sırası", ["SPOR", "YANLIS", "PARK"]),
        done("Zaten kararıyla aynı sembol: hiçbir şey yazılmaz", ["TICARET"]),
        done("NİP kademesi: spor alanı tablodan, park açık", ["SPOR", "NPARK"]),
        done("Çevre düzeni planının sembolleri yok: açık kalır", ["CDP"]),
        done("Tek plan sınırlı çizimde planı yazılmamış alan o planın", ents=one_plan),
        done("Planı yok, yazılmamış (iki plan varken), kademesi yok, kullanımı yok, bilinmeyen kullanım: atlanır",
             ["YOKPLAN", "YAZILMAMIS", "KULLANIMSIZ", "OTEL", "KADEMESIZ"]),
        done("Plan yazmaz", every, op="plan"),
        done("Doğrulama", ["PARK"], op="validate"),
        refused("Sürüm yazısı geçersiz", {"expectedRevision": "abc"}, "invalid_revision",
                "Beklenen sürüm “abc” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü "
                "belgeden ya da komutun planından alın.", "expectedRevision"),
        {"name": "Beklenen sürüm eski: çatışma", "steps": [{"op": "execute", "input": {"expectedRevision": "7"},
                                                           "result": {"status": "conflict", "error": {
                                                               "code": "revision_conflict", "message":
                                                               "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey "
                                                               "yazılmadı. Komutu çizimin şimdiki hâline göre yeniden "
                                                               "hazırlayın.", "path": "expectedRevision",
                                                               "revision": "$current"}}, "expect": SAME}]},
        refused("Seçimin kademesinin sembolleri yok", {"symbols": [pick("cdp", "park", arboretum)]}, "invalid_input",
                "“cdp” kademesinin MPYY sembolleri yok; KentOS uygulama (uip) ve nazım (nip) imar planı kademelerinin "
                "sembollerini bilir.", "symbols[0].level"),
        refused("Seçimin kullanımı listede yok", {"symbols": [pick("uip", "otel", arboretum)]}, "invalid_input",
                "“otel” kullanım listesinde yok (plan-kullanim).", "symbols[0].use"),
        refused("Aynı kademe ve kullanım iki kez", {"symbols": [pick("uip", "konut", gelisme),
                                                              pick("uip", "konut", gelisme)]}, "invalid_input",
                "“uip” kademesinde “konut” kullanımının sembolü iki kez verildi.", "symbols[1]"),
        refused("Başka kademenin sembolü", {"symbols": [pick("uip", "spor", "mpyy.nip.sosyal.spor-alani")]},
                "unknown_plan_symbol", "“mpyy.nip.sosyal.spor-alani”, MPYY › Uygulama imar planı alan sembolleri "
                "arasında yok; kademenin sembollerinden birini seçin.", "symbols[0].symbol"),
        refused("Çizgi sembolü alan sembolü değil", {"symbols": [pick("uip", "konut", "mpyy.uip.idari.mahalle-siniri")]},
                "unknown_plan_symbol", "“mpyy.uip.idari.mahalle-siniri”, MPYY › Uygulama imar planı alan sembolleri "
                "arasında yok; kademenin sembollerinden birini seçin.", "symbols[0].symbol"),
        refused("Karar katmanı yok", {}, "layer_not_found", "“İmar planı › Karar alanları” katmanı yok; imar-plani "
                "profilini uygulayın (cad.profiles.apply).", None,
                d=setup([e for e in ENTS if e["layerId"] == "plan"], decisions=False)),
        refused("Aynı alan iki kez", {"uids": [U("PARK"), U("PARK")]}, "invalid_input",
                f"“{U('PARK')}” alanı iki kez verildi.", "uids[1]"),
        refused("Kimlik UUID değil", {"uids": ["x"]}, "invalid_uid",
                "“x” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir.", "uids[0]"),
        refused("Çizimde olmayan nesne", {"uids": [uid(99)]}, "entity_not_found",
                f"“{uid(99)}” kimlikli nesne çizimde yok.", "uids[0]"),
        refused("Plan sınırı karar alanı değil", {"uids": [U("PARK"), U("PU")]}, "not_a_decision",
                f"“{U('PU')}” imar planının karar katmanında bir alan değil; sembol karar alanlarına verilir.",
                "uids[1]"),
        refused("Kilitli karar katmanı: hiçbir şey yazılmaz", {}, "layer_locked",
                "“İmar planı / Karar alanları” katmanı kilitli: karar alanının sembolü yazılamaz. Kilidi açın.", None,
                d=setup(locked=True)),
        refused("Kilitli katmanda seçili alan", {"uids": [U("TICARET"), U("PARK")]}, "layer_locked",
                "“İmar planı / Karar alanları” katmanı kilitli: karar alanının sembolü yazılamaz. Kilidi açın.",
                "uids[1]", d=setup(locked=True)),
    ]


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.zoning.symbolize", "commandVersion": 1,
           "title": "Sembolü karardan ver", "note": "ADR 4240. Bağımsız başvuru scripts/fixtures/zoning_symbols_cases.py: "
           "tablo sistem kitaplığından ad eşitliğiyle betikte yeniden kurulur ve katalogdakiyle karşılaştırılır; "
           "planın, kademenin ve kullanımın kuralları elle; KentOS kodu yok.", "setup": setup(), "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        same = OUT.exists() and OUT.read_text("utf-8") == text
        print("Sembolü karardan ver ortak durumları güncel." if same else f"{OUT.relative_to(ROOT)} güncel değil.")
        return 0 if same else 1
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
