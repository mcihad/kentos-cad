#!/usr/bin/env python3
"""The profile's IDS (crates/shared/ifc/data/temel-2026.ids, docs/adr/8074 §5) opened and run by an independent checker,
IfcTester (IfcOpenShell's IDS tool; not KentOS code), on KentOS's five files; its failures compared, row by row and object
by object, with what KentOS's own rule finds (the facts' `gaps`, the form's 9th question; EK-7's records the form's 2nd
and 6th questions read). The IDS is opened with its schema check (IDS 1.0's XSD, IfcTester's own copy).

Rows KentOS does not judge (a group the object is part of, an attribute such as the placement) are listed apart, with
what IfcTester found. Writes fixtures/ifc/v1/ids-sonuc/<name>.json; --check compares instead. Needs the network once
(IfcTester 0.9.0, with the owner's approval of 16 November 2026), then runs offline:

    uv run --no-project --python 3.11 --with ifcopenshell==0.9.0.post1 --with ifctester==0.9.0 \\
        python -I scripts/fixtures/ids_ifctester_check.py [--check]
"""

import json
import re
import sys
from pathlib import Path

import ifcopenshell
import ifcopenshell.util.element
import ifctester
from ifctester import ids

ROOT = Path(__file__).resolve().parents[2]
IDS = ROOT / "crates/shared/ifc/data/temel-2026.ids"
OUT = ROOT / "fixtures/ifc/v1/ids-sonuc"
CATALOG = json.loads((ROOT / "crates/shared/ifc/data/temel-2026.json").read_text(encoding="utf-8"))
FILES = {
    "kesit-binasi": "fixtures/arch/v1/ifc/kesit-binasi.ifc",
    "kesit-binasi-kunye": "fixtures/arch/v1/ifc/kesit-binasi-kunye.ifc",
    "kesit-binasi-tam": "fixtures/arch/v1/ifc/kesit-binasi-tam.ifc",
    "yerel-sistem": "fixtures/arch/v1/ifc/yerel-sistem.ifc",
    "kesit-binasi-kapi-eksik": "fixtures/ifc/v1/eksik/kesit-binasi-kapi-eksik.ifc",
}
# The attributes KentOS's rule reads (kentos_ifc::required); the others it leaves to the IDS.
JUDGED = {"Name", "LongName", "Description", "ObjectType", "PredefinedType", "Tag", "AxisTag", "RefElevation"}
# EK-7's records, which the form's 2nd and 6th questions read from the file's records, not from the objects' gaps.
RECORDS = {"IfcPerson", "IfcPersonAndOrganization", "IfcOrganization", "IfcActorRole", "IfcPostalAddress",
           "IfcProjectedCRS", "IfcMapConversion", "IfcGeometricRepresentationContext"}

ROWS = {(r["table"], r["row"]): r for r in CATALOG["requirements"] if r["level"] == "mandatory"}


def row_of(spec, facet):
    """The catalog row a requirement is: its table from the specification's name, its row from the instructions."""
    table = re.search(r"Tablo (\d+\.\d+)", spec.name)
    row = re.search(r"satır (\d+)[);]", facet.instructions or "")
    return ROWS.get((table.group(1), int(row.group(1)))) if table and row else None


def key(e):
    return getattr(e, "GlobalId", None) or f"#{e.id()}"


def judged_by_kentos(r):
    if r["class"] in RECORDS:
        return "records"
    if r["kind"] == "relation":
        return None
    if r["kind"] == "attribute" and r["name"].replace("Attribute:", "").strip() not in JUDGED:
        return None
    return "gaps"


def kentos_records(facts):
    """The EK-7 rows the form reads from the records, as the 2nd and 6th questions read them (K48: a user-defined role
    is asked only when the role is USERDEFINED)."""
    out = {}
    for r in facts["rows"]:
        if not r["mandatory"] or not r["attribute"] or r["class"] not in RECORDS:
            continue
        for rec in facts["records"].get(r["class"]) or []:
            a = rec.get("attributes") or {}
            if r["name"] == "UserDefinedRole" and a.get("Role") != "USERDEFINED":
                continue
            v = a.get(r["name"])
            if v is None or (isinstance(v, str) and not v.strip()) or (isinstance(v, list) and not v):
                out.setdefault(r["id"], []).append(f"#{rec['step']}")
    return {k: sorted(v) for k, v in out.items()}


# EK-6 Tablo 6.7's building heights, asked "sadece yüksekliğin sabit olduğu durumda" (docs/adr/8076 §4): IDS 1.0 has no
# such condition, so the profile asks them always. Where the file's storeys carry differing heights, IfcTester's failure
# is the IDS's want of a condition, not a missing value: kept apart as `conditional`, read from the file here.
CONDITIONAL = {"T6.7/Qto_BuildingBaseQuantities.Height": "GrossHeight",
               "T6.7/Qto_BuildingBaseQuantities.EavesHeight": "NetHeight"}


