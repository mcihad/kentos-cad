"""K₀'s Chebyshev series (docs/adr/0232 §8): e^x·√x·K₀(x) for x ≥ 2 as a
series of Chebyshev polynomials in u = 4/x − 1 (u in (−1, 1]; at u = −1 the
limit √(π/2)), its coefficients from mpmath's besselk at 60 digits on 96
Chebyshev nodes, the first 30 kept (the rest are below 10⁻²⁰). Spline's
basis evaluates it by Clenshaw's recurrence.

    python3 scripts/fixtures/bessel_k0.py           # prints the Rust table
    python3 scripts/fixtures/bessel_k0.py --check   # compares it with the core's copy

The core's copy is `K0_CHEBYSHEV` in crates/shared/raster/src/interp/spline.rs.
"""
import re
import sys

import mpmath

SRC = 'crates/shared/raster/src/interp/spline.rs'
TERMS = 30


def coefficients():
    mpmath.mp.dps = 60

    def f(u):
        if u == -1:
            return mpmath.sqrt(mpmath.pi / 2)
        x = 4 / (1 + u)
        return mpmath.exp(x) * mpmath.sqrt(x) * mpmath.besselk(0, x)
    n = 96
    nodes = [mpmath.cos(mpmath.pi * (j + mpmath.mpf(0.5)) / n) for j in range(n)]
    vals = [f(u) for u in nodes]
    c = [2 * mpmath.fsum(vals[j] * mpmath.cos(mpmath.pi * k * (j + mpmath.mpf(0.5)) / n) for j in range(n)) / n for k in range(n)]
    c[0] /= 2
    assert max(abs(x) for x in c[TERMS:]) < mpmath.mpf('1e-20'), 'the dropped terms are not below 1e-20'
    return [float(x) for x in c[:TERMS]]


def table(cs):
    lines = [f'const K0_CHEBYSHEV: [f64; {TERMS}] = [']
    lines += [f'    {c!r},' for c in cs]
    lines.append('];')
    return '\n'.join(lines)


def main():
    text = table(coefficients())
    if '--check' in sys.argv:
        with open(SRC, encoding='utf-8') as f:
            src = f.read()
        m = re.search(r'const K0_CHEBYSHEV: \[f64; \d+\] = \[.*?\];', src, re.S)
        if not m or m.group(0) != text:
            print(f'{SRC}: K0_CHEBYSHEV differs from the series', file=sys.stderr)
            sys.exit(1)
        print(f'{SRC}: K0_CHEBYSHEV matches')
        return
    print(text)


if __name__ == '__main__':
    main()
