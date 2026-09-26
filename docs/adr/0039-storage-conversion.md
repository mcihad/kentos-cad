# ADR 0039: Saklama biçimleri arası açık dönüştürme: yeni proje olarak

- **Durum:** kabul edildi (2026-09-26). Yön TODOS.md §10.1'den gelir: modlar arasında sessiz geçiş yapılmaz, “PostGIS'e aktar” gibi geçişler açık komutlardır; aynı geometri aynı anda iki bağımsız yazılabilir otoriteye sahip olmaz. `PG-20`'nin “CAD projesini PostGIS'e kaydet” yarısının ilk adımıdır.
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** CLAUDE.md §13, §15; ADR 0028 (kopya), 0031 (dosya projeleri), 0033 (tek anlık görüntü), 0034 (geri yükleme), 0036 (içe aktarım)

## Bağlam

- Proje açılırken saklama biçimi seçilir ve sonra değişmez (ADR 0031, migration 0006'daki tetikleyici). Biçim değiştirmek isteyen kullanıcının yolu yoktu.
- Gereken parçalar bugün hazırdı:
  - dosyayı veritabanı projesine aktarma: ADR 0034 ve 0036'nın ortak içe aktarımı;
  - veritabanı projesinin tek anlık görüntüsü: ADR 0033;
  - yeni projeyi kopya gibi kurma: ADR 0028.

## Karar

- `project.convert` v1 `{ to, name?, tenantId? }`: projenin şimdiki hâlinden, `to` biçiminde **yeni bir proje** açar. `to`, kaynağın öbür biçimidir.
  - Kaynak değişmez. Böylece aynı çizimin iki yazılabilir otoritesi olmaz: yeni proje başka bir projedir.
  - Kaynak kendi biçimine “dönüştürülmek” istenirse alan yolu `to` ile reddedilir.
- **Dosya → veritabanı (“PostGIS'e aktar”):**
  - kaynağın en yeni revizyonu SHA-256'sıyla denetlenip çözülür;
  - her nesne kalıcı kimliğiyle ve 1. sürümüyle yeni veritabanı projesine yazılır; ayarlar, katmanlar ve stiller dosyadandır;
  - analitik CAD tanımları `cad_definition` olarak korunur (ADR 0006); CAD, GIS çizgisine indirgenmez;
  - sunucunun almadığı ilk nesne (±10⁹'u aşan değer) dönüştürmeyi yerini söyleyerek reddeder (`entities[i]`) ve hiçbir proje açılmaz;
  - kaydedilmiş revizyonu olmayan dosya projesi dönüştürülemez.
- **Veritabanı → dosya:**
  - kaynağın tek anlık görüntüsü (ADR 0033), kilit alınmadan çekilir;
  - görüntü yeni dosya projesinin 1. revizyonu olur; nesne sayısı revizyonla saklanır;
  - nesne önce depoya yazılır. Satırlar yazılamazsa nesne silinir. İşlemin sonucu bilinmiyorsa nesne kalır; proje yazılmadıysa saatlik temizlik onu projesi olmayan klasör olarak siler (ADR 0031 eki).
- **Yeni proje kopya gibidir (ADR 0028):**
  - çağıranındır; kaynağın açıklaması, türü ve etiketleri gelir;
  - geçmiş, paylaşım, sık kullanılanlar ve arşiv durumu gelmez;
  - adı verilmezse kaynağın adına biçim eklenir: “Ada 101 (PostGIS)”, “Ada 101 (dosya)”;
  - yanıt kopyanınkidir (`ProjectDuplicated`).
- **Yetki:** kaynakta `project.download` (dönüştürme, kopya gibi, buluta bir indirmedir; kurumun `viewer_download` politikası uygulanır) ve hedef çalışma alanında proje açma hakkı. Kaynak, kilit altında yeniden sorulur.
- Kaynağın denetim kaydı `project.convert` (hedef biçim, kaynağın revizyonu), yeni projeninki kaynağı yazar. Yeniden deneme aynı yeni projeyi alır.
- **Kod:**
  - yeni projeyi bir çizimden kuran ortak kod `importing::create_from_drawing`'dir; kontrol noktasından geri yükleme (ADR 0034) de onu kullanır;
  - içerik ya nesnelerdir (veritabanı projesi) ya da depoda bir revizyondur (dosya projesi).

### Bu dilimde bulunan hata

- Katalog kaydı (`listing.rs`) ve erişim listesi (`people.rs`) saklama biçimini dosya projelerinden önceki gibi hep `database` veriyordu. Web kataloğu dosya projesini yanlış gösterirdi.
- Düzeltildi: ikisi de projenin gerçek biçimini okuyor. `project_files.rs`'e gerileme denetimi eklendi; eski sabit değerle test düştü.

## Bu dilimde olmayanlar

- **Yerinde dönüştürme** (aynı projenin biçimini değiştirmek): kimlik, paylaşım ve geçmişin taşınması ayrı bir karardır.
- **“GIS geometrisine dönüştür/export et”:** tolerans ve kayıp önizlemesiyle (`PG-20`'nin öbür yarısı).
- **Dış PostGIS'e aktarma** (`PG-01..06`).
- Web ve masaüstü arayüzü: katalogda “PostGIS'e aktar” ve “Dosya projesine çevir”. Web ajanına gidecek.

## Doğrulama (26 Eylül 2026, Linux)

- `crates/server/application/tests/project_convert.rs`, gerçek veritabanı:
  - dosya → veritabanı: 14 nesne (13 türün hepsi) nesne nesne dosyayla aynıdır; ayarlar, katmanlar ve stiller de aynıdır. Kaynak dosya projesi olarak kalır. Yeniden deneme aynı projeyi verir;
  - uç değerli nesnesi olan dosya `entities[13]` ile reddedilir; proje listesi değişmez;
  - veritabanı → dosya: yeni projenin 1. revizyonu kaynağın görüntüsüyle nesne nesne aynıdır, nesne sayısı 2'dir, kaynak iki nesnesiyle kalır;
  - reddedilenler: aynı biçime dönüştürme (`to`), indirme izni kapalı görüntüleyici (403), revizyonsuz dosya projesi.
- Kontrol noktası geri yüklemesi ortak koda geçtikten sonra da testleri geçiyor (`project_checkpoints.rs`, 6).
- Kasıtlı bozma: aynı biçim denetimi kaldırılınca test düştü; geri alındı.
