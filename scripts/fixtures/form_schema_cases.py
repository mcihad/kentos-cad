"""Checks the form's published JSON Schema (catalog/v1/forms/form-v1.schema.json, docs/adr/0434) with the `jsonschema`
package, without KentOS code: every form the repository's shared files hold (the KCAD samples, the command cases'
drawings) is valid, and forms broken on purpose (an unknown key, an item without its id, an unknown widget, a release
without its digest, a workflow without steps' states) are not.

    python3 scripts/fixtures/form_schema_cases.py --check
"""
import copy
import json
import sys
from pathlib import Path

import jsonschema

ROOT = Path(__file__).resolve().parents[2]
SCHEMA = ROOT / "catalog/v1/forms/form-v1.schema.json"


def forms_in(value, out):
    """Every `settings.forms` list's forms under `value`."""
    if isinstance(value, dict):
        settings = value.get("settings")
        if isinstance(settings, dict) and isinstance(settings.get("forms"), list):
            out.extend(settings["forms"])
        for v in value.values():
            forms_in(v, out)
    elif isinstance(value, list):
        for v in value:
            forms_in(v, out)


def main() -> int:
    schema = json.loads(SCHEMA.read_text("utf-8"))
    jsonschema.Draft202012Validator.check_schema(schema)
    validator = jsonschema.Draft202012Validator(schema)
    files = sorted((ROOT / "fixtures/commands/v1").glob("*.json")) + sorted((ROOT / "fixtures/kcad/v2").glob("*.json"))
    found = []
    for f in files:
        forms = []
        forms_in(json.loads(f.read_text("utf-8")), forms)
        found += [(f.relative_to(ROOT), x) for x in forms]
    problems = []
    seen = set()
    for where, form in found:
        key = json.dumps(form, sort_keys=True)
        if key in seen:
            continue
        seen.add(key)
        errors = sorted(validator.iter_errors(form), key=lambda e: list(e.path))
        if errors:
            problems.append(f"{where}: “{form.get('id')}”: {errors[0].message} ({'/'.join(map(str, errors[0].path))})")
    base = next(x for _, x in found if x.get("releases") and x["draft"].get("items"))
    broken = {
        "bilinmeyen anahtar": lambda f: f.update(color="#FF0000"),
        "kimliksiz öğe": lambda f: f["draft"]["items"][0].pop("id"),
        "bilinmeyen giriş biçimi": lambda f: next(i for i in walk(f["draft"]["items"]) if i["type"] == "field").update(widget="harita"),
        "özetsiz yayın": lambda f: f["releases"][0].pop("digest"),
        "adımsız iş akışı": lambda f: f["draft"].update(workflow={"field": "Durum", "initial": "x"}),
    }
    for name, change in broken.items():
        f = copy.deepcopy(base)
        change(f)
        if validator.is_valid(f):
            problems.append(f"bozuk form geçerli sayıldı: {name}")
    if problems:
        print("\n".join(problems), file=sys.stderr)
        return 1
    print(f"form şeması: {len(seen)} form geçerli, {len(broken)} bozuk form reddedildi ({SCHEMA.relative_to(ROOT)})")
    return 0


def walk(items):
    for i in items:
        yield i
        yield from walk(i.get("items", []))


if __name__ == "__main__":
    sys.exit(main())
