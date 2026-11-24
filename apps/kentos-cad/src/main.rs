//! The one KentOS desktop program (docs/adr/0801, 0802 §1): `kentos-cad`, every module at once, and each
//! module under its own name (`kentos-mimari`, `kentos-harita`, `kentos-gis`, `kentos-yol`: links to this file) or with
//! `--modul`: the library's own words (`snapshot`, `kullan`), then `[--modul <id>] [dosya]`. A module's program
//! registers its own package alone, kentos-cad every one, before the window starts (docs/adr/9199 §3.1, §4.15).

use std::ffi::OsString;

use kentos_launch::Module;

fn main() -> std::process::ExitCode {
    let args: Vec<OsString> = std::env::args_os().collect();
    match module_of(&args) {
        Some(module) => register(module),
        None => Module::ALL.iter().copied().for_each(register),
    }
    kentos_desktop::program_main(args)
}

/// The module this start is the program of: the name it was started under, else `--modul`; none for kentos-cad (and
/// for its own words, which take every module).
fn module_of(args: &[OsString]) -> Option<Module> {
    let named = args.first().and_then(|a| Module::of_program_name(a));
    named.or_else(|| kentos_launch::args::parse(args.iter().skip(1).cloned()).ok().and_then(|l| l.module))
}

/// A module's package's parts among the program's; a new module does not build without its package here.
fn register(module: Module) {
    match module {
        Module::Architecture => kentos_desktop_arch::register(),
        Module::Gis => kentos_desktop_cbs::register(),
        Module::Map => kentos_desktop_map::register(),
        Module::Road => kentos_desktop_road::register(),
    }
}
