//! The SVG editor's bar above the canvas (the web's `svgMenus.ts` and the
//! file buttons of `svgFile.ts`): Dosya, Altlık…, Bitmap izle…, Dışa
//! aktar… and Kaynak; the Yol (path operations), Nesne (arranging, the
//! tabs, the order) and Seç (selection helpers) menus; the snapping switch
//! with its kinds, the rulers' switch, and the zoom. Every entry says its
//! key; a menu that does not fit the window scrolls.

use iced::widget::{Row, button, container, row, space};
use iced::{Center, Element, Fill, Theme};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::style as ui_style;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Menu, MenuButton, Tip, tip};

use super::actions::{Action, PathOp, Restack, Same};
use super::files::{self, FileCmd};
use super::icons::svg_icon;
use super::state::{SNAP_KINDS, SvgEditor, Tab};
use super::{Event, change, ev};
use crate::app::Message;

/// A button of the bar: an icon, words and a chevron when it opens a menu.
fn face<'a>(glyph: Icon, words: &str, chevron: bool) -> Element<'a, Message> {
    let mut r = row![
        icon(glyph).size(14.0),
        label::body(words.to_owned()).wrapping(iced::widget::text::Wrapping::None)
    ]
    .spacing(6)
    .align_y(Center);
    if chevron {
        r = r.push(icon(Icon::ChevronDown).size(12.0).tone(Tone::Muted));
    }
    container(r).padding([3, 8]).into()
}

fn bar_button<'a>(glyph: Icon, words: &str, tip_text: &str, press: Message, pressed: bool) -> Element<'a, Message> {
    tip(
        button(face(glyph, words, false))
            .padding(0)
            .style(move |t: &Theme, s| {
                if pressed {
                    ui_style::button::tool(true)(t, s)
                } else {
                    ui_style::button::secondary(t, s)
                }
            })
            .on_press(press),
        Tip::new(tip_text.to_owned()),
        iced::widget::tooltip::Position::Bottom,
    )
}

fn menu_button<'a>(
    glyph: Icon,
    words: &str,
    tip_text: &str,
    menu: impl Fn() -> Menu<Message> + 'a,
) -> Element<'a, Message> {
    let b = container(face(glyph, words, true)).style(|t: &Theme| container::Style {
        border: iced::Border {
            color: Tokens::of(t).border,
            width: 1.0,
            radius: 4.0.into(),
        },
        ..container::Style::default()
    });
    tip(
        MenuButton::new(b, menu),
        Tip::new(tip_text.to_owned()),
        iced::widget::tooltip::Position::Bottom,
    )
}

