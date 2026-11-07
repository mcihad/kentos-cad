# 1031 — Sahaya giden dosya sistemini söyler

Tarih: 2026-11-07. Durum: kabul (Y96). Kapsam ROAD-SEC-10 (bir kısmı), ADR 0908 §6 ve §9 (LandXML ve aplikasyon
gönderimi), ADR 1029 (karşılaştırılan kotlar), ADR 0519 (projenin kotları).

## Sorun

Aplikasyon dosyası (`road.stakeout.send`) ve makine kontrolüne giden tasarım (`file.export.landxml`) sahada bir aletin
uygulayacağı kotları taşır. İkisi de kotların hangi düşey sistemde olduğunu söylemiyordu; LandXML dosyası koordinat
sistemini de söylemiyordu. Projenin kotlarının sistemi bilinmese de dosya yazılıyordu.

GNSS gezgini elipsoit yüksekliği verir; bir jeoit modeli ya da yerelleştirme onu ortometriğe çevirene kadar
aradaki fark Türkiye'de 30–40 m'dir. Sistemi söylemeyen bir dosya, sahada yanlış bir varsayımla uygulanır.

## Araştırma (7 Kasım 2026)

- **BÖHHBÜY** (Resmî Gazete 26.06.2018), Madde 1(1)(b): harita bilgileri "Türkiye Ulusal Referans Çerçevesi
  (TUREF)'ne dayalı üç boyutlu kartezyen koordinatları (X,Y,Z)" ve "Türkiye Ulusal Düşey Kontrol Ağı-1999
  (TUDKA99)'a dayalı Helmert ortometrik yükseklikleri (H)" ile üretilir
  ([metin](https://www.alomaliye.com/2018/06/26/buyuk-olcekli-harita/amp/)). Yönetmeliğin aplikasyon ve jeoit
  maddelerine bu araştırmada ulaşılamadı.
- **TG-20** HGM'nin resmî jeoit modelidir (ADR 1029'un kaynakları).
- **KGM:** harita mühendisliği teknik şartnamesinin aplikasyon ve kot maddeleri bu araştırmada bulunamadı.
- **LandXML 1.2 şeması** (`fixtures/road/v1/landxml/LandXML-1.2.xsd`): kökün seçimlerinden biri `CoordinateSystem`.
  Öznitelikleri arasında `name`, `epsgCode`, `horizontalDatum`, `horizontalCoordinateSystemName`, `verticalDatum`,
  `verticalCoordinateSystemName` ve `desc` var. Örnek: `name="AGD66 - AMG Zone 52" epsgCode="20252"`.
- **buildingSMART Finlandiya'nın LandXML 1.2 profili**
  ([başlık verileri](https://buildingsmart.fi/en_GB/infra/bSI_LandXML12_MVD/pages/1_Header.html)):
  - `CoordinateSystem` başlıkta zorunludur.
  - EPSG sistemi `name` ve `epsgCode` ile yazılır; yerel sistem `horizontalCoordinateSystemName` ile yazılır, EPSG
    alanları boş kalır.
  - Yatay ve düşey sistem ayrıysa `verticalCoordinateSystemName` yazılmalıdır (örnek "N60").
- **ABD karayolu idarelerinin makine kontrolü şartnameleri:** kontrol noktalarını idare kurar, yüzey ve aks dosyası
  LandXML olabilir ([DelDOT](https://webarchive.library.unt.edu/web/20130215120252/http://www.efl.fhwa.dot.gov/files/technology/3d-modeling/DelDOT-Machine-Control-Spec.pdf)).
  Yerelleştirmede yatay uyum (öteleme, döndürme, ölçek) ile düşey uyum (blok kaydırma, eğik düzlem ya da jeoit) ayrı
  ayrı kurulur ([Trimble Access](https://help.fieldsystems.trimble.com/trimble-access/2023.10/en/Coordinate-system-vertical-adjustment.htm)).
- **Matematik:** hesap yok; dosyanın söylediği, projenin ayarından ve EPSG'nin adlarından gelir.

## Karar

- **LandXML'in `CoordinateSystem`'i:** `file.export.landxml` ve `road.stakeout.send`'in string yolu projenin
  sistemlerini yazar.
  - EPSG sistemi: `name` (EPSG'nin adı, "TUREF / TM30"), `epsgCode`, `horizontalDatum` (kayıttaki datum, "TUREF").
    Projenin kendi tanımladığı sistem `horizontalCoordinateSystemName` ile yazılır; yerel (sistemsiz) projede yatay
    öznitelik yazılmaz.
  - Kotlar: ortometrikse `verticalCoordinateSystemName` (EPSG'nin adı, "Antalya height") ve `verticalDatum`
    ("Antalya"); elipsoit yüksekliğiyse `verticalCoordinateSystemName="ellipsoidal height"`.
  - `desc` kotları sözle söyler: "Kotlar: ortometrik, Antalya height (EPSG:5775), geoit TG-20"; epok varsa o da.
  - Proje hiçbirini bilmiyorsa öğe yazılmaz: dosya eskisi gibidir.
- **Kotların sistemi söylenmeli** (`field_heights_unknown`):
  - `road.stakeout.send` her biçimde: proje kotlarının düşey sistemini söylemiyorsa ya da ortometrik deyip sistemi
    belirtmiyorsa ret. Ret ilk numara ve saat denetiminden sonra gelir. Elipsoit yüksekliği bir sistemdir, gönderilir.
  - `file.export.landxml`'in yeni `machineControl` seçeneğiyle aynı ret. Seçeneksiz LandXML, alışveriş dosyası olarak
    eskisi gibi yazılır.
  - Varsayımla gönderme yolu yok: sahaya giden bir dosya taslak olamaz. Çözüm projenin ayarında bir kerelik bir
    seçimdir.
- **Makine kontrolüne giden yüzey** (`unproven_heights`): yüzeyin buluttan alınan zemini projenin sisteminde
  gösterilmeli (ADR 1029'un kaydı). Sistemi bilinmeyen ya da başka olan bulut zemini olan yüzey gönderilmez.
- **Masaüstü:**
  - Koridor › Aplikasyon'a "Makine kontrolüne ver…" (`landxml.machine`): LandXML ver'in dosyası `machineControl`
    ile yazılır, iletide kotların sistemi söylenir.
  - Aplikasyon cetvelinin alt satırında gönderilecek kotlar ("Kotlar: ortometrik, Antalya height (EPSG:5775), geoit
    TG-20.") ya da söylenmemişse uyarı.
- **Kapsam dışı (ROAD-SEC-10'un kalanı):**
  - Yerelleştirme (saha kalibrasyonu): kontrol noktalarıyla yatay ve düşey uyum ve artıkları yok (ROAD-SURVEY-02).
  - Kalite kabul şartı: yüzeyin kontrol noktalarına göre kabulü ve tasarımın onayı dosyaya bağlanmadı.
  - JobXML, GSI, GTS-7, Nikon RAW ve CSV dosyaları sistemi taşımıyor; yalnız ret kapısından geçiyorlar.

## Ölçüm

- **Bağımsız başvurular** (KentOS kodu yok):
  - `road_landxml_cases.py` ve `road_stakeout_send_cases.py` `CoordinateSystem`'in özniteliklerini PROJ'un
    veritabanından (pyproj: 5254 "TUREF / TM30", 5775 "Antalya height", datum "Antalya") ve CRS kaydından kurar.
  - `--written`: KentOS'un yazdığı `yol.xml` ve `aplikasyon.xml` xmllint ile LandXML 1.2 şemasına uyuyor ve
    öznitelikleri başvuruyla aynı.
- **`file.export.landxml`** 29 durum, 7'si yeni:
  - makine kontrolü geçer;
  - kotsuz proje makine kontrolü olmadan yazılır;
  - iki `field_heights_unknown` reddi;
  - bulut zemini projenin sisteminde geçer;
  - sistemi bilinmeyen ya da başka olan bulut zemini reddedilir.
- **`road.stakeout.send`** 32 durum, 3'ü yeni: iki ret ve elipsoit yüksekliğiyle gönderim. Eski 29 durumun çizimi
  artık kotlarını söylüyor (Antalya height, TG-20); çıktıları değişmedi.
- **Masaüstü:**
  - Makine kontrolüne ver yazar ya da reddeder; reddedilen çizim LandXML ver ile yine yazılır.
  - Aplikasyon cetveli kotsuz projede göndermez.
  - Resimler `.run/gui-evidence/yol-y96/`: şeritte yeni düğme, ret ve yazıldı iletisi (açık, koyu); cetvelin alt
    satırı iki hâliyle.
