"""Independent reference of Kullanılmayanları temizle's rule (docs/adr/0177 §5), and the cases both platforms play.

What may go: the layers (not the active one, not locked, with its groups' locks), the groups (not locked), the block
definitions, and the project library's symbols and assets. Something stays when anything that stays uses it:

- a layer, when an object of the drawing is on it, or an object of a definition that stays;
- a group, when a node under it stays;
- a definition, when an insert of the drawing places it, or an insert in a definition that stays;
- a symbol, when an object of the drawing or of a definition that stays draws with it, a layer that stays names it in
  its style (`ref`), or a template of any library names it (`symbol`, its members' too);
- an asset, when a symbol that stays (of any library) draws with it, a layer that stays does in its style (`asset`), or
  a picture of the drawing shows it (its `asset`, docs/adr/0192 §2).

Found is the largest set of those that nothing staying uses: start with all of them and give back, round after round,
what something staying uses. Locked empty layers are listed too, to be unlocked first; they stay. Removing the checked
ones takes the same rule over the checked ones alone: one left unchecked may keep others (a layer its symbol, a block
its layer). Definitions go in rounds (one an insert in another goes after it), groups deepest first, so each is empty
when it goes.

    python3 scripts/fixtures/layer_purge_cases.py           # writes fixtures/layers/v1/purge.json
    python3 scripts/fixtures/layer_purge_cases.py --check   # writes nothing; compares

The rule is written here again, from the ADR, not from either platform's code.
"""

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/layers/v1/purge.json"

KINDS = ("layers", "groups", "blocks", "symbols", "assets")


def strings_at(value, key, out):
    """Every string under `key` anywhere in a JSON value."""
    if isinstance(value, dict):
        for k, v in value.items():
            if k == key and isinstance(v, str):
                out.add(v)
            else:
                strings_at(v, key, out)
    elif isinstance(value, list):
        for v in value:
            strings_at(v, key, out)
    return out


def walk(nodes, parent=None):
    for n in nodes:
        yield n, parent
        yield from walk(n["children"], n)


def paths(tree):
    out = {}

    def go(nodes, above):
        for n in nodes:
            out[n["id"]] = above + [n["name"]]
            go(n["children"], out[n["id"]])

    go(tree, [])
    return {k: " / ".join(v) for k, v in out.items()}


def locked_ids(tree):
    out = set()

    def go(nodes, above):
        for n in nodes:
            here = above or n["locked"]
            if here:
                out.add(n["id"])
            go(n["children"], here)

    go(tree, False)
    return out


def removable(doc):
    """What may go: (kind, id) pairs."""
    locked = locked_ids(doc["tree"])
    out = set()
    for n, _ in walk(doc["tree"]):
        if n["id"] in locked:
            continue
        if n["type"] == "layer" and n["id"] != doc["active"]:
            out.add(("layers", n["id"]))
        if n["type"] == "group":
            out.add(("groups", n["id"]))
    for b in doc["blocks"]:
        out.add(("blocks", b["id"]))
    for it in doc["library"]:
        if it["source"] == "project" and it["kind"] in ("symbol", "asset"):
            out.add((it["kind"] + "s", it["id"]))
    return out


def used_by_staying(doc, going):
    used = set()

    def objects(objs):
        for o in objs:
            used.add(("layers", o["layer"]))
            if o.get("symbol"):
                used.add(("symbols", o["symbol"]))
            if o.get("block"):
                used.add(("blocks", o["block"]))
            if o.get("asset"):
                used.add(("assets", o["asset"]))

    objects(doc["objects"])
    for b in doc["blocks"]:
        if ("blocks", b["id"]) not in going:
            objects(b["objects"])
    for n, parent in walk(doc["tree"]):
        kind = "layers" if n["type"] == "layer" else "groups"
        if (kind, n["id"]) in going:
            continue
        if parent is not None:
            used.add(("groups", parent["id"]))
        if n["type"] == "layer":
            renderer = n["style"].get("renderer")
            for ref in strings_at(renderer, "ref", set()):
                used.add(("symbols", ref))
            for asset in strings_at(renderer, "asset", set()):
                used.add(("assets", asset))
    for it in doc["library"]:
        if it["kind"] == "template":
            for ref in strings_at(it["template"], "symbol", set()):
                used.add(("symbols", ref))
        if it["kind"] == "symbol" and ("symbols", it["id"]) not in going:
            for asset in strings_at(it["symbol"], "asset", set()):
                used.add(("assets", asset))
    return used


