"""The command protocol (docs/adr/0253): what a call expects, what an answer
says it touched, a command run as a job, and the drawing's journal.

- :class:`Expected`: the state a write is prepared against: the drawing's
  revision, objects' and block definitions' state stamps, the project's
  parts' stamps, the plan the write must equal. When one no longer holds,
  nothing is written and the call raises :class:`RevisionConflict`
  (``revision_conflict``, ``stale_state`` with ``uids``, ``plan_changed``).
- :class:`Summary`: what a plan or a write touched (added, changed, removed,
  the dependents that changed with them), its box, the stamps before and
  after, the undo step, the algorithm.
- :class:`Job`: a command computed on a copy of the drawing on another
  thread (``Document.start``); its write comes in when it is looked at.
- :class:`ReplayReport`: a journal (``Document.journal()``) run again on
  another drawing (``Document.replay``).

    >>> plan = cad.entities.transform.answer(doc, uids=[u], transform=…, op="plan")
    >>> cad.entities.transform(doc, uids=[u], transform=…, expected=Expected(plan=plan.plan_id))
"""

from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass, field
from typing import TYPE_CHECKING, Any

from ._runtime import CommandNote, Outcome, call_outcome
from .errors import host_error
from .types import Bounds

if TYPE_CHECKING:
    from .document import Document


@dataclass(frozen=True, slots=True)
class Expected:
    """What must still hold for a write (docs/adr/0253 §2, §5). Stamps are
    16 hex digits: a record's ``stamp``, ``DocumentInfo.stamps``, or an
    answer's ``summary.stamps``."""

    revision: str | None = None
    """The drawing's revision (decimal text)."""
    objects: Mapping[str, str] = field(default_factory=dict)
    """Objects by persistent id, and the stamp each must still have."""
    blocks: Mapping[str, str] = field(default_factory=dict)
    """Block definitions by id, and the stamp each must still have."""
    layers: str | None = None
    settings: str | None = None
    styles: str | None = None
    sheets: str | None = None
    plan: str | None = None
    """A plan's id (``Outcome.plan_id``): the write must equal that plan."""
    dimension_sets: str | None = None

    def to_json(self) -> dict[str, Any]:
        out: dict[str, Any] = {}
        for key in ("revision", "layers", "settings", "styles", "sheets", "plan"):
            value = getattr(self, key)
            if value is not None:
                out[key] = value
        if self.dimension_sets is not None:
            out["dimensionSets"] = self.dimension_sets
        if self.objects:
            out["objects"] = dict(self.objects)
        if self.blocks:
            out["blocks"] = dict(self.blocks)
        return out


@dataclass(frozen=True, slots=True)
class Stamps:
    """State stamps: of objects and block definitions by id, of the project's parts."""

    revision: str | None = None
    objects: dict[str, str] = field(default_factory=dict)
    blocks: dict[str, str] = field(default_factory=dict)
    layers: str | None = None
    settings: str | None = None
    styles: str | None = None
    sheets: str | None = None
    dimension_sets: str | None = None

    @classmethod
    def from_json(cls, raw: Mapping[str, Any] | None) -> Stamps:
        raw = raw or {}
        return cls(
            revision=raw.get("revision"),
            objects=dict(raw.get("objects") or {}),
            blocks=dict(raw.get("blocks") or {}),
            layers=raw.get("layers"),
            settings=raw.get("settings"),
            styles=raw.get("styles"),
            sheets=raw.get("sheets"),
            dimension_sets=raw.get("dimensionSets"),
        )


@dataclass(frozen=True, slots=True)
class Summary:
    """What a plan or a write touched (docs/adr/0253 §5), from the write itself."""

    added: list[str]
    changed: list[str]
    removed: list[str]
    dependents: list[str]
    """Of the changed and removed, those the input did not name: what changed with them."""
    bounds: Bounds | None
    """The box of what was touched, before and after."""
    layers: dict[str, list[str]]
    """Layers and groups ``added``, ``changed``, ``removed``."""
    blocks: dict[str, list[str]]
    project: list[str]
    """The project's parts changed outside the undo history (``settings``, ``styles``, ``sheets``, ``layerFlags``, ``name``)."""
    undo_label: str | None
    """The write's undo step's name; None inside a group or without one."""
    undo_outside: list[str]
    warnings: tuple[CommandNote, ...]
    sources: Stamps
    """The stamps before: of what was touched and the definitions it rests on."""
    stamps: Stamps
    """The stamps after."""
    algorithm: dict[str, Any]
    dimension_sets: dict[str, list[str]] = field(default_factory=dict)
    """Automatic dimension recipes added, changed or removed, by persistent id."""

    @classmethod
    def from_json(cls, raw: Mapping[str, Any]) -> Summary:
        b = raw.get("bounds")
        undo = raw.get("undo") or {}
        return cls(
            added=list(raw.get("added") or []),
            changed=list(raw.get("changed") or []),
            removed=list(raw.get("removed") or []),
            dependents=list(raw.get("dependents") or []),
            bounds=None if b is None else Bounds(min_x=b["minX"], min_y=b["minY"], max_x=b["maxX"], max_y=b["maxY"]),
            layers={k: list(v) for k, v in (raw.get("layers") or {}).items()},
            blocks={k: list(v) for k, v in (raw.get("blocks") or {}).items()},
            project=list(raw.get("project") or []),
            undo_label=undo.get("label"),
            undo_outside=list(undo.get("outside") or []),
            warnings=tuple(CommandNote(w["code"], w["message"], w.get("path")) for w in raw.get("warnings") or []),
            sources=Stamps.from_json(raw.get("sources")),
            stamps=Stamps.from_json(raw.get("stamps")),
            algorithm=dict(raw.get("algorithm") or {}),
            dimension_sets={k: list(v) for k, v in (raw.get("dimensionSets") or {}).items()},
        )


