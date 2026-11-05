#!/usr/bin/env python3
"""Writes fixtures/commands/v1/road.hauls.{edit,query}.json: the shared cases of KentOS Yol's mass haul commands
(plan/road.md Y9, docs/adr/0907), their expectations built here without KentOS code: the drawing, its corridors and
volumes are scripts/fixtures/road_volume_command_cases.py's (the volume sheet's mass ordinate), the diagram
scripts/fixtures/road_haul_cases.py's layers in fractions. The desktop (crates/native/application/tests/all/fixtures.rs)
and Python (python/tests/test_command_cases.py) run them.

    uv run --no-project --python 3.11 --with triangle --with numpy --with mpmath --with pyproj \\
        python3 scripts/fixtures/road_haul_command_cases.py
    ... --check   # compares the files with the ones on disk
"""

import copy
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_corridor_command_cases as ccmd  # noqa: E402
import road_haul_cases as href  # noqa: E402
import road_volume_command_cases as vcmd  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"

completed, failed, step, case = vcmd.completed, vcmd.failed, vcmd.step, vcmd.case
UNCHANGED, STEPPED, CONFLICT, BAD_REVISION = vcmd.UNCHANGED, vcmd.STEPPED, vcmd.CONFLICT, vcmd.BAD_REVISION
NOT_ROAD = failed("not_a_road_project", "Brükner yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.")
VOLUMES, CORRIDORS = vcmd.START, vcmd.CORRIDORS

H1 = {"id": "h-1", "name": "Ana Brükner", "volume": "v-1", "revision": 1, "free": 150.0}
H2 = {"id": "h-2", "name": "Yan Brükner", "volume": "v-2", "revision": 1,
      "sites": [{"name": "Döküm 1", "kind": "waste", "station": 300.0, "volume": 6000.0}]}
START = [H1, H2]


def road_of(hauls, volumes=VOLUMES, corridors=CORRIDORS):
    out = ccmd.road_of(corridors, volumes=volumes)
    if hauls:
        out["hauls"] = copy.deepcopy(hauls)
    return out


def setup(hauls=START, volumes=VOLUMES, corridors=CORRIDORS, work="road"):
    base = vcmd.setup(volumes, corridors, work=work)
    base["name"] = "Yol Brükneri"
    if work == "road":
        base["settings"]["road"] = road_of(hauls, volumes, corridors)
    return base


class Refused(Exception):
    def __init__(self, code, message, path=None):
        super().__init__(message)
        self.result = failed(code, message, path)


def ordinate(report):
    out = []
    for r in report["rows"]:
        if not out:
            out.append([r["from"], 0.0])
        elif out[-1][0] != r["from"]:
            out.append([r["from"], out[-1][1]])
        out.append([r["to"], r["mass"]])
    return out


def step_at(points, s, delta):
    i = sum(1 for p in points if p[0] <= s)
    if i > 0 and points[i - 1][0] == s:
        v = points[i - 1][1]
    else:
        p, q = points[i - 1], points[i]
        v = p[1] + (q[1] - p[1]) * (s - p[0]) / (q[0] - p[0])
    out = [list(p) for p in points[:i]]
    if not out or out[-1][0] != s:
        out.append([s, v])
    out.append([s, v + delta])
    out += [[p[0], p[1] + delta] for p in points[i:]]
    return out


