# ADR 0518: Raster oturtmanın kalıcı kaydı

- Durum: Kabul edildi (19 Ekim 2026)
- Kapsam: plan/gis.md `CBS-29` (georeferans GCP/denetim noktası ve warp işlem kaydını kalıcılaştır). Kapsamı ben belirledim.
- İlgili: ADR 0204 §6 (Raster oturt, altı dönüşüm, yeniden örnekleme), ADR 0156 (Vektör oturtma'nın çözümleri), ADR 0158 (ince
  plaka), ADR 0168 (projenin sistemleri ve datum seçimleri), ADR 0253 (komut protokolü), ADR 0265 (şema değişikliğinin kuralı),
  ADR 0349 (bağlı dosyaların aranması).
- Web: ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

Raster oturt (ADR 0204 §6) altı dönüşümü, artıkları ve m0'ı, yeniden örneklemeyi biliyordu; ama yaptığı iş kalmıyordu. Uygula
rasterin `affine`'ini ya da kaynağını değiştiriyor, noktalar pencerenin belleğinde kalıyordu: çizim yeniden açılınca hangi noktalarla,
hangi yöntemle, hangi kaynaktan oturtulduğu bilinmiyordu; aynı oturtma yeniden üretilemiyordu. Doğruluk yalnız kontrol noktalarının m0'ıyla
söyleniyordu; dönüşüme girmeyen denetim noktası yoktu (ince plakada m0 bile yoktur: noktalardan tam geçer). Hedefler projenin
sisteminde olmak zorundaydı. Yeniden örneklenmiş raster çıktısını gösterdiği için ikinci bir oturtma çıktıyı yeniden örnekliyordu.
Yazma `cad.entities.edit`'in hazır geometri alan `rasterGeoref` işlemindeydi (CLAUDE.md §9.2'ye aykırı); Python ve yapay zekâ ajanı
rasteri oturtamıyordu. Noktalar dosyaya yazılıp okunamıyordu.

## Karar

### 1. Kayıt: rasterin `georef`'i (`.kcad` şema 88)

`RasterFields.georef` (`kentos_contracts::RasterGeoref`), oturtmanın yeniden üretilebilir kaydı:

- `method` (`helmert`, `affine`, `projective`, `poly2`, `poly3`, `thinPlate`);
- `points`: verildiği gibi noktalar; `pixel` kaynağın pikseli (sütun, satır; **piksel köşeleri**, sol üst pikselin sol üst köşesi
  0, 0: GDAL'ın GCP'leri gibi; yarım piksel kayma yok), `target` `srid`'in sisteminde; `check` **denetim noktası** (dönüşüme girmez,
  hatası ölçülür), `off` kullanılmayan; isteğe bağlı `name`;
- `srid`: hedefler projeninkinden başka sistemde verildiyse onun EPSG kodu;
- `source`: noktaların pikselleri olan raster: boyu ve `asset`, `file` ya da `url`'si, okunduysa baytlarının SHA-256'sı (`digest`:
  noktaların ait olduğu kaynak sürümü);
- `m0` (kontrol noktaları; ince plakada ve fazlası yokken yok), `checkRmse` ve `checkMax` (denetim noktalarının karesel ortalama ve
  en büyük hatası; denetim noktası varken ikisi, yokken hiçbiri);
- `output`: yalnız yeniden örnekleyen yöntemlerde: kuzeye bakan çıktı ızgarası `[x₀, s, 0, y₀, 0, −s]`, boyu, pikseli `s`,
  `resampling` (`nearest` ya da çift doğrusal) ve yazılan dosyanın SHA-256'sı.

Tutarlılık: çıktılı kaydın ızgarası rasterin `affine`, `width`, `height`'ıdır; çıktısızın kaynağının boyu rasterin boyudur
(`RasterGeoref::problem`, `misfit`; `RasterFields::problem` onları da sorar). Veri setli raster kayıt taşımaz. KCAD'de kayıt
sözleşmenin JSON'u olarak yazılır (dış başvurunun `xref`'i gibi), okuyucu yazıcının biçiminden sapanı reddeder; tipli sütunlarda
rasterin 5. seçeneği, kayıt tek JSON metni (`FORMATS_VERSION` 94). Şema 88, ADR 0265'in altı adımıyla: örnek `georef.kcad`, şema 87'de
reddedilen örnek, yedi bozuk örnek, bağımsız okuyucu `tools/kcad/kcad.py` aynı kuralla. Nesnenin yerini değiştiren her dönüşüm (Taşı,
Döndür, Vektör oturtma, birim değiştirme …) kaydı düşürür: raster artık kaydın koyduğu yerde değildir (`geometry::contract_raster`).

