//! The SVG editor's bar above the canvas (the web's `svgMenus.ts` and the
//! file buttons of `svgFile.ts`): Dosya, Altlık…, Bitmap izle…, Dışa
//! aktar… and Kaynak; the Yol (path operations), Nesne (arranging, the
//! tabs, the order) and Seç (selection helpers) menus; the snapping switch
//! with its kinds, the rulers' switch, and the zoom. Every entry says its
//! key; a menu that does not fit the window scrolls.

use iced::Element;
use kentos_ui::icon::Icon;
use kentos_ui::widget::Menu;

use super::actions::{Action, PathOp, Restack, Same};
use super::files::{self, FileCmd};
use super::icons::svg_icon;
use super::state::{SNAP_KINDS, SvgEditor, Tab};
use super::{Event, change, ev};
use crate::app::Message;

fn path_menu(ed: &SvgEditor) -> Menu<Message> {
    let offset = kentos_expression::js::number::to_string(ed.ui.offset);
    let simplify = kentos_expression::js::number::to_string(ed.ui.simplify);
    let p =
        |m: Menu<Message>, op: PathOp, words: &str, keys: Option<&str>, detail: Option<&str>| {
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
    m = p(
        m,
        PathOp::Union,
        "Birleşim",
        Some("Ctrl++"),
        Some("Seçilenlerin kapladığı her yer tek yol"),
    );
    m = p(
        m,
        PathOp::Difference,
        "Fark",
        Some("Ctrl+-"),
        Some("Alttakinden üsttekiler çıkar"),
    );
    m = p(
        m,
        PathOp::Intersection,
        "Kesişim",
        Some("Ctrl+*"),
        Some("Yalnızca hepsinin ortak yeri"),
    );
    m = p(
        m,
        PathOp::Exclusion,
        "Dışlama",
        Some("Ctrl+^"),
        Some("Ortak yerler boşalır"),
    );
    m = p(
        m,
        PathOp::Division,
        "Bölme",
        Some("Ctrl+/"),
        Some("Alttaki, üsttekilerin çizgileriyle parçalara bölünür"),
    );
    m = p(
        m,
        PathOp::Cut,
        "Yolu kes",
        Some("Ctrl+Alt+/"),
        Some("Alttakinin çizgisi kesişimlerde açık parçalara ayrılır"),
    );
    m = m.separator();
    m = p(m, PathOp::Combine, "Tek yolda topla", Some("Ctrl+K"), None);
    m = p(
        m,
        PathOp::BreakApart,
        "Parçalara ayır",
        Some("Ctrl+Shift+K"),
        None,
    );
    m = p(
        m,
        PathOp::Split,
        "Parçalara ayır, delikler kalsın",
        None,
        None,
    );
    m = m.separator();
    m = p(
        m,
        PathOp::ToPath,
        "Nesneyi yola çevir",
        Some("Ctrl+Shift+C"),
        Some("Dikdörtgen ve elips düğümlü yol olur"),
    );
    m = p(
        m,
        PathOp::StrokeToPath,
        "Çizgiyi yola çevir",
        Some("Ctrl+Alt+C"),
        Some("Çizginin boyadığı alan dolgulu yol olur"),
    );
    m = p(
        m,
        PathOp::Inset,
        &format!("İçe küçült ({offset})"),
        Some("Ctrl+("),
        None,
    );
    m = p(
        m,
        PathOp::Outset,
        &format!("Dışa büyüt ({offset})"),
        Some("Ctrl+)"),
        None,
    );
    m = p(
        m,
        PathOp::Simplify,
        &format!("Sadeleştir (%{simplify})"),
        Some("Ctrl+L"),
        Some("Daha az düğüm; mesafe ve tolerans Özellikler’de"),
    );
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
    let r = |m: Menu<Message>, op: Restack, words: &str, glyph: &'static str, keys: &str| {
        m.item(words.to_owned(), change(move |ed| ed.restack(op)))
            .icon(Icon::Svg(glyph))
            .shortcut(keys.to_owned())
    };
    let a =
        |m: Menu<Message>, act: Action, words: &str, glyph: &'static str, keys: Option<&str>| {
            let m = m
                .item(words.to_owned(), change(move |ed| ed.action(act)))
                .icon(Icon::Svg(glyph));
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
    m = r(m, Restack::Top, "En öne", svg_icon("toTop"), "Home");
    m = r(m, Restack::Raise, "Bir öne", svg_icon("raise"), "PageUp");
    m = r(
        m,
        Restack::Lower,
        "Bir arkaya",
        svg_icon("lower"),
        "PageDown",
    );
    m = r(m, Restack::Bottom, "En arkaya", svg_icon("toBottom"), "End");
    m = m.separator();
    m = a(
        m,
        Action::FlipH,
        "Yatay çevir",
        svg_icon("flipH"),
        Some("H"),
    );
    m = a(
        m,
        Action::FlipV,
        "Dikey çevir",
        svg_icon("flipV"),
        Some("Shift+H"),
    );
    m = a(m, Action::Rot90, "90° döndür", svg_icon("rotate"), None);
    m = m.separator();
    m = a(
        m,
        Action::Group,
        "Grupla",
        svg_icon("sheetGroup"),
        Some("Ctrl+G"),
    );
    m = a(
        m,
        Action::Ungroup,
        "Grubu çöz",
        svg_icon("sheetUngroup"),
        Some("Ctrl+Shift+G"),
    );
    m = a(
        m,
        Action::Duplicate,
        "Çoğalt",
        svg_icon("copy"),
        Some("Ctrl+D"),
    );
    a(m, Action::Delete, "Sil", svg_icon("trash"), Some("Delete"))
}

