#!/usr/bin/env python3
"""The links among objects copied together (docs/adr/0793 §2): the shared cases of cad.entities.transform's copy,
cad.entities.array and cad.entities.paste, written from the ADR's rule without KentOS code.

The rule: a copy, an array's items and a paste make each object's copy in turn (the array item by item, in the
order of its `uids`; the paste in its clip's order), the copies taking the drawing's next slots. A tag's `of` naming
an object copied with it (in the same item) names that object's copy; naming another, it stays. A transform's copy
and an array's item drop a label's object, a hatch's boundaries and a dimension's references (docs/adr/0175 §4,
docs/adr/0249 §3); a paste drops the label's and the hatch's (docs/adr/0309 §6) and points a dimension's references at
the copies. Positions move by the transform's displacement (moves and arrays of whole numbers: exact in binary).

    python3 scripts/fixtures/copy_links_cases.py          # writes fixtures/commands/v1/cad.entities.{transform,array,paste}.links.json
    python3 scripts/fixtures/copy_links_cases.py --check  # fails when a file differs from what this writes
"""

import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1"


def uid(n):
    return f"0192f5a1-7930-7000-8000-{n:012x}"


STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}


def layer(i, name, locked=False):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": dict(STYLE), "children": []}


LAYERS = [layer("kolon", "Kolon"), layer("etiket", "Etiket"), layer("cizim", "Çizim"), layer("olcu", "Ölçü")]


def column(i, x, y, tag):
    return {"kind": "column", "id": i, "layerId": "kolon", "attrs": {}, "at": {"x": x, "y": y}, "width": 0.4,
            "depth": 0.5, "tag": tag}


def tag(i, of, x, y):
    return {"kind": "tag", "id": i, "layerId": "etiket", "attrs": {}, "of": uid(of), "at": {"x": x, "y": y},
            "text": ""}


def line(i, a, b):
    return {"kind": "line", "id": i, "layerId": "cizim", "attrs": {}, "a": {"x": a[0], "y": a[1]},
            "b": {"x": b[0], "y": b[1]}}


def dimension(i, of):
    return {"kind": "dimension", "id": i, "layerId": "olcu", "attrs": {}, "style": "aligned",
            "a": {"x": 20.0, "y": 0.0}, "b": {"x": 30.0, "y": 0.0}, "offset": 1.5, "height": 0.25,
            "refs": {"a": {"uid": uid(of), "place": {"kind": "start"}}, "b": {"uid": uid(of), "place": {"kind": "end"}}}}


# The drawing: two columns with their tags, a line with its dimension, a third column untagged.
ENTITIES = [
    column(1, 0.0, 0.0, "S1"),
    column(2, 10.0, 0.0, "S2"),
    tag(3, 1, 0.0, 2.0),
    tag(4, 2, 10.0, 2.0),
    line(5, (20.0, 0.0), (30.0, 0.0)),
    dimension(6, 5),
    column(7, 40.0, 0.0, "S3"),
]
SETUP = {
    "format": "kentos.document", "version": 2, "name": "Kopyanın bağları",
    "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                 "plotScale": 50.0, "workspace": "cad", "drawingUnit": "m"},
    "origin": {"x": 0.0, "y": 0.0}, "layers": LAYERS, "activeLayer": "cizim",
    "styles": {"categories": [], "items": []},
    "entities": ENTITIES, "uids": [uid(e["id"]) for e in ENTITIES],
}
NEXT = max(e["id"] for e in ENTITIES) + 1
BY_ID = {e["id"]: e for e in ENTITIES}


def moved(p, d):
    return {"x": p["x"] + d[0], "y": p["y"] + d[1]}


def copies(objects, origins, items, keep_refs):
    """The copies in turn: (slot, object, fields to expect). `objects` the originals (their ids in `origins` order),
    `items` the displacements, one item each."""
    out = []
    slot = NEXT
    for d in items:
        group = {}
        for o in origins:
            group[o] = slot
            slot += 1
        for o in origins:
            e = objects[o]
            want = {}
            if e["kind"] == "tag":
                of = int(e["of"].rsplit("-", 1)[1], 16)
                want["of"] = f"$uidOf:{group[of]}" if of in group else e["of"]
                want["at"] = moved(e["at"], d)
            elif e["kind"] == "column":
                want["at"] = moved(e["at"], d)
            elif e["kind"] == "dimension":
                if keep_refs:
                    refs = copy.deepcopy(e["refs"])
                    for k in ("a", "b"):
                        of = int(refs[k]["uid"].rsplit("-", 1)[1], 16)
                        refs[k]["uid"] = f"$uidOf:{group[of]}" if of in group else refs[k]["uid"]
                    want["refs"] = refs
                else:
                    want["refs"] = None
            out.append((group[o], o, want))
    return out


