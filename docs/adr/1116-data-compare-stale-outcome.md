# 1116 — Veri karşılaştırmanın sonucu girdiler değişince eski sayılır

Tarih: 2026-11-20. gis-usage-report.md GIS-056. Durum: kabul edildi.

İlgili: ADR 0210 §8 (Veri karşılaştır, tarihler).

## Sorun

0,001 m toleransla karşılaştırıp tolerans 2 m yapılınca, yeniden Karşılaştır'a basılmadan kaydedilen CSV 0,400 m
kayan parseli hâlâ “Geometrisi değişti” diye yazdı; ekran 2 m derken rapor eski ayarlarındı. Kopyala, Kaydet ve
Çizime yaz etkin kaldı.

## Karar

Pencere sonucun hangi girdilerle hesaplandığını izler: eski/yeni veri ve dosya, katmanlar, eşleme, anahtar alan,
arama mesafesi, tolerans, tarihler ve yok sayılan alanlar değişince sonuç “eski” olur. Eski sonuç gösterilmeye devam
eder ama üstünde “Ayarlar değişti: aşağıdaki sonuç önceki ayarlarla…” yazar; Kopyala, Kaydet ve Çizime yaz kapanır
(komut yolundan gelse de reddedilir). Karşılaştır sonucu tazeler. “Yalnız farklar” yalnız gösterimi süzdüğü için
sonucu eskitmez.

## Kanıt

`apps/desktop/src/gis/data_compare.rs` `a_changed_tolerance_makes_the_outcome_stale_until_compared_again`: 0,4 m
kayan parsel 0,001 m'de farklı; Yalnız farklar tazeliği bozmaz; tolerans 2 m olunca eski, Çizime yaz reddedilir;
yeniden karşılaştırınca parsel fark değildir.
