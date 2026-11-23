# ADR 9195: Ekranı açık projenin modülü belirler; modüller ve istemciler gevşek bağlı

- **Durum:** kabul edildi (2026-11-23); plan [plan/ayrim.md](../../plan/ayrim.md) AY-01 (karar), AY-02…AY-04 ve AY-07
  (uygulama). ADR 0801 ve 0802'nin “her modül ayrı program” kararını korur. Yüzeyleri programa bağlayan kuralını
  değiştirir: yüzeyleri artık açık projenin modülü belirler.
- **İstek:** sahibin sözleri (23 Kasım 2026), sırasıyla:
  - “bizde de böyle bir ayrım var ama hem paneller hem alt paneller hem menülerde çok ciddi karmaşa var mesela saat
    katmanlar diğer panellerin olduğu yerde bütün paneller hepsinde gözüküyor. Alt paneller hepsinde gözüküyor mesela
    CAD denetimi gis içinde gözükmesin, katman sağ tuş menüdeki mesela öğelerin hepsi hepsi ile ilgili değil, ayrıca
    bir proje tipi seçip proje oluşturduk diyelim daha sonra yeni projeye bastığımızda bu sefer bütün menüler
    geliyor”;
  - sorunun cevabı (ekranı açık projenin türü belirlesin): “1 numarayı uygulayalım ama ileride ayrı ayrı
    yazılımlara dönüştürülecek gibi gevşek bağlı olsun, bunu claude.md içerisine de kati kural olarak ekle”;
  - “İleride mobil görüntüleyiciler düzenleyiciler de geliştireceğiz, başka modüller de, ayrımı netleştirdikten sonra
    bir daha sistemin bozulmaması için kati kurallar olarak claude.md içerisine ekle bu kuralı da”.
- **Kural:** CLAUDE.md §25 (sekiz kesin kural) ve kılavuz §9.9 (yeni modül ve yeni istemci tarifi).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmez. **Komut:** yok.

## §1 Bağlam

ADR 0802 her modülü ayrı uygulama yaptı. Tek program dosyası adıyla açılınca modül olur: `kentos-mimari`,
`kentos-harita`, `kentos-gis`, `kentos-yol`. `kentos-cad` hepsini birden açar.

**Ayrım programa bağlı kaldı.** Bu ADR'nin incelemesinde (23 Kasım) bulunanlar:

- **Şerit** açık projenin türüne bakar (`App::ribbon_tabs`: `work_mode`, `ribbon_work`).
- **Şeridin dışındaki yüzeyler** yalnız programa bakar (`module_surfaces.rs`, `App::surfaces()` → `self.program`):
  - sağ panelin 12 paneli, alt panelin 16 sekmesi, durum çubuğunun hücreleri;
  - Proje ayarları ve Uygulama ayarları'nın bölümleri;
  - katman menüsünün modüle özgü öğeleri, Dosya menüsünün biçimleri, Kaynaklar'ın bölümleri.
- `kentos-cad`'de (`program = None`) bunların hepsi her projede açıktır: bir CBS projesinde CAD denetimi, bina
  Denetimi, Bloklar ve Gezgin görünür. Bu ADR'nin kabulünde çekilen resimler de bunu gösteriyordu (ADR 9194'ün CBS
  projesinde Denetim sekmesi).
- **Projeye bakan iki yama:** CBS projesinde bina komutları (ADR 9127, `building_only_here`) ve CBS'de Kalınlık
  lambası (ADR 9185, `shows_aid`).
- **Katman menüsü** (`cad/layering/menus.rs`) yaklaşık 80 öğe taşır. Yalnız 11'i modüle göre kesilir (`LayerEntry`),
  hiçbiri katmanın türüne göre kesilmez.
- **Kod:** çalışan modül 114 dosyada yaklaşık 250 yerde okunur. Masaüstü tek paket ve 676 bin satır. Kabuktan
  modüllere bağlı dosya sayısı: cad 227, gis 110, arch 57, road 45, map 35. Modüller arasında gis→cad 64, cad→gis 40.

