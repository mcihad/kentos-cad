//! What starting an app runs (docs/adr/0800 §4). A KentOS module runs the
//! one KentOS program under its own name (`kentos-gis`, a link to it) when
//! that is installed, else as `kentos-cad --modul cbs` (docs/adr/0802 §1: one
//! file, every module), found next to the launcher and then on `PATH`
//! (`kentos_launch::programs`). Any other program is the manifest's path
//! (relative to its folder) or a name on `PATH`. A place the user showed
//! (Konumu seç…) comes first. What is not found says why, so that the card
//! can say it and offer to show the place.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use kentos_launch::{Module, args};

use crate::manifest::{Launch, Manifest, State};

/// The program every KentOS module can run in (docs/adr/0801).
pub const KENTOS_CAD: &str = "kentos-cad";

/// Finds a program by its name.
pub trait Finder {
    fn find(&self, name: &str) -> Option<PathBuf>;
}

/// Next to the launcher, then on `PATH`.
pub struct System;

impl Finder for System {
    fn find(&self, name: &str) -> Option<PathBuf> {
        kentos_launch::programs::find(name)
    }
}

/// What an app is, ready to start or not.
#[derive(Debug, Clone, PartialEq)]
pub enum Found {
    /// A program to run, with the arguments that come before a file's.
    Program(Spec),
    /// A web address for the browser.
    Url(String),
    /// Not out yet.
    Soon,
    /// Nothing to run: why, as the card says it.
    Missing(String),
}

/// A process to start.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Spec {
    pub program: PathBuf,
    pub args: Vec<OsString>,
    pub env: Vec<(String, String)>,
    pub cwd: Option<PathBuf>,
    /// For a KentOS module: the module the program is asked for, when the
    /// program is `kentos-cad` (its own program needs no flag).
    pub module: Option<Module>,
    /// The manifest's own arguments, before `{dosya}` is put in.
    pub template: Vec<String>,
}

/// What `app` runs. `place` is where the user showed its program.
pub fn resolve(app: &Manifest, place: Option<&Path>, finder: &dyn Finder) -> Found {
    if app.state == State::Soon {
        return Found::Soon;
    }
    let Some(launch) = &app.launch else {
        return Found::Missing("Manifest nasıl başlayacağını söylemiyor.".to_owned());
    };
    match launch {
        Launch::Url(url) => Found::Url(url.clone()),
        Launch::Kentos(module) => {
            if let Some(place) = place {
                return if place.is_file() {
                    Found::Program(kentos_spec(*module, place.to_path_buf()))
                } else {
                    Found::Missing(format!(
                        "Gösterilen yer “{}” artık yok. Konumu yeniden seçin.",
                        place.display()
                    ))
                };
            }
            // Each module is an application of its own (docs/adr/0802; the owner: “her biri ayrı uygulama
            // olarak çalışacak”) in the one KentOS program: under its own name, else kentos-cad as it.
            match finder
                .find(module.program())
                .or_else(|| finder.find(KENTOS_CAD))
            {
                Some(program) => Found::Program(kentos_spec(*module, program)),
                None => Found::Missing(format!(
                    "{} bulunamadı: ne {} ne kentos-cad launcher'ın yanında ya da PATH'te. {}'yı kurun ya da Konumu seç… ile programı gösterin.",
                    module.title(),
                    module.program(),
                    module.title()
                )),
            }
        }
        Launch::Program {
            program,
            args,
            env,
            cwd,
        } => {
            let base = app.origin.base();
            let path = match place {
                Some(place) if place.is_file() => place.to_path_buf(),
                Some(place) => {
                    return Found::Missing(format!(
                        "Gösterilen yer “{}” artık yok. Konumu yeniden seçin.",
                        place.display()
                    ));
                }
                None => {
                    let given = Path::new(program);
                    let has_folder = given.is_absolute() || given.components().count() > 1;
                    if has_folder {
                        let path = match base {
                            Some(base) if !given.is_absolute() => base.join(given),
                            _ => given.to_path_buf(),
                        };
                        if !path.is_file() {
                            return Found::Missing(format!(
                                "“{}” yok. Programı kurun ya da Konumu seç… ile yerini gösterin.",
                                path.display()
                            ));
                        }
                        path
                    } else {
                        match finder.find(program) {
                            Some(path) => path,
                            None => {
                                return Found::Missing(format!(
                                    "“{program}” launcher'ın yanında ve PATH'te bulunamadı. Programı kurun ya da Konumu seç… ile yerini gösterin."
                                ));
                            }
                        }
                    }
                }
            };
            let cwd = match (cwd, base) {
                (Some(cwd), Some(base)) if cwd.is_relative() => Some(base.join(cwd)),
                (Some(cwd), _) => Some(cwd.clone()),
                (None, base) => base.map(Path::to_path_buf),
            };
            Found::Program(Spec {
                program: path,
                args: Vec::new(),
                env: env.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                cwd,
                module: None,
                template: args.clone(),
            })
        }
    }
}

