//! Mekânsal sayfa: ViewCube, araçlar, nesne yakalama ve biçimlendirme.

use iced::widget::{Row, button, column, container};
use iced::{Bottom, Center, Element, Fill, Theme};

use kentos_rc::icon::icon;
use kentos_rc::label;
use kentos_rc::spatial::query::SnapKind;
use kentos_rc::spatial::{LonLat, Tool, ViewCube, format, model_space};
use kentos_rc::style;
use kentos_rc::widget::table::{self, Table};

use super::entry;
use crate::app::Showcase;
use crate::command::{self, Command};
use crate::message::{Message, RibbonTab};

impl Showcase {
    pub(super) fn spatial_page(&self) -> Vec<Element<'_, Message>> {
        let cubes = container(
            Row::with_children([64.0, 96.0, 128.0].map(|size: f32| {
                column![
                    ViewCube::new(self.cube_rotation).size(size),
                    label::mono_caption(format!("{size:.0}"))
                ]
                .spacing(6)
                .align_x(Center)
                .into()
            }))
            .spacing(32)
            .align_y(Bottom),
        )
        .padding(16)
        .style(|theme: &Theme| {
            style::container::solid(model_space::Style::of(theme).background)(theme)
        });

        let tools = Table::new([
            table::Column::new("").width(16),
            table::Column::new("Araç").width(100),
            table::Column::new("Ne yapar").width(Fill),
            table::Column::new("Komut").width(100),
        ])
        .extend(
            Tool::NAVIGATION
                .into_iter()
                .chain(Tool::DRAWING)
                .map(|tool| {
                    table::Row::new([
                        icon(tool.icon()).into(),
                        label::body(tool.label()).into(),
                        label::muted(tool.description()).into(),
                        label::mono(command::name(Command::Tool(tool))).into(),
                    ])
                }),
        );

        let snaps = Table::new([
            table::Column::new("Tür").width(100),
            table::Column::new("Etiket").width(100),
            table::Column::new("İşaret").width(Fill),
        ])
        .extend(
            [
                (SnapKind::Endpoint, "Kare: çizginin başı ya da sonu"),
                (
                    SnapKind::Vertex,
                    "Baklava: çizgi ara köşesi veya alan köşesi",
                ),
                (SnapKind::Node, "İçi çarpılı daire: nokta öğe"),
            ]
            .map(|(kind, marker)| {
                table::Row::new([
                    label::mono(format!("{kind:?}")).into(),
                    label::body(kind.label()).into(),
                    label::muted(marker).into(),
                ])
            }),
        );

        let istanbul = LonLat::new(28.9784, 41.0082);

        let formatting = Table::new([
            table::Column::new("Çağrı").width(Fill),
            table::Column::new("Sonuç").width(280),
        ])
        .extend(
            [
                (
                    "format::integer(15_840_900.0)",
                    format::integer(15_840_900.0),
                ),
                ("format::distance(950.0)", format::distance(950.0)),
                ("format::distance(346_710.0)", format::distance(346_710.0)),
                (
                    "format::dms(41.0082, \"K\", \"G\")",
                    format::dms(41.0082, "K", "G"),
                ),
                (
                    "format::coordinates(istanbul)",
                    format::coordinates(istanbul),
                ),
                ("format::decimal(istanbul)", format::decimal(istanbul)),
            ]
            .map(|(call, result)| {
                table::Row::new([label::mono(call).into(), label::mono(result).into()])
            }),
        );

        vec![
            entry(
                "Model alanı",
                "kentos_rc::spatial::ModelSpace",
                "Katmanları, ızgarayı, ölçümü ve çizimi gösteren etkileşimli alan. Durum \
                 tutmaz; kullanıcının yaptığı her şeyi bir olay olarak bildirir. Canlı \
                 örneği Giriş sekmesindedir.",
                button(label::body("Giriş sekmesinde aç"))
                    .on_press(Message::RibbonTabSelected(RibbonTab::Home))
                    .padding([4, 12])
                    .style(style::button::secondary),
                Some(
                    "ModelSpace::new(viewport, &layers, Message::ModelSpace)\n    \
                     .tool(tool)\n    .selection(selection)\n    \
                     .view_cube(ViewCube::new(rotation))",
                ),
            ),
            entry(
                "ViewCube",
                "kentos_rc::spatial::ViewCube",
                "wgpu ile çizilen yön küpü; arka planı saydamdır. Döndürme uygulamanındır; \
                 burada Giriş sekmesindekiyle aynı açıyı kullanır.",
                cubes,
                Some("ViewCube::new(rotation).size(96.0)"),
            ),
            entry(
                "Araçlar",
                "kentos_rc::spatial::Tool",
                "Her aracın adı, açıklaması ve ikonu kütüphanededir; komut satırı \
                 karşılıkları uygulamanındır.",
                tools,
                Some(
                    "ribbon::Button::large(tool.icon(), tool.label())\n    \
                     .active(current == tool)\n    \
                     .on_press(Message::ToolSelected(tool))",
                ),
            ),
            entry(
                "Nesne yakalama",
                "kentos_rc::spatial::query",
                "Nokta girişi alan araçlarda imlece en yakın köşe yakalanır. İşaretler \
                 AutoCAD'deki nesne yakalama işaretlerini izler.",
                snaps,
                Some(
                    "if let Some(snap) = query::snap(&layers, &viewport, cursor, query::SNAP_TOLERANCE) {\n    \
                     snap.kind.label() // \"Uç nokta\"\n}",
                ),
            ),
            entry(
                "Türkçe biçimlendirme",
                "kentos_rc::spatial::format",
                "Sayılar binlik ayraçla, koordinatlar enlem-boylam sırasıyla ve K/G/D/B \
                 yönleriyle yazılır.",
                formatting,
                None,
            ),
        ]
    }
}
