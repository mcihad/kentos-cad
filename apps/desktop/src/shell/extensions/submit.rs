//! Kataloğa gönder (plan/eklenti.md ADIM 21 › Akış: "istenirse yayına gönderme"; docs/adr/8057 §107): an extension the
//! user develops (installed from a working folder) packed from its folder, uploaded to a catalog as a draft and sent to
//! its review, as a program may (docs/adr/8059 §10). Approving and publishing are a person's, in the catalog's window
//! on the web: the desktop sends, it decides nothing.
//!
//! - **Where to:** the account's personal catalog, the catalogs of its organisations, the public one; the server says
//!   whether the account may publish there.
//! - **What goes:** the folder as `kentos kek pack` packs it, the gates' findings and the server's answer said plainly;
//!   the extension's log keeps what was sent where.
//! - **Asked by an agent:** an agent opens this window with the extension chosen (`extension.workshop.submit`), the
//!   user sends: publishing is never the agent's silent step (PY-AI-05).

use std::path::PathBuf;

use iced::Task;
use iced::widget::column;
use kentos_commands::{Me, TenantKind};
use kentos_extension_host::SourceKind;
use kentos_extensions::Limits;
use kentos_ui::label;
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Banner, Form, InputField, Toast};
use serde_json::Value;

use super::{Event, msg};
use crate::app::{App, Message};
use crate::exchange::words;

/// The window while it is open.
#[derive(Clone, Debug, Default)]
pub(crate) struct Submit {
    pub(crate) ext: String,
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) folder: PathBuf,
    /// Its place in [`App::submit_catalogs`].
    pub(crate) choice: usize,
    pub(crate) note: String,
    pub(crate) busy: bool,
    /// Why the last sending did not go.
    pub(crate) failed: Option<String>,
    /// An agent asked for it: the window says so.
    pub(crate) asked_by: Option<String>,
}

/// The window's controls.
#[derive(Clone, Debug)]
pub(crate) enum SubmitEvent {
    Open(String),
    Catalog(usize),
    Note(String),
    Send,
    Sent(Result<Value, String>),
    Cancel,
}

/// The catalogs `me` may send to, in the window's words: the personal one, the organisations' and the public one.
pub(crate) fn catalogs(me: &Me) -> Vec<(String, String)> {
    let mut out = vec![(
        format!("personal:{}", me.user.id),
        "Kişisel kataloğum".to_owned(),
    )];
    for m in &me.memberships {
        if m.active && m.tenant_kind == TenantKind::Organization {
            out.push((
                format!("tenant:{}", m.tenant_slug),
                format!("{} kurum kataloğu", m.tenant_name),
            ));
        }
    }
    out.push(("public".to_owned(), "Genel katalog".to_owned()));
    out
}

/// What the server's view of a release says of where it stands: its state's word.
pub(crate) fn state_words(view: &Value) -> &'static str {
    let state = view
        .pointer("/release/state")
        .or_else(|| view.get("state"))
        .and_then(Value::as_str)
        .unwrap_or("");
    match state {
        "draft" => "taslak",
        "submitted" => "incelemeye gönderildi; kapılar çalışıyor",
        "inReview" => "incelemede",
        "changesRequested" => "değişiklik istendi",
        "approved" => "onaylandı",
        "published" => "yayımlandı",
        _ => "gönderildi",
    }
}

impl App {
    /// The catalogs the signed-in account may send to; none without a session.
    pub(crate) fn submit_catalogs(&self) -> Vec<(String, String)> {
        self.cloud.me.as_ref().map(catalogs).unwrap_or_default()
    }

