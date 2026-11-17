#!/usr/bin/env python3
"""The profile's IDS (crates/shared/ifc/data/temel-2026.ids, docs/adr/8074 §5) read with Python's own XML parser,
without KentOS code: its structure is IDS 1.0's (buildingSMART's schema: info then specifications; a specification's
name and ifcVersion; applicability then requirements; the facets' order, cardinalities and data types), and its
requirements, read over the facts of KentOS's files (fixtures/ifc/v1/olgular/), find the very gaps KentOS's own rule
finds (the facts' ``gaps``, the form's 9th question): the same rows, the same objects.

This is KentOS's own reading of the file, not the independent checker the plan asks for (IFC-08: a second checker
opens it); that one awaits the owner's approval to fetch.

    python3 -I scripts/fixtures/ids_check.py
"""

from __future__ import annotations

import json
import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

KOK = Path(__file__).resolve().parents[2]
IDS = KOK / "crates/shared/ifc/data/temel-2026.ids"
OLGULAR = KOK / "fixtures/ifc/v1/olgular"
N = "{http://standards.buildingsmart.org/IDS}"
XS = "{http://www.w3.org/2001/XMLSchema}"

FACETS = ["entity", "partOf", "classification", "attribute", "property", "material"]
INFO = ["title", "copyright", "version", "description", "author", "date", "purpose", "milestone"]


def hata(mesaj: str) -> None:
    print(f"✗ {mesaj}", file=sys.stderr)
    sys.exit(1)


def yerel(e: ET.Element) -> str:
    return e.tag.replace(N, "")


def degerler(v: ET.Element | None) -> list[str] | None:
    """idsValue: a simple value or a restriction's enumeration; none for no value element."""
    if v is None:
        return None
    s = v.find(f"{N}simpleValue")
    if s is not None:
        return [s.text or ""]
    r = v.find(f"{XS}restriction")
    if r is None:
        hata(f"değer ne simpleValue ne xs:restriction: {ET.tostring(v)[:80]!r}")
    return [e.get("value") for e in r.findall(f"{XS}enumeration")]


def yapi(kok: ET.Element) -> list[dict]:
    if yerel(kok) != "ids":
        hata("kök ids değil")
    cocuklar = [yerel(c) for c in kok]
    if cocuklar != ["info", "specifications"]:
        hata(f"ids'in çocukları {cocuklar}")
    info = [yerel(c) for c in kok[0]]
    if not info or info[0] != "title" or [INFO.index(i) for i in info] != sorted(INFO.index(i) for i in info):
        hata(f"info'nun sırası {info}")
    specs = []
    for s in kok[1]:
        if yerel(s) != "specification" or not s.get("name") or s.get("ifcVersion") != "IFC4X3_ADD2":
            hata(f"specification: {s.attrib}")
        parca = [yerel(c) for c in s]
        if parca != ["applicability", "requirements"]:
            hata(f"{s.get('name')}: {parca}")
        app, req = s[0], s[1]
        if app.get("minOccurs") not in ("0", "1") or app.get("maxOccurs") != "unbounded":
            hata(f"{s.get('name')}: applicability {app.attrib}")
        siniflar = degerler(app.find(f"{N}entity/{N}name"))
        sira = [FACETS.index(yerel(f)) for f in req]
        if sira != sorted(sira):
            hata(f"{s.get('name')}: yönlerin sırası {[yerel(f) for f in req]}")
        istek = []
        for f in req:
            k = f.get("cardinality")
            if k not in ("required", "optional", "prohibited") and yerel(f) != "entity":
                hata(f"{s.get('name')}: cardinality {k}")
            d = f.get("dataType")
            if d is not None and not re.fullmatch(r"[A-Z]+", d):
                hata(f"{s.get('name')}: dataType {d}")
            istek.append(f)
        specs.append({"id": s.get("identifier"), "classes": siniflar, "requirements": istek})
    return specs


