# ADR 0149: Ölçü doğruluğu ve gösterim yuvarlaması

- **Durum:** kabul edildi (2026-10-01). Sahibin bildirimi ve isteği (1 Ekim): bir çemberin yarıçap ve çap ölçüsü arasında 0,001 fark görüldü; “bu bir CAD yazılımı, hesaplamalar kusursuz olmalı”, “başka ölçümlerde de sorun varsa”, “yuvarlama sorunu kabul edilemez”. Bu iş `HYB-01`'den (ADR 0148) önce yapılır.
- **Tarih:** 2026-10-01
- **Bağlam belgesi:** CLAUDE.md §5, §23 (sayısal ve kadastral doğruluk); `crates/shared/geometry-core/src/numeric.rs` (kadastro değerlerinin ondalık politikası); `apps/web/src/app/format.ts` ve `crates/native/interaction/src/format.rs` (gösterim); ADR 0061 ve 0147 (ölçüler), ADR 0100 (ifadelerin geometri değerleri ve bağımsız başvurusu).

## Bağlam

Yarıçap ve çap ölçüsünün değerleri yeniden üretildi (TM koordinatlarında, 100 000 rastgele çember ve elle yazılmış yarıçaplarla, çekirdeğin formülleriyle):

- **Hesap doğrudur.** Yarıçap ölçüsünün değeri çemberin yarıçapından en çok 4,7·10⁻¹⁰ m farklıdır. Bu fark, 4,4 milyonluk koordinatın kayan noktalı sayıdaki çözünürlüğüdür (yaklaşık 10⁻⁹ m). Alan yerel başlangıca göre hesaplanır, büyük koordinatta sayı kaybı yoktur.
- **Kusur gösterimdedir.** Değer tam yarımda biten bir sayıysa (iki basamakla 12,125; üç basamakla 5,0005; 7,8465), gösterilen değer ölçünün hangi yöne konduğuna göre aşağı ya da yukarı yuvarlanır: 12,12 ya da 12,13. Çemberin kendi yarıçapıyla da çelişebilir. Nedeni 10⁻¹⁰ m düzeyindeki gürültünün yarımın hangi yanına düştüğüdür. Yazılan 5,0005 de ikili sayıda 5,000499999… olduğu için bugün 5,000 gösterilir; kâğıtta 5,001'dir.
- **Bağımsız yuvarlama hata değildir.** Fareyle çizilen r = 7,8465123'lük çemberde R 7,847, Ø 15,693'tür; 2 × 7,847 = 15,694. İki değer de kendi tam değerinin doğru yuvarlamasıdır (d = 15,6930246). Rastgele yarıçapların yarısında böyle olur. Çapı yuvarlanmış yarıçaptan türetmek yanlış bir değer göstermek olurdu; bunu yapan CAD de yoktur.
- **Gösterim tek yerden geçmiyor.** Web'de 77 doğrudan `toFixed` kullanımı var; masaüstünde bazı değerler `format!` ile yazılıyor. Kural ilk değişiklikte dağılır.

Kadastro değerleri için açık ve sürümlü bir yuvarlama kuralı vardır (`numeric.rs`, CLAUDE.md §23.2). Ekranda ve çizimdeki yazıda görünen sayılar için yoktur.

## Karar

### 1. Gösterim kuralı (gösterim v1)

Bir sayı, gösterildiği birimde (m, m², dönüm, ha, grad, derece, %) şöyle yazılır:

1. **Gürültü basamağı:** değer önce 7 ondalık basamağa yuvarlanır. Kayan noktalı sayının tam ikili değerinden, yarım sıfırdan uzağa. 10⁻⁷ m (0,1 µm), TM koordinatlarının hesap gürültüsünün (10⁻⁹ m düzeyi) yüz katıdır, en ince ölçme basamağının (mm) on bin katı küçüğüdür. OpenCASCADE de iki noktayı 10⁻⁷ içinde aynı sayar (`Precision::Confusion`).
2. **Gösterilen basamak:** sonra bu ondalık değer gösterilecek basamağa yuvarlanır, yarım sıfırdan uzağa (kâğıttaki gibi: 12,125 → 12,13; −2,5 → −3).
3. **İnce gösterim:** gösterilecek basamak 7 ya da daha çoksa tek adımda, o basamağa yuvarlanır.
4. **İşaret:** sıfıra yuvarlanan değer eksi işaretsiz yazılır (−0,0001 → 0,000).
5. **Değişmeyenler:** ondalık ayırıcı noktadır. Sonlu olmayan değer “NaN”, “Infinity” ve “-Infinity” olarak yazılır (arayüz göstermez). Kural 10²¹'den küçük değerler içindir.

