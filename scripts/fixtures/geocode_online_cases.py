#!/usr/bin/env python3
"""Adres arama's online providers' shared cases (docs/adr/0229 §10), written from the ADR without KentOS code.

The ADR's rules are written here again: each kind's request (its parameters in order, values percent-encoded as the
map services' are: RFC 3986's unreserved characters and `,` `:` `/` `*` kept; coordinates with 7 decimals and no
trailing zeros), each kind's answer read into candidates (place, text, street, door, area names, neighbourhood,
district, province, postcode, name, kind, the provider's score), an online candidate's KentOS score (its street read as
a street column is, compared with the query's; the best area name for the neighbourhood; §7's kind and door penalties;
a place name or an area otherwise), a batch's pick and unique addresses, and a batch row's one line.

The answers are fixtures/geocode/v1/answers (real ones from OpenStreetMap's servers, ODbL, and ones written by the
providers' documents). The cases are written to fixtures/geocode/v1/online.json; `--check` says whether the file is
what this script writes. An answer's error is compared by its being one, not by its words.
"""

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import geocode_cases as local  # noqa: E402  (the local rules, KentOS-free too)

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures" / "geocode" / "v1"
OUT = DIR / "online.json"
TABLE = json.loads((DIR / "providers.json").read_text("utf-8"))


def encode(text):
    out = []
    for b in text.encode("utf-8"):
        c = chr(b)
        if (c.isascii() and c.isalnum()) or c in "-._~,:/*":
            out.append(c)
        else:
            out.append("%%%02X" % b)
    return "".join(out)


def coord(x):
    s = "%.7f" % x
    s = s.rstrip("0").rstrip(".")
    return "0" if s == "-0" else s


def query(base, params):
    return base + "".join(("?" if i == 0 else "&") + k + "=" + encode(v) for i, (k, v) in enumerate(params))


def provider(pid, own):
    """A provider of the table with a device's setting of it, or none (§10, §16): a ready provider without a setting
    asks for TR; with one, for the setting's country."""
    preset = next((p for p in TABLE["presets"] if p["id"] == pid), None)
    if preset is not None:
        country = "TR" if own is None else own.get("country")
        return {"kind": preset["kind"], "url": preset["url"], "keyParam": preset.get("keyParam"), "country": country,
                "storing": bool(own and own.get("store"))}
    kind = next(k for k in TABLE["servers"]["kinds"] if k["kind"] == own["kind"])
    return {"kind": own["kind"], "url": own["url"].rstrip("/"), "keyParam": kind.get("keyParam"),
            "country": own.get("country"), "storing": False}


def forward(p, text, limit, bias):
    text = text.strip()
    k, u = p["kind"], p["url"]
    if k == "nominatim":
        params = [("q", text), ("format", "jsonv2"), ("addressdetails", "1"), ("limit", str(limit))]
        if p["country"]:
            params.append(("countrycodes", p["country"].lower()))
        if bias:
            params.append(("viewbox", ",".join(coord(bias[x]) for x in ("west", "south", "east", "north"))))
        params.append(("accept-language", "tr"))
        base = u + "/search"
    elif k == "photon":
        params = [("q", text), ("limit", str(limit))]
        if bias:
            params += [("lat", coord(bias["lat"])), ("lon", coord(bias["lon"]))]
        base = u + "/api"
    elif k == "pelias":
        params = [("text", text), ("size", str(limit))]
        if bias:
            params += [("focus.point.lat", coord(bias["lat"])), ("focus.point.lon", coord(bias["lon"]))]
        if p["country"]:
            params.append(("boundary.country", p["country"]))
        base = u + "/v1/search"
    elif k == "arcgis":
        params = [("SingleLine", text), ("f", "json"), ("outSR", "4326"), ("outFields", "*"), ("maxLocations", str(limit))]
        if p["country"]:
            params.append(("sourceCountry", p["country"]))
        if bias:
            params.append(("location", coord(bias["lon"]) + "," + coord(bias["lat"])))
        params += [("langCode", "TR"), ("forStorage", "true" if p["storing"] else "false")]
        base = u + "/findAddressCandidates"
    else:
        params = [("address", text), ("language", "tr")]
        if p["country"]:
            params.append(("region", p["country"].lower()))
        base = u
    return {"url": query(base, params), "keyParam": p["keyParam"]}


