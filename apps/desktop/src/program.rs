//! Which KentOS program this is (docs/adr/0801): `kentos-cad`, every module at
//! once, or one module's own program (KentOS Mimari, KentOS Harita, KentOS
//! GIS; `kentos-cad --modul <id>` is the same). The module a drawing belongs to
//! by its type and CAD work (kentos-launch's rule), the ribbon's work, the
//! window's title, the recent files a module shows, and starting the program a
//! file belongs to.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use kentos_commands::{CadWork, ProjectSettings, ProjectType, Workspace};
use kentos_launch::Module;

use iced::Task;

use crate::app::{App, Message};
use crate::catalog::catalog;
use crate::opening::{Opened, Purpose};
use crate::project::ask_work::{give_work, guessed_work_in};
use crate::recent::Recent;

/// The module a project belongs to: none for a CAD project not asked its work
/// yet and a project not asked its type.
pub(crate) fn module_of(s: &ProjectSettings) -> Option<Module> {
    Module::of_project(
        s.project_type().map(Workspace::id),
        s.cad_work().map(CadWork::id),
    )
}

/// The CAD works a module's projects are drawn for, the one its new projects start with first.
pub(crate) fn works_of(m: Module) -> Vec<CadWork> {
    m.cad_works()
        .iter()
        .filter_map(|id| CadWork::from_id(id))
        .collect()
}

/// The cloud catalog's types of a module's projects (kentos-launch's `Module::project_types`, docs/adr/0802), in
/// its order: the first is a new cloud project's.
pub(crate) fn project_types_of(m: Module) -> Vec<ProjectType> {
    m.project_types()
        .iter()
        .filter_map(|name| ProjectType::from_name(name))
        .collect()
}

/// The module a cloud project of catalog type `t` belongs to.
pub(crate) fn module_of_project_type(t: ProjectType) -> Option<Module> {
    Module::of_project_type(t.name())
}

/// The type a module's projects have.
pub(crate) fn type_of(m: Module) -> Workspace {
    match m.project_type() {
        "gis" => Workspace::Gis,
        _ => Workspace::Cad,
    }
}

/// The module a CAD work's projects belong to.
pub(crate) fn module_of_work(w: CadWork) -> Module {
    Module::of_project(Some(Workspace::Cad.id()), Some(w.id())).unwrap_or(Module::Architecture)
}

/// “KentOS GIS'te aç”: a module's program as the button that starts it says it.
pub(crate) fn open_in(m: Module) -> &'static str {
    match m {
        Module::Architecture => "KentOS Mimari'de aç",
        Module::Map => "KentOS Harita'da aç",
        Module::Gis => "KentOS GIS'te aç",
    }
}

/// “KentOS GIS'te”: a module's program in a sentence, where something opens.
pub(crate) fn in_program(m: Module) -> &'static str {
    match m {
        Module::Architecture => "KentOS Mimari'de",
        Module::Map => "KentOS Harita'da",
        Module::Gis => "KentOS GIS'te",
    }
}

/// “KentOS Harita'nın”: whose something is, a module's program in a sentence.
pub(crate) fn of_program(m: Module) -> &'static str {
    match m {
        Module::Architecture => "KentOS Mimari'nin",
        Module::Map => "KentOS Harita'nın",
        Module::Gis => "KentOS GIS'in",
    }
}

/// What a module's projects are, in a sentence (“bir KentOS GIS projesi”).
pub(crate) fn a_project_of(m: Module) -> String {
    format!("bir {} projesi", m.title())
}

/// A module program's command line (`kentos-gis [dosya]`): `--modul` may name
/// only the program's own module; the file to open, if any.
pub(crate) fn fixed_launch(
    module: Module,
    args: impl IntoIterator<Item = OsString>,
) -> Result<Option<PathBuf>, String> {
    let launch = kentos_launch::args::parse(args)?;
    match launch.module {
        Some(other) if other != module => Err(format!(
            "{} yalnız {} olarak çalışır; {} için {} programını ya da kentos-cad --modul {} komutunu kullanın.",
            module.program(),
            module.title(),
            other.title(),
            other.program(),
            other.id()
        )),
        _ => Ok(launch.path),
    }
}

