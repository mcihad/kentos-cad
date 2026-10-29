"""The server's commands through a Connection, against a stand-in server that
records what it is sent: the session cookie, the program's header, the
envelope, and the server's refusals as typed errors (docs/adr/0131)."""

from __future__ import annotations

import json
import threading
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any

import kentos.cad as cad

ROOT = Path(__file__).resolve().parents[2]
PROJECT = json.loads((ROOT / "fixtures/cloud/v1/catalog.json").read_text(encoding="utf-8"))["project"]
TENANT = PROJECT["tenantId"]


class Server:
    """Answers as kentosd would; `replies` maps a path to (status, body)."""

    def __init__(self) -> None:
        self.requests: list[dict[str, Any]] = []
        self.replies: dict[str, tuple[int, Any, dict[str, str]]] = {}
        outer = self

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *args: Any) -> None:
                pass

            def answer(self) -> None:
                length = int(self.headers.get("Content-Length") or 0)
                body = self.rfile.read(length) if length else b""
                is_json = "json" in (self.headers.get("Content-Type") or "")
                outer.requests.append(
                    {
                        "method": self.command,
                        "path": self.path,
                        "headers": {k.lower(): v for k, v in self.headers.items()},
                        "body": (json.loads(body) if body else None) if is_json or not body else body,
                    }
                )
                status, reply, headers = outer.replies.get(self.path.split("?")[0], (404, {"error": "not_found", "message": "Yok."}, {}))
                data = json.dumps(reply).encode()
                self.send_response(status)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(data)))
                for k, v in headers.items():
                    self.send_header(k, v)
                self.end_headers()
                self.wfile.write(data)

            do_GET = answer
            do_POST = answer
            do_PUT = answer

        self.httpd = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self.url = f"http://127.0.0.1:{self.httpd.server_address[1]}"
        self.thread = threading.Thread(target=self.httpd.serve_forever, daemon=True)
        self.thread.start()

    def close(self) -> None:
        self.httpd.shutdown()
        self.httpd.server_close()


