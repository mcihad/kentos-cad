#!/usr/bin/env python3
"""Writes fixtures/style/v1/template-layers.json: where an object template draws
in a drawing's layer tree (docs/adr/0176 §3), worked out here from the rule, in
plain Python, without KentOS code.

The rule: the layer is found by its name (white space around names left out);
of several, the one whose groups from the top have the template's path, else
the first in the tree. A layer locked by itself or by a group above it keeps
the template from starting. Without one, the path's groups are followed as far
as the tree has them (the first group of each name at each level), and the
rest are opened, the layer last; a locked group there keeps it from starting
too. The desktop (`kentos_interaction::templates::find_layer`) and the web
(`model/objectTemplate.ts`'s `templateLayer`) both read these cases.

    python3 scripts/fixtures/template_layer_cases.py          # writes the cases
    python3 scripts/fixtures/template_layer_cases.py --check  # fails when they differ
"""

import json
import sys

PATH = "fixtures/style/v1/template-layers.json"


def L(id_, name, locked=False):
    node = {"id": id_, "name": name, "type": "layer"}
    if locked:
        node["locked"] = True
    return node


def G(id_, name, children, locked=False):
    node = {"id": id_, "name": name, "type": "group", "children": children}
    if locked:
        node["locked"] = True
    return node


def answer(tree, layer):
    """The rule, read the way it is written: a pre-order walk with the groups above."""
    name = layer["name"].strip()
    path = [g.strip() for g in layer["path"]]
    found = []  # (node, groups above, locked along)

    def walk(nodes, groups, locked):
        for n in nodes:
            here = locked or n.get("locked", False)
            if n["type"] == "layer":
                if n["name"].strip() == name:
                    found.append((n, list(groups), here))
            else:
                walk(n["children"], groups + [n["name"].strip()], here)

    walk(tree, [], False)
    if found:
        chosen = next((f for f in found if f[1] == path), found[0])
        return {"locked": chosen[0]["id"]} if chosen[2] else {"found": chosen[0]["id"]}
    level, parent, locked, depth = tree, None, False, 0
    for g in path:
        nxt = next((n for n in level if n["type"] == "group" and n["name"].strip() == g), None)
        if nxt is None:
            break
        parent, level, depth = nxt, nxt["children"], depth + 1
        locked = locked or nxt.get("locked", False)
    if parent is not None and locked:
        return {"locked": parent["id"]}
    return {"open": {"parent": parent["id"] if parent else None, "create": path[depth:]}}


TWO_PARCELS = [
    G("eski", "Eski", [L("eski-parsel", "Parsel")]),
    G("kadastro", "Kadastro", [L("parsel", "Parsel"), L("bina", "Bina")]),
    L("cizim", "Çizim"),
]

