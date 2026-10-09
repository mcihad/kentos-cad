#!/usr/bin/env python3
"""The processing cases of Ağ analizi's İşlemler tools (docs/adr/0209 §6, §7, §10), written without KentOS code.

The road network of the independent reference (scripts/fixtures/network_cases.py: its graph, places, Dijkstra and the
service areas with shapely) drawn as a drawing, fixtures/processing/v1/network.kcad: the streets on Yol with their
direction, speed and fee as attributes and the closed one by the network's expression, three schools on Okul and four
addresses on Adres (the last far from every street); the project's network Yollar. The cases run En yakın tesis,
Maliyet matrisi and Hizmet alanları on it: their written objects (attributes exactly; lengths and areas within the
file's tolerance, the areas being shapely's), their tables, their notes and their refusals, as both apps' runners must
give them (apps/web/src/processing/cases.test.ts, crates/native/processing/tests/cases.rs). Numbers written into
attributes and tables follow the display rule (scripts/fixtures/numeric_display.py).

    python3 scripts/fixtures/network_processing_cases.py [--check]
"""

import argparse
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from network_cases import INF, ROADS, X0, Y0, Net, run_area, run_nearest  # noqa: E402
from numeric_display import shown  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
PROC_DIR = ROOT / "fixtures/processing/v1"
CASES_OUT = PROC_DIR / "network.json"
DOC_OUT = PROC_DIR / "network.kcad"

DECIMALS = 3
REACH = 100.0
COSTS = ["Uzunluk", "Süre", "Ücret"]

NETWORK = {
    "id": "yollar",
    "name": "Yollar",
    "kind": "road",
    "edges": [{"layer": "yol"}],
    "connect": "ends",
    "tolerance": 0.01,
    "direction": ROADS["rules"]["direction"],
    "costs": ROADS["rules"]["costs"],
    "closed": "kapali = 'evet'",
}


def style(color, weight=0.25, **more):
    return {"color": color, "lineType": "continuous", "lineWeight": weight, **more}


def layer(i, name, st):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True, "style": st, "children": []}


LAYERS = [
    layer("cizim", "Çizim", style("fg")),
    layer("yol", "Yol", style("fg", 0.5)),
    layer("okul", "Okul", style("#E5484D", point={"symbol": "triangle", "size": 8})),
    layer("adres", "Adres", style("#3E63DD", point={"symbol": "ring", "size": 6})),
]

SCHOOLS = [("Okul A", (50, 1)), ("Okul B", (300, 255)), ("Okul C", (395, 1))]
ADDRESSES = [("Adres 1", (20, 150)), ("Adres 2", (300, 0.5)), ("Adres 3", (4, 75)), ("Adres 4", (1000, 1000))]


def street(e):
    """A street of the reference's scene as a drawing's object: its direction, speed and fee as attributes."""
    attrs = {}
    if e["direction"] is not None:
        attrs["yon"] = e["direction"]
    for c, v in zip(ROADS["rules"]["costs"], e["costs"]):
        attrs[c["field"]] = v
    if e["closed"]:
        attrs["kapali"] = "evet"
    out = {"kind": "polyline", "id": int(e["id"]), "layerId": "yol", "attrs": attrs, "pts": [{"x": x, "y": y} for x, y in e["pts"]]}
    if e.get("bulges"):
        out["bulges"] = e["bulges"]
    return out


def point(i, layer_id, name, xy):
    return {"kind": "point", "id": i, "layerId": layer_id, "attrs": {"Ad": name}, "p": {"x": X0 + xy[0], "y": Y0 + xy[1]}}


ENTITIES = [street(e) for e in ROADS["edges"]]
SCHOOL_IDS = list(range(11, 11 + len(SCHOOLS)))
ADDRESS_IDS = list(range(21, 21 + len(ADDRESSES)))
ENTITIES += [point(i, "okul", n, xy) for i, (n, xy) in zip(SCHOOL_IDS, SCHOOLS)]
ENTITIES += [point(i, "adres", n, xy) for i, (n, xy) in zip(ADDRESS_IDS, ADDRESSES)]

