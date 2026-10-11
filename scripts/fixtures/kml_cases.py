#!/usr/bin/env python3
"""Independent reference of KML and KMZ (docs/adr/0222 §2–§4).

Writes fixtures/formats/v1/kml/ from OGC KML 2.2 (07-147r2), Google's gx extension and the ADR's rules, with Python's
expat and zipfile and PROJ (pyproj), no KentOS code:

- sample files (*.kml, *.kmz) and what reading each gives (cases.json `reads`): its objects in WGS 84, layers, colours,
  line weights, attributes, what was left out and what was noted, or why nothing was read;
- what moving a reading into a project's system gives (cases.json `moves`, §2): every vertex moved by PROJ along the
  ADR's EPSG paths (scripts/fixtures/crs_transform_cases.py), within 1e-6 m;
- the writer's inputs (write/<name>.case.json). With --check, the file the writer wrote for each (write/<name>.kml or
  .kmz, written by `KENTOS_WRITE_KML=1 cargo test -p kentos-formats kml`) is read here and held to its input: its
  folders, placemarks, names, data and styles, and every coordinate moved by PROJ within 1.5e-10 degrees.

Reading (§3):

1. A KMZ (a zip archive) gives its doc.kml, else its first .kml at the root, else its first .kml anywhere; without one
   nothing is read. A text that is not well-formed XML, that nests deeper than 256 levels, or whose root is not kml is
   not read.
2. A Folder (its name, else “Adsız klasör”, noted) adds its name to the path of what is in it; a Document adds nothing.
   A placemark's layer is its path joined by " / "; outside every Folder, the first Document's name, else “KML”. A
   layer is hidden when one of its placemarks sits in a Folder (or under one) whose visibility is 0.
3. A placemark's geometry children (Point, LineString, LinearRing, Polygon, MultiGeometry, gx:Track, gx:MultiTrack)
   give points, paths and areas: a Point one position; a LineString two or more; a LinearRing three or more (a closed
   path as written); a Polygon its outerBoundaryIs ring and its innerBoundaryIs rings as holes, each ring without a
   closing position that repeats the first, left out (an outline with its area) when fewer than three corners remain
   after consecutive repeats are taken once; a MultiGeometry its members'; a gx:Track its gx:coord positions
   (“lon lat alt”), two or more; a gx:MultiTrack its tracks'. A Model is counted and left out. The placemark becomes
   one object of each kind it gave, points first, then paths, then areas: a point or a multi-point object; a line (a
   path of two positions), a polyline or a multi-part polyline; an area or a multi-part area.
4. Coordinates are lon,lat[,alt] tuples apart by white space (a word that begins with a comma, or follows a word that
   ends with one, joins it); a value is a decimal (an optional sign, digits with an optional point and digits, or a
   point and digits, an optional exponent) rounded once to the nearest float64, finite, |lon| ≤ 180, |lat| ≤ 90. A
   geometry whose coordinates are not so is left out, said at its placemark's line (an outline's as the Polygon's, a
   hole's as the LinearRing's). An altitude is the vertex's elevation only when the geometry's altitudeMode is
   absolute; else it is left, noted once per placemark.
5. Attributes: Ad (name), Açıklama (description), Zaman (TimeStamp's when), Başlangıç and Bitiş (TimeSpan's begin
   and end) and each ExtendedData Data (its value) and SimpleData (its text) by its name, a later one of a name
   winning; a text is its character data (CDATA's too) trimmed, and an empty one is none.
6. The style: the placemark's own Style, else the one its styleUrl names (#id: a Style, or a StyleMap's normal pair's
   Style or styleUrl); a styleUrl into another file is noted. A point's colour is the IconStyle's, else the
   LineStyle's; a path's the LineStyle's; an area's the LineStyle's (else the PolyStyle's) when outlined, else the
   PolyStyle's (else the LineStyle's). A colour aabbggrr is #rrggbb. A path's or an area's line weight is the
   LineStyle's width (pixels, 25.4/96 mm each) as the nearest of 0.13, 0.18, 0.25, 0.35, 0.5, 0.7, 1, 1.4 and 2 mm (the
   first of two as near). A layer's colour is its objects' most common one (the first met of the most), else ink.
7. NetworkLink, GroundOverlay, ScreenOverlay, PhotoOverlay and gx:Tour are counted and left out.

Writing (§4): the objects move from the project's system into WGS 84 longitude and latitude (§2's way), ten decimals;
a Folder per layer inside its groups' Folders, the layers in their order, a layer not listed after them by its name;
a Placemark per object in its layer's order, named by its text (a text, as a Point) or its label, else its Ad
attribute; its attributes as ExtendedData Data; a shared Style s1, s2, … in the order first used: a LineStyle of the
object's colour (else its layer's, else black) and width (its line weight, else its layer's, else 0.25 mm, in
pixels to two decimals, the thinnest line 0 one pixel), a PolyStyle of that colour two fifths opaque for an area, an IconStyle for a point. A
geometry whose every vertex has an elevation is written with it and altitudeMode absolute. Areas' outlines run
counter-clockwise and holes clockwise (first vertex kept); a circle is the app's sampling, 72 points a turn from angle
0, closed. A KMZ holds the KML as doc.kml.

Moving (§2): every vertex of a reading moves from WGS 84 into the project's system (elevations stay); the layers keep
their objects.
"""

import argparse
import io
import json
import math
import re
import sys
import xml.parsers.expat
import zipfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from crs_transform_cases import system, transformer  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures" / "formats" / "v1" / "kml"
WRITE = DIR / "write"
REGISTRY = ROOT / "fixtures" / "crs" / "v1" / "registry.json"
DEPTH = 256
WEIGHTS = [0.13, 0.18, 0.25, 0.35, 0.5, 0.7, 1.0, 1.4, 2.0]
MM_PER_PIXEL = 25.4 / 96
NUMBER = re.compile(r"[+-]?(\d+\.?\d*|\.\d+)([eE][+-]?\d+)?")
GEOMETRIES = {"Point", "LineString", "LinearRing", "Polygon", "MultiGeometry", "Track", "MultiTrack", "Model"}
OVERLAYS = {"NetworkLink", "GroundOverlay", "ScreenOverlay", "PhotoOverlay"}
SKIP_REASON = {
    "NetworkLink": "başka bir dosyaya ya da adrese bağlantı; uzaktaki içerik alınmaz",
    "GroundOverlay": "resim kaplaması; alınmadı (resmi Resim ekle ile yerleştirin)",
    "ScreenOverlay": "resim kaplaması; alınmadı (resmi Resim ekle ile yerleştirin)",
    "PhotoOverlay": "resim kaplaması; alınmadı (resmi Resim ekle ile yerleştirin)",
    "Model": "3B model; alınmadı",
    "gx:Tour": "gezinti; alınmadı",
}
BAD = "koordinatları okunmuyor (boylam, enlem[, yükseklik] bekleniyordu); alınmadı"
ERRORS = {
    "xml": "KML okunamadı: dosya XML değil; hiçbir şey alınmadı.",
    "deep": f"KML okunamadı: XML öğeleri {DEPTH} düzeyden derin iç içe; hiçbir şey alınmadı.",
    "root": "KML değil: kök öğe “{root}” (kml bekleniyordu); hiçbir şey alınmadı.",
    "nokml": "KMZ arşivinde .kml dosyası yok; hiçbir şey alınmadı.",
}
NOTES = {
    "closing": ("Halka kapanışının Z'si", "kapanış konumu ilkiyle x ve y'de aynı ama Z'si farklı; alanın köşesi ilk konumun Z'sini aldı"),
    "relative": ("Yükseklik", "yükseklik kipi absolute değil (zemine ya da yere göre); kot olarak alınmadı"),
    "unnamed": ("Adsız klasör", "adı olmayan klasör “Adsız klasör” adıyla katman yolunda yer aldı"),
    "mixed": ("Karışık MultiGeometry", "türleri ayrı nesneler oldu (noktalar, çizgiler, alanlar)"),
    "external": ("Dış biçem", "başka dosyadaki biçeme bağlı; renk alınmadı"),
}


