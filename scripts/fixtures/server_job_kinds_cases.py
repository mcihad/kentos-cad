#!/usr/bin/env python3
"""The shared cases of what `project.import` and `project.atlas` server jobs decide without a database
(docs/adr/0561 §1, §2): the inputs checked when a job is asked for, the format a staged file's name says, and the memory
an import is estimated to need.

Written from the ADR's rules and plain Python, not from KentOS code; the Rust test
(crates/server/application/tests/job_kinds_cases.rs) reads the file and compares what its functions answer.

    python3 scripts/fixtures/server_job_kinds_cases.py [--check]
"""

from __future__ import annotations

import json
import sys
import unicodedata
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/server-jobs/v1/kinds.json"

PARAMETERS_MAX = 256
MEMORY_FLOOR_MB = 64
U64 = (1 << 64) - 1
MEMORY_BASE = 192 * 1024 * 1024
MEMORY_FACTOR = 24
IMPORT_FACTOR = 32


def uid(i: int) -> str:
    return f"00000000-0000-4000-8000-{i:012x}"


def is_uuid(text: str) -> bool:
    parts = text.split("-")
    if [len(p) for p in parts] != [8, 4, 4, 4, 12]:
        return False
    return all(c in "0123456789abcdef" for p in parts for c in p)


def is_control(c: str) -> bool:
    return unicodedata.category(c) == "Cc"


def file_value(value, at: str):
    if isinstance(value, dict):
        if isinstance(value.get("path"), str):
            return f"{at}.path"
        for k, v in value.items():
            found = file_value(v, f"{at}.{k}")
            if found:
                return found
    elif isinstance(value, list):
        for i, v in enumerate(value):
            found = file_value(v, f"{at}[{i}]")
            if found:
                return found
    return None


def limits_fault(request: dict):
    expected = request.get("expectedRevision")
    if expected is not None and not (expected.isdigit() and int(expected) <= (1 << 63) - 1):
        return "input.expectedRevision"
    if request.get("cpuSeconds") == 0:
        return "input.cpuSeconds"
    if request.get("memoryMb") is not None and request["memoryMb"] < MEMORY_FLOOR_MB:
        return "input.memoryMb"
    return None


def text_fault(text: str, most: int) -> bool:
    return not text.strip() or len(text) > most or any(is_control(c) for c in text)


def import_fault(request: dict):
    if not is_uuid(request.get("upload", "")):
        return "input.upload"
    options = request.get("options", {})
    if len(options) > PARAMETERS_MAX:
        return "input.options"
    if "path" in options:
        return "input.options.path"
    for name, value in options.items():
        found = file_value(value, f"input.options.{name}")
        if found:
            return found
    label = request.get("label")
    if label is not None and len(label) > 200:
        return "input.label"
    return limits_fault(request)


def atlas_fault(request: dict):
    if text_fault(request.get("sheet", ""), 200):
        return "input.sheet"
    fields = request.get("fields")
    if fields is not None:
        if len(fields) > PARAMETERS_MAX:
            return "input.fields"
        for i, f in enumerate(fields):
            if text_fault(f, 200):
                return f"input.fields[{i}]"
    theme = request.get("theme")
    if theme is not None and text_fault(theme, 200):
        return "input.theme"
    time = request.get("time")
    if time is not None and text_fault(time, 40):
        return "input.time"
    if request.get("crs") == 0:
        return "input.crs"
    label = request.get("label")
    if label is not None and len(label) > 200:
        return "input.label"
    return limits_fault(request)


def import_cases() -> list[dict]:
    good = {"upload": uid(7)}
    inputs = [
        ("an upload alone", good),
        ("a format and options", {**good, "format": "gpkg", "options": {"reproject": True, "tables": ["parsel"]}}),
        ("a revision and a budget", {**good, "expectedRevision": "12", "cpuSeconds": 600, "memoryMb": 2048, "label": "Parseller"}),
        ("an upload that is not an id", {"upload": "dosya"}),
        ("an upper-case id is not an id", {"upload": uid(255).upper()}),
        ("an empty upload", {"upload": ""}),
        ("a path among the options", {**good, "options": {"path": "/etc/passwd"}}),
        ("a path as an option's value is a value", {**good, "options": {"layer": "/etc/passwd"}}),
        ("a file value in an option", {**good, "options": {"kaynak": {"path": "/etc/passwd"}}}),
        ("a file value deep in a list", {**good, "options": {"a": [1, {"b": {"path": "x"}}]}}),
        ("a path that is not a text is a value", {**good, "options": {"a": {"path": 3}}}),
        ("the most options", {**good, "options": {f"o{i}": i for i in range(PARAMETERS_MAX)}}),
        ("one option too many", {**good, "options": {f"o{i}": i for i in range(PARAMETERS_MAX + 1)}}),
        ("a label of 200", {**good, "label": "l" * 200}),
        ("a label of 201", {**good, "label": "l" * 201}),
        ("a revision that is not a number", {**good, "expectedRevision": "bir"}),
        ("revision zero", {**good, "expectedRevision": "0"}),
        ("a revision past the database's", {**good, "expectedRevision": "9223372036854775808"}),
        ("no CPU", {**good, "cpuSeconds": 0}),
        ("memory under the floor", {**good, "memoryMb": 63}),
        ("memory at the floor", {**good, "memoryMb": 64}),
        ("the first fault is told: the upload before the options", {"upload": "x", "options": {"path": "y"}}),
        ("the first fault is told: the options before the revision", {**good, "options": {"path": "y"}, "expectedRevision": "x"}),
    ]
    return [{"name": n, "input": i, "fault": import_fault(i)} for n, i in inputs]


