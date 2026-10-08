//! ASPRS's point classes (LAS 1.4 R15 Table 17, docs/adr/0207 §5.1): their
//! Turkish names and the colours `classification` draws them with; the same
//! on both platforms.

/// A class's name in the interface.
pub fn name(class: u8) -> String {
    let n = match class {
        0 => "Hiç sınıflanmamış",
        1 => "Sınıflanmamış",
        2 => "Zemin",
        3 => "Düşük bitki",
        4 => "Orta bitki",
        5 => "Yüksek bitki",
        6 => "Bina",
        7 => "Düşük gürültü",
        8 => "Model anahtar noktası",
        9 => "Su",
        10 => "Demiryolu",
        11 => "Yol yüzeyi",
        12 => "Örtüşme",
        13 => "Koruyucu tel",
        14 => "İletken tel",
        15 => "İletim kulesi",
        16 => "Tel bağlantısı",
        17 => "Köprü tabliyesi",
        18 => "Yüksek gürültü",
        19 => "Havai yapı",
        20 => "Yok sayılan zemin",
        21 => "Kar",
        22 => "Zaman dışı",
        23..=63 => return format!("Ayrılmış ({class})"),
        _ => return format!("Kullanıcının ({class})"),
    };
    n.to_owned()
}

/// The user classes' colours, by (class − 64) mod 12.
const USER: [[u8; 3]; 12] = [
    [0x4E, 0x79, 0xA7],
    [0xF2, 0x8E, 0x2B],
    [0x76, 0xB7, 0xB2],
    [0xED, 0xC9, 0x48],
    [0xB0, 0x7A, 0xA1],
    [0xFF, 0x9D, 0xA7],
    [0x9C, 0x75, 0x5F],
    [0xBA, 0xB0, 0xAC],
    [0x59, 0xA1, 0x4F],
    [0xE1, 0x57, 0x59],
    [0x86, 0xBC, 0xB6],
    [0xD4, 0xA6, 0xC8],
];

/// A class's colour (sRGB).
pub fn colour(class: u8) -> [u8; 3] {
    match class {
        0 => [0xA0, 0xA0, 0xA0],
        1 => [0xC8, 0xC8, 0xC8],
        2 => [0xA9, 0x7C, 0x50],
        3 => [0xC1, 0xE3, 0x9A],
        4 => [0x6D, 0xBE, 0x45],
        5 => [0x1E, 0x7B, 0x34],
        6 => [0xE0, 0x4B, 0x3A],
        7 => [0xFF, 0x00, 0xFF],
        8 => [0xFF, 0xD5, 0x4F],
        9 => [0x3A, 0x86, 0xFF],
        10 => [0x6D, 0x4C, 0x41],
        11 => [0x7A, 0x7A, 0x7A],
        12 => [0xFF, 0xAB, 0x40],
        13 => [0xB0, 0xBE, 0xC5],
        14 => [0xFF, 0xEE, 0x58],
        15 => [0x8D, 0x6E, 0x63],
        16 => [0x26, 0xC6, 0xDA],
        17 => [0x95, 0x75, 0xCD],
        18 => [0xD5, 0x00, 0xF9],
        19 => [0xF4, 0x8F, 0xB1],
        20 => [0xBC, 0xAA, 0xA4],
        21 => [0xE1, 0xF5, 0xFE],
        22 => [0x90, 0xA4, 0xAE],
        23..=63 => [0x8C, 0x8C, 0x8C],
        c => USER[usize::from(c - 64) % USER.len()],
    }
}

/// The returns' colours: single, first, intermediate, last, and one the file numbers wrongly.
pub const RETURNS: [[u8; 3]; 5] = [
    [0x4E, 0x79, 0xA7],
    [0x59, 0xA1, 0x4F],
    [0xED, 0xC9, 0x48],
    [0xE1, 0x57, 0x59],
    [0x9E, 0x9E, 0x9E],
];

/// Which of [`RETURNS`] a return `ret` of `n` is.
pub fn return_kind(ret: u8, n: u8) -> usize {
    if ret == 0 || n == 0 || ret > n {
        4
    } else if n == 1 {
        0
    } else if ret == 1 {
        1
    } else if ret == n {
        3
    } else {
        2
    }
}
