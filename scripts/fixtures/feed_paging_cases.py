#!/usr/bin/env python3
"""How a data service take ends, and what an OGC API service says it can do (docs/adr/0513), without KentOS code.

Writes fixtures/services/v1/paging.json:

- ``profiles``: conformance pages read into the parts KentOS asks of a service, by the class URI's family (after
  ``/spec/``) and class (after ``/conf/``), whatever the version; a filter is sent only to a service that declares
  Part 3's ``features-filter`` and CQL2's ``cql2-text`` or ``cql2-json`` (as JSON when it reads no text; docs/adr/0655 §4). The lists are written here as the OGC documents name
  the classes (OGC API - Features Parts 1–4, CQL2), in the shapes servers write them: final 1.0 URIs, draft versions,
  a page without Part 3, an unreadable page.
- ``takes``: a simulated service (its kind, WFS version, how many objects it has, the most it gives in one page,
  whether it says how many it matched, whether it gives OGC API ``next`` links) and the most objects asked; the rule
  of ADR 0513 played against it here: the requests, the objects taken and received, and how the take ended
  (``complete``, ``capped``, ``short``, ``pages``).

The application's test (crates/shared/services/tests/all/paging.rs) plays the same service against KentOS's client.

  python3 scripts/fixtures/feed_paging_cases.py            # writes the file
  python3 scripts/fixtures/feed_paging_cases.py --check    # compares
"""

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/services/v1/paging.json"

PAGE = 2000
MAX_PAGES = 1000

F1 = "http://www.opengis.net/spec/ogcapi-features-1/1.0/conf/"
F2 = "http://www.opengis.net/spec/ogcapi-features-2/1.0/conf/"
F3 = "http://www.opengis.net/spec/ogcapi-features-3/1.0/conf/"
F4 = "http://www.opengis.net/spec/ogcapi-features-4/1.0/conf/"
CQL = "http://www.opengis.net/spec/cql2/1.0/conf/"

PARTS = {
    ("ogcapi-features-1", "core"): "core",
    ("ogcapi-features-1", "geojson"): "geojson",
    ("ogcapi-features-2", "crs"): "crs",
    ("ogcapi-features-3", "filter"): "filter",
    ("ogcapi-features-3", "features-filter"): "featuresFilter",
    ("ogcapi-features-3", "queryables"): "queryables",
    ("cql2", "cql2-text"): "cql2Text",
    ("cql2", "cql2-json"): "cql2Json",
    ("cql2", "basic-cql2"): "basicCql2",
    ("cql2", "basic-spatial-functions"): "spatial",
    ("cql2", "basic-spatial-operators"): "spatial",
    ("cql2", "temporal-functions"): "temporal",
    ("ogcapi-features-4", "create-replace-delete"): "write",
}
FLAGS = ["core", "geojson", "crs", "filter", "featuresFilter", "queryables", "cql2Text", "cql2Json", "basicCql2",
         "spatial", "temporal", "write"]


def profile(page):
    """A conformance page (its JSON text, or None when it could not be read) as the parts declared."""
    out = {f: False for f in FLAGS}
    out["read"] = False
    if page is None:
        out["filters"] = False
        return out
    try:
        classes = json.loads(page)["conformsTo"]
        assert isinstance(classes, list)
    except (ValueError, KeyError, TypeError, AssertionError):
        out["filters"] = False
        return out
    out["read"] = True
    for uri in classes:
        if not isinstance(uri, str) or "/spec/" not in uri:
            continue
        rest = uri.strip().split("/spec/", 1)[1]
        if "/" not in rest:
            continue
        family, rest = rest.split("/", 1)
        if "/conf/" not in rest:
            continue
        cls = rest.split("/conf/", 1)[1].rstrip("/")
        part = PARTS.get((family, cls))
        if part:
            out[part] = True
    out["filters"] = out["featuresFilter"] and (out["cql2Text"] or out["cql2Json"])
    return out


