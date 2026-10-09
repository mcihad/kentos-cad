#!/usr/bin/env python3
"""The network analysis' scene (docs/adr/0209): fixtures/interaction/v1/networks.kcad, written without KentOS code.

A small town around (487000, 4420000) in TUREF / TM30: a street grid of 120 m blocks (vertical streets at x = −180,
−60, 60 and 180, horizontal ones at y = −120, 0 and 120), each street between two crossings its own object so their
ends meet; Atatürk Caddesi (y = 0) at 50 km/h, the side streets at 30; Kışla Sokağı (x = 60 from y = 0 to 120) one
way to the north (drawn that way, `yon` FT); Pazar Sokağı (x = −60 from y = −120 to 0) closed (`durum` kapalı); the
street on y = 120 between x = −60 and 60 a curved one (a bulge of 0.2). Schools and a fire station beside the
streets. A water network north of Atatürk Caddesi (y = 6), drawn from the depot in the east with the flow to the
west, with two branches (x = 66 north, x = −54 south), valves along it and one of them closed. The project's networks:
Yollar (the streets: direction by `yon`, Süre from `hiz`, closed by `durum`) and İçme suyu (the pipes, the depot a
source, the valves; drawn with the flow). The interaction traces (network-route.json, network-service-area.json,
network-trace.json) and both apps' pictures play it.

    python3 scripts/fixtures/network_scene.py [--check]
"""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/interaction/v1/networks.kcad"
X0, Y0 = 487000.0, 4420000.0


def style(color, weight=0.25, **more):
    return {"color": color, "lineType": "continuous", "lineWeight": weight, **more}


def layer(i, name, st):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True, "style": st, "children": []}


BESIDE = {"placement": "beside", "size": 9}
LAYERS = [
    layer("cizim", "Çizim", style("fg")),
    layer("yol", "Yol ekseni", style("#8C9AAA", 0.35)),
    layer("okul", "Okul", style("#E5484D", point={"symbol": "triangle", "size": 9}, label=BESIDE)),
    layer("itfaiye", "İtfaiye", style("#F76B15", point={"symbol": "cross", "size": 9}, label=BESIDE)),
    layer("su", "Su hattı", style("#0EA5A0", 0.35)),
    layer("vana", "Vana", style("#0EA5A0", point={"symbol": "ring", "size": 7}, label=BESIDE)),
    layer("depo", "Depo", style("#0B7A75", point={"symbol": "triangle", "size": 10}, label=BESIDE)),
]

entities = []


def add(kind, layer_id, attrs, **geom):
    entities.append({"kind": kind, "id": len(entities) + 1, "layerId": layer_id, "attrs": attrs, **geom})


def place(layer_id, name, x, y, **attrs):
    entities.append({"kind": "point", "id": len(entities) + 1, "layerId": layer_id, "attrs": {"Ad": name, **attrs}, "label": name, "p": p(x, y)})


def p(x, y):
    return {"x": X0 + x, "y": Y0 + y}


def street(a, b, name, speed, bulge=None, **more):
    geom = {"pts": [p(*a), p(*b)]}
    if bulge:
        geom["bulges"] = [bulge]
    add("polyline", "yol", {"Ad": name, "hiz": str(speed), **more}, **geom)


XS = [-180, -60, 60, 180]
YS = [-120, 0, 120]
NAMES_X = {-180: "Lale Sokağı", -60: "Pazar Sokağı", 60: "Kışla Sokağı", 180: "Çınar Sokağı"}
NAMES_Y = {-120: "Okul Caddesi", 0: "Atatürk Caddesi", 120: "Bahar Sokağı"}

# Horizontal streets, west to east, each block its own object.
for y in YS:
    for a, b in zip(XS, XS[1:]):
        speed = 50 if y == 0 else 30
        if y == 120 and a == -60:
            street((a, y), (b, y), NAMES_Y[y], speed, bulge=0.2)
        else:
            street((a, y), (b, y), NAMES_Y[y], speed)
# Vertical streets, south to north.
for x in XS:
    for a, b in zip(YS, YS[1:]):
        more = {}
        if x == 60 and a == 0:
            more["yon"] = "FT"
        if x == -60 and a == -120:
            more["durum"] = "kapalı"
        street((x, a), (x, b), NAMES_X[x], 30, **more)

place("okul", "Atatürk İlkokulu", -120, 124)
place("okul", "Cumhuriyet Ortaokulu", 120, -116)
place("okul", "Fen Lisesi", 184, 60)
place("itfaiye", "Merkez İtfaiye", -176, -60)

# The water network, drawn with the flow: from the depot in the east, west along y = 6; two branches.
def pipe(a, b, **more):
    add("polyline", "su", {"Malzeme": "PE100", "Çap": "110", **more}, pts=[p(*a), p(*b)])


pipe((176, 6), (66, 6))
pipe((66, 6), (-54, 6))
pipe((-54, 6), (-176, 6))
pipe((66, 6), (66, 114))
pipe((-54, 6), (-54, -114))
place("depo", "Doğu Deposu", 176, 6)
place("vana", "V-1", 120, 6, durum="açık")
place("vana", "V-2", 0, 6, durum="açık")
place("vana", "V-3", -120, 6, durum="açık")
place("vana", "V-4", 66, 60, durum="açık")
place("vana", "V-5", -54, -60, durum="kapalı")

NETWORKS = [
    {
        "id": "yollar",
        "name": "Yollar",
        "kind": "road",
        "edges": [{"layer": "yol"}],
        "connect": "ends",
        "tolerance": 0.01,
        "direction": {"kind": "field", "field": "yon", "forward": ["FT"], "backward": ["TF"], "closed": ["N"]},
        "costs": [{"name": "Süre", "kind": "speed", "field": "hiz", "speed": 50.0}],
        "closed": "durum = 'kapalı'",
    },
    {
        "id": "icme-suyu",
        "name": "İçme suyu",
        "kind": "utility",
        "edges": [{"layer": "su"}],
        "junctions": [{"layer": "depo", "role": "source"}, {"layer": "vana", "role": "valve", "closed": "durum = 'kapalı'"}],
        "connect": "ends",
        "tolerance": 0.01,
        "direction": {"kind": "digitized"},
    },
]

DOCUMENT = {
    "format": "kentos.document",
    "version": 1,
    "name": "Etkileşim izi: Ağ analizi",
    "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
                 "workspace": "gis", "drawingFont": "barlow", "networks": NETWORKS},
    "origin": {"x": X0, "y": Y0},
    "layers": LAYERS,
    "activeLayer": "cizim",
    "entities": entities,
    "styles": {"items": [], "categories": []},
}


def main():
    text = json.dumps(DOCUMENT, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} is not what this script writes: run it without --check and read the difference.")
            return 1
        print(f"{OUT.relative_to(ROOT)} matches")
        return 0
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} written: {len(entities)} objects")
    return 0


if __name__ == "__main__":
    sys.exit(main())
