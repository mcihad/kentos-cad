# 1095 — Vektör oturtmanın kapsamı hedef kontrol noktalarını taşımaz

Tarih: 2026-11-20. gis-usage-report.md GIS-038. Durum: kabul edildi.

İlgili: ADR 0158 (Vektör oturtma), CLAUDE.md §7 (veri doğruluğu).

## Sorun

Hiçbir nesne seçili değilken pencere Uygulanacak nesneleri “Tümü”ye çeviriyordu. Kontrol noktalı Helmert'te Tümü
hedef (TUREF) noktalarını da içerdiğinden hedefler de dönüşümle milyonlarca metre taşındı; kayıp sessizdi.

## Karar

1. Seçim boşsa ve yöntem Kontrol noktalarıysa kapsam **Katman**, katman **kaynak noktalarının katmanı**dır (yerel ölçü
   ile birlikte ölçülen nesneler). Kaynak katmanı değişince Katman da onu izler (kullanıcı başka katman seçmediyse).
   Kaynak katmanı yoksa ya da Parametrelerle yönteminde eski davranış (Tümü) sürer.
2. Kontrol noktalı yöntemde **Tümü hedef katmanı içermez** (kaynakla aynı değilse); sayısı da (“Tümü (10)”) onsuz
   verilir. Hedef katmanı taşımak isteyen onu Katman ya da seçimle açıkça seçer.
3. Uygula'nın üstünde **taşınacak katmanlar ve nesne sayıları** yazılır (“Taşınacak: “Yerel ölçü” 10 nesne; hedef
   katman “TUREF noktaları” yerinde kalır.”). Hedef katman kapsamdaysa ayrıca uyarı: “Hedef kontrol katmanı … de
   taşınır: hedef noktaların koordinatları değişir.”
4. Dönüşüm yine `cad.entities.transform` ile tek geri alma adımıdır (Oturt).

## Kanıt

`apps/desktop/src/gis/calc/fit/tests.rs` `the_default_fit_moves_the_source_and_never_the_targets`: açılışta kapsam
kaynak katmanı; Tümü hedefi dışarıda bırakır; hedef katman açıkça seçilince uyarı; Uygula sonrası altı hedef nokta
birebir aynı, kaynak üstlerine oturur; geri alma kaynağı geri getirir. Resim
`.run/gui-evidence/cbs-kullanim/gis-038-oturtma-kapsam.png`.