fn select_menu() -> Menu<Message> {
    let same = |what: Same| change(move |ed| ed.select_same(what));
    Menu::new()
        .item("Tümünü seç", change(|ed| ed.select_all()))
        .icon(Icon::Svg(svg_icon("selectAll")))
        .shortcut("Ctrl+A")
        .item("Seçimi ters çevir", change(|ed| ed.invert_selection()))
        .icon(Icon::Svg(svg_icon("selectInvert")))
        .shortcut("!")
        .item("Seçimi kaldır", change(|ed| ed.select(Vec::new())))
        .icon(Icon::Svg(svg_icon("deselect")))
        .shortcut("Esc")
        .separator()
        .item("Aynı dolguyu seç", same(Same::Fill))
        .icon(Icon::Svg(svg_icon("selectSame")))
        .item("Aynı çizgiyi seç", same(Same::Stroke))
        .icon(Icon::Svg(svg_icon("selectSameStroke")))
        .item("Aynı dolgu ve çizgiyi seç", same(Same::Both))
        .icon(Icon::Svg(svg_icon("selectSameBoth")))
        .item("Aynı türü seç", same(Same::Kind))
        .icon(Icon::Svg(svg_icon("selectSameKind")))
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

/// The bar over the canvas (style/strip.rs): the file's group, the drawing's menus, then history, the
/// aids and the zoom; it wraps group after group on a narrow middle.
pub fn bar<'a>(ed: &'a SvgEditor) -> Element<'a, Message> {
    use crate::style::strip;
    let snap_on = ed.options.snap_objects;
    let rulers_on = ed.options.rulers;
    let items: Vec<Element<'a, Message>> = vec![
        strip::menu(
            Icon::Folder,
            "Dosya",
            "Dosya: Aç, kitaplıktan aç, panodan al, dışa aktar, farklı kaydet, belge özellikleri",
            move || files::file_menu(ed),
        ),
        strip::icon_button(
            crate::icons::from_web(Some("layers")),
            "Altlık…: PNG, JPEG ya da SVG görüntüsünü kilitli, yarı saydam izleme altlığı olarak koyar",
            Some(ev(Event::File(files::Event::Cmd(FileCmd::AddReference)))),
        ),
        strip::icon_button(
            crate::icons::from_web(Some("spline")),
            "Bitmap izle…: Bir görüntüyü (altlığı ya da dosyayı) delikli yollara çevirir",
            Some(ev(Event::File(files::Event::Cmd(FileCmd::Trace)))),
        ),
        strip::icon_button(
            Icon::Export,
            "Dışa aktar…: SVG (sembol ya da düz renkli) ya da PNG olarak kaydet veya panoya kopyala (Ctrl+Shift+E)",
            Some(ev(Event::File(files::Event::Cmd(FileCmd::Export)))),
        ),
        strip::icon_toggle(
            Icon::Terminal,
            "Kaynak: SVG kaynağı; seçilen şeklin öğesi vurgulanır, düzenleyip Uygula ile çizime aktarın (Ctrl+Shift+X)",
            ed.files.source.is_some(),
            ev(Event::File(files::Event::Cmd(FileCmd::ToggleSource))),
        ),
        strip::divider(),
        strip::menu(
            Icon::Svg(svg_icon("pathUnion")),
            "Yol",
            "Yol: Birleşim, fark, kesişim, çizgiyi yola çevir, küçült/büyüt, sadeleştir",
            move || path_menu(ed),
        ),
        strip::menu(
            Icon::Svg(svg_icon("alignLeft")),
            "Nesne",
            "Nesne: Hizala, dönüştür, dizi, sıra, grup",
            object_menu,
        ),
        strip::menu(
            Icon::Svg(svg_icon("selectSame")),
            "Seç",
            "Seç: Tümünü seç, ters çevir, benzerini seç",
            select_menu,
        ),
        strip::divider(),
        strip::icon_button(
            Icon::Undo,
            "Geri al (Ctrl+Z)",
            ed.can_undo().then(|| ev(Event::Undo)),
        ),
        strip::icon_button(
            Icon::Redo,
            "Yinele (Ctrl+Y)",
            ed.can_redo().then(|| ev(Event::Redo)),
        ),
        strip::divider(),
        strip::split_toggle(
            Icon::Magnet,
            "Kenet: Şekillere, kılavuzlara ve tuvale kenetle (%)",
            snap_on,
            change(move |ed| {
                ed.options.snap_objects = !snap_on;
                ed.snapper.reset();
            }),
            "Kenet türleri",
            move || snap_menu(ed),
        ),
        strip::icon_toggle(
            Icon::Svg(svg_icon("rulers")),
            "Cetvel: Kılavuz çekmek için cetvelden sürükleyin",
            rulers_on,
            change(move |ed| {
                ed.options.rulers = !rulers_on;
                ed.touch();
            }),
        ),
        strip::divider(),
        strip::icon_button(
            Icon::ZoomOut,
            "Uzaklaş (−)",
            Some(ev(Event::ZoomBy(1.0 / 1.25))),
        ),
        strip::readout(ed.camera.percent(), "Görünümün ölçeği", 46.0),
        strip::icon_button(Icon::ZoomIn, "Yakınlaş (+)", Some(ev(Event::ZoomBy(1.25)))),
        strip::icon_button(Icon::ZoomExtents, "Tuvale sığdır (0)", Some(ev(Event::Fit))),
    ];
    iced::widget::column![strip::strip(items), strip::rule()].into()
}
