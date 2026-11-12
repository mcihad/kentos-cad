//! Veri profili's Kurum adları page (docs/adr/5180; plan/map-cad.md MAP-17): every field of the chosen profile with its
//! name in the institution by the chosen mapping and a form of names (lowerCamelCase or BÜYÜK_HARF, Turkish letters kept
//! or written in ASCII), where the name comes from and what is wrong with it (`cad.profiles.names`, read only). Biçimi
//! eşlemeye yaz writes the form into the chosen mapping (`cad.profiles.import` of it again, where it is kept); from then
//! on Bulgular and Uygula read the institution's names by it.

use crate::gis;
use iced::widget::{Column, row};
use iced::{Center, Element};
use kentos_commands::{
    CommandResult, MappingNames, NameCase, NameLetters, NameProblem, NameSource, ProfileNames,
    ProfileSource, ProfileStore, ProfilesImport, ProfilesNames,
};
use kentos_interaction::Level;
use kentos_native_application::profile_files::Shelf;
use kentos_native_application::{ExecutionContext, profile_files, profiles_names};
use kentos_ui::widget::{Form, Segmented};

use crate::app::{App, Message};
use crate::data_profile::Event;
use crate::exchange::words::{self, Kind};
use crate::road_dialog::{col, sheet_chosen};

pub(crate) const FORM_LIST: &str = "Ad biçimi";
pub(crate) const ASCII: &str = "Türkçe harfleri ASCII yaz (ç → c, ı → i)";
pub(crate) const WRITE: &str = "Biçimi eşlemeye yaz";

/// The window's two pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Findings,
    Names,
}

impl Page {
    pub(crate) const ALL: [Page; 2] = [Page::Findings, Page::Names];
}

impl std::fmt::Display for Page {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Page::Findings => "Bulgular",
            Page::Names => "Kurum adları",
        })
    }
}

/// The form the names are read with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameForm {
    /// The mapping's own (none without a mapping, or a mapping without a form).
    Mapping,
    Camel,
    Snake,
}

impl NameForm {
    pub(crate) const ALL: [NameForm; 3] = [NameForm::Mapping, NameForm::Camel, NameForm::Snake];
}

impl std::fmt::Display for NameForm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            NameForm::Mapping => "Eşlemedeki",
            NameForm::Camel => "lowerCamelCase",
            NameForm::Snake => "BÜYÜK_HARF",
        })
    }
}

/// The page's state.
#[derive(Debug, Clone)]
pub(crate) struct Names {
    pub page: Page,
    pub form: NameForm,
    pub ascii: bool,
    pub answer: Result<ProfileNames, String>,
    /// The row pressed: its problem said under the table.
    pub chosen: Option<usize>,
}

impl Default for Names {
    fn default() -> Self {
        Names {
            page: Page::Findings,
            form: NameForm::Mapping,
            ascii: false,
            answer: Err(String::new()),
            chosen: None,
        }
    }
}

impl Names {
    /// The form chosen to read the names with; none: the mapping's own.
    pub(crate) fn names(&self) -> Option<MappingNames> {
        let case = match self.form {
            NameForm::Mapping => return None,
            NameForm::Camel => NameCase::LowerCamelCase,
            NameForm::Snake => NameCase::UpperSnakeCase,
        };
        Some(MappingNames {
            case,
            letters: if self.ascii {
                NameLetters::Ascii
            } else {
                NameLetters::Keep
            },
        })
    }
}

fn source_words(s: NameSource) -> &'static str {
    match s {
        NameSource::Profile => "profil",
        NameSource::Form => "biçim",
        NameSource::Mapping => "eşleme",
    }
}

fn basis(form: NameForm) -> &'static str {
    match form {
        NameForm::Mapping => {
            "Kurumdaki ad önce eşlemenin kendi yazdığıdır, sonra eşlemenin ad biçimi, yoksa profilin adı. Bir biçim seçip önizleyin; tutarsa eşlemeye yazın."
        }
        NameForm::Camel => {
            "TUCBS Veri Tanımlama Dokümanı Şablonu §5.2.3: öznitelik adı lowerCamelCase yazılır; tümü büyük harfli kısaltmalar kalır. Türkçe harf için yazılı kural yok (tema dokümanları ASCII yazar): harfleri siz seçin."
        }
        NameForm::Snake => {
            "BÖHHBÜY Ek-1'in öznitelik kodları büyük harf, ASCII ve “_” ile yazılmıştır; yönetmelik bunu kural olarak yazmaz, kolon adını Ek-2'de veri tabanının kabulüne bırakır. Ek-1 kodları addan türetilemez: kurumun kodunu eşlemede yazın."
        }
    }
}

impl App {
    /// `cad.profiles.names` for the window's profile, mapping and form.
    pub(crate) fn data_profile_names_read(&self) -> Result<ProfileNames, String> {
        let (Some(w), Some(doc)) = (self.gis.data_profile.as_ref(), self.document.as_ref()) else {
            return Err(String::new());
        };
        match profiles_names::run_in(
            &doc.model,
            &ProfilesNames {
                name: Some(w.profile.clone()),
                mapping_name: w.mapping.clone(),
                names: w.names.names(),
                drawing: self.profile_drawing(),
                ..ProfilesNames::default()
            },
        ) {
            CommandResult::Completed { output, .. } => Ok(output),
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => Err(error.message),
            _ => Err("Adlar okunamadı.".to_owned()),
        }
    }

