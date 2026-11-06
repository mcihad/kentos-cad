"""The coordinate chain of `cad.crs.transform` (docs/adr/1230), written from the ADR's rules and PROJ's database.

No KentOS code. Imported by the scripts that write `cad.crs.transform`'s shared cases (crs_vertical_cases.py,
geoid_cases.py, crs_chain_cases.py): each completed case gets its `chain` and, when the chain is limited, the
`chain_limited` warning. The sides' family, unit, EPSG axis order and datum ensemble accuracy are read from PROJ's
database (pyproj) and checked against the registry the command reads (fixtures/crs/v1/registry.json); the verdict's
reasons follow the ADR's §3 in its order; the budget is §4's.
"""

import json

from pyproj import CRS

# docs/adr/1230 §4: the engine against PROJ, the target's horizontal units (metres; degrees) and metres of height.
BUDGET_METRES, BUDGET_DEGREES, BUDGET_HEIGHT = 1e-6, 1e-9, 1e-6

REASONS = ["points_outside", "accuracy_unknown", "height_accuracy_unknown", "heights_unchanged", "epoch_unknown",
           "datum_ensemble", "unofficial_operation"]
TEXT = {
    "points_outside": "bazı noktalar izdüşümün ya da ızgaranın kapsamı dışında",
    "accuracy_unknown": "bir adımın yatay doğruluğu bilinmiyor",
    "height_accuracy_unknown": "kotların doğruluğu bilinmiyor (geoit ya da hız modeli doğruluk söylemiyor)",
    "heights_unchanged": "kotların ne olduğu söylenmedi, değişmeden kaldı",
    "epoch_unknown": "yalnız bir yanın epoku söylendi; öbürünün aynı epokta olduğu varsayıldı",
    "datum_ensemble": "WGS 84 bir datum topluluğu: EPSG'ye göre 2 m'den iyi olamaz",
    "unofficial_operation": "ED50'nin EPSG işlemi resmî dönüşüm değil",
}
VERTICAL_NAMES = {5775: ("Antalya height", "m"), 3855: ("EGM2008 height", "m"), 5773: ("EGM96 height", "m"),
                  5714: ("MSL height", "m"), 8050: ("MSL height (ft)", "ft"), 5735: ("Black Sea height", "m")}


def _proj_side(srid, entry):
    """Family, unit, EPSG's axis order and the ensemble's accuracy of a registry system, from PROJ's database."""
    crs = CRS.from_epsg(srid)
    axes = [a.direction for a in crs.axis_info[:2]]
    units = {a.unit_name for a in crs.axis_info[:2]}
    if crs.is_geographic:
        family, order = "geographic", {("north", "east"): "latlon"}[tuple(axes)]
    else:
        method = crs.coordinate_operation.method_name
        family = {"Transverse Mercator": "transverseMercator",
                  "Popular Visualisation Pseudo Mercator": "pseudoMercator"}[method]
        order = {("north", "east"): "ne", ("east", "north"): "en"}[tuple(axes)]
    unit = {"degree": "deg", "metre": "m"}[units.pop()]
    assert not units
    assert order == entry["axisOrder"], (srid, order, entry["axisOrder"])
    # PROJJSON says a datum ensemble and its accuracy (EPSG's) where the system rests on one.
    j = json.loads(crs.to_json())
    ens = j.get("datum_ensemble") or j.get("base_crs", {}).get("datum_ensemble")
    ensemble = float(ens["accuracy"]) if ens else None
    return family, unit, order, ensemble


def vertical(h):
    if not h or h["kind"] == "unknown":
        return None
    v = {"kind": h["kind"]}
    if h["kind"] == "ellipsoidal":
        v.update({"name": "elipsoit yüksekliği", "unit": "m"})
        return v
    code = h.get("verticalSrid")
    if code is not None:
        v["srid"] = code
    if code in VERTICAL_NAMES:
        v["name"], v["unit"] = VERTICAL_NAMES[code]
    else:
        v["name"], v["unit"] = "ortometrik, düşey sistemi belirtilmemiş" + (
            f", geoit {h['geoid']['name']}" if h.get("geoid") else ""), "m"
    if h.get("geoid"):
        v["geoid"] = h["geoid"]
    return v


