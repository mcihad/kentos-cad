#!/usr/bin/env python3
"""Independent reference for STAC discovery and preparation (docs/adr/0528).

Written from the ADR and the STAC specification (item, asset, eo, raster and projection extensions; STAC API item
search) without any KentOS code. It reads the recorded STAC answers in fixtures/services/v1/stac (three of them
are real Earth Search answers of 2026-10-19, trimmed; the `synthetic-*` ones are written by this script) and
writes:

  fixtures/services/v1/stac/synthetic-*.json, index.json   the recorded transport (request -> file or error)
  fixtures/stac/v1/cases.json                              the services core's cases (endpoints, datetimes,
                                                           addresses, requests, pages)
  fixtures/commands/v1/cad.stac.search.json                the shared command cases of cad.stac.search
  fixtures/commands/v1/cad.stac.prepare.json               the shared command cases of cad.stac.prepare

`--check` rebuilds everything and compares it with the files; it never writes.
"""
import copy
import datetime as dt
import json
import re
import sys
from decimal import Decimal
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from stac_access_cases import blob_parts, s3_url  # noqa: E402  (the access rules, docs/adr/0644 §1)

ROOT = Path(__file__).resolve().parents[2]
REC = ROOT / "fixtures/services/v1/stac"
EP = "https://earth-search.aws.element84.com/v1"

# ---------------------------------------------------------------------------------------------------------------
# The rules (ADR 0528 §3-§6).

def canonical(v):
    """Compact JSON, keys in alphabetical order, integral numbers without a point."""
    if isinstance(v, dict):
        return "{" + ",".join(json.dumps(k, ensure_ascii=False) + ":" + canonical(v[k]) for k in sorted(v)) + "}"
    if isinstance(v, list):
        return "[" + ",".join(canonical(x) for x in v) + "]"
    if isinstance(v, bool) or v is None or isinstance(v, str):
        return json.dumps(v, ensure_ascii=False)
    if isinstance(v, float) and v == int(v) and abs(v) < 1e15:
        return str(int(v))
    return json.dumps(v)


def normalize_endpoint(text):
    t = text.strip()
    if not t or any(c.isspace() or ord(c) < 32 for c in t):
        return None
    m = re.match(r"^([A-Za-z]+)://(.*)$", t)
    if not m or m.group(1).lower() not in ("http", "https"):
        return None
    scheme, rest = m.group(1).lower(), m.group(2)
    if "?" in rest or "#" in rest:
        return None
    slash = rest.find("/")
    authority, path = (rest, "") if slash < 0 else (rest[:slash], rest[slash:])
    if "@" in authority or not authority or authority.startswith(":"):
        return None
    return f"{scheme}://{authority.lower()}{path.rstrip('/')}"


def origin(url):
    m = re.match(r"^([A-Za-z]+)://([^/?#]*)", url.strip())
    if not m or not m.group(2) or "@" in m.group(2):
        return None
    return f"{m.group(1).lower()}://{m.group(2).lower()}"


def resolve(endpoint, href):
    h = href.strip()
    if "://" in h:
        return h
    if h.startswith("/"):
        o = origin(endpoint)
        return None if o is None else f"{o}/{h[1:]}"
    return f"{endpoint.rstrip('/')}/{h}" if h else None


DATE = re.compile(r"^(\d{4})-(\d{2})-(\d{2})$")
INSTANT = re.compile(r"^(\d{4})-(\d{2})-(\d{2})[Tt](\d{2}):(\d{2}):(\d{2})(?:\.(\d{1,9}))?(Z|z|[+-]\d{2}:\d{2})$")


def real_date(y, m, d):
    try:
        return dt.date(int(y), int(m), int(d))
    except ValueError:
        return None


def parse_instant(text):
    """(normalized text, sortable key) or None."""
    m = INSTANT.match(text)
    if not m:
        return None
    y, mo, d, h, mi, s, frac, off = m.groups()
    if real_date(y, mo, d) is None or int(h) > 23 or int(mi) > 59 or int(s) > 60:
        return None
    if off in ("Z", "z"):
        delta = 0
    else:
        oh, om = int(off[1:3]), int(off[4:6])
        if oh > 23 or om > 59:
            return None
        delta = (oh * 3600 + om * 60) * (-1 if off[0] == "-" else 1)
    base = dt.datetime(int(y), int(mo), int(d), int(h), int(mi), 0) + dt.timedelta(seconds=int(s) - delta)
    key = (base.replace(tzinfo=dt.timezone.utc).timestamp(), (frac or "").ljust(9, "0"))
    normal = text[:10] + "T" + text[11:]
    if normal.endswith("z"):
        normal = normal[:-1] + "Z"
    return normal, (int(base.replace(tzinfo=dt.timezone.utc).timestamp()), key[1])


def end_of(text, is_end):
    t = text.strip()
    if t in ("", ".."):
        return "open"
    if DATE.match(t):
        if real_date(*DATE.match(t).groups()) is None:
            return None
        t = t + ("T23:59:59Z" if is_end else "T00:00:00Z")
    return parse_instant(t)


def normalize_datetime(text):
    t = text.strip()
    if not t:
        return None
    if "/" not in t:
        if DATE.match(t):
            return normalize_datetime(f"{t}/{t}")
        r = end_of(t, False)
        return None if r in (None, "open") else r[0]
    a, _, b = t.partition("/")
    if "/" in b:
        return None
    s, e = end_of(a, False), end_of(b, True)
    if s is None or e is None:
        return None
    if s == "open" and e == "open":
        return None
    if s == "open":
        return f"../{e[0]}"
    if e == "open":
        return f"{s[0]}/.."
    return None if s[1] > e[1] else f"{s[0]}/{e[0]}"


PROOF = {"sig", "signature", "x-amz-signature", "x-amz-credential", "x-amz-security-token", "awsaccesskeyid",
         "x-goog-signature", "x-goog-credential", "token", "sas_token", "access_token", "api_key", "apikey",
         "key-pair-id", "policy"}


def stable_href(href):
    """(address without a signature or a login, whether one was dropped)."""
    s = href.strip()
    signed = False
    head = ""
    m = re.match(r"^([^:/?#]+)://([^/?#]*)(.*)$", s, re.S)
    if m:
        scheme, authority, rest = m.groups()
        if "@" in authority:
            signed = True
            authority = authority.rsplit("@", 1)[1]
        head = f"{scheme}://{authority}"
        s = rest
    path, q, query = s.partition("?")
    fragment = None
    if q:
        query, hash_, fragment = query.partition("#")
        fragment = fragment if hash_ else None
    else:
        query = None
        path, hash_, fragment = path.partition("#")
        fragment = fragment if hash_ else None
    path = path.split("#")[0]
    proof = query is not None and any(p.split("=")[0].lower() in PROOF for p in query.split("&"))
    out = head + path
    if proof:
        signed = True
    elif query:
        out += "?" + query
    if fragment:
        out += "#" + fragment
    return out, signed


def is_http(href):
    low = href.strip().lower()
    return (low.startswith("http://") and len(low) > 7) or (low.startswith("https://") and len(low) > 8)


def text_of(v, key):
    x = v.get(key)
    if isinstance(x, str) and x.strip():
        return x.strip()
    return None


def num_of(v, key):
    x = v.get(key)
    if isinstance(x, (int, float)) and not isinstance(x, bool):
        return float(x)
    return None


def texts_of(v, key):
    x = v.get(key)
    return [s for s in x if isinstance(s, str)] if isinstance(x, list) else []


def numbers(v, n):
    if not isinstance(v, list) or len(v) < n:
        return None
    out = []
    for x in v[:n]:
        if not isinstance(x, (int, float)) or isinstance(x, bool):
            return None
        out.append(float(x))
    return out


def code_epsg(code):
    auth, sep, n = code.strip().partition(":")
    if not sep or auth.lower() != "epsg" or not n.isdigit() or int(n) <= 0 or int(n) > 2**32 - 1:
        return None
    return int(n)


def epsg_of(v):
    x = v.get("proj:epsg")
    if type(x) is int and 0 < x <= 2**32 - 1:
        return x
    c = v.get("proj:code")
    return code_epsg(c) if isinstance(c, str) else None


def shape_of(v):
    a = v.get("proj:shape")
    if not isinstance(a, list) or len(a) != 2 or any(type(x) is not int or x <= 0 or x > 2**32 - 1 for x in a):
        return None
    return a


def transform_of(v):
    return numbers(v.get("proj:transform"), 6)


def band_of(v):
    return compact_band({
        "name": text_of(v, "name"),
        "commonName": text_of(v, "common_name") or text_of(v, "eo:common_name"),
        "centerWavelength": num_of(v, "center_wavelength") if num_of(v, "center_wavelength") is not None
        else num_of(v, "eo:center_wavelength"),
    })


def compact_band(d):
    return {k: v for k, v in d.items() if v is not None}


def raster_band_of(v):
    return compact_band({
        "dataType": text_of(v, "data_type"),
        "nodata": num_of(v, "nodata"),
        "scale": num_of(v, "scale"),
        "offset": num_of(v, "offset"),
        "spatialResolution": num_of(v, "spatial_resolution"),
        "unit": text_of(v, "unit"),
    })


def listed(v, key):
    x = v.get(key)
    return x if isinstance(x, list) else []


def read_asset(key, v, props):
    raw = text_of(v, "href")
    if raw is None:
        return None
    href, signed = stable_href(raw)
    unified, eo, rb = listed(v, "bands"), listed(v, "eo:bands"), listed(v, "raster:bands")
    bands = [band_of(b) for b in (unified if not eo else eo)]
    if rb:
        rbands = [raster_band_of(b) for b in rb]
    else:
        rbands = [raster_band_of(b) for b in unified if any(k in b for k in ("data_type", "scale", "nodata"))]
    a = {"key": key, "href": href}
    if signed:
        a["signed"] = True
    for k, val in (("title", text_of(v, "title")), ("type", text_of(v, "type"))):
        if val is not None:
            a[k] = val
    roles = texts_of(v, "roles")
    if roles:
        a["roles"] = roles
    if bands:
        a["bands"] = bands
    if rbands:
        a["rasterBands"] = rbands
    epsg = epsg_of(v) or epsg_of(props)
    shape = shape_of(v) or shape_of(props)
    tr = transform_of(v) or transform_of(props)
    if epsg:
        a["epsg"] = epsg
    if shape:
        a["shape"] = shape
    if tr:
        a["transform"] = tr
    region, pays = storage_of(v, props)
    if region:
        a["region"] = region
    if pays:
        a["requesterPays"] = True
    return a


