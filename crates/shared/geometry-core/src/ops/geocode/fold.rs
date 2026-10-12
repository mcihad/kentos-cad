//! Folding and words (docs/adr/0229 §3): every comparison is of folded text.
//! Turkish letters and the circumflexed vowels become their plain ASCII
//! letters (`I`, `ı` and `İ` all `i`), combining marks go, any other
//! character takes its lower case when that is one character. The folded
//! text is cut into words (runs of letters and digits; a run is also cut
//! before an ASCII digit that follows a non-digit, `no12` → `no`, `12`, but
//! not after one, `12a`) and the marks `/`, `,` and `-`; anything else
//! separates. A word that is one of an address's kinds is its tag.

/// A folded address's piece.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token {
    Word(String),
    Slash,
    Comma,
    Dash,
}

impl Token {
    /// The word, if it is one.
    pub fn word(&self) -> Option<&str> {
        match self {
            Token::Word(w) => Some(w),
            _ => None,
        }
    }
}

/// What a whole word says of an address (docs/adr/0229 §3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Tag {
    Neighbourhood,
    Village,
    Avenue,
    Street,
    Boulevard,
    Square,
    Road,
    Number,
}

impl Tag {
    /// A street's kind: cadde, sokak, bulvar, meydan, yol.
    pub fn is_street(self) -> bool {
        matches!(
            self,
            Tag::Avenue | Tag::Street | Tag::Boulevard | Tag::Square | Tag::Road
        )
    }

    /// An area's kind: mahalle, köy.
    pub fn is_area(self) -> bool {
        matches!(self, Tag::Neighbourhood | Tag::Village)
    }

    /// The tag's name in the rules' words.
    pub fn name(self) -> &'static str {
        match self {
            Tag::Neighbourhood => "MAHALLE",
            Tag::Village => "KÖY",
            Tag::Avenue => "CADDE",
            Tag::Street => "SOKAK",
            Tag::Boulevard => "BULVAR",
            Tag::Square => "MEYDAN",
            Tag::Road => "YOL",
            Tag::Number => "NO",
        }
    }
}

/// A folded word's tag, if it is one.
pub fn tag_of(word: &str) -> Option<Tag> {
    Some(match word {
        "mahalle" | "mahallesi" | "mah" | "mh" => Tag::Neighbourhood,
        "koy" | "koyu" => Tag::Village,
        "cadde" | "caddesi" | "cad" | "cd" => Tag::Avenue,
        "sokak" | "sokagi" | "sok" | "sk" => Tag::Street,
        "bulvar" | "bulvari" | "blv" | "bulv" => Tag::Boulevard,
        "meydan" | "meydani" | "myd" => Tag::Square,
        "yol" | "yolu" => Tag::Road,
        "no" | "numara" | "nu" | "nolu" => Tag::Number,
        _ => return None,
    })
}

/// `text` folded (docs/adr/0229 §3).
pub fn fold(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            'Ç' | 'ç' => out.push('c'),
            'Ğ' | 'ğ' => out.push('g'),
            'I' | 'ı' | 'İ' => out.push('i'),
            'Ö' | 'ö' => out.push('o'),
            'Ş' | 'ş' => out.push('s'),
            'Ü' | 'ü' => out.push('u'),
            'Â' | 'â' => out.push('a'),
            'Î' | 'î' => out.push('i'),
            'Û' | 'û' => out.push('u'),
            '\u{300}'..='\u{36f}' => {}
            _ => {
                let mut low = c.to_lowercase();
                match (low.next(), low.next()) {
                    (Some(l), None) => out.push(l),
                    _ => out.push(c),
                }
            }
        }
    }
    out
}

/// The pieces of folded text, in order.
pub fn tokens(folded: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut word = String::new();
    let mut last_digit = false;
    let flush = |word: &mut String, out: &mut Vec<Token>| {
        if !word.is_empty() {
            out.push(Token::Word(std::mem::take(word)));
        }
    };
    for c in folded.chars() {
        if c.is_alphanumeric() {
            let digit = c.is_ascii_digit();
            if digit && !last_digit && !word.is_empty() {
                flush(&mut word, &mut out);
            }
            word.push(c);
            last_digit = digit;
            continue;
        }
        flush(&mut word, &mut out);
        last_digit = false;
        match c {
            '/' => out.push(Token::Slash),
            ',' => out.push(Token::Comma),
            '-' => out.push(Token::Dash),
            _ => {}
        }
    }
    flush(&mut word, &mut out);
    out
}

/// Whether a word is numbered: it starts with an ASCII digit.
pub fn numbered_word(word: &str) -> bool {
    word.as_bytes().first().is_some_and(u8::is_ascii_digit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turkish_letters_fold_to_ascii() {
        assert_eq!(fold("KIZILAY Mahallesi"), "kizilay mahallesi");
        assert_eq!(fold("İĞNECİ ŞÜKRÜ Çağ"), "igneci sukru cag");
        assert_eq!(fold("i\u{307}"), "i");
        assert_eq!(fold("Âşık Î Û"), "asik i u");
    }

    #[test]
    fn words_are_cut_before_a_digit_after_a_letter() {
        let w = |t: &str| {
            tokens(&fold(t))
                .into_iter()
                .map(|t| match t {
                    Token::Word(w) => w,
                    Token::Slash => "/".into(),
                    Token::Comma => ",".into(),
                    Token::Dash => "-".into(),
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(w("No12/A"), ["no", "12", "/", "a"]);
        assert_eq!(w("12a  ,1203/1. Sk."), ["12a", ",", "1203", "/", "1", "sk"]);
        assert_eq!(
            w("Çankaya-Ankara: 06420"),
            ["cankaya", "-", "ankara", "06420"]
        );
    }
}
