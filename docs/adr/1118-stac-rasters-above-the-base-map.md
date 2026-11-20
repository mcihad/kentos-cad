# 1118 — STAC rasterleri altlığın üstüne eklenir

Tarih: 2026-11-20. gis-usage-report.md GIS-059. Durum: kabul edildi (kısmi).

İlgili: ADR 0644 (STAC'ta ara, Hazırla), ADR 0208 §15 (servis katmanının yeri).

## Sorun

İki opak OGC harita servisi olan projede STAC'tan hazırlanan COG başarıyla eklendi ama haritada görünmedi: yeni grup
ağacın sonuna (her şeyin altında çizilen yere) eklendiği için iki altlığın altında kaldı; kullanıcı altlıkların gözünü
kapatınca gördü.

## Karar

`cad.stac.prepare` grubu ağacın üst düzeyinde ilk altlığın (bir harita servisi katmanı ya da onu tutan grup) hemen
üstüne koyar (`stac_prepare::base_index`): üstteki çizim katmanları üstte kalır, altlıklar altta. Altlık yoksa yeni
grup eskisi gibi en alta gider. Masaüstü sonucu söylerken grubun hangi altlığın üstünde çizildiğini de yazar.

## Sınır

Kullanıcının kendi özel sıralamasında yeni rasteri örten başka bir katmanı adıyla belirtip “Üstüne taşı” önermek
yapılmadı; hedef grup ve çizim sırası pencerede önceden gösterilmiyor (sonuç iletisinde söyleniyor).

## Kanıt

`apps/desktop/src/gis/rasters/stac.rs` `the_rasters_go_above_the_base_map`: ağacın başında opak bir XYZ altlığı varken
hazırlanan grup onun üstünde; ileti “grubu “Altlık” altlığının üstünde çizilir” der. Paylaşılan örnekler (altlıksız)
değişmedi.
