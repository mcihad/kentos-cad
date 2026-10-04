#!/usr/bin/env python3
"""Independent reference of the Trimble JobXML field book's reading (docs/adr/0169 §1; step 5c).

Writes fixtures/field/v1/jobxml.json from Trimble's description alone (the JobXML schema, version 5.3,
https://ww2.trimble.com/schema/jobxml/5_3/jobxmlschema-5.3.xsd, and its annotations), no KentOS code: the FieldBook's
records read into the book's stations and observations, with Python's own XML parser (expat).

The rules:

1. The file is XML (UTF-8); one that is not well-formed, or whose elements nest deeper than 256 levels (the root is level
   1; a guard for the readers' parsers), is said at line 1 and not read. Its root element must be JOBFile
   and have a FieldBook child (the first); otherwise said at the root's line and not read. Lines are the elements'
   start tags'.
2. All angles are decimal degrees and all distances metres (the schema's units): the book's unit is degrees.
3. The FieldBook's records in order (each element's text trimmed):
   - TargetRecord (attribute ID): its TargetHeight (blank: none) is the height of the observations that name its ID.
   - StationRecord (attribute ID): begins a station named StationName with its TheodoliteHeight (blank: none); its
     coordinates those of the last PointRecord of its name with a Grid read before it (East, North, Elevation).
   - PointRecord: one whose Deleted is "true" is passed over. Its Grid (North, East, Elevation) gives its Name's
     coordinates. With a Circle it is an observation when its Method is DirectReading, AverageMeasurements, AngleOnly or
     HorizontalAngleOnly (raw readings); any other method with a Circle (offsets, computed) is said and not read.
     An observation: its values HorizontalCircle, VerticalCircle, EDMDistance (in that order: one that is not a number,
     an angle not from 0 up to 360, a distance below zero leaves it out, said); its station the StationRecord of its
     StationID (none: said, not read); its horizontal reading (none: said, not read); its target height the
     TargetRecord's of its TargetID (a TargetID that names none: said, the observation read without one); its target
     Name, its code Code.
   - Other records are passed over.
4. A number: an optional sign, digits with an optional point, or a point and digits, an optional exponent (the schema's
   double); its value is the float64 nearest to it. INF and NaN are not numbers.
"""

import argparse
import json
import re
import sys
import xml.parsers.expat
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "field" / "v1" / "jobxml.json"
TEXTS = {
    "xml": "Satır 1: dosya XML olarak okunamadı; karne okunmadı.",
    "deep": "Satır 1: XML öğeleri {depth} düzeyden derin iç içe; karne okunmadı.",
    "root": "Satır {line}: XML dosyası JobXML değil (kök öğe {root}); karne okunmadı.",
    "fieldbook": "Satır {line}: JobXML dosyasında FieldBook yok; karne okunmadı.",
    "value": "Satır {line}: “{data}” sayı değil; kayıt okunmadı.",
    "turn": "Satır {line}: açı “{data}” sıfırla 360 derece arasında değil; kayıt okunmadı.",
    "negative": "Satır {line}: eğik uzunluk “{data}” sıfırdan küçük; kayıt okunmadı.",
    "method": "Satır {line}: {name} gözlemi {method} yöntemiyle; ham gözlem değil, okunmadı.",
    "station": "Satır {line}: {name} gözleminin istasyonu (StationID {id}) yok; okunmadı.",
    "horizontal": "Satır {line}: {name} gözleminin yatay açısı yok; okunmadı.",
    "target": "Satır {line}: {name} gözleminin prizma kaydı (TargetID {id}) yok; prizma yüksekliği okunmadı.",
}
NUMBER = re.compile(r"[+-]?([0-9]+(\.[0-9]*)?|\.[0-9]+)([eE][+-]?[0-9]+)?")
RAW = {"DirectReading", "AverageMeasurements", "AngleOnly", "HorizontalAngleOnly"}
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

    def value(self, tag):
        c = self.child(tag)
        return c.text if c is not None else ""


def parse(text):
    parser = xml.parsers.expat.ParserCreate()
    stack, root = [], []

    def start(tag, attrs):
        if len(stack) + 1 > DEPTH:
            raise TooDeep()
        node = Node(tag, attrs, parser.CurrentLineNumber)
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
    def __init__(self, key, data):
        super().__init__(key)
        self.key, self.data = key, data


def number(data):
    if data == "":
        return None
    if not NUMBER.fullmatch(data):
        raise Unread("value", data)
    return float(data)


def angle(data):
    v = number(data)
    if v is not None and not (0 <= v < 360):
        raise Unread("turn", data)
    return v


