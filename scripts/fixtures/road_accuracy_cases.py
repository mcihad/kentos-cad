#!/usr/bin/env python3
"""A surface's vertical accuracy from check points, shared cases (docs/adr/0945, plan/road.md Y39.1; ROAD-SURVEY-05),
from an independent reference: numpy's median and quantiles (linear interpolation, its default), scipy's χ² quantile,
skewness and kurtosis (biased moments, Fisher's excess); the NMAD as Höhle and Höhle define it. No KentOS code.

    uv run --no-project --python 3.11 --with numpy --with scipy python3 scripts/fixtures/road_accuracy_cases.py
    ... --check   # fails when fixtures/road/v1/accuracy.json is not what this writes
"""

import json
import sys
from pathlib import Path

import numpy as np
from scipy import stats

OUT = Path(__file__).resolve().parents[2] / "fixtures" / "road" / "v1" / "accuracy.json"
NMAD, FAR = 1.4826, 3.0


def accuracy(d, survey=None):
    d = np.asarray(d, dtype=float)
    n = len(d)
    rmse = float(np.sqrt(np.mean(d * d)))
    ss = n * rmse * rmse
    med = float(np.median(d))
    nmad = NMAD * float(np.median(np.abs(d - med)))
    m2 = float(np.mean((d - d.mean()) ** 2))
    # Equal differences have no spread: their mean's rounding leaves m₂ ~1e-36, not 0; within 1e-12 of RMSE² it is 0.
    spread = m2 > 1e-12 * rmse * rmse
    skew = float(stats.skew(d, bias=True)) if spread else 0.0
    kurt = float(stats.kurtosis(d, fisher=True, bias=True)) if spread else 0.0
    out = {"count": n, "mean": float(d.mean()), "rmse": rmse,
           "rmseLow": float(np.sqrt(ss / stats.chi2.ppf(0.975, n))),
           "rmseHigh": float(np.sqrt(ss / stats.chi2.ppf(0.025, n))),
           "median": med, "nmad": nmad,
           "q68": float(np.quantile(np.abs(d), 0.683)), "q95": float(np.quantile(np.abs(d), 0.95)),
           "skewness": skew, "kurtosis": kurt,
           "far": [i for i, x in enumerate(d) if nmad > 0 and abs(x - med) > FAR * nmad]}
    if survey is not None:
        out["combined"] = float(np.sqrt(rmse * rmse + survey * survey))
    return out


def cases():
    rng = np.random.default_rng(39)
    out = []

    def case(name, d, survey=None):
        d = [round(float(x), 4) for x in d]
        out.append({"name": name, "differences": d, **({"survey": survey} if survey is not None else {}),
                    "expected": accuracy(d, survey)})

    case("Normal, σ 5 cm, 40 nokta", rng.normal(0, 0.05, 40))
    case("Normal, 2 cm kayma", rng.normal(0.02, 0.05, 40))
    case("Bitkili zemin: sağa çarpık", rng.exponential(0.12, 60) - 0.02)
    case("Normal ve üç aykırı", list(rng.normal(0, 0.04, 37)) + [0.6, -0.45, 0.9])
    case("Düzgün dağılım", rng.uniform(-0.1, 0.1, 50))
    case("İki nokta", [0.03, -0.01])
    case("Üç nokta", [0.05, 0.02, -0.04])
    case("Hepsi aynı", [0.02] * 10)
    case("Yarısı sıfır", [0.0] * 6 + [0.1, -0.1, 0.05, -0.05, 0.2, -0.2])
    case("Kontrol noktalarının kendi doğruluğu 2 cm", rng.normal(0, 0.05, 30), survey=0.02)
    case("Büyük örnek, 1000 nokta", rng.normal(0, 0.03, 1000))
    case("Tek yönlü sistematik", 0.1 + rng.normal(0, 0.01, 25))
    case("Ağır kuyruklu (t, 3 serbestlik)", 0.05 * rng.standard_t(3, 80))
    case("Tam sayılar", [1, 2, 3, 4, 5, -1, -2, -3])
    case("Otuz nokta sınırında", rng.normal(0, 0.08, 30), survey=0.0)
    return out


def chi2_cases():
    return [{"p": p, "k": k, "x": float(stats.chi2.ppf(p, k))}
            for k in (1, 2, 3, 5, 10, 20, 30, 40, 60, 100, 200, 1000) for p in (0.025, 0.5, 0.975)]


def main():
    data = {"cases": cases(), "chi2": chi2_cases()}
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text() != text:
            sys.exit(f"{OUT} is not what scripts/fixtures/road_accuracy_cases.py writes")
        print(f"{OUT}: {len(data['cases'])} cases")
        return
    OUT.write_text(text)
    print(f"wrote {OUT}: {len(data['cases'])} cases, {len(data['chi2'])} χ² quantiles")
    for c in data["cases"]:
        e = c["expected"]
        print(f"  {c['name'][:42]:42} RMSE {e['rmse']:.4f} [{e['rmseLow']:.4f}, {e['rmseHigh']:.4f}] medyan {e['median']:.4f} "
              f"NMAD {e['nmad']:.4f} q68 {e['q68']:.4f} q95 {e['q95']:.4f} çarpıklık {e['skewness']:.2f} aykırı {len(e['far'])}")


if __name__ == "__main__":
    main()
