#!/usr/bin/env python3
"""Every generator's `--check` in the generators' pinned environment, with a summary (docs/adr/9303).

    make fixtures-check                                         # all of them
    scripts/fixtures/py scripts/fixtures/check_all.py -j 4 wall road_   # those whose path holds `wall` or `road_`
    scripts/fixtures/py scripts/fixtures/check_all.py --list    # which scripts, without running them

A generator is a `scripts/<dir>/<name>.py` that takes `--check`: the flag is compared with its arguments or given to
argparse (read from the script's syntax tree, not its text), or the script only checks whatever it is given
(CHECK_ONLY). Each runs from the repository's root with this Python, one process per script, `-j` at a time; `--check`
writes nothing. The result is

- green: exit 0;
- cannot run: a module this environment does not have (only QGIS's own, ADR 9303), the KentOS Python SDK not built, or
  a service that is not there (a database), each with what it needs;
- red: anything else, the last lines of its output kept; a script over `--timeout` seconds is red too.

A `--check` must write nothing: the tracked files that changed during the run are listed at the end (in a checkout
others work in, their edits show there too). The exit status is 1 when one is red, else 0. `--json FILE` writes every
result.
"""

from __future__ import annotations

import argparse
import ast
import concurrent.futures
import json
import os
import re
import subprocess
import sys
import time
import warnings
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

# Scripts that only check, whatever their arguments: `--check` is the common form's flag and they ignore it. Each was
# read: none writes a file without `--files`/`--written`.
CHECK_ONLY = {
    "scripts/fixtures/aggregate_qgis_cases.py",
    "scripts/fixtures/compress_cases.py",
    "scripts/fixtures/form_schema_cases.py",
    "scripts/fixtures/parquet_cases.py",
    "scripts/fixtures/processing_sink_cases.py",
    "scripts/fixtures/service_rules_cases.py",
    "scripts/fixtures/spatial_esda_cases.py",
    "scripts/fixtures/wide_values_cases.py",
}

# Scripts whose `--check` needs more than itself; not run here, listed in the summary with the reason.
OTHER_FORM = {
    "scripts/desktop/module_links.py": "--check needs the folder the program was built into (target/release)",
    "scripts/mevzuat/olcum.py": "--check needs the IFC files to measure",
    "scripts/mevzuat/pdfa_bagimsiz.py": "not a generator: it gives qpdf its --check and takes a PDF",
    "scripts/fixtures/check_all.py": "this runner",
}

# What a script that cannot run here says: (pattern, what it needs).
CANNOT = [
    (re.compile(r"No module named '(qgis)[.']"), "QGIS's Python (not in this environment, ADR 9303)"),
    (re.compile(r"KentOS'un Python SDK'sı yüklenemedi"),
     "the KentOS Python SDK built at this commit (maturin, ADR 0131)"),
    (re.compile(r"(?:ModuleNotFoundError|ImportError): No module named '?([\w.]+)'?"), "the module {0}"),
    (re.compile(r"psycopg2\.OperationalError|connection to server .* failed|Connection refused"),
     "a PostgreSQL server"),
]


def takes_check(tree: ast.AST) -> bool:
    """Whether `--check` is compared with the arguments or given to argparse (not merely passed to another program)."""
    for node in ast.walk(tree):
        if isinstance(node, ast.Compare):
            parts = [node.left, *node.comparators]
            for p in parts:
                items = p.elts if isinstance(p, (ast.List, ast.Tuple, ast.Set)) else [p]
                if any(isinstance(i, ast.Constant) and i.value == "--check" for i in items):
                    return True
        elif isinstance(node, ast.Call) and getattr(node.func, "attr", None) == "add_argument":
            if any(isinstance(a, ast.Constant) and a.value == "--check" for a in node.args):
                return True
    return False


def generators() -> tuple[list[str], dict[str, str]]:
    found, other = [], {}
    for path in sorted(ROOT.glob("scripts/*/*.py")):
        rel = path.relative_to(ROOT).as_posix()
        text = path.read_text(encoding="utf-8", errors="replace")
        if "--check" not in text:
            continue
        if rel in OTHER_FORM:
            other[rel] = OTHER_FORM[rel]
            continue
        try:
            with warnings.catch_warnings():
                warnings.simplefilter("ignore")  # a script's own invalid escapes are not this runner's to report
                tree = ast.parse(text)
        except SyntaxError as e:
            other[rel] = f"does not parse: {e}"
            continue
        if rel in CHECK_ONLY or takes_check(tree):
            found.append(rel)
        else:
            other[rel] = "--check only in its text, not in its arguments"
    return found, other


