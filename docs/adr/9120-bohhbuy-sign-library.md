# ADR 9120: BÖHHBÜY işaret kitaplığı

- **Durum:** kabul edildi (2026-11-22). Genel (ADR 9118) ve Kent (ADR 9119) dallarının yanında üçüncü dal.
- **İstek:** sahibin sözü (22 Kasım 2026): “Çok güzel devam et, BÖHHBÜY sembollerini de ekle”.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** biçim değişmedi. **Komut:** yok. **Bağımlılık:** yok.

## §1 Bağlam ve kaynak

**Yönetmelik:** Büyük Ölçekli Harita ve Harita Bilgileri Üretim Yönetmeliği (BÖHHBÜY), RG 26.06.2018/30460 Mükerrer.
Ek-3 “Harita özel işaretleri” büyük ölçekli haritaların işaret kataloğudur. Her işaretin satırında şunlar vardır:

- sıra numarası, adı ve tipi: Ç çizgi, N nokta, A alan, Y yazı;
- 1/500–1/2000 ve 1/5000 çizimi, ölçüleri milimetre olarak yazılı;
- iki örnek ve açıklama.

**İşaret listesi:** yönetmeliğin metnindeki tablo 272 işaret sayar. Sınıfı, veri tabanı kodu (KY 0002, HD 0017 …) ve
öznitelik koşulu yazılıdır. Gazetenin çizimlerinde listede olmayan beş işaret daha vardır: 0645 kıyı kenar çizgisi,
0646 pompa istasyonu, 0647 regülatör, 0829 GSM baz istasyonu, 1027 harap duvar.

**Okunan kopya:** gazetenin kendi taraması, yerel PDF'in 83–133. sayfaları (gazetenin 271–321. sayfaları). Derlenmiş
kopyanın taraması ölçüleri okumaya yetmiyor. Sayfalar 400 dpi'de açıldı; ölçü yazıları büyütülerek okundu.

**Çizimler ölçekli değil.** Aynı çizimde 2 mm'lik bir aralık 5 mm'lik bir çizgiden uzun çizilebiliyor (sınırlarda
noktaların araları abartılı çizilmiş). Bu yüzden:

- yazılı ölçü esastır;
- yazılmayan ölçü, aynı çizimde yazılı bir ölçüye göre ölçülür ve açıklamada “çizimden” diye söylenir.

## §2 Karar: BÖHHBÜY dalı

Sistem kitaplığında **BÖHHBÜY** adlı yeni dal: 277 işaret, 13 sınıf kategorisi ve 33 piktogram
(“BÖHHBÜY › Piktogramlar”).

| Sınıf | İşaret |
|---|---|
| 0000 Eşyükseklik eğrileri ve kod numaraları | 6 |
| 0200 Karayolları ve tesisleri | 20 |
| 0400 Demiryolu, metro, tramvay ve tesisleri | 7 |
| 0600 Hidrografik bilgiler | 47 |
| 0800 Nakil hatları ve haberleşme tesisleri | 29 |
| 1000 Sınırlar | 27 |
| 1200 Yapılar | 36 |
| 1400 Bitki örtüsü ve tarım alanları | 26 |
| 1600 Taşlık ve kayalıklar | 10 |
| 1800 Şevler | 11 |
| 2000 Tamamlayıcı işaretler | 37 |
| 2400 Kontrol noktaları işaretleri | 5 |
| 2600 Yazı ve rakam işaretleri | 16 |

- **Kimlik ve ad:** kimlik `bohhbuy.<sıra no>`'dur. Ad numarayla başlar: “0201 Sert satıhlı yol”.
- **Kaynak bilgisi:** her işaretin kaynağı gazetenin sayfasıdır, örneğin “BÖHHBÜY Ek-3, RG 30460 Mük. s. 272”.
  Açıklaması okunan ve ölçülen ölçüleri söyler.
- **Ölçü:** işaret 1/500–1/2000 çizimidir, kâğıt milimetresiyle. 1/5000'in farklı ölçüleri açıklamadadır.
- **Renk:** yönetmelik renk vermez. Bütün işaretler mürekkeptir (temanın çizim rengi). İşaretin içinden çizgi
  geçmemesi gereken yerlerde zemin rengi (`paper`) kullanılır.
- **Alanlardan okunanlar:**

  | Alan | İşaret |
  |---|---|
  | Kot | 0004, 0005, 2601–2605 |
  | Numara | 0214 yol numarası, 2601–2605 |
  | Boy | 0821 anten (metre) |
  | Cins | 0827 vana |
  | Açı | 0620, 1018, 1024 (derece, saat yönünün tersine) |
  | Ad, Kat, Ada, Parsel | 2606–2616 |

  Yazı işaretinin (2606–2616) alanı yoksa alanın adı yazılır. İşaret böylece görünmez kalmaz, hangi alanı beklediğini
  söyler.

## §3 Karar: çizimin ve motorun gerektirdiği yorumlar

- **Sınırlar (1002–1007):** 5 mm çizgi ve ardından nokta öbeği. Nokta sayısı il 1, ilçe 2, bucak 3, köy 4, belediye
  5, mücavir alan 6'dır; sayılar çizimden okundu. Nokta 1/2 mm, aralar 1/2 mm'dir; ilde 1 mm.
- **Şevler (1805–1808):** tabloda alan işaretidir. KentOS şev kenarına çizilen çizgi işareti olarak verir:
  - kenar kesik çizgidir (7 mm çizgi, 1 mm boşluk);
  - uzun (5 mm) ve kısa (2,5 mm) taramalar çizginin sağına düşer, aralıkları 1,25 mm'dir.

  1805 yarma ile 1806 dolmanın çizimleri aynıdır; ayrım öznitelikte (AR 0007, AR 0006).
