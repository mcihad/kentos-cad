//! The import window (the web's `svgImport.ts`, Inkscape's SVG import
//! dialog): the file's drawing as it will come in, what was kept, flattened
//! or left out, whether it opens as a new drawing or is added to this one
//! (fitted to the canvas), and which of its colours become the symbol's
//! colour and the second colour.

use iced::widget::{Column, Row, button, canvas, column, container, row, space};
use iced::{Center, Color, Element, Fill, Length, Theme};
use kentos_svg_core::import::{ColorUse, ImportOptions, SymbolColor, map_colors};
use kentos_svg_core::values::is_near_black;
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::style as ui_style;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::color::parse_hex;
use kentos_ui::widget::{Dialog, Tip, tip};

use super::super::doc::Drawing;
use super::super::panels::{Opt, seg};
use super::super::state::SvgEditor;
use super::super::{change, ev};
use super::preview::Preview;
use super::read::{read_svg, summary};
use super::{FileDialog, Pending};
use crate::app::Message;
use crate::style::fields;

/// What becomes the symbol's colour: near-black colours, the dominant one, one colour, or none.
#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    Black,
    Dominant,
    Color(String),
    Nothing,
}

impl Target {
    fn as_arg(&self) -> Option<&str> {
        match self {
            Target::Black => Some("black"),
            Target::Dominant => Some("dominant"),
            Target::Color(c) => Some(c),
            Target::Nothing => None,
        }
    }
}

pub struct ImportDialog {
    pub name: String,
    /// The drawing as read, its colours as the file has them.
    pub raw: Drawing,
    pub colors: Vec<ColorUse>,
    pub skipped: Vec<String>,
    pub done: String,
    pub lost: Vec<String>,
    pub symbol: Target,
    pub second: Option<String>,
    /// Opens as a new drawing (else added to this one).
    pub new: bool,
}

impl ImportDialog {
    fn dominant(&self) -> Option<&str> {
        self.colors.first().map(|c| c.color.as_str())
    }

    fn is_symbol(&self, c: &str) -> bool {
        match &self.symbol {
            Target::Black => is_near_black(c),
            Target::Dominant => self.dominant() == Some(c),
            Target::Color(x) => x == c,
            Target::Nothing => false,
        }
    }

    /// The drawing with the chosen colours mapped.
    pub fn mapped(&self) -> Drawing {
        map_colors(
            &self.raw.to_obj(),
            self.symbol.as_arg(),
            self.second.as_deref(),
        )
        .ok()
        .and_then(|o| Drawing::from_obj(&o).ok())
        .map(|mut d| {
            d.guides.clone_from(&self.raw.guides);
            d
        })
        .unwrap_or_else(|| self.raw.clone())
    }
}

/// SVG text into the import window (`openImportDialog`), or what went wrong.
pub fn open(ed: &mut SvgEditor, text: &str, name: &str) {
    let opts = ImportOptions {
        symbol_color: SymbolColor::Target(None),
        second_color: None,
        editor: false,
    };
    let r = match read_svg(text, &opts) {
        Ok(r) => r,
        Err(e) => {
            ed.warn(format!("“{name}”: {e}"));
            return;
        }
    };
    let (done, lost) = summary(&r.imported.report);
    let symbol = if r.imported.report.symbol_paint {
        Target::Nothing
    } else {
        Target::Black
    };
    ed.files.dialog = Some(FileDialog::Import(ImportDialog {
        name: name.to_owned(),
        raw: r.doc,
        colors: r.imported.colors.clone(),
        skipped: r.imported.skipped.clone(),
        done,
        lost,
        symbol,
        second: None,
        new: ed.doc.shapes.is_empty(),
    }));
    ed.touch();
}

fn with_dialog(f: impl Fn(&mut ImportDialog) + Send + Sync + 'static) -> Message {
    change(move |ed| {
        if let Some(FileDialog::Import(d)) = &mut ed.files.dialog {
            f(d);
        }
    })
}

