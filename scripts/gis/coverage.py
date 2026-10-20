#!/usr/bin/env python3
"""The coverage matrix of commands and processing tools (docs/adr/0548; plan/gis.md CBS-41): not command names alone but
operation + option + data type + host, each cell with the shared cases that exercise it.

Product commands (apps/web/src/contracts/generated/commandCatalog.json):
- hosts: masaüstü (the catalog's `desktop` host), başsız (`headless`), Python (every command is a `kentos.cad` call,
  scripts/python/sdk.py), MCP (every command with a desktop host is a tool, crates/native/mcp/src/tools.rs), sunucu;
- options: the input schema's choice points: a tagged union's variant (`kind: "distance"`), a string enum's value, a
  boolean's true; read through `$ref`s and `oneOf`/`anyOf`;
- data types: the capabilities' target kinds (`capabilities.targets[].kinds`);
- evidence: fixtures/commands/v1/<command>*.json (and desktop/). A choice is exercised when a case's command step has it
  in its input (by the key's name, anywhere in the input), or it is the schema's default and a case leaves the key out;
  a target kind is exercised when a command step names (`$uidOf:N`) an object of that kind in the case's document.

Processing tools (`cad.processing.tools` on a new drawing, through the Python SDK; run this script with .run/py's Python):
- options: a `choice` parameter's values, a `boolean`'s true and false;
- evidence: fixtures/processing/v1/*.json (`run.tool`, `values`; a value left out is the default), and whether the tool
  is run through the command: the `cad.processing.run` / `.batch` cases, and every case of a file the equivalence test
  plays both ways (crates/native/processing/tests/cases/equivalence.rs, docs/adr/0548: the window's run and the command's
  give the same drawing), the headless, Python and MCP path.

Writes docs/gis/coverage.json (every cell) and docs/gis/coverage.md (counts and the open cells). `--check` says
whether both are what this script writes.

    .run/py/bin/python scripts/gis/coverage.py [--check]
"""

from __future__ import annotations

import json
import re
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CATALOG = ROOT / "apps/web/src/contracts/generated/commandCatalog.json"
CASES = [ROOT / "fixtures/commands/v1", ROOT / "fixtures/commands/v1/desktop"]
PROCESSING = ROOT / "fixtures/processing/v1"
EQUIVALENCE = ROOT / "crates/native/processing/tests/cases/equivalence.rs"
OUT_JSON = ROOT / "docs/gis/coverage.json"
OUT_MD = ROOT / "docs/gis/coverage.md"

# ── Schemas: the choice points ─────────────────────────────────────────


def resolve(schema: dict, root: dict) -> dict:
    seen = 0
    while isinstance(schema, dict) and "$ref" in schema and seen < 32:
        name = schema["$ref"].rsplit("/", 1)[-1]
        schema = {**root.get("$defs", {}).get(name, {}), **{k: v for k, v in schema.items() if k != "$ref"}}
        seen += 1
    return schema


def choices(schema: dict) -> list[dict]:
    """Every choice point of an input schema: {key, value, default?}."""
    out: list[dict] = []
    seen_defs: set[str] = set()

    def walk(s: dict, key: str | None, depth: int) -> None:
        if depth > 24 or not isinstance(s, dict):
            return
        if "$ref" in s:
            name = s["$ref"].rsplit("/", 1)[-1]
            if name in seen_defs and depth > 1:
                # A recursive definition is read once.
                s = {k: v for k, v in s.items() if k != "$ref"}
            else:
                seen_defs.add(name)
        s = resolve(s, schema)
        default = s.get("default")
        for alt in s.get("oneOf", []) + s.get("anyOf", []):
            alt = resolve(alt, schema)
            tagged = False
            for pk, ps in (alt.get("properties") or {}).items():
                if isinstance(ps, dict) and "const" in ps and isinstance(ps["const"], str):
                    out.append({"key": pk, "value": ps["const"]})
                    tagged = True
            if not tagged and "const" in alt and isinstance(alt["const"], str) and key:
                out.append({"key": key, "value": alt["const"], **({"default": True} if default == alt["const"] else {})})
            if not tagged and isinstance(alt.get("enum"), list) and key:
                for v in alt["enum"]:
                    if isinstance(v, str):
                        out.append({"key": key, "value": v, **({"default": True} if default == v else {})})
            walk(alt, key, depth + 1)
        if isinstance(s.get("enum"), list) and key:
            for v in s["enum"]:
                if isinstance(v, str):
                    out.append({"key": key, "value": v, **({"default": True} if default == v else {})})
        types = s.get("type")
        if key and (types == "boolean" or (isinstance(types, list) and "boolean" in types)):
            out.append({"key": key, "value": True})
        for pk, ps in (s.get("properties") or {}).items():
            walk(ps, pk, depth + 1)
        if isinstance(s.get("items"), dict):
            walk(s["items"], key, depth + 1)
        if isinstance(s.get("additionalProperties"), dict):
            walk(s["additionalProperties"], key, depth + 1)

    walk(schema, None, 0)
    unique: dict[tuple, dict] = {}
    for c in out:
        k = (c["key"], json.dumps(c["value"]))
        if k not in unique or c.get("default"):
            unique[k] = c
    return sorted(unique.values(), key=lambda c: (c["key"], json.dumps(c["value"])))