def tracked() -> dict[str, tuple[int, int]]:
    """Each tracked file's modification time and size: what a `--check` must leave as it was."""
    names = subprocess.run(["git", "ls-files", "-z"], cwd=ROOT, capture_output=True, check=True).stdout.split(b"\0")
    out = {}
    for name in names:
        if name:
            try:
                st = (ROOT / name.decode()).stat()
            except OSError:
                continue
            out[name.decode()] = (st.st_mtime_ns, st.st_size)
    return out


def run(rel: str, timeout: float) -> dict:
    env = dict(os.environ, PYTHONDONTWRITEBYTECODE="1", KENTOS_QGIS_PYTHON="1")
    start = time.monotonic()
    try:
        p = subprocess.run([sys.executable, rel, "--check"], cwd=ROOT, env=env, capture_output=True, text=True,
                           timeout=timeout, errors="replace")
    except subprocess.TimeoutExpired:
        return {"script": rel, "state": "red", "secs": round(time.monotonic() - start, 1),
                "tail": [f"süre aşıldı: {timeout:g} s"]}
    out = (p.stdout + "\n" + p.stderr).strip().splitlines()
    tail = [line[:400] for line in out[-4:]]
    result = {"script": rel, "secs": round(time.monotonic() - start, 1), "code": p.returncode, "tail": tail}
    if p.returncode == 0:
        result["state"] = "green"
        return result
    for pattern, need in CANNOT:
        m = pattern.search(p.stderr) or pattern.search(p.stdout)
        if m:
            result["state"] = "cannot"
            result["needs"] = need.format(*m.groups())
            return result
    result["state"] = "red"
    return result


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("only", nargs="*", help="run only the scripts whose path holds one of these")
    ap.add_argument("-j", "--jobs", type=int, default=max(1, (os.cpu_count() or 2) // 2))
    ap.add_argument("--timeout", type=float, default=1800.0, help="seconds per script (1800)")
    ap.add_argument("--shard", default="1/1", help="K/N: only every N-th script from the K-th (to split a long run)")
    ap.add_argument("--json", type=Path, help="write every result here")
    ap.add_argument("--list", action="store_true", help="list the scripts and what is not run, run nothing")
    args = ap.parse_args()

    found, other = generators()
    if args.only:
        found = [s for s in found if any(o in s for o in args.only)]
        other = {s: why for s, why in other.items() if any(o in s for o in args.only)}
    k, n = (int(v) for v in args.shard.split("/"))
    found = found[k - 1::n]
    if args.list:
        print("\n".join(found))
        for s, why in other.items():
            print(f"# {s}: {why}")
        return 0
    if ".run/fixtures-py" not in Path(sys.prefix).as_posix():
        print(f"uyarı: bu Python üreteçlerin ortamı değil ({sys.prefix}); scripts/fixtures/py ile çalıştırın.",
              file=sys.stderr)

    results = []
    before = tracked()
    start = time.monotonic()
    with concurrent.futures.ThreadPoolExecutor(args.jobs) as pool:
        futures = {pool.submit(run, s, args.timeout): s for s in found}
        for n, f in enumerate(concurrent.futures.as_completed(futures), 1):
            r = f.result()
            results.append(r)
            mark = {"green": "yeşil", "red": "KIRMIZI", "cannot": "koşamıyor"}[r["state"]]
            print(f"[{n}/{len(found)}] {mark:9} {r['secs']:7.1f} s  {r['script']}", flush=True)
    results.sort(key=lambda r: r["script"])

    green = [r for r in results if r["state"] == "green"]
    red = [r for r in results if r["state"] == "red"]
    cannot = [r for r in results if r["state"] == "cannot"]
    print()
    print(f"Üreteçlerin --check'i ({time.monotonic() - start:.0f} s, {args.jobs} iş): {len(green)} yeşil, "
          f"{len(red)} kırmızı, {len(cannot)} koşamıyor, {len(other)} başka biçimde (koşturulmadı).")
    if red:
        print("\nKırmızı:")
        for r in red:
            print(f"  {r['script']}: {r['tail'][-1] if r['tail'] else ''}")
    if cannot:
        print("\nKoşamıyor:")
        for r in cannot:
            print(f"  {r['script']}: {r['needs']}")
    if other:
        print("\nBaşka biçimde, koşturulmadı:")
        for s, why in other.items():
            print(f"  {s}: {why}")
    # A --check writes nothing; a file that changed meanwhile was written by one (or by someone else in this checkout).
    after = tracked()
    written = sorted(name for name, stamp in after.items() if before.get(name, stamp) != stamp)
    if written:
        print(f"\nKoşu sırasında değişen izlenen dosyalar ({len(written)}; --check yazmamalı):")
        for name in written[:40]:
            print(f"  {name}")
    if args.json:
        args.json.write_text(json.dumps({"results": results, "other": other}, ensure_ascii=False, indent=1) + "\n",
                             encoding="utf-8")
    return 1 if red else 0


if __name__ == "__main__":
    sys.exit(main())