fn path_menu(ed: &SvgEditor) -> Menu<Message> {
    let offset = kentos_expression::js::number::to_string(ed.ui.offset);
    let simplify = kentos_expression::js::number::to_string(ed.ui.simplify);
    let p = |m: Menu<Message>, op: PathOp, words: &str, keys: Option<&str>, detail: Option<&str>| {
        // Every entry with its icon: the names line up whether or not a line of detail follows.
        let mut m = m
            .item(words.to_owned(), change(move |ed| ed.path_op(op)))
            .icon(Icon::Svg(svg_icon(op_icon(op))));
        if let Some(k) = keys {
            m = m.shortcut(k.to_owned());
        }
        if let Some(d) = detail {
            m = m.detail(d.to_owned());
        }
        m
    };
    let mut m = Menu::new();
    m = p(m, PathOp::Union, "Birleşim", Some("Ctrl++"), Some("Seçilenlerin kapladığı her yer tek yol"));
    m = p(m, PathOp::Difference, "Fark", Some("Ctrl+-"), Some("Alttakinden üsttekiler çıkar"));
    m = p(m, PathOp::Intersection, "Kesişim", Some("Ctrl+*"), Some("Yalnızca hepsinin ortak yeri"));
    m = p(m, PathOp::Exclusion, "Dışlama", Some("Ctrl+^"), Some("Ortak yerler boşalır"));
    m = p(m, PathOp::Division, "Bölme", Some("Ctrl+/"), Some("Alttaki, üsttekilerin çizgileriyle parçalara bölünür"));
    m = p(m, PathOp::Cut, "Yolu kes", Some("Ctrl+Alt+/"), Some("Alttakinin çizgisi kesişimlerde açık parçalara ayrılır"));
    m = m.separator();
    m = p(m, PathOp::Combine, "Tek yolda topla", Some("Ctrl+K"), None);
    m = p(m, PathOp::BreakApart, "Parçalara ayır", Some("Ctrl+Shift+K"), None);
    m = p(m, PathOp::Split, "Parçalara ayır, delikler kalsın", None, None);
    m = m.separator();
    m = p(m, PathOp::ToPath, "Nesneyi yola çevir", Some("Ctrl+Shift+C"), Some("Dikdörtgen ve elips düğümlü yol olur"));
    m = p(m, PathOp::StrokeToPath, "Çizgiyi yola çevir", Some("Ctrl+Alt+C"), Some("Çizginin boyadığı alan dolgulu yol olur"));
    m = p(m, PathOp::Inset, &format!("İçe küçült ({offset})"), Some("Ctrl+("), None);
    m = p(m, PathOp::Outset, &format!("Dışa büyüt ({offset})"), Some("Ctrl+)"), None);
    m = p(m, PathOp::Simplify, &format!("Sadeleştir (%{simplify})"), Some("Ctrl+L"), Some("Daha az düğüm; mesafe ve tolerans Özellikler’de"));
    m = m.separator();
    m = p(m, PathOp::Reverse, "Yönü çevir", None, None);
    m = p(m, PathOp::Close, "Yolu kapat", None, None);
    p(m, PathOp::Open, "Yolu aç", None, None)
}

/// A path operation's icon (the web's `svgIcons.ts`).
fn op_icon(op: PathOp) -> &'static str {
    match op {
        PathOp::Union => "pathUnion",
        PathOp::Difference => "pathDifference",
        PathOp::Intersection => "pathIntersection",
        PathOp::Exclusion => "pathExclusion",
        PathOp::Division => "pathDivision",
        PathOp::Cut => "pathCut",
        PathOp::Combine => "pathCombine",
        PathOp::BreakApart => "pathBreak",
        PathOp::Split => "pathSplit",
        PathOp::ToPath => "toPath",
        PathOp::StrokeToPath => "strokeToPath",
        PathOp::Inset => "inset",
        PathOp::Outset => "outset",
        PathOp::Simplify => "simplify",
        PathOp::Reverse => "reverse",
        PathOp::Close => "closePath",
        PathOp::Open => "openPath",
    }
}

fn object_menu() -> Menu<Message> {
    let tab = |t: Tab| {
        change(move |ed| {
            ed.ui.tab = t;
            ed.touch();
        })
    };
    let r = |m: Menu<Message>, op: Restack, words: &str, keys: &str| {
        m.item(words.to_owned(), change(move |ed| ed.restack(op)))
            .shortcut(keys.to_owned())
    };
    let a = |m: Menu<Message>, act: Action, words: &str, keys: Option<&str>| {
        let m = m.item(words.to_owned(), change(move |ed| ed.action(act)));
        match keys {
            Some(k) => m.shortcut(k.to_owned()),
            None => m,
        }
    };
    let mut m = Menu::new()
        .item("Hizala ve dağıt…", tab(Tab::Align))
        .icon(Icon::Svg(svg_icon("alignLeft")))
        .shortcut("Ctrl+Shift+A")
        .item("Dönüştür…", tab(Tab::Transform))
        .icon(crate::icons::from_web(Some("move")))
        .shortcut("Ctrl+Shift+M")
        .item("Dizi ve aynalı kopya…", tab(Tab::Array))
        .icon(crate::icons::from_web(Some("array")))
        .separator();
    m = r(m, Restack::Top, "En öne", "Home");
    m = r(m, Restack::Raise, "Bir öne", "PageUp");
    m = r(m, Restack::Lower, "Bir arkaya", "PageDown");
    m = r(m, Restack::Bottom, "En arkaya", "End");
    m = m.separator();
    m = a(m, Action::FlipH, "Yatay çevir", Some("H"));
    m = a(m, Action::FlipV, "Dikey çevir", Some("Shift+H"));
    m = a(m, Action::Rot90, "90° döndür", None);
    m = m.separator();
    m = a(m, Action::Group, "Grupla", Some("Ctrl+G"));
    m = a(m, Action::Ungroup, "Grubu çöz", Some("Ctrl+Shift+G"));
    m = a(m, Action::Duplicate, "Çoğalt", Some("Ctrl+D"));
    a(m, Action::Delete, "Sil", Some("Delete"))
}

