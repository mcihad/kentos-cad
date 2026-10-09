"""kentos.services (docs/adr/0208 §15) against a server of the test's own on the loopback: a WMS's
capabilities, a GeoJSON address and an ArcGIS layer in pages behind a key."""

from __future__ import annotations

import json
import threading
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from typing import Any
from urllib.parse import parse_qs, urlsplit

from kentos import cad, services

WMS = """<?xml version="1.0" encoding="UTF-8"?>
<WMS_Capabilities version="1.3.0" xmlns="http://www.opengis.net/wms" xmlns:xlink="http://www.w3.org/1999/xlink">
  <Service><Name>WMS</Name><Title>Deneme WMS</Title></Service>
  <Capability>
    <Request>
      <GetCapabilities><Format>text/xml</Format><DCPType><HTTP><Get><OnlineResource xlink:href="{base}/wms?"/></Get></HTTP></DCPType></GetCapabilities>
      <GetMap><Format>image/png</Format><Format>image/jpeg</Format><DCPType><HTTP><Get><OnlineResource xlink:href="{base}/wms?"/></Get></HTTP></DCPType></GetMap>
    </Request>
    <Layer>
      <Title>Kök</Title>
      <CRS>EPSG:3857</CRS><CRS>EPSG:5255</CRS>
      <Layer queryable="1"><Name>parsel</Name><Title>Parseller</Title>
        <EX_GeographicBoundingBox><westBoundLongitude>32.8</westBoundLongitude><eastBoundLongitude>32.9</eastBoundLongitude><southBoundLatitude>39.9</southBoundLatitude><northBoundLatitude>40.0</northBoundLatitude></EX_GeographicBoundingBox>
        <Style><Name>default</Name><Title>Varsayılan</Title></Style>
      </Layer>
    </Layer>
  </Capability>
</WMS_Capabilities>"""

# Kızılay in WGS 84 and in TUREF / TM33 (pyproj): 32.8541 E, 39.9208 N.
KIZILAY = (32.8541, 39.9208)
TM33 = (487_526.0, 4_420_745.0)

GEOJSON = {
    "type": "FeatureCollection",
    "features": [
        {"type": "Feature", "properties": {"ad": "Kızılay", "no": "1"}, "geometry": {"type": "Point", "coordinates": list(KIZILAY)}},
        {"type": "Feature", "properties": {"ad": "Güvenpark", "no": "2"}, "geometry": {"type": "Point", "coordinates": [32.8533, 39.9198]}},
        {"type": "Feature", "properties": {"ad": "Boş", "no": "3"}, "geometry": None},
    ],
}

ARCGIS_LAYER = {"type": "Feature Layer", "name": "Duraklar", "geometryType": "esriGeometryPoint", "maxRecordCount": 2000, "supportedQueryFormats": "JSON, geoJSON"}


def point(x: float, y: float, no: str) -> dict[str, Any]:
    return {"type": "Feature", "properties": {"NO": no}, "geometry": {"type": "Point", "coordinates": [x, y]}}


class Handler(BaseHTTPRequestHandler):
    """The test's services; an ArcGIS request without the key is refused (401)."""

    def log_message(self, format: str, *args: Any) -> None:
        pass

    def answer(self, status: int, body: str, media: str) -> None:
        data = body.encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", media)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self) -> None:
        url = urlsplit(self.path)
        q = {k.lower(): v[0] for k, v in parse_qs(url.query).items()}
        base = f"http://127.0.0.1:{self.server.server_address[1]}"
        if url.path == "/wms":
            self.answer(200, WMS.replace("{base}", base), "text/xml")
        elif url.path == "/veri.geojson":
            self.answer(200, json.dumps(GEOJSON), "application/geo+json")
        elif url.path.startswith("/arcgis/"):
            if q.get("apikey") != "gizli":
                self.answer(401, '{"error":{"code":401,"message":"anahtar yok"}}', "application/json")
            elif url.path.endswith("/FeatureServer/0"):
                self.answer(200, json.dumps(ARCGIS_LAYER), "application/json")
            elif url.path.endswith("/FeatureServer/0/query"):
                if q.get("resultoffset", "0") == "0":
                    page = {"type": "FeatureCollection", "exceededTransferLimit": True, "features": [point(TM33[0], TM33[1], "1"), point(TM33[0] + 10, TM33[1], "2")]}
                else:
                    page = {"type": "FeatureCollection", "features": [point(TM33[0] + 20, TM33[1], "3")]}
                self.answer(200, json.dumps(page), "application/geo+json")
            else:
                self.answer(404, "yok", "text/plain")
        else:
            self.answer(404, "yok", "text/plain")


