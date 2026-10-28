# ADR 0726: Blok kitaplığı — DWG/DXF klasörleri, gerçek boy, birim ve boyut varyantı

- **Durum:** kabul edildi (28 Ekim 2026); ARCH-LIB-01'in kalan alt maddeleri.
- **Bağlam:** `plan/architecture.md` ARCH-LIB-01. Dayandığı kararlar:
  - ADR 0702 (KentOS'un gerçek boylu mobilya ve vitrifiye blokları);
  - ADR 0315 §4 (bloğun birimi);
  - ADR 0316 §3 (`file.import.block`);
  - ADR 0318 (dinamik bloklar);
  - ADR 0257 (DXF'in birimi);
  - ADR 0586 (DWG kodeği).
- **Web:** ertelendi (ADR 0228). TODOS.md §0.2'de.

## Bağlam

ADR 0702 KentOS'un kendi kitaplığını gerçek boylu bloklarla doldurdu. Üç madde açık kaldı:

- **DWG kitaplıklarının kategoriye alınması.** Bir ofisin elindeki mobilya ve vitrifiye blokları çoğu zaman DWG ya da
  DXF dosyalarında, klasörlerde durur. KentOS onları ancak Başka çizimden al ile, kategorisiz ve birim sorusu olmadan
  alabiliyordu. `file.import.block` yalnız `.kcad` okuyordu.
- **Önizlemede fiziksel en/boy etiketi.** Blok ekle hayaleti gösteriyordu, ama ne yerleşeceğini söylemiyordu. Gerçek
  en ve boy, bloğun birimi, taban noktası ve dönüş görünmüyordu.
- **m/mm/DXF kabul matrisi.**

Planın kuralı: kapıdan büyük yatak üretecek belirsiz bir blok birimi olmasın. Birim bildirmeyen bir DWG'nin sayıları
milimetreyken metre sanılırsa 2000 m'lik bir yatak gelir. ADR 0624 bir çizimi açarken böyle bir dosyayı milimetre
varsayıp raporlar; bir bloğu başka bir çizime almak ise boyu doğrudan değiştirir.

### Araştırma (üretici belgeleri)

- [AutoCAD 2024 — Blocks Palette](https://help.autodesk.com/cloudhelp/2024/ENU/AutoCAD-LT/files/GUID-E3413A03-8EEF-44D5-B5C7-8B345A9257FE.htm)
  ve [2026 — To Insert a Block Using the Libraries Tab](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-LT-MAC/files/GUID-9DA4A49D-79E9-486F-96BD-24A9607AD7BF.htm):
  - Sekmeler: Current Drawing, Recent, Favorites ve Libraries.
  - Libraries bir çizimin ya da bir klasörün blok tanımlarını gösterir. Klasörde bir çizime çift tıklayınca o çizimin
    blokları açılır; "Insert DWG as Block" çizimin tamamını tek blok yapar.
  - Yerleştirme seçenekleri altta: Insertion Point, Scale (eksi ölçek aynalar), Rotation, Repeat Placement, Explode.
  - Önizlemede dinamik bloğun işareti vardır.
- [AutoCAD 2027 — About Block Units and Insertion Scale](https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-LT-MAC/files/GUID-6C46049D-8636-442D-8BAC-CF4FD515FDC0.htm)
  ve [INSUNITS](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-LT/files/GUID-A58A87BB-482B-4042-A00A-EEF55A2B4FD8.htm):
  - Bloğun birimi yerleştirmedeki ölçeği belirler.
  - Birimsiz kaynak için kullanıcının açıkça verdiği "Source Content Units" geçersiz kılması kullanılır.
  - Birimler tutarsızsa bloklar 0,0394, 12 ya da 1000 kat yanlış boyda gelir.

**KentOS kararı:**

- AutoCAD'in Libraries iş akışı alınır.
- Birimsiz kaynakta geçersiz kılma bir aygıt ayarı değildir. Dosya başına, kullanıcı boyları görerek açıkça seçer.
- Seçim yapılmadan KentOS hiçbir boy söylemez ve hiçbir şey almaz.

## Karar

### 1. Bloklar › Kitaplık

Bloklar panelinin başında **Bu çizim | Kitaplık** seçimi vardır (AutoCAD'in Current Drawing ve Libraries'i).

- **Kitaplıklar:**
  - + düğmesi bir klasörü kitaplık yapar.
  - Kitaplıklar cihazdadır: son dosyaların yanında `blok-kitapliklari.json` (`kentos.block-libraries`, sürüm 1).
  - Sağ tıkta "Listeden çıkar"; dosyalara dokunulmaz.
- **Klasör:**
  - Alt klasörleri ve DWG, DXF, KCAD çizimleri doğal sırada listelenir; nokta ile başlayanlar gizlidir.
  - Üstte Kitaplıklar › kitaplık › klasör … yolu vardır; her parça oraya döner. Arama bulunulan yeri süzer.
- **Bir çizim açılınca:**
  - Önce "Dosyanın tamamı" gelir, ardından blokları.
  - Her satırda bloğun resmi, gerçek en × boyu (çizimin biriminde) ve birimi yazılıdır.
  - Okuma çizimin iş parçacığı dışında, `cad_open::read_in` ile yapılır (Aç'ın okuyucusu, ADR 0624).
- **Birim bildirmeyen DWG/DXF:**
  - "Bu dosya birimini bildirmiyor; blokların boyu birimi seçilince söylenir" der ve mm, cm, m, inç, fit sunar.
  - Seçilmeden boy yazılmaz, Yerleştir kapalıdır.
  - Seçim oturum boyunca o dosyanındır.
- **Yerleştir** (basma ya da sağ tık):
  - `file.import.block` ile alır, kategori kitaplıktandır:
    - bir blok: kitaplığın adı / alt klasörler / dosyanın adı (`kitaplik/yatak-odasi/yatak-odasi`);
    - dosyanın tamamı: kitaplığın adı / alt klasörler.
  - Sonra Blok ekle o blokla başlar.
  - Çizimde aynı adlı blok varsa çizimdeki kullanılır ve bu söylenir (Başka çizimden yeniden tanımla… ile değiştirilir).
- **Dosyadan blok ekle** artık DWG ve DXF de seçer. Birim bildirmeyen dosyayı Kitaplık'ta açar; birim orada sorulur.

### 2. `file.import.block`: DWG, DXF, birim, kategori ve boylar

Sözleşme v1'de kalır; alanlar eklenir.

- **`path`:** `.kcad` (v2 ya da v1 JSON), `.dxf` ya da `.dwg`. CAD dosyası `cad_open::read_in` ile okunur:
  - sayılar metreye çevrilir;
  - bloğun birimi BLOCK_RECORD 70'ten, yoksa dosyanın biriminden gelir (ADR 0315 §4, 0257 §2).
- **`sourceUnit`** (mm, cm, m, in, ft):
  - CAD dosyasının sayıları bu birimde okunur, dosya ne bildirirse bildirsin.
  - Birim bildirmeyen dosya bu alan olmadan **`source_unit_unknown`** ile reddedilir.
  - KentOS çizimi birimini taşır; `sourceUnit` verilirse `invalid_input` ile reddedilir.
- **`category`:**
  - Alınan bloklardan kategorisi olmayanların kartına yazılır (sürüm 1).
  - Kategori kartın kuralına uyar, boş parça olamaz; uymayanı `invalid_block_card` reddeder.
- **Dosyanın tamamı (`drawing`)** CAD dosyasında dosyanın biriminde çizilmiş sayılır (bloğun `units`'i).
- **Cevap:**
  - `sizes`: eklenen ve yeniden tanımlanan blokların adı, birimi, eni (x) ve boyu (y). Ölçü, bloğun ölçek 1'de çizdiğinin
    kutusudur (taban noktası değil, parçaları); metre cinsindendir.
  - `unit`: dosyanın birim kararı (`UnitDecision`).
- **Okuyucunun alamadığı nesneler** (3DSOLID gibi) `objects_left_out` ile söylenir.
- **Ret sırası:** `invalid_file_path`, `invalid_block_card`, `invalid_input`, `file_not_found`, `import_unreadable`,
  `unsupported_cad_version`, `source_unit_unknown`, sürüm, `nothing_to_import`, `unknown_block`, `block_refused`.

### 3. Ne yerleşeceği tıklamadan önce söylenir

- **Blok ekle'nin hayaletinin yanında etiket:**
  - "Yatak 90: 0.900 × 2.000 m";
  - "birimi milimetre · dönüş 180.0° · aynalı".
  - Ölçü, bloğun kutusunun en ve boyu × ölçekler × birimsiz bloğun çarpanıdır; çizimin biriminde yazılır.
  - Taban noktasında + işareti vardır.
- **Etkin aracın ayarları (görünür ayarlar, ADR 0657):**
  - Blok, En × boy (salt okunur), Bloğun birimi (salt okunur);
  - dinamik bloğun parametreleri (§4);
  - Ölçek, Y ölçeği, Dönüş, Aynala ve (duvarlı çizimde) Duvara hizala.
- **Öznitelikler:** yerleştirmenin satırlarına "En × boy" (gerçek) ve "Bloğun birimi" eklenir.
- **Birimin sözleri** tek yerdedir (`block_insert::unit_words`):
  - milimetre, santimetre, metre, inç, fit;
  - "birimsiz (çizimin birimiyle büyür)";
  - "belirtilmemiş (metre sayılır)".

### 4. Boyut varyantı tıklamadan önce

Dinamik bloğun parametreleri Blok ekle'nin ayarlarında satırdır (en çok sekiz):

- tablo (lookup) ve görünürlük satırı seçimdir;
- çevirme açık/kapalıdır;
- doğrusal parametre uzunluktur (çizimin biriminde yazılır, birimsiz blokta düz sayı).

Ayrıca:

- **Varyant (V)** ilk seçim listesini sıradakine geçirir; komut satırının seçeneğidir.
- Değerler `cad.blocks.insert`'in `parameters`'ına gider.
- Hayalet, çözülmüş değerlerle değişkeyi çizer (`Store::insert_outlines_with`).
- Bir tablo satırı uzunlukları değiştirince uzunluk satırı da onu gösterir.
- Kurala uymayan değer (En 1600 mm, kural 800–1400) ayarlarda "Değer" notuyla söylenir; tıklama komutun reddiyle döner.
- Açı ve nokta parametreleri yerleştirmeden sonra tutamaçla düzenlenir (ADR 0319).

### 5. Kabul matrisi

Başsız sunucu testi `block_library.rs`, metre ve milimetre planda aynı akışı koşar:

- Yatak odası DXF'inden yatak 0,90 × 2,00 m gelir; DWG ikizi aynı boyu verir.
- Yemek masası dört iç sandalye yerleştirmesiyle gelir (1,60 × 1,90 m).
- Klozet birimsiz dosyadır: önce reddedilir, milimetreyle 0,38 × 0,55 m gelir.
- Çeyrek dönük ve aynalı yatak zeminde 2,00 × 0,90 m durur.
- `.kcad` kaydedilip açılınca:
  - bloğun birimi ve kategorisi kalır;
  - kilim kendi santimetresindedir;
  - yerleştirme aynı kutudadır.
- DXF'e yazılır. Python başvurusu (`--written`) KentOS kodu olmadan okur:
  - `$INSUNITS` ve bloğun kaydı dosyanın birimidir (ADR 0315 §4: bloğun sayıları dosyanın biriminde);
  - blok 0,90 × 2,00 m;
  - INSERT aynalı (42 = −1) ve 90° dönük;
  - zeminde 2,00 × 0,90 m, (10, 20) m'de.

## Doğrulama

- **Ortak durumlar** (`scripts/fixtures/block_library_cases.py`, KentOS kodu olmadan):
  - `file.import.block.library.json`: 30 durum. Dört DXF'lik kitaplık, DWG ikizleri, birimler (mm, cm, inç,
    bildirmeyen), kayıt birimi farklı blok, iç içe sandalyeler, kategori, korunan ve yeniden tanımlanan ad, plan,
    retler.
  - Kutular kesin: kesirli birim çarpanları, çeyrek dönüşler, yay uçları ve dördün noktaları.
  - DWG ikizleri kodekle üretilir (`cargo run -p kentos-dwg --example corpus`); eski DWG örnekleri bayt bayt aynı
    kaldı.
  - `block_file_cases.py`'nin 25 durumu `sizes` ile yeniden yazıldı (çizgi, yay, çokgen ve iç yerleştirme kutuları).
- **Takımlar:**
  - uygulama 67 + 243 (yetenek testi dahil), etkileşim 66 + 792 (üç yeni: etiket, varyant, kurala uymayan uzunluk),
    başsız sunucu (kabul matrisi), komutlar, sözleşmeler, MCP;
  - masaüstü: kitaplık akışı ve kategori testleri; bütün takım 1360 test.
  - Bütün takımlar bu dilimde koşuldu: komutlar 921, sözleşmeler 1063 + 33 + 10, çekirdek 534 + 284, etkileşim 791,
    başsız sunucu 65, MCP 22 + 2, masaüstü 1360, Python 374 (ortak durum koşucusuna `blockFields` eklendi); clippy
    değişen sekiz crate'te temiz.
- **Bulunan ve düzeltilen:** Blok ekle artık görünür ayar taşıdığı için ADR 0657'nin kuralı (ayarı olan aracın
  seçeneği komut satırına odak verir) klavyeden basılan seçeneklere de uygulanıyordu: ikinci B komut satırına
  yazılıyor, Blok ekle izi düşüyordu. Klavyeden basılan seçenek harfi odağı çizimde bırakır (ardından harfler yine
  seçenek, rakamlar değer alanı); yalnız komut çubuğundan seçilen seçenek komut satırına odak verir
  (`apps/desktop/src/input.rs`). Bütün izler üç çeşitte geçti.
- **Gerçek arayüz resimleri** (`.run/shots/arac-*`, iki tema, 1440×900 ve 1100×650):
  - kitaplik-klasor, kitaplik-birim, kitaplik-birim-secildi, kitaplik-yerlestir;
  - blok-ekle-varyant, blok-ekle-mm;
  - blok-oznitelik-boyut.

## Açık kalanlar

- **Kitaplıkta `.kblok` dosyaları:** KentOS'un kendi kitaplık biçimi `cad.blocks.edit` import'uyla ayrıca alınır.
- **"Dosyanın tamamı"nın küçük resmi:** şimdilik yalnız boyu yazılır.
- **AutoCAD'in Recent ve Favorites karşılıkları:** son kullanılan ve sık kullanılan bloklar.
- **Bütün kitaplıklarda arama:** şimdilik yalnız bulunulan yer aranır.
- **DXF'te bloğun kendi birimi:** dosyanın birimine çevrilir (ADR 0315 §4); kaynak birimin adı DXF'te korunmaz.
