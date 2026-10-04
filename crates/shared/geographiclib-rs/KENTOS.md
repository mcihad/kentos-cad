# geographiclib-rs 0.2.7, libm on every target

The crates.io release of [geographiclib-rs](https://github.com/georust/geographiclib-rs) 0.2.7 (MIT, `LICENSE`),
GeographicLib's geodesics and polygon areas (Karney 2013), for the plane, ellipsoid and ground values of
docs/adr/0171. One change: each transcendental float method call (`sin`, `cos`, `sin_cos`, `atan2`, `hypot`, `cbrt`,
`atanh`, `atan`) goes through `libm` (`src/lm.rs`), so that the desktop and the browser's WASM give the same bits
(docs/adr/0008); the release calls Rust's own methods, glibc's on the desktop. The binary `geodsolve` is left out.

Written by `python3 scripts/vendor/geographiclib.py` from the release's archive (its SHA-256 is Cargo's); `--check`
fails when this folder is not what the release and the rule give. Do not edit the files by hand.

Measured on Karney's GeodTest (500 000 geodesics with exact answers; 4 October 2026): the inverse problem's largest
length error is 11 nm here and 7.5 nm in the release (GeographicLib states 15 nm), 2.8 nm in both for geodesics under
10 km; the direct problem's 9.5 nm and 8.7 nm in latitude, 12.6 nm in longitude in both. Four of the release's unit
tests pin glibc's last bit with `assert_eq!` and part by one ulp here; the rest pass.
