#!/usr/bin/env python3
"""NetCDF-4 (HDF5), Zarr and GRIB files and what they hold, without KentOS code (docs/adr/0645).

`--files` writes the files of fixtures/multidim/v2 with the formats' own libraries (netCDF4-python and h5py over
libhdf5, zarr-python and numcodecs, ecCodes) and `cases.json`: each file's dimensions, variables (dimensions, type,
every value, attributes) as those libraries read them, laid out by ADR 0645's rules (dimension names, GRIB's
variables and levels, Zarr's dimensions). `--check` reads the files again and says whether cases.json still matches.
`--mesh-files` writes only the NetCDF-4 and Zarr copies of the classic UGRID mesh layered.nc.

Run with: uv run --no-project --with h5py --with netCDF4 --with zarr --with numcodecs --with eccodes \
    python3 scripts/fixtures/chunked_formats_cases.py --check
"""

import json
import math
import os
import shutil
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures" / "multidim" / "v2"
HIDDEN = {"CLASS", "NAME", "REFERENCE_LIST", "DIMENSION_LIST", "DIMENSION_LABELS", "_Netcdf4Dimid",
          "_Netcdf4Coordinates", "_nc3_strict", "_NCProperties", "_IsNetcdf4", "_SuperblockVersion"}
KIND = {"int8": "byte", "uint8": "ubyte", "int16": "short", "uint16": "ushort", "int32": "int", "uint32": "uint",
        "int64": "int64", "uint64": "uint64", "float32": "float", "float64": "double", "bool": "ubyte"}


def num(v):
    v = float(v)
    if math.isnan(v):
        return None
    if math.isinf(v):
        return "inf" if v > 0 else "-inf"
    return v


def values(a):
    a = np.asarray(a)
    if a.dtype == bool:
        a = a.astype("uint8")
    return [num(x) for x in a.ravel(order="C")]


def attr_json(v):
    if isinstance(v, bytes):
        v = v.decode("utf-8", "replace")
    if isinstance(v, str):
        return {"text": v}
    a = np.atleast_1d(np.asarray(v))
    if a.dtype.kind in "US":
        return {"text": "\n".join(str(x) for x in a)}
    return {"numbers": [num(x) for x in a]}


# ---------------------------------------------------------------- the files

def grid(nt, ny, nx, seed):
    rng = np.random.default_rng(seed)
    base = np.linspace(270, 300, ny * nx).reshape(ny, nx)
    return np.stack([base + rng.normal(0, 2, (ny, nx)) + 3 * t for t in range(nt)])


