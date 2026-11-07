# block 0.1.6, its extern static inhabited

The crates.io release of [block](https://github.com/SSheldon/rust-block) 0.1.6 (MIT, Steven Sheldon), Apple's C
blocks for Rust, which wgpu-hal's Metal backend uses through the `metal` crate (iced 0.14, wgpu 27). One change in
`src/lib.rs`: `enum Class { }` becomes an opaque zero-sized `#[repr(C)]` struct. The release's extern static
`_NSConcreteStackBlock: Class` was a static of an uninhabited type, a future error (rust-lang/rust#74840) that cargo
reported at every build; only its address is taken, which is the same. The release's tests are left out (they need an
`objc_test_utils` crate the release does not ship). Its `extern` blocks name no ABI (the C ABI, written so in 2017):
as a path dependency its lints are not capped as a crates.io one's are, so the manifest allows that one lint.

Written by `python3 scripts/vendor/block.py` from the release's archive (its SHA-256 is the one Cargo locked);
`--check` fails when this folder is not what the release and the rule give. Do not edit the files by hand. When iced
and wgpu move to `objc2`'s blocks, take the copy out and the patch with it.