class TooDeep(Exception):
    pass


class Node:
    """An element: its local name, attributes, start line and contents (texts and elements) in order."""

    def __init__(self, tag, attrs, line):
        self.tag, self.attrs, self.line, self.items = tag, attrs, line, []

    @property
    def children(self):
        return [i for i in self.items if isinstance(i, Node)]

    def child(self, tag):
        return next((c for c in self.children if c.tag == tag), None)

    def all_text(self):
        return "".join(i if isinstance(i, str) else i.all_text() for i in self.items)


def text(node):
    """An element's character data and its descendants', trimmed; None when empty or absent."""
    if node is None:
        return None
    t = node.all_text().strip()
    return t or None


def parse(data):
    parser = xml.parsers.expat.ParserCreate(namespace_separator=" ")
    stack, root = [], []

    def start(tag, attrs):
        if len(stack) + 1 > DEPTH:
            raise TooDeep()
        node = Node(tag.rsplit(" ", 1)[-1], {k.rsplit(" ", 1)[-1]: v for k, v in attrs.items()}, parser.CurrentLineNumber)
        if stack:
            stack[-1].items.append(node)
        else:
            root.append(node)
        stack.append(node)

    def end(_):
        stack.pop()

    def chars(s):
        if stack:
            stack[-1].items.append(s)

    parser.StartElementHandler, parser.EndElementHandler, parser.CharacterDataHandler = start, end, chars
    parser.Parse(data, True)
    return root[0]


# ── Reading (§3) ───────────────────────────────────────────────────────────────


def number(t):
    t = t.strip()
    if not NUMBER.fullmatch(t):
        return None
    v = float(t)
    return v if math.isfinite(v) else None


def positions(t):
    """A coordinates text's positions (lon, lat, alt or None), or None when one is not read."""
    tuples = []
    for word in t.split():
        if tuples and (tuples[-1].endswith(",") or word.startswith(",")):
            tuples[-1] += word
        else:
            tuples.append(word)
    out = []
    for tup in tuples:
        values = tup.split(",")
        if len(values) not in (2, 3):
            return None
        nums = [number(v) for v in values]
        if any(n is None for n in nums):
            return None
        lon, lat = nums[0], nums[1]
        if abs(lon) > 180 or abs(lat) > 90:
            return None
        out.append((lon, lat, nums[2] if len(nums) == 3 else None))
    return out


class Ring:
    def __init__(self, pts, zs):
        self.pts = pts
        self.zs = zs if len(zs) == len(pts) and any(z is not None for z in zs) else None


def open_ring(r):
    """A ring without a closing position that repeats the first; whether the closing's elevation differs."""
    differs = False
    if len(r.pts) > 1 and r.pts[0] == r.pts[-1]:
        pts = r.pts[:-1]
        zs = None
        if r.zs is not None:
            closing = r.zs[-1]
            zs = r.zs[:-1]
            differs = closing is not None and closing != zs[0]
            if all(z is None for z in zs):
                zs = None
        r = Ring(pts, zs if zs is not None else [None] * len(pts))
    return r, differs


class Shapes:
    def __init__(self):
        self.points, self.paths, self.areas = [], [], []
        self.relative = 0
        self.bad = []
        self.closing = False
        self.thin = 0
        self.models = 0

    def ring(self, pts, absolute):
        if not absolute and any(p[2] is not None for p in pts):
            self.relative += 1
        return Ring([(x, y) for x, y, _ in pts], [z if absolute else None for _, _, z in pts])

    def coordinates(self, g, what):
        pts = positions(text(g.child("coordinates")) or "")
        if pts is None:
            self.bad.append(what)
        return pts

    def closed(self, pts, absolute):
        r, differs = open_ring(self.ring(pts, absolute))
        self.closing |= differs
        distinct = [p for i, p in enumerate(r.pts) if i == 0 or p != r.pts[i - 1]]
        if len(distinct) < 3:
            self.thin += 1
            return None
        return r

    def geometry(self, g):
        absolute = text(g.child("altitudeMode")) == "absolute"
        tag = g.tag
        if tag == "Point":
            pts = self.coordinates(g, "Point")
            if pts is None:
                return
            if len(pts) != 1:
                self.bad.append("Point")
                return
            x, y, z = pts[0]
            if not absolute and z is not None:
                self.relative += 1
            self.points.append(((x, y), z if absolute else None))
        elif tag in ("LineString", "LinearRing"):
            pts = self.coordinates(g, tag)
            if pts is None:
                return
            if len(pts) >= (2 if tag == "LineString" else 3):
                self.paths.append(self.ring(pts, absolute))
            else:
                self.bad.append(tag)
        elif tag == "Polygon":
            outer = g.child("outerBoundaryIs")
            outer = outer.child("LinearRing") if outer is not None else None
            if outer is None:
                self.bad.append("Polygon")
                return
            pts = self.coordinates(outer, "Polygon")
            if pts is None:
                return
            r = self.closed(pts, absolute)
            if r is None:
                return
            holes = []
            for inner in (c for c in g.children if c.tag == "innerBoundaryIs"):
                ring = inner.child("LinearRing")
                if ring is None:
                    continue
                hp = self.coordinates(ring, "LinearRing")
                if hp is not None:
                    h = self.closed(hp, absolute)
                    if h is not None:
                        holes.append(h)
            self.areas.append((r, holes))
        elif tag == "MultiGeometry":
            for m in g.children:
                self.geometry(m)
        elif tag == "Track":
            pts = []
            for c in (c for c in g.children if c.tag == "coord"):
                p = positions(",".join((text(c) or "").split()))
                if p is None or len(p) != 1:
                    pts = None
                    break
                pts.append(p[0])
            if pts is not None and len(pts) >= 2:
                self.paths.append(self.ring(pts, absolute))
            else:
                self.bad.append("gx:Track")
        elif tag == "MultiTrack":
            for t in (c for c in g.children if c.tag == "Track"):
                self.geometry(t)
        elif tag == "Model":
            self.models += 1


def xy(p):
    return [p[0], p[1]]


def zlist(zs):
    return list(zs)


def area_obj(o, outline, holes):
    o["pts"] = [xy(p) for p in outline.pts]
    if outline.zs is not None:
        o["zs"] = zlist(outline.zs)
    if holes:
        o["holes"] = [[xy(p) for p in h.pts] for h in holes]
        if any(h.zs is not None for h in holes):
            o["holeZs"] = [None if h.zs is None else zlist(h.zs) for h in holes]


def path_obj(r):
    """A run as one object: a line of two positions, else a polyline."""
    if len(r.pts) == 2:
        o = {"kind": "line", "a": xy(r.pts[0]), "b": xy(r.pts[1])}
        if r.zs is not None:
            for key, z in zip(("za", "zb"), r.zs):
                if z is not None:
                    o[key] = z
        return o
    o = {"kind": "polyline", "pts": [xy(p) for p in r.pts]}
    if r.zs is not None:
        o["zs"] = zlist(r.zs)
    return o


