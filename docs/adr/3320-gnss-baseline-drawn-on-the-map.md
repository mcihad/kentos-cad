# 3320 — GNSS bazının haritada çizilmesi

Tarih: 2026-11-09. plan/map-cad.md MAP-03, altıncı tur. Durum: taslak. ADR 2920'nin “Sonra”sındaki “bazın satırından
haritada bazın çizilmesi (iki uç arasında geçici çizgi)”.

İlgili:
- [ADR 1740](1740-gnss-network-from-archive.md): GNSS ağ dengelemesi;
- [ADR 2920](2920-archive-panel-baselines-and-adjustments.md): arşiv panelinde bazlar, bilinen noktalar ve
  dengelemeler;
- [ADR 3160](3160-pipe-crossings-on-the-drawing.md): komutun cevabının çizimde geçici işaretle gösterilmesi.

## Dayanak

- **Trimble Business Center** ([2024.10 sürüm notları](https://help.fieldsystems.trimble.com/tbc/pdf/tbc-2024.10-release-notes.pdf),
  [Observation vector arrows](https://community.trimble.com/blogs/matt-mclean1/2020/11/18/tip-of-the-week-62-observation-vector-arrows),
  9 Kasım 2026'da okundu):
  - GNSS vektörleri plan görünüşünde iki noktası arasında çizgi olarak durur;
  - grafik görünüşlerde yatay uzunluklarıyla etiketlenebilir;
  - gözlemin yönü okla gösterilir;
  - bir vektör çizgisi seçilince vektörün listesi açılır.

  Ölçmeci bazı haritada, uçlarıyla birlikte görür.
- **KentOS bugün** (ADR 2920): Bazlar satırı iki ucun kabul noktalarını seçer ama bazın kendisi haritada görünmez.
  Bazın yönü ve hangi noktaları bağladığı yalnız satırın sözlerindedir.

## Karar

1. **Bazın satırı** (GNSS arşivi › Bazlar):
   - iki ucun kabul noktalarını eskisi gibi seçer ve gösterir;
   - bazı haritada iki uç arasında geçici bir çizgi olarak bırakır.
2. **Çizgi:**
   - vurgu renginde kesikli, zeminden ayrılsın diye haleli;
   - uçlarında halka, bitişinde ok (bazın yönü, başından sonuna);
   - ortasında, çizginin solunda haleli yazı: “bz-5 · 3200.424 m” (arşivdeki uzunluk, vektörün uzunluğu).
3. **Uçlar** kabul noktalarının nesneleridir, çizimin bugünkü hâliyle:
   - nokta taşınırsa çizgi onunla gelir;
   - nokta silinirse çizilmez;
   - başka bir çizim açılınca kalkar.
4. **Ucu çizilemeyen baz:** bir ucun kabul noktası yoksa (örneğin bilinen noktanın kendi kabul noktası olmayan işareti)
   bulunan uç seçilir. Çizgi çizilmez, durum çubuğu söyler: “bz-1 çizilemedi: BN1 → N1 uçlarından birinin kabul noktası
   yok.”
5. **Kaldırma:** İşareti kaldır (`data.unmark`), başka bir bazın satırı ya da başka bir çizim. İşareti kaldır yalnız bir
   şey işaretliyken açıktır; baz da sayılır.
6. **Hiçbir şey yazılmaz;** sözleşme değişmez. Çizgi `map.survey.query`'nin cevabından ve çizimin noktalarından
   çizilir. Python ve MCP bazların uçlarını ve uzunluklarını aynı cevaptan okur.
7. **Kapsam işareti:** ADR 2920'deki gibi panel **Harita**'nındır.

## Kanıt

- **Masaüstü** (`map_survey_network_rows_tests::a_baseline_s_row_draws_it_between_its_ends`; `map.survey.network`
  ortak durumunun çizimi, ağ C2'de kabul edilmiş):
  - kabul noktası olmayan uçlu baz çizilmez, ileti onu adlar;
  - iki ucu kabullü baz iki noktayı seçer;
  - çizginin uçları kabul noktalarının çizimdeki yerleridir, yazısı “bz-N · uzunluk m”dir;
  - İşareti kaldır açıktır ve çizgiyi kaldırır.
- **Resimler:** `.run/gui-evidence/baz-haritada-3320/` (açık geniş, koyu dar): N1 → N4 bazı.

  Arşiv paneli testleri ve çapraz denetimler geçer.

## Sonra

- Bilinen noktanın kendi yerinden (enlem, boylamdan izdüşürülmüş) bazın çizilmesi; kabul noktası olmayan uç da
  çizilsin.
- Bütün bazların bir arada çizilmesi (ağın görünüşü); dışlananın ayrı renkte.
- GVX oturum korelasyonları; yeni noktaların hızı (ADR 2520 Sonra).
- Web (TODOS §0.2).
