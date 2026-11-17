//! Uygulama ayarları › Yapay zekâ (docs/adr/8056 §4): the user's own AI agents. Each is a provider, a model, an
//! address and an API key; one is the default, the one Asistan talks to.
//!
//! The agents are kept in a file of their own, readable by the user alone (`yapay-zeka-ajanlari.json`, 0600,
//! `kentos_agent::profiles`), not in the settings: the settings file is exported and shared (Ayar dosyası) and a key
//! must not travel with it. A key is kept in that file only when “Bu bilgisayarda sakla” is chosen; “Yalnız bu
//! oturum” keeps it in memory until the program closes, “Ortam değişkeni” reads the provider's own variable
//! (`DEEPSEEK_API_KEY` …). The section lists the agents and writes a change to them at once; an agent is added or
//! changed in a window of its own over the settings window, with its own Vazgeç and Kaydet (the settings window's
//! Kaydet is the settings'). Two rows of Vazgeç and Kaydet in one window, the form's and the window's, asked which one
//! kept the agent (13 November 2026). An agent's token limits, per conversation and per day, are its form's too
//! (docs/adr/8056 §6).

use iced::widget::{Column, column, row};
use iced::{Center, Element, Fill, Task};
use kentos_agent::profiles::{AgentProfile, KeySource, ProfileField, PythonRuns};
use kentos_agent::providers::ProviderKind;
use kentos_agent::{read_tokens, spending_words, tokens_words};
use kentos_ui::widget::{Banner, Choice, Dialog, Form, InputField, Select, Suggest, Suggestion};
use kentos_ui::{label, style};

use crate::app::{App, Message};
use crate::exchange::words::{self, Kind as Line};
use crate::settings_view::Edit;

/// What the section's controls ask.
#[derive(Debug, Clone)]
pub enum AgentsEdit {
    /// Ajan ekle: a new agent's form.
    Add,
    /// Düzenle: an agent's form, by its id.
    Change(String),
    Remove(String),
    MakeDefault(String),
    Kind(usize),
    Name(String),
    Model(String),
    BaseUrl(String),
    China,
    Key(String),
    Keep(usize),
    Default,
    /// Öznitelik değerleri gönderilmesin (docs/adr/8057 §103).
    Withhold,
    /// Kod çalıştırma › Atölyedeki kodu: its place in [`PythonRuns::ALL`] (docs/adr/8057 §104).
    Python(usize),
    /// The token limits, as typed (docs/adr/8056 §6).
    ConversationTokens(String),
    DailyTokens(String),
    /// Modelleri getir: the provider's list of models, with the key given.
    FetchModels,
    Models(Result<Vec<String>, String>),
    /// Bağlantıyı dene: the cheapest call that proves the key works.
    Check,
    Checked(Result<String, String>),
    Save,
    Cancel,
}

fn msg(edit: AgentsEdit) -> Message {
    Message::Settings(Edit::Agents(edit))
}

/// Where a key may be kept for a provider, as the form offers it: its environment variable only where it has one.
fn keeps(kind: ProviderKind) -> Vec<KeySource> {
    let mut out = vec![KeySource::Stored, KeySource::Session];
    if kind.preset().key_env.is_some() {
        out.push(KeySource::Environment);
    }
    out
}

/// Whether the form asks for a key: a provider that needs one, or a server elsewhere that may (a local one needs none).
fn asks_key(kind: ProviderKind) -> bool {
    let preset = kind.preset();
    preset.needs_key || !preset.local
}

/// The agent being added or changed.
#[derive(Debug)]
pub struct Editor {
    /// The agent changed; none: a new one.
    id: Option<String>,
    kind: ProviderKind,
    name: String,
    model: String,
    base_url: String,
    china: bool,
    /// What is typed; empty while the agent has a key: that key stays.
    key: String,
    /// The agent has a key now (kept, this session's or the environment's).
    kept: bool,
    keep: KeySource,
    default: bool,
    /// The token limits as typed; empty: none.
    conversation_tokens: String,
    daily_tokens: String,
    /// No attribute value is sent to it (docs/adr/8057 §103).
    withhold: bool,
    /// What it may do with the Python it writes (docs/adr/8057 §104).
    python: PythonRuns,
    /// The provider's models, once fetched.
    models: Vec<String>,
    fetching: bool,
    checking: bool,
    checked: Option<Result<String, String>>,
    problems: Vec<(ProfileField, String)>,
    key_problem: Option<String>,
    save_problem: Option<String>,
}

