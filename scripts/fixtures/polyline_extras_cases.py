#!/usr/bin/env python3
"""cad.polyline.create's closed and wide polylines (docs/adr/0258), from the ADR's rules, without KentOS code.

The cases whose names start with `PREFIX` in fixtures/commands/v1/cad.polyline.create.json are this script's; the
others are written by hand. `--check` compares, otherwise the file's own cases are written back with these after them.

The rules, as ADR 0258 §3 gives them:
- the checks in order: at least 2 points; finite coordinates (point by point, x before y); one bulge per edge when
  bulges are given (one fewer than the points, as many on a closed line); finite bulges; one width pair per edge when
  widths are given (as bulges); each edge's widths finite, then not below 0; the line weight; the revision; the layer;
- a closed line's bulges and widths are stored as given; an open line's get one more, 0 and [0, 0], for the closing
  edge it does not have; widths all 0 are none; `closed: false` is an open line, stored without the field.
"""

import argparse
import copy
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CASES = ROOT / "fixtures/commands/v1/cad.polyline.create.json"
PREFIX = "Kapalı ve genişlikli: "

X0, Y0 = 487010, 4420010
SQUARE = [{"x": X0, "y": Y0}, {"x": X0 + 20, "y": Y0}, {"x": X0 + 20, "y": Y0 + 12.5}, {"x": X0, "y": Y0 + 12.5}]
THREE = SQUARE[:3]


