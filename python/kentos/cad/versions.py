"""The SDK and its host agreeing on command versions (docs/adr/0443 §3).

Each call carries its command's version and the host refuses one it does not run
(``unknown_version``, :class:`NotRunHere`); :func:`compatibility` tells it before any call:
the commands this SDK was generated for against the ones the native host runs. A
command whose version changes is a new version: the old one keeps its meaning until it
is retired (docs/adr/0443 §3), and the form profile a form is written to
(:data:`FORM_PROFILE`) does not change under it.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from .. import _native
from ._index import COMMANDS
from ._runtime import LocalCommand
from .errors import NotRunHere

#: The form profile this SDK writes and reads (docs/specs/forms-v1.md): a form keeps its
#: meaning under it; a later profile is a new name, never this one changed.
FORM_PROFILE = "kentos.forms/1"


@dataclass(frozen=True)
class Compatibility:
    """The SDK's local commands against the host's: ``missing`` the host does not run,
    ``older`` it runs at a lower version, ``newer`` at a higher one, ``extra`` this SDK
    does not know."""

    missing: list[str] = field(default_factory=list)
    older: dict[str, tuple[int, int]] = field(default_factory=dict)
    newer: dict[str, tuple[int, int]] = field(default_factory=dict)
    extra: list[str] = field(default_factory=list)

    @property
    def ok(self) -> bool:
        """Every command of the SDK runs on the host at its version."""
        return not (self.missing or self.older or self.newer)


def compatibility(host: list[tuple[str, int]] | None = None) -> Compatibility:
    """The SDK's local commands against ``host`` (``(id, version)`` pairs; none: the native host's)."""
    have = dict(_native.local_commands() if host is None else host)
    ours = {w.id: w.version for w in COMMANDS.values() if isinstance(w, LocalCommand)}
    out = Compatibility(extra=sorted(set(have) - set(ours)))
    for cid, v in sorted(ours.items()):
        if cid not in have:
            out.missing.append(cid)
        elif have[cid] < v:
            out.older[cid] = (v, have[cid])
        elif have[cid] > v:
            out.newer[cid] = (v, have[cid])
    return out


def require(*ids: str, host: list[tuple[str, int]] | None = None) -> None:
    """Raises :class:`NotRunHere` unless the host runs each of ``ids`` at this SDK's version."""
    c = compatibility(host)
    wrong = [i for i in ids if i in c.missing or i in c.older or i in c.newer]
    unknown = [i for i in ids if i not in COMMANDS]
    if unknown:
        raise NotRunHere("unknown_command", f"Bu SDK {', '.join(unknown)} komutunu bilmiyor; SDK'yı güncelleyin.")
    if wrong:
        said = []
        for i in wrong:
            if i in c.missing:
                said.append(f"{i} (bu sürüm çalıştırmıyor)")
            else:
                sdk, host_v = (c.older | c.newer)[i]
                said.append(f"{i} (SDK v{sdk}, KentOS v{host_v})")
        raise NotRunHere(
            "unknown_version",
            "KentOS bu komutları SDK'nın sürümüyle çalıştırmıyor: " + ", ".join(said)
            + ". SDK'yı ve KentOS'u aynı sürüme getirin."
        )


__all__ = ["FORM_PROFILE", "Compatibility", "compatibility", "require"]