def storage_of(v, props):
    """The storage extension's region and requester pays (docs/adr/0644 §1): v1's `storage:region` and
    `storage:requester_pays` on the asset, else on the item; v2's first of the asset's `storage:refs` among the item's
    `storage:schemes`."""
    def v1(o):
        r = o.get("storage:region")
        rp = o.get("storage:requester_pays")
        return (r.strip() if isinstance(r, str) and r.strip() else None), (rp if isinstance(rp, bool) else None)
    r, rp = v1(v)
    pr, prp = v1(props)
    refs = v.get("storage:refs")
    schemes = props.get("storage:schemes")
    sr, srp = None, None
    if isinstance(refs, list) and isinstance(schemes, dict):
        for ref in refs:
            sch = schemes.get(ref) if isinstance(ref, str) else None
            if isinstance(sch, dict):
                x = sch.get("region")
                sr = x.strip() if isinstance(x, str) and x.strip() else None
                y = sch.get("requester_pays")
                srp = y if isinstance(y, bool) else None
                break
    region = r or pr or sr
    pays = rp if rp is not None else (prp if prp is not None else srp)
    return region, bool(pays)


def read_item(catalog, v):
    iid = text_of(v, "id")
    if iid is None:
        return None
    props = v.get("properties") if isinstance(v.get("properties"), dict) else {}
    assets = []
    if isinstance(v.get("assets"), dict):
        for k in sorted(v["assets"]):
            a = read_asset(k, v["assets"][k], props) if isinstance(v["assets"][k], dict) else None
            if a:
                assets.append(a)
    links = listed(v, "links")

    def link(rel):
        for l in links:
            if isinstance(l, dict) and l.get("rel") == rel:
                return text_of(l, "href")
        return None

    item = {"catalog": catalog, "id": iid}
    pairs = [
        ("collection", text_of(v, "collection")),
        ("datetime", text_of(props, "datetime") or text_of(props, "start_datetime")),
        ("endDatetime", text_of(props, "end_datetime")),
        ("bbox", numbers(v.get("bbox"), 4)),
        ("cloudCover", num_of(props, "eo:cloud_cover")),
        ("platform", text_of(props, "platform")),
        ("instruments", texts_of(props, "instruments") or None),
        ("gsd", num_of(props, "gsd")),
        ("epsg", epsg_of(props)),
        ("license", text_of(props, "license")),
        ("licenseHref", stable_href(link("license"))[0] if link("license") else None),
        ("selfHref", stable_href(link("self"))[0] if link("self") else None),
    ]
    for k, val in pairs:
        if val is not None:
            item[k] = val
    item["assets"] = assets
    return dict(sorted(item.items(), key=lambda kv: 0))  # key order is irrelevant to JSON equality


def read_page(endpoint, doc):
    """(items, returned, matched, next) of a search answer; None when it is not one."""
    if not isinstance(doc, dict) or not isinstance(doc.get("features"), list):
        return None
    items = [i for i in (read_item(endpoint, f) if isinstance(f, dict) else None for f in doc["features"]) if i]
    matched = doc.get("numberMatched")
    if type(matched) is not int or matched < 0:
        ctx = doc.get("context")
        matched = ctx.get("matched") if isinstance(ctx, dict) else None
        if type(matched) is not int or matched < 0:
            matched = None
    nxt = None
    for l in listed(doc, "links"):
        if isinstance(l, dict) and l.get("rel") == "next":
            href = text_of(l, "href")
            href = resolve(endpoint, href) if href else None
            if href:
                method = "POST" if (text_of(l, "method") or "").upper() == "POST" else "GET"
                nxt = {"method": method, "href": href}
                if isinstance(l.get("body"), dict):
                    nxt["body"] = l["body"]
                if l.get("merge") is True:
                    nxt["merge"] = True
            break
    return items, len(doc["features"]), matched, nxt


def within_cloud(items, mx):
    if mx is None:
        return items, 0
    kept = [i for i in items if i.get("cloudCover") is None or i["cloudCover"] <= mx]
    return kept, len(items) - len(kept)


def bbox_problem(b):
    w, s, e, n = b
    return not (-180 <= w <= 180 and -180 <= e <= 180 and -90 <= s <= 90 and -90 <= n <= 90 and w < e and s <= n)


def search_request(inp):
    """(request, endpoint, maxCloud) or (None, code, path)."""
    ep = normalize_endpoint(inp["endpoint"])
    if ep is None:
        return None, "invalid_stac_endpoint", "endpoint"
    if inp.get("bbox") is not None and bbox_problem(inp["bbox"]):
        return None, "invalid_stac_bbox", "bbox"
    when = None
    if inp.get("datetime") is not None:
        when = normalize_datetime(inp["datetime"])
        if when is None:
            return None, "invalid_stac_datetime", "datetime"
    mc = inp.get("maxCloudCover")
    if mc is not None and not (0 <= mc <= 100):
        return None, "invalid_stac_cloud_cover", "maxCloudCover"
    lim = inp.get("limit")
    if lim is not None and not (1 <= lim <= 100):
        return None, "invalid_stac_limit", "limit"
    for i, c in enumerate(inp.get("collections", [])):
        if not c.strip() or any(ord(ch) < 32 for ch in c):
            return None, "invalid_stac_collection", f"collections[{i}]"
    for i, c in enumerate(inp.get("ids", [])):
        if not c.strip() or any(ord(ch) < 32 for ch in c):
            return None, "invalid_stac_collection", f"ids[{i}]"
    body = {}
    if inp.get("bbox") is not None:
        body["bbox"] = inp["bbox"]
    if inp.get("collections"):
        body["collections"] = [c.strip() for c in inp["collections"]]
    if when:
        body["datetime"] = when
    if inp.get("ids"):
        body["ids"] = [c.strip() for c in inp["ids"]]
    body["limit"] = lim if lim is not None else 10
    if mc is not None:
        body["query"] = {"eo:cloud_cover": {"lte": mc}}
    nxt = inp.get("next")
    if nxt is None:
        return {"method": "POST", "url": f"{ep}/search", "body": canonical(body)}, ep, mc
    href = nxt["href"].strip()
    if origin(ep) != origin(href):
        return None, "stac_next_foreign", "next.href"
    if nxt["method"] == "GET":
        return {"method": "GET", "url": href}, ep, mc
    merged = dict(body) if nxt.get("merge") else {}
    merged.update(nxt.get("body") or {})
    return {"method": "POST", "url": href, "body": canonical(merged)}, ep, mc


# --- Preparation ------------------------------------------------------------------------------------------------

SAMPLES = {"uint8": "u8", "int8": "i8", "uint16": "u16", "int16": "i16", "uint32": "u32", "int32": "i32",
           "float32": "f32", "float64": "f64"}


def default_mask(key):
    key = key.lower()
    if key == "scl":
        return {"kind": "values", "values": [0, 1, 3, 8, 9, 10]}
    if key == "qa_pixel":
        return {"kind": "bits", "bits": [0, 1, 2, 3, 4]}
    return None


def normal_mask(m):
    if m["kind"] == "values":
        v = sorted(set(m["values"]))
        return {"kind": "values", "values": v} if v and all(x <= 65535 for x in v) else None
    v = sorted(set(m["bits"]))
    return {"kind": "bits", "bits": v} if v and all(x < 32 for x in v) else None


def mask_text(m):
    k = "values" if m["kind"] == "values" else "bits"
    return f"{k}:" + ",".join(str(x) for x in m[k])


def short(x):
    """A number as a request writes it: the shortest text that reads back, no exponent, -0 as 0."""
    if x == 0:
        return "0"
    text = format(Decimal(repr(float(x))), "f")
    return text.rstrip("0").rstrip(".") if "." in text else text


def system(srid):
    return "yerel" if srid == 0 else f"EPSG:{srid}"


