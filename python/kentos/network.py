"""Ağ analizi from Python (docs/adr/0209 §11).

A project's networks are its settings (:func:`networks`): define one with
:func:`define` (``cad.network.define``, as Ağlar's Kaydet does) and take one
away with :func:`remove`. A network is built from the drawing as it is, as the
apps' İşlemler tools build it, and asked what the apps' tools ask:

- :func:`route`: the shortest (or cheapest) way through stops, in their order or
  the best one (En kısa yol);
- :func:`service_area`: the network within each break of facilities, its lines
  and its areas (Hizmet alanı);
- :func:`closest_facility` and :func:`od_matrix`: each incident's nearest
  facilities with their ways (En yakın tesis), a cost matrix (Maliyet matrisi);
- :func:`trace`: what is connected, downstream or upstream, or what closing the
  nearest open valves isolates (Şebeke izleme);
- :func:`check`: Denetle.

Each builds the network anew; :class:`Network` builds it once for many
questions. Points are ``(east, north)`` in the project's system; a point finds
the network within ``reach`` metres. Costs are named as the network names
them, ``Uzunluk`` (metres) first; a break, a cutoff and a total are in the
cost's unit. A refusal (a stop off the network, no way between two stops)
raises :class:`NetworkError` in the apps' words. Nothing is written: a way's
:meth:`Line.geometry` and an area's :attr:`ServiceArea.shape` go to
``kentos.cad.entities.create`` as they are. A network is the drawing's when it
was built: build it again after the drawing changes.
"""

from __future__ import annotations

import json
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import Any, Literal

from . import _native
from .cad import network as _network
from .cad.document import Document
from .cad.types import EntityGeometry, NetworkDef, NetworkDefined, PolylineEntityGeometry, Vec2

__all__ = [
    "DEFAULT_REACH",
    "LENGTH",
    "Check",
    "Line",
    "Network",
    "NetworkError",
    "Place",
    "Problem",
    "Reached",
    "Route",
    "ServiceArea",
    "ServiceAreas",
    "ServiceLine",
    "Trace",
    "Valve",
    "check",
    "closest_facility",
    "define",
    "networks",
    "od_matrix",
    "remove",
    "route",
    "service_area",
    "trace",
]

#: How far a point looks for the network, metres (the İşlemler tools' default).
DEFAULT_REACH = 100.0
#: The cost every network has: the length in metres.
LENGTH = "Uzunluk"

Point = tuple[float, float]
Reorder = Literal["none", "keepFirst", "keepFirstLast"]
TraceKind = Literal["connected", "downstream", "upstream", "isolation"]


class NetworkError(Exception):
    """A network could not be built or asked, or refused a question: why, in the apps' words."""


# ------------------------------------------------------------------ answers


@dataclass(frozen=True, slots=True)
class Line:
    """A way along the network: its vertices and the DXF bulges of its edges (arcs where it follows them)."""

    pts: list[Point]
    bulges: list[float]

    def geometry(self) -> PolylineEntityGeometry:
        """The way as a polyline for ``kentos.cad.entities.create`` (its bulges only where it has an arc)."""
        arcs = any(b != 0.0 for b in self.bulges)
        return PolylineEntityGeometry(pts=[Vec2(x=x, y=y) for x, y in self.pts], **({"bulges": list(self.bulges)} if arcs else {}))

    @staticmethod
    def _of(data: Mapping[str, Any]) -> Line:
        return Line(pts=[(p[0], p[1]) for p in data["pts"]], bulges=list(data.get("bulges", [])))


@dataclass(frozen=True, slots=True)
class Place:
    """Where a point sits on the network: its piece, how far along it, the place and its distance from the point."""

    piece: int
    offset: float
    x: float
    y: float
    d: float


@dataclass(frozen=True, slots=True)
class Route:
    """En kısa yol: the stops' order (from 0), the cost, each of the network's costs along the way (None where it
    cannot be travelled with it), the way and each leg's cost."""

    order: list[int]
    cost: float
    totals: list[float | None]
    line: Line
    legs: list[float]


@dataclass(frozen=True, slots=True)
class ServiceLine:
    """A stretch of the network in a band (0 the first break's) of a facility (None: all of them merged)."""

    facility: int | None
    band: int
    line: Line