def objects_of(s):
    """One object of each kind: points, paths, areas (geometry only, with the kind's family)."""
    out = []
    if s.points:
        (p, z), rest = s.points[0], s.points[1:]
        o = {"kind": "point", "p": xy(p)}
        if z is not None:
            o["z"] = z
        if rest:
            o["parts"] = [{"p": xy(q), **({"z": w} if w is not None else {})} for q, w in rest]
        out.append(("point", o))
    if s.paths:
        if len(s.paths) == 1:
            out.append(("path", path_obj(s.paths[0])))
        else:
            first, rest = s.paths[0], s.paths[1:]
            o = {"kind": "polyline", "pts": [xy(p) for p in first.pts]}
            if first.zs is not None:
                o["zs"] = zlist(first.zs)
            o["parts"] = [{"pts": [xy(p) for p in r.pts], **({"zs": zlist(r.zs)} if r.zs is not None else {})} for r in rest]
            out.append(("path", o))
    if s.areas:
        (outline, holes), rest = s.areas[0], s.areas[1:]
        o = {"kind": "polygon"}
        area_obj(o, outline, holes)
        if rest:
            parts = []
            for r, hs in rest:
                part = {}
                area_obj(part, r, hs)
                parts.append(part)
            o["parts"] = parts
        out.append(("area", o))
    return out


def parse_color(t):
    t = (t or "").strip().lstrip("#")
    if len(t) != 8 or any(c not in "0123456789abcdefABCDEF" for c in t):
        return None
    return t[6:8].lower() + t[4:6].lower() + t[2:4].lower()  # rr gg bb (alpha dropped)


def flag(t, default):
    return True if t in ("1", "true") else False if t in ("0", "false") else default


def style_of(n):
    line, poly, icon = n.child("LineStyle"), n.child("PolyStyle"), n.child("IconStyle")
    width = None
    if line is not None and text(line.child("width")) is not None:
        try:
            w = float(text(line.child("width")))
            width = w if math.isfinite(w) and w > 0 else None
        except ValueError:
            width = None
    return {
        "line": parse_color(text(line.child("color"))) if line is not None else None,
        "width": width,
        "poly": parse_color(text(poly.child("color"))) if poly is not None else None,
        "fill": flag(text(poly.child("fill")) if poly is not None else None, True),
        "outline": flag(text(poly.child("outline")) if poly is not None else None, True),
        "icon": parse_color(text(icon.child("color"))) if icon is not None else None,
    }


def weight(px):
    mm = px * MM_PER_PIXEL
    return min(WEIGHTS, key=lambda w: abs(w - mm))


def descendants(n):
    for c in n.children:
        yield c
        yield from descendants(c)


def styles_of(root):
    own, maps = {}, {}
    for n in descendants(root):
        sid = n.attrs.get("id")
        if sid is None:
            continue
        if n.tag == "Style":
            own[sid] = style_of(n)
        elif n.tag == "StyleMap":
            normal = next((p for p in n.children if p.tag == "Pair" and (text(p.child("key")) or "normal") == "normal"), None)
            if normal is not None:
                url = text(normal.child("styleUrl"))
                if url is not None:
                    maps[sid] = url
                elif normal.child("Style") is not None:
                    key = f"{sid}#normal"
                    own[key] = style_of(normal.child("Style"))
                    maps[sid] = "#" + key
    return own, maps


def named(styles, url):
    own, maps = styles
    if not url.startswith("#"):
        return None
    sid = url[1:]
    if sid in own:
        return own[sid]
    mapped = maps.get(sid)
    if mapped is None or not mapped.startswith("#"):
        return None
    return own.get(mapped[1:])


class Report:
    def __init__(self):
        self.counts, self.skipped, self.notes, self.source = {}, [], [], []

    def add(self, items, what, reason, count, line=0):
        if count <= 0:
            return
        item = next((i for i in items if i["what"] == what and i["reason"] == reason), None)
        if item is None:
            item = {"what": what, "reason": reason, "count": 0, "lines": []}
            items.append(item)
        item["count"] += count
        if line > 0 and line not in item["lines"][-1:]:
            item["lines"].append(line)


def kml_text(data):
    """The KML text and what the file is, or an error."""
    if data[:4] == b"PK\x03\x04":
        with zipfile.ZipFile(io.BytesIO(data)) as z:
            names = z.namelist()
            chosen = next((n for n in names if n.lower() == "doc.kml"), None)
            chosen = chosen or next((n for n in names if "/" not in n and n.lower().endswith(".kml")), None)
            chosen = chosen or next((n for n in names if n.lower().endswith(".kml")), None)
            if chosen is None:
                return None, None, ERRORS["nokml"]
            return z.read(chosen), f"KMZ ({chosen})", None
    return data, "KML", None


