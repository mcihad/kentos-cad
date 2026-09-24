//! Model alanı araçları.

use crate::icon::Icon;

/// Model alanında imlecin ne yaptığını belirleyen araç.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tool {
    /// Seçim; varsayılan araç.
    Select,
    /// Sürükleyerek gezinme.
    Pan,
    /// Noktalar arası mesafe ölçümü.
    Measure,
    Line,
    Polyline,
    Polygon,
    Rectangle,
    Circle,
    Point,
}

impl Tool {
    /// Gezinme ve ölçüm araçları.
    pub const NAVIGATION: [Tool; 3] = [Tool::Select, Tool::Pan, Tool::Measure];

    /// Model alanına geometri ekleyen araçlar.
    pub const DRAWING: [Tool; 6] = [
        Tool::Line,
        Tool::Polyline,
        Tool::Polygon,
        Tool::Rectangle,
        Tool::Circle,
        Tool::Point,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Tool::Select => "Seç",
            Tool::Pan => "Kaydır",
            Tool::Measure => "Ölç",
            Tool::Line => "Çizgi",
            Tool::Polyline => "Çoklu çizgi",
            Tool::Polygon => "Alan",
            Tool::Rectangle => "Dikdörtgen",
            Tool::Circle => "Daire",
            Tool::Point => "Nokta",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Tool::Select => "Tıklanan öğenin özelliklerini gösterir.",
            Tool::Pan => "Sürükleyerek gezinir, tekerlek yakınlaştırır.",
            Tool::Measure => "Tıklanan noktalar arası mesafeyi ölçer. Sağ tık temizler.",
            Tool::Line => "Art arda doğru parçaları çizer. Sağ tık veya Esc bitirir.",
            Tool::Polyline => "Tek parça çoklu çizgi çizer. Sağ tık veya Esc bitirir.",
            Tool::Polygon => "Kapalı alan çizer. Sağ tık veya Esc alanı kapatır.",
            Tool::Rectangle => "İki köşe tıklayarak dikdörtgen çizer.",
            Tool::Circle => "Merkezi, sonra yarıçap noktasını tıklayarak daire çizer.",
            Tool::Point => "Tıklanan konuma nokta koyar.",
        }
    }

    pub fn icon(self) -> Icon {
        match self {
            Tool::Select => Icon::Select,
            Tool::Pan => Icon::Pan,
            Tool::Measure => Icon::Measure,
            Tool::Line => Icon::Line,
            Tool::Polyline => Icon::Polyline,
            Tool::Polygon => Icon::Polygon,
            Tool::Rectangle => Icon::Rectangle,
            Tool::Circle => Icon::Circle,
            Tool::Point => Icon::Point,
        }
    }

    /// Model alanına geometri ekler mi.
    pub fn is_drawing(self) -> bool {
        Self::DRAWING.contains(&self)
    }

    /// Tıklamayı bir nokta girişi olarak mı alır (ölçüm ve çizim). Bu
    /// araçlarda nesne yakalama uygulanır.
    pub fn takes_points(self) -> bool {
        self == Tool::Measure || self.is_drawing()
    }
}
