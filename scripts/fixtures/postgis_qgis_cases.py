#!/usr/bin/env python3
"""KentOS and QGIS editing one PostGIS table at the same time (docs/adr/0582 §2, plan/gis.md CBS-04).

    KENTOS_PG_URL=postgresql://postgres@127.0.0.1:PORT/postgres PGPASSWORD=… KENTOS_PG_PORT=PORT \\
        .run/py/bin/python scripts/fixtures/postgis_qgis_cases.py

Run by postgis_cases.py against its throwaway database (never the developer's 5432), with the Python SDK's
interpreter; QGIS 4.2's own Python bindings and psycopg2 come from the system's packages. QGIS edits through its
PostgreSQL provider, as a colleague at another desk would:

1. KentOS binds sorgu.nokta by an area (12 points) and changes two of them, A and B.
2. QGIS changes A and a third point C, and saves.
3. KentOS's write is refused: `source_changed`, the error names A's object. The table is QGIS's.
4. KentOS gives up its change on A and writes again: B goes through with `source_others`; QGIS's A and C stay.
5. A read brings QGIS's A and C into KentOS's layer. psycopg2 checks the table after each step.
"""

import os
import sys
import warnings
from pathlib import Path

sys.path.extend(["/usr/lib/python3/dist-packages", "/usr/share/qgis/python"])
os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")

import psycopg2  # noqa: E402
from qgis.core import QgsApplication, QgsVectorLayer  # noqa: E402

import kentos.cad as cad  # noqa: E402
from kentos.cad import CommandWarning  # noqa: E402
from kentos.cad.errors import CommandError  # noqa: E402
from kentos.cad.types import LinkSourcesEditOp  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
AREA = [486995.0, 4419995.0, 487105.0, 4420025.0]


def table(port, password):
    c = psycopg2.connect(host="127.0.0.1", port=port, user="postgres", password=password, dbname="postgres")
    cur = c.cursor()
    cur.execute("SELECT fid, ad FROM sorgu.nokta WHERE fid IN (1, 2, 3) ORDER BY fid")
    rows = dict(cur.fetchall())
    c.close()
    return rows


def main():
    url = os.environ["KENTOS_PG_URL"]
    port = int(os.environ["KENTOS_PG_PORT"])
    password = os.environ["PGPASSWORD"]
    problems = []

    doc = cad.Document.new("QGIS ile", srid=5256)
    linked = cad.sources.edit(doc, edit=LinkSourcesEditOp(path=url, table="sorgu.nokta", extent=AREA))
    layer = linked.link.layer
    by_name = {r.entity.attrs.get("ad"): r for r in doc.entities(layer=layer)}
    a, b, c = by_name["N1"], by_name["N2"], by_name["N3"]
    if linked.objects != 12:
        problems.append(f"alanla {linked.objects} nesne, 12 bekleniyordu")
    cad.entities.set(doc, uids=[a.uid], operation="attributes", attrs={"ad": "kentos-A"})
    cad.entities.set(doc, uids=[b.uid], operation="attributes", attrs={"ad": "kentos-B"})

    app = QgsApplication([], False)
    app.initQgis()
    uri = (f"dbname='postgres' host=127.0.0.1 port={port} user='postgres' password='{password}' "
           "key='fid' srid=5256 type=Point checkPrimaryKeyUnicity='1' table=\"sorgu\".\"nokta\" (geom)")
    qgis = QgsVectorLayer(uri, "nokta", "postgres")
    if not qgis.isValid():
        raise SystemExit("QGIS PostGIS katmanını açamadı")
    field = qgis.fields().indexOf("ad")
    qgis.startEditing()
    qgis.changeAttributeValue(1, field, "qgis-A")
    qgis.changeAttributeValue(3, field, "qgis-C")
    if not qgis.commitChanges():
        raise SystemExit(f"QGIS kaydedemedi: {qgis.commitErrors()}")
    if table(port, password) != {1: "qgis-A", 2: "N2", 3: "qgis-C"}:
        problems.append(f"QGIS'ten sonra tablo {table(port, password)}")

    try:
        cad.sources.sync(doc, layer=layer, direction="write")
        problems.append("aynı satır yazıldı, reddedilmeliydi")
    except CommandError as e:
        if e.code != "source_changed" or list(e.uids) != [a.uid]:
            problems.append(f"ret {e.code} {list(e.uids)}, source_changed [{a.uid}] bekleniyordu")
    if table(port, password) != {1: "qgis-A", 2: "N2", 3: "qgis-C"}:
        problems.append(f"retten sonra tablo {table(port, password)}")

    cad.entities.set(doc, uids=[a.uid], operation="attributes", attrs={"ad": "N1"})
    with warnings.catch_warnings(record=True) as said:
        warnings.simplefilter("always")
        written = cad.sources.sync(doc, layer=layer, direction="write")
    codes = [w.message.code for w in said if isinstance(w.message, CommandWarning)]
    if written.modified != 1 or "source_others" not in codes:
        problems.append(f"yazma {written.modified} satır, uyarılar {codes}; 1 ve source_others bekleniyordu")
    if table(port, password) != {1: "qgis-A", 2: "kentos-B", 3: "qgis-C"}:
        problems.append(f"yazmadan sonra tablo {table(port, password)}")

    read = cad.sources.sync(doc, layer=layer, direction="read")
    now = {r.uid: r.entity.attrs.get("ad") for r in doc.entities(layer=layer)}
    if read.modified != 2 or (now[a.uid], now[b.uid], now[c.uid]) != ("qgis-A", "kentos-B", "qgis-C"):
        problems.append(f"okuma {read.modified}, katman {now[a.uid]}, {now[b.uid]}, {now[c.uid]}")
    app.exitQgis()

    if problems:
        print("; ".join(problems), file=sys.stderr)
        return 1
    print("QGIS ile eşzamanlı düzenleme: ortak satır reddedildi, ayrı satırlar ikisinden de yazıldı, okuma QGIS'inkileri getirdi")
    return 0


if __name__ == "__main__":
    sys.exit(main())