/// The window's primary button: the drawing comes in.
fn apply(ed: &mut SvgEditor) {
    let Some(FileDialog::Import(d)) = ed.files.dialog.take() else {
        return;
    };
    let mapped = d.mapped();
    let tail = if d.lost.is_empty() {
        String::new()
    } else {
        format!("; {}", d.lost.join(", "))
    };
    let warn = !d.lost.is_empty();
    if d.new {
        let said = (format!("{}: yeni çizim olarak açıldı{tail}.", d.done), warn);
        let pending = Pending::Doc {
            doc: Box::new(mapped),
            name: d.name.clone(),
            said,
            reference: None,
        };
        if ed.may_replace(pending.clone())
            && let Pending::Doc {
                doc, name, said, ..
            } = pending
        {
            ed.open(*doc, None, name, None);
            ed.files.reference = None;
            ed.status(said.0, said.1);
        }
    } else {
        let ids = ed.add_shapes(&mapped);
        ed.status(
            format!(
                "{}{}{tail}.",
                d.done,
                if ids.is_empty() {
                    ""
                } else {
                    " ve çizime eklendi"
                }
            ),
            warn,
        );
    }
}

/// A colour chip: a swatch and its name, lit when chosen.
fn chip<'a>(
    words: &str,
    on: bool,
    color: Option<Color>,
    tip_text: &str,
    press: Message,
) -> Element<'a, Message> {
    let mut face = Row::new().spacing(6).align_y(Center);
    if let Some(c) = color {
        face = face.push(
            container(space())
                .width(12)
                .height(12)
                .style(move |t: &Theme| container::Style {
                    background: Some(iced::Background::Color(c)),
                    border: iced::Border {
                        color: Tokens::of(t).border,
                        width: 1.0,
                        radius: 2.0.into(),
                    },
                    ..container::Style::default()
                }),
        );
    }
    face = face.push(label::caption(words.to_owned()));
    tip(
        button(face)
            .padding([3, 8])
            .style(ui_style::button::tool(on))
            .on_press(press),
        Tip::new(tip_text.to_owned()),
        iced::widget::tooltip::Position::Top,
    )
}

fn chips<'a>(items: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    Row::with_children(items)
        .spacing(6)
        .wrap()
        .vertical_spacing(6)
        .into()
}

fn num2(v: f64) -> String {
    kentos_native_style::classify::rounded(v, 2)
}

