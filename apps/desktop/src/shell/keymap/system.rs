//! The operating system's own keys (docs/adr/9308): the keys macOS, Windows and a Linux desktop take before the
//! program sees them, which no command of the user's may have where the program runs.

/// An operating system, for its keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum System {
    MacOs,
    Windows,
    Linux,
}

impl System {
    /// The one this program runs on.
    pub fn this() -> Self {
        if cfg!(target_os = "macos") {
            Self::MacOs
        } else if cfg!(target_os = "windows") {
            Self::Windows
        } else {
            Self::Linux
        }
    }
}

/// The keys the operating system takes before the program sees them, as the input routing writes them (Ctrl is
/// ⌘ on macOS), with what they do there. Esc, Enter and Tab's are KentOS's already (`reserved_key`).
const SYSTEM_KEYS: [(System, &str, &str); 18] = [
    (System::MacOs, "Ctrl+Q", "⌘Q macOS'ta uygulamadan çıkar."),
    (
        System::MacOs,
        "Ctrl+H",
        "⌘H macOS'ta uygulamanın pencerelerini gizler.",
    ),
    (
        System::MacOs,
        "Ctrl+Alt+H",
        "⌥⌘H macOS'ta öbür uygulamaları gizler.",
    ),
    (System::MacOs, "Ctrl+M", "⌘M macOS'ta pencereyi küçültür."),
    (
        System::MacOs,
        "Ctrl+Alt+M",
        "⌥⌘M macOS'ta uygulamanın bütün pencerelerini küçültür.",
    ),
    (
        System::MacOs,
        "Ctrl+Space",
        "⌘Boşluk macOS'ta aramayı açar; Control ile Boşluk klavye dilini değiştirir.",
    ),
    (
        System::MacOs,
        "Ctrl+Alt+Space",
        "⌥⌘Boşluk macOS'ta arama sonuçlarını açar.",
    ),
    (
        System::MacOs,
        "Ctrl+Shift+3",
        "⇧⌘3 macOS'ta ekran görüntüsü alır.",
    ),
    (
        System::MacOs,
        "Ctrl+Shift+4",
        "⇧⌘4 macOS'ta ekranın bir bölümünün görüntüsünü alır.",
    ),
    (
        System::MacOs,
        "Ctrl+Shift+5",
        "⇧⌘5 macOS'ta ekran görüntüsü ve kaydı araçlarını açar.",
    ),
    (
        System::MacOs,
        "Ctrl+Alt+D",
        "⌥⌘D macOS'ta Dock'u gizler ya da gösterir.",
    ),
    (
        System::MacOs,
        "Ctrl+Alt+8",
        "⌥⌘8 macOS'ta ekran büyütmeyi açar ya da kapatır.",
    ),
    (
        System::Windows,
        "Alt+F4",
        "Alt+F4 Windows'ta pencereyi kapatır.",
    ),
    (
        System::Windows,
        "Alt+Space",
        "Alt+Boşluk Windows'ta pencerenin menüsünü açar.",
    ),
    (
        System::Windows,
        "Ctrl+Alt+Delete",
        "Ctrl+Alt+Delete Windows'un güvenlik ekranını açar.",
    ),
    (
        System::Linux,
        "Alt+F2",
        "Alt+F2 masaüstünde komut penceresini açar.",
    ),
    (
        System::Linux,
        "Ctrl+Alt+Delete",
        "Ctrl+Alt+Delete masaüstünde kapatma penceresini açar.",
    ),
    (
        System::Linux,
        "Ctrl+Alt+Shift+R",
        "Ctrl+Alt+Shift+R masaüstünde ekran kaydını başlatır.",
    ),
];

/// What the operating system this program runs on does with a key it takes; none when the program gets it.
pub fn system_key(chord: &str) -> Option<&'static str> {
    system_key_on(System::this(), chord)
}

/// [`system_key`] on `system`, every system's rule testable on any.
pub fn system_key_on(system: System, chord: &str) -> Option<&'static str> {
    SYSTEM_KEYS
        .iter()
        .find(|(s, k, _)| *s == system && *k == chord)
        .map(|(_, _, why)| *why)
}
