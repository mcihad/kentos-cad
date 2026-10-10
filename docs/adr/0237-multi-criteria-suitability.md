# ADR 0237: Çok ölçütlü uygunluk

- **Durum:** kabul edildi (2026-10-10). Kapsam sahibin sözleridir (10 Ekim): “Önce pull yap daha sonra da GIS-37, 38, 42, 43
  maddelerini tamamla”, “Yeni branch içinde yap bunları”; önceki kararları: “yüksek performans ilk önceliğimiz”, simgeler sorulmadan
  seçilir, iki platform. Madde tek parçada biter; dal `gis-37-38-42-43`.
- **Bağlam belgesi:** TODOS.md `GIS-37` (ilgili `CITY-15`), ADR 0231 (raster çözümleme altyapısı), ADR 0233 (operasyon işi, rasterlerin
  tek ızgarada okunması, Yeniden sınıflandır'ın tablosu, Bölgesel istatistik'in alanları), ADR 0084 ve ADR 0200 (İşlemler'in
  parametreleri ve tablo çıktısı).

## Bağlam

Yer seçimi (konut, sanayi, düzenli depolama, enerji), afet duyarlılığı (heyelan, sel) ve arazi sentezi çok ölçütlü uygunluk
çözümlemesidir: her ölçüt (eğim, yola uzaklık, jeoloji, arazi örtüsü …) ortak bir ölçeğe getirilir, ağırlıklarıyla birleştirilir; sonuç
bilinen olaylarla doğrulanır. Netcad bunu Analist › İleri Konumsal Analizler'de (Weighted Overlay, Fuzzy Logic, mAHP, ROC) ve PLANET'in
Arazi Sentezi ile Yerleşilebilirlik Sentezi'nde verir. ArcGIS Pro'nun Spatial Analyst'inde Weighted Overlay, Weighted Sum, Fuzzy
Membership, Fuzzy Overlay ve Assign Weights By Pairwise Comparison, QGIS'te Fuzzify raster araçları vardır. KentOS'ta bugün yalnız Raster
hesaplayıcı ve Yeniden sınıflandır vardır (ADR 0233): ağırlıklar, bulanık mantık, ikili karşılaştırma ve doğrulama yoktur.

Araştırmada bulunanlar (10 Ekim):
- **Ağırlıklı çakıştırma** (ArcGIS Weighted Overlay): her raster bir tabloyla ortak ölçeğe (1–9, 1–5 …) sınıflanır, her rasterin etkisi
  yüzdedir ve toplamı 100'dür; sonuç ölçek değerlerinin etkilerle ağırlıklı toplamıdır, tam sayıya yuvarlanır. “Restricted” (kısıtlı)
  sınıf sonucu ölçeğin alt ucunun bir eksiği yapar, NoData sonucu NoData.
- **Ağırlıklı toplam** (Weighted Sum): rasterler ağırlıklarıyla çarpılıp toplanır, ondalık sonuç, yeniden ölçekleme yok.
- **Bulanık üyelik** (Fuzzy Membership; Zadeh 1965): değer 0–1 üyeliğe çevrilir. ArcGIS'in işlevleri Gaussian e^(−f₂(x−f₁)²), Large
  1/(1+(x/f₁)^(−f₂)), Small 1/(1+(x/f₁)^f₂), Near 1/(1+f₂(x−f₁)²) ve Linear'dır; QGIS'in Fuzzify raster'ı doğrusal ve üslü (power)
  üyelik verir.
- **Bulanık çakıştırma** (Fuzzy Overlay; Bonham-Carter 1994): And (en küçük), Or (en büyük), Product (çarpım), Sum (cebirsel toplam
  1 − Π(1 − μ)), Gamma (Sum^γ · Product^(1−γ)).
- **İkili karşılaştırma** (AHP; Saaty 1980, *The Analytic Hierarchy Process*): ölçütler ikişer ikişer 1–9 ölçeğiyle karşılaştırılır;
  ağırlıklar karşılaştırma matrisinin baş özvektörüdür, tutarlılık oranı CR = CI / RI (CI = (λ_en büyük − n)/(n − 1), RI Saaty'nin
  rastgele indeksi), CR > 0,10 tutarsızdır. Netcad'in mAHP'si (Nefeslioglu ve arkadaşları 2013) karşılaştırma sayısını azaltan bir
  değişiktir; klasik AHP seçildi (§7).
