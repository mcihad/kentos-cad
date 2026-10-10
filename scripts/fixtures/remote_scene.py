#!/usr/bin/env python3
"""Uzaktan algılama'nın resimlerinin çizimi (docs/adr/0242): paylaşılan vadinin arazi örtüsünden (suitability_scene.py'nin
ortu.tif'i) ve yükseklik modelinin gölgesinden türetilen dört bantlı (mavi, yeşil, kırmızı, yakın kızılötesi) 16 bit
uydu görüntüsü, iki yıl sonrasının görüntüsü (yolun kuzeyinde yeni yapılar, yamaçta kesilen orman), görüntünün 3,2 m'lik
çok bantlı ve 1,6 m'lik pankromatik çifti, eğitim alanları ve referans noktaları.

    python3 scripts/fixtures/remote_scene.py          # fixtures/interaction/v1/remote.kcad'i ve rasterlerini yazar
    python3 scripts/fixtures/remote_scene.py --check  # hiçbir şey yazmaz; karşılaştırır

KentOS kodu kullanılmaz.
- Yansıma (×10 000, Sentinel-2 L2A gibi): her örtünün imzası (mavi, yeşil, kırmızı, YKÖ), tarlalarda doğu–batı şeritleri
  çıplak toprak ve yeşil ürün arasında, yumuşak bir doku (tohumlu rastgele kaba ızgaranın çift doğrusal büyütülmesi) ve
  yamacın aydınlığı (yükseklik modelinin 315°/45° gölgesi, suda yok). Değersiz köşe 0.
- İki yıl sonrası: ince dokusu başka mevsimin, %2 parlak algılayıcı ve biraz daha yüksek güneş; yolun kuzeyinde yeni yapılar,
  ormanın en sık olduğu 44 × 44 hücrelik pencerede kesilen ağaçlar.
- Çok bantlı: görüntünün 2 × 2 hücrelerinin ortalaması (3,2 m); pankromatik: görüntünün mavi, yeşil ve kırmızısının
  ortalaması (1,6 m).
- Eğitim alanları: her örtüde bütünüyle o örtüden iki kare (10 × 10 hücre, olmazsa 6 × 6, olmazsa 4 × 4; satır satır
  taranan ilk uygun ve ondan en uzak olan); Sınıf alanı örtünün adı.
- Referans noktaları: her örtüden tohumlu rastgele 14 hücrenin merkezi; Sınıf alanı denetimli sınıflandırmanın o örtüye
  verdiği değer (adların doğal sırası: Mera 1, Orman 2, Su 3, Tarım 4, Yerleşim 5).
"""

import json
import sys
from pathlib import Path

import numpy as np
from osgeo import gdal

gdal.UseExceptions()

ROOT = Path(__file__).resolve().parents[2]
COVER = ROOT / "fixtures/interaction/v1/suitability/ortu.tif"
DEM = ROOT / "fixtures/interaction/v1/rasters/dem.tif"
OUT = ROOT / "fixtures/interaction/v1/remote.kcad"
DIR = ROOT / "fixtures/interaction/v1/remote"

# The land cover's classes (suitability_scene.py): 1 orman, 2 mera, 3 tarım, 4 yerleşim, 5 su.
NAMES = {1: "Orman", 2: "Mera", 3: "Tarım", 4: "Yerleşim", 5: "Su"}
VALUE = {name: k + 1 for k, name in enumerate(sorted(NAMES.values()))}
# Reflectance × 10 000: blue, green, red, near infrared.
SIGNATURE = {1: (300, 560, 380, 3400), 2: (520, 820, 760, 2700), 4: (1250, 1330, 1420, 1850), 5: (620, 540, 330, 180)}
BARE = (980, 1250, 1600, 2150)
CROP = (420, 900, 560, 4100)
# Two years on: houses north of the road (cells: columns, rows); a felled stand where the forest is densest.
HOUSES = (150, 228, 228, 262)
# The fields' parcels (cells) and their kinds: bare soil, a green crop, half grown.
PARCEL = (34, 20)


