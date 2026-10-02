# ADR 0159: Kenar eşleme

- **Durum:** kabul edildi (2026-10-02). `HYB-05`'in son işi. Ayrıntılar bu ADR'nin varsayılanlarıdır.
- **Tarih:** 2026-10-02
- **Bağlam belgesi:** TODOS.md `HYB-05`, ADR 0156 (Vektör oturtma), ADR 0158 (Kauçuk levha), ADR 0148 (Topolojik temizlik), ADR 0142 (köşe kotu), ADR 0047 (`cad.entities.edit`), ADR 0138 (NCZ paftaları), [araştırma kaydı](../research/2026-10-01-netcad-arcgis-qgis.md); ArcGIS Pro [Generate Edgematch Links](https://doc.esri.com/en/arcgis-pro/latest/tool-reference/editing/generate-edgematch-links.html) ve [Edgematch Features](https://doc.esri.com/en/arcgis-pro/latest/tool-reference/editing/edgematch-features.html), QGIS Snap geometries to layer, Netcad Birleştir (Ucuna Bağla).

## Bağlam

Paftalar ayrı ayrı sayısallaştırılınca ya da ayrı ölçülünce, pafta kenarında yol, bina ve parsel çizgileri komşu paftadaki devamıyla buluşmaz. Uçlar arasında santimetrelerden desimetrelere aralık kalır, bazen çizgiler üst üste biner. Vektör oturtma ve Kauçuk levha bütün paftayı taşır; kenardaki bu küçük, yerel farkı gidermez.

Topolojik temizlik (ADR 0148) toleransla uçları birleştirir, ama pafta kenarı için yetmez:

- tek bir küme içinde çalışır; iki paftanın çizgilerini ayırmaz;
- ucu en yakın köşeye götürür, oysa doğru eşleşme en yakın değil, çizginin devamıdır: iki çizgi kenardaki uçlarıyla birbirine bakmalıdır;
- yalnız var olan köşeye taşır; iki ucu ortada ya da pafta çerçevesinde buluşturamaz;
- desimetrelik aralıklarda en yakın köşe kuralı yanlış birleşmeler yapar.

ArcGIS bunu iki adımda yapar. Önce bağlar bulunur (Generate Edgematch Links): arama uzaklığındaki kopuk ama birbirinin devamı olan çizgiler, isteğe bağlı öznitelik eşleşmesiyle. Sonra çizgiler düzeltilir (Edgematch Features), üç yöntemle:

- ucu taşı;
- uca parça ekle;
- köşeleri ayarla: uçtaki kayma çizgi boyunca öbür uca doğru azalarak dağıtılır.

Komşu çizgiler de verilirse iki uç bağın ortasında buluşur, sınır verilirse sınırda buluşur. QGIS'te karşılığı Snap geometries to layer'dır (toleransla uç ve köşe yakalama). Netcad'de iki çizgi Birleştir'in Ucuna Bağla'sıyla tek tek bağlanır.

## Karar

### 1. Pencere ve kümeler

**Kenar eşleme** (`transform.edgematch`) Vektör oturtma'nın yanında bir Hesap penceresidir (Harita › Koordinatlar ▾). Takma adlar: KENARESLE, EDGEMATCH.

Pencere iki küme ister:

- **Kaynak:** düzeltilecek nesneler: seçili nesneler ya da bir katman.
- **Komşu:** komşu paftanın nesneleri: bir katman. Kaynakta da olan nesne komşudan çıkarılır. Böylece tek katmandaki iki pafta, birinin seçimiyle eşlenebilir.

Kümelerin kuralları:

- **Türler:** çizgi ve açık çoklu çizgi (yaylı kenarlarıyla). Yay, kapalı yol, alan, eğri ve öbür türler katılmaz; sayıları söylenir.
- **Kilit:** kilitli katmandaki nesne düzeltilmez. Komşu "Komşunun ucunda" buluşmada hiç değişmez, kilitli olabilir.
- **Gizli katman:** katılmaz.

### 2. Sınır

İsteğe bağlı olarak pafta çerçevesi verilir: çizimden seçilen (Sahneden seç) bir çizgi, çoklu çizgi ya da alan. NCZ'den gelen paftalar çerçeveleriyle gelir (ADR 0138).

Sınır verilirse:

- yalnız sınıra arama uzaklığından yakın uçlar bağ olabilir;
- "Sınırda" buluşma seçilebilir.

### 3. Uçlar

- **Uç:** çizginin iki ucundan biri ya da açık çoklu çizginin ilk ya da son köşesi.
- **Dışa yön u:** uçtaki kenarın uca doğru birim yönü. Kenar yaylıysa yayın uçtaki teğetidir.
- **Kavşak:** aynı kümenin başka bir çizgisinin, çoklu çizgisinin ya da alanının ucu ya da köşesi 1 µm içindeyse uç bir kavşaktır. Kavşak ucu eşlenmez, sayısı söylenir; yalnız biri taşınırsa kavşak bozulurdu.
- **Bağlı uç:** öbür kümenin bir ucu 1 µm içindeyse uç zaten bağlıdır, bağ gerekmez.

### 4. Bağlar

Bir kaynak ucu s (dışa yönü u) ile bir komşu ucu t (dışa yönü v) şu koşullarla **aday**dır:

1. 1 µm < |t − s| ≤ d. d arama uzaklığıdır: metre, sonlu ve sıfırdan büyük; ilk değeri 0,5 m.
2. **Devam:** iki uç birbirine bakar: u ile −v arasındaki açı θ ≤ α. α açı toleransıdır, ilk değeri 30°. Aralığın yönü ölçülmez: yana kaymış devam da, üst üste binen devam da adaydır.
3. **Eşleşme ölçütü** (isteğe bağlı) aynıdır:
   - **Katman adı:** katmanın ağaçtaki adı, grubunun yolu olmadan.
   - **Bir öznitelik:** iki nesnede de aynı değer. Değerler kırpılır ve Türkçe küçük harfle karşılaştırılır.
4. Sınır verilmişse s ve t'nin ikisi de sınıra en çok d uzaklıktadır.

Her adayın bir **puanı** vardır: θ/α + |t − s|/d. Puan 0 ile 2 arasındadır, küçüğü iyidir.

**Eşleme:** her uç en çok bir bağdadır.

- Adaylar puana göre sıralanır. Eşitlikte kaynak ucunun çizimdeki sırası, sonra komşununki gelir (nesnenin sırası, sonra ilk uç).
- Sırayla gezilir. İki ucu da boşta olan aday bağ olur.

Sonuç iki platformda aynıdır: puanlar ortak çekirdekte hesaplanır.

**Eşsiz uçlar:** bağ olmayan ve sınıra (sınır yoksa öbür kümenin bir ucuna) arama uzaklığından yakın kaynak uçları eşsizdir. Sayılır ve listelenir.

### 5. Buluşma yeri

| Buluşma | Ne olur |
|---|---|
| **Komşunun ucunda** (ilk değer) | Komşu yerinde kalır; kaynak ucu t'ye gider. |
| **Ortada** | İki uç da (s + t)/2'ye gider; komşu da aynı yöntemle düzeltilir. |
| **Sınırda** | İki uç da sınırın (s + t)/2'ye en yakın noktasına gider. Yalnız sınır verilmişse seçilebilir. |

### 6. Yöntem

Uç e, buluşma yeri m'ye gidecek olsun; Δ = m − e.

- **Ucu taşı** (ilk değer): yalnız uç taşınır. Ucun kenarı doğru kalır; yaylıysa kabarıklığını korur.
- **Parça ekle:** uç yerinde kalır, uçtan m'ye düz bir kenar eklenir.
  - Çizgi üç köşeli bir çoklu çizgi olur; kimliği ve verisi aynı kalır.
  - Aralık 1 µm'den kısaysa parça eklenmez, uç taşınır.
- **Köşeleri ayarla:** her köşe Δ·(1 − sᵢ/L) kadar taşınır.
  - sᵢ köşenin taşınan uca yol boyunca uzaklığıdır; L yolun uzunluğudur. Yaylı kenarlar yay uzunluğuyla sayılır.
  - Öbür uç yerinde kalır, kabarıklıklar korunur.
  - Taşınan uç buluşma yerine tam oturur (aritmetiğin yuvarlaması kalmaz).
  - Çizgide Ucu taşı ile aynıdır.

Bir nesnenin iki ucu da bağlıysa kaymalar toplanır. Köşe i, Δ₁·(1 − sᵢ/L) + Δ₂·(sᵢ/L) kadar taşınır (Δ₁ ilk ucun, Δ₂ son ucun kayması; Ucu taşı ve Parça ekle'de iki uç ayrı ayrı).

Sonuçta art arda iki köşe 1 µm içine gelecekse (çizgi sıfırlanır ya da yolda sıfır boylu kenar kalır), o nesnenin bağları yazılmaz ve söylenir.

### 7. Kot

- Taşınan köşeler kotlarını korur.
- Eklenen köşe ucun kotunu alır (kotsuzsa kotsuz).
- Çizgi çoklu çizgi olunca `za` ve `zb` `zs` olur (ADR 0142).

### 8. Komut

Düzeltme `cad.entities.edit`'in yeni işlemi **`edgematch`** ile yazılır. Adım adı "Kenar eşle"dir.

- Topolojik temizlik gibi geometri verilir. Pencere çekirdeğin hesabını gönderir; komut önizlemenin gösterdiğini yazar.
- Her düzeltilen nesne `update` olur: yeri, kimliği, katmanı, verisi ve sembolü aynı kalır, geometrisi kotlarıyla yazılır.
- Kilitli katman, sonlu sayı ve tek adım kuralları `cad.entities.edit`'in kurallarıdır.

### 9. Pencerenin düzeni

**Alanlar:**

- **Kaynak:** Seçili (n) ya da Katman ▾.
- **Komşu katman ▾.**
- **Sınır:** bir nesne; Sahneden seç ve Temizle.
- **Arama uzaklığı** (m) ve **Açı toleransı** (°).
- **Eşleşme ölçütü:** Yok, Katman adı ya da bir öznitelik ▾.
- **Buluşma:** Komşunun ucunda, Ortada, Sınırda.
- **Yöntem:** Ucu taşı, Parça ekle, Köşeleri ayarla.

**Bağlar tablosu:** her değişiklikte yeniden bulunur. Sütunları: Kullan, #, Kaynak (katman ve nesne), Komşu, Aralık (mm), Açı farkı (°). En büyük aralık işaretlenir.

**Göster:** satırın düğmesi pencereyi kapatır, çizimi o bağa yakınlaştırır ve bağı işaretler. Tıklama, Enter ya da Esc pencereyi aynı hâliyle geri getirir.

**Özet:**

- bağ sayısı, en büyük ve ortalama aralık;
- eşsiz uçlar, kavşak uçları, katılmayan türler ve kilitli nesneler, sayılarıyla;
- yazılamayacak bağlar.

**Uygula** kullanılan bağları tek adımda yazar ve düzeltilen nesneleri seçer. **Rapor** bağları, aralıklarını ve açılarını sekmeyle ayrılmış olarak panoya kopyalar.

### 10. Ortak çekirdek

- **`ops::edgematch`** (WASM `edgematchLinks`, `edgematchApply`):
  - uçlar, kavşaklar ve bağlı uçlar;
  - adaylar, puan ve eşleme;
  - buluşma yerleri;
  - üç yöntemin geometrisi, kotlarıyla.
- **Bağımsız başvuru:** `scripts/fixtures/edgematch_cases.py`. Uzaklıkları ve açıları mpmath ile 50 basamakta hesaplar; eşlemeyi ve yöntemleri kurallardan kurar. Eşitliğe yakın puanlar bulunmaz: durumlar puanların ayrımını denetleyerek yazılır.
- **Durumlar:**
  - düz pafta kenarı (santimetrelik aralıklarla beş çizgi);
  - üst üste binme;
  - açısı tutmayan aday;
  - iki aday arasından devam;
  - kavşak ucu;
  - üç buluşma yeri;
  - yaylı kenar;
  - Köşeleri ayarla ve Parça ekle'nin geometrisi;
  - iki ucu da bağlı nesne;
  - öznitelikle eşleme;
  - kotlar.

### 11. İş sırası

1. Çekirdek: `ops::edgematch`, WASM, bağımsız başvuru ve ortak durumlar.

   *(2 Ekim: tamam.)*
   - **Çekirdek:** `ops::edgematch` (`links`, `apply`, `Member`, `Settings`, `Link`, `Found`, `Meet`, `Method`). Uçlar, kavşaklar ve bağlı uçlar bir ızgara diziniyle bulunur, binlerce çizgide de doğrusal kalır. Arama uzaklığı ya da açı sıfırdan büyük bir sayı değilse bağ bulunmaz. Sınırda buluşma sınır ister (`no_border`). WASM `edgematchLinks`, `edgematchApply`; web cephesi `model/ops/edgematch.ts`.
   - **Başvuru:** `scripts/fixtures/edgematch_cases.py`, mpmath ile 50 basamakta, KentOS kodu olmadan. Her kararı eşiğinden ve aynı ucun öbür adaylarından uzak olduğunu denetleyerek yazar. 10 durum (`fixtures/fit/v1/edgematch.json`):
     - beş yollu pafta kenarı, dört buluşma ve yöntem birleşimiyle ve bir bölümü kullanılarak;
     - iki adaydan devam;
     - dik gelen çizgi;
     - iki çizginin ve bir alan köşesinin kavşağı;
     - yaylı kenar;
     - kotlar;
     - iki ucu bağlı nesne;
     - eşleşme ölçütü;
     - sıfır boylu kenar;
     - katılmayan türler.
   - **Sonuç:** çekirdek (yerli) ve web (WASM) bağları, sayıları, geometriyi 1e-9 m, açıyı 1e-9°, puanı 1e-12 içinde, kotları ve yazılmayan bağları bire bir verir. Puanın açı payını bozan bir deneme testi 17 uyuşmazlıkla düşürür.
2. Komut: `cad.entities.edit`'in `edgematch` işlemi iki platformda, ortak komut durumlarıyla.

   *(2 Ekim: tamam.)*
   - **Sözleşme:** `EditOperation`'ın `edgematch` değeri (katalog, TS tipleri, Python SDK'sının tipleri yeniden üretildi); adım “Kenar eşle”.
   - **Durum:** `scripts/fixtures/edit_command_cases.py` bir durum ekler. Parça ekle çizgiyi yerinde çoklu çizgi yapar; kimliği, verisi ve kotları kalır. Öbür nesnenin ucu taşınır. Geri alma tek adımdır.
   - **Bulunan:** web'de türü değiştiren `update`, eski türün kot alanlarını (çizginin `za` ve `zb`'si) yeni `zs`'nin yanında bırakıyordu. Bu yolu önce hiçbir işlem kullanmadığı için gizli kalmıştı. Masaüstü nesneyi geometriden kurar. Web artık tür değişince eski türün kotlarını da bırakır (`entitiesEdit.ts` `reshaped`).
   - **Sonuç:** masaüstünün, web'in ve Python SDK'sının çalıştırıcıları 86 durumu geçer.
