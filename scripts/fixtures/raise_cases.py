#!/usr/bin/env python3
"""The shared cases of `cad.entities.raise` (Referanstan kot ver; plan/map-cad.md MAP-06, docs/adr/3780 and 4980),
without KentOS code.

    python3 scripts/fixtures/raise_cases.py           # writes fixtures/commands/v1/cad.entities.raise.json
    python3 scripts/fixtures/raise_cases.py --check   # writes nothing; compares

The rule, by hand: the vertex `at` (a line's ends 0 and 1, a polyline's or an area's vertices of the ring `ring` of the
part `part`, docs/adr/4980: 0 the object's own vertices and the outer ring, 1 on the further parts and the holes) stands d = √((x − xr)² + (y − yr)²) from the reference; its new elevation is
zr + grade·d, or zr + difference. Every other elevation of the object (the holes', every part's) moves by as much as the
vertex: by = new − old; the vertex takes the new elevation itself. A vertex without an elevation stays without. The
answer's grade is the grade given, or (new − zr) / d for a difference; none when d is 0. No such part or ring:
`wrong_place` on `part` or `ring`; no such vertex on the ring: `wrong_place` on `at`, naming the ring.
"""

import copy
import json
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.entities.raise.json"
X0, Y0 = 500000.0, 4400000.0
STEP = "Referanstan kot ver"


def uid(n):
    return f"0192f5c0-3780-7000-8000-{n:012x}"


def layer(id_, name, locked=False):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def P(x, y):
    return {"x": X0 + x, "y": Y0 + y}


# A kerb with one vertex left without an elevation and an arc edge (the arc does not count: d is straight).
POLY = {"kind": "polyline", "id": 1, "layerId": "tasarim", "attrs": {},
        "pts": [P(0, 0), P(30, 0), P(30, 40), P(80, 40), P(100, 40)],
        "bulges": [0.0, 0.0, 0.5, 0.0, 0.0], "zs": [850.0, None, 850.4, 851.25, 851.0]}
LINE = {"kind": "line", "id": 2, "layerId": "tasarim", "attrs": {}, "a": P(0, 100), "b": P(60, 180), "za": 900.0,
        "zb": 901.5}
# An area of two parts, each with a hole; the second part's hole without elevations.
AREA = {"kind": "polygon", "id": 3, "layerId": "tasarim", "attrs": {},
        "pts": [P(200, 0), P(240, 0), P(240, 30), P(200, 30)], "zs": [120.0, 120.2, 120.5, 120.3],
        "holes": [{"pts": [P(210, 10), P(220, 10), P(220, 20)], "zs": [119.9, 119.95, 120.0]}],
        "parts": [{"pts": [P(300, 0), P(330, 0), P(330, 25)], "zs": [121.0, None, 121.4],
                   "holes": [{"pts": [P(310, 5), P(320, 5), P(320, 15)]}]}]}
LOCKED = {"kind": "polyline", "id": 4, "layerId": "kilitli", "attrs": {}, "pts": [P(0, 300), P(10, 300)],
          "zs": [10.0, 10.5]}
TEXT = {"kind": "text", "id": 5, "layerId": "tasarim", "attrs": {}, "text": "Kot", "p": P(0, 400), "height": 2.0,
        "rotation": 0.0}
BARE = {"kind": "polyline", "id": 6, "layerId": "tasarim", "attrs": {}, "pts": [P(0, 500), P(20, 500), P(20, 520)]}
# A polyline of two parts: the second part's elevations move too.
PARTS = {"kind": "polyline", "id": 7, "layerId": "tasarim", "attrs": {},
         "pts": [P(400, 0), P(450, 0)], "zs": [860.0, 860.5],
         "parts": [{"pts": [P(400, 20), P(450, 20), P(450, 60)], "zs": [861.0, None, 861.8]}]}
POINT = {"kind": "point", "id": 8, "layerId": "tasarim", "attrs": {}, "p": P(0, 600), "z": 5.0}
ENTS = [POLY, LINE, AREA, LOCKED, TEXT, BARE, PARTS, POINT]

# References: a door's threshold beside the kerb, a manhole lid, a typed one.
ESIK = {"x": X0 - 20.0, "y": Y0 + 10.0, "z": 849.75}
KAPAK = {"x": X0 + 90.0, "y": Y0 + 140.0, "z": 899.2}
YAZILAN = {"x": X0 + 260.0, "y": Y0 - 15.0, "z": 120.75}


