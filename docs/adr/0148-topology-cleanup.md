# ADR 0148: Topolojik temizlik

- **Durum:** kabul edildi (2026-10-01). Sıra sahibin kararıdır: `CAD-17`'den sonra TODOS.md §16'nın sırası, önce hibrit işler (§16.0), ilki `HYB-01`. Ayrıntılar bu ADR'nin varsayılanlarıdır.
- **Tarih:** 2026-10-01
- **Bağlam belgesi:** TODOS.md `HYB-01` (ilgili: `HYB-02`, `GIS-04`, `NUM-10`), [araştırma kaydı](../research/2026-10-01-netcad-arcgis-qgis.md); ADR 0140 (Çizimi temizle, Parçala, Sadeleştir), ADR 0047 (Buda, Uzat; `cad.entities.edit`), ADR 0142 (köşe kotu ve düzenlemelerde kot), ADR 0143 (çok parçalı alan), ADR 0029 (depo); CLAUDE.md §23.3.

## Bağlam

Kâğıttan sayısallaştırılmış, DXF ya da NCZ'den gelen çizimlerde çizgiler birbirine tam oturmaz:

- **Uçlar buluşmaz:** iki çizginin ucu aynı köşede olması gerekirken birkaç santim ayrıdır.
- **Kısa kalır:** bir çizgi öbürüne varmadan biter (T kavşağında boşluk).
- **Taşar:** bir çizgi öbürünü keser ve birkaç santim öteye geçer.
- **Köşeler çakışmaz:** komşu parsellerin ortak köşeleri birbirinden biraz ayrıdır.

Göze görünmeyen bu kusurlar alan kapatmayı (`HYB-02`, İçine tıklayarak alan), taramayı, alan hesabını ve CBS'ye aktarmayı bozar. Netcad bunu Topoloji › Düzelt ile (Son Nokta, Uzat-Kes, Kesişim), AutoCAD Map 3D Drawing Cleanup ile, ArcGIS Integrate, Snap, Extend Line ve Trim Line ile, QGIS Snap geometries to layer ile çözer.

KentOS'ta bugün yalnız Çizimi temizle (yinelenen, boş nesne, art arda tekrarlanan köşe; tolerans yok) ve tek tek Buda, Uzat vardır. Toleransla çalışan, önce gösteren, tek adımda düzelten bir araç yoktur.

## Karar

### 1. Araç

**Topolojik temizlik** (`tool.topology`): çizgi ağını ve alan sınırlarını verilen tolerans içinde düzeltir; önce gösterir, Enter tek adımda yazar. Şeritte Değiştir › Nesne ▾ listesinde Çizimi temizle'nin yanındadır. Takma adlar: TOPOLOJI, TOPOLOJIKTEMIZLIK, MAPCLEAN (AutoCAD Map 3D).

### 2. Kapsam: düzeltilenler ve dayanaklar

