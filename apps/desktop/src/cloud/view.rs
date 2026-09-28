//! The cloud's windows (docs/adr/0041), in the web's words: “Buluta giriş”,
//! “Bulut projeleri”, “Buluta yükle”, the conflict windows and an open's
//! progress. The status bar's cells are cells.rs's.

use iced::widget::{
    Column, button, column, container, row, scrollable, text, text_editor, text_input,
};
use iced::{Element, Fill, Length};

use kentos_contracts::ProjectStorage;
use kentos_ui::theme::typography;
use kentos_ui::widget::{Banner, Choice, Dialog, Form, RadioGroup, Select, overlay, progress};
use kentos_ui::{label, style};

use crate::app::{App, Message};
use crate::cloud::forms_plan::fields;
use crate::cloud::plan::{PROJECT_TYPES, TYPE_HINT, type_label};
use crate::cloud::revisions::AnswerKind;
use crate::cloud::{Event, words};

fn cloud(event: Event) -> Message {
    crate::cloud::msg(event)
}

pub(super) fn primary(caption: &'static str, on: Option<Message>) -> Element<'static, Message> {
    button(label::body(caption))
        .on_press_maybe(on)
        .padding([5, 16])
        .style(style::button::primary)
        .into()
}

pub(super) fn secondary(caption: &'static str, on: Option<Message>) -> Element<'static, Message> {
    button(label::body(caption))
        .on_press_maybe(on)
        .padding([5, 16])
        .style(style::button::secondary)
        .into()
}

fn field<'a>(
    placeholder: &str,
    value: &'a str,
    on_input: Option<fn(String) -> Event>,
) -> iced::widget::TextInput<'a, Message> {
    let input = text_input(placeholder, value)
        .padding([5, 8])
        .style(style::field::input);
    match on_input {
        Some(event) => input.on_input(move |t| cloud(event(t))),
        None => input,
    }
}