@dataclass(frozen=True, slots=True)
class ServiceArea:
    """A band's area of a facility (None: merged), the polygon to write; None when the band reaches nothing."""

    facility: int | None
    band: int
    shape: EntityGeometry | None


@dataclass(frozen=True, slots=True)
class ServiceAreas:
    lines: list[ServiceLine]
    areas: list[ServiceArea]


@dataclass(frozen=True, slots=True)
class Reached:
    """A target reached: its place in the targets, the cost, and with a way each cost along it and the way."""

    target: int
    cost: float
    totals: list[float | None] | None
    line: Line | None


@dataclass(frozen=True, slots=True)
class Valve:
    id: int
    x: float
    y: float


@dataclass(frozen=True, slots=True)
class Trace:
    """Şebeke izleme: the objects reached and their length, their stretches; with ``isolation`` the valves to close
    and what no source feeds once they are (its objects, length and stretches)."""

    objects: list[int]
    valves: list[Valve]
    unfed: list[int]
    length: float
    unfed_length: float
    lines: list[Line]
    unfed_lines: list[Line]


@dataclass(frozen=True, slots=True)
class Problem:
    """A problem Denetle found: its kind (``detached``, ``nearMiss``, ``crossing``, ``offNetwork``, ``short``,
    ``unread``), where, its objects, and a detached part's length or an unread value's cost."""

    kind: str
    x: float
    y: float
    ids: list[int]
    value: float | None
    cost: int | None


@dataclass(frozen=True, slots=True)
class Check:
    """Denetle: the network's nodes, pieces and length, its connected parts (pieces, length) the largest first, its
    dead ends, each problem kind's count and the problems."""

    nodes: int
    pieces: int
    length: float
    parts: list[tuple[int, float]]
    dead_ends: int
    counts: dict[str, int]
    problems: list[Problem]


def _id(v: float) -> int:
    return int(v)


def _reached(row: Sequence[Mapping[str, Any]]) -> list[Reached]:
    return [
        Reached(
            target=f["target"],
            cost=f["cost"],
            totals=f.get("totals"),
            line=Line._of(f["line"]) if f.get("line") is not None else None,
        )
        for f in row
    ]


# ------------------------------------------------------------------ the network


def _call(call: Any, *args: Any) -> Any:
    try:
        return call(*args)
    except _native.HostError as e:
        code, message = e.args if len(e.args) == 2 else ("network_refused", str(e))
        raise NetworkError(message) from None


def _points(points: Sequence[Point] | Sequence[Sequence[float]], what: str) -> str:
    out = []
    for p in points:
        if len(p) != 2:
            raise NetworkError(f"{what}: her nokta (doğu, kuzey) olmalı; {p!r} yazıldı.")
        out.append([float(p[0]), float(p[1])])
    return json.dumps(out)


