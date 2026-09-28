"""The catalog from Python: every command with its schemas and its wrapper
here (docs/adr/0131; TODOS.md AI-02, PY-16)."""

from __future__ import annotations

import json
from dataclasses import dataclass
from typing import Any

from .. import _native
from ._index import COMMANDS
from .errors import NotRunHere


@dataclass(frozen=True, slots=True)
class CommandInfo:
    """One command of the catalog: what it is, where it runs, its schemas."""

    id: str
    version: int
    title: str
    summary: str
    aliases: list[str]
    hosts: list[str]
    permissions: list[str]
    effect: str
    undo: str
    input_schema: dict[str, Any]
    output_schema: dict[str, Any]
    plan_schema: dict[str, Any] | None
    examples: list[dict[str, Any]]
    wrapper: Any
    """Its wrapper here (``kentos.cad.polygon.create``)."""


def catalog() -> list[CommandInfo]:
    """Every product command, from the native module's catalog."""
    out = []
    for c in json.loads(_native.catalog())["commands"]:
        out.append(
            CommandInfo(
                id=c["id"],
                version=c["version"],
                title=c["title"],
                summary=c["summary"],
                aliases=list(c["aliases"]),
                hosts=list(c["hosts"]),
                permissions=list(c["permissions"]),
                effect=c["effect"],
                undo=c["undo"],
                input_schema=c["input"],
                output_schema=c["output"],
                plan_schema=c.get("plan"),
                examples=list(c["examples"]),
                wrapper=COMMANDS.get(c["id"]),
            )
        )
    return out


def command(command_id: str) -> Any:
    """The wrapper of a command by its id (``"cad.polygon.create"``)."""
    try:
        return COMMANDS[command_id]
    except KeyError:
        raise NotRunHere("unknown_command", f"“{command_id}” katalogda yok.") from None


__all__ = ["CommandInfo", "catalog", "command"]