class Services(unittest.TestCase):
    server: ThreadingHTTPServer
    base: str

    @classmethod
    def setUpClass(cls) -> None:
        cls.server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        cls.base = f"http://127.0.0.1:{cls.server.server_address[1]}"
        threading.Thread(target=cls.server.serve_forever, daemon=True).start()

    @classmethod
    def tearDownClass(cls) -> None:
        cls.server.shutdown()
        cls.server.server_close()

    def doc(self) -> cad.Document:
        return cad.Document.new("Servisler", srid=5255)

    def service_layers(self, doc: cad.Document) -> list[dict[str, Any]]:
        return [n for n in json.loads(doc._session.layers()) if n.get("service")]

    def test_a_ready_basemap_goes_to_the_bottom_and_another_takes_its_place(self) -> None:
        ids = {p["id"] for p in services.presets()}
        self.assertTrue({"osm-standard", "esri-imagery", "ofm-liberty"} <= ids)
        doc = self.doc()
        first = services.add_basemap(doc, "osm-standard")
        roots = json.loads(doc._session.layers())
        self.assertEqual(roots[-1]["id"], first)
        self.assertEqual(roots[-1]["service"]["preset"], "osm-standard")
        again = services.add_basemap(doc, "osm-topo")
        self.assertEqual(again, first)
        self.assertEqual(len(self.service_layers(doc)), 1)
        self.assertEqual(json.loads(doc._session.layers())[-1]["service"]["preset"], "osm-topo")
        with self.assertRaises(ValueError):
            services.add_basemap(doc, "yok")

    def test_a_wms_is_read_and_added_over_the_basemap_in_the_projects_system(self) -> None:
        offer = services.read_service("wms", f"{self.base}/wms")
        self.assertEqual(offer["title"], "Deneme WMS")
        self.assertIn("parsel", [i["id"] for i in offer["items"]])
        doc = self.doc()
        services.add_basemap(doc, "osm-standard")
        layer = services.add_service(doc, "wms", f"{self.base}/wms", items=["parsel"])
        roots = json.loads(doc._session.layers())
        node = next(n for n in roots if n["id"] == layer)
        self.assertEqual(node["name"], "Parseller")
        service = node["service"]
        self.assertEqual((service["kind"], service["layers"], service["srid"]), ("wms", ["parsel"], 5255))
        # A clear service sits over the basemap at the bottom.
        self.assertEqual(roots[-2]["id"], layer)
        self.assertEqual(roots[-1]["service"]["preset"], "osm-standard")

    def test_a_geojson_address_comes_into_the_project_and_what_had_no_geometry_is_said(self) -> None:
        doc = self.doc()
        taken = services.import_features(doc, "geojson", f"{self.base}/veri.geojson", name="Noktalar")
        self.assertEqual(taken.taken, 2)
        self.assertEqual(sum(i["count"] for i in taken.skipped), 1)
        self.assertTrue(taken.words.startswith("2 nesne alındı. 1 nesne servisten geometrisi boş geldi"), taken.words)
        node = next(n for n in json.loads(doc._session.layers()) if n["id"] == taken.layer)
        self.assertEqual(node["name"], "Noktalar")
        self.assertEqual(node["feed"]["kind"], "geojson")
        self.assertIn("fetched", node["feed"])
        self.assertEqual([f["name"] for f in node["fields"]], ["ad", "no"])
        page = json.loads(doc._session.entities(taken.layer))
        first = next(i["entity"] for i in page["items"] if i["entity"]["attrs"]["ad"] == "Kızılay")
        self.assertAlmostEqual(first["p"]["x"], TM33[0], delta=1.5)
        self.assertAlmostEqual(first["p"]["y"], TM33[1], delta=1.5)
        # One undo step takes the layer and its objects back.
        self.assertEqual(doc.undo(), "Servisten veri al")
        self.assertFalse(any(n["id"] == taken.layer for n in json.loads(doc._session.layers())))

    def test_an_arcgis_layer_behind_a_key_is_taken_in_pages_and_the_key_stays_out_of_the_drawing(self) -> None:
        url = f"{self.base}/arcgis/rest/services/Duraklar/FeatureServer/0"
        connection = {"id": "anahtar", "name": "Deneme", "origin": self.base, "auth": "query", "names": ["apikey"]}
        with self.assertRaises(services.ServiceError):
            services.read_feed("arcgis", url)
        with self.assertRaises(services.ServiceError) as missing:
            services.read_feed("arcgis", url, connection=connection)
        self.assertIn("Deneme", str(missing.exception))
        secret = {"id": "anahtar", "values": ["gizli"]}
        doc = self.doc()
        taken = services.import_features(doc, "arcgis", url, connection=connection, secret=secret, key="NO")
        self.assertEqual((taken.taken, taken.dropped, taken.skipped), (3, 0, []))
        self.assertEqual(taken.words, "3 nesne alındı.")
        settings = json.loads(doc._session.summary())["settings"]
        self.assertEqual(settings["connections"], [connection])
        self.assertNotIn("gizli", doc._session.summary())
        self.assertNotIn(b"gizli", doc.to_bytes())
        node = next(n for n in json.loads(doc._session.layers()) if n["id"] == taken.layer)
        self.assertEqual((node["feed"]["key"], node["feed"]["connection"], node["feed"]["srid"]), ("NO", "anahtar", 5255))


if __name__ == "__main__":
    unittest.main()
