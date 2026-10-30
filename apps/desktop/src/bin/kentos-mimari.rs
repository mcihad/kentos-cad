//! `kentos-mimari [dosya]`: KentOS Mimari, the desktop library fixed to its module (docs/adr/0801).

fn main() -> std::process::ExitCode {
    kentos_desktop::module_program(
        kentos_launch::Module::Architecture,
        std::env::args_os().skip(1),
    )
}
