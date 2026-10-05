"""Independent reference of the layer states' rules (docs/adr/0177 §4), and the cases both platforms play.

A layer state keeps the tree as it was: every node in the tree's order with its own visibility; saved with locks, each
node's own lock; saved with styles, each layer's style (a group has none). Applying it changes what differs: the
visibility and the lock of the nodes it names that the tree still has, the style of those that are layers; a node the
tree no longer has is passed over and counted, a node the state does not name stays as it is. The tree is in a state
when at least one of the state's nodes is in the tree and nothing would change.

    python3 scripts/fixtures/layer_state_cases.py           # writes fixtures/layers/v1/states.json
    python3 scripts/fixtures/layer_state_cases.py --check   # writes nothing; compares

The rules are written here again, from the ADR, not from either platform's code.
"""

import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/layers/v1/states.json"


def style(color="fg", line_type="continuous", weight=0.25, **extra):
    return {"color": color, "lineType": line_type, "lineWeight": weight, **extra}


def node(id_, name, kind="layer", visible=True, locked=False, st=None, children=None):
    return {"id": id_, "name": name, "type": kind, "visible": visible, "locked": locked, "expanded": True,
            "style": st or style(), "children": children or []}


def walk(nodes):
    for n in nodes:
        yield n
        yield from walk(n["children"])


def capture(tree, id_, name, locks, styles):
    out = []
    for n in walk(tree):
        e = {"node": n["id"], "visible": n["visible"]}
        if locks:
            e["locked"] = n["locked"]
        if styles and n["type"] == "layer":
            e["style"] = copy.deepcopy(n["style"])
        out.append(e)
    return {"id": id_, "name": name, "nodes": out}


def changes(tree, state):
    by = {n["id"]: n for n in walk(tree)}
    visible, locked, styles, missing = [], [], [], 0
    for e in state["nodes"]:
        n = by.get(e["node"])
        if n is None:
            missing += 1
            continue
        if n["visible"] != e["visible"]:
            visible.append([e["node"], e["visible"]])
        if "locked" in e and n["locked"] != e["locked"]:
            locked.append([e["node"], e["locked"]])
        if "style" in e and n["type"] == "layer" and n["style"] != e["style"]:
            styles.append([e["node"], e["style"]])
    return {"visible": visible, "locked": locked, "styles": styles, "missing": missing}


def matches(tree, state):
    c = changes(tree, state)
    present = len(state["nodes"]) - c["missing"]
    return present > 0 and not (c["visible"] or c["locked"] or c["styles"])


def tree():
    return [
        node("kadastro", "Kadastro", "group", children=[
            node("parsel", "Parsel", st=style("#E5484D", weight=0.5)),
            node("bina", "Bina", locked=True, st=style("#8C9AAA", "dashed", 0.18, fill="#8C9AAA33")),
        ]),
        node("yol", "Yol", visible=False),
        node("cizim", "Çizim"),
    ]


def find(t, id_):
    return next(n for n in walk(t) if n["id"] == id_)


def cases():
    t = tree()
    captures = [
        {"name": "Yalnız görünürlük: her düğüm ağacın sırasıyla, kendi görünürlüğüyle", "locks": False, "styles": False},
        {"name": "Kilitlerle: düğümün kendi kilidi de", "locks": True, "styles": False},
        {"name": "Stillerle: katmanın stili de, grubun yok", "locks": False, "styles": True},
        {"name": "Kilit ve stil birlikte", "locks": True, "styles": True},
    ]
    for c in captures:
        c["tree"] = t
        c["id"] = "durum"
        c["stateName"] = "Durum"
        c["state"] = capture(t, "durum", "Durum", c["locks"], c["styles"])

    saved = capture(t, "durum", "Durum", True, True)
    applies = []

    def apply_case(name, changed_tree, state=None):
        st = state or saved
        applies.append({"name": name, "tree": changed_tree, "state": st, "changes": changes(changed_tree, st),
                        "matches": matches(changed_tree, st)})

    apply_case("Kaydedildiği ağaçta hiçbir şey değişmez; ağaç durumdadır", tree())
    t2 = tree()
    find(t2, "parsel")["visible"] = False
    find(t2, "yol")["visible"] = True
    find(t2, "bina")["locked"] = False
    find(t2, "parsel")["style"] = style("#4F8EF7", weight=0.35)
    apply_case("Gizlenen, gösterilen, kilidi açılan ve stili değişen düğümler durumun sırasıyla geri gelir", t2)
    t3 = tree()
    t3[0]["children"] = [n for n in t3[0]["children"] if n["id"] != "bina"]
    t3.append(node("yeni", "Yeni", visible=False))
    find(t3, "cizim")["visible"] = False
    apply_case("Silinmiş düğüm atlanıp sayılır; durumun bilmediği yeni katman olduğu gibi kalır", t3)
    only_visible = capture(tree(), "gorunum", "Görünüm", False, False)
    t4 = tree()
    find(t4, "bina")["locked"] = False
    find(t4, "parsel")["style"] = style("#4F8EF7")
    apply_case("Kilitsiz ve stilsiz kaydedilen durum kilide ve stile dokunmaz", t4, only_visible)
    ghost = {"id": "hayalet", "name": "Hayalet", "nodes": [{"node": "silinmis", "visible": True}]}
    apply_case("Düğümlerinin hiçbiri ağaçta olmayan durum hiçbir şey değiştirmez, ağaç onda sayılmaz", tree(), ghost)
    group_style = {"id": "grup", "name": "Grup", "nodes": [{"node": "kadastro", "visible": True, "style": style("#000000")}]}
    apply_case("Grubun düğümündeki stil uygulanmaz", tree(), group_style)
    return {"format": "kentos.layer-state-cases", "version": 1,
            "note": "Written by scripts/fixtures/layer_state_cases.py from docs/adr/0177 §4, not from either platform's code. "
                    "`changes` lists what applying `state` to `tree` changes, in the state's order: visibility and lock "
                    "pairs [node, value], styles [layer, style]; `missing` counts the state's nodes the tree lacks.",
            "captures": captures, "applies": applies}


def main():
    text = json.dumps(cases(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil: python3 scripts/fixtures/layer_state_cases.py ile yeniden yazın")
            sys.exit(1)
        print(f"Katman durumu durumları tutarlı: {OUT.relative_to(ROOT)}")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
