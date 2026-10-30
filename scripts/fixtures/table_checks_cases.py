#!/usr/bin/env python3
"""A bound table's CHECK constraints as the databases judge them (docs/adr/0755 §3), without KentOS code.

    python3 scripts/fixtures/table_checks_cases.py --check                 # SQLite's answers again, PostgreSQL's from the file
    python3 scripts/fixtures/table_checks_cases.py --check --pg            # PostgreSQL's again too (a throwaway server)
    python3 scripts/fixtures/table_checks_cases.py --write                 # write fixtures/table-checks/v1/cases.json

Each case is a table with one CHECK constraint, the fields KentOS reads its columns as, and records as KentOS holds
them (their texts). Every record is written into the table as KentOS writes it (SQLite: an INTEGER column's text as an
integer, a DOUBLE's as a real, a BOOLEAN's as 1 or 0; PostgreSQL: each text cast to the column's type) and the
database's own answer is kept (`ok`). The constraint's text is what the database gives back, without the blanks at
its ends: SQLite's as written in its CREATE TABLE, PostgreSQL's from pg_get_constraintdef. PostgreSQL is a throwaway postgres:16-alpine container on a
free port (never the developer's database on 5432).

KentOS's verdict (`kentos`): for a constraint KentOS judges whole it is the table's (it refuses what the table refuses);
for one it judges in part or leaves to the table it is written by hand, and it may refuse only what the table refused.
A list a constraint gives a field is tried value by value against the table: each goes in, one outside does not.
"""

import argparse
import json
import os
import socket
import sqlite3
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/table-checks/v1/cases.json"
IMAGE = "postgres:16-alpine"
PASSWORD = "kentos-gecici"
OUTSIDE = "kentos-disarida"


def case(name, dialect, columns, check, rows, *, constraint="k", judged="whole", said=None, values=None,
         kentos=None, table_only=False):
    """`columns`: (name, declared type, field kind[, extra declaration]); `rows`: the records' values by field."""
    return {
        "name": name,
        "dialect": dialect,
        "columns": columns,
        "check": check,
        "constraint": constraint,
        "rows": rows,
        "judged": judged,
        "said": said,
        "list": values,
        "kentos": kentos,
        "tableOnly": table_only,
    }


