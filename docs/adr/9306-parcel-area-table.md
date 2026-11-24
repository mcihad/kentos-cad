# ADR 9306: Parsel alan çizelgesi (`map.parcelReport`; `cad.parcels.report`, `cad.parcels.table`)

- **Durum:** kabul edildi (2026-11-24). `todos-new.md` C1 (bekleyen Harita komutu), C2 ve F6. Sahibin kararı (F6, 24 Kasım
  2026): “ayrılsın, şimdi yapılsın”: kadastronun teknik, hukuki olmayan çıktıları şimdi yapılır; ifraz, tevhit, hisse,
  tescile esas alanın yuvarlanması ve kurum onayı (GIS-06/07) sona kalır.
- **İlgili:** [ADR 1400](1400-parcel-area-check-and-identity.md) (Parsel alan denetimi, yanılma sınırı, parsel kimliği),
  [ADR 0184](0184-tables.md) (tablo), [ADR 1030](1030-data-extraction-recipes-and-table-parts.md) (tabloyu imleçten
  yerleştirme), [ADR 0172](0172-vertex-table.md) (halkaların adları), [ADR 0149](0149-measurement-accuracy-and-display-rounding.md) (gösterim
  kuralı), [ADR 9303](9303-generator-environment.md) (üreteç ortamı).
- **Kural:** CLAUDE.md “Modül ve otomasyon denetimi” (24 Kasım 2026), “Araç = komut”, §7, §23, §25.
- **Web:** iptal edildi. **`.kcad`:** değişmedi (tablo kaynağını saklamaz, §6). **Bağımlılık:** yeni kitaplık yok.

## Bağlam

**Bugün olanlar.** `map.parcelReport` (“Parsel alan çizelgesi…”) katalogda bekliyordu: şeritte yeri yoktu,
`scripts/ui/ribbon.py`'nin `MODULES.only` tablosu onu yalnız Harita'ya veriyordu, ürün komutu yoktu, manifestte düzeyi
`none` idi. KentOS'ta yakın işler vardı ama hiçbiri çizelgenin kendisi değildi:

- `cad.parcels.check` (ADR 1400): hesap alanı, tapu alanı, fark ve yanılma sınırı; köşe yok.
- Tablo ekle'nin Koordinat çizelgesi ve Alan çizelgesi (ADR 0184 §3): noktalar ya da alanlar ayrı ayrı; parsel parsel
  köşe, kenar ve alan yok.
- Köşelere nokta ve `builtin.parcelSheet` modeli: köşelere numara, kenarlara yazı; tablo değil.

### Araştırma (24 Kasım 2026'da okundu)

