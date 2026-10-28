#!/usr/bin/env python3
"""ADR 0724 §2's shared cases: Mimari denetim's stairs group (`cad.arch.audit` with a stair profile in `profiles`), from
the ADR's rules and the profiles' values (catalog/v1/arch-profiles), without KentOS code.

Writes fixtures/commands/v1/cad.arch.audit.stairs.json; `--check` compares and writes nothing. Each case draws its
stairs with `cad.stairs.create` on a drawing without levels (no other group finds anything there) and asks the audit.

The rules, as ADR 0724 §2 gives them, for each stair in the drawing's order, with the profile's `stair` rules:

- a riser higher than `riser` is `stair_riser_over_profile`; a tread shorter than `tread` is `stair_tread_short`; two
  risers and a tread outside `step` is `stair_step_over_profile` (each 10⁻⁹ tolerant; none has a fix: the stair is
  designed again);
- a flight narrower than `width` is `stair_width_narrow`, fixed by setting the width with `cad.arch.edit`;
- with `landingWidth`, a stair with a landing (its walking line's turns, those less than a tread apart one) whose
  landing is shorter than its width is `stair_landing_short`, at its first landing's middle, fixed by setting its
  landing to null (its width);
- a curved stair whose tread at its inner side (the tread × (radius − width/2) / radius) is narrower than `winder` is
  `stair_winder_narrow`;
- with `bothRailings`, a stair without a railing on a side is `stair_railing_missing`, fixed by railings on both sides;
- every other finding at the stair's first point; on a locked layer none has a fix; the numbers in its words as the
  display rule writes them (numeric_display.py), the profile's id and “mevzuat onayı değildir” in each repair;
- order: the codes' order, then x, then y. No stair profile: nothing. `profiles` holds at most one profile with ramp
  rules and one with stair rules: an unknown one or a second of a kind is refused at its place.
"""

import argparse
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import arch_cases as ac  # noqa: E402  (the walking line's landings, a bulged line's arc)
from numeric_display import shown as fixed  # noqa: E402  (the display rule)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.arch.audit.stairs.json"
COMMAND = "cad.arch.audit"
PROFILES = ROOT / "catalog/v1/arch-profiles"


def profile(name):
    return json.loads((PROFILES / f"{name}.json").read_text("utf-8"))


KONUT = profile("imar-merdiven-konut-1")
ASANSORSUZ = profile("imar-merdiven-konut-asansorsuz-1")
DIGER = profile("imar-merdiven-diger-1")
DAIRE = profile("imar-merdiven-daire-ici-1")
RAMPA = profile("erisilebilirlik-rampa-1")
# The catalog's profiles in its order (crates/shared/commands' ARCH_PROFILES), as a refusal names them.
KNOWN = ["erisilebilirlik-rampa/1", "imar-merdiven-konut/1", "imar-merdiven-konut-asansorsuz/1", "imar-merdiven-diger/1",
         "imar-merdiven-daire-ici/1"]
CODES = ["stair_riser_over_profile", "stair_tread_short", "stair_step_over_profile", "stair_width_narrow",
         "stair_landing_short", "stair_winder_narrow", "stair_railing_missing"]
FIRST = 2  # the setup holds one line; the first stair made is 2
EPS = 1e-9


def P(x, y):
    return {"x": float(x), "y": float(y)}


def layer(id, name, locked=False):
    return {"id": id, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


SETUP = {
    "format": "kentos.document", "version": 1, "name": "Merdiven profilleri",
    "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 50,
                 "workspace": "cad"},
    "origin": {"x": 0, "y": 0},
    "layers": [layer("merdiven", "Merdiven"), layer("kilitli", "Kilitli katman")],
    "activeLayer": "merdiven",
    "styles": {"items": [], "categories": []},
    "entities": [{"kind": "line", "id": 1, "layerId": "merdiven", "attrs": {}, "a": P(0, -5), "b": P(5, -5)}],
}


def first_landing_middle(pts, width, landing, tread):
    """The middle of a straight stair's first landing (its turns' landings, those less than a tread apart one), as a
    point on its walking line; none without a turn."""
    from fractions import Fraction as F
    fp = [(F(p["x"]), F(p["y"])) for p in pts]
    at = ac.walk(fp)
    lands = ac.stair_landings(fp, at, F(landing), F(tread))
    if not lands:
        return None
    a, b = lands[0]
    p, _ = ac.point_on(fp, at, (a + b) / 2)
    return (float(p[0]), float(p[1]))