impl Editor {
    fn new(kind: ProviderKind, default: bool) -> Editor {
        let preset = kind.preset();
        Editor {
            id: None,
            kind,
            name: preset.name.to_owned(),
            model: preset
                .low_cost_model
                .or(preset.flagship_model)
                .unwrap_or_default()
                .to_owned(),
            base_url: String::new(),
            china: false,
            key: String::new(),
            kept: false,
            keep: KeySource::Stored,
            default,
            conversation_tokens: String::new(),
            daily_tokens: String::new(),
            withhold: false,
            python: PythonRuns::Ask,
            models: Vec::new(),
            fetching: false,
            checking: false,
            checked: None,
            problems: Vec::new(),
            key_problem: None,
            save_problem: None,
        }
    }

    fn of(profile: &AgentProfile, kept: bool) -> Editor {
        let preset = profile.kind.preset();
        let base_url = profile.base_url.clone().unwrap_or_default();
        Editor {
            id: Some(profile.id.clone()),
            kind: profile.kind,
            name: profile.name.clone(),
            model: profile.model.clone(),
            china: preset.china_base_url.is_some_and(|c| c == base_url),
            base_url,
            key: String::new(),
            kept,
            keep: profile.key,
            default: profile.default,
            conversation_tokens: profile
                .conversation_tokens
                .map(tokens_words)
                .unwrap_or_default(),
            daily_tokens: profile.daily_tokens.map(tokens_words).unwrap_or_default(),
            withhold: profile.withhold_values,
            python: profile.python_runs,
            models: Vec::new(),
            fetching: false,
            checking: false,
            checked: None,
            problems: Vec::new(),
            key_problem: None,
            save_problem: None,
        }
    }

    /// The agent as the form says it, its id the one it had (a new one's is made when it is added).
    fn profile(&self) -> AgentProfile {
        let mut p = AgentProfile::new(self.kind, self.name.trim(), self.model.trim());
        if let Some(id) = &self.id {
            p.id = id.clone();
        }
        let url = self.base_url.trim();
        p.base_url = (!url.is_empty()).then(|| url.to_owned());
        p.key = self.keep;
        p.default = self.default;
        p.conversation_tokens = read_tokens(&self.conversation_tokens).unwrap_or(None);
        p.daily_tokens = read_tokens(&self.daily_tokens).unwrap_or(None);
        p.withhold_values = self.withhold;
        p.python_runs = self.python;
        p
    }

    /// What the limits' fields say that is not a limit.
    fn limit_problems(&self) -> impl Iterator<Item = (ProfileField, String)> {
        [
            (ProfileField::ConversationTokens, &self.conversation_tokens),
            (ProfileField::DailyTokens, &self.daily_tokens),
        ]
        .into_iter()
        .filter_map(|(field, typed)| read_tokens(typed).err().map(|why| (field, why)))
    }

    fn problem(&self, field: ProfileField) -> Option<String> {
        self.problems
            .iter()
            .find(|(f, _)| *f == field)
            .map(|(_, why)| why.clone())
    }
}

/// The section's state: the form, and what the last action said.
#[derive(Debug, Default)]
pub struct Agents {
    pub(crate) editor: Option<Editor>,
    note: Option<(bool, String)>,
}

/// Where an agent's key comes from, in words.
fn key_words(origin: Option<KeySource>, env: Option<&str>) -> String {
    match origin {
        Some(KeySource::Stored) => "Anahtar bu bilgisayarda".into(),
        Some(KeySource::Session) => "Anahtar yalnız bu oturumda".into(),
        Some(KeySource::Environment) => {
            format!("Anahtar ortam değişkeninden ({})", env.unwrap_or("?"))
        }
        None => "Anahtar yok".into(),
    }
}

impl App {
    /// The section's controls; the chat's header follows what they change.
    pub(crate) fn agents_edit(&mut self, edit: AgentsEdit) -> Task<Message> {
        let task = self.agents_change(edit);
        self.assistant.refresh();
        task
    }

