//! Functions and variables of the expression language (the TypeScript's
//! `EXPR_FUNCTIONS` and `EXPR_VARIABLES`). Names are matched
//! Turkish-insensitively (yuvarla = YUVARLA, $çevre = $cevre); every
//! function also answers to its English (QGIS) name. The menus, the help
//! and the field's completion read these tables; what a function computes
//! is `scalar::call`.

use crate::js::text;

/// The expression builder's groups (docs/adr/0100 §5), in the tree's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Group {
    /// The fields of the objects the expression runs on (from the host).
    Fields,
    Variables,
    Conversions,
    Geometry,
    Operators,
    Conditionals,
    Math,
    Text,
}

impl Group {
    pub const ALL: [Group; 8] = [
        Group::Fields,
        Group::Variables,
        Group::Conversions,
        Group::Geometry,
        Group::Operators,
        Group::Conditionals,
        Group::Math,
        Group::Text,
    ];

    /// The group's heading.
    pub fn title(self) -> &'static str {
        match self {
            Group::Fields => "Alanlar ve değerler",
            Group::Variables => "Değişkenler",
            Group::Conversions => "Dönüşümler",
            Group::Geometry => "Geometri",
            Group::Operators => "İşleçler",
            Group::Conditionals => "Koşullar",
            Group::Math => "Matematik",
            Group::Text => "Metin",
        }
    }

    /// A stable name for the group (for the pages and the fixtures).
    pub fn id(self) -> &'static str {
        match self {
            Group::Fields => "fields",
            Group::Variables => "variables",
            Group::Conversions => "conversions",
            Group::Geometry => "geometry",
            Group::Operators => "operators",
            Group::Conditionals => "conditionals",
            Group::Math => "math",
            Group::Text => "text",
        }
    }
}

/// An operator or a word of the language (ve, doğru, boş …), for the builder.
pub struct OpDef {
    /// As written in an expression.
    pub symbol: &'static str,
    /// The other ways to write it.
    pub aliases: &'static [&'static str],
    pub signature: &'static str,
    pub description: &'static str,
    pub examples: &'static [(&'static str, &'static str)],
}