def reverse(p, lon, lat):
    x, y = coord(lon), coord(lat)
    k, u = p["kind"], p["url"]
    if k == "nominatim":
        base, params = u + "/reverse", [("lat", y), ("lon", x), ("format", "jsonv2"), ("addressdetails", "1"), ("zoom", "18"), ("accept-language", "tr")]
    elif k == "photon":
        base, params = u + "/reverse", [("lat", y), ("lon", x), ("limit", "1")]
    elif k == "pelias":
        base, params = u + "/v1/reverse", [("point.lat", y), ("point.lon", x), ("size", "1")]
    elif k == "arcgis":
        base, params = u + "/reverseGeocode", [("location", x + "," + y), ("f", "json"), ("outSR", "4326"), ("langCode", "TR"), ("forStorage", "true" if p["storing"] else "false")]
    else:
        base, params = u, [("latlng", y + "," + x), ("language", "tr")]
    return {"url": query(base, params), "keyParam": p["keyParam"]}


# ── Answers read ────────────────────────────────────────────────────────


def s(v, key):
    x = v.get(key) if isinstance(v, dict) else None
    if isinstance(x, str):
        return x.strip()
    if isinstance(x, bool) or x is None:
        return ""
    if isinstance(x, (int, float)):
        return json.dumps(x)
    return ""


def num(x):
    if isinstance(x, bool):
        return None
    if isinstance(x, (int, float)):
        return float(x)
    if isinstance(x, str):
        try:
            v = float(x.strip())
        except ValueError:
            return None
        return v if v == v and abs(v) != float("inf") else None
    return None


def first(v, keys):
    return next((s(v, k) for k in keys if s(v, k)), "")


def every(v, keys):
    out = []
    for k in keys:
        t = s(v, k)
        if t and t not in out:
            out.append(t)
    return out


def hood(v, keys):
    values = [s(v, k) for k in keys if s(v, k)]
    return next((t for t in values if local.tagged_hood(t)), values[0] if values else "")


def place(lon, lat):
    if lon is None or lat is None or not (-180 <= lon <= 180 and -90 <= lat <= 90):
        return None
    return lon, lat


def cand(lon, lat, **f):
    out = {"lon": lon, "lat": lat, "label": "", "street": "", "door": "", "areas": [], "neighbourhood": "", "district": "",
           "province": "", "postcode": "", "name": "", "kind": "", "providerScore": None}
    out.update(f)
    return out


NOM_STREETS = ["road", "pedestrian", "footway", "cycleway", "path", "square"]
NOM_HOODS = ["suburb", "neighbourhood", "quarter", "village", "hamlet"]
NOM_DISTRICTS = ["town", "city_district", "district", "county", "municipality"]
NOM_PROVINCES = ["province", "state", "city"]
NOM_AREAS = NOM_HOODS + ["town", "city_district", "district", "county", "municipality", "city", "province", "state"]


def nominatim(v):
    pl = place(num(v.get("lon")), num(v.get("lat")))
    if pl is None:
        return None
    a = v.get("address") or {}
    return cand(*pl, label=s(v, "display_name"), street=first(a, NOM_STREETS), door=s(a, "house_number"),
                areas=every(a, NOM_AREAS), neighbourhood=hood(a, NOM_HOODS), district=first(a, NOM_DISTRICTS),
                province=first(a, NOM_PROVINCES), postcode=s(a, "postcode"), name=s(v, "name"),
                kind=first(v, ["addresstype", "type"]), providerScore=num(v.get("importance")))


def point_of(f):
    c = ((f.get("geometry") or {}).get("coordinates")) or []
    return place(num(c[0]) if len(c) > 0 else None, num(c[1]) if len(c) > 1 else None)


def photon(f):
    pl = point_of(f)
    if pl is None:
        return None
    p = f.get("properties") or {}
    kind = s(p, "type")
    street = s(p, "street") or (s(p, "name") if kind == "street" else "")
    state, city = s(p, "state"), s(p, "city")
    district = city if city and city != state else s(p, "county")
    label = []
    for t in [s(p, "name"), (street + " " + s(p, "housenumber")).strip(), s(p, "district"), s(p, "city"), s(p, "state"), s(p, "postcode")]:
        if t and t not in label:
            label.append(t)
    return cand(*pl, label=", ".join(label), street=street, door=s(p, "housenumber"),
                areas=every(p, ["district", "locality", "city", "county", "state"]), neighbourhood=hood(p, ["district", "locality"]),
                district=district, province=state, postcode=s(p, "postcode"), name=s(p, "name"), kind=kind)


