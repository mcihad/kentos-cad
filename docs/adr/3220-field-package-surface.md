# 3220 — Saha paketinin yüzeyi ve yüzey farkının paketin yüzeyiyle olması

Tarih: 2026-11-09. plan/map-cad.md MAP-19, beşinci tur. Durum: taslak. ADR 2820'nin “Sonra”sındaki “paketin yüzeyinin
kendisi (bugün yalnız kaynak özeti): yüzey farkının da paketin yüzeyiyle olması”.

İlgili:
- [ADR 1700](1700-asbuilt-check.md): Uygulama kontrolü (`cad.asbuilt.check`);
- [ADR 1920](1920-field-package.md): Saha paketi (`cad.field.package`);
- [ADR 2420](2420-field-record-checked-with-its-package.md): kaydın saha paketiyle denetlenmesi;
- [ADR 2820](2820-record-checked-with-its-package-design.md): kaydın paketin kendi tasarımıyla denetlenmesi.

## Dayanak

- **Trimble Access, DTM'ler ve yüzeyler**
  ([Map files: surfaces](https://help.fieldsystems.trimble.com/trimble-access/2024.00/en/Map-files-surfaces.htm),
  [Digital terrain models](https://help.fieldsystems.trimble.com/trimble-access/2023.00/en/Map-files-DTMs.htm),
  9 Kasım 2026'da okundu):
  - arazi cihazı yüzeyi dosyadan okur: ızgaralı DTM (`.dtm`), üçgenli TTM (`.ttm`), DXF 3B yüzleri ve LandXML
    dosyasındaki üçgenli DTM (`.xml`);
  - aplikasyonda kazı/dolgu, konumun üstünde durduğu üçgenden hesaplanır;
  - “Measure to surface” ölçülen noktanın yüzeye en yakın uzaklığını saklar.
- **Trimble Access, veri klasörleri**
  ([Data folders and files](https://help.fieldsystems.trimble.com/trimble-access/latest/en/data-folders.htm)):
  - `.ttm`, `.dtm`, `.dxf` ve `.xml` dosyaları projenin klasöründedir;
  - bu dosyalar projedeki her iş tarafından kullanılabilir.

  Ekip sahada çizimin yüzeyini değil, kendisine verilen yüzey dosyasını kullanır.
- **Leica Captivate**
  ([Stake DTM ve Stake pts & DTM uygulamaları](https://leica-geosystems.com/products/total-stations/software/leica-captivate/leica-captivate-apps-your-digital-toolbox/leica-captivate-staking-surfaces-objects-apps)):
  - yüzey, cihaza ayrı bir DTM işi olarak gider;
  - tasarım noktaları yükseklikleri o DTM'den alınarak aplikasyon edilir.
- **LandXML 1.2:** `Surface/Definition surfType="TIN"`, `Pnts/P` (kuzey doğu kot) ve `Faces/F` (üç noktanın kimliği).
  KentOS'un okuyucusu ve yazıcısı vardır (`kentos_formats::landxml`, `landxml_write`; Yol'un LandXML dışa aktarımı).
- **KentOS bugün:**
  - ADR 1920'nin paketi yüzeyin yalnız kimliğini, adını ve kaynaklarının özetini taşıyordu;
  - ADR 2820'nin kontrolü paketin noktalarını ve çizgilerini paketten okuyordu, yüzey farkını ise çizimin bugünkü
    yüzeyiyle hesaplıyordu;
  - paket yazıldıktan sonra yüzey değiştiyse kayıt sahaya verilmeyen bir yüzeyle denetleniyordu.

## Karar

1. **Paket yüzeyin kendisini taşır.** `cad.field.package` yüzey verilince pakete `yuzey.xml` yazar:
   - yüzeyin bugün kurulduğu hali (köşeler ve tutulan üçgenler), LandXML 1.2 TIN;
   - Yol'un LandXML yazıcısıyla, her sayı geri okununca aynı f64'ü veren en kısa yazımla (yuvarlama yok);
   - koordinat sistemi projeninki; tarih ve saat paketin damgası.
2. **Paketin bildirgesi** (`saha-paketi.json`, cevabın `surface`'i; sözleşmeye ek alanlar, komut sürümü değişmez):
   - yüzeyin dosyası (`file: "yuzey.xml"`), köşe ve üçgen sayısı, dosyanın SHA-256'sı;
   - `files` listesinde `yuzey.xml` dördüncüdür.

   Yüzey verilmeyen paket eskisi gibi üç dosyadır.
3. **Kontrol paketin yüzeyiyle** (`cad.asbuilt.check`, `packageDesign`):
   - paket yüzeyini taşıyorsa yüzey farkı paketin yüzeyiyledir; `surface` verilmese de;
   - dosya bildirgedeki SHA-256'ya geri okunur;
   - yükseklik, konumu tutan üçgenin düzleminden alınır. Yüzeyin dışı ve boşluğu `offSurface`'tir;
   - cevabın `package.surface`'i `true` olur.
4. **Yüzeyin değişmesi** (`surfaceChanged`, ek alan): paket bir yüzey adlıyorsa ve `surface` verildiyse söylenir. Verilen
   yüzey paketinkinden başkaysa ya da kaynaklarının özeti paketinkinden farklıysa değişmiştir:
   - paketin tasarımıyla denetlemede çizimin yüzeyi kurulmaz, yalnız kaynak özeti okunur;
   - paketin tasarımı istenmezse eskisi gibi çizimin yüzeyiyle denetlenir, değişiklik yine söylenir. ADR 1920'den beri
     yazılan paketler de kaynak özetini taşıdığı için onlarda da söylenir.
5. **Ret** (`invalid_asbuilt_check`, `package`'ta):
   - “… paketinin yüzeyi (yuzey.xml) özetiyle tutmuyor; paket bozulmuş ya da elle değiştirilmiş. Paketi Saha paketi
     yaz… ile yeniden yazın.”;
   - dosya yoksa ya da okunamıyorsa “… paketinin yüzeyi (yuzey.xml) okunamadı: …”.
6. **Eski paketler:** `yuzey.xml` taşımayan paket ADR 2820'deki gibi denetlenir (yüzey farkı çizimin yüzeyiyle).
7. **Masaüstü** (Uygulama kontrolü):
   - paketin satırı “tasarım paketin noktaları, çizgileri ve yüzeyi” der;
   - paketin yüzeyiyle denetlenip çizimin yüzeyi değiştiyse uyarı: “Çizimdeki yüzey paketten sonra değişmiş; yüzey
     farkları sahaya verilen yüzeyle, paketin yüzeyiyle (yuzey.xml).”;
   - paketin tasarımı istenmeden değiştiyse hata satırı: “Yüzey, saha paketinin yazıldığı yüzeyden değişmiş.”;
   - Saha paketi yaz… iletisi yüzeyin dosyasını ve üçgen sayısını söyler.
8. **Kapsam işareti:** ADR 1920 ve 2820'deki gibi **CAD + CBS** (Harita, Yol ve CBS projeleri). Arazi yüzeyi Harita ve
   Yol projelerindedir.

## Kanıt

- **Bağımsız başvurular** (KentOS kodu yok):
  - `scripts/fixtures/asbuilt_cases.py` → `fixtures/commands/v1/cad.asbuilt.check.json` (48 durum, 6'sı yeni):
    - paketin yüzeyi PG, çizimin G düzleminin 5 cm altında, yalnız güney yarısı ([0, 100] × [0, 50]);
    - dört köşe ve iki yüz olarak Python'da yazılmış LandXML;
    - P1'in yüzey farkı 100,31 − PG = +0,0598 m (çizimin yüzeyiyle +0,0098) ve 3 cm sınırında dışarıda;
    - kuzeydeki X9 ve B2 yüzeyin dışında;
    - `surface` verilince `surfaceChanged`;
    - Saha paketi yaz… ile ilk adımda yazılan paketin yüzeyi çizimin yüzeyidir: sonuç katmanlarla denetlemeninkiyle aynı,
      değişiklik söylenmez;
    - yüksekliği 1 cm değiştirilmiş `yuzey.xml` ve bildirgenin adladığı ama pakette olmayan `yuzey.xml` reddedilir.
  - `scripts/fixtures/field_package_cases.py` (29 durum): yüzeyin 121 köşesi ve 2n − b − 2 = 200 üçgeni, dosyası
    `yuzey.xml`, SHA-256'sı komutunki.
- **Rust ve Python ortak durumları** iki komutta da geçer.
- **Bağımsız okuma** (Python'un `zipfile`, `xml.etree`, `hashlib`'i):
  - `python/tests/test_field_package.py` (5 test): `yuzey.xml`'in noktaları çizimin 121 noktasıdır, tam olarak;
    200 yüzün her birinin plan alanı 50 m², toplamı 100 m × 100 m; SHA-256 ve sayılar bildirgeninki;
  - `python/tests/test_asbuilt_package_surface.py` (2 test): KentOS'un yazdığı paketin `yuzey.xml`'i Python'da 5 cm
    alçaltılıp SHA-256'sı yeniden yazılınca her noktanın yüzey farkı 5 cm büyür (kontrol dosyayı okur, çizimi değil);
    SHA-256'sı eski kalırsa ret.
- **MCP** (`crates/native/mcp/tests/asbuilt_package_surface.rs`): `.kcad` olarak açılan çizimin paketi `yuzey.xml`'i
  121 köşe ve 200 üçgenle taşır; paketin tasarımıyla kontrol `package.surface` der, değişiklik söylemez. Yüzeyli paket
  ortak durumun işaretlerini ve farklarını verir; bozuk paket reddedilir.
- **Masaüstü** (`asbuilt_window_tests`):
  - `the_record_is_checked_with_its_package_s_own_surface`: ortak durumun cevabı, uyarı ve paketin satırı;
  - Saha paketi yaz… testinde paket dört dosyadır, ileti “yüzey Tasarım (yuzey.xml, 200 üçgen)” der;
  - resimler `.run/gui-evidence/paketin-yuzeyi-3220/` (açık geniş, koyu dar).

  Çapraz denetimler geçer.

## Sonra

- Paketin yüzeyini TTM ya da DXF 3B yüz olarak da yazmak (cihazın kendi biçimleri; MAP-19'un “cihaz biçimleri”).
- ~~Yüzeye en yakın uzaklık (Trimble Access'in “Measure to surface”i); bugün düşey fark.~~
  [ADR 4020](4020-asbuilt-nearest-distance-to-surface.md).
- ~~İş türleri (as-built, aplikasyon, deformasyon)~~ ADR 3620; zaman serisi; yetkili inceleme kaydı.
- Web (TODOS §0.2).
