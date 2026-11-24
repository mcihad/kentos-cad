"""Correctly rounded elementary functions for the generators (docs/adr/9303): `import crmath as math`.

A reference must not depend on the platform's mathematics library. CPython's `math.sin`, `cos`, `tan`, `atan2`,
`exp`, `log`, `pow`… (and the float `**`) call the C library, and the libraries round differently: the same
generator gave different last digits on Linux (glibc), on macOS 26 and on macOS 27 (ADR 9303, Bağlam). These functions
give, for every finite input, the double nearest to the exact value (ties to even): mpmath computes it at 256 bits,
far past the hardest cases of double precision, and Python's exact integer division rounds it once.

What the C library does not decide comes from the math module unchanged: sqrt, fabs, floor, ceil, fmod, copysign,
fsum, prod, hypot, dist, degrees, radians (IEEE 754 operations or CPython's own code, the same everywhere), the
constants, isclose, isfinite… Special inputs keep the C library's (C99 Annex F) answers and errors: zeros with their
sign, infinities, NaN, a domain error's ValueError, an overflow's OverflowError.

A generator that uses this module also writes `x * x` for `x ** 2` and `math.pow(x, y)` for another float power:
the float `**` calls the C library's pow, and macOS 27's is not exact even for the square (ADR 9303, Bağlam).
"""

from __future__ import annotations

import math as _m
from functools import lru_cache as _cache
from math import *  # noqa: F401,F403  (the constants and the functions that are already the same everywhere)

import mpmath as _mpmath

_ctx = _mpmath.MPContext()  # its own context: the generators' own mpmath precision is left alone
_ctx.prec = 256


def _nearest(v) -> float:
    """The double nearest to the mpf v (ties to even): its mantissa and exponent divided exactly by Python."""
    sign, man, exp, _ = v._mpf_
    if not man:
        return -0.0 if sign else 0.0
    x = man / (1 << -exp) if exp < 0 else float(man << exp)
    return -x if sign else x


@_cache(maxsize=None)
def _one(name: str, x: float) -> float:
    return _nearest(getattr(_ctx, name)(_ctx.mpf(x)))


def _unary(name: str, whole_int: bool = False):
    """The function `name` correctly rounded; `whole_int`: an int argument is taken whole, as CPython's log and log10
    take it (an int may be past the floats); every other function takes the float the int rounds to, as CPython does."""
    libm = getattr(_m, name)

    def f(x):
        r = libm(x)  # the C library's domain checks, errors and special values
        if r == 0.0 or not _m.isfinite(r):
            return r
        if whole_int and isinstance(x, int) and not isinstance(x, bool):
            return _nearest(getattr(_ctx, name)(_ctx.mpf(x)))
        x = float(x)
        return _one(name, x) if _m.isfinite(x) else r

    f.__name__ = name
    f.__doc__ = f"{name}(x), correctly rounded (crmath)."
    return f


sin = _unary("sin")
cos = _unary("cos")
tan = _unary("tan")
asin = _unary("asin")
acos = _unary("acos")
atan = _unary("atan")
sinh = _unary("sinh")
cosh = _unary("cosh")
tanh = _unary("tanh")
asinh = _unary("asinh")
acosh = _unary("acosh")
atanh = _unary("atanh")
exp = _unary("exp")
expm1 = _unary("expm1")
log1p = _unary("log1p")
log10 = _unary("log10", whole_int=True)
cbrt = _unary("cbrt")
erf = _unary("erf")
erfc = _unary("erfc")


@_cache(maxsize=None)
def _log2(x: float) -> float:
    return _nearest(_ctx.log(_ctx.mpf(x), 2))


def log2(x):
    """log2(x), correctly rounded (crmath)."""
    r = _m.log2(x)
    if r == 0.0 or not _m.isfinite(r):
        return r
    if isinstance(x, int) and not isinstance(x, bool):
        return _nearest(_ctx.log(_ctx.mpf(x), 2))
    x = float(x)
    return _log2(x) if _m.isfinite(x) else r


@_cache(maxsize=None)
def _exp2(x: float) -> float:
    return _nearest(_ctx.power(2, _ctx.mpf(x)))


def exp2(x):
    """exp2(x), correctly rounded (crmath)."""
    r = _m.exp2(x)
    if r == 0.0 or not _m.isfinite(r) or not _m.isfinite(float(x)):
        return r
    return _exp2(float(x))


_ln = _unary("log", whole_int=True)


def log(x, base=None):
    """log(x) correctly rounded; log(x, base) as CPython makes it, the two logarithms divided in doubles."""
    if base is None:
        return _ln(x)
    _m.log(x, base)  # the C library's domain checks and errors
    return _ln(x) / _ln(base)


@_cache(maxsize=None)
def _atan2(y: float, x: float) -> float:
    return _nearest(_ctx.atan2(_ctx.mpf(y), _ctx.mpf(x)))


def atan2(y, x):
    """atan2(y, x), correctly rounded (crmath)."""
    r = _m.atan2(y, x)
    y, x = float(y), float(x)
    if r == 0.0 or not (_m.isfinite(r) and _m.isfinite(y) and _m.isfinite(x)) or (y == 0.0 and x == 0.0):
        return r
    return _atan2(y, x)


@_cache(maxsize=None)
def _pow(x: float, y: float) -> float:
    return _nearest(_ctx.power(_ctx.mpf(x), _ctx.mpf(y)))


def pow(x, y):  # noqa: A001  (the math module's name)
    """pow(x, y), correctly rounded (crmath): what `x ** y` should give for floats."""
    r = _m.pow(x, y)
    x, y = float(x), float(y)
    if r == 0.0 or not (_m.isfinite(r) and _m.isfinite(x) and _m.isfinite(y)) or x == 0.0 or y == 0.0:
        return r
    if x < 0.0:  # an integer power of a negative number: the magnitude's, with the sign C gives
        return _m.copysign(_pow(-x, y), r)
    return _pow(x, y)


def _self_test() -> None:
    """A few values whose correctly rounded result is known."""
    assert tan(_m.pi / 8) == 0.41421356237309503  # macOS 27's libm gives 0.4142135623730951
    assert sin(0.0) == 0.0 and _m.copysign(1.0, sin(-0.0)) == -1.0
    assert atan2(-0.0, 1.0) == 0.0 and _m.copysign(1.0, atan2(-0.0, 1.0)) == -1.0
    assert log(8.0, 2.0) == 3.0 and exp(0.0) == 1.0 and pow(2.0, 10.0) == 1024.0
    assert pow(-2.0, 3.0) == -8.0


if __name__ == "__main__":
    _self_test()
    print("crmath: doğru yuvarlanmış işlevler sınandı.")
