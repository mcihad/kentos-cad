# 1081 — İşlemler araması: adıyla bulunanlar önce, tam ad en üstte

Tarih: 2026-11-20. gis-usage-report.md GIS-015. Durum: kabul edildi.

İlgili: [ADR 0802](0802-strict-module-separation.md) (sunulan araçlar).

## Sorun

İşlemler'de “En yakın tesis” aranınca ilk sonuç “En yakın merkeze bağla” (merkezlere düz çizgi) oldu; adla birebir
eşleşen ağ aracı “En yakın tesis” Ağ analizi grubunda aşağıda kaldı. İki araç da çalıştığı için yanlış hesap başarı
bildirimiyle teslim edilebiliyordu.

## Karar

1. `Registry::named(query)`: sorgunun her sözcüğü aracın adında (Türkçe harfler katlanarak) ya da sorgu bir komut
   satırı adıyla aynıysa; sıra: tam ad ya da komut adı, adı sorguyla başlayanlar, adında sözcükleri geçenler (adlarının
   sırasıyla).
2. Panel, arama doluyken bunları en üstte “Adıyla bulunanlar” grubunda gösterir; anahtar sözcük ya da açıklamayla
   bulunanlar altında kendi kategorilerinde kalır (aynı araç iki yerde görünmez). Arama boşken kategori ağacı aynıdır.

## Sınama

- `processing::tests::a_search_puts_the_tool_named_exactly_first`: “En yakın tesis” (ve “en yakin tesis”) için ilk
  araç `network.closestFacility`, “En yakın merkeze bağla” için `proximity.hub`. Resim `.run/gui-evidence/gis-015/`.