    fn agents_change(&mut self, edit: AgentsEdit) -> Task<Message> {
        let store = &self.assistant.store;
        let agents = &mut self.assistant.agents;
        match edit {
            AgentsEdit::Add => {
                agents.editor = Some(Editor::new(ProviderKind::ALL[0], store.list().is_empty()));
                agents.note = None;
            }
            AgentsEdit::Change(id) => {
                if let Some(p) = store.get(&id) {
                    let kept = store.key_origin(&p).is_some();
                    agents.editor = Some(Editor::of(&p, kept));
                    agents.note = None;
                }
            }
            AgentsEdit::Remove(id) => {
                let name = store.get(&id).map(|p| p.name).unwrap_or_default();
                agents.note = Some(match store.remove(&id).and_then(|_| store.save()) {
                    Ok(()) => (false, format!("“{name}” silindi.")),
                    Err(why) => (true, why),
                });
            }
            AgentsEdit::MakeDefault(id) => {
                let name = store.get(&id).map(|p| p.name).unwrap_or_default();
                agents.note = Some(match store.set_default(&id).and_then(|()| store.save()) {
                    Ok(()) => (false, format!("Asistan bundan sonra “{name}” ile konuşur.")),
                    Err(why) => (true, why),
                });
            }
            AgentsEdit::Save => return self.agents_save(),
            AgentsEdit::Cancel => agents.editor = None,
            edit => {
                let Some(e) = agents.editor.as_mut() else {
                    return Task::none();
                };
                match edit {
                    AgentsEdit::Kind(i) => {
                        let Some(&kind) = ProviderKind::ALL.get(i) else {
                            return Task::none();
                        };
                        let before = e.kind.preset();
                        let after = kind.preset();
                        // A name or model the form gave follows the provider; one the user typed stays.
                        if e.name.trim().is_empty() || e.name == before.name {
                            e.name = after.name.to_owned();
                        }
                        if e.model.trim().is_empty() || before.models.contains(&e.model.as_str()) {
                            e.model = after
                                .low_cost_model
                                .or(after.flagship_model)
                                .unwrap_or_default()
                                .to_owned();
                        }
                        e.kind = kind;
                        e.base_url.clear();
                        e.china = false;
                        e.models.clear();
                        e.checked = None;
                        if !keeps(kind).contains(&e.keep) {
                            e.keep = KeySource::Stored;
                        }
                        e.key_problem = None;
                    }
                    AgentsEdit::Name(t) => e.name = t,
                    AgentsEdit::ConversationTokens(t) => e.conversation_tokens = t,
                    AgentsEdit::DailyTokens(t) => e.daily_tokens = t,
                    AgentsEdit::Model(t) => e.model = t,
                    AgentsEdit::BaseUrl(t) => {
                        e.base_url = t;
                        e.china = false;
                    }
                    AgentsEdit::China => {
                        e.china = !e.china;
                        e.base_url = if e.china {
                            e.kind
                                .preset()
                                .china_base_url
                                .unwrap_or_default()
                                .to_owned()
                        } else {
                            String::new()
                        };
                    }
                    AgentsEdit::Key(t) => {
                        e.key = t;
                        e.key_problem = None;
                    }
                    AgentsEdit::Keep(i) => {
                        if let Some(&keep) = keeps(e.kind).get(i) {
                            e.keep = keep;
                        }
                    }
                    AgentsEdit::Default => e.default = !e.default,
                    AgentsEdit::Withhold => e.withhold = !e.withhold,
                    AgentsEdit::Python(i) => {
                        if let Some(&runs) = PythonRuns::ALL.get(i) {
                            e.python = runs;
                        }
                    }
                    AgentsEdit::FetchModels | AgentsEdit::Check => {
                        let profile = e.profile();
                        let typed = e.key.trim().to_owned();
                        let key = if typed.is_empty() {
                            store.key_for(&profile)
                        } else {
                            Some(typed)
                        };
                        if matches!(edit, AgentsEdit::FetchModels) {
                            e.fetching = true;
                            return Task::perform(kentos_agent::list_models(&profile, key), |r| {
                                msg(AgentsEdit::Models(r.map_err(|f| f.to_string())))
                            });
                        }
                        e.checking = true;
                        e.checked = None;
                        return Task::perform(kentos_agent::check(&profile, key), |r| {
                            msg(AgentsEdit::Checked(r.map_err(|f| f.to_string())))
                        });
                    }
                    AgentsEdit::Models(r) => {
                        e.fetching = false;
                        match r {
                            Ok(models) => e.models = models,
                            Err(why) => e.checked = Some(Err(why)),
                        }
                    }
                    AgentsEdit::Checked(r) => {
                        e.checking = false;
                        e.checked = Some(r);
                    }
                    _ => {}
                }
            }
        }
        Task::none()
    }

