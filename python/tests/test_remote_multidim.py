"""Addressed NetCDF-4 and Zarr meshes read from Python (docs/adr/0645 §5).

The Python module reads an address with Python's own urllib (`kentos._remote.Http`). Here a local HTTP server that
answers ranges (and 404 for an object a store does not hold) serves fixtures/multidim/v2; the shared cases' mesh
drawing (fixtures/commands/v1/cad.meshes.region.json) gets layered.nc's NetCDF-4 and Zarr copies as addresses, and
`cad.meshes.region` and `.query` answer them as they answer the files.
"""

from __future__ import annotations

import http.server
import json
import re
import threading
import unittest
from pathlib import Path

import kentos.cad as cad

ROOT = Path(__file__).resolve().parents[2]
SERVED = ROOT / "fixtures/multidim/v2"
RING = [(499999.0, 4419999.0), (500021.0, 4419999.0), (500021.0, 4420011.0), (499999.0, 4420011.0)]


class Ranges(http.server.BaseHTTPRequestHandler):
    def do_GET(self) -> None:  # noqa: N802
        path = (SERVED / self.path.lstrip("/").split("?")[0]).resolve()
        if SERVED not in path.parents or not path.is_file():
            self.send_error(404)
            return
        data = path.read_bytes()
        asked = re.fullmatch(r"bytes=(\d+)-(\d+)", self.headers.get("Range") or "")
        if asked:
            a, b = int(asked[1]), min(int(asked[2]), len(data) - 1)
            self.send_response(206)
            self.send_header("Content-Range", f"bytes {a}-{b}/{len(data)}")
            body = data[a:b + 1]
        else:
            self.send_response(200)
            body = data
        self.send_header("Content-Length", str(len(body)))
        self.send_header("ETag", '"v1"')
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *_: object) -> None:
        pass


class RemoteMultidimTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Ranges)
        cls.base = f"http://127.0.0.1:{cls.server.server_address[1]}"
        threading.Thread(target=cls.server.serve_forever, daemon=True).start()

    @classmethod
    def tearDownClass(cls) -> None:
        cls.server.shutdown()

    def drawing(self) -> cad.Document:
        setup = json.loads((ROOT / "fixtures/commands/v1/cad.meshes.region.json").read_text("utf-8"))["setup"]
        for e in setup["entities"]:
            if e["id"] == 17:
                del e["file"]
                e["url"] = f"{self.base}/layered-nc4.nc"
            if e["id"] == 18:
                del e["file"]
                e["url"] = f"{self.base}/layered.zarr"
        text = re.sub(r"\$fixture:([^\s\"]+)", lambda m: str(ROOT / "fixtures" / m.group(1)), json.dumps(setup))
        return cad.Document.from_bytes(text.encode())

    def uid(self, doc: cad.Document, id_: int) -> str:
        return next(r.uid for r in doc.entities(page_size=1000) if r.entity.id == id_)

    def test_addressed_copies_answer_as_the_file(self) -> None:
        doc = self.drawing()
        file, nc4, zarr = (self.uid(doc, i) for i in (5, 17, 18))
        got = cad.meshes.region(doc, ring=RING, uids=[file, nc4, zarr], faces=True)
        self.assertEqual(len(got.meshes), 3, got)
        first = got.meshes[0]
        self.assertEqual(first.count, 2)
        for other in got.meshes[1:]:
            self.assertEqual((other.count, other.faces, other.datasets), (first.count, first.faces, first.datasets))
        at = (500005.0, 4420005.0)
        answers = [cad.meshes.query(doc, at=at, uids=[u]) for u in (file, nc4, zarr)]
        self.assertTrue(answers[0].face is not None)
        for a in answers[1:]:
            self.assertEqual(a.face.face, answers[0].face.face)
            self.assertEqual(a.values, answers[0].values)

    def test_a_missing_address_is_said(self) -> None:
        setup = json.loads((ROOT / "fixtures/commands/v1/cad.meshes.region.json").read_text("utf-8"))["setup"]
        for e in setup["entities"]:
            if e["id"] == 17:
                del e["file"]
                e["url"] = f"{self.base}/yok.nc"
        text = re.sub(r"\$fixture:([^\s\"]+)", lambda m: str(ROOT / "fixtures" / m.group(1)), json.dumps(setup))
        doc = cad.Document.from_bytes(text.encode())
        got = doc.call("cad.meshes.region", {"ring": [{"x": x, "y": y} for x, y in RING], "uids": [self.uid(doc, 17)]})
        self.assertEqual([w.code for w in got.warnings], ["mesh_source_unreadable"])
        self.assertIn("yok.nc", got.warnings[0].message)


if __name__ == "__main__":
    unittest.main()