def read(h, volumes=VOLUMES, corridors=CORRIDORS, points=False):
    v = next((x for x in volumes if x["id"] == h["volume"]), None)
    if v is None:
        raise Refused("unknown_volume", f"“{h['volume']}” kimlikli kübaj hesabı projede yok.", "volume")
    try:
        rep, vw = vcmd.compute(v, corridors)
    except vcmd.Refused as r:
        raise Refused("invalid_haul", f"“{v['name']}” kübajı hesaplanamıyor: {r.result['error']['message']}", "volume") from None
    warnings = [w for w in vw if w["code"] in ("corridor_stale", "volume_gap")]
    curve = ordinate(rep)
    if len(curve) < 2:
        raise Refused("invalid_haul", f"“{v['name']}” kübajının hacimli aralığı yok; Brükner çizilemez.", "volume")
    c = next(x for x in corridors if x["id"] == v["corridor"])
    text = lambda s: ccmd.km_text(c["alignment"], s)  # noqa: E731
    first, last = curve[0][0], curve[-1][0]
    sites = sorted(enumerate(h.get("sites", [])), key=lambda t: (t[1]["station"], t[0]))
    for j, site in sites:
        if site["station"] < first or site["station"] > last:
            raise Refused("invalid_haul", f"“{site['name']}” alanı kübajın km'leri dışında: kübaj {text(first)} – {text(last)} arası.",
                          f"sites[{j}]")
        curve = step_at(curve, site["station"], site["volume"] if site["kind"] == "borrow" else -site["volume"])
    d = href.diagram([(p[0], p[1]) for p in curve], h.get("balance", 0.0), h.get("free", 0.0))
    loops = []
    for lp in d["loops"]:
        out = {"from": lp["from"], "to": lp["to"], "fromText": text(lp["from"]), "toText": text(lp["to"]),
               "forward": lp["forward"], "volume": lp["volume"], "moment": lp["moment"], "overVolume": lp["overVolume"],
               "overMoment": lp["overMoment"], "unbalanced": lp["unbalanced"]}
        if lp["volume"] > 0:
            out["average"] = lp["moment"] / lp["volume"]
        loops.append(out)
    if d["waste"] > 1e-6 or d["borrow"] > 1e-6:
        warnings.append({"code": "haul_unbalanced", "path": "haul",
                         "message": f"“{h['name']}” Brüknerinde dengelenmeyen hacim var: döküm {d['waste']:.3f} m³, "
                                    f"ödünç {d['borrow']:.3f} m³."})
    report = {"id": h["id"], "name": h["name"], "revision": h.get("revision", 0), "volume": h["volume"],
              "balance": h.get("balance", 0.0), "stale": rep["stale"], "balancePoints": d["balance"], "loops": loops,
              "hauled": d["volume"], "moment": d["moment"], "overVolume": d["overVolume"], "overMoment": d["overMoment"],
              "waste": d["waste"], "borrow": d["borrow"]}
    if "free" in h:
        report["free"] = h["free"]
    if d["volume"] > 0:
        report["average"] = d["moment"] / d["volume"]
    report["method"] = "layers"
    if h.get("cost"):
        report["cost"] = cost(h["cost"], d)
    if points:
        report["points"] = curve
    return report, warnings


def cost(c, d):
    """The diagram's cost (docs/adr/0907 §5): the overhaul moment in m³·km times its price, the waste and the borrow
    times theirs, the total of those priced; the trips everything moved over the payload, rounded up."""
    out = {"currency": c["currency"]}
    parts = []
    for key, amount in (("overhaul", d["overMoment"] / 1000), ("waste", d["waste"]), ("borrow", d["borrow"])):
        if key in c:
            out[key] = amount * c[key]
            parts.append(out[key])
    out["total"] = sum(parts)
    if "payload" in c:
        out["trips"] = float(max(0, math.ceil((d["volume"] + d["waste"] + d["borrow"]) / c["payload"] - 1e-9)))
    return out


def broken(h, why):
    out = {"id": h["id"], "name": h["name"], "revision": h.get("revision", 0), "volume": h["volume"],
           "balance": h.get("balance", 0.0), "stale": False, "balancePoints": [], "loops": [], "hauled": 0.0,
           "moment": 0.0, "overVolume": 0.0, "overMoment": 0.0, "waste": 0.0, "borrow": 0.0, "method": "layers",
           "problem": why}
    if "free" in h:
        out["free"] = h["free"]
    return out


def report(h, volumes=VOLUMES, corridors=CORRIDORS):
    try:
        return read(h, volumes, corridors)[0]
    except Refused as r:
        return broken(h, r.result["error"]["message"])


def listed(hauls):
    return [{"id": h["id"], "name": h["name"], "volume": h["volume"], "revision": h.get("revision", 0)} for h in hauls]


