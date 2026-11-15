#!/usr/bin/env python3
"""The rule catalog's baseline, measured from written IFC files (plan/bim-plan.md BIM-01 “Başlangıç ölçümü”,
plan/ifc-uyum.md IFC-01; docs/adr/8062 §7) by IfcOpenShell, not by KentOS: for each file, how many objects of each of
EK-5's classes it holds and, for every mandatory row of EK-6 and EK-7 whose class it holds, on how many of the class's
objects the row's value is there (an attribute set and not blank, a property or quantity in its set, a material with a
name, the relation made). A predefined type of NOTDEFINED is there but said; a material's name is checked for EK-6's
“Malzeme İsmi_Dayanım” form.

    uv run --no-project --python 3.11 --with ifcopenshell==0.9.0.post1 \\
        python3 scripts/mevzuat/olcum.py [--check] FILE.ifc …

It writes fixtures/arch/v1/ifc/katalog-olcum.json (each file with its SHA-256) and prints the Markdown table the ADR
keeps. The catalog and the decisions are read as data: a name the source prints otherwise (a decision's `replaces`) is
measured by the name used.
"""
from __future__ import annotations

import hashlib
import json
import re
import sys
from collections import Counter
from pathlib import Path

import ifcopenshell
import ifcopenshell.util.element as elements

ROOT = Path(__file__).resolve().parents[2]
CATALOG = ROOT / "crates" / "shared" / "ifc" / "data" / "temel-2026.json"
DECISIONS = ROOT / "crates" / "shared" / "ifc" / "data" / "kararlar-2026.json"
OUT = ROOT / "fixtures" / "arch" / "v1" / "ifc" / "katalog-olcum.json"
MATERIAL_FORM = re.compile(r"^[^\s_]+(_[^\s_]+)+$")


def blank(v) -> bool:
    return v is None or (isinstance(v, str) and not v.strip())


def materials_of(obj):
    """The material names an object carries, through its association (a material, a layer set's layers, a list)."""
    m = elements.get_material(obj, should_skip_usage=True)
    if m is None:
        return [], []
    names, layers = [], []
    if m.is_a("IfcMaterial"):
        names.append(m.Name)
    elif m.is_a("IfcMaterialLayerSet"):
        for layer in m.MaterialLayers or ():
            layers.append(layer)
            if layer.Material is not None:
                names.append(layer.Material.Name)
    elif m.is_a("IfcMaterialConstituentSet"):
        for c in m.MaterialConstituents or ():
            if c.Material is not None:
                names.append(c.Material.Name)
    elif m.is_a("IfcMaterialList"):
        names.extend(x.Name for x in m.Materials or ())
    return [n for n in names if not blank(n)], layers


def there(obj, row, used) -> tuple[bool, str | None]:
    """Whether the row's value is on the object, and a note (NOTDEFINED, a material's form)."""
    kind, name = row["kind"], used(row["name"])
    if kind == "attribute":
        if not hasattr(obj, name):
            return False, "şemada yok"
        v = getattr(obj, name)
        if name == "PredefinedType" and v in ("NOTDEFINED",):
            return True, "NOTDEFINED"
        if name == "PredefinedType" and v == "USERDEFINED" and blank(getattr(obj, "ObjectType", None)):
            return True, "USERDEFINED, ObjectType boş"
        return not blank(v) and v != (), None
    if kind in ("property", "quantity"):
        sets = elements.get_psets(obj, qtos_only=(kind == "quantity"), psets_only=(kind == "property"))
        v = sets.get(used(row["set"]), {}).get(name)
        return not blank(v), None
    if kind == "material":
        names, layers = materials_of(obj)
        if name == "Name":
            if not names:
                return False, None
            return True, None if all(MATERIAL_FORM.match(n) for n in names) else "biçim: " + ", ".join(sorted(set(names)))
        return any(not blank(getattr(layer, name, None)) for layer in layers), None
    if kind == "relation":
        target = (row.get("path") or "").split(">")[-1]
        for rel in getattr(obj, "HasAssignments", ()) or ():
            if rel.is_a("IfcRelAssignsToGroup") and rel.RelatingGroup.is_a(target):
                return True, None
        return False, None
    return False, "okunmadı"


