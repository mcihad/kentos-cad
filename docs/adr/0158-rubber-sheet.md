# ADR 0158: Kauçuk levha

- **Durum:** kabul edildi (2026-10-02). `HYB-05`'in ikinci yarısının ilk işi; komşu pafta kenar eşlemesi kendi ADR'sindedir.
- **Tarih:** 2026-10-02
- **Bağlam belgesi:** TODOS.md `HYB-05`, ADR 0156 (Vektör oturtma: kontrol noktaları, pencere, `cad.entities.transform`, `ops::warp`), [araştırma kaydı](../research/2026-10-01-netcad-arcgis-qgis.md); ArcGIS Pro Rubbersheet Features, QGIS Georeferencer (Thin Plate Spline).

## Bağlam

Vektör oturtma çizimi tek bir dönüşümle (Helmert, afin, projektif) ülke sistemine taşır. Eski bir paftanın sayısallaştırılması ya da yerel bir ölçü yer yer farklı bozulmalar taşır. Tek dönüşümden sonra kontrol noktalarında santimetreler kalır (artıklar).

Kauçuk levha (rubbersheet) bu artıkları yerel olarak giderir:

- her kontrol noktası hedefine **tam** oturur;
- çevresindeki nesneler yakınlıklarıyla orantılı kayar;
- uzaktaki nesneler genel eğilimle (dönüşümle) gelir.

Yerinde kalması istenen yerler sabit noktalarla (kaynağı hedefi olan bağ) tutulur.

ArcGIS bunu bağlar (displacement links), sabit noktalar (identity links) ve iki yöntemle (Linear: üçgen ağı; Natural Neighbor) yapar. QGIS kestirmeyi ince plaka eğrisiyle (Thin Plate Spline) yapar.

## Karar

### 1. Bağlar

Kauçuk levha Vektör oturtma penceresinin dördüncü dönüşümüdür: Helmert, Afin, Projektif, **Kauçuk levha**. Kontrol noktaları tablosu, Adla eşle, Kullan ve çizimden seçme aynen kullanılır.

- **Bağ:** kullanılan her çift bir bağdır, kaynaktan hedefe.
- **Sabit nokta:** kaynağıyla hedefi aynı olan bağ. Satırın "Sabit" düğmesi hedefe kaynağı yazar.
- **En az:** 3 bağ gerekir ve kaynakları bir doğru üstünde olmamalıdır.
- **Aynı kaynak:** iki bağın kaynağı aynıysa çözüm yoktur, söylenir.

### 2. Yöntem: ince plaka eğrisi

Kayma alanı d(p) = hedef − kaynak, bağlardan geçen ince plaka eğrisidir (thin plate spline):

- d(p) = a₀ + a₁·x + a₂·y + Σ wᵢ·φ(|p − sᵢ|), φ(r) = r²·ln r (φ(0) = 0);
- d(sᵢ) = dᵢ, Σ wᵢ = 0, Σ wᵢ·xᵢ = 0, Σ wᵢ·yᵢ = 0;
- her bileşen (doğu, kuzey) için aynı (n+3)×(n+3) denklem takımı çözülür;
- nokta p'nin görüntüsü p + d(p)'dir.

Özellikleri:

- **Tam:** bağlardan tam geçer.
- **Pürüzsüz:** bükülme enerjisi en küçük olan alandır; kıvrımsız ve kırıksızdır.
- **Her yerde tanımlı:** bağların dışında genel eğilime (afin kısma) yaklaşır; yırtılma ve sıçrama yoktur.
- **Tek:** üçgenleme yoktur; sonuç bağların sırasına bağlı değildir.

Sayısal kurallar:

- Koordinatlar bağ kaynaklarının merkezine göre ve en büyük uzaklığa bölünerek ölçeklenir. İnce plaka eğrisi ölçekten bağımsızdır (ölçek yalnız afin kısmı değiştirir), takımın koşulu iyileşir.
- Takım, kısmi pivotlu Gauss yöntemiyle çözülür.
- Pivot, takımın en büyük öğesinin 1e-12'sinin altına düşerse çözüm yoktur. Köşegen ölçü olamaz: φ(0) = 0'dır ve afin bloğun köşegeni sıfırdır.
- En çok 1000 bağ.

