# 4940 — Katalog geçiş raporu: çizimin verisinin bir nesne kataloğundan ötekine

Tarih: 2026-11-11. plan/map-cad.md MAP-04, onuncu tur. Durum: taslak. ADR 1760, 4140 ve 4540'ın “Sonra”sındaki
“katalog (profil) değişikliğinin geçiş raporu”; planın “katalog değişikliği mevcut veri için geçiş raporu üretsin” ve
kabulün “model/katalog değiştiğinde kabul edilmiş insan düzeltmeleri korunabilsin” satırları.

İlgili:
- [ADR 0541](0541-data-profiles.md) ve [ADR 0558](0558-institution-profile-import-migration-mapping.md): veri profili,
  kurumun kendi profili, bir profilin sürümleri arasında göç (`cad.profiles.migrate`);
- [ADR 0735](0735-profile-files-inside-the-project.md): çizimin taşıdığı profil dosyaları;
- [ADR 1360](1360-field-codes-and-kodla-ciz.md): sınıfların saha kodları, Harita varlıkları profili;
- [ADR 4140](4140-reconcile-decisions-as-review-notes.md): Detay eşleştirmenin kararları çözülmüş inceleme notu.

## Kapsamın seçimi

ADR 2140 “katalog geçiş raporu `cad.profiles.migrate`'te (plan ile) zaten vardır” diyordu. Göçün planı yalnız **aynı
profilin** iki sürümü arasında, profilin kendi yazdığı adımlarla çalışır (katmanı ya da alanı yeniden adlandır, türünü
değiştir, kodları eşle, kodu kaldır). Eksik olan:
- **iki ayrı katalog** (KentOS'un Harita varlıkları ile kurumun yeni kataloğu, ya da kurumun iki ayrı yayını) arasında
  sınıfların nereye gittiği: aynı, yeniden adlandırılmış, bölünmüş, birleşmiş, kalkmış, yeni;
- her değişikliğin **çizimdeki kaç nesneye** dokunduğu ve hangisinin **karar istediği**;
- Detay eşleştirmenin **kayıtlı insan kararlarına** (ADR 4140) ne olacağı.

Bu tur raporu yapar; veriyi taşımaz (taşıma Profil göçü ve Veri profilini uygula'dır). AI çıkarımı ve gerçek
çıkarımın adaylarıyla ölçüm sahibin bekleyen işleridir, bu turda yok.

## Dayanak

Ayrıntı: `.run/research/map-cad/mevzuat/MAP-04.10.md` ve `.run/research/map-cad/uygulama/MAP-04.10.md` (resimler
`uygulama/MAP-04.10-resim/`).

- **TUCBS Genel Kavramsal Model** (Sürüm 1.0, 2019; 10.11.2026 okundu): §9.4.1 uygulama şemasının adı ve sürümü
  vardır, detay kataloğu sağlanır; §9.3 ve §17 kayıtlarda değişen ya da kalkan öğe kayıtta kalır ve her öğenin kalıcı,
  belirsizliğe yer vermeyen tanımlayıcısı vardır; §14.5 sürüm tanımlayıcısı kimliğin parçası değildir; §19 bakımda
  değişikliğin nedeni kullanıcıya yararlıdır. Buradan: sınıf **kalıcı kimliğiyle** izlenir; eşleme türleri aynı,
  yeniden adlandırılmış, bölünmüş, birleşmiş, kalkmış; etkilenen nesne sayısı.
- **CBK 49** Geçici m. 1(2): kurumlar eski verilerini yayımdan sonraki üç yılda uyumlu hâle getirir. Yalnız kamu
  kurumlarının TUCBS temaları içindir; KentOS iş kuralı yapmaz.
- **BÖHHBÜY** Ek-2 §3.1.1 (UVDF `KatalogDosyasiAdi`), §7 (eski BÖHYY kodları “*” önekiyle): katalog dosyada adıyla
  anılır; eski kodun yaşaması mevzuatta örneği olan bir yoldur.
- **Netcad:** wiki bu oturumda açılamadı (sitenin sertifika zinciri; denetim kapatılmadı, arşiv kopyası yok). Arama
  özetine göre Kod Kataloğu kodun katmanını, rengini ve çizgi tipini tanımlar ve dosyaya kaydeder (doğrulanmadı; değer
  alınmadı). VGA (Veri Güncelleme Aracı, ürün sayfası 11.11.2026 okundu): CAD verisi önceden tanımlı kurallarla
  veritabanına aktarılır, zorunlu alanlar tasarlanır, değişen bütün veri için önce/sonra durumu raporlanır.
- **hale studio** (GeoServer eğitimi, 11.11.2026 okundu): şemalar arası eşleme önce türler (Retype, birden çok kaynaktan
  Join), sonra özellikler (Rename, Assign); hedefte hücresi olmayan özellik boş kalır.
- **ArcGIS Pro Feature Compare** (11.11.2026 okundu): alan tanımlarını da karşılaştırır, sonucu satır başına fark
  işaretli bir tablodur; kayıt eklenince ya da silinince sıra kayar, eşleme kimlikle yapılmalıdır.

## Karar

1. **Yeni komut `cad.profiles.compare` v1 — Katalog geçiş raporu.** Salt okunur (Query, geri alma yok); plan aynı
   cevabı verir. Girdi: eski katalog `from` (yerleşik ya da projenin profilinin adı: çizimin taşıdığı, yoksa `profiller`
   klasöründeki) ya da `fromProfile` (profilin kendisi; verilince ad okunmaz); yeni katalog `to` ya da `toProfile`;
   `drawing` (ev sahibi kendisi doldurur). Retler sırayla, önce eski sonra yeni: `unknown_profile` (`from`, `to`),
   `invalid_profile` (`fromProfile`, `toProfile`; projenin bozuk dosyası `from`, `to`). İletiler “Eski/Yeni katalog …”.
2. **Sınıfın izlenmesi.** Her eski sınıf, eski kataloğun sırasıyla:
   - yeni katalogda **aynı `id`** varsa ona gider: katlanmış katman yolu, geometri, kodlar ve alanlar aynıysa `same`,
     değilse `changed`;
   - yoksa **saha kodlarıyla**: kodlarından birini taşıyan yeni sınıflar (yeni kataloğun sırasıyla). Hiçbiri yoksa
     `retired`; iki ve daha çoğuysa `split`; biriyse, o sınıf bir eski sınıfın `id`'sini taşıyor ya da `id`'siyle
     eşleşmeyen başka bir eski sınıf da ona gidiyorsa `merged`, değilse `renamed`;
   - hiçbir eski sınıfın gitmediği yeni sınıf, sonda, `added`.
   - Addan, başlıktan ya da konumdan tahmin yapılmaz. Kodsuz ve `id`'si değişmiş sınıf `retired` görünür; kurum `id`'yi
     korumalı ya da sınıfa eski kodunu vermelidir.
3. **Sınıfın satırı:** eski ve yeni sınıflar, katman yolları (`A › B`), kodlar, eski sınıfın katman yolunun götürdüğü
   katmandaki **bütün nesneler** (çizimin sırasıyla, kalıcı kimlikleri en çok 1000; kilitli ve gizli katman da sayılır,
   rapor yazmaz), açıklama (“Değişti: katman … → …; geometri çizgi → alan; kodlar …; n alan değişikliği.”, “Kodları
   bölünür: KN → …, PN → ….”). **Karar ister:** nesnesi olan `retired` ya da `split` sınıf, geometrisi değişen sınıf.
4. **Alanlar** (tek sınıfa giden sınıfta), eski sınıfın sırasıyla, ilk tutan: `removed` (yeni sınıfta o adda alan yok;
   değeri olan nesneler taşınamaz), `kindChanged`, `listChanged` (değer listesi değişti ya da kalktı; değeri olan
   nesneler yeniden okunmalı), `nowRequired` (boş olan nesneler); sonra yeni sınıfın yeni alanları `added` (boş olan
   nesneler). Alan türünün sözcüğü ve listesiyle anılır (`ondalık sayı`, `metin, sebeke`). Karar ister: nesnesi olan
   satır; `added` yalnız zorunluysa. Değişmeyen alan satır yazmaz.
5. **Değer listelerinin kodları**, eski listenin sırasıyla: yeni listede (ya da liste yeni katalogda) yoksa `removed`,
   yeni listede kullanımdan kalkmışsa `retired`, etiketi değişmişse `relabelled`; sonra yeni listenin yeni kodları ve
   yalnız yeni katalogdaki listelerin kodları `added`. Nesneler: eski sınıfların katmanlarındaki, listeye bağlı alanı o
   kodu taşıyan nesneler (çok değerli alanın `;` ile ayrılmış her kodu). Eski listede zaten kalkmış kod satır yazmaz.
   Karar ister: nesnesi olan `removed` ya da `retired` kod.
6. **Kayıtlı kararlar.** Çözülmüş inceleme notlarından kararı Detay eşleştirmenin (`map.features.reconcile`) olan ve
   ret olmayanlar, numara sırasıyla. Sınıfı: düzelttiği sınıf eski katalogda varsa o, yoksa varlığın katmanının sınıfı;
   ikisi de yoksa not dışarıda kalır. `classRetired`, `classSplit`; verdiği alan kalkıyorsa `fieldDropped`; sınıfı
   yeniden adlandırılmış ya da birleşmişse `follows`; değilse `kept`. Karar ister: ilk üçü. Böylece “kabul edilmiş insan
   düzeltmesi korunabilir mi” sorusu satır satır cevaplanır.
7. **Özet:** `decide` karar isteyen satırların sayısı; iki sabit not (rapor yazmaz, sınıflar kimlik ve kodla izlenir;
   karşılaştırma mevzuata ya da TUCBS kayıtlarına uygunluk iddiası değildir); `revision`.
8. **Masaüstü: Katalog geçiş raporu…** (`map.catalogReport`): Eski katalog ve Yeni katalog seçicileri (KentOS'un ve
   projenin geçerli profilleri; açılışta Harita varlıkları → projenin kataloğu), özet, Sınıflar | Alanlar | Kodlar |
   Kararlar sekmeleri ve kitin veri tablosu, **Karar isteyenleri seç** (gösterilen tablonun karar isteyen satırlarının
   nesneleri; kararlarda aday ve varlık). Şeritte Ölçme › Noktalar panelinin ▾ menüsünde, Detay eşleştirme'nin yanında (Harita işi) ve CBS'nin Veri › Standart
   panelinde Veri profili'nin yanında (CBS). Kapsam işareti: **Harita + CBS**.
9. **Python ve MCP:** `kentos.cad.profiles.compare` (`.plan`, `.validate`); MCP aracı `cad.profiles.compare` salt
   okunurdur.
10. **`.kcad` şeması değişmedi**; web ertelendi (TODOS §0.2).

## Sınırlar

- Rapor taşımaz: sınıfı değişen nesnelerin yeni katmana alınması, alanların yeniden adlandırılması ve kodların eşlenmesi
  bugün Profil göçü (aynı profilin sürümleri arasında) ve Veri profilini uygula iledir. İki ayrı katalog arasında
  rapordan göç adımı üretmek ayrı iştir.
- Sınıf kimlikle, yoksa kodla izlenir; kodun bölündüğü sınıfta hangi nesnenin hangi yeni sınıfa gideceği nesnenin kendi
  kodundan da okunabilirdi, bu tur okumaz (bölünen sınıfın nesneleri karar ister).
- Alanlar yalnız adla eşlenir; yeniden adlandırılmış alan `removed` + `added` görünür (Profil göçünün `renameField`'ı
  aynı profilin sürümlerinde bunu bilir).
- `kindChanged` ve `listChanged` değerleri yeni kurala göre denemez, yalnız sayar; değer değer sonucu göçün planı verir.
- Kurumun eşlemesi (ADR 0558 `mapping`) raporda okunmaz: kataloglar kendi adlarıyla karşılaştırılır.
- CBK 49'un üç yıllık süresi iş kuralı değildir.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/profile_compare_cases.py`; KentOS kodu yok, kurallar bu ADR'den): eski
  katalog KentOS'un `harita-varliklari` profili (veri), yenisi betikte yazılan `belediye-harita` 2. sürüm (kontrol
  noktası KN/PN'ye bölünür, direk yeniden adlandırılır, bordür ile yol kenarı birleşir, ağaç kalkar, rögarın kapak kotu
  kalkar ve derinliği metin olur, zorunlu malzeme gelir, kapı numarası Numarataj olur, duvar alan olur, bina izinin kat
  sayısı zorunlu olur, levha sınıfı gelir; şebeke listesinde doğal gaz kalkar, haberleşme kullanımdan kalkar, yağmur
  suyunun etiketi değişir, atık su gelir; direk türü listesi kalkar, levha türü gelir). 21 durum: ana rapor (16 karar
  isteyen satır), ters yön, aynı katalog, sınıf katmanları olmayan çizim, çok değerli alan, kilitli ve gizli katman,
  Türkçe büyük harfli katman yolu, gruba çıkan yol, varlığın katmanından okunan karar sınıfı, kalkan liste, kimliği
  kalan sınıfın kodunun gitmesi, eski sınıfın kimliğine birleşme, kodu düşen tek hedef, boş katmanlar, zaten dolu
  zorunlu alan, eski listede kalkmış kod, dört ret. `KENTOS_CASES_ONLY=cad.profiles.compare … every_command_case
  capabilities` geçer; Python koşucusu (`test_command_cases`) geçer.