def pelias(f):
    pl = point_of(f)
    if pl is None:
        return None
    p = f.get("properties") or {}
    return cand(*pl, label=s(p, "label"), street=s(p, "street"), door=s(p, "housenumber"),
                areas=every(p, ["neighbourhood", "borough", "locality", "localadmin", "county", "region"]),
                neighbourhood=hood(p, ["neighbourhood", "borough"]), district=first(p, ["county", "localadmin", "locality"]),
                province=s(p, "region"), postcode=s(p, "postalcode"), name=s(p, "name"), kind=s(p, "layer"),
                providerScore=num(p.get("confidence")))


def arcgis(c):
    loc = c.get("location")
    if not isinstance(loc, dict):
        return None
    pl = place(num(loc.get("x")), num(loc.get("y")))
    if pl is None:
        return None
    a = c.get("attributes") or {}
    street = " ".join(t for t in (s(a, "StPreType"), s(a, "StName"), s(a, "StType")) if t)
    return cand(*pl, label=first(a, ["LongLabel", "Match_addr"]) or s(c, "address"), street=street, door=s(a, "AddNum"),
                areas=every(a, ["Nbrhd", "District", "City", "Subregion", "Region"]), neighbourhood=hood(a, ["Nbrhd"]),
                district=first(a, ["City", "District", "Subregion"]), province=s(a, "Region"), postcode=s(a, "Postal"),
                name=s(a, "PlaceName"), kind=s(a, "Addr_type"), providerScore=num(c.get("score")))


def arcgis_reverse(v):
    loc = v.get("location")
    if not isinstance(loc, dict):
        return None
    pl = place(num(loc.get("x")), num(loc.get("y")))
    if pl is None:
        return None
    a = v.get("address") or {}
    door, address = s(a, "AddNum"), s(a, "Address")
    words = address.split()
    if door and words and words[0] == door:
        words = words[1:]
    elif door and words and words[-1] == door:
        words = words[:-1]
    street = " ".join(words) if door else address
    return cand(*pl, label=first(a, ["LongLabel", "Match_addr"]), street=street, door=door,
                areas=every(a, ["Neighborhood", "District", "City", "Subregion", "Region"]), neighbourhood=hood(a, ["Neighborhood"]),
                district=first(a, ["City", "District", "Subregion"]), province=s(a, "Region"), postcode=s(a, "Postal"),
                name=s(a, "PlaceName"), kind=s(a, "Addr_type"))


def component(r, kind):
    for c in r.get("address_components") or []:
        if kind in (c.get("types") or []):
            return s(c, "long_name")
    return ""


def google(r):
    loc = (r.get("geometry") or {}).get("location") or {}
    pl = place(num(loc.get("lng")), num(loc.get("lat")))
    if pl is None:
        return None
    hoods = [t for t in (component(r, k) for k in ("neighborhood", "sublocality_level_1", "administrative_area_level_4")) if t]
    areas = []
    for k in ("neighborhood", "sublocality_level_2", "sublocality_level_1", "sublocality", "administrative_area_level_4",
              "administrative_area_level_3", "locality", "administrative_area_level_2", "administrative_area_level_1"):
        t = component(r, k)
        if t and t not in areas:
            areas.append(t)
    types = r.get("types") or []
    return cand(*pl, label=s(r, "formatted_address"), street=component(r, "route"), door=component(r, "street_number"),
                areas=areas, neighbourhood=next((t for t in hoods if local.tagged_hood(t)), hoods[0] if hoods else ""),
                district=component(r, "administrative_area_level_2"), province=component(r, "administrative_area_level_1"),
                postcode=component(r, "postal_code"), kind=types[0] if types and isinstance(types[0], str) else "")


def failed(kind, v):
    if kind == "arcgis":
        return isinstance(v, dict) and "error" in v
    if kind == "google":
        st = s(v, "status")
        return bool(st) and st not in ("OK", "ZERO_RESULTS")
    return isinstance(v, dict) and "error" in v


def read_forward(kind, v):
    if failed(kind, v):
        return {"error": True}
    if kind == "nominatim":
        items, fn = (v if isinstance(v, list) else []), nominatim
    elif kind in ("photon", "pelias"):
        items, fn = (v.get("features") or []), (photon if kind == "photon" else pelias)
    elif kind == "arcgis":
        items, fn = (v.get("candidates") or []), arcgis
    else:
        items, fn = (v.get("results") or []), google
    out, skipped = [], 0
    for item in items[:50]:
        c = fn(item)
        if c is None:
            skipped += 1
        else:
            out.append(c)
    return {"candidates": out, "skipped": skipped}


