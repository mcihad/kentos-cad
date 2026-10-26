#!/usr/bin/env python3
"""The three-way merge of an object two editors changed (docs/adr/0553 §4, 0648 §1), written from the ADRs without KentOS
code: against the copy both started from (the base), every top-level field of the object's JSON except `id` takes the side
that changed it; `attrs` are merged one attribute at a time by the same rule; a field (or an attribute, named
`attrs.<name>`) both changed to different values is a clash; both changed alike, it is that value. `id` is mine's. An
absent field is a value too (removing is a change). The clashes are listed in the order of the fields' names (an
attribute at `attrs`, by its name).

    python3 scripts/fixtures/entity_merge_cases.py           # writes fixtures/cloud/v1/merge.json
    python3 scripts/fixtures/entity_merge_cases.py --check   # compares
"""
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/cloud/v1/merge.json"
MISSING = object()


def pick(b, m, t):
    if m == b and t == b:
        return "same", m
    if t == b:
        return "mine", m
    if m == b:
        return "theirs", t
    if m == t:
        return "same", m
    return "clash", None


def merge(base, mine, theirs):
    out, from_mine, from_theirs, clashes = {}, [], [], []
    for key in sorted(set(base) | set(mine) | set(theirs)):
        if key == "id":
            if "id" in mine:
                out["id"] = mine["id"]
            continue
        if key == "attrs":
            ba, ma, ta = base.get("attrs", {}), mine.get("attrs", {}), theirs.get("attrs", {})
            merged = {}
            for name in sorted(set(ba) | set(ma) | set(ta)):
                side, v = pick(ba.get(name, MISSING), ma.get(name, MISSING), ta.get(name, MISSING))
                if side == "mine":
                    from_mine.append(f"attrs.{name}")
                elif side == "theirs":
                    from_theirs.append(f"attrs.{name}")
                elif side == "clash":
                    clashes.append(f"attrs.{name}")
                    v = ma.get(name, MISSING)
                if v is not MISSING:
                    merged[name] = v
            if merged or "attrs" in mine or "attrs" in theirs:
                out["attrs"] = merged
            continue
        side, v = pick(base.get(key, MISSING), mine.get(key, MISSING), theirs.get(key, MISSING))
        if side == "mine":
            from_mine.append(key)
        elif side == "theirs":
            from_theirs.append(key)
        elif side == "clash":
            clashes.append(key)
            v = mine.get(key, MISSING)
        if v is not MISSING:
            out[key] = v
    if clashes:
        return {"clashes": clashes}
    return {"merged": out, "fromMine": from_mine, "fromTheirs": from_theirs}


def point(x=487000.0, y=4420000.0, **extra):
    e = {"kind": "point", "id": 1, "layerId": "parsel", "attrs": {"ad": "A1", "ada": "101"}, "p": {"x": x, "y": y}}
    e.update(extra)
    return e


def polygon(pts=None, **extra):
    pts = pts or [[0, 0], [10, 0], [10, 10], [0, 10]]
    e = {"kind": "polygon", "id": 4, "layerId": "parsel", "attrs": {"parsel_no": "5", "malik": "Ali"},
         "pts": [{"x": 487000 + x, "y": 4420000 + y} for x, y in pts]}
    e.update(extra)
    return e


def line(**extra):
    e = {"kind": "line", "id": 2, "layerId": "yol", "attrs": {}, "a": {"x": 487000.0, "y": 4420000.0},
         "b": {"x": 487050.0, "y": 4420000.0}}
    e.update(extra)
    return e


def with_attrs(e, **attrs):
    e = copy.deepcopy(e)
    for k, v in attrs.items():
        if v is None:
            e["attrs"].pop(k, None)
        else:
            e["attrs"][k] = v
    return e


def cases():
    out = []

    def add(name, base, mine, theirs):
        out.append({"name": name, "base": base, "mine": mine, "theirs": theirs, **merge(base, mine, theirs)})

    b = point()
    add("ikisi de değiştirmedi: aynı", b, b, b)
    add("yalnız benim taşımam", b, point(487005.0), b)
    add("yalnız onların taşıması", b, b, point(487005.0))
    add("ben taşıdım, onlar öznitelik değiştirdi: ikisi de", b, point(487005.0), with_attrs(b, ada="102"))
    add("aynı özniteliği farklı: çatışma", b, with_attrs(b, ada="102"), with_attrs(b, ada="103"))
    add("aynı özniteliği aynı değere: çatışma değil", b, with_attrs(b, ada="102"), with_attrs(b, ada="102"))
    add("farklı öznitelikler: ikisi de", b, with_attrs(b, ad="A2"), with_attrs(b, ada="102"))
    add("ben öznitelik ekledim, onlar sildi başkasını", b, with_attrs(b, kod="K"), with_attrs(b, ada=None))
    add("ben sildim, onlar değiştirdi: çatışma", b, with_attrs(b, ada=None), with_attrs(b, ada="102"))
    add("ikisi de aynı özniteliği sildi", b, with_attrs(b, ada=None), with_attrs(b, ada=None))
    add("ikisi de farklı yere taşıdı: geometri çatışması", b, point(487005.0), point(487006.0))
    add("ikisi de aynı yere taşıdı", b, point(487005.0), point(487005.0))
    add("onlar etiket verdi, ben renk", b, point(color="#FF0000"), point(label="kapı"))
    add("ikisi farklı etiket: çatışma", b, point(label="a"), point(label="b"))
    add("onlar katmanını değiştirdi, ben taşıdım", b, point(487005.0), point(layerId="nokta"))
    add("ikisi farklı katmana: çatışma", b, point(layerId="a"), point(layerId="b"))
    add("benim yuvamın kimliği kalır", b, point(487005.0, id=9), point(label="x"))
    add("kot yalnız onlarda", b, point(487005.0), point(z=1024.5))
    add("geometri ve iki öznitelik çatışması, sırası adlarına göre", b,
        with_attrs(point(487005.0), ad="B", ada="1"), with_attrs(point(487006.0), ad="C", ada="2"))
    pb = polygon()
    add("alan: ben köşe taşıdım, onlar malik değiştirdi", pb, polygon([[0, 0], [12, 0], [10, 10], [0, 10]]),
        with_attrs(pb, malik="Veli"))
    add("alan: ikisi de köşe taşıdı farklı: çatışma", pb, polygon([[0, 0], [12, 0], [10, 10], [0, 10]]),
        polygon([[0, 0], [11, 0], [10, 10], [0, 10]]))
    add("alan: onlar delik ekledi, ben parsel no", pb, with_attrs(pb, parsel_no="6"),
        polygon(holes=[{"pts": [{"x": 487002, "y": 4420002}, {"x": 487004, "y": 4420002}, {"x": 487004, "y": 4420004}]}]))
    lb = line()
    add("çizgi: ben bir ucu, onlar öbür ucu", lb, line(a={"x": 487001.0, "y": 4420000.0}),
        line(b={"x": 487051.0, "y": 4420000.0}))
    add("çizgi: öznitelik yokken ikisi de ekledi farklı", lb, with_attrs(lb, ad="X"), with_attrs(lb, ad="Y"))
    add("çizgi: öznitelik yokken yalnız onlar ekledi", lb, lb, with_attrs(lb, ad="Y"))
    return out


def main():
    data = {"format": "kentos.entity-merge-cases", "version": 1, "cases": cases()}
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil; betiği --check'siz çalıştırın", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)} güncel: {len(data['cases'])} durum.")
        return 0
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(data['cases'])} durum")
    return 0


if __name__ == "__main__":
    sys.exit(main())
