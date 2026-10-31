#!/usr/bin/env python3
"""The building program's buildings, wings, levels and shares (docs/adr/0784), from the ADR's rules, without KentOS code.

Writes fixtures/commands/v1/cad.program.check.shares.json and cad.program.edit.buildings.json; `--check` compares and
writes nothing. Run with shapely (the check's other numbers come from program_cases.py, which needs it):
`uv run --no-project --with shapely python3 scripts/fixtures/program_share_cases.py --check`.

The rules, as the ADR gives them:

- A building and a wing may name the levels they span (each once, the project's); a wing's are its building's when the
  building names any. A building or a wing groups still name is not taken away (`building_in_use`, before the
  program's own problem); a level the project lacks is `level_not_found`, after it.
- Rule `kentos.share/1`: each common area is its own pool, its rooms (no voids, no outdoor spaces) summed in the
  drawing's order and taken to whole hundredths of a square metre, the exact binary value rounded half to even. Its
  scope: its wing's independent units when it names a wing, else its building's, else all of them. A unit's weight:
  1 (`equal`), its own rooms in steps (`net`) or its `shareWeight` read exactly (`weight`). Each unit gets the floor of
  T·w/W steps; the steps left go one each to the largest remainders, the earlier unit first on a tie. A pool with no
  unit, a unit without a weight, or weights adding up to nothing is not shared and says why (and a warning). A unit's
  gross is its own rooms in steps plus its shares, of the whole program whatever the check reads.
- Tags and schedules read the space's unit's share and gross.
"""

import argparse
import copy
import re
import sys
from fractions import Fraction
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import program_cases as pc  # noqa: E402  (the ADR 0773 reference: its check, its setup)

P, rect, space, refusal, Cases, LEVELS = pc.P, pc.rect, pc.space, pc.refusal, pc.Cases, pc.LEVELS
RULE = "kentos.share/1"
LABEL = {"equal": "Eşit", "net": "Net alana göre", "weight": "Ağırlığa göre"}

# ── The plan: building A (a west and an east wing), building B, building C without units, a common area of the whole
#    program; a duplex upstairs, a shaft, a balcony, a garden ─────────────────────────────────────────────────────

SP = {
    10: space(rect(0, 0, 6, 5), "zemin", "d1", "Salon", "101"),
    11: space(rect(6, 0, 9, 4), "zemin", "d1", "Yatak odası", "102"),
    12: space(rect(0, -2, 3, -0.5), "zemin", "d1", "Balkon", "103"),
    13: space(rect(10, 0, 15, 5), "zemin", "d2", "Salon", "201"),
    14: space(rect(10, 0, 14, 4.5), "kat-1", "d2", "Yatak odası", "202"),
    15: space(rect(16, 0, 19.3, 5), "zemin", "d3", "Salon", "301"),
    16: space(rect(30, 0, 37, 7), "zemin", "d4", "Salon", "401"),
    17: space(rect(0, 6, 15.01, 7.3), "zemin", "ortak", "Koridor", "001"),
    18: space(rect(15.01, 6, 16, 7.3), "zemin", "ortak", "Şaft", "002", void=True),
    19: space(rect(10, 8, 13, 9.1), "zemin", "kdogu", "Merdiven", "003"),
    20: space(rect(30, 8, 33, 10.02), "zemin", "bortak", "Koridor", "004"),
    21: space(rect(40, 0, 42.14, 1), "zemin", "genel", "Bekçi", "005"),
    22: space(rect(50, 0, 60, 10), None, "bahce", "Bahçe", "B1"),
    23: space(rect(70, 0, 72, 2), None, None, "Depo", "D"),
}

