# ADR 0229: Adres arama

- **Durum:** kabul edildi (2026-10-12). Kapsamı sahibin kurallarıyla ben belirledim. İlke “Performance First”; simgeler sorulmadan
  seçilir. Madde tek parçada biter. Web'e ara verildikten sonraki ilk madde: yalnız masaüstünde ve bulutta, Python'da ve MCP'de yapılır.
- **Bağlam belgesi:** TODOS.md `GIS-40`, ADR 0228 (web'e ara, katalog ek dosyası), ADR 0208 (harita servisleri: ağ istekleri, cihazdaki
  gizli değerler, OpenStreetMap'in kuralları), ADR 0178 (Veride ara: alt panelin sekmesi, Koordinata git'in işareti), ADR 0209 (projede
  saklanan, katmanlara başvuran tanım ve onun komutu), ADR 0167 ve 0168 (koordinat dönüşümleri), ADR 0199 (katmanın alanları), ADR 0223
  (tablo dosyaları), ADR 0165 §2 (yerel projenin birimi).
- **Web:** ertelendi (ADR 0228); geri dönüşte gerekenler TODOS.md §0.2'de.

## Bağlam

Belediye ve kurum çalışanı bir adresin yerini her gün arar: ruhsat dosyasındaki adres, şikâyetin adresi, abone listesindeki adresler.
Bunun tersi de sorulur: tıklanan yerin adresi nedir, bu noktaların her birinin adresi nedir? KentOS'ta bugün adresi aramanın yolu yoktur
(ADR 0178 §9 coğrafi kodlamayı kapsam dışı bıraktı).

- **ArcGIS Pro:**
  - Locate bölmesi tek kutuda adres arar, tıklanan yerin adresini söyler.
  - Geocode Addresses tablo tablo eşler; her satıra Status (M eşleşti, T berabere, U eşleşmedi), Score, Match_addr ve Addr_type yazar.
    Rematch Addresses berabere ve eşleşmeyen satırları elle düzeltir; Reverse Geocode noktalara adres yazar.
  - Kurumun verisinden “locator” kurulur: PointAddress (kapı noktası), StreetAddress (yol), Locality (mahalle, ilçe) rolleri.
  - Esri'nin World Geocoding Service'i belirteç ister. Sonuç saklanacaksa `forStorage=true` gerekir ve kredi harcar.
- **QGIS:** Locator çubuğunda Nominatim. İşlemler'de Batch Nominatim geocoder; Nominatim'in kuralına uyar: saniyede bir istek, tek iş.
- **Netcad:** NetGIS'in adres sorgusu, belediyelerin numarataj verisi üzerinde çalışır.

Türkiye'de adres il, ilçe, mahalle ya da köy, yol (cadde, sokak, bulvar, meydan), dış kapı numarası (12, 12/A), iç kapı numarası ve
posta kodundan oluşur; Ulusal Adres Veri Tabanı her adrese on haneli bir adres kodu verir (UAVT). Belediyelerin numarataj verisi kapı
noktaları (her dış kapı bir nokta: mahalle, yol, kapı numarası, adres kodu), yol orta hatları ve mahalle sınırlarıdır. Yazılan adreslerin
biçimi çok çeşitlidir: “Kızılay Mah. Atatürk Blv. No:12/3 Çankaya/Ankara”, “1203/1 Sk. No 5”, “Cumhuriyet Mahallesi 1234. Sokak 7”.

Çevrimiçi sağlayıcıların koşulları farklıdır:

- **Nominatim (OpenStreetMap):**
  - Saniyede en çok bir istek; uygulamayı tanıtan bir kullanıcı ajanı ister.
  - Otomatik tamamlama kesinlikle yasaktır. Sonuçlar istemcide önbelleğe alınmalıdır.
  - Toplu kodlama yalnız küçük, bir kerelik işlerde, tek iş ve tek makinede yapılabilir. Bir günden uzun ya da düzenli çalışan betik
    dakikada dört istekle sınırlıdır.
  - Veri ODbL'dir ve atıf ister ([kullanım ilkesi](https://operations.osmfoundation.org/policies/nominatim/)).
- **Photon'un komoot sunucusu:** adil kullanım ister. Yoğun kullanım yavaşlatılır, erişilebilirlik güvence altında değildir.
- **Esri'nin World Geocoding Service'i:** belirteç ister. Sonuçlar `forStorage=false`'ta yalnız gösterilebilir, saklanamaz; saklamak
  `forStorage=true` ister.
