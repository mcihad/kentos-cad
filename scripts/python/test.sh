#!/bin/sh
# The Python SDK (docs/adr/0131): the generated layer matches the catalog, the native module
# builds into .run/py (made here with maturin when missing), the tests pass, and the extension
# crate is clippy-clean. `--live` also runs scripts/python/live.py: the real kentosd on a
# throwaway database (build it first: cargo build -p kentos-api --bin kentosd --example e2e_database).
set -eu
cd "$(dirname "$0")/../.."
python3 scripts/python/sdk.py --check
if [ ! -x .run/py/bin/maturin ]; then
  python3 -m venv .run/py
  .run/py/bin/python -m pip install -q 'maturin>=1.9.4,<2'
fi
(cd python && VIRTUAL_ENV="$(cd ../.run/py && pwd)" ../.run/py/bin/maturin develop -q)
.run/py/bin/python -m unittest discover -s python/tests
cargo clippy -q -p kentos-python --all-targets -- -D warnings
if [ "${1:-}" = "--live" ]; then
  .run/py/bin/python scripts/python/live.py
fi
