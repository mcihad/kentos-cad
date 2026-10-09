"""Map services from Python (docs/adr/0208 §15).

The apps' services core (``kentos._native``) reads what a service has and
what an answer means; this module sends the requests (``urllib``) and writes
with the catalog's commands, each a single undo step:

- :func:`presets` and :func:`add_basemap`: the ready basemaps, at the bottom
  of the layer tree in place of the one there, as the apps' Altlık ▾ does;
- :func:`read_service` and :func:`add_service`: a map service (XYZ / TMS,
  WMS, WMTS, OGC API Tiles, vector tiles, ArcGIS REST, Google) read and added
  as a layer (``cad.layers.service``);
- :func:`read_feed` and :func:`import_features`: a data service (WFS, OGC API
  Features, an ArcGIS layer, a GeoJSON address) read, its objects taken page
  by page into the project's system and written into a layer that
  remembers its source (``cad.layers.service`` ``addFeed`` and
  ``cad.entities.create`` in one step).

A connection (``ServiceConnection``: its kind, origin and the names of its
values) goes into the project; its secret (``ConnectionSecret``) never does:
it is used for the requests and forgotten. Drawing the tiles is the apps'.
"""

from __future__ import annotations

import json
import time
import urllib.error
import urllib.request
from collections.abc import Mapping, Sequence
from dataclasses import dataclass, field
from typing import Any

from . import _native
from .cad.document import Document

__all__ = [
    "ServiceError",
    "Taken",
    "add_basemap",
    "add_service",
    "import_features",
    "presets",
    "read_feed",
    "read_service",
]

#: The most objects a take brings unless told otherwise (the apps' En çok nesne).
DEFAULT_MOST = 50_000
#: Requests Bağlan sends at most (capabilities, then what they lead to).
_MOST_STEPS = 16


class ServiceError(Exception):
    """A service could not be read or asked, or a write was refused: why, in the apps' words."""


def _json(value: Any) -> str:
    return json.dumps(value, ensure_ascii=False, separators=(",", ":"))


def _native_call(call: Any, *args: Any) -> Any:
    try:
        return call(*args)
    except _native.HostError as e:
        code, message = e.args if len(e.args) == 2 else ("service_failed", str(e))
        raise ServiceError(message) from None


@dataclass
class _Proof:
    """A connection and its secret, for the requests to its origin."""

    connection: Mapping[str, Any]
    secret: Mapping[str, Any] | None
    token: str | None = None

    def apply(self, wire: dict[str, Any], referer: str, timeout: float) -> dict[str, Any]:
        conn = _json(self.connection)
        missing = _native_call(_native.services_auth_missing, conn, None if self.secret is None else _json(self.secret))
        if missing:
            raise ServiceError(missing)
        if self.secret is None:
            return wire
        secret = _json(self.secret)
        if self.connection.get("auth") in ("arcgis", "oauth2"):
            stale = self.token is None or json.loads(self.token)["expiresMs"] <= time.time() * 1000 + 60_000
            if stale:
                asked = _native_call(_native.services_token_request, conn, secret, referer)
                if asked:
                    body = _send(json.loads(asked), timeout)
                    self.token = _native_call(_native.services_read_token, conn, body, time.time() * 1000)
        applied: dict[str, Any] = json.loads(_native_call(_native.services_auth_apply, conn, secret, _json(wire), self.token))
        return applied


def _proof(connection: Mapping[str, Any] | None, secret: Mapping[str, Any] | None) -> _Proof | None:
    return None if connection is None else _Proof(dict(connection), None if secret is None else dict(secret))


def _send(wire: Mapping[str, Any], timeout: float) -> str:
    """A request as the core writes it, sent; its answer's text, or why not."""
    body = wire.get("body")
    data = None if body is None else str(body).encode("utf-8")
    req = urllib.request.Request(str(wire["url"]), data=data, method="GET" if data is None else "POST")
    for name, value in wire.get("headers") or []:
        req.add_header(str(name), str(value))
    if data is not None:
        req.add_header("Content-Type", str(wire.get("media") or "application/octet-stream"))
    req.add_header("User-Agent", f"KentOS-Python/{_native.__version__}")
    try:
        with urllib.request.urlopen(req, timeout=timeout) as res:
            raw: bytes = res.read()
            charset = res.headers.get_content_charset() or "utf-8"
    except urllib.error.HTTPError as e:
        e.close()
        if e.code in (401, 403):
            raise ServiceError(f"Sunucu erişimi reddetti ({e.code}): bağlantının değerlerini denetleyin.") from None
        if e.code == 404:
            raise ServiceError("Adres bulunamadı (404).") from None
        raise ServiceError(f"Sunucu {e.code} dedi.") from None
    except (urllib.error.URLError, TimeoutError, OSError) as e:
        reason = getattr(e, "reason", e)
        raise ServiceError(f"Servise ulaşılamadı: {reason}.") from None
    return raw.decode(charset, errors="replace")


