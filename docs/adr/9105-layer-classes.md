# ADR 9105: Katman ağacında stil sınıfları

- **Durum:** kabul edildi (2026-11-21). Plan: [plan/veri.md](../../plan/veri.md) VY-07.
- **İstek:** sahibin sözü (20 Kasım 2026): “katmanlara kategorize semboloji falan eklediğimizde katman altına sanal
  katmanlar oluşuyor qgis üzerinde mesela büyüklüğe göre renklendirdiğimde aynı katmanın alt dalı olarak ilgili renkler
  gözüküyor ve açılıp kapatılabiliyor burası da öyle olmalı”. 21 Kasım: satırlar katman stilinin sınıflandırmasıyla
  gelir.
- **Web:** ertelendi (ADR 0228). **`.kcad`:** CBS verisi sürüm 4 (§4). **Komut:** yeni `cad.layers.classes`;
  `cad.layers.set`'e `classVisible`, `classInvert`, `classCounts`.
- **Bağımlılık:** yeni dış kitaplık yok.

## §1 Bağlam ve araştırma

**Masaüstü CBS** (QGIS 4.2; kullanım kılavuzu “General Tools › Layers panel”, 21 Kasım 2026'da okundu; kurulu programın
ekranları `.run/gui-evidence/veri/qgis/01..03`):

- Kategorili, dereceli ve kurallı sembolojide “class entries are listed below the layer in the Layers panel”. Her
  sınıfın kutusu vardır; harita temaları “the layer checked node items” saklar.
- Sınıfın sağ tuşu: Toggle Items, Show All Items, Hide All Items; Select Features (“selects in the layer all the features
  matching that class”); Show in Attribute Table (“opens an attribute table filtered to only the features matching that
  class”); renk çarkı; Edit Symbol…; Copy/Paste Symbol. Çift tık sembol penceresini açar.
- Katmanın sağ tuşunda Show Feature Count: sayılar katmanın ve sınıfların adından sonra köşeli parantezde gösterilir.
- Kapalı sınıfın nesneleri çizilmez, etiketlenmez; kenetleme dizini de yalnız çizilen nesneleri alır.

**Türkiye:** TUCBS Veri Tanımlama Şablonu (TUCBS_VTS, ÇŞB CBS Genel Müdürlüğü, sürüm 1.0, 2020) §11 Kartografik
Gösterim: görüntüleme servis sağlayıcısı “her bir katmana hangi bilgilerin dâhil edileceğini, çıkarılacağını veya bir
araya getirileceğini tanımlamalı ve her bilgi kategorisi için kartografik gösterim kurallarını açıklamalıdır”; her
katmanın okunabilir başlığı ve en az bir varsayılan stili olur. Sınıfların katmanın altında adları ve sembolleriyle
listelenmesi bu kategori kurallarını kullanıcıya gösterir. Mekânsal Planlar Yapım Yönetmeliği'nin gösterimleri için
docs/STYLE.md §1.1.

## §2 Karar: sınıflar katman stilinin sınıflandırmasıdır

- **Kaynak:** katmanın işleyicisi (docs/STYLE.md §4). Kategorili: her kategori, varsa Diğer değerler. Aralıklı: her
  aralık. Kurallı: her kural, alt kuralları ondan sonra, derinliğiyle. Tek sembol, tematik işleyiciler ve katmanın kendi
  görünüşü sınıf üretmez.
- **Kimlik:** `c3` (kategori), `other`, `g2` (aralık), `r0.1` (kuralın yolu). İşleyici değişmedikçe kalıcıdır.
- **Koşul:** her sınıfın İfadeyle seç dilindeki koşulu üretilir: `kullanim = 'Konut'`; boş değerli kategori
  `(kullanim boş veya kullanim = '')`; Diğer değerler `kullanim değil içinde (…)` (boş değerli kategori yoksa
  `veya kullanim boş`); aralık `alan >= 500 ve alan < 1000` (sonuncusu `<=`); alt kural `(üst) ve (kendi)`; değilse
  kuralı `değil ((kardeş1) veya (kardeş2))`.