def findings(stairs, rules, profile_id):
    """The stairs group's findings of `stairs`: (id, fields, locked) in the drawing's order."""
    out = []
    regulation = f"{profile_id}, mevzuat onayı değildir"
    for i, s, locked in stairs:
        uid = f"$uidOf:{i}"
        start = P(s["pts"][0]["x"], s["pts"][0]["y"])

        def say(code, message, repair, at, fix=None):
            f = {"check": "stairs", "code": code, "message": message, "repair": repair, "uids": [uid], "at": at}
            if fix is not None and not locked:
                f["fix"] = fix
            out.append(f)

        def edit(label, operation):
            return {"label": label, "step": {"command": "cad.arch.edit",
                                              "input": {"uids": [uid], "operation": {"kind": "set", **operation}}}}

        riser, tread, width = s["riserHeight"], s["tread"], s["width"]
        if riser > rules["riser"] + EPS:
            say(CODES[0], f"Merdivenin rıhtı {fixed(riser, 3)} m; en çok {fixed(rules['riser'], 2)} m olmalı.",
                f"Rıht sayısını artırın; merdiven uzar ({regulation}).", start)
        if tread < rules["tread"] - EPS:
            say(CODES[1], f"Merdivenin basamağı {fixed(tread, 3)} m; en az {fixed(rules['tread'], 2)} m olmalı.",
                f"Basamağı derinleştirin; merdiven uzar ({regulation}).", start)
        step = 2.0 * riser + tread
        lo, hi = rules["step"]
        if step < lo - EPS or step > hi + EPS:
            say(CODES[2], f"Merdivenin 2a+b'si {fixed(step, 3)} m; {fixed(lo, 2)}–{fixed(hi, 2)} m olmalı.",
                f"Rıhtı ya da basamağı değiştirin ({regulation}).", start)
        if width < rules["width"] - EPS:
            say(CODES[3], f"Merdivenin kolu {fixed(width, 2)} m geniş; en az {fixed(rules['width'], 2)} m olmalı.",
                f"Merdiveni genişletin ({regulation}).", start,
                edit(f"Genişliği {fixed(rules['width'], 2)} m yap", {"width": rules["width"]}))
        curved = bool(s.get("bulges"))
        landing = s.get("landing", width)
        if rules["landingWidth"] and not curved:
            mid = first_landing_middle(s["pts"], width, landing, tread)
            if mid is not None and landing < width - EPS:
                say(CODES[4], f"Merdivenin sahanlığı {fixed(landing, 2)} m; en az kol genişliği ({fixed(width, 2)} m) kadar olmalı.",
                    f"Sahanlığı kol genişliği kadar yapın ({regulation}).", P(*mid),
                    edit("Sahanlığı kol genişliği yap", {"landing": None}))
        if curved:
            a, b = [(q["x"], q["y"]) for q in s["pts"]]
            _, r, _, _ = ac.arc_of(a, b, s["bulges"][0])
            inner = tread * (r - width / 2.0) / r
            if inner < rules["winder"] - EPS:
                say(CODES[5], f"Döner merdivenin basamağı dar uçta {fixed(inner, 3)} m; en az {fixed(rules['winder'], 2)} m olmalı.",
                    f"Yarıçapı büyütün ya da merdiveni daraltın ({regulation}).", start)
        if rules["bothRailings"] and s.get("railings") != "both":
            missing = {"left": "Merdivenin sağ yanında korkuluk yok", "right": "Merdivenin sol yanında korkuluk yok"}.get(
                s.get("railings"), "Merdivende korkuluk yok")
            say(CODES[6], f"{missing}; iki yanında olmalı.", f"İki yanına korkuluk koyun ({regulation}).", start,
                edit("İki yana korkuluk koy", {"railings": "both"}))
    return sorted(out, key=lambda f: (CODES.index(f["code"]), f["at"]["x"], f["at"]["y"]))


def create(stairs, layer_id="merdiven"):
    return {"op": "execute", "command": "cad.stairs.create", "input": {"layerId": layer_id, "stairs": stairs},
            "result": "$any"}


def audit(found, inp, op="execute"):
    result = {"status": "completed", "output": {"findings": found, "revision": "$current"}, "warnings": []}
    if op == "validate":
        result = {"status": "completed", "output": None, "warnings": []}
    return {"op": op, "command": COMMAND, "input": inp, "result": result, "within": 1e-9, "expect": {"revision": "same"}}


def refused(inp, message, path):
    return {"op": "execute", "command": COMMAND, "input": inp,
            "result": {"status": "failed", "error": {"code": "invalid_arch_audit", "message": message, "path": path}},
            "expect": {"revision": "same"}}


def stair(pts, width=1.2, riser=0.17, tread=0.28, risers=16, **kw):
    s = {"pts": [P(*p) for p in pts], "width": width, "risers": risers, "riserHeight": riser, "tread": tread,
         "railings": "both"}
    s.update(kw)
    return {k: v for k, v in s.items() if v is not None}


def straight(**kw):
    return stair([(0, 0), (4.2, 0)], **kw)


