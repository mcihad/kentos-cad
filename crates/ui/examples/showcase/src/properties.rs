//! Katman özellikleri penceresi: bölümler ve düzenlenen taslak.
//!
//! Pencere katmanın bir taslağını düzenler; "Uygula" ve "Tamam" taslağı
//! katmana yazar, "İptal" atar. Böylece pencere açıkken harita değişmez,
//! kullanıcı değişikliklerini bir arada dener ve geri dönebilir.

use iced::Color;

use kentos_rc::icon::Icon;
use kentos_rc::spatial::Layer;

/// Pencerenin bölümleri.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    General,
    Source,
    Symbology,
    Labels,
    Fields,
}

impl Section {
    pub const ALL: [Section; 5] = [
        Section::General,
        Section::Source,
        Section::Symbology,
        Section::Labels,
        Section::Fields,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Section::General => "Genel",
            Section::Source => "Kaynak",
            Section::Symbology => "Sembolizasyon",
            Section::Labels => "Etiketler",
            Section::Fields => "Alanlar",
        }
    }

    pub fn icon(self) -> Icon {
        match self {
            Section::General => Icon::Info,
            Section::Source => Icon::Document,
            Section::Symbology => Icon::Drop,
            Section::Labels => Icon::Type,
            Section::Fields => Icon::Table,
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Section::General => "Katmanın adı, görünürlüğü ve opaklığı.",
            Section::Source => "Verinin nereden geldiği, geometrisi ve kapsamı.",
            Section::Symbology => "Katmanın haritadaki rengi ve çizgi kalınlığı.",
            Section::Labels => {
                "Öğelerin haritada hangi alanla ve hangi ölçekten itibaren adlandırılacağı."
            }
            Section::Fields => {
                "Öznitelik alanları, türleri ve kısıtları. Alanlar burada değiştirilmez."
            }
        }
    }
}

/// Katmanın düzenlenebilen özellikleri.
#[derive(Debug, Clone, PartialEq)]
pub struct Draft {
    pub name: String,
    pub visible: bool,
    pub opacity: f32,
    pub color: Color,
    pub stroke: f32,
    pub label_field: Option<usize>,
    pub label_zoom: Option<f64>,
}

impl Draft {
    /// Katmanın şimdiki özellikleri; görünürlük ağaçtaki kendi işaretidir.
    pub fn of(layer: &Layer, visible: bool) -> Self {
        Self {
            name: layer.name.clone(),
            visible,
            opacity: layer.opacity,
            color: layer.color,
            stroke: layer.stroke_width,
            label_field: layer.label_field,
            label_zoom: layer.label_zoom,
        }
    }

    /// Taslağı katmana yazar (görünürlük hariç; o ağaçtadır).
    pub fn apply(&self, layer: &mut Layer) {
        layer.name = self.name.trim().to_owned();
        layer.opacity = self.opacity;
        layer.color = self.color;
        layer.stroke_width = self.stroke;
        layer.label_field = self.label_field;
        layer.label_zoom = self.label_zoom;
    }

    /// Adın sorunu; yoksa `None`.
    pub fn problem(&self, layers: &[Layer], index: usize) -> Option<String> {
        let name = self.name.trim();

        if name.is_empty() {
            Some("Katman adı boş olamaz.".to_owned())
        } else if layers
            .iter()
            .enumerate()
            .any(|(other, layer)| other != index && layer.name == name)
        {
            Some(format!("\"{name}\" adında başka bir katman var."))
        } else {
            None
        }
    }
}

/// Taslağa yapılan değişiklik.
#[derive(Debug, Clone, PartialEq)]
pub enum Edit {
    Name(String),
    Visible(bool),
    Opacity(f32),
    Color(Color),
    Stroke(f32),
    LabelField(Option<usize>),
    LabelZoom(Option<f64>),
}

/// Açık pencerenin durumu.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerProperties {
    pub layer: usize,
    pub section: Section,
    pub draft: Draft,
    /// Pencere açıldığında ya da son uygulandığında katman.
    pub original: Draft,
}

impl LayerProperties {
    pub fn new(layer: usize, draft: Draft) -> Self {
        Self {
            layer,
            section: Section::General,
            original: draft.clone(),
            draft,
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.draft != self.original
    }

    pub fn edit(&mut self, edit: Edit) {
        let draft = &mut self.draft;

        match edit {
            Edit::Name(name) => draft.name = name,
            Edit::Visible(visible) => draft.visible = visible,
            Edit::Opacity(opacity) => draft.opacity = opacity,
            Edit::Color(color) => draft.color = color,
            Edit::Stroke(stroke) => draft.stroke = stroke,
            Edit::LabelField(field) => draft.label_field = field,
            Edit::LabelZoom(zoom) => draft.label_zoom = zoom,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drafts_track_changes_and_names() {
        let layer = Layer::points("Şehirler", Color::BLACK);
        let other = Layer::points("Yollar", Color::BLACK);
        let mut properties = LayerProperties::new(0, Draft::of(&layer, true));

        assert!(!properties.is_dirty());

        properties.edit(Edit::Opacity(0.5));
        assert!(properties.is_dirty());

        properties.edit(Edit::Name("Yollar".to_owned()));
        assert!(
            properties
                .draft
                .problem(&[layer.clone(), other], 0)
                .is_some()
        );

        properties.edit(Edit::Name("  İller ".to_owned()));
        let mut applied = layer;
        properties.draft.apply(&mut applied);
        assert_eq!(applied.name, "İller");
        assert_eq!(applied.opacity, 0.5);
    }
}