def prepare_plan(inp, srid):
    """The preparation's output (without layers and uids) and warnings, or (None, code, path)."""
    item = inp["item"]
    assets = item["assets"]
    if not item["id"].strip() or any(ord(c) < 32 for c in item["id"]):
        return None, "invalid_stac_item", "item.id"
    if normalize_endpoint(item["catalog"]) is None:
        return None, "invalid_stac_item", "item.catalog"
    seen = []
    for i, a in enumerate(assets):
        if not a["key"].strip() or any(ord(c) < 32 for c in a["key"]) or a["key"] in seen:
            return None, "invalid_stac_item", f"item.assets[{i}].key"
        seen.append(a["key"])
    keys = []
    for i, k in enumerate(inp["assets"]):
        if k not in [x for x, _ in keys]:
            keys.append((k, f"assets[{i}]"))
    if not keys:
        return None, "stac_assets_empty", "assets"
    if len(keys) > 32:
        return None, "stac_too_many_assets", "assets"
    by = {a["key"]: a for a in assets}
    wanted = []
    for k, at in keys:
        if k not in by:
            return None, "stac_asset_not_found", at
        wanted.append((by[k], False, at))
    qa = inp.get("qa")
    if qa:
        if qa["asset"] not in by:
            return None, "stac_asset_not_found", "qa.asset"
        if any(k == qa["asset"] for k, _ in keys):
            return None, "stac_qa_is_data", "qa.asset"
        wanted.append((by[qa["asset"]], True, "qa.asset"))
    local = {k: v for k, v in sorted((inp.get("local") or {}).items())}
    for key, path in local.items():
        if key not in [a["key"] for a, _, _ in wanted]:
            return None, "stac_asset_not_found", f"local.{key}"
        if not path.strip():
            return None, "invalid_file_path", f"local.{key}"
    def local_of(key):
        return local[key].strip() if key in local else None
    for a, _, at in wanted:
        if local_of(a["key"]) is None and not is_http(a["href"]) and not a["href"].strip().startswith("s3://"):
            return None, "stac_href_not_http", at
    for a, _, at in wanted:
        if "shape" not in a or "transform" not in a or "epsg" not in a:
            return None, "stac_no_grid", at
    for a, _, at in wanted:
        if len(a.get("rasterBands", [])) > 1 or len(a.get("bands", [])) > 1:
            return None, "stac_multiband", at
    grids = []
    for a, _, at in wanted:
        rb = (a.get("rasterBands") or [{}])[0]
        if rb.get("dataType") not in SAMPLES:
            return None, "stac_unsupported_sample", at
        t = a["transform"]
        grids.append({
            "sample": SAMPLES[rb["dataType"]], "nodata": rb.get("nodata"), "scale": rb.get("scale"),
            "offset": rb.get("offset"), "width": a["shape"][1], "height": a["shape"][0],
            "affine": [t[2], t[0], t[1], t[5], t[3], t[4]], "epsg": a["epsg"],
        })
    mask = None
    if qa:
        if qa.get("mask") is not None:
            m = normal_mask(qa["mask"])
            if m is None:
                return None, "stac_qa_unknown", "qa.mask"
            mask = (m, False)
        else:
            d = default_mask(qa["asset"])
            if d is None:
                return None, "stac_qa_unknown", "qa.mask"
            mask = (d, True)
    reproject = bool(inp.get("reproject"))
    for (a, _, at), g in zip(wanted, grids):
        if g["epsg"] != srid and (not reproject or srid == 0):
            return None, "crs_mismatch", at
    for (a, _, at), g in zip(wanted, grids):
        if g["epsg"] != srid and local_of(a["key"]) is None:
            return None, "stac_local_missing", "local"
    for (a, _, _), g in zip(wanted, grids):
        path = local_of(a["key"])
        if path is not None and path.startswith("/") and not Path(path).is_file():
            return None, "file_not_found", f"local.{a['key']}"
        if path is not None and g["epsg"] != srid:
            why = local_problem(path, g["epsg"])
            if why:
                return None, why, f"local.{a['key']}"
    g_in = inp.get("group")
    if g_in is None:
        group = f"STAC: {item['id']}"
    else:
        gname = g_in.strip()
        if not gname or len(gname) > 120 or any(ord(c) < 32 for c in gname):
            return None, "invalid_group_name", "group"
        group = gname
    bands, prepared, qa_prep = [], [], None
    qa_attrs = None
    for (a, is_qa, _), g in zip(wanted, grids):
        attrs = {"stac.catalog": item["catalog"], "stac.item": item["id"], "stac.asset": a["key"],
                 "stac.href": a["href"], "stac.role": "qa" if is_qa else "data"}
        acc = access_of(item["catalog"], a)
        if acc:
            attrs["stac.access"] = acc
        if a.get("requesterPays"):
            attrs["stac.requester_pays"] = "true"
        if local_of(a["key"]) is not None:
            attrs["stac.local"] = local_of(a["key"])
        if g["epsg"] != srid:
            attrs["stac.reprojected_from"] = f"EPSG:{g['epsg']}"
        if item.get("collection"):
            attrs["stac.collection"] = item["collection"]
        if item.get("datetime"):
            attrs["stac.datetime"] = item["datetime"]
        if item.get("cloudCover") is not None:
            attrs["stac.cloud_cover"] = short(item["cloudCover"])
        if item.get("platform"):
            attrs["stac.platform"] = item["platform"]
        lic = item.get("license") or item.get("licenseHref")
        if lic:
            attrs["stac.license"] = lic
        s = inp.get("search")
        if s:
            parts = []
            if s.get("bbox"):
                parts.append("bbox=" + ",".join(short(x) for x in s["bbox"]))
            if s.get("datetime"):
                parts.append("datetime=" + s["datetime"])
            if s.get("collections"):
                parts.append("collections=" + ",".join(s["collections"]))
            if s.get("maxCloudCover") is not None:
                parts.append("maxCloudCover=" + short(s["maxCloudCover"]))
            if parts:
                attrs["stac.search"] = "; ".join(parts)
        if g["nodata"] is not None:
            attrs["stac.nodata"] = short(g["nodata"])
        raster = {
            "kind": "raster", "affine": g["affine"], "width": g["width"], "height": g["height"], "bands": 1,
            "sample": g["sample"], "srid": g["epsg"], "attrs": attrs,
            "style": raster_style(g, is_qa),
        }
        if local_of(a["key"]) is not None:
            raster["file"] = local_of(a["key"])
        else:
            raster["url"] = read_url(a)
        moved = None
        if g["epsg"] != srid:
            moved = reprojected_of(local_of(a["key"]), g["epsg"], srid)
            raster.update({"affine": moved["affine"], "width": moved["width"], "height": moved["height"],
                           "srid": srid, "file": moved["file"]})
        if is_qa:
            m, default = mask
            attrs["stac.qa_mask"] = mask_text(m)
            qa_attrs = raster
            qa_prep = {"asset": a["key"], "href": a["href"], "sample": g["sample"], "mask": m, "defaultMask": default,
                       "width": g["width"], "height": g["height"], "affine": g["affine"]}
            if a.get("signed"):
                qa_prep["signed"] = True
            if g["nodata"] is not None:
                qa_prep["nodata"] = g["nodata"]
            if local_of(a["key"]) is not None:
                qa_prep["file"] = local_of(a["key"])
            if moved:
                qa_prep["reprojected"] = moved
        else:
            scale = g["scale"] if g["scale"] is not None else 1.0
            offset = g["offset"] if g["offset"] is not None else 0.0
            common = ((a.get("bands") or [{}])[0]).get("commonName")
            if common:
                attrs["stac.band"] = common
            attrs["stac.scale"] = short(scale)
            attrs["stac.offset"] = short(offset)
            bands.append(raster)
            b = {"asset": a["key"], "href": a["href"], "sample": g["sample"], "scale": scale, "offset": offset,
                 "width": g["width"], "height": g["height"], "affine": g["affine"], "epsg": g["epsg"]}
            if common:
                b["commonName"] = common
            if a.get("signed"):
                b["signed"] = True
            if g["nodata"] is not None:
                b["nodata"] = g["nodata"]
            if local_of(a["key"]) is not None:
                b["file"] = local_of(a["key"])
            if moved:
                b["reprojected"] = moved
            prepared.append(b)
    warnings = []
    signed = [a["key"] for a, _, _ in wanted if a.get("signed") and access_of(item["catalog"], a) != "planetary-computer"
              and local_of(a["key"]) is None]
    if signed:
        warnings.append(("stac_signed_href_stripped", "assets", signed))
    paying = [a["key"] for a, _, _ in wanted if a.get("requesterPays") and local_of(a["key"]) is None]
    if paying:
        warnings.append(("stac_requester_pays", "assets", paying))
    if item.get("datetime") is None or item.get("cloudCover") is None:
        warnings.append(("stac_item_incomplete", "item", None))
    if prepared:
        f = prepared[0]
        if not all((b["width"], b["height"], b["affine"]) == (f["width"], f["height"], f["affine"]) for b in prepared):
            warnings.append(("stac_grid_differs", "assets", None))
        if not all((b["scale"], b["offset"]) == (f["scale"], f["offset"]) for b in prepared):
            warnings.append(("stac_scale_differs", "assets", None))
        if qa_prep and (qa_prep["width"], qa_prep["height"], qa_prep["affine"]) != (f["width"], f["height"], f["affine"]):
            warnings.append(("stac_qa_grid_differs", "qa.asset", qa_prep["asset"]))
    refl = None
    if prepared:
        f = prepared[0]
        data_assets = [a for a, q, _ in wanted if not q]
        if all("reflectance" in a.get("roles", []) for a in data_assets) and all(
                (b["scale"], b["offset"]) == (f["scale"], f["offset"]) for b in prepared):
            refl = {"scale": f["scale"], "offset": f["offset"]}
    rec = {"format": "kentos.stac-preparation", "version": 1, "catalog": item["catalog"], "item": item["id"],
           "bands": prepared}
    if inp.get("search"):
        rec["search"] = inp["search"]
    for k, kk in (("collection", "collection"), ("datetime", "datetime"), ("cloudCover", "cloudCover"),
                  ("platform", "platform"), ("license", "license"), ("licenseHref", "licenseHref")):
        if item.get(kk) is not None:
            rec[k] = item[kk]
    if qa_prep:
        rec["qa"] = qa_prep
    if refl:
        rec["reflectance"] = refl
    return {"group": group, "rasters": bands, "qa": qa_attrs, "record": rec, "warnings": warnings}, None, None


PLANETARY_HOST = "planetarycomputer.microsoft.com"


def access_of(catalog, a):
    """`stac.access` (docs/adr/0644 §1): aws for an s3 address, planetary-computer for the Planetary Computer's blobs."""
    if a["href"].strip().startswith("s3://"):
        return "aws"
    host = catalog.split("://", 1)[1].split("/")[0].split("?")[0].split("#")[0].lower() if "://" in catalog else ""
    return "planetary-computer" if host == PLANETARY_HOST and blob_parts(a["href"]) else None


def read_url(a):
    return s3_url(a["href"].strip(), a.get("region")) if a["href"].strip().startswith("s3://") else a["href"]


REPROJECTED = {}


def local_problem(local, src):
    """Why a local copy cannot be carried over: not a raster (GDAL cannot open it), or its middle does not reach the
    project's system in its own (or the asset's) system."""
    import math
    from osgeo import gdal
    import raster_reproject_cases as rr
    name = local.split("/")[-1]
    gdal.UseExceptions()
    try:
        ds = gdal.Open(str(rr.DIR / name))
    except RuntimeError:
        return "import_unreadable"
    ref = ds.GetSpatialRef()
    srid = int(ref.GetAuthorityCode(None)) if ref is not None and ref.GetAuthorityCode(None) else src
    x0, a, b, y0, c, d = ds.GetGeoTransform()
    u, v = ds.RasterXSize // 2, ds.RasterYSize // 2
    t, _, _ = rr.ct.transformer(rr.REGISTRY[srid], rr.REGISTRY[rr.PROJECT])
    x, y = t.transform(x0 + a * u + b * v, y0 + c * u + d * v)
    return None if math.isfinite(x) and math.isfinite(y) else "crs_unreached"


def reprojected_of(local, src, dst):
    """A local copy carried into the project's system: raster_reproject_cases.py's grid and way (nearest)."""
    import raster_reproject_cases as rr
    name = local.split("/")[-1]
    s = rr.Source(name)
    assert s.srid == src and dst == rr.PROJECT, (name, s.srid, src, dst)
    affine, w, h = rr.grid_of(s, None)
    stem = name.rsplit(".", 1)[0]
    folder = local.rsplit("/", 1)[0]
    return {"from": src, "to": dst, "way": rr.way_of(src, dst), "file": f"{folder}/{stem}-epsg{dst}.tif",
            "width": w, "height": h, "affine": affine}


def raster_style(g, is_qa):
    stretch = None
    if g["sample"] == "u8":
        stretch = None
    elif is_qa:
        stretch = "minMax"
    else:
        stretch = "percent"
    st = {"render": "gray", "bands": [1]}
    if stretch:
        st["stretch"] = stretch
    if g["nodata"] is not None:
        st["nodata"] = g["nodata"]
    return st


def warning_message(code, path, extra, item=None):
    if code == "stac_signed_href_stripped":
        return (f"{', '.join(extra)} varlığının adresindeki imza (geçici erişim anahtarı) projeye yazılmadı; "
                "dosyayı okumak için imza gerekiyorsa raster açılmaz.")
    if code == "stac_item_incomplete":
        return "Öğenin çekim zamanı ya da bulut oranı yok; kayıt bunları boş taşır."
    if code == "stac_grid_differs":
        return ("Bantların ızgarası (piksel boyu ya da yeri) aynı değil; birlikte işlemeden önce ortak ızgaraya "
                "örneklenmeli.")
    if code == "stac_scale_differs":
        return ("Bantların ölçeği ya da ötelemesi aynı değil; Spektral indis tek ölçek ve öteleme alır, bantlar ayrı "
                "ayrı yansımaya çevrilmeli.")
    if code == "stac_requester_pays":
        return (f"{', '.join(extra)} varlığının kovası istek ödemeli: okumak için AWS kimlik bilgisi gerekir "
                "(AWS_ACCESS_KEY_ID ve AWS_SECRET_ACCESS_KEY ya da ~/.aws/credentials) ve okumanın ücretini siz ödersiniz.")
    if code == "stac_qa_grid_differs":
        return (f"Kalite bandının ({extra}) ızgarası veri bantlarınınkinden başka; maske uygulanmadan önce "
                "örneklenmeli.")
    raise KeyError(code)


