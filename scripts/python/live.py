#!/usr/bin/env python3
"""The Python SDK against the real server (docs/adr/0131): a drawing made in
Python becomes a cloud project and comes back as a file.

A throwaway database with this build's migrations and the development
accounts (apps/api/examples/e2e_database.rs; kentos_cad is not touched), the
real kentosd on a free port, then with kentos.cad only:

- ayse signs in; a new drawing gets a closed area (250 m²);
- the drawing becomes a database project in her organisation
  (``project.create``) and the area goes to it under its own id
  (``project.changes``);
- rename, favourite, description and tags; the favourites list has it;
- a checkpoint; archived, a change is refused; unarchived;
- mehmet, not shared, does not see it (404); shared as a viewer, he may not
  rename it (403); his access taken away;
- trashed, it answers 410; restored;
- the project's snapshot opens as a Document: the same area under the same id;
- then the same through the MCP server (kentos-mcp, docs/adr/0134) with ayse's
  account in its environment: the project found in the catalog, opened and
  measured (the same id, 250 m²), shared with mehmet and taken back, and a
  refusal of the server read with its field.

Run with the SDK's environment after building the server and the MCP server:

    cargo build -p kentos-api --bin kentosd --example e2e_database
    cargo build -p kentos-mcp
    .run/py/bin/python scripts/python/live.py

The development password is read from .env.local and never printed.
"""

from __future__ import annotations

import json
import os
import re
import socket
import subprocess
import sys
import time
import urllib.request
from collections.abc import Callable
from pathlib import Path
from typing import TypeVar
from urllib.parse import urlsplit, urlunsplit

import kentos.cad as cad

ROOT = Path(__file__).resolve().parents[2]
BUILD = ROOT / "target/debug"
SQUARE = [(423500, 4512300), (423520, 4512300), (423520, 4512312.5), (423500, 4512312.5)]


def say(text: str) -> None:
    print(f"  {text}", flush=True)


def settings_of(path: Path) -> dict[str, str]:
    values = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        if line and not line.startswith("#") and "=" in line:
            key, _, value = line.partition("=")
            values[key.strip()] = value.strip()
    return values


def free_port() -> int:
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return int(s.getsockname()[1])


E = TypeVar("E", bound=cad.KentosError)


def expect(error: type[E], call: Callable[..., object], *args: object, **kwargs: object) -> E:
    try:
        call(*args, **kwargs)
    except error as e:
        return e
    raise AssertionError(f"{error.__name__} bekleniyordu")