def setup():
    return {"format": "kentos.document", "version": 2, "name": "Referanstan kot",
            "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                         "plotScale": 1000.0, "workspace": "cad", "cadWork": "map"},
            "origin": {"x": X0, "y": Y0}, "layers": [layer("0", "0"), layer("tasarim", "Tasarım"),
                                                     layer("kilitli", "Kilitli", locked=True)],
            "activeLayer": "tasarim", "entities": copy.deepcopy(ENTS), "uids": [uid(i + 1) for i in range(len(ENTS))],
            "styles": {"items": [], "categories": []}}


SAME = {"revision": "same", "canUndo": False}


def ring_of(e, rg):
    """The ring `rg` (part, ring) of a polyline or an area (docs/adr/4980): the dict holding its pts and zs."""
    part, ring = rg
    owner = e if part == 0 else e["parts"][part - 1]
    return owner if ring == 0 else owner["holes"][ring - 1]


def numbered(e, rg=(0, 0)):
    """The vertices `at` counts, each with its elevation."""
    if e["kind"] == "line":
        return [(e["a"], e.get("za")), (e["b"], e.get("zb"))]
    r = ring_of(e, rg)
    zs = r.get("zs", [None] * len(r["pts"]))
    return list(zip(r["pts"], zs))


def lifted(zs, n, by):
    """A list of elevations moved by `by` (none stays none), how many moved and how many have none."""
    if zs is None:
        return None, 0, n
    out = [None if z is None else z + by for z in zs]
    return out, sum(z is not None for z in zs), sum(z is None for z in zs)


def raised(e, at, ref, grade=None, difference=None, rg=(0, 0)):
    """The object after, and the answer without the revision."""
    p, z0 = numbered(e, rg)[at]
    d = math.hypot(p["x"] - ref["x"], p["y"] - ref["y"])
    new = ref["z"] + grade * d if grade is not None else ref["z"] + difference
    by = new - z0
    after = copy.deepcopy(e)
    moved = unset = 0
    if e["kind"] == "line":
        zs, m, u = lifted([e.get("za"), e.get("zb")], 2, by)
        zs[at] = new
        after["za"], after["zb"] = zs
        moved, unset = m, u
    else:
        def ring(r):
            nonlocal moved, unset
            out, m_, u_ = lifted(r.get("zs"), len(r["pts"]), by)
            moved, unset = moved + m_, unset + u_
            if out is not None:
                r["zs"] = out

        ring(after)
        for h in after.get("holes", []):
            ring(h)
        for part in after.get("parts", []):
            ring(part)
            for h in part.get("holes", []):
                ring(h)
        # The vertex takes the new elevation itself, on its ring (docs/adr/4980).
        ring_of(after, rg)["zs"][at] = new
    g = None if d == 0.0 else (grade if grade is not None else (new - ref["z"]) / d)
    out = {"distance": d, "before": z0, "after": new, "by": by}
    if g is not None:
        out["grade"] = g
    out.update({"moved": moved, "unset": unset})
    return after, out


KEYS = ["zs", "za", "zb", "holes", "parts"]


def ok(name, slot, at, ref, op="execute", grade=None, difference=None, rg=(0, 0)):
    e = ENTS[slot - 1]
    after, out = raised(e, at, ref, grade, difference, rg)
    input_ = {"uid": uid(slot), "at": at, **{k: v for k, v in (("part", rg[0]), ("ring", rg[1])) if v}, "reference": ref}
    if grade is not None:
        input_["grade"] = grade
    if difference is not None:
        input_["difference"] = difference
    if op == "validate":
        result = {"status": "completed", "output": None, "warnings": []}
    else:
        result = {"status": "completed", "output": dict(out, revision="$current"), "warnings": []}
    if op != "execute":
        return {"name": name, "steps": [{"op": op, "input": input_, "result": result, "expect": SAME, "within": 1e-9}]}
    fields = {k: after[k] for k in KEYS if k in after}
    return {"name": name, "steps": [
        {"op": "execute", "input": input_, "result": result, "within": 1e-9,
         "expect": {"revision": "changed", "canUndo": True, "fields": {str(slot): fields}}},
        {"op": "undo", "returns": STEP, "within": 1e-9,
         "expect": {"fields": {str(slot): {k: e[k] for k in KEYS if k in e}}}}]}