pub static OPERATORS: &[OpDef] = &[
    OpDef {
        symbol: "=",
        aliases: &["=="],
        signature: "a = b",
        description: "Eşit mi: sayılar değerleriyle (12 = '12.0' doğru), metin aynen (büyük/küçük harf ayrı); boş yalnız boşa eşit",
        examples: &[
            ("Nitelik = 'Arsa'", "Arsa olanlar için doğru"),
            ("Ada = boş", "adası olmayanlar için doğru"),
        ],
    },
    OpDef {
        symbol: "!=",
        aliases: &["<>"],
        signature: "a != b",
        description: "Eşit değil mi",
        examples: &[("Nitelik != 'Yol'", "yol olmayanlar için doğru")],
    },
    OpDef {
        symbol: "<",
        aliases: &[],
        signature: "a < b",
        description: "Küçük mü: sayılar sayı olarak, metin Türkçe sıraya göre; boş bir değerle yanlış",
        examples: &[("$alan < 500", "500 m²'den küçükler için doğru")],
    },
    OpDef {
        symbol: "<=",
        aliases: &[],
        signature: "a <= b",
        description: "Küçük ya da eşit mi",
        examples: &[("Kat <= 3", "3 ve daha az katlılar için doğru")],
    },
    OpDef {
        symbol: ">",
        aliases: &[],
        signature: "a > b",
        description: "Büyük mü: sayılar sayı olarak, metin Türkçe sıraya göre; boş bir değerle yanlış",
        examples: &[("$alan > 500", "500 m²'den büyükler için doğru")],
    },
    OpDef {
        symbol: ">=",
        aliases: &[],
        signature: "a >= b",
        description: "Büyük ya da eşit mi",
        examples: &[("Kat >= 4", "4 ve daha çok katlılar için doğru")],
    },
    OpDef {
        symbol: "+",
        aliases: &[],
        signature: "a + b",
        description: "Toplar; iki taraf da sayı değilse metinleri birleştirir; alanı olmayan bir tarafla boş",
        examples: &[("Parsel + 1", "13 (Parsel 12 ise)")],
    },
    OpDef {
        symbol: "-",
        aliases: &[],
        signature: "a - b",
        description: "Çıkarır; sayı olmayan bir tarafla boş",
        examples: &[("[Tapu alanı] - $alan", "tapu ile ölçü farkı")],
    },
    OpDef {
        symbol: "*",
        aliases: &[],
        signature: "a * b",
        description: "Çarpar",
        examples: &[("$alan * 2", "1200")],
    },
    OpDef {
        symbol: "/",
        aliases: &[],
        signature: "a / b",
        description: "Böler; sıfıra bölme boş",
        examples: &[("$alan / 10000", "hektar")],
    },
    OpDef {
        symbol: "%",
        aliases: &[],
        signature: "a % b",
        description: "Bölümden kalan",
        examples: &[("$sıra % 2 = 0", "çift sıradakiler için doğru")],
    },
    OpDef {
        symbol: "||",
        aliases: &[],
        signature: "a || b",
        description: "Metinleri birleştirir",
        examples: &[("Ada || '/' || Parsel", "'1245/12'")],
    },
    OpDef {
        symbol: "ve",
        aliases: &["and"],
        signature: "a ve b",
        description: "İki koşul da doğru mu",
        examples: &[(
            "Nitelik = 'Arsa' ve $alan > 500",
            "500 m²'den büyük arsalar için doğru",
        )],
    },
    OpDef {
        symbol: "veya",
        aliases: &["or"],
        signature: "a veya b",
        description: "Koşullardan en az biri doğru mu",
        examples: &[(
            "Nitelik = 'Arsa' veya Nitelik = 'Tarla'",
            "arsa ve tarlalar için doğru",
        )],
    },
    OpDef {
        symbol: "değil",
        aliases: &["not"],
        signature: "değil a",
        description: "Koşulun tersi",
        examples: &[("değil boş(Ada)", "adası olanlar için doğru")],
    },
    OpDef {
        symbol: "doğru",
        aliases: &["true"],
        signature: "doğru",
        description: "Doğru değeri",
        examples: &[("eğer(doğru, 1, 2)", "1")],
    },
    OpDef {
        symbol: "yanlış",
        aliases: &["false"],
        signature: "yanlış",
        description: "Yanlış değeri",
        examples: &[("değil yanlış", "doğru")],
    },
    OpDef {
        symbol: "boş",
        aliases: &["null"],
        signature: "boş",
        description: "Boş değer: alanı olmayan ya da boş metin; yalnız boşa eşittir",
        examples: &[("Ada = boş", "adası olmayanlar için doğru")],
    },
];

/// A variable: what it reads of the object.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Var {
    Area,
    Length,
    Vertices,
    Kind,
    Layer,
    Label,
    Y,
    X,
    Index,
    Id,
    Scale,
    CentroidY,
    CentroidX,
    MinY,
    MaxY,
    MinX,
    MaxX,
    Width,
    Height,
}

pub struct VarDef {
    pub var: Var,
    /// Name without "$", as shown.
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub description: &'static str,
    /// Where the expression builder lists it.
    pub group: Group,
    /// Expressions that use it, each with what it gives.
    pub examples: &'static [(&'static str, &'static str)],
}

