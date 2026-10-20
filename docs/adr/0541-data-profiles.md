# ADR 0541: Veri profilleri ve referans belediye projesi

- **Durum:** kabul edildi (2026-10-20). `plan/gis.md` CBS-34'ün ilk dilimi. Kapsamı ben belirledim.
- **Bağlam belgesi:**
  - ADR 0199 (katman alanları);
  - 0430 (değer listeleri);
  - 0405 (ilişkiler);
  - 0229 (adres arama);
  - 0312 (çizim standardı: aynı “denetle ve ayrı, planlı uygula” biçimi);
  - 0540 (komut türleri `kentos-commands`'ta).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - kurumun kendi TUCBS profilinin (alan adları, kodlar, metadata) içe alınması ve profil sürümleri arasında göç: ADR 0558 yaptı;
  - kurum eşlemeleri (bir alanın kurumdaki adı): ADR 0558 yaptı;
  - adres normalizasyonunun profile bağlı ayarları;
  - yinelenen adres kodunun adres sekmesinde gösterilmesi;
  - ifraz, tevhit, hisse dağıtımı ve PlanGML (bağımsız kurum kabulüne tabi uzman teslimleri);
  - referans projenin gerçek kullanıcılarla günlük kullanım kabulü (CBS-40).

## Bağlam

Ölçme, parsel üretimi, adres arama, kayıtlı alan ve CAD kesinliği vardı. Ama bir belediye projesinin verisi bir modele göre
kurulmuyor, ona göre denetlenmiyordu. Plan şunları istiyor:
- TUCBS'nin veri sözlüğü ve modeli üzerinden sürümlü bir profil;
- uygulama koduna gömülmemiş kolon adları;
- birbirinden ayrı tutulan geometrik alan, kayıtlı alan ve hak;
- yinelenen kodun ve adres eşleşmesinin açıklanması;
- kişisel veri içermeyen gerçekçi bir örnek proje.

Resmî çıktı ya da kurum kabulü olmadan mevzuata uygunluk iddia edilmemeli.

## Karar

1. **Profil veridir.** `catalog/v1/profiles/*.json` (`kentos.data-profile` 1) şunları tutar:
   - sınıflar: katman yolu (grupların ve katmanın adları), rengi, geometrisi (alan, çizgi, nokta, kayıt), alanları
     (`LayerField`), kimlik alanı ve kayıtlı alanı (alan ve toleransı, m²);
   - projenin değer listeleri, sınıflar arası ilişkiler ve adres verisinin eşlemesi (kaynakların `layer`'ı bir sınıf).

   Uygulama kodunda hiçbir kolon adı yoktur. İlk profil `tucbs-temel`:
   - sınıfları mahalle, ada, parsel, bina, yol orta hattı, kapı (adres) ve planlı alan kullanımı;
   - dört değer listesi;
   - parsel–bina ve bina–adres ilişkileri;
   - adres araması kapı, yol ve mahalleden.

   `basis` açıkça şunu söyler: resmî TUCBS uygulama şeması değildir, kurumun profiliyle eşlenip kurumca kabul edilmeden
   uygunluk iddia edilmez. Toleransı profilin değeridir, mevzuattaki hata sınırı değildir.

   Türler `kentos-commands`'ta (`cad_profiles.rs`), kuralları `profile_problem`'dadır.
2. **`cad.profiles.apply`** profilin çizimde olmayan parçalarını ekler, var olanı korur.
   - Katman yolu Türkçe katlanmış adlarla bulunur.
   - Eksik gruplar ve katmanlar, alanları değer listelerine bağlı olarak açılır.
   - Var olan katmana eksik alanlar eklenir.
   - Türü farklı alan (`profile_field_differs`) ve farklı değer listesi (`profile_domain_differs`) korunur.
   - Kilitli katman ve yolunda katman ya da grup olmayan sınıf atlanır (`profile_layer_skipped`).
   - Katmanlar ve alanlar tek geri alma adımıdır (“Veri profili”). Değer listeleri, ilişkiler ve adres verisi proje
     ayarıdır.
   - Retler sırayla: `unknown_profile`, `invalid_profile`, `invalid_revision`, `revision_conflict`.
3. **`cad.profiles.check`** salt okunurdur. Bulgular sırasıyla:
   - eksik değer listesi;
   - sınıf sınıf: eksik katman, eksik alan, türü farklı alan, başka geometri, boş zorunlu alan, kurala uymayan değer
     (değer başına, alanın `check_value`'su ile), yinelenen kimlik, alan farkı;
   - ardından eksik ilişki ve kopuk bağ;
   - en sonda eksik adres verisi.

   Her bulgu nesnelerin kalıcı kimliklerini (en çok 1000) ve sayısını, Uygula'nın düzeltip düzeltmediğini ve Türkçe
   iletisini taşır. Alan farkında iki alan da söylenir; **hiçbiri öbürünün yerine yazılmaz**. Geometrik alan delikler
   çıkarılarak, çizim biriminden m²'ye çevrilerek ölçülür. `clean` bulgu olmadığını söyler, mevzuata uygunluğu değil.
4. **Referans proje** `fixtures/profiles/v1/belediye.json`, kişisel veri içermez.
   - İçeriği: Sivas'ta uydurma bir mahalle, iki ada, altı parsel (biri delikli), beş bina (biri nokta), iki yol, dört kapı.
   - İlişki bağları `links.json`'dadır; açılıştan sonra `cad.entities.set` ile yazılır. v1 çiziminin kimlikleri
     içeriğinden türer, bu yüzden bağ çizimin kendisinde tutulamaz.
   - Kasıtlı sorunlar: yinelenen parsel kimliği, 12,40 m² alan farkı, liste dışı nitelik, aralık dışı kat sayısı, liste
     dışı yol türü, boş kapı no, uzun posta kodu, kopuk bağ.
5. **Masaüstü.** CBS › Veri › Standart'ta Veri profili penceresi:
   - profilin adı ve temeli;
   - bulgular tablosu: tür, sınıf, konu, nesne sayısı, ✓ Uygula düzeltir;
   - satıra basınca nesneler seçilir ve gösterilir;
   - Uygula'nın önizlemesi komutun planıdır.

   Komut `profile.window` (`catalog/v1/additions.json`, `dataProfile` ikonu).
6. **Python ve MCP** komutları katalogdan alır. Uygula yalnız ekler; MCP'de yıkıcı değildir.

Yolda düzelenler:
- masaüstünün geçmiş başlıklarında eksik `cad.time.query`;
- ek kaydında web envanteriyle çakışan `exportCsv` ikonu (`exportCsvTable` oldu);
- iki komutun `KARSILASTIR` takma adı (Kaydırmalı karşılaştır'ınki `KAYDIRKARSILASTIR` oldu);
- şerit testinin geniş pencere sınırı (CBS'nin Veri sekmesi 4524 px);
- ADR 0540'ta şema özelliği kapalıyken derlenmeyen iki `use` satırı.

## Doğrulama

- `python3 scripts/fixtures/profile_cases.py --check`: `cad.profiles.apply` ve `.check`'in 22 + 22 durumu, KentOS kodu
  olmadan.
  - Kurallar bu ADR'den ve profilin verisinden yazılmıştır.
  - Değerlerin kuralı `layer_field_cases.py`'nin `check`'idir; alan, delikleri çıkarılmış Gauss (shoelace) toplamıdır.
  - Durumlar: uygulama, ikinci uygulama, boş çizim, türü farklı alan, kilitli katman, engelli yol ve Türkçe katlama,
    kendisiyle verilen profil, korunan ilişki ve adres verisi, beklenen sürüm, sekiz geçersiz profil ve retler; denetimde
    bütün bulgular, Uygula'dan sonrası, temiz proje, delik, tolerans sınırı, anahtarla bulunan alan, grup engeli, üçlü
    kimlik, yalnız boşluk, santimetre çizimi, kopuk bağ.
- `KENTOS_CASES_ONLY=cad.profiles cargo test -p kentos-native-application --test all fixtures` ve
  `KENTOS_CASES_ONLY=cad.profiles .run/py/bin/python -m unittest python.tests.test_command_cases`: 44 durum iki
  tarafta.
- Resimler: `KENTOS_SHOTS_ONLY=profil-bulgular,profil-uygulandi,profil-serit cargo test -p kentos-desktop tools_screens --
  --ignored --nocapture` (`.run/shots/arac-profil-*`).
