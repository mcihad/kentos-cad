# 1133 — Nokta tablosunun düzenlenebilir hücresi bütün alanıyla tıklanır

Tarih: 2026-11-20. gis-usage-report.md GIS-062. Durum: kabul edildi.

İlgili: ADR 0153 (Nokta editörü), ADR 1540 (tek tıkla ad/kod).

## Sorun

CBS çiziminin Noktalar sekmesinde Kod hücresinin yazısız kısmına çift tıklamak satırı seçti, düzenleyici açmadı; boş Z
hücresinin ortasında ve kenarında da düzenleyici açılmadı. Tam hücre sarmalayıcısı yalnız Harita CAD projesinde
kuruluyordu; öbür projelerde tıklama alanı yazının kendisi kadardı.

## Karar

Her düzenlenebilir hücre (Ad, Y, X, Z, Kod, Katman) proje türünden bağımsız olarak satır yüksekliği ve sütun
genişliğince tıklanır; tek tık ya da çift tık kuralı eskisi gibi (seçili tek noktanın Ad ve Kod'u tek tık, Harita
CAD'in ölçü hücreleri tek tık, öbürleri çift tık). Sıra sütununun çift tıkla yakınlaştırması korunur. Boş kota 0
yazılınca kot 0'dır (bilinmeyen kot boş kalır).

## Kanıt

`apps/desktop/src/gis/points/tests.rs` `a_double_click_anywhere_in_a_cell_edits_it`: gerçek tıklamalarla Z sütununun
ortasına çift tık kotu düzenler, 0 yazılınca kot Some(0); kısa kodun sağındaki boşluğa çift tık Kod'u düzenler. Test
düzeltme geri alınınca Kod adımında kırılır (denendi).
