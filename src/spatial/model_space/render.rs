//! Model alanının çizim katmanları, alttan üste: ızgara, öğeler, etiketler,
//! ölçüm, çizim önizlemesi, artı imleç, UCS simgesi ve ölçek çubuğu.

use iced::alignment::Vertical;
use iced::widget::canvas::{Frame, LineCap, LineDash, LineJoin, Path, Stroke, Text};
use iced::widget::text::Alignment;
use iced::{Color, Point, Size, Vector};

use super::Style;
use super::program::Program;
use crate::spatial::query::SnapKind;
use crate::spatial::{Feature, Geometry, Layer, LayerKind, LonLat, Tool, format, measure};
use crate::theme::typography::MONO;

/// Seç aracında artı imlecin ortasındaki seçim kutusunun yarı boyu.
const PICKBOX: f32 = 5.0;
/// Kısa artı imleç kollarının uzunluğu.
const CROSSHAIR_ARM: f32 = 36.0;
/// Seçili öğede tutamaç çizilecek en fazla köşe sayısı.
const MAX_GRIPS: usize = 400;
/// Ölçüm ve önizleme çizgilerinin kesik çizgi deseni.
const DASH: [f32; 2] = [7.0, 6.0];

/// Bilgi kutularının yüksekliği ve harf başına yaklaşık genişliği.
const TAG_HEIGHT: f32 = 18.0;
const TAG_CHAR_WIDTH: f32 = 6.6;

fn stroke(color: Color, width: f32) -> Stroke<'static> {
    Stroke {
        style: color.into(),
        width,
        ..Stroke::default()
    }
}

fn dashed(color: Color, width: f32) -> Stroke<'static> {
    Stroke {
        line_dash: LineDash {
            segments: &DASH,
            offset: 0,
        },
        ..stroke(color, width)
    }
}

fn polyline(points: &[Point], closed: bool) -> Path {
    Path::new(|builder| {
        let Some((first, rest)) = points.split_first() else {
            return;
        };

        builder.move_to(*first);

        for point in rest {
            builder.line_to(*point);
        }

        if closed {
            builder.close();
        }
    })
}

fn tag_width(content: &str) -> f32 {
    content.chars().count() as f32 * TAG_CHAR_WIDTH + 12.0
}

