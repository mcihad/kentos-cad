#!/usr/bin/env python3
"""docs/adr/0602's aggregates against QGIS 4.2's own, without KentOS code.

QGIS evaluates q1, q3, iqr, range, majority, minority, count_missing, min_length, max_length and concatenate_unique on a
memory layer; the same values are computed here from the ADR's rule (Tukey's hinges: each half ⌈n/2⌉ long; ties of
majority and minority: the least value; lengths in characters; each value once in the order first seen). The rule the
language's own reference writes (scripts/fixtures/expression_extras.py, fixtures/expression/v2/extras.json) is the same.

  QT_QPA_PLATFORM=offscreen /usr/bin/python3 scripts/fixtures/aggregate_qgis_cases.py --check
"""

import os
import sys
from decimal import Decimal

SETS = [
    [3, 1, 4, 1, 5, 9, 2, 6],
    [3, 1, 4, 1, 5, 2, 6],
    [3, 1, 4, 1, 5, 2],
    [4],
    [1, 1, 2],
    [2.5, 0.1, 0.2, 7.25, 7.25, 0.1, 3],
    [10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110],
]
TEXTS = [
    ["a", "bb", "a", "ccc", "a", "dddd", "bb", "e"],
    ["Arsa", "Tarla", "Bahçe", "Tarla", "Arsa", "Yol"],
    ["x", "y", "z"],
]


def mid(ds):
    n = len(ds)
    return ds[n // 2] if n % 2 else (ds[n // 2 - 1] + ds[n // 2]) / 2


def hinges(xs):
    ds = sorted(Decimal(repr(x)) for x in xs)
    half = (len(ds) + 1) // 2
    return float(mid(ds[:half])), float(mid(ds[len(ds) - half:]))


def frequent(vs, most):
    counts = {}
    for v in vs:
        counts[v] = counts.get(v, 0) + 1
    sign = -1 if most else 1
    return min(counts, key=lambda v: (sign * counts[v], v))


def rule(values, numeric):
    out = {}
    if numeric:
        q1, q3 = hinges(values)
        out["q1"], out["q3"], out["iqr"] = q1, q3, q3 - q1
        out["range"] = float(max(values) - min(values))
    out["majority"] = frequent(values, True)
    out["minority"] = frequent(values, False)
    if not numeric:
        lengths = [len(v) for v in values]
        out["min_length"], out["max_length"] = min(lengths), max(lengths)
        seen = []
        for v in values:
            if v not in seen:
                seen.append(v)
        out["concatenate_unique"] = ",".join(seen)
    return out


def qgis_values(values, numeric):
    from qgis.core import QgsExpression, QgsExpressionContext, QgsExpressionContextUtils, QgsFeature, QgsGeometry
    from qgis.core import QgsPointXY, QgsProject, QgsVectorLayer

    kind = "double" if numeric else "string"
    layer = QgsVectorLayer(f"Point?field=v:{kind}", "k", "memory")
    feats = []
    for v in values:
        f = QgsFeature(layer.fields())
        f.setAttributes([v])
        f.setGeometry(QgsGeometry.fromPointXY(QgsPointXY(0, 0)))
        feats.append(f)
    layer.dataProvider().addFeatures(feats)
    QgsProject.instance().addMapLayer(layer)
    context = QgsExpressionContext()
    context.appendScopes(QgsExpressionContextUtils.globalProjectLayerScopes(layer))
    context.setFeature(next(layer.getFeatures()))
    names = ["q1", "q3", "iqr", "range", "majority", "minority"] if numeric else \
        ["majority", "minority", "min_length", "max_length"]
    out = {}
    for n in names:
        out[n] = QgsExpression(f'{n}("v")').evaluate(context)
    if not numeric:
        out["concatenate_unique"] = QgsExpression("concatenate_unique(\"v\", concatenator:=',')").evaluate(context)
    QgsProject.instance().removeMapLayer(layer.id())
    return out


def main():
    os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")
    sys.path.insert(0, "/usr/share/qgis/python")
    from qgis.core import QgsApplication
    from qgis.PyQt.QtCore import QLocale

    QLocale.setDefault(QLocale(QLocale.Language.C))
    app = QgsApplication([], False)
    app.initQgis()
    wrong = []
    count = 0
    for values, numeric in [(s, True) for s in SETS] + [(t, False) for t in TEXTS]:
        want = rule(values, numeric)
        said = qgis_values(values, numeric)
        for name, w in want.items():
            q = said.get(name)
            count += 1
            same = abs(float(q) - float(w)) < 1e-12 if numeric or isinstance(w, int) else q == w
            if not same:
                wrong.append(f"{name}({values}): QGIS {q!r}, kural {w!r}")
    if wrong:
        print("\n".join(wrong))
        return 1
    print(f"Toplamanın ekleri QGIS 4.2'yle tutarlı: {count} değer")
    return 0


if __name__ == "__main__":
    sys.exit(main())