def read(data):
    body, what, error = kml_text(data)
    if error:
        return {"error": error}
    try:
        root = parse(body)
    except TooDeep:
        return {"error": ERRORS["deep"]}
    except xml.parsers.expat.ExpatError:
        return {"error": ERRORS["xml"]}
    if root.tag != "kml":
        return {"error": ERRORS["root"].format(root=root.tag)}
    styles = styles_of(root)
    document = root.child("Document")
    name = text(document.child("name")) if document is not None else None
    report = Report()
    walk = {"objects": [], "hidden": set(), "skipped": {}, "bad": {}, "relative": 0, "closing": 0, "thin": 0,
            "mixed": 0, "external": 0, "unnamed": 0}

    def container(n, path, visible):
        for c in n.children:
            if c.tag == "Document":
                container(c, path, visible)
            elif c.tag == "Folder":
                folder = text(c.child("name"))
                if folder is None:
                    walk["unnamed"] += 1
                    folder = "Adsız klasör"
                container(c, path + [folder], visible and text(c.child("visibility")) != "0")
            elif c.tag == "Placemark":
                placemark(c, path, visible)
            elif c.tag in OVERLAYS:
                walk["skipped"][c.tag] = walk["skipped"].get(c.tag, 0) + 1
            elif c.tag == "Tour":
                walk["skipped"]["gx:Tour"] = walk["skipped"].get("gx:Tour", 0) + 1

    def placemark(p, path, visible):
        s = Shapes()
        for g in p.children:
            if g.tag in GEOMETRIES:
                s.geometry(g)
        if s.models:
            walk["skipped"]["Model"] = walk["skipped"].get("Model", 0) + s.models
        walk["relative"] += 1 if s.relative else 0
        walk["closing"] += 1 if s.closing else 0
        walk["thin"] += s.thin
        for w in s.bad:
            count, line = walk["bad"].get(w, (0, p.line))
            walk["bad"][w] = (count + 1, line)
        objects = objects_of(s)
        if len(objects) > 1:
            walk["mixed"] += 1
        attrs = {}
        for key, value in (("Ad", text(p.child("name"))), ("Açıklama", text(p.child("description")))):
            if value is not None:
                attrs[key] = value
        stamp = p.child("TimeStamp")
        if stamp is not None and text(stamp.child("when")) is not None:
            attrs["Zaman"] = text(stamp.child("when"))
        span = p.child("TimeSpan")
        if span is not None:
            for key, tag in (("Başlangıç", "begin"), ("Bitiş", "end")):
                if text(span.child(tag)) is not None:
                    attrs[key] = text(span.child(tag))
        data = p.child("ExtendedData")
        if data is not None:
            for d in descendants(data):
                if d.tag not in ("Data", "SimpleData"):
                    continue
                key = (d.attrs.get("name") or "").strip()
                if not key:
                    continue
                attrs[key] = (text(d.child("value")) if d.tag == "Data" else text(d)) or ""
        inline = p.child("Style")
        url = text(p.child("styleUrl"))
        if inline is None and url is not None and not url.startswith("#"):
            walk["external"] += 1
        style = style_of(inline) if inline is not None else (named(styles, url) if url is not None else None)
        layer = " / ".join(path)
        if not visible:
            walk["hidden"].add(layer)
        for family, o in objects:
            color, lw = None, None
            if style is not None:
                if family == "point":
                    color = style["icon"] or style["line"]
                elif family == "area":
                    color = (style["line"] or style["poly"]) if style["outline"] else (style["poly"] or style["line"])
                    lw = weight(style["width"]) if style["width"] is not None else None
                else:
                    color = style["line"]
                    lw = weight(style["width"]) if style["width"] is not None else None
            o = {**o, "layer": layer, "attrs": dict(sorted(attrs.items()))}
            if color is not None:
                o["color"] = "#" + color
            if lw is not None:
                o["lineWeight"] = lw
            walk["objects"].append(o)
            kind = "point" if family == "point" else ("polygon" if family == "area" else o["kind"])
            report.counts[kind] = report.counts.get(kind, 0) + 1

    container(root, [], True)
    for w in sorted(walk["skipped"]):
        report.add(report.skipped, w, SKIP_REASON[w], walk["skipped"][w])
    for w in sorted(walk["bad"]):
        count, line = walk["bad"][w]
        report.add(report.skipped, w, BAD, count, line)
    report.add(report.skipped, "Halka", "üçten az farklı köşesi var; alınmadı", walk["thin"])
    for key in ("closing", "relative", "unnamed", "mixed", "external"):
        report.add(report.notes, *NOTES[key], walk[key])
    report.source.append(["Biçim", what])
    if name is not None:
        report.source.append(["Ad", name])
    report.source.append(["Koordinat sistemi", "WGS 84 boylam, enlem (EPSG:4326)"])
    default = name if name is not None else "KML"
    layers = []
    for o in walk["objects"]:
        if o["layer"] == "":
            o["layer"] = default
        layer = next((l for l in layers if l["name"] == o["layer"]), None)
        if layer is None:
            layer = {"name": o["layer"], "color": "ink", "visible": o["layer"] not in walk["hidden"], "count": 0, "met": []}
            layers.append(layer)
        layer["count"] += 1
        if "color" in o:
            met = next((m for m in layer["met"] if m[0] == o["color"]), None)
            if met is None:
                layer["met"].append([o["color"], 1])
            else:
                met[1] += 1
    for layer in layers:
        if layer["met"]:
            most = max(n for _, n in layer["met"])
            layer["color"] = next(c for c, n in layer["met"] if n == most)
        del layer["met"]
    elevated = sum(1 for o in walk["objects"] if elevated_object(o))
    if elevated:
        report.source.append(["Kotlu nesne", str(elevated)])
    return {
        "objects": walk["objects"],
        "layers": layers,
        "counts": dict(sorted(report.counts.items())),
        "skipped": report.skipped,
        "notes": report.notes,
        "source": report.source,
        "declared": {"srid": 4326, "text": "WGS 84 (KML)", "source": "kml"},
    }


def elevated_object(o):
    def any_z(d):
        return any(d.get(k) is not None for k in ("z", "za", "zb")) or any(z is not None for z in d.get("zs") or [])

    return any_z(o) or any(any_z(p) for p in o.get("parts", [])) or any(
        z is not None for zs in o.get("holeZs") or [] if zs for z in zs) or any(
        z is not None for p in o.get("parts", []) for zs in p.get("holeZs") or [] if zs for z in zs)


# ── Sample files ───────────────────────────────────────────────────────────────

