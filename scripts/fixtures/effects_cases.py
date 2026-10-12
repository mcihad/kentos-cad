"""Writes fixtures/compose/v1/cases.json: layer composition and draw effects (docs/adr/0226 §2–§4) computed from the
ADR's definitions with numpy, without KentOS code: the expected pictures of small scenes the renderers draw and read
back (crates/render/wgpu/tests/gpu/compose.rs with a real GPU; apps/web/scripts/e2e/compose.mjs in a browser's WebGPU
and WebGL2).

The definitions, as the ADR writes them: the picture is 8-bit, premultiplied, sRGB-encoded values mixed as they are;
outside a picture is transparent. An effect group's picture K (alpha A) and its effects, laid over the target in this
order with source-over: shadow S = c·o·Bσ(T(A)), outer glow G = c·o·Bσ(Dₛ(A)), the group (Bσ(K) when blurred), inner
shadow I = c·o·(1 − Bσ(T(A)))·A, inner glow J = c·o·Bσ(Dₛ(1 − A))·A. Bσ: Gaussian of σ = blur/3 px, weights
exp(−k²/2σ²) over k = −⌈3σ⌉ … ⌈3σ⌉ divided by their sum, separable (x then y), zero beyond the picture; for σ > 8 the
map is first shrunk to a half (σ ≤ 32) or a quarter by 2 × 2 boxes (zero beyond), blurred by σ/level and read back
bilinearly (texel centres, the last texel clamped). Dₛ: the largest value within ⌊s + ½⌋ px along x then y (zero
beyond), its inner glow's twin the smallest. T: the map read where the shadow's offset puts it (bilinear, zero beyond
the picture). A layer over the drawing: its premultiplied colour times opacity and 1 − mask; another mode: co = (1 − αs)·cb
+ αs·B(cb, cs) (W3C Compositing and Blending Level 1's separable B), addition co = min(1, cb + Cs). A mask box: a
rounded box of half size h grown by r, coverage clamp(½ − d) with d the box's signed distance; boxes meet by their
largest.

The GPU keeps each step in 8 bits (a blur's two passes, a map, the layer's picture) and lays an effect group's effects
on one another in one pass, storing the whole once: the expected pictures are made the same way (rounded to /255 where
the GPU stores), so the comparison's tolerance (3/255) covers only the order of float operations and the GPU's bilinear
weights, not the method.

    python3 scripts/fixtures/effects_cases.py           # writes the file
    python3 scripts/fixtures/effects_cases.py --check   # compares it with the one on disk
"""
import json
import math
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/compose/v1/cases.json"

W, H = 48, 40
# The drawing's ground, an 8-bit grey as the GPU stores it.
GROUND8 = 223
GROUND = (GROUND8 / 255.0,) * 3

MODES = ["normal", "multiply", "screen", "overlay", "darken", "lighten", "colorDodge", "colorBurn", "hardLight", "softLight",
         "difference", "exclusion", "addition"]


def q8(a):
    """Stored in 8 bits, as a GPU's unorm target keeps it."""
    return np.round(np.clip(a, 0.0, 1.0) * 255.0) / 255.0


def hex_rgba(h, a=1.0):
    h = h.lstrip("#")
    return [int(h[0:2], 16) / 255.0, int(h[2:4], 16) / 255.0, int(h[4:6], 16) / 255.0, a]


def rect_alpha(x0, y0, x1, y1):
    """A box's coverage, its edges on pixel edges (y down from the top)."""
    a = np.zeros((H, W))
    a[y0:y1, x0:x1] = 1.0
    return a


def premultiplied(color, alpha):
    """A solid colour over `alpha`, premultiplied, in 8 bits."""
    out = np.zeros((H, W, 4))
    for c in range(3):
        out[..., c] = color[c] * color[3] * alpha
    out[..., 3] = color[3] * alpha
    return q8(out)


def over(top, bottom):
    """Source-over of premultiplied pictures."""
    return top + bottom * (1.0 - top[..., 3:4])


def shift_read(m, dx, dy):
    """`m` read at (x − dx, y − dy) bilinearly, zero beyond (a shadow's offset; y down)."""
    h, w = m.shape
    ys, xs = np.mgrid[0:h, 0:w].astype(float)
    sx, sy = xs - dx, ys - dy
    x0, y0 = np.floor(sx).astype(int), np.floor(sy).astype(int)
    fx, fy = sx - x0, sy - y0

    def at(yy, xx):
        ok = (xx >= 0) & (xx < w) & (yy >= 0) & (yy < h)
        out = np.zeros_like(m)
        out[ok] = m[yy[ok], xx[ok]]
        return out

    return (at(y0, x0) * (1 - fx) * (1 - fy) + at(y0, x0 + 1) * fx * (1 - fy) + at(y0 + 1, x0) * (1 - fx) * fy
            + at(y0 + 1, x0 + 1) * fx * fy)


