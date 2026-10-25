# ADR 0636: Parçanın bandı her yerde, anahtarlı bağda geometri, bağ ayarlarının geri alınması

- **Durum:** kabul edildi (2026-10-25). `plan/gis.md` CBS-18'in kalanı: PostGIS parçasında bant, başka ölçekli açıklama
  kopyasında ve başsız kitap PDF'inde bant, bandın zamanla ve süzgeçle ilişkisi, büyük dosyada bandın süresi; ADR 0533'ün
  ve 0578'in sonraya bıraktıkları (bağ ayarlarının geri alınması, anahtar alanının ve Denetle'nin pafta ölçeğinin
  arayüzü, anahtarı eksik ya da yinelenen satırın iğnesi, uzun kaydırmada alan ve eğri adları). Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0533 (iğneler ve bağlı katman), 0578 (parçanın bandı, park, anahtarsız eşleme), 0510 (parça),
  0582 (PostGIS'te alan), 0589 (topolojinin bandı), 0597 (dosyanın SHA-256'sının tutulması), 0323 (ölçekli açıklamalar),
  0534 (atlas), 0210 (zaman), 0211 (katman süzgeci).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **`.kcad`:** şema değişmez.
- **Sonraya kalan:** çok parçalı alanın parça başına önceki yerleşimi ölçülmedi; bandın temaya göre süzülmesi (bölünmüş
  görünümün kendi teması) ana görünümün katman görünürlüğüyle sınırlı.

## Karar

### 1. PostGIS parçasının bandı

