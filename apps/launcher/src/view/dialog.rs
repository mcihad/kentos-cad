//! Uygulama ekle / Uygulamayı düzenle and Kaldır's question (docs/adr/0800
//! §2): any program, in any language, or a web address becomes a card. The
//! dialog writes the manifest into the user's folder; what it says is the
//! format's own (docs/launcher/README.md), so the file can be edited by hand.

use iced::widget::tooltip::Position;
use iced::widget::{Column, Row, button, column, container, row, text, text_input};
use iced::{Alignment, Element, Fill, Length};
use kentos_ui::label;
use kentos_ui::theme::{metrics, typography};
use kentos_ui::widget::{Confirm, Dialog, Form, Segmented, Tip, tip};

use crate::app::{Field, Launcher, Message};
use crate::draft::{Draft, Kind};
use crate::glyphs::Glyph;
use crate::resolve;
use crate::tones::Tone;
use crate::view::style::{self, px};
use crate::view::tile;

fn input<'a>(
    placeholder: &'a str,
    value: &'a str,
    on_input: impl Fn(String) -> Field + 'a,
) -> Element<'a, Message> {
    text_input(placeholder, value)
        .on_input(move |v| Message::Draft(on_input(v)))
        .on_submit(Message::SaveDraft)
        .size(typography::body())
        .font(typography::ui())
        .padding(metrics::padding(metrics::control(), 8.0))
        .style(kentos_ui::style::field::input)
        .width(Fill)
        .into()
}

pub fn edit<'a>(launcher: &'a Launcher, draft: &'a Draft) -> Element<'a, Message> {
    let problems = draft.problems();
    let shown = |problem: &Option<String>| if draft.tried { problem.clone() } else { None };
    let title = if draft.base.is_some() {
        "Uygulamayı düzenle"
    } else {
        "Uygulama ekle"
    };

    let mut form = Form::new()
        .label_width(140.0)
        .field("Ad", input("Kartta görünecek ad", &draft.name, Field::Name))
        .required()
        .error(shown(&problems.name))
        .field(
            "Açıklama",
            input("Ne işe yaradığı, tek satır", &draft.note, Field::Note),
        )
        .field(
            "Tür",
            Segmented::new([Kind::Program, Kind::Web], draft.kind, |k| {
                Message::Draft(Field::Kind(k))
            })
            .width(Fill),
        );
    form = match draft.kind {
        Kind::Program => form
            .field(
                "Program",
                row![
                    input("python3 ya da /yol/program", &draft.program, Field::Program),
                    button(label::body("Seç…"))
                        .padding(metrics::padding(metrics::control(), 12.0))
                        .style(kentos_ui::style::button::secondary)
                        .on_press(Message::PickProgram),
                ]
                .spacing(px(8.0))
                .align_y(Alignment::Center),
            )
            .required()
            .help("Tam yolu ya da PATH'te bulunan adı (ör. python3, java). Python, Java, Electron… her dilde program olur.")
            .error(shown(&problems.program))
            .field("Bağımsız değişkenler", input("ör. rapor.py {dosya}", &draft.args, Field::Args))
            .help(format!(
                "Boşlukla ayrılır; boşluklu değeri tırnağa alın. {} açılacak dosyanın yeridir.",
                resolve::FILE
            ))
            .error(problems.args.clone()),
        Kind::Web => form
            .field("Web adresi", input("https://…", &draft.url, Field::Url))
            .required()
            .help("Sistemin tarayıcısında açılır.")
            .error(shown(&problems.url)),
    };

    let tones = Tone::ALL
        .into_iter()
        .fold(Row::new().spacing(px(6.0)), |row, tone| {
            let swatch = container(iced::widget::Space::new())
                .width(px(22.0))
                .height(px(22.0))
                .style(style::tile(tone.paint(), px(22.0)));
            row.push(tip(
                button(swatch)
                    .padding(px(3.0))
                    .style(style::choice(draft.tone == tone))
                    .on_press(Message::Draft(Field::Tone(tone))),
                Tip::new(tone.name()),
                Position::Top,
            ))
        });
    // The glyphs in two rows: a row of eleven does not fit the field.
    let glyphs = Glyph::ALL
        .chunks(6)
        .fold(Column::new().spacing(px(6.0)), |rows, chunk| {
            rows.push(
                chunk
                    .iter()
                    .fold(Row::new().spacing(px(6.0)), |row, &glyph| {
                        row.push(tip(
                            button(tile(glyph, draft.tone, px(28.0)))
                                .padding(px(3.0))
                                .style(style::choice(
                                    draft.icon_file.is_none() && draft.glyph == glyph,
                                ))
                                .on_press(Message::Draft(Field::Glyph(glyph))),
                            Tip::new(glyph.name()),
                            Position::Top,
                        ))
                    }),
            )
        });
    form = form
        .field("Renk", tones)
        .field("Simge", container(glyphs).width(Fill));
    if let Some(own) = &draft.icon_file {
        form = form.help(format!(
            "Şimdiki simgesi kendi dosyası: {}. Bir simge seçilirse onun yerine geçer.",
            own.display()
        ));
    }

    let preview = container(
        row![
            match (&draft.icon_file, &draft.base) {
                (Some(_), Some(base)) => crate::view::app_tile(launcher, base, px(44.0)),
                _ => tile(draft.glyph, draft.tone, px(44.0)),
            },
            column![
                text(if draft.name.trim().is_empty() {
                    "Ad"
                } else {
                    draft.name.trim()
                })
                .size(px(16.0))
                .font(typography::ui_strong()),
                label::muted(if draft.note.trim().is_empty() {
                    "Açıklama"
                } else {
                    draft.note.trim()
                }),
            ]
            .spacing(px(2.0)),
        ]
        .spacing(px(12.0))
        .align_y(Alignment::Center),
    )
    .padding(px(12.0))
    .width(Fill)
    .style(style::card(false));

    let mut body = column![preview, form].spacing(px(16.0));
    if let Some(refusal) = &draft.refusal {
        body = body.push(label::caption(refusal.as_str()).style(kentos_ui::style::text::danger));
    }
    let folder = launcher.folders.user.as_ref().map_or_else(
        || "kullanıcının uygulama klasörü".to_owned(),
        |f| f.display().to_string(),
    );
    let save = button(
        container(label::strong(if draft.base.is_some() {
            "Kaydet"
        } else {
            "Ekle"
        }))
        .center_x(Length::Shrink)
        .center_y(Length::Fixed(metrics::button())),
    )
    .padding([0.0, px(18.0)])
    .style(kentos_ui::style::button::primary)
    .on_press(Message::SaveDraft);
    let cancel = button(
        container(label::body("Vazgeç"))
            .center_x(Length::Shrink)
            .center_y(Length::Fixed(metrics::button())),
    )
    .padding([0.0, px(16.0)])
    .style(kentos_ui::style::button::secondary)
    .on_press(Message::CloseDialog);

    Dialog::new(title)
        .hint("Esc")
        .scroll(body)
        .aside(label::caption(format!("Manifest: {folder}")))
        .action(cancel)
        .action(save)
        .width(560.0)
        .fixed()
        .into()
}

pub fn remove<'a>(id: &'a str, name: &'a str) -> Element<'a, Message> {
    Confirm::new(
        format!("“{name}” kaldırılsın mı?"),
        Message::RemoveConfirmed(id.to_owned()),
        Message::CloseDialog,
    )
    .message("Manifest dosyası silinir ve kart kaybolur; programın kendisi silinmez.")
    .confirm("Kaldır")
    .cancel("Vazgeç")
    .destructive()
    .into()
}
