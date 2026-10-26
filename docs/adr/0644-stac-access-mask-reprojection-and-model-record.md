# ADR 0644: STAC varlıklarına erişim, kalite maskesi, yeniden izdüşüm ve modelin kaydı

- **Durum:** kabul edildi (2026-10-26). `plan/gis.md` CBS-31'in kalanı. Kapsamı ben belirledim.
- **Kalan işler (ADR 0528'den):**
  - imzalı adres akışı;
  - s3 adresleri;
  - yeniden izdüşüm;
  - maskenin hesaplara uygulanması;
  - eğitim ve test örneği ile modelin kaydı.
- **Bağlam belgesi:**
  - ADR 0528 (STAC keşfi ve hazırlama);
  - ADR 0204 §6 (rasterin yeniden örneklenmesi, `warp`);
  - ADR 0242 (uzaktan algılama);
  - ADR 0167 ve 0168 (koordinat dönüşümleri, projenin datum seçimleri);
  - ADR 0583 (katmanın yeniden izdüşümü);
  - ADR 0642 (adresteki raster, İndir).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **`.kcad`:** şema değişmez. Erişimin, maskenin ve modelin kaydı rasterin özniteliklerindedir (`stac.*`, `model.*`).
- **Sonraya kalan:**
  - Google Cloud (`gs://`) adresleri. Reddedilir; yerel kopyası `local` ile verilir.
  - Planetary Computer dışındaki SAS veren kataloglar. İmzası atılan başka adres önceki gibi uyarır.
  - Yeniden izdüşüm arayüzün iş parçacığında çalışır, ilerleme göstermez ve durdurulamaz. Komut iş olarak çalışınca
    durdurulur.
  - Mekânsal doğrulama yalnız örtüşmeyi sayar; eğitim ve test bölgelerini ayıran örnekleme yoktur.
  - Datum ve sensör belirsizliğinin yayılımı raporlanmaz.

## Bağlam

ADR 0528 STAC'ta aramayı ve seçilen varlıkları adresli raster olarak hazırlamayı getirdi. Plan dört şey daha istiyor.

- **Erişim.** Gerçek kataloglarda sık görülen iki durum vardı ve ikisi de reddediliyordu:
  - Landsat'ın Earth Search'teki varlıkları `s3://usgs-landsat/…` adresleriyle gelir. Kovası istek ödemelidir
    (`storage:requester_pays`), okumak AWS imzası ister.
  - Planetary Computer'ın blob adresleri SAS belirteciyle okunur. Belirteç geçicidir; ADR 0528 onu projeye yazmamak için
    atar, raster de açılmaz.
- **Kalite maskesi.** Hazırlama kalite bandını ve kuralını (`stac.qa_mask`) kaydediyordu, ama hiçbir hesap onu
  kullanmıyordu. Planın ölçütü: bulutlu NoData pikseli bitki kaybı diye sınıflanmamalı.
- **Yeniden izdüşüm.** Projenin sisteminden başka sistemdeki varlık `crs_mismatch` ile reddediliyordu. Raster ekle de
  başka sistemdeki dosyayı reddediyordu; KentOS rasteri hiç yeniden izdüşürmüyordu.
- **Modelin kaydı.** Sınıflandırma sonucu hangi yöntemle, hangi sınıflarla ve hangi eğitim alanlarından üretildiğini
  taşımıyordu. Doğruluk analizi eğitimle aynı örneklerle yapılan doğrulamayı fark etmiyordu.

## Karar

### §1 Erişim

**Saklama uzantısı.** STAC okuyucusu varlığın `region`'ını ve `requesterPays`'ini okur:

- v1'in `storage:region` ve `storage:requester_pays` alanları (önce varlıkta, sonra öğede);
- v2'nin `storage:refs`'i: varlığın ilk başvurusu, öğenin `storage:schemes`'inde aranır.

**s3 adresi.** `s3://kova/anahtar` artık reddedilmez. Raster kovanın bölgesindeki HTTPS adresinden okunur:

- olağan biçim `https://kova.s3.<bölge>.amazonaws.com/<anahtar>`;
- adında nokta olan kova yol biçimindedir (`https://s3.<bölge>.amazonaws.com/kova/anahtar`), çünkü TLS noktalı kova adını
  tanımaz;
- bölge yoksa `us-east-1` sayılır;
- anahtar AWS'nin URI kodlamasıyla bir kez kodlanır.

Rasterin `stac.access`'i `aws` olur. İstek ödemeli kovada `stac.requester_pays` `true` olur ve hazırlama
`stac_requester_pays` uyarısı verir. `gs://` ve öbür adresler `stac_href_not_http` ile reddedilir; yerel kopya
verilmişse adres aranmaz.

**İmza.** Masaüstünün aralık okuyucusu (`kentos_cloud::range`) bir S3 kovasına giden isteği, kullanıcının AWS kimlik
bilgisiyle AWS Signature Version 4'le imzalar:

- kimlik bilgisi `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY` ve `AWS_SESSION_TOKEN`'dan, yoksa `~/.aws/credentials`'ın
  `AWS_PROFILE` ya da `default` bölümünden okunur;
- `range` ve `x-amz-request-payer: requester` başlıkları imzaya girer;
- imza hesabı saf çekirdektedir (`kentos_services::stac::access::sign`): kanonik istek, imzalanacak metin, HMAC-SHA256
  zinciri;
- kimlik bilgisi yoksa istek imzasız gider. Kova 403 verirse söz, kimlik bilgisinin nasıl verileceğini söyler.

**SAS.** Katalog Planetary Computer'sa ve adres bir blob adresiyse rasterin `stac.access`'i `planetary-computer` olur. İmza
atıldığı için verilen uyarı (`stac_signed_href_stripped`) burada verilmez. Okuyucu akışı şöyledir:

1. Önce düz adresi okur.
2. Blob sunucusu 403, 404 ya da 409 verirse Planetary Computer'dan kapsayıcının belirtecini alır
   (`/api/sas/v1/token/<hesap>/<kapsayıcı>`) ve isteği yineler.
3. Belirteci kapsayıcı başına saklar: süresi bitmeden beş dakika öncesine kadar, cevap süre söylemiyorsa yarım saat.

Belirteç ve anahtarlar süreçte kalır; çizime yazılmaz. İndir ve dosya olarak ekle (ADR 0642 §3) aynı yolu kullanır.

**Başvuru.** `scripts/fixtures/stac_access_cases.py` 26 durum yazar: 12 imza, 8 s3 adresi, 6 SAS.

- İmzalar Python'un `hashlib` ve `hmac`'ıyla AWS'nin belgesinden yazılır.
- Belgenin kendi örneği (GET Object, Range 0–9, imza `f0e8bdb8…`) birebir tutar.
- `--botocore` her imzayı botocore'un kendi imzacısıyla karşılaştırır; 12'si de aynıdır.

### §2 Kalite maskesi

İşlemler'in Uzaktan algılama kategorisine **Kalite maskesi uygula** (`remote.qaMask`) eklenir. Girdileri: veri rasteri,
kalite rasteri ve maske kuralı.

**Kural.** STAC hazırlamasının yazdığı biçimdir:

- `values:0,1,3,8,9,10`: bu sınıf değerleri gizler (Sentinel-2 SCL);
- `bits:0,1,2,3,4`: bu bitlerden biri açıksa gizler (Landsat QA_PIXEL).

Kural boşsa kalite rasterinin `stac.qa_mask` özniteliğinden alınır. Değer 0–65535, bit 0–31 aralığında olmalıdır.

**Hücre.** Her hücrenin kalite değeri, merkezinin altındaki kalite hücresinden okunur (en yakın). SCL 20 m, bantlar 10 m
olabilir. Kural değeri gizliyorsa hücre değersiz olur. Kalite rasterinin örtmediği ya da değersiz olduğu hücre de gizlenir.

**Sonuç.** Veri rasterinin ızgarasında, bantlarında ve örnek türündedir; görünüşü veri rasterininkidir. Analiz ortamını
(ADR 0643) alır. Özet `Maskelenen m / n hücre (%p).` der; hepsi gizlendiyse uyarır.

**Değişim tespiti.** Maskeden sonra gizli hücre değersizdir; fark, oran ve normalize fark onu saymaz. `degisim-maskeli`
durumu bunu gösterir: bulutlu hücreler azalan sayılmaz.

**Başvuru.** `scripts/fixtures/remote_cases.py`'ye 11 durum eklendi (57 durum):

- SCL'nin 20 m'lik ızgarası 10 m'lik görüntüde;
- QA_PIXEL bitleri;
- kalite rasterinin dışında kalan hücreler;
- alfa bantlı 8 bit RGB;
- hepsi gizlenen;
- yazılan kural;
- dört ret;
- maskeden sonra değişim.

`remote_processing_cases.py`'nin İşlemler durumları 26'dır (önce 21). Biri kural kalite rasterinin özniteliğinden, biri
yazılan kural; ikisi ret.

### §3 Yeniden izdüşüm

**Komut.** `cad.rasters.reproject` (Rasteri yeniden izdüşür) bir raster dosyasını projenin sistemine çevirir. Tek geri
alma adımıdır.

- **Sistem.** Dosyanın bildirdiği sistemdir, yoksa `srid`. Bildirilenden başka `srid` `crs_overridden` uyarısıyla sayılır.
- **Dönüşüm.** Koordinat dönüştür'ün yoludur (projenin datum seçimleriyle). Cevabın `way`'i yolu ve doğruluğu söyler.
  ED50'nin EPSG işlemi resmî değildir: `crs_unofficial`.
- **Izgara.** Kaynağın kenarlarının (kenar başına 20 adım) projedeki görüntüsünü örter. Köşesi piksel boyunun katındadır.
  Bir pikselin milyonda birinden ince kenar şeridi satır ya da sütun yapmaz (`warp::Grid::covering`).
- **Piksel boyu.** `cell` verilmezse kaynağın orta pikselinin projede kapladığı alanın kareköküdür, altı anlamlı basamağa
  yuvarlanır. Yuvarlama, dönüşümün son bitlerinin ızgarayı değiştirmemesi içindir.
- **Örnekleme.** Her piksel merkezi kaynağın pikseline geri döner (`warp::Job::keeping`).
  - `nearest`: merkezin altındaki pikseli alır; sınıf ve kalite kodları korunur.
  - `bilinear`: dört pikselin ağırlıklı ortalamasıdır; değersiz pikseller ağırlıklarıyla dışarıda kalır (GDAL'ın kuralı).
- **Değerler.** Korunur: 8 bitlik raster RGBA'ya çevrilmez (Raster oturt'un yeniden örneklemesi çevirir).
  - Kaynağın nodata'sı varsa çıktıya yazılır.
  - Yoksa dışarısı 0'dır (kayan noktada NaN) ve nodata yazılmaz; gdalwarp'ın da öntanımlısı budur.
- **Çıktı.** GeoTIFF, `output`'a ya da kaynağın yanına `<ad>-epsg<kod>.tif` olarak yazılır. Raster ona bağlı olarak
  `layer`'a ya da dosyanın adıyla açılan yeni katmana eklenir.
- **Retler.**
  - Var olan dosya `overwrite` verilmeden yazılmaz (`file_exists`). Kaynağın kendisinin üzerine yazılmaz.
  - Yerel projede `crs_mismatch`. Zaten projenin sistemindeki dosyada `crs_mismatch`, yanında Raster ekle önerisi.

**Raster ekle.** Başka sistemdeki dosyayı artık reddetmez, **Projenin sistemine çevir ve ekle** düğmesini sunar.

**STAC hazırlaması.** `cad.stac.prepare`'e üç alan eklendi:

- `local`: varlığın anahtarından yerel kopyaya;
- `reproject`;
- `drawing`.

Kurallar:

- Yerel kopyası verilen varlığın rasteri adresi değil dosyayı okur.
- `reproject` açıkken başka sistemdeki varlık reddedilmez. Yerel kopyası komutun kuralıyla (en yakın) projenin sistemine
  çevrilir ve raster çevrilen dosyayı okur.
- Kopyası yoksa `stac_local_missing`.
- Kayıtta bandın `file`'ı ve `reprojected`'ı (kaynak ve hedef sistem, yol, dosya, ızgara) durur. Rasterin
  `stac.reprojected_from`'u kaynağın sistemidir.

STAC penceresi, seçilen varlıklardan biri başka sistemdeyse **Projenin sistemine çevir** seçeneğini sunar. Hazırla önce
varlıkları çizimin `veri` klasörüne indirir (ADR 0642'nin yolu, imzayla), sonra hazırlar.

**Başvuru.** `scripts/fixtures/raster_reproject_cases.py` (GDAL ve PROJ, KentOS kodu olmadan) 29 durum yazar: 13 çevirme,
16 ret. Kaynaklar:

- UTM 36N, coğrafi DEM, TM30 (aynı datum), ED50 UTM 36N;
- 8 bitlik sınıf rasteri, üç bantlı renkli raster, sistemsiz dosya.

Pikseller iki ayrı yoldan denetlenir:

- `--written` KentOS'un yazdığı 12 dosyayı kuralın değerleriyle karşılaştırır. En yakın komşu piksel piksel aynıdır. KentOS
  ters dönüşümü sekiz pikselde bir tam, arasını çift doğrusal hesaplar (en çok 0,02 piksel); bu yüzden piksel kenarına
  0,025 pikselden yakın merkezler sayılmaz. Çift doğrusalda fark eğimle sınırlıdır.
- Her durum gdalwarp'ın aynı ızgaraya aynı PROJ yoluyla yaptığıyla karşılaştırılır:
  - en yakın komşu aynıdır (renkli rasterde gdalwarp 0 değerini 1 yapar);
  - çift doğrusalda gdalwarp tam sayıya yuvarlar ve küçülten yönde çekirdeği genişletir (fark 0,5 ve 0,22).

STAC'ın ortak durumları 51'dir (önce 38): s3, saklama v2, gs, yerel kopya, Planetary Computer, yeniden izdüşüm ve retleri.

### §4 Modelin kaydı ve sızıntı

**Denetimli sınıflandırma.** Sonuç rasteri modelin kaydını taşır:

- `model.method`;
- `model.classes`: `1=Orman; 2=Su; …`, tablonun değerleri ve adları;
- `model.samples`: sınıfı dolu eğitim nesnesi sayısı;
- `model.training_layer` ve `model.training`: eğitim katmanlarının kimlikleri ve adları;
- `model.input`: girdi rasterinin dosyası ya da adresi;
- `model.sources`: girdi bir STAC varlığıysa öğesi ve çekim zamanı. Dış kaynağın sürümü budur.

**Denetimsiz sınıflandırma.** `model.method` `kmeans`, `model.classes` küme sayısı, `model.iterations`. `model.start`
başlangıcın rastgele olmadığını söyler: aynı girdi aynı sonucu verir; tohum yoktur.

**Doğruluk analizi.** Raster bir model kaydı taşıyorsa doğrulama nesnelerinden eğitim katmanının nesneleriyle kesişenleri
sayar (Konuma göre seç'in Kesişen'i). Sayı `leaked` çıktısıdır. Sıfırdan büyükse uyarır:

> n doğrulama nesnesi rasterin eğitim alanlarıyla örtüşüyor: doğruluk iyimser çıkar; eğitimde kullanılmayan nesnelerle
> doğrulayın.

Doğrulamanın eğitimle aynı örneklerden gizlice oluşması böylece görünür. Başvuru `remote_processing_cases.py`'nin
`accuracy-leak` durumudur; noktaların ve dikdörtgenlerin kesişimi Python'da kesin hesaplanır.

## Doğrulama

- `python3 scripts/fixtures/stac_access_cases.py --check`; `--botocore` ile botocore'un imzacısı.
- `cargo test -p kentos-services --lib access` (26 durum) ve `--lib stac`; `cargo test -p kentos-cloud --lib access`
  (AWS zamanı, RFC 3339 süresi, düz adres).
- `/usr/bin/python3 scripts/fixtures/raster_reproject_cases.py --check`;
  `KENTOS_CASES_ONLY=cad.rasters.reproject cargo test -p kentos-native-application --test all every_command_case`;
  `KENTOS_WRITE_REPROJECT=1 cargo test -p kentos-native-application --test all reproject_files`, ardından `--written`.
- `python3 scripts/fixtures/stac_cases.py --check`;
  `KENTOS_CASES_ONLY=cad.stac cargo test -p kentos-native-application --test all every_command_case`.
- `/usr/bin/python3 scripts/fixtures/remote_cases.py --check`; `cargo test -p kentos-raster --test all remote`;
  `/usr/bin/python3 scripts/fixtures/remote_processing_cases.py --check`; `cargo test -p kentos-processing --test cases remote`.
- `cargo test -p kentos-desktop --bin kentos-cad rasters::stac`: UTM 36 varlıkları TM36 projesine çevrilir.

Canlı Planetary Computer ve istek ödemeli kova denemesi yapılmadı: bu makinede AWS hesabı yok. İmza, belgenin örneği ve
botocore ile; SAS akışı birim testleriyle doğrulandı.
