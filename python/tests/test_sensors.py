"""Sensor observations from a SensorThings address, read from Python (docs/adr/0646 §4, §7).

A local HTTP server answers a Datastream's Observations in two pages, the first naming the second by
`@iot.nextLink`; `cad.sensors.ingest` follows it (or stops at `pages`), `cad.sensors.latest` then says each asset's
latest value and its age. The drawing is the shared cases' (fixtures/commands/v1/cad.sensors.ingest.json).
"""

from __future__ import annotations

import http.server
import json
import threading
import unittest
from pathlib import Path

import kentos.cad as cad

ROOT = Path(__file__).resolve().parents[2]


def page(base: str, n: int) -> bytes:
    pages = {
        1: {"@iot.nextLink": f"{base}/v1.1/Datastreams(3)/Observations?$skip=2",
            "value": [{"phenomenonTime": "2024-06-01T11:00:00Z", "result": 4.5, "resultQuality": "İyi"},
                      {"phenomenonTime": "2024-06-01T11:10:00Z", "result": 4.75}]},
        2: {"value": [{"phenomenonTime": "2024-06-01T11:20:00Z/2024-06-01T11:25:00Z", "result": 5},
                      {"phenomenonTime": "2024-06-01T11:05:00Z", "result": 4.6}]},
    }
    return json.dumps(pages[n]).encode()


class Observations(http.server.BaseHTTPRequestHandler):
    def do_GET(self) -> None:  # noqa: N802
        base = f"http://127.0.0.1:{self.server.server_address[1]}"
        if self.path == "/v1.1/Datastreams(3)/Observations":
            body = page(base, 1)
        elif self.path == "/v1.1/Datastreams(3)/Observations?$skip=2":
            body = page(base, 2)
        else:
            self.send_error(404)
            return
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *_: object) -> None:
        pass


class SensorTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Observations)
        cls.base = f"http://127.0.0.1:{cls.server.server_address[1]}"
        threading.Thread(target=cls.server.serve_forever, daemon=True).start()

    @classmethod
    def tearDownClass(cls) -> None:
        cls.server.shutdown()

    def drawing(self) -> cad.Document:
        setup = json.loads((ROOT / "fixtures/commands/v1/cad.sensors.ingest.json").read_text("utf-8"))["setup"]
        return cad.Document.from_bytes(json.dumps(setup).encode())

    def test_every_page_is_followed(self) -> None:
        doc = self.drawing()
        got = cad.sensors.ingest(doc, layer="olcum", value_field="deger", quality_field="kalite",
                                 url=f"{self.base}/v1.1/Datastreams(3)/Observations", asset="S9",
                                 received_at="2024-06-01T11:30:00Z")
        self.assertEqual((len(got.added), got.late, got.pages, got.more), (4, 1, 2, False), got)
        latest = cad.sensors.latest(doc, layer="olcum", value_field="deger", quality_field="kalite",
                                    at="2024-06-01T11:40:00Z", stale_after=600, expected_assets=["S9", "S99"])
        s9 = next(a for a in latest.assets if a.asset == "S9")
        self.assertEqual((s9.observed_at, s9.value, s9.age_seconds, s9.stale),
                         ("2024-06-01T11:20:00Z", "5", 1200.0, True))
        self.assertEqual(latest.missing, ["S99"])

    def test_pages_stop_the_reading(self) -> None:
        doc = self.drawing()
        got = cad.sensors.ingest(doc, layer="olcum", value_field="deger",
                                 url=f"{self.base}/v1.1/Datastreams(3)/Observations", asset="S9", pages=1,
                                 received_at="2024-06-01T11:30:00Z")
        self.assertEqual((len(got.added), got.pages, got.more), (2, 1, True), got)

    def test_an_address_not_there(self) -> None:
        doc = self.drawing()
        with self.assertRaises(cad.CommandFailed) as caught:
            cad.sensors.ingest(doc, layer="olcum", value_field="deger", url=f"{self.base}/yok", asset="S9",
                               received_at="2024-06-01T11:30:00Z")
        self.assertEqual(caught.exception.code, "source_unreachable")


if __name__ == "__main__":
    unittest.main()
