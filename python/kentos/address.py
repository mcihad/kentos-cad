"""Adres arama from Python (docs/adr/0229 §14).

A project's address data is its settings (:func:`locators`): define one with
:func:`define` (``cad.addresses.define``, as Adres verisi's Kaydet does) and
take one away with :func:`remove`. A definition names the layers of the door
points, the street centre lines and the neighbourhood, district and province
areas, and which of their attributes say what. The apps build it into an
index from the drawing as it is, and so does :class:`Locator`:

- :func:`parse`: an address's parts, as the apps read them (§4: ``Mah.``,
  ``Cad.``, ``Sk.``, ``Blv.``, ``No:``, ``12/A``, ``1203/1``, a postcode);
- :meth:`Locator.locate` and :func:`locate`: an address's candidates, the best
  first, with their scores (0–100), kinds (Kod, Kapı, Kapı (aradeğer), Yol,
  Mahalle) and the best one's status (Eşleşti, Belirsiz, Eşleşmedi), in one line
  or in columns;
- :meth:`Locator.reverse` and :func:`reverse`: a place's address: the nearest
  door within the reach, else the nearest street, and the areas it is in;
- :meth:`Locator.check`: Adres verisi's Denetle.

Places are ``(east, north)`` in the project's system and unit; a distance and
the reach are metres. Nothing is written: a candidate's :attr:`Candidate.at`
goes to ``kentos.cad.point.create`` or ``kentos.cad.entities.create`` as it is.
An index is the drawing's when it was built: build it again after the drawing
changes. Online providers are not here: their terms are accepted in the apps,
on a device (§1).
"""

from __future__ import annotations

import json
from collections.abc import Mapping
from dataclasses import dataclass
from typing import Any

from . import _native
from .cad import addresses as _addresses
from .cad.document import Document
from .cad.types import AddressesDefined, AddressLocator

__all__ = [
    "AddressError",
    "Candidate",
    "Check",
    "Found",
    "Locator",
    "Parts",
    "Place",
    "Texts",
    "define",
    "locate",
    "locators",
    "parse",
    "remove",
    "reverse",
]

Point = tuple[float, float]


class AddressError(Exception):
    """A project's address data could not be built or asked: why, in the apps' words."""


def _call(call: Any, *args: Any, **kwargs: Any) -> Any:
    try:
        return call(*args, **kwargs)
    except _native.HostError as e:
        _code, message = e.args if len(e.args) == 2 else ("address_refused", str(e))
        raise AddressError(message) from None


@dataclass(frozen=True, slots=True)
class Parts:
    """An address's parts (§4): the street with its kind (CADDE, SOKAK, BULVAR, MEYDAN, YOL), the door's key
    (``12a``), the neighbourhood, district and province, the postcode, the words left, and whether the address had no
    tag at all (``Kızılay``)."""

    postcode: str | None
    street: str | None
    street_kind: str | None
    door: str | None
    neighbourhood: str | None
    district: str | None
    province: str | None
    rest: list[str]
    untagged: bool

    @staticmethod
    def _of(d: Mapping[str, Any]) -> Parts:
        return Parts(
            postcode=d["postcode"],
            street=d["street"],
            street_kind=d["streetKind"],
            door=d["door"],
            neighbourhood=d["neighbourhood"],
            district=d["district"],
            province=d["province"],
            rest=list(d["rest"]),
            untagged=bool(d["untagged"]),
        )


@dataclass(frozen=True, slots=True)
class Texts:
    """A found address's words, as its door or street gives them."""

    street: str
    door: str
    neighbourhood: str
    district: str
    province: str
    postcode: str
    building: str
    code: str

    @staticmethod
    def _of(d: Mapping[str, Any]) -> Texts:
        return Texts(**{k: d[k] for k in ("street", "door", "neighbourhood", "district", "province", "postcode", "building", "code")})


@dataclass(frozen=True, slots=True)
class Candidate:
    """A place an address may be: its kind, score (0–100), place, the address in one line (§9) and its words."""

    kind: str
    score: int
    at: Point
    label: str
    texts: Texts


@dataclass(frozen=True, slots=True)
class Found:
    """An address's candidates, the best first (at most 20), and the best one's status."""

    status: str
    candidates: list[Candidate]

    @property
    def best(self) -> Candidate | None:
        """The best candidate when it matched (not a tie, not under the lowest score)."""
        return self.candidates[0] if self.status == "Eşleşti" and self.candidates else None

    @staticmethod
    def _of(d: Mapping[str, Any]) -> Found:
        return Found(
            status=d["status"],
            candidates=[
                Candidate(kind=c["kind"], score=int(c["score"]), at=(c["at"][0], c["at"][1]), label=c["label"], texts=Texts._of(c["texts"]))
                for c in d["candidates"]
            ],
        )


