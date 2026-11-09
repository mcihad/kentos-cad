# 3100 — Dönüşümün nokta bulutuna uygulanması (Dönüşüm uygula)

Tarih: 2026-11-09. plan/map-cad.md MAP-13, beşinci tur. Durum: taslak. ADR 2700'ün “Sonra”sındaki “dönüşümün buluta
uygulanması”.

İlgili:
- [ADR 0207](0207-point-clouds-and-large-data.md) §7: nokta bulutu işlemleri (sonuç dosyası, ızgara, LAS, LAZ ve COPC);
- [ADR 1800](1800-scan-registration-check.md): tarama kaydı (`cad.pointclouds.register`), dönüşüm tarifi ve matris;
- [ADR 2300](2300-scan-registration-robust-weights.md), [ADR 2700](2700-scan-registration-rejections.md): ağırlıklar ve
  ayıklama.

## Dayanak

- **PDAL, filters.transformation** ([belge](https://pdal.io/en/stable/stages/filters.transformation.html), 9 Kasım
  2026'da okundu):
  - matris 16 sayıdır, boşlukla ayrılır, satır sıralıdır; her xyz üçlüsüne uygulanır;
  - `invert` verilen matrisin tersini uygular;
  - filtre mekânsal başvuru bilgisini uygulamaz ve dikkate almaz.
- **CloudCompare, Apply Transformation** ([belge](https://cloudcompare.org/doc/wiki/index.php/Apply_Transformation), 9
  Kasım 2026'da okundu):
  - dönüklük üst sol 3×3'te, öteleme dördüncü sütunun üst kısmında, alt satır her zaman 0 0 0 1;
  - matris, eksen ve açı ya da Euler açıları ve öteleme ile girilir; ters dönüşüm seçilebilir;
  - dönüşüm seçili nesnelere uygulanır ve her nesnenin dönüşüm geçmişine eklenir.
- **LAS 1.4 R15** (ADR 0207'nin okuması): koordinat, kayıttaki tam sayı × ölçek + ötelemedir; başlığın kutusu noktaların
  kutusudur.
- **KentOS bugün** (ADR 1800–2700):
  - Tarama kaydı dönüşümü bulur, raporlar ve CSV'ye matrisi yazar; bulut taşınmaz;
  - matrisi buluta uygulamak için KentOS'un dışına çıkmak gerekiyordu.

## Karar

1. **Dönüşüm uygula** (`pointcloud.transform`, İşlemler aracı; masaüstünde `processing.run.pointcloud.transform`;
   Python ve MCP'de `cad.processing.run`):
   - girdi bir bulut (sanal bulutun dosyaları birlikte);
   - matris satır satır 16 sayı. Boşluk, satır sonu, virgül ya da noktalı virgülle ayrılır; köşeli ayraç atılır;
     ondalık için nokta;
   - sonuç LAS, LAZ ya da COPC: kaynağın yanına `‹ad›-donusmus` adıyla ya da verilen yere;
   - istenirse yeni katmanda nesne (öntanımlı “Dönüştürülmüş bulut”), tek geri alma adımı. Kaynak dosyaya dokunulmaz.
2. **Hesap** (`kentos_pointcloud::ops::transform`):
   - nokta = tam sayı × ölçek + öteleme. Taşınan koordinat, satırın üç çarpımı ve ötelemesidir, bu sırayla toplanır;
   - sonuç kaynağın ızgarasına (aynı ölçek ve öteleme) yarım sıfırdan uzağa yuvarlanır. Bir koordinat en çok ölçeğin
     yarısı kadar değişir; ileti bunu ölçekle söyler;
   - noktanın öbür alanları (sınıf, renk, dönüşler, GPS zamanı, tarama açısı, ek baytlar) aynen kalır. Ek baytlardaki
     normaller döndürülmez;
   - koordinat sistemi değişmez (PDAL gibi): aynı sistemde taşımadır, sistemler arası dönüşüm değildir.
3. **Retler** (hiçbir şey yazılmaz):
   - 16 sayı değil; sayı olmayan söz; sonlu olmayan sayı;
   - son satır 0 0 0 1 değil: yalnız afin dönüşüm, perspektif yok;
   - doğrusal kısım tekil (determinantı sıfır);
   - bulut ızgaranın 32 bitlik erimini aşıyor. Dosyaların kutularının sekiz köşesi taşınarak yazmadan önce denetlenir:
     afin dönüşüm her noktayı köşelerin taşındığı gövdenin içinde tutar.
4. **Özet ve çıktılar:**
   - taşınan nokta sayısı ve bulutun ortasının (kutunun ortası) kayması;
   - katı matriste dönüklük açısı (eksen çevresinde; atan2 ile, küçük açıda da kesin);
   - katı değilse (RᵀR 10⁻⁹ içinde birim değil ya da determinant pozitif değil) uyarı: uzunluklar ve açılar değişir;
   - çıktılar `count`, `shift`, `angle`.
5. **Masaüstü:**
   - Tarama kaydı'nda **Buluta uygula…**: oturtulan taramayı seçer, pencereyi kapatır ve Dönüşüm uygula'yı kaydın
     matrisiyle ve “Kaydedilmiş bulut” katmanıyla açar. Matrisin sayıları Rust'ın yazdığı en kısa biçimdedir, aynı
     okunur. Dosya, araç çalışınca yazılır;
   - yer: Nokta bulutu bağlam sekmesi › İşlemler, Bulutları birleştir'in altında;
   - simge `cloudTransform`: kesikli kaynak bulut, ok, dönmüş ve taşınmış bulut. Seçenek sayfası
     `.run/gui-evidence/donusum-uygula-3100/secenekler.png`, A seçildi.
6. **Ters matris yapılmadı** (PDAL'ın `invert`'ü, CloudCompare'in tersi): bağımsız başvurunun aynı sayıları üretmesi için
   tersin hesap sırası sabitlenmeli.
7. **Kapsam işareti:** ADR 1800'deki gibi Harita + CBS (İşlemler araç kutusu her ikisinde).

## Kanıt

- **Çekirdek** (`ops/transform.rs`, 6 birim testi): yazımın okunması, retlerin iletileri, katılık ve açı, kayma, ızgara
  dışı kutu, kayıtların yarım sıfırdan uzağa taşınması (çeyrek dönüş).
- **Ortak durumlar** (`scripts/fixtures/pointcloud_processing_cases.py`, KentOS kodu yok; laspy ve numpy): üreteç
  noktaları matrisle kendisi taşır, yarım sıfırdan uzağa yuvarlar ve tam sayıların FNV-1a'sını yazar.
  - Köyün LiDAR'ı (71.212 nokta): ortası çevresinde 0,002 rad dönüp ötelenen matris (LAZ, çizime); Tarama kaydı'nın
    tarifi Rz·Ry·Rx (çizime değil); yalnız öteleme (LAS, tam sayılar aynen kayar; köşeli ve virgüllü yazım); COPC;
    1,0001 ölçek (uyarı, açı yok);
  - X ve Y'si 0,4 mm'lik ızgaradaki ikinci bulut: kendi ızgarasında kalır, yuvarlama eksen eksen söylenir;
  - beş ret: 12 sayı, sayı olmayan söz, perspektif satırı, tekil matris, 3000 km öteleme;
  - 41 durum (30'dan). Rust'ın koşucusu noktaların tam sayılarını bayt bayt, çıktıları ve notları birebir karşılaştırır;
    geçer.
- **Python** (`python/tests/test_headless_clouds.py`, `TransformTests`): on bir durum başsız sunucuda; özet, çıktılar,
  başlığın sayısı ve kutusu, LAS'ta noktaların FNV-1a'sı.
- **MCP** (`crates/native/mcp/tests/cloud_transform.rs`):
  - ızgara dışına taşıyan matris reddedilir, dosya yazılmaz;
  - döndürülmüş matrisle özet, çıktılar, notlar, adım, katman ve nesnenin kutusu durumunki.
- **Masaüstü** (`register_window_tests.rs`, `the_fit_is_applied_to_the_moving_scan`):
  - Buluta uygula, Dönüşüm uygula'yı kaydın matrisiyle açar; oturtulan tarama seçilidir;
  - çalışınca 4.320 nokta taşınır, kaynak XYZ dosyası değişmez, yeni bulut “Kaydedilmiş bulut” katmanındadır;
  - sabit taramayla yeni bulut yeniden oturtulunca başlangıç RMS'i ilk kaydın bitişindekidir (14,78 ve 14,70 mm; fark
    ızgara ve vokseldir). Kot ötelemesi 0,2 mm; kontrol noktalarında KOH_Z ilk kaydınkiyle 1 mm içinde aynı;
  - ikinci ICP planda zayıf yönde (geometri gücü 0,13) κ 3,4·10⁻⁴ rad ve 3 mm kayar ama RMS'i yalnız 0,03 mm düşürür.
    Bu, kaydın kendi belirsizliğidir, uygulamanın hatası değil; test bu yüzden RMS'i, kotu ve KOH_Z'yi denetler.

  Tarama kaydı'nın testleri ve çapraz denetimler geçer. Resimler `.run/gui-evidence/donusum-uygula-3100/`.
- **Manifest:** araç `processing.*` kuralıyla `processing` düzeyinde (`cad.processing.run`); 1030 satır.

## Sonra

- Ters matris (`invert`; yapıldı, [ADR 3500](3500-point-cloud-inverse-and-history.md)) ve eksen-açı ya da Euler girişi.
- Dönüşümün yeni dosya yazmadan nesnede tutulması ve nesnenin dönüşüm geçmişi (CloudCompare'in geçmişi gibi; geçmiş
  yapıldı, [ADR 3500](3500-point-cloud-inverse-and-history.md)).
- Ek baytlardaki normallerin döndürülmesi; tarama açısının düzeltilmesi.
- Çoklu tarama demeti; gerçek veriyle süre, bellek ve yoğunluk ölçümü (MAP-13'ün öbür maddeleri).
- Web (TODOS §0.2).