def page(classes):
    return json.dumps({"links": [], "conformsTo": classes})


PROFILES = [
    ("Bölüm 1, 2, 3 ve CQL2 metni ile JSON (1.0)", page([
        F1 + "core", F1 + "oas30", F1 + "html", F1 + "geojson", F2 + "crs", F3 + "filter", F3 + "features-filter",
        F3 + "queryables", F3 + "queryables-query-parameters", CQL + "basic-cql2", CQL + "cql2-text",
        CQL + "cql2-json", CQL + "basic-spatial-functions", CQL + "temporal-functions"])),
    ("Yalnız Bölüm 1 ve 2", page([F1 + "core", F1 + "geojson", F1 + "html", F2 + "crs"])),
    ("Bölüm 3 taslak sürümüyle, CQL2 SNAPSHOT", page([
        F1 + "core",
        "http://www.opengis.net/spec/ogcapi-features-3/1.0.0-draft.1/conf/filter",
        "http://www.opengis.net/spec/ogcapi-features-3/1.0.0-draft.1/conf/features-filter",
        "http://www.opengis.net/spec/cql2/1.0.0-SNAPSHOT/conf/cql2-text",
        "http://www.opengis.net/spec/cql2/1.0.0-SNAPSHOT/conf/basic-spatial-operators"])),
    ("Bölüm 3 var, CQL2 metni yok (yalnız JSON)", page([F1 + "core", F3 + "filter", F3 + "features-filter",
                                                        CQL + "cql2-json"])),
    ("CQL2 metni var, nesne süzgeci yok", page([F1 + "core", F3 + "filter", CQL + "cql2-text"])),
    ("Bölüm 4 taslağı (yazma)", page([F1 + "core", F1 + "geojson", F4 + "create-replace-delete"])),
    ("Sonunda eğik çizgi ve boşluk", page([F1 + "core/", " " + F3 + "features-filter ", CQL + "cql2-text"])),
    ("Başka belgelerin sınıfları", page(["http://www.opengis.net/spec/ogcapi-common-1/1.0/conf/core",
                                         "http://www.opengis.net/spec/ogcapi-tiles-1/1.0/conf/core"])),
    ("Boş liste", page([])),
    ("Uygunluk sayfası değil", json.dumps({"links": [], "title": "Açılış"})),
    ("JSON değil", "<html>404</html>"),
    ("İstek başarısız", None),
]


def take(kind, total, most, *, cap=10**9, matched=True, version="2.0.0", next_links=True):
    """The rule of ADR 0513 played against a simulated service: the requests, the objects taken and received and the
    ending."""
    asked = min(PAGE, most)
    offset = taken = pages = 0
    known = None
    requests = 0
    while True:
        requests += 1
        # The service: WFS 1.x does not page (it gives from the start again), the others from the offset asked.
        start = 0 if (kind == "wfs" and not version.startswith("2")) else offset
        given = max(0, min(asked, cap, total - start))
        reports = matched and not (kind == "wfs" and not version.startswith("2"))
        if reports:
            known = total
        if kind == "wfs":
            paging = version.startswith("2")
            if known is not None:
                more = given > 0 and offset + given < known
            elif paging:
                more = given > 0
            else:
                more = given >= asked
        elif kind == "ogcFeatures":
            more = next_links and start + given < total
        else:  # arcgis: exceededTransferLimit
            more = start + given < total
        read = min(given, most - taken)
        cut = given > read
        taken += read
        offset += given
        pages += 1
        go_on = more and given > 0 and taken < most and pages < MAX_PAGES
        if not go_on:
            if taken >= most and (more or cut):
                ending = "capped"
            elif more and given > 0:
                ending = "pages"
            elif known is not None and known > offset:
                ending = "short"
            else:
                ending = "complete"
            return {"requests": requests, "taken": taken, "received": offset, "ending": ending,
                    "matched": known}


