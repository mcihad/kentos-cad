"""The errors and warnings of ``kentos.cad`` (docs/adr/0131).

Three families, as the answers come:

- :class:`CommandError`: a command's own refusal, its ``CommandResult``
  (``failed``, ``needs_input``, ``conflict``, ``cancelled``). Nothing was
  written. ``code`` is the catalog's stable code (``layer_locked``), ``path``
  the input field it is about (``pts[2].y``).
- :class:`HostError`: the local host could not do what it was asked: an
  unknown or server-only command, input that is not the command's type, a
  file that cannot be read or written.
- :class:`ServerError`: the KentOS server's refusal (its ``ApiError``), with
  the HTTP status; :class:`Unreachable` when no answer came.

Every one is a :class:`KentosError` with a stable ``code`` for programs and a
Turkish ``message`` for people. Programs branch on the code, never on the
message.
"""

from __future__ import annotations

from typing import Any, ClassVar


class KentosError(Exception):
    """Every error ``kentos.cad`` raises."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(code, message)
        self.code = code
        self.message = message

    def __str__(self) -> str:
        return f"{self.message} ({self.code})"


# ---------------------------------------------------------------- a command's refusal


class CommandError(KentosError):
    """A command refused; nothing was written."""

    status: ClassVar[str] = ""

    def __init__(
        self,
        command: str,
        code: str,
        message: str,
        path: str | None = None,
        revision: str | None = None,
    ) -> None:
        super().__init__(code, message)
        self.command = command
        self.path = path
        self.revision = revision

    def __str__(self) -> str:
        where = f", {self.path}" if self.path else ""
        return f"{self.command}: {self.message} ({self.code}{where})"


class CommandFailed(CommandError):
    """Refused: ``message`` says why and how to fix it."""

    status = "failed"


class NeedsInput(CommandError):
    """Something only the user can give is missing; ``path`` names it."""

    status = "needs_input"


class RevisionConflict(CommandError):
    """The drawing is no longer at the input's ``expected_revision``;
    ``revision`` is where it is now. Plan again from the drawing as it is."""

    status = "conflict"


class CommandCancelled(CommandError):
    """Cancelled; nothing was written."""

    status = "cancelled"


# ---------------------------------------------------------------- the local host


class HostError(KentosError):
    """The local host could not do what it was asked."""


class InvalidInput(HostError):
    """Input that is not the command's type, or a value JSON cannot carry
    (``invalid_input``, ``not_finite``)."""


class UnknownObject(HostError):
    """No object, or layer, with that id in this drawing (``unknown_object``)."""


class FileError(HostError):
    """A file that cannot be read or written (``file_unreadable``,
    ``file_unwritable``), an old v1 file a save would write over
    (``legacy_file``), a save with nowhere to go (``no_path``)."""


class NotRunHere(HostError):
    """Not done here: the server's command (``server_command``; call it
    through :class:`kentos.cad.Connection`), an unknown one
    (``unknown_command``), another version (``unknown_version``); in the
    desktop's console what the desktop does itself (``not_in_console``: undo,
    save), no drawing open (``no_document``); outside it, the desktop's
    drawing (``no_desktop``)."""


class Busy(HostError):
    """The drawing is in a state that refuses it: an open group (``busy``)."""


class UnknownSystem(HostError):
    """A coordinate system the registry does not know (``unknown_system``)."""


_HOST: dict[str, type[HostError]] = {
    "invalid_input": InvalidInput,
    "not_finite": InvalidInput,
    "unknown_object": UnknownObject,
    "file_unreadable": FileError,
    "file_unwritable": FileError,
    "legacy_file": FileError,
    "no_path": FileError,
    "server_command": NotRunHere,
    "not_in_console": NotRunHere,
    "no_desktop": NotRunHere,
    "no_document": NotRunHere,
    "unknown_command": NotRunHere,
    "unknown_version": NotRunHere,
    "busy": Busy,
    "unknown_system": UnknownSystem,
}


def host_error(code: str, message: str) -> HostError:
    """The typed error of a host code (``HostError`` for a code it does not know)."""
    return _HOST.get(code, HostError)(code, message)


class DecodeError(KentosError):
    """An answer that is not of the type it should be: a server or a native
    module newer than this package (``decode``). Update the package."""


# ---------------------------------------------------------------- the server


class ServerError(KentosError):
    """The server refused (its ``ApiError``).

    ``retryable`` says the same request may be sent again unchanged, after
    ``retry_after`` seconds when given; otherwise something must change first.
    """

    def __init__(
        self,
        status: int,
        code: str,
        message: str,
        *,
        path: str | None = None,
        revision: str | None = None,
        retryable: bool = False,
        retry_after: int | None = None,
        request_id: str | None = None,
        conflicts: list[dict[str, Any]] | None = None,
    ) -> None:
        super().__init__(code, message)
        self.status = status
        self.path = path
        self.revision = revision
        self.retryable = retryable
        self.retry_after = retry_after
        self.request_id = request_id
        self.conflicts = conflicts or []

    def __str__(self) -> str:
        where = f", {self.path}" if self.path else ""
        return f"{self.message} ({self.status} {self.code}{where})"


class NotSignedIn(ServerError):
    """401: no session, or it ended. Sign in again (``Connection.login``)."""


class Forbidden(ServerError):
    """403: the account may not do this in this project."""


class NotFound(ServerError):
    """404: no such project, or one this account cannot see."""


class ServerConflict(ServerError):
    """409: the project changed since the versions the request rests on;
    ``revision`` is where it is now."""


class Gone(ServerError):
    """410: the project is in the trash."""


_STATUS: dict[int, type[ServerError]] = {
    401: NotSignedIn,
    403: Forbidden,
    404: NotFound,
    409: ServerConflict,
    410: Gone,
}


def server_error(status: int, body: dict[str, Any]) -> ServerError:
    """The typed error of a server's answer."""
    cls = _STATUS.get(status, ServerError)
    return cls(
        status,
        str(body.get("error") or "http"),
        str(body.get("message") or f"Sunucu {status} ile yanıt verdi."),
        path=body.get("path"),
        revision=body.get("revision"),
        retryable=bool(body.get("retryable", False)),
        retry_after=body.get("retryAfter"),
        request_id=body.get("requestId"),
        conflicts=body.get("conflicts"),
    )


