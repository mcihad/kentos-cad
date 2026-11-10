# 4120 — Bilinen noktanın projedeki kendi yeri; bazın oradan çizilmesi

Tarih: 2026-11-09. plan/map-cad.md MAP-03, sekizinci tur. Durum: taslak. ADR 3320'nin ve 3720'nin “Sonra”sındaki
“Bilinen noktanın kendi yerinden (enlem, boylamdan izdüşürülmüş) bazın çizilmesi; kabul noktası olmayan uç da
çizilsin. Çerçeve ve epok farkıyla ne kadar kaydığı söylenmeli”.

İlgili:
- [ADR 1340](1340-gnss-campaign-archive.md) §3: kabul noktasının koordinat zinciriyle projeye taşınması;
- [ADR 1740](1740-gnss-network-from-archive.md): GNSS ağ dengelemesi, bilinen noktalar;
- [ADR 2520](2520-known-point-own-velocity.md): bilinen noktanın kendi hızı;
- [ADR 3320](3320-gnss-baseline-drawn-on-the-map.md) ve [ADR 3720](3720-gnss-network-drawn-on-the-map.md): bazın ve
  ağın haritada çizilmesi.

## Dayanak

- **Trimble Business Center**, [Understanding Points and Coordinates](https://help.fieldsystems.trimble.com/tbc/5926.htm)
  (9 Kasım 2026'da okundu): bir noktanın, her biri ayrı yer ve kalitede birden çok koordinatı olabilir. Yani bilinen
  noktanın yayımlanmış koordinatı ile ölçülüp kabul edilen yeri ayrı yerlerdir; ikisi de gösterilebilir.
- **Trimble Business Center**, [Run a Point Comparison Report](https://help.fieldsystems.trimble.com/tbc/21022.htm)
  (9 Kasım 2026'da okundu): rapor yakın nokta çiftlerini, aralarındaki farkları ve fark uzaklığını verir; kullanıcının
  toleransını aşan çift kırmızıyla işaretlenir. Toleransı kullanıcı verir.
- **Trimble Access 2020.20 sürüm notları**,
  [2020.20](https://help.fieldsystems.trimble.com/trimble-access-release-notes/en/2020.20.htm) (9 Kasım 2026'da okundu):
  ITRF 2014 koordinatı ölçme epokundan işin küresel başvuru epokuna taşınır; yerel deformasyon modeli varsa onunla,
  yoksa küresel levha modelinden seçilen levhayla.
- **KentOS bugün:**
  - kabul noktası koordinat zinciriyle projeye taşınır (ADR 1340 §3);
  - ağ dengelemesi bilinen noktaları ölçme epokuna kendi hızıyla ya da hız ızgarasıyla taşır (ADR 2120, 2520) ama
    projedeki yerlerini söylemez; projeye yalnız yeni işaretleri koyar;
  - bazın haritadaki çizgisi iki ucun kabul noktasıyla çizilir; bilinen noktanın kabul noktası çoğu zaman yoktur, o
    bazlar çizilmez (ADR 3320, 3720).

## Karar

1. **`map.survey.query`'nin `places`'i** (sürüm aynı, alan eklendi): cevaba `knownPlaces`: arşivin sırasıyla her
   bilinen noktanın projedeki kendi yeri.
2. **Yer, kabul noktasının yolu:**
   - epok kararı, işaretine dokunan ilk bazın (arşivin sırasıyla) arazi ölçümünündür; yoksa projeninki;
   - nokta başka bir epoktaysa önce kendi hızıyla (yer merkezli X, Y, Z'de doğrusal, IERS Conventions 2010 §4.1), hızı
     yoksa o arazi ölçümünün hız ızgarasıyla zincirin kendisi taşır (ADR 2520'nin sırası). `moved` neyle taşındığını,
     epokunu, yılları ve verilen yerinde doğu, kuzey, yukarı kaymayı söyler;
   - sonra koordinat zinciri projeye taşır: x, y (projede kot varsa z), zincirin durumu (`complete`, `limited`) ve
     `epoch`.
3. **Kabul noktası:** işaretin kabul noktası varsa (`point`) `dx`, `dy`, `dz` = yer − kabul noktası (proje birimi).
4. **Yeri olmayan nokta** `why` ile söylenir, ret değildir:
   - epoku farklı, ne kendi hızı ne hız ızgarası olan nokta: “… 2026,84 epokunda, projenin epok kararı 2005,00:
     projeye taşımak için noktanın kendi hızı … ya da arazi ölçümünün hız ızgarası … gerekir.”;
   - zincirin kendi reddi (ızgara bu cihazda yok …) ve coğrafi olmayan çerçeve, kendi sözleriyle.
5. **Masaüstü** (GNSS arşivi):
   - bazın satırı ve Ağı göster, kabul noktası olmayan ucu, işaretindeki dışlanmamış bilinen noktanın kendi yerinden
     çizer. O uçta halka yerine üçgen ve işaretin adı durur; görünüm bütün uçlara sığar;
   - durum çubuğu söyler: “bz-1: BN1 ucu kabul noktası olmadığından bilinen noktanın projedeki kendi yerinden çizildi
     (üçgen).”, “Ağ: 8 baz çizildi. BN1, BN2 bilinen noktanın projedeki kendi yerinde (üçgen).”;
   - ne kabul noktası ne yeri olan bilinen noktası olan uç çizilmez, nedeni söylenir;
   - Bilinen noktalar satırının ipucu yeri, epok kaymasını ve kabul noktasına uzaklığı ya da yerinin olmama nedenini
     söyler.
6. **Hiçbir şey yazılmaz.** Bilinen noktanın yeri çizimde nesne değildir.
7. **Kapsam işareti:** **Harita** (arşiv ve paneli harita projesinindir).

## Sınırlar

- Çerçeve farkının büyüklüğü ayrıca söylenmez: zincirin kendi dönüşümüdür, durumu (tam ya da sınırlı) söylenir. Epok
  farkının kaydırdığı ayrıca söylenir.
- Kabul noktasına uzaklık için tolerans yok. TBC'de olduğu gibi toleransı kullanıcı verir; KentOS değer uydurmaz
  (kullanıcının sınırı: [ADR 4520](4520-known-place-tolerance.md)).
- Dışlanan bilinen nokta bazın ucu olmaz.

## Kanıt

- **Bağımsız başvuru** (KentOS kodu yok):
  - `scripts/fixtures/map_survey_cases.py`'nin `known_place`'i. Kendi hızı PROJ'la (TUREF 3B → yer merkezli ve geri),
    hız ızgarası PROJ'un deformasyon adımıyla, zincir crs_vertical_cases.py'nin yolu;
  - `scripts/fixtures/map_survey_network_cases.py`'nin `query_cases`'i, ağın arşivinde altı yeni `map.survey.query`
    durumu (28 durum):
    - 2005,0'da iki bilinen nokta;
    - 2026,84'te kendi hızıyla (yer 2005,0'dakine 0,1 mm yakın);
    - aynı nokta hız ızgarasıyla;
    - ne hızı ne ızgarası olan nokta: neden;
    - N1'de hem bilinen hem kabul noktası: 6 mm doğu, 4 mm güney, 12 mm yukarı;
    - plan.
  - Rust ve Python geçer; eski durumlar aynı.
- **Python** (`python/tests/test_known_places.py`), testin kendi hesabıyla:
  - kayma R(φ, λ)·V·Δt;
  - geri taşınan yer 2005,0'dakinden 1 mm içinde;
  - N1'de uzaklık √(6² + 4²) mm ve kot farkı 12 mm;
  - neden; `places` olmadan hiçbiri.
- **MCP** (`crates/native/mcp/tests/known_places.rs`), şema 207 örneği:
  - ANKR'nin yeri `cad.crs.transform`'un aynı koordinat için verdiği;
  - N1'in eski C3 koordinatının yeri yok, nedeni söylenir;
  - N3'e 2026,84'te hızıyla eklenen nokta V·Δt kadar kayar ve kabul noktasıyla karşılaştırılır.
- **Masaüstü** (`map_survey_network_rows_tests`):
  - `a_baseline_s_row_draws_it_between_its_ends`: BN1'den gelen baz BN1'in yerinden, adıyla; BN1 dışlanınca çizilmez;
  - `ağı_göster_draws_every_baseline_at_once`: BN2 dışlanınca onun bazları sayılır, BN1 adlanır;
  - `a_known_point_s_row_says_its_own_place`: satırın sözleri.

  Çapraz denetimler geçer. Resimler `.run/gui-evidence/bilinen-yeri-4120/`: sekiz baz birden, BN1 ve BN2 üçgenle.

## Sonra

- Kabul noktası ile bilinen noktanın yeri arasındaki farkın kullanıcı toleransıyla karşılaştırılması (TBC'nin nokta
  karşılaştırma raporu gibi).
- GVX oturum korelasyonları; yeni noktaların hızı (ADR 2520 Sonra).
- Web (TODOS §0.2).