- **ROC ile doğrulama** (heyelan duyarlılık haritalarının başarı eğrisi; Chung ve Fabbri 2003): eşik büyükten küçüğe inerken doğru
  pozitif oranı yanlış pozitif oranına (ya da alan oranına) karşı; eğrinin altındaki alan AUC, Mann–Whitney'in U'sunun oranıdır.

## Karar

### 1. Kapsam

İşlemler'in yeni Uygunluk analizi kategorisinde altı araç:

| Araç | Kimlik | Çıktı |
|---|---|---|
| Bulanık üyelik | `suitability.fuzzyMembership` | raster: 0–1 üyelik |
| Bulanık çakıştırma | `suitability.fuzzyOverlay` | raster: 0–1 |
| Ağırlıklı toplam | `suitability.weightedSum` | raster: Σ ağırlık · değer |
| Ağırlıklı çakıştırma | `suitability.weightedOverlay` | raster: ölçeğin tam sayı sınıfı |
| İkili karşılaştırma (AHP) | `suitability.pairwise` | tablo: ağırlıklar ve tutarlılık; isteğe bağlı ağırlıklı toplam rasteri |
| ROC ile doğrulama | `suitability.roc` | tablo: eğrinin noktaları; özet: AUC ve en iyi eşik |

İşlemler'e iki yeni parametre türü gelir (§9): rasterlere birer değer (ağırlıklar, etkiler, sınıflar) ve raster çiftleri (ikili
karşılaştırmalar); ikisi de girdinin rasterlerinden kurulan tıklanır tablolardır.

**Kapsam dışı:** Netcad'in mAHP'si, bulanık AHP, MS Large ve MS Small (rasterin ortalamasıyla), vektör katmanlarla doğrudan çakıştırma
(PLANET'in Arazi Sentezi; alanlar önce Rasterleştir ile rastere çevrilir, ADR 0234), duyarlılık çözümlemesi (ağırlıkların oynatılması),
ROC'un grafiği (`GIS-39`'un grafikleri), Afet analizi çalışma modu.

### 2. Ortak kurallar

- **Değer, yer, hücre:** ADR 0233 §2'deki gibi: bandın örneği; NaN, nodata ve alfası 0 olan piksel değersizdir; hücre (i, j)'nin merkezi
  (i + ½, j + ½). Bant parametresi bütün girdilerde aynı bandı okur (varsayılan 1).
- **Sıra:** girdinin rasterleri ADR 0233 §2'nin sırasıyladır (Katmanlar panelinde üstten aşağı; aynı katmanda sonra eklenen önce);
  adları Raster hesaplayıcı'nınkidir (katmanın adı; aynı katmandaki ikinci raster “Ad (2)” …).
- **Çok rasterli araçların ızgarası** (Bulanık çakıştırma, Ağırlıklı toplam, Ağırlıklı çakıştırma, İkili karşılaştırma'nın rasteri):
  - Taban, hücresinin alanı (|a·d − b·c|) en küçük girdidir; eşitse sırada önce gelen.
  - Her girdinin dört köşesi tabanın hücre uzayına (u, v) alınır; girdinin kutusu [u_en küçük, u_en büyük] × [v_en küçük, v_en büyük].
    Kutuların kesişimi [U₀, U₁] × [V₀, V₁].
  - Sonucun sütunları i₀ = ⌈U₀ − ½ − 10⁻⁹⌉ … i₁ = ⌊U₁ − ½ + 10⁻⁹⌋ (merkezi kesişimde olan sütunlar), satırları aynı kuralla.
    Boşsa ret: “Rasterler örtüşmüyor: ortak alanlarında hiç hücre merkezi yok.”
  - Her girdi sonucun hücre merkezinde en yakın hücresiyle okunur (ADR 0233 §2). Girdinin dışına düşen merkez değersizdir.
- **Değersiz hücre:** çok rasterli araçlarda girdilerden birinin değersiz olduğu hücre sonuçta değersizdir; Bulanık üyelik'te girdinin
  değersiz hücresi değersiz kalır.
- **Sonuçlar:** ADR 0233 §2'nin dosyası ve nesnesi: karolu, Deflate'li, önizleme katlı GeoTIFF; ilk girdinin yanında (adı ek alarak) ya da
  yazılan yerde; Çizime ekle açıksa ilk girdinin katmanının hemen üstündeki yeni katmanda. Ondalık sonuçlarda Sonuç türü 32 bit
  (varsayılan) ya da 64 bit, değersiz NaN.
- **Sayılar:** her toplam ve çarpım float64'te, girdilerin sırasıyla, burada yazılan işlem sırasıyla yapılır; 32 bit sonuç float64
  değerin en yakın float32'sidir (eşitlikte çift). `exp` ve `pow` `libm`'indir (ADR 0008).

### 3. Bulanık üyelik

- **Girdi:** bir raster ve bandı; İşlev ve onun ayarları; Sonuç türü.
- **İşlevler** (x hücrenin değeri, μ üyelik):
  - **Doğrusal** (Alt değer a, Üst değer b; a ≠ b):
    - a < b ise x ≤ a'da 0, x ≥ b'de 1, arada (x − a) / (b − a);
    - a > b ise (azalan) x ≤ b'de 1, x ≥ a'da 0, arada (a − x) / (a − b).
  - **Üslü** (a, b, Üs e > 0): Doğrusal'ın μ'sü üssü e'ye yükseltilir: pow(μ_doğrusal, e). Arada x'in alt uca uzaklığı e > 1'de yavaş,
    e < 1'de hızlı yükselir (QGIS'in power üyeliği).
  - **Gauss** (Orta nokta m, Yayılım s > 0): μ = exp(−(s · (d · d))), d = x − m.
  - **Büyük** (m > 0, Diklik s > 0): x ≤ 0'da 0; yoksa μ = 1 / (1 + pow(x / m, −s)).
  - **Küçük** (m > 0, s > 0): x ≤ 0'da 1; yoksa μ = 1 / (1 + pow(x / m, s)).
  - **Yakın** (m, Yayılım s > 0): μ = 1 / (1 + s · (d · d)), d = x − m.