/// Starts the program `path` belongs to: `module`'s own beside this one or on
/// `PATH`, else `kentos-cad --modul <id>`. The program started, or why none
/// was: nothing is opened here either way.
pub(crate) fn start_program(module: Module, path: &Path) -> Result<String, String> {
    use kentos_launch::{args, programs};
    let (program, args) = match programs::find(module.program()) {
        Some(own) => (own, args::command_line(None, Some(path))),
        None => match programs::find("kentos-cad") {
            Some(all) => (all, args::command_line(Some(module), Some(path))),
            None => {
                return Err(format!(
                    "{} bulunamadı: {} ya da kentos-cad bu programın yanında ya da PATH'te yok. {} kurulunca dosyayı onunla açın.",
                    module.title(),
                    module.program(),
                    module.title()
                ));
            }
        },
    };
    let mut child = std::process::Command::new(&program)
        .args(&args)
        .spawn()
        .map_err(|e| {
            format!(
                "{} başlatılamadı ({}): {e}.",
                module.title(),
                program.display()
            )
        })?;
    // The program goes on by itself; its exit is collected so it leaves nothing behind.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(program.display().to_string())
}

impl App {
    /// What a program does with a drawing it has read (docs/adr/0801):
    /// kentos-cad places every one. A module's program places its own module's
    /// drawing; asks a CAD drawing without a work, or a project without a type,
    /// what it is for, with its own work first; hands a file of another module
    /// to that module's program, and a CAD drawing whose work is not known to a
    /// CAD program from KentOS GIS, opening nothing. A DWG, DXF or NCZ a CAD
    /// module opens is drawn for its work. A recovery copy opens wherever it is
    /// offered: it is unsaved work.
    pub(crate) fn opened_in_program(&mut self, mut opened: Opened) -> Task<Message> {
        let Some(program) = self.program else {
            return self.place_opened(opened);
        };
        if matches!(opened.purpose, Purpose::Recovery { .. }) {
            return self.place_opened(opened);
        }
        let settings = opened.doc.settings();
        match (module_of(settings), settings.project_type()) {
            (Some(m), _) if m == program => self.place_opened(opened),
            (Some(m), _) => {
                self.ask_hand_over(&opened, vec![m]);
                Task::none()
            }
            (None, Some(Workspace::Cad)) if program == Module::Gis => {
                self.ask_hand_over(&opened, vec![Module::Architecture, Module::Map]);
                Task::none()
            }
            (None, Some(Workspace::Cad)) if opened.external => {
                let work = guessed_work_in(program, settings);
                give_work(&mut opened, work);
                self.place_opened(opened)
            }
            (None, Some(Workspace::Cad)) => {
                self.ask_cad_work_held(opened, false);
                Task::none()
            }
            (None, None) => {
                self.ask_cad_work_held(opened, true);
                Task::none()
            }
            // An announced type (plan3d …) shows as the fallback wherever it opens.
            (None, Some(_)) => self.place_opened(opened),
        }
    }

    /// The type a project not asked its type yet shows as: the program's own;
    /// kentos-cad's, CBS (docs/adr/0165 §1).
    pub(crate) fn program_type(&self) -> Option<Workspace> {
        self.program.map(type_of)
    }

    /// The work the CAD ribbon is cut to (docs/adr/0801): the drawing's own, a
    /// CAD module's first while the drawing has none; none (the whole CAD
    /// ribbon) in kentos-cad for a CAD project not asked its work, and for
    /// any other type.
    pub(crate) fn ribbon_work(&self) -> Option<CadWork> {
        if self.work_mode() != Workspace::Cad {
            return None;
        }
        let own = self.document.as_ref().and_then(|d| d.settings().cad_work());
        own.or_else(|| self.program.and_then(|m| works_of(m).first().copied()))
    }

    /// The module of the drawing on screen, as the recent files keep it.
    pub(crate) fn document_module(&self) -> Option<Module> {
        self.document.as_ref().and_then(|d| module_of(d.settings()))
    }