### 2. `cad.rasters.georef` v1

Anlamsal komut; girdisi kullanıcının verdiği: rasterin kimliği, yöntem, noktalar, isteğe bağlı `srid`, `pixel`, `resampling`,
`output`, `drawing`, `made`, `expectedRevision`. Sırası:

1. Hedefler projenin sistemine (`srid` projeninki değilse): kayıttaki sistemden projeninkine, projenin datum seçimleriyle
   (`crs::transform_in`; ADR 0168'in yolu). Yerel proje başka sistemi almaz (`target_not_converted`).
2. Kontrol noktalarıyla dönüşüm (`ops::georef::solve`); denetim ve kullanılmayan noktaların artığı da hesaplanır. Ret kodları:
   `too_few_control_points`, `control_points_duplicate`, `control_points_collinear`, `georef_singular`.
3. Yeniden örnekleyen yöntemde ızgara: kaynağın sınırının kenar başına 64 noktasının ileri dönüşümü, köşesi pikselin katında
   (`warp::Grid::covering`); piksel verilmezse kontrol noktalarındaki ortalama ölçek. Aynı kayıt aynı ızgarayı verir; kayıt pikseli
   tuttuğu için yeniden çalıştırma kaydın pikseliyle aynı ızgarayı ve aynı dosyayı bayt bayt üretir.
4. Okuma: yeniden örneklemede kaynağın başlığı (bant, tür, palet; kaynağın boyu kayıttakinden başkaysa ret) ve SHA-256'sı; yerleştiren
   yöntemde yalnız özet (bulunamazsa `source_not_read` uyarısı, kayıt özetsiz).
5. Yazma, tek adım “Raster oturt”: Helmert ve afin rasterin `affine`'ini değiştirir; ötekiler kaynağı yeniden örnekler: bağlı
   kaynağın çıktısı `output`'a ya da kaynağın yanına `<ad>-oturtulmus.tif` (önce `.part`, bütünse yeniden adlandırılır; kaynağın üstüne
   yazılmaz), gömülü kaynağınki projenin kitaplığına (en çok 32 MB). **Kaynak dosya hiçbir zaman değişmez.**

Yeniden örneklenmiş rasterin noktaları kaydının kaynağının pikselleridir: yeni oturtma çıktıdan değil kaynaktan yapılır; yerleştiren
yöntem rasteri kaynağına geri döndürür (boyu, bantları, türü kaynağın; görünüş bantlara uymuyorsa kaynağın varsayılanı).

**`made`:** masaüstü uzun yeniden örneklemeyi kendi iş parçacığında, Durdur'lu panelle, komutun planındaki ızgara ve tersle
(`rasters_georef::prepare`) yapar ve çıktısını komuta verir (dosya ya da kitaplıktaki raster). Komut onu okur; ızgarası (konum, piksel,
boy) planınkiyle bit bit aynı değilse `georef_output_mismatch`, hiçbir şey yazılmaz. Python, MCP ve başsız sunucu `made` vermez: komut
yeniden örneklemeyi kendisi yapar (GeoTIFF'in başlığı ve blokları gerektikçe okunur; JPEG sıkıştırmalı TIFF bu ortamda reddedilir).

Uyarılar: `no_check_points` (m0 yalnız kontrol noktalarının uyumunu söyler; ince plakada hiçbir şey söylemez), `source_not_read`.
Plan yazmadan her noktanın artığını, denetim hatalarını ve rasterin yeni hâlini (kaydıyla) verir. `cad.entities.edit`'in
`rasterGeoref` işlemi iç yazma yolu olarak kalır; Raster oturt aracı artık bu komutla yazar (`catalog/v1/operations.json`: `semantic`).

### 3. Pencere

Raster oturt'ta (ADR 0204 §6) tabloya **Denetim** ve **Ad** sütunları, **Hedeflerin sistemi (EPSG)** alanı; özet komutun kendi
hesabından (`rasters_georef::fit`: dosya okumadan) her değişiklikte: m0, denetim noktalarının karesel ortalama ve en büyük hatası,
uyarılar ve retler komutun sözleriyle, yeniden örneklemede çıktı ızgarasının boyu ve pikseli. Kaydı olan raster seçilince pencere
noktaları, yöntemi, sistemi, pikseli ve örneklemeyi kayıttan açar ve “Kayıtlı oturtma: …” satırı kaynağı ve özetini söyler.
Yeniden örneklenmiş rasterin üstünde tıklanan pikselin kaynağın pikseli kaydın dönüşümünün tersinden bulunur. İnce plakada kontrol
noktalarının artık hücreleri boş kalır (artık yok); denetim noktalarınınki dolar.

### 4. Noktalar dosyaya ve dosyadan

**Noktaları al…** ve **Noktaları kaydet…**: QGIS'in georeferencer'ının `.points`'i (GDAL'ın GCP sütunları): `mapX,mapY,sourceX,
sourceY,enable,dX,dY,residual`, `sourceY` eksi satır (piksel köşeleri), isteğe bağlı ilk `#CRS:` satırı (okunmaz, söylenir). KentOS iki
sütun ekler, QGIS'in görmezden geldiği: `role` (`control`, `check`, `off`) ve `name`; okurken `role` varsa o, yoksa `enable` (1 kontrol,
0 kullanılmayan) karar verir. Sütunlar başlıktaki adlarıyla bulunur; başlıksız dosya QGIS'in sırasıyla okunur. Yazılan artık uzunluğu
√(dx² + dy²) (her platformda aynı bitler). `kentos_formats::raster::gcp`.

