#!/usr/bin/env python3
"""Grades as KentOS holds them (docs/adr/0977): rise over run, −0.025 for −2.5 %.

The reference scripts keep their own model of a road in percent, as the engineer writes it (KGM, LandXML); KentOS's
sources hold grades. This module turns what a script writes into KentOS's form at the very end: a template's cross
slopes and benches' fall, a superelevation schedule's normal slopes, rates and rows, a culvert's fall, a pad's grade, a
volume's benched ground slope; in a command's input and output the same fields and the few answers that carry them.
Nothing else is touched: a side slope is horizontal per vertical, a profile's grades and a relative gradient were
ratios already, A stays percent (K = L/|A|), a share is a share. Each value is divided by 100, the same IEEE division
KentOS makes when it reads an older file, so the two agree to the bit.

    import road_grades
    data = road_grades.fixture(data)                # a command's cases (fixtures/commands/v1)
    road_grades.settings(doc["settings"]["road"])   # a drawing's road settings (fixtures/kcad/v2, interaction)
"""

import json


def grade(v):
    """A percent value as a grade."""
    return v / 100 if isinstance(v, (int, float)) and not isinstance(v, bool) else v


def _scale(obj, key):
    if isinstance(obj, dict) and key in obj:
        obj[key] = grade(obj[key])


def _pairs(obj, key):
    """A list of (station, percent) pairs: the second of each."""
    if isinstance(obj, dict) and isinstance(obj.get(key), list):
        obj[key] = [[p[0], grade(p[1])] + list(p[2:]) if isinstance(p, list) and len(p) >= 2 else p for p in obj[key]]


def side(s):
    """A template side: its components' cross slopes and its benches' fall."""
    if not isinstance(s, dict):
        return
    for c in s.get("components", []) or []:
        _scale(c, "slope")
    for b in ("bench", "fillBench"):
        _scale(s.get(b), "fall")


def template(t):
    if not isinstance(t, dict):
        return
    side(t.get("left"))
    side(t.get("right"))


def method(m):
    if not isinstance(m, dict):
        return
    for c in m.get("curves", []) or []:
        _scale(c, "rate")
    for r in m.get("rows", []) or []:
        _scale(r, "left")
        _scale(r, "right")


def superelevation(d):
    """A schedule's definition, or its report (normal slopes and (station, slope) break points)."""
    if not isinstance(d, dict):
        return
    _scale(d, "normalLeft")
    _scale(d, "normalRight")
    method(d.get("method"))


def report(d):
    """A schedule's report: its normal slopes and its sides' (station, slope) break points."""
    if not isinstance(d, dict):
        return
    _scale(d, "normalLeft")
    _scale(d, "normalRight")
    _pairs(d, "left")
    _pairs(d, "right")


def settings(road):
    """A drawing's road settings, in place; returned too."""
    if not isinstance(road, dict):
        return road
    for t in road.get("templates", []) or []:
        template(t)
    for d in road.get("superelevations", []) or []:
        superelevation(d)
    for c in road.get("culverts", []) or []:
        _scale(c, "slope")
    for p in road.get("platforms", []) or []:
        _scale(p, "grade")
    for v in road.get("volumes", []) or []:
        _scale(v.get("bench"), "slope")
    return road


def _input(command, x):
    if not isinstance(x, dict):
        return
    if command == "road.templates.edit":
        template(x)
    elif command == "road.superelevations.edit":
        superelevation(x)
    elif command in ("road.culverts.edit",):
        _scale(x, "slope")
    elif command in ("road.platforms.edit", "road.routes.find", "road.routes.draw"):
        _scale(x, "grade")
    elif command == "road.junctions.tie":
        if isinstance(x.get("crossfall"), list):
            x["crossfall"] = [grade(v) for v in x["crossfall"]]
        _scale(x, "maxGradeChange")
    elif command == "road.volumes.edit":
        _scale(x.get("bench"), "slope")


def _output(command, o):
    if not isinstance(o, dict):
        return
    if command == "road.templates.edit":
        template(o.get("template"))
    elif command in ("road.superelevations.edit", "road.superelevations.query"):
        report(o.get("superelevation"))
        for d in o.get("superelevations", []) or []:
            report(d)
        for a in o.get("at", []) or []:
            _scale(a, "left")
            _scale(a, "right")
    elif command == "road.superelevations.transitions":
        for r in o.get("rows", []) or []:
            _scale(r, "rate")
    elif command == "road.routes.find":
        _scale(o, "steepest")
    elif command == "road.routes.draw":
        _scale(o.get("route"), "steepest")
    elif command == "road.junctions.tie":
        for k in ("grades", "changes"):
            if isinstance(o.get(k), list):
                o[k] = [grade(v) for v in o[k]]
        for t in o.get("ties", []) or []:
            _scale(t, "crossfall")


def fixture(data):
    """A command's cases (fixtures/commands/v1), converted on a copy: a script's cases often share one template or
    schedule dict, which converted in place would be divided once per sharer."""
    data = json.loads(json.dumps(data))
    command = data.get("command", "")
    setup = data.get("setup")
    if isinstance(setup, dict):
        settings((setup.get("settings") or {}).get("road"))
    for case in data.get("cases", []):
        if isinstance(case.get("setup"), dict):
            settings((case["setup"].get("settings") or {}).get("road"))
        for step in case.get("steps", []):
            c = step.get("command") or command
            _input(c, step.get("input"))
            result = step.get("result")
            if isinstance(result, dict):
                _output(c, result.get("output"))
            expect = step.get("expect")
            if isinstance(expect, dict):
                settings(expect.get("road"))
    return data
