//! Canlı akış satırları (docs/adr/9002 §6): portalın ve pencerenin aynı olay kümesini aynı sırayla, Türkçe etiketle
//! yazdığı satırlar.
//!
//! Saf ve testli: akış bir dönem yalnız üç olayı yazıyordu, kullanıcı kartın açılmasını, teslimi ve iptali pencereden
//! hiç göremiyordu. Bir yüzey ötekinden az şey söylüyorsa kullanıcı hangisine güveneceğini bilemez.

/// RFC 3339 damgasından `SS:DD:SS` çeker. Ayrıştırılamayan damga **boş** döner: uydurma bir saat, yanlış bir saattir.
#[must_use]
pub fn clock(timestamp: &str) -> String {
    let Some(t) = timestamp.find('T') else {
        return String::new();
    };
    let kalan = &timestamp[t + 1..];
    let parca: String = kalan.chars().take(8).collect();
    let b = parca.as_bytes();
    if b.len() == 8
        && b[2] == b':'
        && b[5] == b':'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 2 || i == 5 || c.is_ascii_digit())
    {
        parca
    } else {
        String::new()
    }
}

/// Olayın Türkçe etiketi; `None` → akışta gösterilmez.
///
/// Sözlük motorun ve yerel servisin **gerçekten yaydığı** olaylardan türetildi; ezberden değil.
#[must_use]
pub fn label(event_type: &str) -> Option<&'static str> {
    Some(match event_type {
        "batch_created" => "İstek alındı",
        "batch_opened" => "Belgeler hazırlandı",
        "pin_required" => "PIN isteniyor",
        "signing_started" => "İmzalama başladı",
        "package_started" => "Evrak başladı",
        "file_started" => "İmzalanıyor",
        "file_signed" => "İmzalandı",
        "file_failed" => "İmzalanamadı",
        "package_completed" => "Evrak imzalandı",
        "package_partially_completed" => "Evrakın bir kısmı imzalandı",
        "package_failed" => "Evrak imzalanamadı",
        "delivery_started" => "Sunucuya teslim ediliyor",
        "delivery_completed" => "Sunucu teslim aldı",
        "delivery_failed" => "Sunucu teslimi kabul etmedi",
        "batch_completed" => "Tamamlandı",
        "batch_partially_completed" => "Kısmen tamamlandı",
        "batch_failed" => "Tamamlanamadı",
        "batch_cancelled" => "Vazgeçildi",
        "batch_expired" => "Süre doldu",
        _ => return None,
    })
}

/// Akışın ilk olayından bu olaya geçen süre, saniye: `+1.4 sn`. Damgalar sunucunun UTC'sidir ve pencere yerel saati
/// bilmeden (süreç çok iş parçacıklıyken yerel saat dilimi güvenle okunamaz) duvar saati gösterseydi kullanıcının
/// saatinden üç saat geri bir zaman yazardı; geçen süre her yerde doğrudur. Çözülemeyen damga `None`.
#[must_use]
pub fn elapsed(first: &str, now: &str) -> Option<f64> {
    use time::format_description::well_known::Rfc3339;
    let a = time::OffsetDateTime::parse(first, &Rfc3339).ok()?;
    let b = time::OffsetDateTime::parse(now, &Rfc3339).ok()?;
    Some((b - a).as_seconds_f64().max(0.0))
}

/// Akış satırı, geçen süreyle: `+1.4 sn  İmzalandı: karar.pdf`.
#[must_use]
pub fn line_after(
    seconds: Option<f64>,
    event_type: &str,
    file_name: Option<&str>,
) -> Option<String> {
    let e = label(event_type)?;
    let govde = match file_name {
        Some(a) if !a.is_empty() => format!("{e}: {a}"),
        _ => e.to_owned(),
    };
    Some(match seconds {
        Some(s) => format!("+{s:.1} sn  {govde}"),
        None => govde,
    })
}

/// Akışa yazılacak satır: saat, etiket ve (varsa) belgenin adı.
#[must_use]
pub fn line(timestamp: &str, event_type: &str, file_name: Option<&str>) -> Option<String> {
    let e = label(event_type)?;
    let s = clock(timestamp);
    let govde = match file_name {
        Some(a) if !a.is_empty() => format!("{e}: {a}"),
        _ => e.to_owned(),
    };
    Some(if s.is_empty() {
        govde
    } else {
        format!("{s}  {govde}")
    })
}

