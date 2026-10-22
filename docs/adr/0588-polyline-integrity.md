# 0588 — Çoklu çizgi aktarımında bütünlük ve açık kayıp kabulü

22 Ekim 2026. [ADR 0587](0587-cad-io-layout-xref-and-loss-contract.md) devamı; CAD-IO-15 için **kısmi uygulama**. Madde bütünüyle kapatılmış değildir. Geometriyi aktaramamayı açıkça söylemek, o geometriyi yerel ve düzenlenebilir biçimde desteklemekle aynı kabul edilmez.

## Düzeltilen gerçek kayıplar

- Optimize JSON okuyucusu INSERT `yScale` alanını atıyordu. Alan artık tipli sözleşmeyle aynı okunur. Ayna/dönüş ve birbirine çok yakın fakat farklı X/Y ölçekleri ayrı kalır; farkı yok eden 1e-9 eşiği kaldırıldı.
- LWPOLYLINE'da açık 40/41 sıfırları, 43 sabit genişliğine yanlışlıkla geri düşüyordu. Köşe genişlik alanları varsa sabit genişlik kullanılmaz. Bütün genişlikler sıfırsa gereksiz bir genişlik veya kayıp üretilmez.
- Aynı X/Y'deki son köşenin kotu ilkinden farklıysa bu gerçek bir düşey kenardır. Köşe artık kapanış tekrarı sayılıp silinmez. Aynı XYZ'nin tekrarı sadeleştirilebilir.
- DWG codec'inin DXF yazıcısı sabit 43 genişliğinin yanına her köşede sıfır 40/41 yazıyordu. DXF'te bunlar sabit genişliği geçersiz kılar. Adaptör, **çözülen geçici DWG belgesinde** sabit genişliği köşelere açar; kaynak baytları ve upstream Git kodu değiştirilmez. Blok ve paper-space nesneleri de bu geçiştedir. Sabit ve daralan/genişleyen çizgiler yedi hedef binary sürümde sayısal olarak sınanır.
- Host'tan gelen rapor ve blok/pafta raporları birleştirilirken önem ve `details` kaybolmaz. Optimize yazıcı girdisi `notes`, `uids`, `wallDetail` alanlarını da korur.

## Varsayılan davranış

`ImportDxf` (DWG ile ortak), `ExportDxf`, `ExportDwg`, `DxfReadOptions` ve `DxfWriteInput` yeni isteğe bağlı `allowPolylineLoss` alanını taşır. Yok veya false ise:

1. Eğik OCS veya eşit olmayan ölçek altında yayı örneklemek/genişliği atmak,
2. Geniş kapalı çizgiyi genişliği olmayan alana çevirmek,
3. Farklı köşe kotlarıyla genişliği/yayı birlikte standart tek çoklu çizgiye yazamamak,
4. Pafta kitabının çizgi öğesinde çoklu çizgi genişliği, yay veya sıfır dışı kotu kaybetmek

`polyline_geometry_loss` ile reddedilir. Okuma dönüşümü tamamlanmadan belgeye hiçbir şey uygulanmaz; dışa aktarımda geçici/hedef dosya yayımlanmaz. Mevcut hedef korunur. Ret, kayıp özelliğini ve mevcutsa gerçek kaynak handle/satırını söyler; dışa aktarma örnekleri yerel nesnenin UID'sini taşır.

Bu kural **yalnız çoklu çizgi dönüşüm bütünlüğü içindir**. Diğer desteklenmeyen entity türlerine ilişkin mevcut açık kayıp raporunu evrensel bir kayıpsız dosya garantisine dönüştürmez.

`allowPolylineLoss=true` yalnız açıkça kabul edilmiş yaklaşık yolu açar; başarı raporu `polyline.loss.*` özelliklerini, adetleri ve sınırlandırılmış örnekleri taşır. Önce plan ile incelemek, sonra aynı seçeneklerle çalıştırmak mümkündür. İçe alma ayrıca `import_losses` uyarısı verir. Python SDK ve AI kataloğu aynı alan/kodu kullanır; ayrı bir tolerans veya sessiz fallback yoktur.