Doğrusal yöntem (üçgen ağı, ArcGIS'in Linear'ı) bu ADR'de yoktur. Pafta ızgarası gibi düzenli kontrol noktalarında her kare eş-çemberlidir, Delaunay üçgenlemesi tek değildir: köşegen seçimine göre sonuç değişir. Kanonik bir köşegen kuralı ve kesin incircle yüklemiyle ayrı bir adımda eklenebilir.

### 3. Nesneler: yalnız köşeler

Kauçuk levha küçük, yerel düzeltmedir. Nesnelerin biçimi korunur, köşeleri taşınır:

- **Noktalar ve köşeler:** noktalar, çizgilerin uçları, yolların ve halkaların köşeleri (delikler ve parçalar dahil), eğrinin denetim noktaları haritayla taşınır.
- **Kenarlar:** doğru kenarlar doğru kalır; araya köşe eklenmez (parsel kenarına sahte köşe gelmez).
- **Yaylar ve çemberler:**
  - Yolların yaylı kenarlarında yayın şişkinliği (bulge) korunur, uçları taşınır.
  - Daire, yay ve elipsin merkezi haritayla taşınır. Biçimi merkezdeki en yakın benzerlikle değişir (√|det J| ölçek, J'nin ilk sütununun dönüklüğü); türü değişmez.
- **Yazı, not, blok, ölçü ve tarama:** ADR 0156 §4'teki gibi, yerindeki en yakın benzerlikle.

Gerçek görüntüden sapma sayılır ve söylenir. Doğru kenarın ve yayın gerçek görüntüsü eğridir; tutulan kenar ve yay bundan sapar. Her kenar ve yay için orta ve çeyrek noktalarda sapma ölçülür. 0,1 mm'yi aşan nesneler uyarıyla sayılır (`rubber_bends`): kaç nesne ve en çok kaç mm saptığı. Sapma kenarın boyuna ve düzeltmenin değişimine bağlıdır. Ortak durumlardaki 40 × 25 m'lik ızgarada santimetrelik düzeltmeler 30 m'lik kenarı 8 mm'ye kadar büker. Köşe eklemek parsel kenarına sahte köşe getireceğinden kenar doğru kalır, sapma söylenir.

### 4. Komut

`cad.entities.transform`'un yeni türü `rubbersheet`:

- **Girdi:** `{ kind: "rubbersheet", links: [{ from, to }] }`.
- **Retler:** `invalid_links` (yolu `transform.links`): 3'ten az bağ, aynı kaynak, bir doğru üstündeki kaynaklar, çözümsüz takım, 1000'den çok bağ.
- **Geri alma adımı:** "Kauçuk levha".
- **Uyarılar:** `warp_shapes` (ADR 0156) ve `rubber_bends`.

Kilitli katman, kopya ve kimlik kuralları dönüşümün öbür türleri gibidir.

### 5. Pencere

Dönüşüm "Kauçuk levha" iken:

- Artık sütunları Helmert artıklarını gösterir: her bağda yerel olarak giderilecek düzeltmedir.
- Özet şunları söyler:
  - bağ ve sabit nokta sayısı;
  - bağların tam geçtiği;
  - en büyük ve ortalama yerel düzeltme;
  - Helmert'in m0'ı.
- Satırın düğmeleri: kaynağı ve hedefi çizimden seçme, ayrıca **Sabit** (hedefe kaynağı yazar).
- Uygula, rapor ve kapsam (Seçili, Katman, Tümü; Kopya) aynıdır.

Rapor bağları, Helmert artıklarını ve yöntemi yazar.

### 6. Ortak çekirdek

- **`ops::rubber`** (WASM `rubberSheet`): bağlardan çözüm, haritanın ve türevinin (Jacobian) değeri, çözümsüzlüğün nedeni.
- **`ops::warp`:** `Warp::Sheet` ile §3'ün kuralları ve sapmanın ölçümü.
- **Bağımsız başvuru:** `scripts/fixtures/rubber_cases.py`. Takımı mpmath ile 50 basamakta çözer, haritayı ve türevi bu duyarlıkla hesaplar. Bağ kümeleri:
  - düzenli ızgara;
  - dağınık noktalar;
  - sabit noktalı yerel bozulma;
  - büyük TM koordinatları;
  - en az bağ;
  - çözümsüzlükler.

  Ortak durumlar: `fixtures/fit/v1/rubber.json`.

### 7. İş sırası

1. Çözüm: `ops::rubber`, WASM, bağımsız başvuru ve ortak durumlar.

   *(2 Ekim: tamam.)*
   - **Çekirdek:** `ops::rubber` (`Sheet::solve`, `map`, `jacobian`, `Link`, `RubberError`); WASM `rubberSheet`, web cephesi `model/ops/rubber.ts`.
   - **Başvuru:** `scripts/fixtures/rubber_cases.py`, takımı ölçeksiz, kendi koordinatlarında mpmath ile 50 basamakta çözer (çekirdek ölçekler; eğri aynıdır). 9 durum yazar (`fixtures/fit/v1/rubber.json`):
     - üç bağda yalnız afin;
     - kenarı sabit 4×4 pafta ızgarası;
     - TM koordinatlarında dağınık 12 bağ;
     - sekiz sabit noktayla yerel bozulma;
     - hepsi sabit;
     - ince yol şeridi;
     - üç çözümsüzlük.
   - **Sonuç:** çekirdek (yerli) ve web (WASM) görüntüleri 1e-9 m, türevleri 1e-9 içinde verir.
   - **Koşullanma:** 200 m'de 1 mm'lik "neredeyse doğru" bağlar kasıtlı kötü koşullu bir sınamaydı: float64 60 nm farka düşüyordu. Şeridin 3,5 m genişliği gerçekçi durumu sınar; yalnız tam doğru reddedilir.
2. Nesneler: `ops::warp`'ın `Sheet`'i, §3'ün kuralları ve sapma; ortak durumlar.

   *(2 Ekim: tamam.)*
   - **Çekirdek:** `ops::warp`'ta harita bir arayüzdür (`point`, türev). Dönüşüm ve levha yazı, not, blok, ölçü, tarama, nokta, çizgi, eğri, sonsuz doğru ve ışın kurallarını paylaşır (`common`). Levhanın kendi kuralları `sheet_shape`'tedir: yollar ve halkalar yalnız köşeler, daire, yay ve elips merkezdeki en yakın benzerlik. `bend_of` sapmayı ölçer, `sheet_shapes` hepsini birlikte işler. WASM `rubberShapes`, web cephesi `model/ops/warp.ts`.
   - **Başvuru:** `scripts/fixtures/rubber_warp_cases.py`. Haritayı ve türevi mpmath başvurusundan (`rubber_cases.py`), ortak türlerin kurallarını `warp_cases.py`'den alır. 3 durum yazar (`fixtures/fit/v1/rubber-warp.json`):
     - 5×5 pafta ızgarası, santimetrelik kaymalar: 17 tür, delikli ve çok parçalı alan dahil; 10 nesne 0,1 mm'den çok sapar, en çok 8,1 mm;
     - metrelerce güçlü levha: en çok 0,81 m;
     - bir doğru üstündeki bağlar.
   - **Sonuç:** çekirdek (yerli) ve web (WASM) geometriyi 1e-9 m, öbür sayıları göreli 1e-12, sayımları bire bir, en büyük sapmayı 1e-9 m içinde verir. Dönüşümlerin ortak durumları (`warp.json`) değişmedi.
3. Komut: `rubbersheet` türü iki platformda, ortak komut durumlarıyla.

   *(2 Ekim: tamam.)*
   - **Sözleşme:** `Transform`'un `rubbersheet` türü ve `RubberLink` (katalog, TS tipleri, Python SDK'sının tipleri yeniden üretildi); ret `invalid_links`, uyarı `rubber_bends`, adım “Kauçuk levha”.
   - **İşleyiciler:** masaüstü `crates/native/application/src/transform.rs`, web `product/entitiesTransform.ts`. Bağların sayıları sırayla denetlenir ("2. bağın hedefinin kuzey (X) değeri …", yolu `transform.links[1].to.y`), sonra levha çekirdekte çözülür.
   - **Durumlar:** `scripts/fixtures/transform_command_cases.py` 4 durum ekler:
     - sekiz nesne: nokta, çizgi, delikli alan, yaylı çoklu çizgi, daire, yay, yazı ve kilitli katmanda çizgi; doğrulama, yazma ve geri alma;
     - kopya;
     - retler: sonlu olmayan sayı (sayılar az bağdan önce), 3'ten az bağ, aynı kaynak, bir doğru, ülke koordinatında bir bit aralıklı iki kaynakla tek çözümsüz takım.
   - **Beklenen değerler:** bit bit karşılaştırılır. Levhanın bitleri çözümünün aritmetiğidir. `scripts/fixtures/sheet_f64.py` levhayı çekirdekle aynı işlem sırasıyla çözer:
     - V8'in `Math.hypot`'u;
     - eşit pivotta sonuncuyu seçen kısmi pivotlama;
     - libm'in `log`'u. Python'a taşındı: 150 000 girdide libm ile bit bit aynıdır; glibc'nin `log`'u bunların %5,6'sında son bitte ayrılır.

     Doğruluğu 50 basamaklı başvuruya bağlıdır: `rubber.json`'un durumlarında 1e-12 m içindedir.
   - **Uyarı:** sapma uyarısının sayımı ve gösterilen milimetresi, eşiğe ve yuvarlama sınırına uzaklığı denetlenerek yazılır.
   - **Sonuç:** masaüstünün, web'in ve Python SDK'sının çalıştırıcıları 48 durumu geçer.
