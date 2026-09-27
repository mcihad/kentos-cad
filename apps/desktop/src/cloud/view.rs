//! The cloud's windows (docs/adr/0041), in the web's words: “Buluta giriş”,
//! “Bulut projeleri”, “Buluta yükle”, the conflict windows and an open's
//! progress. The status bar's cells are cells.rs's.

use iced::widget::{Column, button, column, container, row, scrollable, text, text_input};
use iced::{Element, Fill};

use kentos_contracts::ProjectStorage;
use kentos_ui::widget::{Banner, Dialog, Form, RadioGroup, overlay, progress};
use kentos_ui::{label, style};

use crate::app::{App, Message};
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

    /// “Buluta yükle”.
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
            .padding([5, 8])
            .style(style::field::input);
        let storage = RadioGroup::new(u.storage, |s| cloud(Event::UploadStorage(s)))
            .option(
                ProjectStorage::File,
                "Dosya (her kayıt bir revizyon)",
                "Proje sunucuda değişmez .kcad revizyonları olarak saklanır; Kaydet yeni bir revizyon yazar ve arada başkası kaydettiyse üzerine yazmaz.",
            )
            .option(
                ProjectStorage::Database,
                "Veritabanı (PostGIS)",
                "Nesneler sunucudaki veritabanında tek tek saklanır; her değişiklik kendiliğinden kaydedilir ve erişimi olan herkes hemen görür.",
            );
        form = form
            .field("Proje adı", name)
            .field("Saklama", storage)
            .row(label::caption(format!(
                "{} nesne, katman ağacı, proje ayarları ve proje stilleri yüklenir. Proje sizin olur; başkaları paylaşımla eklenir. Yüklenen proje sunucudan açılır.",
                doc.entity_count()
            )));
        if let Some(stage) = &u.stage {
            form = form.row(column![progress::bar(None), label::caption(stage.clone())].spacing(6));
        }
        if let Some(error) = &u.error {
            form = form.row(Banner::error(error.as_str()));
        }
        let cancel = if working {
            cloud(Event::UploadStop)
        } else {
            cloud(Event::Close)
        };
        overlay::blocking(
            Dialog::new("Buluta yükle")
                .push(form)
                .action(secondary(
                    if working { "Durdur" } else { "Vazgeç" },
                    Some(cancel),
                ))
                .action(primary(
                    "Buluta yükle",
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
                    "“Sunucudakini al” sizin değişikliklerinizi bırakır. “Benimkini koru” başkasının değişikliğinin üzerine sizinkini yazar.",
                ))
                .action(secondary("Sonra", Some(cloud(Event::Close))))
                .action(secondary("Benimkini koru", Some(cloud(Event::KeepMine))))
                .action(primary("Sunucudakini al", Some(cloud(Event::TakeTheirs))))
                .width(600.0),
        )
    }

    /// The conflict window of a file project's save.
    pub(crate) fn file_conflict_view(&self) -> Element<'_, Message> {
        let (Some(c), Some(doc)) = (&self.cloud.file_conflict, &self.document) else {
            return text("").into();
        };
        let name = doc.name();
        let base = if c.based_on == 0 {
            "çiziminiz projenin ilk kaydından önceki hâline dayanıyor".to_owned()
        } else {
            format!("çiziminiz revizyon {} üzerine kurulu", c.based_on)
        };
        overlay::blocking(
            Dialog::new("Kayıt çakışması")
                .push(label::body(format!(
                    "“{name}” projesini siz açtıktan sonra başka biri kaydetti: sunucudaki son revizyon {}, {base}. Hiçbir şeyin üzerine yazılmadı; değişiklikleriniz yalnız bu cihazda.",
                    c.server
                )))
                .push(label::caption(format!(
                    "“Sunucudaki son revizyonu aç” sizin değişikliklerinizi bırakır (önce sorulur). “Ayrı proje olarak kaydet” çiziminizi aynı çalışma alanında “{name} (kopya)” adıyla yeni bir dosya projesi yapar."
                )))
                .action(secondary("Vazgeç", Some(cloud(Event::Close))))
                .action(secondary("Sunucudaki son revizyonu aç", Some(cloud(Event::OpenLatest))))
                .action(primary("Ayrı proje olarak kaydet", Some(cloud(Event::SaveCopy))))
                .width(600.0),
        )
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

