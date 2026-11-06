#!/usr/bin/env python3
"""The shared cases of `road.guardrails.edit`, `.query` and `.draw` (plan/road.md Y11.1, docs/adr/0909 §1;
ROAD-JUNC-12).

The corridor's crosses every `step` metres are road_stakeout_cases.py's (the corridor's own reference: hinge and
daylight points); the rule is written here from the ADR, no KentOS code: a side's fill height is the hinge above the
daylight point, its slope their run per that rise; a barrier is warranted where any row holds (fill at least `height`,
steeper than 1:`slope` when given); a run of warranted crosses is lengthened before and after (on the right the
approach length before, on the left after), held to the corridor, and runs closer than `gap` join. The rail is drawn at
each cross's hinge plus the offset outward, its ends between crosses read linearly, placed on the alignment by
road_alignment_cases.py's reference at 40 digits. The rule's numbers are the cases' own, not a standard's.

    uv run --no-project --python 3.11 --with mpmath --with triangle --with numpy --with pyproj python3 \
        scripts/fixtures/road_guardrail_cases.py [--check]
"""

import copy
import json
import math
import sys
from pathlib import Path

import mpmath as mp

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_alignment_cases as aref  # noqa: E402
import road_corridor_command_cases as ccmd  # noqa: E402
import road_grades  # noqa: E402
import road_stakeout_cases as so  # noqa: E402

mp.mp.dps = 40
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
completed, failed, step, case = ccmd.completed, ccmd.failed, ccmd.step, ccmd.case
UNCHANGED, STEPPED, CONFLICT, BAD_REVISION = ccmd.UNCHANGED, ccmd.STEPPED, ccmd.CONFLICT, ccmd.BAD_REVISION
W = 1e-6
CODES = ["left.hinge", "left.daylight", "right.hinge", "right.daylight"]
NOT_ROAD = failed("not_a_road_project", "Otokorkuluk yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.")

RULE = {"source": "Ortak durumların deneme tablosu (bir standardın değeri değil)",
        "warrants": [{"height": 1.0, "slope": 1.75}, {"height": 1.5}], "approach": 30.0, "leaving": 15.0, "gap": 40.0}
G1 = {"id": "o-1", "name": "Ana korkuluk", "corridor": "c-1", "corridorRevision": 1, "revision": 1, "rule": RULE,
      "step": 20.0}
G2 = {"id": "o-2", "name": "Yan korkuluk", "corridor": "c-2", "revision": 1,
      "rule": {**RULE, "warrants": [{"height": 0.5}]}, "step": 25.0, "offset": 0.5}
START = [G1, G2]
# A corridor whose regions cross: it does not build.
C4 = {**copy.deepcopy(ccmd.C1), "id": "c-4", "name": "Kurulamayan koridor",
      "regions": [{"from": 0.0, "to": 100.0, "template": "tk-1"}, {"from": 50.0, "to": 200.0, "template": "tk-1"}]}
CORRIDORS = so.CORRIDORS + [C4]