def tasir(o: dict, f: ET.Element) -> bool | None:
    """Whether an object of the facts meets a requirement; none where the facts say nothing of it (KentOS's rule does
    not judge it either: a relation, an attribute the facts do not carry)."""
    t = yerel(f)
    if t == "attribute":
        ad = degerler(f.find(f"{N}name"))[0]
        alan = {"Name": "name", "LongName": "longName", "Description": "description", "ObjectType": "objectType",
                "PredefinedType": "predefinedType", "Tag": "tag", "AxisTag": "tag", "RefElevation": "elevation"}
        if ad not in alan:
            return None
        v = o.get(alan[ad])
        if v is None or (isinstance(v, str) and not v.strip()):
            return False
        izin = degerler(f.find(f"{N}value"))
        return izin is None or v in izin
    if t == "property":
        kume = degerler(f.find(f"{N}propertySet"))[0]
        ad = degerler(f.find(f"{N}baseName"))[0]
        for alan in ("sets", "typeSets", "quantities", "typeQuantities"):
            v = (o.get(alan) or {}).get(kume, {}).get(ad)
            if v is not None and not (isinstance(v, str) and not v.strip()):
                return True
        return False
    if t == "material":
        return any(m and m.strip() for m in o.get("materials") or [])
    return None


def karsilastir(specs: list[dict], ad: str) -> None:
    f = json.loads((OLGULAR / f"{ad}.json").read_text(encoding="utf-8"))
    # EK-6 Tablo 6.7's building heights, asked only where the height is constant (docs/adr/8076 §4): IDS 1.0 cannot
    # say so; where the file's storeys carry differing heights the IDS's failure is set apart, as KentOS's rule does.
    def farkli(ad: str) -> bool:
        h = [((o.get("quantities") or {}).get("Qto_BuildingStoreyBaseQuantities") or {}).get(ad)
             for o in f["objects"] if o["class"] == "IfcBuildingStorey"]
        h = [x for x in h if isinstance(x, (int, float))]
        return any(abs(x - h[0]) > 1e-9 for x in h)
    kosullu = {"Height": "GrossHeight", "EavesHeight": "NetHeight"}
    bulunan: dict[tuple, list[str]] = {}
    for s in specs:
        siniflar = {c.lower() for c in s["classes"] or []}
        nesneler = [o for o in f["objects"] if o["class"].lower() in siniflar]
        if not nesneler:
            continue
        for r in s["requirements"]:
            if yerel(r) == "property" and degerler(r.find(f"{N}propertySet"))[0] == "Qto_BuildingBaseQuantities":
                miktar = degerler(r.find(f"{N}baseName"))[0]
                if miktar in kosullu and farkli(kosullu[miktar]):
                    continue
            sonuc = [(o, tasir(o, r)) for o in nesneler]
            if all(x is None for _, x in sonuc):
                continue
            eksik = [o["globalId"] for o, x in sonuc if x is False]
            if eksik:
                bulunan[(s["id"], r.get("instructions"))] = eksik
    kentos = {}
    for g in f["gaps"]:
        tablo = f"T{g['table']}"
        kentos[(tablo, g["path"])] = sorted(g["missing"])
    ids_eksik = {}
    for (tablo, ne), eksik in bulunan.items():
        ids_eksik[(tablo, ne)] = sorted(eksik)
    # The two lists are keyed differently (the IDS by its row's words, KentOS by its path): compare by table and objects.
    a = sorted((t, tuple(e)) for (t, _), e in ids_eksik.items())
    b = sorted((t, tuple(e)) for (t, _), e in kentos.items())
    if a != b:
        yalniz_ids = [x for x in a if x not in b]
        yalniz_kentos = [x for x in b if x not in a]
        hata(f"{ad}: IDS ile KentOS'un kuralı ayrışıyor; yalnız IDS {yalniz_ids[:3]}, yalnız KentOS {yalniz_kentos[:3]}")
    print(f"✓ {ad}: IDS'in {len(a)} eksiği KentOS'un kuralınınkiyle aynı (satır ve nesne)")


def main() -> None:
    kok = ET.parse(IDS).getroot()
    specs = yapi(kok)
    print(f"✓ {IDS.relative_to(KOK)}: IDS 1.0 yapısı, {len(specs)} tanım, "
          f"{sum(len(s['requirements']) for s in specs)} gereklilik")
    for ad in ("kesit-binasi", "kesit-binasi-kunye", "kesit-binasi-tam", "yerel-sistem", "kesit-binasi-kapi-eksik"):
        karsilastir(specs, ad)


if __name__ == "__main__":
    main()