PROGRAM = {
    "buildings": [
        {"id": "a", "name": "A Blok", "wings": [{"id": "a-bati", "name": "Batı kanadı"},
                                                {"id": "a-dogu", "name": "Doğu kanadı"}]},
        {"id": "b", "name": "B Blok"},
        {"id": "c", "name": "C Blok"},
    ],
    "units": [
        {"id": "d1", "number": "1", "name": "Daire 1", "building": "a", "wing": "a-bati", "recordedArea": 50.0},
        {"id": "d2", "number": "2", "name": "Daire 2", "building": "a", "wing": "a-dogu"},
        {"id": "d3", "number": "3", "name": "Daire 3", "building": "a", "wing": "a-dogu"},
        {"id": "d4", "number": "4", "name": "Daire 4", "building": "b"},
        {"id": "ortak", "number": "O", "name": "A ortak", "kind": "common", "building": "a"},
        {"id": "kdogu", "number": "OD", "name": "Doğu merdiveni", "kind": "common", "building": "a", "wing": "a-dogu"},
        {"id": "bortak", "number": "OB", "name": "B ortak", "kind": "common", "building": "b"},
        {"id": "genel", "number": "G", "name": "Site ortak", "kind": "common"},
        {"id": "bahce", "number": "B", "name": "Bahçe", "kind": "outdoor"},
    ],
    "briefs": [{"usage": "Balkon", "outdoor": True}],
    "shareRule": "equal",
}


def program_with(rule=None, weights=None, change=None):
    p = copy.deepcopy(PROGRAM)
    if rule is None:
        p.pop("shareRule", None)
    else:
        p["shareRule"] = rule
    for u in p["units"]:
        if weights and u["id"] in weights:
            u["shareWeight"] = weights[u["id"]]
    if change:
        change(p)
    return p


def setup_with(program, spaces=None):
    return pc.setup(walls={}, spaces=spaces or SP, program=program, name="Bina programı payları")


# ── The rule ──────────────────────────────────────────────────────────────────────────────────────────────────────


def steps(x):
    """Whole hundredths of a square metre: the exact binary value, halves to even (Python's round of a fraction)."""
    return round(Fraction(x) * 100)


def stext(n):
    sign = "-" if n < 0 else ""
    a = abs(n)
    return f"{sign}{a // 100}.{a % 100:02d}"


def weight_units(t):
    if not re.fullmatch(r"[0-9]{1,12}(\.[0-9]{1,6})?", t):
        return None
    return int(Fraction(t) * 10**6)


def wtext(n):
    w, f = divmod(n, 10**6)
    return str(w) if f == 0 else f"{w}.{f:06d}".rstrip("0")


def frac(num, den):
    f = Fraction(num, den)
    return str(f.numerator) if f.denominator == 1 else f"{f.numerator}/{f.denominator}"


def rooms_of(spaces, program):
    """Each group's rooms, m², summed in the drawing's order: no voids, no spaces whose brief is outdoor."""
    units = {u["id"] for u in program.get("units", [])}
    outdoor = {b["usage"] for b in program.get("briefs", []) if b.get("outdoor")}
    out = {}
    for _, s in sorted(spaces.items()):
        if s["unit"] not in units or s["void"] or s["usage"] in outdoor:
            continue
        a = pc.area_of(s)
        # A tiny difference in the last bit between two shoelaces must not cross a half step.
        assert abs((Fraction(a) * 100) % 1 - Fraction(1, 2)) > Fraction(1, 10**6), s
        out[s["unit"]] = out.get(s["unit"], 0.0) + a
    return out


