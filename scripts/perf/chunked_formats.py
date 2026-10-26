#!/usr/bin/env python3
"""The timing files of docs/adr/0645 §8: a reanalysis-sized field in the three chunked formats, written by their own
libraries into .run/perf/chunked (about 400 MB), once.

    uv run --no-project --with netCDF4 --with zarr --with numcodecs --with eccodes python3 scripts/perf/chunked_formats.py

- era.nc: NetCDF-4 (HDF5), t2m on (time 48, lat 721, lon 1440) float32, zlib 4 and byte shuffle, chunks 1×181×360;
- era.zarr: the same as a Zarr v3 store, Zstandard, chunks 1×181×360, consolidated;
- gfs.grib2: 24 GRIB2 messages of the 0.25° global grid (721×1440), simple packing at 16 bits.

The values are a smooth field with a daily wave, as a model's would be: the compressors work as on real data.
"""

import shutil
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / ".run" / "perf" / "chunked"
NT, NY, NX = 48, 721, 1440


def field(t):
    lat = np.linspace(90, -90, NY)[:, None]
    lon = np.linspace(0, 359.75, NX)[None, :]
    base = 288 - 40 * np.sin(np.radians(lat)) ** 2 + 6 * np.cos(np.radians(lon) * 3 + np.radians(lat) * 2)
    return (base + 4 * np.sin(2 * np.pi * t / 24 + np.radians(lon))).astype("f4")


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    import netCDF4
    import zarr
    path = OUT / "era.nc"
    if not path.exists():
        d = netCDF4.Dataset(str(path), "w", format="NETCDF4")
        d.createDimension("time", NT)
        d.createDimension("lat", NY)
        d.createDimension("lon", NX)
        t = d.createVariable("time", "f8", ("time",))
        t.units = "hours since 2024-01-01 00:00:00"
        t[:] = np.arange(NT)
        la = d.createVariable("lat", "f8", ("lat",))
        la.units = "degrees_north"
        la[:] = np.linspace(90, -90, NY)
        lo = d.createVariable("lon", "f8", ("lon",))
        lo.units = "degrees_east"
        lo[:] = np.linspace(0, 359.75, NX)
        v = d.createVariable("t2m", "f4", ("time", "lat", "lon"), zlib=True, complevel=4, shuffle=True,
                             chunksizes=(1, 181, 360))
        v.units = "K"
        for k in range(NT):
            v[k] = field(k)
        d.close()
        print(f"yazıldı: {path}")
    store = OUT / "era.zarr"
    if not store.exists():
        g = zarr.open_group(str(store), mode="w", zarr_format=3)
        t = g.create_array("time", shape=(NT,), dtype="f8", chunks=(NT,), dimension_names=("time",))
        t[:] = np.arange(NT)
        t.attrs["units"] = "hours since 2024-01-01 00:00:00"
        la = g.create_array("lat", shape=(NY,), dtype="f8", chunks=(NY,), dimension_names=("lat",))
        la[:] = np.linspace(90, -90, NY)
        la.attrs["units"] = "degrees_north"
        lo = g.create_array("lon", shape=(NX,), dtype="f8", chunks=(NX,), dimension_names=("lon",))
        lo[:] = np.linspace(0, 359.75, NX)
        lo.attrs["units"] = "degrees_east"
        v = g.create_array("t2m", shape=(NT, NY, NX), dtype="f4", chunks=(1, 181, 360), fill_value=float("nan"),
                           compressors=zarr.codecs.ZstdCodec(level=3), dimension_names=("time", "lat", "lon"))
        for k in range(NT):
            v[k] = field(k)
        v.attrs["units"] = "K"
        zarr.consolidate_metadata(str(store))
        print(f"yazıldı: {store}")
    grib = OUT / "gfs.grib2"
    if not grib.exists():
        import eccodes
        with open(grib, "wb") as f:
            for k in range(24):
                h = eccodes.codes_grib_new_from_samples("GRIB2")
                for key, val in [("Ni", NX), ("Nj", NY), ("latitudeOfFirstGridPointInDegrees", 90.0),
                                 ("longitudeOfFirstGridPointInDegrees", 0.0),
                                 ("latitudeOfLastGridPointInDegrees", -90.0),
                                 ("longitudeOfLastGridPointInDegrees", 359.75),
                                 ("iDirectionIncrementInDegrees", 0.25), ("jDirectionIncrementInDegrees", 0.25),
                                 ("dataDate", 20240701), ("dataTime", 0), ("discipline", 0),
                                 ("parameterCategory", 0), ("parameterNumber", 0), ("typeOfFirstFixedSurface", 103),
                                 ("scaleFactorOfFirstFixedSurface", 0), ("scaledValueOfFirstFixedSurface", 2),
                                 ("stepUnits", 1), ("forecastTime", k), ("packingType", "grid_simple"),
                                 ("bitsPerValue", 16)]:
                    eccodes.codes_set(h, key, val)
                eccodes.codes_set_values(h, field(k).astype("f8").ravel())
                eccodes.codes_write(h, f)
                eccodes.codes_release(h)
        print(f"yazıldı: {grib}")
    for p in (path, grib):
        print(f"{p.name}: {p.stat().st_size / 1e6:.1f} MB")
    print(f"era.zarr: {sum(f.stat().st_size for f in store.rglob('*') if f.is_file()) / 1e6:.1f} MB")
    return 0


if __name__ == "__main__":
    if "--clean" in sys.argv:
        shutil.rmtree(OUT, ignore_errors=True)
    sys.exit(main())
