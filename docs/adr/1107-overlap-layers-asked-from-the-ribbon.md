# 1107 — Şeritten “Seçili katmanlarda önle” katmanları hemen sorar

Tarih: 2026-11-20. gis-usage-report.md GIS-050. Durum: kabul edildi.

İlgili: ADR 0162 (Çakışma denetimi), ADR 0622 §9 (alan kuralları).

## Sorun

Veri › Çakışma › Seçili katmanlarda önle şeritten seçildiğinde katman listesi açılmadı; boş kapsam ancak komut
geçmişinde söylendi. Bitişik alan aynı satırda “alan olduğu gibi yazıldı” deyip hemen ardından “Reddedildi, hiçbir şey
yazılmadı” yazdı. Katmanlar yalnız durum çubuğunun sağ tık menüsünden seçilebiliyordu.

## Karar

1. Şeritten (komut `draft.overlap.layers`) seçildiğinde kip açılır; çizimde var olan seçili katman yoksa “Seçili
   katmanlarda önle” penceresi açılır: çizimin katmanları onay kutularıyla, üstte seçili sayı ve kipin ne yaptığı;
   hiç seçilmemişse “Katman seçilmedi: yeni alanlar hiçbir katmanla çakışma denetiminden geçmez”. Tamam çizime döner.
   Katmanlar seçiliyse bir şey sorulmaz.
2. Durum çubuğu menüsündeki aynı seçenek pencere açmaz (katmanlar menünün içinde).
3. Boş kapsamda çizim aracının uyarısı yazım sonucunu önceden söylemez: “katmanlar arası çakışma denetlenmedi”; yazılıp
   yazılmadığını komutun kendi sonucu söyler.

## Kanıt

`apps/desktop/src/tests/overlap_tests.rs` `the_ribbon_s_layered_mode_asks_its_layers`: katmansızken pencere açılır,
iki katman işaretlenince “2 katman seçili”, Tamam kapatır, katmanlar varken yeniden seçmek bir şey sormaz; durum
çubuğu menüsü testi değişmeden geçer. Resim `.run/gui-evidence/cbs-kullanim/gis-050-katmanlar.png`.
