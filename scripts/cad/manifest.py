#!/usr/bin/env python3
"""The CAD operation manifest (docs/adr/0244; plan/cad.md §4 CAD-AUD-01..06, 11, 12).

Every interface item the app publishes (the frozen web inventory's commands and tools, each tool's methods, and
catalog/v1/additions.json's commands) gets one row: what it does as a product operation and through which surfaces
it can be done. The level of an item comes from catalog/v1/operations.json (hand-kept: an explicit entry, else the
first matching rule); everything else is read from the sources:

- the scope (cad, gis, shared; none: on no ribbon) from the two project types' ribbons, with the additions' places;
- the desktop column from the inventory's (apps/desktop/ported.json) and apps/desktop/equivalents.json;
- the product command a tool writes through from the inventory (`productCommand`), an operation of
  `cad.entities.create` / `.edit` from the mapping, checked against the contract's enums;
- Python and MCP from the product command catalog (every command with a desktop host is a `kentos.cad` call and an
  MCP tool) or from the mapping's own `python` / `mcp` entries (kentos.address, Document.undo …).

Writes docs/cad/manifest.json (the rows) and docs/cad/manifest.md (counts and the gaps of the CAD side). `--check`
says whether both are what this script writes, and fails on an item without a level, a mapping entry naming no
item, an unknown product command or operation, or a tool whose mapped command disagrees with the inventory.

    python3 scripts/cad/manifest.py [--check]
"""

from __future__ import annotations

import argparse
import fnmatch
import json
import re
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
INVENTORY = ROOT / "docs/inventory/web.json"
ADDITIONS = ROOT / "catalog/v1/additions.json"
OPERATIONS = ROOT / "catalog/v1/operations.json"
CATALOG = ROOT / "apps/web/src/contracts/generated/commandCatalog.json"
EQUIVALENTS = ROOT / "apps/desktop/equivalents.json"
EDIT_RS = ROOT / "crates/shared/contracts/src/cad_edit.rs"
CREATE_RS = ROOT / "crates/shared/contracts/src/cad_create.rs"
LAYER_TREE_RS = ROOT / "crates/shared/contracts/src/cad_layer_tree.rs"
OUT_JSON = ROOT / "docs/cad/manifest.json"
OUT_MD = ROOT / "docs/cad/manifest.md"

LEVELS = ["semantic", "geometry", "processing", "none", "query", "session", "host"]
# Levels whose item changes the drawing or the project.
WRITES = {"semantic", "geometry", "processing", "none"}


def camel(name: str) -> str:
    return name[0].lower() + name[1:]


def enum_names(path: Path, enum: str) -> set[str]:
    """The serde (camelCase) names of a unit enum's variants, read from the contract's source."""
    text = path.read_text(encoding="utf-8")
    body = re.search(r"pub enum " + enum + r" \{(.*?)\n\}", text, re.S)
    if not body:
        raise SystemExit(f"{path}: {enum} yok")
    names = re.findall(r"^\s{4}([A-Z][A-Za-z0-9]*)\s*,", body.group(1), re.M)
    return {camel(n) for n in names}


def ribbon_commands(tabs: list) -> set[str]:
    found: set[str] = set()

    def walk(x: object) -> None:
        if isinstance(x, dict):
            for k, v in x.items():
                if k == "command" and isinstance(v, str):
                    found.add(v)
                elif k in ("items", "overflow") and isinstance(v, list):
                    for i in v:
                        if isinstance(i, str):
                            found.add(i)
                        else:
                            walk(i)
                else:
                    walk(v)
        elif isinstance(x, list):
            for i in x:
                walk(i)

    walk(tabs)
    return found