GEZI = """<?xml version="1.0" encoding="UTF-8"?>
<kml xmlns="http://www.opengis.net/kml/2.2" xmlns:gx="http://www.google.com/kml/ext/2.2">
<Document>
  <name>Gezi</name>
  <Style id="kirmizi"><LineStyle><color>ff0000ff</color><width>3</width></LineStyle><PolyStyle><color>7f00ff00</color></PolyStyle></Style>
  <Style id="mavi-nokta"><IconStyle><color>ffff0000</color></IconStyle></Style>
  <StyleMap id="yesil"><Pair><key>normal</key><styleUrl>#yesil-n</styleUrl></Pair><Pair><key>highlight</key><styleUrl>#kirmizi</styleUrl></Pair></StyleMap>
  <Style id="yesil-n"><LineStyle><color>ff00aa00</color><width>1</width></LineStyle><PolyStyle><color>8000cc00</color><fill>1</fill><outline>0</outline></PolyStyle></Style>
  <StyleMap id="sari"><Pair><key>normal</key><Style><LineStyle><color>ff00ffff</color><width>5.3</width></LineStyle></Style></Pair></StyleMap>
  <Placemark>
    <name>Başlangıç</name>
    <description><![CDATA[<b>Kamp</b> yeri]]></description>
    <styleUrl>#mavi-nokta</styleUrl>
    <TimeStamp><when>2026-10-04T10:15:00Z</when></TimeStamp>
    <Point><altitudeMode>absolute</altitudeMode><coordinates>32.8597,39.9334,850.25</coordinates></Point>
  </Placemark>
  <Folder>
    <name>Yollar</name>
    <Folder>
      <name>Ana yollar</name>
      <Placemark>
        <name>D-750</name>
        <styleUrl>#kirmizi</styleUrl>
        <ExtendedData><Data name="Şerit"><value>4</value></Data><Data name="Kaplama"><value> asfalt </value></Data></ExtendedData>
        <LineString><tessellate>1</tessellate><coordinates>
          32.80,39.90 32.81,39.91
          32.82 ,39.92 32.83, 39.925
        </coordinates></LineString>
      </Placemark>
      <Placemark><name>Bağlantı</name><styleUrl>#kirmizi</styleUrl><LineString><coordinates>32.83,39.93 32.84,39.94</coordinates></LineString></Placemark>
    </Folder>
    <Placemark>
      <name>Halka yol</name>
      <Style><LineStyle><color>ff00ffff</color><width>2.5</width></LineStyle></Style>
      <TimeSpan><begin>2026-01-01</begin><end>2026-12-31</end></TimeSpan>
      <LinearRing><coordinates>32.85,39.95 32.86,39.95 32.86,39.96 32.85,39.95</coordinates></LinearRing>
    </Placemark>
    <Placemark><name>Sarı</name><styleUrl>#sari</styleUrl><LineString><coordinates>32.87,39.97 32.88,39.98 32.89,39.97</coordinates></LineString></Placemark>
  </Folder>
  <Folder>
    <name>Parseller</name>
    <visibility>0</visibility>
    <Placemark>
      <name>Ada 101</name>
      <styleUrl>#yesil</styleUrl>
      <ExtendedData><SchemaData schemaUrl="#parsel"><SimpleData name="Ada">101</SimpleData><SimpleData name="Parsel">7</SimpleData></SchemaData></ExtendedData>
      <Polygon>
        <altitudeMode>absolute</altitudeMode>
        <outerBoundaryIs><LinearRing><coordinates>32.90,39.90,10 32.90,39.91,13 32.91,39.91,12 32.91,39.90,11 32.90,39.90,10</coordinates></LinearRing></outerBoundaryIs>
        <innerBoundaryIs><LinearRing><coordinates>32.902,39.902,10 32.904,39.902,10 32.904,39.904,10 32.902,39.902,10</coordinates></LinearRing></innerBoundaryIs>
        <innerBoundaryIs><LinearRing><coordinates>32.905,39.905 32.906,39.906 32.905,39.905</coordinates></LinearRing></innerBoundaryIs>
      </Polygon>
    </Placemark>
    <Placemark>
      <name>Kesişen alanlar</name>
      <styleUrl>#kirmizi</styleUrl>
      <MultiGeometry>
        <Polygon><outerBoundaryIs><LinearRing><coordinates>32.92,39.92 32.93,39.92 32.93,39.93 32.92,39.92</coordinates></LinearRing></outerBoundaryIs></Polygon>
        <Point><coordinates>32.95,39.95</coordinates></Point>
        <Polygon><outerBoundaryIs><LinearRing><coordinates>32.94,39.92 32.95,39.92 32.95,39.93 32.94,39.92</coordinates></LinearRing></outerBoundaryIs></Polygon>
        <Point><coordinates>32.96,39.96,5</coordinates></Point>
      </MultiGeometry>
    </Placemark>
    <Placemark>
      <name>Kapanışı farklı</name>
      <Polygon><altitudeMode>absolute</altitudeMode><outerBoundaryIs><LinearRing><coordinates>32.97,39.90,1 32.98,39.90,2 32.98,39.91,3 32.97,39.90,9</coordinates></LinearRing></outerBoundaryIs></Polygon>
    </Placemark>
  </Folder>
  <Folder>
    <Placemark>
      <name>İz</name>
      <gx:Track><altitudeMode>clampToGround</altitudeMode><when>2026-10-04T10:00:00Z</when><gx:coord>32.70 39.80 900</gx:coord><gx:coord>32.71 39.81 905</gx:coord><gx:coord>32.72 39.81 910</gx:coord></gx:Track>
    </Placemark>
    <Placemark><name>Göreli</name><Point><coordinates>32.72,39.82,15</coordinates></Point></Placemark>
    <Placemark>
      <name>İki iz</name>
      <gx:MultiTrack><gx:Track><gx:coord>32.73 39.83 0</gx:coord><gx:coord>32.74 39.84 0</gx:coord></gx:Track><gx:Track><altitudeMode>absolute</altitudeMode><gx:coord>32.75 39.85 1</gx:coord><gx:coord>32.76 39.86 2</gx:coord><gx:coord>32.77 39.87 3</gx:coord></gx:Track></gx:MultiTrack>
    </Placemark>
  </Folder>
  <Placemark><name>Bozuk</name><LineString><coordinates>32.1,39.1 abc,39.2</coordinates></LineString></Placemark>
  <Placemark><name>Taşkın</name><Point><coordinates>190,39.2</coordinates></Point></Placemark>
  <Placemark><name>Delik bozuk</name><Polygon><outerBoundaryIs><LinearRing><coordinates>32.6,39.6 32.7,39.6 32.7,39.7 32.6,39.6</coordinates></LinearRing></outerBoundaryIs><innerBoundaryIs><LinearRing><coordinates>32.61,39.61 x,y</coordinates></LinearRing></innerBoundaryIs></Polygon></Placemark>
  <Placemark><name>Dış biçem</name><styleUrl>stiller.kml#a</styleUrl><Point><coordinates>32.5,39.5</coordinates></Point></Placemark>
  <NetworkLink><name>Uzak</name><Link><href>http://example.com/a.kml</href></Link></NetworkLink>
  <GroundOverlay><name>Pafta</name><Icon><href>pafta.png</href></Icon><LatLonBox><north>40</north><south>39</south><east>33</east><west>32</west></LatLonBox></GroundOverlay>
  <Placemark><name>Model</name><Model><Location><longitude>32</longitude><latitude>39</latitude></Location></Model></Placemark>
  <Placemark><name>İnce</name><Polygon><outerBoundaryIs><LinearRing><coordinates>32.3,39.3 32.4,39.4 32.4,39.4 32.3,39.3</coordinates></LinearRing></outerBoundaryIs></Polygon></Placemark>
  <gx:Tour><name>Tur</name></gx:Tour>
</Document>
</kml>
"""

ALANLAR = """<?xml version="1.0" encoding="UTF-8"?>
<kml xmlns="http://www.opengis.net/kml/2.2">
<Document>
  <Placemark><name>Meydan</name><Polygon><outerBoundaryIs><LinearRing><coordinates>29.00,41.00 29.01,41.00 29.01,41.01 29.00,41.01 29.00,41.00</coordinates></LinearRing></outerBoundaryIs></Polygon></Placemark>
  <Folder><name>Bina</name>
    <Placemark><name>Kule</name><Style><PolyStyle><color>ff336699</color><outline>0</outline></PolyStyle></Style>
      <Polygon><altitudeMode>relativeToGround</altitudeMode><outerBoundaryIs><LinearRing><coordinates>29.002,41.002,30 29.003,41.002,30 29.003,41.003,30 29.002,41.002,30</coordinates></LinearRing></outerBoundaryIs></Polygon></Placemark>
  </Folder>
</Document>
</kml>
"""


# The sample drawing's parcel 101/7 (its corners as fixtures/gnss/v1/sample.gpx measured them), its west boundary walked
# and a well: what the windows' pictures and the apps' tests import.
PARSEL = """<?xml version="1.0" encoding="UTF-8"?>
<kml xmlns="http://www.opengis.net/kml/2.2">
<Document>
  <name>101/7 parseli</name>
  <Style id="parsel"><LineStyle><color>ff2f8f2f</color><width>2.65</width></LineStyle><PolyStyle><color>662f8f2f</color></PolyStyle></Style>
  <Style id="iz"><LineStyle><color>ff1c6fd9</color><width>1.89</width></LineStyle></Style>
  <Style id="kuyu"><IconStyle><color>ffd98a1c</color></IconStyle></Style>
  <Folder>
    <name>Kadastro</name>
    <Placemark>
      <name>101/7</name>
      <description>Saha ölçüsüyle, 4 Ekim 2026</description>
      <styleUrl>#parsel</styleUrl>
      <ExtendedData><Data name="Ada"><value>101</value></Data><Data name="Parsel"><value>7</value></Data></ExtendedData>
      <Polygon><outerBoundaryIs><LinearRing><coordinates>
        35.842247366,39.915760683 35.842520993,39.915771718 35.842549422,39.916044306 35.842269912,39.916058057 35.842247366,39.915760683
      </coordinates></LinearRing></outerBoundaryIs></Polygon>
    </Placemark>
  </Folder>
  <Folder>
    <name>Saha</name>
    <Placemark>
      <name>Batı sınırı</name>
      <styleUrl>#iz</styleUrl>
      <LineString><altitudeMode>absolute</altitudeMode><coordinates>
        35.842266011,39.916035122,1062.6 35.842261488,39.915967568,1062.28 35.842256964,39.915900015,1061.96 35.842252441,39.915832461,1061.64 35.842247917,39.915764907,1061.32
      </coordinates></LineString>
    </Placemark>
    <Placemark>
      <name>Kuyu</name>
      <styleUrl>#kuyu</styleUrl>
      <Point><altitudeMode>absolute</altitudeMode><coordinates>35.84238,39.91591,1062.1</coordinates></Point>
    </Placemark>
  </Folder>
</Document>
</kml>
"""


