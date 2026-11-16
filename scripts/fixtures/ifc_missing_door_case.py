#!/usr/bin/env python3
"""EK-9/8's acceptance case (plan/ifc-uyum.md IFC-08 "İlk kabul", docs/adr/8074 §7): the section building's file
(fixtures/arch/v1/ifc/kesit-binasi.ifc) with one of its doors, K02, and the records only it names taken out by
IfcOpenShell, not by KentOS: its fill of the wall's opening, its place on the storey, its sets and its type's
assignment. The opening stays in its wall. The file is still valid IFC 4.3 (IfcOpenShell's validator, with the EXPRESS
rules, finds nothing) and the doors left keep what they had, so neither the schema nor the profile's IDS says anything
is missing: the model quality control form's 8th question does, from the source's objects (`expected`).

Writes fixtures/ifc/v1/eksik/kesit-binasi-kapi-eksik.ifc and its note (.json: the door taken, the records before and
after, the validator's findings); --check compares instead. Runs under IfcOpenShell, offline:

    uv run --offline --no-project --python 3.11 --with ifcopenshell==0.9.0.post1 --with pytest --with lark \\
        python -I scripts/fixtures/ifc_missing_door_case.py [--check]
"""

import json
import sys
from pathlib import Path

import ifcopenshell
import ifcopenshell.api
import ifcopenshell.validate

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "fixtures" / "arch" / "v1" / "ifc" / "kesit-binasi.ifc"
OUT = ROOT / "fixtures" / "ifc" / "v1" / "eksik"
FILE = OUT / "kesit-binasi-kapi-eksik.ifc"
NOTE = OUT / "kesit-binasi-kapi-eksik.json"

# The door taken: K02 on the ground floor (its GlobalId is its KentOS id, compressed as IFC compresses a GUID).
DOOR = "01alMXGa9m08000000000E"


def made():
    f = ifcopenshell.open(str(SOURCE))
    before = len(list(f))
    door = f.by_guid(DOOR)
    tag, name = door.Tag, door.Name
    ifcopenshell.api.run("root.remove_product", f, product=door)
    log = ifcopenshell.validate.json_logger()
    ifcopenshell.validate.validate(f, log, express_rules=True)
    note = {
        "source": "fixtures/arch/v1/ifc/kesit-binasi.ifc",
        "removed": {"globalId": DOOR, "class": "IfcDoor", "tag": tag, "name": name},
        "records": {"before": before, "after": len(list(f))},
        "doors": {"before": 5, "after": len(f.by_type("IfcDoor"))},
        "validator": [str(s.get("message", s)) for s in log.statements],
    }
    return f.to_string(), json.dumps(note, ensure_ascii=False, indent=2) + "\n"


def main():
    text, note = made()
    if "--check" in sys.argv:
        for path, want in ((FILE, text), (NOTE, note)):
            if not path.is_file() or path.read_text(encoding="utf-8") != want:
                sys.exit(f"{path.relative_to(ROOT)} güncel değil: betiği --check olmadan çalıştırıp farkı okuyun.")
        print("✓ güncel")
        return
    OUT.mkdir(parents=True, exist_ok=True)
    FILE.write_text(text, encoding="utf-8")
    NOTE.write_text(note, encoding="utf-8")
    print(f"{FILE.relative_to(ROOT)}: {json.loads(note)}")


if __name__ == "__main__":
    main()