def gauss_1d(m, sigma, axis):
    if sigma <= 0:
        return m.copy()
    r = int(math.ceil(3 * sigma))
    k = np.array([math.exp(-(i * i) / (2 * sigma * sigma)) for i in range(-r, r + 1)])
    k = k / k.sum()
    out = np.zeros_like(m)
    n = m.shape[axis]
    for i, wgt in zip(range(-r, r + 1), k):
        shifted = np.zeros_like(m)
        if abs(i) < n:
            if axis == 1:
                if i >= 0:
                    shifted[:, : n - i] = m[:, i:]
                else:
                    shifted[:, -i:] = m[:, : n + i]
            else:
                if i >= 0:
                    shifted[: n - i, :] = m[i:, :]
                else:
                    shifted[-i:, :] = m[: n + i, :]
        out += wgt * shifted
    return out


def blur(m, sigma):
    """Bσ in two passes, each stored in 8 bits."""
    return q8(gauss_1d(q8(gauss_1d(m, sigma, 1)), sigma, 0))


def square_filter(m, n, largest):
    """Dₛ (largest) or its twin (smallest) within n px, x then y, each pass in 8 bits; zero beyond."""
    if n < 1:
        return m.copy()

    def one(a, axis):
        pad = [(0, 0), (0, 0)]
        pad[axis] = (n, n)
        p = np.pad(a, pad, constant_values=0.0)
        stack = [np.take(p, range(i, i + a.shape[axis]), axis=axis) for i in range(2 * n + 1)]
        return q8(np.max(stack, axis=0) if largest else np.min(stack, axis=0))

    return one(one(m, 1), 0)


def level_of(sigma):
    return 1 if sigma <= 8 else 2 if sigma <= 32 else 4


