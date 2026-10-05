"""Independent reference of the layer list (docs/adr/0177 §6), and the cases both platforms play.

Every node of the tree in its order (a group before what it holds) is a row: its path, its name, its kind, its own
visibility and lock, whether it is the active layer, a layer's colour (the name of one of the drawing colours, else the
value in capitals), line type and weight in mm (two decimals, a decimal comma), and how many objects are on it (a
group: on all its layers). The CSV file is UTF-8 with a byte order mark, fields between semicolons, lines ending in CR
LF, for Excel's Turkish settings; the clipboard takes the same rows between tabs, lines ending in LF. A field holding
the separator, a quote or a line break is quoted, its quotes doubled.

    python3 scripts/fixtures/layer_list_cases.py           # writes fixtures/layers/v1/list.json
    python3 scripts/fixtures/layer_list_cases.py --check   # writes nothing; compares

The rule is written here again, from the ADR, not from either platform's code.
"""

import json
import sys
from decimal import ROUND_HALF_UP, Decimal
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/layers/v1/list.json"

HEADER = ["Yol", "Ad", "Tür", "Görünür", "Kilitli", "Etkin", "Renk", "Çizgi tipi", "Kalınlık (mm)", "Nesne sayısı"]
COLORS = {"ink": "Siyah", "#E5484D": "Kırmızı", "#F2C94C": "Sarı", "#5FBF77": "Yeşil", "#4CC3D9": "Camgöbeği",
          "#4F8EF7": "Mavi", "#C86DD7": "Eflatun", "#8C9AAA": "Gri"}
LINE_TYPES = {"continuous": "Sürekli", "dashed": "Kesikli", "dashdot": "Noktalı kesik", "dotted": "Noktalı"}


def yes(b):
    return "Evet" if b else "Hayır"


def weight(w):
    # Two decimals, halves away from zero, as the display rule does (docs/adr/0149).
    return str(Decimal(repr(w)).quantize(Decimal("0.01"), rounding=ROUND_HALF_UP)).replace(".", ",")


def rows(tree, active, counts):
    out = [HEADER]

    def leaves(n):
        return [n] if n["type"] == "layer" else [l for c in n["children"] for l in leaves(c)]

    def go(nodes, above):
        for n in nodes:
            path = above + [n["name"]]
            layer = n["type"] == "layer"
            st = n["style"]
            out.append([
                " / ".join(path),
                n["name"],
                "Katman" if layer else "Grup",
                yes(n["visible"]),
                yes(n["locked"]),
                yes(n["id"] == active),
                COLORS.get(st["color"], st["color"].upper()) if layer else "",
                LINE_TYPES[st["lineType"]] if layer else "",
                weight(st["lineWeight"]) if layer else "",
                str(sum(counts.get(l["id"], 0) for l in leaves(n))),
            ])
            go(n["children"], path)

    go(tree, [])
    return out


def field(text, sep):
    if any(c in text for c in (sep, '"', "\n", "\r")):
        return '"' + text.replace('"', '""') + '"'
    return text


def csv(rs):
    return "﻿" + "".join(";".join(field(f, ";") for f in r) + "\r\n" for r in rs)


def tsv(rs):
    return "".join("\t".join(field(f, "\t") for f in r) + "\n" for r in rs)


def style(color="fg", line_type="continuous", w=0.25):
    return {"color": color, "lineType": line_type, "lineWeight": w}


def node(id_, name, kind="layer", visible=True, locked=False, st=None, children=None):
    return {"id": id_, "name": name, "type": kind, "visible": visible, "locked": locked, "expanded": True,
            "style": st or style(), "children": children or []}


def cases():
    tree = [
        node("kadastro", "Kadastro", "group", children=[
            node("parsel", "Parsel", st=style("#E5484D", w=0.5)),
            node("bina", "Bina", locked=True, st=style("#8C9AAA", "dashed", 0.18)),
        ]),
        node("yol", "Yol", visible=False, st=style("ink", "dashdot", 0.35)),
        node("not", 'Not; "taslak"', st=style("#123abc", "dotted", 0.125)),
        node("cizim", "Çizim", st=style("fg", "continuous", 0.13)),
    ]
    counts = {"parsel": 12, "bina": 3, "not": 1}
    rs = rows(tree, "parsel", counts)
    return {"format": "kentos.layer-list-cases", "version": 1,
            "note": "Written by scripts/fixtures/layer_list_cases.py from docs/adr/0177 §6, not from either platform's code. "
                    "`counts` are the objects on each layer; `rows` start with the header; `csv` is the file's text "
                    "(with its byte order mark), `tsv` the clipboard's.",
            "cases": [{"name": "Gruplu ağaç: gizli, kilitli ve etkin katman, adı ayırıcı ve tırnak taşıyan katman, renk adları, "
                               "kalınlığın yarımı", "tree": tree, "active": "parsel", "counts": counts, "rows": rs,
                       "csv": csv(rs), "tsv": tsv(rs)}]}


def main():
    text = json.dumps(cases(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil: python3 scripts/fixtures/layer_list_cases.py ile yeniden yazın")
            sys.exit(1)
        print(f"Katman listesi durumları tutarlı: {OUT.relative_to(ROOT)}")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
