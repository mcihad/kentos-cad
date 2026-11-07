# 1029 — Karşılaştırılan kotlar tek düşey sistemde

Tarih: 2026-11-07. Durum: kabul (Y95). Kapsam ROAD-NUM-04, ADR 0928 (buluttan arazi), ADR 0942 (dönem metrajı),
ADR 0916 §3 (yüzey farkı), ADR 0519 (projenin kotları) ve KCAD şeması.

## Sorun

İki zeminin karşılaştırması (dönem metrajı: tasarım, ilk arazi ve ara ölçü; yüzey farkı) bir kot farkıdır. İki yüzeyin
kotları ayrı düşey sistemlerdeyse sonuç her yerde sistemlerin ayrımı kadar kayar. Türkiye'de elipsoit yüksekliği
ortometrik yüksekliğin 30–40 m kadar üstündedir. EGM2008 ile TÜDKA-99 arasındaki ayrım da desimetreler mertebesindedir.
1 km'lik, 30 m genişliğinde bir yolda 10 cm'lik bir kayma 3.000 m³ demektir.

KentOS buluttan zemin alırken (`road.terrains.cloud`) dosyanın düşey sistemine bakmıyordu. Dosya düşey sistemini söylese
bile karşılaştırma onu bilmeden yapılıyordu: sessiz bir sıfır ayrım.

## Araştırma (7 Kasım 2026)

