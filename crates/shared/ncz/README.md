# kentos-ncz

Netcad NCZ drawings read into KentOS's objects (docs/adr/0138).

## Where the code comes from

The block reader (`src/format.rs`, `src/attributes.rs`) is a port of `ncz_pure.py`
from Erdinç Örsan ÜNAL's QGIS plugin **NCZ Reader**, version 1.4.3
(<https://github.com/erdincunal/Jeomatik-NCZ-Reader>), licensed GPL-2.0-or-later.
Every block rule, offset, validity test and heuristic is kept, so a file reads into
the same records with the same values in the same order; the places it differs on
purpose are listed at the top of `src/format.rs`. It was ported to C++ first (in
KentOSCad, 28 September 2026) and from there to Rust.

What is not from the plugin: reading Netcad 8's smart objects ("akıllı nesne": the
settlement, construction, road-width, plan-note and function-name symbols of Planet)
and drawing them (`src/symbols.rs`), sweeping unknown blocks for geometry, the line
width at +28, and the mapping to KentOS's objects (`src/emit.rs`). Those were worked
out from real plans; nothing publishes them.

"Jeomatik" is the author's trademark; it is named here only to identify the source.

## The condition on distribution

Because the port is a derivative of GPL-2.0-or-later code, this crate is
GPL-2.0-or-later, and so is any program distributed with it in. The owner's
decision (28 September 2026): the port lives in this private repository now, and
**no build that contains it — the desktop app or the web's NCZ module — is
distributed** until the author has given KentOS a permissive licence (MIT or
Apache-2.0) or written permission for it. TODOS.md `NCZ-01` holds that gate; the
dependency register (`docs/deps/README.md`) records it.
