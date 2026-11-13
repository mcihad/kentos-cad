//! The user's agents (docs/adr/8056 §4): each a name, a provider, a model, an address when it is not the provider's,
//! where its API key comes from, and whether it is the default one. Exactly one agent is the default while there is
//! any.
//!
//! The agents live in `$XDG_CONFIG_HOME/kentos-cad/yapay-zeka-ajanlari.json` (else `~/.config/kentos-cad/`), readable
//! by the user alone (0600), as the device's database connections do; a file that does not read is set aside, never
//! written over. A key is kept in that file only for an agent whose key is [`KeySource::Stored`]; one typed for this
//! session stays in memory ([`ProfileStore::remember_session_key`]); the provider's environment variable is the last
//! resort ([`ProfileStore::key_for`]). A key never shows in `Debug`, in an error or in a log.

mod file;

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde::{Deserialize, Serialize};

use crate::providers::{Preset, ProviderKind};

pub use file::device_file;

/// Where an agent's API key comes from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KeySource {
    /// Kept in the agents' file (readable by the user alone).
    Stored,
    /// Typed for this session, kept in memory until the program ends.
    #[default]
    Session,
    /// The provider's environment variable (`ANTHROPIC_API_KEY` …).
    Environment,
}

/// An agent: a provider, a model and how to reach it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentProfile {
    /// Unique on the device; given when the agent is added.
    #[serde(default)]
    pub id: String,
    /// Its name in the lists and the chat window.
    pub name: String,
    pub kind: ProviderKind,
    pub model: String,
    /// The address when it is not the provider's (a region, the China address, a local server's port).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(default)]
    pub key: KeySource,
    #[serde(default)]
    pub default: bool,
}

/// A field of the agent window, for where a problem is said.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileField {
    Name,
    Model,
    BaseUrl,
}

impl AgentProfile {
    /// A new agent of `kind` (no id yet, the key typed for the session, not the default).
    pub fn new(
        kind: ProviderKind,
        name: impl Into<String>,
        model: impl Into<String>,
    ) -> AgentProfile {
        AgentProfile {
            id: String::new(),
            name: name.into(),
            kind,
            model: model.into(),
            base_url: None,
            key: if kind.preset().needs_key {
                KeySource::Session
            } else {
                KeySource::Environment
            },
            default: false,
        }
    }

    pub fn preset(&self) -> Preset {
        self.kind.preset()
    }

    /// The address requests go to: the agent's own, else the provider's; none when neither is there.
    pub fn endpoint(&self) -> Option<String> {
        self.base_url
            .as_deref()
            .map(str::trim)
            .filter(|u| !u.is_empty())
            .or(self.preset().base_url)
            .map(|u| u.trim_end_matches('/').to_owned())
    }

    /// What is wrong with it, field by field, before it is kept or tried.
    pub fn problems(&self) -> Vec<(ProfileField, String)> {
        let mut out = Vec::new();
        if self.name.trim().is_empty() {
            out.push((
                ProfileField::Name,
                "Ajana bir ad verin; sohbet penceresinde bu adla görünür.".to_owned(),
            ));
        }
        if self.model.trim().is_empty() {
            out.push((
                ProfileField::Model,
                "Bir model seçin ya da modelin kimliğini yazın.".to_owned(),
            ));
        }
        match self.endpoint() {
            None => out.push((
                ProfileField::BaseUrl,
                "Sunucunun adresini yazın (örneğin http://localhost:8000/v1).".to_owned(),
            )),
            Some(url) if !(url.starts_with("https://") || url.starts_with("http://")) => out.push((
                ProfileField::BaseUrl,
                "Adres https:// ile başlamalı (bu bilgisayardaki bir sunucu için http://localhost:…).".to_owned(),
            )),
            Some(_) => {}
        }
        out
    }
}

/// An API key; never printed.
#[derive(Clone, PartialEq, Eq)]
struct Secret(String);

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<gizli>")
    }
}

#[derive(Default)]
struct State {
    profiles: Vec<AgentProfile>,
    stored: HashMap<String, Secret>,
    session: HashMap<String, Secret>,
    /// The file set aside when it did not read, for the user to be told once.
    set_aside: Option<PathBuf>,
    /// Why the file could not be read at all (not a parse error): nothing is written over it.
    unreadable: Option<String>,
}

/// The device's agents, read once and written whole on each change.
pub struct ProfileStore {
    path: Option<PathBuf>,
    state: Mutex<State>,
}

impl fmt::Debug for ProfileStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = self.lock();
        let mut stored: Vec<&String> = s.stored.keys().collect();
        let mut session: Vec<&String> = s.session.keys().collect();
        stored.sort();
        session.sort();
        f.debug_struct("ProfileStore")
            .field("path", &self.path)
            .field("profiles", &s.profiles)
            .field("stored_keys_for", &stored)
            .field("session_keys_for", &session)
            .finish()
    }
}

impl ProfileStore {
    /// The agents kept in `path` (a file; its folder is made when first written).
    pub fn at(path: impl Into<PathBuf>) -> ProfileStore {
        let path = path.into();
        let state = file::read(&path);
        ProfileStore {
            path: Some(path),
            state: Mutex::new(state),
        }
    }