CASES = [
    # ── SQLite ────────────────────────────────────────────────────────────────────────────────────────────────────────
    case("SQLite: değer listesi", "sqlite", [("tur", "TEXT", "text")], "tur IN ('is', 'personel', 'butce_disi')",
         [{"tur": "is"}, {"tur": "personel"}, {"tur": "Is"}, {"tur": ""}, {}, {"tur": "butce_disi"}],
         said="tur şunlardan biri olmalı: is, personel, butce_disi", values=["is", "personel", "butce_disi"]),
    case("SQLite: boş ya da listeden", "sqlite", [("durum", "TEXT", "text")], "durum IS NULL OR durum IN ('A', 'P')",
         [{"durum": "A"}, {"durum": "B"}, {}, {"durum": "a"}],
         said="durum şunlardan biri olmalı: A, P", values=["A", "P"]),
    case("SQLite: listede olmamalı", "sqlite", [("kod", "TEXT", "text")], "kod NOT IN ('x', 'y')",
         [{"kod": "x"}, {"kod": "z"}, {}], said="kod şunlardan biri olmamalı: x, y"),
    case("SQLite: arasında", "sqlite", [("oncelik", "INTEGER", "integer")], "oncelik BETWEEN 1 AND 3",
         [{"oncelik": "1"}, {"oncelik": "3"}, {"oncelik": "0"}, {"oncelik": "4"}, {}],
         said="oncelik 1 ile 3 arasında olmalı"),
    case("SQLite: iki uçlu aralık", "sqlite", [("oncelik", "INTEGER", "integer")], "oncelik >= 1 AND oncelik <= 3",
         [{"oncelik": "2"}, {"oncelik": "-1"}, {"oncelik": "7"}], said="oncelik 1 ile 3 arasında olmalı"),
    case("SQLite: metin sütunu sayıyla metin olarak karşılaştırılır", "sqlite", [("x", "TEXT", "text")], "x > 10",
         [{"x": "9"}, {"x": "2"}, {"x": "10"}, {"x": "1"}, {"x": "100"}, {"x": "abc"}],
         said="x 10 değerinden büyük olmalı"),
    case("SQLite: tam sayı sütunu metinle sayı olarak karşılaştırılır", "sqlite", [("n", "INTEGER", "integer")],
         "n <> '5'", [{"n": "5"}, {"n": "6"}], said="n 5 olmamalı"),
    case("SQLite: iki tarih", "sqlite", [("baslangic", "DATE", "date"), ("bitis", "DATE", "date")],
         "bitis > baslangic",
         [{"baslangic": "2024-01-01", "bitis": "2024-02-01"}, {"baslangic": "2024-02-01", "bitis": "2024-01-01"},
          {"baslangic": "2024-01-01"}, {"baslangic": "2024-03-01", "bitis": "2024-03-01"}],
         said="bitis, baslangic alanından büyük olmalı"),
    case("SQLite: GLOB", "sqlite", [("kod", "TEXT", "text")], "kod GLOB '[0-9][0-9]'",
         [{"kod": "06"}, {"kod": "6"}, {"kod": "ab"}, {"kod": "061"}], said="kod şu kalıba uymalı: [0-9][0-9]"),
    case("SQLite: LIKE büyük küçük harfe bakmaz (ASCII)", "sqlite", [("ad", "TEXT", "text")], "ad LIKE 'A%'",
         [{"ad": "Ankara"}, {"ad": "ankara"}, {"ad": "İzmir"}, {"ad": "Bursa"}], said="ad şu kalıba uymalı: A%"),
    case("SQLite: boşluksuz uzunluk", "sqlite", [("gerekce", "TEXT", "text")], "length(trim(gerekce)) >= 3",
         [{"gerekce": "abc"}, {"gerekce": " a "}, {"gerekce": "  abcd"}, {"gerekce": ""}, {}],
         said="gerekce baştaki ve sondaki boşluklar sayılmadan en az 3 karakter olmalı"),
    case("SQLite: boş metin olmamalı", "sqlite", [("ad", "TEXT", "text")], "ad <> ''",
         [{"ad": ""}, {"ad": "a"}, {}], said="ad boş metin olmamalı"),
    case("SQLite: typeof", "sqlite", [("deger", "TEXT", "text")], "typeof(deger) <> 'text' OR length(deger) <= 3",
         [{"deger": "abc"}, {"deger": "abcd"}, {}]),
    case("SQLite: açık COLLATE NOCASE", "sqlite", [("ad", "TEXT", "text")], "ad COLLATE NOCASE IN ('ankara', 'izmir')",
         [{"ad": "Ankara"}, {"ad": "ANKARA"}, {"ad": "Bursa"}], said="ad şunlardan biri olmalı: ankara, izmir"),
    case("SQLite: sütunun harmanlaması yalnız tablonun", "sqlite", [("ad", "TEXT", "text", "COLLATE NOCASE")],
         "ad IN ('ankara')", [{"ad": "Ankara"}, {"ad": "x"}], judged="tableOnly", table_only=True,
         kentos=["takes", "takes"]),
    case("SQLite: toplam", "sqlite", [("a", "INTEGER", "integer"), ("b", "INTEGER", "integer"), ("c", "INTEGER", "integer")],
         "a + b <= c", [{"a": "1", "b": "2", "c": "3"}, {"a": "2", "b": "2", "c": "3"}, {"a": "1", "c": "0"}]),
    case("SQLite: tam sayı bölmesi", "sqlite", [("a", "INTEGER", "integer")], "a / 2 = 1",
         [{"a": "2"}, {"a": "3"}, {"a": "4"}, {"a": "1"}, {"a": "-3"}]),
    case("SQLite: gerçel sayı", "sqlite", [("x", "DOUBLE", "decimal")], "x >= 0.1",
         [{"x": "0.1"}, {"x": "0.05"}, {"x": "0.30000000000000004"}, {"x": "1e-1"}], said="x en az 0.1 olmalı"),
    case("SQLite: gerçel toplamın son biti", "sqlite", [("x", "DOUBLE", "decimal"), ("y", "DOUBLE", "decimal")],
         "x + y <= 0.3", [{"x": "0.1", "y": "0.1"}, {"x": "0.1", "y": "0.2"}, {"x": "0.2", "y": "0.2"}],
         kentos=["takes", "takes", "refuses"]),
    case("SQLite: CASE", "sqlite", [("tur", "TEXT", "text"), ("alan", "DOUBLE", "decimal")],
         "CASE WHEN tur = 'park' THEN alan > 100 ELSE 1 END",
         [{"tur": "park", "alan": "150"}, {"tur": "park", "alan": "50"}, {"tur": "yol", "alan": "5"}, {"tur": "park"}]),
    case("SQLite: coalesce", "sqlite", [("n", "INTEGER", "integer")], "coalesce(n, 0) >= 0",
         [{"n": "-1"}, {}, {"n": "5"}]),
    case("SQLite: date", "sqlite", [("d", "DATE", "date")], "d = date(d)",
         [{"d": "2024-01-15"}, {"d": "2024-02-30"}, {}], kentos=["takes", "takes", "takes"]),
    case("SQLite: upper ASCII'dir", "sqlite", [("kod", "TEXT", "text")], "kod = upper(kod)",
         [{"kod": "AB"}, {"kod": "ab"}, {"kod": "çI"}, {"kod": "çi"}]),
    case("SQLite: substr", "sqlite", [("kod", "TEXT", "text")], "substr(kod, 1, 2) = '06'",
         [{"kod": "0601"}, {"kod": "0701"}, {"kod": "0"}]),
    case("SQLite: replace", "sqlite", [("kod", "TEXT", "text")], "length(replace(kod, '.', '')) = 4",
         [{"kod": "12.34"}, {"kod": "1.2"}]),
    case("SQLite: evet/hayır", "sqlite", [("aktif", "BOOLEAN", "boolean"), ("neden", "TEXT", "text")],
         "aktif OR neden IS NOT NULL",
         [{"aktif": "true"}, {"aktif": "false"}, {"aktif": "false", "neden": "x"}, {}]),
    case("SQLite: adsız kısıt", "sqlite", [("x", "INTEGER", "integer")], "x > 0", [{"x": "0"}, {"x": "1"}],
         constraint=None, said="x 0 değerinden büyük olmalı"),
    case("SQLite: çift tırnaklı değer", "sqlite", [("tur", "TEXT", "text")], 'tur IN ("a", "b")',
         [{"tur": "a"}, {"tur": "c"}], values=["a", "b"], said="tur şunlardan biri olmalı: a, b"),
    case("SQLite: abs", "sqlite", [("x", "INTEGER", "integer")], "abs(x) <= 10", [{"x": "-11"}, {"x": "-10"}]),
    case("SQLite: IS NOT", "sqlite", [("x", "TEXT", "text")], "x IS NOT ''", [{"x": ""}, {}, {"x": "a"}],
         said="x boş metin olmamalı"),
    case("SQLite: bilinmeyen işlev", "sqlite", [("x", "TEXT", "text")], "x <> '' AND hex(x) <> '41'",
         [{"x": ""}, {"x": "A"}, {"x": "B"}], judged="partly", kentos=["refuses", "takes", "takes"]),
    case("SQLite: büyük harfli sütun adı", "sqlite", [("Tur", "TEXT", "text")], "TUR IN ('a')",
         [{"Tur": "a"}, {"Tur": "b"}], values=["a"], said="Tur şunlardan biri olmalı: a"),
    # ── PostgreSQL ────────────────────────────────────────────────────────────────────────────────────────────────────
    case("PostgreSQL: değer listesi", "postgres", [("tur", "varchar(20)", "text")],
         "tur IN ('is', 'personel', 'butce_disi')",
         [{"tur": "is"}, {"tur": "Is"}, {}, {"tur": "butce_disi"}, {"tur": "is "}],
         said="tur şunlardan biri olmalı: is, personel, butce_disi", values=["is", "personel", "butce_disi"]),
    case("PostgreSQL: listede olmamalı", "postgres", [("kod", "text", "text")], "kod NOT IN ('x', 'y')",
         [{"kod": "x"}, {"kod": "z"}], said="kod şunlardan biri olmamalı: x, y"),
    case("PostgreSQL: arasında", "postgres", [("oncelik", "integer", "integer")], "oncelik BETWEEN 1 AND 3",
         [{"oncelik": "1"}, {"oncelik": "4"}, {}], said="oncelik 1 ile 3 arasında olmalı"),
    case("PostgreSQL: iki tarih", "postgres", [("baslangic", "date", "date"), ("bitis", "date", "date")],
         "bitis IS NULL OR bitis > baslangic",
         [{"baslangic": "2024-01-01", "bitis": "2024-02-01"}, {"baslangic": "2024-02-01", "bitis": "2024-01-01"},
          {"baslangic": "2024-01-01"}], said="bitis, baslangic alanından büyük olmalı"),
    case("PostgreSQL: düzenli ifade", "postgres", [("kod", "varchar(10)", "text")], "kod ~ '^[0-9]{2}$'",
         [{"kod": "06"}, {"kod": "6"}, {"kod": "ç6"}], said="kod şu kalıba uymalı: ^[0-9]{2}$",
         kentos=["takes", "refuses", "takes"]),
    case("PostgreSQL: boşluksuz uzunluk", "postgres", [("gerekce", "text", "text")], "length(btrim(gerekce)) >= 3",
         [{"gerekce": "abc"}, {"gerekce": " a "}, {}],
         said="gerekce baştaki ve sondaki boşluklar sayılmadan en az 3 karakter olmalı"),
    case("PostgreSQL: metinlerin sırası", "postgres", [("ad", "text", "text")], "ad > 'm'",
         [{"ad": "a"}, {"ad": "z"}], judged="partly", kentos=["takes", "takes"]),
    case("PostgreSQL: JSON işlevi", "postgres", [("veri", "jsonb", "json")], "jsonb_array_length(veri) > 0",
         [{"veri": "[1]"}, {"veri": "[]"}], judged="partly", kentos=["takes", "takes"]),
    case("PostgreSQL: bilinen yanı yanlış", "postgres", [("ad", "text", "text"), ("veri", "jsonb", "json")],
         "ad <> '' AND jsonb_typeof(veri) = 'array'",
         [{"ad": "", "veri": "[]"}, {"ad": "a", "veri": "{}"}, {"ad": "a", "veri": "[]"}],
         judged="partly", kentos=["refuses", "takes", "takes"]),
    case("PostgreSQL: numeric", "postgres", [("tutar", "numeric(12,2)", "decimal")], "tutar >= 0.01",
         [{"tutar": "0.01"}, {"tutar": "0"}, {"tutar": "12.5"}], said="tutar en az 0.01 olmalı"),
    case("PostgreSQL: evet/hayır", "postgres", [("aktif", "boolean", "boolean"), ("neden", "text", "text")],
         "aktif OR neden IS NOT NULL",
         [{"aktif": "true"}, {"aktif": "false"}, {"aktif": "false", "neden": "x"}, {}]),
    case("PostgreSQL: ILIKE", "postgres", [("ad", "text", "text")], "ad ILIKE 'a%'",
         [{"ad": "Ankara"}, {"ad": "bursa"}, {"ad": "Ağrı"}], said="ad şu kalıba uymalı: a%",
         kentos=["takes", "refuses", "takes"]),
    case("PostgreSQL: LIKE", "postgres", [("kod", "text", "text")], "kod LIKE 'TR%'",
         [{"kod": "TR01"}, {"kod": "tr01"}], said="kod şu kalıba uymalı: TR%"),
    case("PostgreSQL: coalesce", "postgres", [("n", "integer", "integer")], "COALESCE(n, 0) >= 0",
         [{"n": "-1"}, {}, {"n": "2"}]),
    case("PostgreSQL: CASE", "postgres", [("tur", "text", "text"), ("alan", "numeric", "decimal")],
         "CASE WHEN tur = 'park' THEN alan > 100 ELSE true END",
         [{"tur": "park", "alan": "150"}, {"tur": "park", "alan": "50"}, {"tur": "yol", "alan": "5"}]),
    case("PostgreSQL: bugünün tarihi", "postgres", [("d", "date", "date")], "d <= CURRENT_DATE",
         [{"d": "2000-01-01"}, {"d": "2999-01-01"}], judged="partly", kentos=["takes", "takes"]),
    case("PostgreSQL: uuid", "postgres", [("u", "uuid", "uuid")], "u <> '00000000-0000-0000-0000-000000000000'",
         [{"u": "00000000-0000-0000-0000-000000000000"}, {"u": "0190f0a4-7c2e-7a3b-9d4e-1f2a3b4c5d6e"}],
         said="u 00000000-0000-0000-0000-000000000000 olmamalı"),
    case("PostgreSQL: iki alan", "postgres", [("a", "integer", "integer"), ("b", "integer", "integer")], "a < b",
         [{"a": "1", "b": "2"}, {"a": "2", "b": "2"}, {"a": "1"}], said="a, b alanından küçük olmalı"),
    case("PostgreSQL: tarih ve saat", "postgres", [("t", "timestamp", "datetime")], "t >= '2020-01-01 00:00:00'",
         [{"t": "2019-12-31T23:59:59"}, {"t": "2020-01-01T00:00:00"}], said="t en az 2020-01-01 00:00:00 olmalı"),
    case("PostgreSQL: karakter sayısı", "postgres", [("kod", "varchar(10)", "text")], "char_length(kod) = 2",
         [{"kod": "06"}, {"kod": "6"}, {"kod": "çö"}], said="kod 2 karakter olmalı"),
    case("PostgreSQL: character(n) yalnız tablonun", "postgres", [("durum", "character(1)", "text")],
         "durum IN ('A', 'P')", [{"durum": "A"}, {"durum": "X"}], judged="tableOnly", table_only=True,
         kentos=["takes", "takes"]),
    case("PostgreSQL: upper ASCII dışında bilinmez", "postgres", [("kod", "text", "text")], "kod = upper(kod)",
         [{"kod": "AB"}, {"kod": "ab"}, {"kod": "Çi"}], kentos=["takes", "refuses", "takes"]),
    case("PostgreSQL: NULLIF", "postgres", [("ad", "text", "text")], "NULLIF(ad, '') IS NOT NULL",
         [{"ad": ""}, {"ad": "a"}, {}]),
    case("PostgreSQL: dizi metni", "postgres", [("x", "text", "text")], "x = ANY ('{a,b}'::text[])",
         [{"x": "a"}, {"x": "c"}], values=["a", "b"], said="x şunlardan biri olmalı: a, b"),
    case("PostgreSQL: tam sayı listesi", "postgres", [("seviye", "integer", "integer")], "seviye IN (1, 2, 3)",
         [{"seviye": "1"}, {"seviye": "4"}], values=["1", "2", "3"], said="seviye şunlardan biri olmalı: 1, 2, 3"),
    case("PostgreSQL: tam sayı bölmesi", "postgres", [("a", "integer", "integer")], "a / 2 = 1",
         [{"a": "2"}, {"a": "3"}, {"a": "4"}]),
    case("PostgreSQL: birleştirme", "postgres", [("il", "text", "text"), ("ilce", "text", "text")],
         "(il || '-' || ilce) <> 'x-y'", [{"il": "x", "ilce": "y"}, {"il": "x", "ilce": "z"}, {"il": "x"}]),
]


