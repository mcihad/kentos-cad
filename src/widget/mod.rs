//! Uygulama çerçevesi bileşenleri.
//!
//! Bileşenler yapıcı (builder) desenini izler ve `Element`'e dönüşür:
//!
//! ```ignore
//! Panel::new("Katmanlar", table).meta("6 katman").into()
//! ```
//!
//! Hepsi uygulamanın `Message` türünden bağımsızdır ve renklerini temadan
//! okur.

pub mod app_menu;
pub mod assets;
mod axis;
pub mod chips;
pub mod color;
pub mod command_line;
pub mod compass;
pub mod context_menu;
pub mod date_picker;
pub mod dialog;
pub mod dock;
pub mod docking;
mod dropdown;
pub mod floating;
pub mod form;
pub mod inspector;
pub mod legend;
pub mod mini_toolbar;
pub mod navigation_bar;
pub mod notice;
pub mod number;
pub mod overlay;
pub mod progress;
pub mod properties;
pub mod property_grid;
pub mod query_builder;
pub mod radial;
pub mod radio;
pub mod range;
pub mod ribbon;
pub mod rulers;
pub mod sash;
pub mod segmented;
pub mod select;
pub mod severity;
pub mod status_bar;
pub mod switch;
pub mod table;
pub mod tabs;
pub mod timeline;
pub mod toast;
pub mod toolbar;
pub mod tree_view;
pub mod viewports;
pub mod virtual_list;
pub mod wizard;

mod tip;

pub use app_menu::AppMenu;
pub use assets::AssetBrowser;
pub use chips::ChipInput;
pub use color::ColorPicker;
pub use command_line::CommandLine;
pub use compass::Compass;
pub use context_menu::{ContextMenu, Menu, MenuButton};
pub use date_picker::{DatePicker, TimePicker};
pub use dialog::{Confirm, Dialog, ShortcutList};
pub use dock::{Dock, Panel};
pub use docking::{DockSpace, Docks, Pane};
pub use floating::{Floating, ToolWindow};
pub use form::Form;
pub use inspector::Inspector;
pub use legend::Legend;
pub use mini_toolbar::MiniToolbar;
pub use navigation_bar::NavigationBar;
pub use notice::{Banner, EmptyState};
pub use number::{Dial, NumberInput};
pub use progress::{Task, TaskList};
pub use properties::PropertiesDialog;
pub use property_grid::PropertyGrid;
pub use query_builder::QueryBuilder;
pub use radial::RadialMenu;
pub use radio::RadioGroup;
pub use range::RangeSlider;
pub use ribbon::Ribbon;
pub use rulers::{Guides, Rulers};
pub use sash::Sash;
pub use segmented::Segmented;
pub use select::{Choice, Select};
pub use severity::Severity;
pub use status_bar::StatusBar;
pub use switch::Switch;
pub use table::Table;
pub use tabs::{Tab, Tabs};
pub use timeline::Timeline;
pub use tip::{Tip, tip};
pub use toast::{Toast, Toaster, Toasts};
pub use toolbar::Toolbar;
pub use tree_view::{Toggle, TreeView};
pub use viewports::{View, Viewports, Views};
pub use virtual_list::VirtualList;
pub use wizard::Wizard;

use iced::widget::{Rule, container, rule, space};
use iced::{Color, Element};

use crate::label;
use crate::style;

/// Yatay, 1 piksellik bölücü çizgi.
pub fn horizontal_divider<'a>() -> Rule<'a> {
    rule::horizontal(1).style(style::field::hairline)
}

/// Dikey, 1 piksellik bölücü çizgi.
pub fn vertical_divider<'a>() -> Rule<'a> {
    rule::vertical(1).style(style::field::hairline)
}

/// Katman veya sembol rengini gösteren küçük kare.
pub fn swatch<'a, Message: 'a>(color: Color) -> Element<'a, Message> {
    container(space::horizontal())
        .width(11)
        .height(11)
        .style(style::container::swatch(color))
        .into()
}

/// Kenarlı, eş aralıklı kısa etiket (ör. dosya biçimi "PDF").
pub fn badge<'a, Message: 'a>(content: &'a str) -> Element<'a, Message> {
    container(label::mono_caption(content).style(style::text::default))
        .width(64)
        .padding([3, 0])
        .align_x(iced::Center)
        .style(style::container::badge)
        .into()
}
