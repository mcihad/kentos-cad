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
pub mod command_line;
pub mod context_menu;
pub mod date_picker;
pub mod dialog;
pub mod dock;
mod dropdown;
pub mod inspector;
pub mod navigation_bar;
pub mod overlay;
pub mod property_grid;
pub mod query_builder;
pub mod ribbon;
pub mod segmented;
pub mod select;
pub mod status_bar;
pub mod table;
pub mod toolbar;
pub mod tree_view;

mod tip;

pub use app_menu::AppMenu;
pub use command_line::CommandLine;
pub use context_menu::{ContextMenu, Menu, MenuButton};
pub use date_picker::{DatePicker, TimePicker};
pub use dialog::{Dialog, ShortcutList};
pub use dock::{Dock, Panel};
pub use inspector::Inspector;
pub use navigation_bar::NavigationBar;
pub use property_grid::PropertyGrid;
pub use query_builder::QueryBuilder;
pub use ribbon::Ribbon;
pub use segmented::Segmented;
pub use select::{Choice, Select};
pub use status_bar::StatusBar;
pub use table::Table;
pub use tip::{Tip, tip};
pub use toolbar::Toolbar;
pub use tree_view::TreeView;

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