def read_reverse(kind, v):
    if failed(kind, v):
        if kind == "nominatim" and s(v, "error") == "Unable to geocode":
            return {"candidate": None}
        return {"error": True}
    if kind == "nominatim":
        return {"candidate": nominatim(v)}
    if kind == "arcgis":
        return {"candidate": arcgis_reverse(v)}
    r = read_forward(kind, v)
    return {"candidate": r["candidates"][0] if r["candidates"] else None}


# ── KentOS's score of an online candidate ───────────────────────────────


def score(q, c):
    sv = local.street_value(c["street"])
    hood_asked = q["neighbourhood"]
    b = None
    if q["street"] and sv:
        b = local.similar(q["street"], sv[0])
        words = q["street"].split(" ")
        if b is None and len(words) >= 2:
            for k in range(1, len(words)):
                bb = local.similar(" ".join(words[k:]), sv[0])
                if bb is not None:
                    if hood_asked is None:
                        hood_asked = " ".join(words[:k])
                    b = bb
                    break
    areas = [a for a in (local.area_value(x) for x in c["areas"]) if a]

    def best_area(name):
        found = [m for m in (local.similar(name, a) for a in areas) if m is not None]
        return max(found) if found else None

    if b is not None and sv:
        pen = 100 - b
        if q["streetKind"] and sv[1] and q["streetKind"] != sv[1]:
            pen += 5
        if hood_asked and areas:
            m = best_area(hood_asked)
            pen += (100 - m) if m is not None else 30
        own = local.door_value(c["door"])
        asked = q["door"]
        if asked is not None and own is not None and asked == own:
            extra, kind = 0, "Kapı"
        elif asked is not None and own is not None and local.door_number(asked) is not None and local.door_number(asked) == local.door_number(own):
            extra, kind = 5, "Kapı"
        elif asked is not None:
            extra, kind = 25, "Yol"
        else:
            extra, kind = 15, "Yol"
        return max(0, 100 - pen - extra), kind
    if q["untagged"] and q["street"] and c["name"]:
        bb = local.similar(q["street"], local.words_key(c["name"]))
        if bb is not None:
            return max(0, bb - 15), "Yer adı"
    h = q["street"] if q["untagged"] else hood_asked
    if h:
        m = best_area(h)
        if m is not None:
            return max(0, m - 40), "Bölge"
    return 0, "Diğer"


def pick(q, cands, min_score):
    if not cands:
        return None
    scored = [score(q, c) for c in cands]
    best = max(range(len(cands)), key=lambda i: (scored[i][0], -i))
    sc, kind = scored[best]
    if sc < min_score:
        status = "Eşleşmedi"
    elif any(i != best and scored[i][0] == sc and (cands[i]["lon"], cands[i]["lat"]) != (cands[best]["lon"], cands[best]["lat"]) for i in range(len(cands))):
        status = "Belirsiz"
    else:
        status = "Eşleşti"
    return {"index": best, "score": sc, "kind": kind, "status": status}


def batch_key(text):
    return " ".join(local.fold(text).split())


def line_of(hood_, street, door, district, province, postcode):
    parts = []
    head = (street.strip() + " " + door.strip()).strip()
    if head:
        parts.append(head)
    n = hood_.strip()
    if n:
        parts.append(n if local.tagged_hood(n) else n + " Mahallesi")
    parts += [x.strip() for x in (district, province, postcode) if x.strip()]
    return ", ".join(parts)


# ── The cases ───────────────────────────────────────────────────────────

BIAS = {"lon": 32.8541, "lat": 39.9208, "west": 32.84, "south": 39.91, "east": 32.87, "north": 39.93}
SETTINGS = [
    ("nominatim-osm", None),
    ("nominatim-osm", {"id": "nominatim-osm", "enabled": True, "accepted": 1}),
    ("photon-komoot", None),
    ("arcgis-world", None),
    ("arcgis-world", {"id": "arcgis-world", "enabled": True, "accepted": 1, "country": "TR", "store": True}),
    ("google", None),
    ("sunucu-1", {"id": "sunucu-1", "kind": "pelias", "name": "Pelias", "url": "https://pelias.ornek.bel.tr/", "country": "TR"}),
    ("sunucu-2", {"id": "sunucu-2", "kind": "nominatim", "name": "Yerel", "url": "http://10.0.0.5:8080"}),
    ("sunucu-3", {"id": "sunucu-3", "kind": "arcgis", "name": "Kurum", "url": "https://cbs.ornek.bel.tr/arcgis/rest/services/Adres/GeocodeServer"}),
    ("sunucu-4", {"id": "sunucu-4", "kind": "photon", "name": "Photon", "url": "https://photon.ornek.bel.tr"}),
]
TEXTS = ["Atatürk Bulvarı 15, Çankaya, Ankara", "  İnönü Cd. No:3/A *ç&a=1  "]
PLACES = [(32.8541, 39.9208), (-0.00000001, 41.0000000449)]