Kaynak INSERT patlatılmazsa farklı X/Y ölçekleri korunur. Patlatmanın geometriyi değiştireceği yerde kullanıcı bu seçeneği kapatabilir. Çok küçük bilinen ölçek farkları veya eğik normal, bir matris benzerliği epsilon'uyla yok sayılmaz; kaynak dönüşümünün anlamı ayrıca izlenir. İç içe dönüşümler için bu koruma muhafazakârdır: birbirini iptal eden nonuniform dönüşümler de açık kabul gerektirebilir.

Masaüstü önizlemesi kayıp raporunu gösterir; varsayılan İçe aktar kapalıdır. Kabul kutusu yeni dosyada veya birim/kapalı çizgi/patlatma seçeneği değişip rapor yenilendiğinde yeniden kapanır. Pafta tanıları okuma iş parçacığında bir kez hesaplanır; her çizim karesinde eğri örneklenmez. Küçük/ilerlemeli içe alma, headless, Python, MCP ve ortak WASM yazıcısı denetimi atlamaz. Düşük seviyeli `dxf::write` rapor döndüren uyumluluk yordamı olarak kalır; ürün yolları `write_checked` kullanır.

## CAD-IO-15'in açık kalan kabulü

Mevcut PathEntity dünya XY köşeleri, dairesel bulge ve skaler genişlik tutar; genel bir kaynak düzlemi/afin çerçeve tutmaz. Eğik bir daire yayı izdüşümde eliptik olabilir; genişlik de genel afin dönüşüm altında tek dünya genişliğiyle temsil edilemez. Farklı kotlu bulge yolu ise standart tek 3B POLYLINE'a kayıpsız yazılamaz. Bu durumları reddetmek veriyi korur, fakat düzenlenebilir desteği tamamlamaz.

Tam kapanış için gerekenler:

- Tipli, birim bilgisi açık bir kaynak düzlemi: 3B orijin ve iki bağımsız baz vektörü; yerel köşeler, bulge ve kenar başı/sonu genişlikler ayrı tutulmalı. Baz genel afin dönüşümü taşıyabilmeli; normal tek başına yeterli değildir.
- Kalıcı KCAD, undo/redo, kopyalama, blok tanımları, değişiklik özeti ve Python/AI geometri sözleşmeleri bu tek otoriteyi korumalı. Ham JSON'u kullanıcı özniteliklerine gizlemek veya eski kaynak geometrisini düzenlenen geometriye tekrar uygulamak çözüm değildir.
- Çizim, seçim, kenet, kapsam, uzunluk/alan, düzenleme ve dönüştürme kaynak çerçevesini kullanmalı. Render örneklemesi analitik kaynağın yerine geçmemeli; kenar görünüşlü düzlemde ters XY eşlemesinin belirsizliği açık davranış almalı.
- Normalle ifade edilebilen düzlem standart OCS'ye doğrudan yazılmalı. Genel afin yol için standart blok/INSERT bileşimi veya açıkça ilan edilen başka analitik temsil gerekir; özelleştirilmiş KentOS XDATA tek dışa aktarım kanıtı olamaz.
- Bağımsız 3B/afin referans; eğik/ters/kenar görünüşlü düzlemler, nested ve aynalı nonuniform bloklar, daralan genişlik, büyük koordinat/küçük detay ve düzenleme sonrası tekrar yazmayı sınamalı. Aynı writer/reader eşitliği yeterli değildir.

Bu kapsam yapılmadan plan satırı işaretlenmez. CAD-IO-20 kullanıcının önceki açık kararıyla atlanmıştır; gerçek AutoCAD görsel/düzenleme testi yapılmış sayılmaz.

## Doğrulama kaynakları