3. Pencere iki platformda; sahne `fixtures/interaction/v1/edgematch.kcad` (kenarında aralıklı çizgileri olan iki pafta); resimler.

   *(2 Ekim: tamam.)*
   - **Pencere:** `transform.edgematch` (Harita › Koordinatlar, Vektör oturtma'nın yanında, kendi simgesiyle). Masaüstünde `calc/edgematch/`, web'de `ui/calc/EdgematchDialog.ts` ve `edgematchWords.ts`.
   - **Hesap:** bağlar her değişiklikte çekirdekte yeniden bulunur. Kullan, bağın uçlarıyla hatırlanır; bağlar yeniden bulunduğunda da dışarıda kalır. Uygula'nın yazacağı da her değişiklikte çekirdekte hesaplanır; yazılamayacak bağlar özette satır numaralarıyla söylenir.
   - **Sınır:** Sahneden seç ile tek nesne seçilir (çizgi, çoklu çizgi ya da alan). Esc vazgeçer, seçim eski hâline döner.
   - **Göster:** pencere kenara çekilir, bağın iki çizgisi seçilir ve çizim bağa yakınlaşır. Tıklama, Enter ya da Esc pencereyi ve eski seçimi geri getirir; bunun için iki platformda küçük bir bakma aracı var (`tools/lookTool.ts`, `kentos_interaction::look`).
   - **Tablo:** ekleme düğmesi yoktur; satırlar bağlardan gelir (iki platformda ızgaranın yeni seçeneği).
   - **Kilitli katman:** kilitli katmandaki kaynak hiç katılmaz. Komşu da, ortada ya da sınırda buluşmada katılmaz; ikisi de sayısıyla söylenir.
   - **Testler:** masaüstünde 8 test, sahnenin gerçek akışıyla:
     - bağlar ve özet;
     - Kullan;
     - Sınır'ın tıklama ve Enter'la seçimi, Esc'le vazgeçilmesi;
     - sınırda Köşeleri ayarla ile dört bağın tek adımda tam kenarda buluşması ve geri alınması;
     - Göster'in dönüşü;
     - Parça ekle ile çizginin yerinde çoklu çizgi olması;
     - uyarılar;
     - rapor.
   - **Resimler:** web'in beş sahnesi denetimleriyle, ikisi aynı sayılarla (4 bağ; en büyük aralık 215,4 mm, ortalama 123,7 mm), iki temada ve iki boyutta. Yerleşim denetimi pencereyi açar.

Her adım iki platformda, ortak fixture'larla, kendi commit'inde ilerler.

### 12. Kapsam dışı

- **Alanların sınırdaki köşeleri:** iki paftaya bölünmüş parsellerin ortak kenarı. Alan halkasında uç yoktur; ayrı bir kural ister.
- **Birleştirme:** buluşan iki çizgiyi tek nesne yapmak. Kenar eşlemeden sonra Birleştir aracı yapar.
- **Bağlarla Kauçuk levha:** bağların sabit noktalarla Kauçuk levhaya verilmesi. Vektör oturtma penceresinin tablosuna aktarılarak ayrıca eklenebilir.

## Sonuçlar

- Komşu paftaların kenarındaki çizgiler, çizginin devamı ölçütüyle eşlenir ve tek adımda buluşturulur. Her bağ tabloda görülür, çıkarılabilir.
- Topolojik temizlikten farkı: iki küme, yönle eşleme, ortada ya da sınırda buluşma, azalan kaydırma.
- Nesnelerin kimliği ve verisi korunur; yalnız Parça ekle çizgiyi çoklu çizgiye çevirir.

## Doğrulama

- **Çekirdek:** bağımsız başvuruya göre iki platformda bağlar, buluşma yerleri ve geometri.
- **Komut:** ortak komut durumlarında bit bit.
- **Arayüz:** resimler iki temada, 1440×900 ve 1100×650.
