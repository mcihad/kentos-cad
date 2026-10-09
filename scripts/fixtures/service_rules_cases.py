"""Checks fixtures/services/v1/rules.json with tools/kcad/kcad.py's own copy of the rules (docs/adr/0208 §2): the KCAD
reader written from the specification refuses exactly the cases the contract gives words for, and gives the same
origins. The words themselves are the contract's (Rust and the browser compare them exactly); this checks the verdicts.

    python3 scripts/fixtures/service_rules_cases.py --check
"""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT / "tools/kcad"))
import kcad  # noqa: E402


def origin_of(url):
    """The origin of an address, by RFC 3986's authority: scheme and host in lower case, the scheme's port left out."""
    text = url.strip()
    if "://" not in text:
        return None
    scheme, rest = text.split("://", 1)
    scheme = scheme.lower()
    if scheme not in ("http", "https"):
        return None
    for stop in "/?#":
        rest = rest.split(stop, 1)[0]
    rest = rest.rsplit("@", 1)[-1]
    if rest.startswith("["):
        close = rest.find("]")
        if close < 0:
            return None
        host, after = rest[: close + 1], rest[close + 1:]
        if after and not after.startswith(":"):
            return None
        port = after[1:] if after else None
    else:
        host, sep, port = rest.rpartition(":")
        if not sep:
            host, port = rest, None
    if not host:
        return None
    host = host.lower()
    default = "443" if scheme == "https" else "80"
    return f"{scheme}://{host}:{port}" if port and port != default else f"{scheme}://{host}"


def main():
    cases = json.loads((ROOT / "fixtures/services/v1/rules.json").read_text(encoding="utf-8"))
    wrong = []
    for group, problem in (("services", kcad.service_problem), ("feeds", kcad.feed_problem), ("connections", kcad.connections_problem)):
        for c in cases[group]:
            refused = problem(c["value"]) is not None
            if refused != (c["problem"] is not None):
                wrong.append(f"{group}/{c['name']}: okuyucu {'reddediyor' if refused else 'kabul ediyor'}")
    for c in cases["origins"]:
        if origin_of(c["url"]) != c["origin"]:
            wrong.append(f"origins/{c['url']}: {origin_of(c['url'])} ≠ {c['origin']}")
    if wrong:
        print("\n".join(wrong))
        return 1
    n = sum(len(cases[g]) for g in ("services", "feeds", "connections", "origins"))
    print(f"Servis kuralları tutarlı: {n} durum, okuyucu sözleşmeyle aynı karar veriyor.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
