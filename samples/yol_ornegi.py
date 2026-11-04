"""A road project for review (plan/road.md Y26): an 860 m rural road in Sivas over a hill and a stream valley,
built through KentOS Yol's product commands, the same calls the desktop's windows make.

Run from the repository root:

    .run/py/bin/python samples/yol_ornegi.py

It writes ``samples/yol-ornegi.kcad``: open it with ``kentos-yol`` (or ``kentos-cad``). Inside:

- **Arazi:** about 1 300 surveyed points (20 m grid, small jitter) on *Zemin noktaları*, the surface *Mevcut zemin*
  from them, contours every metre (every fifth a major one).
- **Güzergâh:** the axis *Ana yol* (a 300 m curve with 60 m clothoids), km marks; the profile *Proje kotu* (three
  parabolic vertical curves); the superelevation *Ana yol deveri* (6 % on the curve).
- **Enkesit:** the template *Tip 1* (3.50 m lanes, 1.00 m shoulders, a ditch in the cut) with a four-layer pavement
  on the fill slope's edge; the sample line set *Enkesitler* every 20 m.
- **Koridor:** *Ana yol koridoru* and its slope plan.
- **Kübaj:** *Formasyon–Bitkisel Toprak* (30 cm topsoil, swell 1.20, shrink 0.90) and *Yol–Zemin* side by side; the
  pavement layers' volumes; the mass haul diagram *Brükner* with a 100 m free haul and a waste site at the end.

The numbers (thicknesses, slopes, factors) are examples for review, not a standard's: KentOS fills none in.
Coordinates: TUREF / TM36 (Sivas, plate 58); heights orthometric (TUDKA-99).
"""

from __future__ import annotations

import math
import random
from pathlib import Path

import kentos.cad as cad
from kentos.cad import Document

HERE = Path(__file__).resolve().parent
OUT = HERE / "yol-ornegi.kcad"

# Where the road lies: a corner of the survey in TM36, the survey 900 × 520 m.
X0, Y0 = 585_000.0, 4_400_000.0
WIDTH, DEPTH, STEP = 900.0, 520.0, 20.0


def ground(x: float, y: float) -> float:
    """The terrain: rising to the north-east, a hill in the middle, a stream valley crossing from the north."""
    u, v = x - X0, y - Y0
    base = 1278.0 + 0.012 * u + 0.018 * v
    hill = 9.0 * math.exp(-(((u - 420.0) / 140.0) ** 2 + ((v - 330.0) / 110.0) ** 2))
    stream = -6.5 * math.exp(-((u - 0.55 * v - 560.0) / 45.0) ** 2)
    ripple = 0.6 * math.sin(u / 37.0) * math.cos(v / 53.0)
    return base + hill + stream + ripple


def camel(name: str) -> str:
    head, *rest = name.split("_")
    return head + "".join(w[:1].upper() + w[1:] for w in rest)


def run(command: str, doc: Document, **kw):
    """The command with its input written as JSON (the catalog's shape), through its typed wrapper; a refusal raises."""
    wrapper = cad.command(command)
    return wrapper.execute_input(doc, wrapper.input_type.from_json({camel(k): v for k, v in kw.items()}))