impl App {
    /// “Buluta giriş”.
    pub(crate) fn sign_in_view(&self) -> Element<'_, Message> {
        let Some(s) = &self.cloud.sign_in else {
            return text("").into();
        };
        let submit = (!s.busy()).then(|| cloud(Event::SignInSubmit));
        let server = field(
            "https://kentos.kurum.gov.tr",
            &s.server,
            (!s.server_fixed).then_some(Event::SignInServer as fn(String) -> Event),
        )
        .on_submit_maybe(submit.clone());
        let login =
            field("Giriş adı", &s.login, Some(Event::SignInLogin)).on_submit_maybe(submit.clone());
        let password = field("Parola", &s.password, Some(Event::SignInPassword))
            .secure(true)
            .on_submit_maybe(submit.clone());
        // A halo round the field that has the keyboard (docs/adr/0127).
        let (server, login, password) = (
            kentos_ui::widget::focus_ring(server),
            kentos_ui::widget::focus_ring(login),
            kentos_ui::widget::focus_ring(password),
        );
        let mut form = Form::new()
            .label_width(90.0)
            .field("Sunucu", server)
            .help(if s.server_fixed {
                "Açık bulut projesinin sunucusu; başka bir sunucu için önce yerel bir çizim açın."
            } else {
                "Bu bilgisayardaki sunucuya http, başka her sunucuya https ile bağlanılır."
            })
            .field("Giriş adı", login)
            .field("Parola", password);
        if let Some(error) = &s.error {
            form = form.row(Banner::error(error.as_str()));
        }
        overlay::blocking(
            Dialog::new("Buluta giriş")
                .push(form)
                .push(label::caption(
                    "Oturum yalnız bu programın belleğinde tutulur; hiçbir dosyaya yazılmaz.",
                ))
                .action(secondary("Vazgeç", Some(cloud(Event::Close))))
                .action(primary(
                    if s.busy() {
                        "Giriş yapılıyor…"
                    } else {
                        "Giriş yap"
                    },
                    submit,
                ))
                .width(440.0),
        )
    }

    /// “Bu cihazdan kaldır” over a copy whose draft still holds unsent work.
    /// The notice of a project the server ended for this account (the web's
    /// AccessLostNotice): what happened, what stays, and a local copy offered.
    pub(crate) fn ended_view(&self) -> Element<'_, Message> {
        let Some((title, message, detail)) = self.ended_notice() else {
            return text("").into();
        };
        overlay::modal(
            kentos_ui::widget::Confirm::new(title, cloud(Event::SaveLocal), cloud(Event::Close))
                .message(message)
                .detail(detail)
                .confirm("Yerel kopya kaydet…")
                .cancel("Tamam"),
            cloud(Event::Close),
        )
    }

    pub(crate) fn remove_copy_view(&self) -> Element<'_, Message> {
        let Some(r) = &self.cloud.removing else {
            return text("").into();
        };
        overlay::modal(
            kentos_ui::widget::Confirm::new(
                format!("Gönderilmemiş {} değişiklik de silinsin mi?", r.unsent),
                cloud(Event::RemoveConfirmed),
                cloud(Event::Close),
            )
            .message(format!(
                "“{}” projesinin bu cihazdaki kopyası kaldırılacak; proje sunucuda olduğu gibi duruyor.",
                r.name
            ))
            .detail("Bu cihazda gönderilmemiş değişiklikler de silinir ve geri gelmez. Saklamak için Vazgeç'e basıp projeyi açın ve Farklı kaydet ile yerel bir dosyaya kaydedin.")
            .confirm("Değişikliklerle birlikte kaldır")
            .destructive(),
            cloud(Event::Close),
        )
    }

    pub(crate) fn cloud_opening_view(&self) -> Option<Element<'_, Message>> {
        let o = self.cloud.opening.as_ref()?;
        if self.dialog == Some(crate::app::Dialog::Catalog)
            || o.started.elapsed() < std::time::Duration::from_millis(250)
        {
            return None;
        }
        let body = column![
            label::body(format!("“{}”", o.name)),
            progress::bar(o.fraction),
            label::caption(o.stage.clone()),
            label::muted(
                "Açık çizim, proje bütünüyle okunup denetlenene kadar olduğu gibi kalır; Vazgeç ona dokunmaz."
            ),
        ]
        .spacing(10);
        Some(overlay::blocking(
            Dialog::new("Bulut projesi açılıyor")
                .push(body)
                .action(secondary("Vazgeç", Some(cloud(Event::OpenCancel))))
                .width(460.0),
        ))
    }

    /// “Buluta yükle” (the web's UploadDialog.ts): the workspace, the name,
    /// the storage mode for good, the catalog's type, description and tags,
    /// what goes up, and how far it is.
    pub(crate) fn upload_view(&self) -> Element<'_, Message> {
        let (Some(u), Some(doc)) = (&self.cloud.upload, &self.document) else {
            return text("").into();
        };
        let working = u.working();
        let mut form = Form::new().label_width(120.0);
        if u.tenants.is_empty() {
            form = form.row(Banner::error(
                "Proje açma yetkiniz olan bir çalışma alanı yok (project.create); kurum yöneticinize başvurun.",
            ));
        } else {
            let group = u.tenants.iter().enumerate().fold(
                RadioGroup::new(u.tenant, |i| cloud(Event::UploadTenant(i))),
                |g, (i, m)| {
                    g.option(
                        i,
                        words::workspace(m.tenant_kind, &m.tenant_name, true),
                        String::new(),
                    )
                },
            );
            form = form.field("Çalışma alanı", group);
        }
        let name = text_input("Proje adı", &u.name)
            .on_input_maybe((!working).then_some(|t| cloud(Event::UploadName(t))))
            .on_submit(cloud(Event::UploadSubmit))
            .size(typography::body())
            .padding([5, 8])
            .style(style::field::input);
        let name = kentos_ui::widget::focus_ring(name);
        // The storage mode, for good (docs/adr/0031): each option says what it means.
        let storage = [ProjectStorage::Database, ProjectStorage::File]
            .into_iter()
            .fold(
                RadioGroup::new(u.storage, |s| cloud(Event::UploadStorage(s))),
                |g, s| g.option(s, words::storage_title(s), words::storage_detail(s)),
            );
        let kind = Select::new(
            PROJECT_TYPES.map(|t| Choice::new(type_label(t))),
            PROJECT_TYPES.iter().position(|t| *t == u.project_type),
            |i| cloud(Event::UploadType(PROJECT_TYPES[i])),
        )
        .searchable(false);
        let description = text_editor(&u.description)
            .placeholder(fields::DESCRIPTION_PLACEHOLDER)
            .on_action(|a| cloud(Event::UploadDescription(a)))
            .height(Length::Fixed(typography::from_default(66.0)))
            .size(typography::body())
            .padding([5, 8])
            .style(style::field::text_area);
        let tags = text_input(fields::TAGS_PLACEHOLDER, &u.tags)
            .on_input_maybe((!working).then_some(|t| cloud(Event::UploadTags(t))))
            .on_submit(cloud(Event::UploadSubmit))
            .size(typography::body())
            .padding([5, 8])
            .style(style::field::input);
        let tags = kentos_ui::widget::focus_ring(tags);
        let count = doc.entity_count();
        let hint = match u.storage {
            ProjectStorage::File => format!(
                "{count} nesne, katman ağacı, proje ayarları ve proje stilleri tek bir .kcad dosyası olarak yüklenir ve projenin 1. revizyonu olur. Proje sizin olur; başkaları paylaşımla eklenir. Sonra Kaydet (Ctrl+S) yeni bir revizyon yazar; kendiliğinden kaydedilmez."
            ),
            ProjectStorage::Database => format!(
                "{count} nesne, katman ağacı, proje ayarları ve proje stilleri yüklenir ve veritabanına tek işlemde aktarılır. Proje sizin olur; başkaları paylaşımla eklenir. Sonra her değişiklik kendiliğinden kaydedilir."
            ),
        };
        form = form
            .field("Proje adı", name)
            .row(label::body("Saklama biçimi (sonradan değişmez)").font(typography::ui_strong()))
            .row(storage)
            .field(fields::TYPE, kind)
            .row(label::caption(TYPE_HINT).style(style::text::muted))
            .field(fields::DESCRIPTION, description)
            .field(fields::TAGS, tags)
            .row(label::caption(hint));
        // How far it is and what went wrong stay in view under the scrolling fields.
        let mut status = Column::new().spacing(8);
        if let Some(stage) = &u.stage {
            status = status
                .push(column![progress::bar(u.fraction), label::caption(stage.clone())].spacing(6));
        }
        if let Some(error) = &u.error {
            status = status.push(Banner::error(error.as_str()));
        }
        let cancel = if working {
            cloud(Event::UploadStop)
        } else {
            cloud(Event::Close)
        };
        overlay::blocking(
            Dialog::new("Buluta yükle")
                // The fields scroll in a low window; the stage and the buttons stay in view.
                .scroll(form)
                .push(status)
                .action(secondary(
                    if working { "Durdur" } else { "Vazgeç" },
                    Some(cancel),
                ))
                .action(primary(
                    match u.storage {
                        ProjectStorage::File => "Buluta dosya olarak kaydet",
                        ProjectStorage::Database => "Buluta yükle",
                    },
                    (!working && !u.tenants.is_empty()).then(|| cloud(Event::UploadSubmit)),
                ))
                .width(600.0),
        )
    }

    /// The conflict window of a database project.
    pub(crate) fn conflicts_view(&self) -> Element<'_, Message> {
        let (Some(live), Some(doc)) = (&self.cloud.live, &self.document) else {
            return text("").into();
        };
        let list = live.sync.conflicts();
        let shown = 12;
        let mut rows = Column::new().spacing(4);
        for c in list.iter().take(shown) {
            let what = if c.reason == kentos_contracts::ConflictReason::Project {
                "Proje bilgileri (katman ağacı, ayarlar, ad, stiller)".to_owned()
            } else {
                describe(doc, &c.id, c.server.as_ref().map(|r| &r.entity))
            };
            rows = rows.push(
                row![
                    label::body(what).width(Fill),
                    label::caption(words::reason(c.reason)),
                ]
                .spacing(12),
            );
        }
        if list.len() > shown {
            rows = rows.push(label::caption(format!(
                "… ve {} nesne daha",
                list.len() - shown
            )));
        }
        overlay::blocking(
            Dialog::new("Kayıt çakışması")
                .push(label::body(format!(
                    "{} değişikliğiniz kaydedilmedi: aynı nesneleri başka biri daha önce kaydetti. Hiçbir şeyin üzerine yazılmadı; seçene kadar değişiklikleriniz yalnız bu cihazda.",
                    list.len()
                )))
                .push(container(scrollable(rows).direction(style::field::body_scrollbar())).max_height(300))
                .push(label::caption(
                    "“Sunucudakini al” sizin değişikliklerinizi bırakır. “Benimkini kaydet” başkasının değişikliğinin üzerine sizinkini yazar.",
                ))
                .aside(secondary("Sonra", Some(cloud(Event::Close))))
                .action(secondary("Benimkini kaydet", Some(cloud(Event::KeepMine))))
                .action(primary("Sunucudakini al", Some(cloud(Event::TakeTheirs))))
                .width(600.0),
        )
    }

    /// A question about the open file project's revisions (revisions.rs,
    /// the web's FileConflict.ts): what happened, what each answer does, and
    /// the answers in the bar's order; one that drops work stands apart on
    /// the left. Esc, × and the backdrop change nothing.
    pub(crate) fn revision_view(&self) -> Element<'_, Message> {
        let Some(q) = &self.cloud.question else {
            return text("").into();
        };
        let mut dialog = Dialog::new(q.title).push(label::body(q.message.clone()));
        if !q.details.is_empty() {
            let points = q.details.iter().fold(Column::new().spacing(4), |col, d| {
                col.push(row![label::body("•"), label::body(d.clone()).width(Fill)].spacing(6))
            });
            dialog = dialog.push(points);
        }
        for a in &q.answers {
            let answer = button(label::body(a.label))
                .on_press(cloud(Event::RevisionAnswer(a.value)))
                .padding([5, 16])
                .style(match a.kind {
                    Some(AnswerKind::Primary) => style::button::primary,
                    Some(AnswerKind::Danger) => style::button::danger_outline,
                    None => style::button::secondary,
                });
            dialog = if a.aside {
                dialog.aside(answer)
            } else {
                dialog.action(answer)
            };
        }
        overlay::modal(dialog.width(600.0), cloud(Event::RevisionAnswer(q.cancel)))
    }
}

/// A conflict's object as a row says it: its kind and layer, its label.
pub(super) fn describe(
    doc: &crate::document::Document,
    id: &str,
    server: Option<&kentos_contracts::Entity>,
) -> String {
    let here = crate::cloud::uuid(id)
        .and_then(|uid| doc.model.slot_of(uid))
        .and_then(|slot| doc.model.get(slot));
    let Some(entity) = here.or(server) else {
        return "Nesne".to_owned();
    };
    let base = entity.base();
    let layer = doc
        .model
        .layers()
        .get(&base.layer_id)
        .map_or(base.layer_id.clone(), |l| l.name.clone());
    let label = base
        .label
        .as_deref()
        .filter(|l| !l.is_empty())
        .map(|l| format!(" · {l}"))
        .unwrap_or_default();
    format!("{} · {layer}{label}", words::kind(entity.kind()))
}
