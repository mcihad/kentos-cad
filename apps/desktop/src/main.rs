//! The one KentOS desktop program (docs/adr/0801, 0802 §1): `kentos-cad`, every module at once, and each
//! module under its own name (`kentos-mimari`, `kentos-harita`, `kentos-gis`: links to this file): the
//! library's own words (`snapshot`, `kullan`), then `[--modul <id>] [dosya]`.
// The linker's note that __eh_frame passes 16 MB (docs/adr/1136): about 205,000 of the program's functions are
// written in DWARF mode by the compiler, and the compact table cannot point that far; unwinding still works, only
// its lookup is slower. No linker setting shrinks it, so the note alone is quiet here.
#![allow(linker_messages)]

fn main() -> std::process::ExitCode {
    kentos_desktop::program_main(std::env::args_os())
}