def measure(path: Path, catalog, used) -> dict:
    f = ifcopenshell.open(str(path))
    data = path.read_bytes()
    classes = Counter()
    for c in catalog["classes"]:
        n = len(f.by_type(used(c["class"]), include_subtypes=False)) if _known(f, used(c["class"])) else 0
        if n:
            classes[c["class"]] = n
    for t in catalog["tables"]:
        if t["table"].startswith("7.") and _known(f, t["class"]):
            n = len(f.by_type(t["class"], include_subtypes=False))
            if n:
                classes[t["class"]] = n
    rows = {}
    for r in catalog["requirements"]:
        if r["level"] != "mandatory" or not classes.get(r["class"]):
            continue
        objs = f.by_type(used(r["class"]), include_subtypes=False)
        have, notes = 0, Counter()
        for o in objs:
            ok, note = there(o, r, used)
            have += ok
            if note:
                notes[note] += 1
        status = "var" if have == len(objs) else ("kısmen" if have else "yok")
        rows[r["id"]] = {"class": r["class"], "n": len(objs), "have": have, "status": status, "notes": dict(sorted(notes.items()))}
    return {
        "file": str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else path.name,
        "bytes": len(data),
        "sha256": hashlib.sha256(data).hexdigest(),
        "schema": f.schema_identifier,
        "classes": dict(sorted(classes.items())),
        "rows": rows,
    }


def _known(f, name: str) -> bool:
    try:
        f.by_type(name)
        return True
    except Exception:
        return False


def markdown(results, catalog) -> str:
    rows_of = {r["id"]: r for r in catalog["requirements"]}
    out = []
    for res in results:
        counts = Counter(v["status"] for v in res["rows"].values())
        out.append(f"#### {res['file']} ({res['bytes']} bayt, SHA-256 {res['sha256'][:12]}…)")
        out.append("")
        out.append(
            f"Sınıflar: {', '.join(f'{k} {v}' for k, v in res['classes'].items())}. Uygulanan zorunlu satır: "
            f"{len(res['rows'])} — var {counts['var']}, kısmen {counts['kısmen']}, yok {counts['yok']}."
        )
        out.append("")
        out.append("| Satır | Yol | Nesne | Değeri olan | Durum | Not |")
        out.append("|---|---|---|---|---|---|")
        for rid, v in res["rows"].items():
            r = rows_of[rid]
            path = f"{r['set']}.{r['name']}" if r["set"] else (f"{r['path'].split('>')[-1]}.{r['name']}" if r["kind"] == "material" else (r["path"] or r["name"]))
            note = "; ".join(f"{k} ({n})" for k, n in v["notes"].items())
            out.append(f"| {r['class']} {r['table']}/{r['row']} | {path} | {v['n']} | {v['have']} | {v['status']} | {note} |")
        out.append("")
    return "\n".join(out)


def main(argv: list[str]) -> int:
    check = "--check" in argv
    files = [Path(a).resolve() for a in argv if not a.startswith("--")]
    if not files:
        print(__doc__, file=sys.stderr)
        return 2
    catalog = json.loads(CATALOG.read_text(encoding="utf-8"))
    decisions = json.loads(DECISIONS.read_text(encoding="utf-8"))["decisions"]
    replaced = {r["printed"]: r["used"] for d in decisions for r in d.get("replaces", [])}

    def used(name: str) -> str:
        return replaced.get(name, name)

    results = [measure(p, catalog, used) for p in files]
    data = {"format": 1, "ifcopenshell": ifcopenshell.version, "files": results}
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if check:
        have = OUT.read_text(encoding="utf-8") if OUT.exists() else None
        if have != text:
            print(f"olcum: {OUT.relative_to(ROOT)} is not what the files give now", file=sys.stderr)
            return 1
    else:
        OUT.write_text(text, encoding="utf-8")
    print(markdown(results, catalog))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
