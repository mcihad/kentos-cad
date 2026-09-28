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
                outer.requests.append(
                    {
                        "method": self.command,
                        "path": self.path,
                        "headers": {k.lower(): v for k, v in self.headers.items()},
                        "body": json.loads(body) if body else None,
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
