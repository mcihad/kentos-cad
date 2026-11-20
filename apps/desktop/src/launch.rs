//! Command-line entry points and window startup.

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use crate::{
    app, blocks_library, cloud, dialog_sizes, forms, grids, layout, material_library, processing,
    program, python, python_setup, recent, recovery, settings, sheets, snapshot, sources, style,
    usage, windows,
};

use kentos_launch::Module;
use kentos_ui::theme::typography;

/// The exit code of a command line the program could not read.
const BAD_ARGUMENTS: u8 = 2;

/// The program's entry (docs/adr/0802 §1): by the name it was started under (`args`' first word), a
/// module's own program (`kentos-mimari`, a link to this one file) or `kentos-cad`, every module.
pub fn program_main(args: impl IntoIterator<Item = OsString>) -> ExitCode {
    let mut args = args.into_iter();
    let named = args.next();
    match named.as_deref().and_then(Module::of_program_name) {
        Some(module) => module_program(module, args),
        None => all_in_one(args),
    }
}

/// `kentos-cad`: its own words first (`snapshot`, `kullan`), then
/// `[--modul <id>] [dosya]` (kentos-launch's command line). A command line it
/// cannot read is said on stderr, with exit code 2.
pub fn all_in_one(args: impl IntoIterator<Item = OsString>) -> ExitCode {
    let args: Vec<OsString> = args.into_iter().collect();
    let words = || {
        args.iter()
            .skip(1)
            .map(|a| a.to_string_lossy().into_owned())
    };
    match args.first().and_then(|a| a.to_str()) {
        // `kentos-cad snapshot çıktı.png [çizim.kcad]`: the window without a window.
        Some("snapshot") => return said(snapshot::run(words())),
        // `kentos-cad kullan <iz>`: a usage scenario played and pictured step by step.
        Some("kullan") => return said(usage::run(words())),
        _ => {}
    }
    match kentos_launch::args::parse(args) {
        Ok(launch) => run(launch.module, launch.path),
        Err(why) => refused(&why),
    }
}

/// A module's own program (`kentos-mimari [dosya]`, the one file under the module's name): `--modul` may
/// name only this module, any other is refused with the program to start instead.
pub fn module_program(module: Module, args: impl IntoIterator<Item = OsString>) -> ExitCode {
    match program::fixed_launch(module, args) {
        Ok(path) => run(Some(module), path),
        Err(why) => refused(&why),
    }
}