pub static VARIABLES: &[VarDef] = &[
    VarDef {
        var: Var::Area,
        name: "alan",
        aliases: &[],
        description: "Alan (m²): kapalı alan (delikler düşülür), daire, tam elips, tarama",
        group: Group::Geometry,
        examples: &[
            ("$alan", "600"),
            ("$alan / 10000", "hektar"),
            ("$alan > 500", "doğru ya da yanlış"),
        ],
    },
    VarDef {
        var: Var::Length,
        name: "uzunluk",
        aliases: &["çevre", "length", "perimeter"],
        description: "Uzunluk ya da çevre (m)",
        group: Group::Geometry,
        examples: &[
            ("$uzunluk", "100"),
            ("metin($uzunluk, 2) || ' m'", "'100.00 m'"),
        ],
    },
    VarDef {
        var: Var::Vertices,
        name: "köşe",
        aliases: &["vertices"],
        description: "Köşe sayısı (delikler dahil)",
        group: Group::Geometry,
        examples: &[("$köşe", "4")],
    },
    VarDef {
        var: Var::Kind,
        name: "tür",
        aliases: &["type"],
        description: "Nesne türü: “Kapalı alan”, “Çizgi” …",
        group: Group::Variables,
        examples: &[("$tür = 'Kapalı alan'", "kapalı alanlar için doğru")],
    },
    VarDef {
        var: Var::Layer,
        name: "katman",
        aliases: &["layer"],
        description: "Katman adı",
        group: Group::Variables,
        examples: &[("$katman", "'Parsel sınırı'")],
    },
    VarDef {
        var: Var::Label,
        name: "etiket",
        aliases: &["label"],
        description: "Çizimde görünen etiket (parsel no, nokta adı)",
        group: Group::Variables,
        examples: &[("$etiket", "'12'")],
    },
    VarDef {
        var: Var::Y,
        name: "y",
        aliases: &[],
        description: "Y (sağa): nesnenin yer noktası",
        group: Group::Geometry,
        examples: &[("$y", "yer noktasının Y'si")],
    },
    VarDef {
        var: Var::X,
        name: "x",
        aliases: &[],
        description: "X (yukarı): nesnenin yer noktası",
        group: Group::Geometry,
        examples: &[("$x", "yer noktasının X'i")],
    },
    VarDef {
        var: Var::CentroidY,
        name: "merkez_y",
        aliases: &[],
        description: "Ağırlık merkezinin Y'si (sağa): kapalı alanda, taramada, dairede ve tam elipste alanın merkezi (delikler düşülür); öbür nesnelerde yer noktası",
        group: Group::Geometry,
        examples: &[(
            "yuvarla($merkez_y, 2)",
            "ağırlık merkezinin Y'si, 2 ondalıkla",
        )],
    },
    VarDef {
        var: Var::CentroidX,
        name: "merkez_x",
        aliases: &[],
        description: "Ağırlık merkezinin X'i (yukarı): kapalı alanda, taramada, dairede ve tam elipste alanın merkezi (delikler düşülür); öbür nesnelerde yer noktası",
        group: Group::Geometry,
        examples: &[(
            "yuvarla($merkez_x, 2)",
            "ağırlık merkezinin X'i, 2 ondalıkla",
        )],
    },
    VarDef {
        var: Var::MinY,
        name: "min_y",
        aliases: &[],
        description: "Sınır kutusunun en küçük Y'si (sağa)",
        group: Group::Geometry,
        examples: &[("$min_y", "kutunun batı kenarının Y'si")],
    },
    VarDef {
        var: Var::MaxY,
        name: "max_y",
        aliases: &[],
        description: "Sınır kutusunun en büyük Y'si (sağa)",
        group: Group::Geometry,
        examples: &[("$max_y", "kutunun doğu kenarının Y'si")],
    },
    VarDef {
        var: Var::MinX,
        name: "min_x",
        aliases: &[],
        description: "Sınır kutusunun en küçük X'i (yukarı)",
        group: Group::Geometry,
        examples: &[("$min_x", "kutunun güney kenarının X'i")],
    },
    VarDef {
        var: Var::MaxX,
        name: "max_x",
        aliases: &[],
        description: "Sınır kutusunun en büyük X'i (yukarı)",
        group: Group::Geometry,
        examples: &[("$max_x", "kutunun kuzey kenarının X'i")],
    },
    VarDef {
        var: Var::Width,
        name: "genişlik",
        aliases: &["width"],
        description: "Sınır kutusunun genişliği, Y yönünde (m)",
        group: Group::Geometry,
        examples: &[("$genişlik", "20")],
    },
    VarDef {
        var: Var::Height,
        name: "yükseklik",
        aliases: &["height"],
        description: "Sınır kutusunun yüksekliği, X yönünde (m)",
        group: Group::Geometry,
        examples: &[("$yükseklik", "30")],
    },
    VarDef {
        var: Var::Index,
        name: "sıra",
        aliases: &["row_number"],
        description: "Bu çalıştırmadaki sırası: 1, 2, 3 …",
        group: Group::Variables,
        examples: &[("'P' || doldur($sıra, 5)", "'P00001', 'P00002' …")],
    },
    VarDef {
        var: Var::Id,
        name: "id",
        aliases: &[],
        description: "Nesne numarası",
        group: Group::Variables,
        examples: &[("$id", "7")],
    },
    VarDef {
        var: Var::Scale,
        name: "ölçek",
        aliases: &["scale"],
        description: "Çizim ölçeğinin paydası (1/1000 için 1000); yalnızca sembol çizilirken. Metreyi kâğıt mm’sine çevirir: m × 1000 / $ölçek",
        group: Group::Variables,
        examples: &[("3 * 1000 / $ölçek", "3 m, 1/1000 ölçekte kâğıtta 3 mm")],
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Func {
    Round,
    Text,
    Number,
    Int,
    Abs,
    Min,
    Max,
    Upper,
    Lower,
    Trim,
    Length,
    Substr,
    Pad,
    Replace,
    Contains,
    Starts,
    Ends,
    If,
    Empty,
    Coalesce,
}

pub struct FuncDef {
    pub func: Func,
    pub name: &'static str,
    /// English (QGIS) and spelling variants.
    pub aliases: &'static [&'static str],
    /// Least and most arguments (None: any number).
    pub arity: (usize, Option<usize>),
    pub signature: &'static str,
    pub description: &'static str,
    /// Where the expression builder lists it.
    pub group: Group,
    /// Each argument as the signature names it, and what it is.
    pub args: &'static [(&'static str, &'static str)],
    /// Expressions that call it, each with what it gives.
    pub examples: &'static [(&'static str, &'static str)],
}

pub static FUNCTIONS: &[FuncDef] = &[
    FuncDef {
        func: Func::Round,
        name: "yuvarla",
        aliases: &["round"],
        arity: (1, Some(2)),
        signature: "yuvarla(sayı, basamak)",
        description: "Verilen ondalık basamağa yuvarlar: yuvarla(12.345, 2) → 12.35",
        group: Group::Math,
        args: &[
            ("sayı", "Yuvarlanacak sayı"),
            (
                "basamak",
                "Virgülden sonra kalacak basamak, 0–12; verilmezse 0",
            ),
        ],
        examples: &[
            ("yuvarla(12.345, 2)", "12.35"),
            ("yuvarla(1.005, 2)", "1.01"),
            ("yuvarla($alan)", "alan, tam sayıya"),
        ],
    },
    FuncDef {
        func: Func::Text,
        name: "metin",
        aliases: &["to_string", "text", "format_number"],
        arity: (1, Some(2)),
        signature: "metin(değer, basamak)",
        description: "Metne çevirir; basamak verilirse sabit ondalıkla: metin(452.1, 2) → \"452.10\"",
        group: Group::Conversions,
        args: &[
            ("değer", "Metne çevrilecek değer"),
            ("basamak", "Verilirse sayı bu kadar ondalıkla yazılır, 0–12"),
        ],
        examples: &[
            ("metin(452.1, 2)", "'452.10'"),
            ("metin($alan, 2) || ' m²'", "'600.00 m²'"),
            ("metin(doğru)", "'doğru'"),
        ],
    },
    FuncDef {
        func: Func::Number,
        name: "sayı",
        aliases: &["to_real", "to_number", "number"],
        arity: (1, Some(1)),
        signature: "sayı(metin)",
        description: "Metindeki sayıyı okur; sayı değilse boş",
        group: Group::Conversions,
        args: &[("metin", "Sayısı okunacak metin; ondalık ayırıcı nokta")],
        examples: &[("sayı('452.13')", "452.13"), ("sayı('abc')", "boş")],
    },
    FuncDef {
        func: Func::Int,
        name: "tamsayı",
        aliases: &["int", "to_int", "tam"],
        arity: (1, Some(1)),
        signature: "tamsayı(sayı)",
        description: "Ondalık kısmı atar",
        group: Group::Conversions,
        args: &[("sayı", "Ondalık kısmı atılacak sayı")],
        examples: &[("tamsayı(-2.7)", "-2"), ("tamsayı('12.9')", "12")],
    },
    FuncDef {
        func: Func::Abs,
        name: "mutlak",
        aliases: &["abs"],
        arity: (1, Some(1)),
        signature: "mutlak(sayı)",
        description: "Mutlak değer",
        group: Group::Math,
        args: &[("sayı", "Sayı")],
        examples: &[("mutlak(-5)", "5")],
    },
    FuncDef {
        func: Func::Min,
        name: "min",
        aliases: &["en_az", "enaz"],
        arity: (1, None),
        signature: "min(a, b, …)",
        description: "En küçük sayı",
        group: Group::Math,
        args: &[(
            "a, b, …",
            "Karşılaştırılacak sayılar; biri sayı değilse sonuç boş",
        )],
        examples: &[("min(3, 7, 1)", "1")],
    },
    FuncDef {
        func: Func::Max,
        name: "max",
        aliases: &["en_çok", "encok"],
        arity: (1, None),
        signature: "max(a, b, …)",
        description: "En büyük sayı",
        group: Group::Math,
        args: &[(
            "a, b, …",
            "Karşılaştırılacak sayılar; biri sayı değilse sonuç boş",
        )],
        examples: &[("max(3, 7, 1)", "7")],
    },
    FuncDef {
        func: Func::Upper,
        name: "büyük",
        aliases: &["upper"],
        arity: (1, Some(1)),
        signature: "büyük(metin)",
        description: "Büyük harfe çevirir (Türkçe: i → İ)",
        group: Group::Text,
        args: &[("metin", "Çevrilecek metin")],
        examples: &[("büyük('kadıköy')", "'KADIKÖY'")],
    },
    FuncDef {
        func: Func::Lower,
        name: "küçük",
        aliases: &["lower"],
        arity: (1, Some(1)),
        signature: "küçük(metin)",
        description: "Küçük harfe çevirir (Türkçe: I → ı)",
        group: Group::Text,
        args: &[("metin", "Çevrilecek metin")],
        examples: &[("küçük('IŞIK')", "'ışık'")],
    },
    FuncDef {
        func: Func::Trim,
        name: "kırp",
        aliases: &["trim"],
        arity: (1, Some(1)),
        signature: "kırp(metin)",
        description: "Baştaki ve sondaki boşlukları siler",
        group: Group::Text,
        args: &[("metin", "Kırpılacak metin")],
        examples: &[("kırp('  Arsa ')", "'Arsa'")],
    },
    FuncDef {
        func: Func::Length,
        name: "uzunluk",
        aliases: &["length", "len"],
        arity: (1, Some(1)),
        signature: "uzunluk(metin)",
        description: "Metnin karakter sayısı (nesne uzunluğu için $uzunluk)",
        group: Group::Text,
        args: &[("metin", "Karakterleri sayılacak metin")],
        examples: &[("uzunluk('Ağaç')", "4")],
    },
    FuncDef {
        func: Func::Substr,
        name: "parça",
        aliases: &["substr", "parca"],
        arity: (2, Some(3)),
        signature: "parça(metin, başlangıç, uzunluk)",
        description: "Metnin bir parçası; ilk karakter 1: parça(\"P00012\", 2, 3) → \"000\"",
        group: Group::Text,
        args: &[
            ("metin", "Parçası alınacak metin"),
            ("başlangıç", "İlk karakterin sırası; ilk karakter 1"),
            ("uzunluk", "Kaç karakter; verilmezse sonuna kadar"),
        ],
        examples: &[
            ("parça('P00012', 2, 3)", "'000'"),
            ("parça('1245/12', 6)", "'12'"),
        ],
    },
    FuncDef {
        func: Func::Pad,
        name: "doldur",
        aliases: &["lpad"],
        arity: (2, Some(3)),
        signature: "doldur(değer, uzunluk, karakter)",
        description: "Soldan doldurur: doldur(12, 5, \"0\") → \"00012\" (karakter verilmezse 0)",
        group: Group::Text,
        args: &[
            ("değer", "Doldurulacak değer"),
            ("uzunluk", "Varılacak uzunluk"),
            ("karakter", "Doldurma karakteri; verilmezse 0"),
        ],
        examples: &[
            ("doldur(12, 5)", "'00012'"),
            ("doldur(Parsel, 4, '_')", "'__12'"),
            ("'P' || doldur($sıra, 5)", "'P00001', 'P00002' …"),
        ],
    },
    FuncDef {
        func: Func::Replace,
        name: "değiştir",
        aliases: &["replace", "degistir"],
        arity: (3, Some(3)),
        signature: "değiştir(metin, aranan, yeni)",
        description: "Metindeki her aranan parçayı yenisiyle değiştirir",
        group: Group::Text,
        args: &[
            ("metin", "Değişecek metin"),
            ("aranan", "Aranan parça"),
            ("yeni", "Yerine gelecek metin"),
        ],
        examples: &[("değiştir('1245/12', '/', '-')", "'1245-12'")],
    },
    FuncDef {
        func: Func::Contains,
        name: "içerir",
        aliases: &["contains", "icerir"],
        arity: (2, Some(2)),
        signature: "içerir(metin, aranan)",
        description: "Metin aranan parçayı içeriyor mu (büyük/küçük harf ve Türkçe harf farkı gözetilmez)",
        group: Group::Text,
        args: &[
            ("metin", "İçinde aranacak metin"),
            ("aranan", "Aranan parça"),
        ],
        examples: &[
            ("içerir('Arsa', 'ARS')", "doğru"),
            ("içerir(Nitelik, 'bahçe')", "Nitelik Bahçe ise doğru"),
        ],
    },
    FuncDef {
        func: Func::Starts,
        name: "başlar",
        aliases: &["starts_with", "baslar"],
        arity: (2, Some(2)),
        signature: "başlar(metin, aranan)",
        description: "Metin aranan parçayla başlıyor mu (harf farkı gözetilmez)",
        group: Group::Text,
        args: &[("metin", "Metin"), ("aranan", "Başta aranan parça")],
        examples: &[("başlar('Çınar', 'cin')", "doğru")],
    },
    FuncDef {
        func: Func::Ends,
        name: "biter",
        aliases: &["ends_with"],
        arity: (2, Some(2)),
        signature: "biter(metin, aranan)",
        description: "Metin aranan parçayla bitiyor mu (harf farkı gözetilmez)",
        group: Group::Text,
        args: &[("metin", "Metin"), ("aranan", "Sonda aranan parça")],
        examples: &[("biter('pafta.DXF', '.dxf')", "doğru")],
    },
    FuncDef {
        func: Func::If,
        name: "eğer",
        aliases: &["if", "eger"],
        arity: (3, Some(3)),
        signature: "eğer(koşul, doğruysa, yanlışsa)",
        description: "Koşula göre iki değerden birini verir",
        group: Group::Conditionals,
        args: &[
            ("koşul", "Doğru ya da yanlış veren ifade"),
            ("doğruysa", "Koşul doğruysa değer"),
            ("yanlışsa", "Koşul yanlışsa değer"),
        ],
        examples: &[(
            "eğer($alan > 1000, 'büyük', 'küçük')",
            "'küçük' (alan 600 ise)",
        )],
    },
    FuncDef {
        func: Func::Empty,
        name: "boş",
        aliases: &["is_empty", "bos", "is_null"],
        arity: (1, Some(1)),
        signature: "boş(değer)",
        description: "Değer boş mu (alan yok ya da boş metin)",
        group: Group::Conditionals,
        args: &[("değer", "Denetlenecek değer")],
        examples: &[("boş(Ada)", "Ada yoksa ya da boşsa doğru")],
    },
    FuncDef {
        func: Func::Coalesce,
        name: "varsayılan",
        aliases: &["coalesce", "varsayilan"],
        arity: (2, None),
        signature: "varsayılan(a, b, …)",
        description: "Boş olmayan ilk değer",
        group: Group::Conditionals,
        args: &[("a, b, …", "Sırayla denenecek değerler")],
        examples: &[("varsayılan(Ada, '?')", "Ada boşsa '?'")],
    },
];

/// The lookup key of a name: Turkish-folded, white space removed.
fn key(name: &str) -> String {
    text::fold_turkish(name)
        .chars()
        .filter(|&c| !text::is_space(c))
        .collect()
}

pub fn find_function(name: &str) -> Option<&'static FuncDef> {
    let k = key(name);
    FUNCTIONS.iter().find(|f| {
        std::iter::once(f.name)
            .chain(f.aliases.iter().copied())
            .any(|n| key(n) == k)
    })
}

