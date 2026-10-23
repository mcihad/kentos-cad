# ADR 0592: Arayüz, Python ve AI için ortak pafta işlemleri

- Tarih: 22 Ekim 2026
- Kapsam: CAD-SHEET-07; ADR 0252 ve 0591 üzerine.

## Karar

1. Tasarımcı ve otomasyon aynı `kentos.sheet/1` kitabını ve `ops` işlemlerini kullanır. Pafta ekle/adlandır/sil/çoğalt/sırala, öğe ekle/değiştir/grupla/hizala, sayfa ayarları, değişkenler ve kaynak kimlikleri için ayrı Python modeli kurulmaz. Ayrıntılı işlem şeması ve tipler ADR 0591'de açılmıştır.
2. `cad.sheets.set(details=true)` kitapla birlikte sistem şablonlarının tam verisini döndürür. Varsayılan yanıt önceki küçük pafta setidir; eski istemciler yeni alanları almak zorunda değildir. `pictures=true` gömülü baytları ayrıca ister. Sorgu revizyonu, nesneleri, kitabı ve geçmişi değiştirmez.
3. `cad.sheets.create` sistem şablonu veya tam özel şablonla yeni pafta oluşturur. `options` sayfa seçimi, ad, model merkezi (metre), ölçek paydası ve değişken cevaplarıdır. `id` pafta kimliğidir; öğe ve ana sayfa kimlikleri deterministik türetilir. Çok uzun ama geçerli pafta kimlikleri çocuk kimliğini taşırmaz; SHA-256 öneki kullanılır. `index` pafta sırasıdır.
4. Şablon galerisi ile komut aynı `kentos_sheet::create::from_template` fonksiyonunu çağırır. Fonksiyon doğrulama, soru cevapları, kimlik ve kaynak bağlarının yeniden eşlenmesi, ana sayfa, resim metadata'sı ve pafta ekleme işlemlerini bir kez hesaplar. Baytlar da beraber döner. Geçersiz veya sistem kimliğini taklit eden özel şablon reddedilir; bilinmeyen/tekrarlanan soruya cevap sessizce atılmaz.
5. `cad.sheets.fit` seçilen `item` görünüm penceresini `uids` nesnelerine veya `bounds=[minX,minY,maxX,maxY]` kutusuna sığdırır. İkisi birlikte verilemez; ikisi de yoksa bütün model kullanılır. `uids=[]` boş seçimdir ve reddedilir. Kaynak hiçbir zaman oturumun gizli seçimi değildir; arayüz seçimi açık kapsam/kimlik olarak verir.
6. Model kutuları aynı native geometri ve blok tanımlarından hesaplanır; bloklar açılarak doğru kapsamı verir. Arayüz ve otomasyon aynı `kentos_sheet::fit::map` fonksiyonundan işlem üretir. İç boşluk ve karelaj yazı bandından kalan alan kullanılır; dönüş korunur. `marginPct=5` nesne kutusunun her yanında %5 ek yer açar. Varsayılan, sığan en küçük tam paydadır; `standardScales=true` standart paydaya yükseltir. Çizgi de sığdırılır; tek noktada mevcut ölçek korunur. Taşan/sırasız/sonlu olmayan kutu reddedilir.
7. Arayüzde “Çizime sığdır” ve “Seçime sığdır” aynı hesabı %5 payla kullanır; eski “Görünüme sığdır” da aynı çekirdeğe geçer ve standart ölçeği seçer. Hızlı işlemler özellik alanlarından önce gösterilir. Pencerenin çerçeve kilidi ve sabit/atlas ayrımı korunur. Ayrı görünüm kilidi CAD-SHEET-09'un konusudur.
8. Yeni komutlar `cad.sheets.edit`in yazma yoluna işlemler üretir; model 1:1 kalır. Plan/doğrulama yazmaz, hata bütün işlemi reddeder. Bağımsız çağrı kâğıdın `inverse` geçmişindedir; açık grup veya changeset içinde ADR 0590'ın tek model+pafta undo/redo adımı olur. Resimler ve sonraki bağımsız düzenleme koruması da aynı yoldadır.
9. JSON olarak doğrulanan şablon ve seçenek alanları katalogda ayrıntılı şemalarıyla sunulur (`x-raw-record`). Python tam ham kaydı ya da üretilmiş tipi kabul eder; `from_json` bilinmeyen alanları temizleyip hatalı isteği başarılı hale getirmez. Son doğrulama ortak çekirdeğe aittir.

## Kullanım

```python
import kentos.cad as cad

library = cad.sheets.set(doc, details=True)
with doc.group("Kat planı paftası"):
    cad.sheets.create(
        doc, id="a101",
        source=cad.SystemSheetTemplateSource(id="sys:cad-mimari"),
        options={"name": "A-101 Zemin kat", "scale": 50},
    )
    book = cad.sheets.set(doc, details=True).book.to_json()
    page = next(s for s in book["sheets"] if s["id"] == "a101")
    viewport = next(i for i in page["items"] if i["kind"]["type"] == "map")
    cad.sheets.fit(doc, item=viewport["id"], uids=[room_uid], margin_pct=5)
```

## Doğrulama kaynakları

- `scripts/fixtures/sheet_tools_cases.py`: KentOS çağırmadan şablon kuralları ve temel trigonometriyle beklenen sonuçları üretir. `cad.sheets.create` için 23, `cad.sheets.fit` için 29 durum; ortak native/Python/MCP koşucularından yürür.
- `crates/sheet-ui/tests/designer.rs`: bağımsız komut senaryoları iki arayüz düğmesinden aynı fiziksel sonucu ve geri almayı verir.
- `crates/native/application/tests/all/sheet_tools.rs`: kitap/şablon/resimlerin salt okunur okunması, gerçek sistem şablonunun kaynak bağları, uzun kimlikler.
- `python/tests/test_sheet_tools.py`: gerçek Python host'unda modelin değişmemesi, kitap/şablon sorgusu, grup undo/redo ve KCAD yeniden açma; ham seçenek doğrulaması.
- MCP kabulü: create+fit tek changeset, aynı kitabı okuma ve tek undo ile eski kitaba dönüş.
- `sheet07_tools_screens`: gerçek model ve şablonla GPU ekranları, açık/koyu 1440×900 ve 18 px yazıyla 1100×650; dosyalar `.run/shots/sheet07-tools-*.png`.
- Web arayüzü ADR 0228 gereği ertelidir; ortak sözleşmeler ve WASM derlenebilirliği sürer.

- GPU ekranları açılıp incelendi. Açık/koyu 13 px ekranların oda yazısı bölgesinde 215 koyu piksel ve bire bir aynı RGB değerleri doğrulandı; önizleyicide bir defa eksik görünen yazı gerçek PNG'de eksik değildi. Gerçek yazı piksellerini sayan kabul kontrolü eklendi. Şablonun boş kurumsal/çizim numarası alanları beklenen ön denetim uyarılarıdır.

- Doğrulama sonucu: 215 Python testi (8294 ortak komut durumu dahil), native komut/kabiliyet/protokol testleri, 369 shared birim testi + 52 sheet entegrasyonu, 44 UI birim testi + 31 tasarımcı + 7 kitaplık testi; headless/MCP testleri ve yeni changeset kabulü geçti. Clippy ve mevcut WASM hedeflerinin derlemesi geçti.