- **Köşeden köşeye çarpı:** sundurma (1212) ve kapalı üstgeçit ile durak (0210, 0213) için. Motor bir çokgenin
  köşegenini çizemez. KentOS çarpıyı alanın ortasına 3 mm çizer.
- **Sera (1213):** iç çizgi kenardan a/4 içeridedir ve binanın boyuna göre değişir. KentOS iç çizgiyi 1 mm içeride
  çizer; köşe çizgileri köşelere konur.
- **Örnekten tamamlananlar:**
  - ölçeğe sığmayan su yolu köprüsü (0620) su yolunun iki yanına karşılıklı çizilir;
  - dalgakıran (0638) eksenin iki yanına simetriktir;
  - ölçeğe sığmayan su yolu tünelinin (0625) ağızları örnekteki gibi dışa açılır.
- **Alan içi desenleri:** çeltik, doğal gaz istasyonu, nükleer alan ve bitki örtüsü desenleri sınırsızdır. Alanın
  sınırı kendi işaretiyle (orman sınırı 1401, çit, tel örgü …) ayrıca çizilir.
- **Şerare:** trafonun (0812) ve santralin (0817) içinde 1/5000 boyunda, 4 mm çizilir.
- **Aynı çizilenler:** 1419 fidanlık 1402 ile, 1806 dolma 1805 ile yönetmelikte aynı çizilmiştir.

## §4 Karar: kaynak ve yükleme

- **Kaynak:** `scripts/style/bohhbuy_library.py`. Betik `crates/native/style/assets/bohhbuy-library.kstil`'i yazar.
  `--check` dosya betikten geri kaldığında düşer, `--svg` piktogramları döker.
- **Yardımcılar:** Genel'in betiğinin yardımcıları kullanılır (şekil, çizgi, desen, piktogram). İşarete özgü olanlar
  bu betiktedir:
  - `seg`: noktanın milimetreleriyle düz çizgi; işaretin kendi döndürmesi kaydırmayı da döndürür (ADR 9118 §5);
  - `Box`: piktogramı milimetreyle çizer, işaretin noktasını kutunun ortasından kaydırır;
  - `head`: ucu yerinde duran ok başı.
- **Yükleme:** `kentos_native_style::system` üç dosyayı okur ve tek sistem kaynağında birleştirir. Öğeler salt
  okunurdur ve kopyalanabilir.

## §5 Kanıt

- **Kitaplık denetimi** (`crates/native/style/tests/all/bohhbuy_library.rs`):
  - 277 işaret, her sınıfta Ek-3'ün sayısı kadar;
  - kimlik dört haneli numaradır, ad numarayla başlar, kategori numaranın sınıfıdır;
  - her katman Genel'in ortak denetiminden geçer: türünün çizdiği katman, bilinen şekil, yazı yüzü ve piktogram;
  - her işaret önizleme çizer;
  - her piktogram masaüstünün okuduğu vektör çizimidir.
- **Sistem kitaplığı:** `system::tests` yeni toplamı (695 + 340 + 277 sembol) ve bir BÖHHBÜY işaretini arar.
- **Resimler** (`.run/gui-evidence/bohhbuy-kitaplik`, `bohhbuy_screens` sınamaları):
  - her sınıfın 1:1000 katalog sayfaları, açık ve koyu: on iki işaret bir sayfa, numara sırasıyla, 29 sayfa; resim iki
    kat pikselle çekilir, ince çizgiler büyütünce okunur;
  - sınıf başına Stil yöneticisi, açık ve koyu;
  - bütün işaretler yakından (`yakin-*.png`, 18 resim, 4 × 4 göz): her işaret çizimiyle karşılaştırıldı.
- **Resimlerden düzeltilenler:**
  - **Desenin işaret kaydırması:** desen döşemesi bir işareti kaydırmasına koyar, sonra orada döndürür. Nokta ve çizgi
    işaretinde ise döndürme kaydırmayı da döndürür (ADR 9118 §5). `seg` ile yazılan dönük çizgiler desende yanlış
    yere düşüyordu: mezar işaretleri, vana sapı, turunçgil, çay, çayır, kamış, yanmış orman, blok kaya. Betiğin
    `bpattern`'i desene giren dönük katmanın kaydırmasını geri döndürür.
  - Sıra kayanın üçgenleri kırık çizgide dönüyordu; artık hep aşağı bakar.
  - Minareye türbenin taban çizgisi karışmıştı; minarenin kendi piktogramı var.
  - İstikamet okunun 3 mm başı motorun ok ucuyla çok genişti (2,4 mm); çizimdeki gibi 1,4 mm eninde çizilir.
  - Eşyükseklik kotu ve yüksek gerilim hattının şeraresi kısa çizgilerde hiç görünmüyordu; ilki 1 cm'den başlar.
  - Deniz, göl ve dere adları dik yazılıyordu: paketle gelen sans yüzünün eğik biçimi yok. Bu üç yazı arayüz yüzünün
    eğik biçimiyle yazılır.

## §6 Kalan

- **1/5000 çizimleri:** ayrı işaret olarak ya da ölçeğe bağlı kuralla.
- **Motor:** desendeki işaretin kaydırması nokta ve çizgi işaretindeki gibi döndürmeyle dönsün mü? Değişiklik MPYY'nin
  desenlerini de etkiler; ayrı karar. Eğik yazı istenip yüzün eğik biçimi yoksa motor yazıyı kendisi eğebilir.
- **Gerçek veri:** bir belediyenin ya da kadastronun BÖHHBÜY paftasıyla karşılaştırma.
- **Web** (TODOS.md §0.2).
