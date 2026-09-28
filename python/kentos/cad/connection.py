"""A KentOS server: :class:`Connection`, for the commands the server runs
(``project.*``) and the account's project catalog (docs/adr/0131).

A local account signs in with :meth:`Connection.login`; the session goes
with every request as the server's cookie, with ``x-kentos-client: python``
(the server's rule for a program, docs/adr/0007, 0040). The server checks
the account's rights on every command; nothing here decides them.
Standard library only (``urllib``).
"""

from __future__ import annotations

import json
import socket
import urllib.error
import urllib.parse
import urllib.request
import uuid
from collections.abc import Mapping
from dataclasses import dataclass
from typing import Any

from ._runtime import decode, dumps
from .errors import NotSignedIn, Unreachable, server_error
from .types import ProjectSummary

SESSION_COOKIE = "kentos_session"
CLIENT = "python"


@dataclass(frozen=True, slots=True)
class ProjectPage:
    """A page of the account's project catalog."""

    projects: list[ProjectSummary]
    total: int
    """How many the whole list holds with this search."""
    next: str | None
    """The cursor of the next page (``after=``); None on the last."""
    trash_retention_days: int


class Connection:
    """A session with a KentOS server.

    >>> conn = Connection.login("http://127.0.0.1:8787", "ayse", parola)
    >>> page = conn.projects("mine")
    >>> ada = conn.project(page.projects[0])
    >>> cad.project.rename(ada, name="Ada 101 (revize)")
    """

    __slots__ = ("_token", "timeout", "url")

    def __init__(self, url: str, *, token: str | None = None, timeout: float = 30.0) -> None:
        self.url = url.rstrip("/")
        self._token = token
        self.timeout = timeout

    @classmethod
    def login(cls, url: str, login: str, password: str, *, timeout: float = 30.0) -> Connection:
        """Signs in with a local account. The password is sent once, over
        the connection ``url`` names; use https beyond this computer."""
        conn = cls(url, timeout=timeout)
        body = dumps({"login": login.strip(), "password": password})
        response = conn._send("POST", "/v1/auth/login", body, session=False)
        token = _session_of(response.headers.get_all("Set-Cookie") or [])
        response.close()
        if token is None:
            raise NotSignedIn(200, "no_session", "Sunucu oturum çerezi göndermedi; giriş tamamlanmadı.")
        conn._token = token
        return conn

    @property
    def signed_in(self) -> bool:
        return self._token is not None

    def logout(self) -> None:
        """Ends the session here at once, and on the server when it answers."""
        if self._token is None:
            return
        try:
            self._request("POST", "/v1/auth/logout", {})
        finally:
            self._token = None

    def me(self) -> dict[str, Any]:
        """The signed-in account and its memberships (the server's ``Me``)."""
        answer: dict[str, Any] = self._request("GET", "/v1/me")
        return answer

    def projects(
        self,
        view: str = "mine",
        *,
        tenant: str | None = None,
        q: str | None = None,
        type: str | None = None,
        sort: str | None = None,
        limit: int | None = None,
        after: str | None = None,
    ) -> ProjectPage:
        """A page of the account's projects: ``view`` is ``mine``,
        ``organization`` (with ``tenant``), ``shared``, ``recent``,
        ``favorites``, ``archived`` or ``trash``."""
        query = {"view": view, "tenant": tenant, "q": q, "type": type, "sort": sort, "limit": limit, "after": after}
        raw = self._request("GET", "/v1/me/catalog", query={k: v for k, v in query.items() if v is not None})
        return ProjectPage(
            projects=[decode(ProjectSummary, p, "Connection.projects") for p in raw["projects"]],
            total=int(raw["total"]),
            next=raw.get("next"),
            trash_retention_days=int(raw["trashRetentionDays"]),
        )

    def project(self, project: str | ProjectSummary, tenant: str | None = None) -> ProjectRef:
        """A project the ``project.*`` commands act on: a catalog entry, or
        its id with its organisation's (``tenant``)."""
        if isinstance(project, ProjectSummary):
            return ProjectRef(self, project.tenant_id, project.id)
        if tenant is None:
            raise ValueError("Bir projenin kimliğiyle kurumunun kimliği de gerekir (tenant=).")
        return ProjectRef(self, tenant, project)

    def tenant(self, tenant: str) -> TenantRef:
        """An organisation or a personal space: where ``project.create`` makes a project."""
        return TenantRef(self, tenant)

    # ------------------------------------------------------------ the wire

    def _request(
        self,
        method: str,
        path: str,
        body: Any = None,
        *,
        query: Mapping[str, Any] | None = None,
    ) -> Any:
        if query:
            path = f"{path}?{urllib.parse.urlencode(query)}"
        text = None if body is None else dumps(body)
        response = self._send(method, path, text)
        with response:
            data = response.read()
        return json.loads(data) if data else None

    def _bytes(self, path: str) -> bytes:
        response = self._send("GET", path, None, accept="application/octet-stream")
        with response:
            data: bytes = response.read()
        return data

    def _send(
        self,
        method: str,
        path: str,
        body: str | None,
        *,
        session: bool = True,
        accept: str = "application/json",
    ) -> Any:
        headers = {"Accept": accept, "x-kentos-client": CLIENT}
        if body is not None:
            headers["Content-Type"] = "application/json"
        if session and self._token is not None:
            headers["Cookie"] = f"{SESSION_COOKIE}={self._token}"
        request = urllib.request.Request(
            self.url + path,
            data=None if body is None else body.encode("utf-8"),
            method=method,
            headers=headers,
        )
        try:
            return urllib.request.urlopen(request, timeout=self.timeout)
        except urllib.error.HTTPError as e:
            with e:
                raw = e.read()
            try:
                answer = json.loads(raw) if raw else {}
            except ValueError:
                answer = {}
            if not isinstance(answer, dict):
                answer = {}
            error = server_error(e.code, answer)
            if e.code == 401 and session:
                self._token = None
            raise error from None
        except (urllib.error.URLError, socket.timeout, TimeoutError, ConnectionError) as e:
            reason = getattr(e, "reason", e)
            raise Unreachable(
                "unreachable",
                f"{self.url} yanıt vermedi ({reason}). Sunucunun çalıştığını ve adresi denetleyin; "
                "aynı anahtarla yeniden denemek güvenlidir.",
            ) from None