# ---------------------------------------------------------------------------------------------------------------
# Recorded answers.

def load(name):
    return json.loads((REC / name).read_text())


def synthetic_files():
    """The `synthetic-*` answers (hand-written from the STAC specification and the extensions' examples)."""
    out = {}
    note = "kentos:note"

    def item(iid, collection, props, assets, links=None, bbox=(36.0, 39.0, 37.0, 40.0)):
        return {"type": "Feature", "stac_version": "1.0.0", "id": iid, "collection": collection, "bbox": list(bbox),
                "properties": props, "assets": assets,
                "links": links or [{"rel": "self", "href": f"{EP}/collections/{collection}/items/{iid}"}]}

    def band(common, scale=None, offset=None, nodata=0, dtype="uint16", res=10):
        rb = {"nodata": nodata, "data_type": dtype, "spatial_resolution": res}
        if scale is not None:
            rb["scale"] = scale
        if offset is not None:
            rb["offset"] = offset
        return {"eo:bands": [{"name": common, "common_name": common}], "raster:bands": [rb]}

    sas = "?st=2026-10-19T08%3A00%3A00Z&se=2026-10-20T08%3A00%3A00Z&sp=r&sv=2021-06-08&sr=c&sig=AbCdEf%2Bgh%3D"
    pc = "https://sentinel2l2a01.blob.core.windows.net/sentinel2-l2/36/S/UD/2026/07/13/S2B_MSIL2A_x/GRANULE/IMG_DATA"
    grid10 = {"proj:shape": [10980, 10980], "proj:transform": [10, 0, 399960, 0, -10, 4400040]}
    grid20 = {"proj:shape": [5490, 5490], "proj:transform": [20, 0, 399960, 0, -20, 4400040]}
    out["synthetic-signed.json"] = {
        note: "Written by scripts/fixtures/stac_cases.py: a Planetary-Computer-like answer whose asset addresses carry a SAS signature; item-level proj:code; an item without id, one without cloud cover and a datetime interval.",
        "type": "FeatureCollection", "stac_version": "1.0.0", "numberReturned": 3, "numberMatched": 3,
        "features": [
            item("S2B_MSIL2A_20260713_R121_T36SUD", "sentinel-2-l2a",
                 {"datetime": "2026-07-13T08:29:11.025000Z", "eo:cloud_cover": 1.2, "platform": "Sentinel-2B",
                  "proj:code": "EPSG:32636", "license": "proprietary"},
                 {"B04": {"href": f"{pc}/R10m/T36SUD_20260713T082911_B04_10m.tif{sas}", "type": "image/tiff; application=geotiff; profile=cloud-optimized", "roles": ["data", "reflectance"], **grid10, **band("red", 0.0001, -0.1)},
                  "B08": {"href": f"{pc}/R10m/T36SUD_20260713T082911_B08_10m.tif{sas}", "roles": ["data", "reflectance"], **grid10, **band("nir", 0.0001, -0.1)},
                  "SCL": {"href": f"{pc}/R20m/T36SUD_20260713T082911_SCL_20m.tif{sas}", "roles": ["data"], **grid20,
                          "raster:bands": [{"nodata": 0, "data_type": "uint8", "spatial_resolution": 20}]}},
                 links=[{"rel": "self", "href": f"{EP}/collections/sentinel-2-l2a/items/S2B_MSIL2A_20260713_R121_T36SUD"},
                        {"rel": "license", "href": "https://scihub.copernicus.eu/twiki/pub/SciHubWebPortal/TermsConditions/Sentinel_Data_Terms_and_Conditions.pdf?sig=x&se=1"}]),
            {"type": "Feature", "properties": {"datetime": "2026-07-14T00:00:00Z"}, "assets": {}, "links": []},
            item("NOCLOUD_1", "sentinel-2-l2a",
                 {"start_datetime": "2026-07-01T00:00:00Z", "end_datetime": "2026-07-31T23:59:59Z", "proj:epsg": 32636},
                 {"B04": {"href": "https://h.example/b04.tif?versionId=7", **grid10, **band("red", 0.0001, -0.1)}}),
        ],
        "links": [],
    }
    utm = {"proj:shape": [30, 40], "proj:transform": [30, 0, 725340, 0, -30, 4373310]}
    utm20 = {"proj:shape": [20, 30], "proj:transform": [20, 0, 725340, 0, -20, 4373310]}
    out["synthetic-storage.json"] = {
        note: "Written by scripts/fixtures/stac_cases.py: the storage extension v2 (an item's storage:schemes, an asset's storage:refs), s3 and gs addresses, and an item in UTM 36 (EPSG:32636) whose assets fixtures/raster/v1/reproject's files copy (docs/adr/0644).",
        "type": "FeatureCollection", "stac_version": "1.1.0", "numberReturned": 2,
        "features": [
            item("STORAGE_1", "c", {"datetime": "2026-08-01T08:00:00Z", "eo:cloud_cover": 2, "proj:epsg": 32637,
                                    "proj:shape": [100, 100], "proj:transform": [10, 0, 300000, 0, -10, 4400000],
                                    "storage:schemes": {"aws": {"type": "aws-s3", "platform": "https://{bucket}.s3.{region}.amazonaws.com",
                                                                "bucket": "kova-tr", "region": "eu-central-1", "requester_pays": False},
                                                        "pays": {"type": "aws-s3", "region": "us-west-2", "requester_pays": True}}},
                 {"b04": {"href": "s3://kova-tr/a/B04 bant.tif", "storage:refs": ["aws"], "roles": ["data"], **band("red")},
                  "b08": {"href": "s3://kova-tr/a/B08.tif", "storage:refs": ["yok", "pays"], "roles": ["data"], **band("nir")},
                  "dotted": {"href": "s3://my.bucket/x.tif", "storage:region": "us-west-2", "roles": ["data"], **band("swir16")},
                  "noregion": {"href": "s3://kova/y.tif", "roles": ["data"], **band("green")},
                  "gs": {"href": "gs://bucket/x.tif", "roles": ["data"], **band("blue")}}),
            item("REPROJ_1", "c", {"datetime": "2026-08-02T08:00:00Z", "eo:cloud_cover": 1, "proj:epsg": 32636},
                 {"red": {"href": "https://h.example/utm36.tif", "roles": ["data", "reflectance"], **utm, **band("red", 0.0001, -0.1)},
                  "scl": {"href": "https://h.example/siniflar.tif", "roles": ["data"], **utm20,
                          "raster:bands": [{"nodata": 0, "data_type": "uint8", "spatial_resolution": 20}]}}),
        ],
        "links": [],
    }
    out["synthetic-cloudy.json"] = {
        note: "Written by scripts/fixtures/stac_cases.py: a server that ignored the cloud filter; items at 25 and 5 percent and one without a cloud cover.",
        "type": "FeatureCollection", "stac_version": "1.0.0", "numberReturned": 3,
        "features": [
            item("CLOUDY_25", "c", {"datetime": "2026-07-01T10:00:00Z", "eo:cloud_cover": 25}, {}),
            item("CLEAR_5", "c", {"datetime": "2026-07-02T10:00:00Z", "eo:cloud_cover": 5}, {}),
            item("UNKNOWN", "c", {"datetime": "2026-07-03T10:00:00Z"}, {}),
        ],
        "links": [],
    }
    out["synthetic-unified.json"] = {
        note: "Written by scripts/fixtures/stac_cases.py: STAC 1.1's unified `bands` array and a license given by the item.",
        "type": "FeatureCollection", "stac_version": "1.1.0", "context": {"matched": 1, "returned": 1},
        "features": [
            item("UNIFIED_1", "c", {"datetime": "2026-06-01T09:00:00Z", "eo:cloud_cover": 0.0, "proj:epsg": 32637, "license": "CC-BY-4.0", "gsd": 30, "platform": "landsat-9", "instruments": ["oli2", "tirs2"]},
                 {"red": {"href": "https://h.example/red.tif", "roles": ["data", "reflectance"], "proj:shape": [100, 100], "proj:transform": [30, 0, 300000, 0, -30, 4400000, 0, 0, 1],
                          "bands": [{"name": "SR_B4", "eo:common_name": "red", "eo:center_wavelength": 0.65, "data_type": "uint16", "nodata": 0, "scale": 2.75e-05, "offset": -0.2}]}}),
        ],
        "links": [],
    }
    out["synthetic-grids.json"] = {
        note: "Written by scripts/fixtures/stac_cases.py: assets a preparation refuses (no grid, a type a raster does not hold, three bands) and ones it takes, with the grid in the item's properties.",
        "type": "FeatureCollection", "stac_version": "1.0.0", "numberReturned": 2, "numberMatched": 2,
        "features": [
            item("GRIDS_1", "c", {"datetime": "2026-05-05T08:00:00Z", "eo:cloud_cover": 3, "proj:epsg": 32637, "proj:shape": [200, 300], "proj:transform": [10, 0, 300000, 0, -10, 4400000]},
                 {"plain": {"href": "https://h.example/plain.tif", "roles": ["data"], "raster:bands": [{"data_type": "float32", "nodata": -9999}]},
                  "complex": {"href": "https://h.example/complex.tif", "raster:bands": [{"data_type": "cint16"}]},
                  "notype": {"href": "https://h.example/notype.tif", "eo:bands": [{"common_name": "red"}]},
                  "rgb": {"href": "https://h.example/rgb.tif", "eo:bands": [{"common_name": "red"}, {"common_name": "green"}, {"common_name": "blue"}]},
                  "dem": {"href": "https://h.example/dem.tif", "roles": ["data"], "raster:bands": [{"data_type": "int16", "nodata": -32768, "scale": 1, "offset": 0}]}}),
            item("GRIDS_2", "c", {"datetime": "2026-05-06T08:00:00Z", "eo:cloud_cover": 4, "proj:epsg": 32637},
                 {"nogrid": {"href": "https://h.example/nogrid.tif", "raster:bands": [{"data_type": "uint16"}]},
                  "halfgrid": {"href": "https://h.example/halfgrid.tif", "proj:shape": [10, 10], "raster:bands": [{"data_type": "uint16"}]}}),
        ],
        "links": [],
    }
    out["synthetic-get-next-1.json"] = {
        note: "Written by scripts/fixtures/stac_cases.py: a page whose next link is a GET with a relative address.",
        "type": "FeatureCollection", "stac_version": "1.0.0", "numberReturned": 1,
        "features": [item("PAGE_1", "c", {"datetime": "2026-01-01T00:00:00Z"}, {})],
        "links": [{"rel": "next", "href": "/v1/search?token=next%3Apage2", "method": "GET"}],
    }
    out["synthetic-get-next-2.json"] = {
        note: "Written by scripts/fixtures/stac_cases.py: the page after.",
        "type": "FeatureCollection", "stac_version": "1.0.0", "numberReturned": 1,
        "features": [item("PAGE_2", "c", {"datetime": "2026-01-02T00:00:00Z"}, {})],
        "links": [],
    }
    out["synthetic-empty.json"] = {
        note: "Written by scripts/fixtures/stac_cases.py: no item matches.",
        "type": "FeatureCollection", "stac_version": "1.0.0", "numberMatched": 0, "numberReturned": 0,
        "features": [], "links": [],
    }
    out["synthetic-no-features.json"] = {
        note: "Written by scripts/fixtures/stac_cases.py: an error document, not an item collection.",
        "code": "BadRequest", "description": "Unknown collection",
    }
    return out


