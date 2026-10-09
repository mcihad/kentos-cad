#!/usr/bin/env python3
"""Ağlar's form rules (docs/adr/0209 §10) as shared cases: fixtures/network/v1/form.json, written without KentOS code.

The form holds a network as typed: its tolerance in the project's length unit (a dot or a comma for the decimals),
the direction's values as lists with commas or semicolons, a speed cost's default speed (km/sa; empty for 50), the
expressions and the unit as typed. Cases: a form and the project's unit, and the network it writes (names and values
trimmed, empty expressions and units left out, the tolerance in metres) or the form's own words (a tolerance or a
speed that is not a number); every network written holds by the contract's rules (tools/kcad/kcad.py's copy). Then
how the kind changes what is still the other kind's default, and networks shown in the form and written back. The web
(apps/web/src/model/networkForm.test.ts) and the desktop (kentos_interaction::network::form) play the file.

    python3 scripts/fixtures/network_form_cases.py [--check]
"""
import argparse
import copy
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/network/v1/form.json"
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT / "tools/kcad"))
import kcad  # noqa: E402

# How many of the unit make a metre: a typed length is divided by it (the apps' `toMetres`).
PER_METRE = {"m": 1.0, "cm": 100.0, "mm": 1000.0}
DEFAULT_SPEED = 50.0
DECIMAL = re.compile(r"^\s*[-+]?(\d+([.,]\d*)?|[.,]\d+)\s*$")


def decimal(text):
    return float(text.strip().replace(",", ".")) if DECIMAL.match(text) else None


def values_of(text):
    return [v.strip() for v in re.split(r"[,;]", text) if v.strip()]


def def_of(form, unit):
    """The network a form writes, or the form's own words."""
    name = form["name"].strip()
    who = f"“{name}” ağının" if name else "Ağın"
    t = decimal(form["tolerance"])
    if t is None:
        return {"problem": f"{who} toleransı bir sayı olmalı (ör. 0,01)."}
    costs = []
    for c in form["costs"]:
        cost = c["name"].strip()
        if c["kind"] == "speed":
            typed = c["speed"].strip()
            speed = decimal(typed) if typed else DEFAULT_SPEED
            if speed is None:
                return {"problem": f"{who} “{cost}” maliyetinin hızı bir sayı olmalı (km/sa)."}
            costs.append({"name": cost, "kind": "speed", "field": c["field"].strip(), "speed": speed})
        else:
            out = {"name": cost, "kind": "field", "field": c["field"].strip()}
            if c["unit"].strip():
                out["unit"] = c["unit"].strip()
            costs.append(out)

    def opt(key, s):
        return {key: s.strip()} if s.strip() else {}

    d = {"id": form["id"], "name": name, "kind": form["kind"], "edges": [{"layer": e["layer"], **opt("filter", e["filter"])} for e in form["edges"]]}
    if form["junctions"]:
        d["junctions"] = [{"layer": j["layer"], "role": j["role"], **opt("filter", j["filter"]), **opt("closed", j["closed"])} for j in form["junctions"]]
    d["connect"] = form["connect"]
    d["tolerance"] = t / PER_METRE[unit]
    if form["direction"] == "field":
        d["direction"] = {"kind": "field", "field": form["field"].strip(), "forward": values_of(form["forward"]), "backward": values_of(form["backward"]),
                          "closed": values_of(form["shut"])}
    else:
        d["direction"] = {"kind": form["direction"]}
    if costs:
        d["costs"] = costs
    d.update(opt("closed", form["closed"]))
    return d


def form(**over):
    base = {
        "id": "ag-1", "name": "Yollar", "kind": "road", "edges": [{"layer": "yol", "filter": ""}], "junctions": [], "connect": "ends", "tolerance": "0.01",
        "direction": "both", "field": "", "forward": "", "backward": "", "shut": "", "costs": [], "closed": "",
    }
    base.update(over)
    return base


SURE = {"name": "Süre", "kind": "speed", "field": "hiz", "unit": "", "speed": "50"}