- **Bozma denemesi:** ana durumda direğin `renamed`'i `same`, aynı katalog durumunda ilk kararın `kept`'i `follows`
  yapıldı; koşucu tam bu iki durumu yakaladı (2/21), dosya geri kondu.
- **Python** (`python/tests/test_profile_compare.py`), testin kendi okumasıyla: her sınıf satırının nesne sayısı
  katmanının nesneleri; kimliği kalan sınıf same/changed, kodla giden renamed/merged ve hedefi eski kodları taşır;
  `decide` karar isteyen satırların toplamı; `profiller` klasöründeki katalog adıyla bulunur ve kendisiyle verilenle
  aynı raporu verir; sürüm değişmez; bilinmeyen katalog `unknown_profile`.
- **MCP** (`crates/native/mcp/tests/profiles_compare.rs`): çizimin taşıdığı yeni katalog kendisiyle ve adıyla ilk
  durumun raporunu verir; bilinmeyen katalog reddedilir; sürüm değişmez.
- **Masaüstü** (`catalog_report_window_tests`): pencere Harita varlıkları → projenin kataloğu ile açılır ve ortak
  durumun raporunu verir; dört tablonun satırları; Karar isteyenleri seç karar isteyen sınıfların dört nesnesini seçer;
  aynı katalog karar istemez; Harita ve CBS açar. Çapraz denetimler (module_tests, ribbon_tests, history::text, catalog,
  settings) geçer. Resimler `.run/gui-evidence/katalog-gecisi-4940/` (açık ve koyu, geniş ve dar, şerit).

## Sonra

- Rapordan iki katalog arasında göç adımları üretmek ve uygulamak (sınıfı değişen nesnelerin yeni katmana taşınması,
  alanların ve kodların eşlenmesi) tek geri alma adımıyla; bölünen sınıfın nesnelerinin kendi kodlarıyla dağıtılması.
- AI çıkarımı; gerçek çıkarımın adaylarıyla ölçüm (sahibin bekleyen işleri).
- Web (TODOS §0.2).