def build() -> tuple[dict, str, list[str]]:
    inv = json.loads(INVENTORY.read_text(encoding="utf-8"))
    add = json.loads(ADDITIONS.read_text(encoding="utf-8"))
    ops = json.loads(OPERATIONS.read_text(encoding="utf-8"))
    cat = json.loads(CATALOG.read_text(encoding="utf-8"))
    # What the desktop has beyond the inventory's column (docs/inventory/README.md).
    equivalents = json.loads(EQUIVALENTS.read_text(encoding="utf-8")).get("commands", {})
    problems: list[str] = []

    catalog = {c["id"]: c for c in cat["commands"]}
    local = {i for i, c in catalog.items() if "desktop" in c.get("hosts", [])}
    operations = {
        "cad.entities.edit": enum_names(EDIT_RS, "EditOperation"),
        "cad.entities.create": enum_names(CREATE_RS, "CreateOperation"),
        # The layer tree's commands (docs/adr/0248).
        "cad.layers.edit": enum_names(LAYER_TREE_RS, "LayerEditOperation"),
        "cad.layers.set": enum_names(LAYER_TREE_RS, "LayerSetOperation"),
        "cad.layers.states": enum_names(LAYER_TREE_RS, "LayerStatesOperation"),
    }

    rb = inv["layout"]["ribbonByMode"]
    cad, gis = ribbon_commands(rb["cad"]), ribbon_commands(rb["gis"])
    for place in add.get("ribbon", []):
        ids = {i["command"] for i in place["items"]} | set(place.get("overflow", []))
        if "cad" in place["modes"]:
            cad |= ids
        if "gis" in place["modes"]:
            gis |= ids

    def scope(command: str) -> str:
        a, b = command in cad, command in gis
        return "shared" if a and b else "cad" if a else "gis" if b else "none"

    def level_of(cid: str) -> dict | None:
        if cid in ops["commands"]:
            return ops["commands"][cid]
        for rule in ops["rules"]:
            if fnmatch.fnmatchcase(cid, rule["match"]):
                return rule
        return None

    def surfaces(entry: dict, via: list[str]) -> tuple[str | None, str | None]:
        python = entry.get("python")
        mcp = entry.get("mcp")
        if entry["level"] in ("semantic", "geometry") and via and all(v in catalog for v in via):
            names = [v for v in via]
            python = python or ", ".join(f"kentos.cad.{v.replace('cad.', '', 1)}" if v.startswith("cad.") else f"Connection: {v}" for v in names)
            mcp = mcp or ", ".join(names if all(v in local or v.startswith("project.") for v in names) else [])
        return python or None, mcp or None

    def check_via(where: str, via: list[str], operation: str | None) -> None:
        for v in via:
            if v not in catalog:
                problems.append(f"{where}: “{v}” ürün komutu katalogda yok")
        if operation:
            known = [operations[v] for v in via if v in operations]
            if not known or not any(operation in k for k in known):
                problems.append(f"{where}: “{operation}” işlemi {', '.join(via) or '—'} komutunda yok")

    rows: list[dict] = []
    tools = {t["id"]: t for t in inv["tools"]}
    for key in ops["tools"]:
        if key not in tools:
            problems.append(f"operations.json tools: “{key}” aracı envanterde yok")
    for key in ops["commands"]:
        known = {c["id"] for c in inv["commands"]} | {c["id"] for c in add["commands"]}
        if key not in known:
            problems.append(f"operations.json commands: “{key}” komutu envanterde ve eklerde yok")

    # Tools and their methods.
    for tid, t in sorted(tools.items()):
        entry = ops["tools"].get(tid)
        command = f"tool.{tid}"
        if entry is None:
            problems.append(f"araç “{tid}”: düzeyi yok (catalog/v1/operations.json tools)")
            continue
        if entry["level"] not in LEVELS:
            problems.append(f"araç “{tid}”: “{entry['level']}” düzeyi yok")
        via = entry.get("via") or ([t["productCommand"]] if t.get("productCommand") else [])
        # The inventory is frozen (docs/adr/0228): a tool moved to another command says which it was.
        if (
            entry.get("via")
            and t.get("productCommand")
            and t["productCommand"] not in entry["via"]
            and entry.get("was") != t["productCommand"]
        ):
            problems.append(f"araç “{tid}”: eşleme {entry['via']} diyor, envanter {t['productCommand']}")
        operation = entry.get("operation")
        check_via(f"araç “{tid}”", via, operation)
        python, mcp = surfaces(entry, via)
        base = {
            "scope": scope(command),
            "desktop": t["platforms"].get("desktop", "none"),
            "tests": len(t.get("tests", [])),
        }
        rows.append({
            "id": command, "kind": "tool", "title": t["label"], "level": entry["level"], "via": via,
            "operation": operation, "python": python, "mcp": mcp, "note": entry.get("note"), **base,
        })
        methods = entry.get("methods", {})
        # The inventory's methods, then the additions' (docs/adr/0287).
        listed = list(t.get("methods") or []) + [
            m["label"] for m in add.get("methods", []) if m["command"] == command
        ]
        for m in listed:
            if m not in methods:
                problems.append(f"araç “{tid}”: “{m}” yönteminin düzeyi yok")
                continue
            level = methods[m]
            mp, mm = surfaces({"level": level}, via)
            rows.append({
                "id": f"{command}#{m}", "kind": "method", "title": f"{t['label']}: {m}", "level": level, "via": via,
                "operation": operation, "python": mp, "mcp": mm, "note": None, **base,
            })
        for m in methods:
            if m not in listed:
                problems.append(f"araç “{tid}”: “{m}” yöntemi envanterde yok")

    # Commands (the tools' commands are their tools' rows).
    commands = [(c, "inventory") for c in inv["commands"]] + [(c, "additions") for c in add["commands"]]
    for c, source in sorted(commands, key=lambda x: x[0]["id"]):
        cid = c["id"]
        if cid.startswith("tool.") and cid[5:] in tools:
            continue
        entry = level_of(cid)
        if entry is None:
            problems.append(f"komut “{cid}”: düzeyi yok (catalog/v1/operations.json)")
            continue
        if entry["level"] not in LEVELS:
            problems.append(f"komut “{cid}”: “{entry['level']}” düzeyi yok")
        via = list(entry.get("via", []))
        operation = entry.get("operation")
        check_via(f"komut “{cid}”", via, operation)
        python, mcp = surfaces(entry, via)
        rows.append({
            "id": cid, "kind": "command", "title": c["title"], "level": entry["level"], "via": via,
            "operation": operation, "python": python, "mcp": mcp, "note": entry.get("note"),
            "scope": scope(cid),
            "desktop": "implemented" if source == "additions"
            else equivalents.get(cid, {}).get("desktop") or c["platforms"].get("desktop", "none"),
            "tests": len(c.get("tests", [])),
        })

    rows.sort(key=lambda r: r["id"])
    manifest = {
        "format": "kentos.cad-manifest",
        "version": 1,
        "sources": {
            "inventory": str(INVENTORY.relative_to(ROOT)),
            "additions": str(ADDITIONS.relative_to(ROOT)),
            "operations": str(OPERATIONS.relative_to(ROOT)),
            "catalog": str(CATALOG.relative_to(ROOT)),
            "equivalents": str(EQUIVALENTS.relative_to(ROOT)),
        },
        "levels": ops["levels"],
        "rows": rows,
    }
    return manifest, markdown(rows, ops["levels"]), problems