- **Varsayılanlar:** İşlev Doğrusal; a 0, b 100; e 2; m 1; Gauss ve Yakın'ın Yayılım'ı 0,1; Büyük ve Küçük'ün Diklik'i 5 (ArcGIS'in
  varsayılanları). Sayılar sonlu olmalı; Yayılım, Diklik ve Üs 0'dan büyük, 10⁶'dan küçük; Büyük ve Küçük'te m > 0; Doğrusal ve
  Üslü'de a ≠ b; değilse ret, nedeniyle.
- **Sonuç:** Spektral rampa, el ile gerdirme 0–1.

### 4. Bulanık çakıştırma

- **Girdi:** en az iki raster (üyelikler) ve bandı; İşleç; Gamma (yalnız Gamma'da, 0–1, varsayılan 0,9); Sonuç türü.
- **Üyelik:** 0 ≤ μ ≤ 1 olmalıdır; bir girdinin değeri bu aralığın dışındaysa hücre değersizdir ve sayılır (özette uyarı).
- **İşleçler** (μ₁ … μₙ girdilerin sırasıyla):
  - **Ve:** en küçük μ.
  - **Veya:** en büyük μ.
  - **Çarpım:** p = 1; her k için p = p · μₖ.
  - **Toplam:** q = 1; her k için q = q · (1 − μₖ); sonuç 1 − q.
  - **Gamma** (γ): pow(1 − q, γ) · pow(p, 1 − γ) (p ve q yukarıdaki gibi).
- **Sonuç:** Spektral, el ile gerdirme 0–1.

### 5. Ağırlıklı toplam

- **Girdi:** en az bir raster ve bandı; Ağırlıklar (her rastere bir sayı, §9; yazılmayanın ağırlığı 1; |w| ≤ 10⁹); Sonuç türü.
- **Hesap:** v = 0; her k için v = v + wₖ · xₖ.
- **Sonuç:** Spektral, en küçükten en büyüğe gerdirme.

### 6. Ağırlıklı çakıştırma

- **Girdi:**
  - en az iki raster ve bandı;
  - Ölçek: alt uç L (varsayılan 1) ve üst uç U (varsayılan 9), tam sayılar, −10⁶ ≤ L < U ≤ 10⁶;
  - Etki (%): her rastere bir sayı (§9), 0–100, en çok dört ondalık, toplamları tam 100;
  - Sınıflar: her rastere isteğe bağlı bir tablo (§9);
  - Sınırlar: “alt < değer ≤ üst” (varsayılan) ya da “alt ≤ değer < üst” (bütün tablolara).
- **Etkiler:** Wₖ = round(wₖ · 10⁴) (float64). |wₖ · 10⁴ − Wₖ| > 10⁻⁶ ise ret (en çok dört ondalık). Etkisi yazılmayan raster ret
  (“Eğim: etkisi yazılmadı.”). ΣWₖ ≠ 1 000 000 ise ret: “Etkilerin toplamı 100 olmalı; şimdi 90.”
- **Sınıf tablosu:** Yeniden sınıflandır'ın dili (ADR 0233 §4: “alt üst yeni”, “değer yeni”, “boş yeni”; `*` açık uç; kurallar `;`
  ya da satırla); yeni değer bir tam sayı, `boş` ya da `kısıt` (kısıtlı). Yeni değeri L–U'nun dışında ya da tam sayı olmayan kural ret:
  “Eğim: 2. kuralda yeni değer 12 ölçeğin dışında (1–9).”