# ── SQLite ─────────────────────────────────────────────────────────────────────────────────────────────────────────


def sqlite_cell(text, decl):
    """A value as KentOS writes it into a GeoPackage column of `decl` (docs/adr/0750 §3)."""
    if text is None:
        return None
    base = decl.upper().split("(")[0].strip()
    try:
        if base in ("TINYINT", "SMALLINT", "MEDIUMINT", "INT", "INTEGER"):
            return int(text)
        if base in ("FLOAT", "DOUBLE", "REAL"):
            return float(text)
    except ValueError:
        return text
    if base == "BOOLEAN":
        return {"true": 1, "false": 0}.get(text, text)
    if base == "DATETIME" and len(text) == 19:
        return text + "Z"
    return text


def quote(name):
    return '"' + name.replace('"', '""') + '"'


def sqlite_columns(check, names):
    """The table's columns a constraint's text names, in the table's order: its words and quoted names compared as
    SQLite compares names (ASCII letters' case aside); a string in single quotes names none."""
    words = set()
    i, s = 0, check
    while i < len(s):
        ch = s[i]
        if ch == "'":
            j = i + 1
            while j < len(s):
                if s[j] == "'" and (j + 1 >= len(s) or s[j + 1] != "'"):
                    break
                j += 2 if s[j] == "'" else 1
            i = j + 1
        elif ch in '"[`':
            end = {"[": "]"}.get(ch, ch)
            j = s.index(end, i + 1)
            words.add(s[i + 1:j].lower())
            i = j + 1
        elif ch.isalpha() or ch == "_" or ord(ch) > 127:
            j = i
            while j < len(s) and (s[j].isalnum() or s[j] in "_$" or ord(s[j]) > 127):
                j += 1
            words.add(s[i:j].lower())
            i = j
        else:
            i += 1
    return [n for n in names if n.lower() in words]


