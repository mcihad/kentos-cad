# ADR 9348: Tablo yeni alanı hemen okur, değer listesi etiketiyle aranır, ifade oluşturucu kaynağın satırlarını okur, tablo katman süzgecini izler; Parsel alan denetimi

- **Durum:** kabul edildi (2026-11-27). Kaynak: CBS kullanım incelemesi, 27 Kasım 2026, kullanım turu 4
  (`gis-eksik-27-11-2026.md`, T11–T14 P2, T15 P3). Sahibin sözü: “yeni maddeler ekledim onları düzelt komple”.
- **İlgili:**
  - [ADR 9318](9318-data-layer-columns.md) (veri katmanının alanı tablonun sütunudur);
  - [ADR 9188](9188-null-empty-text-and-codes-told-apart.md) (kodun etiketi tabloda);
  - [ADR 9132](9132-table-conditions-by-layer.md) (tablonun katman başına koşulları);
  - [ADR 9109](9109-expression-to-the-source.md) (süzgeç kaynakta);
  - [ADR 9306](9306-parcel-area-table.md) (parsel alan çizelgesi).

## Bağlam

- **T11:** bağlı GeoPackage katmanının tablosu açıkken Alanlar'da yeni alan eklendi; tabloda sütun görünmedi. Katmana
  geri dönünce geldi, başlıkta takma ad yerine fiziksel ad yazdı.
- **T12:** `Y → Yeşil Alan` listeli alanda Ara “Yeşil Alan” ile sıfır, `Y` ile bir satır buldu.
- **T13:** Katman süzgeci › ε'de alanın Örnek değerler'i ve önizleme “nesne yok” dedi; satırlar kaynaktaydı.
- **T14:** `qa_turu = 'Y'` katman süzgeci “1 / 3 satır görünür” dedi; tablo üç satırı göstermeyi sürdürdü. “Göster Tümü”
  tablonun kaynağı mı süzülmüş katmanı mı gösterdiğini söylemiyordu.
- **T15:** Veri › Tablo › “Alan denetimi” öznitelik alanlarının denetimi sanıldı; pencere Parsel alan denetimiydi.
  Uygun parsel yokken ne gerektiğini söylemiyordu.

## Araştırma

