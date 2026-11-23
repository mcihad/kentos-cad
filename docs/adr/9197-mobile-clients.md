# ADR 9197: Mobil görüntüleyici ve mobil uygulama: istemcilerin ayrımı

- **Durum:** kabul edildi (2026-11-23); plan [plan/ayrim.md](../../plan/ayrim.md) AY-08. ADR 9195'in istemci kuralını
  (§2.6) ve ADR 9196'nın saklama modelini istemcilere uygular.
- **İstek:** sahibin sözleri (23 Kasım 2026), sırasıyla:
  - “İleride mobil görüntüleyiciler düzenleyiciler de geliştireceğiz, başka modüller de, ayrımı netleştirdikten sonra
    bir daha sistemin bozulmaması için kati kurallar olarak claude.md içerisine ekle bu kuralı da”;
  - “Mobil görüntüleyici ve uygulama da geliştireceğiz ileride ona göre de ayrım yap”.
- **Kural:** CLAUDE.md §25 kural 1, 4, 6, 7 ve 8. Tarif kılavuz §9.9 “Yeni istemci”.
- **Web:** ertelendi (ADR 0228); web dönünce aynı ortak tanımı okur. **`.kcad`:** bu adımda değişmez.

## §1 Bağlam

**Bugün olanlar:**

- Modülün ortak tanımı `kentos-launch`'tadır ve `catalog/v1/modules.json`'a üretilir: kimlik, proje türleri, saklama
  türü, katman türleri, komut kümesi (ADR 9195 §6).
- Saklama türü proje türündendir: CBS verisi veritabanında, CAD projesi proje dosyasında (ADR 9196).
- Bulut API'si:
  - CBS verisini rolün okuduğu alanlarla verir (`FeaturePage`, ADR 9194);
  - yazmayı formun kuralları ve rolün kapsamıyla denetler (`form_guard`, `kentos_forms::reach`);
  - CAD projesini dosya revizyonu olarak verir (ADR 0031).
- Erişim anahtarının kapsamı vardır: `TokenScope::Read` yalnız `GET` ve `HEAD` yapar.

