//! A batch's addresses made unique (docs/adr/0229 §10): an address's key is
//! its folded text with its runs of white space made one space and trimmed;
//! the first row of a key asks, the others take its answer.

use kentos_geometry_core::ops::geocode::fold;

/// An address's key.
pub fn key(text: &str) -> String {
    fold(text).split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The unique keys in their first rows' order, and each row's key's place; an empty address has none.
pub fn unique(addresses: &[String]) -> (Vec<String>, Vec<Option<usize>>) {
    let mut keys: Vec<String> = Vec::new();
    let mut at: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let rows = addresses
        .iter()
        .map(|a| {
            let k = key(a);
            if k.is_empty() {
                return None;
            }
            Some(*at.entry(k.clone()).or_insert_with(|| {
                keys.push(k);
                keys.len() - 1
            }))
        })
        .collect();
    (keys, rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_of_one_address_ask_once() {
        let rows: Vec<String> = ["Atatürk Blv. 12", "ATATÜRK  blv. 12 ", "", "İnönü Cd. 3"]
            .iter()
            .map(|s| (*s).to_owned())
            .collect();
        let (keys, at) = unique(&rows);
        assert_eq!(keys, ["ataturk blv. 12", "inonu cd. 3"]);
        assert_eq!(at, [Some(0), Some(0), None, Some(1)]);
    }
}
