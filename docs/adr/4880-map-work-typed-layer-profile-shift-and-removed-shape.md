# 4880 — Harita işinde tipli katman profili, köşe köşe kayma ve silinenin şekli

Tarih: 2026-11-10. plan/map-cad.md MAP-01, onuncu tur. Durum: taslak. MAP-01'in sıradaki işi: tipli katman profili;
köşe köşe kayma (en büyük kayma) ve silinenin şekli (ADR 4480'in “Sonra”sı).

İlgili:
- [ADR 1200](1200-map-work-structure-and-provenance.md): iş yapısı; “anlam katman profilindedir”;
- [ADR 1201](1201-map-work-relative-files-relink-and-contextual-tools.md) “Sonra”: katman türü (ADR 0622 §5) var, öğe
  türüne göre profil yok;
- [ADR 0541](0541-data-profiles.md): veri profili (sınıfın katmanı, geometrisi, alanları), `cad.profiles.apply/check`;
- [ADR 3680](3680-map-work-approved-values.md), [4080](4080-map-work-restore-as-approved.md),
  [4480](4480-map-work-approved-shape-in-the-drawing.md): onayın değerleri, Onaylı hâline getir, onaylanan şekil.

## Dayanak

- **Mevzuat** (`.run/research/map-cad/mevzuat/MAP-01.10.md`, 10 Kasım 2026'da okundu):
  - **BÖHHBÜY** katman adı tanımlamaz; kodlama detay sınıfı, 4 haneli detay kodu ve öznitelik tablosudur (Ek-1), sembol
    kodları Ek-3'te. m. 45(1)(b), 62(4): detaylar Ek-1'e göre kodlanır; m. 90: ölçü kayıtlarının kataloğa uygunluğu
    denetlenir. Tipli katmanın resmî karşılığı katman ↔ (detay sınıfı, kodu, öznitelikleri) eşlemesidir; katman adları
    idarenin ya da kullanıcının şablonudur.
  - **Kayma için resmî değerler** başka işlerin: m. 46(1) detay konumu ±7 cm; Tapu Planlarında Yanılma Sınırının
    Belirlenmesi Hakkında Yönetmelik m. 9(1) üretim yöntemine ve ölçeğe göre nokta konum doğruluğu. Onaydan sonraki
    kaymayı bunlarla karşılaştırmak kullanıcının ayarıdır; “teknik hata değildir” denmez.
  - Silinen nesnenin şeklinin saklanması ve gösterilmesi için kural yok: KentOS'un kararı.
- **Benzer uygulamalar** (`.run/research/map-cad/uygulama/MAP-01.10.md`; ekran resimleri `MAP-01.10-resim/`, 10 Kasım
  2026'da tarayıcıda bakıldı):
  - **Netcad, Veritabanı Yönetimi › Tablo Düzenle:** tipli katman bir uzamsal tablodur: Kolon Adı | Veri Türü |
    Gerekli; Tablo Adı, Sınıf Adı, Geometri Türü, Projeksiyon. Tablolar, İlişkiler ve Kurallar bağlantının altında; öğe ya
    da iş türüne göre hazır profil yok. Nesne düzeyinde değişiklik geçmişi belgelenmemiş (proje sürümü yedektir).
  - **QGIS, yeni katman penceresi:** geometri türü, Z/M, koordinat sistemi ve alan listesi.
  - **AutoCAD Map 3D, nesne sınıflandırması:** sınıfa girebilen nesne türleri ve özellikler (tür, aralık, öntanımlı);
    sınıflanmamış nesne sıradan CAD nesnesi kalır.
  - **ArcGIS Pro, farklar görünümü:** Insert / Delete / Update grupları, özellik tablosunda Current | Target | Common
    Ancestor, şekiller iki haritada yan yana; sayısal kayma yok. **Detect Feature Changes:** temel ve güncel çizgiler,
    değişim toleransı tamponu; silinen (D) çizgi de çıktıda.
  - **QGIS ifadeleri:** `hausdorff_distance` (yoğunlaştırmayla yaklaşık), `shortest_line`.

## Karar

1. **Tipli katman profili öğenin veri profilidir** (`WorkItem.profile`, `.kcad` şema 247): bir veri profilinin adı
   (KentOS'un, çizimin kopyası ya da projenin klasöründeki) ve öğenin tuttuğu sınıfları (boş: hepsi). Sınıfın katmanı
   türünü ve alanlarını taşır (ADR 0622, 0199); denetimi `cad.profiles.check`'inkidir. Öğenin öbür katmanları serbest
   CAD katmanı kalır. Öğe türüne göre öntanımlı profil yok (kaynaklarda yok; Netcad'de de proje kendi tablolarını kurar).
2. **`map.work.edit`'in `profile` işlemi** (tek geri alma adımı “Harita işi öğesine profil ver”): profil sınıflara
   daraltılır (bağladıkları değer listeleri, iki ucu kalan ilişkiler; adres verisi yalnız bütün profilde), çizimde
   olmayan sınıf katmanları profilin yerine eklenir, katmanın eksik alanları eklenir (`cad.profiles.apply`'ın yazıcısı:
   başka türde alan korunur, kilitli katmana alan eklenmez, uyarılarıyla), sınıfların katmanları öğeye girer. Başka
   öğenin katmanı `layer_in_work_item`; profilde olmayan sınıf `unknown_profile_class`. `name` verilmezse profil kalkar,
   katmanlar öğede kalır. `drawing` (açık projenin dosyası) projenin profil klasörü içindir.
3. **Onay:** profili olan öğe, sınıflarının bulgusu varken onaylanmaz (`approval_profile`; ileti bulgu sayısını ve
   ilkini söyler). Bulgusuz olmak onay değildir; onay ofisin kaydıdır.
4. **`map.work.query`'nin satırında `profile`:** bulundu mu, başlığı, sürümü, sınıflar (katmanı, nesne ve bulgu sayısı),
   sınıfların bulguları (değer listesi, ilişki ya da adres verisinin projede olmaması öğenin değil projenin bulgusudur) ve
   sınıfı olmayan, nesnesi olan **serbest katmanlar**.
5. **Köşe köşe kayma** (`changes.objects[].shift`, geometrisi değişen ve onay değerleri tutulan nesnede; ortak çekirdek
   `kentos_geometry_core::ops::shape_shift`):
   - aynı türde ve aynı köşe sayısında köşeler sırayla eşlenir: her köşenin kayması, **en büyük kayma** ve köşe numarası
     (eşitlerin ilki), kaymaların KOH'u;
   - her durumda iki şeklin **Hausdorff uzaklığı** (birinin bir noktasının öbürüne en büyük uzaklığı, iki yönün büyüğü)
     ve yeri (`from` onaylanan şekilde, `to` şimdikinde). Kesin hesap: bir kenar boyunca öbür şekle uzaklık yürünen
     boydan çok değişmez (1-Lipschitz); uçları `f0`, `f1`, boyu `L` olan parçada en uzak nokta `(f0 + f1 + L) / 2`'yi
     geçemez; daha uzağı tutabilecek parçalar ikiye bölünür, fark 10⁻⁹ m altına inene dek. Yaylar çemberi üstünde
     yürünür, kirişe çevrilmez. Esri'nin tolerans tamponu bunun eşik hâlidir (H ≤ t).
   - Eşik verilmez; resmî değerlerle karşılaştırma kullanıcınındır.
6. **Silinenin şekli:** Değişenler silinen nesnelerin onaylanan şeklini de kesik çizgiyle çizer (sorgunun
   `removedObjects`'i); durum çubuğu “… silinen k nesnenin onaylanan şekli …” der.
7. **Masaüstü** (Harita işi): başta **Tipli katman** (öğe) ve **Sınıf** (“Kontrol noktası · Harita varlıkları …”) ve +
   düğmesi; aynı profilin ikinci sınıfı öncekilerin yanına eklenir, başka profilin sınıfı onun yerine geçer (söylenir).
   Profili olan öğenin altında **Profil** satırı (sınıf sayısı; “Uyuyor” ya da “k bulgu”; ipucunda sınıflar, ilk bulgular,
   serbest katmanlar; düğmesi profili kaldırır). Değişenler'e basınca ağacın altında **Onaydan sonra kaymalar** tablosu
   (kitin veri tablosu): Nesne | Köşe | En büyük (m) | No | KOH (m) | Hausdorff (m); durum çubuğu en büyük Hausdorff'u
   söyler.
8. Kesik çizgiyle çizilen şekiller ayrı yollarla çizilir (tek yolda kesik çizgi parçaların uçlarını birleştiriyordu: iki
   şekil olunca görüldü).
9. **Kapsam:** yalnız Harita (CAD), ADR 2880–4480 gibi.

## Sınırlar

- Öğe türüne göre öntanımlı profil, profil sınıfının BÖHHBÜY detay koduna eşlenmesi yok (kurum profili, MAP-17).
- Kayma için eşik ve “sınır içinde” yargısı yok.
- Bağlı bir parçası değişen çok parçalı nesnede köşeler parça sırasıyla eşlenir; parça sırası değişirse eşlenmez sayılır
  (Hausdorff yine verilir).
- Python başvurusu düz kenarlıdır; yaylı şeklin Hausdorff'u çekirdek testinde (eş merkezli yaylar).

## Kanıt

- **Çekirdek** (`shape_shift` testleri): taşınan köşe en büyük kayma ve Hausdorff; eklenen köşe eşlenmez, Hausdorff
  eklenen köşede; iki köşe arasındaki en uzak nokta (açıortay noktası, kapalı formül); eş merkezli yaylar yarıçap farkı;
  noktalar.
- **Bağımsız başvuru** (`scripts/fixtures/map_work_cases.py`; KentOS kodu yok; tipli katmanı `cad.profiles`'ın kendi
  bağımsız başvurusu `profile_cases.py` yazar ve denetler): Hausdorff her kenarda uçlar ve iki uzaklık işlevinin eşit
  olduğu yerler (iki nokta, nokta–doğru, iki doğru) denenerek kesin. query 38'den 41'e (köşe eklenen çizgi, tipli katman
  satırı, bulunmayan profil; taşınan çizginin iki durumu kaymasıyla), edit 57'den 65'e (profil ver ve geri al, bilinmeyen
  profil, sınıf, iki kez sınıf, kaldır, kaldıracak profil yok, bulguda onay reddi, başka öğenin katmanı). Bozma denemesi:
  bir Hausdorff 10⁻⁶ ve bir serbest katman oynatılınca koşucu tam o iki durumu yakalar.
- **`.kcad` şema 247:** `map-work-profile.kcad` ve 246'da reddedilen, boş sınıf listesi, iki kez sınıf, boşluklu ad
  örnekleri; bağımsız okuyucu `tools/kcad/kcad.py` aynı sonucu verir. Bilinmeyen sürüm örneği `schema-version-248.kcad`.
- **Python** (`python/tests/test_map_work_profile_shift.py`, bağımsız okuyucuyla), **MCP**
  (`crates/native/mcp/tests/map_work_profile_shift.rs`), **masaüstü** (`map_work_profile_tests`: sınıf ver, ikinci
  sınıf, kaldır; Değişenler iki şekil ve kayma tablosu; çapraz denetimler). Resimler
  `.run/gui-evidence/tipli-katman-4880/`.

## Sonra

- Öğe türüne göre önerilen profil (kurum profiliyle, MAP-17).
- Kayma eşiği ve yanılma sınırıyla karşılaştırma (kullanıcının seçtiği resmî değerle).
- Web (TODOS §0.2).
