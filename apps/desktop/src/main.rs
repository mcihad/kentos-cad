//! The one KentOS desktop program (docs/adr/0801, 0802 §1): `kentos-cad`, every module at once, and each
//! module under its own name (`kentos-mimari`, `kentos-harita`, `kentos-gis`: links to this file): the
//! library's own words (`snapshot`, `kullan`), then `[--modul <id>] [dosya]`.

fn main() -> std::process::ExitCode {
    kentos_desktop::program_main(std::env::args_os())
}