def originals_kept():
    """Every original as it was: its tag still names its own object."""
    return {"3": {"of": uid(1)}, "4": {"of": uid(2)}}


def case(name, command, inp, made, result_output, warnings=None):
    created = [f"$uidOf:{s}" for s, _, _ in made]
    fields = originals_kept()
    for s, _, want in made:
        if want:
            fields[str(s)] = want
    ids = [e["id"] for e in ENTITIES] + [s for s, _, _ in made]
    return {
        "name": name,
        "steps": [
            {"op": "execute", "input": inp,
             "result": {"status": "completed", "output": result_output(created), "warnings": warnings or []},
             "expect": {"ids": ids, "fields": fields, "revision": "changed", "canUndo": True}},
            {"op": "undo", "expect": {"ids": [e["id"] for e in ENTITIES], "fields": originals_kept(),
                                      "canUndo": False}},
        ],
    }


def transform_cases():
    out = lambda created: {"changed": [], "created": created, "locked": [], "revision": "$current"}
    cases = []
    picks = [
        ("etiket nesnesiyle birlikte: kopya etiket kopya kolonu adlar", [1, 3]),
        ("önce etiket, sonra nesnesi: kopya etiket yine kopya kolonu adlar", [3, 1]),
        ("iki kolon iki etiketiyle: her kopya etiket kendi kolonunun kopyasını adlar", [1, 2, 3, 4]),
        ("etiketler önde, kolonlar arkada", [3, 4, 1, 2]),
        ("yalnız etiket: kopya etiket özgün kolonu adlamayı sürdürür", [3]),
        ("yalnız kolon: etiket kopyalanmaz", [1]),
        ("bir kolon iki etiket: nesnesi gelen kopyayı, öbürü özgününü adlar", [1, 3, 4]),
        ("etiketsiz kolonla: etiketler kendi kolonlarının kopyasını adlar", [7, 1, 3]),
        ("ölçü çizgisiyle: kopyanın noktaları kendinindir (ADR 0249 §3), etiketi kolonunu izler", [5, 6, 1, 3]),
    ]
    moves = [(12.0, 0.0), (0.0, -8.0), (5.5, 3.25)]
    for name, picks_ in picks:
        for d in moves[:2]:
            made = copies(BY_ID, picks_, [d], keep_refs=False)
            inp = {"uids": [f"$uidOf:{i}" for i in picks_], "transform": {"kind": "move", "dx": d[0], "dy": d[1]},
                   "copy": True}
            cases.append(case(f"Kopyala {d[0]:g}, {d[1]:g}: {name}", "cad.entities.transform", inp, made, out))
    # A turn, a mirror and a scale move the tag with the copy; only its object is asked here.
    for kind, extra in [("rotate", {"center": {"x": 0.0, "y": 0.0}, "angle": 1.5707963267948966}),
                        ("mirror", {"a": {"x": 5.0, "y": -1.0}, "b": {"x": 5.0, "y": 1.0}}),
                        ("scale", {"center": {"x": 0.0, "y": 0.0}, "factor": 2})]:
        made = copies(BY_ID, [1, 3], [(0.0, 0.0)], keep_refs=False)
        for _, _, want in made:
            want.pop("at", None)
        inp = {"uids": ["$uidOf:1", "$uidOf:3"], "transform": {"kind": kind, **extra}, "copy": True}
        cases.append(case(f"{kind} kopyası: etiket kopya kolonu adlar", "cad.entities.transform", inp, made, out))
    return cases