def shares(program, spaces):
    """`(report, {unit id: part})` or None when the program shares nothing."""
    rule = program.get("shareRule")
    if rule is None:
        return None
    rooms = rooms_of(spaces, program)
    order = program.get("units", [])
    units = [(i, u) for i, u in enumerate(order) if u.get("kind", "unit") == "unit"]
    got = [0] * len(order)
    pools, shared, unshared = [], 0, 0
    names = {b["id"]: b["name"] for b in program.get("buildings", [])}
    wings = {w["id"]: w["name"] for b in program.get("buildings", []) for w in b.get("wings", [])}
    for c in order:
        if c.get("kind", "unit") != "common":
            continue
        total = steps(rooms.get(c["id"], 0.0))
        if c.get("wing"):
            scope = [(i, u) for i, u in units if u.get("wing") == c["wing"]]
            place = f"“{wings[c['wing']]}” kanadında"
        elif c.get("building"):
            scope = [(i, u) for i, u in units if u.get("building") == c["building"]]
            place = f"“{names[c['building']]}” binasında"
        else:
            scope = list(units)
            place = "Programda"
        pool = {"unit": c["id"], "number": c["number"]}
        for k in ("building", "wing"):
            if c.get(k):
                pool[k] = c[k]
        pool["area"] = stext(total)
        why, ws = None, None
        if not scope:
            why = f"{place} bağımsız bölüm yok"
        elif rule == "equal":
            ws = [1] * len(scope)
        elif rule == "net":
            ws = [steps(rooms.get(u["id"], 0.0)) for _, u in scope]
        else:
            missing = [u["number"] for _, u in scope if "shareWeight" not in u]
            if missing:
                why = ", ".join(f"“{n}”" for n in missing) + " bölümünün pay ağırlığı yok"
            else:
                ws = [weight_units(u["shareWeight"]) for _, u in scope]
        if ws is not None and sum(ws) == 0:
            why = ("Kapsamdaki bölümlerin net alanı sıfır" if rule == "net"
                   else "Kapsamdaki bölümlerin pay ağırlıklarının toplamı sıfır")
        if why is not None:
            pool["unshared"] = f"{why}; ortak alan paylaştırılmadı."
            unshared += total
        else:
            whole = sum(ws)
            exact = [Fraction(total * w, whole) for w in ws]
            part = [e.numerator // e.denominator for e in exact]
            left = total - sum(part)
            ranked = sorted(range(len(ws)), key=lambda k: (-(exact[k] - part[k]), k))
            for k in ranked[:left]:
                part[k] += 1
            assert sum(part) == total
            pool["shares"] = []
            for k, (i, u) in enumerate(scope):
                got[i] += part[k]
                pool["shares"].append({
                    "unit": u["id"],
                    "weight": "1" if rule == "equal" else stext(ws[k]) if rule == "net" else wtext(ws[k]),
                    "share": stext(part[k]),
                    "exact": frac(total * ws[k], whole * 100),
                })
            shared += total
        pools.append(pool)
    parts, net = {}, 0
    for i, u in units:
        own = steps(rooms.get(u["id"], 0.0))
        net += own
        parts[u["id"]] = {"net": stext(own), "share": stext(got[i]), "gross": stext(own + got[i])}
    report = {"rule": f"{RULE} · {LABEL[rule]}", "pools": pools, "net": stext(net), "shared": stext(shared),
              "unshared": stext(unshared), "gross": stext(net + shared)}
    assert net + shared == sum(steps(rooms.get(u["id"], 0.0)) + got[i] for i, u in units)
    return report, parts


def warnings_of(report):
    return [{"code": "share_unshared", "message": f"“{p['number']}” ortak alanı: {p['unshared']}"}
            for p in (report or {}).get("pools", []) if "unshared" in p]


def check_output(program, spaces=None, units=None, levels=None, findings=False):
    spaces = spaces or SP
    out = pc.check({}, spaces, program, (), units_filter=units, levels_filter=levels, findings_on=findings)
    got = shares(program, spaces)
    if got:
        report, parts = got
        for row in out["units"]:
            if row["id"] in parts:
                row["share"] = parts[row["id"]]
        out["shares"] = report
    return out, warnings_of(got[0] if got else None)


def check_step(program, spaces=None, inp=None, op="execute", units=None, levels=None, findings=False):
    out, warns = check_output(program, spaces, units, levels, findings)
    i = dict(inp or {})
    if not findings:
        i["findings"] = False
    return {"op": op, "command": "cad.program.check", "input": i,
            "result": {"status": "completed", "output": out, "warnings": warns},
            "expect": {"revision": "same"}, "within": 1e-3}


# ── cad.program.check: the shares ─────────────────────────────────────────────────────────────────────────────────


def tag_step(slot, at, content, text):
    return {"op": "execute", "command": "cad.tags.create",
            "input": {"layerId": "etiket", "tags": [{"of": f"$uidOf:{slot}", "at": at, "content": content}]},
            "result": {"status": "completed", "output": "$any", "warnings": "$any"},
            "expect": {"revision": "changed", "fields": {str(max(SP) + 1): {"text": text}}}}


def share_check_cases():
    c = Cases()

    def case(name, program, steps_, spaces=None):
        c.add(name, steps_, setup_with(program, spaces))

    eq = program_with("equal")
    case("Eşit pay: her ortak alan kendi kapsamında, kalan adım en büyük artığa ve sırayla", eq, [check_step(eq)])
    case("Plan aynı payları verir, yazmaz", eq, [check_step(eq, op="plan")])
    net = program_with("net")
    case("Net alana göre: ağırlık bölümün odaları, balkon ve şaft dışı", net, [check_step(net)])
    w = program_with("weight", {"d1": "120", "d2": "1.5", "d3": "0.333333", "d4": "7"})
    case("Ağırlığa göre: ağırlıklar tam okunur, paylar havuza tam eşit", w, [check_step(w)])
    w0 = program_with("weight", {"d1": "120", "d2": "1.5", "d4": "7"})
    case("Ağırlığı olmayan bölüm: havuz paylaştırılmaz, uyarı söyler", w0, [check_step(w0)])
    wz = program_with("weight", {"d1": "0", "d2": "0", "d3": "0.000000", "d4": "0"})
    case("Ağırlıkların toplamı sıfır: paylaştırılmaz", wz, [check_step(wz)])
    roomless = program_with("net", change=lambda p: (
        p["buildings"][0]["wings"].append({"id": "a-kuzey", "name": "Kuzey kanadı"}),
        p["units"].insert(4, {"id": "d5", "number": "5", "name": "Daire 5", "building": "a", "wing": "a-kuzey"}),
        p["units"].append({"id": "kkuzey", "number": "OK", "kind": "common", "building": "a", "wing": "a-kuzey"})))
    sp_r = dict(SP)
    sp_r[24] = space(rect(84, 0, 86, 2), "zemin", "kkuzey", "Merdiven", "006")
    case("Net alanı olmayan kapsam: kanadın tek dairesinin odası yok, paylaştırılmaz", roomless,
         [check_step(roomless, sp_r)], sp_r)
    nobody = program_with("equal", change=lambda p: p["units"].append(
        {"id": "cortak", "number": "OC", "kind": "common", "building": "c"}))
    sp_c = dict(SP)
    sp_c[24] = space(rect(80, 0, 83, 3), "zemin", "cortak", "Koridor", "007")
    case("Bölümsüz binanın ortak alanı paylaştırılmaz", nobody, [check_step(nobody, sp_c)], sp_c)
    lone_wing = program_with("equal", change=lambda p: (
        p["buildings"][0]["wings"].append({"id": "a-kuzey", "name": "Kuzey kanadı"}),
        p["units"].append({"id": "kkuzey", "number": "OK", "kind": "common", "building": "a", "wing": "a-kuzey"})))
    sp_k = dict(SP)
    sp_k[24] = space(rect(84, 0, 86, 2), "zemin", "kkuzey", "Merdiven", "008")
    case("Bölümsüz kanadın ortak alanı paylaştırılmaz", lone_wing, [check_step(lone_wing, sp_k)], sp_k)
    case("Bölüm süzgeci payı değiştirmez: bütün program", net,
         [check_step(net, inp={"units": ["d2"]}, units=["d2"])])
    case("Kat süzgeci payı değiştirmez: dubleksin üst katı da sayılır", net,
         [check_step(net, inp={"levels": ["zemin"]}, levels=["zemin"])])
    none = program_with(None)
    case("Kural yoksa pay yok: eski cevap aynı", none, [check_step(none)])
    case("Doğrulama yazmaz", eq, [{"op": "validate", "command": "cad.program.check", "input": {"findings": False},
                                   "result": {"status": "completed", "output": None, "warnings": []},
                                   "expect": {"revision": "same"}}])
    # A building's and a wing's levels.
    lv = program_with("equal", change=lambda p: p["buildings"][0].update(levels=["zemin"]))
    case("Binanın katında olmayan mahal bulgu verir", lv, [check_step(lv, findings=True)])
    lvw = program_with("equal", change=lambda p: (p["buildings"][0].update(levels=["zemin", "kat-1"]),
                                                    p["buildings"][0]["wings"][1].update(levels=["zemin"])))
    case("Kanadın katında olmayan mahal kanadı söyler", lvw, [check_step(lvw, findings=True)])
    lvok = program_with("equal", change=lambda p: p["buildings"][0].update(levels=["zemin", "kat-1"]))
    case("Bütün katları kapsayan bina bulgu vermez", lvok, [check_step(lvok, findings=True)])
    # Tags and schedules.
    _, parts = shares(net, SP)
    d2 = parts["d2"]
    case("Etiket bölümün payını ve brütünü yazar", net,
         [tag_step(13, P(12, 2), "{number}\nPay {share}\nBrüt {gross}", f"201\nPay {d2['share']} m²\nBrüt {d2['gross']} m²")])
    moved = copy.deepcopy(net)
    sp_m = copy.deepcopy(SP)
    sp_m[11]["unit"] = None
    _, parts_m = shares(net, sp_m)
    case("Başka bölümün mahali çıkınca etiketin payı aynı adımda yenilenir", net, [
        tag_step(13, P(12, 2), "Pay {share}", f"Pay {d2['share']} m²"),
        {"op": "execute", "command": "cad.program.edit", "input": {"assign": [{"uids": ["$uidOf:11"], "unit": None}]},
         "result": {"status": "completed", "output": "$any", "warnings": []},
         "expect": {"revision": "changed", "fields": {str(max(SP) + 1): {"text": f"Pay {parts_m['d2']['share']} m²"}}}},
        check_step(moved, sp_m)])
    _, parts_w = shares(w, SP)
    case("Kural değişince etiket aynı adımda yenilenir", net, [
        tag_step(13, P(12, 2), "{gross}", f"{d2['gross']} m²"),
        {"op": "execute", "command": "cad.program.edit", "input": {"program": w},
         "result": {"status": "completed", "output": "$any", "warnings": []},
         "expect": {"revision": "changed", "fields": {str(max(SP) + 1): {"text": f"{parts_w['d2']['gross']} m²"}}}}])
    rows = [["Bölüm", "Pay (m²)", "Brüt (m²)"]]
    named = {u["id"]: f"{u['number']} {u['name']}" if u.get("name") else u["number"] for u in net["units"]}
    for _, sp in sorted(SP.items()):
        if sp["level"] != "zemin":
            continue
        part = parts.get(sp["unit"], {})
        row = [named.get(sp["unit"], sp["unit"] or ""), part.get("share", ""), part.get("gross", "")]
        if row not in rows[1:]:
            rows.append(row)
    case("Mahal çizelgesinin Pay ve Brüt sütunları bölüme göre gruplanır", net, [
        {"op": "execute", "command": "cad.schedules.create",
         "input": {"layerId": "etiket", "at": P(0, -20), "of": "spaces",
                   "columns": [{"field": "unit"}, {"field": "share"}, {"field": "gross"}],
                   "filter": {"levels": ["zemin"]}, "group": True},
         "result": {"status": "completed", "output": "$any", "warnings": "$any"},
         "expect": {"revision": "changed", "fields": {str(max(SP) + 1): {"cells": rows}}}}])
    return c.cases


# ── cad.program.edit: buildings, wings, levels, the rule ──────────────────────────────────────────────────────────


def building_edit_cases():
    c = Cases()
    base = copy.deepcopy(pc.PROGRAM)

    def edit(name, inp, out_program=None, op="execute", result=None, steps=()):
        if result is None:
            result = {"status": "completed", "output": {"program": out_program, "revision": "$current"}, "warnings": []}
            exp = {"revision": "changed" if op == "execute" else "same",
                   "settings": {"arch": {"levels": LEVELS, "program": out_program if op == "execute" else base}}}
        else:
            exp = {"revision": "same"}
        st = {"op": op, "command": "cad.program.edit", "input": inp, "result": result, "expect": exp}
        if op == "validate":
            st["result"] = {"status": "completed", "output": None, "warnings": []}
            st["expect"] = {"revision": "same"}
        c.add(name, list(steps) + [st])

    def changed(f):
        p = copy.deepcopy(base)
        f(p)
        return p

    def write_step(program):
        return {"op": "execute", "command": "cad.program.edit", "input": {"program": program},
                "result": {"status": "completed", "output": "$any", "warnings": []}, "expect": {"revision": "changed"}}

    pb = changed(lambda p: p["buildings"].append({"id": "b", "name": "B Blok", "levels": ["zemin"]}))
    edit("Katlarıyla bina eklenir", {"program": pb}, pb)
    edit("Plan yazmaz", {"program": pb}, pb, op="plan")
    edit("Doğrulama yazmaz", {"program": pb}, op="validate")
    pl = changed(lambda p: p["buildings"][0].update(levels=["zemin", "kat-1"]))
    edit("Binaya katları verilir", {"program": pl}, pl)
    pw = changed(lambda p: (p["buildings"][0].update(levels=["zemin", "kat-1"]),
                            p["buildings"][0]["wings"][0].update(levels=["kat-1"])))
    edit("Kanada binasının katlarından biri verilir", {"program": pw}, pw)
    pf = changed(lambda p: p["buildings"][0]["wings"][0].update(levels=["zemin"]))
    edit("Binası kat vermeyen kanat katını serbest seçer", {"program": pf}, pf)
    pr = changed(lambda p: (p["buildings"][0].update(name="A Blok (konut)"),
                            p["buildings"][0]["wings"][0].update(name="Doğu")))
    edit("Bina ve kanat yeniden adlandırılır", {"program": pr}, pr)
    edit("Kullanılmayan kanat kaldırılır", {"program": changed(lambda p: p["buildings"][0].update(wings=[
        {"id": "a-dogu", "name": "Doğu kanadı"}]))},
         changed(lambda p: p["buildings"][0].update(wings=[{"id": "a-dogu", "name": "Doğu kanadı"}])),
         steps=[write_step(changed(lambda p: p["buildings"][0]["wings"].append({"id": "a-bati", "name": "Batı kanadı"})))])
    edit("Kullanılmayan bina kaldırılır", {"program": base}, base, steps=[write_step(pb)])
    moved = changed(lambda p: (p["buildings"].__setitem__(0, {"id": "b", "name": "B Blok"}),
                               [u.update(building="b") or u.pop("wing", None) for u in p["units"] if u.get("building") == "a"]))
    edit("Bölümleri aynı çağrıda başka binaya verilen bina kaldırılır", {"program": moved}, moved)
    ps = changed(lambda p: p.update(shareRule="net"))
    edit("Ortak alanların pay kuralı yazılır", {"program": ps}, ps)
    pwt = changed(lambda p: (p.update(shareRule="weight"), p["units"][0].update(shareWeight="120.5"),
                             p["units"][1].update(shareWeight="80")))
    edit("Bölümlerin pay ağırlıkları yazılır", {"program": pwt}, pwt)
    gone = changed(lambda p: p.update(buildings=[]))
    edit("Bölümleri olan bina kaldırılmaz", {"program": gone}, result=refusal(
        "building_in_use", "“A Blok” binası kaldırılamaz: “1”, “2”, “O” bölümü ona bağlı; önce bölümleri başka binaya "
        "ya da kanada verin.", "program.buildings"))
    wgone = changed(lambda p: p["buildings"][0].update(wings=[]))
    edit("Bölümü olan kanat kaldırılmaz", {"program": wgone}, result=refusal(
        "building_in_use", "“Doğu kanadı” kanadı kaldırılamaz: “2” bölümü ona bağlı; önce bölümleri başka binaya ya da "
        "kanada verin.", "program.buildings"))
    edit("Projede olmayan kat binaya verilmez", {"program": changed(lambda p: p["buildings"][0].update(levels=["zemin", "kat-9"]))},
         result=refusal("level_not_found", "“kat-9” katı projede yok; önce Proje ayarları › Mimari'de katı ekleyin.",
                        "program.buildings[0].levels[1]"))
    edit("Projede olmayan kat kanada verilmez", {"program": changed(lambda p: p["buildings"][0]["wings"][0].update(levels=["cati"]))},
         result=refusal("level_not_found", "“cati” katı projede yok; önce Proje ayarları › Mimari'de katı ekleyin.",
                        "program.buildings[0].wings[0].levels[0]"))
    edit("Kanadın katı binasının katlarından olmalı", {"program": changed(lambda p: (
        p["buildings"][0].update(levels=["zemin"]), p["buildings"][0]["wings"][0].update(levels=["kat-1"])))},
         result=refusal("invalid_program", "“kat-1” katı “A Blok” binasının katlarında yok; kanadın katları binasının "
                        "katlarından seçilir.", "program.buildings[0].wings[0].levels[0]"))
    edit("Kat iki kez yazılmaz", {"program": changed(lambda p: p["buildings"][0].update(levels=["zemin", "zemin"]))},
         result=refusal("invalid_program", "“zemin” katı iki kez yazıldı.", "program.buildings[0].levels[1]"))
    for name, t in (("Virgüllü ağırlık okunmaz", "1,5"), ("Altıdan çok ondalık okunmaz", "0.1234567"),
                    ("Eksi ağırlık okunmaz", "-2")):
        edit(name, {"program": changed(lambda p, t=t: p["units"][0].update(shareWeight=t))},
             result=refusal("invalid_program", f"Pay ağırlığı “{t}” okunamadı: noktadan önce en çok 12, sonra en çok 6 "
                            "basamaklı bir sayı yazın (120, 1.5).", "program.units[0].shareWeight"))
    c.add("Geri alma katları ve kuralı birlikte geri verir", [
        write_step(changed(lambda p: (p["buildings"][0].update(levels=["zemin"]), p.update(shareRule="equal")))),
        {"op": "undo", "returns": "Bina programı"},
        {"op": "execute", "command": "cad.program.check", "input": {"findings": False},
         "result": {"status": "completed", "output": "$any", "warnings": []},
         "expect": {"revision": "same", "settings": {"arch": {"levels": LEVELS, "program": base}}}}])
    return c.cases


def write(name, command, title, note, setup_doc, cases, check_only):
    data = {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": note, "setup": setup_doc, "cases": cases}
    text = pc.json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    path = pc.OUT / name
    if check_only:
        if not path.exists() or path.read_text() != text:
            print(f"{path.relative_to(pc.ROOT)} güncel değil: --check'siz çalıştırın.")
            return False
        return True
    path.write_text(text)
    print(f"{path.relative_to(pc.ROOT)}: {len(cases)} durum")
    return True


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    note = ("ADR 0784. Beklenenler scripts/fixtures/program_share_cases.py'nin ADR'den yazılmış kurallarından, KentOS "
            "kodu olmadan; paylar Python'un kesirleriyle (fractions), denetimin öbür sayıları program_cases.py'den.")
    ok = write("cad.program.check.shares.json", "cad.program.check", "Bina programının payları: kural, kapsam, artık, "
               "paylaştırılamayanlar, katlar, etiket ve çizelge", note, setup_with(program_with("equal")),
               share_check_cases(), args.check)
    ok &= write("cad.program.edit.buildings.json", "cad.program.edit", "Bina ve kanat: katlar, kaldırma, pay kuralı ve "
                "ağırlıklar; retler", note, pc.setup(), building_edit_cases(), args.check)
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