def write_synthetic(files):
    for name, doc in files.items():
        (REC / name).write_text(json.dumps(doc, indent=1, ensure_ascii=False) + "\n")
    (REC / "not-json.txt").write_text("<html><body>502 Bad Gateway</body></html>\n")


# ---------------------------------------------------------------------------------------------------------------
# The cases.

NOTE = "Written by scripts/fixtures/stac_cases.py from docs/adr/0528 and the STAC specification; do not edit by hand."

LAYER_STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}


def setup_doc(srid=32637):
    layer = lambda i, name, **kw: {"id": i, "name": name, "type": kw.get("type", "layer"), "visible": True, "locked": False,
                                   "expanded": True, "style": LAYER_STYLE, "children": []}
    return {"format": "kentos.document", "version": 1, "name": "STAC",
            "settings": {"srid": srid, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                         "plotScale": 1000, "workspace": "gis"},
            "origin": {"x": 399960.0, "y": 4400040.0},
            "layers": [layer("altlik", "Altlık")], "activeLayer": "altlik", "entities": [],
            "styles": {"items": [], "categories": []}}


def request_info(req):
    r = {"method": req["method"], "url": req["url"]}
    if req.get("body") is not None:
        r["body"] = req["body"]
    return r


class Index:
    """The recorded transport's index, built from the requests the cases make."""

    def __init__(self):
        self.entries = []

    def add(self, req, file=None, error=None):
        e = {"method": req["method"], "url": req["url"]}
        if req.get("body") is not None:
            e["body"] = req["body"]
        if file:
            e["file"] = file
        if error:
            e["error"] = error
        if e not in self.entries:
            self.entries.append(e)


SEARCH_SETUP_NOTE = "kurulum: çizim gerekmez, yalnız komutun çalışacağı belge"


def search_cases(index):
    cases = []
    base = {"endpoint": EP, "bbox": [36.9, 39.7, 37.1, 39.8], "datetime": "2024-07-01/2024-07-20",
            "collections": ["sentinel-2-l2a"], "maxCloudCover": 20, "limit": 2}

    def refused(name, inp, op="execute"):
        req, code, path = search_request(inp)
        assert req is None, name
        return {"name": name, "steps": [{"op": op, "input": inp, "result": {"status": "failed", "error": {"code": code, "message": MESSAGES[code](inp), "path": path}}}]}

    def answered(name, inp, file, op="execute", note=None, error=None, code=None, message=None):
        req, ep, mc = search_request(inp)
        assert req is not None, name
        if error:
            index.add(req, error=error)
            said = f"{req['method']} {req['url']} isteği cevap vermedi: {error}"
            return {"name": name, "steps": [{"op": op, "input": inp, "result": {"status": "failed", "error": {"code": code, "message": said}}}]}
        index.add(req, file=file)
        return {"name": name, "_req": req, "_ep": ep, "_mc": mc, "_file": file, "_op": op, "_input": inp, "_note": note}

    # Refusals, in the order of the contract.
    cases.append(refused("Ret: uç nokta http değil", {**base, "endpoint": "ftp://x.example/v1"}))
    cases.append(refused("Ret: uç noktada sorgu var", {**base, "endpoint": EP + "?api_key=1"}))
    cases.append(refused("Ret: uç noktada kullanıcı adı ve parola var", {**base, "endpoint": "https://ali:gizli@x.example/v1"}))
    cases.append(refused("Ret: uç nokta boş", {**base, "endpoint": "  "}))
    cases.append(refused("Ret: kutuda batı doğudan büyük", {**base, "bbox": [37.1, 39.7, 36.9, 39.8]}))
    cases.append(refused("Ret: kutuda enlem 90'dan büyük", {**base, "bbox": [36.9, 39.7, 37.1, 90.5]}))
    cases.append(refused("Ret: kutuda boylam 180'den büyük", {**base, "bbox": [179.5, 10, 180.5, 11]}))
    cases.append(refused("Ret: tarih okunamıyor", {**base, "datetime": "geçen ay"}))
    cases.append(refused("Ret: tarihte başlangıç bitişten sonra", {**base, "datetime": "2024-07-20/2024-07-01"}))
    cases.append(refused("Ret: tarihte 29 Şubat artık yıl değil", {**base, "datetime": "2023-02-29"}))
    cases.append(refused("Ret: tarih aralığının iki ucu açık", {**base, "datetime": "../.."}))
    cases.append(refused("Ret: bulut oranı 100'den büyük", {**base, "maxCloudCover": 120}))
    cases.append(refused("Ret: bulut oranı eksi", {**base, "maxCloudCover": -1}))
    cases.append(refused("Ret: sayfa boyu 0", {**base, "limit": 0}))
    cases.append(refused("Ret: sayfa boyu 101", {**base, "limit": 101}))
    cases.append(refused("Ret: boş koleksiyon kimliği", {**base, "collections": ["sentinel-2-l2a", " "]}))
    cases.append(refused("Ret: boş öğe kimliği", {**base, "ids": [""]}))
    cases.append(refused("Ret: sonraki sayfa başka sunucuda", {**base, "next": {"method": "POST", "href": "https://kotu.example/v1/search", "body": {"next": "x"}}}))
    cases.append(refused("Ret: sonraki sayfa başka düzenekte (http)", {**base, "next": {"method": "GET", "href": EP.replace("https", "http") + "/search?token=x"}}))
    cases.append(refused("validate: ret", {**base, "limit": 500}, op="validate"))
    cases.append({"name": "validate: istek kurulur, ağa gidilmez", "steps": [{"op": "validate", "input": base, "result": {"status": "completed", "output": None, "warnings": []}}]})
    # Answered from the recorded folder.
    cases.append(answered("Sentinel-2: iki öğe, toplam 4, sonraki sayfa (gerçek Earth Search cevabı)", base, "earth-search-s2-page1.json",
                          note="Bulut en çok 20; sunucu next bağlantısını POST gövdesiyle verir (merge false)."))
    page1 = load("earth-search-s2-page1.json")
    nxt = read_page(EP, page1)[3]
    cases.append(answered("Sentinel-2: sonraki sayfa next bağlantısıyla (gerçek cevap)", {**base, "next": nxt}, "earth-search-s2-page2.json",
                          note="Gövde next.body'dir (merge false): kutu, tarih ve sayfa anahtarı sunucunun yazdığıdır."))
    cases.append(answered("Landsat: s3 adresli varlıklar ve bulut oranı (gerçek cevap)",
                          {"endpoint": EP, "bbox": [36.9, 39.7, 37.1, 39.8], "datetime": "2024-07-01/2024-07-31", "collections": ["landsat-c2-l2"], "maxCloudCover": 30, "limit": 1},
                          "earth-search-landsat.json"))
    cases.append(answered("Plan da aynı cevabı verir (salt okunur)", base, "earth-search-s2-page1.json", op="plan"))
    cases.append(answered("Sunucu bulut süzgecini yok saydı: gelende yeniden denetlenir, bulutsuz bilinmeyen kalır",
                          {"endpoint": EP, "collections": ["c"], "maxCloudCover": 10, "limit": 5}, "synthetic-cloudy.json"))
    cases.append(answered("İmzalı adresler: imza hiçbir yerde görünmez, signed işaretlenir; kimliksiz öğe atlanır",
                          {"endpoint": EP, "collections": ["sentinel-2-l2a"], "datetime": "2026-07-13", "limit": 3, "ids": ["S2B_MSIL2A_20260713_R121_T36SUD"]},
                          "synthetic-signed.json"))
    cases.append(answered("STAC 1.1'in birleşik bands dizisi okunur; context.matched toplamı verir",
                          {"endpoint": EP + "/", "collections": ["c"], "datetime": "2026-06-01T00:00:00Z/..", "limit": 1}, "synthetic-unified.json"))
    cases.append(answered("Sonraki bağlantı GET ve göreli: uç noktanın sunucusuna çevrilir",
                          {"endpoint": EP, "collections": ["c"], "limit": 1}, "synthetic-get-next-1.json"))
    cases.append(answered("GET sonraki sayfa", {"endpoint": EP, "collections": ["c"], "limit": 1,
                          "next": {"method": "GET", "href": f"{EP.rsplit('/v1', 1)[0]}/v1/search?token=next%3Apage2"}}, "synthetic-get-next-2.json"))
    cases.append(answered("Sonuç yok", {"endpoint": EP, "collections": ["c"], "bbox": [0, 0, 1, 1], "limit": 1}, "synthetic-empty.json"))
    cases.append(answered("Sunucu 503 verir", {"endpoint": EP, "collections": ["down"]}, None,
                          error="Sunucu HTTP 503 dedi.", code="stac_request_failed",
                          message=None))
    cases.append(answered("Cevap JSON değil", {"endpoint": EP, "collections": ["html"]}, "not-json.txt"))
    cases.append(answered("Cevap öğe koleksiyonu değil", {"endpoint": EP, "collections": ["error"]}, "synthetic-no-features.json"))
    cases.append(answered("Tarih gün olarak, öğe kimlikleri ve kutu tamsayı: gövde sıralı ve sayılar noktasız",
                          {"endpoint": EP, "bbox": [36, 39, 37, 40], "ids": ["A"], "limit": 100, "maxCloudCover": 12.5, "datetime": "2024-07-13"},
                          "synthetic-empty.json"))
    return cases


MESSAGES = {}


def finish_search(cases):
    """Fills the answered cases' results from the recorded files."""
    out = []
    for c in cases:
        if "_req" not in c:
            out.append(c)
            continue
        doc = load(c["_file"]) if c["_file"].endswith(".json") else None
        step = {"op": c["_op"], "input": c["_input"]}
        if doc is None or read_page(c["_ep"], doc) is None:
            code = "stac_response_unreadable"
            why = "JSON okunamadı" if doc is None else "Cevapta öğe listesi (features) yok."
            step["result"] = {"status": "failed", "error": {"code": code, "message": "$any"}}
        else:
            items, returned, matched, nxt = read_page(c["_ep"], doc)
            items, dropped = within_cloud(items, c["_mc"])
            o = {"request": request_info(c["_req"]), "items": items, "returned": returned, "dropped": dropped}
            if matched is not None:
                o["matched"] = matched
            if nxt:
                o["next"] = nxt
            step["result"] = {"status": "completed", "output": o, "warnings": []}
        case = {"name": c["name"], "steps": [step]}
        if c.get("_note"):
            case["note"] = c["_note"]
        out.append(case)
    return out