def refusal(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def check(inp):
    """The first refusal of the command's checks, or None (the checks this file's cases reach)."""
    pts = inp["pts"]
    n = len(pts)
    if n < 2:
        return refusal("too_few_points", f"Çoklu çizginin en az 2 noktası olmalı; {n} nokta verildi. Eksik noktaları ekleyin.", "pts")
    closed = inp.get("closed", False)
    edges = n if closed else n - 1
    if "bulges" in inp and len(inp["bulges"]) != edges:
        return refusal(
            "bulge_count",
            f"Yay değerleri kenar sayısı kadar olmalı, her kenara bir değer: {n} nokta, {edges} kenar, {len(inp['bulges'])} yay değeri verildi. Eksik ya da fazla değerleri düzeltin.",
            "bulges",
        )
    if "widths" in inp:
        widths = inp["widths"]
        if len(widths) != edges:
            return refusal(
                "invalid_polyline",
                f"Genişlikler kenar sayısı kadar olmalı, her kenara bir başlangıç ve bir bitiş genişliği: {n} nokta, {edges} kenar, {len(widths)} genişlik verildi.",
                "widths",
            )
        for i, (a, b) in enumerate(widths):
            if not (math.isfinite(a) and math.isfinite(b)):
                return refusal(
                    "not_finite",
                    f"{i + 1}. kenarın genişliği sonlu bir sayı değil (NaN ya da sonsuz). Genişliği çizim biriminde, sıfır ya da büyük verin.",
                    f"widths[{i}]",
                )
            if a < 0 or b < 0:
                return refusal(
                    "invalid_polyline",
                    f"{i + 1}. kenarın genişliği eksi olamaz: genişlik sıfır ya da büyük olmalı (çizim biriminde).",
                    f"widths[{i}]",
                )
    if "lineWeight" in inp and not (0 <= inp["lineWeight"] <= 100):
        return refusal(
            "invalid_line_weight",
            "Çizgi kalınlığı 0 ile 100 mm arasında bir sayı olmalı (0 en ince çizgidir). Bir kalınlık ya da “Katmana göre” seçin.",
            "lineWeight",
        )
    if inp["layerId"] == "kilitli":
        return refusal(
            "layer_locked",
            "“Kilitli katman” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.",
            "layerId",
        )
    return None


def stored(inp, id_):
    """The polyline the document keeps of `inp`."""
    closed = inp.get("closed", False)
    e = {"kind": "polyline", "id": id_, "layerId": inp["layerId"], "attrs": {}, "pts": inp["pts"]}
    if "lineWeight" in inp:
        e["lineWeight"] = inp["lineWeight"]
    if "bulges" in inp:
        e["bulges"] = list(inp["bulges"]) + ([] if closed else [0])
    if closed:
        e["closed"] = True
    widths = inp.get("widths")
    if widths is not None and any(a != 0 or b != 0 for a, b in widths):
        e["widths"] = [list(w) for w in widths] + ([] if closed else [[0, 0]])
    return e


def written(inp, id_=10, ids=(4, 9), **expect):
    return {
        "op": "execute",
        "input": inp,
        "result": {"status": "completed", "output": {"uid": "$uid", "id": id_, "revision": "$current"}, "warnings": []},
        "expect": {"ids": [*ids, id_], "entities": {str(id_): stored(inp, id_)}, "revision": "changed", **expect},
    }


def refused(inp, non_finite=None, **expect):
    result = check(inp if non_finite is None else with_non_finite(inp, non_finite))
    assert result is not None, inp
    step = {"op": "execute", "input": inp}
    if non_finite is not None:
        step["nonFinite"] = non_finite
    step["result"] = result
    step["expect"] = {"ids": [4, 9], "revision": "same", **expect}
    return step


def with_non_finite(inp, table):
    """`inp` with the numbers `table` names (JSON holds none) put in: `widths[i][k]`."""
    out = copy.deepcopy(inp)
    for path, word in table.items():
        value = {"NaN": math.nan, "Infinity": math.inf, "-Infinity": -math.inf}[word]
        i, k = path[len("widths["):-1].split("][")
        out["widths"][int(i)][int(k)] = value
    return out


def case(name, *steps):
    return {"name": PREFIX + name, "steps": list(steps)}


def cases():
    yapi = lambda **f: {"layerId": "yapi", **f}  # noqa: E731
    out = [
        case(
            "kapalı çizgi yazılır; son noktası ilkine bir kenarla bağlanır, alan değildir",
            written(yapi(pts=SQUARE, closed=True), canUndo=True, dirty=True),
        ),
        case(
            "kapalı çizginin bükümleri kenar başına: kapanan kenarınki sonuncusu, nokta sayısı kadar; olduğu gibi saklanır",
            written(yapi(pts=THREE, bulges=[0, 0, 0.5], closed=True)),
        ),
        case(
            "iki noktalı kapalı çizgi yazılır: iki yaylı kenar bir halkadır",
            written(yapi(pts=SQUARE[:2], bulges=[1, 1], closed=True)),
        ),
        case(
            "kapalı çizgide açık çizginin büküm sayısı eksiktir: bulge_count kapanan kenarı da sayar",
            refused(yapi(pts=THREE, bulges=[0, 0.5], closed=True), canUndo=False),
        ),
        case(
            "closed false açık çizgidir; alan yazılmaz",
            written(yapi(pts=THREE, closed=False)),
        ),
        case(
            "kapalı çizgi tek adımda geri alınır ve yinelenir",
            written(yapi(pts=SQUARE, closed=True)),
            {"op": "undo", "returns": "Ekle", "expect": {"ids": [4, 9], "canRedo": True}},
            {"op": "redo", "returns": "Ekle", "expect": {"ids": [4, 9, 10], "entities": {"10": stored(yapi(pts=SQUARE, closed=True), 10)}}},
        ),
        case(
            "sabit genişlik: her kenara aynı çift; belge açık çizginin sonuna [0, 0] ekler",
            written(yapi(pts=THREE, widths=[[0.5, 0.5], [0.5, 0.5]])),
        ),
        case(
            "kenarın başlangıç ve bitiş genişliği ayrıdır: daralan ve genişleyen kenar",
            written(yapi(pts=THREE, widths=[[0, 1.25], [1.25, 0.5]])),
        ),
        case(
            "kapalı çizginin genişliği: kapanan kenar da, olduğu gibi saklanır",
            written(yapi(pts=SQUARE, closed=True, widths=[[0.3, 0.3], [0.3, 0.3], [0.3, 0.3], [0.3, 0.6]])),
        ),
        case(
            "yaylı kenar da genişlikli olur; büküm ve genişlik kenar kenar",
            written(yapi(pts=THREE, bulges=[0.4, 0], widths=[[1, 1], [0, 0]])),
        ),
        case(
            "hepsi sıfır genişlik yok demektir: alan yazılmaz",
            written(yapi(pts=THREE, widths=[[0, 0], [0, 0]])),
        ),
        case(
            "bazı kenarlar genişliksiz kalabilir: o kenar çizginin kendisidir",
            written(yapi(pts=SQUARE, widths=[[0, 0], [2, 2], [0, 0]])),
        ),
        case(
            "genişlik çizimin uzunluğudur, kâğıdın kalınlığı değil: büyük genişlik de geçerli, kalınlıkla birlikte yazılır",
            written(yapi(pts=THREE, widths=[[250, 250], [250, 250]], lineWeight=0.5)),
        ),
        case(
            "genişlikler kenar sayısı kadar olmalı: açık çizgide noktadan bir eksik",
            refused(yapi(pts=THREE, widths=[[1, 1], [1, 1], [0, 0]]), canUndo=False),
        ),
        case(
            "kapalı çizgide genişlik nokta sayısı kadar: kapanan kenarınki eksikse reddedilir",
            refused(yapi(pts=SQUARE, closed=True, widths=[[1, 1], [1, 1], [1, 1]])),
        ),
        case(
            "boş genişlik listesi de bir listedir: kenar sayısı kadar değil",
            refused(yapi(pts=THREE, widths=[])),
        ),
        case(
            "eksi genişlik yazılmaz; yol ve ileti kenarı söyler",
            refused(yapi(pts=THREE, widths=[[0.5, 0.5], [0.5, -0.25]])),
        ),
        case(
            "NaN ya da sonsuz genişlik not_finite: önce sonluluk, sonra işaret",
            refused(yapi(pts=THREE, widths=[[0.5, 0.5], [0.5, 0.5]]), non_finite={"widths[1][0]": "NaN"}),
            refused(yapi(pts=THREE, widths=[[-1, 0.5], [0.5, 0.5]]), non_finite={"widths[0][1]": "Infinity"}),
        ),
        case(
            "denetim sırası: büküm sayısı genişlikten, genişlik kalınlıktan önce",
            refused(yapi(pts=THREE, bulges=[0], widths=[[1, 1]], lineWeight=200)),
            refused(yapi(pts=THREE, widths=[[1, 1]], lineWeight=200)),
        ),
        case(
            "kilitli katmana kapalı ve genişlikli çizgi de yazılmaz",
            refused({"layerId": "kilitli", "pts": SQUARE, "closed": True, "widths": [[1, 1]] * 4}),
        ),
        case(
            "doğrulama hiçbir şey yazmaz; genişliğin reddini de verir",
            {"op": "validate", "input": yapi(pts=SQUARE, closed=True, widths=[[1, 1]] * 4), "result": {"status": "completed", "output": None, "warnings": []}, "expect": {"ids": [4, 9], "revision": "same"}},
            {"op": "validate", "input": yapi(pts=THREE, widths=[[1, 1]]), "result": check(yapi(pts=THREE, widths=[[1, 1]])), "expect": {"ids": [4, 9], "revision": "same"}},
        ),
        case(
            "plan kapalı ve genişlikli çizgiyi saklanacağı biçimde gösterir; yazma planı yazar",
            {
                "op": "plan",
                "input": yapi(pts=THREE, bulges=[0, 0.25], widths=[[0.2, 0.4], [0.4, 0.2]]),
                "result": {"status": "completed", "output": {"entity": stored(yapi(pts=THREE, bulges=[0, 0.25], widths=[[0.2, 0.4], [0.4, 0.2]]), 0), "revision": "$current"}, "warnings": []},
                "expect": {"ids": [4, 9], "revision": "same"},
            },
            written(yapi(pts=THREE, bulges=[0, 0.25], widths=[[0.2, 0.4], [0.4, 0.2]], expectedRevision="$current")),
        ),
    ]
    return out


def inline(v):
    """A value on one line, in the file's hand-written style: `{ "k": v }`, `[a, b]`."""
    if isinstance(v, dict):
        return "{ " + ", ".join(f"{json.dumps(k, ensure_ascii=False)}: {inline(x)}" for k, x in v.items()) + " }" if v else "{}"
    if isinstance(v, list):
        return "[" + ", ".join(inline(x) for x in v) + "]"
    return json.dumps(v, ensure_ascii=False)


def block(c):
    """A case as the file writes one: its name, then a step a line."""
    steps = ",\n".join(f"        {inline(st)}" for st in c["steps"])
    return f'    {{\n      "name": {json.dumps(c["name"], ensure_ascii=False)},\n      "steps": [\n{steps}\n      ]\n    }}'


# The file's own cases end here; this script's follow them.
TAIL = "\n  ]\n}\n"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    text = CASES.read_text()
    data = json.loads(text)
    mine = cases()
    have = [c for c in data["cases"] if c["name"].startswith(PREFIX)]
    if args.check:
        if have != json.loads(json.dumps(mine)):
            names = {c["name"] for c in have} ^ {c["name"] for c in mine}
            print(f"cad.polyline.create'in kapalı ve genişlikli durumları farklı ({len(have)} var, {len(mine)} olmalı; {sorted(names)[:3]})")
            return 1
        print(f"cad.polyline.create: {len(mine)} kapalı ve genişlikli durum başvuruyla aynı.")
        return 0
    # The hand-written part: up to this script's first case, or the end of the list.
    first = text.find('\n    {\n      "name": ' + json.dumps(PREFIX, ensure_ascii=False)[:-1])
    head = text[: first if first >= 0 else len(text) - len(TAIL)]
    head = head.rstrip().rstrip(",")
    assert text.endswith(TAIL), "dosyanın sonu beklenen biçimde değil"
    CASES.write_text(head + ",\n" + ",\n".join(block(c) for c in mine) + TAIL)
    assert json.loads(CASES.read_text())["cases"][-len(mine):] == json.loads(json.dumps(mine))
    print(f"{len(mine)} durum yazıldı: {CASES.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
