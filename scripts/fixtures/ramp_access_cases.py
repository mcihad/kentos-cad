#!/usr/bin/env python3
"""ADR 0723 §4's shared cases: Mimari denetim's accessibility group (`cad.arch.audit` with `profile`), from the ADR's
rules and the profile's values, without KentOS code.

Writes fixtures/commands/v1/cad.arch.audit.accessibility.json; `--check` compares and writes nothing. Each case draws
its ramps with `cad.ramps.create` on a drawing without levels (no other group finds anything there) and asks the audit.

The rules, as ADR 0723 §4 gives them, with the profile `erisilebilirlik-rampa/1` (catalog/v1/accessibility):

- A ramp's slope as drawn (the rise's size over its sloped length, `ramp_geometry.py`) above the steepest its rise's
  size allows (the first band whose bound it does not pass; 0.15 m: %8, 0.50: %7, 1.00: %6, above: %5) is
  `ramp_slope_over_profile`; a slope equal to it passes. A ramp without a rise or a sloped run is not measured.
- Each sloped run (between the start, the landings and the end) longer than 10 m on a ramp steeper than %6 is
  `ramp_long_run_slope`, at the run's start.
- A ramp with a landing shorter than 1.5 m is `ramp_landing_short` (at its first landing's middle; fixed by setting
  the landing to 1.5 m with `cad.arch.edit`); a ramp that turns (its direction changes at an inner point; a straight
  corner does not) narrower than 1.5 m is `ramp_turn_narrow` (at its first turn; fixed by setting its width). On a
  locked layer there is no fix.
- Order: the codes' order, then x, then y. No profile: nothing; an unknown one is refused.
"""

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ramp_geometry as rg  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.arch.audit.accessibility.json"
COMMAND = "cad.arch.audit"
PROFILE = json.loads((ROOT / "catalog/v1/accessibility/erisilebilirlik-rampa-1.json").read_text("utf-8"))
RULES = PROFILE["ramp"]
CODES = ["ramp_slope_over_profile", "ramp_long_run_slope", "ramp_landing_short", "ramp_turn_narrow"]
FIRST = 2  # the setup holds one line; the first ramp made is 2


def P(x, y):
    return {"x": float(x), "y": float(y)}


def num(x):
    """A number as Rust's `{}` writes an f64."""
    x = float(x)
    return str(int(x)) if x.is_integer() and abs(x) < 1e16 else repr(x)


def fixed(v, d):
    t = f"{abs(v):.{d}f}"
    return ("-" + t) if v < 0 and float(t) != 0 else t


def layer(id, name, locked=False):
    return {"id": id, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


SETUP = {
    "format": "kentos.document", "version": 1, "name": "Erişilebilir rampalar",
    "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 50,
                 "workspace": "cad"},
    "origin": {"x": 0, "y": 0},
    "layers": [layer("rampa", "Rampa"), layer("kilitli", "Kilitli katman")],
    "activeLayer": "rampa",
    "styles": {"items": [], "categories": []},
    "entities": [{"kind": "line", "id": 1, "layerId": "rampa", "attrs": {}, "a": P(0, -5), "b": P(5, -5)}],
}


def slope_for(rise):
    for band in RULES["slopes"]:
        if band.get("upTo") is None or rise <= band["upTo"]:
            return band["max"]
    return None