Sonuç: aynı büyüklük her yolda (çemberin yarıçapı, yarıçap ölçüsü, Öznitelikler, üzerine gelme kartı) aynı yazılır. Yazılan değer kâğıttaki gibi yuvarlanır. İki platform aynı karakterleri yazar.

### 2. Tek yer

- **Çekirdek:** `kentos_geometry_core::display::fixed`. Masaüstünün `Format`'ı ve çekirdekte metin üreten her yer (kenar ve köşe yazıları, raporlar) bunu kullanır.
- **Web:** `core/displayNumber.ts`'in `fixed`'i. `Formatter` ve ölçü yazan her yer bunu kullanır. Web her sayı için WASM çağırmaz (etiketlerin sıcak yolu); iki uygulama aynı ortak durumlarla eşit tutulur.
- **Doğrudan yuvarlama yasak:** ölçü, koordinat, alan, açı ya da çizime yazılan sayı `toFixed` ya da `format!("{:.N}")` ile yazılmaz. Ölçü olmayan sayılar bunun dışındadır (SVG düzenleyicisinin pikselleri, ilerleme yüzdesi, süreler); onlar da gözden geçirilir.

### 3. Ölçülerin denetimi

Ölçüler bağımsız, yüksek hassasiyetli bir başvuruyla iki platformda sınanır:

- **Başvuru:** `scripts/fixtures/measure_cases.py`. Yalnız Python'un ve `mpmath`'in 50 basamaklı aritmetiğiyle, KentOS kodu olmadan. Çıktısı `fixtures/measure/v1/cases.json`.
- **Ölçüler:**
  - uzunluk (çizgi, yaylı çoklu çizgi, yay, daire çevresi);
  - alan ve çevre (yaylı, delikli, çok parçalı alan);
  - yarıçap, çap, yay açısı;
  - iki nokta arası uzaklık, ΔY, ΔX, semt (grad ve derece), eğim;
  - açı;
  - on ölçü türünün değeri;
  - koordinat okuması.
- **Durumlar:** TM koordinatlarında (487 000, 4 420 000) ve başlangıca yakın; elle seçilmiş sınırlar (yarımda biten değerler, çok küçük ve çok büyük yaylar, neredeyse paralel kenarlar, çok küçük alanlar) ve rastgele durumlar.
- **Ölçüt:**
  1. hesaplanan değer kesin değerden en çok belirtilen sınır kadar uzaktır (uzunlukta 10⁻⁸ m, alanda 10⁻⁶ m², açıda 10⁻¹² radyan);
  2. 0–6 basamakta gösterilen yazı, kesin değerin aynı kuralla yazılışıyla birebir aynıdır.
- **Platformlar:** çekirdek yerelde ve web WASM'ında aynı durumları geçer. Gösterim kuralının ortak durumları (`fixtures/numeric/v1/display.json`, bağımsız başvuru `scripts/fixtures/numeric_display.py`) çekirdeği ve web'in TypeScript uygulamasını sınar; WASM üzerinden rastgele karşılaştırma da yapılır.

Denetimde bulunan her sapma kaynağında düzeltilir. Tolerans büyütülmez, beklenen değer hataya göre yenilenmez (CLAUDE.md §23.4).

### 4. Kapsam dışı