    /// The recent files this program shows: a module's own and those no module
    /// was kept for; every one in kentos-cad.
    pub(crate) fn shown_recent(&self) -> Vec<&Recent> {
        self.recent
            .list()
            .iter()
            .filter(|r| match (self.program, r.module) {
                (Some(program), Some(module)) => program == module,
                _ => true,
            })
            .collect()
    }

    /// Whether this program offers a command (docs/adr/0802): every one in kentos-cad, a module's own in a
    /// module's program (Komut ara, the command line, the shortcuts, the quick access bar, F1 and the drawing's
    /// menus find nothing else; `App::run` refuses it, `App::available` is false).
    pub(crate) fn offers(&self, id: &str) -> bool {
        self.program.is_none_or(|m| catalog().module_has(m, id))
    }

    /// Why a command typed or found is not run here (docs/adr/0802): the programs that have it.
    pub(crate) fn not_offered(&self, id: &str, title: &str) -> Option<String> {
        let here = self.program?;
        if catalog().module_has(here, id) {
            return None;
        }
        let others: Vec<&str> = Module::ALL
            .into_iter()
            .filter(|m| *m != here && catalog().module_has(*m, id))
            .map(in_program)
            .collect();
        Some(if others.is_empty() {
            format!("“{title}” {} yok.", in_program(here))
        } else {
            format!(
                "“{title}” {} yok; {} var.",
                in_program(here),
                others.join(" ve ")
            )
        })
    }

    /// The program's name in the window's title and on the start screen.
    pub(crate) fn program_title(&self) -> &'static str {
        self.program.map_or("KentOS CAD", Module::title)
    }

    /// Why this program does not run a tool's method typed by its name (`SEMT`: Ölçülendirme's Semt), none
    /// when it does (docs/adr/0802): a module's program runs a method its own ribbon shows, or one no
    /// ribbon shows of a tool it offers; kentos-cad every one.
    pub(crate) fn method_not_offered(&self, id: &str, option: &str, label: &str) -> Option<String> {
        let here = self.program?;
        if let Some(why) = self.not_offered(id, label) {
            return Some(why);
        }
        let mine = self.ribbon_tabs().chain(self.contextual_tabs());
        if splits_hold(mine, id, option) {
            return None;
        }
        let anywhere = catalog().every_tab().chain(
            catalog()
                .contextual_for_work(Workspace::Cad, None)
                .chain(catalog().contextual_for_work(Workspace::Gis, None)),
        );
        if !splits_hold(anywhere, id, option) {
            return None;
        }
        let others: Vec<&str> = Module::ALL
            .into_iter()
            .filter(|m| {
                let mode = type_of(*m);
                let work = works_of(*m).first().copied();
                *m != here
                    && splits_hold(
                        catalog()
                            .tabs_for(mode, work)
                            .chain(catalog().contextual_for_work(mode, work)),
                        id,
                        option,
                    )
            })
            .map(in_program)
            .collect();
        Some(if others.is_empty() {
            format!("“{label}” {} yok.", in_program(here))
        } else {
            format!(
                "“{label}” {} yok; {} var.",
                in_program(here),
                others.join(" ve ")
            )
        })
    }
}

