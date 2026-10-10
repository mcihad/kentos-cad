"""NetCDF classic files written and read here, from Unidata's “NetCDF Classic and 64-bit Offset Format” and “CDF-5” notes,
without KentOS code and without libnetcdf: the reference's own reader and writer (docs/adr/0243). GDAL's multidimensional API
(libnetcdf) cross-checks them in multidim_cases.py.

A file is a header (magic 'CDF' and its version, the record count, the dimensions, the global attributes, the variables) and
the data, big-endian. Version 1 has 32-bit offsets, 2 64-bit offsets, 5 64-bit sizes and the unsigned and 64-bit types.
"""

import struct

NC_DIMENSION, NC_VARIABLE, NC_ATTRIBUTE = 10, 11, 12
TYPES = {  # nc_type → (struct code, size)
    1: ("b", 1), 2: ("c", 1), 3: ("h", 2), 4: ("i", 4), 5: ("f", 4), 6: ("d", 8),
    7: ("B", 1), 8: ("H", 2), 9: ("I", 4), 10: ("q", 8), 11: ("Q", 8),
}
NAMES = {"byte": 1, "char": 2, "short": 3, "int": 4, "float": 5, "double": 6, "ubyte": 7, "ushort": 8, "uint": 9, "int64": 10,
         "uint64": 11}
STREAMING = 0xFFFFFFFF


def pad4(n):
    return (n + 3) // 4 * 4


class Var:
    def __init__(self, name, dims, kind, values, attrs=None):
        self.name, self.dims, self.kind, self.values = name, list(dims), NAMES[kind] if isinstance(kind, str) else kind, values
        self.attrs = attrs or {}


def _attr_bytes(value, five):
    """An attribute's type, count and padded values: str → char, int list → int, float list → double, (kind, list) → that type."""
    if isinstance(value, str):
        data = value.encode("utf-8")
        kind, n = 2, len(data)
    else:
        if isinstance(value, tuple):
            kind, items = NAMES[value[0]], list(value[1])
        else:
            items = value if isinstance(value, list) else [value]
            kind = 6 if any(isinstance(v, float) for v in items) else 4
        code, size = TYPES[kind]
        data = struct.pack(">" + code * len(items), *items)
        n = len(items)
    out = struct.pack(">i", kind) + (struct.pack(">q", n) if five else struct.pack(">i", n)) + data
    return out + b"\0" * (pad4(len(data)) - len(data))


def write(path, version, dims, gattrs, variables, numrecs=0, streaming=False):
    """A classic file of `version` (1, 2, 5): `dims` [(name, length)] (length 0: the record dimension), global attributes, `variables`
    (each value a flat list in C order; a record variable's values record after record); written at `path` (none: only returned).
    The last fixed variable's values may be None in a file without records: the file then stops where they would begin (a file
    of gigabytes whose header and coordinates alone are kept; its size is that variable's begin and size)."""
    five = version == 5
    nelems = (lambda n: struct.pack(">q", n)) if five else (lambda n: struct.pack(">i", n))

    def name(s):
        b = s.encode("utf-8")
        return nelems(len(b)) + b + b"\0" * (pad4(len(b)) - len(b))

    def att_list(attrs):
        if not attrs:
            return struct.pack(">i", 0) + nelems(0)
        out = struct.pack(">i", NC_ATTRIBUTE) + nelems(len(attrs))
        for k, v in attrs.items():
            out += name(k) + _attr_bytes(v, five)
        return out

    dim_len = {d: n for d, n in dims}
    dim_id = {d: k for k, (d, _) in enumerate(dims)}
    record = next((d for d, n in dims if n == 0), None)

    def is_record(v):
        return bool(v.dims) and v.dims[0] == record

    def vsize(v):
        return pad4(TYPES[v.kind][1] * _slab(v, dim_len, record))

    rec_vars = [v for v in variables if is_record(v)]
    one = len(rec_vars) == 1
    head = b"CDF" + bytes([version])
    if streaming:
        head += struct.pack(">Q", 0xFFFFFFFFFFFFFFFF) if five else struct.pack(">I", STREAMING)
    else:
        head += struct.pack(">q", numrecs) if five else struct.pack(">i", numrecs)
    if dims:
        head += struct.pack(">i", NC_DIMENSION) + nelems(len(dims)) + b"".join(name(d) + nelems(n) for d, n in dims)
    else:
        head += struct.pack(">i", 0) + nelems(0)
    head += att_list(gattrs)
    offset = ">i" if version == 1 else ">q"

    def var_entries(begins):
        if not variables:
            return struct.pack(">i", 0) + nelems(0)
        out = struct.pack(">i", NC_VARIABLE) + nelems(len(variables))
        for v, begin in zip(variables, begins):
            out += name(v.name) + nelems(len(v.dims)) + b"".join(nelems(dim_id[d]) for d in v.dims)
            # vsize is unsigned in 32 bits; past 2³² − 4 bytes it is 2³² − 1 (the format notes' “Note on vsize”).
            size = vsize(v)
            out += att_list(v.attrs) + struct.pack(">i", v.kind) + (
                struct.pack(">q", size) if five else struct.pack(">I", size if size <= 0xFFFFFFFC else 0xFFFFFFFF))
            out += struct.pack(offset, begin)
        return out

    at = len(head) + len(var_entries([0] * len(variables)))
    begins = {}
    for v in variables:
        if not is_record(v):
            begins[v.name] = at
            at += vsize(v)
    for v in rec_vars:
        begins[v.name] = at
        at += TYPES[v.kind][1] * _slab(v, dim_len, record) if one else vsize(v)
    out = bytearray(head + var_entries([begins[v.name] for v in variables]))
    fixed = [v for v in variables if not is_record(v)]
    for v in fixed:
        if v.values is None:
            assert v is fixed[-1] and not rec_vars, "only the last fixed variable of a file without records may be left out"
            break
        data = _pack(v)
        out += data + b"\0" * (vsize(v) - len(data))
    for r in range(numrecs):
        for v in rec_vars:
            slab = _slab(v, dim_len, record)
            data = _pack(v, r * slab, (r + 1) * slab)
            out += data if one else data + b"\0" * (vsize(v) - len(data))
    if path is not None:
        with open(path, "wb") as f:
            f.write(out)
    return bytes(out)


