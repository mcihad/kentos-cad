# ADR 9184: CBS projesi tek sayfada kurulur: ad, amaç, konum ve saklama

- **Güncelleme (2026-11-24, [ADR 9202](9202-cbs-new-project-simple.md)):** sahibin sözüyle (“amaç bölümü gereksiz”,
  “wizard kaldırma ama sadeleştir”) amaç ve saklama kartları ve Kurum şablonu kalktı. Sayfa ad, il ve koordinat
  sistemidir; proje boş açılır (yalnız Açıklama), yeri ilk Kaydet'te sorulur.

- **Durum:** kabul edildi (2026-11-23); adım VY-83. Plan: [plan/veri.md](../../plan/veri.md) VY-83.
- **İstek:** sahibin sözü (23 Kasım 2026): “büyük arayüz işi (3, 12, 13, 15) ile devam”. Değerlendirmenin 3. maddesinin
  kalanı (ADR 9138): “sihirbazın tek karar sayfası (ad, amaç, CRS, saklama ve sonuçları), boş kent haritası şablonu.”
  Maddenin kabulü:
  - ilk sayfada ad, amaç (boş kent haritası, kadastro, kurum şablonu), projenin koordinat sistemi ve saklama tercihi;
  - dosyanın, GeoPackage içinde projenin, PostgreSQL bağlantısının ve veriyi kopyalamanın sonuçları birer cümleyle;
  - kurum görevlisi yeni bir kent bilgi sistemini “Kadastro paftası” kabul etmeye zorlanmadan kurabilmeli.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi; şablonun aktardıkları mevcut alanlardır.
  **Komut:** yok (Dosya › Yeni proje aynı komuttur). **Fixture:** `fixtures/project/v1/wizard.json`'a iki CBS durumu
  eklendi.

## §1 Bağlam ve araştırma

- Yeni proje sihirbazı CBS projesinde üç adımdı: tür, konum (il ve sistem), ad ve özet.
  - Özet her projeye “Kadastro paftası: Taslak, Kadastro, Ulaşım, …” diyordu; başka bir başlangıç seçilemiyordu.
  - Saklama yeri sorulmuyordu. İlk Kaydet her zaman bir `.kcad` dosyası istiyordu; projeyi GeoPackage'da ya da
    PostgreSQL'de saklamak için Dosya › Veri kaynağı penceresini ayrıca bilmek gerekiyordu (ADR 9107).