def smooth_noise(shape, cell, seed):
    """A smooth field in −1…1: a seeded coarse grid, bilinear on the cells."""
    h, w = shape
    rng = np.random.RandomState(seed)
    gh, gw = h // cell + 2, w // cell + 2
    g = rng.uniform(-1.0, 1.0, (gh, gw))
    ys = (np.arange(h) + 0.5) / cell
    xs = (np.arange(w) + 0.5) / cell
    y0, x0 = np.floor(ys).astype(int), np.floor(xs).astype(int)
    fy, fx = (ys - y0)[:, None], (xs - x0)[None, :]
    a = g[y0][:, x0]
    b = g[y0][:, x0 + 1]
    c = g[y0 + 1][:, x0]
    d = g[y0 + 1][:, x0 + 1]
    return (a * (1 - fx) + b * fx) * (1 - fy) + (c * (1 - fx) + d * fx) * fy


def hillshade(z, cell):
    gy, gx = np.gradient(z, cell)
    slope = np.arctan(np.hypot(gx, gy))
    aspect = np.arctan2(-gx, gy)
    az, alt = np.radians(315.0), np.radians(45.0)
    s = np.sin(alt) * np.cos(slope) + np.cos(alt) * np.sin(slope) * np.cos(az - aspect)
    return np.clip(s, 0.0, 1.0)


