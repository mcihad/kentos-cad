"""The hand-written core under the generated types and wrappers
(docs/adr/0131): the unset marker, the codecs' helpers, the bases of the
command wrappers and their answers.
"""

from __future__ import annotations

import json
import warnings
from collections.abc import Callable, Mapping
from dataclasses import dataclass
from enum import Enum
from typing import TYPE_CHECKING, Any, ClassVar, Final, Generic, TypeVar

from .errors import (
    CommandCancelled,
    CommandError,
    CommandFailed,
    CommandWarning,
    DecodeError,
    InvalidInput,
    NeedsInput,
    RevisionConflict,
)

if TYPE_CHECKING:
    from .connection import ProjectRef, TenantRef
    from .document import Document


class Unset(Enum):
    """The value of an optional field that is left out (:data:`UNSET`): it is
    not sent, and the command treats it as absent. ``None`` sends null.
    A one-member enum, so a type checker narrows ``x is not UNSET``."""

    UNSET = "UNSET"

    def __repr__(self) -> str:
        return "UNSET"

    def __bool__(self) -> bool:
        return False


UNSET: Final = Unset.UNSET
"""An optional field left out."""


class _StrEnum(str, Enum):
    """A catalog enum: its members are its names on the wire, and print as
    them (``str(ProjectRole.OWNER) == "owner"``), as 3.11's StrEnum does."""

    def __str__(self) -> str:
        return str(self.value)


class _Model:
    """A catalog type: its JSON with ``to_json()``, read with ``from_json()``."""

    __slots__ = ()

    def to_json(self) -> dict[str, Any]:
        raise NotImplementedError

    @classmethod
    def from_json(cls, data: Mapping[str, Any]) -> Any:
        raise NotImplementedError


class _Union(_Model):
    """A tagged union: the base of its variants; ``TAG`` names the field the
    wire tells them apart by, ``TAG_VALUE`` is each variant's."""

    __slots__ = ()
    TAG: ClassVar[str]
    TAG_VALUE: ClassVar[str]


def _enum_out(value: Any) -> Any:
    return value.value if isinstance(value, Enum) else value


def _enum_in(cls: type[Enum], value: Any) -> Any:
    # A name this package does not know (a newer server) stays the plain text.
    try:
        return cls(value)
    except ValueError:
        return value


def _vec2_out(value: Any) -> dict[str, float]:
    if hasattr(value, "x") and hasattr(value, "y"):
        return {"x": float(value.x), "y": float(value.y)}
    x, y = value
    return {"x": float(x), "y": float(y)}


_U = TypeVar("_U")


def _variant(table: Mapping[str, type[_U]], union: str, tag: str, data: Mapping[str, Any]) -> type[_U]:
    try:
        return table[data[tag]]
    except KeyError:
        raise DecodeError(
            "decode",
            f"{union}: “{data.get(tag)!s}” bilinen bir {tag} değil. Paket sunucudan eski olabilir; güncelleyin.",
        ) from None


def _opt(value: Any, convert: Callable[[Any], Any]) -> Any:
    return value if value is UNSET or value is None else convert(value)


def dumps(value: Any) -> str:
    """JSON text as the host reads it; NaN and infinity cannot cross."""
    try:
        return json.dumps(value, ensure_ascii=False, allow_nan=False, separators=(",", ":"))
    except ValueError:
        raise InvalidInput(
            "not_finite",
            "Girdide NaN ya da sonsuz bir sayı var; JSON bunları taşımaz. Değerleri denetleyin.",
        ) from None


def decode(cls: type[Any], data: Any, where: str) -> Any:
    """``cls.from_json(data)``, a mismatch reported as :class:`DecodeError`."""
    try:
        return cls.from_json(data)
    except DecodeError:
        raise
    except (KeyError, TypeError, ValueError, AttributeError) as e:
        raise DecodeError(
            "decode",
            f"{where}: yanıt {cls.__name__} olarak okunamadı ({type(e).__name__}: {e}). "
            "Paket sunucudan ya da yerel modülden eski olabilir; güncelleyin.",
        ) from e


# ---------------------------------------------------------------- answers


@dataclass(frozen=True, slots=True)
class CommandNote:
    """A warning of a command that completed: ``code`` for programs,
    ``message`` for people."""

    code: str
    message: str
    path: str | None = None
    """The input field it is about (``layerId``), when one is."""


O = TypeVar("O")
I = TypeVar("I", bound=_Model)  # noqa: E741
P = TypeVar("P")

_ERRORS: dict[str, type[CommandError]] = {
    "failed": CommandFailed,
    "needs_input": NeedsInput,
    "conflict": RevisionConflict,
}