- **Hangi nesneyi alır:** çizimin kuralıyla, tek yerde (`kentos_scene_query::style_classes::ClassReader`). Nesne
  değerinin ilk kategorisine ve sayısını alan ilk aralığa gider. Alt kural üst kuralın nesnelerinden alır. Değilse kuralı
  hiçbir açık kardeşin almadığını alır. Yazılar ve ölçüler stil motorunca çizilmez, sayılmaz. Değerler katman süzgecinin
  sütun motoruyla okunur; ifadeler bir kez derlenir, parça parça gelen satırlara sorulur.
- **Mimari:** sınıf mantığı `scene-query`'dedir, çünkü hem uygulama crate'i hem masaüstünün stil motoru onu
  kullanabilir (`scripts/arch/deps.mjs`: uygulama stil motorunu kullanamaz). Uygulama onu `style_classes` adıyla yeniden
  dışa verir.
- **Komut:** `cad.layers.classes` sınıfları listeler. `count` ile her sınıfın nesne sayısını ve katmanın toplamını verir.
  Veri katmanında satırlar kaynaktan bir kez, yalnız sınıfların okuduğu alanlarla okunur (geometriyi okuyan koşulda
  geometriyle). Sınıfsız veri katmanının toplamını kaynak sayar. Çizim katmanında süzgecin gizlediği nesneler sayılmaz.

## §3 Karar: sınıfı açıp kapamak

- **Yazıldığı yer işleyicidir:** kategori ve aralıkta `enabled: false`, Diğer değerlerde `otherEnabled: false`, kuralda
  `enabled: false`. Yalnız `false` yazılır; açılınca alan kalkar. Stil çekirdeğine aralıkların ve Diğer değerlerin
  kapanması eklendi (kategori ve kural zaten kapanıyordu). Dosya biçimi işleyiciyi opak taşır; sürüm gerekmez.
- **Kapalı sınıf** nesnelerini tutar ama çizmez. Önceki sınıf onları almaz; Diğer değerler de almaz. Nesneleri
  etiketlenmez ve kenet almaz (veri katmanında okumanın etiket ve kenet için tuttuğu nesnelerden çıkarılır). Tabloda,
  aramada ve sayımda görünmeye devam eder. Basılı lejant kapalı aralığı ve Diğer değerleri de, kategori ve kural gibi,
  dışarıda bırakır.
- **Geri alma adımı değildir:** sınıf kutusu katmanın gözü gibi ağacın kendi değişikliğidir (`cad.layers.set`
  `classVisible`, `classInvert`; docs/adr/0020). Başarısız bir komutun geri koyduğu katman bayraklarına işleyici ve sayı
  ayarı eklendi. Sınıfın rengi bir stil değişikliğidir (`cad.layers.renderer`, geri alma adımı “Katman stili”).
- **Katman süzgecinden ayrıdır:** süzgeç nesneyi her yerden gizler; sınıfı kapamak yalnız çizimi durdurur.

## §4 Karar: Katmanlar'da

- **Satırlar:** açık katmanın altında, sınıfın derinliğiyle girintili; kutu, sembolün ana rengiyle bir örnek (alan
  kare, çizgi ve nokta katman türünün simgesiyle), ad. İpucu koşulu ve kapalıysa bunu söyler. Katmanın oku sınıfları
  açar kapar (`expanded`, dosyada saklanır). Sınıf satırı sürüklenmez ve üstüne bırakılan alınmaz; klavye gezintisi
  düğümden düğüme gider.
- **Sınıfın sağ tuşu:** Tümünü göster, Tümünü gizle, Tersine çevir; Bu sınıfın nesnelerini seç; Öznitelik tablosunda
  göster; Renk ▸; Sembolü düzenle…; Nesne sayısını göster.
  - Bu sınıfın nesnelerini seç: çizim katmanında nesneler hemen seçilir. Veri katmanında satırlar arka planda okunur,
    tabloda seçilir ve haritada çerçevelenir; Seç gibi en çok 10 000 satır. Seçim öncekinin yerine geçer.
  - Öznitelik tablosunda göster: tablo o katmanda, İfade süzgeci sınıfın koşuluyla açılır.
  - Renk ▸: sınıfın sembolü o renkle yazılır; kütüphane sembolü önce sınıfa alınır.
