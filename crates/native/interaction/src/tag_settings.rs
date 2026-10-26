//! Transient Etiket controls; the existing semantic create command still plans and writes.
use super::*;
use crate::tool_settings::choices;
use kentos_commands::{TAG_SIZE_MM, tag_template};
use kentos_geometry_core::tools::point_text::parse_number;

impl TagTool {
    pub(super) fn tag_choices(&self, key: &str) -> Vec<OptionChoice> {
        match key {
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
                "Bu ayarlar yeni etikete uygulanır; seçili nesne değişmez. Yazı boyu kağıt üzerinde mm'dir. Boş şablon türün şablonunu kullanır; \\n yeni satırdır.",
            ),
            Setting::notice(
                "A",
                "Şablon alanları",
                "{number}: numara · {name}: ad · {area}: alan · {level}: kat · {usage}: kullanım · {attr:Alan}: öznitelik. En çok 200 karakter, 6 satır.",
            ),
        ];
        if let Some(why) = &self.problem {
            rows.push(Setting::notice("E", "Uygulanamıyor", why));
        }
        rows
    }

    pub(super) fn configure_tag(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        match key {
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
