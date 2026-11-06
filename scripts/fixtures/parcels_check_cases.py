#!/usr/bin/env python3
"""The shared cases of `cad.parcels.check` (Parsel alan denetimi; plan/map-cad.md MAP-08, docs/adr/1400), without
KentOS code.

    python3 scripts/fixtures/parcels_check_cases.py           # writes fixtures/commands/v1/cad.parcels.check.json
    python3 scripts/fixtures/parcels_check_cases.py --check   # writes nothing; compares

The rules, by hand:
- The geometric area: the outer ring's shoelace in rationals of the floats (at map coordinates a float shoelace loses
  digits), each arc edge's segment added with its bulge's sign (θ = 4·atan|b|, r = c / (2·sin(θ/2)), segment
  r²/2·(θ − sin θ)), the holes' areas taken off.
- The tolerance: Tapu Planlarında Yanılma Sınırının Belirlenmesi Hakkında Yönetmelik (RG 27.09.2022 / 31966),
  Article 8(2)'s table for archive-approved geometries, increasing by slices 0–10 (0.05), 11–100 (0.02), 101–500
  (0.01), 501–1000 (0.005), 1001–5000 (0.004), 5001–25000 (0.003), over 25000 (0.0015); a fraction between two slices
  by the lower one.
- The recorded area: the attribute trimmed; a plain decimal (digits, at most one point or comma, digits) is read;
  other text is invalidRecord, none or empty noRecord; |area − recorded| ≤ tolerance is within, else outside.
- The identity: İl/İlçe/Mahalle/Ada/Parsel; a parcel without Ada or Parsel has none; one held by two is a duplicate.
"""

import copy
import json
import math
import sys
from fractions import Fraction
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.parcels.check.json"
X0, Y0 = 487000.0, 4420000.0
FIELD = "Tapu alanı (m²)"
RULE = "tkgm-2022-m8-arsiv"


def uid(n):
    return f"0192f5c0-1400-7000-8000-{n:012x}"


def layer(id_, name):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def P(x, y):
    return {"x": X0 + x, "y": Y0 + y}


def rect(x0, y0, x1, y1):
    return [P(x0, y0), P(x1, y0), P(x1, y1), P(x0, y1)]


PLACE = {"İl": "Ankara", "İlçe": "Çankaya", "Mahalle": "Merkez"}


def parcel(n, pts, ada, number, recorded=None, layer_="parsel", bulges=None, holes=None, place=PLACE):
    attrs = dict(place)
    if ada is not None:
        attrs["Ada"] = ada
    if number is not None:
        attrs["Parsel"] = number
    if recorded is not None:
        attrs[FIELD] = recorded
    e = {"kind": "polygon", "id": n, "layerId": layer_, "attrs": attrs, "pts": pts}
    if bulges:
        e["bulges"] = bulges
    if holes:
        e["holes"] = [{"pts": h} for h in holes]
    return e


ENTS = [
    parcel(1, rect(0, 0, 20, 15), "101", "1", "301,2"),                     # 300 m², within ±4.3
    parcel(2, rect(30, 0, 60, 20), "101", "2", "590"),                      # 600 m², 10 over ±6.8: outside
    parcel(3, rect(100, 0, 120, 20), "101", "3", bulges=[0.0, 0.0, 0.25, 0.0]),  # an arc edge; no record
    parcel(4, rect(0, 30, 10, 40), "102", "1", "tapuda yok"),               # the same number in another Ada
    parcel(5, rect(70, 0, 78, 1.25), "101", "2", " 10.4 "),                 # Ada 101 Parsel 2 again; 10 m², ±0.5
    parcel(6, rect(0, 50, 40, 90), None, "9", "1495,75", holes=[rect(10, 60, 20, 70)]),  # no Ada; 1500 m²
    parcel(7, rect(200, 0, 300, 300), "103", "1", "30.010", layer_="cizim"),  # another layer: only when given
    {"kind": "line", "id": 8, "layerId": "parsel", "attrs": {}, "a": P(0, -5), "b": P(50, -5)},
    {"kind": "text", "id": 9, "layerId": "parsel", "attrs": {}, "text": "Ada 101", "p": P(0, -10), "height": 2.0,
     "rotation": 0.0},
    parcel(10, rect(130, 0, 140, 10.03), "101", "4", "1.250,50"),           # a thousands separator: not read
    # Off the parcel layer, given by id: the slices' edges and a fraction between two slices (by the lower one).
    parcel(11, rect(400, 0, 500, 50), "201", "1", "5003", layer_="cizim"),      # 5000 m², ±24.8: within
    parcel(12, rect(400, 100, 500, 350), "201", "2", "24900", layer_="cizim"),  # 25000 m², ±84.8: outside by 100
    parcel(13, rect(400, 400, 410, 410.075), "201", "3", "98,44", layer_="cizim"),  # 100.75 m², 2.31 off: within ±2.315 (11–100), not ±2.3075
    parcel(14, rect(400, 500, 402, 505), "201", "4", "10,6", layer_="cizim"),    # 10 m², ±0.5: outside by 0.6
    parcel(15, rect(400, 600, 700, 900), "201", "5", "90000", layer_="cizim"),   # 90000 m², ±182.3: within
    parcel(16, rect(400, 1000, 410, 1001.05), "201", "6", "10.0", layer_="cizim"),  # 10.5 m², ±0.525 (0–10)
]