fn select_menu() -> Menu<Message> {
    let same = |what: Same| change(move |ed| ed.select_same(what));
    Menu::new()
        .item("Tümünü seç", change(|ed| ed.select_all()))
        .shortcut("Ctrl+A")
        .item("Seçimi ters çevir", change(|ed| ed.invert_selection()))
        .shortcut("!")
        .item("Seçimi kaldır", change(|ed| ed.select(Vec::new())))
        .shortcut("Esc")
        .separator()
        .item("Aynı dolguyu seç", same(Same::Fill))
        .item("Aynı çizgiyi seç", same(Same::Stroke))
        .item("Aynı dolgu ve çizgiyi seç", same(Same::Both))
        .item("Aynı türü seç", same(Same::Kind))
}

fn snap_menu(ed: &SvgEditor) -> Menu<Message> {
    let mut m = Menu::new().header("Kenet türleri");
    for (k, name) in SNAP_KINDS {
        let on = ed.options.snap_kinds.contains(&k);
        m = m.check(
            name,
            on,
            change(move |ed| {
                let now = ed.options.snap_kinds.clone();
                ed.options.snap_kinds = SNAP_KINDS
                    .iter()
                    .map(|(x, _)| *x)
                    .filter(|x| if *x == k { !on } else { now.contains(x) })
                    .collect();
                ed.snapper.reset();
            }),
        );
    }
    let grid = ed.options.snap_grid;
    m.separator().check(
        "Izgaraya kenetle",
        grid,
        change(move |ed| ed.options.snap_grid = !grid),
    )
}