class Network:
    """A project's network built once from the drawing as it is now, for many questions.

    ``network`` is the network's id. :attr:`problems` says what building could
    not take (expressions that did not compile); :attr:`counts` the nodes,
    pieces, length and the objects not taken."""

    __slots__ = ("_native", "costs", "counts", "definition", "problems")

    def __init__(self, doc: Document, network: str) -> None:
        session = doc._session
        if not isinstance(session, _native.Session):
            # The desktop's console: the drawing's copy in this process.
            session = _call(_native.Session.from_bytes, doc.to_bytes())
        self._native = _call(_native.Network.build, session, network)
        s = json.loads(self._native.summary())
        #: The network's definition, as the project keeps it.
        self.definition: NetworkDef = NetworkDef.from_json(s["network"])
        #: Its costs' names, ``Uzunluk`` first.
        self.costs: list[str] = s["costs"]
        self.counts: dict[str, Any] = s["counts"]
        self.problems: list[str] = s["problems"]

    def __repr__(self) -> str:
        c = self.counts
        return f"<kentos.network.Network {self.definition.name!r}: {c['nodes']} düğüm, {c['pieces']} parça>"

    def _cost(self, cost: str) -> int:
        try:
            return self.costs.index(cost)
        except ValueError:
            raise NetworkError(f"“{cost}” maliyeti “{self.definition.name}” ağında yok; maliyetleri: {', '.join(self.costs)}.") from None

    def locate(self, at: Point, *, reach: float = DEFAULT_REACH) -> Place | None:
        """Where the network is nearest ``at`` within ``reach``, or None."""
        p = json.loads(self._native.locate(float(at[0]), float(at[1]), float(reach)))
        return None if p is None else Place(**p)

    def route(
        self,
        stops: Sequence[Point],
        *,
        cost: str = LENGTH,
        reorder: Reorder = "none",
        barriers: Sequence[Point] = (),
        reach: float = DEFAULT_REACH,
    ) -> Route:
        """En kısa yol through ``stops`` with the least ``cost``, not passing ``barriers``; ``reorder`` puts the
        stops in the best order with the first (``keepFirst``) or the first and the last (``keepFirstLast``) kept."""
        r = json.loads(_call(self._native.route, _points(stops, "duraklar"), _points(barriers, "engeller"), float(reach), self._cost(cost), reorder))
        return Route(order=r["order"], cost=r["cost"], totals=r["totals"], line=Line._of(r["line"]), legs=r["legs"])

    def service_area(
        self,
        facilities: Sequence[Point],
        breaks: Sequence[float],
        *,
        cost: str = LENGTH,
        toward: bool = False,
        separate: bool = False,
        rings: bool = False,
        trim: float = 50.0,
        barriers: Sequence[Point] = (),
        reach: float = DEFAULT_REACH,
        areas: bool = True,
    ) -> ServiceAreas:
        """Hizmet alanı: the network within each of the rising ``breaks`` of ``facilities`` (``toward``: of the way
        to them; ``separate``: each facility its own), band by band; with ``areas`` each band's area, its lines
        widened by ``trim`` metres (``rings``: without the bands inside it)."""
        a = json.loads(
            _call(
                self._native.service_area,
                _points(facilities, "tesisler"),
                [float(b) for b in breaks],
                float(reach),
                self._cost(cost),
                bool(toward),
                bool(separate),
                _points(barriers, "engeller"),
                float(trim),
                bool(rings),
                bool(areas),
            )
        )
        return ServiceAreas(
            lines=[ServiceLine(facility=x.get("facility"), band=x["band"], line=Line._of(x["line"])) for x in a["lines"]],
            areas=[
                ServiceArea(facility=x.get("facility"), band=x["band"], shape=EntityGeometry.from_json(x["shape"]) if x.get("shape") is not None else None)
                for x in a["areas"]
            ],
        )

    def closest_facility(
        self,
        incidents: Sequence[Point],
        facilities: Sequence[Point],
        *,
        count: int = 1,
        cutoff: float | None = None,
        cost: str = LENGTH,
        direction: Literal["toFacility", "fromFacility"] = "toFacility",
        barriers: Sequence[Point] = (),
        reach: float = DEFAULT_REACH,
    ) -> list[list[Reached]]:
        """En yakın tesis: for each of ``incidents`` its ``count`` nearest ``facilities`` within ``cutoff``, the way
        from the incident (``fromFacility``: from the facility); a row an incident, the cheapest first."""
        return self._nearest(incidents, facilities, count, cutoff, cost, direction == "fromFacility", barriers, reach, True)

    def od_matrix(
        self,
        origins: Sequence[Point],
        destinations: Sequence[Point],
        *,
        count: int | None = None,
        cutoff: float | None = None,
        cost: str = LENGTH,
        barriers: Sequence[Point] = (),
        reach: float = DEFAULT_REACH,
    ) -> list[list[Reached]]:
        """Maliyet matrisi: from each of ``origins`` the cost to each of ``destinations`` (the ``count`` cheapest,
        within ``cutoff``), the cheapest first; no ways."""
        return self._nearest(origins, destinations, count, cutoff, cost, False, barriers, reach, False)

    def _nearest(
        self,
        origins: Sequence[Point],
        targets: Sequence[Point],
        count: int | None,
        cutoff: float | None,
        cost: str,
        reverse: bool,
        barriers: Sequence[Point],
        reach: float,
        paths: bool,
    ) -> list[list[Reached]]:
        if count is not None and count < 1:
            raise NetworkError(f"Tesis sayısı en az 1 olmalı; {count} yazıldı.")
        rows = json.loads(
            _call(
                self._native.nearest,
                _points(origins, "başlangıçlar"),
                _points(targets, "varışlar"),
                float(reach),
                self._cost(cost),
                bool(reverse),
                _points(barriers, "engeller"),
                bool(paths),
                count,
                None if cutoff is None else float(cutoff),
            )
        )
        return [_reached(row) for row in rows]

    def trace(
        self,
        starts: Sequence[Point],
        *,
        kind: TraceKind = "connected",
        barriers: Sequence[Point] = (),
        reach: float = DEFAULT_REACH,
    ) -> Trace:
        """Şebeke izleme from ``starts``: what is ``connected``, ``downstream`` or ``upstream`` (the edges'
        directions), or with ``isolation`` the nearest open valves to close and what no source feeds once they are."""
        t = json.loads(_call(self._native.trace, _points(starts, "başlangıçlar"), _points(barriers, "engeller"), float(reach), kind))
        return Trace(
            objects=[_id(i) for i in t["objects"]],
            valves=[Valve(id=_id(v["id"]), x=v["x"], y=v["y"]) for v in t["valves"]],
            unfed=[_id(i) for i in t["unfed"]],
            length=t["length"],
            unfed_length=t["unfedLength"],
            lines=[Line._of(x) for x in t["lines"]],
            unfed_lines=[Line._of(x) for x in t["unfedLines"]],
        )

    def check(self) -> Check:
        """Denetle."""
        c = json.loads(self._native.check())
        return Check(
            nodes=c["nodes"],
            pieces=c["pieces"],
            length=c["length"],
            parts=[(p[0], p[1]) for p in c["parts"]],
            dead_ends=c["deadEnds"],
            counts={k["kind"]: k["count"] for k in c["counts"]},
            problems=[
                Problem(kind=p["kind"], x=p["x"], y=p["y"], ids=[_id(i) for i in p["ids"]], value=p.get("value"), cost=p.get("cost"))
                for p in c["problems"]
            ],
        )


