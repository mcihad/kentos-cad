# 0943 — Alternatiflerin karşılaştırması: nerede ayrılıyorlar, ne kadar tutuyorlar

Tarih: 2026-11-04. Durum: kabul (Y37.1 çekirdek, Y37.2 komut, Y37.3 pencere). Kapsam ROAD-ALIGN-12 ve ROAD-VOL-15'in kalanı.

## Sorun

Bir yolun birkaç alternatifi çizilir: kurp yarıçapı büyütülmüş, PI kaydırılmış ya da kırmızı kot yükseltilmiş. Karar
için iki soru sorulur:

- Alternatif tabandan **nerede** ayrılıyor?
- Hacmi, uzunluğu ve onaylı hâli **ne kadar** farklı?

Bugün kübajlar tek tek açılıyor; iki aksın nerede ayrıldığı çizimde gözle aranıyor.

ROAD-ALIGN-12 şunu ister: "Alternative alignment ve compare için parent/source digest ve değişen kilometre aralığı
korunsun." ROAD-VOL-15'in kalanı da alternatif miktarların kaynak özetiyle karşılaştırılmasıdır.

## Dayanak

- **Araştırma (4 Kasım 2026):** OpenRoads ve Civil 3D'de iki alternatif aksı yan yana karşılaştıran yerleşik bir araç
  bulunamadı. Kullanıcılar kesit raporlarını tabloya aktarıp karşılaştırıyor. Trimble QuantM'in güzergâh
  eniyilemesinde bir "Compare Alignments" ekranı var.
- **Uzaklığın Lipschitz sınırı:** bir noktanın bir kümeye uzaklığı 1-Lipschitz'tir. Alternatifin üstünde birim hızla
  ilerleyen bir noktanın tabana uzaklığı da metre başına en çok 1 m değişir.

## Karar

### §1 Planda ayrılma (`kentos_road::alternatives::plan`)

- d(s), alternatifin s istasyonundaki noktasının taban aksına uzaklığıdır: en yakın ayağın ofseti; ayak yoksa yakın
  ucun uzaklığı.
- **Ayrılan aralık:** d(s) > t olan aralıktır (t tolerans).
- **Adım:** d, s'den itibaren |d − t| metre içinde t'ye ulaşamaz. Her adım bu kadar atılır, en az 1 cm. Tarafı
  değiştiren adım ikiye bölünerek 1e-9 m'ye inceltilir.
- **Garanti:** 1 cm'den kısa bir ayrılma kaçabilir; daha uzunu kaçmaz. Adım uzaklıkla büyüdüğü için uzak kısımlar
  birkaç adımda geçilir.
- **Aralığın uçları** tabanın istasyonlarıyla da verilir. Tabanın ucunun ötesindeki noktanın istasyonu yoktur.
- **En büyük ayrılma:** yarım metrede bir örneklenir, en iyi örneğin çevresinde altın oranla inceltilir.
- **Ters yönde çizilmiş aynı aks** ayrılmaz; ölçülen geometrinin kendisidir.

### §2 Kotta ayrılma (`kentos_road::alternatives::height`)

- **Nerede karşılaştırılır:** iki aksın t içinde birlikte gittiği yerlerde. Δz, alternatifin kotu ile tabanın en yakın
  istasyonundaki kotu arasındaki farktır.
- **Ayrılan aralık:** |Δz| > h olan aralıktır.
- **Lipschitz sabiti:**
  - Alternatifin en büyük eğimi artı tabanınki.
  - Tabanın istasyonu alternatifinkinden en çok 1/(1 − κ·t) kat hızlı ilerler (κ tabanın en büyük eğriliği). Tabanın
    eğimi bu oranla büyütülür.
  - Eğimler bir parçanın uçları arasında monotondur (parabol ve daire); en büyüğü uçlardadır.
- **Planda ayrılan yerde** kot karşılaştırılmaz; planın ayrılması zaten söyler.

### §3 Komut: `road.volumes.compare` (Y37.2)

Komut yalnız okur ve şemayı değiştirmez.

- **Girdi:** iki ile altı arasında kübaj (ilki taban), isteğe bağlı `tolerance` ve `height` (m).
- **Öntanımlı toleranslar 1 cm'dir.** Bunlar mühendislik kararı değildir; yalnız "iki çizgi aynı çizgi" eşiğidir.
- **Her alternatifin raporu:**
  - koridor, aks, boykesit ve yüzey, revizyonlarıyla;
  - yüzeyin **kaynak özeti** (digest);
  - aksın uzunluğu;
  - kazı ve dolgu (doğal ve düzeltilmiş);
  - onaylıysa onay günü ve onaydan bu yana değişim.