    pub(crate) fn ext_submit_event(&mut self, event: SubmitEvent) -> Task<Message> {
        match event {
            SubmitEvent::Open(id) => {
                if let Err(why) = self.ext_submit_open(&id, None) {
                    self.extensions_say(true, why);
                }
            }
            SubmitEvent::Catalog(i) => {
                if let Some(s) = &mut self.extensions.submit {
                    s.choice = i;
                }
            }
            SubmitEvent::Note(text) => {
                if let Some(s) = &mut self.extensions.submit {
                    s.note = text;
                }
            }
            SubmitEvent::Cancel => self.extensions.submit = None,
            SubmitEvent::Send => return self.ext_submit_send(),
            SubmitEvent::Sent(ended) => self.ext_submit_sent(ended),
        }
        Task::none()
    }

    /// The window opened on `id`, an extension installed from a working folder; `asked_by` an agent's name when an
    /// agent asked. Why not, in words.
    pub(crate) fn ext_submit_open(
        &mut self,
        id: &str,
        asked_by: Option<&str>,
    ) -> Result<(), String> {
        let Some(registry) = self.extensions.registry.as_ref() else {
            return Err(self.extensions.missing.clone().unwrap_or_default());
        };
        let Some(view) = registry.extensions().into_iter().find(|v| v.id == id) else {
            return Err(format!("“{id}” kurulu değil."));
        };
        if view.source.kind != SourceKind::DeveloperFolder {
            return Err(format!(
                "{} bir çalışma klasöründen kurulmadı ({}): kataloğa geliştirilen klasöründen gönderilir.",
                view.name,
                view.source.kind.words()
            ));
        }
        self.extensions.submit = Some(Submit {
            ext: view.id.clone(),
            name: view.name.clone(),
            version: view.version.clone(),
            folder: PathBuf::from(&view.source.location),
            asked_by: asked_by.map(str::to_owned),
            ..Submit::default()
        });
        Ok(())
    }

    /// An agent's `extension.workshop.submit` (docs/adr/8057 §107): the window opened for the user, over Eklentiler.
    pub(crate) fn agent_submit(&mut self, id: &str, who: &str) -> Result<Value, String> {
        self.ext_submit_open(id, Some(who))?;
        self.open_extensions();
        Ok(serde_json::json!({
            "opened": true,
            "where": "Eklentiler › Kataloğa gönder",
            "note": "Kataloğu kullanıcı seçer ve gönderir; onay ve yayın kataloğun inceleyicisinindir."
        }))
    }

    /// Gönder: the folder packed, uploaded as a draft and sent to review, in the background.
    fn ext_submit_send(&mut self) -> Task<Message> {
        let catalogs = self.submit_catalogs();
        let Some(client) = self.cloud.signed_in().cloned() else {
            if let Some(s) = &mut self.extensions.submit {
                s.failed = Some("Göndermek için KentOS hesabınızla oturum açın.".to_owned());
            }
            return Task::none();
        };
        let Some(s) = &mut self.extensions.submit else {
            return Task::none();
        };
        let Some((catalog, _)) = catalogs.get(s.choice).cloned() else {
            return Task::none();
        };
        // Packed as `kentos kek pack` packs it: what does not pass the gates is not sent.
        let packed = kentos_extension_host::store::read_files(&s.folder)
            .map_err(|p| p.message.clone())
            .and_then(|files| {
                kentos_extensions::pack(&files, &Limits::default()).map_err(|problems| {
                    let first = problems
                        .first()
                        .map(|p| p.message.clone())
                        .unwrap_or_default();
                    format!(
                        "Paket kapılardan geçmedi ({} sorun): {first}",
                        problems.len()
                    )
                })
            });
        let bytes = match packed {
            Ok(packed) => packed.bytes,
            Err(why) => {
                s.failed = Some(why);
                return Task::none();
            }
        };
        s.busy = true;
        s.failed = None;
        let (ext, version) = (s.ext.clone(), s.version.clone());
        let note = (!s.note.trim().is_empty()).then(|| s.note.trim().to_owned());
        Task::perform(
            async move {
                client
                    .upload_release(&catalog, bytes)
                    .await
                    .map_err(|e| e.to_string())?;
                client
                    .submit_release(&catalog, &ext, &version, note)
                    .await
                    .map_err(|e| e.to_string())
            },
            |ended| msg(Event::Submit(SubmitEvent::Sent(ended))),
        )
    }

