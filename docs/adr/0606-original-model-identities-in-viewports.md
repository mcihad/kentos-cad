# ADR 0606 — Viewport'ta özgün model kimliğiyle etkileşim

Tarih: 22 Ekim 2026. CAD-SHEET-16.

## Tek kaynak, türetilmiş sorgu görünümü

Pafta viewport'u yeni bir model veya yeni nesne kümesi oluşturmaz. Kaynak dokümandaki `Slot`, UUID ve kaynak 1:1 koordinatları değişmez. Kâğıt geçmişinin kamera, katman ve kırpma bilgileri yalnız türetilmiş geometri dizininin hangi temsili sorguladığını belirler. Ölçüm, gelen UUID ile özgün nesneyi ölçer; temsilin yazı boyunu model uzunluğu gibi sunmaz.

`Viewport` ortak merkez/dönüş/kâğıt dönüşümüne ek olarak fiziksel payda, açıklama paydası, katman seçimi/ezmeleri ve modeldeki tam kırpma halkasını verir. Katman görünürlüğü/stili çözümü saf `kentos_sheet::layers::map_layers` içindedir. Native plot, masaüstü ve sorgu uygulaması aynı çözümü kullanır; ikinci bir freeze/theme kuralı yoktur.

Native `viewport_scope` kaydı özgün id/katmanıyla üretir. Yalnız annotative temsil farklıysa nesnenin geometri değeri türetilir; katılmadığı temsil dizinden çıkar. Kaynak `Document` kopyalanıp değiştirilmez. Masaüstünün aktif dizini bu politikayı artımlı değişiklik günlüğüne uygular; ekleme, değişiklik, silme ve undo aynı kaynak kimlikleriyle güncellenir. Kamera paydası/temsil değişince geometri yeniden okunur; yalnız pan veya kırpma değişince kaynak geometri yeniden kurulmaz. Sabit işaretçi hareketinde kaynak dizini yeniden okunmaz.

Kırpma içindeki özgün kimlikler spatial ağacın çokgen sorgusundan alınır ve `Store.scope_visible` içinde tutulur. Pencere/çit/hepsi/benzerleri gibi sorgular bu üyeliği diğer görünürlük kurallarıyla birlikte uygular. Nesne alt referansları ve blok parçalarının yolları özgün nesneye aittir. Kenet adaylarının tamamı toplanır; kırpma dışındakiler elenir ve kalanların sırası korunarak sonuç seçilir. En yakındaki dış nokta içerideki geçerli noktayı düşüremez. Gizli mimari fazlar ve baskı dışı PDF altlıkları da pafta etkileşiminde hedef değildir. Native başsız dizinin daha önce bütün kenet türlerini açık tutan katman maskesi düzeltildi: native ve interaction aynı çekirdek `layer_mask` hesabıyla katmanın kenet kapatma/tür seçimini uygular; Uç nokta–Çeyrek ve Nokta–Ekleme noktası grupları korunur.

## Python, AI ve arayüz

`cad.entities.select`, `cad.entities.pick`, `cad.snap.find` isteğe bağlı `modelScope: {sheet,item}` alır. Nokta ve toleranslar hâlâ model metresidir. Bulunamayan/sayfaya ait olmayan pencere, kopuk adlı görünüm ve konumlanmamış kamera aynı ortak kaynak çözümünden hata verir; validate/plan/execute kaynak değiştirmez.

Açık `cad.changeset.apply.modelScope` içindeki bu üç okuma komutu kapsamı devralır. Adım başka bir kapsam isteyemez. Model yazma komutları kaynağın normal komut kayıt yolunu, atomik rollback ve model undo geçmişini kullanmaya devam eder. Yanıt UUID'leri doğrudan `Document.entity`, `Document.measure` ve model düzenleme komutlarında kullanılabilir; kâğıt öğesi kimliği bunların yerine geçirilmez.

