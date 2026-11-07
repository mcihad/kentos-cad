#!/usr/bin/env python3
"""Writes the KentOS wordmarks and the application mark at the sizes the native apps draw them
(crates/ui/src/widget/brand.rs), once, from assets/images/*.png; the apps draw each one pixel for pixel.

    python3 scripts/assets/brand_sizes.py           # writes assets/images/brand/
    python3 scripts/assets/brand_sizes.py --check   # writes nothing; compares

A large picture drawn small by the GPU is sampled bilinearly without mipmaps (iced_wgpu's image atlas), so a
wordmark 1 915 pixels wide drawn 100–260 wide lost most of its pixels and its edges came out jagged. Here every
size is filtered once:
- the wordmark is trimmed to its visible pixels (alpha above 8, as brand.rs did at run time);
- the colours are taken to linear light (the sRGB curve), premultiplied by alpha, filtered with Lanczos-3 over the
  whole reduction (Pillow's LANCZOS on 32-bit float channels), divided back by alpha and written back in sRGB;
- each size's height keeps the trimmed aspect, rounded to a whole pixel.

The sizes are the widths the apps use (logical pixels: the ribbon's 100, the launcher's 150, the start page's 160,
Hakkında's 260; the mark's 20 and 28) at one and two device pixels each (two: a Retina screen).
"""

import io
import sys
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "assets" / "images"
OUT = SRC / "brand"
LOGO_WIDTHS = sorted({w * s for w in (100, 150, 160, 260) for s in (1, 2)})
MARK_SIZES = sorted({w * s for w in (20, 28) for s in (1, 2)})


def to_linear(c):
    return np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)


def to_srgb(c):
    c = np.clip(c, 0.0, 1.0)
    return np.where(c <= 0.0031308, c * 12.92, 1.055 * c ** (1 / 2.4) - 0.055)


def lanczos(channel, size):
    return np.asarray(Image.fromarray(channel.astype(np.float32), mode="F").resize(size, Image.Resampling.LANCZOS))


def resized(rgba, size):
    """`rgba` (h, w, 4) floats 0–1 in sRGB, straight alpha → `size` (w, h) as 8-bit RGBA."""
    a = rgba[..., 3]
    lin = to_linear(rgba[..., :3]) * a[..., None]
    a2 = np.clip(lanczos(a, size), 0.0, 1.0)
    rgb = np.stack([lanczos(lin[..., i], size) for i in range(3)], axis=-1)
    with np.errstate(invalid="ignore", divide="ignore"):
        straight = np.where(a2[..., None] > 1e-6, rgb / a2[..., None], 0.0)
    out = np.concatenate([to_srgb(straight), a2[..., None]], axis=-1)
    return Image.fromarray(np.round(out * 255.0).astype(np.uint8), mode="RGBA")


def trimmed(path):
    im = Image.open(path).convert("RGBA")
    box = im.getchannel("A").point(lambda v: 255 if v > 8 else 0).getbbox()
    return im.crop(box)


def png(im):
    b = io.BytesIO()
    im.save(b, format="PNG", optimize=True)
    return b.getvalue()


def files():
    out = {}
    for name in ("kentos_light_logo", "kentos_dark_logo"):
        im = trimmed(SRC / f"{name}.png")
        rgba = np.asarray(im, dtype=np.float64) / 255.0
        for w in LOGO_WIDTHS:
            h = max(1, round(w * im.height / im.width))
            out[f"{name}_{w}.png"] = png(resized(rgba, (w, h)))
    icon = Image.open(SRC / "kentos_app_icon.png").convert("RGBA")
    rgba = np.asarray(icon, dtype=np.float64) / 255.0
    for s in MARK_SIZES:
        out[f"kentos_app_icon_{s}.png"] = png(resized(rgba, (s, s)))
    return out


def main():
    made = files()
    if "--check" in sys.argv[1:]:
        stale = [n for n, b in made.items() if not (OUT / n).exists() or (OUT / n).read_bytes() != b]
        extra = sorted(p.name for p in OUT.glob("*.png") if p.name not in made)
        if stale or extra:
            print(f"assets/images/brand güncel değil: {', '.join(stale + extra)}")
            return 1
        print(f"assets/images/brand güncel: {len(made)} dosya.")
        return 0
    OUT.mkdir(parents=True, exist_ok=True)
    for n, b in made.items():
        (OUT / n).write_bytes(b)
    print(f"yazıldı: assets/images/brand ({len(made)} dosya, {sum(len(b) for b in made.values()) // 1024} KB)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