- **Araştırma.**
  - QGIS 3.44 kılavuzu, “Working with Project Files”
    ([project_files.html](https://docs.qgis.org/3.44/en/docs/user_manual/introduction/project_files.html), okundu
    23 Kasım 2026):
    - yeni proje Project › New ile ya da New from template ile başlar;
    - başlangıç ekranı şablonları ve son projeleri adları ve dosya yollarıyla listeler;
    - proje dosyaya ya da Project › Save to ile PostgreSQL, GeoPackage veya Oracle'a kaydedilir;
    - kaydetme penceresi GeoPackage'da bağlantıyı ve projeyi, PostgreSQL'de bağlantıyı, şemayı ve projeyi sorar;
    - proje dosyası katmanları, stillerini, tablo ilişkilerini ve baskı düzenlerini taşır.
  - ArcGIS Pro 3.8 yardımı, “Create a project”
    ([create-a-new-project](https://doc.esri.com/en/arcgis-pro/latest/help/projects/create-a-new-project.html),
    okundu 23 Kasım 2026):
    - projelerin çoğu başlangıç içeriğini belirleyen bir şablondan kurulur: Map, Catalog, Global Scene, Local Scene ya
      da kurumun `.aptx` şablonu;
    - şablonsuz da başlanabilir;
    - Yeni Proje penceresi adı ve yeri sorar;
    - proje dosyası veritabanı ve klasör bağlantılarını tutar, verinin kendisini değil.
  - **Sonuç.** İki program da başlangıcı bir şablon seçimi yapar ve saklamayı ayrı bir karar olarak sunar; boş
    başlangıç her zaman vardır. KentOS bunları tek sayfada, her seçimin sonucunu bir cümleyle söyleyerek toplar.

## §2 Karar

1. **Ortak model** (`kentos_project::wizard`):
   - `Purpose`, projenin amacı; başladığı katmanları belirler. `PURPOSES` kartları adı ve bir cümlelik anlamı taşır.
     - Boş kent haritası: yalnız Taslak katmanı (`new_project::city_layers`). Harita araçlarının katmanları ilk
       yazdıklarında kurulur (mevcut `standard_layer`).
     - Kadastro paftası: önceki standart ağaç; öntanımlı.
     - Kurum şablonu: §2.3.
   - `Storage`, projenin saklandığı yer. `STORAGES` sonuçları birer cümleyle söyler:
     - Dosya (.kcad), öntanımlı: proje tek dosyadır; bağlı tabloların satırları kaynaklarında kalır, dosya taşınırken
       onlar da taşınmalı.
     - GeoPackage içinde: proje verisiyle aynı `.gpkg` dosyasında durur; tek dosya taşınır ve paylaşılır, tabloları
       başka CBS programları da okur.
     - PostgreSQL veritabanında: proje bir şemada durur; yetkili herkes açar, kimin ne yazdığını veritabanının rolleri
       denetler.
   - `DATA_NOTE`: bağlanan tabloların satırları kaynağında kalır, herkes aynı satırları görür; içe aktarılan veri
     projeye kopyalanır ve kaynaktaki sonraki değişiklikleri görmez.
   - **Adımlar** (`Draft::steps`). CBS projesinin adımları ikidir: Proje türü ve Proje. Ad, amaç, il ve koordinat
     sistemi, saklama aynı sayfadadır. CAD projesinin üç adımı değişmedi.
   - **Özet ve ray.** Özet amacı, amaca göre katmanları ve saklamayı da yazar. Ray notu “ad · sistem”dir.
   - **Durdurma.** Oluştur sayfada durur: ad boşsa, sistem bu sürümde tanımlı değilse, kurum şablonu seçilmemişse.
2. **Masaüstü sayfası** (`project/wizard/pages.rs`, `decision`):
   - sırasıyla Proje adı, Amaç (üç kart), İl ve Koordinat sistemi (listeler bu sayfada alçaktır), TM dilimleri şeridi,
     Saklama (üç kart) ve veri notu, ekrandaki çizimin notu;
   - KentOS GIS'in sihirbazı doğrudan bu sayfada açılır; KentOS CAD'de tür sayfasından sonra gelir.
3. **Kurum şablonu** (`project/wizard/template.rs`):
   - Şablon seç… bir `.kcad` CBS projesi ister. Dosya arayüzü bekletmeden okunur; sayfa şablonun ne getirdiğini söyler:
     “Belediye şablonu.kcad: 3 katman, 1 form, 1 ilişki, 1 topoloji kuralı; nesneleri alınmaz.” Sonraki seçim öncekinin
     okumasını geçersiz kılar. Bir CAD projesi nedeniyle reddedilir.
   - Yeni proje şablondan şunları alır:
     - katmanları (veri katmanları kaynaklarına bağlı kalır), stillerini, sembollerini;
     - formları, değer listelerini, ilişkileri ve birleşik görünümleri;
     - yazı, ölçü, açıklama ve tablo stillerini, çizgi türlerini ve taramaları;
     - servis bağlantılarını, ağları, olay bağlarını, adres bulucuları ve değişkenleri;
     - topoloji kurallarını ve profilini, katman durumlarını, görünüm şablonlarını, standart belgelerini ve onaylı
       pafta şablonlarını; ölçeği de şablonundur.
   - Almadıkları: nesneler, paftalar, soy kaydı, inceleme notları, ölçme dengelemeleri, topoloji istisnaları ve dosyaya
     bağlı tabloların eşleşme durumu. Bunlar şablonun kendi işine aittir. Ad, sistem, il ve açılış görünümü
     sihirbazındır.
4. **Saklama.**
   - GeoPackage ya da PostgreSQL seçilince proje hiçbir yere yazılmaz (`Source::Wanted`).
   - İlk Kaydet, Veri kaynağı penceresini o tür yerle ve projenin adıyla açar. Vazgeç bir şey yazmaz, seçim kalır.
     Farklı kaydet dosya ister; proje o dosyanın olur.
   - Dosya menüsü “Kaydedilmedi · GeoPackage içinde saklanacak; Kaydet yeri sorar” der. Açılış iletisi ilk Kaydet'in
     ne soracağını söyler.
5. **Ad.** Proje adı sayfanın ilk alanıdır. Önerilen ad kaydedilince dosyanın adını alır (ADR 9138). Aynı adlı iki
   sekme klasörüyle ayrılır (mevcut).
6. **Betik.** Python ve MCP'nin yeni projesi (`drawing.new`) değişmedi: CBS projesi kadastro paftası ağacıyla başlar
   (§4).

## §3 Kanıt

- **Ortak** (`kentos-project`): `a_cbs_project_decides_its_purpose_and_storage_on_one_page`. Fixture'a iki yeni durum
  eklendi: “Ankara, boş kent haritası, GeoPackage içinde” ve “Sivas, kurum şablonu, PostgreSQL”. Bütün durumların ray
  notları gösterilen adımlara göredir.
- **Masaüstü** (`project/wizard/tests.rs`):
  - `a_cbs_project_is_decided_on_one_page_and_its_first_save_goes_where_it_was_chosen`:
    - tek sayfada ad, boş kent haritası, Ankara ve GeoPackage seçiliyor; proje yalnız Taslak katmanıyla açılıyor
      (TM33);
    - Kaydet Veri kaynağı'nı GeoPackage'da ve adıyla açıyor, Vazgeç bir şey yazmıyor;
    - PostgreSQL seçilince Kaydet veritabanında arıyor; Farklı kaydet dosyaya yazıyor.
  - `a_template_gives_its_layers_and_settings_and_none_of_its_objects`:
    - şablonsuz Oluştur nedenini söylüyor;
    - şablonun üç katmanı, formu, ilişkisi, kuralı ve 1:500 ölçeği geliyor; nesneleri ve istisnası gelmiyor;
    - sistem ve açılış İzmir'in (TM27);
    - CAD projesi şablon olarak reddediliyor.
  - Mevcut sihirbaz ve modül sınamaları iki adımlı CBS akışına göre güncellendi.
- **Resimler:** `.run/gui-evidence/kullanim-9184` (Git'e girmez).

## §4 Kalan

- Betikte (Python, MCP) amaç ve şablon.
- Şablonun paftaları: kapsamları şablonun yerine bağlıdır, yeni projede kapsamın sorulmasıyla birlikte alınmalı.
- Gerçek pencerede deneme; web (TODOS.md §0.2).