def scenario(url: str, password: str) -> tuple[cad.ProjectInfo, str, str]:
    ayse = cad.Connection.login(url, "ayse", password)
    office = next(m for m in ayse.me()["memberships"] if m["tenantSlug"] == "ornek-buro")
    say(f"giriş: ayse, {office['tenantName']}")

    doc = cad.Document.new("Python E2E", srid=5256)
    made = cad.polygon.create(doc, layer_id=doc.active_layer, pts=SQUARE)
    assert doc.measure(made.uid).area == 250.0
    say(f"çizim: kapalı alan {made.uid}, 250 m²")

    info = doc.info()
    created = cad.project.create(
        ayse.tenant(office["tenantId"]),
        name="Python E2E",
        settings=info.settings,
        origin=(423510, 4512306),
        layers=doc.layers(),
        active_layer=info.active_layer,
        styles=cad.ProjectStyles(items=[], categories=[]),
        project_type="cad",
        tags=["python"],
    )
    project = ayse.project(created.id, tenant=created.tenant_id)
    say(f"proje: {created.name} ({created.id}), {created.access.role}")

    entity = doc.entity(made.uid).entity
    commit = cad.project.changes(project, features=[cad.CreateFeatureChange(id=made.uid, entity=entity)])
    assert made.uid in commit.versions, commit
    say(f"project.changes: nesne sunucuda, veri revizyonu {commit.data_revision}")

    renamed = cad.project.rename(project, name="Python E2E (revize)")
    assert renamed.changed and renamed.project.name == "Python E2E (revize)"
    cad.project.favorite(project, favorite=True)
    cad.project.metadata.update(project, description="Python'dan", tags=["python", "e2e"])
    favorites = ayse.projects("favorites")
    assert [p.id for p in favorites.projects] == [created.id], favorites
    say("ad, favori, açıklama ve etiketler; Favoriler listesinde")

    checkpoint = cad.project.checkpoint.create(project, name="Python kontrol noktası")
    assert checkpoint.checkpoint is not None and checkpoint.checkpoint.name == "Python kontrol noktası"
    say(f"kontrol noktası: {checkpoint.checkpoint.id}")

    cad.project.archive(project)
    refused = expect(cad.ServerError, cad.project.rename, project, name="Arşivde")
    say(f"arşivde ad değişmez: {refused.status} {refused.code}")
    cad.project.unarchive(project)

    mehmet = cad.Connection.login(url, "mehmet", password)
    his = mehmet.project(created.id, tenant=created.tenant_id)
    expect(cad.NotFound, cad.project.rename, his, name="Mehmet'ten")
    say("mehmet paylaşılmamış projeyi görmez: 404")
    mehmet_id = mehmet.me()["user"]["id"]
    cad.project.share(project, user_id=mehmet_id, role="viewer")
    forbidden = expect(cad.ServerError, cad.project.rename, his, name="Mehmet'ten")
    assert forbidden.status == 403, forbidden
    say("görüntüleyici olarak paylaşıldı: adı değiştiremez, 403")
    cad.project.access.revoke(project, user_id=mehmet_id)
    expect(cad.NotFound, cad.project.rename, his, name="Mehmet'ten")
    say("erişimi geri alındı: yine 404")

    cad.project.trash(project)
    gone = expect(cad.ServerError, cad.project.rename, project, name="Çöpte")
    say(f"çöpte: {gone.status} {gone.code}")
    cad.project.restore(project)

    back = cad.Document.from_bytes(project.snapshot())
    assert back.measure(made.uid).area == 250.0
    record = back.entity(made.uid)
    assert isinstance(record.entity, cad.PolygonEntity)
    say(f"projenin görüntüsü açıldı: {len(back)} nesne, aynı kimlikle 250 m²")
    ayse.logout()
    expect(cad.NotSignedIn, ayse.me)
    say("çıkış yapıldı")
    return created, made.uid, str(mehmet_id)


class Mcp:
    """kentos-mcp over its stdio, as an MCP client speaks it (2026-07-28)."""

    META = {
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": {"name": "live.py", "version": "1"},
        "io.modelcontextprotocol/clientCapabilities": {},
    }

    def __init__(self, url: str, password: str) -> None:
        env = {**os.environ, "KENTOS_URL": url, "KENTOS_LOGIN": "ayse", "KENTOS_PASSWORD": password}
        self.process = subprocess.Popen(
            [str(BUILD / "kentos-mcp")], stdin=subprocess.PIPE, stdout=subprocess.PIPE, env=env
        )
        self.id = 0

    def call(self, tool: str, **arguments: object) -> dict[str, object]:
        assert self.process.stdin is not None and self.process.stdout is not None
        self.id += 1
        request = {
            "jsonrpc": "2.0",
            "id": self.id,
            "method": "tools/call",
            "params": {"_meta": self.META, "name": tool, "arguments": arguments},
        }
        self.process.stdin.write((json.dumps(request) + "\n").encode())
        self.process.stdin.flush()
        answer = json.loads(self.process.stdout.readline())
        assert answer.get("id") == self.id, answer
        result: dict[str, object] = answer["result"]
        return result

    def close(self) -> None:
        if self.process.stdin is not None:
            self.process.stdin.close()
        self.process.wait(timeout=30)