def write_netcdf4():
    import netCDF4
    # 1. Chunked with zlib and shuffle, an unlimited time, CF grid, packed short, an attribute-heavy variable.
    p = DIR / "nc4-cf.nc"
    with netCDF4.Dataset(p, "w", format="NETCDF4") as d:
        d.title = "KentOS NetCDF-4 sınaması"
        d.createDimension("time", None)
        d.createDimension("lat", 13)
        d.createDimension("lon", 17)
        t = d.createVariable("time", "f8", ("time",))
        t.units = "hours since 2024-07-01 00:00:00"
        t.calendar = "standard"
        la = d.createVariable("lat", "f4", ("lat",))
        la.units = "degrees_north"
        la[:] = np.linspace(42, 36, 13)
        lo = d.createVariable("lon", "f4", ("lon",))
        lo.units = "degrees_east"
        lo[:] = np.linspace(26, 34, 17)
        v = d.createVariable("t2m", "f4", ("time", "lat", "lon"), zlib=True, shuffle=True, complevel=4,
                             chunksizes=(1, 5, 7), fill_value=-9999.0)
        v.units = "K"
        v.long_name = "2 metre temperature"
        for k in range(9):
            v.setncattr(f"note{k}", f"öznitelik {k}")
        data = grid(4, 13, 17, 1).astype("f4")
        data[1, 2:4, 3:6] = -9999.0
        v[:] = data
        t[:] = [0, 6, 12, 18]
        s = d.createVariable("precip", "i2", ("time", "lat", "lon"), zlib=True, chunksizes=(2, 13, 17))
        s.scale_factor = 0.01
        s.add_offset = 0.0
        s.units = "mm"
        s.set_auto_maskandscale(False)
        s[:] = (np.abs(grid(4, 13, 17, 2) - 285) * 10).astype("i2")
        e = d.createVariable("elev", "f8", ("lat", "lon"), contiguous=True)
        e[:] = grid(1, 13, 17, 3)[0]
        # Chunks never written: the fill.
        u = d.createVariable("sparse", "f4", ("lat", "lon"), chunksizes=(4, 4), fill_value=np.float32(1.5))
        u[0:4, 0:4] = 7.0
        g = d.createGroup("ek")
        g.createDimension("n", 5)
        x = g.createVariable("say", "i4", ("n",))
        x[:] = [1, 2, 3, 4, 5]
    # 2. Many variables (the group's links in a fractal heap), big-endian, uint64, a scalar, a char variable.
    p = DIR / "nc4-many.nc"
    with netCDF4.Dataset(p, "w", format="NETCDF4") as d:
        d.createDimension("x", 6)
        d.createDimension("y", 4)
        d.createDimension("s", 3)
        for k in range(12):
            v = d.createVariable(f"v{k:02d}", "f8", ("y", "x"), endian="big" if k % 2 else "little",
                                 zlib=k % 3 == 0)
            v[:] = np.arange(24).reshape(4, 6) * (k + 1) + 0.25
        w = d.createVariable("big", "u8", ("x",))
        w[:] = np.array([0, 1, 2**40, 2**52, 7, 9], dtype="u8")
        sc = d.createVariable("scalar", "f4", ())
        sc[...] = 3.5
        c = d.createVariable("code", "S1", ("s",))
        c[:] = np.array([b"A", b"B", b"C"])


def write_hdf5():
    import h5py
    # Plain HDF5 of the newest format: version 4 layouts (fixed array, extensible array, version 2 B-tree, single
    # chunk, implicit), a compact dataset, many attributes (dense).
    p = DIR / "h5-latest.h5"
    with h5py.File(p, "w", libver="latest") as f:
        a = np.arange(6 * 10, dtype="f4").reshape(6, 10) / 4
        f.create_dataset("fixed", data=a, chunks=(2, 4), compression="gzip", shuffle=True)
        e = f.create_dataset("grow", shape=(0, 10), maxshape=(None, 10), chunks=(1, 5), dtype="i4")
        e.resize((7, 10))
        e[...] = np.arange(70).reshape(7, 10)
        b = f.create_dataset("both", shape=(3, 4), maxshape=(None, None), chunks=(2, 2), dtype="f8",
                             compression="gzip")
        b[...] = np.arange(12).reshape(3, 4) * 1.5
        s = f.create_dataset("single", data=np.arange(8, dtype="i2").reshape(2, 4), chunks=(2, 4),
                             compression="gzip")
        f.create_dataset("implicit", data=np.arange(12, dtype="u2").reshape(3, 4), chunks=(1, 4),
                         fillvalue=5, track_times=False)
        f.create_dataset("compact", data=np.array([1.25, 2.5, 3.75], dtype="f4"),
                         dcpl=compact_plist(h5py))
        for k in range(20):
            s.attrs[f"a{k:02d}"] = k * 1.5
        s.attrs["text"] = "uzun bir açıklama"
        f.attrs["kaynak"] = "h5py"


def compact_plist(h5py):
    pl = h5py.h5p.create(h5py.h5p.DATASET_CREATE)
    pl.set_layout(h5py.h5d.COMPACT)
    return pl