def findings(ramps):
    """The accessibility findings of `ramps`: (id, fields, locked) in the drawing's order."""
    out = []
    for i, r, locked in ramps:
        pts = [(p["x"], p["y"]) for p in r["pts"]]
        land = r.get("landing", r["width"])
        at = rg.stations(pts)
        run = rg.sloped_length(pts, land)
        rise = abs(r["rise"])
        uid = f"$uidOf:{i}"
        if run > 1e-9 and rise > 0.0:
            slope = rise / run * 100.0
            top = slope_for(rise)
            if top is not None and slope > top + 1e-9:
                out.append({"check": "accessibility", "code": CODES[0],
                            "message": f"Rampanın eğimi %{fixed(slope, 2)}; {fixed(rise, 2)} m kot farkı için en çok %{num(top)}.",
                            "repair": f"Rampayı uzatın ya da kot farkını azaltın; eğim en çok %{num(top)} olmalı "
                                      f"({PROFILE['id']}, mevzuat onayı değildir).",
                            "uids": [uid], "at": P(*pts[0])})
            for k, (a, b) in enumerate(rg.runs(pts, land)):
                if b - a > RULES["longRun"]["over"] + 1e-9 and slope > RULES["longRun"]["max"] + 1e-9:
                    out.append({"check": "accessibility", "code": CODES[1],
                                "message": f"Rampanın {k + 1}. eğimli kolu {fixed(b - a, 2)} m, eğimi %{fixed(slope, 2)}; "
                                           f"{num(RULES['longRun']['over'])} m'den uzun kolda en çok %{num(RULES['longRun']['max'])}.",
                                "repair": "Rampayı En uzun kol ile ara sahanlıklı yeniden çizin ya da eğimini düşürün.",
                                "uids": [uid], "at": P(*rg.point(pts, at, a)[0])})
        lands = rg.landings(pts, land)
        if lands and land < RULES["landing"] - 1e-9:
            a, b = lands[0]
            f = {"check": "accessibility", "code": CODES[2],
                 "message": f"Rampanın sahanlığı {fixed(land, 2)} m; en az {fixed(RULES['landing'], 2)} m olmalı.",
                 "repair": "Sahanlığı uzatın.", "uids": [uid], "at": P(*rg.point(pts, at, (a + b) / 2.0)[0])}
            if not locked:
                f["fix"] = {"label": f"Sahanlığı {fixed(RULES['landing'], 2)} m yap",
                            "step": {"command": "cad.arch.edit",
                                     "input": {"uids": [uid], "operation": {"kind": "set", "landing": RULES["landing"]}}}}
            out.append(f)
        turns = rg.turn_points(pts)
        if turns and r["width"] < RULES["turn"] - 1e-9:
            t = fixed(RULES["turn"], 2)
            f = {"check": "accessibility", "code": CODES[3],
                 "message": f"Rampa dönüşte {fixed(r['width'], 2)} m geniş; yön değiştiren sahanlık en az {t} × {t} m olmalı.",
                 "repair": "Rampayı genişletin.", "uids": [uid], "at": P(*pts[turns[0]])}
            if not locked:
                f["fix"] = {"label": f"Genişliği {t} m yap",
                            "step": {"command": "cad.arch.edit",
                                     "input": {"uids": [uid], "operation": {"kind": "set", "width": RULES["turn"]}}}}
            out.append(f)
    return sorted(out, key=lambda f: (CODES.index(f["code"]), f["at"]["x"], f["at"]["y"]))


def create(ramps, layer_id="rampa"):
    return {"op": "execute", "command": "cad.ramps.create", "input": {"layerId": layer_id, "ramps": ramps}, "result": "$any"}


def audit(found, inp=None, op="execute"):
    inp = {"profile": PROFILE["id"], "checks": ["accessibility"]} if inp is None else inp
    result = {"status": "completed", "output": {"findings": found, "revision": "$current"}, "warnings": []}
    if op == "validate":
        result = {"status": "completed", "output": None, "warnings": []}
    return {"op": op, "command": COMMAND, "input": inp, "result": result, "within": 1e-9, "expect": {"revision": "same"}}


def ramp(pts, width, rise, **kw):
    return {"pts": [P(*p) for p in pts], "width": width, "rise": rise, **kw}


def one(name, r, inp=None, locked=False):
    """A case: `r` drawn (on Kilitli katman, locked after, when `locked`), then audited."""
    steps = [create([r], "kilitli" if locked else "rampa")]
    if locked:
        steps.append({"op": "execute", "command": "cad.layers.set",
                      "input": {"operation": "locked", "layer": "kilitli", "value": True}, "result": "$any"})
    asked = inp is None or (inp.get("profile") is not None
                            and (not inp.get("checks") or "accessibility" in inp["checks"]))
    steps.append(audit(findings([(FIRST, r, locked)]) if asked else [], inp))
    return {"name": name, "steps": steps}