def cases():
    out = []

    def add(title, f, unit="m"):
        got = def_of(f, unit)
        if "problem" not in got:
            assert kcad.network_problem(got) is None, (title, kcad.network_problem(got))
        out.append({"name": title, "unit": unit, "form": f, **({"problem": got["problem"]} if "problem" in got else {"network": got})})

    add("en yalın yol ağı", form())
    add("virgüllü tolerans, adı ve alanları kırpılır", form(name="  Ana yollar ", tolerance="0,05", edges=[{"layer": "yol", "filter": "  tur = 'cadde'  "}, {"layer": "sokak", "filter": "   "}]))
    add("milimetrelik projede tolerans metreye çevrilir", form(tolerance="10"), unit="mm")
    add("santimetrelik projede", form(tolerance="2,5"), unit="cm")
    add("yön alandan: virgül ve noktalı virgülle listeler, boşlar düşer",
        form(direction="field", field=" yon ", forward="FT, ileri ,", backward="TF; geri", shut="N;; kapalı"))
    add("süre maliyeti: boş hız 50, alan kırpılır", form(costs=[{**SURE, "field": " hiz ", "speed": ""}]))
    add("süre maliyeti: virgüllü hız", form(costs=[{**SURE, "speed": "32,5"}]))
    add("alan maliyeti: birim kırpılır, boş birim yazılmaz",
        form(costs=[{"name": "Ücret", "kind": "field", "field": "ucret", "unit": " TL ", "speed": ""}, {"name": "Puan", "kind": "field", "field": "puan", "unit": "  ", "speed": "9"}]))
    add("şebeke: düğüm katmanları, roller, süzgeç ve kapalı ifadesi; kapalı kenarlar",
        form(id="ag-2", name="İçme suyu", kind="utility", direction="digitized", connect="vertices", tolerance="0.005",
             junctions=[{"layer": "vana", "role": "valve", "filter": "", "closed": " durum = 'kapalı' "}, {"layer": "depo", "role": "source", "filter": "tip = 'depo'", "closed": ""}],
             closed="durum = 'arızalı'"))
    add("tolerans sayı değil", form(tolerance="bir santim"))
    add("tolerans boş", form(tolerance=" "))
    add("hız sayı değil", form(costs=[{**SURE, "speed": "hızlı"}]))
    add("adsız ağın toleransı sayı değil", form(name="  ", tolerance="x"))
    return out


def retyped(f, kind):
    """The form with another kind: what is still the old kind's default becomes the new kind's."""
    if f["kind"] == kind:
        return f
    out = copy.deepcopy(f)
    out["kind"] = kind
    road_costs = len(f["costs"]) == 1 and f["costs"][0]["name"] == "Süre" and f["costs"][0]["kind"] == "speed" and f["costs"][0]["field"] == "hiz" and \
        decimal(f["costs"][0]["speed"] or "50") == DEFAULT_SPEED
    if kind == "utility":
        if f["direction"] == "both":
            out["direction"] = "digitized"
        if road_costs:
            out["costs"] = []
    else:
        if f["direction"] == "digitized":
            out["direction"] = "both"
        if not f["costs"]:
            out["costs"] = [dict(SURE)]
    return out


def retypes():
    pairs = [
        ("yeni yol ağı şebeke olur: çizim yönü, süre gider", form(costs=[dict(SURE)]), "utility"),
        ("değiştirilmiş yön ve maliyet kalır", form(direction="field", field="yon", forward="FT", costs=[{**SURE, "speed": "40"}]), "utility"),
        ("şebeke yol ağı olur: iki yön, süre gelir", form(kind="utility", direction="digitized"), "road"),
        ("şebekenin kendi maliyeti kalır", form(kind="utility", direction="both", costs=[{"name": "Boy", "kind": "field", "field": "boy", "unit": "m", "speed": ""}]), "road"),
        ("aynı tür", form(), "road"),
    ]
    return [{"name": n, "form": f, "kind": k, "expect": retyped(f, k)} for n, f, k in pairs]


def round_trips():
    """Networks shown in the form (`formOf`) and written back (`defOf`): the same network."""
    nets = [c["network"] for c in cases() if "network" in c and c["unit"] == "m"]
    return [n for n in nets]


def build():
    return {
        "format": "kentos.network-form",
        "version": 1,
        "note": "scripts/fixtures/network_form_cases.py yazar; Ağlar penceresinin form kuralları (docs/adr/0209 §10), KentOS kodu olmadan.",
        "cases": cases(),
        "retype": retypes(),
        "roundTrip": round_trips(),
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} is not what this script writes: run it without --check and read the difference.")
            return 1
        print(f"{OUT.relative_to(ROOT)} matches")
        return 0
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} written")
    return 0


if __name__ == "__main__":
    sys.exit(main())
