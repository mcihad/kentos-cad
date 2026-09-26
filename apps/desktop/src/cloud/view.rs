//! The cloud's windows and status bar cells (docs/adr/0041), in the web's
//! words: “Buluta giriş”, “Bulut projeleri”, “Buluta yükle”, the conflict
//! windows and an open's progress; in the status bar the account with Çıkış,
//! the open project and where its save stands.

use iced::widget::text::Wrapping;
use iced::widget::{
    Column, button, column, container, mouse_area, row, scrollable, text, text_input,
};
use iced::{Center, Element, Fill, Length, mouse};

use kentos_cloud::SaveState;
use kentos_contracts::{CatalogView, ProjectState, ProjectStorage, ProjectSummary};
use kentos_ui::icon::Icon;
use kentos_ui::widget::status_bar::Readout;
use kentos_ui::widget::{Banner, Dialog, Form, RadioGroup, Tip, badge, overlay, progress};
use kentos_ui::{label, style};

use crate::app::{App, Message};
use crate::cloud::catalog::List;
use crate::cloud::copy::Link;
use crate::cloud::{Event, words};

fn cloud(event: Event) -> Message {
    crate::cloud::msg(event)
}

fn primary(caption: &'static str, on: Option<Message>) -> Element<'static, Message> {
    button(label::body(caption))
        .on_press_maybe(on)
        .padding([5, 16])
        .style(style::button::primary)
        .into()
}

fn secondary(caption: &'static str, on: Option<Message>) -> Element<'static, Message> {
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

    /// “Bulut projeleri”: the lists on the left, the chosen one in the middle.
    pub(crate) fn catalog_view(&self) -> Element<'_, Message> {
        let Some(c) = &self.cloud.catalog else {
            return text("").into();
        };
        let opening = self.cloud.opening.as_ref();
        let online = self.cloud.me.is_some();
        let orgs = self.organizations();
        let nav_button = |caption: String, list: List, enabled: bool| {
            let selected = c.list == list;
            button(label::body(caption))
                .on_press_maybe(
                    (opening.is_none() && enabled).then(|| cloud(Event::CatalogView(list))),
                )
                .width(Fill)
                .padding([5, 10])
                .style(style::button::navigation(selected))
        };
        let mut nav = Column::new().spacing(2).width(210);
        for view in words::VIEWS {
            if view == CatalogView::Organization {
                if orgs.is_empty() {
                    nav = nav.push(nav_button(
                        words::view(view).label.to_owned(),
                        List::Organization(String::new()),
                        online,
                    ));
                }
                for (id, name) in &orgs {
                    nav = nav.push(nav_button(
                        format!("Kurum: {name}"),
                        List::Organization(id.clone()),
                        online,
                    ));
                }
            } else {
                nav = nav.push(nav_button(
                    words::view(view).label.to_owned(),
                    List::View(view),
                    online,
                ));
            }
        }
        nav = nav
            .push(iced::widget::space::vertical().height(8))
            .push(nav_button(
                "Bu cihazdaki projeler".to_owned(),
                List::Device,
                true,
            ));
        if !online {
            nav = nav.push(iced::widget::space::vertical().height(8)).push(
                column![
                    label::caption("Sunucudaki listeler için giriş yapın."),
                    button(label::body("Buluta giriş…"))
                        .on_press(Message::Run("cloud.signIn"))
                        .padding([5, 10])
                        .style(style::button::secondary),
                ]
                .spacing(6),
            );
        }
        let device = c.list == List::Device;
        let (title, note, empty) = match &c.list {
            List::View(v) => (
                words::view(*v).label.to_owned(),
                words::view(*v).note,
                words::view(*v).empty,
            ),
            List::Organization(id) => (
                orgs.iter()
                    .find(|(t, _)| t == id)
                    .map_or("Kurum projeleri".to_owned(), |(_, n)| {
                        format!("Kurum projeleri: {n}")
                    }),
                "",
                words::view(CatalogView::Organization).empty,
            ),
            List::Device => (
                "Bu cihazdaki projeler".to_owned(),
                "Bu bilgisayarda kopyası tutulan projeler bağlantı olmadan da açılır; değişiklikler bağlantı gelince gönderilir.",
                "Bu bilgisayarda kopyası tutulan bir proje yok. Bir bulut projesini bir kez açınca kopyası burada tutulur.",
            ),
        };
        let count = if device {
            format!("{} proje", c.device.len())
        } else if c.total > 0 {
            format!("{} proje", c.total)
        } else {
            String::new()
        };
        let search = text_input("Ara: ad, açıklama, etiket", &c.search)
            .on_input(|t| cloud(Event::CatalogSearch(t)))
            .padding([5, 8])
            .style(style::field::input);
        let mut main = column![
            row![
                label::heading(title),
                iced::widget::space::horizontal(),
                label::caption(count)
            ]
            .align_y(Center),
            search,
        ]
        .spacing(8)
        .width(Fill);
        if !note.is_empty() {
            main = main.push(label::caption(note));
        }
        let list: Element<'_, Message> = if let Some(error) = &c.error {
            column![
                Banner::error(error.as_str()),
                secondary("Yeniden dene", Some(cloud(Event::CatalogRetry))),
            ]
            .spacing(8)
            .into()
        } else if device && !c.device.is_empty() {
            let rows = c.device.iter().fold(
                Column::new().spacing(2).padding(iced::padding::right(14)),
                |col, k| col.push(self.kept_row(k, c)),
            );
            scrollable(rows)
                .direction(style::field::body_scrollbar())
                .height(Fill)
                .into()
        } else if !device && !c.projects.is_empty() {
            let rows = c.projects.iter().fold(
                Column::new().spacing(2).padding(iced::padding::right(14)),
                |col, p| col.push(self.catalog_row(p, c)),
            );
            scrollable(rows)
                .direction(style::field::body_scrollbar())
                .height(Fill)
                .into()
        } else {
            let empty = if c.loading() {
                "Projeler yükleniyor…"
            } else if !c.search.trim().is_empty() {
                "Aramanıza uyan proje yok. Başka sözcüklerle deneyin."
            } else if matches!(&c.list, List::Organization(id) if id.is_empty()) {
                "Etkin üyeliğiniz olan bir kurum yok; kurum projeleri burada görünür."
            } else {
                empty
            };
            container(label::muted(empty)).padding(12).into()
        };
        main = main.push(container(list).height(Length::Fixed(400.0)));
        if !device && c.next.is_some() {
            main = main.push(secondary(
                if c.loading() {
                    "Yükleniyor…"
                } else {
                    "Daha fazla göster"
                },
                (!c.loading()).then(|| cloud(Event::CatalogMore)),
            ));
        }
        if let Some(o) = opening {
            main = main.push(
                column![
                    label::body(format!("“{}” açılıyor", o.name)),
                    progress::bar(o.fraction),
                    label::caption(o.stage.clone()),
                ]
                .spacing(6),
            );
        } else if let Some(status) = &c.status {
            main = main.push(Banner::info(status.as_str()));
        }
        let (can_open, read_only) = if device {
            let k = c.picked_kept();
            (k.is_some(), k.is_some_and(|k| k.ended.is_some()))
        } else {
            let p = c.picked();
            (
                p.is_some(),
                p.is_some_and(|p| p.state == ProjectState::Archived),
            )
        };
        let open_caption = if read_only { "Salt okunur aç" } else { "Aç" };
        let cancel = if opening.is_some() {
            cloud(Event::OpenCancel)
        } else {
            cloud(Event::Close)
        };
        let mut dialog = Dialog::new("Bulut projeleri").push(row![nav, main].spacing(16));
        if device {
            dialog = dialog.action(secondary(
                "Bu cihazdan kaldır",
                (can_open && opening.is_none()).then(|| cloud(Event::CatalogRemove)),
            ));
        }
        overlay::blocking(
            dialog
                .action(secondary("Vazgeç", Some(cancel)))
                .action(primary(
                    open_caption,
                    (can_open && opening.is_none()).then(|| cloud(Event::CatalogOpen)),
                ))
                .width(1000.0),
        )
    }

    /// A project of this device's copies: its name, where it is, how it is
    /// kept, the account's role, and when the copy was last written.
    fn kept_row<'a>(
        &'a self,
        k: &'a kentos_cloud::Kept,
        c: &crate::cloud::Catalog,
    ) -> Element<'a, Message> {
        let info = &k.info;
        let selected = c.picked.as_deref() == Some(info.id.as_str());
        let open = self
            .document
            .as_ref()
            .and_then(|d| d.cloud_source())
            .is_some_and(|s| s.info.id == info.id);
        let place = words::workspace(info.tenant_kind, &info.tenant_name, true);
        let mut name = row![label::strong(info.name.as_str())]
            .spacing(6)
            .align_y(Center);
        if open {
            name = name.push(label::caption("Açık"));
        }
        if let Some(ended) = k.ended {
            name = name.push(label::caption(words::ended(ended)));
        }
        let saved = words::ago_ms(k.saved_at);
        let body = row![
            column![
                name,
                label::caption(format!("{place} · Bu cihazda: {saved}")),
            ]
            .spacing(2)
            .width(Fill),
            badge(words::storage_badge(info.storage)),
            container(label::caption(words::role(info.access.role)))
                .width(96)
                .align_x(iced::Right),
        ]
        .spacing(12)
        .align_y(Center);
        mouse_area(
            container(body)
                .padding([6, 10])
                .width(Fill)
                .style(style::container::suggestion(selected)),
        )
        .on_press(cloud(Event::CatalogPick(info.id.clone())))
        .on_double_click(cloud(Event::CatalogOpen))
        .interaction(mouse::Interaction::Pointer)
        .into()
    }

    /// One project of a list: its name and marks; where it is, whose, when it
    /// changed; how it is kept and the account's role.
    fn catalog_row<'a>(
        &'a self,
        p: &'a ProjectSummary,
        c: &crate::cloud::Catalog,
    ) -> Element<'a, Message> {
        let selected = c.picked.as_deref() == Some(p.id.as_str());
        let open = self
            .document
            .as_ref()
            .and_then(|d| d.cloud_source())
            .is_some_and(|s| s.info.id == p.id);
        let own = self.cloud.membership(&p.tenant_id).is_some();
        let place = words::workspace(p.tenant_kind, &p.tenant_name, own);
        let owner = if p.owner_name.is_empty() {
            "görünmüyor"
        } else {
            p.owner_name.as_str()
        };
        let mut name = row![label::strong(p.name.as_str())]
            .spacing(6)
            .align_y(Center);
        if p.favorite {
            name = name.push(label::caption("★"));
        }
        if open {
            name = name.push(label::caption("Açık"));
        }
        if p.state == ProjectState::Archived {
            name = name.push(label::caption("Arşivde"));
        }
        let sub = format!(
            "{place} · Sahibi: {owner} · Güncellendi: {}",
            words::ago(&p.updated_at)
        );
        let body = row![
            column![name, label::caption(sub)].spacing(2).width(Fill),
            badge(words::storage_badge(p.storage)),
            container(label::caption(words::role(p.access.role)))
                .width(96)
                .align_x(iced::Right),
        ]
        .spacing(12)
        .align_y(Center);
        mouse_area(
            container(body)
                .padding([6, 10])
                .width(Fill)
                .style(style::container::suggestion(selected)),
        )
        .on_press(cloud(Event::CatalogPick(p.id.clone())))
        .on_double_click(cloud(Event::CatalogOpen))
        .interaction(mouse::Interaction::Pointer)
        .into()
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
    /// The status bar's cloud cells: the open project with its workspace,
    /// where its save stands and the connection's dot, then the account with
    /// Çıkış. One line each, bounded, so they fit beside the drawing's cells.
    pub(crate) fn cloud_cells(&self) -> Vec<Element<'_, Message>> {
        let line = |s: String| text(s).wrapping(Wrapping::None);
        let mut cells: Vec<Element<'_, Message>> = Vec::new();
        if let Some((doc, source)) = self
            .document
            .as_ref()
            .and_then(|d| d.cloud_source().map(|s| (d, s)))
        {
            let revision = source
                .revision
                .as_ref()
                .map_or(String::new(), |r| format!(" Revizyon {}.", r.number));
            cells.push(
                Readout::new(line(format!("{} › {}", source.workspace, doc.name())))
                    .width(170.0)
                    .icon(Icon::Globe)
                    .tip(Tip::new("Bulut projesi").body(format!(
                        "{} › {}. Saklama: {}. Rolünüz: {}.{revision}",
                        source.workspace,
                        doc.name(),
                        words::storage_title(source.storage()),
                        words::role(source.info.access.role)
                    )))
                    .into(),
            );
            let (state, tone, on) = self.save_cell();
            let tip = self.save_tip();
            let (dot, dot_tone, about) = self.link_cell();
            // Signed out, the save cell says it all (“Çevrimdışı — değişiklikler bu
            // cihazda saklanıyor”); signed in, the dot beside it says how the connection is.
            let signed_out = self.cloud.me.is_none();
            let save = Readout::new(
                row![
                    text("●").style(move |theme: &iced::Theme| dot_tone.style(theme)),
                    line(state).style(move |theme: &iced::Theme| tone.style(theme)),
                ]
                .spacing(5)
                .align_y(Center),
            )
            .on_press(on)
            .tip(Tip::new("Bulut kaydı").body(tip));
            cells.push(save.into());
            // The connection's word, unless the save's words say it already.
            let says = signed_out
                || self
                    .cloud
                    .live
                    .as_ref()
                    .is_some_and(|l| l.sync.state() == SaveState::Offline);
            if !says {
                cells.push(
                    Readout::new(line(dot.to_owned()))
                        .tip(Tip::new("Bağlantı").body(about))
                        .into(),
                );
            }
        }
        match &self.cloud.me {
            Some(me) => {
                cells.push(
                    Readout::new(line(me.user.display_name.clone()))
                        .tip(Tip::new("Bulut hesabı").body(format!(
                            "{} olarak giriş yapıldı ({}).",
                            me.user.display_name,
                            self.cloud.client.as_ref().map_or("", |c| c.server())
                        )))
                        .into(),
                );
                cells.push(
                    Readout::new(line("Çıkış".to_owned()))
                        .on_press(Message::Run("cloud.signOut"))
                        .tip("Bulut oturumunu kapatır")
                        .into(),
                );
            }
            None => cells.push(
                Readout::new(line("Buluta giriş".to_owned()))
                    .icon(Icon::Globe)
                    .on_press(Message::Run("cloud.signIn"))
                    .tip("KentOS sunucusunda hesabınızla oturum açar")
                    .into(),
            ),
        }
        cells
    }

    /// The connection's dot: çevrimiçi, çevrimdışı or eşitleniyor. Without a
    /// session the work stays on this device, and the dot says so.
    pub(crate) fn link_cell(&self) -> (&'static str, Tone, &'static str) {
        if self.cloud.me.is_none() {
            (
                "Çevrimdışı — değişiklikler bu cihazda saklanıyor",
                Tone::Danger,
                "Oturum açık değil ya da sunucuya ulaşılamıyor. Çalışmaya devam edebilirsiniz: değişiklikler bu cihazda saklanır; giriş yapınca gönderilir.",
            )
        } else if self.cloud.link == Link::Offline {
            (
                "Çevrimdışı",
                Tone::Danger,
                "Sunucuya ulaşılamıyor. Çalışmaya devam edebilirsiniz: değişiklikler bu cihazda saklanır ve bağlantı gelince gönderilir; sunucu 15 saniyede bir yeniden denenir.",
            )
        } else if self.syncing() {
            (
                "Eşitleniyor",
                Tone::Accent,
                "Değişiklikler sunucuya gidiyor.",
            )
        } else {
            (
                "Çevrimiçi",
                Tone::Good,
                "Sunucuya bağlı; başkalarının değişiklikleri kaydedildikleri anda gelir.",
            )
        }
    }

    /// Where the open cloud project's save stands: its words, its tone, and
    /// what a click does (the conflict window, send now, Kaydet).
    pub(crate) fn save_cell(&self) -> (String, Tone, Message) {
        let save = Message::Run("file.save");
        if let Some(c) = &self.cloud.file_conflict {
            return (
                format!("Çakışma: r{} kaydedilmiş", c.server),
                Tone::Danger,
                Message::Run("cloud.conflicts"),
            );
        }
        let signed_out = self.cloud.me.is_none();
        if let Some(live) = &self.cloud.live {
            let state = live.sync.state();
            let tone = match state {
                SaveState::Conflict
                | SaveState::Error
                | SaveState::Deleted
                | SaveState::Revoked
                | SaveState::Archived => Tone::Danger,
                SaveState::ReadOnly | SaveState::Offline => Tone::Muted,
                _ => Tone::Plain,
            };
            let on = if state == SaveState::Conflict {
                Message::Run("cloud.conflicts")
            } else {
                save
            };
            // Without a session nothing goes: the work stays on this device.
            if signed_out && !state.ended() && state != SaveState::Conflict {
                let text = if live.sync.all_sent() {
                    "Çevrimdışı".to_owned()
                } else {
                    "Çevrimdışı — değişiklikler bu cihazda saklanıyor".to_owned()
                };
                return (text, Tone::Muted, on);
            }
            let text = match (state, live.sync.error()) {
                (SaveState::Error, Some(why)) => format!("Kaydedilmedi: {why}"),
                _ => words::save_state(state, live.sync.pending()),
            };
            return (text, tone, on);
        }
        let Some((doc, source)) = self
            .document
            .as_ref()
            .and_then(|d| d.cloud_source().map(|s| (d, s)))
        else {
            return (String::new(), Tone::Plain, save);
        };
        // A file project, in the web's words (docs/adr/0038).
        if self.saving.is_some() {
            return ("Kaydediliyor…".to_owned(), Tone::Plain, save);
        }
        if source.archived() {
            return ("Arşivlendi".to_owned(), Tone::Danger, save);
        }
        if !source.can_write() {
            return ("Salt okunur".to_owned(), Tone::Muted, save);
        }
        let base = source.revision.as_ref().map_or(0, |r| r.number);
        let kept = self
            .cloud
            .held
            .as_ref()
            .is_some_and(|h| h.kept_save && h.session == doc.session);
        match (doc.dirty(), kept, base) {
            (true, _, 0) => ("Kaydedilmedi".to_owned(), Tone::Plain, save),
            (true, _, n) => (format!("Kaydedilmedi · r{n} üstüne"), Tone::Plain, save),
            (false, true, _) => (
                "Kaydedildi (bu cihazda) · gönderilecek".to_owned(),
                Tone::Muted,
                save,
            ),
            (false, false, 0) => ("Henüz revizyon yok".to_owned(), Tone::Muted, save),
            (false, false, n) => (format!("Buluta kaydedildi · r{n}"), Tone::Plain, save),
        }
    }

    fn save_tip(&self) -> String {
        match &self.cloud.live {
            Some(_) => "Değişiklikler kendiliğinden kaydedilir; Ctrl+S hemen gönderir. Gönderilmemiş değişiklikler bu cihazdaki taslakta saklanır; bağlantı yokken de çalışabilirsiniz.".to_owned(),
            None => "Dosya projesi: Kaydet (Ctrl+S) çizimi projenin yeni revizyonu olarak kaydeder; arada başkası kaydettiyse üzerine yazmaz. Bağlantı yokken kayıt bu cihazda tutulur ve bağlantı gelince gönderilir.".to_owned(),
        }
    }
}

/// The tone of a cloud cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Plain,
    Muted,
    Danger,
    Accent,
    Good,
}

impl Tone {
    pub(crate) fn style(self, theme: &iced::Theme) -> iced::widget::text::Style {
        match self {
            Tone::Plain => style::text::default(theme),
            Tone::Muted => style::text::muted(theme),
            Tone::Danger => style::text::danger(theme),
            Tone::Accent => style::text::accent(theme),
            Tone::Good => iced::widget::text::Style {
                color: Some(kentos_ui::theme::Tokens::of(theme).success),
            },
        }
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
