"""KentOS Yol from Python (plan/road.md Y2.3, docs/adr/0900 §10–§11).

A road project's terrain surfaces are its settings, made from the drawing: the points and breaklines of layers and
objects, a boundary and voids. Their triangles are made by the same core as the desktop's (``kentos.cdt/1``) every
time they are asked for, so a surface never disagrees with its sources. The commands are ``kentos.cad.road.terrains``'s
(``road.terrains.create``, ``.edit``, ``.query``); these are their shorter forms:

- :func:`surfaces`: every surface as its sources give it now: state (``current``, ``stale``, ``new``, ``unbuildable``),
  digest, the sources it could not use, figures;
- :func:`create`: a new surface (Yüzey oluştur), accepted;
- :func:`accept`: a surface made again from its changed sources, accepted;
- :func:`heights`: the heights at places (none outside and in a void, never extrapolated);
- :func:`section`: the ground line along a line: where it crosses the triangles' edges, with stations;
- :func:`contours`: the contour lines every ``interval`` metres written on a layer (Eş yükselti eğrileri);
- :func:`paste`: another surface pasted into a surface (Yüzey yapıştır, docs/adr/3920): its triangles come in as they
  are, the surface's records under them go, the rest is triangulated round them.

Points are ``(east, north)`` in metres in the project's plane. A refusal raises the SDK's command errors in the apps'
words (``not_a_road_project``, ``terrain_unbuildable`` …).
"""

from __future__ import annotations

import dataclasses
from collections.abc import Iterable, Sequence
from typing import Any

from .cad.document import Document
from .cad.road import terrains as _terrains
from .cad._runtime import UNSET
from .cad.types import (
    PasteSurfaceEdit,
    SurfaceSources,
    TerrainHeight,
    TerrainReport,
    TerrainSectionPoint,
    TerrainsContoured,
    TerrainsWritten,
)

Point = Sequence[float]


def _sources(layers: Iterable[str] | None, uids: Iterable[str] | None) -> SurfaceSources:
    return SurfaceSources(layers=list(layers or []), uids=list(uids or []))


def surfaces(doc: Document) -> list[TerrainReport]:
    """Every surface of the road project as its sources give it now."""
    return list(_terrains.query(doc).surfaces)


def create(
    doc: Document,
    name: str,
    *,
    point_layers: Iterable[str] | None = None,
    point_uids: Iterable[str] | None = None,
    breakline_layers: Iterable[str] | None = None,
    breakline_uids: Iterable[str] | None = None,
    boundary: str | None = None,
    voids: Iterable[str] = (),
    max_edge: float | None = None,
    tolerance: float | None = None,
    id: str | None = None,
) -> TerrainsWritten:
    """Makes a surface of the drawing's points and breaklines and keeps its definition, accepted (Yüzey oluştur)."""
    args: dict[str, Any] = {"name": name, "voids": list(voids)}
    if point_layers or point_uids:
        args["points"] = _sources(point_layers, point_uids)
    if breakline_layers or breakline_uids:
        args["breaklines"] = _sources(breakline_layers, breakline_uids)
    for key, value in (("boundary", boundary), ("max_edge", max_edge), ("tolerance", tolerance), ("id", id)):
        if value is not None:
            args[key] = value
    return _terrains.create(doc, **args)


def accept(doc: Document, id: str) -> TerrainsWritten:
    """Accepts the surface ``id`` as its sources give it now."""
    return _terrains.edit(doc, operation="accept", id=id)


def remove(doc: Document, id: str) -> TerrainsWritten:
    """Takes the surface ``id`` out of the project."""
    return _terrains.edit(doc, operation="remove", id=id)


def heights(doc: Document, surface: str, places: Sequence[Point]) -> list[TerrainHeight]:
    """The heights of ``surface`` (its id or name) at ``places``."""
    return list(_terrains.query(doc, surface=surface, at=[(float(p[0]), float(p[1])) for p in places]).heights or [])


def section(doc: Document, surface: str, line: Sequence[Point]) -> list[TerrainSectionPoint]:
    """The ground line of ``surface`` along ``line`` (two places or more)."""
    return list(_terrains.query(doc, surface=surface, along=[(float(p[0]), float(p[1])) for p in line]).section or [])


def contours(
    doc: Document, surface: str, interval: float, layer: str, *, base: float | None = None, major_every: int | None = None
) -> TerrainsContoured:
    """Writes ``surface``'s contour lines every ``interval`` metres on ``layer`` (one undo step)."""
    args: dict[str, Any] = {"surface": surface, "interval": float(interval), "layer_id": layer}
    if base is not None:
        args["base"] = float(base)
    if major_every is not None:
        args["major_every"] = int(major_every)
    return _terrains.contours(doc, **args)


def paste(doc: Document, into: str, surface: str) -> TerrainsWritten:
    """Pastes the surface ``surface`` into the surface ``into`` (their ids), at the end of its edits (Yüzey yapıştır,
    docs/adr/3920): where ``surface``'s triangles lie, ``into``'s records made so far go and those triangles come in as
    they are. Written with the definition; ``into`` is out of date until accepted. A paste that comes back to ``into``,
    names no surface or one that is not made is written and said (the report's ``left``: ``cycle``, ``no_surface``,
    ``unbuildable``); one of ``into`` itself is refused (``invalid_terrain``)."""
    road = doc.settings.road
    defined = next((d for d in (road.surfaces if road else []) if d.id == into), None)
    if defined is None:
        return _terrains.edit(doc, operation="accept", id=into)  # refused in the apps' words: unknown_terrain
    edits = [*(defined.edits or []), PasteSurfaceEdit(terrain=surface)]
    return _terrains.edit(doc, operation="set", surface=dataclasses.replace(defined, edits=edits, built=UNSET))


__all__ = ["accept", "contours", "create", "heights", "paste", "remove", "section", "surfaces"]
