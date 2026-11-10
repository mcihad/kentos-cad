# 4840 — İzlemede dönemlerden doğru ve iki eşik

Tarih: 2026-11-10. plan/map-cad.md MAP-19, dokuzuncu tur. Durum: taslak. ADR 4420'nin “Sonra”sındaki “dönemlerden doğru
uydurma (hız ve belirsizliği), uyarı ve alarm eşikleri”. Maddenin sıradaki işinin ikinci parçasıdır. Birinci parça,
yetkili inceleme kaydı, projenin kaydına yeni bir bölüm (`.kcad` şeması) ve inceleyenin kimliğini ister; açık kalır
(Sonra).

İlgili:
- [ADR 3620](3620-asbuilt-job-kinds-and-monitoring.md): iş türleri, iki dönemli izleme, kontrol noktaları;
- [ADR 4420](4420-monitoring-series-of-epochs.md): dönem dizisi, iz ve ortalama hız;
- [ADR 2160](2160-network-similarity-article-15-1-c.md): çekirdeğin Student t nicemi.

## Dayanak

10 Kasım 2026'da okundu; notlar `.run/research/map-cad/uygulama/MAP-19.9.md` ve `.run/research/map-cad/mevzuat/MAP-19.9.md`.

- **Trimble 4D Control, alarm tanımı**
  ([belge](https://help.fieldsystems.trimble.com/trimble-4d-control/alarms-add-basic.htm)):
  - koşulun türü gözlem (Observation) ya da hız (Velocity);
  - hız, kayan penceredeki gözlemlere uydurulan doğrusal regresyonun eğimidir, her yeni gözlemde yeniden bulunur;
  - üç düzey: Attention, Warning, Alarm; değerlendirme son değerle, son X'in ortalamasıyla ya da tepeyle;
  - en az gözlem sayısını ve hızın belirsizliğini belge yazmaz.
- **Trimble Business Center, Monitor Point Movement** ([belge](https://help.fieldsystems.trimble.com/tbc/25359.htm)):
  noktalara Warning ve Alarm yer değiştirme eşikleri; aşan nokta grafik görünümlerde sarı ya da kırmızı. Hız uydurmaz.
- **Leica GeoMoS Monitor** (ürün listesinin özeti; resmî kılavuz açık erişimde bulunamadı): sarı, turuncu ve kırmızı
  düzeyli sınır denetimi; denetim türleri mutlak, kısa süreli, uzun süreli ve regresyon.
- **Bentley iTwin IoT, Data Alerts**
  ([belge](https://communities.bentley.com/products/itwin-iot/w/itwin-iot-wiki/66099/data-alerts)): uyarı onaylanana
  dek yeniden tetiklenir. Onay inceleme kaydının işidir (Sonra).
- **Mevzuat:**
  - DSİ'nin, TBDY 2018'in, AFAD'ın ve KGM'nin aranan metinlerinde deformasyon izlemesi için uyarı ya da alarm eşiği yok;
    bulunanlar akademik çalışmalardır, eşik vermez;
  - BÖHHBÜY'ün hız kullanan tek hükmü C derece noktalarını referans epoğuna taşımaktır (m. 21). İzleme noktalarına
    doğru uydurma ya da hız sınaması için hüküm yok.

  Sonuç: eşikler projenin ya da tasarımcının değeridir, KentOS öntanımlı sayı koymaz. Doğru uydurma bir yöntem seçimidir,
  sonucu “deformasyon kanıtı” diye sunulmaz.
- **Yöntem:** en küçük karelerle doğru d(t) = a + v·t:
  - v = Σ(t − t̄)(d − d̄) / Σ(t − t̄)², a = d̄ − v·t̄;
  - artıklar r = d − (a + v·t), s² = Σr² / (n − 2), σᵥ = s / √Σ(t − t̄)²;
  - bileşen, c güvenle |v| > t(n − 2; (1 + c)/2)·σᵥ ise anlamlı hareket eder (ADR 2160'ın Student t'si).
- **KentOS bugün** (ADR 4420): hız iki uç dönemden ortalamadır, belirsizliği yoktur; tek sınır vardır.

## Karar

1. **`cad.asbuilt.check`'in izleme dizisine `trend: {confidence}`** (komut sürümü değişmez). Günlü dönemlerinden en
   az üçünde ölçülen her nokta için doğu, kuzey ve kot hareketine ayrı doğru:
   - t, noktanın ilk günlü döneminden yıl (365,25 gün); d, başvurusuna göre hareketi (ADR 4420'nin adımı);
   - kot doğrusu iki kotun da bulunduğu günlü dönemlerden, en az üç;
   - dönemler ağırlıksızdır: kendi standart sapmaları bilinmez.

   Cevapta izin `fit`'i:
   - `epochs`: doğrudaki dönem sayısı;
   - `east`, `north`, `up` (m/yıl) ve `sigmaEast`, `sigmaNorth`, `sigmaUp`;
   - `speed`: √(east² + north²);
   - `moving`: anlamlı hareket eden bileşenler (`east`, `north`, `up`).

   Üçten az günlü dönemde `fit` yoktur. Kusursuz bir doğruda (σᵥ = 0) bileşen, v sıfır değilse hareket eder.
2. **`thresholds: {warning, alarm}`**, her düzeyde dört isteğe bağlı sınır:
   - `horizontal`: son yatay hareket (m);
   - `vertical`: son kot hareketinin mutlak değeri (m);
   - `horizontalRate`: yatay hız (m/yıl), doğrunun `speed`'i, doğru yoksa ortalama `rateHorizontal`;
   - `verticalRate`: kot hızının mutlak değeri, doğrununki, yoksa ortalama `rateVertical`.

   Boş sınır denetlenmez. Her izin `state`'i:
   - alarm düzeyinin bir sınırını aşan `alarm`;
   - değilse uyarı düzeyininkini aşan `warning`;
   - öbürleri `normal`.

   `exceeded`, o düzeyde aşılan sınırlardır. Aşmak kesin büyüktür: sınıra eşit değer aşmaz.
3. **Retler** (`invalid_asbuilt_check`):
   - dizisiz doğru ya da eşik (`trend`, `thresholds`): “Doğru uydurma ve eşikler izleme işinin dönem dizisinindir
     (kind: monitoring ve önceki dönemler, epochs).”;
   - 0 ile 1 arasında olmayan güven (`trend.confidence`);
   - sıfır ya da eksi sınır (`thresholds.warning.vertical` gibi), “… 0'dan büyük bir sayı olmalı; denetlenmeyecekse boş
     bırakın.”;
   - alarmınkinden küçük olmayan uyarı sınırı (`thresholds.warning.horizontalRate` gibi).
4. **Masaüstü** (Uygulama kontrolü › İzleme), önceki dönemler yazılınca **Dönem dizisi** bölümü:
   - **Doğru uydurma:** “noktanın günlü dönemlerinden en küçük karelerle” kutusu ve güven yüzdesi. Formun hazır değeri
     %95'tir; komutun öntanımlısı yoktur;
   - **Uyarı eşiği yatay / düşey (m)** ve **Alarm eşiği yatay / düşey (m)**: iki hareket kutusu, “hız (m/yıl)”, iki hız
     kutusu. Birimler pencerenin hareket sınırınınkidir (m);
   - özette “Eşikler: a nokta alarmda, w uyarıda, n olağan. Eşikler sizindir; izleme için yönetmelik değeri yoktur.”;
   - dönem dizisinin altında **Hareketler | Doğru ve eşikler** seçimi, bir anda bir tablo. Doğru ve eşikler: Nokta,
     Dönem, Doğu, Kuzey ve Kot m/yıl ± σ, Yatay hız m/yıl, Anlamlı hareket, Durum (“Alarm (yatay, yatay hız)”). Seçim
     yeniden denetlemez;
   - Önceki dönemler kutusu üç dört dönemin adı sığsın diye genişledi.
5. **Kapsam işareti:** ADR 3620 gibi **Harita + Yol + CBS**.

## Sınırlar

- Doğru ağırlıksızdır. İvme, kayan pencere (T4D'nin hesap penceresi) ve ters hız yok.
- Anlamlılık bileşen bileşen t sınamasıdır: doğu ile kuzeyin birlikte sınanması yok, MAP-05'in sınamalarına bağlı
  değil. Sonuç yöntemin cevabıdır, deformasyonun kanıtı değil.
- Eşiklerin kaynağı (proje, şartname, gün) yazılmaz; durum kaydedilmez, çizime bir şey yazılmaz; onay ve bildirim yok.
- Büyük koordinatlar: üç neredeyse doğrusal dönemin σ'sı 5·10⁻⁵ m/yıl kadarken koordinatların son bitleri onu 10⁻⁹
  oynatır. Başvuru noktaları küçük yerel sayılarıyla taşır; ortak durumların payı ADR 4420'deki gibi 10⁻⁶'dır.
- Sınırda karar kesin büyükle verilir. Ortak durumlar sınama sınırından en az %1, eşiklerden en az 1 µm uzaktır.
- Yetkili inceleme kaydı yok.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/asbuilt_series_cases.py`; KentOS kodu yok), 12 yeni durum (34):
  - dördüncü dönemli çizim (2026-10-15): M1 ile M2'nin hareketi sürer, M3 oturmayı sürdürür, öbürleri yerinde durur;
  - Student t'nin kapalı biçimleri (bir ve iki serbestlik);
  - doğru: üç günlü dönem (iki dönemli noktada doğru yok), dört dönem, güven %50;
  - eşikler: hareketle, doğrunun hızıyla, doğru istenmeden ortalama hızla, dört dönemde doğruyla birlikte;
  - beş ret.

  Eski 22 durum bayt bayt aynı. Rust ve Python geçer. Bozma denemesi: bir durumun `alarm`'ı `warning` yazılınca koşucu
  yakalar (34'te 1).
- **Çekirdek** (`survey::trend::tests`): kusursuz doğruda σ = 0; elle üç nokta (v = 1,5, σ = √(1/12)); az nokta.
- **Python** (`python/tests/test_asbuilt_trend.py`), testin kendi hesabıyla:
  - her doğru `numpy.polyfit`'in, σ'ları artıklardan, t'ler kapalı biçimlerinden (üç ve dört dönemli noktalar);
  - durumlar iki düzeyden.

  ADR 4420'nin Python testi geçer.
- **MCP** (`crates/native/mcp/tests/asbuilt_trend.rs`): üç durumun izleri (doğru, eşikler, dört dönemde birlikte),
  uyarı ≥ alarm reddi; durumun payıyla. Dizinin MCP testi geçer.
- **Masaüstü** (`asbuilt_trend_tests`):
  - dört dönemli çizimde form durumun izlerini verir;
  - özet satırı “2 nokta alarmda, 2 uyarıda, 5 olağan”;
  - M2'nin satırı (dört dönem, doğu ± σ, “doğu, kuzey”, “Alarm (yatay, yatay hız)”), yerinde duran bileşen işaretsiz;
  - tablo seçimi yeniden denetlemez;
  - uyarı ≥ alarm ve sayı olmayan güven söylenir; doğru ve eşikler kalkınca dizi eskisi gibi.

  Uygulama kontrolü testleri ve çapraz denetimler geçer. Resimler `.run/gui-evidence/dogru-esik-4840/` (form ve tablo;
  açık geniş, koyu dar).

## Sonra

- Yetkili inceleme kaydı (MAP-19'un öbür maddesi): yeni `.kcad` bölümü; inceleyenin rolü, unvanı, adı, günü, ibaresi
  (“Kontrol edilmiştir”, “Görülmüştür”), imza türü; kaynak değişince kayıt eski sonuca aittir.
- Eşiklerin kaynağıyla projede saklanması.
- Hareketin zaman grafiği (ölçüler, doğru, eşik çizgileri).
- Dönem σ'larıyla ağırlıklı doğru, kayan pencere, doğu ile kuzeyin birlikte sınanması.
- Web (TODOS §0.2).