def cases():
    out = []
    out.append(one("15 cm kot farkında tam %8 rampa geçer", ramp([(0, 0), (1.875, 0)], 1.5, 0.15)))
    out.append(one("15 cm kot farkında %8'i aşan rampa", ramp([(0, 0), (1.8, 0)], 1.5, 0.15)))
    out.append(one("50 cm kot farkında %7'yi aşan rampa", ramp([(0, 0), (7, 0)], 1.5, 0.5)))
    out.append(one("50 cm kot farkında %7 içinde kalan rampa geçer", ramp([(0, 0), (7.5, 0)], 1.5, 0.5)))
    out.append(one("16 cm kot farkı %7 bandında: %7.62 rampa", ramp([(0, 0), (2.1, 0)], 1.5, 0.16)))
    out.append(one("1 m kot farkında %6'yı aşan 16 m kollu rampa: eğim ve uzun kol", ramp([(0, 0), (16, 0)], 1.5, 1.0)))
    out.append(one("1,20 m kot farkında %5 rampa geçer, uzun kolu %6 içinde", ramp([(0, 0), (24, 0)], 1.5, 1.2)))
    out.append(one("1,20 m kot farkında %5'i aşan rampa", ramp([(0, 0), (20, 0)], 1.5, 1.2)))
    out.append(one("inen rampanın eğimi kot farkının büyüklüğüyle sınanır", ramp([(0, 0), (6, 0)], 1.5, -0.5, base=0.5)))
    out.append(one("dar dönüşlü L rampa: kısa sahanlık ve dar dönüş, iki düzeltme",
                   ramp([(0, 0), (6, 0), (6, 6)], 1.2, 0.5)))
    out.append(one("geniş L rampa 1,50 m sahanlıkla geçer", ramp([(0, 0), (6, 0), (6, 6)], 1.5, 0.5, landing=1.5)))
    out.append(one("U rampa 1,40 m geniş: dar dönüş, sahanlık yeterli",
                   ramp([(0, 0), (8, 0), (8, 3), (0, 3)], 1.4, 0.6, landing=1.5)))
    out.append(one("düz köşeli rampa dönmez: yalnız kısa ara sahanlık",
                   ramp([(0, 0), (9.6, 0), (19.2, 0)], 1.2, 0.9, landing=1.2)))
    out.append(one("eğimsiz (düz) rampa ölçülmez, sahanlığı da yok", ramp([(0, 0), (5, 0)], 1.2, 0.0)))
    out.append(one("eğimli kolu kalmayan rampa: yalnız dar dönüş",
                   ramp([(0, 0), (1, 0), (1, 1)], 1.0, 0.2, landing=5.0)))
    out.append(one("kilitli katmandaki rampanın bulguları düzeltmesiz",
                   ramp([(0, 0), (6, 0), (6, 6)], 1.2, 0.5), locked=True))
    out.append(one("profil verilmeyince erişilebilirlik bir şey bulmaz", ramp([(0, 0), (1.8, 0)], 1.5, 0.15),
                   inp={"checks": ["accessibility"]}))
    out.append(one("profil verilip grup sorulmayınca bulgu yok", ramp([(0, 0), (1.8, 0)], 1.5, 0.15),
                   inp={"profile": PROFILE["id"], "checks": ["levels"]}))

    # Every group asked (checks empty): only the accessibility group finds something in this drawing.
    r = ramp([(0, 0), (6, 0), (6, 6)], 1.2, 0.5)
    out.append({"name": "bütün gruplar profille: yalnız erişilebilirlik bulguları",
                "steps": [create([r]), audit(findings([(FIRST, r, False)]), {"profile": PROFILE["id"]})]})
    # Several ramps: the codes' order, then x, then y.
    rs = [ramp([(10, 0), (11.8, 0)], 1.5, 0.15), ramp([(0, 5), (6, 5), (6, 11)], 1.2, 0.5),
          ramp([(0, 0), (1.8, 0)], 1.5, 0.15)]
    out.append({"name": "üç rampa: kodların sırası, sonra x, sonra y",
                "steps": [create(rs), audit(findings([(FIRST + k, r, False) for k, r in enumerate(rs)]))]})
    # The fix run as it is: the landing set; audited again, only the narrow turn is left.
    r = ramp([(0, 0), (6, 0), (6, 6)], 1.2, 0.5)
    fixed_r = dict(r, landing=RULES["landing"])
    out.append({"name": "sahanlık düzeltmesi çalıştırılınca yalnız dar dönüş kalır",
                "steps": [create([r]), audit(findings([(FIRST, r, False)])),
                          {"op": "execute", "command": "cad.arch.edit",
                           "input": {"uids": [f"$uidOf:{FIRST}"], "operation": {"kind": "set", "landing": RULES["landing"]}},
                           "result": "$any"},
                          audit(findings([(FIRST, fixed_r, False)]))]})
    out.append({"name": "plan aynı bulguları verir",
                "steps": [create([r]), audit(findings([(FIRST, r, False)]), op="plan")]})
    out.append({"name": "doğrula bir şey yazmaz", "steps": [create([r]), audit([], op="validate")]})
    out.append({"name": "katalogda olmayan profil reddedilir",
                "steps": [create([r]), {"op": "execute", "command": COMMAND, "input": {"profile": "yok/1"},
                                        "result": {"status": "failed", "error": {
                                            "code": "invalid_arch_audit",
                                            "message": f"“yok/1” erişilebilirlik profili yok; {PROFILE['id']} verin.",
                                            "path": "profile"}},
                                        "expect": {"revision": "same"}}]})
    return out


def document():
    note = ("ADR 0723 §4. Beklenenler scripts/fixtures/ramp_access_cases.py'nin ADR'den ve profilin değerlerinden "
            "(catalog/v1/accessibility/erisilebilirlik-rampa-1.json) yazılmış kurallarından, KentOS kodu olmadan. "
            "Kurulumdaki çizimde bir çizgi var, kat yok; rampalar 2'den başlar.")
    return {"format": "kentos.command-cases", "version": 1, "command": COMMAND, "commandVersion": 1,
            "title": "Mimari denetim: rampaların erişilebilirlik profili (eğim bandı, uzun kol, sahanlık, dönüş); düzeltme; retler",
            "note": note, "setup": SETUP, "cases": cases()}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    doc = document()
    assert len(doc["cases"]) >= 20, len(doc["cases"])
    text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} başvurudan farklı; --check'siz yeniden yazın ve farkı okuyun.")
            return 1
        print(f"{COMMAND} (erişilebilirlik): {len(doc['cases'])} durum başvuruyla aynı.")
        return 0
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(doc['cases'])} durum yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