DOCUMENT = {
    "format": "kentos.document",
    "version": 1,
    "name": "İşlem durumları: ağ analizi",
    "settings": {"srid": 5254, "lengthDecimals": DECIMALS, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
                 "workspace": "gis", "drawingFont": "barlow", "networks": [NETWORK]},
    "origin": {"x": X0, "y": Y0},
    "layers": LAYERS,
    "activeLayer": "cizim",
    "entities": ENTITIES,
    "styles": {"items": [], "categories": []},
}


def layer_id(name):
    """A new layer's id as the runner makes it: “islem-” and the name folded, lower case, other characters as “-”."""
    fold = str.maketrans("çğıöşüÇĞİÖŞÜâîû", "cgiosuCGIOSUaiu")
    out, dash = "", False
    for ch in name.translate(fold).lower():
        if "a" <= ch <= "z" or "0" <= ch <= "9":
            out += ch
            dash = False
        elif not dash:
            out += "-"
            dash = True
    return "islem-" + out


def new_layer(name, st):
    return {"id": layer_id(name), "name": name, "style": st}


ROUTES_STYLE = style("#7B1FA2", 0.5)
MATRIX_STYLE = style("#00897B", 0.18)
AREA_STYLE = style("#1976D2", 0.25, fill="#1976D233")
LINES_STYLE = style("#1565C0", 0.35)


def cost_text(v, c):
    """A cost as written: the length with the project's decimals, the others with two; nothing for no way."""
    return "" if v == INF or v is None or not math.isfinite(v) else shown(v, DECIMALS if c == 0 else 2)


def check_no_tie(v, d, what):
    """
    The display rule rounds to seven decimals first, so a half at the digits shown (5.025 at two) is written alike
    however it was summed; only a value at a half of the seventh decimal could differ: keep the cases clear of those.
    """
    scaled = abs(v) * 10**7
    assert abs(scaled - math.floor(scaled) - 0.5) > 1e-3, f"{what}: {v} is a half at the seventh decimal"


def proc_case(cid, title, tool, values, expect, selection=None):
    out = {"id": cid, "title": title, "document": "network.kcad", "run": {"tool": tool}, "values": values, "expect": expect}
    if selection is not None:
        out["selection"] = selection
    return out


LAY = lambda i: {"scope": "layer", "layerId": i}  # noqa: E731
FAR_NOTE = "1 {} ağa 100.00 m içinde değil (ya da nokta değil); alınmadı."


