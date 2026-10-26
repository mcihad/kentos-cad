# ADR 0652: Her İşlemler aracının ortak durumu ve komutla çalışması: nokta bulutu, çok boyutlu veri ve COG

- **Durum:** kabul edildi (2026-10-26). `plan/gis.md` CBS-41'in dilimi. Kapsamı ben belirledim.
- **Kalan işler:** ADR 0548, 0572 ve 0573'ün “Kalan”ı: kapsam matrisinde ortak durumu olmayan 10 araç, komutla durumu olmayan 13 araç.
- **Bağlam belgesi:** ADR 0207 §7 (nokta bulutu işlemleri), 0243 (çok boyutlu veri), 0515 (COG), 0548 (kapsam matrisi ve eşdeğerlik),
  0572 (bulutun ve NetCDF'in başsız dosya işi).
- **Web:** ertelendi (ADR 0228). TODOS.md §0.2'de.
- **`.kcad`:** şema değişmez.
- **Bağımlılık:** yeni kitaplık yok.
- **Sonraya kalan:**
  - işlem araçlarının kalan 287 açık seçenek hücresi (çoğu raster araçlarının `add=false`'u, Rasterleştir'in ve yüzey analizinin
    seçenekleri) ve komutların açık hücreleri;
  - modellerin ve uzun işin kalanı;
  - doğal dil başarısının API kapsamından ayrı ölçümü (ajanın İşlemler işinin değerlendiricisi, ADR 0446'nın form işi gibi).

## Bağlam

Kapsam matrisi (ADR 0548) İşlemler'in 153 aracından 10'unun hiç ortak durumu olmadığını, 13'ünün de komutla
(`cad.processing.run`, Python, MCP) hiç denenmediğini söylüyordu: dokuz nokta bulutu aracı (Seyrelt, Zemin süzgeci, Yüksekliğe
göre sınıfla, Bulutu kırp, Nokta bulutu alan sorgusu, Karola, Rasterleştir, Sınır çıkar, Bulutları birleştir), COG olarak kaydet
ve üç çok boyutlu araç (Kesit, Zaman serisi, Mesh hesaplayıcı). Nokta bulutu araçları çekirdekte (ADR 0207) bağımsız başvuruyla
sınanıyordu ama araç olarak, yani parametreleriyle, yazdıkları dosyayla ve çizime eklenen nesneyle sınanmıyordu. Çok boyutlu
araçların ortak durumları vardı, eşdeğerlik testine bağlı değildi. Ortak durum koşucusunun bellekteki dosya ev sahibi bulut
açmıyordu.

## Karar

### §1 Nokta bulutu araçlarının ortak durumları

`fixtures/processing/v1/pointcloud.json` (26 durum) ve çizimi `pointcloud.kcad`, `scripts/fixtures/pointcloud_processing_cases.py`
yazar, KentOS kodu olmadan: bulutlar bağımsız başvurunun dosyalarıdır (köyün LiDAR'ı, `sinir.laz`, `ikinci.laz`), her durumun
sonucu `pointcloud_ops_cases.py`'nin işlevleriyle (Seyrelt'in üç yöntemi, SMRF, yüksekliğe göre sınıf, alanlar, Rasterleştir,
Sınır çıkar, Birleştir) laspy ve numpy'la yeniden hesaplanır. Durumlar araçların seçeneklerini gezer: LAZ, LAS ve COPC; çizime
eklenen ve eklenmeyen; Yalnız son dönüşler açık ve kapalı; bütün noktalar ya da yalnız sınıfsızlar; içindekiler ve dışındakiler;
sanal bulutlu ve sanal bulutsuz karolar; Rasterleştir'in beş değeri ve üç sınıf seçimi; üç ret (iki bulut, tek dosya, iki sistem).

Yazılan dosya okunur ve denetlenir (`cloudOf`): nokta sayısı, noktaların X, Y ve Z tam sayılarının FNV-1a 64'ü dosyanın sırasıyla
(COPC'de sıralanmış: dizin noktaları sıralar), sınıfların ya da (Birleştir'de) bütün kayıtların özeti; raster (`cloudRasterOf`) COG
mi ve hücrelerinin float32 bitlerinin özeti; `written` bir çalışmanın bıraktığı bütün dosyaları adlandırır (COPC'nin ara LAZ'ı
silinir). Eklenen nesnenin kutusu ve kaynakları dosyanın toleransıyla (`toleranceFields`).

### §2 Bellekteki ev sahibi bulut okur ve COPC yazar

Ortak durum koşucusunun `FixtureFiles`'ı bulutu durumların dosyalarından ya da bir çalışmanın yazdığından `cloud_files::cloud_read` ile
okur; COPC'yi masaüstünün yoluyla (`cloud_files::copc_file`) kendi geçici klasöründe kurup yazılanlara koyar; `remove` yazılanı siler
(önceden hiçbir şey yapmıyordu). Böylece aynı durum dosyası ortak durum testinde ve eşdeğerlik testinde koşar.

### §3 Eşdeğerlik ve COG

Eşdeğerlik testi (ADR 0548) `pointcloud.json`'u ve `multidim.json`'u da pencereden ve `cad.processing.run`'dan oynatır; yazılan
dosyalar bayt bayt aynı. COG olarak kaydet'in durumu `raster-ops.json`'da (`cog`): raster işlemleri başvurusunun yeni `kopya-kot`
durumu (`{"kind": "copy"}`, değerler kaynağınki) yazılan COG'un 0. katını denetler; sonuç kaynağın görünüşünü taşır.

## Doğrulama

- `python3 scripts/fixtures/pointcloud_processing_cases.py --check`; `cargo test -p kentos-processing --test cases
  the_point_cloud_cases` (26 durum, ilk koşuda 25'i tuttu: ayrılan tek şey Sınır çıkar'ın alan katmanının dolgu rengiydi, biçim).
- `/usr/bin/python3 scripts/fixtures/raster_ops_cases.py --check` (75 durum), `raster_ops_processing_cases.py --check` (16 durum);
  `cargo test -p kentos-raster --test all raster_ops`, `cargo test -p kentos-processing --test cases the_raster_operations_cases`.
- `cargo test -p kentos-processing --test cases the_window_and_the_command` (pointcloud ve multidim dahil).
- Kapsam matrisi (`.run/py/bin/python scripts/gis/coverage.py`): ortak durumu olmayan araç 10 → 0, komutla durumu olmayan araç
  13 → 0, işlem araçlarının denenen seçenek hücresi 453 → 501 (788'den).
- Başvurunun iki küçük bulutu her yazılışta başka bayttı (laspy başlığa bugünün gününü yazıyordu); gün dosyaların kendi günüyle
  sabitlendi, `pointcloud_ops_cases.py --check` artık her gün aynı.

## Ölçümler

Çalışan kod değişmedi; yalnız testler ve durumlar. Debug, bu makinede: nokta bulutu durumları 6,3 s (26 durum, COPC'lerle),
eşdeğerlik testi 16,7 s (pointcloud ve multidim eklenince).
