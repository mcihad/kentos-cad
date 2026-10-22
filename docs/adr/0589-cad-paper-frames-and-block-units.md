# ADR 0589: CAD kâğıt çizgilerinin çerçevesi ve blokların gerçek birimi

- Tarih: 22 Ekim 2026
- Durum: Uygulandı; CAD-SHEET-08'in dar bir düzeltmesi ve CAD-BLOCK-04 regresyonu. CAD-SHEET bütünü tamamlanmadı.
- Bağlantılar: ADR 0251, 0257 §2, 0315 §4, 0587. ADR 0257'nin bütün tanımları dosya birimiyle ölçekleme kuralının yerini bu karar alır.
- Kaynak: Autodesk [Block Units and Insertion Scale](https://help.autodesk.com/cloudhelp/2025/ENU/AutoCAD-Core/files/GUID-6C46049D-8636-442D-8BAC-CF4FD515FDC0.htm), [INSUNITS](https://help.autodesk.com/cloudhelp/2025/ENG/AutoCAD-Core/files/GUID-A58A87BB-482B-4042-A00A-EEF55A2B4FD8.htm). Bunlar kaynak birim davranışının başvurusudur; KentOS'un bütün AutoCAD davranışlarını desteklediği anlamına gelmez.

## Gerçek dosyalardaki iki sorun

1. `Tek-katli-verandali-koy-evi-dwg.dwg`, `CAD paftası geçersiz: Yükseklik (µm) 1000 ile 10000000 arasında olmalı; 1 verildi` hatasıyla alınamıyordu. Kâğıt uzayındaki yatay çizginin sıfır yüksekliği 1 µm çerçeveye çevriliyor, pafta çekirdeğinin en az 1 mm seçilebilir öğe çerçevesi kuralını ihlal ediyordu. Model verisi okunabilir olmasına rağmen bütün aktarım reddediliyordu.
2. `2-1-ev-kat-plani.dwg` mm bildiriyor; `g.don1050x2100` bloğunun kayıttaki birimi de mm, mevcut INSERT ölçeği 0,1. Tanımın yüksekliği 2100 mm iken mevcut kopya 210 mm oluyor; panelden ölçek 1 ile eklenen kopya on kat büyük. Ayrıca kullanıcı dosyanın birimini cm seçince eski okuyucu **blok tanımını da on kat büyütüyordu**. Mevcut kopya doğru görünse bile yeniden eklenen kopya yanlış büyüyordu.

Dosyanın yanlış birim bildirmesi ile okuyucunun blok tanımını yanlış normalize etmesi farklı sorunlardır. Mimari içerikten birim tahmin edip sessizce veri değiştirmek yoktur. İçe aktarmanın mevcut **Dosyanın birimi** seçimi ve komut/Python `fileUnit` / `file_unit` seçeneği kullanılır.

## 1. Kâğıt geometrisi ile seçme çerçevesi farklıdır

- Yerel pafta kitabı projenin verisidir; model nesneleriyle aynı katmana karışmaz. CAD kâğıt koordinatları aktarım sözleşmesinde metre, sayfa boyu mm, pafta kitabında tamsayı µm'dir.
- Çizgi/çoklu çizgi/çokgen için her çerçeve kenarı `ITEM_MIN` (1000 µm) altındaysa çerçeve o büyüklüğe tamamlanır.
- Ardından **özgün noktalar**, bu tamamlanan çerçeveye göre kesirli koordinatlara çevrilir. Örneğin 0,5 mm çizginin çerçevesi 1 mm, son noktası çerçeve genişliğinin yarısıdır. Yatay çizginin iki Y değeri aynı kalır. Geometri çerçeveyi dolduracak biçimde gerilmez.
- PDF/SVG ve CAD dışa aktarma aynı öğe noktalarını kullanır. Seçilebilir alanın büyümesi, fiziksel çizginin büyümesi değildir.
- Bu değişiklik çizgi/poligon aktarımına aittir. Küçük elips/daire, özel yazı biçimleri, viewport clipping/layer override, eğrilerin analitik saklanması ve bütün CAD annotation türleri için yeni model ihtiyacı CAD-SHEET-08/09/12/15 kapsamındadır. 1 mm altı bütün öğelerin desteklendiği iddia edilmez.

## 2. Her blok tanımı kendi birimiyle normalize edilir

`D`: dosya için seçilmiş metre/birim katsayısı. `B`: hedef bloğun kendi metre/birim katsayısı. `O`: bir INSERT'in sahibi olan modelin veya üst bloğun katsayısı.

1. Model geometrisi ve INSERT yerleştirme noktaları `D` ile çevrilir.
2. Bir tanımda açık ve tanınan `BLOCK_RECORD` 70 varsa base, geometri ve ATTDEF uzunlukları **o bloğun katsayısıyla** çevrilir. Böyle bir kayıt yoksa mevcut dosya birimi kuralı kullanılır. Bilinmeyen/birimsiz kaynaklardan ölçü tahmin edilmez.
3. Her INSERT'in X ve varsa Y ölçeği `O / B` ile çarpılır. Dönüş, ayna ve blok kimliği değişmez. İç içe bloklarda `O`, üst tanımın birimidir.
4. Böylece mevcut dünya geometrisi korunur: `O × s × ham_geometri = (s × O/B) × (B × ham_geometri)`. Base noktası da tanımla çevrildiği için sıfır olmayan base ile aynıdır.
5. Yeniden kullanılacak tanım gerçek metre boyundadır; `cad.blocks.insert` ölçek 1 ile aynı fiziksel boyu verir. Kayıtlı birim ikinci kez uygulanmaz.
6. Patlatılmış okuma yolu raw geometriyi doğrudan dosya katsayısıyla çevirir. Aynı analitik beklenen koordinatlarla normal blok yolu ve patlatma yolu ayrı sınanır.

Bu kural dosyanın `$INSUNITS` değerini veya kullanıcının açık seçimini değiştirmez. `2-1…` dosyasında varsayılan mm ile eski INSERT 0,1 kalır; cm açık seçildiğinde mevcut INSERT 1 olur, tanımın yüksekliği yine 2,10 m, panelden yeni ekleme de 1 olur. Aynı dosyayı yanlış birimde okumayı otomatik olarak düzeltmiş saymayız.

DXF ve OpenCADCodec'in DWG → bellek DXF köprüsü aynı ortak okuyucuya gelir. UI, ürün komutu, Python ve MCP için ayrı ölçek hesabı yoktur. Yeni sözleşme alanı, `.kcad` şeması veya bağımlılık eklenmedi. Önceden yanlış içe aktarılıp kaydedilmiş projeler tahminle göç ettirilmez; özgün kaynaktan doğru birim seçilerek yeniden alınır.

## Kabul verisi ve sınırlar

- `scripts/fixtures/cad_sheet_cases.py`: KentOS kodu kullanmadan, `Fraction` birim tanımlarından 28 kaynak üretir; mm/cm/m/in/ft kombinasyonları, üç açık dosya birimi değişimi, sıfır olmayan blok base'i, iç içe blok ve farklı X/Y ölçekleri. Tanımın fiziksel boyu ve iki yerleştirmenin dünya uçları analitik olarak yazılır.
- Aynı üreticinin 20 kâğıt örneği: yatay/düşey, ters yönlü, kısa, ince ve kapalı yollar. Kaynak noktalar tamsayı µm olarak verilir. Çizginin uçları, geçerli öğe çerçevesi, model/kâğıt ayrımı ve dışa aktarma sınanır.
- Python gerçek tipli komutları çalıştırır; paftaları bağımsız `tools/kcad/kcad.py` okuyucusuyla okur, çerçeve ve kesirlerden fiziksel noktaları denetler. İçe aktarma undo/redo ve tekrar kullanım sınanır.
- Etkileşim izleri: `cad-sheet-thin-paper`, `cad-sheet-block-units`; gerçek masaüstü içe aktarma penceresi, dosya birimi seçimi, yazma ve geri alma. Önceki DWG içe/dışa aktarma izleri de sürer. Oynatıcı pencere adımındaki `pick` seçimini artık gerçekten gönderir; ayrı test seçimin yazılan INSERT ölçeğini 1000 yerine 10 yaptığını doğrular. İki iz açık/koyu tema ve 1440×900 / 1100×650 boyutlarında görüntülenir.
- Kullanıcının Downloads klasöründeki üç DWG'de plan → execute → undo → redo → `.kcad` kaydet → yeniden aç çalıştırıldı. `2-1…` kopyaları aynı SHA-256'lı dosyadır: 282 model nesnesi / 2 pafta. Köy evi: 9730 model nesnesi / 4 pafta. Kaynak kullanıcı dosyaları depoya eklenmedi.
- Bağımsız ezdxf 1.4.5 ile 28 dışa aktarılmış çizimin iç içe blokları açılıp analitik dünya uçları doğrulandı; ayrıca 20 kâğıt yolunun DXF köşeleri bağımsız okundu. Bu, native okuyucunun kendi çıktısını tekrar okumasından ayrı bir kontroldür.
- Bu oturumda: ortak format/kodek/başsız testleri, uygulamanın 8222 ortak komut durumu, 210 Python testi, MCP testleri, dört seçili masaüstü izinin üç klavye/ekran varyantı, Clippy, WASM derleme ve bağımlılık yönü kontrolü geçti. Performans/harici ortam testlerinin atlananları bu sayılara başarı olarak eklenmedi.
- Bunlar mevcut macOS ARM64 üzerinde KentOS kabulüdür. AutoCAD görsel/düzenleme uyumluluğu kabulü veya bütün DWG türlerinin kayıpsız desteği değildir. CAD-IO-20 kullanıcının önceki kararıyla atlanmış olarak kalır.

## Sonraki CAD-SHEET işleri

- CAD-SHEET-05: model ve paftayı birlikte yazan betik grubunun iptal/undo/redo sahipliği; sonraki bağımsız pafta düzenlemesinin eski model undo'suyla ezilmesini engelleme.
- CAD-SHEET-08: kağıt uzayı için analitik CAD annotation/entity modeli ve mevcut öğelerin yetenek matrisi.
- CAD-SHEET-09–16: kesin ölçek oranı, clipping, görünüm kilidi, katman override ve model kimliğiyle etkileşim.