- **QGIS, Query Builder:** sağlayıcı süzgeci başarılı koşulda katmanı eşleşen kayıtlarla sınırlar; öznitelik tablosu da
  süzülmüş katmanı gösterir. Oluşturucu alanın örnek ve benzersiz değerlerini kaynaktan getirir
  ([Query Builder](https://docs.qgis.org/latest/en/docs/user_manual/working_with_vector/vector_properties.html#query-builder)).
- **QGIS, Value Map:** alanın düzenleyicisi kodu kaydeder, açıklamasını gösterir; tablonun görünüşü açıklamadır
  ([Edit widgets › Value Map](https://docs.qgis.org/latest/en/docs/user_manual/working_with_vector/vector_properties.html#edit-widgets)).
- **OGC GeoPackage şema uzantısı:** değer listesi `gpkg_data_column_constraints`'te kod ve açıklama olarak durur
  ([Schema extension](https://www.geopackage.org/spec140/#extension_schema)).
- **KentOS kararı:**
  - Tablo katmanın alan tanımını izler; başlık takma addır.
  - Arama, gösterilen etiketi kaynağın tuttuğu koda çevirir.
  - Tablonun öntanımlı kapsamı süzülmüş katmandır; kaynağın tümü ayrı bir seçimdir.

## Karar

### 1. Yeni alan ve başlık (T11)

- **Yeniden sorma:** tablonun sorusu katmanın alan adlarını da tutar. Alan eklenince ya da çıkarılınca tablo katmandan
  ayrılmadan yeniden okunur; seçim anahtarla kalır.
- **Başlık:** sütunun başında alanın takma adı yazar (“Saha türü”). Sıralama, koşul ve yazma fiziksel adla sürer.
- **Kaynakta olmayan sütun:** katmanın kaynağında henüz olmayan alan aranmaz ve okunmaz (ADR 9318'in kuralı). Bu dilimde
  bulunan kusur düzeltildi: önceden Ara bu alanı da kaynağa soruyordu, kaynak bütün okumayı reddediyordu.

### 2. Etiketle arama (T12)

- **Kural (`layers_rows`, her kanal):** Ara'nın sözcükleri herhangi bir sütunda ya da listeli bir alanın etiketinde
  geçen satırları bulur.
  - Etiketler Türkçe katlamayla karşılaştırılır (“çınar”, “ÇINAR”).
  - Eşleşen etiketlerin kodları kaynağa `IN` koşulu olarak gider; aynı etiketi paylaşan kodların hepsi sayılır.
  - Sayı sütununda kod sayı olarak sorulur.
  - Sorgu kaynağın tamamındadır, yüklenen sayfada değil.
- **Kod:** kodla arama da sürer.

### 3. İfade oluşturucu kaynağın satırlarında (T13)

- **Satırlar:** veri katmanında oluşturucu, katmanın satırlarını kaynaktan bütün olarak okur (data_rows.rs, arayüzün
  iş parçacığı dışında). Satırlar çizimin bir kopyasına, şekilleri kendi deposuna konur; en çok 5 000.
  - Önizleme ve nesne adımlayıcı bu satırlardadır.
  - Alanların sayıları (“5 nesnede var”) satırlar gelince yeniden sayılır.
- **Değerler:** listeli alanın değeri “Çınar (M1)” diye görünür; çift tık ifadeye `'M1'` yazar.
- **Durumlar:** “Kaynağın satırları okunuyor…”, “Kaynak okunamadı: …”, “Kaynağın tablosunda satır yok.” ve “Bu alanın
  nesnelerde değeri yok.” ayrı sözlerdir.

### 4. Tablonun kapsamı (T14)

- **Öntanımlı:** veri katmanının Katman süzgeci varsa tablo süzülmüş katmanı gösterir. Tablonun kendi İfade süzgeci
  onunla birlikte uygulanır.
- **Kapsam seçici:** katman süzgeci olan katmanda tablonun çubuğunda “Kapsam: Süzülen katman | Kaynağın tümü” durur.
- **Alt satır:** “2 satır · süzgeç ve sıra kaynakta · katman süzgeci: mahalle = 'M1'”. Kaynağın tümünde: “… · kaynağın
  tümü (katman süzgeci uygulanmadı)”.
- **Süzgeci kaldır:** Katman süzgeci kalkınca tablo kaynağın tümünü gösterir (seçici kaybolur).

### 5. Parsel alan denetimi (T15)

- **Ad:** `parcels.check`'in kısa adı “Parsel alan denetimi”dir.
- **Yer:** CBS'de Veri › Tablo'dan çıktı. Veri'nin “Standart” paneli “Denetim” oldu: Standart profili, Katalog raporu,
  İmar planı denetimi, Parsel alan denetimi. Yeni panel 1440 piksellik pencerede iki paneli daha katladığı için ayrı
  panel açılmadı.
- **Gereken:** parsel bulunamayınca pencere ne gerektiğini söyler: “Denetim kapalı alanları okur: seçili alanları, seçim
  yoksa “parsel” katmanındakileri. Her alanda Ada, Parsel ve “Tapu alanı (m²)” değeri olmalı; İl, İlçe ve Mahalle
  kimliği tamamlar. Parselleri seçip denetimi yeniden açın.”

### 6. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Tablonun yeni alanı, başlıkta takma ad, kapsam seçici | yok: Mimari'de veri katmanı yoktur | var: CBS tabloları altlık olabilir | var: aynı gerekçe | var |
| Etiketle arama (`layers_rows`) | yok: aynı gerekçe | var | var | var |
| İfade oluşturucu kaynağın satırlarında | yok: aynı gerekçe | var | var | var |
| Parsel alan denetiminin adı ve sözü | yok: Mimari'de komut yoktur | var | yok: Yol'da komut yoktur | var (Veri › Denetim) |

### 7. Kanallar

| Kanal | Var mı | Gerekçe |
|---|---|---|
| Python | var (arama) | `cad.layers.rows`'un `search`'ü etiketi koda çevirir (§2); tablo ve oluşturucu arayüzün sunumudur. |
| MCP | var (arama) | Aynı komut. |
| Yapay zekâ ajanı | var (arama) | Aynı komut. |
| Eklenti | var (arama) | Satır okuma eklentinin okuma yetkisiyle aynı komuttan geçer. |

## Açık

- **Başlığın ipucu:** tablonun başlığı ipucu taşımıyor; fiziksel ad Alanlar'da ve Öznitelikler'de görünür.
- **Benzersiz değerlerin sayfalanması:** oluşturucu en çok 5 000 satırı okur; daha büyük tabloda değerler kaynakta
  `DISTINCT` ile sorulmalı.
- **Gerçek pencere:** fare ve klavyeyle kabul (`scripts/usage/real/`) bu dilimde yapılmadı.

## Doğrulama

27 Kasım 2026'da koşuldu:

- **Masaüstü** (`features::scope_tests`, 4 geçti):
  - `bakim`'e takma adlı `qa_turu` eklenince tablo yeniden soruyor, 6 satır okunuyor.
  - “çınar” M1'in 2 satırını, “M2” kodla 2 satırı buluyor.
  - `mahalle = 'M1'` katman süzgeciyle tablo 2 satır; Kaynağın tümü 6, geri 2.
  - Oluşturucu 6 kaynak satırında, önizleme değer veriyor; değerlerde “Çınar (M1)” → `'M1'`.
  - `parcels.check`'in kısa adı “Parsel alan denetimi”.
- **Geniş takım:** `-- features:: data_ expression layer_filter forms:: module_tests ribbon_tests history::text
  catalog settings` hepsi geçti; kırmızı tek sınama bilinen bağımlılık tabanıdır. `module_tests` CBS'nin Veri ›
  Denetim panelini sınıyor; `ribbon_tests` 1440 pikselde en çok bir panelin katlandığını.
- **Takımlar:** `cargo test -p kentos-desktop-cbs` 63, `-p kentos-desktop-map parcels_check` 4, `-p kentos-headless
  --test headless -- rows` 2, `-p kentos-cad` (izler) 37 geçti.
- **Kataloglar:** `python3 scripts/ui/ribbon.py --check` güncel; `python3 scripts/cad/manifest.py --check` 1099 satır.
- **Resim** (`.run/gui-evidence/kullanim-9348/`, açık ve koyu tema; büyütülerek bakıldı):
  - `01-tablo-kapsami`: “Kapsam: Süzülen katman”, başlıkta “Mahalle kodu”, iki satır, alt satırda katman süzgeci;
  - `02-ifade-degerleri`: “5 nesnede var”, Çınar (M1), İstasyon (M2), M9; önizleme “doğru”, 1 / 6.
