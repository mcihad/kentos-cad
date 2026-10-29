# ADR 0736: Adres verisini adres kaydıyla (UAVT) karşılaştırmak

- **Durum:** kabul edildi (2026-10-29). `plan/gis.md` CBS-34'ün altıncı dilimi. Kapsamı ben belirledim.
- **Kalan iş:** ADR 0579 ve 0647'den kalan “adres kodunun UAVT gibi bir dış kayıtla karşılaştırılması”.
- **Bağlam belgesi:**
  - ADR 0229 (adres verisi, dizin, adresin anahtarları);
  - ADR 0579 ve 0647 (`cad.addresses.check`, `cad.addresses.normalize`);
  - ADR 0184 §4 ve 0223 §1 (tablo dosyası okuyucusu, İşlemler'in sınırları);
  - ADR 0253 §8 (`cad.changeset.apply`).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **`.kcad`:** şema değişmez.
- **Bağımlılık:** yeni üçüncü taraf kitaplık yok.
- **Sonraya kalan:**
  - kaydın kendi koordinat sistemi (bugün kaydın X ve Y'si projenin sistemindedir; başka sistemdeki kayıt önce
    dönüştürülür);
  - kayıtta olup çizimde olmayan adres için kapı noktası önermek;
  - ifraz, tevhit, hisse ve PlanGML (mevzuat işi, sahibin kararıyla sona kalır);
  - gerçek kullanıcılarla günlük kullanım kabulü.

## Bağlam

Belediyeler adres verisini Ulusal Adres Veri Tabanı'yla (UAVT) birlikte tutar. UAVT'yi Nüfus ve Vatandaşlık İşleri Genel
Müdürlüğü'nün Adres Kayıt Sistemi (AKS) yürütür. Numaralandırmayı ve güncellemeyi yetkili idare (belediye) yapar.

Adresin bileşenleri yönetmeliktedir: il, ilçe, mahalle, yol adları, sabit tanıtım numarası, bina (dış kapı) numarası ve
posta kodu. Adres kodu (UAVT kodu) on hanelidir ve bağımsız bölüm başınadır. Bu yüzden bir binada birden çok kod olur. Kod,
yolun adı ya da kapı numarası değişse de aynı bölümü gösterir.

Kurumların elinde bu kaydın bir dışa aktarımı vardır: adres numarası, yol, dış kapı, mahalle, ilçe, çoğu zaman koordinat.
Biçimi CSV, Excel ya da ODS'dir. KentOS'un adres verisi (ADR 0229) kapı noktalarının adres kodunu bir alanda tutabilir.
`cad.addresses.check` (ADR 0579) yalnız çizimin kendi içine bakar: yinelenen kapı, birden çok kapıdaki kod, farklı
yazılışlar. Çizimdeki kapıların kayıtla uyuşup uyuşmadığını söyleyen bir yol yoktu.

KentOS AKS'ye bağlanmaz: herkese açık bir arayüzü yoktur ve veri kişiseldir. Karşılaştırma kurumun kendi dışa aktarımıyla
yapılır.

## Karar

### §1 Çekirdek: `ops::geocode::register`

Karşılaştırma, adres verisinin dizinini (`Locator`) kayıt satırlarıyla karşılaştırır. Kayıt satırının alanları:

- satır numarası (1'den);
- kod;
- isteğe bağlı yol, kapı no, mahalle, ilçe ve yer.

Bulgular:

1. **Kayıtta olmayan kapı:** kodu hiçbir satırda olmayan kapı.
2. **Farklı yazılan adres:** kodu bir satırda olan, ama yolu, kapı numarası, mahallesi ya da ilçesi satırdakinden başka
   olan kapı. Karşılaştırma dizinin anahtarlarıyla yapılır:
   - “Atatürk Blv.” ile “ATATÜRK BULVARI” aynı yoldur; yazılış farkı `cad.addresses.normalize`'ın işidir.
   - Yolun türü (cadde, sokak, bulvar) yalnız ikisi de söylüyorsa karşılaştırılır.
   - Kapı numarası kapının anahtarıyla (“12/A”, “12 A”) karşılaştırılır.
   - Mahalle MAHALLE ve KÖY etiketleri olmadan, ilçe sözcükleriyle karşılaştırılır.
   - Kapının boş yazdığı ama satırın söylediği parça farktır. Satırın boş bıraktığı parça karşılaştırılmaz.
3. **Uzak kapı:** satırın yerinden toleranstan uzak kapı. Uzaklık metre cinsindendir: projenin birimiyle bölünür.
4. **Çizimde olmayan adres:** kodu hiçbir kapıda olmayan ve yolunda o numarada kapı bulunmayan kayıt satırları.
   - Satırlar yol, kapı, mahalle ve ilçe anahtarlarıyla gruplanır. Bir binanın birden çok bağımsız bölümü bu yüzden tek
     adrestir; kodları ve satırları birlikte söylenir.
   - Bir kapı adresi taşıyor sayılır, eğer yolu ve numarası aynıysa ve mahalle ile ilçe uyuyorsa (ikisi de aynı, ya da
     biri söylenmemiş).
   - Yolu ya da kapı numarası okunmayan satır bu listeye girmez.
5. **Kayıtta yinelenen kod:** birden çok satırdaki kod. Kapı ilk satırla karşılaştırılır.

Kapı listeleri çizimin sırasıyla, kayıt listeleri ilk satırlarıyla sıralanır.

### §2 `cad.addresses.compare` v1

Salt okunur komuttur.

**Girdi:**

- `locator`: adres verisinin kimliği; verilmezse projenin ilki.
- `register`: kaydın yeri ve sütunları.
  - Yer: bir tablo dosyası (`path`, mutlak ya da çizimin klasöründen göreli; çalışma kitabında `sheet`) ya da çizimin bir
    katmanı (`layer`); ikisinden biri.
  - Sütunlar: `code` zorunludur; `street`, `door`, `neighbourhood`, `district`, `x`, `y` verildikleri yerde karşılaştırılır.
  - Mahalle ve ilçe yalnız adres verisinin kapılarında o alan varsa karşılaştırılır. Yoksa kapının boş metni her satırda
    fark sayılırdı.
- `tolerance`: metre; varsayılan 10, en çok 10 000.
- `limit`: her listenin ilk kaçı; 1–10 000, varsayılan 100. Pencere bütün farkları yazabilsin diye üst sınır 10 000'dir.
- `drawing`: çizimin dosyası, göreli yol için.

**Kayıt okuma:**

- Dosya, İşlemler'in tablo okuyucusuyla okunur (ADR 0223 §1): CSV ya da metin (ayırıcı kendiliğinden bulunur), XLSX, ODS;
  250 000 satıra kadar. Fazlası okunmaz ve `register_cut` uyarısı verilir.
- İlk satır sütunları adlandırır. Kodu boş satır alınmaz, sayılır.
- Sayılar şu biçimdedir: isteğe bağlı işaret, tek ondalık noktalı rakamlar, isteğe bağlı üs. Yalnız virgül varsa ondalık
  virgül sayılır (Türkçe dışa aktarımlar için).
- Katmandan okunan kayıtta satırlar katmanın nesneleridir, çizimin sırasıyla. Sütunlar katmanın alanları ve nesnelerin
  öznitelik adlarıdır. `x` ile `y` verilmezse noktanın yeri alınır. Katmanın satırları cevapta nesnelerin kalıcı
  kimlikleriyle de verilir.

**Çıktı:** sayılar (kapılar, kodlu kapılar, kodu kayıtta olanlar, kodlu ve kodsuz satırlar) ve beş liste:
`unregisteredDoors`, `differences` (parça parça: çizimdeki ve kayıttaki metin), `distances`, `undrawnAddresses`,
`registerRepeats`. Her biri önce sayısını, sonra ilk `limit` tanesini verir. Kapılar nesnelerin kalıcı kimlikleriyle verilir.

**Retler, sırasıyla:**

- `invalid_limit`, `invalid_tolerance`;
- `register_missing`: yer verilmedi ya da ikisi birden verildi;
- `invalid_input`: katmanla sayfa, boş kod sütunu;
- `no_address_data`, `unknown_locator`;
- `no_address_codes`: kapılarında kod alanı yok;
- `invalid_file_path`, `file_not_found`;
- `register_unreadable`: tablo olarak okunmuyor, satırı yok ya da o adda sayfası yok; neden okuyucunun sözüyle;
- `layer_not_found`;
- `register_column_missing`: ileti kaydın sütunlarını sayar.

Yeni kodlar: `register_missing`, `no_address_codes`, `register_unreadable`, `register_column_missing`, `register_cut`
(uyarı).

### §3 Masaüstü: Adres kaydıyla karşılaştır

Şeridin Adres panelinin taşma listesinde, CAD ve CBS'de; ikonu yeni (`addressCompare`): solda bir tablo, sağda bir
konum işareti, altta iki yönlü ok.

**Pencere:**

- Adres verisi seçilir: yalnız kapılarında kod alanı olanlar.
- Kayıt: Tablo dosyası ya da Çizimdeki katman. Dosya seçilince ilk satırı okunur; çalışma kitabında sayfa seçilir.
- Sütunlar adlarından tahmin edilir. Adres verisinin Alanları tanı kuralı kullanılır (`fields::doors`); kaydın
  “ADRES_NO”'su da kod sayılır. X ve Y “X”, “Y”, “sağa”, “yukarı”, “doğu”, “kuzey” adlarından tanınır.
- Tolerans metre olarak yazılır.

**Sonuç:**

- Karşılaştır komutu kendi iş parçacığında, çizimin kopyasında çalıştırır (nesneler paylaşılır, ADR 0654): çeyrek
  milyon satırlık kayıt bir saniyeye yakın sürer, pencere donmaz. Sürerken “Karşılaştırılıyor…” yazar, iki düğme
  kapalıdır; cevap yalnız son sorunun numarasıyla gelirse alınır, eskisi atılır.
- Özet satırı sayıları söyler, sonuç bölümleri sayılarıyla gelir. Bulgusu olan ilk liste öne gelir.
- Satıra basınca kapı seçilip gösterilir; katmandan okunan kayıtta satırın nesneleri.

**Farkları kayıttan yaz:** farklı yazılan her kapıya kaydın yol, kapı no, mahalle ve ilçe metinlerini yazar. Yalnız farklı
olan parçalar ve adres verisinin o parçaya alanı olan kapılar yazılır. Bu bir değişiklik kümesidir: kapı başına bir
`cad.entities.set` adımı, tek `cad.changeset.apply`, adı “Adres kaydından yaz”, tek geri alma adımı. Kilitli katman
yazmayı durdurur ve söylenir. Yazmadan sonra karşılaştırma yinelenir.

## Doğrulama

- **Ortak durumlar:** `scripts/fixtures/address_compare_cases.py` KentOS kodu olmadan yazar.
  - Dizin adres başvurusunun kendi kurucusuyla kurulur (`geocode_cases.py`'nin `build`'i). Karşılaştırmanın kuralı bu
    ADR'den yazılmıştır.
  - Kayıt dosyaları da betikten yazılır (`fixtures/geocode/v1/register/`): BOM'lu noktalı virgüllü CSV, sekmeli metin,
    boş dosya, eski Excel başlığı.
  - **35 durum:** tam karşılaştırma, plan, sekmeli dosya, sınır 1 ve 10 000, toleranslar, yalnız kod sütunu, yer vermeyen
    kayıt, mahallesiz adres verisi, kayıt katmanı (yeri noktalarından, satırları kimlikleriyle), yalnız kodlu katman,
    çizimin klasöründen göreli yol, doğrulama ve 21 ret.
  - `KENTOS_CASES_ONLY=cad.addresses.compare cargo test -p kentos-native-application --test all every_command_case` hepsini
    geçer; Python da aynı durumları oynatır. Yetenek testi yeni kodları durumlarda bulur.
- **Çekirdek:** `ops::geocode::register`'ın birim testi.
- **Uygulama:**
  - `addresses_compare::tests` sayıların biçimini sınar.
  - `tests/all/addresses_compare.rs` 250 005 satırlık kaydın `register_cut` ile kesildiğini sınar.
- **Masaüstü:** `address_compare_scenes::tests` şunları sınar:
  - pencere açılır, kayıt seçilince sütunlar tahmin edilir, karşılaştırılır;
  - farklar tek adımda yazılır ve karşılaştırma farksız döner;
  - geri al dört kapıyı geri getirir.
- **Python ve MCP:** SDK yeniden üretildi; komut MCP'de salt okunur araçtır.
- **Resimler:** `KENTOS_SHOTS_ONLY=adres-kayit,adres-kayit-yazildi cargo test -p kentos-desktop tools_screens --
  --ignored --nocapture` (`.run/shots/arac-adres-kayit-*`). 1440×900 ve 1100×650, iki temada.

## Ölçümler

Release, bu makinede (`cargo test --release -p kentos-native-application --test all addresses_compare::compare_timing --
--ignored --nocapture`): 100 000 kapı, 250 000 satırlık CSV kaydı.

| İş | Süre (ortanca) |
|---|---|
| `cad.addresses.compare`: 17 MB'lık CSV'nin okunması, dizin, karşılaştırma | 871 ms |

Sonuç: 100 000 kapının hepsi kayıtta; 1 000 kapı yerinden uzak, 150 000 adres çizimde yok, farklı yazılan yok. Sürenin
çoğu dosyanın okunmasıdır; karşılaştırma kodları bir kez sözlüğe koyar, kapı başına bir arama yapar. Pencerede bu iş
kendi iş parçacığındadır.

## Kaynaklar

- Adres ve Numaralamaya İlişkin Yönetmelik (Resmî Gazete 26245, 31 Temmuz 2006): adresin bileşenleri, numaralandırma.
- Adres Kayıt Sistemi Yönetmeliği (2006), m. 4: adres bileşenleri, Ulusal Adres Veri Tabanı.
- NVİ, Adres Kayıt Sistemi Uygulama Yönergesi.
- UAVT adres kodunun bağımsız bölüm başına on haneli olması: DASK'ın adres kodu açıklamaları (ikincil kaynak).