    /// Kaydet: the agent written to its file, its key where the form says; the default moved when it is asked.
    fn agents_save(&mut self) -> Task<Message> {
        let store = &self.assistant.store;
        let agents = &mut self.assistant.agents;
        let Some(e) = agents.editor.as_mut() else {
            return Task::none();
        };
        let profile = e.profile();
        let limits: Vec<_> = e.limit_problems().collect();
        e.problems = profile.problems();
        e.problems.extend(limits);
        e.key_problem = (e.kind.preset().needs_key
            && e.keep != KeySource::Environment
            && !e.kept
            && e.key.trim().is_empty())
        .then(|| {
            "Bu sağlayıcı bir API anahtarı ister: anahtarı yazın ya da ortam değişkenini seçin."
                .to_owned()
        });
        if !e.problems.is_empty() || e.key_problem.is_some() {
            return Task::none();
        }
        let saved = if e.id.is_some() {
            store.update(profile.clone()).map(|()| profile)
        } else {
            store.add(profile)
        };
        let result = saved.and_then(|p| {
            let key = e.key.trim();
            match p.key {
                KeySource::Stored if !key.is_empty() => store.set_key(&p.id, key)?,
                KeySource::Session if !key.is_empty() => store.remember_session_key(&p.id, key),
                KeySource::Environment => store.forget_key(&p.id)?,
                _ => {}
            }
            if p.default {
                store.set_default(&p.id)?;
            }
            store.save()?;
            Ok(p)
        });
        match result {
            Ok(p) => {
                agents.editor = None;
                agents.note = Some((false, format!("“{}” kaydedildi.", p.name)));
            }
            Err(why) => e.save_problem = Some(why),
        }
        Task::none()
    }