Masaüstünde kâğıt içinden gerçek araç oturumu aynı dizini kullanır. İç araç sorgusu da açık `modelScope` girdisini taşır; doğrulanmış sıcak dizinle çalışırken her işaretçi hareketinde kitap JSON'unu yeniden çözmez. Aktif temsilin seçim vurgusu ve kenet işareti de aynı geometriyi izler; saklı nesne vurgulanmaz. Yeni açıklamanın varsayılan fiziksel boyu ekran zoom'undan ve modelin genel plotScale'inden bağımsız olarak aktif viewport paydasından hesaplanır; 1:37,5 görünümde 2,5 mm yazı 0,09375 m model yüksekliğiyle oluşur. Kâğıda/model sekmesine dönülünce normal kaynak dizini geri gelir. Aktif canlı kaynağın bağı koparsa kapsam sessizce sınırsız modele dönüşmez: araçtan çıkılır, kamera geri yüklenir ve kaynak onarımını söyleyen uyarıyla kâğıda dönülür.

## Çıktı ve önbellek sınırı

Aktif pencerenin etkileşim dizini başka bir pencerenin veya dosya çıktısının veri kaynağı değildir. Pafta boyayıcısı gerekirse sınırlı ölçek önbelleğindeki değişmez okuma görünümünü kullanır; bu geometri/indeks önbelleğidir, projede ikinci model veya kalıcı nesne değildir. Tablo/lejant ve PDF toplama da etkileşimden bağımsız kaynak indeksini okur. Aynı modelin kırpmasız başka görünümünün vektör içeriği, model kapsamına girilmeden önce ve sonra eşit kalmalıdır.

## Kabul verisi

`viewport_identity_cases.py`, KentOS kullanmadan 20 durum üretir: beş fiziksel oran, dört temsil/freeze görünüşü, içbükey kırpma, 500.000/4.500.000 m arazi koordinatları ve sınırın hemen dışında daha yakın olan nokta. İç nokta kaynak id 5, dış nokta id 4'tür. Sorgu sonucu özgün id 1/2/5 ve yalnız görünüyorsa açıklama id 3'tür; kaynakta beş nesne kalır. Kaynak çizgisinin uzunluğu her pencerede 5 m kalır.

Native komut, artımlı interaction dizini, Python ve MCP aynı veri üzerinde seçme/kenet, kaynak kimlikleri, snapshot değişmezliği ve kapsam devrini sınar. 25 sabit scope/sync çağrısı kaynak geometriyi yeniden okumaz; yeni nokta ekleme ve undo dizine artımlı yansır. Masaüstü gerçek araçla seçim, fiziksel yazı boyu, kâğıda dönme ve farklı pencere/çıktı izolasyonunu sınar. Açık/koyu/büyük yazılı GPU ekranları açılarak kaynak noktanın öznitelikleriyle karşılaştırılır.

Web uygulaması ADR 0228 gereği erteli; aynı modelScope, temsil dizini ve kenet sınır politikası web'e dönüşte bağlanmalıdır. TS/katalog/SDK ve ortak WASM derlemeleri sürer.

Kabul kaydı: 232 Python testi; MCP kabul 9; native uygulama entegrasyon 132 (+22 mevcut ignore) ve son 3 viewport kimliği/ret/katman kenet testi; interaction 63 ve son 20 durumlu artımlı sorgu testi geçti. Geometri çekirdeği 514 (+5 mevcut ignore), pafta core 380+63 (+4 mevcut ignore), schema kapalı komut 18; katalog/SDK, dependency gate ve son clippy temiz. Sheet/geometry optimize WASM derlemesi ve mevcut bütün pafta smoke durumları geçti. Gerçek masaüstü kaynak seçimi/özellikler, fiziksel yeni yazı, çıktı kaynak izolasyonu ve kopuk kaynakta kâğıda dönüş testleri geçti. `.run/shots/sheet16-source-selection-{light,dark}-{13,18}.png` mevcut üç varyantı açılarak incelendi.