class ConnectionTests(unittest.TestCase):
    def setUp(self) -> None:
        self.server = Server()
        self.server.replies["/v1/auth/login"] = (
            200,
            {"user": {"id": "u1", "displayName": "Ayşe", "method": "local"}, "memberships": []},
            {"Set-Cookie": "kentos_session=tok-123; Path=/; HttpOnly; SameSite=Strict; Max-Age=3600"},
        )

    def tearDown(self) -> None:
        self.server.close()

    def test_dimension_recipe_create_and_changes_refuse_old_health_before_post(self) -> None:
        conn = cad.Connection(self.server.url, token="tok-123")
        cases = [
            (conn.tenant(TENANT), "project.create", 1, {"dimensionSets": []}),
            (conn.tenant(TENANT), "project.create", 1, {"dimensionSets": [{"id": "recipe"}]}),
            (conn.project(PROJECT["id"], tenant=TENANT), "project.changes", 2, {"features": [], "project": {"dimensionSets": []}}),
            (conn.project(PROJECT["id"], tenant=TENANT), "project.changes", 2, {"features": [], "project": {"dimensionSets": None}}),
        ]
        for health in [{"status": "ok"}, {"projectChangesVersions": [1]}, {"projectChangesVersions": None}]:
            self.server.replies["/v1/health"] = (200, health, {})
            for target, command, version, payload in cases:
                with self.subTest(command=command, health=health, payload=payload):
                    before = len(self.server.requests)
                    with self.assertRaises(cad.ServerError) as error:
                        target._command(command, version, payload, None, "same-key")
                    self.assertEqual(error.exception.code, "server_feature_unavailable")
                    self.assertEqual(error.exception.path, "dimensionSets")
                    self.assertEqual(len(self.server.requests), before + 1)
                    self.assertEqual(self.server.requests[-1]["method"], "GET")
                    self.assertEqual(self.server.requests[-1]["path"], "/v1/health")
        del self.server.replies["/v1/health"]
        with self.assertRaises(cad.ServerError) as error:
            cases[0][0]._command("project.create", 1, {"dimensionSets": []}, None, "same-key")
        self.assertEqual(error.exception.code, "server_feature_unavailable")
        self.assertTrue(conn.signed_in)

    def test_dimension_recipe_supported_health_preserves_key_versions_and_session(self) -> None:
        conn = cad.Connection(self.server.url, token="tok-123")
        self.server.replies["/v1/health"] = (200, {"projectChangesVersions": [1, 2]}, {})
        path = f"/v1/tenants/{TENANT}/projects/{PROJECT['id']}/commands"
        self.server.replies[path] = (200, {"received": True}, {})
        target = conn.project(PROJECT["id"], tenant=TENANT)
        payload = {"features": [], "project": {"dimensionSets": []}}
        self.assertEqual(target._command("project.changes", 2, payload, {"@project": "8"}, "same-key"), {"received": True})
        self.assertEqual([r["method"] for r in self.server.requests], ["GET", "POST"])
        sent = self.server.requests[-1]
        self.assertEqual(sent["body"]["version"], 2)
        self.assertEqual(sent["body"]["input"], payload)
        self.assertEqual(sent["body"]["idempotencyKey"], "same-key")
        self.assertEqual(sent["body"]["expectedVersions"], {"@project": "8"})
        for request in self.server.requests:
            self.assertEqual(request["headers"]["cookie"], "kentos_session=tok-123")
            self.assertEqual(request["headers"]["x-kentos-client"], "python")
        self.assertEqual(conn.timeout, 30.0)
        # Ordinary queued requests use v1 without probing old-server capabilities.
        self.server.requests.clear()
        target._command("project.changes", 1, {"features": []}, None, "old-key")
        self.assertEqual(len(self.server.requests), 1)
        self.assertEqual(self.server.requests[0]["method"], "POST")
        self.assertEqual(self.server.requests[0]["body"]["version"], 1)

    def test_capability_probe_uses_the_existing_unauthorized_session_flow(self) -> None:
        conn = cad.Connection(self.server.url, token="expired")
        self.server.replies["/v1/health"] = (401, {"error": "unauthenticated", "message": "Oturum bitti"}, {})
        target = conn.project(PROJECT["id"], tenant=TENANT)
        with self.assertRaises(cad.NotSignedIn):
            target._command("project.changes", 2, {"features": [], "project": {"dimensionSets": []}}, None, "same-key")
        self.assertFalse(conn.signed_in)
        self.assertEqual([r["method"] for r in self.server.requests], ["GET"])

    def test_a_command_goes_as_the_servers_envelope_with_the_session(self) -> None:
        conn = cad.Connection.login(self.server.url, " ayse ", "parola")
        self.assertTrue(conn.signed_in)
        login = self.server.requests[0]
        self.assertEqual(login["body"], {"login": "ayse", "password": "parola"})
        self.assertNotIn("cookie", login["headers"])

        path = f"/v1/tenants/{TENANT}/projects/{PROJECT['id']}/commands"
        self.server.replies[path] = (200, {"project": PROJECT, "changed": True, "eventSeq": "12"}, {})
        project = conn.project(cad.ProjectSummary.from_json(PROJECT))
        answer = cad.project.rename(project, name="Ada 101 (revize)", expected_versions={"@project": "7"})
        self.assertIsInstance(answer, cad.ProjectCatalogChange)
        self.assertTrue(answer.changed)
        self.assertFalse(answer.replayed)
        self.assertEqual(answer.project.name, "Ada 101")
        self.assertEqual(answer.project.access.role, cad.ProjectRole.OWNER)

        sent = self.server.requests[-1]
        self.assertEqual(sent["headers"]["cookie"], "kentos_session=tok-123")
        self.assertEqual(sent["headers"]["x-kentos-client"], "python")
        envelope = sent["body"]
        self.assertEqual(envelope["commandName"], "project.rename")
        self.assertEqual(envelope["version"], 1)
        self.assertEqual((envelope["tenantId"], envelope["projectId"]), (TENANT, PROJECT["id"]))
        self.assertEqual(envelope["input"], {"name": "Ada 101 (revize)"})
        self.assertEqual(envelope["expectedVersions"], {"@project": "7"})
        self.assertTrue(envelope["idempotencyKey"])

    def test_a_new_project_goes_to_the_organisation(self) -> None:
        conn = cad.Connection.login(self.server.url, "ayse", "parola")
        info = dict(PROJECT)
        self.server.replies[f"/v1/tenants/{TENANT}/commands"] = (201, info, {})
        key = "0199a1b2-3c4d-7e5f-8a9b-0c1d2e3f4a5b"
        try:
            cad.project.create(conn.tenant(TENANT), name="Ada 102", idempotency_key=key, **_create_fields())
        except cad.DecodeError:
            pass  # the stand-in's answer is a summary, not a ProjectInfo: the envelope is what is checked
        envelope = self.server.requests[-1]["body"]
        self.assertEqual((envelope["commandName"], envelope["projectId"]), ("project.create", ""))
        self.assertEqual(envelope["idempotencyKey"], key)

    def test_the_servers_refusals_are_typed(self) -> None:
        conn = cad.Connection.login(self.server.url, "ayse", "parola")
        path = f"/v1/tenants/{TENANT}/projects/{PROJECT['id']}/commands"
        project = conn.project(PROJECT["id"], tenant=TENANT)
        self.server.replies[path] = (
            409,
            {"error": "conflict", "message": "Proje değişti.", "revision": "43", "retryable": False},
            {},
        )
        with self.assertRaises(cad.ServerConflict) as caught:
            cad.project.rename(project, name="x")
        self.assertEqual((caught.exception.status, caught.exception.revision), (409, "43"))
        self.server.replies[path] = (403, {"error": "forbidden", "message": "Yetki yok.", "retryable": False}, {})
        with self.assertRaises(cad.Forbidden):
            cad.project.trash(project)
        self.server.replies[path] = (401, {"error": "unauthorized", "message": "Oturum bitti."}, {})
        with self.assertRaises(cad.NotSignedIn):
            cad.project.archive(project)
        self.assertFalse(conn.signed_in, "a 401 forgets the session")

    def test_a_server_job_is_asked_waited_and_cancelled(self) -> None:
        conn = cad.Connection.login(self.server.url, "ayse", "parola")
        project = conn.project(PROJECT["id"], TENANT)
        base = f"/v1/tenants/{TENANT}/projects/{PROJECT['id']}/jobs"
        job = {"id": "j1", "kind": "project.export", "state": "succeeded", "artifact": {"size": "3", "sha256": "0" * 64, "name": "a.kcad"}}
        self.server.replies[base] = (201, {**job, "state": "queued"}, {})
        self.server.replies[f"{base}/j1"] = (200, job, {})
        self.server.replies[f"{base}/j1/cancel"] = (200, {**job, "state": "cancelled"}, {})
        asked = project.export_job(expected_revision="7", request_id="istek-1", timeout_seconds=60)
        self.assertEqual(asked["state"], "queued")
        sent = self.server.requests[-1]
        self.assertEqual((sent["method"], sent["path"]), ("POST", base))
        self.assertEqual(
            sent["body"],
            {"kind": "project.export", "input": {"expectedRevision": "7"}, "requestId": "istek-1", "timeoutSeconds": 60},
        )
        self.assertEqual(sent["headers"]["cookie"], "kentos_session=tok-123")
        # Built again whatever an earlier job of the same state made (docs/adr/0739 §2): `fresh` only when asked.
        project.export_job(request_id="istek-2", fresh=True)
        self.assertEqual(
            self.server.requests[-1]["body"],
            {"kind": "project.export", "input": {}, "requestId": "istek-2", "fresh": True},
        )
        self.assertEqual(project.wait_job("j1", every=0.01)["state"], "succeeded")
        self.assertEqual(project.cancel_job("j1")["state"], "cancelled")
        self.assertEqual(self.server.requests[-1]["path"], f"{base}/j1/cancel")
        self.server.replies[f"{base}/j1"] = (200, {**job, "state": "running"}, {})
        with self.assertRaises(TimeoutError):
            project.wait_job("j1", every=0.01, within=0.05)
        self.server.replies[f"{base}/yok"] = (404, {"error": "not_found", "message": "İş bulunamadı."}, {})
        with self.assertRaises(cad.NotFound):
            project.job("yok")

    def test_a_checkpoint_job_and_the_organisations_job_limit(self) -> None:
        conn = cad.Connection.login(self.server.url, "ayse", "parola")
        project = conn.project(PROJECT["id"], TENANT)
        base = f"/v1/tenants/{TENANT}/projects/{PROJECT['id']}/jobs"
        self.server.replies[base] = (201, {"id": "j2", "kind": "project.checkpoint", "state": "queued"}, {})
        asked = project.checkpoint_job("Teslim", note="ilk", request_id="istek-2")
        self.assertEqual(asked["kind"], "project.checkpoint")
        self.assertEqual(
            self.server.requests[-1]["body"],
            {"kind": "project.checkpoint", "input": {"name": "Teslim", "note": "ilk"}, "requestId": "istek-2"},
        )
        self.server.replies[base] = (
            429,
            {"error": "quota_exceeded", "message": "Kurumunuzun iş sınırına ulaşıldı.", "retryable": True, "retryAfter": 30},
            {},
        )
        with self.assertRaises(cad.ServerError) as refused:
            project.checkpoint_job("Teslim 2")
        self.assertEqual(refused.exception.code, "quota_exceeded")
        self.assertTrue(refused.exception.retryable)

    def test_a_processing_job_is_asked_as_the_command_takes_the_run(self) -> None:
        conn = cad.Connection.login(self.server.url, "ayse", "parola")
        project = conn.project(PROJECT["id"], TENANT)
        base = f"/v1/tenants/{TENANT}/projects/{PROJECT['id']}/jobs"
        running = {
            "id": "j3",
            "kind": "project.processing",
            "state": "running",
            "progress": 0.4,
            "progressText": "Araç çalışıyor: Tampon",
        }
        self.server.replies[base] = (201, {**running, "state": "queued", "progress": 0.0}, {})
        self.server.replies[f"{base}/j3"] = (200, running, {})
        asked = project.processing_job(
            "geometry.buffer",
            parameters={"mesafe": 5},
            selection=["0199aaaa-0000-7000-8000-000000000001"],
            view=[0.0, 0.0, 10.0, 10.0],
            mode="sink",
            expected_revision="7",
            label="Tampon",
            cpu_seconds=60,
            memory_mb=512,
            request_id="istek-3",
            timeout_seconds=120,
        )
        self.assertEqual(asked["kind"], "project.processing")
        self.assertEqual(
            self.server.requests[-1]["body"],
            {
                "kind": "project.processing",
                "input": {
                    "tool": "geometry.buffer",
                    "mode": "sink",
                    "label": "Tampon",
                    "parameters": {"mesafe": 5},
                    "selection": ["0199aaaa-0000-7000-8000-000000000001"],
                    "view": [0.0, 0.0, 10.0, 10.0],
                    "expectedRevision": "7",
                    "cpuSeconds": 60,
                    "memoryMb": 512,
                },
                "requestId": "istek-3",
                "timeoutSeconds": 120,
            },
        )
        # A tool alone is a commit with nothing else said; the progress words come with the job.
        project.processing_job("points.numberVertices", request_id="istek-4")
        self.assertEqual(
            self.server.requests[-1]["body"],
            {
                "kind": "project.processing",
                "input": {"tool": "points.numberVertices", "mode": "commit"},
                "requestId": "istek-4",
            },
        )
        self.assertEqual(project.job("j3")["progressText"], "Araç çalışıyor: Tampon")
        # A local file in a value is the server's refusal, typed.
        self.server.replies[base] = (
            400,
            {"error": "invalid", "message": "Sunucu işi yerel dosya okumaz.", "path": "input.parameters.dosya.path"},
            {},
        )
        with self.assertRaises(cad.ServerError) as refused:
            project.processing_job("x", parameters={"dosya": {"path": "/etc/passwd"}})
        self.assertEqual(refused.exception.code, "invalid")

    def test_a_file_is_staged_and_imported_and_an_atlas_asked_for(self) -> None:
        import hashlib
        import tempfile

        conn = cad.Connection.login(self.server.url, "ayse", "parola")
        project = conn.project(PROJECT["id"], TENANT)
        root = f"/v1/tenants/{TENANT}/projects/{PROJECT['id']}"
        staged = {"id": "g1", "name": "parsel.gpkg", "size": 9, "sha256": "x", "createdAt": "t", "received": True}
        self.server.replies[f"{root}/job-inputs"] = (201, {**staged, "received": False}, {})
        self.server.replies[f"{root}/job-inputs/g1"] = (200, staged, {})
        self.server.replies[f"{root}/jobs"] = (201, {"id": "j9", "kind": "project.import", "state": "queued"}, {})
        data = b"GPKG-data"
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "parsel.gpkg"
            path.write_bytes(data)
            asked = project.import_job(
                path, options={"reproject": True}, expected_revision="3", label="Parseller", request_id="i-1"
            )
        self.assertEqual(asked["kind"], "project.import")
        begun, sent, job = self.server.requests[-3:]
        # Its name, size and hash are declared, its bytes go in one body, and the job names what the server said.
        self.assertEqual(begun["body"], {"name": "parsel.gpkg", "size": len(data), "sha256": hashlib.sha256(data).hexdigest()})
        self.assertEqual((sent["method"], sent["path"]), ("PUT", f"{root}/job-inputs/g1"))
        self.assertEqual(sent["body"], data)
        self.assertEqual(sent["headers"]["content-type"], "application/octet-stream")
        self.assertEqual(sent["headers"]["content-length"], str(len(data)))
        self.assertEqual(sent["headers"]["cookie"], "kentos_session=tok-123")
        self.assertEqual(
            job["body"],
            {
                "kind": "project.import",
                "input": {
                    "upload": "g1",
                    "options": {"reproject": True},
                    "label": "Parseller",
                    "expectedRevision": "3",
                },
                "requestId": "i-1",
            },
        )
        # A file already sent is named, bytes need their name, and a file and an upload are not both given.
        project.import_job(upload="g1", format="csv", cpu_seconds=60, memory_mb=256, request_id="i-2")
        self.assertEqual(
            self.server.requests[-1]["body"]["input"],
            {"upload": "g1", "format": "csv", "cpuSeconds": 60, "memoryMb": 256},
        )
        with self.assertRaises(ValueError):
            project.import_job(b"abc")
        with self.assertRaises(ValueError):
            project.import_job(data, upload="g1", name="a.gpkg")
        with self.assertRaises(ValueError):
            project.import_job()
        project.import_job(data, name="a.gpkg", request_id="i-3")
        self.assertEqual(self.server.requests[-3]["body"]["name"], "a.gpkg")
        # The atlas: the sheet and what the plan pinned.
        self.server.replies[f"{root}/jobs"] = (201, {"id": "j10", "kind": "project.atlas", "state": "queued"}, {})
        project.atlas_job(
            "atlas-paftasi",
            fields=["Ada"],
            theme="Gece",
            time="2024-06-30",
            crs=5253,
            pin={"digest": "ab"},
            expected_revision="4",
            label="Ada 1",
            cpu_seconds=120,
            memory_mb=1024,
            request_id="a-1",
        )
        self.assertEqual(
            self.server.requests[-1]["body"],
            {
                "kind": "project.atlas",
                "input": {
                    "sheet": "atlas-paftasi",
                    "fields": ["Ada"],
                    "theme": "Gece",
                    "time": "2024-06-30",
                    "crs": 5253,
                    "pin": {"digest": "ab"},
                    "label": "Ada 1",
                    "expectedRevision": "4",
                    "cpuSeconds": 120,
                    "memoryMb": 1024,
                },
                "requestId": "a-1",
            },
        )
        project.atlas_job("a", request_id="a-2")
        self.assertEqual(self.server.requests[-1]["body"]["input"], {"sheet": "a"})
        # A file the server refuses is its refusal, typed.
        self.server.replies[f"{root}/job-inputs"] = (
            422,
            {"error": "invalid", "message": "Dosya çok büyük.", "path": "size"},
            {},
        )
        with self.assertRaises(cad.ServerError) as refused:
            project.stage_file(data, name="b.gpkg")
        self.assertEqual(refused.exception.code, "invalid")

    def test_a_server_that_does_not_answer(self) -> None:
        url = self.server.url
        self.server.close()
        with self.assertRaises(cad.Unreachable):
            cad.Connection.login(url, "ayse", "parola")
        self.server = Server()  # tearDown closes one

    def test_the_catalog_pages(self) -> None:
        conn = cad.Connection.login(self.server.url, "ayse", "parola")
        self.server.replies["/v1/me/catalog"] = (
            200,
            {"projects": [PROJECT], "total": 1, "trashRetentionDays": 30},
            {},
        )
        page = conn.projects("mine", q="Ada")
        self.assertEqual([p.name for p in page.projects], ["Ada 101"])
        self.assertIsNone(page.next)
        self.assertIn("view=mine", self.server.requests[-1]["path"])
        self.assertIn("q=Ada", self.server.requests[-1]["path"])


def _create_fields() -> dict[str, Any]:
    """The required fields of project.create, from its schema."""
    info = next(c for c in cad.catalog() if c.id == "project.create")
    example = info.examples[0]["input"]
    value = cad.ProjectCreate.from_json(example)
    fields = {f: getattr(value, f) for f in value.__dataclass_fields__}
    fields.pop("name")
    return fields


if __name__ == "__main__":
    unittest.main()