# ── Cases: what they exercise ──────────────────────────────────────────


def pairs(value, out: set, keys: set) -> None:
    if isinstance(value, dict):
        for k, v in value.items():
            keys.add(k)
            if isinstance(v, (str, bool)):
                out.add((k, json.dumps(v)))
            pairs(v, out, keys)
    elif isinstance(value, list):
        for v in value:
            pairs(v, out, keys)


UID_OF = re.compile(r"\$uidOf:(\d+)")


def referenced(value) -> set[int]:
    return {int(m) for m in UID_OF.findall(json.dumps(value))}


def kinds_of(document) -> dict[int, str]:
    out: dict[int, str] = {}
    if isinstance(document, dict):
        for e in document.get("entities", []) or []:
            if isinstance(e, dict) and "id" in e and "kind" in e:
                out[int(e["id"])] = e["kind"]
    return out


def command_evidence() -> dict[str, dict]:
    """Per command: the cases, the (key, value) pairs and keys of its command steps' inputs, the kinds they name."""
    ev: dict[str, dict] = defaultdict(lambda: {"cases": 0, "pairs": set(), "omitted": set(), "kinds": set(), "files": set()})
    for folder in CASES:
        for p in sorted(folder.glob("*.json")):
            try:
                data = json.loads(p.read_text(encoding="utf-8"))
            except ValueError:
                continue
            command = data.get("command")
            if not command or not isinstance(data.get("cases"), list):
                continue
            file_kinds = kinds_of(data.get("setup"))
            for case in data["cases"]:
                kinds = {**file_kinds, **kinds_of(case.get("setup"))}
                for step in case.get("steps", []):
                    if step.get("op") not in ("execute", "plan", "validate"):
                        continue
                    name = step.get("command", command)
                    e = ev[name]
                    e["cases"] += 1
                    e["files"].add(str(p.relative_to(ROOT)))
                    inp = step.get("input", {})
                    keys: set = set()
                    pairs(inp, e["pairs"], keys)
                    e["omitted"].add(frozenset(keys))
                    for n in referenced(inp):
                        if n in kinds:
                            e["kinds"].add(kinds[n])
    return ev


def equivalent_files() -> set[str]:
    """The case files the equivalence test plays through the window and the command both."""
    text = EQUIVALENCE.read_text(encoding="utf-8") if EQUIVALENCE.exists() else ""
    found: set[str] = set()
    for m in re.finditer(r"const (?:RASTER_)?FILES: \[&str; \d+\] = \[(.*?)\];", text, re.S):
        found |= set(re.findall(r'"([^"]+\.json)"', m.group(1)))
    return found


def processing_evidence() -> dict[str, dict]:
    equivalent = equivalent_files()
    ev: dict[str, dict] = defaultdict(lambda: {"cases": 0, "values": defaultdict(set), "omitted": set(), "command": 0})
    for p in sorted(PROCESSING.glob("*.json")):
        try:
            data = json.loads(p.read_text(encoding="utf-8"))
        except ValueError:
            continue
        if not isinstance(data, dict) or not isinstance(data.get("cases"), list):
            continue
        both = p.name in equivalent
        for case in data["cases"]:
            run = case.get("run") or {}
            tool = run.get("tool") if isinstance(run, dict) else None
            if not tool and isinstance(run, dict) and run.get("model"):
                tool = f"model:{run['model']}"
            if not tool:
                continue
            e = ev[tool]
            e["cases"] += 1
            if both:
                e["command"] += 1
            values = case.get("values") or {}
            for k, v in values.items():
                if isinstance(v, (str, bool)):
                    e["values"][k].add(json.dumps(v))
            e["omitted"].add(frozenset(values.keys()))
    for name in ("cad.processing.run", "cad.processing.batch"):
        p = ROOT / f"fixtures/commands/v1/{name}.json"
        if not p.exists():
            continue
        data = json.loads(p.read_text(encoding="utf-8"))
        for case in data.get("cases", []):
            for step in case.get("steps", []):
                inp = step.get("input") or {}
                tool = inp.get("tool") if isinstance(inp, dict) else None
                if isinstance(tool, str):
                    ev[tool]["command"] += 1
    return ev


