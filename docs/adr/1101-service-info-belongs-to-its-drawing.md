# 1101 — Servis bilgisi sonucu sorulduğu çizimindir

Tarih: 2026-11-20. gis-usage-report.md GIS-044. Durum: kabul edildi.

İlgili: ADR 0208 (harita servisleri), ADR 1085 (GIS-022/024: belge değişince formlar).

## Sorun

WMS'li projede Servis bilgisi sorulduktan sonra başka bir proje açılınca alt panel önceki projenin noktasını ve
kayıtlarını göstermeyi sürdürdü; iki projede aynı adlı “Parseller” katmanı varken bilgi yanlış parsele atfedilebilirdi.
Ağdan geç gelen cevap da yeni belgeye yazılabiliyordu.

## Karar

1. Her soru bir kuşak numarası alır (`Info::asked`); iş parçacığından dönen cevap (`Event::Info(asked, …)`) yalnız hâlâ
   o soruysa yazılır. Başka bir nokta sorulmuşsa ya da başka çizim açılmışsa geç cevap düşer.
2. Çizim değişince (`show_document`) Servis bilgisinin noktası, cevapları, süren seçimi ve “bekleniyor” durumu silinir,
   kuşak bir artar (`leave_service_info`).
3. Her cevap katmanın adının yanında çizimdeki kimliğini taşır; çizimde artık olmayan katmanın cevabı gösterilmez
   (ad başka çizimde de olabilir, kimlik çizimindir).

## Kanıt

`apps/desktop/src/shell/services/info.rs` `another_drawing_never_shows_the_last_ones_answers`: kendi sorusunun cevabı
gösterilir; başka nokta sorulduktan sonra gelen eski cevap düşer; başka çizim açılınca panel boşalır, bekleme biter,
ardından gelen cevap yeni çizime yazılmaz.