def heights_differ(f, name):
    known = []
    for st in f.by_type("IfcBuildingStorey"):
        h = (ifcopenshell.util.element.get_pset(st, "Qto_BuildingStoreyBaseQuantities") or {}).get(name)
        if isinstance(h, (int, float)):
            known.append(h)
    return any(abs(h - known[0]) > 1e-9 for h in known)


def run(name, path):
    f = ifcopenshell.open(str(ROOT / path))
    specs = ids.open(str(IDS), validate=True)
    specs.validate(f)
    tester = {"gaps": {}, "records": {}, "apart": {}, "conditional": {}, "absent": []}
    for spec in specs.specifications:
        if not spec.applicable_entities:
            # A class every model has (K51) the file does not: the specification fails with nothing to apply to.
            if spec.status is False:
                tester["absent"].append(re.search(r"Tablo \d+\.\d+ (\w+)", spec.name).group(1))
            continue
        for facet in spec.requirements:
            failures = getattr(facet, "failures", None) or []
            if not failures:
                continue
            r = row_of(spec, facet)
            if r is None:
                sys.exit(f"{name}: {spec.name} gerekliliği kataloğun satırı değil: {facet.instructions}")
            where = judged_by_kentos(r) or "apart"
            got = sorted({key(x["element"]) for x in failures})
            if r["id"] in CONDITIONAL and heights_differ(f, CONDITIONAL[r["id"]]):
                tester["conditional"][r["id"]] = got
                continue
            if where == "apart":
                reasons = sorted({str(x.get("reason", ""))[:120] for x in failures})
                tester["apart"][r["id"]] = {"objects": got, "reasons": reasons}
            else:
                tester[where][r["id"]] = got
    facts = json.loads((ROOT / "fixtures/ifc/v1/olgular" / f"{name}.json").read_text(encoding="utf-8"))
    kentos = {"gaps": {g["id"]: sorted(g["missing"]) for g in facts["gaps"] if g["class"] not in RECORDS},
              "records": kentos_records(facts)}
    # The records the form's 2nd and 6th questions find missing: EK-7's classes the file has none of.
    every = sorted({t["class"] for t in CATALOG["tables"] if t["table"].startswith("7.") and t["level"] == "mandatory"
                    and t["class"] != "IfcAnnotation"})
    held = set(facts["records"]) | {o["class"] for o in facts["objects"]}
    kentos_absent = [c for c in every if c not in held]
    tester_absent = sorted(set(tester["absent"]))
    differ = {}
    for where in ("gaps", "records"):
        for rid in sorted(set(tester[where]) | set(kentos[where])):
            a, b = tester[where].get(rid, []), kentos[where].get(rid, [])
            if a != b:
                differ[rid] = {"ifctester": a, "kentos": b}
    if tester_absent != kentos_absent:
        differ["kayıt yok"] = {"ifctester": tester_absent, "kentos": kentos_absent}
    return {
        "file": path,
        "absent": tester_absent,
        "rows": {w: tester[w] for w in ("gaps", "records")},
        "checker": f"IfcTester {getattr(ifctester, '__version__', '?')}, IfcOpenShell {ifcopenshell.version}",
        "same": {w: len(tester[w]) for w in ("gaps", "records")},
        "differ": differ,
        "apart": tester["apart"],
        "conditional": tester["conditional"],
    }


def main():
    check = "--check" in sys.argv
    OUT.mkdir(parents=True, exist_ok=True)
    bad = False
    for name, path in FILES.items():
        got = run(name, path)
        text = json.dumps(got, ensure_ascii=False, indent=1) + "\n"
        out = OUT / f"{name}.json"
        if check:
            if not out.is_file() or out.read_text(encoding="utf-8") != text:
                sys.exit(f"{out.relative_to(ROOT)} güncel değil: betiği --check olmadan çalıştırıp farkı okuyun.")
        else:
            out.write_text(text, encoding="utf-8")
        bad |= bool(got["differ"])
        print(f"{'✗' if got['differ'] else '✓'} {name}: IfcTester'ın {got['same']['gaps']} eksik satırı, "
              f"{got['same']['records']} EK-7 kayıt satırı ve {len(got['absent'])} eksik kayıt sınıfı; ayrışan "
              f"{len(got['differ'])}; KentOS'un ölçmediği {len(got['apart'])} satır ayrı; katların yüksekliği farklı "
              f"olduğundan sorulmayan {len(got['conditional'])} satır")
        for rid, d in list(got["differ"].items())[:6]:
            print(f"    {rid}: IfcTester {len(d['ifctester'])}, KentOS {len(d['kentos'])}")
        for rid, d in list(got["apart"].items())[:8]:
            print(f"    ayrı {rid}: {len(d['objects'])} nesne — {d['reasons'][0] if d['reasons'] else ''}")
    if bad:
        sys.exit(1)


if __name__ == "__main__":
    main()