**Mevzuat.** Yerel kopyalar `.run/research/map-cad/mevzuat/pdf/` altındadır (Git'e eklenmez).

1. **Tapu Planları Tüzüğü** (BKK 28.07.2008/2008-14001, RG 27.08.2008/26980), m. 8(2): “Taşınmazların yüzölçümü, köşe
   noktalarının koordinatları ile hesaplanır.”
2. **Tapu Planlarında Yanılma Sınırının Belirlenmesi Hakkında Yönetmelik** (RG 27.09.2022/31966):
   - m. 4(1)(i): hesap yüzölçümü, çeşitli alan hesabı yöntemleriyle hesaplanan yüzölçümüdür.
   - m. 8(2): tablonun “Alan” değişkeni, “arşiv onaylı koordinatların desimetrekare (virgülden sonra iki basamak)
     mertebesinde kullanılarak bulunan yüzölçümü”dür.
   - m. 8(4): “tescile esas yüzölçümü hesaplarında, metrekare veya desimetrekare mertebesine yuvarlamadan kaynaklı
     farklar” kadastro teknik düzenlemelerine göredir.
   - m. 8(5): F, “yatay koordinatlar kullanılarak yapılan Gauss alan hesabı formülü”dür. Formül gazetede resimdir:
     Σ (yₙ₊₁ − yₙ)(xₙ₊₁ + xₙ) ya da Σ (xₙ − xₙ₊₁)(yₙ + yₙ₊₁).
3. **TKGM 2025/4 Genelgesi** (Tescile Konu Harita ve Planların Yapımı ve Kontrolü, 12.05.2025,
   E-32381507-170.03.02-16435018; 20.10.2025 değişikliğiyle):
   - m. 12(11): kontrolde “yüzölçümü hesapları, yeni oluşan parsellerin yüzölçümlerinin kök kadastro parselleri
     yüzölçümüne uygunluğu” bakılır.
   - m. 12(13): ölçü krokisinde, tescil bildiriminin kroki sayfasında ya da değişiklik tasarımında “yüzölçümü yanılma
     sınırları … Yönetmelik Hükümlerince gösterilir”.
   - m. 21(4): sınırlandırmada “parsel detay noktalarının ölçü, koordinat hesabı, yüzölçümü hesabı, ölçü krokisi
     (Ek-18) … koordinat özet çizelgesi düzenlenir”.
   - m. 22(5): kamulaştırmada tescil bildiriminin kroki yerinde “nokta numaraları ve koordinatları ile yüzölçümleri
     gösterilir”.
   - m. 25 b(1): “Köşe nokta sayısının fazla olması durumunda, koordinatlar, yatay doğrultu ve yatay uzunluk bilgileri
     ölçü krokilerinden bağımsız olarak ayrı bir sayfa şeklinde de düzenlenebilir.”
   - m. 25 b(4), 26(3), 27(3): “ölçü krokisindeki detay numarası, koordinatları ve yüzölçüm hesabının doğruluğu kontrol
     edilir”.
4. **BÖHHBÜY** (RG 26.06.2018/30460 mük., 4.10.2025 değişikliğiyle):
   - m. 9: ağ noktalarının numaraları “kuzeyden başlayarak saat yönünde” verilir;
   - m. 45(1)(b): detay noktalarının numaraları Ek-1'in kataloğuna göredir;
   - m. 98(1)(f): “Koordinat özet çizelge ve çıktıları” arşivlenir;
   - Ek-1'de TAPU_YUZOLCUMU (“detayın tapu yüzölçümü”) ve TECVIZ_DURUMU (“ölçü ve hesapla bulunan değerin tecviz
     içinde olup olmadığı”) öznitelikleri vardır.
   - Yönetmelik bir alan hesap çizelgesinin sütunlarını tanımlamaz.
5. **TKGM'nin arşiv listesi** (2025-01 tarihli Ek 2-4) ve **Kadastro Müdürlüğü ve LİHKAB Hizmet Standartları**
   (19.04.2021): “Yeni İmar Adaları Alan Hesap Cetveli”, “Yüzölçümü Hesapları” ve “Parsel Köşe Noktası Koordinat Özet
   Çizelgeleri” ayrı belgelerdir. Bu satırlar arama dökümünde görüldü; tkgm.gov.tr bu makineye PDF yerine kendi giriş
   sayfasını döndürdü, belgenin kendisi okunamadı.

**Bilimsel örnek.** R. G. Hoşbaş, A. Pırtı (2020), “Parsellerin bölünmesi (ifrazında) ve parsel kırık sınırlarının
düzeltilmesinde dik koordinatlara dayalı kesin yöntem”, NÖHÜ Müh. Bilim. Derg. 9(2): 913–930
(dergipark.org.tr/en/download/article-file/958047):

- Tablo 1'in sütunları: Nokta No, Yᵢ (m), Xᵢ (m), Yᵢ₊₁ − Yᵢ₋₁, Xᵢ(Yᵢ₊₁ − Yᵢ₋₁); altında Σ(Yᵢ₊₁ − Yᵢ₋₁) = 0,00
  denetimi, 2F = 3230,6997 m² ve F = 1615,35 m².
- Köşeler “saat ibresi yönünde” numaralıdır. Koordinatlar santimetre, alan desimetrekare yazılmıştır.
- Tablonun değerleriyle 2F elle yeniden hesaplandı: 3230,6997. Y doğu, X kuzey düzeninde saat yönündeki sıra artı
  alan verir.