    /// Whether the chosen form can be written into the chosen mapping: a form other than the mapping's own that holds.
    pub(crate) fn data_profile_form_writable(&self) -> bool {
        let Some(w) = self.gis.data_profile.as_ref() else {
            return false;
        };
        w.names.names().is_some()
            && w.mapping.is_some()
            && matches!(&w.names.answer, Ok(a) if a.holds)
    }

    /// Biçimi eşlemeye yaz: the chosen mapping imported again with the form, where it is kept, over itself.
    pub(crate) fn data_profile_write_form(&mut self) {
        let drawing = self.profile_drawing();
        let Some(w) = self.gis.data_profile.as_ref() else {
            return;
        };
        let (Some(name), Some(form)) = (w.mapping.clone(), w.names.names()) else {
            return;
        };
        let store = if w.listed.iter().any(|m| {
            m.kind == kentos_commands::ProfileFileKind::Mapping
                && m.name == name
                && m.source == ProfileSource::Drawing
        }) {
            ProfileStore::Drawing
        } else {
            ProfileStore::Folder
        };
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let Some(mut m) = profile_files::find_mapping(Shelf::of(&doc.model, drawing.as_deref()), &name) else {
            self.say(Level::Error, format!("“{name}” eşlemesi okunamadı."));
            return;
        };
        m.names = Some(form);
        let result = profile_files::import::execute(
            &mut ExecutionContext::new(&mut doc.model),
            ProfilesImport {
                mapping: Some(m),
                drawing,
                overwrite: Some(true),
                store: Some(store),
                ..ProfilesImport::default()
            },
        );
        let Some(done) = self.answered(result) else {
            return;
        };
        self.say(
            Level::Success,
            format!(
                "“{}” eşlemesine {} ad biçimi yazıldı; Bulgular ve Uygula kurumun adlarını bununla okur.",
                done.title,
                kentos_commands::case_words(form.case)
            ),
        );
        if let Some(w) = self.gis.data_profile.as_mut() {
            w.names.form = NameForm::Mapping;
        }
    }

    /// The Kurum adları page.
    pub(crate) fn data_profile_names_part<'a>(&'a self, names: &'a Names) -> Element<'a, Message> {
        let msg = |e: Event| Message::Gis(gis::Event::DataProfile(e));
        let (lines, rows, problems): (Vec<Element<'_, Message>>, Vec<Vec<String>>, Vec<Option<String>>) =
            match &names.answer {
                Err(why) if why.is_empty() => (Vec::new(), Vec::new(), Vec::new()),
                Err(why) => (vec![words::text_line(Kind::Warn, why.clone())], Vec::new(), Vec::new()),
                Ok(a) => {
                    let line = if a.holds {
                        words::text_line(
                            Kind::Ok,
                            format!(
                                "Kurumdaki adı profilinkinden başka olan alan: {} / {}; her alanın sınıfında kendi adı var.",
                                a.renamed,
                                a.rows.len()
                            ),
                        )
                    } else {
                        words::text_line(
                            Kind::Warn,
                            format!(
                                "{} alanın adı bu biçimle yazılamıyor, {} alan başka bir alanın adına düşüyor: bu adları eşlemede yazın.",
                                a.unfit, a.clashes
                            ),
                        )
                    };
                    let rows = a
                        .rows
                        .iter()
                        .map(|r| {
                            vec![
                                r.class_title.clone(),
                                r.field.clone(),
                                match r.problem {
                                    Some(NameProblem::Unfit) => "— yazılamıyor".to_owned(),
                                    Some(NameProblem::Clash) => format!("{} — çakışıyor", r.name),
                                    None => r.name.clone(),
                                },
                                source_words(r.source).to_owned(),
                            ]
                        })
                        .collect();
                    let problems = a.rows.iter().map(|r| r.message.clone()).collect();
                    (vec![line], rows, problems)
                }
            };
        let can_write = self.data_profile_form_writable();
        let hint = names
            .chosen
            .and_then(|i| problems.get(i).cloned().flatten())
            .unwrap_or_else(|| "Sorunlu satıra basınca nedeni burada görünür.".to_owned());
        let form = Form::new()
            .label_width(130.0)
            .field(
                FORM_LIST,
                Segmented::new(NameForm::ALL, names.form, move |f| msg(Event::NamesForm(f))),
            )
            .help(basis(names.form))
            .field(
                "Harfler",
                words::check(
                    names.ascii,
                    ASCII,
                    (names.form != NameForm::Mapping).then(|| msg(Event::NamesAscii(!names.ascii))),
                ),
            );
        let table = sheet_chosen(
            vec![
                col("Sınıf", 130.0),
                col("Profildeki ad", 230.0),
                col("Kurumdaki ad", 260.0),
                col("Nereden", 90.0),
            ],
            rows,
            "Profilin alanı yok.",
            300.0,
            names.chosen,
            move |i| msg(Event::NamesPick(i)),
        );
        let write_hint = match self
            .gis
            .data_profile
            .as_ref()
            .and_then(|w| w.mapping.as_ref())
        {
            None => {
                "Biçimi yazmak için bir eşleme seçin ya da içe aktarın; eşlemesiz tablo önizlemedir."
            }
            Some(_) => {
                "Biçim eşlemenin kendi yazdığı adlara dokunmaz; eşleme dosyası yeniden yazılır."
            }
        };
        Column::new()
            .spacing(12)
            .push(form)
            .push(words::summary(lines))
            .push(kentos_ui::label::caption(hint))
            .push(
                row![
                    words::ghost(WRITE, can_write.then(|| msg(Event::NamesWrite))),
                    kentos_ui::label::caption(write_hint),
                ]
                .spacing(12)
                .align_y(Center),
            )
            .push(table)
            .into()
    }
}