    /// What sending did: on success the window closes, a notice says where it stands and the log keeps it; else why,
    /// in the window.
    fn ext_submit_sent(&mut self, ended: Result<Value, String>) {
        let Some(s) = &mut self.extensions.submit else {
            return;
        };
        s.busy = false;
        let view = match ended {
            Ok(view) => view,
            Err(why) => {
                s.failed = Some(why);
                return;
            }
        };
        let Some(s) = self.extensions.submit.take() else {
            return;
        };
        let catalog = self
            .submit_catalogs()
            .get(s.choice)
            .map_or_else(String::new, |(_, words)| words.clone());
        let state = state_words(&view);
        self.extension_worked(
            &s.ext,
            "katalog",
            "release_submitted",
            &format!(
                "{} {} kataloğa gönderildi ({catalog}): {state}.",
                s.name, s.version
            ),
        );
        self.toast(
            Toast::success(format!("{} {} kataloğa gönderildi", s.name, s.version)).body(format!(
                "{catalog}: {state}. Onaylamak ve yayımlamak kataloğun inceleyicisinin kararıdır; durumu web'deki \
                 katalog sayfasında izleyin."
            )),
        );
    }

    /// The window over Eklentiler while it is open.
    pub(crate) fn extension_submit_window(&self) -> Option<iced::Element<'_, Message>> {
        let s = self.extensions.submit.as_ref()?;
        let catalogs = self.submit_catalogs();
        let ev = |e: SubmitEvent| msg(Event::Submit(e));
        let mut form = Form::new();
        if let Some(agent) = &s.asked_by {
            form = form.wide(Banner::info(format!(
                "{agent} bu eklentiyi kataloğa göndermek istiyor. Göndermek sizin kararınız."
            )));
        }
        let signed_in = self.cloud.signed_in().is_some();
        if signed_in {
            let choose = Select::new(
                catalogs.iter().map(|(_, words)| Choice::new(words.clone())),
                Some(s.choice.min(catalogs.len().saturating_sub(1))),
                move |i| ev(SubmitEvent::Catalog(i)),
            )
            .searchable(false);
            form = form
                .field("Katalog", choose)
                .help("Gönderebileceğiniz kataloğu sunucu söyler; yetkiniz yoksa gönderim nedeniyle reddedilir.")
                .field(
                    "İnceleyiciye not",
                    InputField::new("İsteğe bağlı", &s.note).on_input(move |t| ev(SubmitEvent::Note(t))),
                );
        } else {
            form = form
                .wide(Banner::warning(
                    "Göndermek için KentOS hesabınızla oturum açın.",
                ))
                .row(words::secondary(
                    "Oturum aç…",
                    Some(Message::Run("cloud.signIn")),
                ));
        }
        form = form.wide(label::caption(format!(
            "{} klasöründen yeniden paketlenir, kapılardan geçerse kataloğa taslak olarak yüklenir ve incelemeye \
             gönderilir. Onaylamak ve yayımlamak kataloğun inceleyicisinin kararıdır.",
            s.folder.display()
        )));
        if let Some(why) = &s.failed {
            form = form.wide(Banner::error(why.clone()));
        }
        let can = signed_in && !s.busy && !catalogs.is_empty();
        Some(
            Frame::new(format!("Kataloğa gönder: {} {}", s.name, s.version))
                .key("eklenti-gonder")
                .push(column![form].spacing(8))
                .action(words::secondary("Vazgeç", Some(ev(SubmitEvent::Cancel))))
                .action(words::primary(
                    if s.busy {
                        "Gönderiliyor…"
                    } else {
                        "Gönder"
                    },
                    can.then(|| ev(SubmitEvent::Send)),
                ))
                .width(620.0)
                .modal(ev(SubmitEvent::Cancel)),
        )
    }
}