def images():
    ds = gdal.Open(str(COVER))
    cover = ds.GetRasterBand(1).ReadAsArray().astype(np.int32)
    affine = list(ds.GetGeoTransform())
    dem = gdal.Open(str(DEM))
    z = dem.GetRasterBand(1).ReadAsArray().astype(np.float64)
    empty = cover == 0
    z = np.where(empty, np.nanmean(np.where(empty, np.nan, z)), z)
    shade = hillshade(z, affine[1])
    h, w = cover.shape
    rows = np.arange(h)[:, None] * np.ones((1, w), int)
    # Each parcel's kind (seeded), its rows staggered; furrows in the green ones.
    cols = np.arange(w)[None, :] * np.ones((h, 1), int)
    prow = rows // PARCEL[1]
    pcol = (cols + 11 * prow) // PARCEL[0]
    kinds = np.random.RandomState(5).randint(0, 3, (prow.max() + 2, pcol.max() + 2))
    kind = kinds[prow, pcol]
    furrow = (rows % 3 == 0) & (kind == 1)

    def synth(seeds, gain, light0):
        """The four bands of a date: its texture's seeds, its sensor's gain and the slopes' light."""
        tex = smooth_noise((h, w), 12, seeds[0])
        fine = smooth_noise((h, w), 3, seeds[1])
        bands = []
        for b in range(4):
            v = np.zeros((h, w))
            for c, sig in SIGNATURE.items():
                v[cover == c] = sig[b]
            field = cover == 3
            half = 0.5 * (BARE[b] + CROP[b])
            v[field & (kind == 0)] = BARE[b]
            v[field & (kind == 1)] = CROP[b]
            v[field & (kind == 2)] = half
            v[field & furrow] *= 0.93
            v = v * (1.0 + 0.08 * tex + 0.012 * fine) * gain
            light = np.where(cover == 5, 1.0, light0 + 0.4 * shade)
            v = v * light
            v[empty] = 0
            bands.append(np.clip(np.round(v), 1, 65535).astype(np.uint16) * (~empty))
        return np.stack(bands, axis=-1)

    before = synth((7, 11), 1.0, 0.78)
    # Two years on: the fine texture another season's, a sensor 2 % brighter, the sun a little higher.
    later = synth((7, 23), 1.02, 0.80)
    after = later.copy()
    c0, c1, r0, r1 = HOUSES
    for b in range(4):
        block = after[r0:r1, c0:c1, b].astype(np.float64)
        roofs = ((np.arange(r1 - r0)[:, None] // 6 + np.arange(c1 - c0)[None, :] // 7) % 2 == 0)
        block = np.where(roofs, SIGNATURE[4][b] * 1.05, BARE[b] * 0.9)
        after[r0:r1, c0:c1, b] = np.where(empty[r0:r1, c0:c1], 0, np.round(block)).astype(np.uint16)
    c0, c1, r0, r1 = felled(cover)
    for b in range(4):
        sel = cover[r0:r1, c0:c1] == 1
        cut = np.round(np.array(BARE)[b] * 0.85 * 1.02 * (0.80 + 0.4 * shade[r0:r1, c0:c1])).astype(np.uint16)
        after[r0:r1, c0:c1, b] = np.where(sel, cut, after[r0:r1, c0:c1, b])
    ms = before.reshape(h // 2, 2, w // 2, 2, 4).astype(np.float64).mean(axis=(1, 3))
    ms_empty = empty.reshape(h // 2, 2, w // 2, 2).any(axis=(1, 3))
    ms = np.where(ms_empty[:, :, None], 0, np.round(ms)).astype(np.uint16)
    pan = np.round(before[:, :, :3].astype(np.float64).mean(axis=2)).astype(np.uint16)
    pan[empty] = 0
    return affine, cover, before, after, ms, pan


def felled(cover):
    """The 44 × 44-cell window (on a 4-cell stride, in the image's north-east) with the most forest."""
    best, at = -1, None
    for j in range(16, 140, 4):
        for i in range(240, 430, 4):
            n = int((cover[j:j + 44, i:i + 44] == 1).sum())
            if n > best:
                best, at = n, (i, i + 44, j, j + 44)
    return at


def write(path, arr, affine):
    h, w, n = arr.shape
    ds = gdal.GetDriverByName("GTiff").Create(str(path), w, h, n, gdal.GDT_UInt16,
                                              ["TILED=YES", "COMPRESS=DEFLATE", "PREDICTOR=2", "ZLEVEL=9"])
    ds.SetGeoTransform(affine)
    for k in range(n):
        band = ds.GetRasterBand(k + 1)
        band.SetNoDataValue(0)
        band.WriteArray(arr[:, :, k])
    ds.FlushCache()
    ds = None


def squares(cover, affine):
    """Each cover's two training squares, wholly of it (10 × 10 cells, else 6 × 6, else 4 × 4): the first in row order
    and the farthest from it."""
    out = []
    h, w = cover.shape
    for c in sorted(NAMES):
        for n in (10, 6, 4):
            found = [(i, j) for j in range(4, h - n - 4, 2) for i in range(4, w - n - 4, 2) if (cover[j:j + n, i:i + n] == c).all()]
            if found:
                break
        assert found, NAMES[c]
        first = found[0]
        far = max(found, key=lambda p: (p[0] - first[0]) ** 2 + (p[1] - first[1]) ** 2)
        for (i, j) in (first, far):
            x0, y0 = affine[0] + i * affine[1], affine[3] + j * affine[5]
            x1, y1 = x0 + n * affine[1], y0 + n * affine[5]
            out.append(({"kind": "polygon", "pts": [{"x": x0, "y": y1}, {"x": x1, "y": y1}, {"x": x1, "y": y0}, {"x": x0, "y": y0}]},
                        {"Sınıf": NAMES[c]}))
    return out


def references(cover, affine):
    rng = np.random.RandomState(42)
    out = []
    for c in sorted(NAMES):
        cells = np.argwhere(cover == c)
        pick = cells[rng.choice(len(cells), 14, replace=False)]
        for j, i in sorted(map(tuple, pick)):
            x = affine[0] + (i + 0.5) * affine[1]
            y = affine[3] + (j + 0.5) * affine[5]
            out.append(({"kind": "point", "p": {"x": round(x, 2), "y": round(y, 2)}}, {"Sınıf": str(VALUE[NAMES[c]])}))
    return out


def drawing(affine, cover, before):
    h, w = cover.shape
    half = [affine[0], affine[1] * 2, 0.0, affine[3], 0.0, affine[5] * 2]
    rgb = {"render": "rgb", "bands": [3, 2, 1], "stretch": "percent"}
    gray = {"render": "gray", "bands": [1], "stretch": "percent"}
    rasters = [
        ("goruntu", "Görüntü (2024)", "goruntu.tif", affine, w, h, 4, rgb),
        ("sonraki", "Görüntü (2026)", "sonraki.tif", affine, w, h, 4, rgb),
        ("cok-bantli", "Çok bantlı (3,2 m)", "cok-bantli.tif", half, w // 2, h // 2, 4, rgb),
        ("pankromatik", "Pankromatik (1,6 m)", "pankromatik.tif", affine, w, h, 1, gray),
    ]
    entities = []

    def add(e, lid, attrs=None):
        entities.append({**e, "id": len(entities) + 1, "layerId": lid, "attrs": attrs or {}})

    for lid, _, file, a, rw, rh, n, style in rasters:
        add({"kind": "raster", "affine": a, "width": rw, "height": rh, "bands": n, "sample": "u16", "file": f"remote/{file}",
             "srid": 5256, "style": style}, lid)
    for s, attrs in squares(cover, affine):
        add(s, "egitim", attrs)
    for s, attrs in references(cover, affine):
        add(s, "referans", attrs)

    def layer(id, name, color, visible=True, **extra):
        return {"id": id, "name": name, "type": "layer", "visible": visible, "locked": False, "expanded": True,
                "style": {"color": color, "lineType": "continuous", "lineWeight": 0.35, **extra}, "children": []}

    layers = [
        layer("egitim", "Eğitim alanları", "#FFD54F", fill="#FFD54F40"),
        layer("referans", "Referans", "#E53935", point={"symbol": "ring", "size": 5}),
        layer("goruntu", "Görüntü (2024)", "#8D6E63"),
        layer("sonraki", "Görüntü (2026)", "#8D6E63", visible=False),
        layer("cok-bantli", "Çok bantlı (3,2 m)", "#8D6E63", visible=False),
        layer("pankromatik", "Pankromatik (1,6 m)", "#8D6E63", visible=False),
    ]
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Uzaktan algılama: vadinin uydu görüntüsü",
        "settings": {"srid": 5256, "lengthDecimals": 2, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000.0,
                     "workspace": "gis"},
        "origin": {"x": 487_500.0, "y": 4_420_300.0},
        "layers": layers,
        "activeLayer": "egitim",
        "entities": entities,
        "styles": {"items": [], "categories": []},
    }


def main():
    check = "--check" in sys.argv[1:]
    affine, cover, before, after, ms, pan = images()
    half = [affine[0], affine[1] * 2, 0.0, affine[3], 0.0, affine[5] * 2]
    files = {"goruntu.tif": (before, affine), "sonraki.tif": (after, affine), "cok-bantli.tif": (ms, half),
             "pankromatik.tif": (pan[:, :, None], affine)}
    problems = []
    DIR.mkdir(parents=True, exist_ok=True)
    for name, (arr, a) in files.items():
        path = DIR / name
        if not check:
            write(path, arr, a)
        if not path.exists():
            problems.append(f"{path.relative_to(ROOT)} yok")
            continue
        ds = gdal.Open(str(path))
        got = np.stack([ds.GetRasterBand(k + 1).ReadAsArray() for k in range(ds.RasterCount)], axis=-1)
        if list(ds.GetGeoTransform()) != list(a) or not np.array_equal(got, arr):
            problems.append(f"{path.relative_to(ROOT)} yeniden kurulanla aynı değil")
    text = json.dumps(drawing(affine, cover, before), ensure_ascii=False, indent=1) + "\n"
    if check:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            problems.append(f"{OUT.relative_to(ROOT)} yeniden kurulanla aynı değil (betiği --check olmadan çalıştırın)")
    else:
        OUT.write_text(text, "utf-8")
    if problems:
        print("\n".join(problems), file=sys.stderr)
        sys.exit(1)
    sizes = ", ".join(f"{n} {(DIR / n).stat().st_size // 1024} KB" for n in files)
    print(f"{OUT.relative_to(ROOT)}: dört raster ({sizes})")


if __name__ == "__main__":
    main()