4. Pencere: dördüncü dönüşüm iki platformda; resimler.

   *(2 Ekim: tamam.)*
   - **Pencere:** Vektör oturtma'nın Dönüşüm'ü dört seçeneklidir: Helmert, Afin, Projektif, Kauçuk levha. Masaüstünde `calc/fit/rubber.rs`, web'de `ui/calc/fitRubber.ts`.
   - **Tablo:** kullanılan çiftler levhanın bağlarıdır. Artık sütunları Helmert'in artıklarıdır, yani her bağda levhanın yaptığı yerel düzeltme. En büyüğü işaretlenir.
   - **Sabit:** satırın kilit düğmesi hedefe kaynağı yazar. Kaynağı eksik satırda hedefe dokunmaz, nedenini söyler.
   - **Özet:**
     - bağ ve sabit nokta sayısı, levhanın her bağdan tam geçtiği;
     - en büyük yerel düzeltme ve satırı, ortalaması, Helmert m0;
     - hatalı bağın da tam geçildiği, ölçü hatasının Kullan'dan çıkarılacağı;
     - bağlar levha vermiyorsa nedeni (3'ten az, aynı kaynak, bir doğru, tek çözümsüz takım, 1000'den çok). O sırada Uygula kapalıdır.
   - **Uygula ve rapor:** Uygula `rubbersheet` dönüşümünü yazar (adım “Kauçuk levha”); uyarılar günlüğe gider. Rapor yöntemi, bağları, yerel düzeltmeleri ve Helmert'in sayılarını yazar.
   - **Testler:** masaüstünde 5 yeni test. İki adımlı iş de sınanır: Helmert ile Oturt, sonra Adla eşle, Kauçuk levha, P3 sabit. Kullanılan bağlar 1e-6 m içinde oturur, sabit nokta yerinde kalır, daire daire kalır, geri alma tek adımdır.
   - **Resimler:** dört sahne (`oturt-levha`, `-sabit`, `-uygulandi`, `-uyari`) iki platformda, iki temada, iki boyutta. Sayılar iki platformda aynıdır: yerel düzeltme en çok 124,5 mm (P5), ortalama 41,8 mm, m0 ±48,5 mm; Helmert'ten sonra en çok 4,3 mm.

Her adım iki platformda, ortak fixture'larla, kendi commit'inde ilerler.

## Sonuçlar

- Tek dönüşümden sonra kalan artıklar yerel olarak giderilir; kontrol noktaları tam oturur, uzak nesneler genel eğilimle gelir.
- Nesnelerin türü ve köşe sayısı değişmez; gerçek görüntüden sapma sayıyla söylenir.
- Üçgen ağıyla doğrusal yöntem ve kenar eşleme ayrı işlerdir.

## Doğrulama

- **Çözüm:** bağlarda tam geçiş (1e-9 m), ara noktalarda değer ve türev 50 basamaklı bağımsız başvuruya göre iki platformda; ölçekten bağımsızlık; büyük koordinatlar; çözümsüzlükler.
- **Nesneler:** köşeler, şişkinlikler, merkezler ve en yakın benzerlik bağımsız başvuruya göre; sapma sayımı.
- **Arayüz:** resimler iki temada, 1440×900 ve 1100×650.
