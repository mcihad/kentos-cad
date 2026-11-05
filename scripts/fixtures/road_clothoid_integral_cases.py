#!/usr/bin/env python3
"""The clothoid integral's shared cases (docs/adr/0969, ROAD-ALIGN-04): ∫₀ˢ (cos θ, sin θ) dt for
θ(t) = θ0 + k0·t + c·t²/2 by the Fresnel integrals at 60 digits, no KentOS code and no quadrature.

Completing the square, θ = α + (c/2)(t + β)² with β = k0/c and α = θ0 − k0²/(2c). With v = √(|c|/π)(t + β):
∫ e^{iθ} dt = √(π/|c|)·e^{iα}·[C(v) ± i·S(v)] between the ends, + where c > 0, − where c < 0 (mpmath's C and S are
∫₀ˣ cos(πt²/2) and sin). Each value is also checked against mpmath's own quadrature at 60 digits; a disagreement
stops the script.

    uv run --no-project --python 3.11 --with mpmath python3 scripts/fixtures/road_clothoid_integral_cases.py
    ... --check   # fails when fixtures/road/v1/clothoid-integrals.json is not what this writes
"""

import json
import sys
from pathlib import Path

import mpmath as mp

mp.mp.dps = 60
OUT = Path(__file__).resolve().parents[2] / "fixtures" / "road" / "v1" / "clothoid-integrals.json"


def fresnel(theta0, k0, c, s):
    theta0, k0, c, s = (mp.mpf(v) for v in (theta0, k0, c, s))
    beta = k0 / c
    alpha = theta0 - k0 * k0 / (2 * c)
    q = mp.sqrt(abs(c) / mp.pi)
    v0, v1 = q * beta, q * (s + beta)
    sign = 1 if c > 0 else -1
    e = (mp.fresnelc(v1) - mp.fresnelc(v0)) + sign * 1j * (mp.fresnels(v1) - mp.fresnels(v0))
    z = mp.sqrt(mp.pi / abs(c)) * mp.expjpi(alpha / mp.pi) * e
    return z.real, z.imag


def quad(theta0, k0, c, s):
    theta0, k0, c, s = (mp.mpf(v) for v in (theta0, k0, c, s))
    th = lambda t: theta0 + k0 * t + c * t * t / 2
    turn = abs(k0) * abs(s) + abs(c) * s * s / 2
    cuts = mp.linspace(0, s, max(2, int(turn / mp.mpf("0.5")) + 2))
    return mp.quad(lambda t: mp.cos(th(t)), cuts), mp.quad(lambda t: mp.sin(th(t)), cuts)


def spiral(name, theta0, r0, r1, length, fractions=(0.25, 0.5, 1.0)):
    """A clothoid of `length` from radius r0 to r1 (None: straight; negative: turning right)."""
    k0 = 0 if r0 is None else 1 / mp.mpf(r0)
    k1 = 0 if r1 is None else 1 / mp.mpf(r1)
    c = (k1 - k0) / mp.mpf(length)
    rows = []
    for f in fractions:
        s = mp.mpf(length) * mp.mpf(f)
        x, y = fresnel(theta0, k0, c, s)
        qx, qy = quad(theta0, k0, c, s)
        assert abs(x - qx) < mp.mpf("1e-40") and abs(y - qy) < mp.mpf("1e-40"), (name, f, x - qx, y - qy)
        rows.append({"s": float(s), "x": float(x), "y": float(y)})
    turn = abs(k0) * length + abs(c) * length * length / 2
    return {"name": name, "theta0": float(theta0), "k0": float(k0), "c": float(c), "turn": float(turn), "at": rows}


def main():
    cases = [
        spiral("Düzden R 250'ye, L 80", 0, None, 250, 80),
        spiral("R 250'den düze, L 80", 0.7, 250, None, 80),
        spiral("Sağa, düzden R 400'e, L 120", 4.0, None, -400, 120),
        spiral("Uzun ve yumuşak: R 5000, L 500", -2.5, None, 5000, 500),
        spiral("Kısa ve keskin: R 30, L 10", 1.0, None, 30, 10),
        spiral("Ove: R 600'den R 300'e, L 150", 0.3, 600, 300, 150),
        spiral("Ove, genişleyen: R 300'den R 900'e, L 200", 5.9, 300, 900, 200),
        spiral("S klotoid: R 500 solda, R 500 sağda, L 240", 2.0, 500, -500, 240),
        spiral("Neredeyse daire: R 1000'den R 1000,1'e, L 50", 0.0, 1000, 1000.1, 50),
        spiral("Saç tokası: düzden R 15'e, L 300 (10 rad)", 0.0, None, 15, 300, (0.1, 0.5, 0.9, 1.0)),
        spiral("Çok uzun: R 2000, L 3000", 1.2, None, 2000, 3000, (0.2, 0.6, 1.0)),
        spiral("Uzak başlangıç doğrultusu: θ0 = 100 rad", 100.0, None, 250, 80),
    ]
    out = {"format": "kentos-cases", "version": 1,
           "note": "∫₀ˢ (cos θ, sin θ) dt, θ = θ0 + k0·t + c·t²/2: Fresnel integrals at 60 digits, checked against "
                   "mpmath's quadrature (docs/adr/0969); no KentOS code.",
           "cases": cases}
    text = json.dumps(out, ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv:
        sys.exit(0 if OUT.read_text("utf-8") == text else "clothoid-integrals.json eski: yeniden üretin")
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT} ({len(cases)} durum)")


if __name__ == "__main__":
    main()
