# 1079 — Ağın kenarları çizgi tutan katmandan: öneri ve kayıttan önce ret

Tarih: 2026-11-20. gis-usage-report.md GIS-013. Durum: kabul edildi.

İlgili: [ADR 0209](0209-network-analysis.md) (ağ analizi), [ADR 0622](0622-interface-arrangement.md) §5 (katman türü).

## Sorun

Etkin katman yalnız bir poligonlu “Park koruma 10 m” iken Analiz › Ağlar › Yeni ağ bu alan katmanını kenar olarak
önerdi; Kaydet “Ağlar kaydedildi” dedi ve ağ sıfır kenarla kaldı. Neden ancak En kısa yol ilk tıklamada komut
geçmişinde göründü.

## Dayanak

- QGIS ağ analizi araçları ([belgesi](https://docs.qgis.org/3.44/en/docs/user_manual/processing_algs/qgis/networkanalysis.html),
  20 Kasım 2026'da okundu): ağ katmanı girdisi yalnız çizgi katmanı kabul eder.

## Karar

1. Kenar katmanı uygunluğu (`apps/desktop/src/gis/networks/edge_fit.rs`): türü Çizgi olan katman uygundur, boşsa
   şablondur (henüz çizgi girilmemiş saha katmanı kaydedilebilir); türü Alan, Nokta ya da Açıklama olan uygun değildir;
   türsüz katman çizgi (çizgi, çoklu çizgi, yay) tutuyorsa uygundur (öbür nesneleri ağ almaz), hiç tutmuyorsa ama
   nesnesi varsa uygun değildir, boşsa şablondur.
2. Yeni ağın kenarı: etkin katman çizgi tutuyorsa o; değilse ilk çizgi tutan Çizgi katmanı, sonra ilk çizgi tutan
   katman, sonra ilk boş Çizgi katmanı; hiçbiri yoksa boş (kullanıcı seçer).
3. Kaydet hiçbir şey yazmadan önce her ağın kenar katmanlarını denetler; uygun olmayan ilkinde ağın ve katmanın adıyla,
   nedeni ve yapılacağıyla durur.

## Sınama

- `networks::edge_fit_tests`: etkin alan katmanı varken öneri yürüyüş yolu; alan katmanı ve çizgisiz türsüz katman
  nedeniyle reddedilir, hiçbir ağ yazılmaz; boş çizgi şablonu ve çizgili karma katman kaydedilir. Resim
  `.run/gui-evidence/gis-013/`.