`sources::band_job` artık dosyada da veritabanında da parçası olan her bağın işini verir. Veritabanındaki tablo
`load_database` ile, bandın alanıyla (tablonun sisteminde), tablonun şimdiki hâliyle okunur (dosyanın sürüm denetimi
yoktur; ADR 0589'un topoloji bandı gibi). Masaüstünün bant izleyicisi, başsız `cad.labels.report` ve kitap PDF'i aynı
işi kullanır.

### 2. Ölçekli kopyada ve başsız kitapta bant

- Paftanın bir haritası başka ölçekte kendi geometri deposunu kurduğunda (`SheetMaps::scaled`, ADR 0323 §2), ana
  deponun bant satırları o depoya da verilir (`Spatial::ghosts_of`). Kopyanın önbelleği bantların sayısını da anar.
- Başsız kitap PDF'i ve atlas (`kentos_plot::book::with_bands`) her parçanın bandını okur, deposuna ve ölçekli
  kopyalara verir. Okunamayan bant kitabın notlarında söylenir: “…: parçanın çevresindeki komşular okunamadı (…);
  etiketler yalnız parçanın nesnelerine göre yerleşti.”

### 3. Bant, katmanın süzgecine ve zamanına uyar

ADR 0578 bant satırlarını her zaman engel saydı. Artık:

- **Süzgeç:** katmanın süzgeci (ADR 0211) bant okunurken satırlara uygulanır (`layer_filter::compile_filter`, aynı
  sütun motoru); katmanın göstermeyeceği satır engel değildir. Derlenmeyen süzgeç katmanın hiçbir şeyini, bandını da
  göstermez.
- **Zaman:** zaman ayarı olan katmanda her satır zamanını alır (`time::layer_times`); depo onu yanında tutar
  (`Store::set_ghosts_timed`, `ghost_times`) ve zaman sürgüsünün penceresi satırı katmanın kendi nesnesi gibi süzer.
- **Tema:** katmanın görünürlüğü zaten uygulanıyordu (`visible`).

Masaüstü bandı katmanın süzgeci ya da zaman ayarı değişince yeniden okur (`band_shown_key`). Bu anahtar her
değişiklikten sonra sorulduğu için ucuzdur: bant işi yalnız anahtar değişince kurulur.

### 4. Anahtarlı bağda anahtarı eksik ya da yinelenen satır

Anahtarlı bağda iğnenin nesnesinin satırı yoksa ve satır anahtarla bulunamıyorsa, iğne artık kaybolmaz; ADR 0578
§2'nin geometri eşlemesine düşer. Bu iki durumda olur:

- nesnenin anahtar değeri boşsa: katmanın nesnesi olmayan bütün satırlar arasında aranır;
- değer tabloda birden çok satırdaysa: yalnız o değeri taşıyan satırlar arasında aranır.

Değer tabloda hiç yoksa iğne `deleted` olarak kalır. `key_not_unique` uyarısı artık iğnelerin o değerlerde
geometrileriyle taşındığını söyler.

### 5. Bağ ayarları geri alınır

Bağın anahtarı (`cad.sources.edit` `key`) ve alanı (`extent`) belgenin geri alma geçmişine girer. Belgenin yeni işlemi
`Op::Source`, bağı ve uzlaştırma tabanını önce ve sonra hâliyle tutar (`Document::source_state`,
`Document::record_source`):

- anahtar kendi adımıdır: “Bağın anahtarı: tablo”;
- alan, satırları okuyan “Alan sınırı” adımına yazılır.

Geri alma bağı nesnelerle birlikte geri getirir: parça, katmanın özeti ve park edilmiş iğneler. Bağlama, bağı çözme,
yeniden bağlama ve eşitlemenin sürüm alanları eskisi gibi ayardır (ADR 0510); yazılmış adımın geri alınması ADR
0598'in kuralıyla denetlidir.

### 6. Büyük dosyada bant

Bandın sürüm denetimi, dosyanın SHA-256'sını ADR 0597'nin tuttuğu özetten alır. Aynı süreçte ikinci bant okuması
(başsız raporun yinelenmesi, kitabın sayfaları) dosyayı baştan okumaz.

### 7. Arayüz

- **Kaynağa bağla penceresi:** Anahtar alanı (boş: yok; boşluklar atılır), seçilen her tabloya aynı anahtar.
- **Katmanlar › Kaynak ▸ Anahtar alanı ▸:** Yok ve katmanın alanları, şimdiki işaretli. Seçim geri alınabilir; reddi
  (boş ya da yinelenen değer) söylenir. İkon `sourceKey`.
- **Etiketler penceresinde Denetle ölçeği:**
  - görünüm, ekrandaki ölçeğiyle;
  - görünüm, çizim ölçeğinde (1:N, kâğıt);
  - öndeki paftanın her haritası, kendi ölçeği ve alanıyla.

  Seçince sayılar yeniden çıkar (`App::report_scopes`).
- Etiketler'in sözleri “öbür etiketler” yerine “diğer etiketler” (arayüzün sözcük kuralı).

### 8. Uzun kaydırmada alan ve eğri adları

ADR 0533 önceki yerleşimin kısa kaydırmada sıçramayı giderdiğini ölçmüştü. Bu kez uzun bir kaydırma ölçüldü: 30 adım,
her biri bir öncekinden. İki görünümün de içinde kalan alan adı (mahallenin kutbu) ve eğri boyunca ad (dere):

- önceki yerleşim korununca hiç sıçramaz;
- yeni yerleşimde 35 kez sıçrar.

Ölçüm `label_stability.rs`'in `a_long_pan_moves_area_and_curved_names_only_when_they_must` testinde.

## Doğrulama

- **Ortak durumlar:**
  - `cad.sources.sync` 51 durum; yeni üçü anahtarlı geri dönüş: değeri boş nesne, yinelenen anahtar geometri ve
    nearest. Yinelenen anahtar için tablo `cift.gpkg`; GDAL'ın OGR'ı yazar, durumda `komsu.gpkg`'nin yerine konur.
  - `cad.sources.edit` 61 durum; yenileri anahtarın geri alınıp yinelenmesi ve aynı anahtarın adım yazmaması.
    “Alan sınırı” geri alınınca bağ bütün tabloya döner.
  - Hepsi KentOS kodu olmadan `bound_sources_cases.py`'den; Rust ve Python'da oynatılır.
- **PostGIS:** `a_postgis_part_has_its_band`, geçici PostGIS'te: dokuz parselin ortadakini tutan parça, sekiz komşu
  bant. Süzgeç birini dışarıda bırakınca yedi.
- **Çekirdek:** `ghosts.rs` zamanlı bant satırının pencerede ve pencere dışında yer tutması; `label_stability.rs`
  uzun kaydırma.
- **Başsız:** `kentos-plot`'un `bands.rs`'i: kitabın deposu iki komşuyu tutar, dosya değişince söylenir.
- **Uygulama:** `file_digest`'in `a_band_read_twice_reads_the_file_whole_once` testi.
- **Masaüstü:**
  - testler: `a_copy_at_another_scale_takes_the_band`, `the_bind_window_takes_a_key_and_kaynak_changes_it_as_an_undo_step`;
  - resimler: `gpkg-*-14-anahtar`, `gpkg-*-15-anahtar-kalkti`, `arac-etiket-denetle-olcek` (1440×900 ve 1100×650,
    iki tema).
- **Süreler:** bant işinin anahtarı her mesajda bir JSON yazımıdır. Bant okuması ve süzgeci eskisi gibi kendi iş
  parçacığında. 30 adımlık uzun kaydırma ölçümü bu makinede bütün yerleşimle 5,5 s'lik test koşusunun içinde.