def array_cases():
    out = lambda created: {"created": created, "locked": [], "revision": "$current"}
    cases = []
    picks = [
        ("etiket nesnesiyle", [1, 3]),
        ("önce etiket", [3, 1]),
        ("iki kolon iki etiket", [1, 2, 3, 4]),
        ("yalnız etiket: kopyalar özgün kolonu adlar", [3]),
        ("bir kolon iki etiket", [1, 3, 4]),
        ("ölçü çizgisiyle", [5, 6, 1, 3]),
        ("etiketsiz kolonla", [7, 1, 3]),
        ("etiketler önde", [3, 4, 1, 2]),
    ]
    for name, picks_ in picks:
        for kind, layout, items in [
            ("line", {"kind": "line", "count": 3, "dx": 15.0, "dy": 0.0}, [(15.0, 0.0), (30.0, 0.0)]),
            ("grid", {"kind": "grid", "rows": 2, "cols": 2, "dx": 50.0, "dy": -20.0},
             [(50.0, 0.0), (0.0, -20.0), (50.0, -20.0)]),
            ("eğik", {"kind": "line", "count": 2, "dx": 10.0, "dy": 10.0}, [(10.0, 10.0)]),
        ]:
            made = copies(BY_ID, picks_, items, keep_refs=False)
            inp = {"uids": [f"$uidOf:{i}" for i in picks_], "layout": layout}
            cases.append(case(f"Dizi ({kind}): {name}: her öğenin etiketi kendi öğesinin kolonunu adlar",
                              "cad.entities.array", inp, made, out))
    return cases


def paste_cases():
    out = lambda created: {"created": created, "report": {"layers": [], "blocks": [], "kept": [], "styles": []},
                           "revision": "$current"}
    cases = []
    picks = [
        ("kolon ve etiketi: kopya etiket yapıştırılan kolonu adlar", [1, 3]),
        ("önce etiket", [3, 1]),
        ("iki kolon iki etiket", [1, 2, 3, 4]),
        ("yalnız etiket: özgün kolonu adlar", [3]),
        ("çizgi ve ölçüsü: ölçünün bağları yapıştırılan çizgiyedir", [5, 6]),
        ("önce ölçü, sonra çizgi", [6, 5]),
        ("yalnız ölçü: bağları özgün çizgiyi adlar", [6]),
        ("hepsi", [1, 2, 3, 4, 5, 6, 7]),
    ]
    for name, picks_ in picks:
        for at in [(100.0, 0.0), (0.0, 50.0), (-25.5, 12.25)]:
            made = copies(BY_ID, picks_, [at], keep_refs=True)
            clip = {"objects": [BY_ID[i] for i in picks_], "uids": [uid(i) for i in picks_],
                    "layers": LAYERS, "base": {"x": 0.0, "y": 0.0}}
            inp = {"clip": clip, "at": {"x": at[0], "y": at[1]}}
            cases.append(case(f"Yapıştır {at[0]:g}, {at[1]:g}: {name}", "cad.entities.paste", inp, made, out))
    return cases


def document(command, title, cases):
    return {
        "format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
        "note": ("ADR 0793 §2. Bağımsız başvuru scripts/fixtures/copy_links_cases.py: kopyalar sırayla sonraki "
                 "yuvaları alır (dizi öğe öğe); birlikte kopyalanan nesneyi adlayan etiketin (ve yapıştırmada "
                 "ölçünün) bağı onun kopyasınadır, öbürü kalır; KentOS kodu çalıştırılmaz. $uidOf:N, N yuvasındaki "
                 "nesnenin kalıcı kimliğidir."),
        "setup": SETUP, "cases": cases,
    }


FILES = {
    "cad.entities.transform.links.json": lambda: document(
        "cad.entities.transform", "Kopyanın bağları: etiket kopyalanan nesnesinin kopyasını adlar", transform_cases()),
    "cad.entities.array.links.json": lambda: document(
        "cad.entities.array", "Dizinin bağları: her öğenin etiketi kendi öğesindeki nesneyi adlar", array_cases()),
    "cad.entities.paste.links.json": lambda: document(
        "cad.entities.paste", "Yapıştırmanın bağları: etiket ve ölçü yapıştırılan nesneyi adlar", paste_cases()),
}


def main():
    check = "--check" in sys.argv[1:]
    bad = []
    for name, make in FILES.items():
        text = json.dumps(make(), ensure_ascii=False, indent=1) + "\n"
        path = OUT / name
        n = len(json.loads(text)["cases"])
        if check:
            if not path.exists() or path.read_text() != text:
                bad.append(name)
        else:
            path.write_text(text)
        print(f"{name}: {n} durum")
    if bad:
        print("farklı: " + ", ".join(bad), file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