- **Hücrenin ölçek değeri sₖ:**
  - Tablosu varsa ilk tutan kuralın yeni değeri; hiçbir kural tutmazsa hücre değersizdir (“kuralı tutmayan” sayılır).
  - Tablosu yoksa hücrenin kendi değeri; tam sayı değilse ya da L–U'nun dışındaysa hücre değersizdir (“ölçeğin dışında” sayılır).
- **Sonuç** (bu öncelikle):
  1. girdilerden biri değersizse (değeri yok, kuralı `boş`, kuralı tutmuyor ya da ölçeğin dışında) değersiz;
  2. birinin kuralı `kısıt` ise L − 1 (kısıtlı; sayılır);
  3. yoksa S = Σ sₖ · Wₖ (tam sayılarla, kesin) ve sonuç S / 10⁶'nın sıfırdan uzağa yuvarlanmış tam sayısı:
     S ≥ 0'da ⌊(S + 500 000) / 10⁶⌋, S < 0'da −⌊(−S + 500 000) / 10⁶⌋. Sonuç L ile U arasındadır.
- **Sonuç türü:** tam sayı 32 bit, değersiz −2 147 483 648. Görünüş Spektral, el ile gerdirme L − 1 … U, en yakın örnekleme.

### 7. İkili karşılaştırma (AHP)

- **Girdi:** ölçütler: 2–15 raster (sırasıyla); Karşılaştırmalar (§9: her çifte bir değer); Ağırlıklı toplamı yaz (varsayılan açık) ve
  açıkken bant, Sonuç türü, çıktı.