@dataclass(frozen=True, slots=True)
class JobStatus:
    """A job as its host reports it."""

    id: str
    command: str
    phase: str
    """``queued``, ``computing``, ``staged``, ``committing``, ``done``, ``failed``, ``conflict`` or ``cancelled``."""
    progress: float | None
    """How far, 0 to 1, when the command says."""
    answer: Outcome[Any] | None
    """The answer once it is over."""

    @property
    def finished(self) -> bool:
        return self.phase in ("done", "failed", "conflict", "cancelled")

    @classmethod
    def from_json(cls, raw: Mapping[str, Any]) -> JobStatus:
        answer = raw.get("answer")
        command = raw.get("command", "")
        return cls(
            id=raw["id"],
            command=command,
            phase=raw["phase"],
            progress=raw.get("progress"),
            answer=None if answer is None else call_outcome(command, answer, None),
        )


class Job:
    """A command computed on a copy of the drawing on another thread
    (docs/adr/0253 §9). The drawing is free meanwhile; its write comes in,
    whole, when the job is looked at (:meth:`status`, :meth:`wait`): as it
    is when the drawing did not change, by the objects' ids when it did and
    they are as the job saw them; otherwise ``document_changed``."""

    __slots__ = ("_doc", "id")

    def __init__(self, doc: Document, id: str) -> None:
        self._doc = doc
        self.id = id

    def status(self) -> JobStatus:
        return JobStatus.from_json(self._doc._job("job", self.id))

    def wait(self) -> JobStatus:
        """Waits for it (the interpreter's lock let go); its write is in the drawing then."""
        return JobStatus.from_json(self._doc._job("wait", self.id))

    def cancel(self) -> JobStatus:
        """Asks it to stop: nothing is written when the stop comes in time. A
        job already written raises ``already_committed`` (undo takes it back)."""
        return JobStatus.from_json(self._doc._job("cancel", self.id))

    def result(self) -> Outcome[Any]:
        """Waits and gives the answer; a refusal raised."""
        status = self.wait()
        if status.answer is None:
            raise host_error("unknown_job", f"“{self.id}” işinin cevabı yok.")
        status.answer.unwrap()
        return status.answer

    def __repr__(self) -> str:
        return f"<kentos.cad.Job {self.id}>"


@dataclass(frozen=True, slots=True)
class ReplayStep:
    seq: int
    command: str | None
    outcome: str
    """``same``, ``differs``, ``failed`` or ``skipped``."""
    note: str | None
    error: dict[str, Any] | None
    created: list[str]


@dataclass(frozen=True, slots=True)
class ReplayReport:
    """A journal run again (docs/adr/0253 §3)."""

    steps: list[ReplayStep]
    same: int
    differs: int
    failed: int
    skipped: int
    mapped: dict[str, str]
    """The journal's ids and the ids made here in their place."""

    @classmethod
    def from_json(cls, raw: Mapping[str, Any]) -> ReplayReport:
        return cls(
            steps=[
                ReplayStep(
                    seq=s["seq"],
                    command=s.get("command"),
                    outcome=s["outcome"],
                    note=s.get("note"),
                    error=s.get("error"),
                    created=list(s.get("created") or []),
                )
                for s in raw.get("steps") or []
            ],
            same=raw.get("same", 0),
            differs=raw.get("differs", 0),
            failed=raw.get("failed", 0),
            skipped=raw.get("skipped", 0),
            mapped=dict(raw.get("mapped") or {}),
        )


__all__ = ["Expected", "Job", "JobStatus", "ReplayReport", "ReplayStep", "Stamps", "Summary"]
