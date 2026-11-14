//! An extension's tool used by a form's action or a model's step (docs/adr/0441 §6, docs/adr/8057 §12–§13;
//! plan/form.md FORM-API-13, plan/eklenti.md PY-TOOL-20).
//!
//! The action or step names the tool `ext.<eklenti>.<araç>` and, once its author chose it, the extension it came from:
//! its id, version and package digest ([`ExtensionPin`]). On this computer the tool is one of three:
//!
//! - **ready**: the extension is active, its tool listed and its Python can run;
//! - **pending**: the extension is here but its code cannot run yet (Python off, its environment not prepared): a
//!   form's button is off and says what to do, nothing is started to fail;
//! - **unavailable**: the extension is missing, turned off or held, or its version has no such tool: said with the
//!   extension and version the form or model expects. Nothing is installed for it.
//!
//! The designers pin the extension when its tool is chosen and say when the one installed is another version or
//! another build of it (`form_action_pin`, the model step's line): its descriptor, code, lock or capabilities may have
//! changed since.

use kentos_commands::ExtensionPin;

use crate::app::App;

/// Whether an extension's tool runs now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PinnedTool {
    Ready,
    /// The extension is here, its code cannot run yet: why and what to do.
    Pending(String),
    /// Not here: which tool or extension, and what to do.
    Unavailable(String),
}

impl App {
    /// Whether `tool` runs now on this computer, `pin` the extension its action or step was made with.
    pub(crate) fn pinned_tool_state(&self, tool: &str, pin: Option<&ExtensionPin>) -> PinnedTool {
        let extension = kentos_commands::action_tool_extension(tool);
        if self.gis.processing.registry.get(tool).is_some() {
            return match extension
                .and_then(|ext| self.extensions.python_reasons.get(ext))
                .and_then(Option::as_deref)
            {
                Some(why) => PinnedTool::Pending(why.to_owned()),
                None => PinnedTool::Ready,
            };
        }
        let Some(ext) = extension else {
            return PinnedTool::Unavailable(format!(
                "“{tool}” bu bilgisayarda kayıtlı bir işlem aracı değil."
            ));
        };
        // One extension's state and manifest, not the manager's list (it copies every log): this runs while drawing.
        let registry = self.extensions.registry.as_ref();
        let state = registry.and_then(|r| r.state(ext));
        let name = self.extension_name(ext);
        PinnedTool::Unavailable(match state {
            Some(kentos_extension_host::State::Active) => format!(
                "{name} {} bu aracı içermiyor{}.",
                registry
                    .and_then(|r| r.manifest(ext))
                    .map(|m| m.version.to_string())
                    .unwrap_or_default(),
                pin.map_or_else(String::new, |p| format!(
                    " (form {} ile yapıldı)",
                    p.version
                ))
            ),
            Some(state) => format!("{name} — {}. {}", state.label(), state.help()),
            None => match pin {
                Some(p) => format!(
                    "{ext} {} bu bilgisayarda kurulu değil: Eklentiler'den kurun.",
                    p.version
                ),
                None => {
                    format!("{ext} eklentisi bu bilgisayarda kurulu değil: Eklentiler'den kurun.")
                }
            },
        })
    }

    /// The pin of an extension's tool as it is installed now; none for a built-in tool or a missing extension.
    pub(crate) fn tool_pin_of(&self, tool: &str) -> Option<ExtensionPin> {
        let ext = kentos_commands::action_tool_extension(tool)?;
        let registry = self.extensions.registry.as_ref()?;
        Some(ExtensionPin {
            extension: ext.to_owned(),
            version: registry.manifest(ext)?.version.to_string(),
            digest: registry.digest(ext)?,
        })
    }

    /// What the designer says about an extension tool's pin (`form_action_pin`): none when it matches the extension
    /// installed, or when the tool is built in or its extension missing (that is `form_action_unknown`'s).
    pub(crate) fn tool_pin_drift(&self, tool: &str, pin: Option<&ExtensionPin>) -> Option<String> {
        let now = self.tool_pin_of(tool)?;
        let name = self.extension_name(&now.extension);
        match pin {
            None => Some(format!(
                "{name} eklentisinin aracı sabitlenmemiş: aracı listeden yeniden seçin, form hangi sürümle yapıldığını bilsin."
            )),
            Some(p) if p.version != now.version => Some(format!(
                "Araç {name} {} ile seçildi, kurulu sürüm {}: aracın tanımı, kodu ya da istediği yetenekler değişmiş \
                 olabilir. Aracı yeniden seçmek sabitlemeyi yeniler.",
                p.version, now.version
            )),
            Some(p) if p.digest != now.digest => Some(format!(
                "Araç {name} {} ile seçildi; kurulu {} aynı sürüm ama içeriği farklı. Aracı yeniden seçmek \
                 sabitlemeyi yeniler.",
                p.version, now.version
            )),
            Some(_) => None,
        }
    }

    /// What a model's step says about its extension's tool (docs/adr/8057 §13): pending or unavailable with why; the
    /// version it was added with, and the one installed when they differ. None for a built-in tool.
    pub(crate) fn step_pin_note(&self, tool: &str, pin: Option<&ExtensionPin>) -> Option<String> {
        let ext = kentos_commands::action_tool_extension(tool)?;
        match self.pinned_tool_state(tool, pin) {
            PinnedTool::Pending(why) => return Some(format!("Bekliyor: {why}")),
            PinnedTool::Unavailable(why) => return Some(format!("Kullanılamaz: {why}")),
            PinnedTool::Ready => {}
        }
        let name = self.extension_name(ext);
        Some(match (pin, self.tool_pin_of(tool)) {
            (Some(p), Some(now)) if p.version != now.version => format!(
                "{name} {} ile eklendi; kurulu sürüm {}: aracın tanımı, kodu ya da istediği yetenekler değişmiş \
                 olabilir.",
                p.version, now.version
            ),
            (Some(p), Some(now)) if p.digest != now.digest => format!(
                "{name} {} ile eklendi; kurulu {} aynı sürüm ama içeriği farklı.",
                p.version, now.version
            ),
            (Some(p), _) => format!("{name} {} ile eklendi.", p.version),
            (None, _) => format!(
                "{name} eklentisinin aracı; adım sabitlenmeden eklenmiş, model hangi sürümle yapıldığını bilmiyor."
            ),
        })
    }
}