def write_zarr():
    import zarr
    import numcodecs
    for name in ("z2-blosc.zarr", "z2-codecs.zarr", "z3-zstd.zarr", "z3-shard.zarr"):
        shutil.rmtree(DIR / name, ignore_errors=True)
    data = grid(3, 11, 9, 4).astype("f4")
    # v2: Blosc LZ4 with byte shuffle (xarray's default), consolidated, xarray's dimensions.
    g = zarr.open_group(str(DIR / "z2-blosc.zarr"), mode="w", zarr_format=2)
    g.attrs["Conventions"] = "CF-1.8"
    lat = g.create_array("lat", shape=(11,), dtype="f8", chunks=(11,), compressors=None)
    lat[:] = np.linspace(41, 36, 11)
    lat.attrs.update({"_ARRAY_DIMENSIONS": ["lat"], "units": "degrees_north"})
    lon = g.create_array("lon", shape=(9,), dtype="f8", chunks=(9,), compressors=None)
    lon[:] = np.linspace(27, 33, 9)
    lon.attrs.update({"_ARRAY_DIMENSIONS": ["lon"], "units": "degrees_east"})
    tm = g.create_array("time", shape=(3,), dtype="i8", chunks=(3,), compressors=None)
    tm[:] = [0, 1, 2]
    tm.attrs.update({"_ARRAY_DIMENSIONS": ["time"], "units": "days since 2024-01-01", "calendar": "proleptic_gregorian"})
    t = g.create_array("t2m", shape=data.shape, dtype="f4", chunks=(1, 4, 4), fill_value=float("nan"),
                       compressors=numcodecs.Blosc(cname="lz4", clevel=5, shuffle=numcodecs.Blosc.SHUFFLE))
    t[...] = data
    t.attrs.update({"_ARRAY_DIMENSIONS": ["time", "lat", "lon"], "units": "K"})
    zarr.consolidate_metadata(str(DIR / "z2-blosc.zarr"))
    # v2: every compressor and filter, Fortran order, a chunk never written, '/' as the separator; not consolidated.
    g = zarr.open_group(str(DIR / "z2-codecs.zarr"), mode="w", zarr_format=2)
    comps = {
        "blosclz": numcodecs.Blosc(cname="blosclz", clevel=5, shuffle=numcodecs.Blosc.NOSHUFFLE),
        "bitshuf": numcodecs.Blosc(cname="zstd", clevel=3, shuffle=numcodecs.Blosc.BITSHUFFLE),
        "zlib": numcodecs.Zlib(level=6),
        "gzip": numcodecs.GZip(level=6),
        "zstd": numcodecs.Zstd(level=3),
        "lz4": numcodecs.LZ4(),
    }
    ints = np.arange(5 * 7, dtype="i4").reshape(5, 7) * 37 - 100
    for k, (nm, c) in enumerate(comps.items()):
        a = g.create_array(nm, shape=(5, 7), dtype="i4", chunks=(2, 3), compressors=c)
        a[...] = ints + k
        a.attrs["_ARRAY_DIMENSIONS"] = ["y", "x"]
    f = g.create_array("fortran", shape=(5, 7), dtype="f8", chunks=(3, 4), order="F",
                       filters=[numcodecs.Shuffle(elementsize=8)], compressors=numcodecs.Zlib())
    f[...] = ints / 8
    f.attrs["_ARRAY_DIMENSIONS"] = ["y", "x"]
    h = g.create_array("holes", shape=(5, 7), dtype="i2", chunks=(2, 2), fill_value=-1,
                       chunk_key_encoding={"name": "v2", "separator": "/"})
    h[0:2, 0:2] = 9
    h.attrs["_ARRAY_DIMENSIONS"] = ["y", "x"]
    # v3: Zstandard and crc32c, dimension names, consolidated inline.
    g = zarr.open_group(str(DIR / "z3-zstd.zarr"), mode="w", zarr_format=3)
    g.attrs["kaynak"] = "zarr v3"
    a = g.create_array("t2m", shape=data.shape, dtype="f4", chunks=(2, 5, 5), fill_value=float("nan"),
                       compressors=zarr.codecs.ZstdCodec(level=3), dimension_names=("time", "lat", "lon"))
    a[...] = data
    a.attrs["units"] = "K"
    b = g.create_array("mask", shape=(11, 9), dtype="uint8", chunks=(4, 4), fill_value=0,
                       compressors=[zarr.codecs.GzipCodec(), zarr.codecs.Crc32cCodec()],
                       dimension_names=("lat", "lon"), serializer=zarr.codecs.BytesCodec(endian="big"))
    b[...] = (data[0] > 285).astype("uint8")
    zarr.consolidate_metadata(str(DIR / "z3-zstd.zarr"))
    # v3: sharding (inner chunks of an outer one), a shard never written.
    g = zarr.open_group(str(DIR / "z3-shard.zarr"), mode="w", zarr_format=3)
    s = g.create_array("dem", shape=(12, 10), dtype="f8", chunks=(2, 5), shards=(4, 10), fill_value=-1.0,
                       compressors=zarr.codecs.ZstdCodec(), dimension_names=("y", "x"))
    s[0:8, :] = np.arange(80).reshape(8, 10) * 0.5
    zarr.consolidate_metadata(str(DIR / "z3-shard.zarr"))