- **Nesne sayısını göster** (katmanın ve sınıfın menüsünde; `classCounts`, yalnız `true` yazılır): sayılar adlardan
  sonra köşeli parantezde, Türkçe basamak ayracıyla. Çizim katmanınınki burada, veri katmanınınki arka planda sayılır.
  Stil, süzgeç, çizim (çizim katmanında) ya da tabloya yazma (veri katmanında, `Document::data_written`) değişince yeniden
  sayılır.
- **`.kcad`:** CBS verisi sürüm 4: katman düğümünün `classCounts`'u. CBS 3 yükünde bilinmeyen alandır; yazılmış `false`
  ve grupta olması kötü değerdir. Örnekler: `class-counts.kcad`, `broken/class-counts-in-gis-3.kcad`,
  `broken/class-counts-false.kcad`, `broken/class-counts-on-group.kcad`. Bağımsız okuyucu ve başvuru yazıcısı da
  güncellendi.

## §5 Kanıt

- `cad.layers.classes`: 24 ortak durum (`scripts/fixtures/layer_classes_command_cases.py`; beklentiler sınıfların
  anlamıyla Python'da hesaplanır, veri katmanı `satirlar.gpkg`'ı okur). Koşucunun dosyayı okuduğu bir beklenti bozularak
  doğrulandı.
- `cad.layers.set`: 55 durum (13 yeni: sınıf kapat, aç, hepsi, tersine, değişmeyen, sayı, retler).
- `kentos-scene-query` `tests/style_classes.rs`: 8 sınama. Stil çekirdeği: kapalı aralık ve kapalı Diğer değerler hiçbir
  şey çizmez.
- `cargo test -p kentos-kcad`: 281 sınama; geçerli örnek sayısı 289.
- Masaüstü `layer_classes_tests`: çizim katmanında satırlar, kutu (geri alma adımı yok), Tersine çevir, sayılar, seçim,
  tabloda gösterme, renk; veri katmanında sayılar ve seçim GeoPackage'tan. Katmanla ilgili 198 masaüstü sınaması geçer.
- Gerçek pencere, bir milyon satırlık PostGIS tablosu ve bir GeoPackage:
  - sınıflar katmanların altında renk örnekleriyle;
  - Konut kapatılınca parselleri etiketleriyle birlikte haritadan kalktı;
  - Nesne sayısını göster: dört sınıfın her biri 250.000, katman 1.000.000; 25 saniye içinde;
  - Öznitelik tablosunda göster: `kullanim = 'Park'`, 250.000 satır;
  - Bu sınıfın nesnelerini seç: Eğitim'in ilk 10.000 satırı seçildi ve çerçevelendi;
  - Renk ▸ Eflatun: Ticaret parselleri ve ağaçtaki örnek yeni renkte; geri alınabilir.
- Bu sınamada bulunup düzeltildi: kapatılan sınıfın etiketi haritada kalıyordu; sınıf seçimi önceki seçime ekleniyordu.
  Demo projesinin nokta sembolü stil motorunun tanımadığı bir türle yazılmıştı (demo betiği düzeltildi).

## §6 Kalan

- Çizim katmanının etiketleri kapalı sınıfa bakmaz (etiket motoru katmanın bütün nesnelerini alır). CBS projesinde veri
  bir kaynakta durduğu için (plan/veri.md VY-09) öncelik veri katmanınındır; çizim katmanı ayrı iş.
- Sınıf satırında çift tık (QGIS'te sembol penceresi) yok; Sembolü düzenle… menüdedir ve Katman stili'ni katmanda açar,
  sınıfı seçmez.
- Sayım her değişiklikte bütün tabloyu tarar. Kaynakta gruplanmış sayım (`GROUP BY`) sürücü sözleşmesine eklenirse
  kategorili stilin sayısı anında gelir.
- Web (ADR 0228).