**Araştırma** (23 Kasım 2026'da okundu):

- **Esri, *ArcGIS Pro SDK, ProConcepts Configurations*:** bir yapılandırma programın adını, simgesini, açılış ekranını
  ve başlangıç sayfasını değiştirir. `ArcGISPro.exe /config:<ad>` ile başlar ve bir oturumda bir yapılandırma
  yüklenir. Aynı program, farklı kapılardan farklı ürün olarak açılır.
  <https://pro.arcgis.com/en/pro-app/3.6/sdk/api-reference/conceptdocs/docs/ProConcepts-Configurations.html>,
  <https://doc.esri.com/en/arcgis-pro/latest/sdk/api-reference/conceptdocs/docs/ProGuide-Command-line-switches-for-ArcGISPro.exe.html>
- **Autodesk, *AutoCAD included toolsets*:** sektör araç takımları AutoCAD'in üstünde ayrı programlar olarak açılır
  (ADR 0801'de de anıldı). <https://www-int.autodesk.com/in/products/autocad/included-toolsets>
- **QGIS, *QGIS Configuration*:** tek program `--profile` ile başlar; her profilin ayarları, eklentileri ve geçmişi
  ayrıdır. <https://docs.qgis.org/latest/en/docs/user_manual/introduction/qgis_configuration.html>
- **Bricsys, *License levels*:** BricsCAD'in bütün düzeyleri tek kurulumdadır; lisans düzeyi özellikleri açar.
  <https://help.bricsys.com/en-us/document/bricscad/installation-and-licensing/overview-of-licensing-options/license-levels>
- **Vectorworks, *Installing Vectorworks products* (2025):** kurulum bütün ürün ailesini kurar; hangi ürünlerin
  açılacağını seri numarası belirler.
  <https://app-help.vectorworks.net/2025/eng/VW2025_Guide/Start/Installing_Vectorworks_products.htm>
- **Ortak platformda ayrı ürünler:**
  - Bentley, *OpenRoads Designer*: ürün MicroStation'ın bütün işlevini ve yol araçlarını taşır.
    <https://www.bentley.com/software/OpenRoads-designer/>
  - Netcad, *Netcad GIS*: modüller Netcad GIS platformunun üstünde çalışır.
    <https://en.netcad.com/en/products/netcad-gis>

Bu programların hiçbiri kardeş disiplinler için ayrı kod tabanı tutmuyor. Ortak örüntü tek platformdur; ürünü
giriş kapısı ya da lisans belirler.

**KentOS kararı:** tek kod tabanı. Modül programları giriş kapısıdır. Hepsi bir arada programın ekranı açık projeyi
izler. Modüller, ileride ayrı yazılımlara bölünebilecek kadar gevşek bağlıdır.

## §2 Karar

1. **Etkin modül.** Ekranın her yüzeyini etkin modül belirler.
   - Açık proje varsa projenin modülüdür (`module_of(settings)`: proje türü ve CAD işi).
   - Proje yoksa programın modülüdür.
   - Hepsi bir arada programda proje yokken yalnız ortak yüzeyler görünür: Komut geçmişi, Uyarılar, Çalışan işler,
     Python, Asistan.
   - Şerit, komut kümesi (`offers`), paneller, alt sekmeler, durum hücreleri, ayar bölümleri, katman menüsü,
     biçimler, Kaynaklar ve sihirbaz aynı karardan gelir.
   - ADR 9127 ve 9185'in yamaları bu genel kurala döner.
2. **Profil.** Bir modülün ekranda neyi gösterdiğini modülün profili söyler. Profil tabloları joker kolsuzdur: yeni bir
   panel, sekme, menü öğesi ya da ayar her modül için açıkça var ya da yok der, yerleşmeden derlenmez.
   - Katman menüsünün her öğesi hem modülle hem katmanın türüyle yerleşir. Türler: çizim, açıklama, veri tablosu,
     bağlı, servis, raster, nokta bulutu, grup.
   - Matris sınaması her hücreyi sınar. Hepsi bir arada programda bir türün ekranı, o modülün programındakiyle aynı
     olmalıdır.
3. **Yerleşim modül başına.** Her modülün yerleşimi kendi dosyasındadır. Hepsi bir arada programda farklı modüllerin
   projeleri sekmelerde yan yana durabilir; sekme değişince kabuk o modülün yerleşimine ve profiline geçer.
4. **Gevşek bağlılık.**
   - Kabuk modülün içini bilmez. Modül profilini bir kayıtla bildirir (`ModuleProfile`).
   - Modül başka modüle doğrudan bağlanmaz; ortak çekirdeğe ve sözleşmeye bağlanır.
   - Bağımlılık denetimi (`scripts/dev/module_deps.py --check`) bugünkü bağları taban alır. Taban yalnız azalır ve
     yeni bağı reddeder; modül klasörlerinin ve kaydın dışında modül adına göre dalı da reddeder.
5. **Ortak modül tanımı.** Modülün kimliği, proje türleri, işleri, saklama türü (ADR 9196), komut kümesi ve katman
   türleri `kentos-launch`'tadır ve `catalog/v1/modules.json`'a üretilir (`--check`'li üretici). Masaüstü, bulut
   portalı, sunucu ve ileride mobil görüntüleyici ile düzenleyici aynı tanımı okur; modül mantığı istemci başına
   çatallanmaz.
6. **İstemciler.**
   - Görüntüleyici yalnız okur.
   - Düzenleyici her değişikliği komutla yazar: CBS verisini düzenleme oturumuyla ya da sunucu komutuyla, CAD
     projesini dosya revizyonuyla.
   - Rol ve form kuralları her istemcide aynı çözümleyiciden gelir.

## §3 Uygulama sırası

Ayrıntı [plan/ayrim.md](../../plan/ayrim.md)'de:

- **AY-02:** etkin modül; yerleşim modül başına; Yeni proje'nin bütün menüleri getirmesi.
- **AY-03:** yüzey matrisi; katman menüsü modül ve katman türüyle. Matris bu ADR'nin §5'idir.
- **AY-04:** profilin modüllere taşınması, `ModuleProfile` kaydı, bağımlılık denetimi, joker kolsuz tablolar, ortak
  modül kataloğu. Bu ADR'nin §6'sı.
- **AY-07:** paketlere bölme.

## §4 AY-02: ekran açık projeyi izliyor (23 Kasım 2026)

- **Etkin modül** (`App::active_module`, `apps/desktop/src/arch/program.rs`): modül programı kendi modülüdür, çünkü
  yalnız kendi projesini açar, öbürlerini devreder. `kentos-cad` açık projenin modülünü izler (türü ve işi).
  - Modülü belirlenmemiş proje `kentos-cad`'de her yüzeyi gösterir, karar verilene kadar: türsüz, işi sorulmamış CAD,
    eski karma tür, duyurulmuş türler.
  - Sihirbaz, Python ve MCP her yeni projeye tür ve iş verir.
- **Yüzeyler ve komutlar** (`apps/desktop/src/shell/module_surfaces.rs`):
  - Yüzeyler, paneller, araç seçenekleri ve komut kümesi (`offers`, `not_offered`) etkin modülden gelir.
  - `kentos-cad` projesizken yalnız ortak yüzeyleri gösterir (`NO_PROJECT`): Komut geçmişi, Çalışan işler, Uyarılar,
    Python; Katmanlar, Asistan, Eklenti, Öznitelikler.
  - Hiçbir modüle verilmeyen şerit komutları `kentos-cad`'in kendisinindir ve orada kalır (projenin türünü değiştiren
    `workspace.*`).
- **Yamalar genel kurala döndü:**
  - ADR 9127'nin yaması (`building_only_here`) kalktı: CBS projesinde bina komutları modülün kümesinden düşer.
  - ADR 9185'in yaması (`shows_aid`) kalktı: Kalınlık lambası profilin hücresidir (`Cell::LineWeights`).
  - Ret iletisi yeri söyler: ““Duvar” bu CBS projesinde yok; KentOS Mimari'de var.”
- **Paneller:**
  - Modül değişince göstermediği paneller kapanır (`surface_closed`). Onları gösteren bir modülle, Katmanlar'ın
    yuvasındaki sıralarıyla geri gelirler (`Panel::TOP_SLOT`).
  - Kullanıcının kapattığı panel kapalı kalır.
  - Ekran Komut geçmişi'ne ya da Katmanlar'a düştüyse ve yeni modül gösteriyorsa, saklı alt sekme ve öndeki panel
    geri gelir.
  - Bütün bu kural açılışta bir kez uygulanır.
- **GeoPackage:** açmak programın yeteneğidir (`shows_panel_in(program, Sources)`). `kentos-cad` bırakılan
  GeoPackage'ı kendi sekmesinde CBS projesi olarak açar.
- **Yeni proje:** `kentos-cad` projesizken ortak yüzeyleri gösterir; kurulan proje yalnız modülününkileri. Yeni
  proje yeniden açıkken de ekran o projenin modülündedir. Sahibin gördüğü “bütün menüler” buydu: hepsi bir arada
  program her projede her yüzeyi gösteriyordu.
- **Kanıt:**
  - Yeni sınamalar:
    - `module_surface_tests::kentos_cad_shows_the_open_projects_module_as_its_program_does`: dört modülde `kentos-cad`'in
      bütün yüzeyleri ve katalogdaki her komutun sunulması, modül programındakiyle aynı;
    - `kentos_cad_switches_its_surfaces_with_the_project_on_screen`;
    - `wizard::tests::a_new_project_shows_its_modules_surfaces_alone_and_so_does_the_next`.
  - Yeni kurala çevrilen sınamalar:
    - `kentos-cad`'in projesiz satırları (alt sekmeler, paneller, durum hücreleri, ayar bölümleri);
    - ADR 9127'nin sınaması;
    - `module_tests`'in “`kentos-cad` her komutu çalıştırır” iki satırı;
    - CBS türlü örneklerle Harita araçlarını koşan ölçü ağı ve türetme sınamalarının sahnesi Harita projesi oldu;
    - şablon izlerinde CBS projesinin çoklu çizgisi Genişlik sormuyor.
  - **İzlerin geçici dosyası** artık her oynatmada ayrı (`traces::scratch_file`). Aynı izi aynı anda oynatan iki sınama
    birbirinin dosyasını açıyordu.
  - **Masaüstü:** 3047 sınama geçti. Düşen 4 sınama HEAD'de de düşüyordu, ayrı işlenir:
    - `project::tests`'in iki sihirbaz sınaması;
    - `python::tests::every_program_opens_a_link_of_its_own`;
    - `source_size_tests` (`lib.rs` 2010 satır).
  - **Resim:** `.run/gui-evidence/kullanim-9195`: projesiz, CBS, Mimari, Harita ve Yol; açık ve koyu.

## §5 AY-03: yüzey matrisi ve katman menüsü (23 Kasım 2026)

**Araştırma** (23 Kasım 2026'da okundu):

- **QGIS, *General Tools › Layers Panel › Overview of the context menu of the Layers panel*** (3.44): katman
  panelinin bağlam menüsü, her öğesinin grup, vektör, raster, örgü, nokta bulutu ve 3B katmanda çıkıp çıkmadığını
  söyleyen bir tablodur. Örnekler:
  - nesne sayısı ve öznitelik tablosu yalnız vektörde;
  - düzenlemeyi aç/kapat vektörde ve örgüde;
  - katmanın kopyası grupta yok;
  - grubun koordinat sistemi yalnız grupta.
  <https://docs.qgis.org/latest/en/docs/user_manual/introduction/general_tools.html>
- **Esri, *ArcGIS Pro: Contents pane*:** katmanın bağlam menüsündeki komutlar katmanın türüne göre değişir.
  <https://pro.arcgis.com/en/pro-app/latest/help/mapping/map-authoring/contents-pane.htm>

**KentOS kararı:** katman menüsünün her öğesi iki boyutla yerleşir.

- **Modül:** profilin listesi (`Surfaces::layer_menu`, `apps/desktop/src/shell/module_surfaces.rs`).
- **Katmanın türü:** joker kolsuz eşleşme (`LayerEntry::kinds`, `apps/desktop/src/shell/layer_entries.rs`); yeni öğe
  türlere yerleşmeden derlenmez.
- Öğe ikisi de evet derse çıkar (`App::layer_entry_on`). Modülü belirlenmemiş proje `kentos-cad`'de türün bütün
  öğelerini gösterir.
- Eklentinin eklediği öğeleri eklentinin bildirimi yerleştirir.

**Katmanın türü** (`App::layer_kind`), sırayla:

1. grup;
2. servis (`service`);
3. veri tablosu (`data`);
4. bağlı (`settings.sources`);
5. açıklama (türü Açıklama);
6. raster ya da nokta bulutu: yalnız onları tutan katman; etkin katmanın bağlamsal sekmesiyle aynı sorgu
   (`layer_holds_alone`), çizgi katmanında bir nesneye bakar;
7. öbürü çizim.

**Tablo.** Kısaltmalar:

- Modüller: M Mimari, H Harita, Y Yol, C CBS.
- Türler: ç çizim, a açıklama, v veri tablosu, b bağlı, s servis, r raster, n nokta bulutu, g grup.

| Öğe | Modül | Tür |
|---|---|---|
| Etkin katman yap | MHYC | ç a v b r n |
| Düzenleme oturumu (başlat/bitir, kaydet, geri al, incele) | C | v |
| Gizle, Yalnızca bunu göster, Tüm katmanları göster | MHYC | hepsi |
| Kilitle | MHYC | ç a v b r n g |
| Dondur, Bütün katmanları çöz | MHYC | ç a b r n g |
| Yazdır | MHYC | hepsi |
| Sıra ▸ | MHYC | hepsi |
| Servis öğeleri (ayarlar, yükle, önbellek, kapsam, saydamlık) | HYC | s |
| Yenile (servisten beslenen katman) | HYC | ç |
| Nesnelerini seç | MHYC | ç a b r n g |
| Katmana / Gruba yakınlaştır | MHYC | ç a v b r n g |
| Dışa aktar ▸ | HYC | ç v b |
| Veri kaynağına taşı… | C | ç |
| Nesne sayısını göster | C | ç v b |
| Görünüş ▸ Renk | MHYC | ç a v b |
| Görünüş ▸ Çizgi tipi, Kalınlık | MHYC | ç a b |
| Görünüş ▸ Katman stili…, Etiketler… | MHYC; Etiketler HYC | ç v b |
| Görünüş ▸ Stil yükle/kaydet, SLD, QML, MapLibre | HC | ç v b |
| Görünüş ▸ Değişikliğin etkisi… | MHYC | ç a v b |
| Veri ▸ Alanlar…, Formlar…, Bilgi kartı ve eylemler… | HC | ç v b |
| Veri ▸ Veri kümesi bilgisi… | HYC | ç v b |
| Veri ▸ Süzgeç…, Seçimden süzgeç, Süzgeci kaldır | HYC | ç v b |
| Veri ▸ Zaman ayarları… | C | ç v b |
| Veri ▸ İstatistik | HC | ç v b |
| Veri ▸ Grafik | C | ç v b |
| Katman türü ▸ | C | ç a |
| Koordinat sistemi ▸ | HYC | ç a b |
| Kaynak ▸ | HYC | b |
| Kenet ▸ | MHYC | ç a v b g |
| Senaryo ▸ | C | g |
| Yeniden adlandır, Sil | MHYC | hepsi |
| Yanına / İçine yeni katman | MHYC | ç a v b r n g |
| Kopyasını oluştur, Başka katmanlarla birleştir… | MHYC | ç a b r n |

**Türün gerekçeleri:**

- **Veri tablosu:** satırları kaynağındadır.
  - Dondur, Nesnelerini seç, Kopyasını oluştur ve birleştirme yoktur. Seçecek nesnesi yoktur; kopyası boş bir çizim
    katmanı olurdu (`cad.layers.edit`'in kopyası nesneleri taşır).
  - Çizgi tipi ve Kalınlık yoktur; görünüşü Katman stili'ndedir.
  - Katman türü ve Koordinat sistemi tablonundur.
  - Düzenleme oturumu yalnız CBS'dedir: Harita ve Yol'da CBS tablosu salt okunur altlıktır (ADR 9196 §2.3). Öbür
    yollar AY-06'da kapanır.
  - Kenet kalır: çizim araçları veri katmanının satırlarına da kenetlenir (ADR 9103 §5).
- **Açıklama:** yazı, ölçü ve pafta, proje dosyasında kalır (ADR 9196 §2).
  - Katman stili, Etiketler, Veri ▸, Dışa aktar ve Veri kaynağına taşı yoktur.
  - Renk, Çizgi tipi ve Kalınlık nesnelerin katmana göre görünüşü için kalır.
- **Bağlı:** Kaynak ▸ yalnız onda; Katman türü yoktur, tür tablonundur.
- **Servis:** nesne tutmaz. Kendi öğeleri, görünürlük, yazdırma, sıra, ad ve sil.
- **Raster ve nokta bulutu:** görünüşü ve yerleştirmesi kendi bağlamsal sekmesindedir (`raster.style`,
  `raster.georef`, `pointcloud.style`, `pointcloud.register`). Bu yüzden Görünüş ▸, Veri ▸ ve Koordinat sistemi
  yoktur. Kenet de yoktur: kenet bunlara bağlanmaz.
- **Grup:** grubun öğeleri; Senaryo ▸ yalnız senaryo grubunda ve CBS'de.

**Modülün sütunu komut kümesinden:**

- Şeritte karşılığı olan 18 öğe, modülün komut kümesi ne diyorsa odur (`catalog().module_has`). Sınama her modülde
  bunu denetler.
- İstatistik ve Grafik, alt panelin sekmeleriyle gelir.
- Bunun gösterdikleri:
  - CBS'nin şeridinin taşmalarında Dondur (`tool.layerFreeze`, `layer.thawAll`), Çizgi tipleri (`lineTypes.manage`)
    ve Değişikliğin etkisi (`layer.impact`) vardır. Menü de onları sunar; veri tablosunda türü yine keser.
  - Yol'un kümesinde Veri kümesi bilgisi vardır (`layer.dataset`, haritanın komutları).
- Mimari'de bulunmayanlar:
  - Dışa aktar (Mimari'nin biçimleri yalnız çizimdir);
  - Veri ▸ bütünüyle;
  - Etiketler, stil dosyaları, Koordinat sistemi, Kaynak ve servis öğeleri.
- Yalnız CBS'de olanlar: düzenleme oturumu, Veri kaynağına taşı, Nesne sayısını göster, Zaman, Grafik, Katman türü,
  Senaryo.

**Yüzey matrisi** (modülün profili, `module_surfaces.rs`; ✓ var, – yok):

| Panel | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Katmanlar, Asistan, Eklenti, Öznitelikler | ✓ | ✓ | ✓ | ✓ |
| İşlemler, Kaynaklar | – | ✓ | ✓ | ✓ |
| Bloklar | ✓ | ✓ | ✓ | – |
| Şablonlar | ✓ | ✓ | ✓ | ✓ |
| Gezgin | ✓ | – | – | – |
| Yol projesi | – | ✓ | ✓ | – |
| Harita işi, GNSS arşivi | – | ✓ | – | – |

| Alt sekme | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Komut geçmişi, Koordinat listesi, Arama, Çalışan işler, Uyarılar, Python | ✓ | ✓ | ✓ | ✓ |
| Noktalar, Tablo | – | ✓ | ✓ | ✓ |
| İstatistik, Adres, Topoloji, Servis bilgisi | – | ✓ | – | ✓ |
| Grafik | – | – | – | ✓ |
| Denetim (bina) | ✓ | – | – | – |
| CAD denetimi | ✓ | ✓ | ✓ | – |
| Eklenti denetimi (yalnız kuralı olan eklenti ya da koşu varken) | ✓ | ✓ | ✓ | ✓ |

| Durum hücresi | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Harita sistemi, İkinci sistem | – | ✓ | ✓ | ✓ |
| Topoloji ve Çakışma lambası | – | ✓ | – | ✓ |
| Senaryo | – | – | – | ✓ |
| Kat | ✓ | – | – | – |
| Kalınlık lambası | ✓ | ✓ | ✓ | – (Görünüm'de, ADR 9185) |

Ayarlar ve biçimler:

- **Proje ayarları:** Genel, Birimler ve Koordinat sistemi her modülde. Kotlar ve epok ile Ölçme Harita, Yol ve
  CBS'de. Değişkenler Mimari, Harita ve CBS'de. Mimari bölümü yalnız Mimari'de.
- **Uygulama ayarları:**
  - Coğrafi koordinatlar ve Yeni projeler: Harita, Yol, CBS.
  - Çizilen nesnenin formu: Harita ve CBS.
  - Yeni projelerin türü ve birimi: yalnız `kentos-cad` projesizken.
- **Kaynaklar:** Veritabanları Harita ve CBS'de, Servisler Harita, Yol ve CBS'de.
- **Dosya menüsünün biçimleri:** Mimari'de yalnız çizim biçimleri (DWG, DXF, NCZ, PDF), öbürlerinde hepsi.
- **Şerit:** AY-02'den beri modülün komut kümesinden gelir (`offers`).

**Bu maddede değişen yüzeyler:**

- **Yol:**
  - CAD denetimi: CAD programlarınındır.
  - Eklenti denetimi.
  - Harita sistemi hücresi: komutları (`view.mapCrs`, `layer.crs`, `layer.reproject`) Yol'un kümesindedir.
- **Eklenti denetimi:**
  - Yalnız eklenti etkin modüle bir denetim kuralı sunuyorsa ya da bir kuralın koşusu sekmedeyse görünür
    (`offers_ext_audits`, her karede liste kurmadan). Ajanın koşusu da buna dahildir.
  - Kural seçicisi etkin modülün kurallarını sunar. Önceden programınkileri sunuyordu: `kentos-cad`'de boştu.
- **Şablonlar CBS'de kalır.** Planın örneği (“Bloklar, Şablonlar ve Gezgin CBS'de yok”) Şablonlar için düzeltildi:
  - nesne şablonu (ADR 0176 §4) yeni nesnenin türünü, görünüşünü ve özniteliklerini hazır verir; CBS'nin öğe
    şablonudur;
  - şablon izleri CBS projesinde koşar.
- **Katman menüsü:** iki dosyaya bölündü. `cad/layering/menus.rs` menünün kendisi, `submenus.rs` Görünüş ▸ ve
  Veri ▸. Servis katmanının ayrı erken dönüşü kalktı; servis aynı tablodan geçer.

**Kanıt:**

- **Yeni sınamalar** (`apps/desktop/src/tests/layer_menu_tests.rs`):
  - `each_modules_layer_menu_has_its_entries_on_each_kind_of_layer`:
    - her modülün projesi `kentos-cad`'de ve modülün kendi programında, ayrıca modülü belirsiz proje;
    - sekiz türün katmanı, 42 öğe: tablo ve menüdeki sözcük (9 × 8 × 42 hücre);
    - beklenen değerler ADR'nin tablosundan ayrıca yazılıdır;
    - durum koşulu tek: Veri kaynağına taşı yalnız CBS projesinin nesnelerini taşır.
  - `the_layer_menu_follows_the_command_set_and_the_tabs`: 18 öğe komut kümesiyle, İstatistik ve Grafik sekmeleriyle;
    her öğenin türleri tabloyla.
- **Yeni kurala çevrilen sınamalar:**
  - `module_surface_tests::each_program_has_its_own_bottom_tabs`: Eklenti denetimi kuralsız görünmez, Yol'un sekmeleri;
  - `surfaces_of`: 42 öğenin hepsini karşılaştırır.
  - Eski `each_program_has_its_own_layer_menu` yeni matrise taşındı.
- **Resim:** `.run/gui-evidence/kullanim-9195`:
  - `menu-<modül>-<tür>.png`: 4 modül × 8 tür, açık tema;
  - `menu-<modül>.png`: modül başına tek sayfa;
  - `menu-alt-menuler.png`: Görünüş ▸ ve Veri ▸ açık dört resim.

## §6 AY-04: modül profili kaydı, ortak katalog ve bağımlılık denetimi (23 Kasım 2026)

**Araştırma** (23 Kasım 2026'da okundu):

- **ArchUnit, *User Guide* §8.6 “Freezing Arch Rules”:** büyümüş bir kod tabanında kural konunca var olan ihlaller bir
  depoya yazılır. Sonraki koşular yalnız yeni ihlali bildirir; düzeltilen ihlal depodan kendiliğinden düşer, geri dönüş
  olmaz. Depo düz metin dosyasıdır, sürüm denetimine girer.
  <https://www.archunit.org/userguide/html/000_Index.html>
- **The Rust Programming Language, §6.2 “Matches Are Exhaustive”:** eşleşme bütün durumları kapsamazsa program
  derlenmez. Joker kolu olmayan eşleşmede yeni değer her eşleşmede yerini bulmadan derlenmez.
  <https://doc.rust-lang.org/book/ch06-02-match.html>

**KentOS kararı:**

- Profil, joker kolsuz eşleşmelerle modülün klasöründedir.
- Bağımlılık bir tabanla dondurulur; taban yalnız azalır.
- Ortak tanım `kentos-launch`'tadır, katalog oradan üretilir.

**Profil kaydı** (`apps/desktop/src/shell/module_profile.rs`):

- `ModuleProfile`: alt sekme, panel, durum hücresi, proje ayarının bölümü, uygulama ayarı, katman menüsünün öğesi,
  Kaynaklar'ın bölümü ve Öznitelikler'in satırları için birer sorgu. Ayrıca biçimler, pafta ve bulut türleri, araç
  seçenekleri, Nokta hesapla'nın yapıları, tematik çiziciler ve kitaplık dalları.
- Her modülün profili kendi klasöründedir: `arch/profile.rs`, `map/profile.rs`, `road/profile.rs`, `gis/profile.rs`.
  Sorguların her biri joker kolsuz eşleşmedir.
- Kabuk yalnız kayda sorar (`profile`, kabuğun modül adına göre tek dalı). `kentos-cad` projesizken kabuğun kendi
  profili `NoProject`'tir. `module_surfaces.rs`'in tabloları kalktı; dosya artık hiçbir modülü adıyla anmaz.

**Ortak tanım** (`crates/native/launch`):

- `Module::storage()`: CBS `Storage::Database`, Mimari, Harita ve Yol `Storage::File` (ADR 9196).
- `Module::layer_kinds()` (ADR 9196 §2):
  - CBS: açıklama, veri tablosu, servis, raster, nokta bulutu, grup. Çizim ve bağlı katmanı yoktur.
  - Mimari: çizim, raster, nokta bulutu, grup.
  - Harita ve Yol: sekiz türün hepsi; CBS tablosu salt okunur altlıktır.
- `LayerKind` masaüstünden buraya taşındı; katman menüsü aynı türü kullanır.

**Ortak katalog** (`catalog/v1/modules.json`):

- Her modülün kimliği, başlığı, programı, notu, proje türü, işleri, bulut türleri, saklama türü, katman türleri ve
  şeridin ona verdiği komutlar. Komut sayıları: Mimari 489, Harita 592, CBS 551, Yol 564.
- Hiçbir şeridin tutmadığı komut her modülündür ve listelenmez.
- Üretici bir masaüstü sınamasıdır (`module_catalog_tests::the_module_catalog_is_current`): katalog eskiyse düşer,
  `KENTOS_WRITE_MODULES=1` yeniden yazar. Komut kümesini masaüstünün şerit okuyucusu hesaplar (`Catalog::module_command_list`).
- Web (dönünce), mobil istemciler ve bulut portalı bu dosyayı okur.

**Bağımlılık denetimi** (`scripts/dev/module_deps.py`, taban `scripts/dev/module_deps.baseline.json`):

- Masaüstünün kaynağında sınama dışı her dosyanın klasörler arası bağlarını sayar (`crate::` yolları, yorumlar hariç).
- İzlenen yönler:
  - kabuktan CAD çekirdeğine ve modüllere;
  - CAD çekirdeğinden modüllere;
  - CBS'den CAD çekirdeğine;
  - modülden öbür modüllere.
- Serbest yönler: her dosyanın kabuğa bağı, CAD modüllerinin (Mimari, Harita, Yol) CAD çekirdeğine bağı.
- Modül adına göre dal (`Module::Architecture` …): kabukta ve çekirdekte, kaydın dışında sayılır.
- Bugünkü taban (dosya sayısı):

  | Yön | Dosya | Yön | Dosya |
  |---|---|---|---|
  | kabuk → cad | 77 | cad → gis | 30 |
  | kabuk → gis | 44 | cad → arch | 7 |
  | kabuk → arch | 28 | cad → map | 7 |
  | kabuk → map | 13 | cad → road | 5 |
  | kabuk → road | 12 | gis → cad | 54 |
  | gis → road | 4 | arch → road | 2 |
  | map → gis | 4 | map → road | 22 |

  Modül adına göre dal: kabukta 36, çekirdekte 7. Öbür yönler sıfırdır.
- Planın sayıları (kabuk → cad 227 …) sınama dosyaları ve yorumlarla sayılmıştı. Taban, betiğin saydığıdır.
- `--check` taban aşılırsa düşer: hangi yönde, hangi yeni dosyada olduğunu ve çözümü söyler. Azalanı söyler; `--write`
  tabanı indirir, aşılıyorsa yazmaz.
- Masaüstü sınaması denetimi koşar (`module_catalog_tests::the_modules_keep_to_their_dependency_baseline`).

**Kural sınaması** (bilerek bozuldu, sonra geri alındı):

- `BottomTab`'a profile yerleşmeyen `Deneme` sekmesi eklendi: program derlenmedi. `E0004` hataları beş profilin
  hepsinde (`NoProject`, Mimari, Harita, Yol, CBS) ve sekmenin kendi eşleşmelerinde geldi.
- `arch/profile.rs`'e `use crate::map_profile::Map;` eklendi: denetim düştü (“arch → map: 0 dosyadan 1 dosyaya çıktı;
  yeni bağ: arch/profile.rs …”).
- `shell/modes.rs`'e `Module::Gis` eklendi: denetim düştü (“shell: modül adına göre dal sayısı arttı (taban 36, şimdi
  37) …”).

**Kalan:**

- Kabuk hâlâ modüllere bağlı (taban) ve 36 yerde modül adına göre dallanıyor. Taban AY-07'nin paketlere bölmesiyle
  azalır.
- `arch/program.rs` programın kimliğini ve bütün modüllerin dallarını tutar. Yeri kabuk ya da `kentos-launch`'tır;
  taşınması dal sayısını kabuğa ekleyeceği için `kentos-launch`'a doğru yapılır.

## §7 Kalan

AY-05 ve sonrası. Her madde kanıtıyla bu ADR'ye ya da kendi ADR'sine yazılır.