def write_mesh_copies():
    """layered.nc (mesh_cases.py's classic UGRID mesh) as NetCDF-4 (zlib, chunked) and as a consolidated Zarr v2 store:
    the same variables, dimensions and attributes, so cad.meshes.* answer them as the classic file (docs/adr/0645 §7)."""
    import netCDF4
    import zarr
    import numcodecs
    src = netCDF4.Dataset(str(ROOT / "fixtures" / "meshes" / "v1" / "files" / "layered.nc"))
    src.set_auto_maskandscale(False)
    nc4 = DIR / "layered-nc4.nc"
    nc4.unlink(missing_ok=True)
    dst = netCDF4.Dataset(str(nc4), "w", format="NETCDF4")
    for name, d in src.dimensions.items():
        dst.createDimension(name, None if d.isunlimited() else len(d))
    dst.setncatts({k: src.getncattr(k) for k in src.ncattrs()})
    for name, v in src.variables.items():
        fill = v.getncattr("_FillValue") if "_FillValue" in v.ncattrs() else None
        out = dst.createVariable(name, v.dtype, v.dimensions, zlib=v.ndim > 0, shuffle=v.ndim > 0, fill_value=fill)
        out.setncatts({k: v.getncattr(k) for k in v.ncattrs() if k != "_FillValue"})
        out.set_auto_maskandscale(False)
        out[...] = v[...]
    dst.close()
    store = DIR / "layered.zarr"
    shutil.rmtree(store, ignore_errors=True)
    g = zarr.open_group(str(store), mode="w", zarr_format=2)
    g.attrs.update({k: str(src.getncattr(k)) for k in src.ncattrs()})
    for name, v in src.variables.items():
        a = v[...]
        fill = v.getncattr("_FillValue") if "_FillValue" in v.ncattrs() else None
        chunks = tuple(max(1, n // 2) for n in a.shape) if a.ndim else ()
        z = g.create_array(name, shape=a.shape, dtype=a.dtype, chunks=chunks,
                           fill_value=None if fill is None else fill.item(),
                           compressors=numcodecs.Zlib(level=6) if a.ndim else None)
        z[...] = a
        attrs = {}
        for k in v.ncattrs():
            if k == "_FillValue":
                continue
            val = v.getncattr(k)
            attrs[k] = val if isinstance(val, str) else (val.tolist() if hasattr(val, "tolist") else val)
        attrs["_ARRAY_DIMENSIONS"] = list(v.dimensions)
        z.attrs.update(attrs)
    zarr.consolidate_metadata(str(store))
    src.close()


def write_grib():
    import eccodes
    nx, ny = 9, 7

    def message(sample, keys, vals, path, mode):
        h = eccodes.codes_grib_new_from_samples(sample)
        for k, v in keys:
            eccodes.codes_set(h, k, v)
        try:
            eccodes.codes_set_values(h, vals.ravel())
        except Exception:
            print("GRIB yazılamadı:", sample, dict(keys))
            raise
        with open(path, mode) as f:
            eccodes.codes_write(h, f)
        eccodes.codes_release(h)

    common = [("Ni", nx), ("Nj", ny), ("latitudeOfFirstGridPointInDegrees", 42.0),
              ("longitudeOfFirstGridPointInDegrees", 26.0), ("latitudeOfLastGridPointInDegrees", 36.0),
              ("longitudeOfLastGridPointInDegrees", 34.0), ("iDirectionIncrementInDegrees", 1.0),
              ("jDirectionIncrementInDegrees", 1.0)]
    base = grid(1, ny, nx, 5)[0]
    # GRIB2: temperature at three times and two isobaric levels (simple packing), wind at 10 m (complex with spatial
    # differencing), precipitation with missing points (a bitmap, complex packing), pressure in PNG, height in CCSDS.
    p = DIR / "gfs.grib2"
    if p.exists():
        p.unlink()
    mode = "wb"
    for step in (0, 3, 6):
        for lev in (850, 500):
            message("GRIB2", common + [("dataDate", 20240701), ("dataTime", 0), ("discipline", 0),
                                       ("parameterCategory", 0), ("parameterNumber", 0),
                                       ("typeOfFirstFixedSurface", 100), ("level", lev), ("stepUnits", 1),
                                       ("forecastTime", step), ("packingType", "grid_simple"),
                                       ("bitsPerValue", 16)],
                    base + step - lev / 100, p, mode)
            mode = "ab"
        message("GRIB2", common + [("dataDate", 20240701), ("dataTime", 0), ("parameterCategory", 2),
                                   ("parameterNumber", 2), ("typeOfFirstFixedSurface", 103),
                                   ("scaleFactorOfFirstFixedSurface", 0), ("scaledValueOfFirstFixedSurface", 10),
                                   ("stepUnits", 1), ("forecastTime", step),
                                   ("packingType", "grid_complex_spatial_differencing"), ("bitsPerValue", 12)],
                (base - 285) / 3 + step, p, "ab")
    pr = np.abs(base - 285)
    pr[2:4, 2:5] = 9999
    message("GRIB2", common + [("dataDate", 20240701), ("dataTime", 0), ("parameterCategory", 1),
                               ("parameterNumber", 8), ("typeOfFirstFixedSurface", 1), ("stepUnits", 1),
                               ("forecastTime", 6), ("bitmapPresent", 1), ("missingValue", 9999),
                               ("packingType", "grid_complex"), ("bitsPerValue", 10)], pr, p, "ab")
    message("GRIB2", common + [("dataDate", 20240701), ("dataTime", 0), ("parameterCategory", 3),
                               ("parameterNumber", 1), ("typeOfFirstFixedSurface", 101), ("stepUnits", 1),
                               ("forecastTime", 0), ("packingType", "grid_png"), ("bitsPerValue", 12)],
            np.linspace(40, 160, nx * ny).reshape(ny, nx), p, "ab")
    message("GRIB2", common + [("dataDate", 20240701), ("dataTime", 0), ("parameterCategory", 3),
                               ("parameterNumber", 5), ("typeOfFirstFixedSurface", 1), ("stepUnits", 1),
                               ("forecastTime", 0), ("packingType", "grid_ccsds"), ("bitsPerValue", 14)],
            base * 7 - 1500, p, "ab")
    # GRIB1 (ECMWF's table 128): 2 m temperature and mean sea level pressure, two times; rows south to north.
    p = DIR / "era5.grib"
    if p.exists():
        p.unlink()
    mode = "wb"
    for hour in (0, 12):
        for param, scale in ((167, 1.0), (151, 300.0)):
            message("GRIB1", [("Ni", nx), ("Nj", ny), ("latitudeOfFirstGridPointInDegrees", 36.0),
                              ("longitudeOfFirstGridPointInDegrees", 26.0),
                              ("latitudeOfLastGridPointInDegrees", 42.0),
                              ("longitudeOfLastGridPointInDegrees", 34.0), ("iDirectionIncrementInDegrees", 1.0),
                              ("jDirectionIncrementInDegrees", 1.0), ("jScansPositively", 1),
                              ("table2Version", 128), ("indicatorOfParameter", param),
                              ("indicatorOfTypeOfLevel", 1), ("level", 0), ("dataDate", 20240115),
                              ("dataTime", hour * 100), ("bitsPerValue", 16)],
                    base[::-1] * scale + hour, p, mode)
            mode = "ab"


# ---------------------------------------------------------------- what the libraries read

def read_netcdf4(path):
    """A NetCDF-4 file as netCDF4-python reads it, its dimensions named by ADR 0645 §2."""
    import netCDF4
    out = {"dims": [], "vars": []}
    with netCDF4.Dataset(path) as d:
        d.set_auto_maskandscale(False)
        seen = {}

        def walk(g, prefix):
            for dn, dim in g.dimensions.items():
                name = dn if dn not in seen else f"{prefix}{dn}"
                seen.setdefault(dn, True)
                out["dims"].append([name, len(dim)])
            for vn, v in sorted(g.variables.items()):
                dims = [x for x in v.dimensions]
                a = v[...]
                kind = "char" if v.dtype == np.dtype("S1") else KIND[np.dtype(v.dtype).name]
                vals = [float(ord(x)) if isinstance(x, (bytes, np.bytes_)) else x for x in np.asarray(a).ravel()]
                out["vars"].append({
                    "name": f"{prefix}{vn}",
                    "dims": dims,
                    "kind": kind,
                    "values": [num(x) if not isinstance(x, (bytes, np.bytes_)) else float(x[0]) for x in vals],
                    "attrs": {k: attr_json(v.getncattr(k)) for k in v.ncattrs() if k not in HIDDEN},
                })
            for gn, sub in sorted(g.groups.items()):
                walk(sub, f"{prefix}{gn}/")
        walk(d, "")
        out["attrs"] = {k: attr_json(d.getncattr(k)) for k in d.ncattrs() if k not in HIDDEN}
    return out


def read_hdf5(path):
    """A plain HDF5 file as h5py reads it, its axes `phony_dim_n` by ADR 0645 §2's rule."""
    import h5py
    out = {"dims": [], "vars": [], "attrs": {}}
    phony = {}
    with h5py.File(path, "r") as f:
        out["attrs"] = {k: attr_json(v) for k, v in f.attrs.items() if k not in HIDDEN}
        names = []
        f.visititems(lambda n, o: names.append(n) if isinstance(o, h5py.Dataset) else None)
        for n in sorted(names, key=lambda s: s.split("/")):
            ds = f[n]
            dims = []
            for length in ds.shape:
                if length not in phony:
                    phony[length] = f"phony_dim_{len(out['dims'])}"
                    out["dims"].append([phony[length], length])
                dims.append(phony[length])
            out["vars"].append({
                "name": n,
                "dims": dims,
                "kind": KIND[ds.dtype.name],
                "values": values(ds[()]),
                "attrs": {k: attr_json(v) for k, v in ds.attrs.items() if k not in HIDDEN},
            })
    return out


def read_zarr(path):
    """A Zarr store as zarr-python reads it, dimensions by ADR 0645 §3."""
    import zarr
    g = zarr.open_group(str(path), mode="r")
    out = {"dims": [], "vars": [], "attrs": {k: attr_json(v) for k, v in g.attrs.items()}}
    dims = {}
    phony = {}
    for name in sorted(n for n, _ in g.arrays()):
        a = g[name]
        attrs = dict(a.attrs)
        dn = attrs.pop("_ARRAY_DIMENSIONS", None) or (list(a.metadata.dimension_names)
                                                       if getattr(a.metadata, "dimension_names", None) else None)
        names = []
        for k, length in enumerate(a.shape):
            if dn:
                n = dn[k]
                if n not in dims:
                    dims[n] = length
                    out["dims"].append([n, length])
            else:
                if length not in phony:
                    phony[length] = f"phony_dim_{len(out['dims'])}"
                    out["dims"].append([phony[length], length])
                n = phony[length]
            names.append(n)
        fill = a.metadata.fill_value
        fill = float(fill) if fill is not None else float("nan")
        jattrs = {k: attr_json(v) for k, v in attrs.items()}
        if not math.isnan(fill) and "_FillValue" not in jattrs:
            jattrs["_FillValue"] = {"numbers": [num(fill)]}
        out["vars"].append({"name": name, "dims": names, "kind": KIND[a.dtype.name], "values": values(a[...]),
                            "attrs": jattrs})
    return out


G2 = {(0, 0, 0): "t", (0, 2, 2): "u", (0, 1, 8): "tp", (0, 3, 1): "prmsl", (0, 3, 5): "gh"}
G1 = {(128, 167): "t2m", (128, 151): "msl"}


def read_grib(path):
    """A GRIB file as ecCodes reads it, made variables by ADR 0645 §4's rule."""
    import eccodes
    fields = []
    with open(path, "rb") as f:
        while True:
            h = eccodes.codes_grib_new_from_file(f)
            if h is None:
                break
            g = lambda k: eccodes.codes_get(h, k)
            ed = g("edition")
            ni, nj = g("Ni"), g("Nj")
            vals = eccodes.codes_get_values(h).astype(float)
            if g("bitmapPresent"):
                vals[vals == g("missingValue")] = np.nan
            v = vals.reshape(ni, nj).T if g("jPointsAreConsecutive") else vals.reshape(nj, ni)
            if g("iScansNegatively"):
                v = v[:, ::-1]
            if ed == 2:
                name = G2.get((g("discipline"), g("parameterCategory"), g("parameterNumber")))
                lt = eccodes.codes_get(h, "typeOfFirstFixedSurface", ktype=int)
                # ADR 0645 §4: a missing scale factor (all bits set) is 0; a missing value, level 0.
                sf = 0 if eccodes.codes_is_missing(h, "scaleFactorOfFirstFixedSurface") else \
                    g("scaleFactorOfFirstFixedSurface")
                lev = 0.0 if eccodes.codes_is_missing(h, "scaledValueOfFirstFixedSurface") else \
                    float(g("scaledValueOfFirstFixedSurface")) / 10 ** sf
            else:
                name = G1.get((g("table2Version"), g("indicatorOfParameter")))
                lt = eccodes.codes_get(h, "indicatorOfTypeOfLevel", ktype=int)
                lev = float(g("level"))
            vd, vt = g("validityDate"), g("validityTime")
            import datetime
            t = datetime.datetime(vd // 10000, vd // 100 % 100, vd % 100, vt // 100, vt % 100,
                                  tzinfo=datetime.timezone.utc).timestamp()
            lat1, lat2 = g("latitudeOfFirstGridPointInDegrees"), g("latitudeOfLastGridPointInDegrees")
            lats = np.linspace(lat1, lat2, nj) if nj > 1 else np.array([lat1])
            lon_w = g("longitudeOfLastGridPointInDegrees") if g("iScansNegatively") else \
                g("longitudeOfFirstGridPointInDegrees")
            lon_e = g("longitudeOfFirstGridPointInDegrees") if g("iScansNegatively") else \
                g("longitudeOfLastGridPointInDegrees")
            if lon_e < lon_w:
                lon_e += 360
            lons = np.linspace(lon_w, lon_e, ni)
            fields.append(dict(ed=ed, name=name, lt=lt, lev=lev, t=t, v=v, lats=lats, lons=lons))
            eccodes.codes_release(h)
    ed = max(f["ed"] for f in fields)
    times = sorted({f["t"] for f in fields})
    out = {"dims": [["time", len(times)], ["latitude", fields[0]["v"].shape[0]],
                    ["longitude", fields[0]["v"].shape[1]]], "vars": [], "attrs": {"Conventions": {"text": "CF-1.8"}}}
    out["vars"].append({"name": "time", "dims": ["time"], "kind": "double", "values": [num(t) for t in times],
                        "attrs": {}})
    out["vars"].append({"name": "latitude", "dims": ["latitude"], "kind": "double",
                        "values": [round(x, 6) for x in fields[0]["lats"]], "attrs": {}})
    out["vars"].append({"name": "longitude", "dims": ["longitude"], "kind": "double",
                        "values": [round(x, 6) for x in fields[0]["lons"]], "attrs": {}})
    groups = {}
    for f in fields:
        groups.setdefault((f["name"], f["lt"]), []).append(f)
    level_dims = {}
    for (name, lt), fs in sorted(groups.items()):
        levels = sorted({f["lev"] for f in fs})
        dims = ["time"]
        if len(levels) > 1:
            key = (lt, tuple(levels))
            if key not in level_dims:
                dn = {100: "isobaric", 103: "height"}.get(lt, "level")
                level_dims[key] = dn
                out["dims"].append([dn, len(levels)])
                out["vars"].append({"name": dn, "dims": [dn], "kind": "double", "values": levels, "attrs": {}})
            dims.append(level_dims[key])
            vname = name
        else:
            lev = levels[0]
            if (ed, lt) in ((2, 100),):
                vname = f"{name}_{int(lev / 100)}hPa"
            elif lt in (103,) and ed == 2:
                vname = f"{name}_{int(lev)}m"
            else:
                vname = name
        dims += ["latitude", "longitude"]
        shape = [len(times)] + ([len(levels)] if len(levels) > 1 else []) + list(fs[0]["v"].shape)
        arr = np.full(shape, np.nan)
        for f in fs:
            ti = times.index(f["t"])
            if len(levels) > 1:
                arr[ti, levels.index(f["lev"])] = f["v"]
            else:
                arr[ti] = f["v"]
        out["vars"].append({"name": vname, "dims": dims, "kind": "double", "values": values(arr), "attrs": {}})
    return out


FILES = [
    ("nc4-cf.nc", "NetCDF-4 (HDF5)", read_netcdf4),
    ("nc4-many.nc", "NetCDF-4 (HDF5)", read_netcdf4),
    ("h5-latest.h5", "HDF5", read_hdf5),
    ("z2-blosc.zarr", "Zarr v2", read_zarr),
    ("z2-codecs.zarr", "Zarr v2", read_zarr),
    ("z3-zstd.zarr", "Zarr v3", read_zarr),
    ("z3-shard.zarr", "Zarr v3", read_zarr),
    ("layered-nc4.nc", "NetCDF-4 (HDF5)", read_netcdf4),
    ("layered.zarr", "Zarr v2", read_zarr),
    ("gfs.grib2", "GRIB2", read_grib),
    ("era5.grib", "GRIB1", read_grib),
]


def cases():
    out = []
    for name, fmt, read in FILES:
        r = read(DIR / name)
        r["vars"].sort(key=lambda v: v["name"])
        out.append({"file": name, "format": fmt, **r})
    return {"about": "NetCDF-4, HDF5, Zarr and GRIB files as their own libraries read them (docs/adr/0645).",
            "files": out}


def main():
    if "--files" in sys.argv:
        DIR.mkdir(parents=True, exist_ok=True)
        write_netcdf4()
        write_hdf5()
        write_zarr()
        write_grib()
    if "--files" in sys.argv or "--mesh-files" in sys.argv:
        write_mesh_copies()
    made = cases()
    path = DIR / "cases.json"
    text = json.dumps(made, ensure_ascii=False, indent=1, sort_keys=True) + "\n"
    if "--check" in sys.argv:
        if path.read_text() != text:
            print(f"{path}: güncel değil; --files ya da --check'siz çalıştırın")
            sys.exit(1)
        print(f"{path}: {len(made['files'])} dosya; güncel.")
        return
    path.write_text(text)
    n = sum(len(f["vars"]) for f in made["files"])
    print(f"{path}: {len(made['files'])} dosya, {n} değişken yazıldı.")


if __name__ == "__main__":
    main()