    /// The device's agents: `$XDG_CONFIG_HOME/kentos-cad/yapay-zeka-ajanlari.json`, else `~/.config/kentos-cad/…`; in
    /// memory only when neither folder is known. Keep one store for the program's life: session keys live in it.
    pub fn device() -> ProfileStore {
        match device_file() {
            Some(path) => ProfileStore::at(path),
            None => ProfileStore::in_memory(),
        }
    }

    /// Agents kept in memory only.
    pub fn in_memory() -> ProfileStore {
        ProfileStore {
            path: None,
            state: Mutex::new(State::default()),
        }
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Reads the file again (another program changed it); this session's keys stay.
    pub fn reload(&self) {
        let Some(path) = &self.path else { return };
        let fresh = file::read(path);
        let mut s = self.lock();
        let session = std::mem::take(&mut s.session);
        *s = fresh;
        s.session = session;
    }

    /// Writes the agents and the stored keys, readable by the user alone.
    pub fn save(&self) -> Result<(), String> {
        let s = self.lock();
        self.write(&s)
    }

    /// The agents, in the order they were added.
    pub fn list(&self) -> Vec<AgentProfile> {
        self.lock().profiles.clone()
    }

    pub fn get(&self, id: &str) -> Option<AgentProfile> {
        self.lock().profiles.iter().find(|p| p.id == id).cloned()
    }

    /// The default agent: the one marked, there is always one while there is any agent.
    pub fn default_profile(&self) -> Option<AgentProfile> {
        let s = self.lock();
        s.profiles
            .iter()
            .find(|p| p.default)
            .or(s.profiles.first())
            .cloned()
    }

    /// Keeps a new agent and gives it back with its id (given when empty). The first agent, or one marked default,
    /// becomes the default. Refused: its problems, or an id another agent has.
    pub fn add(&self, profile: AgentProfile) -> Result<AgentProfile, String> {
        if let Some((_, why)) = profile.problems().into_iter().next() {
            return Err(why);
        }
        let mut s = self.lock();
        let mut profile = tidy(profile);
        if profile.id.is_empty() {
            profile.id = uuid::Uuid::new_v4().to_string();
        }
        if s.profiles.iter().any(|p| p.id == profile.id) {
            return Err(format!("“{}” kimlikli bir ajan zaten var.", profile.id));
        }
        if profile.default || s.profiles.is_empty() {
            for p in &mut s.profiles {
                p.default = false;
            }
            profile.default = true;
        }
        s.profiles.push(profile.clone());
        self.write(&s)?;
        Ok(profile)
    }

    /// Changes the agent with `profile`'s id. Marked default, it becomes the default; the default stays the default
    /// until another is marked. A key moves with its source: kept → this session's, this session's → kept.
    pub fn update(&self, profile: AgentProfile) -> Result<(), String> {
        if let Some((_, why)) = profile.problems().into_iter().next() {
            return Err(why);
        }
        let mut s = self.lock();
        let profile = tidy(profile);
        let Some(at) = s.profiles.iter().position(|p| p.id == profile.id) else {
            return Err(format!(
                "“{}” adlı ajan bulunamadı; liste yenilenmiş olabilir.",
                profile.name
            ));
        };
        let before = s.profiles[at].key;
        let id = profile.id.clone();
        match (before, profile.key) {
            (KeySource::Stored, KeySource::Session) => {
                if let Some(k) = s.stored.remove(&id) {
                    s.session.insert(id.clone(), k);
                }
            }
            (KeySource::Session, KeySource::Stored) => {
                if let Some(k) = s.session.get(&id).cloned() {
                    s.stored.insert(id.clone(), k);
                }
            }
            (KeySource::Stored, KeySource::Environment) => {
                s.stored.remove(&id);
            }
            _ => {}
        }
        let was_default = s.profiles[at].default;
        if profile.default {
            for p in &mut s.profiles {
                p.default = false;
            }
        }
        s.profiles[at] = AgentProfile {
            default: profile.default || was_default,
            ..profile
        };
        self.write(&s)
    }

    /// Forgets the agent and its keys; whether there was one. The first remaining agent becomes the default when it
    /// was.
    pub fn remove(&self, id: &str) -> Result<bool, String> {
        let mut s = self.lock();
        let Some(at) = s.profiles.iter().position(|p| p.id == id) else {
            return Ok(false);
        };
        let gone = s.profiles.remove(at);
        s.stored.remove(id);
        s.session.remove(id);
        if gone.default
            && let Some(first) = s.profiles.first_mut()
        {
            first.default = true;
        }
        self.write(&s).map(|()| true)
    }

    /// Makes the agent the only default.
    pub fn set_default(&self, id: &str) -> Result<(), String> {
        let mut s = self.lock();
        if !s.profiles.iter().any(|p| p.id == id) {
            return Err(
                "Varsayılan yapılacak ajan bulunamadı; liste yenilenmiş olabilir.".to_owned(),
            );
        }
        for p in &mut s.profiles {
            p.default = p.id == id;
        }
        self.write(&s)
    }

    /// Takes a key typed for the agent where its source says: kept in the file (`Stored`), in memory (`Session`).
    /// An agent whose key comes from the environment takes none.
    pub fn set_key(&self, id: &str, key: &str) -> Result<(), String> {
        let key = key.trim();
        let mut s = self.lock();
        let Some(source) = s.profiles.iter().find(|p| p.id == id).map(|p| p.key) else {
            return Err(
                "Anahtarı verilecek ajan bulunamadı; liste yenilenmiş olabilir.".to_owned(),
            );
        };
        if key.is_empty() {
            return Err("Anahtar boş; sağlayıcının panelinden kopyalayıp yapıştırın.".to_owned());
        }
        match source {
            KeySource::Stored => {
                s.stored.insert(id.to_owned(), Secret(key.to_owned()));
                self.write(&s)
            }
            KeySource::Session => {
                s.session.insert(id.to_owned(), Secret(key.to_owned()));
                Ok(())
            }
            KeySource::Environment => Err(
                "Bu ajanın anahtarı ortam değişkeninden okunuyor; anahtarı saklamak için kaynağını değiştirin."
                    .to_owned(),
            ),
        }
    }

    /// Keeps `key` in memory for the agent until the program ends; it is never written.
    pub fn remember_session_key(&self, id: &str, key: &str) {
        let key = key.trim();
        if key.is_empty() {
            return;
        }
        self.lock()
            .session
            .insert(id.to_owned(), Secret(key.to_owned()));
    }

    /// Forgets the agent's kept and session keys (it was refused, say).
    pub fn forget_key(&self, id: &str) -> Result<(), String> {
        let mut s = self.lock();
        s.session.remove(id);
        if s.stored.remove(id).is_some() {
            self.write(&s)?;
        }
        Ok(())
    }

    /// The agent's key: the kept one, else this session's, else the provider's environment variable.
    pub fn key_for(&self, profile: &AgentProfile) -> Option<String> {
        self.key_and_origin(profile, |name| std::env::var(name).ok())
            .map(|(k, _)| k)
    }

    /// Where the agent's key would come from now; none when there is none.
    pub fn key_origin(&self, profile: &AgentProfile) -> Option<KeySource> {
        self.key_and_origin(profile, |name| std::env::var(name).ok())
            .map(|(_, o)| o)
    }

    fn key_and_origin(
        &self,
        profile: &AgentProfile,
        env: impl Fn(&str) -> Option<String>,
    ) -> Option<(String, KeySource)> {
        let s = self.lock();
        if let Some(k) = s.stored.get(&profile.id) {
            return Some((k.0.clone(), KeySource::Stored));
        }
        if let Some(k) = s.session.get(&profile.id) {
            return Some((k.0.clone(), KeySource::Session));
        }
        drop(s);
        let name = profile.preset().key_env?;
        let k = env(name)?.trim().to_owned();
        (!k.is_empty()).then_some((k, KeySource::Environment))
    }

    /// The file that did not read and was set aside, once.
    pub fn take_set_aside(&self) -> Option<PathBuf> {
        self.lock().set_aside.take()
    }

    fn write(&self, s: &State) -> Result<(), String> {
        match &self.path {
            Some(path) => file::write(path, s),
            None => Ok(()),
        }
    }
}

/// The agent with its text fields trimmed and an empty address dropped.
fn tidy(mut p: AgentProfile) -> AgentProfile {
    p.name = p.name.trim().to_owned();
    p.model = p.model.trim().to_owned();
    p.base_url = p
        .base_url
        .map(|u| u.trim().trim_end_matches('/').to_owned())
        .filter(|u| !u.is_empty());
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_key_comes_from_the_file_then_the_session_then_the_environment() {
        let s = ProfileStore::in_memory();
        let p = s
            .add(AgentProfile {
                key: KeySource::Environment,
                ..AgentProfile::new(ProviderKind::Anthropic, "Claude", "claude-opus-5-5")
            })
            .unwrap();
        let env = |name: &str| (name == "ANTHROPIC_API_KEY").then(|| " ortamdaki ".to_owned());
        assert_eq!(
            s.key_and_origin(&p, env),
            Some(("ortamdaki".into(), KeySource::Environment))
        );
        s.remember_session_key(&p.id, "oturumdaki");
        assert_eq!(
            s.key_and_origin(&p, env),
            Some(("oturumdaki".into(), KeySource::Session))
        );
        s.update(AgentProfile {
            key: KeySource::Stored,
            ..p.clone()
        })
        .unwrap();
        s.set_key(&p.id, "saklanan").unwrap();
        assert_eq!(
            s.key_and_origin(&p, env),
            Some(("saklanan".into(), KeySource::Stored))
        );
        // No variable for a local server.
        let local = s
            .add(AgentProfile::new(ProviderKind::Ollama, "Yerel", "llama"))
            .unwrap();
        assert_eq!(s.key_and_origin(&local, |_| Some("x".into())), None);
        s.forget_key(&p.id).unwrap();
        assert_eq!(s.key_and_origin(&p, |_| None), None);
    }
}