def file(command, title, base, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": "scripts/fixtures/road_haul_command_cases.py yazar; çizim, koridorlar ve kübajlar "
                    "road_volume_command_cases.py'nin, Brükner road_haul_cases.py'nin kesirli katman okumasıdır; KentOS "
                    "kodu yok. `expect.road` projenin yol ayarlarıdır. Brükner geri alma geçmişinde tutulan bir proje "
                    "ayarıdır; okuması her seferinde hesaplanır.",
            "setup": base, "cases": cases}


# ── road.hauls.edit ────────────────────────────────────────────────────────────────────────────────────────────────

def edit_cases():
    cases = []

    def written(after, target):
        out = {"hauls": listed(after), "revision": "$current"}
        if target:
            out["haul"] = report(next(h for h in after if h["id"] == target))
        return out

    def ok(name, input_, after, target, op="execute", undo=False):
        writes = op == "execute" and after != START
        result = completed(None) if op == "validate" else completed(written(after, target))
        expect = {**STEPPED, "road": road_of(after)} if writes else {**UNCHANGED, "road": road_of(START)}
        steps = [step(op, input_, result, expect)]
        if undo:
            steps.append({"op": "undo", "returns": "Brükner", "expect": {"road": road_of(START), "canRedo": True}})
        cases.append(case(name, steps))

    def no(name, input_, result, setup_=None):
        road = setup_["settings"].get("road") if setup_ is not None else road_of(START)
        cases.append(case(name, [step("execute", input_, result, {**UNCHANGED, "road": road})], setup_))

    def rulefail(words, path=None):
        return failed("invalid_haul", f"Brükner kurallarına uymuyor: {words}.", path)

    def put(i, h):
        out = list(START)
        out[i] = h
        return out

    def new(id_, **kw):
        h = {"id": id_, "name": "Yeni Brükner", "volume": "v-1", "revision": 1}
        h.update(kw)
        return h

    base = {"operation": "set", "name": "Yeni Brükner", "volume": "v-1"}
    ok("Yeni Brükner: kimliği ilk boş h-3", base, START + [new("h-3")], "h-3")
    ok("Kübajın adıyla, kendi kimliğiyle", {**base, "haul": "kendi", "volume": "YAN kübaj"}, START + [new("kendi", volume="v-2")],
       "kendi")
    ok("Serbest taşıma ve denge çizgisi", {**base, "free": 200.0, "balance": 1000.0},
       START + [new("h-3", free=200.0, balance=1000.0)], "h-3")
    sites = [{"name": "Ödünç A", "kind": "borrow", "station": 100.0, "volume": 500.0},
             {"name": "Döküm B", "kind": "waste", "station": 500.0, "volume": 3000.0}]
    ok("Ödünç ve döküm alanlarıyla", {**base, "sites": sites}, START + [new("h-3", sites=sites)], "h-3")
    ok("Adı değişir", {"operation": "set", "haul": "h-1", "name": "Brükner A"}, put(0, {**H1, "name": "Brükner A", "revision": 2}),
       "h-1")
    ok("Serbest taşıma değişir", {"operation": "set", "haul": "Ana Brükner", "free": 400.0}, put(0, {**H1, "free": 400.0, "revision": 2}),
       "h-1")
    ok("Serbest taşıma null ile kalkar", {"operation": "set", "haul": "h-1", "free": None},
       put(0, {k: v for k, v in H1.items() if k != "free"} | {"revision": 2}), "h-1")
    ok("Alanlar kaldırılır", {"operation": "set", "haul": "h-2", "sites": []},
       put(1, {k: v for k, v in H2.items() if k != "sites"} | {"revision": 2}), "h-2")
    price = {"currency": "TL", "overhaul": 12.5, "waste": 40.0, "borrow": 85.0, "payload": 15.0}
    ok("Birim fiyatlar ve kamyon yükü", {**base, "cost": price}, START + [new("h-3", cost=price)], "h-3")
    ok("Yalnız fazla taşıma fiyatı", {"operation": "set", "haul": "h-1", "cost": {"currency": "EUR", "overhaul": 0.8}},
       put(0, {**H1, "cost": {"currency": "EUR", "overhaul": 0.8}, "revision": 2}), "h-1")
    hp = {**H1, "cost": price}
    cases.append(case("Fiyatlar null ile kalkar", [step(
        "execute", {"operation": "set", "haul": "h-1", "cost": None},
        completed({"hauls": listed([{**H1, "revision": 2}, H2]), "revision": "$current", "haul": report({**H1, "revision": 2})}),
        {**STEPPED, "road": road_of([{**H1, "revision": 2}, H2])})], setup([hp, H2])))
    # Docs/adr/0944: the cut unsuitable for fill and the barriers (schema 169).
    unsuitable = [{"from": 0.0, "to": 200.0, "share": 35.0}, {"from": 200.0, "to": 400.0, "share": 10.0}]
    barriers = [{"name": "Dere", "station": 250.0, "detour": 400.0}, {"name": "Kavşak", "station": 450.0}]
    ok("Uygunsuz malzeme ve engellerle", {**base, "unsuitable": unsuitable, "barriers": barriers},
       START + [new("h-3", unsuitable=unsuitable, barriers=barriers)], "h-3")
    ok("Uygunsuz malzeme yazılır", {"operation": "set", "haul": "h-1", "unsuitable": unsuitable},
       put(0, {**H1, "unsuitable": unsuitable, "revision": 2}), "h-1")
    hs = {**H1, "unsuitable": unsuitable, "barriers": barriers}
    cases.append(case("Engeller ve uygunsuz malzeme boş listeyle kalkar", [step(
        "execute", {"operation": "set", "haul": "h-1", "unsuitable": [], "barriers": []},
        completed({"hauls": listed([{**H1, "revision": 2}, H2]), "revision": "$current", "haul": report({**H1, "revision": 2})}),
        {**STEPPED, "road": road_of([{**H1, "revision": 2}, H2])})], setup([hs, H2])))
    ok("Aynı değerler: değişiklik yok, adım yok", {"operation": "set", "haul": "h-1", "name": "Ana Brükner"}, START, "h-1")
    ok("Brükner silinir", {"operation": "remove", "haul": "h-2"}, [H1], None)
    ok("Silme tek geri alma adımı", {"operation": "remove", "haul": "Ana Brükner"}, [H2], None, undo=True)
    ok("Plan yazmaz", {"operation": "set", "haul": "h-1", "name": "Brükner A"}, put(0, {**H1, "name": "Brükner A", "revision": 2}),
       "h-1", op="plan")
    ok("Doğrulama yazmaz", {"operation": "set", "haul": "h-1", "balance": -500.0}, put(0, {**H1, "balance": -500.0, "revision": 2}),
       "h-1", op="validate")

    # Docs/adr/0954: an approved plan and its history stay; editing the diagram keeps them (a plan read later says
    # what changed).
    signed = {"number": 1, "at": "2026-11-04", "revision": 1, "volumeRevision": 1, "moment": 1500000.0, "moved": 9000.0}
    ha = {**H1, "approved": signed}
    hh = {**H1, "history": [{**signed, "ended": {"revoked": True, "at": "2026-11-05"}}]}
    cases.append(case("Onaylı Brükner düzenlenir, onay durur", [step(
        "execute", {"operation": "set", "haul": "h-1", "free": 250.0},
        completed({"hauls": listed([{**ha, "free": 250.0, "revision": 2}, H2]), "revision": "$current",
                   "haul": report({**ha, "free": 250.0, "revision": 2})}),
        {**STEPPED, "road": road_of([{**ha, "free": 250.0, "revision": 2}, H2])})], setup([ha, H2])))
    no("Onaylı Brükner silinmez", {"operation": "remove", "haul": "h-1"},
       failed("invalid_haul", "“Ana Brükner” Brüknerinin onaylı taşıma planı ya da onay geçmişi var; onay ve geçmişi silinmez "
                              "(ADR 0954). Brükneri bırakıp yenisini yazın.", "haul"), setup([ha, H2]))
    no("Onay geçmişi olan Brükner silinmez", {"operation": "remove", "haul": "Ana Brükner"},
       failed("invalid_haul", "“Ana Brükner” Brüknerinin onaylı taşıma planı ya da onay geçmişi var; onay ve geçmişi silinmez "
                              "(ADR 0954). Brükneri bırakıp yenisini yazın.", "haul"), setup([hh, H2]))
    no("Yol projesi değil", {"operation": "remove", "haul": "h-1"}, NOT_ROAD, setup(work="map"))
    no("Yeniden bağlanmaz", {"operation": "rebind", "haul": "h-1"},
       failed("invalid_haul", "Brükner kübajını izler, yeniden bağlanmaz; kübajın koridorunu aksa yeniden bağlayın.", "operation"))
    no("Olmayan Brükner silinmez", {"operation": "remove", "haul": "yok"},
       failed("unknown_haul", "“yok” kimlikli ya da adlı Brükner projede yok.", "haul"))
    no("Olmayan kübaj", {**base, "volume": "yok"},
       failed("unknown_volume", "“yok” kimlikli ya da adlı kübaj hesabı projede yok.", "volume"))
    no("Başka kübaja geçmez", {"operation": "set", "haul": "h-1", "volume": "v-2"},
       failed("invalid_haul", "“Ana Brükner” Brüknerinin kübajı değiştirilmez; başka kübaj için yeni bir Brükner yazın.", "volume"))
    no("Değiştirilecek bir şey yok", {"operation": "set", "haul": "h-1"},
       failed("invalid_haul", "Değiştirilecek bir şey yok: name, balance, free, sites, cost, unsuitable ya da barriers verin."))
    no("Yeni Brüknerin kübajı yok", {"operation": "set", "name": "x"},
       failed("invalid_haul", "Yeni Brükner için name ve volume verilmeli.", "volume"))
    no("Aynı ad", {**base, "name": "YAN Brükner"}, rulefail("bu adda başka bir Brükner var", "name"))
    no("Sıfır serbest taşıma", {**base, "free": 0.0}, rulefail("serbest taşıma uzaklığı 0'dan büyük, en çok 100 km olur", "free"))
    no("Hacimsiz alan", {**base, "sites": [{"name": "A", "kind": "waste", "station": 100.0, "volume": 0.0}]},
       rulefail("alanın hacmi 0'dan büyük olmalı", "sites[0]"))
    no("Adsız alan", {**base, "sites": [{"name": " ", "kind": "borrow", "station": 100.0, "volume": 10.0}]},
       rulefail("alanın adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter", "sites[0]"))
    no("Km'lerin dışında alan", {**base, "sites": [{"name": "Uzak", "kind": "waste", "station": 5000.0, "volume": 10.0}]},
       failed("invalid_haul", f"“Uzak” alanı kübajın km'leri dışında: kübaj {ccmd.km_text('g-1', 0.0)} – "
                              f"{ccmd.km_text('g-1', read(new('x'))[0]['loops'][-1]['to'])} arası.", "sites[0]"))
    broken_corr = {**ccmd.C1, "regions": [{"from": 0.0, "to": 100.0, "template": "tk-1"},
                                          {"from": 50.0, "to": 200.0, "template": "tk-1"}]}
    no("Hesaplanamayan kübajın Brükneri", base,
       failed("invalid_haul", "“Ana kübaj” kübajı hesaplanamıyor: “Ana koridor” koridoru kurulamıyor: 1. ve 2. bölge kesişiyor; "
                              "bölgeler en çok uç uca gelir.", "volume"), setup(START, VOLUMES, [broken_corr, ccmd.C2]))
    no("Para birimsiz fiyat", {**base, "cost": {"currency": "", "overhaul": 1.0}},
       rulefail("para birimi boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 12 karakter", "cost"))
    no("Eksi fiyat", {**base, "cost": {"currency": "TL", "waste": -1.0}},
       rulefail("birim fiyat 0 ya da daha büyük, sonlu olmalı", "cost"))
    no("Sıfır kamyon yükü", {**base, "cost": {"currency": "TL", "payload": 0.0}},
       rulefail("kamyon yükü 0'dan büyük, en çok 1000 m³ olur", "cost"))
    no("Yüzde 100'den büyük uygunsuz pay", {**base, "unsuitable": [{"from": 0.0, "to": 100.0, "share": 150.0}]},
       rulefail("uygunsuz malzemenin payı yüzde 0 ile 100 arası olur", "unsuitable[0]"))
    no("Ters uygunsuz satırı", {**base, "unsuitable": [{"from": 300.0, "to": 100.0, "share": 20.0}]},
       rulefail("uygunsuz malzeme satırının sonu başından sonra olmalı", "unsuitable[0]"))
    no("Kesişen uygunsuz satırları", {**base, "unsuitable": [{"from": 0.0, "to": 200.0, "share": 20.0},
                                                            {"from": 150.0, "to": 300.0, "share": 30.0}]},
       rulefail("uygunsuz malzeme satırları kesişmemeli", "unsuitable[1]"))
    no("Adsız engel", {**base, "barriers": [{"name": "", "station": 100.0}]},
       rulefail("engelin adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter", "barriers[0]"))
    no("Eksi dolaşma", {**base, "barriers": [{"name": "Dere", "station": 100.0, "detour": -5.0}]},
       rulefail("dolaşma uzunluğu 0 ile 100 km arası olur", "barriers[0]"))
    no("Geçersiz beklenen sürüm", {"operation": "remove", "haul": "h-2", "expectedRevision": "on"}, BAD_REVISION)
    no("Eski sürüme göre hazırlanmış", {"operation": "remove", "haul": "h-2", "expectedRevision": "41"}, CONFLICT)
    return file("road.hauls.edit", "Brükneri düzenle", setup(), cases)


