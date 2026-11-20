# 1105 — Tablonun katman seçicisi gösterdiği satırların katmanını söyler

Tarih: 2026-11-20. gis-usage-report.md GIS-048. Durum: kabul edildi.

İlgili: ADR 0527 (birleşik görünümler), ADR 1085 (GIS-022: belge değişince tablo).

## Sorun

Birleşik görünüm açıkken başka projeye geçilince tablo yeni projenin Bina katmanının satırlarını gösterirken Katman ▾
seçicisi “Çizimde katman yok” yazdı; Durum hücresi düzenlenip kaydedilebiliyordu.

## Karar

Seçicinin seçili satırı, seçili birleşik görünüm çizimde varsa odur; yoksa tablonun satırlarının katmanı. Görünüm
kimliği başka çizimin olsa da seçici boş kalmaz, satırlarla çelişmez. Belge değişince tablo zaten yeni çizimin
katmanıyla, görünümsüz başlar (ADR 1085); bu, görünüm silindiğinde de seçiciyi tutarlı tutar.

## Kanıt

`apps/desktop/src/gis/features/tests.rs` `the_layer_picker_and_the_rows_never_disagree`: çizimde olmayan bir görünüm
kimliğiyle Tablo açıkken seçici “Kadastro / Parsel (5)” der, “Çizimde katman yok” görünmez; başka çizim açılınca
görünüm silinir. Resim `.run/gui-evidence/cbs-kullanim/gis-048-tablo-katman.png`.