def _ask(wire: Mapping[str, Any], proof: _Proof | None, referer: str, timeout: float) -> str:
    sent = dict(wire) if proof is None else proof.apply(dict(wire), referer, timeout)
    return _send(sent, timeout)


def _settings(doc: Document) -> dict[str, Any]:
    raw: dict[str, Any] = json.loads(doc._session.summary())["settings"]
    return raw


def _layers(doc: Document) -> list[dict[str, Any]]:
    raw: list[dict[str, Any]] = json.loads(doc._session.layers())
    return raw


def _service_command(doc: Document, payload: dict[str, Any]) -> str:
    answer = doc.run("cad.layers.service", payload)
    if answer.get("status") != "completed":
        error = answer.get("error") or {}
        raise ServiceError(str(error.get("message") or "Servis katmanı yazılamadı."))
    layer: str = answer["output"]["layer"]
    return layer


# ---------------------------------------------------------------- basemaps


def presets() -> list[dict[str, Any]]:
    """The ready basemaps: each its ``id``, ``name``, ``group`` (with
    ``groupName``), ``service`` and, for one that needs a key, ``connection``."""
    catalog = json.loads(_native.services_presets())
    groups = {g["id"]: g["name"] for g in catalog["groups"]}
    return [{**p, "groupName": groups.get(p["group"], "")} for p in catalog["presets"]]


def add_basemap(doc: Document, preset: str, *, name: str | None = None) -> str:
    """Shows ready basemap ``preset`` (``osm-standard``, ``esri-imagery`` …):
    the ready basemap at the bottom of the tree changes to it, or it goes
    below everything. The new or changed layer's id. A basemap that needs a
    key adds its connection to the project; its key is the apps' to keep."""
    found = _native.services_preset_layer(preset)
    if found is None:
        known = ", ".join(p["id"] for p in presets())
        raise ValueError(f"“{preset}” bir hazır altlık değil: {known}.")
    layer, connection = found
    p = next(x for x in presets() if x["id"] == preset)
    roots = _layers(doc)
    bottom = roots[-1] if roots else None
    payload: dict[str, Any] = {"name": name or p["name"], "service": json.loads(layer)}
    if connection is not None:
        payload["connections"] = [json.loads(connection)]
    if bottom is not None and bottom.get("type") == "layer" and (bottom.get("service") or {}).get("preset") is not None:
        payload.update(operation="update", layer=bottom["id"])
    else:
        payload["operation"] = "add"
    return _service_command(doc, payload)


# ---------------------------------------------------------------- map services


def _read(reading: Any, url: str, proof: _Proof | None, timeout: float) -> dict[str, Any]:
    nxt = reading.first()
    for _ in range(_MOST_STEPS):
        if nxt is None:
            break
        body = _ask(json.loads(nxt), proof, url, timeout)
        nxt = _native_call(reading.answer, body)
    else:
        if nxt is not None:
            raise ServiceError("Servis okunurken çok fazla istek gerekti; adresi denetleyin.")
    offer = reading.offer()
    if offer is None:
        raise ServiceError("Servis okunamadı.")
    read: dict[str, Any] = json.loads(offer)
    return read


def read_service(
    kind: str,
    url: str = "",
    *,
    connection: Mapping[str, Any] | None = None,
    secret: Mapping[str, Any] | None = None,
    timeout: float = 30.0,
) -> dict[str, Any]:
    """What a map service has, as the apps' Bağlan reads it: its ``title``,
    its ``items`` (layers, tile sets, map types: ``id``, ``title``,
    ``pickable``, ``styles``, ``formats``, ``srids`` …) and ``formats``.
    ``kind`` is ``xyz``, ``wms``, ``wmts``, ``ogcTiles``, ``vector``,
    ``arcgis`` or ``google``."""
    reading = _native_call(_native.ServiceReading, kind, url)
    return _read(reading, url, _proof(connection, secret), timeout)


