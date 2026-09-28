# ADR 0138: Netcad NCZ okuyucusu, ayrı DXF ve NCZ modülleri, kare kare içe aktarma

- **Durum:** kabul edildi (2026-09-28).
- **Tarih:** 2026-09-28
- **Bağlam belgesi:** ADR 0009 (dosya biçimleri), ADR 0030 (tipli işçi sınırı ve sütunlar), ADR 0046 (GeoJSON, Shapefile), ADR 0048 (masaüstünde dosya alışverişi); TODOS.md `FMT-01`, `FMT-06`, `NCZ-01`.

## Bağlam

- Türkiye'de imar planları ve halihazır haritalar Netcad'in NCZ biçiminde dolaşır. KentOS bu dosyaları okuyamıyordu:
  - web'de `file.import.ncz` “bekliyor” diyordu;
  - masaüstünde komut yoktu.
- KentOSCad'de (C++) bir NCZ okuyucusu yazılmıştı. Kaynağı Erdinç Örsan ÜNAL'ın QGIS eklentisi **NCZ Reader**'dır (1.4.3, `ncz_pure.py`). Okuyucu Sivas UİP'nin 72 MB'lık dosyasında eklentiyle aynı kayıtları veriyordu.
- Büyük dosyada içe aktarma arayüzü kilitliyordu:
  - Web, bütün nesneleri tek işlemde, tek dönüşte yazıyordu; sayfa donuyordu.
  - Masaüstü aynı işi arayüz iş parçacığında yapıyordu; son karede 1,7 saniye bekliyordu.
- DXF okuyucusu biçim modülünün içindeydi. Yalnız koordinat listesi alan kullanıcı da DXF okuyucusunu indiriyordu.

## Karar

### 1. `crates/shared/ncz` (`kentos-ncz`)

**Blok okuyucusu** (`format.rs`, `attributes.rs`) eklentinin `ncz_pure.py`'sinin taşımasıdır:

- Her blok kuralı, uzaklık, geçerlilik testi ve sezgi korunur. Kasıtlı farklar `format.rs`'in başında yazılıdır.
- Sivas UİP'de eklentiyle kayıt kayıt karşılaştırıldı: 473 180 satır aynı. Farkın tamamı `atan2`'nin 1 198 değerde son birimidir (libm, CLAUDE.md §14).

**Eklentide olmayanlar:**

- Netcad 8 akıllı nesneleri (Planet) sembolleriyle çizilir (`symbols.rs`):
  - yerleşim: dairesi, nizamı, kat sayısı ve bahçe mesafeleri;
  - yapılaşma: TAKS/KAKS dairesi, ya da Emsal, Hmax, Yençok satırları;
  - yol: dairesi ve genişliği;
  - plan notu: kutusu ve metni;
  - fonksiyon adı.
- Değerler nesnenin öznitelikleridir. Ortalı yazılar çizimin yazı tipiyle ölçülerek yerleşir (`NczReadOptions.drawingFont`).
- Tanınmayan bloklar geometri için taranır.
- Nesne başına çizgi kalınlığı (+28) okunur.
- Sözleşmeye eşleme `emit.rs`'tedir.

**Akıllı nesneler üsttedir.** Nesnelerinin en az yarısı akıllı nesne olan katman listede önce gelir; katman ağacının üstü en son, yani üstte çizilir. Eski akıllı nesnelerin çerçeveleri de sayılır.