- **Matris** A (n × n): aᵢᵢ = 1. i < j çiftinin değeri v ise (§9): v ≥ 1'de aᵢⱼ = v ve aⱼᵢ = 1 / v, v ≤ −2'de aᵢⱼ = 1 / |v| ve
  aⱼᵢ = |v| (tam sayı bir yanda, tersi float64'te öbür yanda). Yazılmayan çiftin değeri 1'dir (eşit). Değer
  {−9, …, −2, 1, 2, …, 9} dışındaysa ret: “Eğim — Yol: karşılaştırma 1, 2 … 9 ya da −2 … −9 olmalı.”
- **Ağırlıklar:** A'nın baş özvektörü, toplamı 1'e bölünmüş. Kuvvet yöntemiyle: w⁰ᵢ = 1/n; her adımda yᵢ = Σⱼ aᵢⱼ wⱼ (j sırasıyla),
  s = Σᵢ yᵢ (i sırasıyla), w'ᵢ = yᵢ / s; maxᵢ |w'ᵢ − wᵢ| ≤ 10⁻¹⁵ olunca ya da 10 000 adımda durulur. A artı değerli olduğundan yöntem
  yakınsar (Perron–Frobenius).
- **Tutarlılık:** λ = Σᵢ (A·w)ᵢ (w'nin toplamı 1); CI = (λ − n)/(n − 1); RI Saaty'nin tablosu (n = 3 … 15: 0,58; 0,90; 1,12; 1,24;
  1,32; 1,41; 1,45; 1,49; 1,51; 1,48; 1,56; 1,57; 1,59); CR = CI / RI. n = 2'de CI = CR = 0. CR > 0,10 ise uyarı: “Karşılaştırmalar
  tutarsız (CR 0,14 > 0,10); en çelişkili çiftleri yeniden gözden geçirin.”
- **Tablo:** Ölçüt, Ağırlık (6 ondalık), Yüzde (2 ondalık); özet λ, CI, RI ve CR'yi söyler.
- **Rasteri:** Ağırlıklı toplamı yaz açıksa §5'in hesabı bu ağırlıklarla (dosyanın eki `-ahp`). Kapalıyken rasterlerin yalnız başlığı
  okunur.
- **Başvuru:** ağırlıklar mpmath'le 50 basamakta hesaplanan özvektörle 10⁻¹²'den, λ, CI ve CR 10⁻¹⁰'dan yakın olmalıdır (yinelemeli
  çözümün float64 yuvarlaması).

### 8. ROC ile doğrulama

- **Girdi:** bir raster (duyarlılık ya da uygunluk) ve bandı; Varlık (noktalar ya da alanlar: heyelanlar, olaylar); Karşılaştırma:
  Bütün hücreler (varsayılan) ya da Yokluk nesneleri (noktalar ya da alanlar); Yüksek değer daha olası (varsayılan açık; kapalıyken
  değerlerin işareti çevrilir).
- **Örnekler hücrelerdir:** nokta içinde olduğu hücreyi verir (i ≤ u < i + 1, j ≤ v < j + 1; çok noktalı nesnenin her noktası), alan
  merkezini içine alan hücreleri (ADR 0233 §6'nın kuralı). Bir hücre birden çok nesneden gelse de bir kez sayılır. Rasterin dışına ya da
  değersiz hücreye düşen örnek atlanır, söylenir.
  - Varlık hücreleri P (sayısı P), karşılaştırma hücreleri N: Bütün hücreler'de rasterin bütün değerli hücreleri (varlık hücreleri de;
    başarı eğrisi), Yokluk'ta yokluk nesnelerinin hücreleri. Bir hücre hem varlık hem yokluksa ikisinde de sayılır, söylenir.
  - P ya da N boşsa ret.
- **AUC:** G = #{(p, q) : sₚ > s_q}, E = #{(p, q) : sₚ = s_q} (p varlık, q karşılaştırma hücresi); AUC = (2G + E) / (2PN), tam
  sayılarla sayılıp sonda bölünür (Mann–Whitney; eşitler yarım).
- **Eğri:** eşikler varlık değerlerinin farklı olanlarıdır, büyükten küçüğe: t₁ > t₂ > … > tₘ. m > 200 ise k = 1 … 200 için
  t_{⌈k·m/200⌉}. Her eşikte DP = #{p : sₚ ≥ t}, YP = #{q : s_q ≥ t}; oranlar DP / P ve YP / N. Tablo: Eşik, Doğru pozitif, Doğru pozitif
  oranı (%), Yanlış pozitif (Bütün hücreler'de Hücre), Yanlış pozitif oranı (%) (Bütün hücreler'de Alan oranı (%)).
- **En iyi eşik** (Youden): tablonun eşiklerinden DP/P − YP/N'si en büyük olan (float64); eşitse büyük eşik. Özet: “AUC 0,8732;
  en iyi eşik 0,62 (doğru pozitif %84,0, alan %21,5).”
- **Hesap:** iki geçiş. Birincide varlık (ve yokluk) hücrelerinin değerleri okunur; Bütün hücreler'de ikinci geçişte her değerli
  hücrenin değeri sıralı varlık değerlerinde ikili aramayla sayılır (G, E ve eşiklerin YP'leri): bellek yalnız varlık kadardır.

### 9. İşlemler'in iki yeni parametre türü

- **Rasterlere değer** (`rasterValues`): bir özellik parametresinin (`of`) rasterlerine birer değer; hücre `number` (en küçük, en büyük)
  ya da `text`; yer tutucu.
  - **Değer:** ad → sayı ya da metin nesnesi (`{ "Eğim": 40, "Jeoloji": 35 }`); varsayılan boş. Ad Raster hesaplayıcı'nın adıdır (§2).
  - **Pencere:** girdinin rasterleri sırasıyla birer satırdır: adı ve alanı. Değerde olup girdide olmayan adlar ardından, soluk ve × ile
    silinir. Model tasarımcısında girdi çalışınca belli oluyorsa satırlar değerin adlarıdır; Ad ekle alanı yeni ad açar.
  - **Denetim:** sayı sonlu ve sınırlarda olmalı; metin metin. Yazılmayan satırın anlamı aracındır (§5, §6).
- **Raster çiftleri** (`rasterPairs`): bir özellik parametresinin rasterlerinin her çiftine bir karşılaştırma.
  - **Değer:** `[a, b, v]` üçlüleri; v 9 … 2: a b'den v kat önemli, 1 eşit, −2 … −9: b a'dan |v| kat önemli. (b, a, v) (a, b, −v)
    demektir (1 1 kalır); aynı çift iki kez yazılmışsa ilki geçer.
  - **Pencere:** girdinin rasterlerinin her i < j çifti bir satır: “a — b” ve 17 seçenekli liste: “a 9 kat” … “a 2 kat”, “Eşit”,
    “b 2 kat” … “b 9 kat”. Girdide olmayan çiftler soluk, × ile silinir.
- **İkisi de:** girdinin özeti rasterlerin adlarını sırasıyla taşır (`rasters`); model girdisi yapılamaz (seçim gibi); son değerler
  saklanır; pencere planı (`fixtures/processing/v1/dialog.json`) ikisini de taşır; web'in tanımlarından masaüstüne (`web_param`) gelir.

### 10. Arayüz

- **Araçlar** iki platformda aynı adlar, parametreler ve varsayılanlarla; **komutlar** `processing.run.<kimlik>`. **Şerit:** CBS'nin
  Raster sekmesinde Uygunluk analizi paneli. Sekme 1100 px'e sığsın diye İnterpolasyon ve Yoğunluk'un araçları (ADR 0232) tek panelde,
  İnterpolasyon'da: ikisi de noktalardan yüzey verir.
- **Parametreler:** rasterler `input`; Bulanık üyelik `function`, `low`, `high`, `exponent`, `midpoint`, `spread`, `steep`; Bulanık
  çakıştırma `op`, `gamma`; Ağırlıklı toplam `weights`; Ağırlıklı çakıştırma `low`, `high`, `influence`, `classes`, `bounds`; İkili
  karşılaştırma `comparisons`, `write`; ROC `presence`, `background`, `absence`, `higher`. Çıktı dosyası, Çizime ekle ve Çıktı katmanı
  ADR 0233'tekiler.
- **Dosya ekleri:** `-uyelik`, `-bulanik`, `-agirlikli`, `-cakistirma`, `-ahp`.
- **Takma adlar:** BULANIKUYELIK, FUZZYMEMBERSHIP; BULANIKCAKISTIR, FUZZYOVERLAY; AGIRLIKLITOPLAM, WEIGHTEDSUM; AGIRLIKLICAKISTIR,
  WEIGHTEDOVERLAY; AHP, IKILIKARSILASTIRMA; ROC, AUC.

### 11. Performans

- Hücre işlemleri ADR 0233'ün operasyon işindedir: 256 satırlık şeritler, blokları iş parçacıklarında çözülür, satırlar iş
  parçacıklarında hesaplanır; masaüstünde İşlemler'in iş parçacığında, web'de çözümleme işçisinde.
- Ağırlıklı çakıştırma'nın tabloları bir kez çözülür; hücrede yalnız kurallar denenir ve tam sayı toplanır.
- ROC'un ikinci geçişi bellek ayırmaz: hücre başına iki ikili arama (log₂ P).
- İkili karşılaştırma'nın hesabı n ≤ 15'te mikro saniyelerdir.
- **Bütçeler** (release, geliştirme makinesi; 4096 × 4096 Float32; web işçide, tek iş parçacığı):

| İş | Masaüstü | Web |
|---|---|---|
| Bulanık üyelik, Gauss | ≤ 0,6 s | ≤ 3 s |
| Bulanık çakıştırma, dört raster, Gamma | ≤ 1,5 s | ≤ 6 s |
| Ağırlıklı toplam, dört raster | ≤ 1,5 s | ≤ 6 s |
| Ağırlıklı çakıştırma, dört raster, sınıf tablolarıyla | ≤ 1,5 s | ≤ 6 s |
| İkili karşılaştırma, 15 ölçüt, yalnız tablo | ≤ 1 ms | |
| ROC, bütün hücreler, 10 000 varlık hücresi | ≤ 1,5 s | ≤ 6 s |

## Uygulama

- **Çekirdek** (`crates/shared/raster`):
  - `suitability`: Bulanık üyelik'in işlevleri (`Membership`), Bulanık çakıştırma'nın işleçleri (`FuzzyOp`), Ağırlıklı toplam'ın
    ağırlıkları (`sum_weights`), Ağırlıklı çakıştırma'nın etkileri, tabloları ve tam sayı toplamı (`Overlay`, `Scaled`), girdilerin ortak
    ızgarası (`common_grid`); `suitability::pairwise` (matris, kuvvet yöntemi, λ, CI, RI, CR); `suitability::roc` (örnek hücreler `Cells`,
    iki geçişli `RocWork`, `Roc`).
  - `ops`: altı aracın `OpsTool`'u; hücre işleri `Work::Suit` (satırlar iş parçacıklarında, notlar atomik sayaçlarla), ROC'un şeritleri
    `Work::Roc`, rastersiz AHP `Work::Weights`; `OpsFinished::Roc` ve `OpsFinished::Weights`, notların `suit`'i (`SuitNotes`).
  - `reclass::parse_with`: tablonun yeni değerinde çağıranın sözcükleri (`kısıt`).
- **WASM** (`crates/wasm/raster-wasm`): notlarda `suit` (sayaçlar ve AHP'nin ağırlıkları), `roc()`.
- **İşlemler, masaüstü** (`crates/native/processing`): `builtin/suitability/` (araçlar, çalıştırmalar, özetler, tablolar), Uygunluk
  analizi kategorisi; iki yeni parametre türü `ParamKind::RasterValues` ve `RasterPairs` (tür adları, varsayılanlar `{}` ve `[]`,
  `fits`, denetim iletileri, `web_param`, model girdisi olamaz), girdinin özetinde `rasters`, satırların kuralları `raster_rows`.
- **Masaüstü** (`apps/desktop`): pencerede iki tablo alanı (`processing/fields.rs`'in `raster_values` ve `raster_pairs`'i; model adımında
  Ad ekle'nin taslağı `Event::Draft`), ikisi de etiketin altında; tablo veren çalıştırmadan sonra form sonuna kayar (`result_shown`,
  `window::FORM`). Resimler `suitability_scenes.rs`, pencere testleri `processing/suitability_tests.rs`.
- **Web** (`apps/web`): `processing/builtin/suitability/` (`shared.ts`, `tools.ts`); parametre türleri `RasterValuesParam`,
  `RasterPairsParam` (`types.ts`, `parameters.ts`), özetin `rasters`'ı (`features.ts`); alan planı `fieldPlan.ts`'in
  `rasterValuesView`, `withRasterValue`, `withoutRaster`, `rasterPairsView`, `pairValue`, `withPair`, `pairLabel`'ı; pencere
  `paramFields.ts`'in `rasterValuesField` ve `rasterPairsField`'ı, `dialogPlan.ts`'in denetim biçimleri, `dialogTexts.ts`'in
  `rasters`'ı; tablo veren çalıştırmadan sonra tablo görünür olur (`ToolDialog.ts`); işçi protokolünde `roc`; CBS'nin Raster sekmesinde
  Uygunluk analizi paneli.
- **Simgeler** (sorulmadan seçildi): Bulanık üyelik `fuzzyMembership` (1'e yükselen S, üstte 1'in kesikli çizgisi), Bulanık çakıştırma
  `fuzzyOverlay` (iki üyeliğin dolgulu ortak kısmı), Ağırlıklı toplam `weightedSum` (Σ ve üç ağırlık çubuğu), Ağırlıklı çakıştırma
  `weightedOverlay` (yüzde işaretli katmanlar; kategorinin de simgesi), İkili karşılaştırma `pairwise` (eğik terazi), ROC ile doğrulama
  `rocCurve` (köşegenin üstündeki eğri ve altındaki alan).
- **Kararlar:**
  - Ağırlıklar, etkiler, sınıflar ve karşılaştırmalar yazılı bir dil yerine girdinin rasterlerinden kurulan tablolarla girilir (fare
    önce); değer raster adıyla saklanır, rasterlerin sırası değişse de doğru rastere gider.
  - Ağırlıklı çakıştırma tam sayılarla kesindir: dört ondalıklı etkiler 10⁻⁴ % birimiyle tam sayı olur, toplam taşmaz (n ≤ 64).
  - Çok rasterli araçların ızgarası girdilerin kesişimidir: biri değersizse hücre zaten değersizdir, birleşimin boş kenarları yazılmaz.
  - ROC'un ikinci geçişi bellek ayırmaz; AUC tam sayılarla sayılır.
  - Tablo veren her araçta (Özet istatistik, Histogram, Bölgesel istatistik … da) tablo çalıştırmadan sonra görünür olur; iki platformda.
  - CBS'nin Raster sekmesi on bir panelle 1100 px'e sığmıyordu (masaüstünün şerit testi): İnterpolasyon ve Yoğunluk tek panel oldu
    (`app/ribbon.ts`'in `POINT_SURFACES`'i).
- **Sonraya kalanlar:** §1'in kapsam dışı listesi; ROC'un grafiği `GIS-39` ile.

## Doğrulama

- **Bağımsız başvuru** `scripts/fixtures/suitability_cases.py --check` (KentOS kodu olmadan, ADR'den; `fixtures/suitability/v1/cases.json`,
  50 durum): Bulanık üyelik 11 (altı işlev, 64 bit, üç ret), Bulanık çakıştırma 8 (beş işleç, farklı ızgaralar ve kesişim, iki ret),
  Ağırlıklı toplam 6 (yazılmayan ağırlık, döndürülmüş girdi, örtüşmeyen ve sınırın dışındaki ağırlığın reddi), Ağırlıklı çakıştırma 9
  (tablosuz ve tablolu, iki sınır kuralı, eksi ölçekte yarımlar, beş ret), İkili karşılaştırma 8 (tutarlı ağırlıklı toplam, dört ölçüt,
  döngülü tutarsız, iki ölçüt, ters ve yinelenen çiftler, 15 ölçüt, iki ret), ROC 8 (noktalar, alanlar, yokluk, düşük değer, 200'den
  çok eşik, üç ret). exp ve pow mpmath'le 50 basamakta (kural `f32ulp`, 64 bitte `f64ulp`), özvektör mpmath'le 10⁻⁴⁵'e, AUC her çiftle
  kaba kuvvetle kesir olarak.
- **Çekirdek** `cargo test -p kentos-raster --test all suitability`: 50 durum bir ve üç iş parçacığıyla, rasterler GeoTIFF olarak, payın
  azalmadığı denetlenerek; birim testleri (`suitability`, `pairwise`, `roc`, `reclass`).
- **Web'in WASM'ı** (`raster.wasm.test.ts`): aynı 50 durum.
- **İşlemler'in ortak durumları** `scripts/fixtures/suitability_processing_cases.py --check` (`fixtures/processing/v1/suitability.json`,
  21 durum): iki platformda; yazılan raster başvurunun aynı adlı durumunun değerleri (`suitabilityOf`), özetler, iletiler, tablolar,
  katmanların yeri, geri alma, varsayılanlar.
- **Pencere:** `dialog.json` altı aracın formunu ve `rasters` sözlerini taşır; satırların kuralları iki platformda aynı beklentilerle
  (`rasterRows.test.ts`, `raster_rows` testleri); masaüstünde üç pencere testi (`processing::suitability_tests`).
- **Görsel inceleme** (iki platform, 1440×900 ve 1100×650, iki tema; `uyg-serit`, `uyg-uyelik`, `uyg-cakistirma`, `uyg-cakistirma-cizim`,
  `uyg-ahp`, `uyg-roc`). Bulunup düzeltilenler: masaüstünde tablolar etiketin yanındaki dar sütundaydı (etiketin altına alındı); web'de
  tablo satırın genişliğini almıyordu (`align-self: stretch`); tablo veren çalıştırmanın sonucu formun altında görünmüyordu (iki
  platformda görünür olur).
- **Süreler** (release, 13th Gen Intel i5-13500, 20 çekirdek; masaüstü 8 iş parçacığı, web işçide tek; 4096² Float32, p50; 10 Ekim):

| İş | Masaüstü | Bütçe | Web | Bütçe |
|---|---|---|---|---|
| Bulanık üyelik, Gauss | 0,32 s | 0,6 s | 1,33 s | 3 s |
| Bulanık çakıştırma, dört raster, Gamma | 1,02 s | 1,5 s | 3,70 s | 6 s |
| Ağırlıklı toplam, dört raster | 0,68 s | 1,5 s | 2,15 s | 6 s |
| Ağırlıklı çakıştırma, dört raster, sınıf tablolarıyla | 0,72 s | 1,5 s | 2,42 s | 6 s |
| Ağırlıklı çakıştırma, 1 iş parçacığı | 1,81 s | 3 s | | |
| İkili karşılaştırma, 15 ölçüt, yalnız tablo | 5,3 µs | 1 ms | | |
| ROC, bütün hücreler, 10 000 varlık hücresi | 0,25 s | 1,5 s | 1,16 s | 6 s |

- **Ölçülmeyenler:** mAHP ve bulanık AHP (kapsam dışı); GRASS'ta karşılığı olmayan araçlar için ikinci bir dış çapraz denetim yapılmadı
  (kurallar basit formüllerdir, başvuru onları mpmath ve kesirlerle hesaplar).
