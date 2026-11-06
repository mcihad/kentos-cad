# ADR 1026: Tablo hücreleri, yüzü SHX yazı stiliyse fontun çizgileriyle yazılır

- **Durum:** kabul edildi (2026-11-06). Kapsam: plan/cad-advanced.md `CAD-ADV-11`, ADR 1125 §3'ün son sınırı (“tablo
  hücreleri yüzle çizilir”). Şema değişmez: tablonun yüzü (`textStyle`) ve tablo stilinin veri hücresi yazı stili
  (ADR 1126) zaten vardı.

## 1. Karar

- Tablonun çizilen yüzü (`kentos_contracts::styled`: kendi yüzü, yoksa tablo stilinin veri hücresinin yazı stili)
  gömülü SHX dosyası adlandıran bir yazı stiliyse, her hücre satırı sol tabanından, hücre stilinin yüksekliğinde
  fontun çizgileriyle çizilir: ekran, PDF ve CPU baskısı (`shx_text::words_in`). Kalın başlık satırı SHX'te kalın
  değildir (SHX fontunun kalınlığı yoktur; AutoCAD de çizgi kalınlığıyla basar).
- Denetim: fontun çizemediği ilk hücre adıyla (`n. satır m. sütun`) `text_shx_unsupported` (hata; PDF reddedilir); font
  gömülü değilse adlandırılmış `text_font_substituted`.

## 2. Kanıt

`kentos-plot` `shx_texts::a_tables_cells_are_written_in_its_shx_face`: “SU” hücresinin çizgileri bağımsız başvurunun
“Sol taban” durumunun hücrenin yerine kaydırılmışıdır (1e-9), tablonun kalınlığında; “SAU” hücresi
`text_shx_unsupported` (U+0041, 1. satır 1. sütun) ve PDF reddi. Tablo stili testleri (`table_styles`) değişmeden
geçer. Resim `shx-olcu-cizim` (dört hücreli SHX tablo, ölçü, kılavuz ve blok yazılarıyla), bakıldı.

## 3. Sınırlar

Hücre başına farklı yazı stili yoktur (tablonun tek yüzü); seçim kutusu TrueType genişliğindedir.