**Sonuç:** kurumun alan hesap cetvelinin resmî bir sütun şablonu bulunamadı. KentOS'un sütunları kaynakların ortak
öğeleridir: nokta numarası, Y, X, yatay uzunluk ve yüzölçümü. Yanılma sınırı TKGM 2025/4 m. 12(13)'ten gelir.

**Yazılımlar** (karşılaştırma yalnız bu kayıttadır; ürünün metinlerinde başka uygulama anılmaz):

- **Autodesk Civil 3D:** Annotate › Labels & Tables › Add Tables › Parcel › Add Area. Line, Curve ve Segment tabloları
  da vardır. Tablonun içeriğini (parsel adı, alan, çevre, segment yönleri…) tablo stili belirler. Tablo parsel
  etiketlerine bağlıdır, çizimle güncellenir; dış başvurudan tablo yapılmaz. Kaynaklar Autodesk yardımının “About
  Adding Parcel Tables” (2023), “To Add Tables of Parcels” (2015) ve “Exercise 1: Creating a Parcel Area Table”
  (2024–2025) sayfalarıdır. Bu bilgi arama dökümünden alındı; sayfalar bu makinede sertifika hatası verdi.
- **Netcad:** “Alan Çıktıları” ve “Koordine Özet” sayfaları (wiki.netcad.com.tr pageId 217389310 ve 217394043;
  [araştırma kaydı](../research/2026-10-01-netcad-arcgis-qgis.md)) bu makinede sertifika hatası verdi, okunamadı.
  ADR 1400'ün ikincil notu: Netmap'te tapu alanı “baz alan” seçilip alan özeti alınır.

**KentOS'un çıkardığı:** çizelge parsel parsel köşe numarası, Y, X, kenar ve alandır; tapu alanı ayrı durur. Civil 3D'nin
canlı tablosunun karşılığı bu dilimde yoktur: tablo bir kez yazılır (§6).

## Karar

### 1. Kapsam

- **Var:**
  - parselin halkaları ve köşeleri sırasıyla, köşenin numarası ve Y, X;
  - kenar (kiriş); yaylı kenarda yarıçap ve yay boyu;
  - köşe koordinatlarından alan;
  - kayıtlı alan yazıldığı gibi; okunursa fark ve yanılma sınırı (bilgi);
  - çizime yazılan tablo ve CSV.
- **Yok, sona kalır** (GIS-06/07, sahibin 7 Ekim ve 24 Kasım kararları):
  - ifraz, tevhit, hisse;
  - tescile esas yüzölçümünün yuvarlanması (m. 8(4));
  - kurum onayı; tescil bildirimi, değişiklik beyannamesi ve fen klasörünün biçimi;
  - geçici koordinatlı parsellerin hata çemberi yöntemi (m. 8(5)).
- **Bu dilimde yok:**
  - Gauss ara sütunları (Yᵢ₊₁ − Yᵢ₋₁, Xᵢ(Yᵢ₊₁ − Yᵢ₋₁)); halkanın alanı cevaptadır;
  - elipsoit ya da zemin alanı;
  - paftanın kendi tablo öğesi: çizimdeki tablo paftaya harita penceresinden girer.
- Sonuç resmî uygunluk iddiası değildir; hesap alanı kayıtlı alanın üstüne hiçbir zaman yazılmaz (CLAUDE.md §7, §23.1).

### 2. İki ürün komutu (Araç = komut)

- **`cad.parcels.report` v1** (sorgu, hiçbir şey yazmaz). Girdi:
  - `uids`: alanlar; yoksa parsel katmanının (`parsel`) bütün alanları;
  - `names`: `points` (öntanımlı) ya da `numbers`;
  - `prefix`: yeni numaranın öneki; en çok 12 harf, denetim karakteri yok;
  - `start`: ilk numara (öntanımlı 1);
  - `lengths`: kenarlar (öntanımlı evet);
  - `field`: kayıtlı alanın özniteliği (boş ya da yoksa `Tapu alanı (m²)`; en çok 120 harf).

  Cevap:
  - `srid` (sistemsiz projede yok), `rule` (`tkgm-2022-m8-arsiv`), `field`;
  - `parcels`: kimlik parçaları, başlık, halkalar (parça, delik, köşeler, alan), alan, kayıt, fark, sınır, durum;
  - toplam `area`; `places`, `named`, `numbered` (köşe yeri, noktadan ad alan, yeni numara alan);
  - `cells`: tablonun sözleri.
