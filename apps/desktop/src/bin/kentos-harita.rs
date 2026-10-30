//! `kentos-harita [dosya]`: KentOS Harita, the desktop library fixed to its module (docs/adr/0801).

fn main() -> std::process::ExitCode {
    kentos_desktop::module_program(kentos_launch::Module::Map, std::env::args_os().skip(1))
}