def kmz(entries):
    """A zip archive of (name, bytes), stored as KML writers do (deflated)."""
    out = io.BytesIO()
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
        for name, data in entries:
            info = zipfile.ZipInfo(name, date_time=(2026, 10, 11, 12, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(info, data)
    return out.getvalue()


def samples():
    return [
        ("gezi.kml", "klasörler, biçemler, StyleMap, veriler, zaman, izler, karışık MultiGeometry, bozuk ve atlanan öğeler", GEZI.encode("utf-8")),
        ("alanlar.kmz", "KMZ: doc.kml, adsız belge, göreli yükseklik, yalnız dolgu", kmz([("files/simge.png", b"\x89PNG\r\n\x1a\n"), ("doc.kml", ALANLAR.encode("utf-8"))])),
        ("parsel.kml", "örnek çizimin 101/7 parseli, batı sınırı ve kuyu (pencerelerin resimleri ve uygulamaların testleri)", PARSEL.encode("utf-8")),
        ("kml-yok.kmz", "içinde KML olmayan KMZ", kmz([("files/simge.png", b"\x89PNG\r\n\x1a\n")])),
        ("bozuk.kml", "XML değil", b"<kml><Document></kml>\n"),
        ("kok.kml", "KML değil", b'<?xml version="1.0"?>\n<gpx version="1.1"/>\n'),
        ("derin.kml", "256 düzeyden derin", ("<kml>" + "<Folder>" * 255 + "<a/>" + "</Folder>" * 255 + "</kml>\n").encode()),
    ]


# ── Moving (§2) ────────────────────────────────────────────────────────────────


def registry():
    return {e["srid"]: e for e in json.loads(REGISTRY.read_text(encoding="utf-8"))["systems"]}


def moved_obj(o, t):
    def mv(p):
        x, y = t.transform(p[0], p[1])
        return [x, y]

    out = dict(o)
    for key in ("p", "a", "b"):
        if key in o:
            out[key] = mv(o[key])
    if "pts" in o:
        out["pts"] = [mv(p) for p in o["pts"]]
    if "holes" in o:
        out["holes"] = [[mv(p) for p in h] for h in o["holes"]]
    if "parts" in o:
        out["parts"] = [moved_obj(p, t) for p in o["parts"]]
    return out


def vertices(o):
    n = sum(1 for k in ("p", "a", "b") if k in o) + len(o.get("pts", [])) + sum(len(h) for h in o.get("holes", []))
    return n + sum(vertices(p) for p in o.get("parts", []))


def moves(reads):
    reg = registry()
    out = []
    for file, srid in (("gezi.kml", 5255), ("gezi.kml", 2321), ("alanlar.kmz", 5253), ("parsel.kml", 5256)):
        src, dst = reg[4326], reg[srid]
        t, accuracy, via = transformer(src, dst)
        got = next(r for r in reads if r["file"] == file)["expect"]
        objects = [moved_obj(o, t) for o in got["objects"]]
        out.append({
            "file": file,
            "srid": srid,
            "system": system(dst),
            "objects": objects,
            "moved": {
                "objects": len(objects),
                "vertices": sum(vertices(o) for o in objects),
                "accuracy": accuracy if via else None,
                "via": via,
                "unofficial": "ED50" in (src["datum"], dst["datum"]),
            },
        })
    return out


# ── Writing (§4): inputs, and the files the writer wrote, checked ────────────────


def write_inputs():
    """The writer's cases: the same objects (TUREF / TM33 numbers near Ankara) from three systems."""
    def pt(x, y):
        return {"x": x, "y": y}

    e0, n0 = 488000.0, 4420000.0
    entities = [
        {"kind": "point", "id": 1, "layerId": "noktalar", "label": "N1", "attrs": {"Ad": "P-1", "Kod": "PN", "Açıklama": "Poligon noktası"},
         "p": pt(e0, n0), "z": 850.125},
        {"kind": "point", "id": 2, "layerId": "noktalar", "attrs": {"Ad": "P-2"}, "p": pt(e0 + 25.5, n0 + 10.25)},
        {"kind": "point", "id": 3, "layerId": "noktalar", "attrs": {}, "p": pt(e0 + 40, n0 + 5), "z": 851.0,
         "parts": [{"p": pt(e0 + 45, n0 + 6), "z": 852.5}, {"p": pt(e0 + 50, n0 + 7)}]},
        {"kind": "line", "id": 4, "layerId": "yollar", "attrs": {"Ad": "Kısa"}, "a": pt(e0, n0 + 100), "b": pt(e0 + 60, n0 + 130),
         "za": 840.0, "zb": 842.5},
        {"kind": "polyline", "id": 5, "layerId": "yollar", "attrs": {"Ad": "Ana yol", "Şerit": "4"}, "color": "#d04020", "lineWeight": 0.7,
         "pts": [pt(e0, n0 + 200), pt(e0 + 100, n0 + 210), pt(e0 + 200, n0 + 250)], "zs": [830.0, 831.5, 833.0]},
        {"kind": "polyline", "id": 6, "layerId": "yollar", "attrs": {}, "pts": [pt(e0, n0 + 300), pt(e0 + 50, n0 + 310), pt(e0 + 90, n0 + 330)],
         "zs": [830.0, None, 833.0]},
        {"kind": "polyline", "id": 7, "layerId": "yollar", "attrs": {"Ad": "İki parça"}, "pts": [pt(e0, n0 + 400), pt(e0 + 30, n0 + 420)],
         "parts": [{"pts": [pt(e0 + 50, n0 + 400), pt(e0 + 80, n0 + 410), pt(e0 + 90, n0 + 440)]}]},
        {"kind": "polygon", "id": 8, "layerId": "parsel", "label": "101/7", "attrs": {"Ada": "101", "Parsel": "7"}, "color": "#ff8800", "lineWeight": 0.5,
         "pts": [pt(e0 + 300, n0), pt(e0 + 300, n0 + 80), pt(e0 + 380, n0 + 80), pt(e0 + 380, n0)],
         "holes": [{"pts": [pt(e0 + 320, n0 + 20), pt(e0 + 340, n0 + 20), pt(e0 + 340, n0 + 40), pt(e0 + 320, n0 + 40)]}]},
        {"kind": "polygon", "id": 9, "layerId": "parsel", "attrs": {"Ada": "101", "Parsel": "8"},
         "pts": [pt(e0 + 400, n0), pt(e0 + 450, n0), pt(e0 + 450, n0 + 50)], "zs": [860.0, 861.0, 862.0],
         "parts": [{"pts": [pt(e0 + 460, n0), pt(e0 + 500, n0), pt(e0 + 500, n0 + 40), pt(e0 + 460, n0 + 40)]}]},
        {"kind": "polygon", "id": 10, "layerId": "yapi", "attrs": {"Kat": "3"}, "pts": [pt(e0 + 600, n0), pt(e0 + 630, n0), pt(e0 + 630, n0 + 20), pt(e0 + 600, n0 + 20)]},
        {"kind": "circle", "id": 11, "layerId": "egriler", "attrs": {"Ad": "Kuyu"}, "c": pt(e0 + 700, n0 + 50), "r": 12.5},
        {"kind": "text", "id": 12, "layerId": "yazilar", "attrs": {}, "text": "Ankara & <Merkez>", "p": pt(e0 + 750, n0 + 60), "height": 2.5, "rotation": 0.0},
        {"kind": "point", "id": 13, "layerId": "kayip", "attrs": {"Ad": "Katmansız"}, "p": pt(e0 + 800, n0)},
    ]
    layers = [
        {"id": "noktalar", "name": "Noktalar", "color": "#0044cc"},
        {"id": "yollar", "name": "Yollar", "path": ["Ulaşım"], "color": "#555555", "lineWeight": 0.35},
        {"id": "parsel", "name": "Parseller", "path": ["Kadastro", "Ada 101"], "color": "#00aa00", "lineWeight": 0.25},
        {"id": "yapi", "name": "Yapılar", "path": ["Kadastro"], "color": "#aa0000"},
        {"id": "bos", "name": "Boş katman", "color": "#123456"},
        {"id": "egriler", "name": "Eğriler", "color": "#888888", "lineWeight": 0.18},
        {"id": "yazilar", "name": "Yazılar", "color": "ink"},
    ]
    reg = registry()
    cases = []
    for name, srid, kmz_out in (("tm33", 5255, False), ("ed50", 2321, True), ("mercator", 3857, False)):
        cases.append((name, {
            "srid": srid,
            "system": system(reg[srid]),
            "choices": [],
            "input": {"name": "Örnek çizim", "layers": layers, "entities": entities, "kmz": kmz_out},
        }))
    return cases


def area2(ring):
    total = 0.0
    for i in range(len(ring)):
        a, b = ring[i], ring[(i + 1) % len(ring)]
        total += a[0] * b[1] - b[0] * a[1]
    return total


def turned(ring, sign):
    """A ring the right-hand way (sign +1 counter-clockwise, -1 clockwise), its first vertex kept."""
    return [ring[0], *ring[:0:-1]] if area2([p[:2] for p in ring]) * sign < 0 else ring


def written_geometry(e):
    """What the writer writes for an input object: ("Point" | "LineString" | "Polygon" | "Multi", data), vertices as
    (x, y, z or None) in the project's system; None when it writes nothing. Circles: ("Circle", (c, r))."""
    kind = e["kind"]

    def v(p, z=None):
        return (float(p["x"]), float(p["y"]), None if z is None else float(z))

    def path(pts, zs):
        zs = zs if zs is not None and len(zs) == len(pts) else [None] * len(pts)
        return [v(p, z) for p, z in zip(pts, zs)]

    if kind == "text":
        return ("Point", v(e["p"]))
    if kind == "point":
        pts = [v(e["p"], e.get("z"))] + [v(q["p"], q.get("z")) for q in e.get("parts", [])]
        return ("Point", pts[0]) if len(pts) == 1 else ("Multi", [("Point", p) for p in pts])
    if kind == "line":
        return ("LineString", [v(e["a"], e.get("za")), v(e["b"], e.get("zb"))])
    if kind == "polyline":
        paths = [path(e["pts"], e.get("zs"))] + [path(q["pts"], q.get("zs")) for q in e.get("parts", [])]
        return ("LineString", paths[0]) if len(paths) == 1 else ("Multi", [("LineString", p) for p in paths])
    if kind == "polygon":
        def area(o):
            rings = [turned(path(o["pts"], o.get("zs")), 1)]
            rings += [turned(path(h["pts"], h.get("zs")), -1) for h in o.get("holes", [])]
            return [r + [r[0]] for r in rings]

        areas = [area(e)] + [area(q) for q in e.get("parts", [])]
        return ("Polygon", areas[0]) if len(areas) == 1 else ("Multi", [("Polygon", a) for a in areas])
    if kind == "circle":
        return ("Circle", ((float(e["c"]["x"]), float(e["c"]["y"])), float(e["r"])))
    return None


def has_all_z(geom):
    kind, data = geom
    if kind == "Point":
        return data[2] is not None
    if kind == "LineString":
        return all(p[2] is not None for p in data)
    if kind == "Polygon":
        return all(p[2] is not None for r in data for p in r)
    if kind == "Multi":
        return all(has_all_z(m) for m in data)
    return False


def to_kml_color(color, alpha="ff"):
    h = color[1:] if color.startswith("#") else None
    if h is None or len(h) not in (3, 6) or any(c not in "0123456789abcdefABCDEF" for c in h):
        return None
    if len(h) == 3:
        h = "".join(c * 2 for c in h)
    h = h.lower()
    return alpha + h[4:6] + h[2:4] + h[0:2]


DEGREES = 1.5e-10


def check_coords(node, want, t, inverse, where, problems, three_d):
    """A coordinates element against the vertices it should hold (project coordinates moved by PROJ)."""
    got = (text(node) or "").split()
    if len(got) != len(want):
        problems.append(f"{where}: {len(got)} konum, beklenen {len(want)}")
        return
    for i, (g, w) in enumerate(zip(got, want)):
        values = [float(x) for x in g.split(",")]
        lon, lat = t.transform(w[0], w[1])
        if abs(values[0] - lon) > DEGREES or abs(values[1] - lat) > DEGREES:
            problems.append(f"{where}: {i}. konum {values[:2]}, PROJ {[lon, lat]}")
        if three_d:
            if len(values) != 3 or values[2] != w[2]:
                problems.append(f"{where}: {i}. konumun yüksekliği {values[2:]} , beklenen {w[2]}")
        elif len(values) != 2:
            problems.append(f"{where}: {i}. konumda yükseklik yazılmış")


def check_geometry(node, geom, t, inverse, where, problems, three_d):
    kind, data = geom
    if kind == "Circle":
        # The app's sampling: 72 points a turn from angle 0, counter-clockwise, closed (the first repeated).
        (cx, cy), r = data
        if node.tag != "LineString":
            problems.append(f"{where}: daire LineString değil ({node.tag})")
            return
        ring = [(cx + math.cos(k / 72 * 2 * math.pi) * r, cy + math.sin(k / 72 * 2 * math.pi) * r, None) for k in range(72)]
        check_coords(node.child("coordinates"), ring + [ring[0]], t, inverse, where, problems, False)
        return
    if node.tag != ("MultiGeometry" if kind == "Multi" else kind):
        problems.append(f"{where}: {node.tag}, beklenen {kind}")
        return
    mode = text(node.child("altitudeMode"))
    if kind != "Multi" and (mode == "absolute") != three_d:
        problems.append(f"{where}: altitudeMode {mode!r}, kotlu: {three_d}")
    if kind == "Point":
        check_coords(node.child("coordinates"), [data], t, inverse, where, problems, three_d)
    elif kind == "LineString":
        check_coords(node.child("coordinates"), data, t, inverse, where, problems, three_d)
    elif kind == "Polygon":
        outer = node.child("outerBoundaryIs")
        inners = [c for c in node.children if c.tag == "innerBoundaryIs"]
        if outer is None or len(inners) != len(data) - 1:
            problems.append(f"{where}: halkalar beklenenler değil")
            return
        for k, (b, ring) in enumerate(zip([outer, *inners], data)):
            check_coords(b.child("LinearRing").child("coordinates"), ring, t, inverse, f"{where} {k}. halka", problems, three_d)
    elif kind == "Multi":
        members = node.children
        if len(members) != len(data):
            problems.append(f"{where}: {len(members)} üye, beklenen {len(data)}")
            return
        for k, (m, g) in enumerate(zip(members, data)):
            check_geometry(m, g, t, inverse, f"{where} {k}. üye", problems, three_d)


def check_written(name, case, data):
    """What is wrong with the file the writer wrote for a case."""
    problems = []
    reg = registry()
    src, dst = reg[case["srid"]], reg[4326]
    t, _, _ = transformer(src, dst)
    inverse, _, _ = transformer(dst, src)
    if system(src) != case["system"]:
        problems.append(f"{name}: girdinin sistemi kayıttakiyle aynı değil")
    spec = case["input"]
    if spec.get("kmz"):
        with zipfile.ZipFile(io.BytesIO(data)) as z:
            if z.namelist() != ["doc.kml"]:
                return [f"{name}: KMZ'de yalnız doc.kml olmalı: {z.namelist()}"]
            data = z.read("doc.kml")
    root = parse(data)
    if root.tag != "kml" or not data.decode("utf-8").startswith('<?xml version="1.0" encoding="UTF-8"?>\n<kml xmlns="http://www.opengis.net/kml/2.2">'):
        return [f"{name}: kök ya da ad alanı KML 2.2'ninki değil"]
    doc = root.child("Document")
    if doc is None or text(doc.child("name")) != spec["name"]:
        return [f"{name}: Document ya da adı beklenen değil"]
    styles = {s.attrs["id"]: s for s in doc.children if s.tag == "Style"}
    if list(styles) != [f"s{i + 1}" for i in range(len(styles))]:
        problems.append(f"{name}: biçemler s1, s2 … sırasıyla değil: {list(styles)}")
    layers = {l["id"]: l for l in spec["layers"]}
    order = [l["id"] for l in spec["layers"]]
    order += [e["layerId"] for e in spec["entities"] if e["layerId"] not in layers and e["layerId"] not in order]
    # The expected folder tree: (path, layer name, [entities]).
    expected = []
    for lid in order:
        written = [e for e in spec["entities"] if e["layerId"] == lid and written_geometry(e) is not None]
        if written:
            layer = layers.get(lid, {"name": lid, "path": []})
            expected.append((layer.get("path", []), layer["name"], written))
    found = []

    def walk(n, path):
        for c in n.children:
            if c.tag == "Folder":
                placemarks = [p for p in c.children if p.tag == "Placemark"]
                if placemarks:
                    found.append((path, text(c.child("name")), placemarks))
                walk(c, path + [text(c.child("name"))])

    walk(doc, [])
    if [(p, n) for p, n, _ in found] != [(p, n) for p, n, _ in expected]:
        return problems + [f"{name}: klasörler {[(p, n) for p, n, _ in found]}, beklenen {[(p, n) for p, n, _ in expected]}"]
    used = []
    for (_, lname, placemarks), (_, _, entities) in zip(found, expected):
        if len(placemarks) != len(entities):
            problems.append(f"{name}/{lname}: {len(placemarks)} yer imi, beklenen {len(entities)}")
            continue
        layer = layers.get(entities[0]["layerId"], {})
        for pm, e in zip(placemarks, entities):
            where = f"{name}/{lname}/{e['id']}"
            want_name = e["text"] if e["kind"] == "text" else (e.get("label") or e["attrs"].get("Ad"))
            if text(pm.child("name")) != want_name:
                problems.append(f"{where}: ad {text(pm.child('name'))!r}, beklenen {want_name!r}")
            data_items = [(d.attrs.get("name"), text(d.child("value")) or "") for d in (pm.child("ExtendedData").children if pm.child("ExtendedData") is not None else [])]
            if data_items != sorted((k, v) for k, v in e["attrs"].items()):
                problems.append(f"{where}: veriler {data_items}, beklenen {sorted(e['attrs'].items())}")
            geom = written_geometry(e)
            node = next((c for c in pm.children if c.tag in ("Point", "LineString", "Polygon", "MultiGeometry")), None)
            if node is None:
                problems.append(f"{where}: geometri yok")
                continue
            check_geometry(node, geom, t, inverse, where, problems, has_all_z(geom))
            # The style: colour, width, and which parts.
            url = text(pm.child("styleUrl")) or ""
            style = styles.get(url[1:])
            if style is None:
                problems.append(f"{where}: biçemi {url!r} yok")
                continue
            used.append(url[1:])
            color = to_kml_color(e["color"]) if e.get("color") else None
            color = color or to_kml_color(layer.get("color", "")) or "ff000000"
            lw = e.get("lineWeight", layer.get("lineWeight", 0.25))
            width = max(round(lw / MM_PER_PIXEL * 100), 1) / 100 if lw > 0 else 1.0
            line = style.child("LineStyle")
            if line is None or text(line.child("color")) != color or float(text(line.child("width"))) != width:
                problems.append(f"{where}: çizgi biçemi {color} {width} değil")
            is_area = geom[0] == "Polygon" or (geom[0] == "Multi" and any(m[0] == "Polygon" for m in geom[1]))
            is_point = geom[0] == "Point" or (geom[0] == "Multi" and any(m[0] == "Point" for m in geom[1]))
            poly = style.child("PolyStyle")
            if is_area != (poly is not None) or (poly is not None and text(poly.child("color")) != "66" + color[2:]):
                problems.append(f"{where}: dolgu biçemi beklenen değil")
            if is_point != (style.child("IconStyle") is not None):
                problems.append(f"{where}: simge biçemi beklenen değil")
    if sorted(set(used)) != sorted(styles):
        problems.append(f"{name}: kullanılmayan biçem var")
    return problems


# ── Main ───────────────────────────────────────────────────────────────────────


def build():
    reads = []
    for file, title, data in samples():
        reads.append({"file": file, "name": title, "expect": read(data)})
    return {
        "format": "kentos.kml",
        "version": 1,
        "source": "scripts/fixtures/kml_cases.py (docs/adr/0222; OGC KML 2.2, expat, zipfile, PROJ)",
        "tolerance": {"metres": 1e-6, "degrees": DEGREES},
        "reads": reads,
        "moves": moves(reads),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true", help="compare with the files instead of writing them")
    args = parser.parse_args()
    files = {DIR / file: data for file, _, data in samples()}
    files[DIR / "cases.json"] = (json.dumps(build(), ensure_ascii=False, indent=2) + "\n").encode("utf-8")
    for name, case in write_inputs():
        files[WRITE / f"{name}.case.json"] = (json.dumps(case, ensure_ascii=False, indent=2) + "\n").encode("utf-8")
    if args.check:
        problems = [f"{p.relative_to(ROOT)} kurallardan çıkanla aynı değil" for p, data in files.items() if not p.exists() or p.read_bytes() != data]
        for name, case in write_inputs():
            out = WRITE / f"{name}.{'kmz' if case['input'].get('kmz') else 'kml'}"
            if not out.exists():
                problems.append(f"{out.relative_to(ROOT)} yok (KENTOS_WRITE_KML=1 cargo test -p kentos-formats kml)")
                continue
            problems += check_written(name, case, out.read_bytes())
        if problems:
            print("\n".join(problems))
            print("Betiği --check olmadan çalıştırıp farkı okuyun.")
            sys.exit(1)
        print(f"{DIR.relative_to(ROOT)}: okumalar, taşımalar ve yazıcının {len(write_inputs())} dosyası kurallarla aynı.")
        return
    WRITE.mkdir(parents=True, exist_ok=True)
    for path, data in files.items():
        path.write_bytes(data)
    print(f"{DIR.relative_to(ROOT)} yazıldı: {len(build()['reads'])} okuma, {len(build()['moves'])} taşıma, {len(write_inputs())} yazma girdisi.")


if __name__ == "__main__":
    main()