/// The bar over the canvas; it wraps to a second line on a narrow middle.
pub fn bar<'a>(ed: &'a SvgEditor) -> Element<'a, Message> {
    let snap_on = ed.options.snap_objects;
    let rulers_on = ed.options.rulers;
    let snap = row![
        bar_button(
            Icon::Magnet,
            "Kenet",
            "Şekillere, kılavuzlara ve tuvale kenetle (%)",
            change(move |ed| {
                ed.options.snap_objects = !snap_on;
                ed.snapper.reset();
            }),
            snap_on,
        ),
        tip(
            MenuButton::new(
                container(icon(Icon::ChevronDown).size(12.0))
                    .center_x(typography::scaled(20.0))
                    .center_y(typography::scaled(28.0)),
                move || snap_menu(ed),
            ),
            Tip::new("Kenet türleri".to_owned()),
            iced::widget::tooltip::Position::Bottom,
        ),
    ]
    .spacing(0)
    .align_y(Center);
    let zoom_btn = |glyph: Icon, words: &str, press: Message| {
        tip(
            button(container(icon(glyph).size(14.0)).center_x(typography::scaled(26.0)).center_y(typography::scaled(26.0)))
                .padding(0)
                .style(ui_style::button::ghost)
                .on_press(press),
            Tip::new(words.to_owned()),
            iced::widget::tooltip::Position::Bottom,
        )
    };
    let files_group: Vec<Element<'a, Message>> = vec![
        menu_button(
            Icon::Folder,
            "Dosya",
            "Aç, kitaplıktan aç, panodan al, dışa aktar, farklı kaydet, belge özellikleri",
            move || files::file_menu(ed),
        ),
        bar_button(
            crate::icons::from_web(Some("layers")),
            "Altlık…",
            "İzleme altlığı: PNG, JPEG ya da SVG görüntüsünü kilitli, yarı saydam arka plan olarak koyar",
            ev(Event::File(files::Event::Cmd(FileCmd::AddReference))),
            false,
        ),
        bar_button(
            crate::icons::from_web(Some("spline")),
            "Bitmap izle…",
            "Bir görüntüyü (altlığı ya da dosyayı) delikli yollara çevirir",
            ev(Event::File(files::Event::Cmd(FileCmd::Trace))),
            false,
        ),
        bar_button(
            Icon::Export,
            "Dışa aktar…",
            "SVG (sembol ya da düz renkli) ya da PNG olarak kaydet veya panoya kopyala (Ctrl+Shift+E)",
            ev(Event::File(files::Event::Cmd(FileCmd::Export))),
            false,
        ),
        bar_button(
            Icon::Terminal,
            "Kaynak",
            "SVG kaynağı: seçilen şeklin öğesi vurgulanır; düzenleyip Uygula ile çizime aktarın (Ctrl+Shift+X)",
            ev(Event::File(files::Event::Cmd(FileCmd::ToggleSource))),
            ed.files.source.is_some(),
        ),
    ];
    let edit_group: Vec<Element<'a, Message>> = vec![
        menu_button(
            Icon::Svg(svg_icon("pathUnion")),
            "Yol",
            "Yol işlemleri: birleşim, fark, kesişim, çizgiyi yola çevir, küçült/büyüt, sadeleştir",
            move || path_menu(ed),
        ),
        menu_button(
            Icon::Svg(svg_icon("alignLeft")),
            "Nesne",
            "Hizala, dönüştür, dizi, sıra, grup",
            object_menu,
        ),
        menu_button(
            Icon::Svg(svg_icon("selectSame")),
            "Seç",
            "Tümünü seç, ters çevir, benzerini seç",
            select_menu,
        ),
    ];
    let history = |glyph: Icon, words: &str, press: Option<Message>| {
        tip(
            button(container(icon(glyph).size(14.0)).center_x(typography::scaled(26.0)).center_y(typography::scaled(26.0)))
                .padding(0)
                .style(ui_style::button::ghost)
                .on_press_maybe(press),
            Tip::new(words.to_owned()),
            iced::widget::tooltip::Position::Bottom,
        )
    };
    let view_group: Vec<Element<'a, Message>> = vec![
        history(Icon::Undo, "Geri al (Ctrl+Z)", ed.can_undo().then(|| ev(Event::Undo))),
        history(Icon::Redo, "Yinele (Ctrl+Y)", ed.can_redo().then(|| ev(Event::Redo))),
        snap.into(),
        bar_button(
            Icon::Svg(svg_icon("rulers")),
            "Cetvel",
            "Cetveller: kılavuz çekmek için cetvelden sürükleyin",
            change(move |ed| {
                ed.options.rulers = !rulers_on;
                ed.touch();
            }),
            rulers_on,
        ),
        zoom_btn(Icon::ZoomOut, "Uzaklaş (−)", ev(Event::ZoomBy(1.0 / 1.25))),
        container(label::body(ed.camera.percent()).font(typography::mono()))
            .center_x(typography::scaled(52.0))
            .into(),
        zoom_btn(Icon::ZoomIn, "Yakınlaş (+)", ev(Event::ZoomBy(1.25))),
        zoom_btn(Icon::ZoomExtents, "Tuvale sığdır (0)", ev(Event::Fit)),
    ];
    let sep = || -> Element<'a, Message> {
        container(space())
            .width(1)
            .height(typography::scaled(18.0))
            .style(|t: &Theme| container::Style {
                background: Some(iced::Background::Color(Tokens::of(t).border)),
                ..container::Style::default()
            })
            .into()
    };
    // One row that wraps button by button on a narrow middle (the web's bar).
    let mut items = files_group;
    items.push(sep());
    items.extend(edit_group);
    items.push(sep());
    items.extend(view_group);
    container(
        Row::with_children(items)
            .spacing(6)
            .align_y(Center)
            .wrap()
            .vertical_spacing(6),
    )
    .padding([8, 10])
    .width(Fill)
    .into()
}