fn kentos_spec(module: Module, program: PathBuf) -> Spec {
    let flagged = is_kentos_cad(&program).then_some(module);
    Spec {
        program,
        module: flagged,
        ..Spec::default()
    }
}

/// Whether a program is `kentos-cad` itself (any module), by its file's stem.
pub fn is_kentos_cad(program: &Path) -> bool {
    program
        .file_stem()
        .is_some_and(|stem| stem.eq_ignore_ascii_case(KENTOS_CAD))
}

impl Spec {
    /// The arguments it starts with, given `file` to open or none.
    /// `kentos-cad` takes `--modul <id>` and the file (`kentos_launch::args`); a
    /// module's own program the file alone; a manifest's program its own
    /// arguments with `{dosya}` replaced, or the file after them. Without a
    /// file, an argument that carries `{dosya}` is left out.
    pub fn with_file(&self, file: Option<&Path>) -> Spec {
        let args = if self.template.is_empty() && self.args.is_empty() {
            args::command_line(self.module, file)
        } else {
            let mut out: Vec<OsString> = self.args.clone();
            let mut placed = false;
            for arg in &self.template {
                if arg.contains(FILE) {
                    match file {
                        Some(file) if arg == FILE => out.push(file.as_os_str().to_owned()),
                        Some(file) => {
                            out.push(OsString::from(arg.replace(FILE, &file.to_string_lossy())))
                        }
                        // No file: the argument that carries it goes.
                        None => {}
                    }
                    placed = true;
                } else {
                    out.push(OsString::from(arg));
                }
            }
            if let (false, Some(file)) = (placed, file) {
                out.push(file.as_os_str().to_owned());
            }
            out
        };
        Spec {
            args,
            template: Vec::new(),
            ..self.clone()
        }
    }
}

/// The word a manifest's argument puts a file in.
pub const FILE: &str = "{dosya}";

/// A file of `files`' extensions (the manifest's), or any file when it lists none.
pub fn opens(app: &Manifest, path: &Path) -> bool {
    app.files.is_empty()
        || path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .is_some_and(|ext| app.files.contains(&ext))
}

/// Splits what the add dialog's arguments field says into arguments: words
/// apart by spaces, a quoted part one word (`"a b"` or `'a b'`), a
/// backslash keeping the character after it.
pub fn split_args(text: &str) -> Result<Vec<String>, String> {
    let mut args = Vec::new();
    let mut word = String::new();
    let mut in_word = false;
    let mut quote: Option<char> = None;
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (None, '"' | '\'') => {
                quote = Some(c);
                in_word = true;
            }
            (_, '\\') => {
                if let Some(next) = chars.next() {
                    word.push(next);
                    in_word = true;
                }
            }
            (None, c) if c.is_whitespace() => {
                if in_word {
                    args.push(std::mem::take(&mut word));
                    in_word = false;
                }
            }
            (_, c) => {
                word.push(c);
                in_word = true;
            }
        }
    }
    if let Some(q) = quote {
        return Err(format!("Kapanmayan tırnak ({q}): tırnağı kapatın."));
    }
    if in_word {
        args.push(word);
    }
    Ok(args)
}