- **Tabana göre:** uzunluk ve hacim farkı, planda ve kotta ayrılan km aralıkları (iki aksın km'siyle) ve en büyük
  ayrılma.
- **En büyük ayrılmanın yeri sözleşmede yoktur.** İki boykesitin paralel gittiği bir aralıkta her istasyon en büyüktür;
  yer tek değildir. İlk yazımda iki uygulama aynı değeri 393,657 ve 394,156 m'de buldu.
- **Uyarı:** `volume_compare_broken`. Hesaplanamayan kübaj rakamsız listelenir; karşılaştırma durmaz.
- **Retler:** `invalid_volume_compare`, `unknown_volume`.

**Ortak durumlar (20):** Python başvurusu (`road_volume_compare_command_cases.py`) çizime ana yolun iki alternatifini
ekler:

- "Geniş kurp": kurbun yarıçapı 150 → 220 m, kendi aksı ve boykesitiyle;
- "Yüksek kot": aynı aks, tepe PVI'ı 1,5 m yukarıda.

Ayrılmaları §1–§2'nin başvurusuyla hesaplar. Rust ve Python tipli katmanı aynı sonucu verir.

**Ölçüm:**

| Alternatif | Planda ayrılan | Kotta ayrılan | Δ kazı | Δ dolgu |
|---|---|---|---|---|
| Geniş kurp | 1+172,38 – 1+376,84 | 1+376,84'ten sona üç aralık | +654 m³ | +112 m³ |
| Yüksek kot | — | 1+143,47 – 1+543,05 | −1.779 m³ | +2.645 m³ |

- **Geniş kurbun kotu kurptan sonra da ayrılıyor.** Aks 3,35 m kısaldığı için boykesitin aynı km'leri artık başka
  yerlere düşüyor. Alternatif kurulurken kırmızı kotun km'ye bağlı olduğu unutulursa bu fark gözden kaçar;
  karşılaştırma onu açıkça gösteriyor.
- **Başvurunun kör noktası:** yan yol ana yolu kesiyor. Kesişmede iki aks yalnız birkaç santimetre boyunca 1 cm içinde
  kalıyor. KentOS'un yürüyüşü bu aralığı buldu; başvurunun 0,25 m'lik örneklemesi kaçırdı. Başvuru, iki örnek
  arasında bir dalış saklanabilecekse (|m_a| + |m_b| < aralık) aralığı 1 mm'ye kadar ikiye bölecek biçimde
  düzeltildi. Kesişme noktasında iki yolun kotları 2,83 m farklı; KentOS bunu 2 cm'lik bir kot ayrılması olarak da
  verir. Başvuru bunu ancak ayrı yerlerde de bir pay ölçüsü (birleşmeye uzaklık, −(d − t)) kullanınca buldu.
  Çekirdek durumlarına kapalı biçimli bir "kesişen iki aks" durumu eklendi: 1 cm toleransta 20 cm birlikte.

### §4 Pencere: Kübaj → Alternatifler (Y37.3)

- **Form:** "Alternatifler" bölümünde öbür kaydedilmiş kübajların her biri için bir "Karşılaştır" anahtarı var. Seçili
  kübaj tabandır; kaydedilmemişse karşılaştırma yapılmaz ve bunun nedeni yazılır.
- **Gösterim:**
  - Bir cümle her alternatifin ayrıldığı km aralıklarını ve hacim farkını söyler.
  - "Karşılaştırma" tablosu uzunlukları, hacimleri, farkları, onayı ve ayrılan aralıkları listeler.
  - Resim bir **ayrılma diyagramıdır**: istasyon boyunca üstte tabana plandaki uzaklık, altta iki aksın birlikte
    gittiği yerde kot farkı, her alternatif kendi renginde, 1 cm toleransı kesikli.
  - İlk sürüm aksları planda çiziyordu ve hiçbir şey göstermiyordu: 600 m'lik yolda 6 m'lik ayrılma bir pikseldir.

Resimler `.run/gui-evidence/yol-y37/` altındadır.

## Ortak durumlar ve ölçüm (çekirdek)

`scripts/fixtures/road_alternatives_cases.py`, KentOS kodu olmadan ve **KentOS'un yönteminden farklı** bir yoldan
çalışır:

1. alternatifi 0,25 m'de bir örnekler;
2. uzaklığı tabanın 5 cm'lik çoklu çizgisine numpy ile ölçer;
3. tarafın değiştiği örnekler arasını, aks başvurusunun 40 basamaklı yerleri ve altın oranla bulunan ayakla kesin
   uzaklık üzerinden ikiye bölerek inceltir.

Toleransa 1e-4 m'den yakın örneklerde taraf kesin uzaklıkla seçilir. İlk yazımda kaba çoklu çizgi bir sınırı yutmuştu;
kapalı biçim denetimi yakaladı.

16 durum var. Kapalı biçimle sınananlar:

- θ açısıyla ayrılan aliyman t / sin θ'da ayrılır;
- tabanın ucunu geçen aks ucun t ötesinde ayrılır;
- paralel 5 m'lik aks t = 1'de baştan sona ayrılır, t = 6'da hiç ayrılmaz;
- ortası 1 m yükseltilmiş kırmızı kot, h = 5 cm'de 25–975 m arasında ayrılır;
- kesişen iki aks (eğim 0,1) 1 cm toleransta yalnız 2t / sin θ = 20 cm birlikte gider; bu, başvurunun örnekleme
  adımından kısadır;
- aynı aks, ters yönde çizilmiş aynı aks ve aynı kotlar hiç ayrılmaz.

Ötekiler:

- yarıçap 300 → 400 (291,141–505,347 m, en çok 2,659 m);
- klotoid 50 → 80;
- PI'ın 15 m kaydırılması;
- ortada sapıp geri dönen aks;
- ötelenmiş koordinatlar;
- düşey kurp boyu 200 → 300;
- planı ayrılan aksın kotu.

Rust 16 durumda da aralık uçlarını, taban istasyonlarını ve en büyük ayrılmayı 1e-6 m içinde aynı bulur
(`crates/shared/road/tests/all/alternatives.rs`).
