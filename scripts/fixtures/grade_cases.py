#!/usr/bin/env python3
"""The shared cases of `cad.entities.grade` (Eğimle kot ver; plan/map-cad.md MAP-06, docs/adr/1290), without KentOS
code.

    python3 scripts/fixtures/grade_cases.py           # writes fixtures/commands/v1/cad.entities.grade.json
    python3 scripts/fixtures/grade_cases.py --check   # writes nothing; compares

The rule, by hand: the horizontal length along the path from `from` to each vertex up to `to` (a straight edge its
chord, an arc edge with bulge b its arc: θ = 4·atan|b|, r = chord / (2·sin(θ/2)), length r·θ); the grade g = (end −
start) / L or the one given; each vertex start + g·s, the last the end as given (or start + g·L); the start is the one
given, else the vertex's own elevation. Vertices outside the range keep theirs.
"""

import copy
import json
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.entities.grade.json"
X0, Y0 = 500000.0, 4400000.0


def uid(n):
    return f"0192f5c0-1290-7000-8000-{n:012x}"


def layer(id_, name, locked=False):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def P(x, y):
    return {"x": X0 + x, "y": Y0 + y}


POLY = {"kind": "polyline", "id": 1, "layerId": "tasarim", "attrs": {}, "pts": [P(0, 0), P(30, 0), P(30, 40), P(80, 40),
                                                                                 P(100, 40)],
        "bulges": [0.0, 0.0, 0.5, 0.0, 0.0], "zs": [850.0, None, None, 851.25, None]}
LINE = {"kind": "line", "id": 2, "layerId": "tasarim", "attrs": {}, "a": P(0, 100), "b": P(60, 180), "za": 900.0}
AREA = {"kind": "polygon", "id": 3, "layerId": "tasarim", "attrs": {}, "pts": [P(200, 0), P(240, 0), P(240, 30),
                                                                               P(200, 30)]}
LOCKED = {"kind": "polyline", "id": 4, "layerId": "kilitli", "attrs": {}, "pts": [P(0, 300), P(10, 300)],
          "zs": [10.0, None]}
TEXT = {"kind": "text", "id": 5, "layerId": "tasarim", "attrs": {}, "text": "Kot", "p": P(0, 400), "height": 2.0, "rotation": 0.0}
ENTS = [POLY, LINE, AREA, LOCKED, TEXT]


def setup():
    return {"format": "kentos.document", "version": 2, "name": "Eğimle kot",
            "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                         "plotScale": 1000.0, "workspace": "cad", "cadWork": "map"},
            "origin": {"x": X0, "y": Y0}, "layers": [layer("0", "0"), layer("tasarim", "Tasarım"),
                                                     layer("kilitli", "Kilitli", locked=True)],
            "activeLayer": "tasarim", "entities": copy.deepcopy(ENTS), "uids": [uid(i + 1) for i in range(len(ENTS))],
            "styles": {"items": [], "categories": []}}


def edge(a, b, bulge):
    c = math.hypot(b["x"] - a["x"], b["y"] - a["y"])
    if bulge == 0.0:
        return c
    t = 4.0 * math.atan(abs(bulge))
    return c / (2.0 * math.sin(t / 2.0)) * t


def path(e):
    if e["kind"] == "line":
        return [e["a"], e["b"]], [0.0], [e.get("za"), e.get("zb")]
    n = len(e["pts"])
    return e["pts"], e.get("bulges", [0.0] * n), e.get("zs", [None] * n)


def graded(e, frm, to, start=None, end=None, grade=None):
    pts, bulges, zs = path(e)
    s0 = start if start is not None else zs[frm]
    along = [0.0]
    for i in range(frm, to):
        along.append(along[-1] + edge(pts[i], pts[(i + 1) % len(pts)], bulges[i] if i < len(bulges) else 0.0))
    L = along[-1]
    g = (end - s0) / L if end is not None else grade
    out = [s0 + g * d for d in along]
    last = end if end is not None else out[-1]
    out[-1] = last
    return {"length": L, "start": s0, "end": last, "grade": g, "zs": out}


SAME = {"revision": "same", "canUndo": False}


def ok(name, slot, frm, to, op="execute", **kw):
    e = ENTS[slot - 1]
    g = graded(e, frm, to, **kw)
    input_ = {"uid": uid(slot), "from": frm, "to": to, **{k: v for k, v in kw.items() if v is not None}}
    out = dict(g, revision="$current")
    if op == "validate":
        result = {"status": "completed", "output": None, "warnings": []}
    else:
        result = {"status": "completed", "output": out, "warnings": []}
    if op != "execute":
        return {"name": name, "steps": [{"op": op, "input": input_, "result": result, "expect": SAME, "within": 1e-9}]}
    after = copy.deepcopy(e)
    if e["kind"] == "line":
        after["za"], after["zb"] = g["zs"][0], g["zs"][1]
        fields = {"za": after["za"], "zb": after["zb"]}
    else:
        zs = list(e.get("zs", [None] * len(e["pts"])))
        for k, v in enumerate(g["zs"]):
            zs[frm + k] = v
        fields = {"zs": zs}
    return {"name": name, "steps": [
        {"op": "execute", "input": input_, "result": result, "within": 1e-9,
         "expect": {"revision": "changed", "canUndo": True, "fields": {str(slot): fields}}},
        {"op": "undo", "returns": "Eğimle kot ver", "expect": {"fields": {str(slot): {k: e.get(k) for k in fields}}}}]}