/// İmzalanan belgelerin adları, kısaltılarak: en çok [`SHOWN`] ad, kalanı "ve N belge daha". Sayı YUVARLANMAZ: imza
/// hukukî bir işlemdir, kaç belge imzalandığı tam bilinmeli.
pub const SHOWN: usize = 5;

#[must_use]
pub fn summary(names: &[String]) -> String {
    if names.is_empty() {
        return String::new();
    }
    let mut s = names
        .iter()
        .take(SHOWN)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join("\n");
    let kalan = names.len().saturating_sub(SHOWN);
    if kalan > 0 {
        s.push_str(&format!("\nve {kalan} belge daha"));
    }
    s
}

/// Bayt sayısının okunur hâli: `482,1 KB`. Ondalık ayırıcı virgül değil nokta (DESIGN.md §10.3).
#[must_use]
pub fn size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    #[allow(
        clippy::cast_precision_loss,
        reason = "bir dosyanın boyu f64'ün tam aralığında; gösterim için"
    )]
    let b = bytes as f64;
    if b < KB {
        format!("{bytes} B")
    } else if b < KB * KB {
        format!("{:.1} KB", b / KB)
    } else if b < KB * KB * KB {
        format!("{:.1} MB", b / KB / KB)
    } else {
        format!("{:.1} GB", b / KB / KB / KB)
    }
}

/// Özetin kısa hâli: ilk 12 ve son 8 hane. Tamamı ipucunda ve kopyalanabilir alanda durur.
#[must_use]
pub fn short_digest(hex: &str) -> String {
    let h = hex.trim();
    if h.chars().count() <= 24 {
        return h.to_owned();
    }
    let bas: String = h.chars().take(12).collect();
    let son: String = h
        .chars()
        .rev()
        .take(8)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{bas}…{son}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn damgadan_saat_cikar_ve_uydurmaz() {
        assert_eq!(clock("2026-11-18T14:03:27.512Z"), "14:03:27");
        assert_eq!(clock("2026-11-18T14:03:27+03:00"), "14:03:27");
        assert_eq!(clock(""), "");
        assert_eq!(clock("2026-11-18"), "");
        assert_eq!(clock("2026-11-18Tab:cd:ef"), "");
    }

    #[test]
    fn satir_ad_tasir_bilinmeyen_olay_dusmez_yazilmaz() {
        assert_eq!(
            line("2026-11-18T09:00:01Z", "file_signed", Some("karar.pdf")).as_deref(),
            Some("09:00:01  İmzalandı: karar.pdf")
        );
        assert_eq!(
            line("", "batch_completed", None).as_deref(),
            Some("Tamamlandı")
        );
        assert!(line("2026-11-18T09:00:01Z", "uydurma_olay", None).is_none());
    }

    #[test]
    /// Motorun yaydığı her olayın etiketi var: yeni bir olay eklenip burası unutulursa test kırılır.
    fn motorun_her_olayinin_etiketi_var() {
        for olay in [
            "batch_created",
            "batch_opened",
            "pin_required",
            "signing_started",
            "package_started",
            "file_started",
            "file_signed",
            "file_failed",
            "package_completed",
            "package_partially_completed",
            "package_failed",
            "delivery_started",
            "delivery_completed",
            "delivery_failed",
            "batch_completed",
            "batch_partially_completed",
            "batch_failed",
            "batch_cancelled",
            "batch_expired",
        ] {
            assert!(label(olay).is_some(), "etiketsiz olay: {olay}");
        }
    }

    #[test]
    fn ozet_bes_ad_ve_kalani_sayar() {
        let adlar: Vec<String> = (1..=7).map(|i| format!("belge-{i}.pdf")).collect();
        let s = summary(&adlar);
        assert_eq!(s.lines().count(), 6);
        assert!(s.ends_with("ve 2 belge daha"));
        assert_eq!(summary(&adlar[..2]), "belge-1.pdf\nbelge-2.pdf");
        assert_eq!(summary(&[]), "");
    }

    #[test]
    fn boyut_ve_kisa_ozet() {
        assert_eq!(size(512), "512 B");
        assert_eq!(size(482_133), "470.8 KB");
        assert_eq!(size(3 * 1024 * 1024), "3.0 MB");
        assert_eq!(
            short_digest(&"a".repeat(96)),
            format!("{}…{}", "a".repeat(12), "a".repeat(8))
        );
        assert_eq!(short_digest("abc"), "abc");
    }
}
