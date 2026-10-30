#!/usr/bin/env python3
"""The KentOS modules' program names beside the one program file (docs/adr/0802 §1).

KentOS Mimari, KentOS Harita and KentOS GIS are one program, `kentos-cad`, run under
their own names: the program reads the name it was started under. This writes
`kentos-mimari`, `kentos-harita` and `kentos-gis` beside `kentos-cad` in a build
folder (`target/debug`, `target/release`) or an install folder: symbolic links,
hard links where those cannot be made (Windows without the right). A file of
the same name that is not already such a link (the separate copies older
builds made) is replaced; nothing else is touched.

    python3 scripts/desktop/module_links.py target/release
    python3 scripts/desktop/module_links.py --check target/release
"""

import argparse
import os
import sys
from pathlib import Path

MODULES = ["kentos-mimari", "kentos-harita", "kentos-gis"]


def exe(name: str) -> str:
    return name + ".exe" if os.name == "nt" else name


def linked(link: Path, program: Path) -> bool:
    """Whether `link` already runs `program`: a symbolic link to it or the same file."""
    if link.is_symlink():
        return Path(os.readlink(link)).name == program.name
    return link.exists() and os.path.samefile(link, program)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("folder", type=Path, help="the folder kentos-cad is in")
    parser.add_argument("--check", action="store_true", help="only say whether the names are in place")
    args = parser.parse_args()
    program = args.folder / exe("kentos-cad")
    if not program.is_file():
        print(f"{program} yok: önce derleyin (cargo build -p kentos-desktop).", file=sys.stderr)
        return 2
    missing = []
    for name in MODULES:
        link = args.folder / exe(name)
        if linked(link, program):
            continue
        if args.check:
            missing.append(link.name)
            continue
        if link.exists() or link.is_symlink():
            link.unlink()
        try:
            link.symlink_to(program.name)
        except OSError:
            os.link(program, link)
        print(f"{link} → {program.name}")
    if missing:
        print("eksik ya da eski: " + ", ".join(missing), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
