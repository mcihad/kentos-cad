//! A tool's prompt as data: the step it waits for and its options, each with
//! the key that chooses it. The web builds the same thing as text,
//! `Araç: adım [Seçenek (TUŞ) / Seçenek (TUŞ): değer]`, and parses it back
//! into buttons (`apps/web/src/ui/promptOptions.ts`); [`Prompt::text`] writes
//! exactly that text, so the command line, the status bar and the traces read
//! the same words on both platforms. A typed `PromptSpec` is TODOS.md UX-02.

use std::borrow::Cow;

/// One option of a prompt: `Geri (G)`, or with its current value
/// `Döndür (D): 30°` (the web's `PromptOption.value`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptOption {
    pub label: &'static str,
    /// What typing it sends: a letter for the tool, or `Enter` (confirm).
    pub key: &'static str,
    /// The state of a toggle or a value option (`kapalı`, `6`), shown after it.
    pub value: Option<String>,
}

/// What the running command waits for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prompt {
    /// The tool's name (`Kapalı alan`); none when no command runs.
    pub tool: Option<&'static str>,
    /// The step: `sonraki noktayı belirtin`; some steps carry a value
    /// (`yarıçapı yazın (Enter: 12.500 m)`).
    pub step: Cow<'static, str>,
    pub options: Vec<PromptOption>,
}

impl Prompt {
    /// The idle prompt: the web's select tool says `Komut`.
    pub fn idle() -> Self {
        Self {
            tool: None,
            step: Cow::Borrowed("Komut"),
            options: Vec::new(),
        }
    }

    pub fn new(tool: &'static str, step: impl Into<Cow<'static, str>>) -> Self {
        Self {
            tool: Some(tool),
            step: step.into(),
            options: Vec::new(),
        }
    }

    pub fn option(mut self, label: &'static str, key: &'static str) -> Self {
        self.options.push(PromptOption {
            label,
            key,
            value: None,
        });
        self
    }

    /// An option with its current value: `Kenar sayısı (S): 6`.
    pub fn option_with(
        mut self,
        label: &'static str,
        key: &'static str,
        value: impl Into<String>,
    ) -> Self {
        self.options.push(PromptOption {
            label,
            key,
            value: Some(value.into()),
        });
        self
    }

    /// The web's prompt text: `Kapalı alan: sonraki noktayı belirtin [Yay (Y) / Geri (G)]`,
    /// `Dikdörtgen: karşı köşeyi belirtin [Döndür (D): 0° / Boyutlar (B)]`.
    pub fn text(&self) -> String {
        let mut text = match self.tool {
            Some(tool) => format!("{tool}: {}", self.step),
            None => self.step.to_string(),
        };
        if !self.options.is_empty() {
            let options: Vec<String> = self
                .options
                .iter()
                .map(|o| match &o.value {
                    Some(value) => format!("{} ({}): {value}", o.label, o.key),
                    None => format!("{} ({})", o.label, o.key),
                })
                .collect();
            text.push_str(&format!(" [{}]", options.join(" / ")));
        }
        text
    }

    /// The option keys in order, as the traces compare them.
    pub fn keys(&self) -> Vec<&'static str> {
        self.options.iter().map(|o| o.key).collect()
    }

    /// The option a pressed letter chooses: its key exactly (Turkish upper
    /// case), else the same letter without Turkish marks, so `Çap (Ç)` also
    /// answers to C (the web's `optionForKey`).
    pub fn option_for_key(&self, key: &str) -> Option<&PromptOption> {
        let key = upper_tr(key);
        self.options
            .iter()
            .find(|o| upper_tr(o.key) == key)
            .or_else(|| self.options.iter().find(|o| fold(o.key) == fold(&key)))
    }
}

/// `toLocaleUpperCase('tr-TR')`: i is İ and ı is I; everything else as Unicode says.
pub fn upper_tr(text: &str) -> String {
    text.chars()
        .flat_map(|c| match c {
            'i' => vec!['İ'],
            'ı' => vec!['I'],
            c => c.to_uppercase().collect(),
        })
        .collect()
}

/// A key without its Turkish mark (`Ç` → `C`), after upper-casing.
fn fold(key: &str) -> String {
    let upper = upper_tr(key);
    match upper.as_str() {
        "Ç" => "C".to_owned(),
        "Ş" => "S".to_owned(),
        "Ğ" => "G".to_owned(),
        "Ö" => "O".to_owned(),
        "Ü" => "U".to_owned(),
        "İ" => "I".to_owned(),
        _ => upper,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arc() -> Prompt {
        Prompt::new("Kapalı alan", "yayın bitiş noktasını belirtin")
            .option("Düz", "D")
            .option("İkinci nokta", "İ")
            .option("Bitir", "Enter")
    }

    #[test]
    fn the_text_is_the_web_s() {
        assert_eq!(Prompt::idle().text(), "Komut");
        assert_eq!(
            Prompt::new("Kapalı alan", "ilk noktayı belirtin").text(),
            "Kapalı alan: ilk noktayı belirtin"
        );
        assert_eq!(
            arc().text(),
            "Kapalı alan: yayın bitiş noktasını belirtin [Düz (D) / İkinci nokta (İ) / Bitir (Enter)]"
        );
        assert_eq!(arc().keys(), ["D", "İ", "Enter"]);
    }

    #[test]
    fn letters_choose_options_the_turkish_way() {
        let p = arc();
        assert_eq!(p.option_for_key("d").map(|o| o.key), Some("D"));
        // i is İ in Turkish; I without the dot reaches İ through the folding.
        assert_eq!(p.option_for_key("i").map(|o| o.key), Some("İ"));
        assert_eq!(p.option_for_key("I").map(|o| o.key), Some("İ"));
        assert_eq!(p.option_for_key("x"), None);
        assert_eq!(upper_tr("ıi"), "Iİ");
    }
}