def shrink(m):
    """A 2 × 2 box, half the size rounded up, zero beyond; 8 bits."""
    h, w = m.shape[:2]
    p = np.zeros(((h + 1) // 2 * 2, (w + 1) // 2 * 2) + m.shape[2:])
    p[:h, :w] = m
    return q8((p[0::2, 0::2] + p[1::2, 0::2] + p[0::2, 1::2] + p[1::2, 1::2]) / 4.0)


def grow_read(m, level, full):
    """A level's map read at the full picture's pixel centres, bilinearly, the last texel clamped: the full picture's
    uv times W / (level · w) is the map's (its texel centres line up with the boxes they shrank)."""
    h, w = m.shape[:2]
    fh, fw = full
    ys = (np.arange(fh) + 0.5) / fh * (fh / (level * h)) * h - 0.5
    xs = (np.arange(fw) + 0.5) / fw * (fw / (level * w)) * w - 0.5

    def lerp1(arr, coords, axis):
        c0 = np.floor(coords).astype(int)
        f = coords - c0
        i0 = np.clip(c0, 0, arr.shape[axis] - 1)
        i1 = np.clip(c0 + 1, 0, arr.shape[axis] - 1)
        a0 = np.take(arr, i0, axis=axis)
        a1 = np.take(arr, i1, axis=axis)
        shape = [1] * arr.ndim
        shape[axis] = len(coords)
        f = f.reshape(shape)
        return a0 * (1 - f) + a1 * f

    return lerp1(lerp1(m, ys, 0), xs, 1)


def alpha_map(a, spread, sigma):
    """A one-channel map of the group's alpha: spread (+) or shrunk (−), blurred at its level; as read at full size."""
    m = q8(a)
    n = int(math.floor(min(abs(spread), 64) + 0.5))
    if n >= 1:
        m = square_filter(m, n, spread > 0)
    sigma = min(sigma, 64)
    level = level_of(sigma)
    at = 1
    while at < level:
        m = shrink(m)
        at *= 2
    m = blur(m, sigma / level)
    return m if level == 1 else grow_read(m, level, (H, W))


def blurred_layer(k, sigma):
    sigma = min(sigma, 64)
    level = level_of(sigma)
    m = k.copy()
    at = 1
    while at < level:
        m = shrink(m)
        at *= 2
    out = np.stack([blur(m[..., c], sigma / level) for c in range(4)], axis=-1)
    return out if level == 1 else grow_read(out, level, (H, W))


def tinted(color, m):
    """An effect's colour times its opacity over a map: premultiplied."""
    alpha = np.clip(color[3] * m, 0.0, 1.0)
    out = np.zeros((H, W, 4))
    for c in range(3):
        out[..., c] = color[c] * alpha
    out[..., 3] = alpha
    return out


def effects_over(target, k, fx):
    """An effect group's picture `k` and its effects `fx` (device px; offsets x right, y down) over `target`: the
    effects laid on one another in their order, then the whole over the target once (source-over is associative)."""
    a = k[..., 3]
    t = np.zeros((H, W, 4))
    if "shadow" in fx:
        s = fx["shadow"]
        m = alpha_map(a, 0, s["blur"] / 3)
        t = over(tinted(s["color"], shift_read(m, s["dx"], s["dy"])), t)
    if "glow" in fx:
        g = fx["glow"]
        m = alpha_map(a, g["spread"], g["blur"] / 3)
        t = over(tinted(g["color"], m), t)
    if fx.get("blur", 0) >= 0.5:
        t = over(blurred_layer(k, fx["blur"] / 3), t)
    else:
        t = over(k, t)
    if "innerShadow" in fx:
        s = fx["innerShadow"]
        m = alpha_map(a, 0, s["blur"] / 3)
        t = over(tinted(s["color"], (1 - shift_read(m, s["dx"], s["dy"])) * a), t)
    if "innerGlow" in fx:
        g = fx["innerGlow"]
        m = alpha_map(a, -g["spread"], g["blur"] / 3)
        t = over(tinted(g["color"], (1 - m) * a), t)
    return q8(over(t, target))


def hard_light(cb, cs):
    return np.where(cs <= 0.5, cb * 2 * cs, cb + (2 * cs - 1) - cb * (2 * cs - 1))


def blend_one(mode, cb, cs):
    if mode == "multiply":
        return cb * cs
    if mode == "screen":
        return cb + cs - cb * cs
    if mode == "overlay":
        return hard_light(cs, cb)
    if mode == "darken":
        return np.minimum(cb, cs)
    if mode == "lighten":
        return np.maximum(cb, cs)
    if mode == "colorDodge":
        v = np.where(cs >= 1, 1.0, np.minimum(1.0, cb / np.maximum(1 - cs, 1e-30)))
        return np.where(cb <= 0, 0.0, v)
    if mode == "colorBurn":
        v = np.where(cs <= 0, 0.0, 1 - np.minimum(1.0, (1 - cb) / np.maximum(cs, 1e-30)))
        return np.where(cb >= 1, 1.0, v)
    if mode == "hardLight":
        return hard_light(cb, cs)
    if mode == "softLight":
        d = np.where(cb <= 0.25, ((16 * cb - 12) * cb + 4) * cb, np.sqrt(cb))
        return np.where(cs <= 0.5, cb - (1 - 2 * cs) * cb * (1 - cb), cb + (2 * cs - 1) * (d - cb))
    if mode == "difference":
        return np.abs(cb - cs)
    if mode == "exclusion":
        return cb + cs - 2 * cb * cs
    return cs


def layer_into(drawing, layer, opacity, mode, mask=None):
    """A composed layer's picture into the opaque drawing (docs/adr/0226 §4)."""
    k = np.full((H, W), opacity) if mask is None else opacity * (1 - mask)
    if mode == "normal":
        return q8(over(layer * k[..., None], drawing))
    alpha = layer[..., 3] * k
    cb = drawing[..., :3]
    if mode == "addition":
        co = np.minimum(1.0, cb + layer[..., :3] * k[..., None])
    else:
        cs = np.clip(np.where(layer[..., 3:4] > 0, layer[..., :3] / np.maximum(layer[..., 3:4], 1e-30), 0.0), 0, 1)
        b = blend_one(mode, cb, cs)
        co = (1 - alpha[..., None]) * cb + alpha[..., None] * b
    co = np.where(alpha[..., None] > 0, co, cb)
    out = np.ones((H, W, 4))
    out[..., :3] = co
    return q8(out)


def mask_boxes(boxes):
    """The mask: rounded boxes, their largest coverage, in 8 bits (`boxFs`)."""
    ys, xs = np.mgrid[0:H, 0:W].astype(float)
    px, py = xs + 0.5, ys + 0.5
    m = np.zeros((H, W))
    for b in boxes:
        cx, cy, hx, hy, turn, grow = b
        c, s = math.cos(turn), math.sin(turn)
        dx, dy = px - cx, py - cy
        # The box's own axes: the inverse of its turn.
        lx, ly = c * dx + s * dy, -s * dx + c * dy
        qx, qy = np.abs(lx) - hx, np.abs(ly) - hy
        d = np.hypot(np.maximum(qx, 0), np.maximum(qy, 0)) + np.minimum(np.maximum(qx, qy), 0) - grow
        m = np.maximum(m, np.clip(0.5 - d, 0, 1))
    return q8(m)


def ground():
    g = np.ones((H, W, 4))
    g[..., :3] = GROUND
    return g


def picture_json(p):
    return [int(v) for v in np.round(np.clip(p, 0, 1) * 255).astype(int).reshape(-1)]


SQUARE = (14, 10, 32, 28)
BLUE = hex_rgba("#2B6CB0")


def effect_cases():
    out = []
    k = premultiplied(BLUE, rect_alpha(*SQUARE))
    black = lambda o: hex_rgba("#000000", o)
    for name, fx in [
        ("gölge", {"shadow": {"dx": 3, "dy": 3, "blur": 6, "color": black(0.6)}}),
        ("dış parıltı", {"glow": {"spread": 2, "blur": 4, "color": hex_rgba("#FFD000", 0.8)}}),
        ("iç gölge", {"innerShadow": {"dx": 2, "dy": 2, "blur": 3, "color": black(0.7)}}),
        ("iç parıltı", {"innerGlow": {"spread": 1, "blur": 3, "color": hex_rgba("#FFFFFF", 0.9)}}),
        ("bulanıklaştır", {"blur": 4}),
        ("beşi birden", {"shadow": {"dx": -2, "dy": 2.5, "blur": 3, "color": black(0.5)}, "glow": {"spread": 1.4, "blur": 2, "color": hex_rgba("#E15759", 0.7)},
                         "blur": 1.5, "innerShadow": {"dx": 1, "dy": 1, "blur": 2, "color": black(0.5)}, "innerGlow": {"spread": 0.6, "blur": 2, "color": hex_rgba("#FFFFFF", 0.6)}}),
        ("geniş gölge yarım çözünürlükte", {"shadow": {"dx": 0, "dy": 4, "blur": 30, "color": black(0.8)}}),
        ("yayılmış, bulanıksız parıltı", {"glow": {"spread": 3, "blur": 0, "color": hex_rgba("#59A14F", 1.0)}}),
    ]:
        expected = effects_over(ground(), k, fx)
        out.append({"name": name, "square": list(SQUARE), "color": BLUE, "effects": fx, "picture": picture_json(expected)})
    return out


def blend_cases():
    out = []
    left, right = hex_rgba("#3366CC"), hex_rgba("#F2C230")
    under = q8(over(premultiplied(right, rect_alpha(24, 0, W, H)), over(premultiplied(left, rect_alpha(0, 0, 24, H)), ground())))
    top = hex_rgba("#CC4422")
    layer = premultiplied(top, rect_alpha(8, 8, 40, 32))
    for mode in MODES:
        opacity = 0.5 if mode == "normal" else 0.8
        expected = layer_into(under, layer, opacity, mode)
        out.append({"name": f"karışım: {mode}", "mode": mode, "opacity": opacity, "under": [left, right], "square": [8, 8, 40, 32], "color": top, "picture": picture_json(expected)})
    return out


def mask_cases():
    out = []
    band = hex_rgba("#4E79A7")
    under = ground()
    layer = premultiplied(band, rect_alpha(0, 12, W, 28))
    boxes = [[14.0, 20.0, 6.0, 3.0, 0.0, 2.0], [34.0, 18.0, 7.0, 2.5, -math.pi / 6, 1.5]]
    expected = layer_into(under, layer, 1.0, "normal", mask_boxes(boxes))
    out.append({"name": "maske: iki etiket kutusu, biri dönük", "band": [0, 12, W, 28], "color": band, "boxes": boxes, "opacity": 1.0, "mode": "normal",
                "picture": picture_json(expected)})
    expected = layer_into(under, layer, 0.6, "multiply", mask_boxes(boxes))
    out.append({"name": "maske, saydamlık ve çoğaltma birlikte", "band": [0, 12, W, 28], "color": band, "boxes": boxes, "opacity": 0.6, "mode": "multiply",
                "picture": picture_json(expected)})
    return out


def build():
    return {
        "format": "kentos.compose-cases",
        "version": 1,
        "note": "Written by scripts/fixtures/effects_cases.py from docs/adr/0226 §2–§4 with numpy; do not edit by hand.",
        "size": [W, H],
        "ground": [GROUND8] * 3,
        "tolerance": 3,
        "effects": effect_cases(),
        "blends": blend_cases(),
        "masks": mask_cases(),
    }


def main() -> int:
    text = json.dumps(build(), ensure_ascii=False, separators=(",", ":")) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)} güncel")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı")
    return 0


if __name__ == "__main__":
    sys.exit(main())
