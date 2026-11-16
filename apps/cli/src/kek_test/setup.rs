//! The test's ground (docs/adr/8057 §74): the folder installed into a registry of its own, KentOS's Python found (or
//! installed when asked), the extension's environment prepared from its lock as the desktop prepares it.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use kentos_extension_host::{Choice, Grant, Options, Registry, Source, SourceKind};
use kentos_extension_tools::Python;
use kentos_extensions::{Limits, Manifest, pack};
use kentos_python_env::compat::compat;
use kentos_python_env::extension::{self, Ready};
use kentos_python_env::python::{self, Cause, InstallError};
use kentos_python_env::settings::PythonSettings;
use kentos_python_env::uv::{self, UvStatus};

use super::Stop;

/// The folder packed and installed into a registry of the test's own under `root`, enabled, everything it asks for
/// granted, the organisation's policy as the computer's; its package folder.
pub(super) fn install_into(
    root: &Path,
    folder: &Path,
    files: &std::collections::BTreeMap<String, Vec<u8>>,
    manifest: &Manifest,
) -> Result<PathBuf, Stop> {
    let packed = pack(files, &Limits::default()).map_err(|p| {
        Stop::Failed(format!(
            "Paketlenemedi: {}",
            p.first().map(|p| p.to_string()).unwrap_or_default()
        ))
    })?;
    let host = kentos_extensions::Host::this(
        kentos_extensions::version::Version::parse(env!("CARGO_PKG_VERSION"))
            .unwrap_or_else(|| kentos_extensions::version::Version::new(0, 1, 0)),
    );
    let options = Options {
        policy: kentos_extension_host::places::load_policy(
            &kentos_extension_host::places::policy_path(),
        ),
        ..Options::default()
    };
    let mut registry = Registry::open(root, host, options)
        .map_err(|p| Stop::CouldNot(format!("Sınamanın kayıt defteri açılamadı: {p}")))?;
    let choice = Choice {
        grant: Grant {
            capabilities: manifest.capabilities().into_iter().collect(),
            python: true,
        },
        enable: true,
        replace_source: false,
    };
    let source = Source {
        kind: SourceKind::DeveloperFolder,
        location: folder.display().to_string(),
    };
    registry
        .install(&packed.bytes, &source, &choice)
        .map_err(|problems| {
            let said: Vec<String> = problems.iter().map(ToString::to_string).collect();
            Stop::Failed(format!("Kurulamadı: {}", said.join("; ")))
        })?;
    registry
        .package_dir(&manifest.id)
        .ok_or_else(|| Stop::Failed("Kuruldu ama paket klasörü bulunamadı.".to_owned()))
}

/// KentOS's Python as the desktop finds it (the Python folder, uv as chosen or found), installed with uv when
/// `install` and it is not there. Not the desktop's on/off choice: the command is its caller's own.
pub(super) fn python_of(install: bool, tell: &dyn Fn(String)) -> Result<Python, Stop> {
    let root = kentos_python_env::places::root().ok_or_else(|| {
        Stop::CouldNot(
            "Bu bilgisayarda kullanıcı veri klasörü yok (XDG_DATA_HOME ya da HOME); KENTOS_PYTHON_DIR ile bir \
             klasör verin."
                .to_owned(),
        )
    })?;
    let chosen = PythonSettings::load(&root);
    let uv = match uv::detect(chosen.uv_path.as_deref().map(Path::new)) {
        UvStatus::Found { path, .. } => path,
        other => return Err(Stop::CouldNot(other.words())),
    };
    let need = &compat().python;
    let installed = || python::installed(&uv, &root, &need.range).map_err(Stop::CouldNot);
    let interpreter = match installed()? {
        Some(found) => found,
        None if install => {
            tell(format!(
                "KentOS'un Python'u kuruluyor: Python {} ({}) …",
                need.install, need.size
            ));
            python::install(
                &uv,
                &root,
                &need.install,
                &AtomicBool::new(false),
                &mut |_| {},
            )
            .map_err(|e| Stop::CouldNot(e.words(&need.install)))?;
            installed()?.ok_or_else(|| {
                Stop::CouldNot("Python kuruldu ama uv onu bulamadı; yeniden deneyin.".to_owned())
            })?
        }
        None => {
            return Err(Stop::CouldNot(format!(
                "KentOS'un Python'u kurulu değil ({}): KentOS masaüstünde Uygulama ayarları › Python ve \
                 eklentiler'den kurun ya da komutu --install-python ile çalıştırın (uv Python {} sürümünü indirir; {}).",
                root.display(),
                need.install,
                need.size
            )));
        }
    };
    Ok(Python {
        interpreter,
        root,
        uv,
    })
}

/// The installed package's environment from its lock, prepared as the desktop prepares it (a ready one used as it
/// is), and whether it was made now. The lock's fault is the extension's; the network's, the disk's and uv's are the
/// computer's.
pub(super) fn environment(
    python: &Python,
    package: &Path,
    manifest: &Manifest,
    project_file: &str,
    lock_file: &str,
) -> Result<(Ready, bool), Stop> {
    let version = manifest.version.to_string();
    let key = extension::key_for(
        package,
        lock_file,
        &manifest.id,
        &version,
        &python.interpreter,
        &[],
    )
    .map_err(Stop::Failed)?;
    let wheel =
        kentos_python_env::sdk::find(&kentos_python_env::sdk::dirs(), env!("CARGO_PKG_VERSION"))
            .map_err(Stop::CouldNot)?;
    let made = extension::ready(&python.root, &key).is_none();
    extension::prepare(
        &python.uv,
        &python.root,
        &python.interpreter,
        &wheel,
        package,
        (project_file, lock_file),
        &key,
        &AtomicBool::new(false),
        &mut |_| {},
    )
    .map_err(|e| {
        let words = format!(
            "Eklentinin Python ortamı hazırlanamadı: {}",
            extension::prepare_words(&e)
        );
        match e {
            InstallError::Failed {
                cause: Cause::Other | Cause::Unavailable,
                ..
            } => Stop::Failed(words),
            _ => Stop::CouldNot(words),
        }
    })
    .map(|ready| (ready, made))
}
