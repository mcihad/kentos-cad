#!/usr/bin/env python3
"""What the split and merged objects came from (docs/adr/0598 §2), as the shared command cases see it: no KentOS code.

The cases are the split and merge cases of `cad.entities.edit`, `cad.entities.break` and `cad.entities.join` that their
own independent references wrote (edit_command_cases.py, reshape_command_cases.py, multipart_edit_cases.py,
field_policy_cases.py); each completed execute, undo and redo step gets the settings' `lineage` it leaves, by the ADR's
rule:

- a split (`areaSplit`, `split`, `break`, `partsSplit`; `cad.entities.break`): the pieces that carry an object's data
  (an update, a kept-data replacement or a kept-data addition from it); when an object has two pieces or more, every
  piece other than the object itself has it as its parent;
- a merge (`areaUnion`, `join`, `partsJoin`; `cad.entities.join`): when objects are removed and one piece carries the
  data, the piece has its object (when it is another) and the removed objects as its parents, in the input's order;
- the records of a step follow the earlier ones, a record already kept stays once; undo takes the step's records back,
  redo gives them again.

Written: fixtures/commands/v1/cad.entities.{edit,break,join}.lineage.json. `--check` compares instead of writing.
"""

import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/commands/v1"

SPLITS = {"areaSplit", "split", "break", "partsSplit"}
MERGES = {"areaUnion", "join", "partsJoin"}
STEP = {
    "areaSplit": "Alan böl", "split": "Parçala", "break": "Kır", "partsSplit": "Parçalara ayır",
    "areaUnion": "Alan birleştir", "join": "Birleştir", "partsJoin": "Parçaları birleştir",
}
SOURCES = {
    "cad.entities.edit": ["cad.entities.edit.json", "cad.entities.edit.policies.json"],
    "cad.entities.break": ["cad.entities.break.json", "cad.entities.break.parts.json", "cad.entities.break.policy.json"],
    "cad.entities.join": ["cad.entities.join.json", "cad.entities.join.policy.json"],
}


def edit_records(inp, out):
    """The records of a `cad.entities.edit` step, by the input's changes."""
    op = inp.get("operation")
    if op not in SPLITS | MERGES:
        return []
    created = list(out.get("created", []))
    pieces, gone, added = [], [], 0
    for c in inp["changes"]:
        kind = c["kind"]
        if kind == "update" or (kind == "replace" and c.get("keepData")):
            pieces.append((c["uid"], c["uid"]))
        elif kind == "add":
            if c.get("keepData"):
                pieces.append((c["from"], created[added]))
            added += 1
        elif kind == "remove":
            gone.append(c["uid"])
    step = STEP[op]
    records = []
    if op in SPLITS:
        sources = []
        for s, _ in pieces:
            if s not in sources:
                sources.append(s)
        for s in sources:
            at = [child for src, child in pieces if src == s]
            if len(at) < 2:
                continue
            for child in at:
                if child != s:
                    records.append({"child": child, "parents": [s], "operation": "split", "step": step})
    elif gone and len(pieces) == 1:
        source, child = pieces[0]
        parents = []
        for p in [source] + gone:
            if p != child and p not in parents:
                parents.append(p)
        if parents:
            records.append({"child": child, "parents": parents, "operation": "merge", "step": step})
    return records


def command_records(command, out):
    """The records of a `cad.entities.break` or `.join` step, by what it wrote: Kır keeps its object and adds the
    other piece; Birleştir keeps the first object and removes the others."""
    if command == "cad.entities.break":
        kept = (out.get("changed") or out.get("removed") or [None])[0]
        return [{"child": c, "parents": [kept], "operation": "split", "step": "Kır"}
                for c in out.get("created", []) if kept and c != kept]
    kept = (out.get("changed") or out.get("created") or [None])[0]
    parents = [r for r in out.get("removed", []) if r != kept]
    if not kept or not parents:
        return []
    return [{"child": kept, "parents": parents, "operation": "merge", "step": "Birleştir"}]