**Renk:** siyaha ya da beyaza çok yakın renkler (#101410, #000100 gibi) `ink` sayılır. Koyu zeminde kaybolmazlar.

**Koordinat sistemi:**

- MPROJ ve TILED_XML blokları `DeclaredCrs` olarak döner (`CrsSource::Ncz`).
- Pencere GeoJSON'daki gibi sorar. Sistem projeninkinden başkaysa içe aktarma kapalıdır.
- Hiçbir şey dönüştürülmez (CLAUDE.md §5).

**Alınamayanlar** türüyle, sayısıyla ve Türkçe nedeniyle raporlanır.

- Nesne başına çizgi kalınlığı şimdilik raporda kalır: model onu henüz taşımıyor.
- Yarım kesilmiş bir dosya, tam blokları kadarıyla okunur.

### 2. Lisans ve dağıtım koşulu

- Kaynak GPL-2.0-or-later'dır. Crate ve NCZ modülü de GPL-2.0-or-later'dır.
- **Sahibin kararı (28 Eylül):** taşıma şimdi bu özel depoda yapılır. İçinde NCZ okuyucusu olan hiçbir derleme dağıtılmaz; buna masaüstü ve web'in NCZ modülü dahildir.
- Bu yasak, yazar KentOS'a izin veren bir lisans (MIT ya da Apache-2.0) ya da yazılı izin verene dek sürer.
- Koşul üç yerde yazılıdır:
  - `crates/shared/ncz/README.md`;
  - bağımlılık kaydı (`docs/deps/README.md`);
  - TODOS.md `NCZ-01`.
- “Jeomatik” yazarın markasıdır ve yalnız kaynağı anmak için geçer.

### 3. Ayrı DXF ve NCZ modülleri, gerektiğinde yüklenir

- `crates/wasm/dxf-wasm` DXF'i okur ve yazar (`readDxf`, `writeDxf`). `crates/wasm/ncz-wasm` NCZ'yi okur (`readNcz`).
- Biçim işçisi (`io/formatsWorker.ts`) her modülü, ona ihtiyaç duyan ilk istekte bir kez yükler (CLAUDE.md §20). `kentos-formats-wasm`'dan DXF çıkarıldı.
- NCZ almayan kullanıcı NCZ okuyucusunu hiç indirmez.
- Açılışta yalnız geometri modülü yüklenir; bu, tarayıcıda doğrulandı.
- Masaüstü aynı crate'leri doğrudan derler.
- Boyutlar (WASM, sıkıştırılmamış):

| Modül | Boyut |
|---|---|
| DXF | 641 KB |
| NCZ | 461 KB |
| Biçim | 1,22 MB |

**Sonuç sınırı.** Okunan çizim işçiden sayfaya iki parça olarak geçer:

- sonucun nesnesiz başı JSON olarak;
- nesneler, açılan `.kcad`'in geçtiği tipli sütunlar olarak (`kentos_kcad::columns`, ADR 0030). Tamponları kopyalanmaz, devredilir.

**İlerleme ve durdurma:**

- Okuyucu ilerlemesini binde olarak söyler (`kentos_formats::watch::Watch`). Web'de işçi en çok 50 ms'de bir ilerleme gönderir; pencerede çubuk olarak görünür.
- Masaüstünde Vazgeç, okumayı bayrakla durdurur. Web'de işçi sonlandırılır.

### 4. Sözleşme (`FORMATS_VERSION` 8)

- Yeni: `NczReadOptions` ve `CrsSource::Ncz`.
- `ImportLayer.kinds`: bir katmanın nesneleri, türe göre. Pencere, yüz binlerce nesneyi her karede dolaşmadan sayar.
- `ImportLayer.bounds`: katmanın kutusu.
- `ImportResult.view`: görünümün içe aktarılanı göstereceği yer. Dosyadaki uzak başıboş nesneler dışarıda kalır. Gerçek bir planda iki çizim 0, 0'daydı ve şehir 4 400 km'lik görünümde bir noktaydı.
- Üçünü `kentos_formats::import::summarise` doldurur; DXF, GIS, koordinat listesi ve NCZ okuyucularının hepsi çağırır.
- Seçilen katmanların görünümü `view_of`'tur. Web'deki `viewOf` aynı dört sayıdır.

### 5. Kare kare içe aktarma, iki platformda aynı

**20 000 nesneye kadar** her şey bir kerede ve tek işlemde yazılır.

**Daha büyük dosyada:**

- Önce katmanlar kurulur, görünüm dosyanın yerine gider.
- Pencere kapanır. Nesneler zaman dilimleriyle yazılır; çizim göz önünde dolar.
- Bir dilim, sayfanın iki dilim arasında geçirdiği sürenin üçte biridir; en az 10, en çok 50 ms.

**Tek geri alma adımı:**

- Bütün iş bir belge grubunun içindedir.
- Sağ alttaki panel yazılan nesneleri sayar.
- Durdur, Esc ya da sayfanın, pencerenin kapanması yapılan her şeyi geri alır; kayda bir şey düşmez.
- Çizimin kabul etmediği bir nesne de her şeyi geri alır ve hangi nesne olduğunu söyler.

**İçe aktarma sürerken yalnız görünüm kıpırdar.** Başka bir düzenleme içe aktarmanın geri alma adımına girer ve Durdur'la onunla birlikte gider.

- Masaüstünde:
  - `while_importing` dar bir mesaj listesine izin verir;
  - çizim grubun içinde de sürüm ilerletir, çizici izler; revizyon ve kaydedilmemiş işaret grup bitince değişir.
- Web'de `app/hold.ts` şunları yapar:
  - görünüm komutlarından başkasını ve her aracı nedenini söyleyerek reddeder;
  - tuşları panelden başka yere iletmez; Esc her yerde durdurur;
  - çizimde yalnız orta tuş ve tekerlek geçer;
  - şerit, paneller, komut satırı ve durum çubuğu `inert` olur.
- Gizli sekme boya yapmaz ve tarayıcı ona saniyede bir kare verir. Bu yüzden gizli sekmede dilimler birbirini izler.

### 6. Ölçümler (28 Eylül, Apple Silicon)

**Masaüstü:** Sivas UİP (72,1 MB, 453 534 nesne) 187 ms'de okunur. Nesneler 19 karede, 1,84 saniyede yazılır; uyarlanan dilimlerden önce 38 kare ve 4,2 saniyeydi.

**Web (Chrome, gerçek GPU, WebGL2; `apps/web/scripts/e2e/drawing-import.mjs`):**

| Dosya | Okuma | Yazma | Kare | Kare süresi (ortanca / en uzun) | İçe aktarmanın kendi işi | En uzun dilim |
|---|---|---|---|---|---|---|
| Sivas NCZ | 0,42 s | 1,8 s | 30 | 30 / 284 ms | 0,48 s | 52 ms |
| Suşehri DXF (48,2 MB, 71 820 nesne) | 0,29 s | 0,37 s | 19 | 17 / 32 ms | — | — |

SwiftShader'lı başsız tarayıcı yarım milyon nesneyi karede saniyelerle çizer. O ölçüm yazılım GPU'sunu ölçer, içe aktarmayı değil (CLAUDE.md §9.5).

## Sonuçlar

**Testler:**

- `crates/shared/ncz/tests/fixtures.rs`: yedi fixture. `scripts/fixtures/ncz_reference.py` dosyaları blok blok, uzaklıklarından yazar; beklenen değerler o betikten elle çıkarılmıştır.
- Aynı dosyaları tarayıcıdaki NCZ modülü de okur (`apps/web/src/io/ncz.wasm.test.ts`).
- DXF modülünün tarayıcı testi: `io/dxf.wasm.test.ts`.
- Kare kare yazma: `io/drawingImport.test.ts` (tek adım, Durdur, reddedilen nesne, kilitli katman) ve masaüstünün `exchange/tests.rs`'i.
- Grubun sürüm ilerletmesi: `crates/native/domain/tests/document.rs`.

**Açık kalanlar:**

- nesne başına çizgi kalınlığı: DXF 370 ve NCZ'nin kalınlığı; sözleşme, KCAD sütunları, iki çizici, Öznitelikler ve DXF yazıcısı birlikte (sıradaki iş);
- kesik dosyanın raporda söylenmesi;
- sunucuda içe aktarma;
- Netcad blok ve sembol kitaplığı: bugün sembol ve blok adıyla nokta olarak gelir;
- dağıtım koşulu (`NCZ-01`).