pub fn find_variable(name: &str) -> Option<&'static VarDef> {
    let k = key(name);
    VARIABLES.iter().find(|v| {
        std::iter::once(v.name)
            .chain(v.aliases.iter().copied())
            .any(|n| key(n) == k)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::functions::call;
    use crate::scalar::{R, Scratch, V};

    /// A function's value on some arguments, text made or borrowed alike.
    fn c(f: Func, args: &[V]) -> String {
        let mut out = String::new();
        match call(f, args, &mut out, &mut Scratch::default()) {
            R::V(V::Text(t)) => format!("t:{t}"),
            R::V(v) => format!("{v:?}"),
            R::Made => format!("t:{out}"),
            R::Thrown => "thrown".into(),
        }
    }

    #[test]
    fn functions_behave_as_the_typescript_did() {
        let (n, t) = (V::Num, V::Text);
        assert_eq!(c(Func::Round, &[n(12.345), n(2.0)]), "Num(12.35)");
        assert_eq!(c(Func::Round, &[n(1.005), n(2.0)]), "Num(1.01)");
        assert_eq!(c(Func::Text, &[n(452.1), n(2.0)]), "t:452.10");
        assert_eq!(c(Func::Pad, &[n(12.0), n(5.0)]), "t:00012");
        assert_eq!(c(Func::Pad, &[t("12"), n(4.0), t("_")]), "t:__12");
        assert_eq!(c(Func::Substr, &[t("P00012"), n(2.0), n(3.0)]), "t:000");
        assert_eq!(c(Func::Upper, &[t("kadıköy")]), "t:KADIKÖY");
        assert_eq!(c(Func::Lower, &[t("IŞIK")]), "t:ışık");
        assert_eq!(c(Func::Contains, &[t("Arsa"), t("ARS")]), "Bool(true)");
        assert_eq!(c(Func::Starts, &[t("Çınar"), t("cin")]), "Bool(true)");
        assert_eq!(c(Func::Starts, &[t("Arsa"), t("rs")]), "Bool(false)");
        assert_eq!(c(Func::Ends, &[t("Arsa"), t("rs")]), "Bool(false)");
        assert_eq!(c(Func::Ends, &[t("ARSA"), t("sa")]), "Bool(true)");
        assert_eq!(c(Func::Contains, &[t("Arsa"), t("rs")]), "Bool(true)");
        assert_eq!(c(Func::Max, &[n(1.0), t("12"), n(3.0)]), "Num(12.0)");
        assert_eq!(c(Func::Int, &[n(-2.7)]), "Num(-2.0)");
        assert_eq!(
            c(Func::Replace, &[t("1245/12"), t("/"), t("-")]),
            "t:1245-12"
        );
        assert_eq!(c(Func::Number, &[t("abc")]), "Null");
        assert_eq!(c(Func::Coalesce, &[V::Null, t(""), t("yok")]), "t:yok");
        assert_eq!(c(Func::Pad, &[t("x"), n(1e12)]), "thrown");
        assert!(find_function("YUVARLA").is_some() && find_function("en_çok").is_some());
        assert!(find_variable("CEVRE").is_some() && find_variable("nope").is_none());
    }
}