# ── The matrix ─────────────────────────────────────────────────────────


def hosts_of(c: dict) -> dict:
    hosts = c.get("hosts", [])
    return {
        "desktop": "desktop" in hosts,
        "headless": bool(c.get("headless")),
        "python": True,
        "mcp": "desktop" in hosts,
        "server": "server" in hosts,
    }


def command_rows(catalog: dict) -> list[dict]:
    ev = command_evidence()
    rows = []
    for c in catalog["commands"]:
        e = ev.get(c["id"])
        options = []
        for ch in choices(c.get("input", {})):
            key, value = ch["key"], json.dumps(ch["value"])
            hit = bool(e) and (key, value) in e["pairs"]
            if not hit and ch.get("default") and e:
                hit = any(key not in keys for keys in e["omitted"])
            if not hit and ch["value"] is True and e:
                # A boolean's false (or absent) is its common case; true is the option.
                hit = (key, "true") in e["pairs"]
            options.append({"key": key, "value": ch["value"], "covered": hit})
        targets = sorted({k for t in (c.get("capabilities") or {}).get("targets", []) for k in t.get("kinds", [])})
        kinds = [{"kind": k, "covered": bool(e) and (k in e["kinds"] or (k == "*" and bool(e["kinds"])))} for k in targets]
        rows.append({
            "id": c["id"],
            "title": c.get("title", ""),
            "effect": c.get("effect", ""),
            "hosts": hosts_of(c),
            "cases": e["cases"] if e else 0,
            "files": sorted(e["files"]) if e else [],
            "options": options,
            "kinds": kinds,
        })
    return sorted(rows, key=lambda r: r["id"])


def processing_rows() -> list[dict] | None:
    try:
        sys.path.insert(0, str(ROOT / "python"))
        import kentos.cad as cad  # noqa: PLC0415
    except ImportError:
        return None
    doc = cad.Document.new(srid=5256)
    tools = cad.processing.tools(doc).tools
    ev = processing_evidence()
    rows = []
    for t in tools:
        e = ev.get(t.id)
        options = []
        for p in t.parameters:
            j = p.to_json()
            values: list = []
            if j["kind"] == "choice":
                values = [v for v in j.get("options") or [] if isinstance(v, str)]
            elif j["kind"] == "boolean":
                values = [True, False]
            for v in values:
                hit = bool(e) and json.dumps(v) in e["values"].get(j["name"], set())
                if not hit and e and v == j.get("default"):
                    hit = any(j["name"] not in keys for keys in e["omitted"])
                options.append({"key": j["name"], "value": v, "covered": hit})
        rows.append({
            "id": t.id,
            "title": t.label,
            "cases": e["cases"] if e else 0,
            "command": e["command"] if e else 0,
            "options": options,
        })
    return sorted(rows, key=lambda r: r["id"])


def count(rows, field):
    total = sum(len(r[field]) for r in rows)
    covered = sum(sum(1 for x in r[field] if x["covered"]) for r in rows)
    return total, covered


