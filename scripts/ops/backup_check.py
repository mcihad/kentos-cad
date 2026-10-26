#!/usr/bin/env python3
"""A KentOS backup folder checked without KentOS code (docs/adr/0650).

The folder `kentosd backup --dir` writes holds `yedek.json` (format `kentos.backup/1`), `veritabani.dump` (pg_dump's
custom format) and `nesneler/<key>` for each object the database's rows name. This checks, with hashlib and
PostgreSQL's own `pg_restore --list`:

- the record's format and fields;
- the dump's size and SHA-256 are the record's, and pg_restore reads its table of contents, which holds the tables
  the objects are named in (`project_file_revision`, `project_checkpoint`, `server_job`, `job_input`) and their data;
- every object the record names is under `nesneler/` with its size and SHA-256, under a key made only of ids,
  numbers and hex (`revisions/…`, `checkpoints/…`, `jobs/…`, `job-inputs/…`), and the key's SHA-256 suffix is its own;
- no file under `nesneler/` is unnamed.

    python3 scripts/ops/backup_check.py KLASÖR
"""

import hashlib
import json
import os
import re
import shutil
import subprocess
import sys

TABLES = {"project_file_revision", "project_checkpoint", "server_job", "job_input"}
ROOTS = {"revisions", "checkpoints", "jobs", "job-inputs"}
PART = re.compile(r"^[A-Za-z0-9-]+$")


def digest(path):
    h = hashlib.sha256()
    size = 0
    with open(path, "rb") as f:
        while True:
            chunk = f.read(1 << 20)
            if not chunk:
                break
            h.update(chunk)
            size += len(chunk)
    return size, h.hexdigest()


def check(folder):
    problems = []
    with open(os.path.join(folder, "yedek.json"), encoding="utf-8") as f:
        record = json.load(f)
    if record.get("format") != "kentos.backup/1":
        problems.append(f"biçim {record.get('format')!r}")
    for field in ("taken", "database", "migration", "dumpSize", "dumpSha256", "objects"):
        if field not in record:
            problems.append(f"yedek.json'da {field} yok")
    if problems:
        return problems
    dump = os.path.join(folder, "veritabani.dump")
    size, sha = digest(dump)
    if (size, sha) != (record["dumpSize"], record["dumpSha256"]):
        problems.append(f"döküm farklı: {size} {sha}")
    restore = shutil.which(os.environ.get("KENTOS_PG_RESTORE", "pg_restore"))
    if restore:
        listing = subprocess.run([restore, "--list", dump], capture_output=True, text=True)
        if listing.returncode != 0:
            problems.append(f"pg_restore dökümü okuyamadı: {listing.stderr.strip()}")
        else:
            for table in sorted(TABLES):
                if not re.search(rf"TABLE kentos {table} ", listing.stdout):
                    problems.append(f"dökümde kentos.{table} tablosu yok")
                if not re.search(rf"TABLE DATA kentos {table} ", listing.stdout):
                    problems.append(f"dökümde kentos.{table} verisi yok")
    else:
        problems.append("pg_restore bulunamadı (KENTOS_PG_RESTORE)")
    objects = os.path.join(folder, "nesneler")
    named = set()
    for o in record["objects"]:
        key = o["key"]
        named.add(key)
        parts = key.split("/")
        if parts[0] not in ROOTS or not all(PART.match(p) for p in parts):
            problems.append(f"anahtar geçersiz: {key}")
            continue
        if o["table"] not in TABLES:
            problems.append(f"{key}: tablo {o['table']!r}")
        if not parts[-1].endswith("-" + o["sha256"]):
            problems.append(f"{key}: anahtarın SHA-256'sı satırınki değil")
        path = os.path.join(objects, *parts)
        if not os.path.exists(path):
            problems.append(f"eksik: {key}")
            continue
        if digest(path) != (o["size"], o["sha256"]):
            problems.append(f"farklı: {key}")
    for root, _, files in os.walk(objects):
        for name in files:
            key = os.path.relpath(os.path.join(root, name), objects).replace(os.sep, "/")
            if key not in named:
                problems.append(f"adlanmamış: {key}")
    return problems


def main():
    if len(sys.argv) != 2:
        print(__doc__)
        return 2
    problems = check(sys.argv[1])
    for p in problems:
        print(p)
    if problems:
        return 1
    with open(os.path.join(sys.argv[1], "yedek.json"), encoding="utf-8") as f:
        record = json.load(f)
    print(f"yedek sağlam: {len(record['objects'])} nesne, döküm {record['dumpSize']} bayt, göç {record['migration']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
