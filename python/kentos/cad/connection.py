"""A KentOS server: :class:`Connection`, for the commands the server runs
(``project.*``) and the account's project catalog (docs/adr/0131).

A local account signs in with :meth:`Connection.login`; the session goes
with every request as the server's cookie, with ``x-kentos-client: python``
(the server's rule for a program, docs/adr/0007, 0040). The server checks
the account's rights on every command; nothing here decides them.
Standard library only (``urllib``).
"""

from __future__ import annotations

import io
import json
import os
import socket
import urllib.error
import urllib.parse
import urllib.request
import uuid
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import Any

from ._runtime import decode, dumps
from .errors import NotSignedIn, ServerError, Unreachable, server_error
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
        timeout: float | None = None,
    ) -> Any:
        if query:
            path = f"{path}?{urllib.parse.urlencode(query)}"
        text = None if body is None else dumps(body)
        response = self._send(method, path, text, timeout=timeout)
        with response:
            data = response.read()
        return json.loads(data) if data else None

    def _dimension_sets_preflight(self, command: str, input: Any) -> None:
        if not isinstance(input, Mapping):
            return
        if command == "project.changes":
            project = input.get("project")
            needed = isinstance(project, Mapping) and "dimensionSets" in project
        elif command == "project.create":
            needed = "dimensionSets" in input
        else:
            needed = False
        if not needed:
            return
        unavailable = ServerError(
            426, "server_feature_unavailable",
            "Sunucu mimari ölçü dizilerini saklamayı desteklemiyor. Yerel çiziminiz korundu; buluta kaydetmek için sunucuyu güncelleyin.",
            path="dimensionSets",
        )
        try:
            health = self._request("GET", "/v1/health", timeout=min(self.timeout, 4.0))
        except ServerError as error:
            if error.status == 404:
                raise unavailable from None
            raise
        versions = health.get("projectChangesVersions") if isinstance(health, Mapping) else None
        if not isinstance(versions, list) or not any(type(version) is int and version == 2 for version in versions):
            raise unavailable

    def _put_file(self, path: str, source: Any, size: int) -> Any:
        """Sends a file's bytes as one streamed body (``source``: an open binary file) and gives the answer."""
        headers = {
            "Accept": "application/json",
            "Content-Type": "application/octet-stream",
            "Content-Length": str(size),
            "x-kentos-client": CLIENT,
        }
        if self._token is not None:
            headers["Cookie"] = f"{SESSION_COOKIE}={self._token}"
        request = urllib.request.Request(self.url + path, data=source, method="PUT", headers=headers)
        try:
            with urllib.request.urlopen(request, timeout=max(self.timeout, 600.0)) as response:
                data = response.read()
        except urllib.error.HTTPError as err:
            with err:
                raw = err.read()
            try:
                answer = json.loads(raw) if raw else {}
            except ValueError:
                answer = {}
            raise server_error(err.code, answer if isinstance(answer, dict) else {}) from None
        except (urllib.error.URLError, socket.timeout, TimeoutError, ConnectionError) as err:
            reason = getattr(err, "reason", err)
            raise Unreachable(
                "unreachable",
                f"{self.url} yanıt vermedi ({reason}). Aynı dosya yeniden gönderilebilir: yarım kalan bayt saklanmaz.",
            ) from None
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
        timeout: float | None = None,
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
            return urllib.request.urlopen(request, timeout=self.timeout if timeout is None else timeout)
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

    # ------------------------------------------------------------ server jobs (docs/adr/0542)

    def _jobs(self) -> str:
        tenant = urllib.parse.quote(self.tenant_id, safe="")
        project = urllib.parse.quote(self.project_id, safe="")
        return f"/v1/tenants/{tenant}/projects/{project}/jobs"

    def export_job(
        self,
        *,
        expected_revision: str | None = None,
        fresh: bool = False,
        request_id: str | None = None,
        timeout_seconds: int | None = None,
    ) -> dict[str, Any]:
        """Asks the server to write the project as one ``.kcad`` file (``project.export``). The same ``request_id``
        sent again gives the same job; ``expected_revision`` refuses the export when the project has moved on. When an
        earlier job wrote the same project state for the same reach, its file is taken (the job's ``cachedFrom`` names
        that job; docs/adr/0739); ``fresh=True`` writes it again."""
        body: dict[str, Any] = {
            "kind": "project.export",
            "input": {} if expected_revision is None else {"expectedRevision": expected_revision},
            "requestId": request_id or str(uuid.uuid4()),
        }
        if timeout_seconds is not None:
            body["timeoutSeconds"] = timeout_seconds
        if fresh:
            body["fresh"] = True
        answer: dict[str, Any] = self.connection._request("POST", self._jobs(), body)
        return answer

    def checkpoint_job(
        self,
        name: str,
        *,
        note: str | None = None,
        request_id: str | None = None,
        timeout_seconds: int | None = None,
    ) -> dict[str, Any]:
        """Asks the server to make a checkpoint of the project (``project.checkpoint``; docs/adr/0547). The job's
        ``result`` holds the checkpoint; the same ``request_id`` sent again gives the same job, and the checkpoint
        is made once even if the worker that began it is lost."""
        input_: dict[str, Any] = {"name": name}
        if note:
            input_["note"] = note
        body: dict[str, Any] = {
            "kind": "project.checkpoint",
            "input": input_,
            "requestId": request_id or str(uuid.uuid4()),
        }
        if timeout_seconds is not None:
            body["timeoutSeconds"] = timeout_seconds
        answer: dict[str, Any] = self.connection._request("POST", self._jobs(), body)
        return answer

    def processing_job(
        self,
        tool: str,
        *,
        parameters: Mapping[str, Any] | None = None,
        selection: Sequence[str] | None = None,
        view: Sequence[float] | None = None,
        mode: str = "commit",
        expected_revision: str | None = None,
        label: str | None = None,
        cpu_seconds: int | None = None,
        memory_mb: int | None = None,
        fresh: bool = False,
        request_id: str | None = None,
        timeout_seconds: int | None = None,
    ) -> dict[str, Any]:
        """Asks the server to run an İşlemler tool on the project (``project.processing``; docs/adr/0559): the same
        run ``cad.processing.run`` makes (the tool's id, its ``parameters`` by name, the ``selection`` as persistent
        ids, the ``view`` box as east/north minimum then maximum), but a worker on the server computes it, the project
        at one revision, under a CPU and memory budget. A table is given as ``{"name": …, "rows": …}``; a local file
        path is refused.

        ``mode="commit"`` writes what the tool made, changed or removed to the project in one commit, only if the
        project is still at the revision the tool read (``expected_revision`` pins it); ``mode="sink"`` leaves the
        project as it is and keeps the objects the tool made as a GeoPackage, ``job_artifact`` fetches it.
        ``cpu_seconds`` and ``memory_mb`` ask for less than the server's budget, never more. The same ``request_id``
        sent again gives the same job. The job's ``progress`` and ``progressText`` say how far a running one is; its
        ``result`` holds the tool's summary, the counts, the new revision and what it took. A ``mode="sink"`` job of
        the same project state, input, reach, user and day takes the earlier job's file (``cachedFrom``; docs/adr/0739)
        unless ``fresh=True``."""
        input_: dict[str, Any] = {"tool": tool, "mode": mode}
        if label:
            input_["label"] = label
        if parameters:
            input_["parameters"] = dict(parameters)
        if selection:
            input_["selection"] = list(selection)
        if view is not None:
            input_["view"] = list(view)
        if expected_revision is not None:
            input_["expectedRevision"] = expected_revision
        if cpu_seconds is not None:
            input_["cpuSeconds"] = cpu_seconds
        if memory_mb is not None:
            input_["memoryMb"] = memory_mb
        body: dict[str, Any] = {
            "kind": "project.processing",
            "input": input_,
            "requestId": request_id or str(uuid.uuid4()),
        }
        if timeout_seconds is not None:
            body["timeoutSeconds"] = timeout_seconds
        if fresh:
            body["fresh"] = True
        answer: dict[str, Any] = self.connection._request("POST", self._jobs(), body)
        return answer

    def stage_file(self, file: str | os.PathLike[str] | bytes, *, name: str | None = None) -> dict[str, Any]:
        """Sends a file to the server for a job to read (docs/adr/0561 §1): the file's path (or its bytes, with
        ``name``), its size and SHA-256 declared, its bytes streamed in one body. A cut-off send keeps nothing; sending
        the same file again is safe. The answer's ``id`` is what ``import_job`` names as its ``upload``."""
        import hashlib

        if isinstance(file, bytes):
            if name is None:
                raise ValueError("Baytlar verilince dosyanın adı (name=) da gerekir.")
            digest = hashlib.sha256(file).hexdigest()
            size = len(file)
            opened: Any = io.BytesIO(file)
        else:
            path = os.fspath(file)
            name = name or os.path.basename(path)
            h = hashlib.sha256()
            with open(path, "rb") as f:
                for chunk in iter(lambda: f.read(1 << 20), b""):
                    h.update(chunk)
            digest = h.hexdigest()
            size = os.path.getsize(path)
            opened = open(path, "rb")
        tenant = urllib.parse.quote(self.tenant_id, safe="")
        project = urllib.parse.quote(self.project_id, safe="")
        base = f"/v1/tenants/{tenant}/projects/{project}/job-inputs"
        try:
            begun: dict[str, Any] = self.connection._request(
                "POST", base, {"name": name, "size": size, "sha256": digest}
            )
            sent: dict[str, Any] = self.connection._put_file(f"{base}/{begun['id']}", opened, size)
        finally:
            opened.close()
        return sent

    def import_job(
        self,
        file: str | os.PathLike[str] | bytes | None = None,
        *,
        upload: str | None = None,
        name: str | None = None,
        format: str | None = None,
        options: Mapping[str, Any] | None = None,
        expected_revision: str | None = None,
        label: str | None = None,
        cpu_seconds: int | None = None,
        memory_mb: int | None = None,
        request_id: str | None = None,
        timeout_seconds: int | None = None,
    ) -> dict[str, Any]:
        """Asks the server to take a file into the project (``project.import``; docs/adr/0561): a GeoPackage, a
        FlatGeobuf, a GML, a CSV or a DXF, read by the same command as the desktop's (``file.import.gpkg`` …), on a
        worker, under a CPU and memory budget, the result committed in one revision if the project is still at the
        revision the worker read. ``file`` is a path (or bytes with ``name``) sent first (``stage_file``), or give the
        ``upload`` id of a file already sent. ``format`` is ``gpkg``, ``fgb``, ``gml``, ``csv`` or ``dxf``; absent, the
        file's name tells. ``options`` are the import command's own (``tables``, ``layers``, ``reproject``, ``srid``,
        ``layer``, ``geometry``, ``fileUnit``, ``explodeBlocks`` …), never a path. The job's ``result`` holds the
        counts, the new revision, the command's answer and its warnings (what the file could not give)."""
        if (file is None) == (upload is None):
            raise ValueError("Ya file ya da upload verin.")
        if upload is None:
            upload = str(self.stage_file(file, name=name)["id"])  # type: ignore[arg-type]
        input_: dict[str, Any] = {"upload": upload}
        if format is not None:
            input_["format"] = format
        if options:
            input_["options"] = dict(options)
        if label:
            input_["label"] = label
        if expected_revision is not None:
            input_["expectedRevision"] = expected_revision
        if cpu_seconds is not None:
            input_["cpuSeconds"] = cpu_seconds
        if memory_mb is not None:
            input_["memoryMb"] = memory_mb
        body: dict[str, Any] = {
            "kind": "project.import",
            "input": input_,
            "requestId": request_id or str(uuid.uuid4()),
        }
        if timeout_seconds is not None:
            body["timeoutSeconds"] = timeout_seconds
        answer: dict[str, Any] = self.connection._request("POST", self._jobs(), body)
        return answer

    def atlas_job(
        self,
        sheet: str,
        *,
        fields: Sequence[str] | None = None,
        theme: str | None = None,
        time: str | None = None,
        crs: int | None = None,
        pin: Mapping[str, Any] | None = None,
        expected_revision: str | None = None,
        label: str | None = None,
        cpu_seconds: int | None = None,
        memory_mb: int | None = None,
        fresh: bool = False,
        request_id: str | None = None,
        timeout_seconds: int | None = None,
    ) -> dict[str, Any]:
        """Asks the server to write the atlas of a sheet as one PDF (``project.atlas``; docs/adr/0561): the same
        ``file.export.atlas`` as the desktop's, on a worker, the project at one revision. ``pin`` is the state a plan
        pinned (``cad.file.export.atlas.plan(...).pin``): a project that is not in that state refuses the job
        (``pin_mismatch``). ``job_artifact`` fetches the PDF; the job's ``result`` holds the pages, the pin's digest,
        what the pages carry and what they withhold. An atlas of the same project state, input, reach, user and day
        takes the earlier job's PDF (``cachedFrom``; docs/adr/0739) unless ``fresh=True``."""
        input_: dict[str, Any] = {"sheet": sheet}
        if fields is not None:
            input_["fields"] = list(fields)
        if theme is not None:
            input_["theme"] = theme
        if time is not None:
            input_["time"] = time
        if crs is not None:
            input_["crs"] = crs
        if pin is not None:
            input_["pin"] = dict(pin)
        if label:
            input_["label"] = label
        if expected_revision is not None:
            input_["expectedRevision"] = expected_revision
        if cpu_seconds is not None:
            input_["cpuSeconds"] = cpu_seconds
        if memory_mb is not None:
            input_["memoryMb"] = memory_mb
        body: dict[str, Any] = {
            "kind": "project.atlas",
            "input": input_,
            "requestId": request_id or str(uuid.uuid4()),
        }
        if timeout_seconds is not None:
            body["timeoutSeconds"] = timeout_seconds
        if fresh:
            body["fresh"] = True
        answer: dict[str, Any] = self.connection._request("POST", self._jobs(), body)
        return answer

    def jobs(self) -> list[dict[str, Any]]:
        """The project's server jobs, newest first."""
        answer: dict[str, Any] = self.connection._request("GET", self._jobs())
        return list(answer["jobs"])

    def job(self, job: str) -> dict[str, Any]:
        answer: dict[str, Any] = self.connection._request("GET", f"{self._jobs()}/{urllib.parse.quote(job, safe='')}")
        return answer

    def cancel_job(self, job: str) -> dict[str, Any]:
        """Cancels a queued job at once and a running one at its worker's next beat."""
        answer: dict[str, Any] = self.connection._request(
            "POST", f"{self._jobs()}/{urllib.parse.quote(job, safe='')}/cancel", {}
        )
        return answer

    def wait_job(self, job: str, *, every: float = 1.0, within: float = 600.0) -> dict[str, Any]:
        """Asks after the job until it is over (succeeded, failed or cancelled) and gives it back."""
        import time

        until = time.monotonic() + within
        while True:
            answer = self.job(job)
            if answer["state"] in ("succeeded", "failed", "cancelled"):
                return answer
            if time.monotonic() >= until:
                raise TimeoutError(f"{job} işi {within:g} saniyede bitmedi; durumu: {answer['state']}.")
            time.sleep(every)

    def job_artifact(self, job: str) -> bytes:
        """A succeeded job's file: an export's ``.kcad`` (``Document.from_bytes`` opens it), a processing job's
        GeoPackage (``mode="sink"``) or an atlas's PDF. A server that limits an organisation's jobs
        answers a request past the limit with ``ServerError`` (code ``quota_exceeded``, ``retryable``)."""
        return self.connection._bytes(f"{self._jobs()}/{urllib.parse.quote(job, safe='')}/artifact")

    def _command(
        self,
        command: str,
        version: int,
        input: Any,
        expected_versions: Mapping[str, str] | None,
        idempotency_key: str | None,
    ) -> Any:
        self.connection._dimension_sets_preflight(command, input)
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
        self.connection._dimension_sets_preflight(command, input)
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