def add_service(
    doc: Document,
    kind: str,
    url: str = "",
    *,
    items: Sequence[str] = (),
    name: str | None = None,
    connection: Mapping[str, Any] | None = None,
    secret: Mapping[str, Any] | None = None,
    style: str | None = None,
    format: str | None = None,
    srid: int | None = None,
    transparent: bool = True,
    dynamic: bool = False,
    opacity: float = 1.0,
    max_zoom: int | None = None,
    tile_size: int | None = None,
    subdomains: Sequence[str] = (),
    y_flip: bool = False,
    attribution: str | None = None,
    timeout: float = 30.0,
) -> str:
    """Reads a map service and adds it as a layer: ``items`` the layers to
    show (a WMS's names, a WMTS's or OGC API's tile set, an ArcGIS layer's
    id; none: the only one there is). An opaque service goes to the bottom
    of the tree, a clear one over the basemaps there, as the apps place it.
    ``connection`` (its secret in ``secret``, used only to read the service)
    joins the project. The new layer's id."""
    reading = _native_call(_native.ServiceReading, kind, url)
    offer = _read(reading, url, _proof(connection, secret), timeout)
    picked = list(items)
    if not picked:
        pickable = [i for i in offer["items"] if i.get("pickable")]
        if len(pickable) != 1:
            names = ", ".join(i["id"] or "(bütün servis)" for i in pickable[:20])
            raise ServiceError(f"Servisin {len(pickable)} katmanı var; items ile seçin: {names}.")
        picked = [pickable[0]["id"]]
    choice = {
        "items": picked,
        "style": style,
        "format": format,
        "srid": srid,
        "transparent": transparent,
        "dynamic": dynamic,
        "opacity": opacity,
        "connection": None if connection is None else connection.get("id"),
        "maxZoom": max_zoom,
        "tileSize": tile_size,
        "subdomains": list(subdomains),
        "yFlip": y_flip,
        "attribution": attribution,
    }
    project = int(_settings(doc).get("srid") or 0)
    service = json.loads(_native_call(reading.layer, _json(choice), project))
    first = next((i for i in offer["items"] if i["id"] == picked[0]), None)
    payload: dict[str, Any] = {
        "operation": "add",
        "name": name or (first or {}).get("title") or offer.get("title") or kind,
        "service": service,
    }
    if connection is not None:
        payload["connections"] = [dict(connection)]
    # An opaque service under everything; a clear one over the services at the bottom (the basemaps).
    if service.get("transparent") or service.get("opacity") is not None:
        roots = _layers(doc)
        basemaps = 0
        for node in reversed(roots):
            if node.get("type") == "layer" and node.get("service"):
                basemaps += 1
            else:
                break
        payload["index"] = len(roots) - basemaps
    return _service_command(doc, payload)


# ---------------------------------------------------------------- data services


@dataclass
class Taken:
    """What :func:`import_features` brought: the layer written, the objects
    taken, what the service said it matched, what was left out and why, and
    the apps' words for all of it."""

    layer: str
    taken: int
    matched: int | None
    dropped: int
    capped: bool
    skipped: list[dict[str, Any]] = field(default_factory=list)
    words: str = ""
    ids: list[str] = field(default_factory=list)


def read_feed(
    kind: str,
    url: str,
    *,
    connection: Mapping[str, Any] | None = None,
    secret: Mapping[str, Any] | None = None,
    timeout: float = 30.0,
) -> dict[str, Any]:
    """What a data service has: its ``title`` and ``items`` (a WFS's feature
    types, an OGC API's collections, an ArcGIS service's layers: ``id``,
    ``title``, ``srids``, ``geometry``). ``kind`` is ``wfs``,
    ``ogcFeatures``, ``arcgis`` or ``geojson``."""
    reading = _native_call(_native.FeedReading, kind, url)
    return _read(reading, url, _proof(connection, secret), timeout)