def setup():
    return {"format": "kentos.document", "version": 2, "name": "Parsel alan denetimi",
            "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                         "plotScale": 1000.0, "workspace": "cad", "cadWork": "map"},
            "origin": {"x": X0, "y": Y0},
            "layers": [layer("0", "0"), layer("parsel", "Parsel sınırı"), layer("cizim", "Çizim")],
            "activeLayer": "parsel", "entities": copy.deepcopy(ENTS), "uids": [uid(i + 1) for i in range(len(ENTS))],
            "styles": {"items": [], "categories": []}}


def ring_area(pts, bulges=None):
    """Signed: counter-clockwise positive."""
    q = [(Fraction(p["x"]), Fraction(p["y"])) for p in pts]
    n = len(q)
    s = sum(q[i][0] * q[(i + 1) % n][1] - q[(i + 1) % n][0] * q[i][1] for i in range(n)) / 2
    area = float(s)
    for i, b in enumerate(bulges or []):
        if not b:
            continue
        a, c = pts[i], pts[(i + 1) % n]
        chord = math.hypot(c["x"] - a["x"], c["y"] - a["y"])
        t = 4.0 * math.atan(abs(b))
        r = chord / (2.0 * math.sin(t / 2.0))
        area += math.copysign(r * r / 2.0 * (t - math.sin(t)), b)
    return area


def area_of(e):
    return abs(ring_area(e["pts"], e.get("bulges"))) - sum(abs(ring_area(h["pts"], h.get("bulges")))
                                                           for h in e.get("holes", []))


SLICES = [(0, 0, 0.0, 0.05), (11, 10, 0.5, 0.02), (101, 100, 2.3, 0.01), (501, 500, 6.3, 0.005),
          (1001, 1000, 8.8, 0.004), (5001, 5000, 24.8, 0.003), (25001, 25000, 84.8, 0.0015)]


def tolerance(a):
    lower, start, base, rate = [s for s in SLICES if a >= s[0]][-1]
    return base + (a - start) * rate


def read(text):
    whole, sep, frac = text.replace(",", ".").partition(".")
    if not whole.isdigit() or not whole.isascii() or (sep and not (frac.isdigit() and frac.isascii())):
        return None
    if text.count(".") + text.count(",") > 1:
        return None
    return float(text.replace(",", "."))


def check(e, field=FIELD):
    a = e["attrs"]
    g = lambda k: a.get(k, "").strip()  # noqa: E731
    area = area_of(e)
    text = g(field) or None
    rec = read(text) if text else None
    diff = area - rec if rec is not None else None
    tol = tolerance(area)
    status = "noRecord" if text is None else "invalidRecord" if rec is None else \
        "within" if abs(diff) <= tol else "outside"
    out = {"uid": uid(e["id"]), "il": g("İl"), "ilce": g("İlçe"), "mahalle": g("Mahalle"), "ada": g("Ada"),
           "parsel": g("Parsel"), "area": area}
    if text is not None:
        out["recordedText"] = text
    if rec is not None:
        out["recorded"] = rec
        out["difference"] = diff
    out["tolerance"] = tol
    out["status"] = status
    if not g("Ada") or not g("Parsel"):
        out["noIdentity"] = True
    return out