class Unreachable(KentosError):
    """No answer from the server: not running, no network, or too slow
    (``unreachable``). The request may or may not have reached it; retrying
    with the same idempotency key is safe."""


# ---------------------------------------------------------------- warnings


class CommandWarning(UserWarning):
    """What a command that completed wants its caller to know (a hidden
    layer). It goes to :mod:`warnings`; ``code`` is the catalog's."""

    def __init__(self, command: str, code: str, message: str) -> None:
        super().__init__(f"{command}: {message} ({code})")
        self.command = command
        self.code = code
        self.message = message


# Named where they are used: `kentos.cad.CommandFailed` in a traceback.
for _error in (
    KentosError, CommandError, CommandFailed, NeedsInput, RevisionConflict, CommandCancelled,
    HostError, InvalidInput, UnknownObject, FileError, NotRunHere, Busy, UnknownSystem,
    DecodeError, ServerError, NotSignedIn, Forbidden, NotFound, ServerConflict, Gone,
    Unreachable, CommandWarning,
):
    _error.__module__ = "kentos.cad"

__all__ = [
    "Busy",
    "CommandCancelled",
    "CommandError",
    "CommandFailed",
    "CommandWarning",
    "DecodeError",
    "FileError",
    "Forbidden",
    "Gone",
    "HostError",
    "InvalidInput",
    "KentosError",
    "NeedsInput",
    "NotFound",
    "NotRunHere",
    "NotSignedIn",
    "RevisionConflict",
    "ServerConflict",
    "ServerError",
    "UnknownObject",
    "UnknownSystem",
    "Unreachable",
]