def asked(p=KONUT):
    return {"profiles": [p["id"]], "checks": ["stairs"]}


def one(name, s, p=KONUT, inp=None, locked=False):
    """A case: `s` drawn (on Kilitli katman, locked after, when `locked`), then audited with profile `p`."""
    steps = [create([s], "kilitli" if locked else "merdiven")]
    if locked:
        steps.append({"op": "execute", "command": "cad.layers.set",
                      "input": {"operation": "locked", "layer": "kilitli", "value": True}, "result": "$any"})
    inp = asked(p) if inp is None else inp
    measured = p["id"] in inp.get("profiles", []) and (not inp.get("checks") or "stairs" in inp["checks"])
    steps.append(audit(findings([(FIRST, s, locked)], p["stair"], p["id"]) if measured else [], inp))
    return {"name": name, "steps": steps}


def cases():
    out = []
    L = [(0, 0), (4, 0), (4, 4)]
    out.append(one("konut profiline uyan düz merdiven geçer", straight()))
    out.append(one("18 cm'yi aşan rıht", straight(riser=0.185, tread=0.27)))
    out.append(one("27 cm'den kısa basamak", straight(riser=0.17, tread=0.26)))
    out.append(one("2a+b 64 cm'yi aşar", straight(riser=0.17, tread=0.32)))
    out.append(one("2a+b 60 cm'nin altında", straight(riser=0.15, tread=0.28)))
    out.append(one("rıht, basamak ve 2a+b birlikte", straight(riser=0.2, tread=0.25)))
    out.append(one("1,20 m'den dar kol, düzeltmesiyle", straight(width=1.0)))
    out.append(one("kol genişliğinden kısa sahanlıklı L, düzeltmesiyle", stair(L, landing=0.8)))
    out.append(one("sahanlığı kol genişliği kadar L geçer", stair(L)))
    out.append(one("sahanlığı kol genişliğinden uzun L geçer", stair(L, landing=1.5)))
    out.append(one("U merdivenin tek sahanlığı kısa", stair([(0, 0), (4, 0), (4, 1.3), (0, 1.3)], landing=0.9)))
    out.append(one("dar uçta 10 cm'den dar döner merdiven",
                   stair([(0.8, 0), (-0.8, 0)], bulges=[1.0, 0.0], width=1.2, risers=10, tread=0.27)))
    out.append(one("dar ucu yeterli döner merdiven geçer",
                   stair([(1.5, 0), (-1.5, 0)], bulges=[1.0, 0.0], width=1.2, risers=12, tread=0.27)))
    out.append(one("yalnız sol korkuluk: sağ yanda yok, düzeltmesiyle", straight(railings="left")))
    out.append(one("yalnız sağ korkuluk: sol yanda yok", straight(railings="right")))
    out.append(one("korkuluksuz merdiven", straight(railings=None)))
    out.append(one("kilitli katmandaki merdivenin bulguları düzeltmesiz", stair(L, width=1.0, landing=0.8, railings=None),
                   locked=True))
    out.append(one("asansörsüz konut profili 16 cm'yi aşan rıhtı bulur", straight(), p=ASANSORSUZ))
    out.append(one("diğer yapılar profili 1,50 m'den dar kolu bulur", straight(), p=DIGER))
    out.append(one("daire içi profili 1,00 m kolu geçirir", straight(width=1.0), p=DAIRE))
    out.append(one("merdiven profili verilmeyince merdivenler denetlenmez", straight(width=1.0),
                   inp={"checks": ["stairs"]}))
    out.append(one("profil verilip grup sorulmayınca bulgu yok", straight(width=1.0),
                   inp={"profiles": [KONUT["id"]], "checks": ["levels"]}))

    # Every group with a ramp profile and a stair profile: the ramps' and the stairs' findings, in the groups' order.
    s = straight(width=1.0)
    ramp = {"pts": [P(0, 10), P(1.8, 10)], "width": 1.5, "rise": 0.15}
    ramp_found = [{"check": "accessibility", "code": "ramp_slope_over_profile",
                   "message": f"Rampanın eğimi %{fixed(0.15 / 1.8 * 100.0, 2)}; 0.15 m kot farkı için en çok %8.",
                   "repair": f"Rampayı uzatın ya da kot farkını azaltın; eğim en çok %8 olmalı ({RAMPA['id']}, mevzuat onayı değildir).",
                   "uids": [f"$uidOf:{FIRST + 1}"], "at": P(0, 10)}]
    out.append({"name": "rampa ve merdiven profili birlikte: iki grubun bulguları",
                "steps": [create([s]), {"op": "execute", "command": "cad.ramps.create",
                                        "input": {"layerId": "merdiven", "ramps": [ramp]}, "result": "$any"},
                          audit(ramp_found + findings([(FIRST, s, False)], KONUT["stair"], KONUT["id"]),
                                {"profiles": [RAMPA["id"], KONUT["id"]]})]})
    # Several stairs: the codes' order, then x, then y.
    ss = [straight(width=1.0, railings="left"), stair([(0, 5), (4.2, 5)], width=1.1), stair([(-3, 2), (1.2, 2)], width=1.0)]
    out.append({"name": "üç merdiven: kodların sırası, sonra x, sonra y",
                "steps": [create(ss), audit(findings([(FIRST + k, x, False) for k, x in enumerate(ss)], KONUT["stair"], KONUT["id"]),
                                            asked())]})
    # The fixes run as they are: audited again, what they mend is gone.
    s = stair(L, width=1.0, landing=0.8, railings="left")
    widened = dict(s, width=1.2)
    out.append({"name": "genişlik düzeltmesi çalıştırılınca dar kol kalkar",
                "steps": [create([s]), audit(findings([(FIRST, s, False)], KONUT["stair"], KONUT["id"]), asked()),
                          {"op": "execute", "command": "cad.arch.edit",
                           "input": {"uids": [f"$uidOf:{FIRST}"], "operation": {"kind": "set", "width": 1.2}}, "result": "$any"},
                          audit(findings([(FIRST, widened, False)], KONUT["stair"], KONUT["id"]), asked())]})
    landed = {k: v for k, v in s.items() if k != "landing"}
    out.append({"name": "sahanlık düzeltmesi çalıştırılınca kısa sahanlık kalkar",
                "steps": [create([s]),
                          {"op": "execute", "command": "cad.arch.edit",
                           "input": {"uids": [f"$uidOf:{FIRST}"], "operation": {"kind": "set", "landing": None}}, "result": "$any"},
                          audit(findings([(FIRST, landed, False)], KONUT["stair"], KONUT["id"]), asked())]})
    railed = dict(s, railings="both")
    out.append({"name": "korkuluk düzeltmesi çalıştırılınca eksik korkuluk kalkar",
                "steps": [create([s]),
                          {"op": "execute", "command": "cad.arch.edit",
                           "input": {"uids": [f"$uidOf:{FIRST}"], "operation": {"kind": "set", "railings": "both"}}, "result": "$any"},
                          audit(findings([(FIRST, railed, False)], KONUT["stair"], KONUT["id"]), asked())]})
    s = straight(width=1.0)
    out.append({"name": "plan aynı bulguları verir",
                "steps": [create([s]), audit(findings([(FIRST, s, False)], KONUT["stair"], KONUT["id"]), asked(), op="plan")]})
    out.append({"name": "doğrula bir şey yazmaz", "steps": [create([s]), audit([], asked(), op="validate")]})
    known = ", ".join(KNOWN)
    out.append({"name": "katalogda olmayan profil reddedilir",
                "steps": [create([s]), refused({"profiles": [KONUT["id"], "yok/1"]},
                                               f"“yok/1” profili yok; şunlardan birini verin: {known}.", "profiles[1]")]})
    out.append({"name": "iki merdiven profili reddedilir",
                "steps": [create([s]), refused({"profiles": [KONUT["id"], DIGER["id"]]},
                                               "Merdivenler için bir profil verilir; ikinci merdiven profilini kaldırın.",
                                               "profiles[1]")]})
    out.append({"name": "aynı rampa profili iki kez reddedilir",
                "steps": [create([s]), refused({"profiles": [RAMPA["id"], RAMPA["id"]]},
                                               "Rampalar için bir profil verilir; ikinci rampa profilini kaldırın.",
                                               "profiles[1]")]})
    v = refused({"profiles": ["yok/1"]}, f"“yok/1” profili yok; şunlardan birini verin: {known}.", "profiles[0]")
    v["op"] = "validate"
    out.append({"name": "doğrula olmayan profili reddeder", "steps": [create([s]), v]})
    return out


def document():
    note = ("ADR 0724 §2. Beklenenler scripts/fixtures/stair_access_cases.py'nin ADR'den ve profillerin değerlerinden "
            "(catalog/v1/arch-profiles) yazılmış kurallarından, KentOS kodu olmadan. Kurulumdaki çizimde bir çizgi var, kat "
            "yok; merdivenler 2'den başlar.")
    return {"format": "kentos.command-cases", "version": 1, "command": COMMAND, "commandVersion": 1,
            "title": "Mimari denetim: merdivenlerin İmar Yönetmeliği md. 31 profilleri (rıht, basamak, 2a+b, kol, sahanlık, "
                     "döner basamak, korkuluk); düzeltme; retler",
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
        print(f"{COMMAND} (merdivenler): {len(doc['cases'])} durum başvuruyla aynı.")
        return 0
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(doc['cases'])} durum yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