/// The arguments written back for the add dialog: quoted where they need it.
pub fn join_args(args: &[String]) -> String {
    args.iter()
        .map(|a| {
            if a.is_empty()
                || a.chars()
                    .any(|c| c.is_whitespace() || c == '"' || c == '\'' || c == '\\')
            {
                format!("\"{}\"", a.replace('\\', "\\\\").replace('"', "\\\""))
            } else {
                a.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::manifest::{self, Folder, Origin};
    use std::collections::HashMap;

    /// Programs found where a test says.
    pub struct Fixed(pub HashMap<&'static str, PathBuf>);

    impl Finder for Fixed {
        fn find(&self, name: &str) -> Option<PathBuf> {
            self.0.get(name).cloned()
        }
    }

    fn app(text: &str, origin: Origin) -> Manifest {
        manifest::read(text, origin).expect("reads").manifest
    }

    fn gis() -> Manifest {
        app(
            r#"{"format": "kentos.launcher-app/1", "id": "kentos-gis", "launch": {"kind": "kentos", "module": "cbs"}}"#,
            Origin::Builtin,
        )
    }

    fn os(args: &[&str]) -> Vec<OsString> {
        args.iter().map(OsString::from).collect()
    }

    #[test]
    fn a_module_runs_under_its_own_name_else_as_kentos_cad() {
        let own = Fixed(HashMap::from([
            ("kentos-gis", PathBuf::from("/opt/kentos/kentos-gis")),
            ("kentos-cad", PathBuf::from("/opt/kentos/kentos-cad")),
        ]));
        let Found::Program(spec) = resolve(&gis(), None, &own) else {
            panic!("found");
        };
        assert_eq!(spec.program, PathBuf::from("/opt/kentos/kentos-gis"));
        assert_eq!(spec.with_file(None).args, Vec::<OsString>::new());
        assert_eq!(
            spec.with_file(Some(Path::new("/ev/harita.kcad"))).args,
            os(&["/ev/harita.kcad"]),
            "its own program: the file alone"
        );

        let cad_only = Fixed(HashMap::from([(
            "kentos-cad",
            PathBuf::from("/opt/kentos/kentos-cad"),
        )]));
        // The one KentOS program without the module's name beside it (docs/adr/0802 §1): kentos-cad as the module.
        let Found::Program(spec) = resolve(&gis(), None, &cad_only) else {
            panic!("kentos-cad runs the module");
        };
        assert_eq!(spec.program, PathBuf::from("/opt/kentos/kentos-cad"));
        assert_eq!(
            spec.with_file(Some(Path::new("/ev/harita.kcad"))).args,
            os(&["--modul", "cbs", "/ev/harita.kcad"])
        );
        // Neither: the card says what to install.
        let none = Fixed(HashMap::new());
        let Found::Missing(why) = resolve(&gis(), None, &none) else {
            panic!("missing");
        };
        assert!(
            why.contains("KentOS GIS bulunamadı")
                && why.contains("kentos-gis")
                && why.contains("kentos-cad"),
            "{why}"
        );
        // kentos-cad shown with Konumu seç… is the user's own choice: it starts in the module.
        let Found::Program(spec) = resolve(
            &gis(),
            Some(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("Cargo.toml")
                    .as_path(),
            ),
            &cad_only,
        ) else {
            panic!("the place shown");
        };
        assert_eq!(
            spec.with_file(None).args,
            Vec::<OsString>::new(),
            "a program not named kentos-cad: no flag"
        );
    }

    #[test]
    fn every_module_builds_the_command_line_the_desktop_reads() {
        let installed = Fixed(
            Module::ALL
                .iter()
                .map(|m| (m.program(), PathBuf::from(format!("/k/{}", m.program()))))
                .collect(),
        );
        for module in Module::ALL {
            let text = format!(
                r#"{{"format": "kentos.launcher-app/1", "id": "m", "launch": {{"kind": "kentos", "module": "{}"}}}}"#,
                module.id()
            );
            let Found::Program(spec) = resolve(&app(&text, Origin::Builtin), None, &installed)
            else {
                panic!("found");
            };
            assert_eq!(
                spec.program,
                PathBuf::from(format!("/k/{}", module.program()))
            );
            let file = Path::new("/ev/çizim dosyası.kcad");
            let line = spec.with_file(Some(file)).args;
            // The module's own program: the file alone, as its main reads it (kentos_launch::args).
            let read = kentos_launch::args::parse(line).expect("the desktop reads it");
            assert_eq!(read.module, None);
            assert_eq!(read.path.as_deref(), Some(file));
        }
    }

    #[test]
    fn a_place_shown_comes_first_and_a_gone_one_is_said() {
        let dir = crate::files::scratch("resolve-place");
        let shown = dir.join("kentos-cad");
        std::fs::write(&shown, b"").expect("a program");
        let none = Fixed(HashMap::new());
        let Found::Program(spec) = resolve(&gis(), Some(&shown), &none) else {
            panic!("found");
        };
        assert_eq!(
            spec.with_file(None).args,
            os(&["--modul", "cbs"]),
            "kentos-cad by its name"
        );
        let gone = dir.join("silindi");
        let Found::Missing(why) = resolve(&gis(), Some(&gone), &none) else {
            panic!("missing");
        };
        assert!(why.contains("artık yok"), "{why}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_manifest_program_by_path_by_name_with_its_arguments() {
        let dir = crate::files::scratch("resolve-program");
        std::fs::create_dir_all(dir.join("bin")).expect("a folder");
        std::fs::write(dir.join("bin/rapor"), b"").expect("a program");
        let origin = Origin::File {
            path: dir.join("rapor.json"),
            folder: Folder::User,
        };
        let by_path = app(
            r#"{"format": "kentos.launcher-app/1", "id": "r", "name": "R",
                "launch": {"kind": "program", "program": "bin/rapor", "args": ["--girdi={dosya}", "-v"],
                           "env": {"A": "1"}, "cwd": "veri"}}"#,
            origin.clone(),
        );
        let none = Fixed(HashMap::new());
        let Found::Program(spec) = resolve(&by_path, None, &none) else {
            panic!("found");
        };
        assert_eq!(
            spec.program,
            dir.join("bin/rapor"),
            "relative to the manifest"
        );
        assert_eq!(spec.cwd, Some(dir.join("veri")));
        assert_eq!(spec.env, [("A".to_owned(), "1".to_owned())]);
        assert_eq!(
            spec.with_file(Some(Path::new("/ev/a b.csv"))).args,
            os(&["--girdi=/ev/a b.csv", "-v"])
        );
        assert_eq!(
            spec.with_file(None).args,
            os(&["-v"]),
            "no file: its argument goes"
        );

        let by_name = app(
            r#"{"format": "kentos.launcher-app/1", "id": "p", "name": "P",
                "launch": {"kind": "program", "program": "python3", "args": ["rapor.py"]}}"#,
            origin.clone(),
        );
        let Found::Missing(why) = resolve(&by_name, None, &none) else {
            panic!("missing");
        };
        assert!(
            why.contains("“python3” launcher'ın yanında ve PATH'te bulunamadı"),
            "{why}"
        );
        let python = Fixed(HashMap::from([(
            "python3",
            PathBuf::from("/usr/bin/python3"),
        )]));
        let Found::Program(spec) = resolve(&by_name, None, &python) else {
            panic!("found");
        };
        assert_eq!(
            spec.cwd,
            Some(dir.clone()),
            "the manifest's folder when none is given"
        );
        assert_eq!(
            spec.with_file(Some(Path::new("/ev/x.csv"))).args,
            os(&["rapor.py", "/ev/x.csv"]),
            "no {{dosya}}: the file after the arguments"
        );

        let missing_path = app(
            r#"{"format": "kentos.launcher-app/1", "id": "y", "name": "Y",
                "launch": {"kind": "program", "program": "/yok/program"}}"#,
            origin,
        );
        let Found::Missing(why) = resolve(&missing_path, None, &python) else {
            panic!("missing");
        };
        assert!(why.contains("“/yok/program” yok"), "{why}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn soon_apps_and_web_addresses() {
        let soon = app(
            r#"{"format": "kentos.launcher-app/1", "id": "kentos-yol", "name": "KentOS YOL", "state": "soon"}"#,
            Origin::Builtin,
        );
        assert_eq!(resolve(&soon, None, &System), Found::Soon);
        let web = app(
            r#"{"format": "kentos.launcher-app/1", "id": "w", "name": "W", "launch": {"kind": "url", "url": "https://kentos.app"}}"#,
            Origin::Builtin,
        );
        assert_eq!(
            resolve(&web, None, &System),
            Found::Url("https://kentos.app".into())
        );
    }

    #[test]
    fn files_an_app_opens() {
        let mimari = Registry::builtin_app("kentos-mimari");
        assert!(opens(&mimari, Path::new("/a/b.DXF")));
        assert!(!opens(&mimari, Path::new("/a/b.gpkg")));
        assert!(!opens(&mimari, Path::new("/a/b")));
    }

    #[test]
    fn arguments_split_and_join_back() {
        assert_eq!(
            split_args(r#"rapor.py "ada parsel.csv" --kat='1 2' a\ b {dosya}"#).expect("splits"),
            ["rapor.py", "ada parsel.csv", "--kat=1 2", "a b", "{dosya}"]
        );
        assert_eq!(split_args("   ").expect("empty"), Vec::<String>::new());
        assert_eq!(split_args(r#""""#).expect("an empty word"), [""]);
        assert!(
            split_args(r#"a "b"#)
                .expect_err("open quote")
                .contains("Kapanmayan tırnak")
        );
        let args = vec![
            "r.py".to_owned(),
            "a b.csv".to_owned(),
            r#"x"y"#.to_owned(),
            String::new(),
        ];
        assert_eq!(split_args(&join_args(&args)).expect("back"), args);
    }

    use crate::registry::Registry;

    impl Registry {
        fn builtin_app(id: &str) -> Manifest {
            Registry::builtin().get(id).cloned().expect("a bundled app")
        }
    }
}
