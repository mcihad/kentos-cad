//! Transient Etiket controls; the existing semantic create command still plans and writes.
use super::*;
use crate::tool_settings::choices;
use kentos_commands::{TAG_SIZE_MM, tag_template};
use kentos_geometry_core::tools::point_text::parse_number;

// Suggestions only: the chosen text still goes through cad.tags.create's parser.
// No preset id or display value is stored in the document.
const CONTENTS: [(&str, &str, &str); 9] = [
    ("number", "Numara", "{number}"),
    ("name", "Ad", "{name}"),
    ("net", "Net alan", "{netArea}"),
    ("room", "Mahal kimliği", "{number}\n{name}\n{netArea}"),
    (
        "room-levels",
        "Mahal ve kotlar",
        "{number}\n{name}\nNet: {netArea}\nDöşeme: {floorElevation}\nTavan: {ceilingElevation}",
    ),
    // The building program (docs/adr/0773): the space's group, its design target and how far it is out.
    ("program", "Bölüm ve hedef", "{number} · {unit}\n{area}\nHedef: {target}\nFark: {targetDiff}"),
    ("floor", "Döşeme kotu", "{floorElevation}"),
    ("ceiling", "Tavan kotu", "{ceilingElevation}"),
    (
        "opening",
        "Açıklık ve boyut",
        "{number}\n{width} × {height}",
    ),
];

pub(super) fn content_label(content: Option<&str>, bulk: bool) -> &'static str {
    if content.is_none() {
        return if bulk { "Tür / Koru" } else { "Türe göre" };
    }
    CONTENTS
        .iter()
        .find(|(_, _, text)| Some(*text) == content)
        .map_or("Özel", |(_, label, _)| *label)
}

pub(super) fn content_choices(content: Option<&str>, bulk: bool) -> Vec<OptionChoice> {
    let selected = content.map_or("default", |text| {
        CONTENTS
            .iter()
            .find(|(_, _, value)| *value == text)
            .map_or("custom", |(id, _, _)| *id)
    });
    let mut items = vec![("default", if bulk { "Tür / Koru" } else { "Türe göre" })];
    items.extend(CONTENTS.iter().map(|(id, label, _)| (*id, *label)));
    if selected == "custom" {
        items.push(("custom", "Özel şablonu koru"));
    }
    choices(&items, selected)
}

pub(super) fn content_setting(content: Option<&str>, bulk: bool) -> Setting {
    let mut setting = Setting::choice("İ", "Hazır içerik", content_choices(content, bulk));
    setting.value = content_label(content, bulk).into();
    setting
}

pub(super) fn choose_content(content: &mut Option<String>, value: &str) -> bool {
    if value == "default" {
        *content = None;
    } else if let Some((_, _, text)) = CONTENTS.iter().find(|(id, _, _)| *id == value) {
        *content = Some((*text).into());
    } else {
        return value == "custom" && content.is_some();
    }
    true
}

pub(super) fn next_content(content: &mut Option<String>) {
    let next = content
        .as_deref()
        .and_then(|text| CONTENTS.iter().position(|(_, _, value)| *value == text))
        .map_or(0, |index| index + 1);
    *content = CONTENTS.get(next).map(|(_, _, text)| (*text).into());
}

impl TagTool {
    pub(super) fn tag_choices(&self, key: &str) -> Vec<OptionChoice> {
        match key {
            "İ" => content_choices(self.content.as_deref(), false),
            "B" => {
                let mut items = vec![("default", "Türe göre")];
                items.extend(TagShape::ALL.iter().map(|s| (s.name(), s.label())));
                choices(&items, self.shape.map_or("default", TagShape::name))
            }
            "K" => choices(
                &[("yes", "Var"), ("no", "Yok")],
                if self.leader { "yes" } else { "no" },
            ),
            _ => vec![],
        }
    }

    pub(super) fn tag_rows(&self) -> Vec<Setting> {
        let mut rows = vec![
            content_setting(self.content.as_deref(), false),
            Setting::text(
                "Ş",
                "Şablon",
                self.content
                    .as_deref()
                    .unwrap_or_default()
                    .replace('\n', "\\n"),
            ),
            Setting::choice("B", "Çerçeve", self.tag_choices("B")),
            Setting::choice("K", "Kol", self.tag_choices("K")),
            Setting::text(
                "Y",
                "Yazı (mm)",
                self.size.unwrap_or(TAG_SIZE_MM).to_string(),
            ),
            Setting::notice(
                "H",
                "Yeni etiket",
                "Hazır içerik şablon alanını doldurur; alanı ayrıca düzenleyebilirsiniz. Bu ayarlar yeni etikete uygulanır; seçili nesne değişmez. Yazı boyu kağıt üzerinde mm'dir. Boş şablon türün şablonunu kullanır; \\n yeni satırdır.",
            ),
            Setting::notice(
                "A",
                "Şablon alanları",
                "{number}: numara · {name}: ad · {area}: seçili yöntem alanı · {netArea}: net alan · {level}: kat · {usage}: kullanım · {unit}: bölüm · {target}: tasarım hedefi · {targetDiff}: hedef farkı · {attr:Alan}: öznitelik. {floorElevation}: kat kotu; {ceilingElevation}: kat kotu + açık mahal yüksekliği. Kotlar çizim birimindedir. Eksik veri, kopuk net sınır ve elle çizilmiş brüt/eksenin neti boş kalır. En çok 200 karakter, 6 satır.",
            ),
        ];
        if let Some(why) = &self.problem {
            rows.push(Setting::notice("E", "Uygulanamıyor", why));
        }
        rows
    }

    pub(super) fn configure_tag(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        match key {
            "İ" => {
                if !choose_content(&mut self.content, value) {
                    return false;
                }
                self.ask = None;
            }
            "Ş" => {
                let text = value.trim_end_matches(['\r', '\n']).replace("\\n", "\n");
                if !text.trim().is_empty() {
                    if let Err(why) = tag_template(&text) {
                        cx.say(Level::Warn, why);
                        return false;
                    }
                    self.content = Some(text);
                } else {
                    self.content = None;
                }
                self.ask = None;
            }
            "B" => {
                let shape = if value == "default" {
                    None
                } else if let Some(s) = TagShape::ALL.iter().find(|s| s.name() == value) {
                    Some(*s)
                } else {
                    return false;
                };
                self.shape = shape;
            }
            "K" if matches!(value, "yes" | "no") => self.leader = value == "yes",
            "Y" => {
                if value.trim().is_empty() {
                    self.size = None;
                } else {
                    let text = value
                        .trim()
                        .strip_suffix("mm")
                        .unwrap_or(value.trim())
                        .trim();
                    let Some(size) =
                        parse_number(text).filter(|v| v.is_finite() && *v > 0. && *v <= 50.)
                    else {
                        cx.say(Level::Warn, "Yazı boyunu kağıt üzerinde mm olarak yazın: 2.5 veya 3 mm. Sıfırdan büyük, en çok 50 mm olmalı.");
                        return false;
                    };
                    self.size = Some(size);
                }
            }
            _ => return false,
        }
        self.see(cx);
        true
    }
}
