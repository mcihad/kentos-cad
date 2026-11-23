# ADR 9192: Tek kaynak kataloğu: veri katmanı, bağlı katman ve alınmış kopya aynı dosyanın altında

- **Durum:** kabul edildi (2026-11-23); adım VY-91. Plan: [plan/veri.md](../../plan/veri.md) VY-91.
- **İstek:** sahibin sözü (23 Kasım 2026): “Yarım kalan maddeleri tamamla”. Değerlendirmenin 5. maddesinin kalanı (ADR
  9129 §4): “Projeye alınmış kopyaların ve eski bağlı katmanların aynı katalogda fiziksel kaynağa bağlanması.” Maddenin
  kabulü: “Kaynak kataloğu doğrudan veri katmanını, projeye alınmış kopyayı ve eski bağlı katmanı ayrı gösterip hepsini
  fiziksel kaynağa bağlasın.”
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **Komut:** yeni komut yok; `file.import.*`'ın yaptığı katmanlar
  kaynağını taşır.
- **`.kcad`:** katman düğümüne `origin` eklendi, **CBS verisi sürüm 7**. Rust okuyucu ve yazıcı, bağımsız Python okuyucu
  (`tools/kcad/kcad.py`) ve başvuru yazıcısı (`scripts/fixtures/kcad_v2_reference.py`) alanı sürümle kapılar.
  [docs/specs/kcad-v2.md](../specs/kcad-v2.md) yazıldı. Web dururken envanter donmuştur (ADR 0228).

## §1 Bağlam

- Kaynak durumu bağlı katmanları (ADR 0543) ve veri katmanlarını (ADR 9129) ayrı tablolarda gösteriyordu.
- İçe alınan bir tablonun katmanı (bir kopya) nereden geldiğini bilmiyordu: içe alma katmana yalnız veri kümesi bilgisini
  ve kaynak sistemini yazıyordu. Aynı GeoPackage'tan bir veri katmanı, bağlı bir katman ve bir kopya bir arada
  durduğunda, üçünün aynı dosyadan geldiği hiçbir yerde görünmüyordu.
- Türetim kaydı (ADR 0633 §2) kullanılamazdı. Girdisi çizimin bir katmanıdır; çizimde katman olmayan bir girdi katmanı
  “güncel değil” yapardı (`datasets_describe::inputs_now`).

## §2 Karar