def cases_of(command):
    """The command's lineage cases: every case with a completed split or merge (a break of a closed shape or of a
    part leaves none: its expectation says so)."""
    out = []
    setup = None
    for name in SOURCES[command]:
        data = json.loads((DIR / name).read_text(encoding="utf-8"))
        for case in data["cases"]:
            steps = copy.deepcopy(case["steps"])
            kept, undone, done, any_record = [], [], [], False
            for s in steps:
                result = s.get("result") if isinstance(s.get("result"), dict) else {}
                if s.get("op") == "execute" and result.get("status") == "completed":
                    o = s["result"]["output"]
                    records = edit_records(s["input"], o) if command == "cad.entities.edit" else command_records(command, o)
                    added = [r for r in records if r not in kept]
                    kept = kept + added
                    done.append(added)
                    undone.clear()
                    any_record |= bool(added) or command != "cad.entities.edit" or s["input"].get("operation") in SPLITS | MERGES
                elif s.get("op") == "execute":
                    # A refused split or merge leaves no record.
                    any_record |= command != "cad.entities.edit" or s["input"].get("operation") in SPLITS | MERGES
                elif s.get("op") == "undo" and done:
                    last = done.pop()
                    kept = [r for r in kept if r not in last]
                    undone.append(last)
                elif s.get("op") == "redo" and undone:
                    last = undone.pop()
                    kept = kept + [r for r in last if r not in kept]
                    done.append(last)
                else:
                    continue
                s.setdefault("expect", {}).setdefault("settings", {})["lineage"] = copy.deepcopy(kept) or None
            if any_record:
                # A parent named by its slot may be gone when the step is checked: its uid is taken first, by name.
                slots = sorted({int(p[len("$uidOf:"):]) for st in steps
                                for r in (st.get("expect", {}).get("settings", {}).get("lineage") or [])
                                for p in r["parents"] if p.startswith("$uidOf:")})
                if slots:
                    def named(v):
                        return f"$uid:soy{v[len('$uidOf:'):]}" if isinstance(v, str) and v.startswith("$uidOf:") else v
                    for st in steps:
                        for r in (st.get("expect", {}).get("settings", {}).get("lineage") or []):
                            r["parents"] = [named(p) for p in r["parents"]]
                    steps = [{"op": "captureUid", "id": n, "as": f"soy{n}"} for n in slots] + steps
                if setup is None:
                    setup = data["setup"]
                if data["setup"] != setup:
                    # Each source file has its own drawing: the case keeps its own.
                    case = dict(case, setup=data["setup"])
                out.append({**{k: v for k, v in case.items() if k != "steps"}, "steps": steps,
                            "name": f"Soy kaydı: {case['name']}"})
    return setup, out


def written():
    """The GeoPackage the bound-layer test wrote (fixtures/geopackage/v1/kentos-soy.gpkg, KENTOS_WRITE_GPKG=1 cargo test
    -p kentos-native-application --test all a_split_parcel): read with sqlite3. A parcel split, the split written, undone
    and written, done again and written, then the piece merged with another parcel: the union's two parents and the
    piece's own parent stay; the union is a row of the table, the piece and the merged parcel are not; the extension is
    named."""
    import sqlite3
    path = ROOT / "fixtures/geopackage/v1/kentos-soy.gpkg"
    con = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
    rows = con.execute("SELECT table_name, child, parent, operation, step FROM kentos_lineage ORDER BY 2, 3").fetchall()
    uids = {u for (u,) in con.execute(
        "SELECT u.uid FROM kentos_uids u JOIN parsel p ON p.fid = u.fid WHERE u.table_name = 'parsel'")}
    ext = con.execute("SELECT count(*) FROM gpkg_extensions WHERE extension_name = 'kentos_lineage'").fetchone()[0]
    problems = []
    merges = [r for r in rows if r[3] == "merge"]
    splits = [r for r in rows if r[3] == "split"]
    if len(rows) != 3 or len(merges) != 2 or len(splits) != 1:
        problems.append(f"{len(rows)} satır: {rows}")
    if any(r[0] != "parsel" for r in rows):
        problems.append("tablonun adı parsel değil")
    if merges and (len({r[1] for r in merges}) != 1 or merges[0][1] not in uids):
        problems.append("birleşimin nesnesi tablonun bir satırı değil")
    piece = splits[0][1] if splits else None
    if piece is None or piece not in {r[2] for r in merges} or piece in uids:
        problems.append("bölünen parça birleşimin atası ve tablodan çıkmış olmalı")
    if any(r[4] not in ("Alan böl", "Alan birleştir") for r in rows):
        problems.append("adımın adı")
    if ext != 1:
        problems.append("kentos_lineage uzantısı gpkg_extensions'ta yok")
    if problems:
        print("kentos-soy.gpkg: " + "; ".join(problems), file=sys.stderr)
        return 1
    print(f"kentos-soy.gpkg tutarlı: {len(rows)} soy satırı")
    return 0


def main():
    if "--written" in sys.argv:
        return written()
    check = "--check" in sys.argv
    stale = []
    total = 0
    for command in SOURCES:
        setup, cases = cases_of(command)
        total += len(cases)
        doc = {
            "format": "kentos.command-cases",
            "version": 1,
            "command": command,
            "commandVersion": 1,
            "title": "Soy kaydı",
            "note": "ADR 0598 §2. Bağımsız başvuru scripts/fixtures/lineage_cases.py: komutların kendi ortak durumları, "
                    "ADR'nin kuralıyla adım adım ayarların lineage'ı; KentOS kodu yok.",
            "setup": setup,
            "cases": cases,
        }
        path = DIR / f"{command}.lineage.json"
        text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
        if check:
            if not path.exists() or path.read_text(encoding="utf-8") != text:
                stale.append(path.name)
        else:
            path.write_text(text, encoding="utf-8")
    if stale:
        print("güncel değil: " + ", ".join(stale), file=sys.stderr)
        return 1
    print(f"Soy kaydı durumları {'tutarlı' if check else 'yazıldı'}: {total} durum")
    return 0


if __name__ == "__main__":
    sys.exit(main())