- `scripts/fixtures/cad_polyline_policy_cases.py`: KentOS'tan bağımsız 20 DXF grup akışı ve plan/execute için 40 ortak ret durumu. Normalin çok küçük eğimi, mikroskobik ölçek farkı, negatif ölçek ve yay/genişlik ayrı örneklerdir.
- `crates/shared/formats/tests/all/cad_io.rs`: varsayılan ret/açık kabul, gerçek handle, sıfır genişlik önceliği, JSON/INSERT ölçekleri ve host rapor özelliğinin kaybolmaması; standart planar round-trip testinden XDATA çıkarılır.
- `crates/native/headless/tests/headless/dwg.rs`: gerçek DXF/DWG içe alma ve plan; başarısız dışa aktarmada eski hedefin korunması; kabulden sonra ayrıntılı rapor ve değişmez belge.
- `scripts/fixtures/cad_polyline_reference.py`: ezdxf 1.4.5 ile 8 bağımsız kontrol; iki polyline türü × açık/kapalı × ±Z extrusion. KentOS XDATA kaldırılarak yalnız standart genişlik/bulge/kapanış/kot alanları okunur. ezdxf yalnız test ortamındadır, uygulama bağımlılığı değildir.
- `cad-io-polyline-integrity` masaüstü izi: kayıp önizleme, kabul yokken değişmez çizim, açık kabul ve tek undo. İki tema ve 1440×900/1100×650 ekranları `/tmp/kentos-polyline-ui` altında kontrol edilir.
- `python/tests/test_dwg.py` ve MCP `an_agent_must_explicitly_accept_a_reported_polyline_loss`: aynı ürün kuralı ve rapor.

Standart başvurular: [Autodesk LWPOLYLINE](https://help.autodesk.com/cloudhelp/2024/ENU/AutoCAD-DXF/files/GUID-748FC305-F3F2-4F74-825A-61F04D757A50.htm), [POLYLINE](https://help.autodesk.com/cloudhelp/2023/ENU/AutoCAD-DXF/files/GUID-ABF6B778-BE20-4B49-9B58-A94E64CEFFF3.htm), [VERTEX](https://help.autodesk.com/cloudhelp/2024/ENU/AutoCAD-DXF/files/GUID-0741E831-599E-4CBF-91E1-8ADBCFD6556D.htm). Upstream köprü davranışı kilitlenen `ff75fdd` revizyonunun `src/io/dxf/writer/section_writer.rs::write_lwpolyline` yordamında görülebilir.

## Bu devam çalışmasında geçen kontroller

- Formats: 184 birim, 108 okuma/analiz, 50 DXF yazma ve 6 diğer format testi; 7 mevcut performans testi çalıştırılmadı. DWG adaptörünün 5 testi geçti.
- Native application: 40 birim ve 125 entegrasyon testi; ortak korpusta 8.085 durum. 22 mevcut performans testi çalıştırılmadı. Headless dosya/komut testleri: 38 geçti.
- Masaüstü exchange: 104 geçti; 25 mevcut test ignored. Kapatılmış DXF penceresinin geç byte/hata yanıtı yeni pencereye uygulanmaz; kayıp kabulü dosya/seçenek değişince sıfırlanır.
- Python: yeniden derlenen paketle 208 test geçti. MCP’nin açık kabul/varsayılan ret testi geçti.
- Komut/sözleşme paketlerinin tam testleri, üretilen katalog/TS, SDK/manifest ve fixture `--check` kontrolleri geçti. Mimari bağımlılık yönü 47 crate/61 hedefte temiz; ortak/WASM derlemesi ve değişen Rust paketlerinde Clippy geçti.
- Bağımsız ezdxf 1.4.5: 8 kontrol geçti. Masaüstü izinin dört boyut/tema birleşimi geçti; 12 görüntü `/tmp/kentos-polyline-ui-final` altında. 18 px kontrolü ayrıca `/tmp/kentos-polyline-ui-18px.png` altında üretildi.

Bu sonuçlar açık kaynak düzlemi/afin model kabulünü veya atlanan AutoCAD kontrolünü tamamlanmış yapmaz.