/// Whether a split button of these tabs holds the tool `id`'s method `option`.
fn splits_hold<'a>(
    tabs: impl Iterator<Item = &'a crate::catalog::Tab>,
    id: &str,
    option: &str,
) -> bool {
    tabs.flat_map(|tab| tab.panels.iter())
        .flat_map(|panel| panel.items.iter())
        .any(|item| match item {
            crate::catalog::Item::Split { entries, .. } => entries
                .iter()
                .any(|e| e.id == id && e.option == Some(option)),
            _ => false,
        })
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::PathBuf;

    use kentos_commands::{CadWork, Workspace};
    use kentos_launch::Module;

    use super::*;
    use crate::files_testing::app_with_drawing;

    fn os(args: &[&str]) -> Vec<OsString> {
        args.iter().map(OsString::from).collect()
    }

    #[test]
    fn a_project_belongs_by_its_type_and_work() {
        let mut s = crate::files_testing::drawing(0).settings().clone();
        let mut of = |w: Option<Workspace>, work: Option<CadWork>| {
            s.workspace = w;
            s.cad_work = work;
            module_of(&s)
        };
        assert_eq!(of(Some(Workspace::Gis), None), Some(Module::Gis));
        assert_eq!(
            of(Some(Workspace::Cad), Some(CadWork::Architecture)),
            Some(Module::Architecture)
        );
        assert_eq!(
            of(Some(Workspace::Cad), Some(CadWork::Map)),
            Some(Module::Map)
        );
        assert_eq!(
            of(Some(Workspace::Cad), None),
            None,
            "a CAD project not asked its work"
        );
        assert_eq!(of(None, None), None, "a project not asked its type");
        assert_eq!(
            of(Some(Workspace::LegacyHybrid), None),
            None,
            "the former Hibrit"
        );
        // A work on a project that is not CAD is no work.
        assert_eq!(
            of(Some(Workspace::Gis), Some(CadWork::Map)),
            Some(Module::Gis)
        );
        assert_eq!(of(None, Some(CadWork::Map)), None);
        // Every work is one module's, the module's own works its own.
        for w in CadWork::ALL {
            assert!(works_of(module_of_work(w)).contains(&w));
        }
        assert_eq!(works_of(Module::Architecture), [CadWork::Architecture]);
        assert_eq!(works_of(Module::Map), [CadWork::Map]);
        assert!(works_of(Module::Gis).is_empty());
        assert_eq!(
            Module::ALL.map(type_of),
            [Workspace::Cad, Workspace::Cad, Workspace::Gis]
        );
    }

    #[test]
    fn a_module_program_takes_only_its_own_module_and_one_file() {
        assert_eq!(fixed_launch(Module::Gis, os(&[])), Ok(None));
        assert_eq!(
            fixed_launch(Module::Gis, os(&["/a/ada.kcad"])),
            Ok(Some(PathBuf::from("/a/ada.kcad")))
        );
        assert_eq!(
            fixed_launch(Module::Gis, os(&["--modul", "cbs", "/a/ada.kcad"])),
            Ok(Some(PathBuf::from("/a/ada.kcad")))
        );
        let e = fixed_launch(Module::Gis, os(&["--modul=mimari", "x.kcad"]))
            .expect_err("another module");
        assert!(
            e.contains("kentos-gis yalnız KentOS GIS olarak çalışır")
                && e.contains("kentos-mimari")
                && e.contains("--modul mimari"),
            "{e}"
        );
        let e = fixed_launch(Module::Map, os(&["a.kcad", "b.kcad"])).expect_err("two files");
        assert!(e.contains("“b.kcad” fazla"), "{e}");
    }

    #[test]
    fn a_module_shows_its_own_recent_files_and_those_of_no_module() {
        let mut app = app_with_drawing();
        app.recent.add_in(
            std::path::Path::new("/p/bina.kcad"),
            "1".into(),
            Some(Module::Architecture),
        );
        app.recent.add_in(
            std::path::Path::new("/p/pafta.kcad"),
            "2".into(),
            Some(Module::Map),
        );
        app.recent
            .add_in(std::path::Path::new("/p/eski.kcad"), "3".into(), None);
        app.recent.add_in(
            std::path::Path::new("/p/mahalle.kcad"),
            "4".into(),
            Some(Module::Gis),
        );
        let names = |app: &App| {
            app.shown_recent()
                .iter()
                .map(|r| r.name.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            names(&app),
            ["mahalle.kcad", "eski.kcad", "pafta.kcad", "bina.kcad"],
            "kentos-cad shows every one"
        );
        app.program = Some(Module::Map);
        assert_eq!(names(&app), ["eski.kcad", "pafta.kcad"]);
        app.program = Some(Module::Gis);
        assert_eq!(names(&app), ["mahalle.kcad", "eski.kcad"]);
        app.program = Some(Module::Architecture);
        assert_eq!(names(&app), ["eski.kcad", "bina.kcad"]);
    }
}
