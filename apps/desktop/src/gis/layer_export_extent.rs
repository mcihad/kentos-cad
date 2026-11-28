//! Farklı kaydet…'s Kapsam (docs/adr/9112 §2): every object of the layer, those in the view, or those touching a box.
//! The box is its four sides in the project's coordinates, named as the project names its axes (a CBS project's Y east
//! and X north, a CAD project's X and Y): typed, taken from the view or from the layer's extent, or drawn on the map.
//! While it is drawn the window steps aside; two corners are clicked (snaps apply) and the window comes back with the
//! box, or as it was on Esc.

use iced::widget::{column, container, row};
use iced::{Element, Length};
use kentos_interaction::Level;
use kentos_interaction::capture::{Capture, Kind, Shape};

use crate::app::{App, Message};

/// Which objects the box keeps.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Extent {
    #[default]
    All,
    View,
    Box,
}

pub(crate) const EXTENTS: [Extent; 3] = [Extent::All, Extent::View, Extent::Box];

pub(crate) fn extent_label(e: Extent) -> &'static str {
    match e {
        Extent::All => "Bütün katman",
        Extent::View => "Görünümdekiler",
        Extent::Box => "Kutuya değenler",
    }
}

/// The box's sides as typed: west, south, east, north.
pub(crate) type Sides = [String; 4];

/// What the window says when the box is not four numbers around a place.
pub(crate) const BAD_BOX: &str = "Kutunun dört kenarını sayıyla yazın: batı doğudan, güney kuzeyden küçük olmalı. Görünümden alabilir ya da haritada çizebilirsiniz.";

fn number(text: &str) -> Option<f64> {
    let t = text.trim();
    // A Turkish decimal comma reads as a point when the text has no point.
    let t = if t.contains('.') {
        t.to_owned()
    } else {
        t.replace(',', ".")
    };
    t.parse::<f64>().ok().filter(|v| v.is_finite())
}

/// The box `[min x, min y, max x, max y]` the sides make; none unless all four read and the box has room.
pub(crate) fn box_of(sides: &Sides) -> Option<[f64; 4]> {
    let [w, s, e, n] = [
        number(&sides[0])?,
        number(&sides[1])?,
        number(&sides[2])?,
        number(&sides[3])?,
    ];
    (w < e && s < n).then_some([w, s, e, n])
}

/// The sides of a box, written to the project's decimals.
pub(crate) fn sides_of(b: [f64; 4], decimals: usize) -> Sides {
    [b[0], b[1], b[2], b[3]].map(|v| format!("{v:.decimals$}"))
}

impl App {
    fn export_decimals(&self) -> usize {
        self.document
            .as_ref()
            .map_or(3, |d| d.model.settings().length_decimals as usize)
    }

    /// Görünümden al: the view's box.
    pub(crate) fn export_box_from_view(&mut self) {
        let b = self.viewport.camera.visible_bounds();
        let sides = sides_of([b.min_x, b.min_y, b.max_x, b.max_y], self.export_decimals());
        if let Some(w) = self.gis.layer_export.as_mut() {
            w.set_box(sides);
        }
    }

    /// Katmanın kapsamı: a data layer's table box (as last read, moved into the project's system), a drawing layer's
    /// objects' box; said when it has none.
    pub(crate) fn export_box_from_layer(&mut self) {
        let Some(layer) = self
            .gis
            .layer_export
            .as_ref()
            .map(|w| w.layer_id().to_owned())
        else {
            return;
        };
        let Some(doc) = &self.document else { return };
        let Some(node) = doc.model.layers().get(&layer) else {
            return;
        };
        let b = if node.data.is_some() {
            crate::data_layers::extent_in_project(node, doc.settings())
        } else {
            let ids: Vec<f64> = doc
                .model
                .entities()
                .filter(|e| e.base().layer_id == layer)
                .map(|e| f64::from(e.base().id))
                .collect();
            (!ids.is_empty())
                .then(|| self.spatial.store().extent(Some(&ids)))
                .flatten()
                .map(|b| [b.min_x, b.min_y, b.max_x, b.max_y])
        };
        match b {
            Some(b) => {
                let sides = sides_of(b, self.export_decimals());
                if let Some(w) = self.gis.layer_export.as_mut() {
                    w.set_box(sides);
                }
            }
            None => self.say(Level::Warn, "Katmanın bilinen bir kapsamı yok; kutuyu yazın, görünümden alın ya da haritada çizin."),
        }
    }

