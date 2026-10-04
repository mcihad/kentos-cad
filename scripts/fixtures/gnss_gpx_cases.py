#!/usr/bin/env python3
"""Independent reference of the GPX 1.1 GNSS file's reading (docs/adr/0169 §1, §6; step 6a).

Writes fixtures/gnss/v1/gpx.json from the GPX 1.1 schema alone (https://www.topografix.com/GPX/1/1/gpx.xsd and its
annotations), no KentOS code, with Python's own XML parser (expat): a file's waypoints, route points and track points
with their WGS 84 latitude and longitude, heights, fix, satellites, HDOP, name and time.

The rules:

1. The file is XML (UTF-8); one that is not well-formed, or whose elements nest deeper than 256 levels (the root is level
   1), is said at line 1 and not read. Its root element must be gpx; otherwise said at the root's line and not read.
   Lines are the elements' start tags'.
2. Points in the file's order: wpt (the root's children), rtept (in rte), trkpt (in trkseg in trk). A point's attributes
   lat (−90 to 90) and lon (−180 to 180) are decimals (an optional sign, digits with an optional point, or a point and
   digits, at most 30 digits); missing, not decimals or out of range: said, the point not read. Its children (each element's text
   trimmed, the first of a name): ele and geoidheight (decimals; geoidheight is the geoid's height above the WGS 84
   ellipsoid, as NMEA's GGA defines it), name, time (as written), fix (none, 2d, 3d, dgps, pps), sat (ASCII digits, at
   most 4294967295),
   hdop (a decimal). A child that is not so leaves the point out, said.
3. A point whose fix is none had no fix: counted and said once, at its first line.
4. A point's height is ele as written; with geoidheight its ellipsoidal height is ele plus geoidheight (exactly, rounded
   once). Its fix is named 2B, 3B, DGPS or PPS; another text is kept as written.
5. A decimal is its exact value rounded once to the nearest float64.
"""

import argparse
import json
import re
import sys
import xml.parsers.expat
from fractions import Fraction
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "gnss" / "v1" / "gpx.json"
TEXTS = {
    "xml": "Satır 1: dosya XML olarak okunamadı; GNSS noktaları okunmadı.",
    "deep": "Satır 1: XML öğeleri {depth} düzeyden derin iç içe; GNSS noktaları okunmadı.",
    "root": "Satır {line}: XML dosyası GPX değil (kök öğe {root}); GNSS noktaları okunmadı.",
    "position": "Satır {line}: konum (lat “{lat}”, lon “{lon}”) okunmuyor; nokta okunmadı.",
    "value": "Satır {line}: {what} “{data}” okunmuyor; nokta okunmadı.",
    "nofix": "Satır {line}: konumu olmayan noktalar ({count} nokta, fix none) okunmadı.",
}
DECIMAL = re.compile(r"[+-]?([0-9]+(\.[0-9]*)?|\.[0-9]+)")
FIX = {"2d": "2B", "3d": "3B", "dgps": "DGPS", "pps": "PPS"}
DEPTH = 256


class TooDeep(Exception):
    pass


class Node:
    def __init__(self, tag, attrs, line):
        self.tag, self.attrs, self.line, self.children, self.parts = tag, attrs, line, [], []

    @property
    def text(self):
        return "".join(self.parts).strip()

    def child(self, tag):
        return next((c for c in self.children if c.tag == tag), None)


def local(tag):
    return tag.rsplit(":", 1)[-1]


def parse(text):
    parser = xml.parsers.expat.ParserCreate()
    stack, root = [], []

    def start(tag, attrs):
        if len(stack) + 1 > DEPTH:
            raise TooDeep()
        node = Node(local(tag), attrs, parser.CurrentLineNumber)
        if stack:
            stack[-1].children.append(node)
        else:
            root.append(node)
        stack.append(node)

    def end(tag):
        stack.pop()

    def data(s):
        if stack:
            stack[-1].parts.append(s)

    parser.StartElementHandler, parser.EndElementHandler, parser.CharacterDataHandler = start, end, data
    parser.Parse(text.encode("utf-8"), True)
    return root[0]


class Unread(Exception):
    def __init__(self, what, data):
        super().__init__(what)
        self.what, self.data = what, data


def is_decimal(t):
    return bool(DECIMAL.fullmatch(t)) and sum(c.isdigit() for c in t) <= 30


def decimal(what, node):
    if node is None:
        return None
    t = node.text
    if not is_decimal(t):
        raise Unread(what, t)
    return Fraction(t)


def points_of(root):
    for c in root.children:
        if c.tag == "wpt":
            yield "wpt", c
        elif c.tag == "rte":
            for p in c.children:
                if p.tag == "rtept":
                    yield "rtept", p
        elif c.tag == "trk":
            for seg in c.children:
                if seg.tag == "trkseg":
                    for p in seg.children:
                        if p.tag == "trkpt":
                            yield "trkpt", p