def msg_endpoint(inp):
    t = inp["endpoint"].strip()
    if not t:
        return "STAC adresi boş."
    if any(c.isspace() or ord(c) < 32 for c in t):
        return "STAC adresi boşluk ya da denetim karakteri içeremez."
    m = re.match(r"^([A-Za-z]+)://", t)
    if not m or m.group(1).lower() not in ("http", "https"):
        return f"“{t}” bir http ya da https adresi değil (http:// ya da https:// ile başlamalı)."
    rest = t.split("://", 1)[1]
    if "?" in rest or "#" in rest:
        return "STAC adresi sorgu ya da parça içeremez; arama parametreleri komutun alanlarıdır."
    return "STAC adresi kullanıcı adı ya da parola içeremez; gizli değerler adrese yazılmaz."


def msg_bbox(inp):
    w, s, e, n = inp["bbox"]
    g = lambda x: short(x)
    if not (-180 <= w <= 180) or not (-180 <= e <= 180):
        return f"Boylam −180 ile 180 arasında olmalı (batı {g(w)}, doğu {g(e)})."
    if not (-90 <= s <= 90) or not (-90 <= n <= 90):
        return f"Enlem −90 ile 90 arasında olmalı (güney {g(s)}, kuzey {g(n)})."
    if w >= e:
        return "Batı doğudan küçük olmalı; 180° meridyenini aşan kutu iki aramaya bölünür."
    return "Güney kuzeyden büyük olamaz."


def msg_datetime(inp):
    t = inp["datetime"].strip()
    if "/" not in t:
        if DATE.match(t):
            return msg_datetime({"datetime": f"{t}/{t}"})
        return f"“{t}” bir RFC 3339 tarih ya da an değil (2024-07-01 ya da 2024-07-01T08:30:00Z)."
    a, _, b = t.partition("/")
    if "/" in b:
        return "Tarih aralığı tek bir “/” içerir: başlangıç/bitiş."
    for x, is_end in ((a, False), (b, True)):
        if end_of(x, is_end) is None:
            return f"“{x.strip()}” bir RFC 3339 tarih ya da an değil (2024-07-01 ya da 2024-07-01T08:30:00Z)."
    s, e = end_of(a, False), end_of(b, True)
    if s == "open" and e == "open":
        return "Tarih aralığının iki ucu da açık olamaz."
    return f"Başlangıç ({s[0]}) bitişten ({e[0]}) sonra."


def pct(x):
    return short(x)


MESSAGES.update({
    "invalid_stac_endpoint": msg_endpoint,
    "invalid_stac_bbox": msg_bbox,
    "invalid_stac_datetime": msg_datetime,
    "invalid_stac_cloud_cover": lambda i: f"En çok bulut oranı 0 ile 100 arasında olmalı; {pct(i['maxCloudCover'])} verildi.",
    "invalid_stac_limit": lambda i: f"Sayfa boyu 1 ile 100 arasında olmalı; {i['limit']} verildi.",
    "invalid_stac_collection": lambda i: ("Koleksiyon kimliği boş olmamalı ve denetim karakteri içermemeli." if any(not c.strip() for c in i.get("collections", [])) else "Öğe kimliği boş olmamalı ve denetim karakteri içermemeli."),
    "stac_next_foreign": lambda i: f"Sonraki sayfanın adresi (“{i['next']['href'].strip()}”) uç noktanın sunucusunda ({normalize_endpoint(i['endpoint'])}) değil.",
})


# --- prepare cases ----------------------------------------------------------------------------------------------

def item_of(file, index_in_page=0, catalog=EP):
    doc = load(file)
    return read_item(catalog, doc["features"][index_in_page])


def with_assets(item, keys):
    it = copy.deepcopy(item)
    it["assets"] = [a for a in it["assets"] if a["key"] in keys]
    return it


def https_copy(item):
    """The same item with http addresses (a catalog's http copy of s3 ones)."""
    it = copy.deepcopy(item)
    for a in it["assets"]:
        a["href"] = a["href"].replace("s3://usgs-landsat/", "https://usgs-landsat.s3.us-west-2.amazonaws.com/")
    return it


def prepare_case(name, inp, srid=32637, note=None, undo=True, op="execute", expect_extra=None):
    plan, code, path = prepare_plan(inp, srid)
    case = {"name": name, "steps": []}
    if note:
        case["note"] = note
    if srid != 32637:
        case["setup"] = setup_doc(srid)
    if plan is None:
        err = {"code": code, "message": PREPARE_MESSAGES[code](inp, srid), "path": path}
        if code == "invalid_revision":
            pass
        case["steps"].append({"op": op, "input": inp, "result": {"status": "failed", "error": err}})
        return case
    rec = plan["record"]
    warns = [{"code": c, "message": warning_message(c, p, x), "path": p} for c, p, x in plan["warnings"]]
    out = {"preparation": rec, "revision": "$current"}
    if op == "execute":
        out = copy.deepcopy(out)
        out["group"] = "$layer:1"
        for i, b in enumerate(out["preparation"]["bands"]):
            b["layer"] = f"$layer:{2 + i}"
            b["uid"] = f"$uidOf:{1 + i}"
        if "qa" in out["preparation"]:
            n = len(out["preparation"]["bands"])
            out["preparation"]["qa"]["layer"] = f"$layer:{2 + n}"
            out["preparation"]["qa"]["uid"] = f"$uidOf:{1 + n}"
    step = {"op": op, "input": inp, "result": {"status": "completed", "output": out, "warnings": warns}}
    if op == "execute":
        fields = {}
        rasters = plan["rasters"] + ([plan["qa"]] if plan["qa"] else [])
        for i, r in enumerate(rasters):
            f = {"kind": "raster", "affine": r["affine"], "width": r["width"], "height": r["height"], "bands": 1,
                 "sample": r["sample"], "srid": r["srid"], "attrs": r["attrs"], "style": r["style"]}
            if "file" in r:
                f["file"] = r["file"]
            else:
                f["url"] = r["url"]
            fields[str(1 + i)] = f
        step["expect"] = {"canUndo": True, "revision": "changed", "fields": fields}
    else:
        step["expect"] = {"canUndo": False, "revision": "same", "ids": []}
    case["steps"].append(step)
    if op == "execute" and undo:
        case["steps"].append({"op": "undo", "returns": "STAC verisini hazırla",
                              "expect": {"ids": [], "canUndo": False, "canRedo": True}})
    return case


def msg_prepare_not_found(inp, srid):
    asked = list(inp["assets"]) + ([inp["qa"]["asset"]] if inp.get("qa") else [])
    for k in sorted(inp.get("local") or {}):
        if k not in asked and all(k2 in [a["key"] for a in inp["item"]["assets"]] for k2 in asked):
            return f"Yerel kopyası verilen “{k}” eklenecek varlıklar arasında yok."
    keys = [a["key"] for a in inp["item"]["assets"]]
    var = ", ".join(keys) or "hiçbiri"
    for k in inp["assets"]:
        if k not in keys:
            return f"Öğede “{k}” anahtarlı bir varlık yok (var olanlar: {var})."
    return f"Öğede “{inp['qa']['asset']}” anahtarlı bir kalite varlığı yok (var olanlar: {var})."


def msg_invalid_item(inp, srid):
    item = inp["item"]
    if not item["id"].strip():
        return "Öğenin kimliği boş olmamalı ve denetim karakteri içermemeli."
    if normalize_endpoint(item["catalog"]) is None:
        return "Öğenin kataloğu (catalog) bir http ya da https adresi olmalı."
    seen = []
    for a in item["assets"]:
        if not a["key"].strip() or a["key"] in seen:
            return f"Varlık anahtarı boş, tekrarlı ya da denetim karakteri içeriyor (“{a['key']}”)."
        seen.append(a["key"])
    raise AssertionError


def _find(inp, key):
    return next(a for a in inp["item"]["assets"] if a["key"] == key)


def _first_bad(inp, pred):
    ks = list(dict.fromkeys(inp["assets"])) + ([inp["qa"]["asset"]] if inp.get("qa") else [])
    return next(_find(inp, k) for k in ks if pred(_find(inp, k)))


def _no_grid(inp, srid):
    a = _first_bad(inp, lambda a: "shape" not in a or "transform" not in a or "epsg" not in a)
    miss = [n for n, k in (("proj:shape", "shape"), ("proj:transform", "transform"), ("proj:epsg", "epsg")) if k not in a]
    return f"“{a['key']}” varlığının ızgarası bilinmiyor: {', '.join(miss)} yok (varlıkta ya da öğede)."


def _unsupported(inp, srid):
    a = _first_bad(inp, lambda a: (a.get("rasterBands") or [{}])[0].get("dataType") not in SAMPLES)
    shown = (a.get("rasterBands") or [{}])[0].get("dataType") or "yok"
    return (f"“{a['key']}” varlığının veri türü ({shown}) rasterde desteklenmiyor: uint8, int8, uint16, int16, uint32, int32, "
            "float32 ve float64 desteklenir.")


PREPARE_MESSAGES = {
    "invalid_stac_item": msg_invalid_item,
    "stac_assets_empty": lambda i, s: "Eklenecek varlık verilmedi.",
    "stac_too_many_assets": lambda i, s: f"Bir hazırlama en çok 32 varlık ekler; {len(set(i['assets']))} verildi.",
    "stac_asset_not_found": msg_prepare_not_found,
    "stac_qa_is_data": lambda i, s: f"“{i['qa']['asset']}” hem veri bandı hem kalite bandı olarak verilmiş.",
    "stac_href_not_http": lambda i, s: (lambda a: f"“{a['key']}” varlığının adresi (“{a['href']}”) http, https ya da s3 değil; raster bu adresten okunmaz. Kataloğun http adresi veren bir kopyasını kullanın ya da dosyayı indirip local ile verin.")(_first_bad(i, lambda a: not is_http(a["href"]) and not a["href"].strip().startswith("s3://"))),
    "stac_no_grid": _no_grid,
    "stac_multiband": lambda i, s: (lambda a: f"“{a['key']}” varlığının birden çok bandı var; her biri tek bantlı varlıklar eklenir (kırmızı, yakın kızılötesi …).")(_first_bad(i, lambda a: len(a.get("rasterBands", [])) > 1 or len(a.get("bands", [])) > 1)),
    "stac_unsupported_sample": _unsupported,
    "stac_qa_unknown": lambda i, s: (
        "Maske kuralı geçersiz: boş olmamalı; sınıf değerleri 0–65535, bit numaraları 0–31." if i["qa"].get("mask") is not None else
        f"“{i['qa']['asset']}” kalite varlığı için maske kuralı bilinmiyor: Sentinel-2'nin scl'si ve Landsat'ın qa_pixel'i dışında bir varlık için mask verin."),
    "crs_mismatch": lambda i, s: (lambda a: f"“{a['key']}” varlığının sistemi {system(a['epsg'])}, projeninki {system(s)}; raster çevrilmeden eklenmez: reproject ile yerel kopyası projenin sistemine çevrilir.")(_first_bad(i, lambda a: a["epsg"] != s)),
    "stac_local_missing": lambda i, s: (lambda a: f"“{a['key']}” varlığı ({system(a['epsg'])}) projenin sistemine çevrilecek; yerel kopyası gerekir: varlığı indirip local ile verin (masaüstü kendisi indirir).")(_first_bad(i, lambda a: a["epsg"] != s and a["key"] not in (i.get("local") or {}))),
    "invalid_file_path": lambda i, s: (lambda k: f"“{k}” varlığının yerel kopyasının yolu boş.")(next(k for k, v in sorted(i["local"].items()) if not v.strip())),
    "invalid_group_name": lambda i, s: "Grup adı boş olmamalı, en çok 120 harf olmalı ve denetim karakteri içermemeli.",
    "file_not_found": lambda i, s: f"Yerel kopya {i['local'][i['assets'][0]]} bulunamadı.",
    "import_unreadable": lambda i, s: "$any",
    "crs_unreached": lambda i, s: "$any",
}


