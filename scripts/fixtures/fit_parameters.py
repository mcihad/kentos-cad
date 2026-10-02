#!/usr/bin/env python3
"""Independent reference of Vektör oturtma's Parametrelerle (docs/adr/0156 §7).

Writes fixtures/fit/v1/parameters.json from the rule alone, with Python's
standard library and no KentOS code. Both platforms turn every case's
numbers into the transform's linear part with the shared core
(`ops::fit::scale_turn`, WASM `fitScaleTurn`) and must find the same four
numbers within the file's tolerance.

The rule: east (Y, sağa) is scaled by `east`, north (X, yukarı) by `north`,
then the plane turns by `rotation` radians counter-clockwise (as the fit's
own rotation, atan2(b, a), turns). The linear part in the core's `Affine`
order [a, b, c, d] (x′ = a·x + c·y, y′ = b·x + d·y) is

    a = east·cos θ,  b = east·sin θ,  c = −north·sin θ,  d = north·cos θ.

A scale below zero mirrors. With equal scales the map is a similarity:
a = d and b = −c exactly. What a person types in grads or degrees comes in
as radians (grads · π / 200, degrees · π / 180), as both platforms convert.
"""

import argparse
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "fit" / "v1" / "parameters.json"


def grad(g):
    return g * math.pi / 200


def case(name, east, north, rotation):
    c, s = math.cos(rotation), math.sin(rotation)
    return {
        "name": name,
        "east": east,
        "north": north,
        "rotation": rotation,
        "m": [east * c, east * s, -north * s, north * c],
    }


def cases():
    return [
        case("Birim: değişiklik yok", 1.0, 1.0, 0.0),
        case("Yalnız Y (doğu) ölçeği", 1.5, 1.0, 0.0),
        case("Yalnız X (kuzey) ölçeği", 1.0, 0.75, 0.0),
        case("Eşit ölçek ve 50 g: benzerlik", 1.00012, 1.00012, grad(50)),
        case("Ayrı ölçekler ve 22,2817 g", 1.0002, 0.9997, grad(22.2817)),
        case("Çeyrek tur, 100 g", 2.0, 0.5, grad(100)),
        case("Yarım tur, 200 g", 1.0, 1.0, grad(200)),
        case("Saat yönünde, −30 g", 1.25, 1.25, grad(-30)),
        case("Bir turdan büyük, 437,5 g", 0.8, 1.2, grad(437.5)),
        case("Eksi Y ölçeği: aynalar", -1.0, 1.0, grad(12.5)),
        case("Küçük dönüklük, 1 cc", 1.0, 1.0, grad(0.0001)),
        case("Derece ile 33,75°", 1.1, 0.9, 33.75 * math.pi / 180),
    ]


def build():
    return {
        "format": "kentos.fit-parameters",
        "version": 1,
        "description": "Vektör oturtma'nın Parametrelerle'si (docs/adr/0156 §7): Y (doğu) ve X (kuzey) ölçeği, sonra saat yönünün tersine dönüklük (radyan); afinin doğrusal parçası [a, b, c, d]. scripts/fixtures/fit_parameters.py yazar.",
        "tolerance": 1e-15,
        "cases": cases(),
    }


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--check", action="store_true", help="compare with the written file instead of writing it")
    args = ap.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT} güncel değil: python3 scripts/fixtures/fit_parameters.py", file=sys.stderr)
            sys.exit(1)
        print(f"{OUT}: güncel")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text)
    print(f"{OUT}: {len(json.loads(text)['cases'])} durum")


if __name__ == "__main__":
    main()
