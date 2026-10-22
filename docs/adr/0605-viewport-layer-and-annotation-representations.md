# ADR 0605 — Viewport katmanları ve açıklama temsilleri

Tarih: 22 Ekim 2026. CAD-SHEET-15.

## Sözleşme ve kaynak sahipliği

`MapItem.annotationScale` yoksa açıklama temsili fiziksel görünüm paydasını izler; varsa seçilen temsil paydasıdır. Pozitif, sonlu ve en çok 1.000.000 olur. Fiziksel kamera oranı, çerçeve, model koordinatları ve nesne kimlikleri bu seçimle değişmez. Temsilin kendi paydasındaki boyu kâğıda fiziksel kamera ölçeğiyle izdüşürülür; seçilen temsile katılmayan annotative nesne çizilmez.

`setViewport` aynı atomik işlemde `annotation: auto/fixed`, `layers` ve `layerOverrides` alır. Kamera kilidi bu görünüş değişikliklerini engellemez; öğenin bütününü kilitleme kuralı sürer. Auto'ya dönüş ve boş override listesinin yazılması eski değeri gerçekten temizler. Önceden bulunmayan listenin inverse'ındaki null varsayılan boş listeye okunur; undo modeli veya kamerayı değiştirmez.

Kaynak dokümanın annotative temsilleri `Document::annotation_scales` ile sıralı ve tekil olarak keşfedilir. Masaüstü seçicisi ile `cad.sheets.set(details=true).annotationScales` aynı listeyi kullanır. Serbest sayısal payda da kabul edilir; bulunmayan temsilde nesnenin görünmemesi açık davranıştır. Masaüstünde liste belge değiştiğinde yenilenir, işaretçi hareketinde bütün çizim taranmaz.

Türetilmiş sahne ve spatial önbellekleri hem fiziksel hem açıklama paydasını anahtarlar. Mimari semboller fiziksel paydayı, annotative metin/ölçü/kılavuz seçilen açıklama paydasını kullanır. Farklı viewport'ların kaynak nesneleri aynı kalır. Katman ezmesi ByLayer görünüşüne uygulanır; nesnenin kendi rengi korunur. Yazı toplayıcı ve masaüstü yazı boyayıcısı da etkin viewport katman renklerini kullanır.

## Baskı ve görünürlük

Ekrandaki 5–240 piksel CAD yazısı görünürlük sınırı baskının kuralı değildir. `LabelSize::Print` özgün fiziksel boyu korur; küçük veya büyük CAD yazısını ekran LOD'u nedeniyle düşürmez ve büyütmez. Normal model ekranının Gerçek/Kaybolmasın/Ekranda sabit davranışı değişmez. Yazdırma toplayıcısı ve masaüstü pafta görünümü aynı baskı toplama yolunu çağırır.

Etkin katman listesi, kaydedilmiş katman durumu ve frozen/plot ezmeleri yazı yerleştirmeden önce çözülür. Görünmeyen nesneler yerleşimde engel veya komşu ayırmaz; bir viewport teması globalde gizlenen katmanı açabiliyorsa yazısı da gelir. Açık katman listesi CAD çizgilerini ve yazılarını birlikte sınırlar. PDF metinleri aranabilir kalır.

## Kabul verisi

KentOS kullanmayan `scripts/fixtures/viewport_annotation_cases.py` beş fiziksel oran × dört görünüş üretir: otomatik, yarı payda, bulunmayan temsil ve dondurulmuş açıklama katmanı. Kırmızı/mavi/mor/yeşil ByLayer görünüş, 0,25–0,70 mm çizgi ağırlığı ve farklı çizgi türleri aynı model üzerinde sınanır; nesneye açık yeşil renk korunur. Kaynak snapshot'ının değişmemesi, kamera kilidi, undo, Auto/list temizliği ve geçersiz paydalar ayrıca denetlenir.

Ortak core, native vektör/yazı toplayıcısı, tasarımcı, Python, MCP ve mevcut WASM yüzeyi aynı durumları kullanır. GPU ekranlarında dört viewport yan yana, açık/koyu tema ve büyük yazı; Poppler ile PDF görüntüsü karşılaştırılır. Baskıda küçük/büyük metin ve global gizli katmanın tema/list görünürlüğü için regresyonlar vardır.

CAD-SHEET-16 görünüm içinden seçim/kenet/ölçümün özgün nesne kimlikleriyle görünürlüğü ayrı olarak ele alınır. Web arayüzü ADR 0228 gereği ertelidir; TS/katalog/SDK ve ortak WASM korunur.

Kabul kaydı: core kütüphane 380, core entegrasyon 63 (+4 mevcut ignore), native plot 23 ve iki ek görünürlük/temsil testi, tasarımcı 38, MCP kabul 8 geçti. Mevcut 230 Python testi geçti; yeni 20 durumlu Python testi grup undo denetimi düzeltilerek ayrıca geçti. Ekran görüntüsü ve mevcut annotative boyut PDF testi geçti. Clippy, schema kapalı 18 komut testi, katalog/SDK, bağımlılık kapısı, WASM derlemesi ve 20 temsil smoke durumu temiz. Görüntüler `.run/shots/sheet15-annotations-*` ve `sheet15-controls-*`; aynı sahnenin PDF/Poppler görüntüsü incelendi.
