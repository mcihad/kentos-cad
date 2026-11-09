# 3720 — GNSS ağının haritada bir arada görünüşü

Tarih: 2026-11-09. plan/map-cad.md MAP-03, yedinci tur. Durum: taslak. ADR 3320'nin “Sonra”sındaki “bütün bazların bir
arada çizilmesi (ağın görünüşü); dışlananın ayrı renkte”.

İlgili:
- [ADR 2920](2920-archive-panel-baselines-and-adjustments.md): arşiv panelinde bazlar, bilinen noktalar ve
  dengelemeler;
- [ADR 3320](3320-gnss-baseline-drawn-on-the-map.md): bazın satırından haritada bazın çizilmesi.

## Dayanak

- **Trimble Business Center**, [Enable and Disable Vectors](https://help.fieldsystems.trimble.com/tbc/2031.htm)
  (9 Kasım 2026'da okundu):
  - vektörler Project Explorer'da ya da bir grafik görünümde seçilir, oradan devre dışı bırakılır;
  - devre dışı vektörler ağ dengelemesinde kullanılmaz.

  Ağın vektörleri grafik görünümde bir aradadır. Hangisinin dengelemeye girmediği görülmeden ağ okunmaz. Belge devre
  dışı vektörün nasıl göründüğünü söylemez.
- **KentOS bugün** (ADR 3320): bazın satırı yalnız o bazı çiziyordu. Ağın bütününü görmek için bazlara tek tek basmak
  gerekiyordu, dışlanan baz ayırt edilmiyordu.

## Karar

1. **Ağı göster** (GNSS arşivi › Bazlar grubunun başlığındaki düğme): iki ucunun da kabul noktası olan her baz
   haritada birlikte çizilir:
   - çizgi, ok, uçlarda halka ve “bz-5 · 3200.424 m” yazısı ADR 3320'deki gibidir;
   - dışlanan baz (`excluded`) tehlike renginde, noktalı çizilir. Dengelemeye girmediği bir bakışta okunur.
2. **Uçlar** ADR 3320'deki gibi kabul noktalarının nesneleridir, çizimin bugünkü hâliyle: nokta taşınırsa çizgi
   onunla gelir; bir ucu silinen çizgi çizilmez, öbürleri kalır.
3. **Seçim ve görünüm:** çizilen bazların uçları seçilir, görünüm onlara yakınlaşır.
4. **Durum çubuğu:** “Ağ: 3 baz çizildi; 1'i dışlandı, kırmızı noktalı. 5 baz çizilemedi: uçlarından birinin kabul
   noktası yok.” Hiçbiri çizilemezse bu söylenir.
5. **Kaldırma:** İşareti kaldır (`data.unmark`), bir bazın satırı (yalnız o bazı çizer) ya da başka bir çizim.
6. **Hiçbir şey yazılmaz;** sözleşme değişmez. Çizgiler `map.survey.query`'nin cevabından ve çizimin noktalarından
   çizilir. Python ve MCP bazların uçlarını, uzunluklarını ve dışlanmalarını aynı cevaptan okur.
7. **Kapsam işareti:** ADR 2920 ve 3320'deki gibi panel **Harita**'nındır.

## Kanıt

- **Masaüstü** (`map_survey_network_rows_tests::ağı_göster_draws_every_baseline_at_once`, `map.survey.network`'ün
  ortak durumlarının çizimi):
  - bir baz nedenle dışlandıktan sonra Ağı göster, iki ucu kabul noktası olan her bazı çizer, yalnız dışlananı
    dışlanmış diye işaretler;
  - uçları seçilir; çizilemeyenler sayılır, durum çubuğunun cümlesi beklenendir;
  - İşareti kaldır hepsini kaldırır.

  ADR 3320'nin testi tek bazın tek çizgi olduğunu denetler. Arşiv paneli, Ağ dengelemesi ve çapraz denetimler geçer.
  Resimler `.run/gui-evidence/ag-haritada-3720/`.
- **MCP ve Python:** cevap değişmedi (yalnız masaüstü kodu değişti).

## Sonra

- Bilinen noktanın kendi yerinden (enlem, boylamdan izdüşürülmüş) bazın çizilmesi. Çerçeve ve epok farkıyla ne kadar
  kaydığı söylenmeli; ADR 3320 Sonra.
- GVX oturum korelasyonları; yeni noktaların hızı (ADR 2520 Sonra).
- Web (TODOS §0.2).