- **Düzeltilenler:** seçim; seçim yoksa görünen ve kilitsiz katmanlardaki bütün nesneler. Kilitli katmandaki seçili nesne düzeltilmez, sayısıyla söylenir.
- **Dayanaklar:** görünen katmanlardaki öbür nesneler (seçim varken seçilmeyenler, seçim yokken kilitli katmandakiler). Hiç değişmezler; köşeleri birleşme noktası, kenarları uzatma ve budama sınırıdır.
- **Noktalar** (seçimde de olsalar) hiç değişmez: ölçü noktası doğru kabul edilir, çizgiler ona gider.
- **Türler:**
  - düzeltilen: çizgi, çoklu çizgi, yay, alan (halkaları ve delikleriyle, bütün parçaları);
  - yalnız dayanak: nokta (köşe olarak); daire, elips ve eğri (kenar olarak; elips ve eğri 0,1 mm'lik kirişleriyle, ADR 0149 §5.3);
  - katılmayan: yazı, ölçü, tarama, blok yerleştirmesi, kılavuz, yardımcı çizgi, ışın.
- Gizli katman hiç katılmaz: görünmeyen düzeltilmez, dayanak da olmaz.

### 3. Tolerans ve dört iş

**Tolerans** metre cinsinden, sonlu ve en az 1 µm (0,000001 m) bir uzunluktur; kullanıcı yazar, oturum boyunca hatırlanır (ilk değer 0,01 m). Hiçbir köşe toleranstan fazla yer değiştirmez; uzatılan ya da budanan uç da toleranstan fazla uzamaz ya da kısalmaz. Gizli tolerans yoktur (CLAUDE.md §23.3): yalnız kullanıcının yazdığı, önizlemede görülen değer.

Dört iş vardır, her biri açılıp kapanır:

| İş | Harf | İlk değer | Ne yapar |
|---|---|---|---|
| Uçlar | U | açık | Açık bir yolun ucu toleransa giren bir köşeye, köşe yoksa en yakın kenara taşınır. |
| Köşeler | K | kapalı | Her köşe toleransa giren başka bir köşeye taşınır; bir yolun art arda iki köşesi tek köşe olur. |
| Uzat | Z | açık | Boşta kalan uç, son düz kenarının doğrultusunda en yakın kenara toleransa kadar uzar. |
| Buda | B | açık | Boşta kalan uç, kestiği son kenardan en çok tolerans kadar taşıyorsa o kesişimde kısalır. |

Köşeler kapalı başlar: alan sınırlarını da değiştirdiği için bilerek açılır.

### 4. Birleştirme: köşeler nereye gider

Uçlar ve Köşeler işleri köşeleri birleştirir. Birleşme noktası **her zaman var olan bir köşedir**; ortalama alınmaz, yeni koordinat üretilmez.

1. **Düğümler.** Aynı yerdeki köşeler (1 µm içinde; düzenlemelerin kot kuralındaki “köşede” ölçütü, ADR 0142) bir düğümdür. Düğüm, üyelerinden biri sabitse sabittir: dayanak köşesi, nokta ya da o işte taşınamayan köşe (Uçlar'da bir yolun iç köşesi, alan köşesi).
2. **Önem sırası:** noktalar; sonra öbür sabit düğümler; sonra taşınabilenler, en çok nesnenin buluştuğu düğümden aza; eşitlikte çizimde önce gelen.
3. **Temsilciler.** Sabit düğümlerin hepsi temsilcidir. Taşınabilen düğümler önem sırasıyla gezilir: toleransa giren bir temsilci yoksa düğüm de temsilci olur. İki taşınabilir temsilci hiçbir zaman toleransa girmez; zincirleme birleşme olmaz.
4. **Atama.** Temsilci olmayan her düğüm, toleransa giren temsilcilerin en yakınına taşınır (eşit uzaklıkta önem sırası). Böylece her köşe en çok tolerans kadar yer değiştirir.
5. **Aynı nesne.** Bir nesnenin iki köşesi yalnız şu durumlarda birleşir:
   - bir yolun art arda iki köşesi (aradaki kenar kalkar);
   - açık bir yolun ilk ve son köşesi, yolda en az üç ayrı köşe kalıyorsa (yol kapanır).
   
   Başka iki köşesi (yolun art arda olmayan köşeleri, bir çizginin iki ucu, alanın ayrı halkaları ya da parçaları) birleşmez. Yol geçersiz kalacaksa birleşme olmaz: açık yolda en az iki, halkada en az üç köşe kalır (yaylı kenarlı halkada iki). Birleşmesi yasak bir düğüm sıradaki en yakın temsilciye gider, o da yoksa yerinde kalır.
6. **Yaylar.** Köşesi taşınan yaylı kenar kabarıklığını, yani açısını korur; yay nesnesi de böyle yeniden kurulur. Art arda iki köşe birleşince aralarındaki kenar, yaylı olsa da kalkar.

### 5. Uzatma, budama ve kenara taşıma

Birleştirmeden sonra **boşta kalan uçlara** bakılır. Boşta kalan uç, açık bir yolun başka bir nesnenin köşesinde ya da kenarında olmayan ucudur (1 µm içinde).

- **Sınırlar:** öbür nesnelerin kenarları: düzeltilenlerin birleştirmeden sonraki hâlleriyle ve dayanakların. Ucun kendi yolu sınır değildir.
- **Budama:** uçtan geriye, yolun sınırlarla kesiştiği ilk nokta. Taşan parçanın uzunluğu toleransı geçmiyorsa uç oraya kısalır.
- **Uzatma:** yalnız son kenarı düz olan uç. Uçtan, son kenarın doğrultusunda ilk sınır. Uzaklık toleransı geçmiyorsa uç oraya uzar. Yaylı uç uzatılmaz.
- **İkisi birden olabiliyorsa** değişikliği küçük olan yapılır; eşitse budama.
- **Kenara taşıma:** Uçlar açıkken, budanmayan ve uzatılmayan boşta uç toleransa giren en yakın kenar noktasına taşınır.
- Her uç, ötekilerin uzatma ve budamasını beklemeden, birleştirmeden sonraki çizime göre düzeltilir. Sonuç iş sırasından bağımsızdır.
- Kenarına uç gelen nesneye köşe eklenmez (T kavşağı).

### 6. Kot

Kotlar çekirdekte, açıkça hesaplanır (ADR 0142'nin kurallarıyla aynı yönde). Böylece bir köşenin kotu, sayısı değişen nesnede de kaybolmaz:

- **Köşeye taşınan köşe:** gittiği köşenin kotunu alır. Onun kotu yoksa kendi kotu kalır.
- **Kenara taşınan uç:** kendi kotu kalır.
- **Uzayan uç:** son kenarının iki ucu kotluysa eğim sürer (Uzat'ın kuralı). Değilse ucun kendi kotu kalır.
- **Budanan uç:** kesildiği kenarın iki ucu kotluysa kot kenar boyunca alınır (Buda'nın kuralı). Değilse ucun kendi kotu kalır.
- **Kalan köşeler:** kendi kotlarını korur.

### 7. Hesap (ortak çekirdek)

`kentos_geometry_core::ops::topology` (WASM'da `topologyClean`):

- **Girdi:**
  - nesneler çizim sırasıyla; her biri türü, düzeltilen ya da dayanak olduğu ve yolları (köşeler, kabarıklıklar, kapalılık, kotlar);
  - tolerans;
  - dört işin açıklığı.
- **Çıktı:**
  - değişen her nesnenin yeni yolları (kotlarıyla);
  - değişiklikler (tür; nereden, nereye);
  - sayılar: birleşen uç, birleşen köşe, uzayan, budanan, kenara taşınan;
  - en büyük yer değiştirme.
- **Dizin:** köşeler ve kenarlar paketli R-ağacındadır (`store::rtree`). Ağaç yalnız adayları daraltır; kararlar §4 ve §5'in kesin kurallarıyladır, sıra ağaçtan gelmez.
- **Başarım:** 50 000 nesnelik çizim, yerelde bir saniyenin altında.

### 8. Komut

Yazma `cad.entities.edit`'in yeni `topology` işlemiyledir (adım adı “Topolojik temizlik”):

- yalnız güncelleme yapar; nesne eklemez, silmez;
- her geometri kotlarını açıkça taşır;
- kilitli katman ve tek adım kuralları ötekiler gibidir.

İki işleyici, ortak durumlar ve Python SDK'sı bunu geçer.

### 9. Araç ve arayüz

- **Akış:** araç açılınca kapsamı alır (§2) ve hatırlanan toleransla önizlemeyi hemen gösterir.
  - Yazılan sayı toleranstır: önizleme yenilenir, yazmaz.
  - U, K, Z ve B işleri açıp kapatır.
  - Enter (ya da Uygula, ya da hızlı sağ tık) yazar ve araçtan çıkar.
  - Esc çıkar.
  - Düzeltilecek bir şey yoksa söyler, araçta kalır; tolerans değiştirilebilir.
- **Önizleme:**
  - her değişikliğin yerinde ekran boyutu sabit bir işaret (santimlik değişiklik her yakınlıkta görünür): birleşme ve kenara taşıma vurgu renginde, uzatma başarı renginde, budama tehlike renginde;
  - uzayan parça çizilir, budanan parça kesikli çizilir.
- **İstem:** sayılar ve en büyük yer değiştirme, örneğin “7 uç birleşir, 2 uç uzar, 1 uç kısalır; en büyük kayma 0,042 m”; seçenekler komut satırında tıklanır.
- **Sonuç iletisi:** aynı sayılar ve değişen nesne sayısı.

### 10. Kapsam dışı

- Ortalama noktada birleştirme (ArcGIS Integrate'in kümelemesi): koordinat üretir, kaynak koordinatı korunmaz.
- Kenarına uç gelen nesneye köşe ekleme ve alanların ortak kenarlarını eşleme (kenardan kenara kenet): topoloji modeli `NUM-10` ile.
- Kesişimde bölme (Parçala'nın işi), boşta kalan kısa çizgiyi silme, sahte düğümleri birleştirme (AutoCAD Map'in Dissolve Pseudo Nodes'u): sonraki iş olabilir.
- İki boşta ucun uzantılarının buluştuğu yer (AutoCAD Map'in Apparent Intersection'ı).
- Yaylı ucu çemberi boyunca uzatma; elips ve eğri.
- Topoloji doğrulaması ve kural raporu (`GIS-04`).

### 11. İş sırası

1. **Çekirdek.** `ops::topology`, WASM bağlayıcısı; bağımsız başvuru `scripts/fixtures/topology_cases.py` ve `fixtures/topology/v1/clean.json` (elle hesaplı ve rastgele durumlar; çekirdek ve web WASM'ı 1e-9 m içinde aynı); başarım ölçümü.

   *(1 Ekim: tamam.)*
   - **Çekirdek:** `ops::topology` (`topology_clean`, WASM'da `topologyClean`): `join` birleştirir (§4), `ends` boşta kalan uçları düzeltir (§5).
   - **Dizin:** birleştirme ızgarayla (1 µm'lik ve tolerans genişliğinde hücreler), boşta kalan uçlar paketli R-ağacıyla aranır. Adaylar çizim sırasıyla alınır, eşitlik her hedefte aynı yöne düşer.
   - **Başvuru:** `topology_cases.py` kuralları başvurunun kendi geometri işlevleriyle (doğru parçası ve yay kesişimleri, ışın, en yakın nokta) uygular. 74 durum yazar: 25 elle kurulmuş, başlangıçta ve TM koordinatlarında. Bunlar uçların buluşması, noktaya ve iç köşeye gitme, zincirlenmeme, en yakın temsilci, aynı nesne kuralları, yolun kapanması, uzatma, budama, ikisi birden, kenara taşıma, yay, dayanak, köşeler, halka, kot ve elipsin kirişleri gibi sınırları sınar. Kalan 24'ü rastgele çizgi ağlarıdır (T kavşakları, titreşimli ortak kenarlar, yaylı uçlar).
   - **Sonuç:** çekirdek (`tests/topology.rs`) ve web WASM'ı (`model/ops/topology.test.ts`) hepsinde başvuruyla 1e-9 m içinde aynı. 1 µm altındaki tolerans reddedilir.
   - **Başarım:** TM koordinatlarında 49 928 çizgilik ağ 0,2 saniyede temizlenir (release, `tests/topology.rs`'in elle çalıştırılan testi).
2. **Komut.** `EditOperation::Topology`; iki işleyici, ortak durumlar, katalog ve Python SDK'sı. *(1 Ekim: tamam. Sözleşmede `topology` işlemi; adım adı iki işleyicide “Topolojik temizlik”. Ortak durum (`edit_command_cases.py`, 85. durum): bir çizginin ve bir çoklu çizginin geometrisi kotlarıyla yerinde yazılır, tek adımda geri alınır; masaüstü, web ve Python SDK'sı geçer. TypeScript tipi, katalog ve SDK'nın tipleri üretildi.)*
3. **Araç ve arayüz.** İki platformda araç, şerit, takma adlar, önizleme, ortak iz (`fixtures/interaction/v1/topology.json`), testler ve resimler.

Her adım iki platformda, ortak fixture'larla, kendi commit'inde ilerler.

## Sonuçlar

- Çizgi ağı ve alan sınırları tek araçla, önizlemeyle ve tek geri alma adımıyla düzelir; `HYB-02` (çizgi ağından toplu alan) bunun üstüne kurulur.
- Hiçbir nokta toleranstan fazla oynamaz; birleşme noktası her zaman var olan bir köşedir; dayanaklar ve ölçü noktaları değişmez.
- Kotlar korunur ya da açık kurallarla taşınır.

## Doğrulama

- **Hesap:** bağımsız Python başvurusu, elle hesaplı durumlar (uç, köşe, zincirlenmeyen küme, aynı nesne kuralları, yaylar, uzatma, budama, ikisi birden, kenara taşıma, kot) ve büyük koordinatlarda rastgele durumlar; çekirdek ve web WASM'ı aynı sonucu verir.
- **Komut:** ortak durumlar üç koşucuda.
- **Araç:** ortak iz iki platformda; resimler iki temada, 1440×900 ve 1100×650.