def refused(name, input_, code, message, path_):
    return {"name": name, "steps": [{"op": "execute", "input": input_, "result": {"status": "failed", "error": {
        "code": code, "message": message, "path": path_}}, "expect": SAME}]}


def cases():
    both = "Referanstan eğimi (grade) ya da kot farkını (difference) verin, ikisini birden değil."
    on_ref = dict(POLY["pts"][2], z=849.0)
    return [
        ok("Eğimle: köşe eşikten %2 yukarıda; bütün kotlar aynı farkla kayar, kotsuz köşe kotsuz kalır; geri alma", 1, 3,
           ESIK, grade=0.02),
        ok("Eksi eğim: köşe eşikten aşağıda", 1, 0, ESIK, grade=-0.015),
        ok("Kot farkıyla: köşe referanstan 15 cm yukarıda", 1, 4, ESIK, difference=0.15),
        ok("Çizgi: ucu kapaktan %3 yukarıda, öbür ucu aynı farkla", 2, 0, KAPAK, grade=0.03),
        ok("Çizgi: öbür ucu kapaktan 40 cm aşağıda", 2, 1, KAPAK, difference=-0.4),
        ok("Alan: dış halka, delikler ve öbür parça aynı farkla; kotsuz delik kotsuz kalır", 3, 2, YAZILAN, grade=-0.01),
        ok("Alan: kot farkıyla", 3, 0, YAZILAN, difference=0.05),
        ok("Çoklu çizginin öbür parçası da kayar", 7, 1, ESIK, grade=0.005),
        ok("Köşe referansın üstünde: eğim yok, köşe referansın kotunu alır", 1, 2, on_ref, grade=0.05),
        ok("Köşe referansın üstünde, kot farkıyla: eğim yok", 1, 2, on_ref, difference=0.25),
        ok("Sıfır eğim: köşe referansın kotunda", 1, 3, ESIK, grade=0.0),
        ok("Plan yazmaz", 1, 3, ESIK, op="plan", grade=0.02),
        ok("Doğrulama", 1, 3, ESIK, op="validate", grade=0.02),
        refused("Hem eğim hem kot farkı", {"uid": uid(1), "at": 0, "reference": ESIK, "grade": 0.01, "difference": 0.1},
                "invalid_input", both, "grade"),
        refused("Ne eğim ne kot farkı", {"uid": uid(1), "at": 0, "reference": ESIK}, "invalid_input", both, "grade"),
        refused("Sonlu olmayan kot", {"uid": uid(1), "at": 3, "reference": ESIK, "grade": 1e308}, "invalid_input",
                "Hesaplanan kot sonlu bir sayı değil; referansı ve eğimi denetleyin.", "grade"),
        refused("Köşe sayısının dışında", {"uid": uid(1), "at": 5, "reference": ESIK, "grade": 0.01}, "wrong_place",
                "5. köşe yok: nesnenin 5 köşesi var, sıra 0'dan başlar.", "at"),
        refused("Çizginin üçüncü köşesi yok", {"uid": uid(2), "at": 2, "reference": KAPAK, "grade": 0.01},
                "wrong_place", "2. köşe yok: nesnenin 2 köşesi var, sıra 0'dan başlar.", "at"),
        refused("Köşenin kotu yok", {"uid": uid(1), "at": 1, "reference": ESIK, "grade": 0.01}, "elevation_missing",
                "1. köşenin kotu yok; fark bulunamıyor. Kotlu bir köşe seçin ya da önce Kot ver ile kot verin.", "at"),
        refused("Hiçbir köşesinde kot olmayan çizgi", {"uid": uid(6), "at": 0, "reference": ESIK, "difference": 0.1},
                "elevation_missing",
                "0. köşenin kotu yok; fark bulunamıyor. Kotlu bir köşe seçin ya da önce Kot ver ile kot verin.", "at"),
        refused("Yazı bir yol değil", {"uid": uid(5), "at": 0, "reference": ESIK, "grade": 0.01}, "not_a_path",
                "Nesne çizgi, çoklu çizgi ya da alan değil; referanstan kot verilemez. Noktaya kotu Kot ver yazar.", "uid"),
        refused("Nokta bir yol değil", {"uid": uid(8), "at": 0, "reference": ESIK, "grade": 0.01}, "not_a_path",
                "Nesne çizgi, çoklu çizgi ya da alan değil; referanstan kot verilemez. Noktaya kotu Kot ver yazar.", "uid"),
        refused("Kilitli katman", {"uid": uid(4), "at": 0, "reference": ESIK, "grade": 0.01}, "layer_locked",
                "“Kilitli” katmanı kilitli: nesnenin kotları değişemez. Kilidi açın.", "uid"),
        refused("Çizimde olmayan nesne", {"uid": uid(99), "at": 0, "reference": ESIK, "grade": 0.01}, "entity_not_found",
                f"“{uid(99)}” kimlikli nesne çizimde yok.", "uid"),
        refused("Kimlik UUID değil", {"uid": "x", "at": 0, "reference": ESIK, "grade": 0.01}, "invalid_uid",
                "“x” geçerli bir nesne kimliği değil.", "uid"),
        refused("Sürüm yazısı geçersiz", {"uid": uid(1), "at": 0, "reference": ESIK, "grade": 0.01,
                                          "expectedRevision": "abc"},
                "invalid_revision", "Beklenen sürüm “abc” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. "
                "Sürümü belgeden ya da komutun planından alın.", "expectedRevision"),
        # Çok parçalı nesne (ADR 4980).
        ok("Alan: ikinci parçanın köşesi eğimle; bütün kotlar aynı farkla", 3, 2, YAZILAN, grade=0.02, rg=(1, 0)),
        ok("Alan: ilk parçanın deliğinin köşesi, kot farkıyla", 3, 1, YAZILAN, difference=-0.5, rg=(0, 1)),
        ok("Çoklu çizginin ikinci parçasının köşesi; ilk parça aynı farkla", 7, 2, ESIK, grade=0.01, rg=(1, 0)),
        refused("Olmayan parça", {"uid": uid(3), "at": 0, "part": 2, "reference": YAZILAN, "grade": 0.01},
                "wrong_place", "Nesnenin 2 parçası var (0'dan): 2. parça yok.", "part"),
        refused("Olmayan delik", {"uid": uid(3), "at": 0, "ring": 2, "reference": YAZILAN, "grade": 0.01},
                "wrong_place", "0. parçanın 1 deliği var: 2. halka yok (0 dış halka, 1'den delikler).", "ring"),
        refused("Deliğin köşe sayısının dışında", {"uid": uid(3), "at": 3, "part": 1, "ring": 1, "reference": YAZILAN,
                                                   "grade": 0.01},
                "wrong_place", "3. köşe yok: 1. parçanın 1. halkasının 3 köşesi var, sıra 0'dan başlar.", "at"),
        refused("Kotsuz delikteki köşe", {"uid": uid(3), "at": 0, "part": 1, "ring": 1, "reference": YAZILAN,
                                          "grade": 0.01}, "elevation_missing",
                "0. köşenin kotu yok; fark bulunamıyor. Kotlu bir köşe seçin ya da önce Kot ver ile kot verin.", "at"),
        {"name": "Beklenen sürüm eski: çatışma", "steps": [{"op": "execute", "input": {
            "uid": uid(1), "at": 0, "reference": ESIK, "grade": 0.01, "expectedRevision": "7"}, "result": {
            "status": "conflict", "error": {"code": "revision_conflict", "message":
                                            "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu "
                                            "çizimin şimdiki hâline göre yeniden hazırlayın.", "path": "expectedRevision",
                                            "revision": "$current"}}, "expect": SAME}]},
    ]


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.entities.raise", "commandVersion": 1,
           "title": "Referanstan kot ver", "note": "ADR 3780, 4980. Bağımsız başvuru scripts/fixtures/raise_cases.py: "
           "referanstan yatay uzaklık, köşenin yeni kotu ve öbür kotların kayması elle; KentOS kodu yok.",
           "setup": setup(), "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        ok_ = OUT.exists() and OUT.read_text("utf-8") == text
        print("Referanstan kot ver ortak durumları güncel." if ok_ else f"{OUT.relative_to(ROOT)} güncel değil.")
        return 0 if ok_ else 1
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