def report(ids=None, field=FIELD):
    chosen = [ENTS[i - 1] for i in ids] if ids else [e for e in ENTS if e["layerId"] == "parsel" and
                                                       e["kind"] == "polygon"]
    ps = [check(e, field) for e in chosen]
    key = lambda p: "/".join([p["il"], p["ilce"], p["mahalle"], p["ada"], p["parsel"]])  # noqa: E731
    held = {}
    for p in ps:
        if not p.get("noIdentity"):
            held[key(p)] = held.get(key(p), 0) + 1
    dups = []
    for p in ps:
        if not p.get("noIdentity") and held[key(p)] > 1:
            p["duplicate"] = True
            if key(p) not in dups:
                dups.append(key(p))
    count = lambda s: sum(1 for p in ps if p["status"] == s)  # noqa: E731
    return {"rule": RULE, "field": field, "parcels": ps, "within": count("within"), "outside": count("outside"),
            "noRecord": count("noRecord"), "invalidRecord": count("invalidRecord"), "duplicates": dups,
            "noIdentity": sum(1 for p in ps if p.get("noIdentity"))}


SAME = {"revision": "same", "canUndo": False}


def ok(name, op="plan", ids=None, field=None):
    out = report(ids)
    input_ = {}
    if ids:
        input_["uids"] = [uid(i) for i in ids]
    if field:
        input_["field"] = field
        out = report(ids, field=field.strip() or FIELD)
    return {"name": name, "steps": [{"op": op, "input": input_, "result": {
        "status": "completed", "output": None if op == "validate" else out, "warnings": []}, "expect": SAME,
        "within": 1e-9}]}


def refused(name, input_, code, message, path_):
    return {"name": name, "steps": [{"op": "execute", "input": input_, "result": {"status": "failed", "error": {
        "code": code, "message": message, "path": path_}}, "expect": SAME}]}


def cases():
    whole = report()
    assert [p["status"] for p in whole["parcels"]] == ["within", "outside", "noRecord", "invalidRecord", "within",
                                                       "within", "invalidRecord"], whole
    out = [
        ok("plan: parsel katmanının bütün alanları; içinde, dışında, kaydı yok, okunamayan; yinelenen kimlik"),
        ok("uygula: salt okunur, çizim değişmez", op="execute"),
        ok("doğrulama", op="validate"),
        ok("verilen parseller, başka katmandaki alan da", ids=[7, 1]),
        ok("aynı numara başka adada yinelenmiş sayılmaz", ids=[1, 4]),
        ok("yaylı kenarlı parsel: alan yay parçasıyla", ids=[3]),
        ok("delikli parsel: delik alanı düşülür; adasız parsel kimliksiz", ids=[6]),
        ok("başka öznitelikten kayıtlı alan", field="Ada"),
        ok("boş öznitelik adı: Tapu alanı (m²)", field="   "),
        ok("5000 m²: dilim sınırı ±24,8", ids=[11]),
        ok("25000 m²: ±84,8, 100 m² fark dışında", ids=[12]),
        ok("100,75 m²: küsurat alt dilimden (11–100) ±2,315; 2,31 fark içinde", ids=[13]),
        ok("10 m²: ±0,5, 0,6 fark dışında", ids=[14]),
        ok("90000 m²: son dilim ±182,3", ids=[15]),
        ok("10,5 m²: küsurat ilk dilimden ±0,525", ids=[16]),
        ok("kayıt yazısı ondalık noktalı; binlik ayırıcı okunmaz", ids=[7, 10]),
        ok("yinelenen kimlik: aynı ada, aynı parsel", ids=[2, 5]),
        ok("doğrulama: verilen parseller", op="validate", ids=[1, 3]),
        refused("çizgi bir parsel değil", {"uids": [uid(1), uid(8)]}, "not_a_parcel",
                f"“{uid(8)}” bir alan değil; parsel alan denetimi yalnız alanları denetler.", "uids[1]"),
        refused("kimlik UUID değil", {"uids": ["x"]}, "invalid_uid", "“x” geçerli bir nesne kimliği değil.", "uids[0]"),
        refused("çizimde olmayan nesne", {"uids": [uid(99)]}, "entity_not_found",
                f"“{uid(99)}” kimlikli nesne çizimde yok.", "uids[0]"),
    ]
    return out


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.parcels.check", "commandVersion": 1,
           "title": "Parsel alan denetimi",
           "note": "ADR 1400. Bağımsız başvuru scripts/fixtures/parcels_check_cases.py: alan rasyonel shoelace ve yay "
                   "parçası, yanılma sınırı 2022 Yönetmeliği Madde 8(2) tablosundan elle; KentOS kodu yok.",
           "setup": setup(), "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        ok_ = OUT.exists() and OUT.read_text("utf-8") == text
        print("Parsel alan denetimi ortak durumları güncel." if ok_ else f"{OUT.relative_to(ROOT)} güncel değil.")
        return 0 if ok_ else 1
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