@dataclass(frozen=True, slots=True)
class ProjectRef:
    """A cloud project the ``project.*`` commands act on."""

    connection: Connection
    tenant_id: str
    project_id: str

    def snapshot(self) -> bytes:
        """A database project as one ``.kcad`` v2 file of one revision (the
        server's snapshot, docs/adr/0033); ``Document.from_bytes`` opens it."""
        tenant = urllib.parse.quote(self.tenant_id, safe="")
        project = urllib.parse.quote(self.project_id, safe="")
        return self.connection._bytes(f"/v1/tenants/{tenant}/projects/{project}/snapshot")

    def _command(
        self,
        command: str,
        version: int,
        input: Any,
        expected_versions: Mapping[str, str] | None,
        idempotency_key: str | None,
    ) -> Any:
        tenant = urllib.parse.quote(self.tenant_id, safe="")
        project = urllib.parse.quote(self.project_id, safe="")
        return self.connection._request(
            "POST",
            f"/v1/tenants/{tenant}/projects/{project}/commands",
            _envelope(command, version, self.tenant_id, self.project_id, input, expected_versions, idempotency_key),
        )


@dataclass(frozen=True, slots=True)
class TenantRef:
    """An organisation or a personal space: where a new project is made."""

    connection: Connection
    tenant_id: str

    def _command(
        self,
        command: str,
        version: int,
        input: Any,
        expected_versions: Mapping[str, str] | None,
        idempotency_key: str | None,
    ) -> Any:
        tenant = urllib.parse.quote(self.tenant_id, safe="")
        return self.connection._request(
            "POST",
            f"/v1/tenants/{tenant}/commands",
            _envelope(command, version, self.tenant_id, "", input, expected_versions, idempotency_key),
        )


def _envelope(
    command: str,
    version: int,
    tenant: str,
    project: str,
    input: Any,
    expected_versions: Mapping[str, str] | None,
    idempotency_key: str | None,
) -> dict[str, Any]:
    """The server's ``CommandEnvelope``: a retry with the same key gets the
    same answer instead of a second change."""
    return {
        "commandName": command,
        "version": version,
        "tenantId": tenant,
        "projectId": project,
        "requestId": f"python-{uuid.uuid4()}",
        "idempotencyKey": idempotency_key or str(uuid.uuid4()),
        "expectedVersions": dict(expected_versions or {}),
        "input": input,
    }


def _session_of(cookies: list[str]) -> str | None:
    for cookie in cookies:
        name, _, rest = cookie.partition("=")
        if name.strip() == SESSION_COOKIE:
            value = rest.split(";", 1)[0].strip()
            return value or None
    return None


__all__ = ["Connection", "ProjectPage", "ProjectRef", "TenantRef"]