- **Türkiye:**
  - TÜDKA-99 ortometrik yükseklik sistemini esas alır. GNSS elipsoit yüksekliği verir ve ortometriğe geçiş bir jeoit
    modeli ister ([Dergipark, GNSS/nivelman](https://dergipark.org.tr/tr/download/article-file/451145)).
  - HGM'nin 2015–2020 projesiyle TG-20 resmî jeoit modelidir. GNSS/nivelman karşılaştırmasında 2–3 cm doğrulukludur
    ([TÜBİTAK MAM](https://mam.tubitak.gov.tr/en/turkeys-altitude-system-modernized-in-line-with-scientific-advances/)).
  - BÖHHBÜY elipsoit yüksekliklerinin dönüşümünde jeoit modeli kullanılmasını öngörür. Yönetmeliğin madde metnine bu
    araştırmada ulaşılamadı.
  - KGM'nin etüt-proje şartnamesinin düşey datum maddesi bu araştırmada bulunamadı. ADR 0984'teki 2019 şartnamesi ve
    2024 örnek paftaları kotu tek sistemde verir, sistemlerin karıştırılmasına bir kural koymaz.
- **LAS 1.4 (ASPRS):**
  - Koordinat sistemi WKT'yle (`LASF_Projection` 2112) ya da GeoTIFF anahtarlarıyla (34735) yazılır. WKT'ye geçişin
    amacı yatay ve düşey sistemi doğru taşımaktır ([LAS 1.4 R16 taslağı](https://github.com/ASPRSorg/LAS/files/6053302/LAS.pdf),
    [libLAS](https://liblas.org/development/wkt.html)).
  - Düşey sistem WKT'de `COMPD_CS` içindeki `VERT_CS`'tir, GeoTIFF'te `VerticalCSTypeGeoKey` (4096).
  - Dosya söylemeyebilir: KentOS'un kendi sınama bulutu `road-ground.las` söylemez.
- **Civil 3D:**
  - Autodesk forumunda bir kullanıcının içe aktardığı GIS eşyükselti eğrileri ölçümünden yaklaşık 1 m (3,3 ft)
    uzakta çıktı. Yanıt veren, Civil 3D'nin bir yüzeyin ya da verisinin düşey datumuyla ilgilenmediğini söylüyor
    ([Autodesk forumu](https://forums.autodesk.com/t5/civil-3d-forum/surface-from-gis-data-vertical-datum/td-p/9605212)).
  - Hacim yüzeyi iki TIN'i datum sormadan çıkarır.
  - Drone ölçümü yazılımları farklı CRS'lerdeki verinin hacmi yanlış verebileceği uyarısını yapar
    ([Propeller](https://help.propelleraero.com/hc/en-us/articles/35899763388183-Why-do-my-files-look-misaligned-after-switching-CRS)).
- **Netcad:** bu araştırmada kaynak bulunamadı.
- **Matematik:** hesap yok, karşılaştırma var. H = h − N: aynı yerde iki sistemin farkı jeoit (ya da iki datumun)
  ayrımıdır. Sıfır sayılmaz. Tahmin de edilmez: KentOS'un çevirisi (`cad.crs.transform`, ADR 0519) bir geoit
  ızgarası ister.

## Karar

- **Bir yüzeyin kotlarının kaynakları:**
  - Çizimin kendi noktaları ve çizgileri projenin kotlarındadır (`settings.heights`).
  - Buluttan alınan zemin bulut dosyalarının söylediği sistemdedir.
  - Bulut katmanından başka nokta katmanı, nesne ya da kırık hat varsa yüzeyin çizimden de kotu vardır.
- **`SurfaceCloudDef.vertical`** (KCAD şema 202; Dalda 195 olarak yazıldı; main'le birleşirken 193–199 Harita işlerine verilmişti, 202 oldu.): dosyanın düşey sistemi, söylediği gibi. Bu, WKT'nin
  `VERT_CS`/`VERTCRS` adı ya da `EPSG:n`'dir. Boş olmayan, en çok 200 harf olur. Yoksa bilinmiyor demektir: dosya
  söylemiyor, kaynaklardan biri söylemiyor ya da kayıt kuraldan önce yapılmış.
- **`road.terrains.cloud`**, kaynakları okumadan önce başlıklarına bakar (açılmayan kaynak taramanın uyarısına kalır).
  Şu durumlarda `cloud_heights_differ` ile reddeder:
  - iki kaynak ayrı sistem söylüyorsa;
  - kaynakların sistemi projeninkinden başkaysa;
  - yüzeyin öbür bulut kayıtlarından biri başka sistemdeyse.
  Adlar EPSG'nin listesindeki (`VERTICAL_SYSTEMS`) adla eşlenir: "EGM2008 height" ile `EPSG:3855` aynıdır.
- **`road.volumes.progress`** (tasarım, ilk arazi, ara ölçü, önceki ölçü) ve **`road.terrains.compare`**:
  - Kaynakların hepsi çizimin kendisiyse karşılaştırma eskisi gibidir. Proje sistemini söylemese de kotlar tek
    sistemdedir.
  - Aksi hâlde her kaynağın sistemi bilinmeli ve aynı olmalı. Biri bilinmiyorsa `unproven_heights` reddi gelir; ileti
    hangi kaynağın bilinmediğini söyler.
  - `assumeHeights: true` kotları aynı sayar. Sonuç `heights_assumed` uyarısıyla bir taslaktır.
  - Bilinen iki ayrı sistem `assumeHeights` ile de reddedilir: o ayrım bir varsayımla sıfır olmaz. Kotlar önce
    çevrilmelidir.
- **Pencereler:**
  - Kübaj'ın Dönem metrajı bölümünde "Kotlar: Aynı sistemde gösterilmeli / Aynı say (taslak)".
  - Yüzey analizi Fark kipinde "Kotları aynı say: Yok/Var".
  - Pencere iletisi `assumeHeights` yerine kendi seçeneğini söyler.
- **Kapsam dışı:** tasarımın kendi zeminiyle kübajı (`road.volumes`, yöntemlerin hepsi) karşılaştırmalı sayılmadı.
  Boykesit o zeminin üstüne çizilir; tasarım ile zemin aynı kaynaktan gelir.

## Ölçüm

- **Bağımsız başvurular** (KentOS kodu yok):
  - `road_cloud_terrain_cases.py`: aynı noktalarla iki yeni LAS yazar (laspy). `road-ground-antalya.las` GeoTIFF
    anahtarıyla EPSG 5775'i söyler; `road-ground-egm2008.las` `COMPD_CS`/`VERT_CS` "EGM2008 height" ile söyler.
  - `road_terrain_compare_cases.py` ve `road_volume_progress_command_cases.py`: kayıtları verilen bulut zeminleri.
- **`road.terrains.cloud`** 48 durum, 8'i yeni:
  - Dört başarılı durum: iki dosya kaydı, projeyle aynı sistem, biri söylemeyen iki bulut.
  - Dört ret: projeden başka sistem, elipsoit projesi, iki kaynak iki sistem, yüzeyin öbür bulutu.
  - Kayıttaki "EPSG:5775" "EPSG:5776" yapılınca durum düşüyor: alan gerçekten karşılaştırılıyor.
- **`road.terrains.compare`** 27 durumdan 5'i, **`road.volumes.progress`** 24 durumdan 4'ü yeni:
  - bilinmeyen sistem reddi ve varsayımlı taslak (çıktı varsayımsızınkiyle aynı, uyarı başta);
  - aynı sistem;
  - iki ayrı sistem (varsayımla da ret).
- **Öbür ortak durumlar:** eski bütün yol durumları değişmeden geçti (Rust 2737 durum; Python tiplerinden aynı yol
  durumları).
- **Durum dosyalarının boyu:** yeni durumların çizimi yalnız karşılaştırılan yüzeyleri ve noktalarını taşır, nesneler
  yeniden numaralanır (Python katmanı kimliğin dosya sırası olmasını ister).
  - Tam çizimle `road.terrains.compare.json` 405 KB'tan 1,46 MB'a, `road.volumes.progress.json` 1,26 MB'tan
    2,45 MB'a çıkıyordu.
  - Kırpılınca 658 KB ve 1,83 MB oldu.
- **KCAD:** referans ile okuyucu 1650 dosyada aynı sonucu veriyor; yeni örnek `road-cloud-heights` ve
  `broken/road-cloud-heights-in-schema-201.kcad`.
- **Resimler** `.run/gui-evidence/yol-num04/`:
  - Kübaj'da ret ve taslak (1440 açık, 1100 koyu); Yüzey analizinde ret ve taslak.
  - İlk resimlerde pencere iletisi "assumeHeights verin" diyordu. Pencerenin kendi seçeneğini söyler oldu.

## Açık

- **Bulutu çevirmek:** KentOS bulut noktalarını bir geoit ızgarasıyla çeviremiyor. Ayrı sistemdeki bulut yalnız
  dışarıda çevrilip yeniden eklenebilir.
- **Çizime aktarılan dosyalar:** LandXML, GeoPackage ve GIS'ten gelen noktaların sistemi uyarıyla söyleniyor
  (ADR 0631 §4) ama yüzeyde kaydedilmiyor. Onlar çizimin kendi kotu sayılıyor.
- **Hakediş onayı:** dönem metrajının taslağı onaylanan bir kayıt değil; varsayımlı bir sonucun kalıcı izi yok.