CASES = [
    ("katman en üstte adıyla bulunur", [L("cizim", "Çizim"), L("parsel", "Parsel")], [], "Parsel"),
    ("aynı adda iki katmandan şablonun yolundaki", TWO_PARCELS, ["Kadastro"], "Parsel"),
    ("aynı adda iki katman, yol hiçbirinde değil: ağaçta ilk olan", TWO_PARCELS, ["Tapu"], "Parsel"),
    ("aynı adda iki katman, yol en üst: ağaçta ilk olan", TWO_PARCELS, [], "Parsel"),
    ("tek katman başka bir grupta: adıyla bulunur, yol aranmaz", TWO_PARCELS, ["Yapı"], "Bina"),
    ("çizimde yok, yol boş: en üstte açılır", TWO_PARCELS, [], "Yol"),
    ("çizimde yok, grubu var: grubun içinde açılır", TWO_PARCELS, ["Kadastro"], "Yol"),
    ("çizimde yok, yolun ilk grubu var, ikincisi yok: ikincisi açılır",
     TWO_PARCELS, ["Kadastro", "Ulaşım"], "Yol"),
    ("çizimde yok, yolun grupları yok: hepsi en üstte açılır", TWO_PARCELS, ["Ulaşım", "Kent içi"], "Yol"),
    ("katmanın kendisi kilitli", [L("cizim", "Çizim"), L("parsel", "Parsel", locked=True)], [], "Parsel"),
    ("katmanın üstündeki grup kilitli", [G("kadastro", "Kadastro", [L("parsel", "Parsel")], locked=True)],
     ["Kadastro"], "Parsel"),
    ("iki katmandan yoldaki kilitli: kilitli sayılır, öbürüne geçilmez",
     [L("parsel-ust", "Parsel"), G("kadastro", "Kadastro", [L("parsel", "Parsel")], locked=True)],
     ["Kadastro"], "Parsel"),
    ("açılacağı grup kilitli", [G("kadastro", "Kadastro", [L("bina", "Bina")], locked=True)],
     ["Kadastro"], "Parsel"),
    ("açılacağı grubun üstündeki grup kilitli",
     [G("plan", "Plan", [G("kadastro", "Kadastro", [])], locked=True)], ["Plan", "Kadastro"], "Parsel"),
    ("kilitli grup yolun dışında kalır: yolun ilk grubu yoksa en üstte açılır",
     [G("kadastro", "Kadastro", [], locked=True)], ["Tapu"], "Parsel"),
    ("katmanın adında grup var: grup katman değildir, katman açılır",
     [G("parsel-grup", "Parsel", [L("ada", "Ada")])], [], "Parsel"),
    ("adların çevresindeki boşluklar sayılmaz",
     [G("kadastro", " Kadastro ", [L("parsel", "Parsel ")])], ["Kadastro"], " Parsel"),
    ("aynı adda iki grup: ilkinin içinde açılır",
     [G("k1", "Kadastro", [L("bina", "Bina")]), G("k2", "Kadastro", [L("ada", "Ada")])], ["Kadastro"], "Parsel"),
    ("aynı adda iki grup, katman ikincisinde: yolu tuttuğu için o",
     [G("k1", "Kadastro", []), G("k2", "Kadastro", [L("parsel", "Parsel")])], ["Kadastro"], "Parsel"),
    ("ad büyük-küçük harfe duyarlıdır",
     [L("parsel", "parsel")], [], "Parsel"),
    ("derin yolda katman", [G("a", "A", [G("b", "B", [G("c", "C", [L("x", "X")])])])], ["A", "B", "C"], "X"),
    ("derin yolda katman, şablonun yolu kısa: adıyla bulunur",
     [G("a", "A", [G("b", "B", [L("x", "X")])])], ["A"], "X"),
]


def build():
    cases = []
    for name, tree, path, layer_name in CASES:
        layer = {"path": path, "name": layer_name}
        cases.append({"name": name, "tree": tree, "layer": layer, "result": answer(tree, layer)})
    return {
        "format": "kentos.template-layer-cases",
        "version": 1,
        "note": (
            "ADR 0176 §3: nesne şablonunun çizeceği katman. Katman adıyla bulunur (adların çevresindeki boşluklar "
            "sayılmaz, büyük-küçük harf sayılır); aynı adda birden çok katman varsa grupları en üstten şablonun "
            "yolunu tutan, yoksa ağaçta ilk olan. Kendisi ya da üstündeki bir grup kilitliyse şablon başlamaz "
            "(locked). Katman yoksa yolun grupları ağaçta bulundukça izlenir (her düzeyde o addaki ilk grup), kalanı "
            "ve katman açılır (open: parent, create); açılacağı grup kilitliyse şablon başlamaz. Üretici: "
            "scripts/fixtures/template_layer_cases.py (KentOS kodu olmadan)."
        ),
        "cases": cases,
    }


def compact(value):
    return json.dumps(value, ensure_ascii=False, separators=(", ", ": "))


def text_of(doc):
    lines = ["{"]
    for key in ("format", "version", "note"):
        lines.append(f"  {json.dumps(key)}: {compact(doc[key])},")
    lines.append('  "cases": [')
    lines.append(",\n".join(f"    {compact(c)}" for c in doc["cases"]))
    lines.append("  ]")
    lines.append("}")
    return "\n".join(lines) + "\n"


def main():
    text = text_of(build())
    if "--check" in sys.argv[1:]:
        with open(PATH, encoding="utf-8") as f:
            if f.read() != text:
                sys.exit(f"{PATH} is not what this script writes: run it without --check and read the difference.")
        print(f"{len(CASES)} cases match")
        return
    with open(PATH, "w", encoding="utf-8") as f:
        f.write(text)
    print(f"{len(CASES)} cases written")


if __name__ == "__main__":
    main()
