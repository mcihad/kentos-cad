//! Functions and variables of the expression language (the TypeScript's
//! `EXPR_FUNCTIONS` and `EXPR_VARIABLES`). Names are matched
//! Turkish-insensitively (yuvarla = YUVARLA, $çevre = $cevre); every
//! function also answers to its English (QGIS) name. The menus, the help
//! and the field's completion read these tables; what a function computes
//! is `scalar::call`.

use crate::js::text;

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
}

pub static VARIABLES: &[VarDef] = &[
    VarDef {
        var: Var::Area,
        name: "alan",
        aliases: &[],
        description: "Alan (m²): kapalı alan (delikler düşülür), daire, tam elips, tarama",
    },
    VarDef {
        var: Var::Length,
        name: "uzunluk",
        aliases: &["çevre", "length", "perimeter"],
        description: "Uzunluk ya da çevre (m)",
    },
    VarDef {
        var: Var::Vertices,
        name: "köşe",
        aliases: &["vertices"],
        description: "Köşe sayısı (delikler dahil)",
    },
    VarDef {
        var: Var::Kind,
        name: "tür",
        aliases: &["type"],
        description: "Nesne türü: “Kapalı alan”, “Çizgi” …",
    },
    VarDef {
        var: Var::Layer,
        name: "katman",
        aliases: &["layer"],
        description: "Katman adı",
    },
    VarDef {
        var: Var::Label,
        name: "etiket",
        aliases: &["label"],
        description: "Çizimde görünen etiket (parsel no, nokta adı)",
    },
    VarDef {
        var: Var::Y,
        name: "y",
        aliases: &[],
        description: "Y (sağa): nesnenin yer noktası",
    },
    VarDef {
        var: Var::X,
        name: "x",
        aliases: &[],
        description: "X (yukarı): nesnenin yer noktası",
    },
    VarDef {
        var: Var::CentroidY,
        name: "merkez_y",
        aliases: &[],
        description: "Ağırlık merkezinin Y'si (sağa): kapalı alanda, taramada, dairede ve tam elipste alanın merkezi (delikler düşülür); öbür nesnelerde yer noktası",
    },
    VarDef {
        var: Var::CentroidX,
        name: "merkez_x",
        aliases: &[],
        description: "Ağırlık merkezinin X'i (yukarı): kapalı alanda, taramada, dairede ve tam elipste alanın merkezi (delikler düşülür); öbür nesnelerde yer noktası",
    },
    VarDef {
        var: Var::MinY,
        name: "min_y",
        aliases: &[],
        description: "Sınır kutusunun en küçük Y'si (sağa)",
    },
    VarDef {
        var: Var::MaxY,
        name: "max_y",
        aliases: &[],
        description: "Sınır kutusunun en büyük Y'si (sağa)",
    },
    VarDef {
        var: Var::MinX,
        name: "min_x",
        aliases: &[],
        description: "Sınır kutusunun en küçük X'i (yukarı)",
    },
    VarDef {
        var: Var::MaxX,
        name: "max_x",
        aliases: &[],
        description: "Sınır kutusunun en büyük X'i (yukarı)",
    },
    VarDef {
        var: Var::Width,
        name: "genişlik",
        aliases: &["width"],
        description: "Sınır kutusunun genişliği, Y yönünde (m)",
    },
    VarDef {
        var: Var::Height,
        name: "yükseklik",
        aliases: &["height"],
        description: "Sınır kutusunun yüksekliği, X yönünde (m)",
    },
    VarDef {
        var: Var::Index,
        name: "sıra",
        aliases: &["row_number"],
        description: "Bu çalıştırmadaki sırası: 1, 2, 3 …",
    },
    VarDef {
        var: Var::Id,
        name: "id",
        aliases: &[],
        description: "Nesne numarası",
    },
    VarDef {
        var: Var::Scale,
        name: "ölçek",
        aliases: &["scale"],
        description: "Çizim ölçeğinin paydası (1/1000 için 1000); yalnızca sembol çizilirken. Metreyi kâğıt mm’sine çevirir: m × 1000 / $ölçek",
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
}

pub static FUNCTIONS: &[FuncDef] = &[
    FuncDef {
        func: Func::Round,
        name: "yuvarla",
        aliases: &["round"],
        arity: (1, Some(2)),
        signature: "yuvarla(sayı, basamak)",
        description: "Verilen ondalık basamağa yuvarlar: yuvarla(12.345, 2) → 12.35",
    },
    FuncDef {
        func: Func::Text,
        name: "metin",
        aliases: &["to_string", "text", "format_number"],
        arity: (1, Some(2)),
        signature: "metin(değer, basamak)",
        description: "Metne çevirir; basamak verilirse sabit ondalıkla: metin(452.1, 2) → \"452.10\"",
    },
    FuncDef {
        func: Func::Number,
        name: "sayı",
        aliases: &["to_real", "to_number", "number"],
        arity: (1, Some(1)),
        signature: "sayı(metin)",
        description: "Metindeki sayıyı okur; sayı değilse boş",
    },
    FuncDef {
        func: Func::Int,
        name: "tamsayı",
        aliases: &["int", "to_int", "tam"],
        arity: (1, Some(1)),
        signature: "tamsayı(sayı)",
        description: "Ondalık kısmı atar",
    },
    FuncDef {
        func: Func::Abs,
        name: "mutlak",
        aliases: &["abs"],
        arity: (1, Some(1)),
        signature: "mutlak(sayı)",
        description: "Mutlak değer",
    },
    FuncDef {
        func: Func::Min,
        name: "min",
        aliases: &["en_az", "enaz"],
        arity: (1, None),
        signature: "min(a, b, …)",
        description: "En küçük sayı",
    },
    FuncDef {
        func: Func::Max,
        name: "max",
        aliases: &["en_çok", "encok"],
        arity: (1, None),
        signature: "max(a, b, …)",
        description: "En büyük sayı",
    },
    FuncDef {
        func: Func::Upper,
        name: "büyük",
        aliases: &["upper"],
        arity: (1, Some(1)),
        signature: "büyük(metin)",
        description: "Büyük harfe çevirir (Türkçe: i → İ)",
    },
    FuncDef {
        func: Func::Lower,
        name: "küçük",
        aliases: &["lower"],
        arity: (1, Some(1)),
        signature: "küçük(metin)",
        description: "Küçük harfe çevirir (Türkçe: I → ı)",
    },
    FuncDef {
        func: Func::Trim,
        name: "kırp",
        aliases: &["trim"],
        arity: (1, Some(1)),
        signature: "kırp(metin)",
        description: "Baştaki ve sondaki boşlukları siler",
    },
    FuncDef {
        func: Func::Length,
        name: "uzunluk",
        aliases: &["length", "len"],
        arity: (1, Some(1)),
        signature: "uzunluk(metin)",
        description: "Metnin karakter sayısı (nesne uzunluğu için $uzunluk)",
    },
    FuncDef {
        func: Func::Substr,
        name: "parça",
        aliases: &["substr", "parca"],
        arity: (2, Some(3)),
        signature: "parça(metin, başlangıç, uzunluk)",
        description: "Metnin bir parçası; ilk karakter 1: parça(\"P00012\", 2, 3) → \"000\"",
    },
    FuncDef {
        func: Func::Pad,
        name: "doldur",
        aliases: &["lpad"],
        arity: (2, Some(3)),
        signature: "doldur(değer, uzunluk, karakter)",
        description: "Soldan doldurur: doldur(12, 5, \"0\") → \"00012\" (karakter verilmezse 0)",
    },
    FuncDef {
        func: Func::Replace,
        name: "değiştir",
        aliases: &["replace", "degistir"],
        arity: (3, Some(3)),
        signature: "değiştir(metin, aranan, yeni)",
        description: "Metindeki her aranan parçayı yenisiyle değiştirir",
    },
    FuncDef {
        func: Func::Contains,
        name: "içerir",
        aliases: &["contains", "icerir"],
        arity: (2, Some(2)),
        signature: "içerir(metin, aranan)",
        description: "Metin aranan parçayı içeriyor mu (büyük/küçük harf ve Türkçe harf farkı gözetilmez)",
    },
    FuncDef {
        func: Func::Starts,
        name: "başlar",
        aliases: &["starts_with", "baslar"],
        arity: (2, Some(2)),
        signature: "başlar(metin, aranan)",
        description: "Metin aranan parçayla başlıyor mu (harf farkı gözetilmez)",
    },
    FuncDef {
        func: Func::Ends,
        name: "biter",
        aliases: &["ends_with"],
        arity: (2, Some(2)),
        signature: "biter(metin, aranan)",
        description: "Metin aranan parçayla bitiyor mu (harf farkı gözetilmez)",
    },
    FuncDef {
        func: Func::If,
        name: "eğer",
        aliases: &["if", "eger"],
        arity: (3, Some(3)),
        signature: "eğer(koşul, doğruysa, yanlışsa)",
        description: "Koşula göre iki değerden birini verir",
    },
    FuncDef {
        func: Func::Empty,
        name: "boş",
        aliases: &["is_empty", "bos", "is_null"],
        arity: (1, Some(1)),
        signature: "boş(değer)",
        description: "Değer boş mu (alan yok ya da boş metin)",
    },
    FuncDef {
        func: Func::Coalesce,
        name: "varsayılan",
        aliases: &["coalesce", "varsayilan"],
        arity: (2, None),
        signature: "varsayılan(a, b, …)",
        description: "Boş olmayan ilk değer",
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
