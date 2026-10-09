"""Zaman ve senaryolar from Python (docs/adr/0210 §11).

A temporal layer reads its objects' times from their attributes as the apps'
time slider reads them: a start field, and an end field for a ranged layer (an
object lives in [start, end)); without an end each object is a moment, or
shows from its start on when the layer is cumulative (Birikimli). A value is
``2018-06-01`` or ``01.06.2018``, with a time (``2020-09-30T14:30``,
``30.09.2020 14:30:00``) and a zone (``Z``, ``+03:00``) if need be.

- :func:`read` a value as a moment, :func:`write` a moment as a value, :func:`show`
  it as the apps show it;
- :func:`set_time` makes a layer temporal (``cad.layers.time``, as Zaman ayarları'
  Kaydet), :func:`clear_time` makes it timeless again, :func:`temporal_layers`
  lists them;
- :func:`times` gives a layer's objects' times (:class:`LayerTimes`), :func:`shown`
  the objects shown at a moment or within a period, :func:`extent` the period
  they cover;
- :func:`slider` lays the time slider out as the apps open it: its step, its
  positions and their windows (:class:`Slider`);
- :func:`create_scenario` and :func:`apply_scenario` (``cad.scenarios.edit``, as
  Senaryo oluştur and Senaryoyu uygula).

Moments are ``datetime`` objects without a zone, on the apps' clock: a value
without a zone as it is written, one with a zone in UTC (``2020-09-30T14:30+03:00``
is 11:30). A ``date`` is its midnight; a ``datetime`` with a zone is taken to
UTC; a text is read as a value is. Nothing here writes but the commands, and
everything reads the drawing as it is now.
"""

from __future__ import annotations

from collections.abc import Iterable, Sequence
from dataclasses import dataclass
from datetime import date, datetime, timedelta, timezone
from typing import Any, Literal

from . import _native
from .cad import layers as _layers
from .cad import scenarios as _scenarios
from .cad._runtime import UNSET as _UNSET
from .cad.document import Document
from .cad.types import LayerNode, LayerNodeType, LayersTimed, LayerTime, ScenariosEdited

__all__ = [
    "UNITS",
    "LayerTimes",
    "Mode",
    "Moment",
    "Slider",
    "Step",
    "Time",
    "TimeError",
    "Unit",
    "Window",
    "apply_scenario",
    "clear_time",
    "create_scenario",
    "extent",
    "read",
    "set_time",
    "show",
    "shown",
    "slider",
    "temporal_layers",
    "times",
    "write",
]

Unit = Literal["second", "minute", "hour", "day", "week", "month", "year"]
Mode = Literal["range", "instant", "cumulative"]
#: What a moment may be given as.
Moment = datetime | date | str

#: The slider's units, the shortest first.
UNITS: tuple[Unit, ...] = ("second", "minute", "hour", "day", "week", "month", "year")
_WORDS: dict[str, str] = {
    "second": "saniye",
    "minute": "dakika",
    "hour": "saat",
    "day": "gün",
    "week": "hafta",
    "month": "ay",
    "year": "yıl",
}
_EPOCH = datetime(1970, 1, 1)


class TimeError(ValueError):
    """A value that is not a moment, a step out of bounds, a layer that is not temporal, a slider with nothing to show:
    why, in the apps' words."""


def _call(call: Any, *args: Any) -> Any:
    try:
        return call(*args)
    except _native.HostError as e:
        message = e.args[1] if len(e.args) == 2 else str(e)
        raise TimeError(message) from None


# ------------------------------------------------------------------ moments


