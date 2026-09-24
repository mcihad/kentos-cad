/**
 * Turkish-insensitive matching: upper case in the Turkish locale, then the
 * dotted/cedilla letters folded to their plain forms, so "ÇİZGİ", "cizgi"
 * and "Çizgi" all compare equal. Used by command aliases and tool search.
 */
export const foldTurkish = (s: string) =>
  s
    .trim()
    .toLocaleUpperCase('tr-TR')
    .replace(/Ç/g, 'C')
    .replace(/Ğ/g, 'G')
    .replace(/İ/g, 'I')
    .replace(/Ö/g, 'O')
    .replace(/Ş/g, 'S')
    .replace(/Ü/g, 'U');
