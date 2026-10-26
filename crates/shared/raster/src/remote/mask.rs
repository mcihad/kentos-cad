//! Kalite maskesi uygula (docs/adr/0644 §2): a quality (QA) raster's rule, as the STAC preparation writes it
//! (`stac.qa_mask`: `values:0,1,3,8,9,10`, Sentinel-2's SCL classes; `bits:0,1,3,4`, Landsat's QA_PIXEL bits). A data
//! cell whose QA value (the nearest QA cell under its centre) the rule names, or that no QA value covers, is emptied;
//! a cloud is then no value, never a loss of vegetation.

/// A mask rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rule {
    /// A QA value among these hides the cell.
    Values(Vec<u64>),
    /// A QA value with any of these bits set (0 the lowest) hides the cell.
    Bits(Vec<u32>),
}

/// The most a class value may be (a QA band is 16 bits at most).
const MOST_VALUE: u64 = 65_535;
/// The highest bit a QA value may set.
const MOST_BIT: u64 = 31;

/// `values:…` or `bits:…` (any case, spaces round the numbers) read; why not.
pub fn parse(text: &str) -> Result<Rule, String> {
    let t = text.trim();
    let bad = || format!("Maske kuralı okunamadı: “{t}”; values:0,1,3 ya da bits:1,3 yazın.");
    let (kind, rest) = t.split_once(':').ok_or_else(bad)?;
    let kind = kind.trim().to_lowercase();
    if kind != "values" && kind != "bits" {
        return Err(bad());
    }
    let mut nums = Vec::new();
    for tok in rest.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        if !tok.bytes().all(|b| b.is_ascii_digit()) {
            return Err(bad());
        }
        nums.push(tok.parse::<u64>().map_err(|_| bad())?);
    }
    if nums.is_empty() {
        return Err(bad());
    }
    if kind == "bits" {
        if nums.iter().any(|&n| n > MOST_BIT) {
            return Err("Maskenin bitleri 0 ile 31 arasındadır.".into());
        }
        return Ok(Rule::Bits(nums.into_iter().map(|n| n as u32).collect()));
    }
    if nums.iter().any(|&n| n > MOST_VALUE) {
        return Err("Maskenin değerleri 0 ile 65535 arasındadır.".into());
    }
    Ok(Rule::Values(nums))
}

impl Rule {
    /// Whether a QA value hides its cell: no value (NaN: outside the QA raster or its nodata) hides it too.
    pub fn hides(&self, q: f64) -> bool {
        if q.is_nan() {
            return true;
        }
        let n = q.trunc();
        match self {
            Rule::Values(v) => n >= 0.0 && v.iter().any(|&x| x as f64 == n),
            Rule::Bits(bits) => {
                n >= 0.0 && {
                    let n = n as u64;
                    bits.iter().any(|&b| (n >> b) & 1 == 1)
                }
            }
        }
    }
}

/// The summary's tail: the hidden of the cells that had a value.
pub fn tail(valued: u64, hidden: u64) -> String {
    if valued == 0 {
        return "Değerli hücre yok.".into();
    }
    let share = (hidden as f64 * 100.0) / valued as f64;
    format!(
        "Maskelenen {} / {} hücre (%{}).",
        super::count(hidden),
        super::count(valued),
        super::fixed(share, 2)
    )
}

/// The warning when every cell with a value was hidden.
pub fn all_hidden(valued: u64, hidden: u64) -> Option<String> {
    (valued > 0 && hidden == valued)
        .then(|| "Bütün hücreler maskelendi: kural ya da kalite rasteri yanlış olabilir.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_read_as_the_preparation_writes_them() {
        assert_eq!(parse("values:0,1,3").unwrap(), Rule::Values(vec![0, 1, 3]));
        assert_eq!(parse(" VALUES: 9 ,8 ").unwrap(), Rule::Values(vec![9, 8]));
        assert_eq!(parse("bits:0,4").unwrap(), Rule::Bits(vec![0, 4]));
        assert!(parse("renk:1").is_err());
        assert!(parse("values:").is_err());
        assert!(parse("bits:2,40").is_err());
        assert!(parse("values:-1").is_err());
        let r = parse("bits:1,3").unwrap();
        assert!(r.hides(2.0) && r.hides(8.0) && !r.hides(4.0) && r.hides(f64::NAN));
    }
}