pub fn view<'a>(
    ed: &'a SvgEditor,
    d: &'a ImportDialog,
    ink: &str,
    width: f32,
    height: f32,
) -> Element<'a, Message> {
    let mapped = d.mapped();
    let mut options = ed.options.clone();
    options.ink = ink.to_owned();
    let paper = parse_hex(&ed.paper());
    let size_text = format!(
        "{} × {} birim{}",
        num2(d.raw.width),
        num2(d.raw.height),
        d.raw
            .size_mm
            .map_or_else(String::new, |mm| format!(" · {} mm genişlik", num2(mm)))
    );
    let preview = column![
        container(
            canvas(Preview {
                shapes: mapped.shapes.clone(),
                view: [0.0, 0.0, mapped.width, mapped.height],
                options,
                paper,
            })
            .width(Fill)
            .height(Fill)
        )
        .height(typography::scaled(300.0))
        .style(ui_style::container::bordered),
        label::caption(size_text).style(ui_style::text::muted),
    ]
    .spacing(6)
    .width(Length::FillPortion(5));
    let fixed: Vec<&ColorUse> = d.colors.iter().take(12).collect();
    let mut symbol_chips: Vec<Element<'a, Message>> = vec![chip(
        "Siyah",
        d.symbol == Target::Black,
        Some(Color::BLACK),
        "Siyah ve siyaha yakın renkler sembol rengi olur",
        with_dialog(|d| d.symbol = Target::Black),
    )];
    if let Some(dom) = d.dominant() {
        symbol_chips.push(chip(
            "Baskın renk",
            d.symbol == Target::Dominant,
            parse_hex(dom),
            "En çok alanı boyayan renk sembol rengi olur",
            with_dialog(|d| d.symbol = Target::Dominant),
        ));
    }
    symbol_chips.push(chip(
        "Hiçbiri",
        d.symbol == Target::Nothing,
        None,
        "Renkler sabit kalır",
        with_dialog(|d| d.symbol = Target::Nothing),
    ));
    for c in &fixed {
        let color = c.color.clone();
        symbol_chips.push(chip(
            &c.color,
            d.is_symbol(&c.color),
            parse_hex(&c.color),
            &format!("{}: {} kez", c.color, c.count),
            with_dialog(move |d| {
                d.symbol = Target::Color(color.clone());
                if d.second.as_deref() == Some(color.as_str()) {
                    d.second = None;
                }
            }),
        ));
    }
    let mut second_chips: Vec<Element<'a, Message>> = vec![chip(
        "Hiçbiri",
        d.second.is_none(),
        None,
        "Hiçbiri",
        with_dialog(|d| d.second = None),
    )];
    for c in fixed.iter().filter(|c| !d.is_symbol(&c.color)) {
        let color = c.color.clone();
        second_chips.push(chip(
            &c.color,
            d.second.as_deref() == Some(c.color.as_str()),
            parse_hex(&c.color),
            &format!("{}: {} kez", c.color, c.count),
            with_dialog(move |d| d.second = Some(color.clone())),
        ));
    }
    let mut report: Column<'a, Message> = Column::new().spacing(4).push(
        row![
            icon(Icon::Check).size(14.0).tone(Tone::Success),
            label::body(format!("{}.", d.done))
        ]
        .spacing(6)
        .align_y(Center),
    );
    let mut lost: Vec<String> = d.lost.clone();
    if !d.skipped.is_empty() {
        lost.push(format!(
            "Tanınmayan öğeler atlandı: {}",
            d.skipped
                .iter()
                .map(|t| format!("<{t}>"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    for l in lost {
        report = report.push(
            row![
                icon(Icon::Warning).size(14.0).tone(Tone::Warning),
                label::body(l).width(Fill)
            ]
            .spacing(6),
        );
    }
    let form = column![
        fields::labelled(
            "Nasıl alınsın",
            seg(
                &[Opt("new", "Yeni çizim olarak aç"), Opt("add", "Bu çizime ekle")],
                Some(if d.new { "new" } else { "add" }),
                &[
                    "Tuval dosyanınki olur; kaydedince yeni çizim yazılır",
                    "Şekiller bu tuvale sığdırılarak eklenir",
                ],
                |v| with_dialog(move |d| d.new = v == "new"),
            ),
            None,
        ),
        fields::labelled(
            "Sembol rengi olacak renk",
            chips(symbol_chips),
            Some("Haritada sembol hangi rengi verirse bu renkle boyanan yerler o renge döner (currentColor)."),
        ),
        fields::labelled(
            "İkinci renk olacak renk",
            chips(second_chips),
            Some("Sembolün ikinci rengi (param(stroke)); öteki renkler sabit kalır."),
        ),
        report,
    ]
    .spacing(14);
    let primary = button(
        row![
            icon(Icon::Check).size(14.0).tone(Tone::OnAccent),
            label::body(if d.new {
                "Yeni çizim olarak aç"
            } else {
                "Bu çizime ekle"
            })
            .style(ui_style::text::on_accent)
        ]
        .spacing(6)
        .align_y(Center),
    )
    .padding([5, 14])
    .style(ui_style::button::primary)
    .on_press(change(apply));
    let foot = row![
        space::horizontal(),
        button(label::body("Vazgeç"))
            .padding([5, 14])
            .style(ui_style::button::secondary)
            .on_press(change(|ed| ed.files.dialog = None)),
        primary,
    ]
    .spacing(8)
    .align_y(Center);
    let _ = ev;
    Dialog::new(format!("SVG içe al: {}", d.name))
        .push(
            row![
                preview,
                iced::widget::scrollable(container(form).padding(iced::Padding {
                    right: 12.0,
                    ..iced::Padding::ZERO
                }))
                .direction(ui_style::field::body_scrollbar())
                .width(Length::FillPortion(6)),
            ]
            .spacing(18),
        )
        .push(foot)
        .width(typography::unscaled(width))
        .max_height(typography::unscaled(height))
        .into()
}