    /// Haritada çiz: the window steps aside and the box's two corners are clicked on the map.
    pub(crate) fn export_draw_box(&mut self) {
        let Some(w) = self.gis.layer_export.as_mut() else {
            return;
        };
        w.set_drawing(true);
        self.dialog = None;
        self.say(
            Level::Command,
            "Dışa aktarma kapsamı: kutunun iki köşesini belirtin (Esc pencereye döner).",
        );
        self.session.run(Box::new(Capture::new(
            "Dışa aktarma kapsamı",
            Kind::Rectangle,
        )));
        self.with_tool(|s, cx| s.activate(cx));
    }

    /// A shape drawn while the window waited for its box: the box's sides from its corners, the window back (as it was
    /// on Esc). False when the window did not wait for one.
    pub(crate) fn layer_export_box_drawn(&mut self, shape: &Option<Shape>) -> bool {
        let decimals = self.export_decimals();
        let Some(w) = self.gis.layer_export.as_mut().filter(|w| w.is_drawing()) else {
            return false;
        };
        w.set_drawing(false);
        if let Some(Shape::Polygon(corners)) = shape
            && !corners.is_empty()
        {
            let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
            for p in corners {
                lo = [lo[0].min(p.x), lo[1].min(p.y)];
                hi = [hi[0].max(p.x), hi[1].max(p.y)];
            }
            w.set_box(sides_of([lo[0], lo[1], hi[0], hi[1]], decimals));
        }
        self.dialog = Some(crate::app::Dialog::Gis(
            crate::gis::GisDialogKind::LayerExport,
        ));
        true
    }

    /// The project's names of its east and north axes, as its coordinates are named everywhere: a CAD project's X and
    /// Y, any other's Y (east) and X (north), the surveyor's.
    pub(crate) fn export_axis_names(&self) -> (&'static str, &'static str) {
        let cad = self.document.as_ref().is_some_and(|d| {
            d.model.settings().project_type() == Some(kentos_commands::Workspace::Cad)
        });
        if cad { ("X", "Y") } else { ("Y", "X") }
    }
}

/// The box's four sides laid out as a map reads them: north above, west and east beside, south below; the ways to take
/// a box under them.
pub(crate) fn sides_view<'a>(
    sides: &Sides,
    (east, north): (&'static str, &'static str),
    on_side: impl Fn(usize, String) -> Message + Copy + 'a,
    from_view: Message,
    from_layer: Message,
    draw: Message,
) -> Element<'a, Message> {
    let side = |i: usize, label: String| -> Element<'a, Message> {
        row![
            container(kentos_ui::label::caption(label)).width(80),
            container(
                kentos_ui::widget::text_field("", &sides[i]).on_input(move |t| on_side(i, t))
            )
            .width(150),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .into()
    };
    let middle = |e: Element<'a, Message>| {
        container(e)
            .width(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center)
    };
    column![
        middle(side(3, format!("Kuzey ({north})"))),
        row![
            side(0, format!("Batı ({east})")),
            iced::widget::space::horizontal(),
            side(2, format!("Doğu ({east})"))
        ]
        .align_y(iced::Alignment::Center),
        middle(side(1, format!("Güney ({north})"))),
        row![
            crate::exchange::words::secondary("Görünümden al", Some(from_view)),
            crate::exchange::words::secondary("Katmanın kapsamı", Some(from_layer)),
            crate::exchange::words::secondary("Haritada çiz", Some(draw)),
        ]
        .spacing(8),
    ]
    .spacing(8)
    .into()
}