def node(id_, name, **flags):
    out = {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
           "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}
    out.update(flags)
    return out


def road_of(guardrails):
    out = ccmd.road_of(CORRIDORS)
    if guardrails:
        out["guardrails"] = copy.deepcopy(list(guardrails))
    return out


def setup(guardrails=START, work="road"):
    base = so.setup(work=work)
    base["layers"] = base["layers"] + [node("korkuluk", "Korkuluk"), node("kilitli", "Kilitli", locked=True),
                                       node("gizli", "Gizli", visible=False),
                                       node("grup", "Yol", type="group", children=[node("yol-korkuluk", "Yol korkuluk")])]
    if work == "road":
        base["settings"]["road"] = road_of(guardrails)
    return base


FIRST = len(setup()["entities"]) + 1


class Refused(Exception):
    def __init__(self, code, message, path=None):
        super().__init__(message)
        self.result = failed(code, message, path)


def bad(path, words):
    return Refused("invalid_guardrail", words, path or None)


def corridor(cid):
    return next(c for c in CORRIDORS if c["id"] == cid)


def find(list_, name, path="guardrail"):
    for g in list_:
        if g["id"] == name:
            return g
    for g in list_:
        if g["name"].lower() == name.lower():
            return g
    raise Refused("unknown_guardrail", f"“{name}” kimlikli ya da adlı otokorkuluk projede yok.", path)


def runs_of(samples, rule, right, lo, hi):
    """A side's runs by the rule, `samples` (station, fill or None) in station order."""
    def warranted(f):
        h, n = f
        return any(h >= w["height"] and ("slope" not in w or n <= w["slope"]) for w in rule["warrants"])
    out, open_ = [], None
    for s, f in samples:
        if f is not None and warranted(f):
            if open_ is None:
                open_ = {"warranted": [s, s], "height": f[0], "slope": f[1]}
            else:
                open_["warranted"][1] = s
                open_["height"] = max(open_["height"], f[0])
                open_["slope"] = min(open_["slope"], f[1])
        elif open_ is not None:
            out.append(open_)
            open_ = None
    if open_ is not None:
        out.append(open_)
    before, after = (rule["approach"], rule["leaving"]) if right else (rule["leaving"], rule["approach"])
    joined = []
    for r in out:
        r["from"] = max(r["warranted"][0] - before, lo)
        r["to"] = min(r["warranted"][1] + after, hi)
        if joined and r["from"] - joined[-1]["to"] < rule["gap"]:
            last = joined[-1]
            last["to"] = max(last["to"], r["to"])
            last["warranted"][1] = r["warranted"][1]
            last["height"] = max(last["height"], r["height"])
            last["slope"] = min(last["slope"], r["slope"])
        else:
            joined.append(r)
    return joined


def read(g, guardrails):
    """The guardrail's report, its warnings, its sides' crosses and runs."""
    c = corridor(g["corridor"])
    try:
        table, warnings = so.table(c, {"corridor": c["id"], "step": g["step"], "codes": CODES})
    except ccmd.Refused as r:
        why = f"“{c['name']}” koridoru kurulamıyor: {r.result['error']['message']}"
        raise bad("corridor", f"“{g['name']}” otokorkuluğunun koridoru okunamıyor: {why}")
    warnings = [w for w in warnings if w["code"] != "stakeout_unsolved"]
    rows = table["rows"]
    stations = []
    for r in rows:
        if not stations or stations[-1] != r["station"]:
            stations.append(r["station"])
    sides = {"left": [], "right": []}
    for word in ("left", "right"):
        for s in stations:
            if sides[word] and sides[word][-1]["station"] == s:
                continue
            at = {code: next((r for r in rows if r["station"] == s and r["code"] == f"{word}.{code}"), None)
                  for code in ("hinge", "daylight")}
            h, d = at["hinge"], at["daylight"]
            if h is None:
                continue
            fill = None
            if d is not None and h["z"] - d["z"] > 1e-9:
                rise = h["z"] - d["z"]
                fill = (rise, abs(d["offset"] - h["offset"]) / rise)
            sides[word].append({"station": s, "hinge": h["offset"], "fill": fill})
    lo = min(r["from"] for r in c["regions"])
    hi = max(r["to"] for r in c["regions"])
    runs = []
    for word in ("left", "right"):
        for r in runs_of([(x["station"], x["fill"]) for x in sides[word]], g["rule"], word == "right", lo, hi):
            r["side"] = word
            runs.append(r)
    text = lambda s: ccmd.km_text(c["alignment"], s)  # noqa: E731
    rows_out = [{"side": r["side"], "from": r["from"], "to": r["to"], "fromText": text(r["from"]),
                 "toText": text(r["to"]), "length": r["to"] - r["from"], "warranted": r["warranted"],
                 "height": r["height"], "slope": r["slope"]} for r in runs]
    stale = g.get("corridorRevision", 0) != c.get("revision", 0)
    if stale:
        warnings.append({"code": "guardrail_stale", "path": "guardrail",
                         "message": f"“{g['name']}” otokorkuluğunun koridoru değişti; korkulukları denetleyip koridora "
                                    "yeniden bağlayın."})
    report = {"id": g["id"], "name": g["name"], "corridor": g["corridor"], "revision": g["revision"], "stale": stale,
              "source": g["rule"]["source"], "crosses": len(stations), "runs": rows_out,
              "left": sum(r["length"] for r in rows_out if r["side"] == "left"),
              "right": sum(r["length"] for r in rows_out if r["side"] == "right")}
    return report, warnings, sides, runs


def listed(list_):
    return [{"id": g["id"], "name": g["name"], "corridor": g["corridor"], "revision": g["revision"]} for g in list_]


# ── road.guardrails.edit ──────────────────────────────────────────────────────────────────────────────────────────

def edit(input_, start=START):
    """The list after the edit, the target's id; or Refused."""
    own = copy.deepcopy(list(start))
    op = input_["operation"]
    found = None
    if input_.get("guardrail"):
        try:
            found = find(own, input_["guardrail"])
        except Refused:
            found = None
    if op != "set" and found is None:
        find(own, input_.get("guardrail", ""))
    cor = None
    if "corridor" in input_:
        cor = next((c for c in CORRIDORS if c["id"] == input_["corridor"]), None) or next(
            (c for c in CORRIDORS if c["name"].lower() == input_["corridor"].lower()), None)
        if cor is None:
            raise Refused("unknown_corridor", f"“{input_['corridor']}” kimlikli ya da adlı koridor projede yok.",
                          "corridor")
    if op == "remove":
        own.remove(found)
        target = None
    elif op == "rebind":
        now = corridor(found["corridor"]).get("revision", 0)
        d = copy.deepcopy(found)
        d["corridorRevision"] = now
        if d != found:
            d["revision"] = found["revision"] + 1
        own[own.index(found)] = d
        target = d["id"]
    elif found is not None:
        if not any(k in input_ for k in ("name", "corridor", "rule", "step", "offset")):
            raise bad("", "Değiştirilecek bir şey yok: name, corridor, rule, step ya da offset verin.")
        d = copy.deepcopy(found)
        if "name" in input_:
            d["name"] = input_["name"]
        if cor is not None:
            d["corridor"] = cor["id"]
            d["corridorRevision"] = cor.get("revision", 0)
        for k in ("rule", "step", "offset"):
            if k in input_:
                d[k] = input_[k]
        if d.get("offset") == 0.0:
            d.pop("offset")
        if d != found:
            d["revision"] = found["revision"] + 1
        own[own.index(found)] = d
        target = d["id"]
    else:
        missing = next((f for f in ("name", "corridor", "rule", "step") if f not in input_), None)
        if missing:
            raise bad(missing, "Yeni otokorkuluk için name, corridor, rule ve step verilmeli.")
        gid = input_.get("guardrail") or next(f"o-{n}" for n in range(1, 999) if all(g["id"] != f"o-{n}" for g in own))
        d = {"id": gid, "name": input_["name"], "corridor": cor["id"], "corridorRevision": cor.get("revision", 0),
             "revision": 1, "rule": input_["rule"], "step": input_["step"]}
        if input_.get("offset"):
            d["offset"] = input_["offset"]
        if not d["corridorRevision"]:
            d.pop("corridorRevision")
        own.append(d)
        target = gid
    rule_problem(own)
    if target is not None:
        read(next(g for g in own if g["id"] == target), own)
    return own, target


def rule_problem(list_):
    """The definition's rules (contracts' guardrails_problem), the field of the first broken."""
    names = set()
    for g in list_:
        def no(field, words):
            raise bad(field, f"Otokorkuluk kurallarına uymuyor: {words}.")
        name = g["name"]
        if not name.strip() or name.strip() != name or len(name) > 120:
            no("name", "otokorkuluğun adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter")
        if name.lower() in names:
            no("name", "bu adda başka bir otokorkuluk var")
        names.add(name.lower())
        r = g["rule"]
        s = r["source"]
        if not s.strip() or s.strip() != s or len(s) > 200:
            no("rule.source", "kuralın kaynağı boş olmamalı (standart ve tablosu), başında ve sonunda boşluk olmamalı, "
                              "en çok 200 karakter")
        if not 1 <= len(r["warrants"]) <= 32:
            no("rule.warrants", "kuralın 1 ile 32 arası satırı olur")
        for j, w in enumerate(r["warrants"]):
            if not 0 <= w["height"] <= 1000:
                no(f"rule.warrants[{j}].height", "dolgu yüksekliği 0 ile 1000 m arası olur")
            if "slope" in w and not 0 < w["slope"] <= 1000:
                no(f"rule.warrants[{j}].slope", "şev (bir düşeye yatay) 0'dan büyük, en çok 1000 olur")
        for f in ("approach", "leaving", "gap"):
            if not 0 <= r[f] <= 10000:
                no(f"rule.{f}", "uzunluk 0 ile 10 000 m arası olur")
        if not 0 < g["step"] <= 1000:
            no("step", "okuma aralığı 0'dan büyük, en çok 1000 m olur")
        if abs(g.get("offset", 0.0)) > 100:
            no("offset", "korkuluğun şev başından uzaklığı en çok 100 m olur")


def edit_cases():
    cases = []

    def ok(name, input_, op="execute", undo=False):
        own, target = edit(input_)
        report = None if target is None else read(next(g for g in own if g["id"] == target), own)[0]
        out = {"guardrails": listed(own), "revision": "$current"}
        if report is not None:
            out["guardrail"] = report
        if op == "validate":
            result, expect = completed(None), {**UNCHANGED, "road": road_of(START)}
        elif op == "plan":
            result, expect = completed(out), {**UNCHANGED, "road": road_of(START)}
        else:
            result, expect = completed(out), {**STEPPED, "road": road_of(own)}
        steps = [step(op, input_, result, expect)]
        if undo:
            steps.append({"op": "undo", "returns": "Otokorkuluk", "expect": {"road": road_of(START), "canRedo": True}})
        cases.append(case(name, steps))

    def no(name, input_, result=None, setup_=None):
        if result is None:
            try:
                edit(input_)
            except Refused as r:
                result = r.result
            else:
                raise AssertionError(f"{name}: refuses")
        road = None if setup_ is not None else road_of(START)
        cases.append(case(name, [step("execute", input_, result, {**UNCHANGED, "road": road})], setup_))

    new = {"operation": "set", "name": "Kısa korkuluk", "corridor": "Ana koridor", "rule": RULE, "step": 10.0}
    ok("Yeni otokorkuluk, koridorun adıyla", new)
    ok("Yeni, kimliğiyle ve dışa 0,75 m", {**new, "guardrail": "sağ-korkuluk", "offset": 0.75})
    ok("Tek satırlı kural, her şevde", {**new, "rule": {**RULE, "warrants": [{"height": 2.0}]}})
    ok("Aralığı büyütmek korkulukları birleştirir", {"operation": "set", "guardrail": "o-1",
                                                     "rule": {**RULE, "gap": 300.0}})
    ok("Adını değiştir", {"operation": "set", "guardrail": "Ana korkuluk", "name": "Ana yol korkuluğu"})
    ok("Okuma aralığını değiştir", {"operation": "set", "guardrail": "o-1", "step": 10.0})
    ok("Başka koridora taşı", {"operation": "set", "guardrail": "o-1", "corridor": "c-2"})
    ok("Eskiyen korkuluğu yeniden bağla", {"operation": "rebind", "guardrail": "o-2"})
    ok("Kaldır", {"operation": "remove", "guardrail": "Yan korkuluk"})
    ok("Planı", new, op="plan")
    ok("Doğrulama", new, op="validate")
    ok("Tek geri alma adımı", new, undo=True)
    no("Yol projesi değil", new, NOT_ROAD, setup(work="map"))
    no("Olmayan korkuluğu kaldır", {"operation": "remove", "guardrail": "yok"})
    no("Olmayan korkuluğu bağla", {"operation": "rebind", "guardrail": "yok"})
    no("Olmayan koridor", {**new, "corridor": "yok"})
    no("Kuralsız yeni", {k: v for k, v in new.items() if k != "rule"})
    no("Kurulamayan koridora", {**new, "corridor": "c-4"})
    no("Değiştirilecek bir şey yok", {"operation": "set", "guardrail": "o-1"})
    no("Kaynaksız kural", {**new, "rule": {**RULE, "source": " "}})
    no("Satırsız kural", {**new, "rule": {**RULE, "warrants": []}})
    no("Eksi yükseklik", {**new, "rule": {**RULE, "warrants": [{"height": -1.0}]}})
    no("Sıfır şev", {**new, "rule": {**RULE, "warrants": [{"height": 1.0, "slope": 0.0}]}})
    no("Eksi yaklaşma boyu", {**new, "rule": {**RULE, "approach": -5.0}})
    no("Sıfır okuma aralığı", {**new, "step": 0.0})
    no("Çok uzak korkuluk", {**new, "offset": 150.0})
    no("Alınmış ad", {**new, "name": "ana KORKULUK"})
    no("Geçersiz beklenen sürüm", {**new, "expectedRevision": "on"}, BAD_REVISION)
    no("Eski sürüme göre hazırlanmış", {**new, "expectedRevision": "41"}, CONFLICT)
    return cases


# ── road.guardrails.query ─────────────────────────────────────────────────────────────────────────────────────────

def broken_result():
    try:
        read({**G1, "id": "o-4", "name": "Bozuk", "corridor": "c-4"}, START)
    except Refused as r:
        return r.result
    raise AssertionError("refuses")


def query_cases():
    cases = []

    def ok(name, input_, guardrails=START):
        out, warns = [], []
        for g in guardrails if "guardrail" not in input_ else [find(guardrails, input_["guardrail"])]:
            try:
                r, w, _, _ = read(g, guardrails)
            except Refused:
                r, w = {"id": g["id"], "name": g["name"], "corridor": g["corridor"], "revision": g["revision"],
                        "stale": False, "source": g["rule"]["source"], "crosses": 0, "runs": [], "left": 0.0,
                        "right": 0.0}, []
            out.append(r)
            warns += w
        cases.append(case(name, [step("execute", input_, completed({"guardrails": out}, warns),
                                      {**UNCHANGED, "road": road_of(guardrails)})],
                          None if guardrails is START else setup(guardrails)))

    ok("Bütün korkuluklar", {})
    ok("Ana korkuluk", {"guardrail": "o-1"})
    ok("Adıyla, eskiyen yan korkuluk", {"guardrail": "yan KORKULUK"})
    ok("Kesitsiz koridorda korkuluk yok", {"guardrail": "o-3"},
       START + [{**G1, "id": "o-3", "name": "Yüzeysiz", "corridor": "c-3"}])
    ok("Sık okuma, sıkı aralık", {"guardrail": "o-1"}, [{**G1, "step": 5.0, "rule": {**RULE, "gap": 5.0}}])
    ok("Sol yanda yaklaşma boyu sonda", {"guardrail": "o-1"},
       [{**G1, "rule": {**RULE, "approach": 60.0, "leaving": 0.0, "gap": 0.0}}])
    ok("Kural tutmuyor: korkuluk yok", {"guardrail": "o-1"},
       [{**G1, "rule": {**RULE, "warrants": [{"height": 50.0}]}}])
    ok("Her dolguya korkuluk", {"guardrail": "o-1"}, [{**G1, "rule": {**RULE, "warrants": [{"height": 0.0}]}}])
    ok("Hiç korkuluk yok", {}, [])
    ok("Yalnız şevle: 1:1,6'dan dik her dolgu", {"guardrail": "o-1"},
       [{**G1, "rule": {**RULE, "warrants": [{"height": 0.0, "slope": 1.6}]}}])
    ok("10 m'de okuma", {"guardrail": "o-1"}, [{**G1, "step": 10.0}])
    ok("Uzatmasız, aralıksız: yalnız gerekli bölümler", {"guardrail": "o-1"},
       [{**G1, "rule": {**RULE, "approach": 0.0, "leaving": 0.0, "gap": 0.0}}])
    ok("Uzun uzatma koridorla sınırlı", {"guardrail": "o-1"},
       [{**G1, "rule": {**RULE, "approach": 1000.0, "leaving": 1000.0}}])
    ok("Dışa 2 m: korkuluklar aynı", {"guardrail": "o-1"}, [{**G1, "offset": 2.0}])
    ok("Aynı koridorda iki kural", {}, [G1, {**G1, "id": "o-5", "name": "Sıkı korkuluk",
                                            "rule": {**RULE, "warrants": [{"height": 2.5}]}}])
    ok("Bağlı yan korkuluk eskimez", {"guardrail": "o-2"}, [{**G2, "corridorRevision": 2}])
    ok("Kurulamayan koridordaki korkuluk listede boş", {},
       START + [{**G1, "id": "o-4", "name": "Bozuk", "corridor": "c-4"}])
    cases.append(case("Kurulamayan koridordaki korkuluk sorulunca", [step(
        "execute", {"guardrail": "o-4"}, broken_result(), UNCHANGED)],
        setup(START + [{**G1, "id": "o-4", "name": "Bozuk", "corridor": "c-4"}])))
    cases.append(case("Yol projesi değil", [step("execute", {}, NOT_ROAD, UNCHANGED)], setup(work="map")))
    cases.append(case("Olmayan korkuluk", [step("execute", {"guardrail": "yok"},
                                                failed("unknown_guardrail",
                                                       "“yok” kimlikli ya da adlı otokorkuluk projede yok.",
                                                       "guardrail"), UNCHANGED)]))
    return cases


# ── road.guardrails.draw ──────────────────────────────────────────────────────────────────────────────────────────

def place(alignment, s, o):
    b = ccmd.pcmd.ALIGNED[alignment]
    e, (x, y) = aref.place_at(b.els, mp.mpf(s))
    th = e.theta(mp.mpf(s) - e.station)
    return [float(x + mp.sin(th) * o), float(y - mp.cos(th) * o)], float(th)


def hinge_at(sides, s):
    i = sum(1 for x in sides if x["station"] <= s)
    a = sides[i - 1] if i > 0 else None
    b = sides[i] if i < len(sides) else None
    if a and b and b["station"] > a["station"]:
        return a["hinge"] + (b["hinge"] - a["hinge"]) * (s - a["station"]) / (b["station"] - a["station"])
    return (a or b)["hinge"] if (a or b) else 0.0


def drawing(g, labels):
    report, warnings, sides, runs = read(g, [g])
    c = corridor(g["corridor"])
    h = 2.5
    objects, length = [], 0.0
    for r in runs:
        right = r["side"] == "right"
        sd = sides[r["side"]]
        outward = g.get("offset", 0.0) * (1 if right else -1)
        stations = [r["from"]] + [x["station"] for x in sd if r["from"] + 1e-9 < x["station"] < r["to"] - 1e-9] + [r["to"]]
        pts = [place(c["alignment"], s, hinge_at(sd, s) + outward)[0] for s in stations]
        length += r["to"] - r["from"]
        objects.append({"pts": [{"x": p[0], "y": p[1]} for p in pts]})
        if labels:
            mid = (r["from"] + r["to"]) / 2
            p, th = place(c["alignment"], mid, hinge_at(sd, mid) + outward)
            angle = math.degrees(th)
            if not -90 <= angle <= 90:
                angle += -180 if angle > 0 else 180
            side = 1 if right else -1
            t = (f"Otokorkuluk km {ccmd.km_text(c['alignment'], r['from'])} – "
                 f"{ccmd.km_text(c['alignment'], r['to'])}, {r['to'] - r['from']:.1f} m")
            objects.append({"p": {"x": p[0] + math.sin(th) * h * side, "y": p[1] - math.cos(th) * h * side},
                            "text": t, "rotation": angle, "height": h})
    return {"guardrail": g["id"], "runs": len(runs), "length": length}, objects, warnings


def draw_cases():
    cases = []
    hidden = {"code": "layer_hidden", "message": "“Gizli” katmanı gizli; çizilen nesne görünmeyecek.", "path": "layerId"}

    def ok(name, input_, op="execute", undo=False, extra=(), guardrails=START):
        g = find(guardrails, input_["guardrail"])
        out, objects, warns = drawing(g, input_.get("labels", False))
        warns = list(extra) + warns
        out = {**out, "uids": "$any" if op == "execute" else [], "revision": "$current"}
        if op == "validate":
            result, expect = completed(None, warns), {**UNCHANGED, "road": road_of(guardrails)}
        elif op == "plan":
            result, expect = completed(out, warns), {**UNCHANGED, "road": road_of(guardrails)}
        else:
            result = completed(out, warns)
            expect = {**STEPPED, "road": road_of(guardrails), "ids": list(range(1, FIRST + len(objects))),
                      "fields": {str(FIRST + i): o for i, o in enumerate(objects)}}
        s = step(op, input_, result, expect)
        s["within"] = W
        steps = [s]
        if undo:
            steps.append({"op": "undo", "returns": "Otokorkuluk çiz",
                          "expect": {"ids": list(range(1, FIRST)), "canRedo": True}})
        cases.append(case(name, steps, None if guardrails is START else setup(guardrails)))

    def no(name, input_, result, setup_=None, road=START):
        cases.append(case(name, [step("execute", input_, result,
                                      {**UNCHANGED, "road": None if road is None else road_of(road)})], setup_))

    base = {"guardrail": "o-1", "layerId": "korkuluk"}
    ok("Ana korkuluk planda", base)
    ok("Yazılarıyla", {**base, "labels": True})
    ok("Adıyla, eskiyen yan korkuluk dışa 0,5 m", {**base, "guardrail": "yan KORKULUK"})
    ok("Gizli katmana uyarıyla", {**base, "layerId": "gizli"}, extra=[hidden])
    ok("Planı", base, op="plan")
    ok("Doğrulama", base, op="validate")
    ok("Tek geri alma adımı", base, undo=True)
    no("Yol projesi değil", base, NOT_ROAD, setup(work="map"), None)
    no("Olmayan korkuluk", {**base, "guardrail": "yok"},
       failed("unknown_guardrail", "“yok” kimlikli ya da adlı otokorkuluk projede yok.", "guardrail"))
    ok("Gruptaki katmana", {**base, "layerId": "yol-korkuluk"})
    ok("Yazılı planı", {**base, "labels": True}, op="plan")
    ok("Sık okumayla", base, guardrails=[{**G1, "step": 5.0, "rule": {**RULE, "gap": 5.0}}])
    ok("Sol yanda yaklaşma sonda, yazılarıyla", {**base, "labels": True},
       guardrails=[{**G1, "rule": {**RULE, "approach": 60.0, "leaving": 0.0, "gap": 0.0}}])
    ok("Dışa 2 m", base, guardrails=[{**G1, "offset": 2.0}])
    ok("Her dolguya, yazılarıyla", {**base, "labels": True},
       guardrails=[{**G1, "rule": {**RULE, "warrants": [{"height": 0.0}]}}])
    bozuk = START + [{**G1, "id": "o-4", "name": "Bozuk", "corridor": "c-4"}]
    no("Kurulamayan koridor", {**base, "guardrail": "o-4"}, broken_result(), setup(bozuk), bozuk)
    no("Olmayan katman", {**base, "layerId": "yok"},
       failed("layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layerId"))
    no("Katman grubu", {**base, "layerId": "grup"},
       failed("not_a_layer", "“Yol” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin.",
              "layerId"))
    no("Kilitli katman", {**base, "layerId": "kilitli"},
       failed("layer_locked", "“Kilitli” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı "
                              "etkinleştirin.", "layerId"))
    no("Geçersiz beklenen sürüm", {**base, "expectedRevision": "on"}, BAD_REVISION)
    no("Eski sürüme göre hazırlanmış", {**base, "expectedRevision": "41"}, CONFLICT)
    return cases


def file(command, title, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": "scripts/fixtures/road_guardrail_cases.py yazar; kesitler road_stakeout_cases.py'nin (koridorun "
                    "başvurusu), kural ADR 0909 §1'den, yerler road_alignment_cases.py'nin 40 basamaklı başvurusundan; "
                    "KentOS kodu yok. Kuralın sayıları ortak durumlarındır, bir standardın değil.",
            "setup": setup(), "cases": cases}


def main():
    files = {"road.guardrails.edit": file("road.guardrails.edit", "Otokorkuluğu düzenle", edit_cases()),
             "road.guardrails.query": file("road.guardrails.query", "Otokorkuluğu sorgula", query_cases()),
             "road.guardrails.draw": file("road.guardrails.draw", "Otokorkuluk çiz", draw_cases())}
    check = "--check" in sys.argv
    for name, data in files.items():
        path = OUT / f"{name}.json"
        data = road_grades.fixture(data)
        text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
        if check:
            if path.read_text(encoding="utf-8") != text:
                raise SystemExit(f"{path} güncel değil: python3 {Path(__file__).name}")
            print(f"{path.name}: {len(data['cases'])} durum güncel")
        else:
            path.write_text(text, encoding="utf-8")
            print(f"{path.name}: {len(data['cases'])} durum")


if __name__ == "__main__":
    main()
