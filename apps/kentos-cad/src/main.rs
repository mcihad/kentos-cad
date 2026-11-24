//! The one KentOS desktop program (docs/adr/0801, 0802 §1): `kentos-cad`, every module at once, and each
//! module under its own name (`kentos-mimari`, `kentos-harita`, `kentos-gis`: links to this file): the
//! library's own words (`snapshot`, `kullan`), then `[--modul <id>] [dosya]`. The modules that are packages of their own
//! register their parts first (docs/adr/9199 §3.1); the others are still in the core's package.

fn main() -> std::process::ExitCode {
    kentos_desktop_map::register();
    kentos_desktop_road::register();
    kentos_desktop::program_main(std::env::args_os())
}
