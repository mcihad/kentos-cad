# 1090 — Baskı öncesi denetim atlas paftasını sayfa sayfa okur

Tarih: 2026-11-20. gis-usage-report.md GIS-031. Durum: kabul edildi.

İlgili: [ADR 0534](0534-atlas-pdf-pinned-revision.md) (atlas), [ADR 4000](4000-atlas-update-from-its-record.md).

## Sorun

Atlas PDF'e aktarıldıktan sonra Çıktı › Baskı öncesi tam denetim aynı paftada 7 hata bildirdi: iki harita
çerçevesinde merkez eksik, atlasın sayfa adı, kimliği ve sayfa sayısı gibi değişkenler çözümsüz. Aynı tanımla atlas
çıktısında iki harita da basılmış, adlar ve sayfa numaraları doğru çözülmüştü. Denetim paftayı atlas sayfası değil
tek pafta gibi okuyordu ve fark açıklanmıyordu.

## Dayanak

- QGIS tek PDF atlas çıktısı ([belgesi](https://docs.qgis.org/3.44/en/docs/user_manual/processing_algs/qgis/cartography.html)):
  atlas sayfaları kapsam nesnesinin bağlamıyla (harita yeri, alanlar) üretilir; denetim de o bağlamda anlamlıdır.

## Karar

1. Atlası olan pafta için denetim, atlası `kentos_plot::atlas::plan` ile (Atlası PDF yap'ın planıyla) planlar ve her
   sayfayı kendi bağlamında (`RenderInputs::atlas`, `AtlasPlanned::page_contexts`) okur: atlas haritalarının yeri ve
   `@atlas_` alanları sayfadan gelir. En çok 200 sayfa okunur, sayısı söylenir.
2. Yayın türü söylenir: “Atlas paftası: 100 sayfa, her biri kendi nesnesi ve alanlarıyla denetlendi.” (bilgi).
3. Bir bulgu bir kez, geçtiği sayfaların adlarıyla söylenir (ilk üçü ve kaç tane daha); herhangi bir sayfada hata ise
   hatadır. Gerçekten eksik bir kullanıcı değişkeni (ör. `@kaynak`) etkilediği sayfalarla hata verir.
4. Sabit paftalarda eksik merkez denetimi eskisi gibidir.

## Sınama

- `atlas_tests::the_preflight_reads_an_atlas_page_by_page`: atlas fikstürünün paftası 100 sayfa olarak okunur;
  `map_unplaced` ve çözümsüz `@atlas_` alanı yoktur; fikstürün gerçekten eksik `@kaynak` değeri sayfalarıyla hatadır.