- **`cad.parcels.table` v1** (belge, tek geri alma adımı “Parsel alan çizelgesi tablosu”). Girdi: raporun girdisi ile
  `at` (sol üst köşe), `layerId` (yoksa etkin katman), `height` (yoksa projenin tablo yüksekliği çizim ölçeğinde),
  `expectedRevision`.
  - Plan: `table` (yazılacak nesne; yerleştirme önizlemesi bundan), `parcels`, `rows`, `numbered`, `revision`.
  - Cevap: tablonun kalıcı kimliği, `parcels`, `rows`, `numbered`, `revision`.
- **Yer:** sözleşme `crates/shared/commands/src/map/cad_parcels_report.rs`, işleyici
  `crates/native/application/src/map/parcels_report.rs` (`report`, `table`). Kayıt ve yanılma sınırı
  `parcels_check`'in kodudur (`recorded_value`, `tolerance`); ikinci bir kural yazılmadı.

### 3. Kurallar

Bağımsız başvuru bunlardan yazıldı (`scripts/fixtures/parcels_report_cases.py`).

1. **Kapsam:** `uids`'in alanları verildiği sırayla; yoksa parsel katmanının alanları çizimin sırasıyla.
2. **Halkalar:** her parçanın dış halkası, sonra delikleri (ADR 0143'ün sırası).
   - Her halka haritada görüldüğü gibi **saat yönünde** listelenir. İşaretli alanı (shoelace, saat yönünün tersi artı;
     yayların parçası bükümün işaretiyle) artı olan halka tersten yürünür, yayların bükümü ters döner.
   - Gerekçe: Gauss formülü (m. 8(5)) y doğu, x kuzey düzeninde saat yönünde artı alan verir; bilimsel örnek de
     saat yönündedir.
   - **İlk köşe en kuzeydekidir;** aynı kuzeydeki iki köşeden batıdaki, aynı yerdeki iki köşeden önce yürünen.
     BÖHHBÜY m. 9'un “kuzeyden başlayarak saat yönünde” kuralı ağ noktaları içindir; çizelge için benzetmedir.
3. **Köşe adları:**
   - `names: points`: köşe, çizimin sırasıyla 1 µm içinde duran ilk adlı nokta nesnesinin adını alır (etiketi
     kırpılmış; çok noktalı nesnenin k'inci noktası “ad (k)”). 1 µm, kot kurallarının “aynı yer”idir (ADR 0142). Adı
     olmayan nokta sayılmaz.
   - Noktası olmayan köşe, `names: numbers`'ta her köşe **yeni numara** alır: `prefix` ve `start`'tan birer artan tam
     sayı. Çizimdeki bir noktanın adı ya da çizelgenin daha önce verdiği bir adla çakışan numara atlanır: bir yerde iki
     ayrı köşe aynı numarayı taşımaz.
   - Çizelgede daha önce adlanmış bir yer (1 µm, ilki) adını korur: iki parselin ortak köşesi tek ad alır.
4. **Kenar:** köşeden halkadaki sonrakine düz uzaklık, yani kiriş. Arazide iki köşe taşı arasında ölçülen budur. Yaylı
   kenarda r = c / (2 sin(θ/2)) ve yay boyu r·θ da verilir (θ = 4 atan|b|). Uzunluklar projeksiyon düzlemindedir.
5. **Alanlar:** halkanın alanı köşe koordinatlarından, saklandığı gibi: shoelace ve her yayın parçası r²/2 (θ − sin θ),
   bükümün işaretiyle; mutlak değeri.
   - Parselin alanı dış halkaların alanı eksi delikleridir.
   - Çekirdeğin `bulge_ring_area` ve `entity_area` işlevleri kullanılır; Parsel alan denetimi de bunları kullanır.
   - Alan düzlemseldir: projeksiyon düzleminde, kaynak koordinatlardan. Çizimin üçgenlemesi kullanılmaz (CLAUDE.md
     §23.3).
6. **Kayıt** (ADR 1400 ile aynı): `field` kırpılır, düz ondalık (`301,2`, `200.10`) okunur, öbür yazı okunmaz.
   - Fark hesap − kayıttır. Yanılma sınırı 2022 Yönetmeliği m. 8(2)'nin arşiv onaylı geometri tablosundan, hesap
     alanıyla bulunur (artan oranlı dilimler; küsurat alt dilimden).
   - Durum `within`, `outside`, `noRecord` ya da `invalidRecord`'dur.
7. **Başlık:**
   - “{Ada} ada {Parsel} parsel”: yazılı olan parçalar;
   - ikisi de yoksa alanın etiketi;
   - o da yoksa raporun sırasıyla “Alan n”;
   - Mahalle yazılıysa sonuna “ ({Mahalle})”.

   Çizimden gelen her söz tek satır yapılır (denetim karakteri boşluk; 1000 harften ötesi kesilir).
8. **Sözler** (`cells`, satır satır):
   - başlık “Parsel alan çizelgesi (EPSG:5254)”; sistemsiz projede parantez yoktur;
   - sütun başları Nokta, Y (m), X (m); `lengths` ile Kenar (m); kenarlardan biri yaysa Yarıçap (m);
   - her parsel için başlığı; ilk dış halkadan sonraki halkaların üstünde “Delik n”, “Parça n” ya da “Parça n, delik m”
     (Köşe tablosunun adları, ADR 0172 §2);
   - köşe satırları: ad, Y, X, kenar, yarıçap;
   - “Hesap alanı (m²)”; kayıt varsa “Tapu alanı (m²)” yazıldığı gibi; okunursa “Fark (m²)” (artı farkın önünde “+”)
     ve “Yanılma sınırı (m²)” (“±”);
   - birden çok parselde “Toplam hesap alanı (m²)”.
   - **Eksenler:** Y doğu, X kuzeydir. Bu, kadastronun düzenidir (Harita'nın parsel pencereleri de böyle yazar);
     projenin çizim eksenlerinin adından bağımsızdır.
   - **Birimler:** metre ve m². Tapu alanı m²'dir; dönüm ya da hektar seçili projede de çizelge m² yazar.
   - **Sayılar:** gösterim kuralıyla (ADR 0149). Koordinat ve uzunluk projenin uzunluk ondalığıyla, alan alan
     ondalığıyla yazılır. Öntanımlı 2 alan ondalığı desimetrekaredir; m. 8(2)'nin mertebesi budur. Sözler gösterimdir,
     hesaba girmez; cevaptaki sayılar yuvarlanmamıştır (CLAUDE.md §23.2).
9. **Tablo:**
   - Hücreler bu sözlerdir. Başlık, parsel başlıkları ve halka adları satır boyunca birleşir; alan satırlarının adı
     son sütun dışındaki sütunlarda birleşir, değeri son sütundadır.
   - İlk satır başlık satırıdır. Sütun başları ortalı ve kalın, parsel başlıkları kalındır. İlk sütun sola, öbürleri
     sağa dayalıdır.
   - Sütunlar sözlerine göre genişler (Tablo ekle'nin kuralı). Birleşik alanın sözü sığmazsa alanın son sütunu
     genişler.
   - Tablonun sınırlarını (10 000 satır, 100 000 hücre) aşan çizelge `invalid_table` ile reddedilir.

### 4. Retler ve uyarılar

Sırasıyla:

| Kod | Komut | Ne zaman |
|---|---|---|
| `not_finite` | table | `at`'in bir koordinatı sonlu değil |
| `invalid_table` | table | `height` sıfırdan büyük ve sonlu değil; yazılacak tablo tablonun kurallarını aşıyor |
| `invalid_input` | ikisi | `prefix` 12 harften uzun ya da denetim karakterli; `field` 120 harften uzun |
| `invalid_uid`, `entity_not_found`, `not_a_parcel` | ikisi | `uids[i]` kimlik değil, çizimde yok, alan değil |
| `no_objects` | table | yazılacak parsel yok (boş `uids` ya da alansız parsel katmanı); rapor boş liste verir |
| `invalid_revision`, `revision_conflict` | table | beklenen sürüm |
| `layer_not_found`, `not_a_layer`, `layer_locked` | table | tablonun katmanı |
| `layer_hidden` (uyarı) | table | katman gizli: tablo yazılır, görünmez |

Yeni kod eklenmedi.

### 5. Arayüz

- **Şerit:** Harita › Plan › Ada ve parsel › **Parsel alan çizelgesi**, küçük düğme, Parsel alan denetimi'nin yanında.
  - Panelin büyük düğmeleri Parsel ve Sınır olarak kalır. DESIGN.md §7.3.1 ve ADR 9300 bir panelde en çok dört büyük
    düğme ister; çizelge bir çıktıdır, ana araç değildir.
  - Simge envanterin kendi `parcelReport` simgesidir (satırlı sayfa); başka bir komutla paylaşılmaz.
  - Komut `ribbon.py`'nin `MODULES.only` satırından şeride taşındı.
- **Pencere** (`apps/desktop-map/src/parcel_report_window.rs`):
  - tek satırlık öncü; özet (parsel, köşe, noktadan ad alan ve yeni numara alan köşe, toplam alan; sınır dışı kayıt
    varsa uyarı);
  - form: Köşe adları (Noktalardan / Hepsine yeni numara), Numara öneki, İlk numara, Kenar uzunlukları;
  - kitin veri tablosu: çizelgenin satırları, dikey çubuklu;
  - düğmeler: CSV kaydet…, Kapat, **Tabloyu yerleştir**.
  - Kapsam seçimin alanlarıdır, seçimde alan yoksa parsel katmanı. Seçimler program açık kaldıkça kalır.
- **Yerleştirme** (`kentos_interaction::parcel_table_place`): pencere kapanır, tablo sol üst köşesinden imlece asılı
  gelir (çizgileri kesikli, sözleri soluk). Tık ya da yazılan Y,X tabloyu `cad.parcels.table` ile yazar, seçer ve
  ileti verir; Enter ve Esc yazmadan çıkar.
- **CSV kaydet:** tablonun sözleri, `;` ayrılmış, UTF-8. `;` ya da `"` taşıyan söz tırnaklanır.

### 6. Tablo kendini güncellemez

Tablo kaynağını saklamaz. Tabloyu güncelle onu yenilemez; parseller değişince çizelge yeniden yerleştirilir ve
yerleştirme iletisi bunu söyler. Kaynağı saklamak yeni bir `TableSource` ve `.kcad` şeması ister: bu dilimin dışında,
sonraki iştir.

### 7. Modüller

| Modül | Var / yok | Gerekçe |
|---|---|---|
| Harita | **var** | Kadastro ölçmesi, değişiklik ve kamulaştırma haritası Harita'nın işidir (TKGM 2025/4 m. 21, 22, 25). Plan › Ada ve parsel'de durur. `cad.parcels.*` Harita'nın komutlarıdır (`registry::discipline_of`: Map). |
| Mimari | yok | Mimarın parseli imar durumunun girdisidir (`cad_zoning_parcel`). Alan hesap çizelgesini ölçme yapan düzenler; Mimari şeridinde Plan sekmesi yoktur (ADR 0801). |
| Yol | yok | Şeridin kuralına göre (`ribbon.py`) parsel, imar ve adres Yol'un değildir. Yolun kamulaştırma parselleri kamulaştırma işiyle (C2) gelecektir. |
| CBS | yok | Komutlar çizimin alanlarını okur. CBS'de parsel bir veri katmanının kaynaktaki satırıdır (ADR 9101, 9196), çizimin nesnesi değildir. CBS çizim tablosu da yazmaz: Tablo komutları CAD'indir (ADR 0184 §9). CBS'nin karşılığı Veri › Tablo › Parsel alan denetimi'dir (`parcels.check`, CSV'siyle); paftaya giden tablo paftanın katmana bağlı tablo öğesidir. Veri satırlarından parsel çizelgesi ayrı bir iştir. |

- Komut kümeleri değişmedi: `map.parcelReport` önce de yalnız Harita'nındı (`modules.only`), şimdi Harita'nın
  şeridindedir. `catalog/v1/modules.json` yeniden üretildi, fark çıkmadı.
- `harita_offers_it_mimari_yol_and_cbs_do_not` dört modülü sınar; `module_tests`'in modül sınamaları (Mimari ve CBS'de
  yok, Harita'da var) değişmeden geçer.

### 8. Kanallar

Kanallar arayüzle aynı komutları çalıştırır; hesabı kendileri yapmaz.

| Kanal | Var / yok | Nasıl ulaşılır |
|---|---|---|
| Python | var | `kentos.cad.parcels.report(doc, names=…, prefix=…, start=…, lengths=…, field=…, uids=…)` ve `kentos.cad.parcels.table(doc, at=…, …)`; `plan`, `validate` ve `answer` de. SDK katalogdan üretildi. Ortak durumlar Python oynatıcısında da koşar. |
| MCP | var | Katalog araçları `cad.parcels.report` ve `cad.parcels.table`; kapı `map.run`. `table` yeni bir nesne ekler, var olanı değiştirmez: `DESTRUCTIVE`'te değildir. Yine de yazan komut önce `plan`, sonra aynı girdi ve planın kimliğiyle `execute` edilir. |
| Yapay zekâ ajanı | var | MCP ile aynı araç katmanı (`kentos_headless::agent_tools`, `map.run`). Harita programının ajanı bu komutları görür (çekirdek ve Harita). |
| Eklenti | var | `run` ya da `call` ile. Aracı işlemin adına bakar (ADR 8057): `plan` ve `validate` okumadır, `document.read` yeter; `execute` her komutta yazmadır, `document.write` ister. Eklenti raporu `plan` ile okur, tabloyu `execute` ile yazar; tablo eklentinin işinin tek geri alma adımına girer (adı eklentinin komutunun adıdır). Geri al eklentiye kapalıdır. |
| CSV kaydet | pencere | Dosya penceresi kabuğundur. Kanallar aynı sözleri cevabın `cells`'inden alır, kendisi yazar. |

### 9. Performans

- Komut arayüz iş parçacığında anlık çalışır (`Instant`). Pencere her seçimde raporu yeniden okur.
- Köşe adları 1 µm'lik ızgarayla aranır; bir köşe için iş sabittir. Toplam iş köşe ve nokta sayısıyla doğrusaldır.
- Büyük çizimde süre ölçülmedi.

## Uygulama

- **Bölme (kaynak boyu, CLAUDE.md §8):** 2 000 satırı aşan iki dosyaya ekleme yapmadan önce bölündüler.
  - Masaüstünün komut tablosu `DESKTOP_COMMANDS`, `crates/native/application/src/lib.rs`'ten
    `core/desktop_commands.rs`'e taşındı (`lib.rs` 2 130 sayılan satırdan sınırın altına indi, tabandan çıktı).
  - Kataloğun örnek girdi türleri sınaması `crates/shared/commands/src/catalog/mod.rs`'ten `catalog/example_inputs.rs`'e
    taşındı (4 680 → 3 230 satır, taban düşürüldü).
- **Sözleşme:** `crates/shared/commands/src/map/cad_parcels_report.rs` (`ParcelsReport`, `ParcelsReported`,
  `ParcelReported`, `ReportRing`, `ReportCorner`, `CornerNames`, `CornerNamed`, `ParcelsTable`, `ParcelsTablePlan`,
  `ParcelsTabled`); kataloğun iki tanımı (`catalog/map.rs`), yetenekleri (`capabilities/map.rs`: kodlar ve `polygon`
  hedefi, `not_a_parcel` ile), örnek girdi türleri; `commandCatalog.json` ve TS türleri yeniden üretildi.
- **İşleyici:** `crates/native/application/src/map/parcels_report.rs` (kurallar ve `report`),
  `map/parcels_report_table.rs` (`table`); `map/dispatch.rs`, `core/registry.rs`, `core/desktop_commands.rs`.
- **Ortak durumlar:** `scripts/fixtures/parcels_report_cases.py` → `fixtures/commands/v1/cad.parcels.report.json`
  (25 durum) ve `cad.parcels.table.json` (23 durum); masaüstü oynatıcısı (`tests/all/fixtures/run_map.rs`,
  `inputs_map.rs`), yetenek sınamasının tür denemeleri (`tests/all/capabilities.rs`).
- **Kanallar:** Python SDK (`python/kentos/cad/parcels.py`, `types.py`, `_index.py`, `__init__.py`; `scripts/python/sdk.py`),
  MCP sınaması `crates/native/mcp/tests/parcels_table.rs`, eklenti sınaması
  `apps/desktop/src/shell/extensions/trial/broker_tests.rs`, geçmişin adları `apps/desktop/src/cad/history/text.rs`.
- **Arayüz:** `apps/desktop-map/src/parcel_report_window.rs` (pencere), `state.rs`, `parts.rs`, `ext.rs`, `lib.rs`;
  yerleştirme `crates/native/interaction/src/map/parcel_table_place.rs`; şerit `scripts/ui/ribbon.py` →
  `catalog/v1/ribbon.json`; `catalog/v1/operations.json` (`semantic`, `via` iki komut); `apps/desktop/equivalents.json`,
  `apps/desktop/src/shell/catalog/ported.rs`, `apps/desktop/ported.json`; manifest `docs/cad/manifest.{json,md}`.
- **Sınamalar:** `apps/desktop-map/src/tests/parcel_report_window_tests.rs` (rapor, seçimler, seçim kapsamı, CSV,
  yerleştirme ve geri alma, modüller, resimler).

## Doğrulama

24 Kasım 2026, bu makinede (macOS, Apple Silicon):

- `scripts/fixtures/py scripts/fixtures/parcels_report_cases.py --check`: güncel (25 + 23 durum). Başvuru KentOS kodu
  kullanmaz: shoelace kesirlerle, yay parçası 50 basamakla (mpmath), yanılma sınırı yönetmeliğin tablosundan, sözler
  gösterim kuralının kendi başvurusuyla (`numeric_display.py`).
- `cargo test -p kentos-native-application --test all`: `every_command_case_matches_the_desktop_handlers` (iki dosyanın
  48 durumu), `capabilities` (4 sınama; iki komutun tür denemeleri dahil), `catalog`, `disciplines` geçti.
- `KENTOS_WRITE_CATALOG=1 cargo test -p kentos-commands catalog`: 13 sınama geçti (örnek girdi türleri, birimler,
  paylaşılan katalog); TS türleri `cad_parcels_report` sınamalarıyla yazıldı.
- Python: `scripts/python/sdk.py --check` güncel; `KENTOS_CASES_ONLY=cad.parcels.report,cad.parcels.table` ile
  `test_command_cases.py` (tipli katmandan iki dosya) geçti.
- `python3 scripts/cad/manifest.py --check`: tutarlı, 1094 satır; `map.parcelReport` `semantic`, ürün komutu olmayanlar
  20 → 19. `python3 scripts/ui/ribbon.py --check`: güncel. `make arch`'ın iki denetimi (`pnpm arch:deps`,
  `source_size.py`) ve `scripts/dev/module_deps.py --check` temiz.
- `cargo test -p kentos-desktop-map --lib parcel_report_window_tests`: 5 sınama geçti (rapor ortak durumun sözleri, seçimler,
  seçimin kapsamı, CSV, Tabloyu yerleştir ve tık, tek geri alma, Harita var; Mimari, Yol, CBS yok).
- Bu ADR yazılırken sırada olan ya da koşmayanlar raporda ayrıca söylenir (masaüstü sınamaları, Harita paketinin
  pencere sınamaları, MCP sınaması, `kentos-cad` matrisi, resimler).

## Sahibin kararı bekleyenler

- Kurumun alan hesap cetveli biçimi (başlık, imza yerleri, Gauss ara sütunları): bir örnek ya da şablon gelirse sütunlar
  ona göre eklenir.
- Tablonun kendini güncellemesi (§6): yeni kaynak türü ve `.kcad` şeması.
- CBS'de veri satırlarından parsel çizelgesi.