@dataclass(frozen=True, slots=True)
class Place:
    """A place's address: Kapı, Yol or Bölge, where it is (the door, the street's nearest point, the place), how far
    from the place in metres (None for a region), the address in one line and its words."""

    kind: str
    at: Point
    distance: float | None
    label: str
    texts: Texts


@dataclass(frozen=True, slots=True)
class Check:
    """Denetle: what the index holds (doors, streets, street lines, neighbourhoods, areas) and what it left out."""

    counts: dict[str, int]
    left_out: dict[str, int]
    repeated: int
    repeats: list[dict[str, Any]]


def parse(address: str) -> Parts:
    """An address's parts, as the apps read them."""
    return Parts._of(json.loads(_call(_native.address_parse, address)))


def locators(doc: Document) -> list[AddressLocator]:
    """The project's address data, in its order."""
    return list(doc.settings.address_locators or [])


def define(doc: Document, locator: AddressLocator | Mapping[str, Any]) -> AddressesDefined:
    """Writes ``locator`` into the project (in the place of the one with its id, else last), as Adres verisi's Kaydet
    does. A project setting, not an undo step; the rules are the apps' (``cad.addresses.define``)."""
    loc = locator if isinstance(locator, AddressLocator) else AddressLocator.from_json(dict(locator))
    return _addresses.define(doc, operation="set", locator=loc)


def remove(doc: Document, id: str) -> AddressesDefined:
    """Takes the address data ``id`` out of the project."""
    return _addresses.define(doc, operation="remove", id=id)


class Locator:
    """A project's address data built once from the drawing as it is now, for many questions.

    ``locator`` is the definition's id; none: the project's first."""

    __slots__ = ("_native", "definition")

    def __init__(self, doc: Document, locator: str | None = None) -> None:
        session = doc._session
        if not isinstance(session, _native.Session):
            # The desktop's console: the drawing's copy in this process.
            session = _call(_native.Session.from_bytes, doc.to_bytes())
        self._native = _call(_native.Locator.build, session, locator)
        #: The definition, as the project keeps it.
        self.definition: AddressLocator = AddressLocator.from_json(json.loads(self._native.definition()))

    def __repr__(self) -> str:
        return f"<kentos.address.Locator {self.definition.name!r}>"

    def locate(
        self,
        address: str | None = None,
        *,
        neighbourhood: str = "",
        street: str = "",
        door: str = "",
        district: str = "",
        province: str = "",
        postcode: str = "",
        min_score: int | None = None,
    ) -> Found:
        """An address's candidates: in one line (``address``; an address code answers its door), or in columns
        (``street`` or ``neighbourhood`` at least). ``min_score``: under it the best is Eşleşmedi; none: the
        definition's."""
        if address is not None:
            if any((neighbourhood, street, door, district, province, postcode)):
                raise AddressError("Adres ya tek satırda ya da ayrı alanlarda verilir.")
            return Found._of(json.loads(_call(self._native.find, address, min_score)))
        if not (street.strip() or neighbourhood.strip()):
            raise AddressError("Ayrı alanlarda en az yol ya da mahalle verilir.")
        data = _call(self._native.find_columns, neighbourhood, street, door, district, province, postcode, min_score)
        return Found._of(json.loads(data))

    def reverse(self, point: Point, reach: float | None = None) -> Place | None:
        """The address of ``point`` (east, north): the nearest door within ``reach`` metres (none: the definition's),
        else the nearest street, and the areas it is in; None when nothing is near or around it."""
        if len(point) != 2:
            raise AddressError(f"Yer (doğu, kuzey) olmalı; {point!r} yazıldı.")
        data = json.loads(_call(self._native.reverse, float(point[0]), float(point[1]), reach))
        if data is None:
            return None
        return Place(
            kind=data["kind"],
            at=(data["at"][0], data["at"][1]),
            distance=data["distance"],
            label=data["label"],
            texts=Texts._of(data["texts"]),
        )

    def check(self) -> Check:
        """Denetle, as Adres verisi says it."""
        d = json.loads(self._native.check())
        return Check(
            counts={k: d[k] for k in ("doors", "streets", "lines", "neighbourhoods", "areas")},
            left_out={k: d[k] for k in ("notDoors", "emptyStreets", "unreadDoors", "notStreets", "unnamedLines", "notAreas", "unnamedAreas")},
            repeated=d["repeated"],
            repeats=d["repeats"],
        )


def locate(doc: Document, address: str, locator: str | None = None, min_score: int | None = None) -> Found:
    """An address's candidates in the project's address data ``locator`` (none: its first)."""
    return Locator(doc, locator).locate(address, min_score=min_score)


def reverse(doc: Document, point: Point, locator: str | None = None, reach: float | None = None) -> Place | None:
    """The address of ``point`` in the project's address data ``locator`` (none: its first)."""
    return Locator(doc, locator).reverse(point, reach)
