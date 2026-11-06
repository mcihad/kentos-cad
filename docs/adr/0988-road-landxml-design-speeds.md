# 0988 — LandXML'de proje hızları

Tarih: 2026-11-06. Durum: kabul (Y82). Kapsam ROAD-IO (LandXML alışverişi) ve ADR 0985 ile 0986'nın §Açık'ı ("LandXML
DesignSpeed").

## Sorun

Y79'dan beri aksın proje hızı ve kurbun kendi hızı saklanıyor. LandXML ile gelen ya da giden akslar ise hızlarını
taşımıyordu: Civil 3D'de ya da OpenRoads'da hızıyla tasarlanmış bir aks KentOS'a hızsız geliyordu.

## Araştırma (6 Kasım 2026)

- **LandXML 1.2 şeması** (landxml.org'dan indirilen XSD):
  - `DesignSpeed`, `Alignment` altında değildir. `Roadways/Roadway/Speeds` altındadır (şemanın "FHWA IHSDM elements"
    bölümü).
  - Öznitelikleri `staStart`, `staEnd`, `speed`. `speed` tipi "kilometers/hour for Metric units and miles/hour for
    Imperial".
  - `Roadway`, aksı zorunlu `alignmentRefs` ile anar: boşlukla ayrılmış ad listesi.
  - `Units/Metric` ve `Units/Imperial`'ın `velocityUnit`'i: `kilometersPerHour`, `metersPerSecond`; `milesPerHour`,
    `feetPerSecond`.
- **Birim çevirisi:** 1 mil = 1,609344 km ve 1 ft = 0,3048 m tanım gereği tamdır (uluslararası yard ve pound
  anlaşması, 1959). 3516 sayılı Ölçüler ve Ayar Kanunu SI birimlerini esas alır; km/sa'e çevrilir.
- **Türkiye mevzuatı:** LandXML için bir hüküm yok. KGM'nin teknik şartnamesi sayısal teslim biçimi yazmaz.
- **Civil 3D:** aksın "Design Speeds" tablosu km'den başlayan hızlardır. LandXML'e nasıl yazdığı bu araştırmada
  doğrulanamadı.

## Karar

- **Okuyucu** (`kentos_formats::landxml`):
  - `LxAlignment.speeds`: aksı `alignmentRefs`'te anan her `Roadway`'in `DesignSpeed`'leri, km/sa'e çevrilmiş;
    istasyonlar dosyanın uzunluk biriminde metreye.
  - Boşluklu bir ad, refs'in tam metniyle de eşleşir. Şemanın listesi boşlukla bölündüğü için "Ana aks" başka türlü
    anılamazdı.
- **İçe aktarma** (`road.alignments.import`):
  - Toplam en uzun aralığın hızı aksın proje hızı olur.
  - Dairesel kısmının ortası başka hızlı bir aralığa düşen kurbun PI'sı o hızı alır.
  - 0'dan büyük ve en çok 200 km/sa olmayan hız alınmaz, `landxml_left` ile söylenir.
- **Yazıcı** (`file.export.landxml`):
  - Kendi hızı aksınkinden farklı her kurp TS–ST aralığında o hızla, kalan yerler aksın hızıyla yazılır.
  - `Roadway`'in adı ve `alignmentRefs`'i aksın adıdır.
  - `velocityUnit="kilometersPerHour"` yalnız hız yazılan dosyada eklenir; hızsız dosyalar bayt bayt eskisi gibidir.
  - Paralel aksın hızı yazılmaz, kaynağının hızı kaynakla gider.

## Ölçüm

- **Okuyucu testi:** metrik, İngiliz (80 mil/sa → 128,748 km/sa, istasyon ayakla) ve m/s (60 → 216 km/sa); boşluklu
  ad; iki aksı anan refs.
- **Ortak durum** (`road_landxml_cases.py`, KentOS kodu yok): `aks-hiz.xml`.
  - Ana aks 1000–1250 80, 1250–1350 50, 1350–bitiş 80 ve bir 250.
  - Beklenen: p1'in hızı 50 (dairesel ortası ≈1276, üreticide denetlenir), 250 için uyarı.
  - `road.alignments.import` 22 durum geçti, `file.export.landxml` 22 değişmeden geçti.
- **Gidiş-dönüş testi** (`tests/all/landxml_files.rs`):
  - 80 km/sa'lik aks ve 50 km/sa'lik kurbu yazılır: başlangıç–TS 80, TS–ST 50, ST–bitiş 80.
  - Dosya `xmllint` ile LandXML 1.2 şemasına uyar.
  - Boş bir projeye geri alınınca aks 80, p1 50.
- **Testler:** masaüstü 1853 (`RUST_MIN_STACK=67108864`), uygulama, formats, plot ve komutlar 2767.
- **Resim** `.run/gui-evidence/yol-y82/`: içe aktarmadan sonra Güzergâh, proje hızı 80 ve p1'in V'si 50.

## Açık

- **Km aralığıyla hız modeli:** KentOS aks + kurp hızı tutar. Bir aliyman parçasının kendi hızı (Civil 3D'nin
  aralıkları) içe aktarmada kurbu olmayan yerde kaybolur.
- **`DesignSpeed85th` ve `PostedSpeed`** okunmuyor.