def settle(doc, going):
    """The largest part of `going` nothing staying uses."""
    going = set(going)
    while True:
        used = used_by_staying(doc, going)
        kept = {x for x in going if x in used}
        if not kept:
            return going
        going -= kept


def block_rounds(doc, ids):
    """The definitions in the order they can go: one placed in another after it."""
    left = [b for b in doc["blocks"] if b["id"] in ids]
    out = []
    while left:
        inside = {o["block"] for b in left for o in b["objects"] if o.get("block")}
        now = [b["id"] for b in left if b["id"] not in inside]
        if not now:
            raise ValueError("a definition placed in itself")
        out += now
        left = [b for b in left if b["id"] not in now]
    return out


def deepest_first(tree, ids):
    out = []

    def go(nodes):
        for n in nodes:
            go(n["children"])
            if n["id"] in ids:
                out.append(n["id"])

    go(tree)
    return out


def found(doc):
    going = settle(doc, removable(doc))
    p = paths(doc["tree"])
    locked = locked_ids(doc["tree"])
    on_objects = {o["layer"] for o in doc["objects"]}
    on_blocks = {o["layer"] for b in doc["blocks"] if ("blocks", b["id"]) not in going for o in b["objects"]}
    layers = []
    for n, _ in walk(doc["tree"]):
        if n["type"] != "layer":
            continue
        if ("layers", n["id"]) in going:
            layers.append({"id": n["id"], "path": p[n["id"]], "locked": False})
        elif n["id"] in locked and n["id"] != doc["active"] and n["id"] not in on_objects and n["id"] not in on_blocks:
            layers.append({"id": n["id"], "path": p[n["id"]], "locked": True})
    groups = [{"id": n["id"], "path": p[n["id"]]} for n, _ in walk(doc["tree"]) if ("groups", n["id"]) in going]
    blocks = [{"id": b["id"], "name": b["name"]} for b in doc["blocks"] if ("blocks", b["id"]) in going]
    lib = [it for it in doc["library"] if it["source"] == "project"]
    symbols = [{"id": it["id"], "name": it["name"]} for it in lib if ("symbols", it["id"]) in going]
    assets = [{"id": it["id"], "name": it["name"]} for it in lib if ("assets", it["id"]) in going]
    return {"layers": layers, "groups": groups, "blocks": blocks, "symbols": symbols, "assets": assets}


def apply(doc, checked):
    """What removing the checked ones takes, in the order it goes, and how many checked ones stay."""
    asked = {(k, i) for k in KINDS for i in checked.get(k, [])}
    going = settle(doc, asked & removable(doc))
    ids = lambda kind: {i for k, i in going if k == kind}
    lib = [it for it in doc["library"] if it["source"] == "project"]
    removed = {
        "layers": [n["id"] for n, _ in walk(doc["tree"]) if n["type"] == "layer" and n["id"] in ids("layers")],
        "groups": deepest_first(doc["tree"], ids("groups")),
        "blocks": block_rounds(doc, ids("blocks")),
        "symbols": [it["id"] for it in lib if it["kind"] == "symbol" and it["id"] in ids("symbols")],
        "assets": [it["id"] for it in lib if it["kind"] == "asset" and it["id"] in ids("assets")],
    }
    return removed, len(asked) - len(going)


def style(renderer=None):
    s = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
    if renderer is not None:
        s["renderer"] = renderer
    return s