def atlas_cases() -> list[dict]:
    good = {"sheet": "atlas-paftasi"}
    inputs = [
        ("a sheet alone", good),
        ("everything", {**good, "fields": ["Ada", "Parsel"], "theme": "Gece", "time": "2024-06-30T12:00", "crs": 5253, "expectedRevision": "3", "cpuSeconds": 120, "memoryMb": 1024, "label": "Ada 1"}),
        ("an empty sheet", {"sheet": ""}),
        ("a blank sheet", {"sheet": "   "}),
        ("a sheet with a control character", {"sheet": "pa\u0007fta"}),
        ("a sheet of 200 characters", {"sheet": "s" * 200}),
        ("a sheet of 201 characters", {"sheet": "s" * 201}),
        ("no field names asked for", {**good, "fields": []}),
        ("a blank field name", {**good, "fields": ["Ada", " "]}),
        ("a field name with a control character", {**good, "fields": ["A\u0001da"]}),
        ("the most field names", {**good, "fields": [f"f{i}" for i in range(PARAMETERS_MAX)]}),
        ("one field name too many", {**good, "fields": [f"f{i}" for i in range(PARAMETERS_MAX + 1)]}),
        ("a blank theme", {**good, "theme": ""}),
        ("a theme of 201", {**good, "theme": "t" * 201}),
        ("a time of 40", {**good, "time": "t" * 40}),
        ("a time of 41", {**good, "time": "t" * 41}),
        ("a coordinate system of zero", {**good, "crs": 0}),
        ("a coordinate system", {**good, "crs": 5256}),
        ("a label of 201", {**good, "label": "l" * 201}),
        ("a revision that is not a number", {**good, "expectedRevision": "-1"}),
        ("no CPU", {**good, "cpuSeconds": 0}),
        ("memory under the floor", {**good, "memoryMb": 10}),
        ("the sheet is the first fault", {"sheet": "", "crs": 0, "cpuSeconds": 0}),
    ]
    return [{"name": n, "input": i, "fault": atlas_fault(i)} for n, i in inputs]


FORMATS = {"gpkg": "gpkg", "fgb": "fgb", "gml": "gml", "xml": "gml", "csv": "csv", "txt": "csv", "dxf": "dxf",
           "zip": "shp", "geojson": "geojson", "json": "geojson", "kml": "kml", "kmz": "kml", "parquet": "parquet"}
COMMANDS = {
    "gpkg": "file.import.gpkg",
    "fgb": "file.import.fgb",
    "gml": "file.import.gml",
    "csv": "file.import.csv",
    "dxf": "file.import.dxf",
    "shp": "file.import.shp",
    "geojson": "file.import.geojson",
    "kml": "file.import.kml",
    "parquet": "file.import.parquet",
}


def format_of(name: str):
    if "." not in name:
        return None
    return FORMATS.get(name.rsplit(".", 1)[1].lower())


def format_cases() -> list[dict]:
    names = [
        "parsel.gpkg", "PARSEL.GPKG", "ada.fgb", "binalar.gml", "binalar.xml", "noktalar.csv", "noktalar.TXT", "cizim.dxf",
        "CIZIM.Dxf", "a.b.c.gpkg", ".gpkg", "gpkg", "parsel.", "parsel.shp", "parsel.zip", "parsel.geojson", "parsel.kcad", "",
        "PARSEL.ZIP", "arsiv.tar.zip", "noktalar.json", "NOKTALAR.GeoJSON", "yerler.kml", "yerler.KMZ", "yerler.kmz",
        "parsel.parquet", "PARSEL.Parquet", "parsel.geoparquet", "parsel.parq",
    ]
    return [{"name": n, "format": format_of(n), "command": COMMANDS.get(format_of(n) or "")} for n in names]


def estimate(snapshot: int, file: int) -> int:
    processing = min(MEMORY_BASE + min(snapshot * MEMORY_FACTOR, U64), U64)
    return min(processing + min(file * IMPORT_FACTOR, U64), U64)


def memory_cases() -> list[dict]:
    pairs = [
        (0, 0), (1, 1), (1024, 0), (0, 1024), (50 * 1024 * 1024, 100 * 1024 * 1024), (1 << 20, 1 << 30), (1 << 30, 1 << 30),
        (1 << 40, 1 << 40), (U64 // 24, 0), (0, U64 // 32), (0, U64 // 32 + 1), (U64, U64),
    ]
    return [{"snapshot": s, "file": f, "estimate": estimate(s, f)} for s, f in pairs]


def build() -> dict:
    return {
        "doc": "docs/adr/0561-import-atlas-and-mcp-jobs.md",
        "note": "written by scripts/fixtures/server_job_kinds_cases.py; read by crates/server/application/tests/job_kinds_cases.rs",
        "limits": {"parametersMax": PARAMETERS_MAX, "memoryFloorMb": MEMORY_FLOOR_MB, "importFactor": IMPORT_FACTOR},
        "imports": import_cases(),
        "atlases": atlas_cases(),
        "formats": format_cases(),
        "memory": memory_cases(),
    }


def main() -> int:
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print("fixtures/server-jobs/v1/kinds.json eski; scripts/fixtures/server_job_kinds_cases.py'yi --check'siz çalıştırın")
            return 1
        data = json.loads(text)
        print(
            f"Sunucu iş türleri durumları güncel: {len(data['imports'])} içe aktarma, {len(data['atlases'])} atlas, "
            f"{len(data['formats'])} biçim, {len(data['memory'])} bellek"
        )
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
