//! İkonların vektör çizimleri.
//!
//! Her ikon 16×16'lık bir ızgarada tasarlanır ve istenen boyuta ölçeklenir.
//! Çizgi kalınlığı ölçekle birlikte büyür ama 1,2 pikselin altına inmez;
//! böylece küçük boyutlarda da okunaklı kalır.

use std::f32::consts::PI;

use iced::widget::canvas::{self, Frame, LineCap, LineJoin, Path, Stroke, Style, path};
use iced::{Color, Point, Radians, Vector};

use super::Icon;

/// İkonu çerçeveye, çerçevenin boyutunu dolduracak şekilde çizer.
pub(super) fn icon(frame: &mut Frame, icon: Icon, color: Color) {
    let size = frame.size();
    let pen = Pen {
        unit: size.width.min(size.height) / 16.0,
        color,
    };

    pen.draw(frame, icon);
}

/// 16'lık ızgara koordinatlarını piksele çeviren çizim yardımcısı.
struct Pen {
    unit: f32,
    color: Color,
}

impl Pen {
    fn p(&self, x: f32, y: f32) -> Point {
        Point::new(x * self.unit, y * self.unit)
    }

    fn stroke(&self) -> Stroke<'static> {
        Stroke {
            style: Style::Solid(self.color),
            width: (1.35 * self.unit).max(1.2),
            line_cap: LineCap::Round,
            line_join: LineJoin::Round,
            ..Stroke::default()
        }
    }

    fn polyline(&self, frame: &mut Frame, points: &[(f32, f32)], closed: bool) {
        let path = Path::new(|builder| {
            let Some(((x, y), rest)) = points.split_first() else {
                return;
            };

            builder.move_to(self.p(*x, *y));

            for (x, y) in rest {
                builder.line_to(self.p(*x, *y));
            }

            if closed {
                builder.close();
            }
        });

        frame.stroke(&path, self.stroke());
    }

    fn line(&self, frame: &mut Frame, from: (f32, f32), to: (f32, f32)) {
        self.polyline(frame, &[from, to], false);
    }

    fn circle(&self, frame: &mut Frame, center: (f32, f32), radius: f32) {
        frame.stroke(
            &Path::circle(self.p(center.0, center.1), radius * self.unit),
            self.stroke(),
        );
    }

    fn dot(&self, frame: &mut Frame, center: (f32, f32), radius: f32) {
        frame.fill(
            &Path::circle(self.p(center.0, center.1), radius * self.unit),
            self.color,
        );
    }

    fn arc(
        &self,
        builder: &mut path::Builder,
        center: (f32, f32),
        radius: f32,
        from: f32,
        to: f32,
    ) {
        builder.arc(path::Arc {
            center: self.p(center.0, center.1),
            radius: radius * self.unit,
            start_angle: Radians(from),
            end_angle: Radians(to),
        });
    }

    fn magnifier(&self, frame: &mut Frame) {
        self.circle(frame, (6.75, 6.75), 4.75);
        self.line(frame, (10.25, 10.25), (14.0, 14.0));
    }

    fn arrow_head(&self, frame: &mut Frame, tip: (f32, f32), direction: Vector) {
        let size = 2.25;
        let normal = Vector::new(-direction.y, direction.x);
        let back = (tip.0 - direction.x * size, tip.1 - direction.y * size);

        self.polyline(
            frame,
            &[
                (back.0 + normal.x * size, back.1 + normal.y * size),
                tip,
                (back.0 - normal.x * size, back.1 - normal.y * size),
            ],
            false,
        );
    }

    fn document(&self, frame: &mut Frame) {
        self.polyline(
            frame,
            &[
                (3.25, 1.75),
                (9.5, 1.75),
                (12.75, 5.0),
                (12.75, 14.25),
                (3.25, 14.25),
            ],
            true,
        );
        self.polyline(frame, &[(9.5, 1.75), (9.5, 5.0), (12.75, 5.0)], false);
    }

    fn draw(&self, frame: &mut Frame, icon: Icon) {
        match icon {
            Icon::ZoomIn => {
                self.magnifier(frame);
                self.line(frame, (4.5, 6.75), (9.0, 6.75));
                self.line(frame, (6.75, 4.5), (6.75, 9.0));
            }
            Icon::ZoomOut => {
                self.magnifier(frame);
                self.line(frame, (4.5, 6.75), (9.0, 6.75));
            }
            Icon::ZoomExtents => {
                for (corner, dx, dy) in [
                    ((2.0, 2.0), 1.0, 1.0),
                    ((14.0, 2.0), -1.0, 1.0),
                    ((2.0, 14.0), 1.0, -1.0),
                    ((14.0, 14.0), -1.0, -1.0),
                ] {
                    self.polyline(
                        frame,
                        &[
                            (corner.0, corner.1 + dy * 3.5),
                            corner,
                            (corner.0 + dx * 3.5, corner.1),
                        ],
                        false,
                    );
                }

                self.polyline(
                    frame,
                    &[(5.5, 5.5), (10.5, 5.5), (10.5, 10.5), (5.5, 10.5)],
                    true,
                );
            }
            Icon::Home => {
                self.polyline(frame, &[(1.75, 8.25), (8.0, 2.5), (14.25, 8.25)], false);
                self.polyline(
                    frame,
                    &[(3.75, 6.75), (3.75, 13.75), (12.25, 13.75), (12.25, 6.75)],
                    false,
                );
                self.polyline(
                    frame,
                    &[(6.75, 13.75), (6.75, 10.0), (9.25, 10.0), (9.25, 13.75)],
                    false,
                );
            }
            Icon::Target => {
                self.circle(frame, (8.0, 8.0), 4.75);
                self.line(frame, (8.0, 1.5), (8.0, 4.25));
                self.line(frame, (8.0, 11.75), (8.0, 14.5));
                self.line(frame, (1.5, 8.0), (4.25, 8.0));
                self.line(frame, (11.75, 8.0), (14.5, 8.0));
                self.dot(frame, (8.0, 8.0), 1.1);
            }
            Icon::Eye | Icon::EyeOff => {
                let almond = Path::new(|builder| {
                    builder.move_to(self.p(1.5, 8.0));
                    builder.quadratic_curve_to(self.p(8.0, 1.5), self.p(14.5, 8.0));
                    builder.quadratic_curve_to(self.p(8.0, 14.5), self.p(1.5, 8.0));
                });
                frame.stroke(&almond, self.stroke());
                self.circle(frame, (8.0, 8.0), 2.0);

                if icon == Icon::EyeOff {
                    self.line(frame, (2.5, 13.5), (13.5, 2.5));
                }
            }
            Icon::ClearSelection => {
                let dashed = Stroke {
                    line_dash: canvas::LineDash {
                        segments: &[2.0, 2.0],
                        offset: 0,
                    },
                    line_cap: LineCap::Butt,
                    ..self.stroke()
                };
                frame.stroke(
                    &Path::rectangle(
                        self.p(2.0, 2.0),
                        iced::Size::new(12.0 * self.unit, 12.0 * self.unit),
                    ),
                    dashed,
                );
                self.line(frame, (5.75, 5.75), (10.25, 10.25));
                self.line(frame, (10.25, 5.75), (5.75, 10.25));
            }
            Icon::Eraser => {
                self.polyline(
                    frame,
                    &[
                        (2.25, 9.75),
                        (8.75, 3.25),
                        (13.25, 7.75),
                        (7.25, 13.75),
                        (4.25, 13.75),
                    ],
                    true,
                );
                self.line(frame, (5.5, 6.5), (10.0, 11.0));
                self.line(frame, (9.0, 13.75), (14.0, 13.75));
            }
            Icon::Contrast => {
                self.circle(frame, (8.0, 8.0), 6.0);
                let half = Path::new(|builder| {
                    builder.move_to(self.p(8.0, 2.0));
                    self.arc(builder, (8.0, 8.0), 6.0, -PI / 2.0, PI / 2.0);
                    builder.close();
                });
                frame.fill(&half, self.color);
            }
            Icon::Help => {
                self.circle(frame, (8.0, 8.0), 6.25);
                let hook = Path::new(|builder| {
                    self.arc(builder, (8.0, 6.5), 2.0, PI, PI * 2.35);
                    builder.line_to(self.p(8.0, 9.4));
                });
                frame.stroke(&hook, self.stroke());
                self.dot(frame, (8.0, 11.6), 0.85);
            }
            Icon::Terminal => {
                self.polyline(
                    frame,
                    &[(1.75, 2.75), (14.25, 2.75), (14.25, 13.25), (1.75, 13.25)],
                    true,
                );
                self.polyline(frame, &[(4.5, 6.0), (6.75, 8.0), (4.5, 10.0)], false);
                self.line(frame, (8.5, 10.25), (11.5, 10.25));
            }
            Icon::Pan => {
                self.line(frame, (8.0, 1.5), (8.0, 14.5));
                self.line(frame, (1.5, 8.0), (14.5, 8.0));
                self.arrow_head(frame, (8.0, 1.5), Vector::new(0.0, -1.0));
                self.arrow_head(frame, (8.0, 14.5), Vector::new(0.0, 1.0));
                self.arrow_head(frame, (1.5, 8.0), Vector::new(-1.0, 0.0));
                self.arrow_head(frame, (14.5, 8.0), Vector::new(1.0, 0.0));
            }
            Icon::Select => {
                self.polyline(
                    frame,
                    &[
                        (3.5, 1.75),
                        (3.5, 12.75),
                        (6.4, 10.1),
                        (8.4, 14.25),
                        (10.3, 13.35),
                        (8.35, 9.3),
                        (12.25, 9.0),
                    ],
                    true,
                );
            }
            Icon::Document => self.document(frame),
            Icon::DocumentNew => {
                self.document(frame);
                self.line(frame, (5.75, 9.5), (10.25, 9.5));
                self.line(frame, (8.0, 7.25), (8.0, 11.75));
            }
            Icon::Folder => {
                self.polyline(
                    frame,
                    &[
                        (1.75, 3.25),
                        (6.0, 3.25),
                        (7.5, 4.75),
                        (14.25, 4.75),
                        (14.25, 13.0),
                        (1.75, 13.0),
                    ],
                    true,
                );
                self.line(frame, (1.75, 7.0), (14.25, 7.0));
            }
            Icon::Save => {
                self.line(frame, (8.0, 1.75), (8.0, 9.75));
                self.arrow_head(frame, (8.0, 9.75), Vector::new(0.0, 1.0));
                self.polyline(
                    frame,
                    &[(2.0, 9.25), (2.0, 14.0), (14.0, 14.0), (14.0, 9.25)],
                    false,
                );
            }
            Icon::SaveAs => {
                self.document(frame);
                self.line(frame, (8.0, 6.25), (8.0, 11.5));
                self.arrow_head(frame, (8.0, 11.5), Vector::new(0.0, 1.0));
            }
            Icon::Export => {
                self.polyline(
                    frame,
                    &[
                        (7.0, 2.5),
                        (2.5, 2.5),
                        (2.5, 13.5),
                        (13.5, 13.5),
                        (13.5, 9.0),
                    ],
                    false,
                );
                self.line(frame, (7.25, 8.75), (13.5, 2.5));
                self.polyline(frame, &[(9.25, 2.5), (13.5, 2.5), (13.5, 6.75)], false);
            }
            Icon::Print => {
                self.polyline(
                    frame,
                    &[(4.5, 5.25), (4.5, 1.75), (11.5, 1.75), (11.5, 5.25)],
                    false,
                );
                self.polyline(
                    frame,
                    &[
                        (4.5, 11.25),
                        (1.75, 11.25),
                        (1.75, 5.25),
                        (14.25, 5.25),
                        (14.25, 11.25),
                        (11.5, 11.25),
                    ],
                    false,
                );
                self.polyline(
                    frame,
                    &[(4.5, 9.0), (11.5, 9.0), (11.5, 14.25), (4.5, 14.25)],
                    true,
                );
            }
            Icon::Power => {
                let ring = Path::new(|builder| {
                    self.arc(builder, (8.0, 8.75), 5.5, -PI / 2.0 + 0.7, PI * 1.5 - 0.7);
                });
                frame.stroke(&ring, self.stroke());
                self.line(frame, (8.0, 1.5), (8.0, 7.75));
            }
            Icon::ChevronDown => {
                self.polyline(frame, &[(4.0, 6.0), (8.0, 10.0), (12.0, 6.0)], false);
            }
            Icon::ChevronRight => {
                self.polyline(frame, &[(6.0, 4.0), (10.0, 8.0), (6.0, 12.0)], false);
            }
            Icon::Line => {
                self.line(frame, (3.0, 13.0), (13.0, 3.0));
                self.dot(frame, (3.0, 13.0), 1.6);
                self.dot(frame, (13.0, 3.0), 1.6);
            }
            Icon::Polyline => {
                let vertices = [(1.75, 12.5), (5.5, 4.5), (10.0, 10.5), (14.25, 3.25)];
                self.polyline(frame, &vertices, false);

                for vertex in vertices {
                    self.dot(frame, vertex, 1.3);
                }
            }
            Icon::Polygon => {
                self.polyline(
                    frame,
                    &[
                        (8.0, 1.75),
                        (14.25, 6.25),
                        (11.75, 14.0),
                        (4.25, 14.0),
                        (1.75, 6.25),
                    ],
                    true,
                );
            }
            Icon::Rectangle => {
                self.polyline(
                    frame,
                    &[(1.75, 3.75), (14.25, 3.75), (14.25, 12.25), (1.75, 12.25)],
                    true,
                );
            }
            Icon::Circle => {
                self.circle(frame, (8.0, 8.0), 6.25);
                self.dot(frame, (8.0, 8.0), 1.1);
            }
            Icon::Point => {
                self.line(frame, (3.5, 3.5), (12.5, 12.5));
                self.line(frame, (12.5, 3.5), (3.5, 12.5));
                self.dot(frame, (8.0, 8.0), 1.9);
            }
            Icon::Measure => {
                self.line(frame, (1.75, 4.0), (1.75, 12.0));
                self.line(frame, (14.25, 4.0), (14.25, 12.0));
                self.line(frame, (2.0, 8.0), (14.0, 8.0));
                self.arrow_head(frame, (2.0, 8.0), Vector::new(-1.0, 0.0));
                self.arrow_head(frame, (14.0, 8.0), Vector::new(1.0, 0.0));
            }
        }
    }
}