def node(id_, name, kind="layer", locked=False, children=None, renderer=None):
    return {"id": id_, "name": name, "type": kind, "visible": True, "locked": locked, "expanded": True,
            "style": style(renderer), "children": children or []}


AGAC = "0190b3a1-0000-7000-8000-000000000001"
KAPI = "0190b3a1-0000-7000-8000-000000000002"
MENTESE = "0190b3a1-0000-7000-8000-000000000003"
DIREK = "0190b3a1-0000-7000-8000-000000000004"


def drawing():
    lejant_style = {"type": "rules", "rules": [{"id": "r1", "label": "Yol", "symbols": {"line": {"ref": "p-yol"}}}]}
    inline = {"type": "single", "symbols": {"marker": {"type": "marker", "layers": [{"id": "m1", "type": "svg", "asset": "p-isaret", "size": 3}]}}}
    tree = [
        node("kadastro", "Kadastro", "group", children=[
            node("parsel", "Parsel"),
            node("eski", "Eski parsel"),
            node("bina", "Bina", locked=True),
        ]),
        node("arsiv", "Arşiv", "group", children=[
            node("y2019", "2019"),
            node("y2020", "2020"),
            node("bos-grup", "Boş grup", "group"),
        ]),
        node("altyapi", "Altyapı", "group"),
        node("bitki", "Bitki"),
        node("kapi", "Kapı"),
        node("lejant", "Lejant", renderer=lejant_style),
        node("isaret", "İşaret", renderer=inline),
        node("yol", "Yol"),
        node("cizim", "Çizim"),
        node("kilitli-grup", "Kilitli grup", "group", locked=True, children=[node("ic", "İç")]),
    ]
    objects = [
        {"layer": "parsel"},
        {"layer": "yol", "block": AGAC},
        {"layer": "cizim", "symbol": "p-agac"},
        {"layer": "isaret"},
        # A picture shows its image (docs/adr/0192 §2).
        {"layer": "cizim", "asset": "p-foto"},
    ]
    blocks = [
        {"id": AGAC, "name": "Ağaç", "objects": [{"layer": "bitki", "symbol": "p-yaprak"}]},
        {"id": KAPI, "name": "Eski kapı", "objects": [{"layer": "kapi"}, {"layer": "kapi", "block": MENTESE}]},
        {"id": MENTESE, "name": "Menteşe", "objects": [{"layer": "kapi"}]},
        {"id": DIREK, "name": "Direk", "objects": [{"layer": "yol"}]},
    ]
    marker = lambda asset: {"type": "marker", "layers": [{"id": "m1", "type": "svg", "asset": asset, "size": 4}]}
    library = [
        {"id": "p-agac", "kind": "symbol", "source": "project", "name": "Ağaç simgesi", "symbol": marker("p-agac-resmi")},
        {"id": "p-yaprak", "kind": "symbol", "source": "project", "name": "Yaprak", "symbol": marker("p-yaprak-resmi")},
        {"id": "p-yol", "kind": "symbol", "source": "project", "name": "Yol çizgisi",
         "symbol": {"type": "line", "layers": [{"id": "l1", "type": "simpleLine", "color": "#4F8EF7", "width": 0.5}]}},
        {"id": "p-ok", "kind": "symbol", "source": "project", "name": "Ok", "symbol": marker("p-ok-resmi")},
        {"id": "p-uye", "kind": "symbol", "source": "project", "name": "Üye çizgisi",
         "symbol": {"type": "line", "layers": [{"id": "l1", "type": "simpleLine", "color": "#8C9AAA", "width": 0.18}]}},
        {"id": "p-bos", "kind": "symbol", "source": "project", "name": "Kullanılmayan simge", "symbol": marker("p-resim")},
        {"id": "p-agac-resmi", "kind": "asset", "source": "project", "name": "ağaç.svg"},
        {"id": "p-yaprak-resmi", "kind": "asset", "source": "project", "name": "yaprak.svg"},
        {"id": "p-ok-resmi", "kind": "asset", "source": "project", "name": "ok.svg"},
        {"id": "p-resim", "kind": "asset", "source": "project", "name": "resim.png"},
        {"id": "p-isaret", "kind": "asset", "source": "project", "name": "işaret.svg"},
        {"id": "p-logo", "kind": "asset", "source": "project", "name": "logo.svg"},
        {"id": "p-yalniz", "kind": "asset", "source": "project", "name": "yalnız.svg"},
        {"id": "p-foto", "kind": "asset", "source": "project", "name": "saha.jpg"},
        {"id": "u-logo", "kind": "symbol", "source": "user", "name": "Logo", "symbol": marker("p-logo")},
        {"id": "u-sablon", "kind": "template", "source": "user", "name": "Yol şablonu",
         "template": {"tool": "polyline", "layer": {"path": ["Yol"]}, "symbol": "p-ok",
                      "members": [{"offset": 2, "symbol": "p-uye"}]}},
        {"id": "s-sistem", "kind": "symbol", "source": "system", "name": "Sistem simgesi", "symbol": marker("s-resim")},
    ]
    return {"tree": tree, "active": "parsel", "objects": objects, "blocks": blocks, "library": library}


