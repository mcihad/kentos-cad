//! The exact bundled faces shared by drawing layout, desktop and PDF (ADR 0914).
//! Parsing them needs no filesystem, OS font list or network access.

/// Every face, embedded (1.9 MB; OFL, the licences beside them).
pub const DRAWING_FACES: [&[u8]; 22] = [
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/ArchitectsDaughter-400.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/Arimo-400.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/Arimo-500.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/Arimo-600.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/Arimo-700.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/Barlow-400-italic.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/Barlow-400.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/Barlow-500.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/Barlow-600.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/CourierPrime-400-italic.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/CourierPrime-400.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/CourierPrime-700.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/IBMPlexMono-400.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/IBMPlexMono-500.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/Overpass-400.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/Overpass-500.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/Overpass-600.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/Overpass-700.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/Quicksand-400.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/Quicksand-500.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/Quicksand-600.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/Quicksand-700.ttf"),
];

/// Each of [`DRAWING_FACES`] as the sheet core names it (`TextPrim.font`, its weight, italic): what a
/// sheet PDF embeds (docs/sheet/design.md §9a).
pub const DRAWING_KEYS: [(&str, u16, bool); 22] = [
    ("architects-daughter", 400, false),
    ("arimo", 400, false),
    ("arimo", 500, false),
    ("arimo", 600, false),
    ("arimo", 700, false),
    ("barlow", 400, true),
    ("barlow", 400, false),
    ("barlow", 500, false),
    ("barlow", 600, false),
    ("courier-prime", 400, true),
    ("courier-prime", 400, false),
    ("courier-prime", 700, false),
    ("plex-mono", 400, false),
    ("plex-mono", 500, false),
    ("overpass", 400, false),
    ("overpass", 500, false),
    ("overpass", 600, false),
    ("overpass", 700, false),
    ("quicksand", 400, false),
    ("quicksand", 500, false),
    ("quicksand", 600, false),
    ("quicksand", 700, false),
];

/// Unicode fallback faces. These do not change the seven user-selected drawing families.
pub const FALLBACK_FACES: [&[u8]; 7] = [
    include_bytes!("../../data/text-fonts/NotoSans-400.ttf"),
    include_bytes!("../../data/text-fonts/NotoSans-600.ttf"),
    include_bytes!("../../data/text-fonts/NotoSansArabic-400.ttf"),
    include_bytes!("../../data/text-fonts/NotoSansArabic-600.ttf"),
    include_bytes!("../../data/text-fonts/NotoSansHebrew-400.ttf"),
    include_bytes!("../../data/text-fonts/NotoSansHebrew-600.ttf"),
    include_bytes!("../../data/text-fonts/NotoSansMath-400.ttf"),
];

pub const FALLBACK_KEYS: [(&str, u16, bool); 7] = [
    ("noto-sans", 400, false),
    ("noto-sans", 600, false),
    ("noto-sans-arabic", 400, false),
    ("noto-sans-arabic", 600, false),
    ("noto-sans-hebrew", 400, false),
    ("noto-sans-hebrew", 600, false),
    ("noto-sans-math", 400, false),
];