def sqlite_answers(c):
    db = sqlite3.connect(":memory:")
    cols = ", ".join(f"{quote(n)} {decl}" + (f" {col[3]}" if len(col) > 3 else "") for col in c["columns"]
                     for n, decl in [col[:2]])
    named = f"CONSTRAINT {c['constraint']} " if c["constraint"] else ""
    ddl = f"CREATE TABLE t (fid INTEGER PRIMARY KEY, {cols}, {named}CHECK ({c['check']}))"
    db.execute(ddl)
    names = [col[0] for col in c["columns"]]
    decls = {col[0]: col[1] for col in c["columns"]}

    def goes_in(values):
        try:
            db.execute(f"INSERT INTO t ({', '.join(quote(n) for n in names)}) VALUES ({', '.join('?' for _ in names)})",
                       [sqlite_cell(values.get(n), decls[n]) for n in names])
            return True
        except sqlite3.IntegrityError as e:
            if "CHECK constraint failed" not in str(e):
                raise
            return False

    return ddl, c["check"], sqlite_columns(c["check"], names), goes_in


# ── PostgreSQL ─────────────────────────────────────────────────────────────────────────────────────────────────────


def free_port():
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


class Postgres:
    """A throwaway postgres server for the cases' tables."""

    def __enter__(self):
        import psycopg2

        self.port = free_port()
        self.name = f"kentos-checks-{os.getpid()}"
        subprocess.run(["docker", "run", "-d", "--rm", "--name", self.name, "-p", f"127.0.0.1:{self.port}:5432",
                        "-e", f"POSTGRES_PASSWORD={PASSWORD}", IMAGE], check=True, stdout=subprocess.DEVNULL)
        print(f"geçici veritabanı: docker {self.name}, 127.0.0.1:{self.port} (5432'deki veritabanına dokunulmuyor)")
        for _ in range(120):
            try:
                self.c = psycopg2.connect(host="127.0.0.1", port=self.port, user="postgres", password=PASSWORD,
                                          dbname="postgres")
                self.c.autocommit = True
                self.c.cursor().execute("SELECT 1")
                return self
            except Exception:
                time.sleep(1)
        raise SystemExit("geçici PostgreSQL açılmadı")

    def __exit__(self, *_):
        subprocess.run(["docker", "stop", self.name], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    def answers(self, c, k):
        import psycopg2
        import psycopg2.errors

        cur = self.c.cursor()
        table = f"t{k}"
        cur.execute(f"DROP TABLE IF EXISTS {table}")
        cols = ", ".join(f"{quote(col[0])} {col[1]}" for col in c["columns"])
        named = f"CONSTRAINT {c['constraint']} " if c["constraint"] else ""
        ddl = f"CREATE TABLE {table} (fid serial PRIMARY KEY, {cols}, {named}CHECK ({c['check']}))"
        cur.execute(ddl)
        cur.execute("SELECT pg_get_constraintdef(oid), conkey FROM pg_constraint WHERE conrelid = %s::regclass "
                    "AND contype = 'c'", (table,))
        definition, conkey = cur.fetchone()
        assert definition.startswith("CHECK (") and definition.endswith(")"), definition
        # Without the blanks at its ends (a CASE comes back over lines).
        sql = definition[len("CHECK ("):-1].strip()
        cur.execute("SELECT attname FROM pg_attribute WHERE attrelid = %s::regclass AND attnum = ANY(%s) "
                    "ORDER BY attnum", (table, conkey))
        columns = [r[0] for r in cur.fetchall()]
        names = [col[0] for col in c["columns"]]
        types = {col[0]: col[1] for col in c["columns"]}

        def goes_in(values):
            given = [n for n in names if n in values]
            try:
                if given:
                    cur.execute(f"INSERT INTO {table} ({', '.join(quote(n) for n in given)}) VALUES "
                                f"({', '.join(f'%s::{types[n]}' for n in given)})", [values[n] for n in given])
                else:
                    cur.execute(f"INSERT INTO {table} DEFAULT VALUES")
                return True
            except (psycopg2.errors.CheckViolation, psycopg2.errors.NotNullViolation):
                return False

        return ddl, sql, columns, goes_in


# ── The cases ──────────────────────────────────────────────────────────────────────────────────────────────────────


def field_of(col):
    return {"name": col[0], "kind": col[2]}


def built(c, ddl, sql, columns, goes_in):
    rows = []
    for k, values in enumerate(c["rows"]):
        ok = goes_in(values)
        if c["kentos"] is not None:
            kentos = c["kentos"][k]
        else:
            assert c["judged"] == "whole", c["name"]
            kentos = "takes" if ok else "refuses"
        if kentos == "refuses" and ok:
            raise SystemExit(f"{c['name']}: KentOS {values} için reddediyor ama tablo kabul etti")
        rows.append({"values": values, "ok": ok, "kentos": kentos})
    if c["list"] is not None:
        col = columns[0]
        for v in c["list"]:
            if not goes_in({col: v}):
                raise SystemExit(f"{c['name']}: listenin “{v}” değerini tablo kabul etmedi")
        kind = next(k[2] for k in c["columns"] if k[0] == col)
        if goes_in({col: "987654" if kind in ("integer", "bigint", "decimal") else OUTSIDE}):
            raise SystemExit(f"{c['name']}: listede olmayan değeri tablo kabul etti")
    check = {"sql": sql, "columns": columns}
    if c["constraint"]:
        check = {"name": c["constraint"], **check}
    if c["tableOnly"]:
        check["tableOnly"] = True
    out = {
        "name": c["name"],
        "dialect": c["dialect"],
        "ddl": ddl,
        "fields": [field_of(col) for col in c["columns"]],
        "check": check,
        "judged": c["judged"],
    }
    if c["said"] is not None:
        out["said"] = c["said"]
    out["list"] = c["list"]
    out["rows"] = rows
    return out


def cases(pg):
    out = []
    for k, c in enumerate(CASES):
        if c["dialect"] == "sqlite":
            out.append(built(c, *sqlite_answers(c)))
        elif pg is not None:
            out.append(built(c, *pg.answers(c, k)))
        else:
            out.append(None)
    return out


def document(found):
    return {
        "about": "docs/adr/0755 §3: a bound table's CHECK constraints judged by SQLite and PostgreSQL themselves "
                 "(scripts/fixtures/table_checks_cases.py); ok: the table took the record; kentos: KentOS's verdict.",
        "cases": found,
    }


def main():
    a = argparse.ArgumentParser()
    a.add_argument("--check", action="store_true")
    a.add_argument("--write", action="store_true")
    a.add_argument("--pg", action="store_true", help="PostgreSQL's answers again from a throwaway server")
    args = a.parse_args()
    if args.write or args.pg:
        with Postgres() as pg:
            found = cases(pg)
    else:
        found = cases(None)
    if args.write:
        OUT.parent.mkdir(parents=True, exist_ok=True)
        OUT.write_text(json.dumps(document(found), ensure_ascii=False, indent=1) + "\n")
        print(f"{OUT.relative_to(ROOT)}: {len(found)} durum, {sum(len(c['rows']) for c in found)} kayıt")
        return
    kept = json.loads(OUT.read_text())["cases"]
    if len(kept) != len(found):
        raise SystemExit(f"{OUT.relative_to(ROOT)} eski: {len(kept)} durum, betikte {len(found)}")
    bad = 0
    for want, got in zip(kept, found):
        if got is None:
            continue
        if want != got:
            bad += 1
            print(f"farklı: {got['name']}", file=sys.stderr)
    if bad:
        raise SystemExit(f"{bad} durum dosyadakinden farklı; --write ile yeniden yazın")
    told = "PostgreSQL'inkiler de yeniden" if args.pg else "PostgreSQL'inkiler dosyadan"
    print(f"{len(found)} durum tutuyor ({told})")


if __name__ == "__main__":
    main()
