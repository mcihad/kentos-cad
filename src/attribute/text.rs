//! Türkçe metin karşılaştırma: arama için sadeleştirme ve alfabetik sıra.

use std::cmp::Ordering;

/// Türkçe kurallarıyla küçük harfe çevirir: I → ı, İ → i.
pub fn to_lowercase(text: &str) -> String {
    text.chars()
        .flat_map(|character| match character {
            'I' => vec!['ı'],
            'İ' => vec!['i'],
            other => other.to_lowercase().collect(),
        })
        .collect()
}

/// Arama için sadeleştirir: Türkçe küçük harf, ardından ç, ğ, ı, ö, ş, ü ve
/// şapkalı harfler ASCII karşılıklarına çevrilir. Böylece "izmir", "İZMİR"
/// ve "Izmir" aynı sonucu verir; "cizgi" "Çizgi"yi bulur.
pub fn fold(text: &str) -> String {
    to_lowercase(text)
        .chars()
        .map(|character| match character {
            'ç' => 'c',
            'ğ' => 'g',
            'ı' => 'i',
            'ö' => 'o',
            'ş' => 's',
            'ü' => 'u',
            'â' => 'a',
            'î' => 'i',
            'û' => 'u',
            other => other,
        })
        .collect()
}

/// `haystack`, sadeleştirilmiş hâliyle `needle`'ı içeriyor mu.
pub fn contains(haystack: &str, needle: &str) -> bool {
    fold(haystack).contains(&fold(needle))
}

/// Türk alfabesi sırasıyla karşılaştırır (a b c ç d e f g ğ h ı i j …);
/// büyük/küçük harf ayrımı yalnızca harfler eşitse sırayı belirler.
pub fn compare(a: &str, b: &str) -> Ordering {
    let key = |text: &str| to_lowercase(text).chars().map(rank).collect::<Vec<_>>();

    key(a).cmp(&key(b)).then_with(|| a.cmp(b))
}

const ALPHABET: &str = "abcçdefgğhıijklmnoöprsştuüvwxyz";

/// Harfin Türk alfabesindeki sırası; alfabe dışındaki karakterler kod
/// noktalarına göre harflerden sonra gelir. Rakamlar harflerden öncedir.
fn rank(character: char) -> u32 {
    if character.is_ascii_digit() {
        return u32::from(character);
    }

    match ALPHABET.chars().position(|letter| letter == character) {
        Some(position) => 1_000 + position as u32,
        None => 10_000 + u32::from(character),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dotted_and_dotless_i_are_lowercased_correctly() {
        assert_eq!(to_lowercase("İSTANBUL"), "istanbul");
        assert_eq!(to_lowercase("IĞDIR"), "ığdır");
    }

    #[test]
    fn folding_ignores_case_and_turkish_letters() {
        assert!(contains("İzmir", "izmir"));
        assert!(contains("İzmir", "IZMIR"));
        assert!(contains("Çizgi 1", "cizgi"));
        assert!(contains("Şanlıurfa", "sanliurfa"));
        assert!(!contains("Ankara", "izmir"));
    }

    #[test]
    fn sorting_follows_the_turkish_alphabet() {
        let mut cities = vec![
            "Zonguldak",
            "Çorum",
            "Cizre",
            "Iğdır",
            "İzmir",
            "Denizli",
            "Şırnak",
            "Sivas",
        ];
        cities.sort_by(|a, b| compare(a, b));

        assert_eq!(
            cities,
            [
                "Cizre",
                "Çorum",
                "Denizli",
                "Iğdır",
                "İzmir",
                "Sivas",
                "Şırnak",
                "Zonguldak"
            ]
        );
    }
}