def build():
    net = Net(ROADS)
    loc = lambda xy: net.locate((X0 + xy[0], Y0 + xy[1]), REACH)  # noqa: E731
    schools = [loc(xy) for _, xy in SCHOOLS]
    addresses = [loc(xy) for _, xy in ADDRESSES]
    assert all(schools) and all(addresses[:3]) and addresses[3] is None
    found = addresses[:3]
    cases = []

    # En yakın tesis: each address's two nearest schools by Süre, from the address.
    def closest(cid, title, k, c, reverse, cutoff, extra):
        rows = run_nearest(net, found, schools, k, cutoff, c, reverse, [], True)
        shapes, table = [], []
        for i, row in enumerate(rows):
            for rank, f in enumerate(row):
                totals = [net.spans_cost(f["spans"], x) for x in range(len(COSTS))]
                for x, t in enumerate(totals):
                    if math.isfinite(t):
                        check_no_tie(t, DECIMALS if x == 0 else 2, f"{cid} {i} {rank} {x}")
                values = [cost_text(t, x) for x, t in enumerate(totals)]
                incident, facility = ADDRESSES[i][0], SCHOOLS[f["target"]][0]
                table.append([incident, facility, str(rank + 1), *values])
                attrs = {"Olay": incident, "Tesis": facility, "Sıra": str(rank + 1), "Ağ": "Yollar", "Maliyet": COSTS[c], **dict(zip(COSTS, values))}
                shapes.append({"kind": "polyline", "layerId": layer_id("En yakın tesis"), "attrs": attrs, "parts": 1, "holes": 0, "length": totals[0]})
        none = sum(1 for r in rows if not r)
        log = [FAR_NOTE.format("olay")]
        if none:
            log.append(f"{none} olaya {'üst sınır içinde ' if cutoff is not None else ''}ulaşılabilen tesis yok.")
        expect = {
            "status": "ok",
            "summary": f"3 olay için {len(table)} tesis bulundu; {len(shapes)} yol “Yollar” ağında {COSTS[c]} ile yazıldı.",
            "undo": "En yakın tesis",
            "layers": [new_layer("En yakın tesis", ROUTES_STYLE)],
            "addedShapes": shapes,
            "log": [{"level": "warn", "text": t} for t in log],
            "outputs": {"table": {"columns": ["Olay", "Tesis", "Sıra", *COSTS], "rows": table}},
        }
        values = {"network": {"network": "yollar", "cost": COSTS[c]}, "incidents": LAY("adres"), "facilities": LAY("okul"), "count": k, **extra}
        cases.append(proc_case(cid, title, "network.closestFacility", values, expect))

    closest("closest-time", "En yakın tesis: her adrese en yakın iki okul süreyle; uzak adres söylenir", 2, 1, False, None, {})
    closest("closest-reverse", "En yakın tesis: okullardan adreslere, 300 m içinde, uzunlukla", 3, 0, True, 300.0, {"cutoff": 300, "direction": "fromFacility"})

    # Maliyet matrisi: from the schools to the addresses by length.
    rows = run_nearest(net, schools, found, None, None, 0, False, [], False)
    table = []
    for i, row in enumerate(rows):
        for rank, f in enumerate(row):
            check_no_tie(f["cost"], DECIMALS, f"matrix {i} {rank}")
            table.append([SCHOOLS[i][0], ADDRESSES[f["target"]][0], str(rank + 1), cost_text(f["cost"], 0)])
    cases.append(proc_case(
        "matrix", "Maliyet matrisi: okullardan adreslere uzunlukla; tablo, çizgi yok", "network.odMatrix",
        {"network": {"network": "yollar", "cost": "Uzunluk"}, "origins": LAY("okul"), "destinations": LAY("adres")},
        {"status": "ok", "summary": f"3 başlangıçtan 3 varışa {len(table)} satır (“Yollar” ağında Uzunluk).", "undo": None, "layers": [], "addedShapes": [],
         "log": [{"level": "warn", "text": FAR_NOTE.format("varış")}],
         "outputs": {"table": {"columns": ["Başlangıç", "Varış", "Sıra", "Uzunluk"], "rows": table}}}))
    # The nearest one each, with straight lines.
    shapes, table = [], []
    for i, row in enumerate(rows):
        for rank, f in enumerate(row[:1]):
            value = cost_text(f["cost"], 0)
            table.append([SCHOOLS[i][0], ADDRESSES[f["target"]][0], "1", value])
            a, b = SCHOOLS[i][1], ADDRESSES[f["target"]][1]
            attrs = {"Başlangıç": SCHOOLS[i][0], "Varış": ADDRESSES[f["target"]][0], "Sıra": "1", "Ağ": "Yollar", "Maliyet": "Uzunluk", "Uzunluk": value}
            shapes.append({"kind": "line", "layerId": layer_id("Maliyet matrisi"), "attrs": attrs, "parts": 1, "holes": 0, "length": math.hypot(b[0] - a[0], b[1] - a[1])})
    cases.append(proc_case(
        "matrix-lines", "Maliyet matrisi: her okula en yakın adres, düz çizgilerle", "network.odMatrix",
        {"network": {"network": "yollar", "cost": "Uzunluk"}, "origins": LAY("okul"), "destinations": LAY("adres"), "count": 1, "lines": True},
        {"status": "ok", "summary": f"3 başlangıçtan 3 varışa {len(table)} satır (“Yollar” ağında Uzunluk).", "undo": "Maliyet matrisi",
         "layers": [new_layer("Maliyet matrisi", MATRIX_STYLE)], "addedShapes": shapes,
         "log": [{"level": "warn", "text": FAR_NOTE.format("varış")}],
         "outputs": {"table": {"columns": ["Başlangıç", "Varış", "Sıra", "Uzunluk"], "rows": table}}}))

    # Hizmet alanları: Okul A's 150 and 300 m, discs and rings; two schools apart; the lines too.
    def areas(cid, title, facilities, breaks, ring, separate, lines_too, names):
        got = run_area(net, [schools[i] for i in facilities], breaks, 0, False, separate, [], 20.0, shapes=True)
        key = "ring" if ring else "disc"
        shapes = []
        for a in got["areas"]:
            f = a["facility"]
            tesis = ", ".join(names) if f is None else names[f]
            start = breaks[a["band"] - 1] if ring and a["band"] > 0 else 0.0
            attrs = {"Ağ": "Yollar", "Maliyet": "Uzunluk", "Tesis": tesis, "Başlangıç": shown(start, DECIMALS), "Bitiş": shown(breaks[a["band"]], DECIMALS)}
            shapes.append({"kind": "polygon", "layerId": layer_id("Hizmet alanı"), "attrs": attrs, "parts": a[key + "Parts"], "holes": a[key + "Holes"], "area": a[key]})
        new_layers = [new_layer("Hizmet alanı", AREA_STYLE)]
        if lines_too:
            new_layers.append(new_layer("Hizmet alanı çizgileri", LINES_STYLE))
            plain = lambda v: ("%.6f" % v).rstrip("0").rstrip(".")  # noqa: E731
            # The lines in the core's order: by facility, band, piece and where they start (the reference sorts them so).
            for line in got["lines"]:
                f = line["facility"]
                tesis = ", ".join(names) if f is None else names[f]
                lo = breaks[line["band"] - 1] if line["band"] > 0 else 0.0
                attrs = {"Ağ": "Yollar", "Maliyet": "Uzunluk", "Tesis": tesis, "Aralık": f"{plain(lo)}–{plain(breaks[line['band']])}"}
                shapes.append({"kind": "polyline", "layerId": layer_id("Hizmet alanı çizgileri"), "attrs": attrs, "parts": 1, "holes": 0, "length": abs(line["b"] - line["a"])})
        n_areas = len(got["areas"])
        roads = f" ({len(got['lines'])} yol)" if lines_too else ""
        summary = f"{len(facilities)} tesisin {len(breaks)} aralıkta {n_areas} hizmet alanı yazıldı{roads}; “Yollar” ağında Uzunluk."
        values = {"network": {"network": "yollar", "cost": "Uzunluk"}, "facilities": {"scope": "selection"}, "breaks": " ".join(("%g" % b) for b in breaks),
                  "shape": "ring" if ring else "disc", "merged": not separate, "trim": 20, "lines": lines_too}
        cases.append(proc_case(cid, title, "network.serviceAreas", values,
                               {"status": "ok", "summary": summary, "undo": "Hizmet alanları", "layers": new_layers, "addedShapes": shapes, "log": [],
                                "outputs": {"count": n_areas}},
                               selection=[SCHOOL_IDS[i] for i in facilities]))

    areas("areas-discs", "Hizmet alanları: Okul A'dan 150 ve 300 m, diskler", [0], [150.0, 300.0], False, False, False, ["Okul A"])
    areas("areas-rings", "Hizmet alanları: aynı aralıklar halka olarak", [0], [150.0, 300.0], True, False, False, ["Okul A"])
    areas("areas-apart", "Hizmet alanları: iki okul ayrı, 200 m; ulaşılan yollarla", [0, 1], [200.0], False, True, True, ["Okul A", "Okul B"])

    # Refusals.
    cases.append(proc_case("refuse-network", "Projede olmayan ağ adı verilirse çalışmaz", "network.closestFacility",
                           {"network": {"network": "ag-9", "cost": "Uzunluk"}, "incidents": LAY("adres"), "facilities": LAY("okul")},
                           {"status": "invalid", "issues": [{"param": "network", "message": "“Ağ”: “ag-9” ağı projede yok; Ağlar penceresinden tanımlayın ya da başka ağ seçin."}]}))
    cases.append(proc_case("refuse-cost", "Ağda olmayan maliyet verilirse çalışmaz", "network.odMatrix",
                           {"network": {"network": "yollar", "cost": "Yakıt"}, "origins": LAY("okul"), "destinations": LAY("adres")},
                           {"status": "invalid", "issues": [{"param": "network", "message": "“Ağ”: “Yakıt” maliyeti “Yollar” ağında yok."}]}))
    cases.append(proc_case("refuse-breaks", "Aralıklar artan olmazsa çalışmaz", "network.serviceAreas",
                           {"network": {"network": "yollar", "cost": "Uzunluk"}, "facilities": LAY("okul"), "breaks": "300 150"},
                           {"status": "invalid", "issues": [{"message": "“Aralıklar” artan, sıfırdan büyük sayılar olmalı (en çok 10), aralarında boşluk: 5 10 15."}]}))
    cases.append(proc_case("refuse-off-network", "Ağın üstünde tesis yoksa çalışmaz", "network.serviceAreas",
                           {"network": {"network": "yollar", "cost": "Uzunluk"}, "facilities": {"scope": "selection"}, "breaks": "100"},
                           {"status": "error", "message": "Ağın üstünde tesis yok; noktaların ağa yakın olduğunu ya da Arama uzaklığını denetleyin."},
                           selection=[ADDRESS_IDS[3]]))

    active = {"scope": "layer", "layerId": "cizim"}
    first = {"network": "yollar", "cost": "Uzunluk"}
    tools = {
        "network.closestFacility": {"network": first, "incidents": active, "facilities": active, "count": 1, "cutoff": None, "direction": "toFacility", "reach": 100,
                                    "layer": {"newName": "En yakın tesis"}},
        "network.odMatrix": {"network": first, "origins": active, "destinations": active, "count": None, "cutoff": None, "reach": 100, "lines": False,
                             "layer": {"newName": "Maliyet matrisi"}},
        "network.serviceAreas": {"network": first, "facilities": active, "breaks": "500 1000 1500", "direction": "from", "shape": "disc", "merged": True, "trim": 50,
                                 "reach": 100, "layer": {"newName": "Hizmet alanı"}, "lines": False, "linesLayer": {"newName": "Hizmet alanı çizgileri"}},
    }
    defaults = {"lengthDecimals": DECIMALS, "areaDecimals": 2, "angleUnit": "grad", "plotScale": 1000, "drawingFont": "barlow", "activeLayer": "cizim",
                "measureHeightMm": 2, "networks": [NETWORK]}
    return {
        "format": "kentos.processing-cases",
        "version": 1,
        "tolerance": 1e-6,
        # The areas are shapely's (512 segments a quarter circle) against the core's exact arcs.
        "measureTolerance": 0.05,
        "documents": {"network.kcad": {"defaults": defaults, "tools": tools}},
        "cases": cases,
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true", help="compare with the files instead of writing them")
    args = ap.parse_args()
    outs = [(CASES_OUT, json.dumps(build(), ensure_ascii=False, indent=1) + "\n"), (DOC_OUT, json.dumps(DOCUMENT, ensure_ascii=False, indent=1) + "\n")]
    if args.check:
        bad = [p for p, t in outs if not p.exists() or p.read_text(encoding="utf-8") != t]
        for p in bad:
            print(f"{p.relative_to(ROOT)} is not what this script writes: run it without --check and read the difference.")
        if bad:
            return 1
        print("network processing cases match")
        return 0
    for p, t in outs:
        p.write_text(t, encoding="utf-8")
        print(f"{p.relative_to(ROOT)} written")
    return 0


if __name__ == "__main__":
    sys.exit(main())