def render(commands: list[dict], processing: list[dict] | None) -> str:
    lines = [
        "# Komutların ve İşlemler araçlarının kapsam matrisi",
        "",
        "`scripts/gis/coverage.py`'nin çıktısı (docs/adr/0548, plan/gis.md CBS-41); elle düzenlemeyin. Her hücre: işlem, seçenek, veri",
        "türü, ev sahibi ve onu deneyen ortak durumlar. Bütün hücreler `coverage.json`'da.",
        "",
        "## Ürün komutları",
        "",
    ]
    hosts = defaultdict(int)
    for r in commands:
        for h, on in r["hosts"].items():
            hosts[h] += on
    ot, oc = count(commands, "options")
    kt, kc = count(commands, "kinds")
    without = [r for r in commands if r["cases"] == 0]
    # Their cases live elsewhere: the server's in its database tests, the file commands' in the headless tests and the
    # format references (fgb_exchange_cases.py, csv_exchange_cases.py, atlas_cases.py …).
    server = [r["id"] for r in without if r["hosts"]["server"]]
    files = [r["id"] for r in without if r["id"].startswith("file.")]
    rest = [r["id"] for r in without if r["id"] not in server and r["id"] not in files]
    lines += [
        f"- {len(commands)} komut; masaüstü {hosts['desktop']}, başsız {hosts['headless']}, Python {hosts['python']}, MCP {hosts['mcp']}, "
        f"sunucu {hosts['server']}.",
        f"- Seçenekler: {ot} hücre, {oc} tanesi bir durumda deneniyor ({ot - oc} açık).",
        f"- Veri türleri (hedef türler): {kt} hücre, {kc} tanesi bir durumda deneniyor ({kt - kc} açık).",
        f"- Ortak komut durumu olmayan: {len(without)}; {len(server)} sunucu komutu (sunucunun veritabanı testlerinde), "
        f"{len(files)} dosya komutu (başsız testlerde ve biçim başvurularında: {', '.join(files)})"
        + (f", öbürleri: {', '.join(rest)}." if rest else "."),
        "",
        "### Açık hücreler",
        "",
        "| Komut | Denenmeyen seçenekler | Denenmeyen türler |",
        "|---|---|---|",
    ]
    for r in commands:
        opts = [f"{o['key']}={json.dumps(o['value'], ensure_ascii=False)}" for o in r["options"] if not o["covered"]]
        kinds = [k["kind"] for k in r["kinds"] if not k["covered"]]
        if opts or kinds:
            lines.append(f"| `{r['id']}` | {', '.join(opts) or '—'} | {', '.join(kinds) or '—'} |")
    lines.append("")
    if processing is not None:
        pt, pc = count(processing, "options")
        no_cases = [r for r in processing if r["cases"] == 0]
        no_command = [r for r in processing if r["command"] == 0]
        lines += [
            "## İşlemler araçları",
            "",
            f"- {len(processing)} araç; seçenekler {pt} hücre, {pc} tanesi bir durumda deneniyor ({pt - pc} açık).",
            f"- Ortak durumu olmayan araç: {len(no_cases)}.",
            f"- Komutla (`cad.processing.run`, `.batch` ya da eşdeğerlik testi; başsız, Python, MCP) durumu olmayan araç: {len(no_command)}.",
            "",
            "### Açık hücreler",
            "",
            "| Araç | Durum | Komutla | Denenmeyen seçenekler |",
            "|---|---|---|---|",
        ]
        for r in processing:
            opts = [f"{o['key']}={json.dumps(o['value'], ensure_ascii=False)}" for o in r["options"] if not o["covered"]]
            if opts or r["cases"] == 0 or r["command"] == 0:
                lines.append(f"| `{r['id']}` | {r['cases']} | {r['command']} | {', '.join(opts) or '—'} |")
        lines.append("")
    else:
        lines += ["## İşlemler araçları", "", "Python SDK'sı (`kentos`) yüklenemedi; betiği `.run/py/bin/python` ile çalıştırın.", ""]
    return "\n".join(lines)


def main() -> None:
    catalog = json.loads(CATALOG.read_text(encoding="utf-8"))
    commands = command_rows(catalog)
    processing = processing_rows()
    data = {"format": "kentos.coverage", "version": 1, "commands": commands, "processing": processing}
    text_json = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    text_md = render(commands, processing)
    if "--check" in sys.argv:
        stale = [p.relative_to(ROOT) for p, t in ((OUT_JSON, text_json), (OUT_MD, text_md))
                 if not p.exists() or p.read_text(encoding="utf-8") != t]
        if stale:
            sys.exit(f"güncel değil: {', '.join(map(str, stale))}; betiği --check'siz çalıştırın")
        print("Kapsam matrisi güncel.")
        return
    OUT_JSON.parent.mkdir(parents=True, exist_ok=True)
    OUT_JSON.write_text(text_json, encoding="utf-8")
    OUT_MD.write_text(text_md, encoding="utf-8")
    ot, oc = count(commands, "options")
    print(f"yazıldı: {OUT_MD.relative_to(ROOT)}, {OUT_JSON.relative_to(ROOT)} ({len(commands)} komut, {oc}/{ot} seçenek)")


if __name__ == "__main__":
    main()
