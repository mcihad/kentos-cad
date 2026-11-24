#!/usr/bin/env python3
"""Modüllerin bağımlılık denetimi (ADR 9195 §6, CLAUDE.md §25 kural 3).

Masaüstünün kaynağında her dosyanın başka klasördeki modüllere bağlarını ve modül adına göre dallarını sayar: çekirdeğin
paketi (apps/desktop/src) ve kendi paketine ayrılmış modüller (ADR 9199: Harita apps/desktop-map/src). Bir modül
paketinin dosyası o modülün klasörüdür; çekirdek modül paketlerine Cargo'nun kuralıyla da bağlanamaz.

- Klasörler: shell (kabuk), cad (CAD çekirdeği), gis, arch, map, road (modüller). Sınama dosyaları sayılmaz: tests/,
  *_tests.rs, tests.rs, sahneler ve resimler.
- İzlenen bağlar: kabuktan cad'e ve modüllere; cad'den modüllere; modülden öbür modüllere; gis'ten cad'e. CAD
  modüllerinin (arch, map, road) CAD çekirdeğine ve her dosyanın kabuğa bağı serbesttir.
- Modül adına göre dal: `Module::Architecture`, `Map`, `Gis`, `Road`, modül klasörlerinin ve kaydın
  (shell/module_profile.rs) dışında.

Taban (scripts/dev/module_deps.baseline.json) her bağ yönünde bağlı dosyaların sayısını ve listesini, kabukta ve
çekirdekte dalların sayısını tutar. Taban yalnız azalır.

    python3 scripts/dev/module_deps.py            bugünkü sayılar
    python3 scripts/dev/module_deps.py --check    taban aşılırsa düşer; azalanı söyler
    python3 scripts/dev/module_deps.py --write    tabanı bugünkü sayılara indirir; aşılıyorsa yazmaz
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "apps" / "desktop" / "src"
# The modules that are packages of their own (docs/adr/9199): each its source folder.
PACKAGES = {"map": ROOT / "apps" / "desktop-map" / "src", "road": ROOT / "apps" / "desktop-road" / "src"}
BASELINE = Path(__file__).resolve().parent / "module_deps.baseline.json"

MODULES = ("gis", "arch", "map", "road")
FOLDERS = ("shell", "cad") + MODULES
# The registry: the shell's one branch by module for its surfaces.
REGISTRY = "shell/module_profile.rs"

# Tracked directions: what may only shrink.
TRACKED = (
    [("shell", t) for t in ("cad",) + MODULES]
    + [("cad", t) for t in MODULES]
    + [("gis", "cad")]
    + [(m, t) for m in MODULES for t in MODULES if m != t]
)

DECL = re.compile(r'#\[path\s*=\s*"([^"]+)"\]\s*(?:#\[[^\]]*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?mod\s+([a-z_][a-z0-9_]*)\s*;')
CRATE_PATH = re.compile(r"\bcrate::([a-z_][a-z0-9_]*)")
CRATE_GROUP = re.compile(r"\bcrate::\{")
BRANCH = re.compile(r"\bModule::(Architecture|Map|Gis|Road)\b")
LINE_COMMENT = re.compile(r"//[^\n]*")
BLOCK_COMMENT = re.compile(r"/\*.*?\*/", re.S)


def declared() -> dict[str, str]:
    """Every top-level module of the desktop's packages and its folder: the core's from lib.rs and test_suites.rs (the
    file's first folder), a module package's from its lib.rs (the module's folder; `tests` for its tests)."""
    names: dict[str, str] = {}
    for file in ("lib.rs", "test_suites.rs"):
        text = (SRC / file).read_text(encoding="utf-8")
        for path, name in DECL.findall(text):
            names[name] = path.split("/")[0]
    for module, src in PACKAGES.items():
        text = (src / "lib.rs").read_text(encoding="utf-8")
        for path, name in DECL.findall(text):
            names[name] = "tests" if path.startswith("tests/") else module
    return names


def is_test(rel: str) -> bool:
    parts = rel.split("/")
    name = parts[-1]
    return (
        "tests" in parts[:-1]
        or name == "tests.rs"
        or name.endswith("_tests.rs")
        or name.endswith("_scenes.rs")
        or name.endswith("_screens.rs")
        or name == "screens.rs"
    )


def group_items(text: str, start: int) -> list[str]:
    """The first names of a `crate::{…}` group's items, its nested groups included only by their head."""
    depth, i, item, items = 1, start, "", []
    while i < len(text) and depth:
        c = text[i]
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
        if depth == 1 and c == ",":
            items.append(item)
            item = ""
        elif depth >= 1 and not (depth == 1 and c == "}"):
            item += c
        i += 1
    items.append(item)
    heads = []
    for it in items:
        m = re.match(r"\s*([a-z_][a-z0-9_]*)", it)
        if m and m.group(1) not in ("self", "super"):
            heads.append(m.group(1))
    return heads


def references(text: str) -> set[str]:
    text = BLOCK_COMMENT.sub("", LINE_COMMENT.sub("", text))
    names = set(CRATE_PATH.findall(text))
    for m in CRATE_GROUP.finditer(text):
        names.update(group_items(text, m.end()))
    return names


def survey() -> tuple[dict[str, list[str]], dict[str, int]]:
    """Each tracked direction's files, and the branches by module name in the shell and the CAD core."""
    owner = declared()
    edges: dict[str, list[str]] = {f"{a}->{b}": [] for a, b in TRACKED}
    branches: dict[str, int] = {"shell": 0, "cad": 0}
    sources = [(folder, SRC / folder, SRC) for folder in FOLDERS if folder not in PACKAGES]
    sources += [(module, src, src) for module, src in PACKAGES.items()]
    for folder, top, base in sources:
        for file in sorted(top.rglob("*.rs")):
            rel = file.relative_to(base).as_posix()
            if is_test(rel) or (base != SRC and rel == "lib.rs"):
                continue
            if base != SRC:
                rel = f"{base.parent.name}/{rel}"
            text = file.read_text(encoding="utf-8")
            targets = {owner[n] for n in references(text) if n in owner}
            for target in sorted(targets):
                key = f"{folder}->{target}"
                if key in edges:
                    edges[key].append(rel)
            if folder in branches and rel != REGISTRY:
                stripped = BLOCK_COMMENT.sub("", LINE_COMMENT.sub("", text))
                branches[folder] += len(BRANCH.findall(stripped))
    return edges, branches


def words(key: str) -> str:
    a, b = key.split("->")
    return f"{a} → {b}"


def main(argv: list[str]) -> int:
    edges, branches = survey()
    if len(argv) <= 1:
        for key, files in edges.items():
            print(f"{words(key)}: {len(files)} dosya")
        for folder, n in branches.items():
            print(f"{folder}: modül adına göre {n} dal")
        return 0
    if argv[1] not in ("--check", "--write"):
        print(f"Bilinmeyen seçenek: {argv[1]}. --check ya da --write verin.", file=sys.stderr)
        return 2
    base = json.loads(BASELINE.read_text(encoding="utf-8")) if BASELINE.exists() else None
    grown, shrunk = [], []
    if base is not None:
        for key, files in edges.items():
            was = base["edges"].get(key, [])
            if len(files) > len(was):
                new = [f for f in files if f not in was] or files
                grown.append(
                    f"{words(key)}: {len(was)} dosyadan {len(files)} dosyaya çıktı; yeni bağ: {', '.join(new)}. "
                    "Modül başka modüle ya da kabuk modülün içine bağlanmaz: ortak çekirdeği, sözleşmeyi ya da "
                    "modülün profilini (shell/module_profile.rs) kullanın."
                )
            elif len(files) < len(was):
                shrunk.append(f"{words(key)}: {len(was)} → {len(files)}")
        for folder, n in branches.items():
            was = base["branches"].get(folder, 0)
            if n > was:
                grown.append(
                    f"{folder}: modül adına göre dal sayısı arttı (taban {was}, şimdi {n}). Kabuk modül adına göre dallanmaz: "
                    "modülün profiline sorun (shell/module_profile.rs)."
                )
            elif n < was:
                shrunk.append(f"{folder} dalları: {was} → {n}")
    if argv[1] == "--write":
        if grown:
            print("Taban aşıldı, yazılmadı:\n" + "\n".join(grown), file=sys.stderr)
            return 1
        BASELINE.write_text(
            json.dumps(
                {
                    "format": "kentos.module-deps",
                    "version": 1,
                    "note": "scripts/dev/module_deps.py'nin tabanı (ADR 9195 §6): yalnız azalır.",
                    "edges": edges,
                    "branches": branches,
                },
                ensure_ascii=False,
                indent=2,
            )
            + "\n",
            encoding="utf-8",
        )
        print(f"Taban yazıldı: {BASELINE.relative_to(ROOT)}")
        return 0
    if base is None:
        print(f"Taban yok: önce --write ile {BASELINE.relative_to(ROOT)} yazın.", file=sys.stderr)
        return 1
    if grown:
        print("Bağımlılık tabanı aşıldı:\n" + "\n".join(grown), file=sys.stderr)
        return 1
    if shrunk:
        print("Taban küçüldü (--write ile yazın): " + "; ".join(shrunk))
    else:
        print("Bağımlılık tabanı korunuyor.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
