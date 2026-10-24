# ADR 0583: Katmanın kaynak sistemi ve harita sistemi

- **Durum:** kabul edildi (2026-10-22). `plan/gis.md` CBS-07: her katmanın kaynak sistemi korunur; çizimde anında dönüşüm; ters
  dönüşümlü tıklama ve kenet; WKT2 ve kullanım alanı; sistem atama, görüntü dönüşümü ve kalıcı izdüşürme ayrı işlerdir.
- **Sahibin kararı (22 Ekim):** “Tek çalışma sistemi + harita sistemi”. Çizim projenin sisteminde kalır; her katman kaynak
  sistemini taşır; harita sistemi ayrı bir görünüm ayarıdır, veri değişmez.
- **Bağlam belgesi:**
  - ADR 0165: proje türleri ve yerel sistem;
  - ADR 0167 ve 0168: ikinci sistem, dönüşümler, WKT ve PROJ okuma;
  - ADR 0504: kaynağın sistemi ve kutu dönüşümü;
  - ADR 0519: düşey sistem ve epok.
- **Web:** ertelendi (ADR 0228), TODOS.md §0.2.
- **Sonraya kalan:**
  - harita sisteminde rasterler, nokta bulutları ve resimler (bugün harita sistemi projeninkinden farklıyken gizlenir ve
    söylenir);
  - genel PROJ bağdaştırıcısı: kayıtta olmayan sistem, kendi tanımıyla (ADR 0168) ya da hiç;
  - harita sisteminde pafta (pafta kendi haritasının sistemini ADR 0164'teki gibi taşır).

## Bağlam

Çizimin bütün nesneleri projenin sistemindedir. İçe aktarma başka sistemdeki dosyayı açık onayla çevirir (ADR 0222). Bağlı katman
tablonun sistemini bağda tutar (ADR 0504). Üç eksik kalıyordu:

1. **Kaynak bilgisi kayboluyor.** Çevrilen katmanın aslında hangi sistemden geldiği, WKT'si, eksen sırası ve kullanım alanı
   katmanda yoktu.
2. **Yanlış sistemde alınmış veri düzeltilemiyor.** ED50'de olduğu hâlde projenin sisteminde sanılarak alınmış bir katman
   ancak elle bir dönüşümle düzeltilebiliyordu.
3. **Harita yalnız projenin sisteminde görülüyor.** Web Mercator altlıkla ya da coğrafi koordinatlarla bakmak için projeyi
   değiştirmek, yani sistemi yeniden atamak gerekiyordu. Sistem atamak dönüşüm değildir (CLAUDE.md §5).

## Karar

### 1. Katmanın kaynak sistemi (`LayerNode.sourceCrs`, `.kcad` şema 98)

Katmanın nesneleri her zaman projenin sistemindedir. `sourceCrs` onların geldiği sistemi söyler:

- `srid`: EPSG kodu, kayıttaki bir sistemse.
- `name`: sistemin adı.
- `wkt`: WKT2 metni (ISO 19162:2019). Kayıttaki sistem için `crs::text`'in yazdığıdır; dosyanın kendi tanımı ADR 0168'in
  okuduğudur.
- `axisOrder`: `en`, `ne` ya da `latlon`. EPSG'nin eksen sırasıdır; çizimdeki koordinat her zaman doğu, kuzey.
- `unit`: `metre` ya da `degree`.
- `area`: kayıttaki kullanım alanı metni. Kayıt kesin bir kutu vermediği için kutu yazılmaz; kullanım alanı uydurulmaz.

Kaynağın sistemi bilinmiyorsa kayıt yoktur; tahmin edilmez.

**Kimler yazar:**

- dosya içe aktarmaları, dosya bir sistem bildirdiğinde (GeoPackage, Shapefile'ın `.prj`'si, GeoJSON, KML/GPX: WGS 84, GML'in
  `srsName`'i, CSV'nin `.prj`'si);
- bağlı katmanın bağlanması (ADR 0503, 0505): tablonun sistemi;
- §2'nin iki komutu.

### 2. Komutlar

- **`cad.layers.crs` (Sistem ata):**
  - katmanın kaynak sistemi kaydını yazar ya da siler;
  - nesnelere dokunmaz; ayardır, geri alma adımıdır;
  - kilitli katmanda da çalışır, çünkü veri değişmez.
- **`cad.layers.reproject` (Yeniden izdüşür):**
  - katmanın nesnelerini verilen sistemden (`from`, yoksa katmanın kaynak sistemi) projenin sistemine çevirir;
  - kayıt `from` olur; tek geri alma adımıdır;
  - kotlar ve ölçüler kalır, yaylar ve eğriler `move_entity`'nin kuralıyla çevrilir;
  - çevrilemeyen nesne varsa bütün komut reddedilir (`crs_unreached`, `uids`);
  - kilitli katman reddedilir; projenin sistemi yoksa ret `crs_mismatch`.

### 3. Harita sistemi (`settings.mapSrid`, `.kcad` şema 98)

Harita sistemi görünümün sistemidir. Boşsa projenin sistemidir. Değiştirmek hiçbir nesnenin koordinatını değiştirmez ve geri alma
adımı değildir (ayardır).

**Çizim:**

- Vektör katmanların ilkelleri stil motorunun toplayıcısında (`BatchSink`'in izdüşümü) harita sistemine çevrilir.
  - Eğriler önce çizgiye bölünür, sonra köşeler çevrilir.
  - Düz kenarlar sıklaştırılır: kenarın ortası çevrilmiş uçların ortasından görünüm
    ölçeğinde 0,5 pikselden çok saparsa kenar ikiye bölünür.
- Kamera izdüşümü taşır:
  - `world_to_screen` projenin noktasını harita sistemine, sonra ekrana çevirir;
  - `screen_to_world` tersini yapar;
  - `visible_bounds` görünen kutuyu projenin sistemine sıklaştırarak çevirir (`transform_bounds`).
  - Böylece imleç, tıklama, kenet, seçim kutusu, tutamaçlar, önizlemeler ve yazılar projenin noktalarıyla çalışır.
- Servis karoları zaten bir ağla projenin sistemine çevrilir (ADR 0208 §3); ağın hedefi harita sistemi olur.
- Rasterler, nokta bulutları ve resimler harita sistemi projeninkinden farklıyken çizilmez. Durum çubuğu bunu bir kez söyler.
  (25 Ekim eki: ADR 0630'dan beri ağla harita sistemine taşınarak çizilirler; uyarı kalktı.)

**Ölçü ve yazma:** projenin sisteminde kalır. Durum çubuğu imlecin projedeki koordinatını yazar; harita sistemi farklıysa
yanında harita sistemindeki koordinatı da.

**Neden:**

- Bütün araçlar, komutlar, Python ve MCP tek sistemi görmeyi sürdürür.
- Harita sistemi yalnız görünümdür: veriyi değiştirmeden Web Mercator, coğrafi ya da başka bir TM diliminde bakılır.

### 4. Eksen sırası ve gösterim

Çizimin ve komutların koordinatı her zaman doğu, kuzeydir. EPSG:4326'nın enlem, boylam sırası (WMS 1.3, GML) okuyucularda
çözülür (ADR 0208, 0545); kayıt yalnız onu söyler. Türk ölçmeciliğinin Y, X gösterimi bir gösterim kuralıdır (CLAUDE.md §5) ve
harita sisteminden bağımsızdır. Bu üçü ayrı ortak durumlarla denetlenir.

## Doğrulama

- `scripts/fixtures/layer_crs_cases.py`: `cad.layers.crs` ve `cad.layers.reproject` ortak durumları, PROJ'la (pyproj), KentOS
  kodu olmadan:
  - atama, silme, kilitli katman;
  - ED50'de sanılan katmanın çevrilmesi, kotlar, yaylar;
  - çevrilemeyen nesne, eksen sırası (EPSG:4326 ve TM), WKT2 metninin PROJ'da okunması.
- `.kcad` şema 98: `sourceCrs` ve `mapSrid` bağımsız Python yazıcısıyla (`kcad_v2_reference.py`) ve taşıma matrisiyle.
- Harita sistemi: kameranın izdüşümü ve ters izdüşümü, görünen kutu, toplayıcının sıklaştırması birim testleriyle. Masaüstünde
  TUREF/TM36 projesi Web Mercator ve coğrafi harita sisteminde: aynı noktaya tıklamak ve koordinatla sormak aynı nesneyi bulur.
  Resimler alınır.