def read(text):
    stations, problems = [], []
    try:
        root = parse(text)
    except xml.parsers.expat.ExpatError:
        return {"unit": None, "stations": [], "problems": [{"line": 1, "problem": TEXTS["xml"]}]}
    except TooDeep:
        return {"unit": None, "stations": [], "problems": [{"line": 1, "problem": TEXTS["deep"].format(depth=DEPTH)}]}
    if root.tag != "JOBFile":
        return {"unit": None, "stations": [], "problems": [{"line": root.line, "problem": TEXTS["root"].format(line=root.line, root=root.tag)}]}
    book = root.child("FieldBook")
    if book is None:
        return {"unit": None, "stations": [], "problems": [{"line": root.line, "problem": TEXTS["fieldbook"].format(line=root.line)}]}
    coords, by_id, targets = {}, {}, {}
    for r in book.children:
        line = r.line
        try:
            if r.tag == "TargetRecord":
                targets[r.attrs.get("ID", "")] = number(r.value("TargetHeight"))
            elif r.tag == "StationRecord":
                hi = number(r.value("TheodoliteHeight"))
                name = r.value("StationName")
                s = {"station": name}
                if hi is not None:
                    s["instrumentHeight"] = hi
                for key, v in zip(("east", "north", "height"), coords.get(name, (None, None, None))):
                    if v is not None:
                        s[key] = v
                s["observations"] = []
                stations.append(s)
                by_id[r.attrs.get("ID", "")] = s
            elif r.tag == "PointRecord":
                if r.value("Deleted") == "true":
                    continue
                name = r.value("Name")
                grid = r.child("Grid")
                if grid is not None:
                    n, e, z = number(grid.value("North")), number(grid.value("East")), number(grid.value("Elevation"))
                    coords[name] = (e, n, z)
                circle = r.child("Circle")
                if circle is None:
                    continue
                method = r.value("Method")
                if method not in RAW:
                    problems.append({"line": line, "problem": TEXTS["method"].format(line=line, name=name, method=method)})
                    continue
                hz = angle(circle.value("HorizontalCircle"))
                zenith = angle(circle.value("VerticalCircle"))
                slope = number(circle.value("EDMDistance"))
                if slope is not None and slope < 0:
                    raise Unread("negative", circle.value("EDMDistance"))
                station_id = r.value("StationID")
                s = by_id.get(station_id)
                if s is None:
                    problems.append({"line": line, "problem": TEXTS["station"].format(line=line, name=name, id=station_id)})
                    continue
                if hz is None:
                    problems.append({"line": line, "problem": TEXTS["horizontal"].format(line=line, name=name)})
                    continue
                target_id = r.value("TargetID")
                th = None
                if target_id:
                    if target_id in targets:
                        th = targets[target_id]
                    else:
                        problems.append({"line": line, "problem": TEXTS["target"].format(line=line, name=name, id=target_id)})
                o = {"target": name, "hz": hz}
                if zenith is not None:
                    o["zenith"] = zenith
                if slope is not None:
                    o["slope"] = slope
                if th is not None:
                    o["targetHeight"] = th
                code = r.value("Code")
                if code:
                    o["code"] = code
                o["line"] = line
                s["observations"].append(o)
        except Unread as e:
            problems.append({"line": line, "problem": TEXTS[e.key].format(line=line, data=e.data)})
    return {"unit": "deg", "stations": stations, "problems": problems}


def point(id_, name, method, circle=None, grid=None, station="", target="", code="", deleted="false", classification="Normal"):
    parts = [f'    <PointRecord ID="{id_}" TimeStamp="2026-10-04T10:00:00">', f"      <Name>{name}</Name>", f"      <Code>{code}</Code>", f"      <Method>{method}</Method>", "      <SurveyMethod>Fix</SurveyMethod>", f"      <Classification>{classification}</Classification>", f"      <Deleted>{deleted}</Deleted>"]
    if grid is not None:
        n, e, z = grid
        parts.append(f"      <Grid><North>{n}</North><East>{e}</East><Elevation>{z}</Elevation></Grid>")
    if circle is not None:
        hz, v, sd, face = circle
        parts.append(f"      <Circle><HorizontalCircle>{hz}</HorizontalCircle><VerticalCircle>{v}</VerticalCircle><EDMDistance>{sd}</EDMDistance><Face>{face}</Face></Circle>")
    if station:
        parts.append(f"      <StationID>{station}</StationID>")
    if target:
        parts.append(f"      <TargetID>{target}</TargetID>")
    parts.append("    </PointRecord>")
    return parts


def jobfile(records):
    return "\n".join(['<?xml version="1.0" encoding="UTF-8"?>', '<JOBFile jobName="KARNE" product="Trimble Access" productVersion="2026.00" version="5.3">', "  <FieldBook>", *records, "  </FieldBook>", "</JOBFile>"]) + "\n"