# ── road.hauls.query ───────────────────────────────────────────────────────────────────────────────────────────────

def query_cases():
    cases = []

    def q(name, input_, hauls=START, volumes=VOLUMES, corridors=CORRIDORS, op="execute"):
        one = input_.get("haul")
        if one is None:
            reports, warns = [], []
            for h in hauls:
                try:
                    r, w = read(h, volumes, corridors)
                    reports.append(r)
                    warns += w
                except Refused as e:
                    reports.append(broken(h, e.result["error"]["message"]))
            out = {"hauls": reports}
        else:
            h = next(x for x in hauls if x["id"] == one or x["name"].lower() == one.lower())
            r, warns = read(h, volumes, corridors, input_.get("points", False))
            out = {"hauls": [r]}
        result = completed(None, warns) if op == "validate" else completed(out, warns)
        default = hauls is START and volumes is VOLUMES and corridors is CORRIDORS
        cases.append(case(name, [step(op, input_, result, {**UNCHANGED, "road": road_of(hauls, volumes, corridors)})],
                          None if default else setup(hauls, volumes, corridors)))

    def no(name, input_, result, hauls=START, volumes=VOLUMES, corridors=CORRIDORS, setup_=None):
        default = hauls is START and volumes is VOLUMES and corridors is CORRIDORS
        road = setup_["settings"].get("road") if setup_ is not None else road_of(hauls, volumes, corridors)
        cases.append(case(name, [step("execute", input_, result, {**UNCHANGED, "road": road})],
                          setup_ or (None if default else setup(hauls, volumes, corridors))))

    q("Bütün Brüknerler", {})
    q("Bir Brükner adıyla", {"haul": "ANA Brükner"})
    q("Kütle eğrisinin noktaları", {"haul": "h-1", "points": True})
    q("Döküm alanının basamağı", {"haul": "h-2", "points": True})
    q("Serbest taşımasız", {"haul": "h-1"}, [{k: v for k, v in H1.items() if k != "free"}, H2])
    q("Uzun serbest taşıma: fazla taşıma yok", {"haul": "h-1"}, [{**H1, "free": 5000.0}, H2])
    q("Denge çizgisi yukarıda", {"haul": "h-1"}, [{**H1, "balance": 15000.0}, H2])
    q("Denge çizgisi aşağıda: baştan ödünç", {"haul": "h-1"}, [{**H1, "balance": -3000.0}, H2])
    q("Döküm alanı fazla kazıyı alır", {"haul": "h-1"},
      [{**H1, "sites": [{"name": "Döküm", "kind": "waste", "station": 600.0, "volume": 28116.2}]}, H2])
    q("Ödünç alanı ve döküm alanı", {"haul": "h-1", "points": True},
      [{**H1, "sites": [{"name": "Ö", "kind": "borrow", "station": 120.0, "volume": 2000.0},
                        {"name": "D", "kind": "waste", "station": 350.0, "volume": 9000.0}]}, H2])
    q("Aynı km'de iki alan", {"haul": "h-1", "points": True},
      [{**H1, "sites": [{"name": "D1", "kind": "waste", "station": 200.0, "volume": 1000.0},
                        {"name": "D2", "kind": "waste", "station": 200.0, "volume": 500.0}]}, H2])
    q("Kübajın ilk km'sinde alan", {"haul": "h-1"}, [{**H1, "sites": [{"name": "Ö", "kind": "borrow", "station": 0.0, "volume": 800.0}]}, H2])
    c_gap = {**ccmd.C1, "regions": [{"from": 0.0, "to": 150.0, "template": "tk-1"}, {"from": 250.0, "to": 400.0, "template": "tk-2"}]}
    q("Kesitsiz aralıkta eğri düz", {"haul": "h-1", "points": True}, START, VOLUMES, [c_gap, ccmd.C2])
    q("Prizmoidal kübajın Brükneri", {"haul": "h-1"}, START, [{**vcmd.V1, "method": "prismoidal"}, vcmd.V2])
    full = {"currency": "TL", "overhaul": 12.5, "waste": 40.0, "borrow": 85.0, "payload": 15.0}
    q("Maliyet: fazla taşıma, döküm, ödünç ve sefer", {"haul": "h-1"}, [{**H1, "cost": full, "balance": -3000.0}, H2])
    q("Maliyet: yalnız fazla taşıma", {"haul": "h-1"}, [{**H1, "cost": {"currency": "EUR", "overhaul": 0.8}}, H2])
    q("Maliyet: döküm alanıyla, sefer sayısı", {"haul": "h-2"}, [H1, {**H2, "cost": {"currency": "TL", "waste": 40.0, "payload": 12.0}}])
    q("Maliyet: sıfır fiyat", {"haul": "h-1"}, [{**H1, "cost": {"currency": "TL", "overhaul": 0.0, "waste": 0.0}}, H2])
    q("Plan cevabı verir", {"haul": "h-2"}, op="plan")
    q("Doğrulama uyarıları verir", {"haul": "h-2"}, op="validate")
    q("Brükner yok: boş liste", {}, [])
    broken_corr = {**ccmd.C1, "regions": [{"from": 0.0, "to": 100.0, "template": "tk-1"},
                                          {"from": 50.0, "to": 200.0, "template": "tk-1"}]}
    q("Okunamayan Brükner raporda nedeniyle", {}, START, VOLUMES, [broken_corr, ccmd.C2])

    no("Yol projesi değil", {}, NOT_ROAD, setup_=setup(work="map"))
    no("Olmayan Brükner", {"haul": "yok"}, failed("unknown_haul", "“yok” kimlikli ya da adlı Brükner projede yok.", "haul"))
    no("Brükneri verilmeden noktalar", {"points": True},
       failed("invalid_haul", "Noktalar bir Brüknerde sorulur: Brüknerin kimliğini ya da adını (haul) verin.", "haul"))
    no("Okunamayan Brüknerin sorusu", {"haul": "h-1"},
       failed("invalid_haul", "“Ana kübaj” kübajı hesaplanamıyor: “Ana koridor” koridoru kurulamıyor: 1. ve 2. bölge kesişiyor; "
                              "bölgeler en çok uç uca gelir.", "volume"), START, VOLUMES, [broken_corr, ccmd.C2])
    return file("road.hauls.query", "Brükneri sorgula", setup(), cases)


def main():
    files = {"road.hauls.edit.json": edit_cases(), "road.hauls.query.json": query_cases()}
    check = "--check" in sys.argv[1:]
    bad = []
    for name, data in files.items():
        text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
        path = OUT / name
        if check:
            if not path.exists() or path.read_text() != text:
                bad.append(name)
        else:
            path.write_text(text)
            print(f"written {path} ({len(data['cases'])} cases)")
    if bad:
        sys.exit(f"not current: {', '.join(bad)}")
    if check:
        print("road haul cases are current")


if __name__ == "__main__":
    main()