## Doğrulama

- `python3 scripts/fixtures/raster_georef_command_cases.py --check`: `fixtures/commands/v1/cad.rasters.georef.json`'un 38 durumu
  KentOS kodu olmadan: afin ve polinomlar numpy'ın en küçük kareleriyle, ince plaka kendi denklem sistemiyle, Helmert ve projektif
  `raster_georef_cases.py`'nin çözümleriyle; afin GDAL'ın `GCPsToGeoTransform`'una (piksel köşeleri, döndürülmüş paftada dönüşüm
  yerleşimin kendisi), polinomlar ve ince plaka GDAL'ın GCP dönüştürücüsüne karşı denetlenir; EPSG:4326 hedefler PROJ'un GRS80'deki
  TM'siyle; ızgara ADR'nin kuralıyla; kaynakların SHA-256'sı; retler komutun sözleriyle. Varsayılan pikselli ince plaka ızgarası
  10⁻³ m içinde (pikseli ince plakanın türevlerinden; köşesi pikselin milyonlarca katı), ötekiler 10⁻⁶.
- `cargo test -p kentos-native-application --test all every_command_case_matches` (`KENTOS_CASES_ONLY=cad.rasters.georef`),
  `capabilities` (her kod bir durumda), `rasters_georef` (komutun kendi yeniden örneklemesi `georef.json`'un ince plakasında:
  ızgara aynı, tam tersle örnekleme gdalwarp'ınkiyle bit bit aynı, 8 pikselde bir tam ters ve aralarda çift doğrusalla en çok %0,5
  piksel farklı (burada 149 / 70 863, kenarına 0,02 pikselden yakın örnekler); kaynak değişmez; kayıtla yeniden çalıştırma aynı
  ızgarayı ve aynı baytları verir; ev sahibinin çıktısı planın ızgarasıysa alınır, değilse ret).
- Python: aynı 38 durum tipli katmandan (`test_command_cases`), `test_carriage` yeni örnekle.
- `python3 scripts/fixtures/gcp_file_cases.py --check` ve `cargo test -p kentos-formats --test all raster_gcp`: `.points` okuma ve yazma
  Python'un `csv`'siyle.
- KCAD: `cargo test -p kentos-kcad --test all` (`schemas`, `fixtures`, `columns`), `python3 scripts/fixtures/kcad_v2_reference.py --check`.
- Masaüstü: `cargo test -p kentos-desktop raster_oturt` (afin kaydı, yeniden örnekleme `made` ile, noktalar dosyaya ve geri), resimler
  `KENTOS_SHOTS_ONLY=raster-oturt,raster-oturt-ince-plaka,raster-oturt-kayit cargo test -p kentos-desktop tools_screens -- --ignored
  --nocapture`.

## Sonuçlar

- Oturtma çizimle birlikte kalır, yeniden açılır ve aynı sonucu verir; doğruluk RMS'in yanında bağımsız denetim noktalarıyla söylenir.
- Python ve yapay zekâ ajanı rasteri hesabı kendileri yapmadan oturtur (`kentos.cad.rasters.georef`, MCP aracı yıkıcı listede).
- Sınırlar: kayıt hedeflerin sistemini EPSG koduyla tutar (projenin özel tanımı değil); adresten okunan raster yalnız yerleştiren
  yöntemle oturtulur; başsız ortamda JPEG sıkıştırmalı TIFF yeniden örneklenmez; `#CRS:` satırı okunmaz. Büyük bağlı kaynağın özeti
  komut her çalıştığında baştan okunur (masaüstünde arayüzün iş parçacığında).