def new_drawing():
    return {"tree": [node("0", "0")], "active": "0", "objects": [], "blocks": [], "library": []}


def cases():
    out = []
    doc = drawing()
    f = found(doc)
    every = {k: [e["id"] for e in f[k] if not e.get("locked")] for k in KINDS}

    def without(kind, id_):
        c = {k: list(v) for k, v in every.items()}
        c[kind] = [i for i in c[kind] if i != id_]
        return c

    applies = []
    for name, checked in [
        ("Bulunanların hepsi işaretli: hepsi gider; bloklar iç içe sırayla, gruplar en derinden", every),
        ("Lejant işaretsiz: kaldığı için stilindeki Yol çizgisi de kalır", without("layers", "lejant")),
        ("Eski kapı işaretsiz: Kapı katmanı ve Menteşe onun için kalır", without("blocks", KAPI)),
        ("Arşiv / 2019 işaretsiz: Arşiv grubu da kalır, 2020 ve Boş grup gider", without("layers", "y2019")),
        ("Kullanılmayan simge işaretsiz: çizdiği resim.png de kalır", without("symbols", "p-bos")),
        ("Yalnız resim.png işaretli, simgesi işaretsiz: hiçbir şey gitmez", {"assets": ["p-resim"]}),
        ("Kilitli katman ya da etkin katman işaretlense de gitmez", {"layers": ["bina", "parsel", "ic"]}),
    ]:
        removed, kept = apply(doc, checked)
        applies.append({"name": name, "checked": checked, "removed": removed, "kept": kept})
    out.append({"name": "Çizim: boş ve kilitli katmanlar, boş kalan gruplar, iç içe bloklar, projenin sembolleri ve varlıkları",
                "document": doc, "found": f, "applies": applies})
    nd = new_drawing()
    out.append({"name": "Yeni çizim: tek ve etkin katman kalır, temizlenecek bir şey yok", "document": nd, "found": found(nd),
                "applies": []})
    return {"format": "kentos.layer-purge-cases", "version": 1,
            "note": "Written by scripts/fixtures/layer_purge_cases.py from docs/adr/0177 §5, not from either platform's code. "
                    "`document.objects` are the drawing's objects (their layer, library symbol, placed block), each "
                    "block's `objects` its definition's; `found` is what the window lists, each kind in its order "
                    "(layers and groups in the tree's, blocks in the definitions', library items in the project "
                    "library's), a locked layer with `locked`; `removed` what removing `checked` takes in the order it "
                    "goes (definitions in rounds, groups deepest first); `kept` how many checked ones stay.",
            "cases": out}


def main():
    text = json.dumps(cases(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil: python3 scripts/fixtures/layer_purge_cases.py ile yeniden yazın")
            sys.exit(1)
        print(f"Kullanılmayanları temizle durumları tutarlı: {OUT.relative_to(ROOT)}")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