def gap(r: dict) -> bool:
    """A CAD-side item a script or an agent cannot do as the interface does."""
    if r["scope"] == "gis":
        return False
    if r["level"] in ("geometry", "none", "processing"):
        return True
    return r["level"] == "query" and not r["python"]


def markdown(rows: list[dict], levels: dict) -> str:
    out: list[str] = []
    w = out.append
    w("# CAD işlem manifesti")
    w("")
    w("Üretilir: `python3 scripts/cad/manifest.py` (ADR 0244). Elle düzenlenmez; düzeyler `catalog/v1/operations.json`'dadır.")
    w("Satırlar arayüzün komutları, araçları ve araçların yöntemleridir; tamamı `manifest.json`'da.")
    w("")
    w("## Düzeyler")
    w("")
    for k in LEVELS:
        w(f"- **{k}:** {levels[k]}")
    w("")
    w("## Sayılar")
    w("")
    scopes = ["cad", "shared", "none", "gis"]
    w("| Düzey | " + " | ".join(scopes) + " | Toplam |")
    w("|---|" + "---|" * (len(scopes) + 1))
    for lv in LEVELS:
        n = Counter(r["scope"] for r in rows if r["level"] == lv)
        w(f"| {lv} | " + " | ".join(str(n[s]) for s in scopes) + f" | {sum(n.values())} |")
    n = Counter(r["scope"] for r in rows)
    w("| **toplam** | " + " | ".join(str(n[s]) for s in scopes) + f" | {len(rows)} |")
    w("")
    w("Kapsam: `cad` yalnız CAD şeridinde, `gis` yalnız CBS şeridinde, `shared` ikisinde, `none` hiçbir şeritte (kısayol, menü,")
    w("komut satırı).")
    w("")
    cadside = [r for r in rows if r["scope"] != "gis"]
    w("## CAD tarafı: Python ve AI'ın arayüz gibi yapamadıkları")
    w("")
    gaps = [r for r in cadside if gap(r)]
    w(f"CBS'ye özgü olmayan {len(cadside)} satırdan {len(gaps)}'i: {', '.join(f'{lv} {sum(1 for r in gaps if r['level'] == lv)}' for lv in ('geometry', 'none', 'processing', 'query'))}.")
    w("")
    for lv, head in (
        ("geometry", "Sonucu çağıran hesaplar (geometry)"),
        ("none", "Ürün komutu yok (none)"),
        ("processing", "Yalnız İşlemler'den (processing)"),
        ("query", "Okuma, Python'da yok (query)"),
    ):
        part = [r for r in gaps if r["level"] == lv]
        if not part:
            continue
        w(f"### {head} — {len(part)}")
        w("")
        w("| Öğe | Ad | Kapsam | Komut | Not |")
        w("|---|---|---|---|---|")
        for r in part:
            via = ", ".join(r["via"]) + (f" `{r['operation']}`" if r["operation"] else "")
            w(f"| `{r['id']}` | {r['title']} | {r['scope']} | {via or '—'} | {r['note'] or ''} |")
        w("")
    pending = [r for r in rows if r["desktop"] not in ("implemented", "n/a") and r["scope"] != "gis"]
    w(f"## Masaüstünde olmayan CAD tarafı öğeleri — {len(pending)}")
    w("")
    w("Envanterin masaüstü sütunu web'in donduğu gündeki hâlidir (ADR 0228); ekler masaüstündedir.")
    w("")
    for r in pending:
        w(f"- `{r['id']}` {r['title']} ({r['desktop']})")
    w("")
    return "\n".join(out)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    a = ap.parse_args()
    manifest, md, problems = build()
    if problems:
        for p in problems:
            print(p, file=sys.stderr)
        print(f"{len(problems)} sorun", file=sys.stderr)
        return 1
    text = json.dumps(manifest, ensure_ascii=False, indent=1) + "\n"
    if a.check:
        stale = [p for p, t in ((OUT_JSON, text), (OUT_MD, md)) if not p.exists() or p.read_text(encoding="utf-8") != t]
        if stale:
            print("güncel değil: " + ", ".join(str(p.relative_to(ROOT)) for p in stale), file=sys.stderr)
            return 1
        print(f"CAD manifesti tutarlı: {len(manifest['rows'])} satır")
        return 0
    OUT_JSON.parent.mkdir(parents=True, exist_ok=True)
    OUT_JSON.write_text(text, encoding="utf-8")
    OUT_MD.write_text(md, encoding="utf-8")
    print(f"yazıldı: {len(manifest['rows'])} satır")
    return 0


if __name__ == "__main__":
    sys.exit(main())