def _ms(at: Moment, what: str = "an") -> float:
    """A moment as the core takes it: milliseconds since 1970 on the apps' clock."""
    if isinstance(at, str):
        kind, t = _native.time_read(at)
        if kind != 1:
            said = "boş" if kind == 0 else f"“{at}”"
            raise TimeError(f"{what}: {said} bir tarih olarak okunamadı; 05.03.2024 ya da 2024-03-05 gibi yazın.")
        return t
    if isinstance(at, datetime):
        if at.tzinfo is not None:
            at = at.astimezone(timezone.utc).replace(tzinfo=None)
        d = at - _EPOCH
        # Whole milliseconds, below a millisecond dropped (as a value's fraction is).
        return float((d.days * 86_400 + d.seconds) * 1000 + d.microseconds // 1000)
    if isinstance(at, date):
        return float((at - _EPOCH.date()).days * 86_400_000)
    raise TimeError(f"{what}: {at!r} bir an değil; datetime, date ya da tarih metni olmalı.")


def _moment(t: float) -> datetime:
    return _EPOCH + timedelta(milliseconds=int(t))


def _open(t: float) -> datetime | None:
    """An end the core keeps as ±∞: None."""
    return None if t in (float("inf"), float("-inf")) else _moment(t)


def read(value: str | None) -> datetime | None:
    """A value as a moment (§3): None for an empty one; :class:`TimeError` for one that does not read.

    >>> read("01.06.2018")
    datetime.datetime(2018, 6, 1, 0, 0)
    """
    if value is None:
        return None
    kind, t = _native.time_read(value)
    if kind == 0:
        return None
    if kind == 2:
        raise TimeError(f"“{value}” bir tarih olarak okunamadı; 05.03.2024 ya da 2024-03-05 gibi yazın.")
    return _moment(t)


def write(at: Moment, *, date_only: bool = False) -> str:
    """A moment as a value is written (§3): ``2018-06-01`` at midnight (always with ``date_only``), else with its time
    (``2020-09-30T14:30:00``)."""
    return str(_call(_native.time_write, _ms(at), date_only))


def show(at: Moment, unit: Unit = "day") -> str:
    """A moment as the apps show it for a step of ``unit``: ``01.06.2018``, with ``14:30`` under a day (and seconds for
    a step of seconds)."""
    return str(_call(_native.time_show, _ms(at), unit))


# ------------------------------------------------------------------ an object's time


@dataclass(frozen=True, slots=True)
class Window:
    """A slider position's window: the moment :attr:`start` (:attr:`end` None, Anlık), or [start, end) (Aralık)."""

    start: datetime
    end: datetime | None = None

    def _ms(self) -> tuple[float, float | None]:
        return _ms(self.start), None if self.end is None else _ms(self.end)


def _window(at: Moment | None, between: tuple[Moment, Moment] | None) -> Window:
    if (at is None) == (between is None):
        raise TimeError("Bir an (at) ya da bir aralık (between) verin, ikisini birden değil.")
    if at is not None:
        return Window(_moment(_ms(at)))
    assert between is not None
    a, b = _ms(between[0], "aralığın başı"), _ms(between[1], "aralığın sonu")
    if b <= a:
        raise TimeError("Aralığın sonu başından sonra olmalı.")
    return Window(_moment(a), _moment(b))


@dataclass(frozen=True, slots=True)
class Time:
    """An object's time (§4): from :attr:`start` (None: before any moment) until :attr:`end` (None: open), and how it
    shows: ``range`` in [start, end), ``instant`` at its moment (its start and end the same), ``cumulative`` from its
    start on, whatever its end."""

    start: datetime | None
    end: datetime | None
    mode: Mode

    def shows(self, at: Moment | None = None, *, between: tuple[Moment, Moment] | None = None) -> bool:
        """Whether it shows at the moment ``at``, or within ``between`` [a, b) (the apps' rule)."""
        w = _window(at, between)
        s = float("-inf") if self.start is None else _ms(self.start)
        e = float("inf") if self.end is None else _ms(self.end)
        a, b = w._ms()
        return bool(_call(_native.time_shows, s, e, self.mode, a, b))

    @staticmethod
    def _of(crossed: tuple[float, float, str] | None) -> Time | None:
        if crossed is None:
            return None
        s, e, mode = crossed
        return Time(_open(s), _open(e), mode)  # type: ignore[arg-type]


@dataclass(frozen=True, slots=True)
class LayerTimes:
    """A temporal layer's objects' times: :attr:`times` by object id in the drawing's order (None: timeless, shown
    always); how many have a time and how many none; how many values do not read as dates (an unreadable end is an
    open one, an unreadable start leaves the object timeless or open before); the period the times cover."""

    layer: str
    rule: LayerTime
    times: dict[str, Time | None]
    timed: int
    timeless: int
    unreadable: int
    extent: tuple[datetime, datetime] | None


def _session(doc: Document) -> Any:
    session = doc._session
    if not isinstance(session, _native.Session):
        # The desktop's console: the drawing's copy in this process.
        session = _call(_native.Session.from_bytes, doc.to_bytes())
    return session


def _walk(nodes: Iterable[LayerNode], shown: bool = True) -> Iterable[tuple[LayerNode, bool]]:
    for n in nodes:
        here = shown and n.visible
        yield n, here
        yield from _walk(n.children, here)


def temporal_layers(doc: Document, *, shown_only: bool = False) -> list[LayerNode]:
    """The temporal layers in the tree's order; with ``shown_only`` those shown (they and their groups visible), as the
    apps' slider reads them."""
    return [
        n
        for n, here in _walk(doc.layers())
        if n.type == LayerNodeType.LAYER and isinstance(n.time, LayerTime) and (here or not shown_only)
    ]


def _layer_id(doc: Document, layer: str | LayerNode) -> str:
    return layer.id if isinstance(layer, LayerNode) else doc.layer(layer).id


def times(doc: Document, layer: str | LayerNode) -> LayerTimes:
    """The times of a temporal layer's objects (``layer`` its id, its name or its node), as the apps read them."""
    node = layer if isinstance(layer, LayerNode) else doc.layer(layer)
    uids, crossed, ((timed, timeless, unreadable), span) = _call(_native.temporal_layer, _session(doc), node.id)
    rule = node.time if isinstance(node.time, LayerTime) else LayerTime(start="")
    return LayerTimes(
        layer=node.id,
        rule=rule,
        times={u: Time._of(c) for u, c in zip(uids, crossed, strict=True)},
        timed=timed,
        timeless=timeless,
        unreadable=unreadable,
        extent=None if span is None else (_moment(span[0]), _moment(span[1])),
    )


def _ids(doc: Document, layers: Iterable[str | LayerNode] | None) -> list[str] | None:
    return None if layers is None else [_layer_id(doc, x) for x in layers]


def shown(
    doc: Document,
    at: Moment | None = None,
    *,
    between: tuple[Moment, Moment] | None = None,
    layers: Iterable[str | LayerNode] | None = None,
) -> list[str]:
    """The ids of the temporal layers' objects (of ``layers``, else of every temporal layer) shown at the moment ``at``
    or within ``between`` [a, b), in the drawing's order, as the time slider shows them; timeless objects show always.
    The layers' visibility is not asked."""
    a, b = _window(at, between)._ms()
    return list(_call(_native.temporal_shown, _session(doc), a, b, _ids(doc, layers)))


def extent(doc: Document, layers: Iterable[str | LayerNode] | None = None) -> tuple[datetime, datetime] | None:
    """The period the temporal layers' times cover (of ``layers``, else of every temporal layer): the least and the
    greatest of their finite starts and ends; None without a time."""
    ids = _ids(doc, layers)
    if ids is None:
        ids = [n.id for n in temporal_layers(doc)]
    return _extent(_session(doc), ids)


def _extent(session: Any, ids: Sequence[str]) -> tuple[datetime, datetime] | None:
    lo: float | None = None
    hi: float | None = None
    for id in ids:
        _, _, (_, span) = _call(_native.temporal_layer, session, id)
        if span is not None:
            lo = span[0] if lo is None else min(lo, span[0])
            hi = span[1] if hi is None else max(hi, span[1])
    return None if lo is None or hi is None else (_moment(lo), _moment(hi))


# ------------------------------------------------------------------ the slider


@dataclass(frozen=True, slots=True)
class Step:
    """The slider's step: :attr:`n` (1–999) :attr:`unit`\\ s; months and years by the calendar."""

    n: int
    unit: Unit

    def __str__(self) -> str:
        return f"{self.n} {_WORDS[self.unit]}"


@dataclass(frozen=True, slots=True)
class Slider:
    """The time slider over :attr:`extent` as the apps lay it out (§5): :attr:`positions` from the extent's start
    rounded down to the step's unit, the last one not before the extent's end (100 000 at most); a position's window
    is a period up to the next position (:attr:`ranged`, Aralık) or its moment (Anlık). The apps open it at the last
    position."""

    extent: tuple[datetime, datetime]
    step: Step
    positions: list[datetime]
    ranged: bool
    #: The temporal layers it reads, by id.
    layers: list[str]

    @property
    def last(self) -> int:
        """The last position's index (the apps' K)."""
        return len(self.positions) - 1

    def _at(self, k: int) -> float:
        if k < len(self.positions):
            return _ms(self.positions[k])
        return float(_call(_native.time_position, _ms(self.positions[0]), self.step.n, self.step.unit, k))

    def _k(self, k: int) -> int:
        if not 0 <= k <= self.last:
            raise TimeError(f"Konum 0 ile {self.last} arasında olmalı; {k} yazıldı.")
        return k

    def window(self, k: int) -> Window:
        """Position ``k``'s window."""
        k = self._k(k)
        if not self.ranged:
            return Window(self.positions[k])
        return Window(self.positions[k], _moment(self._at(k + 1)))

    def label(self, k: int) -> str:
        """What the apps' bar says of position ``k``: its date, or a period's two (``01.01.2015 – 01.01.2016``)."""
        w = self.window(k)
        first = show(w.start, self.step.unit)
        return first if w.end is None else f"{first} – {show(w.end, self.step.unit)}"

    def shown(self, doc: Document, k: int, layers: Iterable[str | LayerNode] | None = None) -> list[str]:
        """The objects position ``k`` shows of ``layers``, else of the slider's own, as :func:`shown`."""
        w = self.window(k)
        of = self.layers if layers is None else layers
        if w.end is None:
            return shown(doc, w.start, layers=of)
        return shown(doc, between=(w.start, w.end), layers=of)


def slider(
    doc: Document,
    *,
    step: Step | tuple[int, Unit] | None = None,
    ranged: bool | None = None,
    layers: Iterable[str | LayerNode] | None = None,
) -> Slider:
    """The time slider as the apps open it: over the period of ``layers`` (else of the temporal layers shown in the
    drawing), with ``step`` (else the first of a year, a month, a day, an hour, a minute and a second that cuts the
    period into at least 5 steps); its windows periods (``ranged``) or moments, as the apps choose: moments when a
    ranged layer is among them, else periods."""
    nodes = (
        [doc.layer(x) if not isinstance(x, LayerNode) else x for x in layers]
        if layers is not None
        else temporal_layers(doc, shown_only=True)
    )
    session = _session(doc)
    span = _extent(session, [n.id for n in nodes])
    if span is None:
        raise TimeError(
            "Görünen zamansal katmanlarda zamanı olan nesne yok. Bir katmana Zaman ayarları’ndan başlangıç alanı verin."
            if layers is None
            else "Bu katmanlarda zamanı olan nesne yok."
        )
    lo, hi = _ms(span[0]), _ms(span[1])
    if step is None:
        n, unit = _native.time_auto_step(lo, hi)
        chosen = Step(n, unit)  # type: ignore[arg-type]
    else:
        chosen = step if isinstance(step, Step) else Step(int(step[0]), step[1])
    placed = _call(_native.time_positions, lo, hi, chosen.n, chosen.unit)
    if placed is None:
        raise TimeError("Zaman aralığı bu adımla 100 000 konumdan fazla; daha büyük bir adım seçin.")
    anchor, last = placed
    positions = [_moment(_native.time_position(anchor, chosen.n, chosen.unit, k)) for k in range(last + 1)]
    if ranged is None:
        ranged = not any(isinstance(n.time, LayerTime) and n.time.end not in (None, _UNSET) for n in nodes)
    return Slider(extent=span, step=chosen, positions=positions, ranged=ranged, layers=[n.id for n in nodes])


# ------------------------------------------------------------------ commands


def set_time(
    doc: Document,
    layer: str | LayerNode,
    start: str,
    end: str | None = None,
    *,
    key: str | None = None,
    cumulative: bool = False,
) -> LayersTimed:
    """Makes ``layer`` temporal (``cad.layers.time``, one undo step “Zaman ayarları”): its objects' start (or moment)
    in the field ``start``, their end in ``end`` (none: moments), ``key`` naming an object across its versions,
    ``cumulative`` (Birikimli) showing each from its start on. A refusal raises ``CommandError`` in the apps' words."""
    time = LayerTime(
        start=start,
        end=end if end is not None else _UNSET,
        key=key if key is not None else _UNSET,
        cumulative=True if cumulative else _UNSET,
    )
    return _layers.time(doc, layer=_layer_id(doc, layer), time=time)


def clear_time(doc: Document, layer: str | LayerNode) -> LayersTimed:
    """Takes ``layer``'s time away (``cad.layers.time`` without one): its objects show always."""
    return _layers.time(doc, layer=_layer_id(doc, layer), time=None)


def create_scenario(
    doc: Document,
    name: str,
    layers: Iterable[str | LayerNode],
    *,
    copy_objects: bool = True,
    note: str | None = None,
) -> ScenariosEdited:
    """Senaryo oluştur (``cad.scenarios.edit``, one undo step): a group ``name`` on top of the tree, each of ``layers``
    copied into it standing for it (with its objects when ``copy_objects``). The group is hidden as made; the apps'
    Göster shows it. :attr:`ScenariosEdited.layers` pairs each source with its copy."""
    return _scenarios.edit(
        doc,
        operation="create",
        name=name,
        layers=[_layer_id(doc, x) for x in layers],
        copy_objects=copy_objects,
        note=note if note is not None else _UNSET,
    )


def apply_scenario(doc: Document, scenario: str | LayerNode) -> ScenariosEdited:
    """Senaryoyu uygula (``cad.scenarios.edit``, one undo step): each scenario layer's objects take the place of its
    base layer's; the scenario's other layers stay in its group, now an ordinary one."""
    return _scenarios.edit(doc, operation="apply", scenario=_layer_id(doc, scenario))