TAKES = [
    ("WFS 2.0, eşleşen sayıyla, tek sayfa", "wfs", 150, 50000, {}),
    ("WFS 2.0, eşleşen sayıyla, üç tam sayfa", "wfs", 6000, 50000, {}),
    ("WFS 2.0, sunucu sayfayı 500'de kesiyor, eşleşen sayıyla", "wfs", 1800, 50000, {"cap": 500}),
    ("WFS 2.0, sunucu 500'de kesiyor, eşleşen sayı yok: boş sayfaya dek", "wfs", 1800, 50000,
     {"cap": 500, "matched": False}),
    ("WFS 2.0, eşleşen sayı yok, kısa ilk sayfa", "wfs", 30, 50000, {"matched": False}),
    ("WFS 2.0, en çok nesneye ulaşıldı", "wfs", 6000, 2500, {}),
    ("WFS 2.0, en çok nesne tam eşleşene eşit", "wfs", 2500, 2500, {}),
    ("WFS 1.1, sayfalamaz: kısa sayfada durur", "wfs", 300, 50000, {"version": "1.1.0"}),
    ("WFS 1.1, sunucunun sınırı sayfadan küçük", "wfs", 5000, 50000, {"version": "1.1.0", "cap": 1000}),
    ("OGC API, next bağlantılarıyla", "ogcFeatures", 4500, 50000, {}),
    ("OGC API, sunucu 100'de kesiyor, next bağlantılarıyla", "ogcFeatures", 450, 50000, {"cap": 100}),
    ("OGC API, next yok ama eşleşen fazla: eksik", "ogcFeatures", 4500, 50000, {"next_links": False}),
    ("OGC API, next yok, eşleşen sayı yok", "ogcFeatures", 4500, 50000, {"next_links": False, "matched": False}),
    ("OGC API, en çok nesneye ulaşıldı", "ogcFeatures", 4500, 3000, {}),
    ("OGC API, boş koleksiyon", "ogcFeatures", 0, 50000, {}),
    ("ArcGIS, exceededTransferLimit ile üç sayfa", "arcgis", 5000, 50000, {"matched": False}),
    ("ArcGIS, sunucu 1000'de kesiyor", "arcgis", 2500, 50000, {"cap": 1000, "matched": False}),
    ("ArcGIS, en çok nesneye ulaşıldı", "arcgis", 5000, 1200, {"matched": False}),
    ("Sayfa sınırı: sunucu her sayfada bir nesne veriyor", "wfs", 1500, 50000, {"cap": 1}),
    ("Sayfa sınırı tam bitişte değil: 1000 nesne, birer birer", "wfs", 1000, 50000, {"cap": 1}),
]


def build():
    profiles = [{"name": n, "page": p, "profile": profile(p)} for n, p in PROFILES]
    takes = []
    for name, kind, total, most, opts in TAKES:
        service = {"kind": kind, "total": total, "cap": opts.get("cap"), "matched": opts.get("matched", True),
                   "version": opts.get("version", "2.0.0"), "nextLinks": opts.get("next_links", True)}
        takes.append({"name": name, "service": service, "most": most,
                      "expect": take(kind, total, most, **opts)})
    return {"format": "kentos.feed-paging", "version": 1, "page": PAGE, "maxPages": MAX_PAGES,
            "profiles": profiles, "takes": takes}


def main():
    data = build()
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil: betiği --check'siz çalıştırın.")
            return 1
    else:
        OUT.write_text(text)
    endings = {}
    for t in data["takes"]:
        endings[t["expect"]["ending"]] = endings.get(t["expect"]["ending"], 0) + 1
    print(f"Servis alma durumları tutarlı: {len(data['profiles'])} uygunluk sayfası, {len(data['takes'])} alma "
          f"({', '.join(f'{k} {v}' for k, v in sorted(endings.items()))})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
