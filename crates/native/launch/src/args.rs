//! The command line that starts a KentOS program in a module (docs/adr/0801):
//! `kentos-cad [--modul <id>] [dosya]`. The launcher writes it with
//! [`command_line`], the program reads it with [`parse`]; the program's own
//! words (`snapshot`, `kullan`) are its own and come before this.

use std::ffi::OsString;
use std::path::PathBuf;

use crate::Module;

/// The flag naming the module.
pub const MODULE_FLAG: &str = "--modul";

/// What a program was started with.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Launch {
    /// The module to start in; none when not named.
    pub module: Option<Module>,
    /// A file to open.
    pub path: Option<PathBuf>,
}

/// Reads the arguments after the program's name. A file named more than once,
/// an unknown module or a flag without its value is refused with a sentence
/// that says why and what to write instead.
pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Launch, String> {
    let mut launch = Launch::default();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let text = arg.to_string_lossy();
        if let Some(value) = text.strip_prefix(&format!("{MODULE_FLAG}=")) {
            launch.module = Some(module(value)?);
        } else if text == MODULE_FLAG {
            let value = args
                .next()
                .ok_or_else(|| format!("{MODULE_FLAG} bir modül adı ister: {}.", known()))?;
            launch.module = Some(module(&value.to_string_lossy())?);
        } else if launch.path.is_some() {
            return Err(format!(
                "Tek dosya açılır; “{text}” fazla. Öbür dosyayı program açıkken Dosya → Aç ile açın."
            ));
        } else {
            launch.path = Some(PathBuf::from(arg));
        }
    }
    Ok(launch)
}

/// The arguments that start a program in `module` with `path`, in the order
/// [`parse`] reads them.
pub fn command_line(module: Option<Module>, path: Option<&std::path::Path>) -> Vec<OsString> {
    let mut args = Vec::new();
    if let Some(m) = module {
        args.push(OsString::from(MODULE_FLAG));
        args.push(OsString::from(m.id()));
    }
    if let Some(p) = path {
        args.push(p.as_os_str().to_owned());
    }
    args
}

fn module(id: &str) -> Result<Module, String> {
    Module::from_id(id).ok_or_else(|| format!("“{id}” bir KentOS modülü değil; {}.", known()))
}

fn known() -> String {
    let ids: Vec<&str> = Module::ALL.iter().map(|m| m.id()).collect();
    format!("bilinenler {}", ids.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn os(args: &[&str]) -> Vec<OsString> {
        args.iter().map(OsString::from).collect()
    }

    #[test]
    fn a_module_and_a_file_in_either_order_and_either_spelling() {
        let want = Launch {
            module: Some(Module::Map),
            path: Some(PathBuf::from("/a/pafta.kcad")),
        };
        assert_eq!(
            parse(os(&["--modul", "harita", "/a/pafta.kcad"])),
            Ok(want.clone())
        );
        assert_eq!(parse(os(&["/a/pafta.kcad", "--modul=harita"])), Ok(want));
        assert_eq!(parse(os(&[])), Ok(Launch::default()));
        assert_eq!(
            parse(os(&["x.dxf"])),
            Ok(Launch {
                module: None,
                path: Some(PathBuf::from("x.dxf"))
            })
        );
    }

    #[test]
    fn what_is_written_reads_back() {
        for m in [None, Some(Module::Architecture), Some(Module::Gis)] {
            for p in [None, Some(std::path::Path::new("/ev/proje dosyası.kcad"))] {
                let back = parse(command_line(m, p)).expect("reads");
                assert_eq!(back.module, m);
                assert_eq!(back.path.as_deref(), p);
            }
        }
    }

    #[test]
    fn refusals_say_why() {
        let e = parse(os(&["--modul", "yol"])).expect_err("unknown");
        assert!(
            e.contains("“yol”") && e.contains("mimari, harita, cbs"),
            "{e}"
        );
        let e = parse(os(&["--modul"])).expect_err("no value");
        assert!(e.contains("modül adı ister"), "{e}");
        let e = parse(os(&["a.kcad", "b.kcad"])).expect_err("two files");
        assert!(e.contains("“b.kcad” fazla"), "{e}");
    }
}
