# ADR 9119: Kent sembolleri

- **Durum:** kabul edildi (2026-11-22). Genel kitaplığın (ADR 9118) yanında ikinci dal.
- **İstek:** sahibin sözü (22 Kasım 2026): “Park bahçelere özel semboller yollara özel semboller altyapıya özel
  semboller ağaçlara çalılara bitkilere kent mobilyalar nerede sembolleri yani bir kente dair bütün semboller olsun
  mekansal planlar yapım yönetmeliği sembollerinden hariç bu semboller semboller ayrı”.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** biçim değişmedi. **Komut:** yok. **Bağımlılık:** yok.

## §1 Bağlam ve araştırma

**Önce:** Genel dal (ADR 9118) haritanın genel temalarını tutuyordu. Bir kentin ayrıntısı yoktu:

- gerçek boyunda ağaç ve çalı;
- park ve sokak donatısı;
- trafik işareti ve yol çizgisi;
- kaplama;
- altyapının küçük parçaları.

**Peyzaj projesi** (22 Kasım 2026'da arandı):

- KTÜ Peyzaj Mimarlığı'nın proje esasları ([peyzaj_nfx3H.pdf](https://ktu.edu.tr/dosyalar/peyzaj_nfx3H.pdf)): lejantta
  bitki listesi bulunur. Listede her bitkinin adı, kodu, adedi, boyu, gövde çapı ve projedeki sembolü yazar.
- Ankara Üniversitesi açık ders notları ([acikders.ankara.edu.tr](https://acikders.ankara.edu.tr/mod/resource/view.php?id=82196)):
  avan projede korunacak mevcut bitkiler ve dikilecek bitkilerin yeri ve büyüklüğü gösterilir.
- Korunacak, kesilecek ve taşınacak ağaç için ortak bir sembol bulunamadı. KentOS kendi gösterimini koydu (§3).

**Trafik işaretleri:**

- [Trafik İşaretleri Hakkında Yönetmelik](https://www.uab.gov.tr/uploads/legislations/trafik-isaretleri-hakkinda-yonetmelik/trafik-isaretleri-hakkinda-yonetmelik.pdf)
  ve KGM'nin Trafik İşaretleri El Kitabı ([kgm.gov.tr](https://www.kgm.gov.tr/SiteCollectionDocuments/KGMdocuments/Trafik/IsaretlerElKitabi/KarayoluTrafikIsaretlemeStandartlari1.pdf),
  2015 baskısı) üç öbek ayırır:
  - tehlike uyarı işaretleri, kırmızı kenarlı üçgen;
  - trafik tanzim işaretleri, daire;
  - bilgi işaretleri, mavi dikdörtgen.
- Türkiye 17 Mayıs 2023'te Viyana Yol İşaretleri Sözleşmesi'ne katıldı.
- El kitabının ölçüleri (çizgi boyları, kalınlıklar) bu adımda okunmadı (§5).

**Kent mobilyası:** yazında “kentsel donatı elemanları” diye geçer. Öbekleri oturma, aydınlatma, atık, bilgilendirme
ve yönlendirme, sınırlayıcılar (babalar), saksılar, su öğeleri, duraklar, satış birimleri ve oyun elemanlarıdır.

## §2 Karar: Kent dalı

Sistem kitaplığında **Kent** adlı yeni dal, Genel ve MPYY'nin yanında, MPYY'den ayrı: 143 sembol ve 64 yeni piktogram.
Kategoriler kentin alanlarına göredir. Bir kategoride o alanın noktaları, çizgileri ve alanları birlikte durur;
yöneticinin Alan / Çizgi / İşaret süzgeci onları ayırır.

| Kategori | Sembol | İçerik |
|---|---|---|
| Ağaçlar | 14 | yapraklı, iğne yapraklı, palmiye, meyve, servi; korunacak, anıt, kesilecek, taşınacak, dikilecek, fidan; ağaç sırası; ağaçlık, fidanlık |
| Çalı ve bitkiler | 16 | çalı, ardıç, süs bitkisi, sarılıcı, bambu; budanmış ve doğal çit, bordür bitkisi; çim, biçilmiş çim, çiçek tarhı, mevsimlik çiçek, çalı grubu, yer örtücü, süs çimi, kaya bahçesi |
| Park ve bahçe donatıları | 15 | fıskiye, çeşme, pergola, çardak, heykel, piknik masası, bayrak direği, park lambası, bilgi panosu, köpek parkı, WC; süs havuzu, çakıl, ahşap deck, stabilize yol |
| Oyun ve spor elemanları | 14 | salıncak, kaydırak, tahterevalli, kum havuzu, tırmanma, dönence, spor aleti, pota, kale, masa tenisi; kauçuk ve kum zemin, tartan, sentetik çim |
| Kent mobilyaları | 25 | bank, çöp ve geri dönüşüm kutusu, konteyner, sokak lambaları, projektör, babası, saksı, durak, bisiklet parkı, büfe, posta kutusu, ATM, reklam panosu, tabela, saat, kamera, otopark ödeme, engelli erişimi, internet, bilgi ekranı; babası dizisi, korkuluk, bariyer |
| Trafik işaretleri ve ışıkları | 13 | trafik ve yaya ışığı; uyarı, yaya geçidi uyarısı, tanzim, hız sınırı, dur, yol ver, park yapılmaz, mecburi yön, bilgi, yaya geçidi, ana yol |
| Yol çizgileri ve kenarları | 14 | kesikli, sürekli ve çift şerit çizgisi, geçici sarı çizgi, durma çizgisi, yaya geçidi, bordür, bisiklet ve otobüs şeridi, hız kesici; dört yol oku |
| Yol ve zemin kaplamaları | 13 | asfalt, beton, kilit parke, küp taş, kaldırım karosu, bisiklet yolu, refüj, park cebi, engelli park yeri, taralı alan, çim taşı, doğal taş, toprak |
| Altyapı ayrıntıları | 19 | yağmur suyu ve muayene bacası, vana odası, su sayacı, sulama vanası, yağmurlama, elektrik ve aydınlatma panosu, haberleşme menholü, fiber kutusu, doğal gaz vanası, pompa istasyonu; drenaj, damla sulama, kablo kanalı, menfez, aydınlatma kablosu, bölgesel ısıtma, basınçlı atık su |

- **Kimlik:** `kent.<nokta|cizgi|alan>.<grup>.<ad>`.
- **Kaynak:** Genel ile aynı betik ve aynı dosya: `scripts/style/general_library.py`,
  `crates/native/style/assets/general-library.kstil`. Piktogramlar ortaktır, “Genel › Piktogramlar”da durur.

## §3 Karar: gerçek boy ve öznitelikler

- **Gerçek boyunda çizilenler:** ağaçlar, çalılar, çit ve bordür bitkileri, yol çizgileri, yaya geçidi, hız kesici ve
  yol okları. Birimleri metredir (`unit: "m"`), plan görünüşü bir peyzaj ya da yol projesi gibi ölçeklenir. Öbür
  işaretler kâğıt milimetresidir.
- **Taç:** ağacın ve çalının taç çapı nesnenin Taç alanından okunur (metre): `varsayılan(sayı([Taç]), 6)`. Boşsa
  türünün varsayılanı alınır:
  - yapraklı 6, iğne yapraklı 5, palmiye 5, meyve 5, servi 2, anıt 10, fidan 1,5;
  - çalı 1,6, ardıç 1,4, bambu 2.
- **Açı:** yol oku nesnenin Açı alanıyla döner. Derecedir, saat yönünün tersinedir, 0'da ok kuzeye bakar (90 batı,
  −90 doğu). Mahalle resminde doğrulandı (§4).
- **Hız:** hız sınırı işareti sayısını Hız alanından yazar; boşsa 50.
- **Ağacın durumu (KentOS gösterimi):**

  | Durum | Gösterim |
  |---|---|
  | Korunacak | taçla birlikte yeşil çift halka |
  | Anıt ağaç | kırmızı çift halka ve sarı yıldız |
  | Kesilecek | yalnız taç çizgisi ve kırmızı çarpı |
  | Taşınacak | turuncu ok |
  | Dikilecek | kesikli taç ve artı |
  | Fidan | küçük taç ve herek çarpısı |
- **Renk:**
  - park donatısı koyu yeşil, sokak donatısı arduvaz, oyun elemanları çivit (turuncu, kauçuk zeminde seçilmiyordu);
  - trafik işaretleri öbeklerinin renginde;
  - altyapı Genel'in renklerinde.
  - Yol çizgileri beyazdır, kenarları koyu gridir. Açık zeminde (önizleme, katalog) görünür; asfaltta kenar kaybolur,
    beyaz kalır.
- **Ölçüler KentOS'un çizim varsayılanıdır, KGM ölçüsü değildir:** kesikli çizgi 3 m dolu, 5 m boş; çizgi 0,15 m;
  durma çizgisi 0,5 m; yaya geçidi 3 m boyunda, 0,5 m genişliğinde, metrede bir şerit.

## §4 Kanıt

- **Kitaplık denetimi** (`general_library.rs`) iki dalı birlikte sınar. Kimlik dalını ve türünü söyler, katmanlar,
  şekiller, yazı yüzleri ve piktogramlar tanınır, her sembol önizleme çizer, her kategoride sembol vardır, 120 piktogram
  okunur. `system::tests` toplamı (695 + 340 sembol) ve bir Kent sembolünü arar.
- **Resimler** (`.run/gui-evidence/genel-kitaplik`, açık ve koyu):
  - Kent'in dokuz kategorisinin katalog sayfası ve Stil yöneticisi resimleri;
  - yalnız Kent sembolleriyle çizilmiş 1:500 bir mahalle: kavşak, park ve meydan, bütünü, park yakını ve kavşak yakını.
- **Resimden doğrulananlar:**
  - yol okları Açı'yla doğru yöne döner;
  - taçlar Taç'la büyür;
  - dur, hız sınırı ve ışıklar okunur;
  - yaya geçidi şeritleri yolun boyunca durur.
- **Düzeltilenler:**
  - sarılıcı bitki ve kum havuzu piktogramları yeniden çizildi;
  - oyun elemanlarının rengi kauçuk zeminde seçilmiyordu;
  - beyaz yol çizgilerinin kenarı açık zeminde görünmüyordu.

## §5 Kalan

- **KGM ölçüleri:** el kitabındaki çizgi boyları ve kalınlıklar okunup varsayılanlar onlara göre düzeltilmeli.
- **Bütün işaret seti:** el kitabının bütün işaretleri, numaralarıyla, ayrı bir dal olarak.
- **Daha çok yol oku:** sola ve ileri, dönüş, U dönüşü.
- **Gerçek boyunda mobilya:** bank ve piknik masası planda gerçek ölçüsüyle.
- **Web** (TODOS.md §0.2).
