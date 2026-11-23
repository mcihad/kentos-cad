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
- **AY-03:** yüzey matrisi; katman menüsü modül ve katman türüyle. Bu ADR'ye §4 olarak matris eklenir.
- **AY-04:** profilin modüllere taşınması, `ModuleProfile` kaydı, bağımlılık denetimi, joker kolsuz tablolar, ortak
  modül kataloğu.
- **AY-07:** paketlere bölme.

## §4 Kalan

AY-02 ve sonrası. Her madde kanıtıyla bu ADR'ye ya da kendi ADR'sine yazılır.
