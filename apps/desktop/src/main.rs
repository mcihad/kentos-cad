//! `kentos-cad`, every KentOS module at once (docs/adr/0801): the desktop
//! library's own words (`snapshot`, `kullan`), then `[--modul <id>] [dosya]`.

fn main() -> std::process::ExitCode {
    kentos_desktop::all_in_one(std::env::args_os().skip(1))
}