/// A word's run, its error on stderr.
fn said(result: Result<(), String>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn refused(why: &str) -> ExitCode {
    eprintln!("{why}");
    ExitCode::from(BAD_ARGUMENTS)
}

/// The window: `program` the module it works as (none: every module, kentos-cad's
/// own), `path` the drawing it opens at once.
fn run(program: Option<Module>, path: Option<PathBuf>) -> ExitCode {
    // An extension package double-clicked (PY-KEK-15, docs/adr/8057 §3) is not a drawing: its review opens over
    // Eklentiler, nothing is installed until the user says Kur.
    let (path, package) = match path {
        Some(p) if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("kek")) => (None, Some(p)),
        other => (other, None),
    };
    // Embedded faces: nothing has to be installed on the machine.
    typography::load();
    // Bound layers on PostGIS (docs/adr/0505).
    kentos_postgis::register();
    // The data drivers and the device's passwords for data layers (docs/adr/9101).
    crate::data_layers::register();
    kentos_processing::command::register_as("desktop");
    // STAC searches (docs/adr/0528 §2): over the cloud client's HTTP.
    kentos_native_application::stac_host::register(Box::new(kentos_cloud::stac::HttpFetch));
    kentos_native_application::remote_host::register(Box::new(kentos_cloud::range::HttpRemote));

    // Today's midnight for Yeni sürüm oluştur and Sona erdir is the device's (docs/adr/0210 §7).
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    kentos_interaction::time_version::LOCAL_OFFSET.store(
        cloud::local_time::Zone::system().offset_at(now),
        std::sync::atomic::Ordering::Relaxed,
    );

    // One daemon whose windows the app opens (windows/, docs/adr/8060): the main window at once, with the settings it
    // always had, and a window of its own for a dialog taken out of it. A daemon does not end with its last window:
    // the main window's close ends the program (cloud/leaving.rs).
    let result = iced::daemon(
        // The user's settings file (docs/adr/0023); in memory where no configuration folder is known.
        move || {
            let settings = settings::Settings::config_dir()
                .map_or_else(settings::Settings::memory, |dir| {
                    settings::Settings::open(&dir, std::time::SystemTime::now())
                });
            // Recovery copies of unsaved work (docs/adr/0030): none when the data folder cannot be used.
            let recovery =
                match recovery::default_root().map(|root| recovery::Recovery::open(&root)) {
                    Some(Ok(recovery)) => recovery,
                    Some(Err(error)) => recovery::Recovery::unavailable(error),
                    None => recovery::Recovery::off(),
                };
            let (mut app, task) = app::App::start_in(program, path.clone(), settings, recovery);
            // Device drafts of cloud projects (docs/adr/0040, 0041), next to the recovery copies.
            app.cloud.drafts = cloud::default_drafts();
            // Local copies of cloud projects: they open without a connection (docs/adr/0043).
            app.cloud.replicas = cloud::default_replicas();
            // The sheets' books, by the project's key, beside them (docs/sheet/design.md §10).
            app.sheet_store = sheets::default_store();
            // The device's NTv2 grids for the projects' datum choices (docs/adr/0168 §4).
            app.arch.grids.library = grids::Library::device();
            // The forms designer's section library and kept drafts (docs/adr/0441 §1, §5).
            app.gis.form_files = forms::library::FormFiles::device();
            // The user's AI agents, in a file of their own readable by the user alone (docs/adr/8056 §4).
            app.assistant.store = kentos_agent::ProfileStore::device();
            app.assistant.ledger = kentos_agent::UsageLedger::device();
            app.assistant.policy = kentos_extension_host::places::load_policy(
                &kentos_extension_host::places::policy_path(),
            );
            app.assistant.refresh();
            if let Some(kept) = app.assistant.store.take_set_aside() {
                app.output(format!(
                    "Yapay zekâ ajanlarının dosyası okunamadı; {} olarak bir kenara alındı. Ajanları Uygulama ayarları › Yapay zekâ'dan yeniden tanımlayın.",
                    kept.display()
                ));
            }
            // Python ve eklentiler (plan/eklenti.md ADIM 18A): the user's choice; uv looked for in the background only
            // while it is on (python_detect below).
            app.python_setup = python_setup::PythonSetup::device();
            // Eklentiler (docs/adr/8057 §3): the user's registry and IT's policy; the enabled extensions started,
            // none on a start without extensions.
            let (extensions, notes) = crate::extensions::Extensions::device();
            app.extensions = extensions;
            // An extension that did not start is an error with its name (docs/adr/8057 §4), not a passing note.
            for (level, note) in notes {
                app.say(level, note);
            }
            app.extensions_changed();
            // Sorunlu eklentiyi bul going on: its question again, over the manager.
            if app.extensions.bisect.is_some() {
                app.open_extensions();
            }
            if let Some(package) = &package {
                app.review_kek_file(package);
            }
            if let Some(folder) = app.extensions.startup_catalog() {
                app.open_extension_catalog(&folder, false);
            }
            // The recent files, kept beside the program's other history.
            if let Some(folder) = recent::RecentFiles::default_folder() {
                app.recent = recent::RecentFiles::open(&folder);
                // Kaynaklar's folders, beside them (docs/adr/0199 §7).
                app.gis.sources.folders = sources::SourceFolders::open(&folder);
                // Bloklar's libraries, beside them (docs/adr/0726 §1).
                app.blocks_panel.library = blocks_library::Library::open(&folder);
                // Malzemeler's library folders, beside them (docs/adr/0791).
                app.arch.material_libraries = material_library::Libraries::open(&folder);
                // The Python tab's unsaved script, kept beside them (docs/adr/0136).
                app.python.script = python::script::Script::load(&folder);
                // Each processing tool's last values (islemler.json), beside them.
                app.gis.processing.memory = processing::memory::Memory::open(&folder);
                // The user's models (the designer's Kaydet) beside the built-in ones.
                let models = app.gis.processing.memory.models().to_vec();
                app.gis.processing.registry.set_user_models(models);
            }
            // Kitaplığım, the user's own symbols (kitaplik.kstil, docs/adr/0092).
            if let Some(folder) = style::user_library::default_folder()
                && let Some(problem) = app.styles.open_user_library(&folder)
            {
                app.warn(problem);
            }
            // The layout kept last time (yerlesim.json beside the settings, docs/adr/0115), each program's
            // own (docs/adr/0802).
            if let Some(dir) = settings::Settings::config_dir() {
                if let Some(problem) = app.arch.templates_panel.open_personal(&dir) {
                    app.warn(problem);
                }
                // The user's tool palette (arac-paleti.json, docs/adr/1016).
                if let Some(problem) = app.open_personal_tools(&dir) {
                    app.warn(problem);
                }
                // Each window's size (pencereler.json, docs/adr/0746).
                app.dialog_sizes = dialog_sizes::Keeper::open(&dir);
                // Which dialogs open in a window of their own, and its size (ayri-pencereler.json, docs/adr/8060).
                app.windows.memory = windows::Memory::open(&dir);
                app.layout = layout::Keeper::open_for(&dir, program);
                app.apply_layout();
            }
            // Without a drawing named, the start screen (when the preference wants it).
            if path.is_none() {
                app.start_at_launch();
            }
            // The server cell's first answer (the web asks once it is idle after start).
            let server = app.check_server_quietly();
            // The main window, opened first: the app's first tasks run once it is there, as with one window.
            let (main, opened) = iced::window::open(main_window(program));
            app.windows.set_main(main);
            // The main window's device pixels per logical pixel: the brand's pictures are chosen by it.
            let scale = iced::window::scale_factor(main).map(app::Message::DeviceScale);
            let python = app.python_detect();
            let first = iced::Task::batch([task, server, scale, python]);
            #[cfg(target_os = "macos")]
            let first = iced::Task::batch([first, app.accessibility.start()]);
            (app, opened.discard().chain(first))
        },
        app::App::update,
        app::App::window_view,
    )
    .title(app::App::window_title)
    .theme(app::App::window_theme)
    .default_font(typography::ui())
    .subscription(app::App::subscription)
    .antialiasing(true)
    .run();
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

/// The main window: the program's icon, 1440 × 900, its class to the desktop; a close asks first.
fn main_window(_program: Option<Module>) -> iced::window::Settings {
    iced::window::Settings {
        icon: kentos_ui::widget::brand::window_icon(),
        size: iced::Size::new(1440.0, 900.0),
        // The smallest window the layouts are checked at: the ribbon fits (docs/adr/0051),
        // the windows keep their buttons in view.
        min_size: Some(iced::Size::new(1100.0, 650.0)),
        // A drawing with unsaved changes asks before the window closes.
        exit_on_close_request: false,
        // Each program is an application of its own to the desktop (docs/adr/0802): its window's class and
        // Wayland application id is its program's name, as its .desktop file is named (apps/desktop/packaging).
        #[cfg(target_os = "linux")]
        platform_specific: iced::window::settings::PlatformSpecific {
            application_id: _program.map_or("kentos-cad", Module::program).to_owned(),
            ..Default::default()
        },
        ..iced::window::Settings::default()
    }
}