def prepare_cases():
    cases = []
    s2 = item_of("earth-search-s2-page1.json", 0)
    s2_rn = with_assets(s2, ["red", "nir"])
    s2_all = with_assets(s2, ["red", "nir", "swir16", "scl", "visual", "blue"])
    s2_scl = with_assets(s2, ["red", "nir", "scl"])
    search = {"endpoint": EP, "bbox": [36.9, 39.7, 37.1, 39.8], "datetime": "2024-07-01T00:00:00Z/2024-07-20T23:59:59Z",
              "collections": ["sentinel-2-l2a"], "maxCloudCover": 20}
    base = {"item": s2_scl, "assets": ["red", "nir"], "qa": {"asset": "scl"}}
    cases.append(prepare_case("Sentinel-2: kırmızı ve yakın kızılötesi, SCL kalite bandı; yansıma ölçeği 0,0001 ve ötelemesi −0,1 (gerçek öğe)",
                              {**base, "search": search},
                              note="SCL 20 m, bantlar 10 m: kalite ızgarası uyarısı. Kayıt rasterlerin özniteliklerinde de durur; geri alma grubu, katmanları ve rasterleri birlikte alır."))
    cases.append(prepare_case("Plan hiçbir şey yazmaz", base, op="plan"))
    cases.append({"name": "validate: plan kurulur, uyarılar söylenir", "steps": [{"op": "validate", "input": base, "result": {"status": "completed", "output": None, "warnings": [{"code": "stac_qa_grid_differs", "message": warning_message("stac_qa_grid_differs", "qa.asset", "scl"), "path": "qa.asset"}]}}]})
    cases.append(prepare_case("Grup adı verilir, kalite bandı yok: uyarı yok", {"item": s2_rn, "assets": ["red", "nir"], "group": "  Temmuz görüntüsü  "}))
    cases.append(prepare_case("Tek bant", {"item": with_assets(s2, ["red"]), "assets": ["red"]}))
    cases.append(prepare_case("Bantların ızgarası aynı değil: 20 m'lik swir16 ile 10 m'lik red", {"item": s2_all, "assets": ["red", "swir16"]}))
    cases.append(prepare_case("Aynı varlık iki kez verilir: bir kez eklenir", {"item": s2_rn, "assets": ["red", "red", "nir"]}))
    cases.append(prepare_case("Kalite bandının maskesi verilir: sıralanır, tekrarsız; platformun kuralı değildir",
                              {**base, "qa": {"asset": "scl", "mask": {"kind": "values", "values": [9, 3, 8, 3]}}}))
    cases.append(prepare_case("Kalite bandının bitleri verilir", {**base, "qa": {"asset": "scl", "mask": {"kind": "bits", "bits": [4, 1]}}}))
    ls = item_of("earth-search-landsat.json", 0)
    cases.append(prepare_case("Landsat'ın s3:// adresleri: bölgenin HTTPS adresi, istek ödemeli kova uyarısı (gerçek öğe)",
                              {"item": with_assets(ls, ["red", "nir08"]), "assets": ["red", "nir08"]},
                              note="storage:region us-west-2 ve storage:requester_pays (docs/adr/0644 §1): raster AWS imzasıyla okunur."))
    lsh = with_assets(https_copy(ls), ["red", "nir08", "qa_pixel"])
    cases.append(prepare_case("Landsat: katalogun http kopyası, QA_PIXEL'in bit maskesi; ızgara öğenin özelliklerinden (gerçek öğe)",
                              {"item": lsh, "assets": ["red", "nir08"], "qa": {"asset": "qa_pixel"}},
                              note="Landsat Collection 2'nin ölçeği 2,75e-5, ötelemesi −0,2."))
    signed = item_of("synthetic-signed.json", 0)
    cases.append(prepare_case("İmzalı adresler: imza yazılmaz, uyarı", {"item": with_assets(signed, ["B04", "B08", "SCL"]), "assets": ["B04", "B08"], "qa": {"asset": "SCL"}}, srid=32636,
                              note="SAS imzası (sig, se, sp …) adresten atılır; kimlik katalog + koleksiyon + öğe + varlık anahtarıdır."))
    cases.append(prepare_case("Ret: proje sistemi başka (TUREF TM30)", base, srid=5254))
    # Access, local copies and reprojection (docs/adr/0644).
    st = item_of("synthetic-storage.json", 0)
    cases.append(prepare_case("Saklama uzantısı v2: varlığın storage:refs'i öğenin şemasında, bölge ve ödeyen oradan",
                              {"item": st, "assets": ["b04", "b08"]}))
    cases.append(prepare_case("Adında nokta olan kova yol biçiminde; bölgesiz s3 adresi us-east-1",
                              {"item": st, "assets": ["dotted", "noregion"]}))
    cases.append(prepare_case("Ret: gs:// adresi", {"item": st, "assets": ["b04", "gs"]}))
    cases.append(prepare_case("gs:// varlığın yerel kopyası verilir: adres yerine dosya",
                              {"item": st, "assets": ["gs"], "local": {"gs": "$fixture:raster/v1/reproject/utm36.tif"}}, op="plan"))
    cases.append(prepare_case("Ret: yerel kopyası verilen varlık eklenecekler arasında değil",
                              {"item": st, "assets": ["b04"], "local": {"b08": "/veri/b08.tif"}}))
    cases.append(prepare_case("Ret: yerel kopyanın yolu boş", {"item": st, "assets": ["b04"], "local": {"b04": "  "}}))
    pcs = copy.deepcopy(signed)
    pcs["catalog"] = "https://planetarycomputer.microsoft.com/api/stac/v1"
    cases.append(prepare_case("Planetary Computer: blob adresleri SAS ile yeniden imzalanır, imza uyarısı yok",
                              {"item": with_assets(pcs, ["B04", "B08", "SCL"]), "assets": ["B04", "B08"], "qa": {"asset": "SCL"}},
                              srid=32636))
    rp = item_of("synthetic-storage.json", 1)
    rbase = {"item": rp, "assets": ["red"], "qa": {"asset": "scl"}}
    cases.append(prepare_case("Ret: UTM 36 varlıkları TM36 projesinde, reproject yok", rbase, srid=5256))
    cases.append(prepare_case("Ret: reproject var, yerel kopya yok", {**rbase, "reproject": True}, srid=5256))
    local = {"red": "$tmp:stac-izdusum/utm36.tif", "scl": "$tmp:stac-izdusum/siniflar.tif"}
    case = prepare_case("Yeniden izdüşüm: yerel kopyalar TM36'ya çevrilir, rasterler çevrilen dosyaları okur, kayıtta yol ve ızgara",
                        {**rbase, "reproject": True, "local": local}, srid=5256,
                        note="Kopyalar önce $tmp'e alınır; çevrilenler yanlarına <ad>-epsg5256.tif olarak yazılır (cad.rasters.reproject'in kuralı, en yakın komşu).")
    case["steps"][:0] = [{"op": "copyFile", "from": "$fixture:raster/v1/reproject/utm36.tif", "to": "$tmp:stac-izdusum/utm36.tif"},
                         {"op": "copyFile", "from": "$fixture:raster/v1/reproject/siniflar.tif", "to": "$tmp:stac-izdusum/siniflar.tif"}]
    cases.append(case)
    cases.append(prepare_case("Ret: yerel kopya raster değil", {"item": rp, "assets": ["red"], "reproject": True,
                                                                 "local": {"red": "$fixture:raster/v1/reproject/notlar.txt"}},
                              srid=5256))
    wrong = copy.deepcopy(rp)
    for a in wrong["assets"]:
        a["epsg"] = 4326
    cases.append(prepare_case("Ret: sistem bildirmeyen kopya, öğenin sistemi (WGS 84) yanlış: projenin sistemine ulaşmaz",
                              {"item": wrong, "assets": ["red"], "reproject": True,
                               "local": {"red": "$fixture:raster/v1/reproject/sistemsiz.tif"}}, srid=5256))
    cases.append(prepare_case("Ret: yerel kopya dosyası yok", {**rbase, "reproject": True,
                                                                 "local": {"red": "/yok/utm36.tif", "scl": "/yok/siniflar.tif"}},
                              srid=5256))
    cases.append(prepare_case("Ret: varlık verilmedi", {"item": s2_rn, "assets": []}))
    cases.append(prepare_case("Ret: çok varlık", {"item": s2_rn, "assets": [f"b{i}" for i in range(33)]}))
    cases.append(prepare_case("Ret: veri varlığı öğede yok", {"item": s2_rn, "assets": ["red", "yok"]}))
    cases.append(prepare_case("Ret: kalite varlığı öğede yok", {"item": s2_rn, "assets": ["red"], "qa": {"asset": "scl"}}))
    cases.append(prepare_case("Ret: kalite varlığı veri bandı olarak da verilmiş", {"item": s2_scl, "assets": ["red", "scl"], "qa": {"asset": "scl"}}))
    cases.append(prepare_case("Ret: görsel (TCI) üç bantlı", {"item": s2_all, "assets": ["visual"]}))
    grids = item_of("synthetic-grids.json", 0)
    grids2 = item_of("synthetic-grids.json", 1)
    cases.append(prepare_case("Ret: ızgarası bilinmeyen varlık (proj:shape ve proj:transform yok)", {"item": grids2, "assets": ["nogrid"]}))
    cases.append(prepare_case("Ret: ızgarası yarım varlık (yalnız proj:shape var)", {"item": grids2, "assets": ["halfgrid"]}))
    cases.append(prepare_case("Ret: desteklenmeyen veri türü (cint16)", {"item": grids, "assets": ["complex"]}))
    cases.append(prepare_case("Ret: veri türü hiç yok (raster:bands yok)", {"item": grids, "assets": ["notype"]}))
    cases.append(prepare_case("Ret: üç bantlı varlık (eo:bands'te üç bant)", {"item": grids, "assets": ["rgb"]}))
    cases.append(prepare_case("Tek bantlı int16 DEM: ızgara öğenin özelliklerinden, ölçek 1", {"item": grids, "assets": ["dem"]}))
    cases.append(prepare_case("float32: nodata −9999, ölçek ve öteleme yok (1 ve 0), yansıma değil", {"item": grids, "assets": ["plain"]}))
    cases.append(prepare_case("Ret: kalite varlığının kuralı bilinmiyor", {**base, "qa": {"asset": "nir"}, "assets": ["red"]}))
    cases.append(prepare_case("Ret: boş maske", {**base, "qa": {"asset": "scl", "mask": {"kind": "values", "values": []}}}))
    cases.append(prepare_case("Ret: bit numarası 32", {**base, "qa": {"asset": "scl", "mask": {"kind": "bits", "bits": [32]}}}))
    bad = copy.deepcopy(s2_rn)
    bad["id"] = " "
    cases.append(prepare_case("Ret: öğenin kimliği boş", {"item": bad, "assets": ["red"]}))
    bad = copy.deepcopy(s2_rn)
    bad["catalog"] = "earth-search"
    cases.append(prepare_case("Ret: kataloğun adresi http değil", {"item": bad, "assets": ["red"]}))
    bad = copy.deepcopy(s2_rn)
    bad["assets"].append(copy.deepcopy(bad["assets"][0]))
    cases.append(prepare_case("Ret: öğede aynı anahtarlı iki varlık", {"item": bad, "assets": ["red"]}))
    cases.append(prepare_case("Ret: grup adı boş", {"item": s2_rn, "assets": ["red"], "group": "   "}))
    cases.append(prepare_case("Ret: grup adı 121 harf", {"item": s2_rn, "assets": ["red"], "group": "g" * 121}))
    # Differing scale and offset, and an item without a time or a cloud cover.
    diff = copy.deepcopy(s2_rn)
    next(a for a in diff["assets"] if a["key"] == "nir")["rasterBands"][0]["offset"] = 0.0
    cases.append(prepare_case("Bantların öteleme değeri farklı: uyarı, ortak yansıma yok", {"item": diff, "assets": ["red", "nir"]}))
    nocloud = copy.deepcopy(s2_rn)
    del nocloud["cloudCover"]
    del nocloud["datetime"]
    cases.append(prepare_case("Çekim zamanı ve bulut oranı yok: uyarı", {"item": nocloud, "assets": ["red"]}))
    # Revisions.
    rev_case = {"name": "Ret: beklenen sürüm geçersiz", "steps": [{"op": "execute", "input": {**base, "expectedRevision": "x"},
                "result": {"status": "failed", "error": {"code": "invalid_revision", "message": "$any", "path": "expectedRevision"}}}]}
    cases.append(rev_case)
    cases.append({"name": "Çakışma: çizim değişti", "steps": [{"op": "execute", "input": {**base, "expectedRevision": "7"},
                "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": "$any", "path": "expectedRevision", "revision": "$current"}}}]})
    return cases