@dataclass(frozen=True, slots=True)
class Outcome(Generic[O]):
    """A command's whole answer (its ``CommandResult``), when the caller
    wants it rather than a raised refusal (``wrapper.run``)."""

    command: str
    status: str
    """``completed``, ``queued``, ``needs_input``, ``conflict``, ``cancelled`` or ``failed``."""
    output: O | None = None
    warnings: tuple[CommandNote, ...] = ()
    error: CommandError | None = None
    job_id: str | None = None

    @property
    def ok(self) -> bool:
        return self.status == "completed"

    def unwrap(self) -> O:
        """The output; the refusal raised."""
        if self.error is not None:
            raise self.error
        return self.output  # type: ignore[return-value]


def outcome(command: str, answer: Mapping[str, Any], output_type: type[Any] | None) -> Outcome[Any]:
    status = answer.get("status")
    if status == "completed":
        notes = tuple(CommandNote(w["code"], w["message"], w.get("path")) for w in answer.get("warnings") or [])
        raw = answer.get("output")
        output = None if output_type is None or raw is None else decode(output_type, raw, command)
        return Outcome(command, status, output, notes)
    if status == "queued":
        return Outcome(command, status, job_id=answer.get("jobId"))
    if status == "cancelled":
        return Outcome(
            command,
            status,
            error=CommandCancelled(command, "cancelled", "Komut iptal edildi; hiçbir şey yazılmadı."),
        )
    cls = _ERRORS.get(str(status))
    if cls is None:
        raise DecodeError("decode", f"{command}: bilinmeyen sonuç durumu “{status}”.")
    e = answer.get("error") or {}
    return Outcome(
        command,
        str(status),
        error=cls(command, e.get("code", "unknown"), e.get("message", ""), e.get("path"), e.get("revision")),
    )


def _warn(result: Outcome[Any]) -> None:
    # user code → the wrapper's call → execute_input → _warn → warn
    for n in result.warnings:
        warnings.warn(CommandWarning(result.command, n.code, n.message), stacklevel=4)


# ---------------------------------------------------------------- the wrappers' bases


class LocalCommand(Generic[I, O, P]):
    """A command of a drawing (``cad.*``), run here by the desktop's own
    handler: the call writes one undo step, ``plan`` shows what it would
    write, ``validate`` checks; ``run`` gives the whole answer."""

    __slots__ = ()
    id: ClassVar[str]
    version: ClassVar[int]
    title: ClassVar[str]
    scope: ClassVar[str]
    input_type: ClassVar[type[Any]]
    output_type: ClassVar[type[Any]]
    plan_type: ClassVar[type[Any]]

    def run(self, doc: Document, input: I, /, *, op: str = "execute") -> Outcome[Any]:
        """Runs ``op`` (``execute``, ``plan`` or ``validate``) with a built
        input; a refusal is in the answer, not raised."""
        if not isinstance(input, self.input_type):
            raise TypeError(f"{self.id} girdisi {self.input_type.__name__} olmalı, {type(input).__name__} değil.")
        types = {"execute": self.output_type, "plan": self.plan_type, "validate": None}
        if op not in types:
            raise ValueError(f"“{op}” bir çağrı biçimi değil: execute, plan ya da validate.")
        answer = doc._command(self.id, self.version, op, input.to_json())
        return outcome(self.id, answer, types[op])

    def execute_input(self, doc: Document, input: I, /) -> O:
        result = self.run(doc, input)
        _warn(result)
        return result.unwrap()  # type: ignore[no-any-return]

    def plan_input(self, doc: Document, input: I, /) -> P:
        result = self.run(doc, input, op="plan")
        return result.unwrap()  # type: ignore[no-any-return]

    def validate_input(self, doc: Document, input: I, /) -> list[CommandNote]:
        result = self.run(doc, input, op="validate")
        if result.error is not None:
            raise result.error
        return list(result.warnings)

    def __repr__(self) -> str:
        return f"<kentos.cad {self.id} v{self.version}: {self.title}>"


class ServerCommand(Generic[I, O]):
    """A command the KentOS server runs (``project.*``), through a
    :class:`kentos.cad.Connection`; the server checks the account's rights."""

    __slots__ = ()
    id: ClassVar[str]
    version: ClassVar[int]
    title: ClassVar[str]
    scope: ClassVar[str]
    input_type: ClassVar[type[Any]]
    output_type: ClassVar[type[Any]]

    def execute_input(
        self,
        target: ProjectRef | TenantRef,
        input: I,
        /,
        *,
        expected_versions: Mapping[str, str] | None = None,
        idempotency_key: str | None = None,
    ) -> O:
        if not isinstance(input, self.input_type):
            raise TypeError(f"{self.id} girdisi {self.input_type.__name__} olmalı, {type(input).__name__} değil.")
        data = target._command(self.id, self.version, input.to_json(), expected_versions, idempotency_key)
        return decode(self.output_type, data, self.id)  # type: ignore[no-any-return]

    def __repr__(self) -> str:
        return f"<kentos.cad {self.id} v{self.version}: {self.title} (sunucu)>"