def side(entry, heights, epoch, source, definition=None):
    """A side of the chain: a registry `entry` (its srid), or the project's `definition` (name, family, datum)."""
    if definition:
        name, family, datum = definition
        s = {"name": name, "family": family, "datum": datum, "unit": "deg" if family == "geographic" else "m",
             "axes": "longitude,latitude" if family == "geographic" else "east,north"}
        ensemble = 2.0 if datum == "WGS84" else None
    else:
        family, unit, order, ensemble = _proj_side(entry["srid"], entry)
        s = {"srid": entry["srid"], "name": entry["name"], "family": family, "datum": entry["datum"], "unit": unit,
             "axes": "longitude,latitude" if family == "geographic" else "east,north", "epsgAxes": order}
    if ensemble is not None:
        s["ensembleAccuracy"] = ensemble
    v = vertical(heights)
    if v:
        s["vertical"] = v
    if epoch is not None:
        s["epoch"] = epoch
    s["epochSource"] = source
    return s


def basis(step):
    if step["kind"] in ("conversion", "unit"):
        return "exact"
    if step["kind"] in ("geoid", "velocity") or step.get("accuracy") is None:
        return "unknown"
    return "epsg" if step["name"].startswith("EPSG:") else "project"


def add(case, source, target, stated_epochs, grids=()):
    """Adds the chain to a completed `case`: `source` and `target` from [`side`], `stated_epochs` whether each side
    stated an epoch (the call or the project), `grids` the chain's grids (kind, name, sha256, extent[, accuracy]) in
    the record's order."""
    out = case["result"]["output"]
    rec, report = out["operation"], out["report"]
    given_z = any("z" in p for p in case["input"]["points"])
    ensemble = "ensembleAccuracy" in source or "ensembleAccuracy" in target
    found = {
        "points_outside": bool(out["unreached"]),
        "accuracy_unknown": rec.get("accuracy") is None,
        "height_accuracy_unknown": given_z and rec["heights"] in ("ellipsoidal", "geoid")
        and rec.get("heightAccuracy") is None,
        "heights_unchanged": rec["heights"] == "unchanged",
        "epoch_unknown": stated_epochs[0] != stated_epochs[1],
        "datum_ensemble": rec["id"] != "" and ensemble,
        "unofficial_operation": rec["unofficial"],
    }
    reasons = [r for r in REASONS if found[r]]
    geographic = target["family"] == "geographic"
    out["chain"] = {"source": source, "target": target, "bases": [basis(s) for s in rec["steps"]],
                    "grids": list(grids), "status": "limited" if reasons else "complete", "reasons": reasons,
                    "budget": {"horizontal": BUDGET_DEGREES if geographic else BUDGET_METRES,
                               "vertical": BUDGET_HEIGHT}}
    if reasons:
        case["result"]["warnings"].append({
            "code": "chain_limited",
            "message": "Dönüşüm zinciri sınırlı, hassas dönüşüm sayılmaz: " + "; ".join(TEXT[r] for r in reasons) + ".",
            "path": "to"})
    assert report is not None
    return case


def epoch_sources(se, de, own_from, own_to, project_epoch=None):
    """The sides' epochs and their sources as the ADR's §1 has them: the call's, else the project's for the project's
    side; the target takes the source's when it has none (carried)."""
    fe, fs = (se, "given") if se is not None else ((project_epoch, "project") if own_from and project_epoch is not None
                                                     else (None, "none"))
    te, ts = (de, "given") if de is not None else ((project_epoch, "project") if own_to and project_epoch is not None
                                                     else (None, "none"))
    stated = (fe is not None, te is not None)
    if te is None and fe is not None:
        te, ts = fe, "carried"
    return (fe, fs), (te, ts), stated