# ---------------------------------------------------------------------------------------------------------------
# The core's cases.

def core_cases():
    endpoints = [
        "https://earth-search.aws.element84.com/v1", "HTTPS://Earth-Search.AWS.element84.com/v1/", " http://localhost:8080 ",
        "https://x.example", "https://x.example///", "ftp://x/y", "https://x/y?a=1", "https://x/y#a", "https://u:p@x/y",
        "https:///v1", "https://x /v1", "", "earth-search", "https://:8080/x",
    ]
    datetimes = [
        "2024-07-01/2024-07-20", "2024-07-13", "2024-07-13t08:29:15z", "2024-07-01/..", "/2024-07-01", "../2024-07-31T10:00:00Z",
        "2024-07-01T00:00:00.5+03:00/2024-07-01T00:00:00.25Z", "2024-07-01T01:00:00+03:00/2024-06-30T22:00:00Z",
        "2024-07-01T01:00:00+03:00/2024-06-30T21:59:59Z", "2024-02-29", "2023-02-29", "2024-13-01", "2024-07-01T25:00:00Z",
        "2024-07-01T00:00:00", "2024-07-20/2024-07-01", "../..", "", "a/b/c", "2024-07-01T00:00:00.1234567890Z",
        "2024-07-01T23:59:60Z", "2024-07-01T00:00:00+24:00", "1999-12-31T23:59:59-08:00/2000-01-01T07:59:59Z",
    ]
    hrefs = [
        "https://a.blob.core.windows.net/c/x.tif?st=2024&se=2024&sp=r&sv=2021&sr=c&sig=abc%3D",
        "https://b.s3.amazonaws.com/x.tif?X-Amz-Algorithm=AWS4&X-Amz-Credential=k&X-Amz-Signature=s",
        "https://u:p@h.example/x.tif", "https://h.example/x.tif?token=abc", "https://h.example/x.tif?versionId=3",
        "s3://bucket/x.TIF", "  https://h.example/a/b.tif ", "https://h.example/x.tif?a=1&Signature=z&Expires=9#frag",
        "https://h.example/x.tif#frag", "gs://b/x.tif", "https://h.example/x.tif?KEY-pair-id=1&policy=2",
        "https://h.example/x.tif?key=notproof",
    ]
    reqs = []
    for inp in [
        {"endpoint": EP, "bbox": [36.9, 39.7, 37.1, 39.8], "datetime": "2024-07-01/2024-07-20", "collections": ["sentinel-2-l2a"], "maxCloudCover": 20, "limit": 2},
        {"endpoint": EP},
        {"endpoint": EP, "ids": ["a", "b"], "collections": ["c1", "c2"], "limit": 100},
        {"endpoint": EP, "bbox": [36, 39, 37, 40], "maxCloudCover": 12.5, "datetime": "2024-07-13"},
        {"endpoint": "ftp://x"}, {"endpoint": EP, "bbox": [1, 2, 0, 3]}, {"endpoint": EP, "datetime": "x"},
        {"endpoint": EP, "maxCloudCover": 101}, {"endpoint": EP, "limit": 0},
        {"endpoint": EP, "collections": [" "]},
        {"endpoint": EP, "collections": ["c"], "limit": 2, "next": {"method": "POST", "href": EP + "/search", "body": {"next": "t", "limit": 2}, "merge": True}},
        {"endpoint": EP, "collections": ["c"], "limit": 2, "next": {"method": "POST", "href": EP + "/search", "body": {"next": "t", "limit": 3}}},
        {"endpoint": EP, "next": {"method": "GET", "href": EP + "/search?token=t"}},
        {"endpoint": EP, "next": {"method": "GET", "href": "https://evil.example/v1/search"}},
    ]:
        req, a, b = search_request(inp)
        reqs.append({"input": inp, "request": req} if req else {"input": inp, "code": a, "path": b})
    pages = []
    for f, ep in [("earth-search-s2-page1.json", EP), ("earth-search-s2-page2.json", EP), ("earth-search-landsat.json", EP),
                  ("synthetic-signed.json", EP), ("synthetic-cloudy.json", EP), ("synthetic-unified.json", EP),
                  ("synthetic-grids.json", EP), ("synthetic-storage.json", EP), ("synthetic-get-next-1.json", EP), ("synthetic-empty.json", EP),
                  ("synthetic-no-features.json", EP)]:
        r = read_page(ep, load(f))
        if r is None:
            pages.append({"file": f, "endpoint": ep, "unreadable": True})
        else:
            items, returned, matched, nxt = r
            e = {"file": f, "endpoint": ep, "returned": returned, "items": items}
            if matched is not None:
                e["matched"] = matched
            if nxt:
                e["next"] = nxt
            pages.append(e)
    return {
        "format": "kentos.stac-cases", "version": 1, "note": NOTE,
        "endpoints": [{"input": e, "normalized": normalize_endpoint(e)} for e in endpoints],
        "datetimes": [{"input": d, "normalized": normalize_datetime(d)} for d in datetimes],
        "hrefs": [{"input": h, "href": stable_href(h)[0], "signed": stable_href(h)[1]} for h in hrefs],
        "requests": reqs,
        "pages": pages,
    }


# ---------------------------------------------------------------------------------------------------------------

def build():
    files = synthetic_files()
    write_synthetic(files)  # the cases read them back
    index = Index()
    search = finish_search(search_cases(index))
    # The catalog's own reads (the window's GETs) are recorded too.
    for url, f in ((EP, "earth-search-root.json"), (EP + "/collections", "earth-search-collections.json")):
        index.add({"method": "GET", "url": url}, file=f)
    prepare = prepare_cases()
    header = lambda cmd, title: {"format": "kentos.command-cases", "version": 1, "command": cmd, "commandVersion": 1,
                                 "title": title, "note": NOTE, "setup": setup_doc()}
    out = {
        "fixtures/stac/v1/cases.json": core_cases(),
        "fixtures/commands/v1/cad.stac.search.json": {**header("cad.stac.search", "STAC'ta ara"), "cases": search},
        "fixtures/commands/v1/cad.stac.prepare.json": {**header("cad.stac.prepare", "STAC verisini hazırla"), "cases": prepare},
        "fixtures/services/v1/stac/index.json": {"note": NOTE, "requests": index.entries},
    }
    return files, out


def dump(doc):
    return json.dumps(doc, indent=1, ensure_ascii=False) + "\n"


def main():
    check = "--check" in sys.argv
    if check:
        # The synthetic files are compared before they are (re)written in memory only.
        files = synthetic_files()
        problems = []
        for name, doc in files.items():
            p = REC / name
            if not p.exists() or p.read_text() != dump(doc):
                problems.append(str(p.relative_to(ROOT)))
        # The cases read the files on disk; rebuild with those.
        index = Index()
        search = finish_search(search_cases(index))
        for url, f in ((EP, "earth-search-root.json"), (EP + "/collections", "earth-search-collections.json")):
            index.add({"method": "GET", "url": url}, file=f)
        prepare = prepare_cases()
        header = lambda cmd, title: {"format": "kentos.command-cases", "version": 1, "command": cmd, "commandVersion": 1,
                                     "title": title, "note": NOTE, "setup": setup_doc()}
        out = {
            "fixtures/stac/v1/cases.json": core_cases(),
            "fixtures/commands/v1/cad.stac.search.json": {**header("cad.stac.search", "STAC'ta ara"), "cases": search},
            "fixtures/commands/v1/cad.stac.prepare.json": {**header("cad.stac.prepare", "STAC verisini hazırla"), "cases": prepare},
            "fixtures/services/v1/stac/index.json": {"note": NOTE, "requests": index.entries},
        }
        for rel, doc in out.items():
            p = ROOT / rel
            if not p.exists() or p.read_text() != dump(doc):
                problems.append(rel)
        if problems:
            print("out of date:\n  " + "\n  ".join(problems))
            sys.exit(1)
        print(f"ok: {len(search)} search and {len(prepare)} prepare cases, {len(index.entries)} recorded requests")
        return
    files, out = build()
    for rel, doc in out.items():
        p = ROOT / rel
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(dump(doc))
    print(f"written: {len(out)} files, {len(files)} synthetic answers")


if __name__ == "__main__":
    main()