def refused(name, input_, code, message, path_):
    return {"name": name, "steps": [{"op": "execute", "input": input_, "result": {"status": "failed", "error": {
        "code": code, "message": message, "path": path_}}, "expect": SAME}]}


def cases():
    return [
        ok("Eğimle: kendi kotundan %2, düz ve yaylı kenar boyunca; geri alma", 1, 0, 4, grade=0.02),
        ok("Bitiş kotuyla: eğim ondan çıkar", 1, 0, 3, end=852.0),
        ok("Ara köşeden başlar; aralık dışı köşeler kotlarını korur", 1, 1, 3, start=849.5, grade=-0.015),
        ok("Yaylı kenarın boyu yayı boyuncadır", 1, 2, 3, start=100.0, end=101.0),
        ok("Eksi eğim", 1, 0, 2, grade=-0.05),
        ok("Çizginin iki ucu", 2, 0, 1, grade=0.03),
        ok("Çizgi: bitiş kotuyla", 2, 0, 1, end=905.5),
        ok("Alanın dış halkası (kotsuz): başlangıç verilir", 3, 0, 3, start=120.0, grade=0.01),
        ok("Plan yazmaz", 1, 0, 4, op="plan", grade=0.02),
        ok("Doğrulama", 1, 0, 4, op="validate", grade=0.02),
        refused("Kotsuz köşeden başlangıç verilmeden", {"uid": uid(3), "from": 0, "to": 2, "grade": 0.01},
                "elevation_missing", "0. köşenin kotu yok; başlangıç kotunu (start) verin.", "start"),
        refused("Hem bitiş hem eğim", {"uid": uid(1), "from": 0, "to": 2, "end": 851.0, "grade": 0.01}, "invalid_input",
                "Bitiş kotunu (end) ya da eğimi (grade) verin, ikisini birden değil.", "end"),
        refused("Ne bitiş ne eğim", {"uid": uid(1), "from": 0, "to": 2}, "invalid_input",
                "Bitiş kotunu (end) ya da eğimi (grade) verin, ikisini birden değil.", "end"),
        refused("Ters aralık", {"uid": uid(1), "from": 3, "to": 1, "grade": 0.01}, "wrong_place",
                "Köşe aralığı 3–1 olamaz: nesnenin 5 köşesi var (0'dan), başlangıç bitişten önce olmalı.", "to"),
        refused("Köşe sayısının dışında", {"uid": uid(1), "from": 0, "to": 5, "grade": 0.01}, "wrong_place",
                "Köşe aralığı 0–5 olamaz: nesnenin 5 köşesi var (0'dan), başlangıç bitişten önce olmalı.", "to"),
        refused("Yazı bir yol değil", {"uid": uid(5), "from": 0, "to": 1, "grade": 0.01}, "not_a_path",
                "Nesne çizgi, çoklu çizgi ya da alan değil; kotu eğimle verilemez.", "uid"),
        refused("Kilitli katman", {"uid": uid(4), "from": 0, "to": 1, "grade": 0.01}, "layer_locked",
                "“Kilitli” katmanı kilitli: nesnenin kotları değişemez. Kilidi açın.", "uid"),
        refused("Çizimde olmayan nesne", {"uid": uid(99), "from": 0, "to": 1, "grade": 0.01}, "entity_not_found",
                f"“{uid(99)}” kimlikli nesne çizimde yok.", "uid"),
        refused("Kimlik UUID değil", {"uid": "x", "from": 0, "to": 1, "grade": 0.01}, "invalid_uid",
                "“x” geçerli bir nesne kimliği değil.", "uid"),
        refused("Sürüm yazısı geçersiz", {"uid": uid(1), "from": 0, "to": 1, "grade": 0.01, "expectedRevision": "abc"},
                "invalid_revision", "Beklenen sürüm “abc” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. "
                "Sürümü belgeden ya da komutun planından alın.", "expectedRevision"),
        {"name": "Beklenen sürüm eski: çatışma", "steps": [{"op": "execute", "input": {
            "uid": uid(1), "from": 0, "to": 1, "grade": 0.01, "expectedRevision": "7"}, "result": {
            "status": "conflict", "error": {"code": "revision_conflict", "message":
                                            "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu "
                                            "çizimin şimdiki hâline göre yeniden hazırlayın.", "path": "expectedRevision",
                                            "revision": "$current"}}, "expect": SAME}]},
    ]


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.entities.grade", "commandVersion": 1,
           "title": "Eğimle kot ver", "note": "ADR 1290. Bağımsız başvuru scripts/fixtures/grade_cases.py: yay boyu ve "
           "kotlar elle; KentOS kodu yok.", "setup": setup(), "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        ok_ = OUT.exists() and OUT.read_text("utf-8") == text
        print("Eğimle kot ver ortak durumları güncel." if ok_ else f"{OUT.relative_to(ROOT)} güncel değil.")
        return 0 if ok_ else 1
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