def mcp_scenario(url: str, password: str, created: cad.ProjectInfo, uid: str, mehmet: str) -> None:
    mcp = Mcp(url, password)
    try:
        page = mcp.call("project.list", q="Python E2E")
        found = [p for p in page["structuredContent"]["projects"] if p["id"] == created.id]  # type: ignore[index]
        assert found, page
        say(f"MCP project.list: {found[0]['name']}")
        opened = mcp.call("project.open", tenant=created.tenant_id, project=created.id)
        assert opened["isError"] is False, opened
        drawing = opened["structuredContent"]["drawing"]  # type: ignore[index]
        measured = mcp.call("drawing.measure", drawing=drawing, uid=uid)
        assert measured["structuredContent"]["area"] == 250.0, measured  # type: ignore[index]
        say("MCP project.open + drawing.measure: aynı kimlikle 250 m²")
        shared = mcp.call("project.share", tenant=created.tenant_id, project=created.id, userId=mehmet, role="viewer")
        assert shared["isError"] is False and shared["structuredContent"]["changed"] is True, shared  # type: ignore[index]
        revoked = mcp.call("project.access.revoke", tenant=created.tenant_id, project=created.id, userId=mehmet)
        assert revoked["isError"] is False, revoked
        say("MCP project.share ve project.access.revoke: mehmet'e görüntüleyici, sonra geri alındı")
        refused = mcp.call("project.rename", tenant=created.tenant_id, project=created.id, name="")
        error = refused["structuredContent"]["error"]  # type: ignore[index]
        assert refused["isError"] is True and error["status"] == 422 and error["path"] == "name", refused
        say(f"MCP sunucunun reddi: {error['status']} {error['code']} ({error.get('path')})")
    finally:
        mcp.close()


def main() -> int:
    env = settings_of(ROOT / ".env.local")
    password = env.get("KENTOS_DEV_PASSWORD")
    if not password or "KENTOS_DATABASE_URL" not in env:
        print(".env.local içinde KENTOS_DEV_PASSWORD ya da KENTOS_DATABASE_URL yok: önce `pnpm db:setup`.", file=sys.stderr)
        return 2
    for tool in ("kentosd", "examples/e2e_database", "kentos-mcp"):
        if not (BUILD / tool).exists():
            print(
                f"{BUILD / tool} yok: cargo build -p kentos-api --bin kentosd --example e2e_database; cargo build -p kentos-mcp",
                file=sys.stderr,
            )
            return 2
    helper = subprocess.Popen(
        [str(BUILD / "examples/e2e_database")],
        cwd=ROOT,
        env={**os.environ, "KENTOS_DEV_PASSWORD": password},
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
    )
    server = None
    try:
        assert helper.stdout is not None
        name = helper.stdout.readline().decode().strip()
        if not re.fullmatch(r"kentos_cad_test_[0-9a-z_]+", name):
            print("geçici veritabanı açılamadı", file=sys.stderr)
            return 1
        print(f"geçici veritabanı: {name} (kentos_cad'e dokunulmuyor)")
        parts = urlsplit(env["KENTOS_DATABASE_URL"])
        scratch = urlunsplit(parts._replace(path=f"/{name}"))
        port = free_port()
        server = subprocess.Popen(
            [str(BUILD / "kentosd"), "serve"],
            cwd=ROOT,
            env={**os.environ, "KENTOS_API_PORT": str(port), "KENTOS_DATABASE_URL": scratch, "KENTOS_LOG": "warn"},
            stdout=subprocess.DEVNULL,
        )
        url = f"http://127.0.0.1:{port}"
        for _ in range(150):
            try:
                with urllib.request.urlopen(f"{url}/v1/health", timeout=1):
                    break
            except OSError:
                time.sleep(0.1)
        else:
            print("kentosd başlamadı", file=sys.stderr)
            return 1
        print(f"kentosd: {url}")
        created, uid, mehmet = scenario(url, password)
        mcp_scenario(url, password, created, uid, mehmet)
        print("tamam: Python SDK ve MCP sunucusu gerçek sunucuyla")
        return 0
    finally:
        if server is not None:
            server.terminate()
            server.wait(timeout=30)
        if helper.stdin is not None:
            helper.stdin.close()
        helper.wait(timeout=120)


if __name__ == "__main__":
    sys.exit(main())