- **Çapın yarıçaptan türetilmesi:** §Bağlam'daki bağımsız yuvarlama değiştirilmez.
- **Ölçü stilinin kendi basamağı ve yuvarlama artımı** (AutoCAD'in DIMDEC'i, DIMRND'si): `CAD-21`. Bugün ölçüler projenin uzunluk basamağını kullanır.
- **Kadastro değerlerinin ondalık politikası** (`numeric.rs`): değişmez. Bu ADR yalnız gösterimdir; gösterilen yazı hesaba geri girmez.

### 5. İş sırası

1. **Gösterim kuralı:**
   - çekirdekte ve web'de `fixed`;
   - ortak durumlar ve bağımsız başvuru;
   - iki platformun biçimlendiricileri ve ölçü yazan bütün yerler;
   - resimler.

   *(1 Ekim: tamam.)*
   - **Kural:** çekirdekte `display::fixed`, web'de `core/displayNumber.ts`. Bağımsız başvuru `scripts/fixtures/numeric_display.py` 1556 durum yazar (`fixtures/numeric/v1/display.json`): yarımda biten ve yazılan değerler, gürültülü yarımlar, koordinatlar, işaret, taşma, ince basamak. İki uygulama hepsini geçer; WASM üzerinden 10⁵ rastgele değerde aynı yazarlar.
   - **Masaüstü:** `Format::fixed` ve işlem araçlarının `to_fixed`'i çekirdeğinkini çağırır. Kuralı atlayan üç yer (blok yerleştirmenin ölçek ve dönüşü, çizgi kalınlığı, proje orijini) ona geçti.
   - **Web:** `Formatter` ve ölçü yazan 23 dosyadaki doğrudan `toFixed`'ler `fixed`'e geçti: hesap pencereleri, Öznitelikler, araçların istem ve iletileri, kot noktası, Kenar uzunluklarını yaz. Ölçü olmayanlar (yazı tipinin pikseli, süre, dosya boyutu, SVG düzenleyicisi) ve örnek çizimin verisi değişmedi.
   - **Resim:** yarıçapı 50,0005 yazılmış çemberin sekiz yarıçap ölçüsü ve çapı (masaüstü `olcu-yarim-yaricap`, web `shots.mjs rounding`).
2. **Ölçülerin denetimi:**
   - başvuru ve durumlar;
   - çekirdek ve WASM testleri;
   - bulunan sapmaların düzeltilmesi.

   *(1 Ekim: tamam.)*
   - **Başvuru:** `scripts/fixtures/measure_cases.py` (mpmath, 50 basamak) 224 durum yazar (`fixtures/measure/v1/cases.json`), başlangıçta ve TM koordinatlarında. Alan Green teoremiyle, yaylar merkezlerinden, elips ve eğri tümlevle hesaplanır; KentOS'un formülleri kullanılmaz.
   - **Testler:** çekirdek (`tests/measure.rs`) ve web WASM'ı (`wasm/measure.wasm.test.ts`) işlemleri adlarıyla çağırır. Hepsi sınır içindedir; 0–6 basamaktaki yazıları kesin değerinkiyle karakteri karakterine aynıdır.
   - **Bulgu 1, eğrinin uzunluğu:** çizildiği kırık çizgiden (açıklık başına 16 parça) ölçülüyordu. Değer 1,8 cm ile 1,08 m kısa çıkıyordu; bu, CLAUDE.md §23.3'e de aykırıydı. Artık her açıklığın kübik Bézier'inden, uyarlamalı Gauss–Legendre tümleviyle (`geom::quadrature`) 10⁻¹⁴ göreli doğrulukla hesaplanır (`spline_length`). Bu yanlış değeri tutan iki dondurulmuş durum dosyasının sekiz beklentisi (`calls-p5-entities.json`'da bir, `store-v1.json`'da yedi) başvurunun değeriyle güncellendi.
   - **Bulgu 2, çok basık elips:** oranı 0,01 olan elipsin uzunluğu Simpson kuralıyla 1,3·10⁻⁸ m sapıyordu. Artık aynı uyarlamalı tümlevle sekiz parçada hesaplanır.
   - **Sapmasız olanlar:** öbür bütün ölçüler sınır içindedir. Çizgi, yay, yaylı çoklu çizgi, delikli ve çok parçalı alanın alanı ve çevresi, çember, yarıçap ve çap ölçüsü, uzaklık, semt, açı, 3B uzunluk ve on ölçü türünün değeri.
3. **Eğrinin hesaplardaki yaklaşığı:**
   - **Sorun:** kenet ve kesişimler (`ops::edges`), Patlat (`ops::explode`) ve kapalı eğrinin alan işlemleri (`ops::areas`) eğriyi hâlâ açıklık başına 16 ya da 32 kirişle temsil eder. Eğri üstünde alınan nokta gerçek eğriden santimetre düzeyinde sapabilir.
   - **Hedef:** yaklaşık temsil açık bir toleransa bağlanır, örneğin kirişin eğriden en büyük uzaklığı 0,1 mm. Ya da kesin eğri kullanılır. Bağımsız başvuruyla sınanır.

## Sonuçlar

- Aynı büyüklük her yerde aynı yazılır; yarımda biten değer yöne ya da yola göre değişmez.
- Ekrandaki ve çizimdeki her sayının yuvarlaması açık, sürümlü bir kuraldır (CLAUDE.md §23.2).
- Ölçülerin doğruluğu bağımsız bir başvuruyla kanıtlanır, iddia edilmez.

## Doğrulama

- **Gösterim:** `numeric_display.py --check`; çekirdek ve web aynı durumları geçer; WASM üzerinden 10⁵ rastgele değerde iki uygulama aynı yazar.
- **Ölçüler:** `measure_cases.py --check`; çekirdek ve web WASM'ı sınırlar içinde ve aynı yazıyla.
- **Arayüz:** yarımda biten bir yarıçapın ölçüsü sekiz yönde aynı yazılır; resimler iki platformda.