def import_features(
    doc: Document,
    kind: str,
    url: str,
    *,
    item: str | None = None,
    srid: int | None = None,
    bbox: tuple[float, float, float, float] | None = None,
    filter: str | None = None,
    most: int = DEFAULT_MOST,
    key: str | None = None,
    connection: Mapping[str, Any] | None = None,
    secret: Mapping[str, Any] | None = None,
    layer: str | None = None,
    name: str | None = None,
    timeout: float = 30.0,
) -> Taken:
    """Takes a data service's objects into the drawing as one undo step
    (“Servisten veri al”). ``item`` the type to take (none: the only one);
    ``srid`` the system to ask in (none: the project's when the service
    offers it, else WGS 84); ``bbox`` the area in the project's system;
    ``filter`` a CQL filter (an ArcGIS ``where``); ``most`` the most objects;
    ``key`` the attribute that matches objects when the layer is taken
    again. Without ``layer`` a new layer named ``name`` is made at the top,
    with fields from the values, and remembers the source; with it the
    objects go on that layer. The objects come into the project's system
    with its datum choices; what has no place there, and what the service
    gave without geometry, is left out and said."""
    proof = _proof(connection, secret)
    reading = _native_call(_native.FeedReading, kind, url)
    offer = _read(reading, url, proof, timeout)
    if item is None:
        if len(offer["items"]) != 1:
            names = ", ".join(i["id"] for i in offer["items"][:20])
            raise ServiceError(f"Serviste {len(offer['items'])} tür var; item ile seçin: {names}.")
        item = str(offer["items"][0]["id"])
    settings = _settings(doc)
    project = int(settings.get("srid") or 0)
    choice = {
        "item": item,
        "srid": srid,
        "filter": filter,
        "bbox": None if bbox is None else [float(v) for v in bbox],
        "limit": int(most) if most else None,
        "key": key,
        "connection": None if connection is None else connection.get("id"),
    }
    feed = json.loads(_native_call(reading.feed, _json(choice), project))
    asked = int(feed.get("srid") or 4326)
    text = _json(settings)
    area = None
    if bbox is not None:
        area = _native_call(_native.services_box_in, tuple(float(v) for v in bbox), text, asked)
        if area is None:
            raise ServiceError("İstenen alan servisin sisteminde gösterilemiyor.")
    taking = _native_call(_native.FeedTaking, _json(feed), int(most), bool(reading.geojson()), area)
    entities: list[dict[str, Any]] = []
    dropped = 0
    nxt: Any = json.loads(taking.first())
    pages = 0
    while nxt is not None and pages < 1000:
        pages += 1
        body = _ask(nxt, proof, url, timeout)
        page = json.loads(_native_call(taking.answer, body))
        moved = json.loads(_native_call(_native.services_to_project, _json(page["result"]), text, asked))
        entities.extend(moved["result"]["entities"])
        dropped += int(moved["dropped"])
        nxt = page["next"]
    skipped: list[dict[str, Any]] = json.loads(taking.skipped())
    capped = taking.taken >= int(most)
    objects = json.loads(_native_call(_native.services_new_objects, _json(entities)))
    with doc.group("Servisten veri al"):
        if layer is None:
            fields = json.loads(_native.services_infer_fields(_json([e.get("attrs") or {} for e in entities])))
            payload: dict[str, Any] = {
                "operation": "addFeed",
                "name": name or next((i["title"] for i in offer["items"] if i["id"] == item), item) or "Servis verisi",
                "index": 0,
                "feed": {**feed, "fetched": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())},
                "fields": fields,
            }
            if connection is not None:
                payload["connections"] = [dict(connection)]
            layer = _service_command(doc, payload)
        ids: list[str] = []
        if objects:
            answer = doc.run("cad.entities.create", {"layerId": layer, "objects": objects})
            if answer.get("status") != "completed":
                error = answer.get("error") or {}
                raise ServiceError(str(error.get("message") or "Servisin nesneleri yazılamadı."))
            ids = list(answer["output"]["created"])
    matched = taking.matched
    words = _native.services_taken_words(len(entities), matched, _json(skipped), dropped, capped, asked)
    return Taken(
        layer=layer,
        taken=len(entities),
        matched=matched,
        dropped=dropped,
        capped=capped,
        skipped=skipped,
        words=words,
        ids=ids,
    )