def read(text):
    try:
        root = parse(text)
    except xml.parsers.expat.ExpatError:
        return {"points": [], "problems": [{"line": 1, "problem": TEXTS["xml"]}]}
    except TooDeep:
        return {"points": [], "problems": [{"line": 1, "problem": TEXTS["deep"].format(depth=DEPTH)}]}
    if root.tag != "gpx":
        return {"points": [], "problems": [{"line": root.line, "problem": TEXTS["root"].format(line=root.line, root=root.tag)}]}
    points, problems = [], []
    nofix = None
    for kind, n in points_of(root):
        line = n.line
        lat_t, lon_t = n.attrs.get("lat", ""), n.attrs.get("lon", "")
        if not is_decimal(lat_t) or not is_decimal(lon_t) or abs(Fraction(lat_t)) > 90 or abs(Fraction(lon_t)) > 180:
            problems.append({"line": line, "problem": TEXTS["position"].format(line=line, lat=lat_t, lon=lon_t)})
            continue
        try:
            ele = decimal("ele", n.child("ele"))
            geoid = decimal("geoidheight", n.child("geoidheight"))
            hdop = decimal("hdop", n.child("hdop"))
            sat = n.child("sat")
            if sat is not None and not (sat.text.isascii() and sat.text.isdigit() and int(sat.text) <= 0xFFFFFFFF):
                raise Unread("sat", sat.text)
        except Unread as e:
            problems.append({"line": line, "problem": TEXTS["value"].format(line=line, what=e.what, data=e.data)})
            continue
        fix = n.child("fix")
        if fix is not None and fix.text == "none":
            if nofix is None:
                nofix = [line, 0]
            nofix[1] += 1
            continue
        p = {"kind": kind}
        name = n.child("name")
        if name is not None and name.text:
            p["name"] = name.text
        p["lat"] = float(Fraction(lat_t))
        p["lon"] = float(Fraction(lon_t))
        if ele is not None:
            p["height"] = float(ele)
            if geoid is not None:
                p["ellipsoidal"] = float(ele + geoid)
        if geoid is not None:
            p["geoid"] = float(geoid)
        time = n.child("time")
        if time is not None and time.text:
            p["time"] = time.text
        if fix is not None and fix.text:
            p["fix"] = FIX.get(fix.text, fix.text)
        if sat is not None:
            p["satellites"] = int(sat.text)
        if hdop is not None:
            p["hdop"] = float(hdop)
        p["line"] = line
        points.append(p)
    if nofix:
        problems.append({"line": nofix[0], "problem": TEXTS["nofix"].format(line=nofix[0], count=nofix[1])})
        problems.sort(key=lambda p: p["line"])
    return {"points": points, "problems": problems}


def cases():
    good = "\n".join(
        [
            '<?xml version="1.0" encoding="UTF-8"?>',
            '<gpx version="1.1" creator="KentOS" xmlns="http://www.topografix.com/GPX/1/1">',
            '  <wpt lat="40.7539083" lon="29.3853900">',
            "    <ele>105.234</ele><time>2026-10-04T10:15:00Z</time><geoidheight>36.123</geoidheight>",
            "    <name>N1</name><fix>3d</fix><sat>14</sat><hdop>0.6</hdop>",
            "  </wpt>",
            '  <wpt lat="-0.5" lon="+179.999"><name> SINIR </name><fix>dgps</fix></wpt>',
            "  <rte><name>Yol</name>",
            '    <rtept lat="40.75" lon="29.38"><ele>100</ele></rtept>',
            "  </rte>",
            "  <trk><name>İz</name><trkseg>",
            '    <trkpt lat="40.7540" lon="29.3854"><ele>104.9</ele><time>2026-10-04T10:16:00Z</time></trkpt>',
            '    <trkpt lat=".5" lon="-.5"><fix>rtk</fix></trkpt>',
            "  </trkseg></trk>",
            "</gpx>",
        ]
    ) + "\n"
    broken = "\n".join(
        [
            '<?xml version="1.0"?>',
            "<gpx>",
            '  <wpt lat="91" lon="29"/>',
            '  <wpt lat="40" lon="-181"/>',
            '  <wpt lon="29"/>',
            '  <wpt lat="4e1" lon="29"/>',
            '  <wpt lat="40" lon="29"><ele>1,5</ele></wpt>',
            '  <wpt lat="40" lon="29"><sat>-3</sat></wpt>',
            '  <wpt lat="40" lon="29"><hdop>x</hdop></wpt>',
            '  <wpt lat="40" lon="29"><fix>none</fix></wpt>',
            '  <wpt lat="41" lon="29"><fix>none</fix></wpt>',
            '  <wpt lat="40" lon="29"><geoidheight>36</geoidheight></wpt>',
            "</gpx>",
        ]
    ) + "\n"
    return [
        ("yol noktası, rota ve iz noktaları, geoit yüksekliği, fix, uydu, HDOP, ad ve zaman", good),
        ("bozuk konumlar ve değerler, konumu olmayan noktalar, yükseksiz geoit", broken),
        ("XML değil", "<gpx><wpt></gpx>\n"),
        ("GPX değil", '<?xml version="1.0"?>\n<JOBFile version="5.3"/>\n'),
        ("256 düzeyden derin", "<gpx>" + "<a>" * 255 + "<b/>" + "</a>" * 255 + "</gpx>\n"),
    ]


def build():
    out = [{"name": name, "text": text, "expect": read(text)} for name, text in cases()]
    return {"format": "kentos.gnss-gpx", "version": 1, "source": "scripts/fixtures/gnss_gpx_cases.py (docs/adr/0169 §1; GPX 1.1 schema)", "texts": TEXTS, "cases": out}


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
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(build()['cases'])} durum.")


if __name__ == "__main__":
    main()