1. **Katmanın kaynağı** (`LayerNode.origin`, `kentos_contracts::LayerOrigin`). Katmanın nesnelerinin kopyalandığı yer:
   - biçim (`driver`: `gpkg`, `parquet`, `csv`, `fgb`, `gml`; veri sürücülerinin adıyla);
   - dosyanın tam yolu ya da veritabanının parolasız adresi (`address`);
   - okunan tablo (`table`; GeoPackage'ta ve birkaç tablolu dosyada yazılır, tek tablolu dosyada yazılmaz);
   - dosyanın okunduğu andaki SHA-256'sı (`sha256`);
   - okunduğu zaman (`at`, UTC, saniyesiyle).

   Kaynak bağlı değildir, ona bir şey yazılmaz. Yalnız katmanın olur. `layer_origin_problem` biçimi denetler: sözcük
   biçim, boş olmayan ve parolasız adres, 64 küçük onaltılık haneli özet, `2026-11-23T09:30:00Z` biçimli zaman.
2. **İçe alma yazar.** `file.import.gpkg`, `.csv`, `.fgb`, `.gml`, `.parquet` ortak yoldan (`run_read`) geçer ve yaptığı her
   katmana kaynağı yazar. Kayıt içe almanın geri alma adımının içindedir: geri alma kopyayı kaynağıyla götürür, yineleme
   getirir. Satırların gittiği, çizimde zaten olan bir katman kendi kaydını korur. Katmanın bilgisi düzenlenince
   (eylemler, kart, veri kümesi) kaynak kalır.
3. **`.kcad`, CBS verisi 7.**
   - Katmanda `origin` varsa yazıcı `modules`'e `gis: 7` yazar. Alan, kanonik sırada 6 harfli anahtarlar arasında
     `locked`'dan sonra gelir.
   - CBS verisi 6'nın yükünde bilinmeyen alandır.
   - Grupta olması, parolalı adres ve başka biçimde özet `bad_value`'dur.
   - Örnek dosya `layer-origins.kcad`, bozuk örnekleri dört tanedir.
4. **Kaynaklar** (`gis/source_catalog.rs`). Kaynak durumu'nun başında, projenin her fiziksel kaynağı altında ondan
   gelen katmanlar listelenir, ağacın sırasıyla:
   - **Veri katmanı:** satırları kaynakta; sayısı Veri katmanları'nda;
   - **Bağlı katman:** nesneleri projede, kaynağa yazılır; durumu Bağlı katmanlar'da;
   - **Alınmış kopya:** nesneleri projede, kaynak bağlı değil. Ne zaman alındığını ve dosyanın o günden beri değişip
     değişmediğini söyler.
   - Dosya kaynağı adı ve klasörüyle yazılır, yerinde değilse bu söylenir. Veritabanı adresiyle yazılır.
   - Kopyanın dosyası pencere açılınca ve Yenile'de arayüzü bekletmeden yeniden okunur, SHA-256'sı alındığı günkiyle
     karşılaştırılır: “dosya okunuyor…”, “dosya değişmedi”, “dosya sonra değişti”, “dosya yerinde yok”.
   - Satıra basınca katman Katmanlar'da seçilir.
   - Bağlı katmanı olmayan ama kopyası olan projede ilk satır “Projede bağlı katman yok; alınmış kopyaların kaynakları
     Kaynaklar'da.” der.

## §3 Kanıt

- **Sözleşme ve `.kcad`:** `kentos-contracts` (1671) ve `kentos-kcad` sınamaları geçiyor. Başvuru yazıcısı ve bağımsız
  okuyucu 1888 dosyada tutarlı (`--check`). Rust yazıcı yeni örneği başvurunun baytlarıyla aynı yazıyor; bozuk dört
  örnek beklenen hatayla reddediliyor.
- **Alan modeli:** `kentos-domain` sınamaları geçiyor (kaynak, katman bilgisinin geri alma adımında).
- **Masaüstü** (`gis/source_catalog_tests.rs`):
  - `a_files_data_layer_bound_layer_and_copy_stand_under_the_file`:
    - içe alınan kopyanın kaynağı `gpkg`, dosyanın yolu, `parseller`, dosyanın SHA-256'sı;
    - tek kaynağın altında üç yol;
    - pencere açılınca “dosya okunuyor…”, okununca “dosya değişmedi”;
    - dosya değişince “dosya sonra değişti”, silinince “dosya yerinde yok”;
    - satıra basınca katman seçiliyor.
  - `the_imports_undo_takes_the_copy_and_a_written_project_keeps_its_origin`: `.kcad`'e yazılıp okununca kaynak aynı;
    geri alma kopyayı götürüyor, yineleme kaynağıyla getiriyor.
- **Ortak durumlar:** uygulamanın bütün ortak komut durumları (22.471) geçiyor. Başsız içe alma ve CBS sınamaları,
  masaüstünün Kaynak durumu, GeoPackage, veri katmanı ve ortak sınamaları (213) geçiyor.
- Bu işte ortaya çıkan iki eski açık ayrı işlendi:
  - fd348a862: ADR 9191'in işlem sonucuna taşıdığı alanı geri aldı; beş ortak durum düşüyordu.
  - 671a70f6b: ADR 9135'ten beri derlenmeyen sunucu uygulaması düzeldi.
- **Resim:** `.run/gui-evidence/kullanim-9192`: tek GeoPackage'ın altında veri katmanı, bağlı katman ve “dosya
  değişmedi” diyen kopya (Git'e girmez).

## §4 Kalan

- Öbür içe almalar (Shapefile, GeoJSON, KML, DXF) ve veritabanından kopya (`postgis`) kaynağını yazmıyor: ortak yoldan
  geçmiyorlar.
- Kopyayı kaynağından yeniden almak (“kaynaktan tazele”) ve kopyayı veri katmanına çevirmek.
- Kopyanın PostGIS kaynağında tablonun değişip değişmediği (bugün yalnız zamanı yazılır).
- Gerçek pencerede deneme; web (TODOS.md §0.2).
