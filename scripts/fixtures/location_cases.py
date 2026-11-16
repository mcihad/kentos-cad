#!/usr/bin/env python3
"""Independent reference of a building model's place on the map (docs/adr/8072 §3; IFC-05's first acceptance).

Writes fixtures/arch/v1/ifc/yerel-sistem.json from the rules alone, no KentOS code. A synthetic site (no real place):
the section building (fixtures/arch/v1/section-building.kcad) drawn in a local system that stands on TUREF / TM36
(EPSG:5256) turned 23.5° counter-clockwise, scaled 0.99989 and shifted to 587 312.345 E, 4 400 612.789 N. Three control
points spread round the building, known in both systems: their map coordinates written to the millimetre with a little
survey noise, as a survey gives them. The local system's plane is fitted from them by least squares as the Özel
koordinat sistemi window's Ortak noktalardan hesapla fits it (crs_definition_fit_cases.py's exact solution), with each
point's residual (fitted less surveyed). ±0.00 stands at 1285.40 m in Antalya heights (EPSG:5775); the building's
storeys 0 and 3 m over it, their walls standing on the structural slab under the floor's layers (docs/adr/8040;
section_building.py's slabs: the ground's 2 + 5 + 5 + 1 cm, the 1. kat's 2 + 5 cm).

scripts/fixtures/curved_ifc_check.py's location part reads the building's IFC written on this system
(fixtures/arch/v1/ifc/yerel-sistem.ifc, the application's `a_local_model_is_turned_and_scaled_onto_its_base`) with
IfcOpenShell: its system and map conversion against this plane, every control point carried to the map within its
residual and back, the storeys' heights over the datum, the placement chain.
"""

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from crs_definition_fit_cases import rows_of, solve, true_similarity  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "arch" / "v1" / "ifc" / "yerel-sistem.json"

# The control points: round the section building (about 26 × 9 m from its origin), in its local system.
POINTS = [("K1", (-5.0, -5.0)), ("K2", (30.0, -3.0)), ("K3", (8.0, 20.0))]
# The survey's noise on the map coordinates, metres (east, north).
NOISE = [(0.002, -0.001), (-0.001, 0.002), (0.0, -0.002)]
TRUE = {"east": 587_312.345, "north": 4_400_612.789, "rotation": 23.5, "scale": 0.999_89}


def build():
    sim = true_similarity(TRUE["east"], TRUE["north"], TRUE["rotation"], TRUE["scale"])
    rows = rows_of([p for _, p in POINTS], sim, NOISE)
    got = solve(rows, "similarity")
    points = []
    for (name, _), row, res in zip(POINTS, rows, got["residuals"]):
        points.append({"name": name, "typed": row, "local": [float(row[0]), float(row[1])],
                       "map": [float(row[2]), float(row[3])], "residual": res})
    return {
        "format": "kentos.ifc-location",
        "version": 1,
        "source": "scripts/fixtures/location_cases.py (docs/adr/8072 §3; crs_definition_fit_cases.py's solution)",
        "base": {"name": "EPSG:5256", "zone": "36"},
        "localSystem": "Şantiye",
        "survey": TRUE,
        "plane": got["plane"],
        "m0": got["m0"],
        "points": points,
        "datum": 1285.4,
        "verticalDatum": "Antalya height (EPSG:5775)",
        "storeys": [{"name": "Zemin kat", "elevation": 0.0, "wallsFrom": round(0.0 - (0.02 + 0.05 + 0.05 + 0.01), 9)},
                    {"name": "1. kat", "elevation": 3.0, "wallsFrom": round(3.0 - (0.02 + 0.05), 9)}],
        # The fitted plane against KentOS's (the fit's file's), and a point carried by IfcOpenShell against the plane.
        "tolerance": {"metres": 1e-6, "relative": 1e-12, "degrees": 1e-9},
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true", help="compare with the file instead of writing it")
    args = parser.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} kurallardan çıkanla aynı değil; betiği --check olmadan çalıştırıp farkı okuyun.")
            sys.exit(1)
        print(f"{OUT.relative_to(ROOT)} kurallardan çıkanla aynı.")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı.")


if __name__ == "__main__":
    main()