# ------------------------------------------------------------------ the project's networks and one-off questions


def networks(doc: Document) -> list[NetworkDef]:
    """The project's networks, in their order."""
    return list(doc.settings.networks or [])


def define(doc: Document, network: NetworkDef | Mapping[str, Any]) -> NetworkDefined:
    """Writes ``network`` into the project (in the place of the one with its id, else last), as Ağlar's Kaydet does.
    A project setting, not an undo step; the rules are the apps' (``cad.network.define``)."""
    net = network if isinstance(network, NetworkDef) else NetworkDef.from_json(dict(network))
    return _network.define(doc, operation="set", network=net)


def remove(doc: Document, id: str) -> NetworkDefined:
    """Takes the network ``id`` out of the project."""
    return _network.define(doc, operation="remove", id=id)


def check(doc: Document, network: str) -> Check:
    """Denetle the network ``network`` as the drawing is now."""
    return Network(doc, network).check()


def route(doc: Document, network: str, stops: Sequence[Point], **options: Any) -> Route:
    """En kısa yol on the network ``network`` (:meth:`Network.route`'s options)."""
    return Network(doc, network).route(stops, **options)


def service_area(doc: Document, network: str, facilities: Sequence[Point], breaks: Sequence[float], **options: Any) -> ServiceAreas:
    """Hizmet alanı on the network ``network`` (:meth:`Network.service_area`'s options)."""
    return Network(doc, network).service_area(facilities, breaks, **options)


def closest_facility(doc: Document, network: str, incidents: Sequence[Point], facilities: Sequence[Point], **options: Any) -> list[list[Reached]]:
    """En yakın tesis on the network ``network`` (:meth:`Network.closest_facility`'s options)."""
    return Network(doc, network).closest_facility(incidents, facilities, **options)


def od_matrix(doc: Document, network: str, origins: Sequence[Point], destinations: Sequence[Point], **options: Any) -> list[list[Reached]]:
    """Maliyet matrisi on the network ``network`` (:meth:`Network.od_matrix`'s options)."""
    return Network(doc, network).od_matrix(origins, destinations, **options)


def trace(doc: Document, network: str, starts: Sequence[Point], **options: Any) -> Trace:
    """Şebeke izleme on the network ``network`` (:meth:`Network.trace`'s options)."""
    return Network(doc, network).trace(starts, **options)