- **Google Geocoding API:** koşullarının 6.2'sine göre sonuç Google'ın olmayan bir haritayla birlikte kullanılamaz. 6.3.1'e göre enlem
  ve boylam en çok 30 gün saklanabilir
  ([hizmete özel koşullar](https://cloud.google.com/maps-platform/terms/maps-service-terms)).

Sağlayıcıya giden her adres ve konum o sağlayıcının gizlilik ilkesine tabidir: hangi verinin nereye gittiği açıkça seçilmelidir.

## Karar

### 1. Kapsam

- **Adresten konuma:** alt panelin **Adres** sekmesinde tek adres. Toplu adres İşlemler'de **Adresleri eşle** ile: tablo dosyasından ya
  da bir katmanın özniteliklerinden, tek alan ya da ayrı alanlarla.
- **Konumdan adrese:** **Konumdan adres** aracı tıklanan yerin adresini söyler. **Konumlara adres yaz** (İşlemler) bir katmanın
  nesnelerine adreslerini yazar.
- **Kaynaklar:**
  - **Projenin adres verisi:** kurumun kapı noktaları, yolları ve mahalle ve ilçe alanları. Projede tanımlanır (§2), çevrimdışı çalışır,
    hiçbir veri cihazdan çıkmaz.
  - **Çevrimiçi sağlayıcılar:** Nominatim (OpenStreetMap), Photon (komoot), ArcGIS World Geocoding Service ve Google Geocoding API.
    Ayrıca kurumun kendi sunucusu: Nominatim, Photon, Pelias ya da ArcGIS GeocodeServer.
- **Koşullar ve gizlilik:**
  - Çevrimiçi bir sağlayıcı, kullanıcı koşullarını okuyup onaylayana ve onu açana dek hiç kullanılmaz. Varsayılanı yoktur.
  - Neyin nereye gittiği, sınırlar, sonucun saklanıp saklanamayacağı ve atıf her sağlayıcının kartında yazar (§10).
- **Kapsam dışı:**
  - çevrimiçi sağlayıcıda yazdıkça öneri (Nominatim'de yasak, öbürlerinde sonraki iş);
  - numarataj üretmek (kapılara numara vermek) ve UAVT ile eşitlemek;
  - yolun kapı numarası aralıklarıyla aradeğer (TIGER biçimi; Türkiye'deki veride yoktur, §7 kapılar arası aradeğeri yapar);
  - bina alanından kapı;
  - ArcGIS'in toplu `geocodeAddresses` işlemi (toplu iş de aday aday sorar);
  - Python'da çevrimiçi sağlayıcılar (Python projenin verisinde arar, §14);
  - web (ADR 0228).

### 2. Veri modeli: projenin adres verisi

- Proje ayarlarının `addressLocators`'ı (`kentos_contracts::AddressLocator`, en çok 16). Her tanımın alanları şunlardır:
  - `id` (küçük harf, rakam, tire; 1–40) ve `name` (kırpılmış, 1–80 karakter);
  - `doors` (kapı noktaları): `layer` ve alanlar `street` (yol) ve `door` (dış kapı numarası) zorunlu; `neighbourhood` (mahalle),
    `district` (ilçe), `province` (il), `postcode` (posta kodu), `building` (bina adı) ve `code` (adres kodu) isteğe bağlı;
  - `streets` (yol orta hatları): `layer`, `name` zorunlu; `neighbourhood`, `district` isteğe bağlı;
  - `areas` (bölgeler, en çok üç, her düzeyden bir): `layer`, `level` (`neighbourhood`, `district`, `province`), `name` zorunlu;
    mahalle düzeyinde `district` isteğe bağlı;
  - `minScore` (en düşük eşleşme puanı, 50–100, varsayılan 85) ve `reach` (Konumdan adres'in en büyük uzaklığı, metre, 0'dan büyük ve en
    çok 10 000, varsayılan 100).
- Alanlar özniteliklerin adlarıdır (katmanın alanları ya da nesnelerin öznitelikleri, ADR 0199).
- **Kurallar** (`AddressLocator::problem`, `address_locators_problem`; okuyucularda, komutta ve sunucuda aynı):
  - kapı, yol ya da bölgeden en az biri olmalı;
  - alan adları boş, kırpılmamış ya da 64 karakterden uzun olamaz;
  - bir düzey iki kez olamaz;
  - kimlik ve ad (Türkçe büyük küçük harf katlamasıyla) iki kez olamaz.
- Var olmayan katman yazılır, uyarıyla söylenir (§14). Tanımlar katmanın verisini kopyalamaz: dizin her aramada çizimden kurulur (§6).
- **`.kcad` şema 42** (`FORMATS_VERSION` 54): proje ayarlarının `addressLocators`'ı. Yalnız tanım varken yazılır; öbür çizimler bayt bayt
  eskisi gibi kalır. Bağımsız Python okuyucusu ve yazıcısı, örnek `addresses.kcad` ve bozuk dosyalar.

### 3. Katlama ve sözcükler

Bütün karşılaştırmalar katlanmış metin üzerindedir (`ops::geocode::fold`):

- **Katlama:**
  - `Ç ç → c`, `Ğ ğ → g`, `I ı İ → i`, `Ö ö → o`, `Ş ş → s`, `Ü ü → u`, `Â â → a`, `Î î → i`, `Û û → u`.
  - U+0300–U+036F arasındaki birleşen işaretler atılır.
  - Öbür her karakter, Unicode'un küçük harfi tek karakterse ona çevrilir; değilse olduğu gibi kalır.
- **Sözcükler:**
  - Katlanmış metin harf ya da rakam dizilerine (sözcük) ayrılır. Harf ya da rakam: ASCII harf ve rakam, ASCII dışında Unicode'un harf ve
    rakamları.
  - Bir sözcük, ASCII rakam olmayan bir karakterden sonra gelen ASCII rakamın önünde de bölünür: `no12` → `no`, `12`. Rakamdan
    harfe bölünmez: `12a`. Harf ya da rakam: Rust'ın `char::is_alphanumeric`'i (Python'da `str.isalnum`).
  - `/`, `,` ve `-` kendi başına **işaret**tir. Öbür her karakter ayırıcıdır (boşluk, `.`, `:`, `;` dahil).
  - Bir sözcük ASCII rakamla başlıyorsa **sayılı**dır.
- **Etiketler** (yalnız bütün bir sözcük):

  | Etiket | Sözcükler |
  |---|---|
  | MAHALLE | `mahalle`, `mahallesi`, `mah`, `mh` |
  | KÖY | `koy`, `koyu` |
  | CADDE | `cadde`, `caddesi`, `cad`, `cd` |
  | SOKAK | `sokak`, `sokagi`, `sok`, `sk` |
  | BULVAR | `bulvar`, `bulvari`, `blv`, `bulv` |
  | MEYDAN | `meydan`, `meydani`, `myd` |
  | YOL | `yol`, `yolu` |
  | NO | `no`, `numara`, `nu`, `nolu` |

  Yol etiketleri CADDE, SOKAK, BULVAR, MEYDAN ve YOL; bölge etiketleri MAHALLE ve KÖY'dür.
- **Ad dizisi:** bir sıranın önündeki sözcükler, geriye doğru okunur.
  - Sayılı bir sözcük ile önündeki `/` ve onun önündeki sayılı sözcük birleşir: `1203 / 1` → `1203/1`.
  - Okuma başta, `,`, `-`, birleşmeyen `/`, herhangi bir etiket ya da başka bir parçanın aldığı sözcükte durur.
  - Ad, sözcüklerin okunma sırasıyla tek boşlukla birleşimidir; **anahtarı** odur. Boş ad yok sayılır.
- **Alanın değeri:** kapının, yolun ve bölgenin alanı da aynı yolla okunur.
  - Yol alanında bir yol etiketi varsa ad, son yol etiketinin önündeki dizidir ve tür o etikettir (`Atatürk Bulvarı` → `ataturk`,
    BULVAR). Etiket yoksa ad bütün sözcüklerdir, tür yoktur.
  - Mahalle alanında MAHALLE ve KÖY etiketleri atılır (`Kızılay Mahallesi` → `kizilay`).
  - İlçe ve il alanının anahtarı bütün sözcükleridir.

### 4. Adresin parçaları

Tek satırlık adres şu sırayla okunur (`ops::geocode::parse`). Bir parçanın aldığı sözcük **alınmış**tır; alınmamış, etiket olmayan sözcük
**serbest**tir.

1. **Posta kodu:** ilk iki hanesi 01–81 olan beş ASCII rakamlı ilk sözcük. Yerine `,` konur; böylece ad dizilerini böler.
2. **Kapı işareti:** ilk NO etiketi (alınır).
3. **Yol:** yol etiketlerinden kapı işaretinden önceki sonuncusu; kapı işaretinden önce yol etiketi yoksa bütün adresteki sonuncusu.
   Etiket alınır; adı onun ad dizisidir (§3), türü odur. Boş adlı yol yoktur.
4. **Kapı:**
   - NO etiketinden sonraki ilk sözcük (aradaki `-`'ler atlanır); NO yoksa yol etiketinden hemen sonraki sözcük (aradaki `,`'ler
     atlanır). Serbest ve sayılı olmalıdır.
   - Ardından `/` ya da `-` ve tek karakterlik sayılı olmayan serbest bir sözcük geliyorsa o kapıya eklenir (`12/A` → `12a`). `/` ve
     sayılı serbest bir sözcük iç kapıdır; alınır, kullanılmaz.
5. **Mahalle:** bölge etiketlerinden yol etiketinden önceki sonuncusu; yoksa bütün adresteki sonuncusu. Etiket alınır; adı onun ad
   dizisidir.
6. **İlçe ve il** (yalnız yol ya da bölge etiketi varken): sondan başa ilk alınmamış `/` için, `/`'tan geriye ve ileriye sayılı olmayan
   serbest sözcükler okunur; ikisi de boş değilse geriye okunanlar ilçe, ileriye okunanlar il olur (`Çankaya/Ankara`) ve alınır.
7. **Kalan:** serbest sözcükler, sırasıyla. Projenin verisinde ilçe adı aranır: 6'dan ilçe gelmediyse, kalan sözcükler içinde verinin bir
   ilçe anahtarı bütün sözcükleriyle ard arda geçiyorsa (en çok sözcüklüsü, eşitse önce geçeni) o ilçedir.
8. **Etiketsiz adres** (ne yol ne bölge etiketi var):
   - kapı NO'dan okunur (4); NO yoksa ilk sözcük olmayan ilk serbest sayılı sözcükten;
   - ad, baştan başlayıp ilk `,`, `/`, `-` ya da kapıda biten serbest sözcüklerdir (alınır);
   - bu ad yol adı (türsüz) olarak aranır ve (§7.5) bütünüyle mahalle adı olarak da aranır.

- **Kapı anahtarı:** sayılı sözcüğün baştaki rakamları, baştaki sıfırlar atılarak (hepsi sıfırsa `0`), ardından sözcüğün geri kalanı
  ve eklenen harf: `012` → `12`, `12A` → `12a`, `12/a` → `12a`.
  - **Numarası** baştaki rakamların tam sayı değeridir; 10 haneden uzunsa ya da 10⁹'dan büyükse numarası yoktur.
- **Ayrı alanlar** (toplu işte): mahalle, yol, ilçe ve il alanı §3'teki değer kuralıyla okunur. Kapı alanında ilk serbest sayılı sözcük
  4'teki kuralla okunur. Posta kodu kırpılır.

### 5. Benzerlik

- İki anahtarın benzerliği `B = ⌊100 · (L − d) / L⌋`'dir:
  - `d` Levenshtein uzaklığıdır: karakter ekleme, silme ve değiştirme, her biri 1;
  - `L` uzun olanın karakter sayısıdır.
- **Sayılı ad:** bütün sözcükleri sayılı olan addır (`1203/1`, `15`). İki addan biri sayılıysa benzerlik eşitse 100, değilse 0'dır.
- Bir ad **benzer** sayılır: `B ≥ 80`.
- Çekirdek, eşiği geçemeyecek adayları kesin süzgeçlerle atar: uzunluk farkı ve q-gram sayısı. Sonuç, bütün adları tek tek karşılaştırmakla
  aynıdır.

### 6. Dizin

Dizin bir tanımdan ve çizimden kurulur (`ops::geocode::locator`). Katmanın nesneleri çizim sırasıyla okunur; katman süzgecinden
geçmeyenler okunmaz (ADR 0211). Her nesnenin **sırası** çizimdeki yeridir.

- **Bölgeler:** bölge katmanlarının alan nesneleri (kapalı alan, çok parçalı alan, daire).
  - Mahalle düzeyinde ad §3'ün mahalle değeridir, öbürlerinde bütün sözcükleri.
  - Mahalle düzeyinde ilçe de okunur.
  - Adı boş alan atlanır ve sayılır.
- **Kapılar:**
  - **Nesne:** kapı katmanının nokta nesneleri. Çok noktalı nesnenin ilk noktası alınır; öbür türler atlanır ve sayılır.
  - **Okunmayan:** yol adı boş ya da kapı alanında sayılı sözcük olmayan kapı atlanır ve sayılır.
  - **Kapının yolu:** yol adı, yolun türü, mahalle, ilçe ve il anahtarları, posta kodu, bina adı, adres kodu. Hepsi yazıldığı biçimle de
    (kırpılmış) saklanır.
- **Yollar:** yol katmanının çizgi, çoklu çizgi ve yay nesneleri.
  - Yolun **orta noktası**, yolun uzunluğunun yarısındaki noktadır; yayda yay üstündedir.
  - Mahalle alanı verilmemişse ya da değeri boşsa mahalle, orta noktayı içeren ilk mahalle alanından gelir (kenarı da içinde); o da
    yoksa boştur.
  - Adı boş yol atlanır ve sayılır.
- **Yol öbekleri:**
  - Kapılar (yol adı, mahalle, ilçe) anahtarıyla öbeklenir; verilmeyen alanın anahtarı boştur.
  - Bir yol nesnesi, aynı adlı ve uyumlu bir kapı öbeği varsa ona bırakılır. Mahalleleri eşit ya da biri boşsa uyumludur; ilçe için de
    öyle.
  - Bırakılmayan yollar (yol adı, mahalle, ilçe) anahtarıyla kendi öbeklerini kurar: kapısız yol.
  - Öbeklerin sırası ilk nesnelerinin sırasıdır. Öbeğin türü ilk kaydının türüdür.
- **Öbeğin yol düzeyi yeri:**
  - Kapılı öbekte kapılar (numara, anahtar, sıra) ile sıralanır; ortanca kapının (alt ortanca, `(n − 1) / 2`'nci) noktası alınır.
  - Kapısız öbekte en uzun yolun orta noktası alınır (eşitse önce gelen).
- **Mahalleler:** mahalle alanlarının, sonra kapı öbeklerinin, sonra kapısız yolların mahalle anahtarları; her anahtar bir kez, ilk
  geldiği yerle. Mahallenin metni ve ilçesi ilk geldiği yerinkidir (alanın adı ve ilçesi, kapının mahallesi, ilçesi ve ili, yolun
  mahallesi ve ilçesi).
- **Mahallenin yeri:**
  - o adlı mahalle alanı varsa alanı en büyük olanın (eşitse önce gelenin) ulaşılmazlık kutbu (ADR 0219 §6; kesinlik 0,1 m, yerel
    projede birimine çevrilir);
  - yoksa o mahallenin kapılarının ortalaması (sırasıyla toplanır);
  - o da yoksa mahallenin ilk kapısız yolunun yol düzeyi yeri.

### 7. Adresten konuma

Okunan parçalar Q'dur. Bir dizinde aranır (`ops::geocode::forward`).

1. **Adres kodu:** tanımda kod alanı varsa ve kırpılmış adres bir kapının kodunun aynısıysa, o kapılar aday olur (tür **Kod**, puan
   100). Kod tutarsa başka aday aranmaz.
2. **Yol adayları:** Q'nun yol adına benzer adlı her öbek. Q'nun yol adı birden çok sözcükse ve hiçbir öbek benzer değilse:
   - baştan bir, iki … sözcük atılarak yeniden aranır; ilk tutan uzunluk kullanılır;
   - atılan sözcükler, Q'da mahalle yoksa mahalle adı sayılır (“Kızılay Atatürk Bulvarı”).
3. **Puan:** `100 − (100 − B_yol) − P_tür − P_mahalle − P_ilçe − P_posta − P_kapı`. Sıfırın altına inmez. Cezalar:
   - **P_tür:** 5, Q'nun ve öbeğin türü varsa ve farklıysa.
   - **P_mahalle:** Q'da mahalle varsa ve öbeğinki boş değilse:
     - mahalle benzerliği `M ≥ 80` ise `100 − M`;
     - değilse 30.
   - **P_ilçe:** 20, Q'da ilçe varsa, öbeğinki boş değilse ve benzer değilse.
   - **P_posta:** 10, Q'da posta kodu varsa, öbeğin kapılarında posta kodu varsa ve hiçbiri Q'nunki değilse.
   - **P_kapı:** Q'da kapı varsa:
     - öbekte aynı anahtarlı kapı: 0 (tür **Kapı**; birden çok varsa önce gelen);
     - aynı numaralı başka anahtarlı kapı: 5 (tür **Kapı**; harfsiz olan, yoksa anahtarı küçük olan);
     - aradeğer (aşağıda): 10 (tür **Kapı (aradeğer)**);
     - hiçbiri: 25 (tür **Yol**, yol düzeyi yer).

     Q'da kapı yoksa 15'tir (tür **Yol**, yol düzeyi yer).
4. **Kapılar arası aradeğer:**
   - Öbekte Q'nun numarasıyla aynı tekliği olan ve numarası ondan küçük kapıların en büyük numaralısı `a`, büyük olanların en küçük
     numaralısı `b` alınır. Aynı numarada birden çok kapı varsa anahtarı küçük olan, sonra sırası küçük olan seçilir.
   - İkisi de varsa `f = (n − na) / (nb − na)` ve yer `Pa + (Pb − Pa) · f`'dir (bileşen bileşen, bu işlem sırasıyla). Yoksa aradeğer
     yapılmaz.
5. **Mahalle adayları:** Q'nun mahalle adına (2'de atılan sözcüklerden gelen de; etiketsiz adreste bütün adına) benzer her mahalle
   anahtarı aday olur. Tür **Mahalle**, puan `M − 40`, yer §6'nın mahallenin yeri.
   - **Adayların metinleri:** kapı ve kod adayında kapının metinleri; aradeğer ve yol adayında öbeğin ilk kapısının yolu, mahallesi,
     ilçesi ve ili (aradeğerde kapı Q'nun kapı anahtarıdır), kapısız yolda ilk yolunun metinleri; mahalle adayında mahallenin metinleri.
   - **Adayların sırası:** kapıda kapının, aradeğerde iki kapının küçük olanının, yolda öbeğin, mahallede mahallenin ilk geldiği yerin
     sırası.
6. **Liste:** puanı 40 ve üstü olan adaylar, en çok 20 tane.
   - **Sıra:** puan (büyükten küçüğe), tür (Kod, Kapı, Kapı (aradeğer), Yol, Mahalle), yol anahtarı, mahalle anahtarı, kapı anahtarı
     (Unicode sırası) ve çizim sırası.
7. **Durum:**
   - En iyi puan tanımın `minScore`'u ve üstündeyse ve aynı puanlı başka yerde aday yoksa **Eşleşti**.
   - Aynı puanlı başka yerde aday varsa **Belirsiz**.
   - Değilse **Eşleşmedi**.

### 8. Konumdan adrese

Bir noktanın adresi şöyle bulunur (`ops::geocode::reverse`). Uzaklık tanımın `reach`'idir; metredir, yerel projede birimine çevrilir.

1. Bu uzaklık içindeki en yakın kapı (eşitse çizimde önce gelen): tür **Kapı**.
2. Kapı yoksa uzaklık içindeki en yakın yol (yolun kendisine uzaklık; yayda yaya): tür **Yol**.
3. Noktayı içeren mahalle, ilçe ve il alanları (kenarı içinde sayılır; birden çoksa önce gelen). Kapı ve yol yoksa tür **Bölge**.
4. **Alanlar:**
   - mahalle: kapının, yoksa yolun, yoksa mahalle alanının adı;
   - ilçe: kapının, yoksa yolun, yoksa mahalle alanının ilçesi, yoksa ilçe alanının adı;
   - il: kapının, yoksa il alanının adı;
   - yol, kapı, posta kodu, bina adı ve adres kodu: kapının (yol bulunduysa yolun adı);
   - uzaklık: kapıya ya da yola, metre.
5. Hiçbiri yoksa sonuç yoktur: “Bu uzaklıkta adres yok”.

### 9. Adresin yazılışı

Adres metni şu parçaların aralarında boşlukla birleşimidir (verideki yazılışlarıyla):

- mahalle: adında MAHALLE ya da KÖY etiketi yoksa sonuna “ Mahallesi” eklenir;
- yol;
- “No:” ve kapı (aradeğerde Q'nun kapısı);
- posta kodu;
- “İlçe/İl”: biri yoksa yalnız öbürü.

Örnek: “Kızılay Mahallesi Atatürk Bulvarı No:12 06420 Çankaya/Ankara”.

### 10. Çevrimiçi sağlayıcılar

**Hazırlar** (`fixtures/geocode/v1/providers.json`, platformların tek kaynağı; `kentos_services::geocode`):

| Sağlayıcı | Sunucu | Anahtar | Hız | Toplu iş | Çizime yazma | Önbellek |
|---|---|---|---|---|---|---|
| Nominatim (OpenStreetMap) | nominatim.openstreetmap.org | — | 1 istek/s, tek iş | ≤ 1 000 adres | var (ODbL atfıyla) | 90 gün |
| Photon (komoot) | photon.komoot.io | — | 1 istek/s, tek iş | ≤ 1 000 adres | var (ODbL atfıyla) | 90 gün |
| ArcGIS World Geocoding | geocode-api.arcgis.com | belirteç | 10 istek/s, 4 iş | yalnız “Sonuçları sakla” açıkken | yalnız “Sonuçları sakla” açıkken (`forStorage=true`, kredi harcar) | yalnız saklanabilen sonuç, 90 gün |
| Google Geocoding API | maps.googleapis.com | anahtar | 10 istek/s, 2 iş | yok | yok | yok |
| Kurum sunucusu (Nominatim, Photon, Pelias, ArcGIS) | kullanıcının adresi | isteğe bağlı | ayarlanır: 0,1–50 istek/s, 1–8 iş (5 ve 2) | sınırsız | var | 90 gün |

**Kurallar:**

- **Koşulların onayı:**
  - Her sağlayıcının kartı şunları söyler:
    - gönderilen: aranan adres metni ya da tıklanan yerin WGS 84 koordinatları;
    - sunucu, hız ve toplu iş sınırı, çizime yazma, önbellek ve atıf;
    - koşulların ve gizlilik ilkesinin adresi.
  - Kullanıcı “Koşulları okudum; adresler <sunucu> sunucusuna gönderilir” kutusunu işaretlemeden sağlayıcıyı açamaz.
  - Onay, koşulların sürüm numarasıyla saklanır. KentOS kartın metnini değiştirirse sürüm artar ve onay yeniden istenir.
- **Hız:** iki isteğin başlangıcı arasında en az `1/hız` saniye ve aynı anda en çok `iş` kadar istek. Etkileşimli arama ve toplu iş aynı
  sırayı paylaşır.
- **Önbellek:** cihazdadır. Anahtarı yöntem ve gizli değerleri çıkarılmış adrestir. Süresi dolan kayıt sorulmadan atılır. Aynı soru
  sunucuya yinelenmez (Nominatim'in kuralı). “Önbelleği temizle” her şeyi siler.
- **Toplu iş:** önce adresler tekilleştirilir (katlanmış ve boşlukları sadeleşmiş metin). Tekil adres sayısı sağlayıcının sınırını
  aşarsa iş hiç istek göndermeden reddedilir.
- **Google:** sonuçları yalnız görünen bir Google altlığıyla birlikte gösterilir (koşullar 6.2). Google altlığı yoksa sağlayıcı
  listede nedeniyle kapalıdır. Sonuç çizime yazılmaz, önbelleğe alınmaz, toplu işte kullanılmaz (6.3). Panelde “Google Maps” atfı
  görünür.
- **ArcGIS:** “Sonuçları sakla” kapalıyken istek `forStorage=false`'tur; sonuç yalnız gösterilir, Nokta ekle ve toplu iş kapalıdır. Açıkken
  her istek `forStorage=true`'dur.
- **Projenin sistemi:** çevrimiçi sonuç WGS 84'tür ve projenin datum seçimleriyle projenin sistemine çevrilir (ADR 0168). Projenin sistemi
  yoksa çevrimiçi kaynaklar nedeniyle kapalıdır.
- **Kimlik:** istek uygulamayı tanıtan kullanıcı ajanıyla gider (`KentOS-CAD/<sürüm> (+adres)`, ADR 0208 §4); dil Türkçe istenir.
- **Anahtarlar:** cihazın gizli değerleridir (`baglantilar.json`, ADR 0208 §12; kimliği `adres:<sağlayıcı>`). Çizime, günlüğe, önbelleğin
  anahtarına ve iletilere yazılmaz.

**İstekler** (`kentos_services::geocode::request`). Parametreler bu sırayla yazılır. Değerler harita servislerinin kodlamasıyla
(`kentos_services::query::encode`, ADR 0208) yüzde kodlanır: RFC 3986'nın ayrılmamış karakterleri ile `,` `:` `/` `*` olduğu gibi kalır,
boşluk `%20` olur. Koordinatlar 7 ondalıkla yazılır, sondaki sıfırlar ve nokta atılır.

| Tür | Adresten konuma | Konumdan adrese |
|---|---|---|
| Nominatim | `/search?q=&format=jsonv2&addressdetails=1&limit=[&countrycodes=][&viewbox=b,g,d,k]&accept-language=tr` | `/reverse?lat=&lon=&format=jsonv2&addressdetails=1&zoom=18&accept-language=tr` |
| Photon | `/api?q=&limit=[&lat=&lon=]` | `/reverse?lat=&lon=&limit=1` |
| Pelias | `/v1/search?text=&size=[&focus.point.lat=&focus.point.lon=][&boundary.country=][&api_key=]` | `/v1/reverse?point.lat=&point.lon=&size=1[&api_key=]` |
| ArcGIS | `/findAddressCandidates?SingleLine=&f=json&outSR=4326&outFields=*&maxLocations=[&sourceCountry=][&location=b,k]&langCode=TR&forStorage=[&token=]` | `/reverseGeocode?location=b,k&f=json&outSR=4326&langCode=TR&forStorage=[&token=]` |
| Google | `?address=&language=tr[&region=]&key=` | `?latlng=k,b&language=tr&key=` |

- **Sayı:** etkileşimli aramada 10, toplu işte 5 aday istenir.
- **Ülke:** sağlayıcının ayarıdır (ISO 3166-1 alfa-2). Cihazda ayarı olmayan hazırda TR'dir; ayarda boş bırakılırsa ülke sorulmaz.
- **Yanlılık:** etkileşimli aramada görünümün WGS 84 kutusu ya da ortası verilir (Nominatim'de sınırlamadan, `bounded` yok). Toplu işte
  verilmez.
- **Ayrı alanlar:** sağlayıcıya tek satır olarak gider: “<yol> <kapı>, <mahalle> Mahallesi, <ilçe>, <il>, <posta kodu>”. Boş parçalar
  atılır; mahallede etiket varsa eklenmez.

**Okuma** (`kentos_services::geocode::read`):

- Her yanıttan aday çıkar: yer (boylam, enlem), metin, yol, kapı, bölge adları, posta kodu, ad, sağlayıcının türü ve puanı.
- Bölge adlarından mahalle, ilçe ve il seçilir. Mahallede, listede MAHALLE ya da KÖY etiketi taşıyan ilk ad, yoksa ilk ad alınır.
- Puanın mahalle benzerliğinde bakılan bölge adları, türün sırasıyla:
  - Nominatim: `suburb`, `neighbourhood`, `quarter`, `village`, `hamlet`, `town`, `city_district`, `district`, `county`, `municipality`,
    `city`, `province`, `state`;
  - Photon: `district`, `locality`, `city`, `county`, `state`;
  - Pelias: `neighbourhood`, `borough`, `locality`, `localadmin`, `county`, `region`;
  - ArcGIS: `Nbrhd` (tersinde `Neighborhood`), `District`, `City`, `Subregion`, `Region`;
  - Google: `neighborhood`, `sublocality_level_2`, `sublocality_level_1`, `sublocality`, `administrative_area_level_4`,
    `administrative_area_level_3`, `locality`, `administrative_area_level_2`, `administrative_area_level_1`.
- Photon'un metni parçalardan yazılır: ad, “yol kapı”, `district`, `city`, `state` ve posta kodu; boşlar ve yinelenenler atılır, araya
  virgül konur.
- Bozuk yanıt ve 50'den çok aday reddedilmez, sınırlanır. Okunamayan aday atlanır ve sayılır.

| Tür | Yol / kapı | Mahalle | İlçe | İl | Metin |
|---|---|---|---|---|---|
| Nominatim | `road`, `pedestrian`, `footway`, `cycleway`, `path`, `square` / `house_number` | `suburb`, `neighbourhood`, `quarter`, `village`, `hamlet` | `town`, `city_district`, `district`, `county`, `municipality` | `province`, `state`, `city` | `display_name` |
| Photon | `street` (türü `street` ise `name`) / `housenumber` | `district`, `locality` | `city` (`state`'ten farklıysa), `county` | `state` | parçalardan |
| Pelias | `street` / `housenumber` | `neighbourhood`, `borough` | `county`, `localadmin`, `locality` | `region` | `label` |
| ArcGIS | `StPreType StName StType` (tersinde `Address`'ten `AddNum` atılarak) / `AddNum` | `Nbrhd`, `Neighborhood` | `City`, `District`, `Subregion` | `Region` | `LongLabel`, `Match_addr`, `address` |
| Google | `route` / `street_number` | `neighborhood`, `sublocality_level_1`, `administrative_area_level_4` | `administrative_area_level_2` | `administrative_area_level_1` | `formatted_address` |

**Puan:**

- Çevrimiçi adayın puanı sağlayıcınınkine bırakılmaz. Adayın yolu, kapısı ve bölge adları Q ile §7'nin kuralıyla karşılaştırılır. Mahalle
  benzerliği, adayın bütün bölge adlarının en benzeridir; ilçe ve posta cezası yoktur.
- Yolu benzer değilse:
  - etiketsiz adreste adayın adı benzerse tür **Yer adı**, puan `B − 15`;
  - Q'nun mahalle adına bölge adlarından biri benzerse tür **Bölge**, puan `M − 40`;
  - değilse puan 0.
- Sağlayıcının kendi puanı (ArcGIS'in `score`'u, Nominatim'in `importance`'ı, Pelias'ın `confidence`'ı) ayrıca gösterilir ve yazılır.
- **Liste:**
  - Etkileşimli aramada adaylar KentOS puanıyla, büyükten küçüğe listelenir; eşit puanlılar sağlayıcının sırasıyla. Durum toplu işin
    kuralıyla bulunur (en düşük puan 85).
  - Toplu işte en büyük KentOS puanlısı seçilir (eşitse sağlayıcının önce verdiği). Durum §7'deki gibidir; tanım yoksa en düşük puan
    toplu işin parametresidir.

### 11. Adresleri eşle (İşlemler)

- **Girdi:** tablo. Bir dosya (CSV, TXT, Excel ya da ODS; Tablodan nokta'nın kaynağı, ADR 0223) ya da nesneler; nesnelerde öznitelikler
  satırdır.
- **Adres:**
  - **Tek alan:** bir sütun.
  - **Ayrı alanlar:** Mahalle, Yol, Kapı no, İlçe, İl ve Posta kodu sütunları; en az Yol ya da Mahalle verilmeli.
- **Kaynak:** projenin bir adres verisi ya da açık bir çevrimiçi sağlayıcı (§13).
- **Parametreler:**
  - En düşük puan (varsayılan tanımınki, sağlayıcıda 85);
  - Belirsizleri de yaz (varsayılan açık);
  - çıktı katmanı (yeni, “Adresler”).
- **Çıktı:**
  - Eşleşen (ve istenirse belirsiz) her satır, adayın yerinde bir nokta olur. Satırın dolu hücreleri ve şu öznitelikler yazılır:
    `Durum`, `Puan`, `Eşleşme türü`, `Eşleşen adres`, `Mahalle`, `Yol`, `Kapı no`, `İlçe`, `İl`, `Posta kodu`, `Kaynak`, `Girdi adresi`.
  - Çevrimiçi kaynakta ayrıca `Sağlayıcı puanı` ve `Atıf` yazılır.
  - Eşleşmeyen satırlar raporun tablosundadır (satır, adres, en iyi aday, puan; Kaydet ▾ ile dosyaya). Özette sayılar yazar: eşleşti,
    belirsiz, eşleşmedi, atlanan.
- **Ret:** tek geri alma adımıdır.
  - Projede sistem yoksa ve kaynak çevrimiçiyse reddedilir.
  - Google ile ve saklamasız ArcGIS'le reddedilir.
  - Sağlayıcının toplu iş sınırı aşılırsa reddedilir; iş sunucuya hiç istek göndermeden ret nedenini söyler.
- **Süre:** çevrimiçi iş saniyede bir istekle uzun sürebilir. İlerleme “n / N adres” olarak gösterilir, Durdur o ana dek gelenleri yazmadan
  bırakır.

### 12. Konumlara adres yaz (İşlemler)

- **Girdi:** bir katmanın nesneleri. Noktanın kendi yeri alınır; öbür nesnelerin iç noktası (ADR 0218'in iç noktası).
- **Kaynak:** §11'deki gibi.
- **En büyük uzaklık:** yerel kaynakta tanımın `reach`'i, değiştirilebilir.
- **Çıktı:** her nesneye `Adres`, `Mahalle`, `Yol`, `Kapı no`, `İlçe`, `İl`, `Posta kodu`, `Adres uzaklığı` (metre), `Adres türü` ve
  `Adres kaynağı` yazılır. Var olan değerin üstüne yazılır. Adresi bulunamayan nesnenin bu alanları boşaltılır ve sayılır.
- Tek geri alma adımıdır. Sınırlar ve retler §11'deki gibidir. Google ile ve saklamasız ArcGIS'le reddedilir.

### 13. Arayüz (masaüstü)

- **Alt panelin Adres sekmesi** (`address.search`, Arama'nın yanında):
  - Arama kutusu ve **Ara**. Enter da arar; yazdıkça aramaz.
  - **Kaynak** listesi: projenin adres verileri, açılmış çevrimiçi sağlayıcılar ve kapalı olanlar nedenleriyle; kapalılar seçilemez.
  - **Konumdan adres** düğmesi.
  - Aday tablosu: Sıra, Adres, Tür, Puan, Mahalle, İlçe, Kaynak.
  - **Git:** görünümü adaya getirir ve onu işaretler; işaret kaydedilmez, Veride ara'nınki gibidir. Satıra tıklamak da gider. İki işaret
    birbirinin yerine geçer; İşareti kaldır ikisini de kaldırır.
  - **Nokta ekle:** adayın yerine etkin katmana §11'in öznitelikleriyle bir nokta yazar; tek adımdır (“Nokta ekle”).
  - **Taşı:** seçili tek noktayı adayın yerine götürür ve adres özniteliklerini yazar (yeniden eşleme); tek adımdır (“Taşı”).
  - Nokta ekle ve Taşı'da Durum en iyi adayda aramanın durumudur, kullanıcının seçtiği başka bir adayda **Elle seçildi**. Konumdan adres'in
    sonucunda ve sonucu saklanamayan sağlayıcıda (Google, “Sonuçları sakla”sız ArcGIS) ikisi de kapalıdır.
  - Git, Nokta ekle ve Taşı durum satırının yanındadır: dar pencerede arama kutusu daralmaz.
  - **Durum satırı:** aday sayısı, en iyi eşleşmenin durumu ve puanı, sağlayıcının atfı ve “Adres <sunucu> sunucusuna gönderildi”.
    Konumdan adres'te “Tıklanan yerin adresi”; tablonun Puan sütunu Uzaklık olur.
  - Varsayılan kaynak projenin ilk adres verisidir. Kaynak yoksa sekme Adres verisi'ni ve Adres sağlayıcıları'nı önerir.
- **Konumdan adres** (`address.reverse`): bir araç.
  - Tıklanan (kenet açıkken kenetlenen) ya da yazılan yerin adresi sekmenin kaynağından bulunur, sekmede tek satır olur ve yeri işaretlenir.
  - Araç Esc'e dek sürer. Sekme kapalıysa açılır.
- **Adres verisi** penceresi (`address.data`):
  - Solda tanımlar (Yeni, Sil); sağda ad ve üç bölüm: Kapı noktaları, Yollar, Bölgeler.
  - Her bölümde katman ve alan listeleri vardır. Alanlar katmanın alanlarından ve nesnelerin öznitelik adlarından gelir. Katman seçilince
    alanları tanınır.
  - Eşleşme: en düşük puan, Konumdan adres uzaklığı.
  - Düğme çubuğunun solunda **Alanları tanı** adlardan eşler (`ops::geocode::fields`, §13.1).
  - Yanında **Denetle:** kapı, yol öbeği, mahalle ve bölge sayıları, atlanan ve okunamayan kapılar, aynı yolda aynı kapı. Cevap tanım
    değişene dek pencerenin özet satırlarındadır; form nereye kaydırılmışsa görünür.
  - **Kaydet** `cad.addresses.define` ile yazar.
- **Adres sağlayıcıları** penceresi (`address.providers`):
  - Solda hazırlar ve kurum sunucuları (Sunucu ekle, Sil).
  - Sağda koşulların kartı (§10) ve bağlantıları, onay kutusu ve **Kullan**. Anahtar alanı gizli yazılır, göster düğmesiyle görünür.
  - Ülke; ArcGIS'te “Sonuçları sakla”; kurum sunucusunda tür, adres, istek/s ve aynı anda.
  - **Dene:** bir arama yapar ve süresini söyler. **Önbelleği temizle** kayıt sayısını ve boyutu söyler.
  - Ayarlar cihazdadır: `adres-saglayicilari.json` (biçim `kentos.address-providers`, sürüm 1; §16).
- **İşlemler:** yeni **Adres** kategorisinde Adresleri eşle (`processing.run.address.geocode`) ve Konumlara adres yaz
  (`processing.run.address.reverse`). Kaynak parametresi projenin adres verilerini ve cihazın açık sağlayıcılarını listeler.
- **Şerit:**
  - CBS'de Harita sekmesinde Altlık'tan sonra **Adres** paneli: Adres ara (büyük), Konumdan adres, Adres verisi, Adres sağlayıcıları;
    seyrek kullanılan iki toplu iş panelin ▾ listesinde: Adresleri eşle, Konumlara adres yaz (İşlemler'de de). Harita sekmesi böylece
    arayüzün yazı tipleriyle 3292 px'tir; altı düğmeyle 3400 px'i aşıyordu.
  - CAD'de aynı panel Ekle sekmesinin sonunda.
  - Görünüm › Paneller'e Adres ara.
- **Katalog ek dosyası:** komutların adları, ikonları, açıklamaları ve takma adları `catalog/v1/additions.json`'dadır (ADR 0228 §4, §15).
- **Yeni ikonlar (altı):** `addressSearch`, `addressReverse`, `addressGeocode`, `addressReverseBatch`, `addressData`, `addressProviders`.
  Seçenek sayfasıyla seçilir, sahibe resmi gönderilir.

#### 13.1 Alanları tanı

Bir alan adının **sözcükleri** §3'ün katlanmış sözcükleridir (`KAPI_NO` → `kapi`, `no`), **bitişiği** sözcüklerin aralıksız birleşimidir.
Her rol, katmanın alan sırasıyla, kuralını tutan ve kendinden önceki bir rolün almadığı ilk alanı alır.

- **Kapı noktaları** (rollerin sırası): kod, kapı no, posta kodu, yol, mahalle, ilçe, il, bina adı.
- **Yollar:** ad, mahalle, ilçe.
- **Bölgeler:** ad, mahalle düzeyinde ilçe.

**Kurallar:**

- **Kodlu ad:** sözcüklerinden biri `kod`, `kodu`, `id`, `kimlik`, `tip`, `tipi`, `tur`, `turu` ya da `no` olan addır. Yol, mahalle, ilçe,
  il, bina adı ve bölgenin adı kodlu adı almaz.
- **kod:** bitişiğinde `uavt` geçer ya da sözcüklerinde `adres` ile `kod` ya da `kodu` vardır.
- **kapı no:** sözcüklerinde `ic` yoktur; ve sözcüklerinde `kapi` ya da `numara` vardır, ya da bitişiği `kapino` ile başlar, ya da tek
  sözcüğü `no`'dur.
- **posta kodu:** sözcüklerinde `posta` vardır ya da bitişiği `pk` ya da `postakodu`'dur.
- **yol:** bitişiğinde `yol`, `sokak`, `cadde`, `csbm` ya da `bulvar` geçer. Yollar katmanının adı bu kuralla ya da düz adla eşleşir.
- **mahalle:** bitişiğinde `mahalle` ya da `koy` geçer, ya da sözcüklerinde `mah` vardır.
- **ilçe:** bitişiğinde `ilce` geçer.
- **il:** sözcüklerinde `il` vardır, bitişiğinde `sehir` geçer ya da bitişiği `iladi`'dır.
- **bina adı:** bitişiğinde `bina`, `apartman` ya da `site` geçer.
- **düz ad:** sözcüklerinde `ad`, `adi`, `isim` ya da `name` vardır.
- **Bölgenin adı:** düz ad, ya da düzeyinin kuralı (mahalle, ilçe ya da il).

### 14. Komutlar ve otomasyon

- **`cad.addresses.define` v1:**
  - **İşlemler:** `set` (tanım, aynı kimliklinin yerine ya da sona) ve `remove` (kimlik).
  - **Retler** (bu sırayla): `no_locator`, `no_id`, `invalid_locator` (`address_locators_problem`), `invalid_revision`,
    `revision_conflict`, `unknown_locator`. Var olmayan katman `unknown_layer` uyarısıdır.
  - Proje ayarıdır: çizimi kirletir, geri alma adımı değildir (`cad.network.define` gibi).
  - Ortak durumlar `fixtures/commands/v1/cad.addresses.define.json`, sözleşmenin kurallarından bağımsız betikle yazılır.
- **Python:** `kentos.cad.addresses.define` katalogdan üretilir. `kentos.address`, projenin adres verisinde arar:
  - `parse(text)`: §4'ün parçaları;
  - `locate(doc, text | parçalar, locator=None)`: adaylar ve durum;
  - `reverse(doc, (doğu, kuzey), locator=None, reach=None)`.

  Hesap çekirdektedir (`kentos._native`). Çevrimiçi sağlayıcı Python'da yoktur (§1).
- **MCP:** `cad.addresses.define` katalogdan gelir; `remove` bir tanımı sildiği için yıkıcı işaretlidir (`cad.network.define` gibi).
- **Sunucu:** `project.changes`'in proje ayarları denetimi `address_locators_problem`'ı da sorar.

### 15. Katalog ek dosyası (`catalog/v1/additions.json`)

ADR 0228 §4'ün dosyası; ilk tüketicisi bu maddedir. Türleri ve kuralları `kentos_contracts::ui_catalog`'dadır; ts-rs ile TS türleri de
üretilir.

- **Biçim:** `format` `kentos.catalog-additions`, `version` 1. Üç bölüm vardır:
  - `commands`: kimlik, başlık, kısa ad, ikon, açıklama, takma adlar, kısayollar, kategori, durum (`implemented` ya da `pending`);
  - `icons`: ad ve envanterin biçiminde SVG: web setinin 20 × 20'lik ızgarasındaki öğeler, `<svg>` öğesi olmadan (çizgi kalınlığı 1,4
    birim, uçlar yuvarlak; çizen bunu verir);
  - `ribbon`: türler (`cad`, `gis`), sekme, ve ya yeni panel (ad, ikon, `after`: hangi panelden sonra, öğeler) ya da var olan panele
    eklenen öğeler (`into`). Öğeler web'in biçimindedir: komut ve boy. İsteğe bağlı `overflow`: panelin başlığının yanındaki ▾ listesine
    giden seyrek komutlar (envanterin `overflow`'u).
- **Kurallar** (masaüstünün testi denetler):
  - kimlikler tekil ve web envanterinde yok;
  - başvurulan her ikon dosyada ya da web'in ikon setinde var;
  - her sekme o türün şeridinde var;
  - `after` ve `into` panelleri o sekmede var, yeni panelin adı sekmede tekil;
  - öğelerin ve ▾ listesinin komutları envanterde ya da dosyada var, bir yerde bir kez;
  - boylar `large`, `lead`, `small`;
  - ikonun SVG'si envanterin biçimindedir (`<svg>` öğesi yok) ve masaüstünün SVG okuyucusu her öğesini çizer.
- Masaüstü dosyayı envanterle birlikte gömer. Komutları masaüstünde çalışır sayılır; `PORTED` yalnız web komutlarıdır.

### 16. Cihazdaki dosyalar

- `adres-saglayicilari.json` (`$XDG_CONFIG_HOME/kentos-cad/`): sağlayıcıların ayarları, çizime yazılmaz.
  - Her kayıt: kimlik, kurum sunucusunda tür, ad ve adres, `enabled`, onaylanan koşul sürümü ve zamanı, ülke, `store`, `rate`, `parallel`.
  - Kurallar `kentos_services::geocode::providers`'tadır. Okunamayan dosya `adres-saglayicilari-okunamadi-<zaman>.json` olarak
    ayrılır.
- Anahtarlar `baglantilar.json`'dadır (§10).
- Önbellek `$XDG_CACHE_HOME/kentos-cad/adres/`'tedir: en çok 50 MB; en uzun süredir kullanılmayan önce silinir.

### 17. Performans

**Bütçeler** (release, tek iş parçacığı; sentetik şehir: 500 mahallede 10 000 yol ve 200 000 kapı, her yolun orta hattı, 500 mahalle
alanı):

| İş | Bütçe |
|---|---|
| Dizinin kurulması | ≤ 300 ms |
| Tek adres, tam eşleşme | p50 ≤ 20 µs |
| Tek adres, bir harf yanlış, mahallesiz | p50 ≤ 2 ms |
| 100 000 adreslik toplu eşleme (%10'u bir harf yanlış) | ≤ 3 s |
| 100 000 noktaya konumdan adres | ≤ 1 s |
| 100 000 adresin parçalarına ayrılması | ≤ 200 ms |

**Nerede çalışır:**

- Masaüstünde dizin ve aramalar Adres sekmesinin iş parçacığındadır; dizin, çizimin sürümü ve tanım değişmedikçe yeniden kurulmaz.
  Çevrimiçi istekler bulut istemcisinin çalışma zamanındadır (`kentos_cloud::fetch`). Arayüz iş parçacığı beklemez.
- İşlemler araçları işlemlerin iş parçacığındadır (ADR 0124).

## Uygulama

Katman katman (web ADR 0228 gereği yapılmadı; dönüşte gerekenler TODOS.md §0.2'de):

- **Sözleşme** (`kentos_contracts`):
  - `address`: `AddressLocator`, `DoorSource`, `StreetSource`, `AreaSource`, `AreaLevel` ve kuralları (`address_locators_problem`,
    `sanitized_address_locators`, `next_address_locator_id`); `ProjectSettings.address_locators`.
  - `cad_addresses`: `cad.addresses.define` v1, katalogda (yalnız masaüstü ev sahibi, geri alma adımı değil).
  - `ui_catalog`: katalog ek dosyasının türleri ve kuralları (`CatalogAdditions::problem`, `KnownCatalog`).
  - `FORMATS_VERSION` 54. TS türleri üretildi; web'in `version.ts`'i web dururken 53'te kalır.
- **KCAD:** şema 42 (`SCHEMA_WITH_ADDRESSES`), ayarların `addressLocators` anahtarı; `encode/addresses.rs`, `decode/addresses.rs`;
  spesifikasyon `docs/specs/kcad-v2.md`; bağımsız Python okuyucu (`tools/kcad/kcad.py`) ve yazıcı (`kcad_v2_reference.py`); örnek
  `fixtures/kcad/v2/addresses.kcad` ve 19 bozuk dosya.
- **Çekirdek** (`kentos_geometry_core::ops::geocode`):
  - `fold`, `parse` (parçalar, kapı anahtarı, ayrı alanlar), `similar` (Levenshtein, ad dizini: tam eşleşme, uzunluk kovaları ve
    harf kümesi ön süzgeci), `locator` (kapı öbekleri, kapısız yol öbekleri, alanlar, R-ağaçları), `forward` (adaylar önce yalnız
    sıralama anahtarlarıyla kurulur, listeye giren en çok 20'si yazılır), `format`, `reverse`, `fields` (§13.1), `check` (Denetle).
  - Ön süzgeç kesindir: bir düzenleme harf kümesine en çok bir harf ekler ve bir harf çıkarır; kümeleri `2k`'dan çok farklı iki ad `k`
    düzenlemeden uzaktır. Hiçbir benzer ad atlanmaz (bağımsız başvuru bunu denetler).
- **Servisler** (`kentos_services::geocode`): `providers` (hazırlar `fixtures/geocode/v1/providers.json`'dan, cihazın ayarları, kart ve
  onay, hız, toplu iş ve saklama kuralları), `request`, `read`, `score`, `batch`.
- **Uygulama katmanı:**
  - `kentos_native_application::addresses` (çizimden dizinin girdisi, katman süzgeçleri uygulanarak) ve `address_define`.
  - `kentos_processing::address`: kaynaklar, ev sahibinin `AddressHost`'u, `Match` (bulunan adres: satırı ve noktanın öznitelikleri),
    `Plane` (WGS 84 ile projenin sistemi arası), `match_attributes`. İşlemler'in `ParamKind::AddressSource`'u, `Feedback::addresses`;
    araçlar `builtin/address/` (`address.geocode`, `address.reverse`), yalnız arka planda.
  - `kentos_interaction::address_reverse` (araç, `ViewChange::AddressAt`) ve `address_form` (Adres verisi'nin form kuralları).
- **Masaüstü:**
  - `catalog/v1/additions.json` ve okuyucusu (`catalog.rs`): altı komut, altı ikon, üç şerit yeri.
  - `address/`: `mod.rs` (sekmenin durumu ve akışı, işaret, iz denetimleri), `engine.rs` (adreslerin iş parçacığı), `online.rs` (cihazın
    sağlayıcıları: `adres-saglayicilari.json`, anahtarlar `baglantilar.json`'da `adres:<sağlayıcı>`, önbellek, hız, Google kuralı),
    `view.rs` (sekme), `data.rs` (Adres verisi), `providers.rs` (Adres sağlayıcıları), `tests.rs`.
  - Alt panelin Adres sekmesi (`fixtures/shell/v1/log.json`, `layout.json`), İşlemler'in Adres kaynağı alanı ve arka plandaki ev sahibi,
    izlerin `address` beklentisi ve `panel` adımında Adres sekmesi.
- **Sunucu:** proje oluşturma ve `project.changes`'in ayarları `address_locators_problem`'ı sorar.
- **Otomasyon:** Python `kentos.address` (`parse`, `Locator`, `locate`, `reverse`, `define`, `remove`; yerel `address_parse` ve `Locator`),
  `kentos.cad.addresses.define` katalogdan; MCP'de `cad.addresses.define` yıkıcı işaretli.
- **İkonlar** (seçenek sayfasıyla sahibe gönderildi, A'lar seçildi): `addressSearch` (iğne ve büyüteç), `addressReverse` (nişangâh ve adres
  levhası), `addressGeocode` (liste ve iğne), `addressReverseBatch` (noktalar ve liste), `addressData` (mahalle sınırı, yol ve kapılar),
  `addressProviders` (küre ve iğne).
- **Sonraya kalan:** sağlayıcılarla canlı deneme bu maddede yapılmadı (aşağıda).

## Doğrulama

- **Bağımsız başvurular** (KentOS kodu olmadan):
  - `scripts/fixtures/geocode_cases.py` → `fixtures/geocode/v1/cases.json`: 8 katlama, 12 ayrıştırma, 11 benzerlik, 8 alan tanıma; metre
    ve milimetre sahnesinde 22'şer adres, 4'er ayrı alanlı adres, 7'şer konum. Çekirdek bire bir aynı (yerler 1e-6 içinde).
  - `scripts/fixtures/geocode_online_cases.py` → `online.json`: 60 istek adresi, 11 yanıt (Nominatim ve Photon'unkiler gerçek, bir kez
    alındı, ODbL atfıyla), 8 puanlama, toplu işin tekilleştirmesi, 4 tek satır.
  - `scripts/fixtures/address_rules_cases.py` → `rules.json`: 31 tanım ve 8 liste kuralı.
  - `scripts/fixtures/address_define_command_cases.py` → `fixtures/commands/v1/cad.addresses.define.json`: 22 durum.
  - `scripts/fixtures/address_processing_cases.py` → `fixtures/processing/v1/address.*`: 8 durum (dosyadan tek alan, en düşük puan 95,
    katmandan ayrı alanlar, konumlara adres 100 m ve 3 m, üç ret).
  - `scripts/fixtures/address_scene.py` → `fixtures/interaction/v1/addresses.kcad`, `address-search.json`, `address-reverse.json`.
- **`.kcad`:** `kcad_v2_reference.py --check` 606 dosya; Rust kodeği adres verisini gidip gelir, bozuk 19 dosyayı nedenleriyle reddeder.
- **Testler:**
  - Rust: sözleşme (adres kuralları, katalog ek dosyası), KCAD (4), çekirdek (5 başvuru ve 10 birim testi), servisler (4 ve birim
    testleri), ürün komutu (22 durum, başsız sunucu ve Python'da da), İşlemler (8 durum ve sahte ev sahibiyle çevrimiçi yol),
    `address_form` (3), masaüstü (sekme, Nokta ekle ve Taşı, Konumdan adres, dizinin yeniden kurulması, Adres verisi, kaynaklar,
    sağlayıcılar penceresi; çevrimiçi ev sahibinin kuralları, önbelleği ve hızı; katalog ek dosyası).
  - İzler: `address-search` ve `address-reverse` masaüstünde üç çeşitte (us, tr-q, hidpi) geçer.
  - Sunucu: `cargo test -p kentos-api -p kentos-application` geçici bir PostGIS kabında `KENTOS_TEST_DB=required` ile (adres verisinin
    HTTP'den denetimi dahil); kap sonra silindi.
  - Python: `test_address.py` (başvurunun iki sahnesi `kentos.address` ile; ayrıştırma; retler) ve komut durumları.
- **Görsel inceleme** (1440×900 ve 1100×650, açık ve koyu; `.run/shots/arac-adres-*`): 1100 px'te arama kutusu daralıyor ve Taşı
  kesiliyordu (Git, Nokta ekle ve Taşı durum satırının yanına alındı); Puan sağa yaslanmıyordu; yol orta hatları koyu zeminde
  görünmüyordu (sahnenin rengi); Kaynak listesinde nedenler sağlayıcı adlarını kesiyordu (kısaltıldı); konumdan adreste Puan başlığı
  Uzaklık oldu; Adres sağlayıcıları'nda Kaydet yalnız değişiklik varken açık.
- **Süreler** (release, tek iş parçacığı; Apple M5, 10 çekirdek, 32 GB, macOS 26; Rust 1.96; `cargo test --release -p
  kentos-geometry-core --test all geocode::timing -- --ignored --nocapture`; sentetik şehir: 500 mahalle, 10 000 yol, 200 000 kapı,
  2 000 yol adı her biri beş mahallede):

  | İş | Ölçüm | Bütçe |
  |---|---|---|
  | Dizinin kurulması | 179 ms | ≤ 300 ms |
  | Tek adres, tam eşleşme (p50) | 17,6 µs | ≤ 20 µs |
  | Tek adres, bir harf yanlış, mahallesiz (p50) | 3 µs | ≤ 2 ms |
  | 100 000 adreslik toplu eşleme (%10'u bir harf yanlış) | 1,53 s (96 740 eşleşti) | ≤ 3 s |
  | 100 000 noktaya konumdan adres | 0,33 s | ≤ 1 s |
  | 100 000 adresin parçalarına ayrılması | 51 ms | ≤ 200 ms |

  İlk ölçümde tam eşleşme 102 µs, toplu iş 9,6 s'ydi. Benzer ad taramasında ayırmasız Levenshtein, önceden hesaplanan numaralı ad
  bilgisi, harf kümesi ön süzgeci ve adayların iki geçişte kurulmasıyla bütçeye indi; sonuçlar başvuruyla bire bir aynı kaldı.
- **Ölçülmeyenler:** çevrimiçi sağlayıcılara uygulamanın kendi istemcisiyle canlı istek bu maddede gönderilmedi (Nominatim'in kullanım
  ilkesine yük bindirmemek için; gerçek yanıtlar fixture'larda). Web yapılmadı (ADR 0228).
- **Açık kalan (dağıtım):** OSMF'nin Nominatim ilkesi hızı uygulama başına sayar: bütün kullanıcıların toplamı saniyede bir isteği
  geçmemelidir. Uygulama, OSMF isterse hizmeti yazılım güncellemesi olmadan değiştirebilmeli; mümkünse bir vekil ve ortak önbellek
  kurmalıdır. KentOS'un sınırı cihaz başınadır. Hazırlar kapalı başlar, koşullar onaylanınca açılır; kurum sunucusu güncellemesiz eklenir.
  Çok kullanıcıya dağıtımdan önce hazır Nominatim ve Photon için `kentosd` vekili ve ortak önbellek mi, kurumun kendi sunucusu mu
  kullanılacağı sahibin kararıdır (TODOS.md `GIS-40`).