impl<Message> Program<'_, Message> {
    fn project_all(&self, points: &[LonLat]) -> Vec<Point> {
        points
            .iter()
            .map(|location| self.viewport.project(*location))
            .collect()
    }

    // --- Izgara ---------------------------------------------------------

    /// Enlem-boylam ızgarası; her beşinci çizgi daha belirgindir. Boylam
    /// etiketleri üst kenarda, enlem etiketleri sol kenarda yer alır.
    pub(super) fn draw_grid(&self, frame: &mut Frame, style: &Style) {
        let size = frame.size();
        let visible = self.viewport.visible_bounds();
        let step = self.viewport.graticule_step();
        let is_major = |value: f64| (value / (step * 5.0)).fract().abs() < 1e-6;
        let line = |major: bool| {
            if major {
                stroke(style.grid_major, 1.2)
            } else {
                stroke(style.grid, 0.8)
            }
        };
        let label = |content: String, position: Point, align_y: Vertical| Text {
            content,
            position,
            color: style.grid_label,
            size: 10.0.into(),
            font: MONO,
            align_y,
            ..Text::default()
        };

        let mut lon = (visible.south_west.lon / step).floor() * step;

        while lon <= visible.north_east.lon + step {
            let top = self
                .viewport
                .project(LonLat::new(lon, visible.north_east.lat));
            let bottom = self
                .viewport
                .project(LonLat::new(lon, visible.south_west.lat));

            frame.stroke(&Path::line(top, bottom), line(is_major(lon)));

            if lon.abs() > 1e-6 && top.x > 36.0 && top.x < size.width - 12.0 {
                frame.fill_text(label(
                    format!("{}\u{b0}", format::pretty(lon)),
                    Point::new(top.x + 4.0, 6.0),
                    Vertical::Top,
                ));
            }

            lon += step;
        }

        let mut lat = (visible.south_west.lat / step).floor() * step;

        while lat <= visible.north_east.lat + step {
            let left = self
                .viewport
                .project(LonLat::new(visible.south_west.lon, lat));
            let right = self
                .viewport
                .project(LonLat::new(visible.north_east.lon, lat));

            frame.stroke(&Path::line(left, right), line(is_major(lat)));

            // Sol alttaki UCS simgesinin üstünde kalır.
            if lat.abs() > 1e-6 && left.y > 16.0 && left.y < size.height - 80.0 {
                frame.fill_text(label(
                    format!("{}\u{b0}", format::pretty(lat)),
                    Point::new(6.0, left.y - 4.0),
                    Vertical::Bottom,
                ));
            }

            lat += step;
        }
    }

    // --- Öğeler ---------------------------------------------------------

    /// Katmanları listenin sonundan başına çizer; böylece listede önce gelen
    /// katman üstte görünür (seçimle aynı öncelik).
    pub(super) fn draw_layers(&self, frame: &mut Frame, style: &Style) {
        for (layer_index, layer) in self.layers.iter().enumerate().rev() {
            if !layer.visible || layer.opacity <= 0.02 {
                continue;
            }

            for (feature_index, feature) in layer.features.iter().enumerate() {
                let reference = Some(crate::spatial::FeatureRef::new(layer_index, feature_index));

                self.draw_feature(
                    frame,
                    style,
                    layer,
                    feature,
                    self.selection == reference,
                    self.hover == reference,
                );
            }
        }
    }

    fn draw_feature(
        &self,
        frame: &mut Frame,
        style: &Style,
        layer: &Layer,
        feature: &Feature,
        selected: bool,
        hovered: bool,
    ) {
        let opacity = layer.opacity;
        let color = style.layer_color(layer.color);

        match &feature.geometry {
            Geometry::Point(location) => {
                let center = self.viewport.project(*location);
                let radius = match (selected, hovered) {
                    (true, _) => 7.5,
                    (false, true) => 6.5,
                    (false, false) => 5.0,
                };

                let body = Path::circle(center, radius);
                frame.fill(&body, color.scale_alpha(opacity));
                frame.stroke(
                    &body,
                    if selected {
                        stroke(style.selection, 2.4)
                    } else {
                        stroke(style.symbol_outline.scale_alpha(opacity), 1.4)
                    },
                );

                if selected {
                    self.draw_grips(frame, style, &[center]);
                }
            }
            Geometry::Line(points) => {
                let screen = self.project_all(points);

                if screen.len() < 2 {
                    return;
                }

                let width = match (selected, hovered) {
                    (true, _) => layer.stroke_width + 2.0,
                    (false, true) => layer.stroke_width + 1.0,
                    (false, false) => layer.stroke_width,
                };

                frame.stroke(
                    &polyline(&screen, false),
                    Stroke {
                        line_cap: LineCap::Round,
                        line_join: LineJoin::Round,
                        ..stroke(
                            if selected {
                                style.selection
                            } else {
                                color.scale_alpha(opacity)
                            },
                            width,
                        )
                    },
                );

                if selected {
                    self.draw_grips(frame, style, &screen);
                }
            }
            Geometry::Polygon(points) => {
                let screen = self.project_all(points);

                if screen.len() < 3 {
                    return;
                }

                let path = polyline(&screen, true);
                let fill_alpha = match (selected, hovered) {
                    (true, _) => (layer.fill_alpha + 0.25).min(0.8),
                    (false, true) => (layer.fill_alpha + 0.10).min(0.8),
                    (false, false) => layer.fill_alpha,
                };

                frame.fill(&path, color.scale_alpha(opacity * fill_alpha));
                frame.stroke(
                    &path,
                    Stroke {
                        line_join: LineJoin::Round,
                        ..if selected {
                            stroke(style.selection, 2.4)
                        } else {
                            stroke(color.scale_alpha(opacity * 0.95), layer.stroke_width)
                        }
                    },
                );

                if selected {
                    self.draw_grips(frame, style, &screen);
                }
            }
        }
    }

    /// Seçili öğenin köşelerine AutoCAD tarzı kare tutamaçlar.
    fn draw_grips(&self, frame: &mut Frame, style: &Style, points: &[Point]) {
        for point in points.iter().take(MAX_GRIPS) {
            let grip = Path::rectangle(*point - Vector::new(3.5, 3.5), Size::new(7.0, 7.0));

            frame.fill(&grip, style.grip);
            frame.stroke(&grip, stroke(style.symbol_outline, 1.0));
        }
    }

    /// Nokta katmanlarının etiketleri; katmanın etiket yakınlaştırmasından
    /// itibaren gösterilir.
    pub(super) fn draw_labels(&self, frame: &mut Frame, style: &Style) {
        let labelled = self.layers.iter().filter(|layer| {
            layer.visible
                && layer.kind == LayerKind::Point
                && layer
                    .label_zoom
                    .is_some_and(|zoom| self.viewport.zoom >= zoom)
        });

        for layer in labelled {
            for feature in &layer.features {
                let Geometry::Point(location) = &feature.geometry else {
                    continue;
                };

                frame.fill_text(Text {
                    content: feature.name.clone(),
                    position: self.viewport.project(*location) + Vector::new(9.0, -9.0),
                    color: style.label.scale_alpha(layer.opacity.max(0.65)),
                    size: 12.0.into(),
                    align_x: Alignment::Left,
                    align_y: Vertical::Bottom,
                    ..Text::default()
                });
            }
        }
    }

    // --- Ölçüm ve çizim --------------------------------------------------

    /// Ölçüm noktaları, kenarlar, kenar uzunlukları ve imlece uzanan
    /// önizleme.
    pub(super) fn draw_measurement(
        &self,
        frame: &mut Frame,
        style: &Style,
        pointer: Option<Point>,
    ) {
        let points = self.project_all(self.measurement);

        if points.len() >= 2 {
            frame.stroke(
                &polyline(&points, false),
                Stroke {
                    line_cap: LineCap::Round,
                    line_join: LineJoin::Round,
                    ..dashed(style.measure, 2.0)
                },
            );

            for (pair, screen) in self.measurement.windows(2).zip(points.windows(2)) {
                let content = format::distance(measure::haversine_meters(pair[0], pair[1]));
                let middle = Point::new(
                    (screen[0].x + screen[1].x) * 0.5,
                    (screen[0].y + screen[1].y) * 0.5,
                );
                let anchor = middle - Vector::new(tag_width(&content) / 2.0, 24.0);

                self.draw_tag(frame, style, content, anchor, style.measure);
            }
        }

        for point in &points {
            let marker = Path::circle(*point, 4.0);
            frame.fill(&marker, style.measure);
            frame.stroke(&marker, stroke(style.symbol_outline, 1.4));
        }

        if let (Some(last), Some(pointer)) = (points.last(), pointer)
            && self.tool == Tool::Measure
        {
            frame.stroke(
                &Path::line(*last, pointer),
                dashed(style.measure.scale_alpha(0.45), 1.4),
            );
        }
    }

    /// Yapım aşamasındaki geometri: tıklanan noktalar ve imlece uzanan
    /// lastik bant önizlemesi.
    pub(super) fn draw_draft(&self, frame: &mut Frame, style: &Style, pointer: Option<Point>) {
        if !self.tool.is_drawing() || self.draft.is_empty() {
            return;
        }

        let color = style.layer_color(self.draft_color.unwrap_or(style.selection));
        let rubber = dashed(color.scale_alpha(0.8), 1.2);
        let points = self.project_all(self.draft);

        match (self.tool, pointer) {
            (Tool::Rectangle, Some(corner)) => {
                let origin = points[0];
                let rectangle = Path::rectangle(
                    Point::new(origin.x.min(corner.x), origin.y.min(corner.y)),
                    Size::new((corner.x - origin.x).abs(), (corner.y - origin.y).abs()),
                );

                frame.fill(&rectangle, color.scale_alpha(0.12));
                frame.stroke(&rectangle, rubber);
            }
            (Tool::Circle, Some(edge)) => {
                let center = points[0];
                let circle = Path::circle(center, center.distance(edge));

                frame.fill(&circle, color.scale_alpha(0.12));
                frame.stroke(&circle, rubber);
                frame.stroke(&Path::line(center, edge), rubber);
            }
            (Tool::Rectangle | Tool::Circle, None) => {}
            (tool, pointer) => {
                if points.len() >= 2 {
                    frame.stroke(
                        &polyline(&points, false),
                        Stroke {
                            line_cap: LineCap::Round,
                            line_join: LineJoin::Round,
                            ..stroke(color, 1.6)
                        },
                    );
                }

                if let (Some(pointer), Some(last)) = (pointer, points.last()) {
                    frame.stroke(&Path::line(*last, pointer), rubber);

                    if tool == Tool::Polygon && points.len() >= 2 {
                        frame.stroke(&Path::line(pointer, points[0]), rubber);

                        let mut ring = points.clone();
                        ring.push(pointer);
                        frame.fill(&polyline(&ring, true), color.scale_alpha(0.12));
                    }
                }
            }
        }

        for point in &points {
            let marker = Path::rectangle(*point - Vector::new(3.0, 3.0), Size::new(6.0, 6.0));

            frame.fill(&marker, style.background);
            frame.stroke(&marker, stroke(color, 1.2));
        }
    }

    // --- İmleç ----------------------------------------------------------

    /// Artı imleç, seçim kutusu, yakalama işareti ve dinamik giriş kutuları.
    pub(super) fn draw_crosshair(&self, frame: &mut Frame, style: &Style, pointer: Point) {
        let size = frame.size();

        let (color, reach) = if self.options.full_crosshair {
            (style.crosshair, f32::INFINITY)
        } else {
            (style.crosshair_small, CROSSHAIR_ARM)
        };

        // Seçim kutusu yalnızca Seç aracında vardır; diğer araçlarda
        // çizgiler kesintisizdir.
        let gap = if self.tool == Tool::Select {
            PICKBOX
        } else {
            0.0
        };

        let arms = [
            (
                Point::new((pointer.x - reach).max(0.0), pointer.y),
                Point::new(pointer.x - gap, pointer.y),
            ),
            (
                Point::new(pointer.x + gap, pointer.y),
                Point::new((pointer.x + reach).min(size.width), pointer.y),
            ),
            (
                Point::new(pointer.x, (pointer.y - reach).max(0.0)),
                Point::new(pointer.x, pointer.y - gap),
            ),
            (
                Point::new(pointer.x, pointer.y + gap),
                Point::new(pointer.x, (pointer.y + reach).min(size.height)),
            ),
        ];

        for (from, to) in arms {
            frame.stroke(&Path::line(from, to), stroke(color, 1.0));
        }

        if gap > 0.0 {
            frame.stroke_rectangle(
                pointer - Vector::new(gap, gap),
                Size::new(gap * 2.0, gap * 2.0),
                stroke(style.crosshair_small, 1.0),
            );
        }

        if let Some(snap) = self.snap_at(pointer) {
            let point = self.viewport.project(snap.location);

            self.draw_snap_marker(frame, style, point, snap.kind);
            self.draw_tag(
                frame,
                style,
                snap.kind.label().to_owned(),
                point + Vector::new(12.0, -30.0),
                style.snap,
            );
        }

        // Dinamik giriş: imlecin koordinatı ve nokta girişi sürerken son
        // noktaya olan mesafe (dairede yarıçap).
        let location = self.viewport.unproject(pointer);
        let mut tags = vec![format::decimal(location)];

        let anchor = match self.tool {
            Tool::Measure => self.measurement.last(),
            tool if tool.is_drawing() => self.draft.last(),
            _ => None,
        };

        if let Some(last) = anchor {
            let prefix = if self.tool == Tool::Circle {
                "R"
            } else {
                "\u{0394}"
            };

            tags.push(format!(
                "{prefix} {}",
                format::distance(measure::haversine_meters(*last, location))
            ));
        }

        let x = (pointer.x + 18.0).min(size.width - 170.0);
        let mut y = (pointer.y + 18.0).min(size.height - 22.0 * tags.len() as f32 - 6.0);

        for tag in tags {
            self.draw_tag(frame, style, tag, Point::new(x, y), style.tag_text);
            y += 22.0;
        }
    }

    /// AutoCAD nesne yakalama işaretleri: uç noktada kare, köşede baklava,
    /// nokta öğede içi çarpılı daire.
    fn draw_snap_marker(&self, frame: &mut Frame, style: &Style, point: Point, kind: SnapKind) {
        let marker = stroke(style.snap, 2.0);
        let size = 6.0;

        match kind {
            SnapKind::Endpoint => {
                frame.stroke(
                    &Path::rectangle(
                        point - Vector::new(size, size),
                        Size::new(size * 2.0, size * 2.0),
                    ),
                    marker,
                );
            }
            SnapKind::Vertex => {
                let reach = size + 1.0;

                frame.stroke(
                    &polyline(
                        &[
                            point + Vector::new(0.0, -reach),
                            point + Vector::new(reach, 0.0),
                            point + Vector::new(0.0, reach),
                            point + Vector::new(-reach, 0.0),
                        ],
                        true,
                    ),
                    marker,
                );
            }
            SnapKind::Node => {
                frame.stroke(&Path::circle(point, size), marker);

                let arm = size * 0.7;

                for (from, to) in [
                    (Vector::new(-arm, -arm), Vector::new(arm, arm)),
                    (Vector::new(-arm, arm), Vector::new(arm, -arm)),
                ] {
                    frame.stroke(&Path::line(point + from, point + to), marker);
                }
            }
        }
    }

    /// Zeminli, kenarlı küçük bilgi kutusu; `anchor` sol üst köşedir.
    fn draw_tag(
        &self,
        frame: &mut Frame,
        style: &Style,
        content: String,
        anchor: Point,
        color: Color,
    ) {
        let size = Size::new(tag_width(&content), TAG_HEIGHT);

        frame.fill_rectangle(anchor, size, style.tag_background);
        frame.stroke_rectangle(anchor, size, stroke(color.scale_alpha(0.6), 1.0));
        frame.fill_text(Text {
            content,
            position: Point::new(anchor.x + 6.0, anchor.y + TAG_HEIGHT / 2.0),
            color,
            size: 11.0.into(),
            font: MONO,
            align_y: Vertical::Center,
            ..Text::default()
        });
    }

    // --- Kenar süsleri --------------------------------------------------

    /// Sol alttaki kullanıcı koordinat sistemi (UCS) simgesi.
    pub(super) fn draw_ucs(&self, frame: &mut Frame, style: &Style) {
        let origin = Point::new(22.0, frame.size().height - 22.0);
        let length = 40.0;

        for (direction, color, name) in [
            (Vector::new(1.0, 0.0), style.axis_x, "X"),
            (Vector::new(0.0, -1.0), style.axis_y, "Y"),
        ] {
            let tip = origin + direction * length;
            let normal = Vector::new(-direction.y, direction.x);
            let base = origin + direction * (length - 9.0);

            frame.stroke(&Path::line(origin, base), stroke(color, 2.0));
            frame.fill(
                &polyline(&[tip, base + normal * 4.0, base - normal * 4.0], true),
                color,
            );
            frame.fill_text(Text {
                content: name.to_owned(),
                position: tip + direction * 8.0,
                color,
                size: 11.0.into(),
                font: MONO,
                align_x: Alignment::Center,
                align_y: Vertical::Center,
                ..Text::default()
            });
        }

        frame.stroke_rectangle(
            origin - Vector::new(4.0, 4.0),
            Size::new(8.0, 8.0),
            stroke(style.grid_label, 1.0),
        );
    }

    /// İki bölmeli (dolu/boş) kartografik ölçek çubuğu.
    pub(super) fn draw_scale_bar(&self, frame: &mut Frame, style: &Style) {
        let (meters, label) = self.viewport.scale_bar();
        let width = (meters / self.viewport.meters_per_pixel()) as f32;
        let origin = Point::new(96.0, frame.size().height - 24.0);
        let height = 5.0;

        frame.fill_rectangle(origin, Size::new(width / 2.0, height), style.label);
        frame.stroke_rectangle(origin, Size::new(width, height), stroke(style.label, 1.0));

        for (x, content) in [(0.0, "0".to_owned()), (width, label)] {
            frame.fill_text(Text {
                content,
                position: Point::new(origin.x + x, origin.y - 4.0),
                color: style.label,
                size: 10.5.into(),
                font: MONO,
                align_x: Alignment::Center,
                align_y: Vertical::Bottom,
                ..Text::default()
            });
        }
    }
}