def cases():
    good = jobfile(
        [
            *point("00000001", "ST1", "Coordinates", grid=("4521800.000", "412350.000", "105.200")),
            '    <StationRecord ID="00000002" TimeStamp="2026-10-04T10:01:00"><StationName>ST1</StationName><TheodoliteHeight>1.552</TheodoliteHeight><StationType>StandardStation</StationType></StationRecord>',
            '    <BackBearingRecord ID="00000003" TimeStamp="2026-10-04T10:01:30"><Station>ST1</Station><BackSight>P2</BackSight><Face1HorizontalCircle>0.00108</Face1HorizontalCircle></BackBearingRecord>',
            '    <TargetRecord ID="00000004" TimeStamp="2026-10-04T10:02:00"><PrismConstant>0</PrismConstant><TargetHeight>1.700</TargetHeight></TargetRecord>',
            *point("00000005", "P2", "AngleOnly", circle=("0.00108", "89.88885", "", "Face1"), station="00000002", target="00000004", classification="BackSight"),
            *point("00000006", "101", "DirectReading", circle=("78.68889", "91.11105", "63.214", "Face1"), station="00000002", target="00000004", code="BINA"),
            *point("00000007", "101", "DirectReading", circle=("258.69033", "268.89039", "63.216", "Face2"), station="00000002", target="00000004"),
            *point("00000008", "102", "MeanTurnedAngle", circle=("108.4995", "90.396", "82.441", "FaceNull"), station="00000002", target="00000004"),
            *point("00000009", "103", "DirectReading", circle=("189.108", "88.83", "55.123", "Face1"), station="00000002", target="00000004", deleted="true"),
            *point("00000010", "104", "HorizontalAngleOnly", circle=("274.5", "", "", "Face1"), station="00000002", target="00000004"),
            '    <TargetRecord ID="00000011" TimeStamp="2026-10-04T10:05:00"><PrismConstant>0</PrismConstant><TargetHeight></TargetHeight></TargetRecord>',
            *point("00000012", "105", "AverageMeasurements", circle=("1.5E1", "9.0e1", "1.25E+01", "Face1"), station="00000002", target="00000011"),
            *point("00000013", "ST2", "Coordinates", grid=("4521742.208", "412410.512", "104.875")),
            '    <StationRecord ID="00000014" TimeStamp="2026-10-04T10:10:00"><StationName>ST2</StationName><TheodoliteHeight>1.480</TheodoliteHeight></StationRecord>',
            *point("00000015", "ST1", "DirectReading", circle=("0", "90.3006", "83.392", "Face1"), station="00000014", target="00000004"),
        ]
    )
    broken = jobfile(
        [
            '    <StationRecord ID="1"><StationName>S1</StationName><TheodoliteHeight>1.5</TheodoliteHeight></StationRecord>',
            *point("2", "P1", "DirectReading", circle=("10", "90", "10", "Face1"), station="9"),
            *point("3", "P2", "DirectReading", circle=("360", "90", "10", "Face1"), station="1"),
            *point("4", "P3", "DirectReading", circle=("10", "-1", "10", "Face1"), station="1"),
            *point("5", "P4", "DirectReading", circle=("10", "90", "-0.5", "Face1"), station="1"),
            *point("6", "P5", "DirectReading", circle=("1x", "90", "10", "Face1"), station="1"),
            *point("7", "P6", "DirectReading", circle=("INF", "90", "10", "Face1"), station="1"),
            *point("8", "P7", "DirectReading", circle=("", "90", "10", "Face1"), station="1"),
            *point("9", "P8", "DirectReading", circle=("10", "90", "10", "Face1"), station="1", target="77"),
            *point("10", "P9", "DistanceOffset", circle=("10", "90", "10", "Face1"), station="1"),
            '    <StationRecord ID="11"><StationName>S2</StationName><TheodoliteHeight>abc</TheodoliteHeight></StationRecord>',
            *point("12", "P10", "DirectReading", circle=("10", "90", "10", "Face1"), station="11"),
        ]
    )
    return [
        ("iki istasyon, koordinatlar, prizma kayıtları, ham yöntemler, silinmiş ve ortalanmış kayıtlar", good),
        ("bozuk değerler, bulunmayan istasyon ve prizma, ham olmayan yöntem", broken),
        ("XML değil", "<JOBFile><FieldBook></JOBFile>\n"),
        ("JobXML değil (kök öğe gpx, JOBFile yalnız yorumda)", '<?xml version="1.0"?>\n<!-- <JOBFile> -->\n<gpx version="1.1">\n</gpx>\n'),
        ("FieldBook yok", '<?xml version="1.0"?>\n<JOBFile version="5.3">\n  <Reductions/>\n</JOBFile>\n'),
        ("256 düzeyden derin", "<JOBFile><FieldBook>" + "<a>" * 255 + "<b/>" + "</a>" * 255 + "</FieldBook></JOBFile>\n"),
        ("tam 256 düzey", "<JOBFile><FieldBook>" + "<a>" * 253 + "<b/>" + "</a>" * 253 + "</FieldBook></JOBFile>\n"),
    ]


def build():
    out = [{"name": name, "text": text, "expect": read(text)} for name, text in cases()]
    return {"format": "kentos.field-jobxml", "version": 1, "source": "scripts/fixtures/field_jobxml_cases.py (docs/adr/0169 §1; Trimble JobXML schema 5.3)", "texts": TEXTS, "cases": out}


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
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(build()['cases'])} durum.")


if __name__ == "__main__":
    main()