**Araştırma** (23 Kasım 2026'da okundu):

- **QField ve QFieldSync** (QGIS'in mobil istemcisi):
  - Katmanın mobilde ne olacağını masaüstündeki proje belirler (katman eylemi):
    - çevrimdışı düzenleme: değişiklikler bir değişiklik günlüğüne yazılır, eşitlemede kaynağa uygulanır;
    - kopya ya da var olanı koru;
    - projeden çıkar;
    - çevrimiçi kaynağa (PostGIS, WFS) doğrudan erişim.
  - Eylem projede katmanın özelliği olarak saklanır; bulut, projeyi mobile paketlerken uygular.
  - <https://docs.qfield.org/reference/qfieldcloud/system/>,
    <https://docs.qfield.org/get-started/tutorials/get-started-qfs/>
- **Esri, *ArcGIS Field Maps: Configure the map*:**
  - Harita sahaya web'deki tasarımcıda hazırlanır.
  - Haritanın yeteneği ya “düzenlenebilir” ya “salt okunur”dur.
  - Çevrimdışı kullanım ayrı hazırlanır. Veri formlarla girilir. Harita mobil uygulamadan gizlenebilir.
  - <https://doc.esri.com/en/arcgis-field-maps/12.2/prepare-maps/configure-the-map.html>
- **Autodesk, *Access Drawings with the AutoCAD mobile app*:**
  - Mobil uygulama DWG'yi görüntüler, oluşturur ve düzenler; çevrimdışı çalışır, sonra eşitler.
  - Katmanlar, işaretleme ve ölçme vardır. Aboneliksiz kullanım salt okunurdur.
  - <https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-DidYouKnow/files/GUID-20A8F96D-0ACA-49F6-A6FF-D7DD71A8D03A.htm>

**Ortak örüntü:**

- Mobil istemci masaüstünde ya da bulutta hazırlanan projeyi okur.
- Görüntüleme ve düzenleme yeteneği açıkça bildirilir.
- Katmanın mobildeki davranışı projenin verisidir.
- Mobildeki düzenleme masaüstüyle aynı hizmetten geçer (servis, eşitleme günlüğü).

## §2 Karar

1. **İstemciler ortak tanımdadır** (`kentos_launch::Client`): masaüstü, web, mobil görüntüleyici, mobil uygulama.
   Her biri neyi yazdığını saklama türüne göre söyler (`Client::writes`):

   | İstemci | CBS (veritabanı) | CAD (proje dosyası) |
   |---|---|---|
   | Masaüstü | veri satırları (düzenleme oturumu) ve proje dosyası (yapı, stil, açıklama) | proje dosyası (komutlarla) |
   | Web | dönünce masaüstünün kuralıyla | dönünce masaüstünün kuralıyla |
   | Mobil görüntüleyici | hiçbir şey | hiçbir şey |
   | Mobil uygulama | veri satırları (sunucu komutuyla) | hiçbir şey (şimdilik görüntüler) |

2. **Mobil görüntüleyici** her modülün projesini okur, hiçbir şey yazmaz.
   - CBS'de veri katmanlarını rolün okuduğu alanlarla alır (`FeaturePage`); okunmayan alanı göstermez, adını söyler.
   - CAD'de son dosya revizyonunu okur.
   - Bulutta okuma kapsamlı anahtarla çalışır (`TokenScope::Read`): sunucu da yazmasını reddeder.
3. **Mobil uygulama** CBS'de veri katmanlarının satırlarını sunucu komutuyla yazar.
   - Formun kuralları ve rolün kapsamı masaüstündekiyle aynıdır (`form_guard`, `kentos_forms::reach`).
   - Proje dosyasını (yapı, stil, açıklama) yazmaz; onları masaüstü hazırlar.
   - CAD projelerinde görüntüleyicidir. Mobilde CAD düzenleme (işaretleme ya da hafif düzenleme) ayrı bir ADR ister.
4. **Katman türünün yetenekleri ortaktır.**
   - Nerede tutulduğu (`LayerKind::kept`):
     - proje dosyası: çizim, açıklama, bağlı, raster, nokta bulutu (bağlının nesneleri tablosuna da yazılır, rasterin
       pikselleri ve bulutun noktaları yanındaki dosyalardadır);
     - kaynak tablo: veri tablosu;
     - servisin sunucusu: servis;
     - hiçbiri: grup.
   - Yazılışı (`LayerKind::written_as`): proje dosyasının nesnesi ya da kaynağın satırı. Servis ve grup nesne olarak
     yazılmaz.
   - Hangi istemcinin yazdığı ikisinden çıkar (`LayerKind::written_by`): mobil uygulama yalnız CBS'nin veri
     tablosunu yazar, görüntüleyici hiçbir şeyi.
5. **Mobilde katmanın davranışını masaüstündeki proje belirler:** göster, düzenle ya da gösterme (QFieldSync'in
   katman eylemi gibi). Bu proje dosyasının bir alanıdır; mobil uygulamanın ilk diliminde ADR'siyle eklenir.
6. **Ekranı projenin modülü belirler** (kural 1).
   - Mobil istemcinin her modül için kendi profili vardır.
   - Masaüstünün profiline bağlanmaz; ortak tanımı (`modules.json`) okur.
7. **Ortak çekirdek arayüzsüzdür.**
   - Mobil istemcinin kullanacağı crate'ler masaüstü, sunucu ya da tarayıcı çalışma zamanına bağlanmaz:
     `kentos-launch`, `kentos-contracts`, `kentos-kcad`, `kentos-forms`, `kentos-expression`, `kentos-geometry-core`.
   - `scripts/arch/deps.mjs`'in grupları bunu korur. AY-07'nin paketleri bu sınırla bölünür.
   - Mobil uygulamanın teknolojisi (yerel ya da web tabanlı) kendi ADR'sinde seçilir. Ortak tanım ikisine de
     JSON (`modules.json`) ve Rust olarak açıktır.

## §3 Uygulama (23 Kasım 2026)

- `kentos-launch`:
  - `Client` (`ALL`, `id`, `title`, `writes`);
  - `Writes`: proje dosyası ya da veri satırları;
  - `LayerKind::kept` (`Kept`), `LayerKind::written_as` ve `LayerKind::written_by`.
- `catalog/v1/modules.json`:
  - istemciler ve her birinin saklama türüne göre yazdıkları;
  - katman türlerinin tutulduğu yer ve onları yazan istemciler.
- CLAUDE.md §25 kural 7 ve kılavuz §9.9 bu tanıma bağlandı.
- Sınama: `kentos-launch`'ın birim sınamaları; masaüstünün `module_catalog_tests`'i kataloğun bu bölümünü ortak
  tanımla karşılaştırır.

## §4 Kalan

- ~~Katmanın mobil davranışı (göster, düzenle, gösterme): proje dosyasının alanı, ADR'siyle.~~ Yapıldı:
  [ADR 9198](9198-layer-on-mobile.md) (`LayerNode.mobile`, `.kcad` şema 256, `cad.layers.set`'in `mobile` işlemi,
  masaüstünde Mobilde ▸).
- Mobil istemcinin modül profilleri.
- CBS'de çevrimdışı düzenleme: değişiklik günlüğü ve eşitlemede sunucu komutları.
- Mobilde CAD düzenleme kararı.
- Mobil uygulamanın teknolojisi.