def main() -> None:
    doc = Document.new("Yol örneği", workspace="cad", cad_work="road", province=58)

    ids: dict[str, str] = {}
    for key, name, color in [
        ("zemin", "Zemin noktaları", "#8a8f98"),
        ("esyukselti", "Eş yükselti", "#b07d2b"),
        ("km", "Km", "#c0392b"),
        ("koridor", "Koridor planı", "#1565c0"),
        ("enkesit", "Enkesitler", "#2e7d32"),
    ]:
        ids[key] = run("cad.layers.edit", doc, operation="add", name=name, color=color).layer

    # ── Arazi: the survey, the surface, the contours ────────────────────────────────────────────────────────────
    rnd = random.Random(58)
    nx, ny = int(WIDTH / STEP), int(DEPTH / STEP)
    n = 0
    for i in range(nx + 1):
        for j in range(ny + 1):
            x = X0 + i * STEP + (rnd.uniform(-4.0, 4.0) if 0 < i < nx else 0.0)
            y = Y0 + j * STEP + (rnd.uniform(-4.0, 4.0) if 0 < j < ny else 0.0)
            run("cad.point.create", doc, layer_id=ids["zemin"], p={"x": x, "y": y}, z=round(ground(x, y), 3))
            n += 1
    terrain = run("road.terrains.create", doc, name="Mevcut zemin", points={"layers": [ids["zemin"]]})
    surface = terrain.surface.id
    run("road.terrains.contours", doc, surface=surface, interval=1.0, layer_id=ids["esyukselti"], major_every=5)

    # ── Güzergâh: the axis, its km, the profile, the superelevation ──────────────────────────────────────────────
    made = run(
        "road.alignments.create",
        doc,
        name="Ana yol",
        pis=[
            {"id": "a", "x": X0 + 60.0, "y": Y0 + 90.0},
            {"id": "p1", "x": X0 + 470.0, "y": Y0 + 330.0, "radius": 300.0, "spiralIn": 60.0, "spiralOut": 60.0},
            {"id": "b", "x": X0 + 860.0, "y": Y0 + 250.0},
        ],
    )
    axis = made.alignment.id
    run("road.alignments.label", doc, alignment=axis, layer_id=ids["km"], every=20.0, major=100.0)
    length = made.alignment.length

    # The red line: the ground along the axis every 20 m, smoothed by three vertical curves above it.
    at = [s for s in range(0, int(length), 20)] + [length]
    places = run("road.alignments.query", doc, alignment=axis, stations=[float(s) for s in at]).at
    heights = run("road.terrains.query", doc, surface=surface, at=[[p.x, p.y] for p in places]).heights
    z = {s: h.z for s, h in zip(at, heights)}

    def near(s: float) -> float:
        return z[min(at, key=lambda t: abs(t - s))]

    pvis = [
        {"id": "a", "station": 0.0, "elevation": round(near(0.0) + 0.6, 2)},
        {"id": "v1", "station": 220.0, "elevation": round(near(220.0) + 1.2, 2), "length": 120.0},
        {"id": "v2", "station": 470.0, "elevation": round(near(470.0) - 2.5, 2), "length": 160.0},
        {"id": "v3", "station": 680.0, "elevation": round(near(680.0) + 2.0, 2), "length": 120.0},
        {"id": "b", "station": round(length, 3), "elevation": round(near(length) + 0.5, 2)},
    ]
    profile = run("road.profiles.create", doc, name="Proje kotu", alignment=axis, surface=surface, pvis=pvis).profile.id
    sup = run(
        "road.superelevations.edit",
        doc,
        operation="set",
        name="Ana yol deveri",
        alignment=axis,
        normal_left=-2.5,
        normal_right=-2.5,
        method={"method": "curves", "curves": [{"pi": "p1", "rate": 6.0, "runoff": 50.0, "onTangent": 0.3}]},
    ).superelevation.id

    # ── Enkesit: the template with its pavement, the sample line set ─────────────────────────────────────────────
    side = {
        "components": [{"kind": "lane", "width": 3.5, "slope": -2.5}, {"kind": "shoulder", "width": 1.0, "slope": -4.0}],
        "cut": 1.0,
        "fill": 1.5,
        "ditch": {"fore": 2.0, "depth": 0.6, "bottom": 0.5},
    }
    pavement = {
        "layers": [
            {"name": "Aşınma", "thickness": 0.05, "material": "BSK-A"},
            {"name": "Binder", "thickness": 0.07, "material": "BSK-B"},
            {"name": "Plentmiks temel", "thickness": 0.20, "material": "PMT"},
            {"name": "Alttemel", "thickness": 0.25, "material": "AT"},
        ],
        "edge": {"kind": "slope", "h": 1.5},
    }
    template = run("road.templates.edit", doc, operation="set", name="Tip 1", left=side, pavement=pavement).template.id
    sections = run(
        "road.sections.edit",
        doc,
        operation="set",
        name="Enkesitler",
        alignment=axis,
        profile=profile,
        template=template,
        surface=surface,
        superelevation=sup,
        left=30.0,
        right=30.0,
        stations={"method": "range", "from": 0.0, "to": round(length, 3), "every": 20.0, "geometry": True},
    ).set.id

    # ── Koridor ─────────────────────────────────────────────────────────────────────────────────────────────────
    corridor = run(
        "road.corridors.edit",
        doc,
        operation="set",
        name="Ana yol koridoru",
        alignment=axis,
        profile=profile,
        surface=surface,
        superelevation=sup,
        left=30.0,
        right=30.0,
        tangent=20.0,
        curve=10.0,
        regions=[{"from": 0.0, "to": round(length, 3), "template": template}],
    ).corridor.id
    run("road.corridors.draw", doc, corridor=corridor, layer_id=ids["koridor"])
    run(
        "road.sections.draw",
        doc,
        set=sections,
        layer_id=ids["enkesit"],
        at=[X0 + 20.0, Y0 - 260.0],
        stations=[100.0, 200.0, 300.0, 400.0, 500.0, 600.0],
        columns=3,
    )

    # ── Kübaj ve Brükner ────────────────────────────────────────────────────────────────────────────────────────
    strip = [{"from": 0.0, "to": round(length, 3), "depth": 0.3}]
    factors = [{"from": 0.0, "to": round(length, 3), "cut": 1.2, "fill": 0.9}]
    volume = run(
        "road.volumes.edit",
        doc,
        operation="set",
        name="Formasyon–Bitkisel Toprak",
        corridor=corridor,
        method="endArea",
        rule="formationTopsoil",
        strip=strip,
        factors=factors,
    ).volume.id
    run(
        "road.volumes.edit",
        doc,
        operation="set",
        name="Yol–Zemin",
        corridor=corridor,
        method="endArea",
        rule="roadGround",
        factors=factors,
    )
    # The cut left over goes to a waste site past the end of the road.
    run(
        "road.hauls.edit",
        doc,
        operation="set",
        name="Brükner",
        volume=volume,
        free=100.0,
        sites=[{"name": "Döküm sahası", "kind": "waste", "station": round(length, 3), "volume": 12000.0}],
    )

    # The survey points stay, hidden: the surface is made of them, the plan reads better without their symbols.
    run("cad.layers.set", doc, operation="visible", layer=ids["zemin"], value=False)

    saved = doc.save(OUT)
    report = run("road.volumes.query", doc, volume=volume).volumes[0]
    print(f"{saved.path} ({saved.bytes} bayt): {n} nokta, aks {length:.1f} m")
    print(f"  {report.name}: kazı {report.cut:.0f} m³, dolgu {report.fill:.0f} m³, bitkisel toprak {report.strip:.0f} m³")
    for layer in report.layers:
        print(f"  {layer.name}: {layer.volume:.1f} m³")


if __name__ == "__main__":
    main()