    /// The section: the agents, their default, and where they are kept. An agent is added or changed in a window of
    /// its own over this one ([`App::agent_window`]): the settings window's Kaydet is the settings', the agent's is
    /// the agent's.
    pub(crate) fn ai_section<'a>(&'a self) -> Element<'a, Message> {
        let agents = &self.assistant.agents;
        let store = &self.assistant.store;
        let list = store.list();
        let day = super::today();
        let mut rows = Column::new().spacing(8);
        if list.is_empty() {
            rows = rows.push(words::empty(
                "Henüz ajan yok. “Ajan ekle” ile kendi API anahtarınızla bir yapay zekâ sağlayıcısı tanımlayın; \
                 Asistan paneli onunla konuşur.",
            ));
        }
        for p in &list {
            let preset = p.preset();
            let facts = format!(
                "{} · {} · {}",
                preset.name,
                p.model,
                key_words(store.key_origin(p), preset.key_env)
            );
            let badge: Element<'a, Message> = if p.default {
                label::caption("Varsayılan")
                    .style(style::text::accent)
                    .into()
            } else {
                words::secondary(
                    "Varsayılan yap",
                    Some(msg(AgentsEdit::MakeDefault(p.id.clone()))),
                )
            };
            rows = rows.push(
                row![
                    {
                        let mut said = column![
                            label::strong(p.name.clone()),
                            label::caption(facts).style(style::text::muted),
                            label::caption(spending_words(
                                p,
                                self.assistant.ledger.spent(&p.id, &day)
                            ))
                            .style(style::text::muted)
                        ];
                        // The organisation's policy, where the user picks the agent: not first in the chat.
                        if let Some(why) = self.assistant.refusal(p) {
                            said = said.push(label::caption(why).style(style::text::warning));
                        }
                        said.spacing(2).width(Fill)
                    },
                    badge,
                    words::secondary("Düzenle", Some(msg(AgentsEdit::Change(p.id.clone())))),
                    words::secondary("Sil", Some(msg(AgentsEdit::Remove(p.id.clone())))),
                ]
                .spacing(8)
                .align_y(Center),
            );
        }
        let mut form = Form::new()
            .label_width(170.0)
            .section("Ajanlar")
            .wide(rows)
            .wide(words::primary("Ajan ekle", Some(msg(AgentsEdit::Add))));
        if let Some((warn, note)) = &agents.note {
            form = form.wide(if *warn {
                Banner::warning(note.clone())
            } else {
                Banner::info(note.clone())
            });
        }
        let kept = store
            .path()
            .map(|p| {
                format!(
                    "Ajanlar ve saklanan anahtarlar yalnız sizin okuyabildiğiniz bir dosyada: {}",
                    p.display()
                )
            })
            .unwrap_or_else(|| "Ajanlar bu oturumun belleğinde.".into());
        let ledger = self.assistant.ledger.path().map_or_else(
            || "Harcanan tokenlar bu oturumun belleğinde sayılır.".to_owned(),
            |p| {
                format!(
                    "Harcanan tokenlar günlere göre, yalnız sayı olarak (istem, yanıt ya da anahtar değil): {}",
                    p.display()
                )
            },
        );
        form.section("Kayıt")
            .wide(label::caption(kept))
            .wide(label::caption(ledger))
            .into()
    }

    /// The agent being added or changed, in a window over Uygulama ayarları with its own Vazgeç and Kaydet.
    pub(crate) fn agent_window(&self) -> Option<Element<'_, Message>> {
        if self.dialog != Some(crate::app::Dialog::Settings) {
            return None;
        }
        let e = self.assistant.agents.editor.as_ref()?;
        let check = words::secondary(
            if e.checking {
                "Deneniyor…"
            } else {
                "Bağlantıyı dene"
            },
            (!e.checking).then(|| msg(AgentsEdit::Check)),
        );
        let mut window = Dialog::new(if e.id.is_some() {
            "Yapay zekâ ajanı"
        } else {
            "Yeni yapay zekâ ajanı"
        })
        .key("yapay-zeka-ajani")
        .scroll(self.agent_form(e))
        .aside(check)
        .action(words::secondary("Vazgeç", Some(msg(AgentsEdit::Cancel))))
        .action(words::primary("Kaydet", Some(msg(AgentsEdit::Save))))
        .width(640.0)
        .max_height(720.0);
        let status = match (&e.save_problem, &e.checked) {
            (Some(why), _) | (None, Some(Err(why))) => {
                Some(words::text_line(Line::Error, why.clone()))
            }
            (None, Some(Ok(said))) => Some(words::text_line(Line::Ok, said.clone())),
            (None, None) => None,
        };
        if let Some(status) = status {
            window = window.status(status);
        }
        Some(window.modal(msg(AgentsEdit::Cancel)))
    }

    fn agent_form<'a>(&'a self, e: &'a Editor) -> Element<'a, Message> {
        let preset = e.kind.preset();
        let input = |placeholder: &str,
                     value: &'a str,
                     on: fn(String) -> AgentsEdit,
                     field: Option<ProfileField>| {
            InputField::new(placeholder, value)
                .on_input(move |t| msg(on(t)))
                .on_submit(msg(AgentsEdit::Save))
                .invalid(field.is_some_and(|f| e.problem(f).is_some()))
        };
        // A provider's name only: what it does with what it is sent is said under the field and in Gizlilik.
        let kinds = Select::new(
            ProviderKind::ALL
                .iter()
                .map(|k| Choice::new(k.preset().name)),
            ProviderKind::ALL.iter().position(|k| *k == e.kind),
            |i| msg(AgentsEdit::Kind(i)),
        );
        // The model: typed, or picked from the provider's suggestions and its list (a field with suggestions,
        // DESIGN.md §7.13).
        let choices = model_choices(e);
        let names: Vec<String> = choices.iter().map(|(n, _)| n.clone()).collect();
        let mut model = Suggest::new(&e.model, preset.low_cost_model.unwrap_or("Modelin kimliği"))
            .on_input(|t| msg(AgentsEdit::Model(t)))
            .suggestions(choices.into_iter().map(|(name, note)| {
                let s = Suggestion::new(name);
                match note {
                    Some(note) => s.note(note),
                    None => s,
                }
            }))
            .on_pick(move |i| msg(AgentsEdit::Model(names.get(i).cloned().unwrap_or_default())))
            .on_submit(msg(AgentsEdit::Save));
        if e.fetching {
            model = model.note("Sağlayıcının listesi getiriliyor…");
        }
        let fetch = words::secondary(
            "Listeyi getir",
            (!e.fetching && preset.lists_models).then(|| msg(AgentsEdit::FetchModels)),
        );
        let model_help = if e.models.is_empty() {
            "Önerilenlerden seçin ya da modelin kimliğini yazın.".to_owned()
        } else {
            format!(
                "Sağlayıcı {} model listeledi; alana tıklayınca hepsi açılır.",
                e.models.len()
            )
        };
        let mut form = Form::new()
            .label_width(150.0)
            .section("Sağlayıcı")
            .field("Sağlayıcı", kinds);
        if let Some(note) = preset.note {
            form = form.help(note);
        }
        form = form
            .field(
                "Ad",
                input(
                    "Örneğin İş ajanım",
                    &e.name,
                    AgentsEdit::Name,
                    Some(ProfileField::Name),
                ),
            )
            .required()
            .error(e.problem(ProfileField::Name))
            .field(
                "Model",
                row![model, fetch].spacing(8).align_y(Center),
            )
            .required()
            .help(model_help)
            .error(e.problem(ProfileField::Model))
            .field(
                "Adres",
                input(
                    preset.base_url.unwrap_or("https://…/v1"),
                    &e.base_url,
                    AgentsEdit::BaseUrl,
                    Some(ProfileField::BaseUrl),
                ),
            )
            .help(
                "Boş bırakılırsa sağlayıcının kendi adresi; kendi sunucunuz ya da bir ara katman için değiştirin.",
            )
            .error(e.problem(ProfileField::BaseUrl));
        if preset.china_base_url.is_some() {
            form = form.row(words::check(
                e.china,
                "Çin uç noktasını kullan",
                Some(msg(AgentsEdit::China)),
            ));
        }
        let env = preset.key_env.unwrap_or("—");
        if asks_key(e.kind) {
            let placeholder = if e.kept {
                "Kayıtlı anahtar değişmez"
            } else {
                ""
            };
            let places = keeps(e.kind);
            let keep = Select::new(
                places.iter().map(|k| match k {
                    KeySource::Stored => Choice::new("Bu bilgisayarda sakla"),
                    KeySource::Session => Choice::new("Yalnız bu oturum"),
                    KeySource::Environment => Choice::new(format!("Ortam değişkeni ({env})")),
                }),
                places.iter().position(|k| *k == e.keep),
                |i| msg(AgentsEdit::Keep(i)),
            )
            .searchable(false);
            form = form
                .section("Kimlik")
                .field(
                    "API anahtarı",
                    InputField::new(placeholder, &e.key)
                        .secure(true)
                        .on_input(|t| msg(AgentsEdit::Key(t)))
                        .on_submit(msg(AgentsEdit::Save))
                        .invalid(e.key_problem.is_some()),
                )
                .error(e.key_problem.clone())
                .field("Anahtarın yeri", keep)
                .help(match e.keep {
                    KeySource::Stored => "Yalnız sizin okuyabildiğiniz bir dosyada (yapay-zeka-ajanlari.json) \
                                          saklanır; ayarlarla dışa aktarılmaz, çizime ve günlüklere yazılmaz."
                        .to_owned(),
                    KeySource::Session => {
                        "Saklanmaz: bu oturumda yazdığınız kullanılır, program kapanınca unutulur.".to_owned()
                    }
                    KeySource::Environment => {
                        format!("{env} ortam değişkeninden okunur; burada saklanmaz.")
                    }
                });
        }
        // The limits stop an agent calling tools in a loop: nobody types while it spends (docs/adr/8056 §6).
        let today =
            e.id.as_deref()
                .map_or(0, |id| self.assistant.ledger.spent(id, &super::today()));
        form = form
            .section("Token sınırı")
            .field(
                "Sohbet başına",
                input(
                    "Sınır yok",
                    &e.conversation_tokens,
                    AgentsEdit::ConversationTokens,
                    Some(ProfileField::ConversationTokens),
                ),
            )
            .help("Bir sohbetin bütün turlarında gönderilen ve alınan token; sınıra varınca ajan durur, yeni sohbet sıfırdan sayar.")
            .error(e.problem(ProfileField::ConversationTokens))
            .field(
                "Günlük",
                input(
                    "Sınır yok",
                    &e.daily_tokens,
                    AgentsEdit::DailyTokens,
                    Some(ProfileField::DailyTokens),
                ),
            )
            .help(format!(
                "Bu ajanın bu bilgisayarda bir günde harcayacağı en çok token; bugün {} token harcadı.",
                tokens_words(today)
            ))
            .error(e.problem(ProfileField::DailyTokens));
        // What goes to the provider (docs/adr/8057 §103, PY-AI-13): the organisation's policy may decide it for an
        // agent outside this computer; the box then says so and stays ticked.
        let local = e.profile().on_this_computer();
        let forced = self
            .assistant
            .policy
            .as_ref()
            .is_some_and(|p| p.withholds_attribute_values(local));
        form = form
            .section("Gönderilen veri")
            .field(
                "Öznitelik değerleri",
                words::check(
                    e.withhold || forced,
                    "Gönderilmesin",
                    (!forced).then(|| msg(AgentsEdit::Withhold)),
                ),
            )
            .help(
                "İşaretliyken ajan çizimin öznitelik değerlerini (malik adı, kimlik numarası, adres gibi) görmez, \
                 onlara göre seçemez, süzemez ve hesap yapamaz; değer yazabilir. Geometri, katman ve alan adları, \
                 yazılar gider. Bu bilgisayarda çalışan bir sunucuda veri bu bilgisayardan çıkmaz.",
            )
            .warning(forced.then_some(
                "Kurum ilkesi öznitelik değerlerinin bu bilgisayarın dışındaki yapay zekâ hizmetlerine gitmesine izin \
                 vermiyor: bu ajan onları göremez.",
            ));
        // Its own Python, a class apart from its tool calls (docs/adr/8057 §104, PY-AI-11): the organisation's policy
        // may close it for every agent; the choice then says so.
        let closed = self
            .assistant
            .policy
            .as_ref()
            .and_then(kentos_extension_host::Policy::agent_python_refusal);
        let runs = if closed.is_some() {
            PythonRuns::Off
        } else {
            e.python
        };
        let choices = Select::new(
            PythonRuns::ALL.iter().map(|r| {
                let choice = Choice::new(match r {
                    PythonRuns::Ask => "Her seferinde sorsun",
                    PythonRuns::Allow => "Sormadan çalıştırsın",
                    PythonRuns::Off => "Çalıştırmasın",
                });
                // Closed by the policy: the other two shown, not chosen.
                if closed.is_some() && *r != PythonRuns::Off {
                    choice.disabled()
                } else {
                    choice
                }
            }),
            PythonRuns::ALL.iter().position(|r| *r == runs),
            |i| msg(AgentsEdit::Python(i)),
        )
        .searchable(false);
        form = form
            .section("Kod çalıştırma")
            .field("Atölyedeki kodu", choices)
            .help(
                "Ajanın yazdığı eklenti, denenirken ya da kurulu eklentinin üstüne yüklenirken bu bilgisayarda sizin \
                 yetkinizle çalışır; ayrı süreç bir kum havuzu değildir. Çizimi okuması ve komutlarla yazması bu \
                 izinden ayrıdır.",
            )
            .warning(closed);
        form.section("Kullanım")
            .row(words::check(
                e.default,
                "Varsayılan ajan: Asistan bununla konuşur",
                Some(msg(AgentsEdit::Default)),
            ))
            .row(Banner::info(format!("Gizlilik: {}", preset.privacy)))
            .into()
    }
}

/// The model suggestions of the form: the provider's suggested models, then those its list gave, each once, with a
/// word on the flagship and the low-cost one. Every one while the field holds one of them or nothing, else those whose
/// id holds what is typed.
pub(super) fn model_choices(e: &Editor) -> Vec<(String, Option<&'static str>)> {
    let preset = e.kind.preset();
    let mut all: Vec<(String, Option<&'static str>)> = preset
        .models
        .iter()
        .map(|m| {
            let note = if Some(*m) == preset.flagship_model {
                Some("en güçlü")
            } else if Some(*m) == preset.low_cost_model {
                Some("düşük maliyetli")
            } else {
                None
            };
            ((*m).to_owned(), note)
        })
        .collect();
    for m in &e.models {
        if !all.iter().any(|(n, _)| n == m) {
            all.push((m.clone(), None));
        }
    }
    let typed = e.model.trim().to_lowercase();
    if typed.is_empty() || all.iter().any(|(n, _)| n.to_lowercase() == typed) {
        return all;
    }
    all.into_iter()
        .filter(|(n, _)| n.to_lowercase().contains(&typed))
        .collect()
}