ANSWERS = [
    ("nominatim", "forward", "nominatim-search.json"),
    ("nominatim", "reverse", "nominatim-reverse.json"),
    ("nominatim", "reverse", "nominatim-error.json"),
    ("photon", "forward", "photon-search.json"),
    ("photon", "reverse", "photon-reverse.json"),
    ("arcgis", "forward", "arcgis-candidates.json"),
    ("arcgis", "reverse", "arcgis-reverse.json"),
    ("arcgis", "forward", "arcgis-error.json"),
    ("google", "forward", "google-geocode.json"),
    ("google", "forward", "google-denied.json"),
    ("pelias", "forward", "pelias-search.json"),
]

SCORES = [
    ("Atatürk Bulvarı 15, Çankaya, Ankara", "nominatim", "nominatim-search.json"),
    ("Kızılay Mah. Atatürk Blv. No:15", "photon", "photon-search.json"),
    ("Atatürk Bulvarı No:15", "arcgis", "arcgis-candidates.json"),
    ("Atatürk Cad. No:15/B", "google", "google-geocode.json"),
    ("Kızılay Atatürk Bulvarı 15", "pelias", "pelias-search.json"),
    ("Ankara", "pelias", "pelias-search.json"),
    ("15 Temmuz Kızılay Millî İrade", "photon", "photon-search.json"),
    ("Remzi Oğuz Arık", "nominatim", "nominatim-search.json"),
]

BATCHES = [["Atatürk Blv. 12", "ATATÜRK  blv. 12 ", "", "İnönü Cd. 3", "  ", "atatürk blv. 12"]]

LINES = [
    ("Kızılay", "Atatürk Bulvarı", "15", "Çankaya", "Ankara", ""),
    ("Kızılay Mahallesi", "", "", "", "", "06420"),
    ("", "1203/1 Sokak", "No: 5", "", "İzmir", ""),
    ("", "", "", "", "", ""),
]


def build_cases():
    out = {"format": "kentos.geocode-online-cases", "version": 1,
           "note": "docs/adr/0229 §10; written by scripts/fixtures/geocode_online_cases.py from the ADR, without KentOS code. Answers: fixtures/geocode/v1/answers (OpenStreetMap data © OpenStreetMap contributors, ODbL)",
           "requests": [], "answers": [], "scores": [], "batches": [], "lines": []}
    for pid, own in SETTINGS:
        p = provider(pid, own)
        for text in TEXTS:
            for limit, bias in ((10, BIAS), (5, None)):
                out["requests"].append({"id": pid, "own": own, "forward": {"text": text, "limit": limit, "bias": bias}, **forward(p, text, limit, bias)})
        for lon, lat in PLACES:
            out["requests"].append({"id": pid, "own": own, "reverse": {"lon": lon, "lat": lat}, **reverse(p, lon, lat)})
    for kind, direction, name in ANSWERS:
        v = json.loads((DIR / "answers" / name).read_text("utf-8"))
        r = read_forward(kind, v) if direction == "forward" else read_reverse(kind, v)
        out["answers"].append({"kind": kind, "direction": direction, "file": name, **r})
    for text, kind, name in SCORES:
        v = json.loads((DIR / "answers" / name).read_text("utf-8"))
        cands = read_forward(kind, v)["candidates"]
        q = local.parse(text)
        out["scores"].append({"text": text, "kind": kind, "file": name,
                              "scores": [list(score(q, c)) for c in cands], "pick": pick(q, cands, 85)})
    for rows in BATCHES:
        keys, at, seen = [], [], {}
        for r in rows:
            k = batch_key(r)
            if not k:
                at.append(None)
                continue
            if k not in seen:
                seen[k] = len(keys)
                keys.append(k)
            at.append(seen[k])
        out["batches"].append({"rows": rows, "keys": keys, "at": at})
    for cols in LINES:
        out["lines"].append({"columns": list(cols), "line": line_of(*cols)})
    return out


def main():
    text = json.dumps(build_cases(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} betiğin yazdığıyla aynı değil (--check olmadan çalıştırın ve farkı okuyun)", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)} tutarlı.")
        return 0
    OUT.write_text(text, "utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
