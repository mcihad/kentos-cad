# ADR 0818: Kaynak birim kararı ve birim düzeltmesinde canlı ilişkiler

- **Durum:** uygulandı, 1 Kasım 2026; CAD-ADV-02 kapanış dilimi.
- **Önceki karar:** [ADR 0817](0817-unit-correction-keeps-coordinate-frames.md).
- **Başlangıç:** main `e79825bc8`.
- **Kapsam:** yerel CAD model birimi, dış dosya açma, ilişkili nesneler,
  Python/MCP eşliği, ayrıntı yakınlaştırması ve açık ayırıcılı ondalık giriş.

## Araştırma ve korunan fiziksel model

[AutoCAD blok birimi açıklaması](https://help.autodesk.com/cloudhelp/2025/ENU/AutoCAD-Core/files/GUID-6C46049D-8636-442D-8BAC-CF4FD515FDC0.htm)
blok/dosya birim dönüşümü ile kullanıcının ek yerleştirme çarpanını ayırır.
[INSUNITS](https://help.autodesk.com/cloudhelp/2025/ENU/AutoCAD-Core/files/GUID-A58A87BB-482B-4042-A00A-EEF55A2B4FD8.htm)
bildirilmemiş birimi ayrı bir değer olarak tutar. KentOS bu ayrımı kendi
metre tabanlı sözleşmesinde uygular; AutoCAD'in kullanıcı profilindeki bütün
örtük varsayılanlarını kopyalamaz. Model/kâğıt ayrımının diğer dayanağı
[QCAD baskı kılavuzudur](https://qcad.org/en/tutorials/printing/printing_and_pdf_export/).

Kaynak geometri ve Python/komut koordinatları metredir. Çizim birimi giriş
ve gösterimi belirler; pafta oranı ve zoom geometriye ikinci kez uygulanmaz.
Mevcut DXF/DWG blok normalleştirmesi, pano ve fiziksel blok yerleştirme
yolları korundu; bunların doğru çalıştığı karma birim matrisiyle sınandı.
Bir odanın tahmini büyüklüğünden kaynak birimi çıkarılmaz.

## Bulunan sorunlar ve çözüm

### 1. Birimsiz dosya açarken açık karar

`apps/desktop/src/opening.rs` daha önce birimsiz dış çizimi yeni projenin
varsayılanı üzerinden hemen yerleştiriyordu. Artık dış kaynak okunup
`UnitDecision.assumed` dönerse sonuç bekletilir. Açık çizim değişmeden
“Çizimin birimini doğrula” penceresi gösterilir.

- Başlangıçta birim seçilmemiştir; “Bu birimle aç” pasiftir.
- Seçim yaklaşık model genişliği/yüksekliğini metreyle, dönüşüm çarpanını,
  nesne/blok sayısını gösterir. Boyut hesabı ana görünüşten değil gerçek
  geometri kapsamından gelir ve UI iş parçacığında hesaplanmaz.
- Onay, kaynağı açık birimle işçi üzerinde yeniden okur. Bekleme sırasında
  açık belgenin oturumu/sürümü değişmişse eski karar uygulanmaz.
- Vazgeç/Esc açık çizimi korur. Birimi bilinmeyen kaynak için mevcut
  bilinen uzunlukla içe aktarma yolu açıklanır.
- Bildirilen birimi açıkça geçersiz kılma içe aktarma önizlemesinde
  görülür; kaynak dosya değişmez, çarpan yalnız bir kez uygulanır.

Python `Document.open(path, file_unit="cm")` ve MCP `drawing.open` içindeki
`fileUnit` aynı `Session::open_in` yoluna gider. Kararsız dosya açık birim
verilmedikçe `source_unit_required` döndürür. Metre modelini zaten saklayan
KCAD'a kaynak birimi geçersiz kılma verilirse `invalid_input` döner.
Python docstring/stub ve MCP şeması güncellendi; komut/şema sürümü değişmedi.

### 2. Çizilen sonucu değil, düzenlenebilir ilişkileri dönüştürme

`cad.units.change` yalnız nesneleri `update_many` ile taşıyordu. Bu,
ilişkili dizinin üyelerini elle taşınmış sayıyor, ölçü tarifinin yönettiği
ölçülere dokunmayı reddedebiliyordu. Dinamik blok kuralları ve bazı ikincil
anotasyon konumları eski birimde kalıyordu.

Domain `scale_unit_objects`, nesneler ile onları yöneten dizi/ölçü tarifini
aynı işlem grubunda dönüştürür. Orijin merkezli kutupsal dizi gibi sayıları
değişmeyen tarifler de gruba alınır; üyeleri elle tutulmuş sayılmaz.
Ölçü tarifleri yalnız bu dönüşümün oluşturduğu tam nesne yazımlarına izin
verir; başka komutların korunan ölçüleri değiştirme yetkisi genişlemez.
Üye kimlikleri, kenar referansları, saklanan/silinen dizi üyeleri korunur.
Sonraki duvar düzenlemesi aynı ölçü üyelerini yeniden üretebilir.

Dinamik tanım noktaları, uzunluk sınırları/adımları/listeleri, tablo
değerleri ve yerleştirme parametreleri birlikte dönüştürülür. İfade
`s * f(uzunluk/s, açı)` biçiminde taşınır; sabitler ve açı birimleri
anlamlarını korur. Gerçek ifade lexer'ı kullanılır; Unicode parametre adları
ve tırnak içindeki metin basit metin değiştirmeyle bozulmaz.

Model uzunluğu olan sütun yüksekliği, ilave lider kolları, taşınmış ölçü
yazısı, yuvarlama/tolerans ve anotatif konum geçersiz kılmaları da dönüşür.
Anotasyon ölçek paydaları, açılar, boyutsuz oranlar ve kâğıt boşlukları
aynı kalır. İç içe blokların kırpma/öznitelik konumları, hedef tanımların
da ölçeklendiği bu yola özgü yardımcıyla dönüştürülür. Tek blok tanımını
yeniden boyutlandırma komutunun hedef tanımları değiştirmeyen davranışı
genişletilmez.

Pafta kitabının kâğıt boyutları, çerçeveleri ve oranları mevcut sözleşme
gereği korunur. Modelin fiziksel boyunu değiştiren düzeltme `sheets_kept`
uyarısını plan/uygulama sonucunda verir: pafta model görünüşleri yeniden
ayarlanmalıdır. Nokta bulutlarının harici noktaları da mevcut açık uyarıyla
korunur. Bunlar otomatik ölçeklenmiş gibi raporlanmaz.

### 3. Ayrıntı ve yerel sayı girişi

Native ve web kamera üst sınırı 5.000'den 10.000 mantıksal piksel/metreye
çıktı. 2 mm ayrıntı 20 mantıksal piksel, bu Mac'in HiDPI ekranında 40 fiziksel
piksel kaplar. Kaynak f64 koordinatları, hi/lo GPU yerleştirmesi ve yerel
orijin mekanizması değişmedi. Denenen 50.000 sınırı uzun çizgi testini
geçmediği için kullanılmadı. 20 km çizgi için mevcut **0,05 px** eşik ve
gerçek GPU pan kontrolünün **0,02 px** hata eşiği gevşetilmedi.

Noktalı virgülle ayrılan koordinatlarda ve `<` içeren kutupsal girişte
ondalık virgül açıktır: `-1,25m;8m`, `@1,25m<0`. Normal `10,20` hâlâ
iki koordinattır. Native giriş gerçek araç oturumuyla, web'deki aynı ek
davranış kendi testiyle doğrulandı.

## Kabul kanıtı

Ekran görüntüleri ve test logları Git dışında
`/tmp/kentos-cad-advanced-02-2026-11-01/part-2/` altında tutuldu. Bunlar
gerçek macOS uygulamasında fare/klavye ile alındı; üretilmiş UI maketi değil.
İlk dilimdeki UCS/birim onayı görüntüleri bir üst dizindedir.

| Davranış | Gerçek GUI tekrarı | Otomatik kabul |
| --- | --- | --- |
| Birimsiz dosya | 01–03: seçimsiz onay pasif; cm/mm farklı fiziksel önizleme | Masaüstü açma testi: seçim, iptal, kaynak cm, açık çizim korunması; Python/MCP eşliği |
| mm dosyası → m çizimi | 14–17: 6×4 m kapsam; cm geçersiz kılmada 60×40 m; aktarılan #4 nesne 12 m² | Gerçek DXF dışa aktar/aç/içe aktar: 3 kaynak × 3 hedef birim |
| inç → mm pano/blok | 07–11: inç kaynak 4×3 m; eşit iki oda; 1000 mm blok, çarpan 1; tek Undo | 3×3 pano matrisi, 1 m eleman, 12 m², uzak koordinat, kaydet/aç |
| UCS/kamera/uzak koordinat | 05 ve son derleme 20: (500000,4400000), 30° kamera, 12 m²; 12: virgüllü negatif giriş | UCS regresyonları ve gerçek araç oturumu |
| 2 mm ayrıntı | 13 ve 21: son zoom sınırında seçilebilir çizgi | 8 sayısal test ve gerçek Apple M5/Metal render/pan testi |
| Undo ve kalıcılık | 18: import tek Undo ile 3 nesne/1 blok; 19: Redo+kaydet/aç ile 6 nesne/2 blok, 12 m² | Bayt düzeyinde Undo/Redo ve KCAD yeniden açma |
| Dizi/dinamik blok/ölçü tarifi | Birim onayı ilk dilimde gerçek GUI'de | Sonraki düzenleme, üye kimliği, Unicode ifade, kurallar, tablo, açı; çok kenarlı duvar |
| Tarama/anotasyon/pafta | Genel görünüş ve ölçü yazısı GUI'de | Desen ölçeği/aralığı, anotatif konum, lider kolu, tolerans; pafta kitabı aynı, uyarı açık |

`python/tests/test_unit_integrity.py` dokuz kabul testi içerir; matris
alt durumları ayrı test sayısına şişirilmez. Birim (10), UCS (3) ve dış
dosya açma (2) regresyonları da geçti. MCP bütün testi 33/33; native
uygulama 270, masaüstü 1561 test geçti (37/256 isteğe bağlı test atlandı).
Domain ilişki/belge/lib kontrolleri 87+22+18; yerel birimli giriş 4;
sayısal hassasiyet 8; **KENTOS_GPU_TESTS=1** ile gerçek Metal kontrolü 1.
Son kaynakta derleme, Clippy ve Python SDK üretim kontrolü geçti.

**Mevcut web sınırı:** kamera/gezinme testleri 41/41 ve yeni ondalık
virgül testi geçti. Web'in eski TypeScript koordinat okuyucusunun ortak
vakalarda zaten 32 başarısız testi vardı (özellikle UCS, birimli ifadeler
ve ayak/inç). HEAD okuyucu/testiyle ayrıca tekrarlandı: 32 başarısız,
112 başarılı; bu değişiklikten sonra aynı 32 başarısızlık kaldı. Bunlar
masaüstü kabulüne sayılmadı ve bütün web giriş eşliği tamamlanmış olarak
raporlanmadı. Web okuyucusunun çekirdekle eşlenmesi ayrı iş olarak izlenir.