def _slab(v, dim_len, record):
    n = 1
    for d in v.dims[1:] if v.dims and v.dims[0] == record else v.dims:
        n *= dim_len[d]
    return n


def _pack(v, a=None, b=None):
    vals = v.values if a is None else v.values[a:b]
    code, _ = TYPES[v.kind]
    if code == "c":
        return bytes(vals) if isinstance(vals, (bytes, bytearray)) else bytes(vals)
    return struct.pack(">" + code * len(vals), *vals)


class Header:
    def __init__(self):
        self.version = 0
        self.numrecs = 0
        self.dims = []      # [(name, length)]
        self.gattrs = {}
        self.vars = []      # dicts: name, dims (ids), attrs, kind, vsize, begin


def read_header(data):
    if data[:3] != b"CDF" or data[3] not in (1, 2, 5):
        raise ValueError("not classic NetCDF")
    h = Header()
    h.version = data[3]
    five = h.version == 5
    pos = 4

    def take(fmt):
        nonlocal pos
        size = struct.calcsize(">" + fmt)
        v = struct.unpack_from(">" + fmt, data, pos)[0]
        pos += size
        return v

    def n_elems():
        return take("q") if five else take("i")

    def name():
        nonlocal pos
        n = n_elems()
        s = data[pos:pos + n].decode("utf-8")
        pos += pad4(n)
        return s

    def attrs():
        nonlocal pos
        tag = take("i")
        n = n_elems()
        out = {}
        if tag == 0:
            return out
        for _ in range(n):
            k = name()
            kind = take("i")
            m = n_elems()
            code, size = TYPES[kind]
            raw = data[pos:pos + m * size]
            pos += pad4(m * size)
            out[k] = raw.decode("utf-8") if code == "c" else list(struct.unpack(">" + code * m, raw))
        return out

    h.numrecs = take("Q") if five else take("I")
    if five and h.numrecs == 0xFFFFFFFFFFFFFFFF:
        h.numrecs = STREAMING
    tag = take("i")
    n = n_elems()
    for _ in range(n if tag == NC_DIMENSION else 0):
        h.dims.append((name(), n_elems()))
    h.gattrs = attrs()
    tag = take("i")
    n = n_elems()
    for _ in range(n if tag == NC_VARIABLE else 0):
        v = {"name": name()}
        nd = n_elems()
        v["dims"] = [n_elems() for _ in range(nd)]
        v["attrs"] = attrs()
        v["kind"] = take("i")
        v["vsize"] = take("q") if five else take("I")
        v["begin"] = take("q") if h.version != 1 else take("i")
        h.vars.append(v)
    if h.numrecs == STREAMING:
        rec_vars = [v for v in h.vars if v["dims"] and h.dims[v["dims"][0]][1] == 0]
        if rec_vars:
            recsize = sum(v["vsize"] for v in rec_vars) if len(rec_vars) > 1 else rec_vars[0]["vsize"]
            first = min(v["begin"] for v in rec_vars)
            h.numrecs = (len(data) - first) // recsize if recsize else 0
        else:
            h.numrecs = 0
    return h


def read_var(data, h, name):
    """A variable's values (flat, C order; record variables record after record) as Python numbers."""
    v = next(x for x in h.vars if x["name"] == name)
    code, size = TYPES[v["kind"]]
    dims = [h.dims[d] for d in v["dims"]]
    record = bool(dims) and dims[0][1] == 0
    slab = 1
    for _, n in (dims[1:] if record else dims):
        slab *= n
    if not record:
        raw = data[v["begin"]:v["begin"] + slab * size]
        return list(raw) if code == "c" else list(struct.unpack(">" + code * slab, raw))
    rec_vars = [x for x in h.vars if x["dims"] and h.dims[x["dims"][0]][1] == 0]
    recsize = sum(x["vsize"] for x in rec_vars) if len(rec_vars) > 1 else slab * size
    out = []
    for r in range(h.numrecs):
        at = v["begin"] + r * recsize
        raw = data[at:at + slab * size]
        out.extend(list(raw) if code == "c" else struct.unpack(">" + code * slab, raw))
    return out
