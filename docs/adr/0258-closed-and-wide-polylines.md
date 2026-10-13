# ADR 0258: Kapalı ve genişlikli çoklu çizgi

- **Durum:** kabul edildi (2026-10-13). [plan/cad.md](../../plan/cad.md) §6 `CAD-DATA-18` ve `CAD-DATA-19`. Kapsamı ben belirledim
  (sahibin sözleri: “CAD data bölümüne geç ve orayı tamamla”; “Araç = komut” kuralı, 13 Ekim).
- **Bağlam belgesi:** ADR 0142 (köşe kotu ve düzenlemelerde taşınması), 0174 (çok parçalı çizgi), 0139 (nesnenin çizgi kalınlığı),
  0257 (DXF içe almanın komutu), 0025 ve `docs/specs/kcad-v2.md` (KCAD v2), 0090 (stil motoru), 0256 (birimler).
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

KentOS'ta iki yol türü vardı: açık çoklu çizgi (`polyline`) ve kapalı alan (`polygon`). Kapalı bir CAD çizgisinin yeri yoktu:

- **DXF:** kapalı LWPOLYLINE ve POLYLINE her zaman kapalı alan oluyordu. Bir CAD çiziminin kapalı duvar ekseni, kolon çerçevesi ya
  da kapalı yardım çizgisi CBS alanına dönüşüyordu: alanı, iç dolgusu, alan işlemleri ve `$alan`'ı oluyordu.
- **Genişlik:** AutoCAD'in çoklu çizgi genişliği (LWPOLYLINE'ın 43'ü, köşelerin 40 ve 41'i) okunmuyordu; rapor “genişlik alınmadı”
  diyordu. Çizgi kalınlığı (`lineWeight`, kâğıt mm'si) başka bir şeydir: çizimin ölçeğiyle büyümez; genişlik çizimin uzunluğudur.
- **Planın istekleri:** `CAD-DATA-18` sabit genişlik ve kenarın başlangıç ve bitiş genişliği, kâğıt kalınlığından ayrı alan;
  `CAD-DATA-19` açık ve kapalı çoklu çizgi ile alan ayrımı, kapalı CAD çizgisinin zorla alan olmaması.

## Karar

### 1. Kapalı çoklu çizgi

Çoklu çizgiye `closed` alanı eklenir (`PathEntity.closed`, çekirdekte `Shape::Polyline.closed`):

- **Anlamı:** son köşe ilk köşeye bir kenarla bağlanır. Kapanan kenarın bükümü `bulges`'ın sonuncusudur; böylece kapalı çizginin
  `pts` ve `bulges`'ı alanınkiyle aynı düzendedir.
- **Alan değildir:** kapalı çizginin `$alan`'ı yoktur (`entity_area` yok); İşlemler'de alan sayılmaz. Gösterilen alanı
  (`enclosed_area`) Öznitelikler'de ve kartta “Alan” satırıdır; AutoCAD'in LIST'i gibi.
- **Tek parçalıdır:** kapalı çizginin `parts`'ı olmaz; çok parçalı çizgi açıktır (ADR 0174).
- **En az iki köşe:** iki köşeli kapalı çizgi kapanan kenarı yaylıyken bir halkadır.

Çekirdekte yolun kapalılığı tek yerden okunur (`is_closed_path`): alanın halkası ya da kapalı çizgi. Kenarlar, çizim, uzunluk,
kenet, tutamaçlar (kapanan kenarın ortası da), Buda, Kır, Böl, Parçala, Patlat (kapanan kenar da), Ötele (kapalı çizgi kapalı ötelenir),
köşe ekle ve sil, Köşe yuvarla ve Pah (ilk köşe de köşedir), Yönü çevir, Sadeleştir, Paralel kaydır (hedef alan da), Biçim değiştir
(alanın halkası gibi, kapalı kalır), Esnet, taşıma, döndürme, ölçekleme, aynalama ve Oturt kapalılığı korur. Ucu olmayan çizgide
Uzat, Uzat-kısalt ve Sürdür çalışmaz; Birleştir onu atlar (`CLOSED_HAS_NO_END`). Buda ve Kır kapalı çizgiyi açar, AutoCAD gibi.

Kapalı eğrinin Patlat'ı ve tam elipsin ötelemesi şimdilik alan verir: web'in kaydedilmiş çağrılarıyla eşlik korunur (§5).

### 2. Genişlik

Çoklu çizgiye `widths` alanı eklenir: köşe başına bir çift, o köşeden çıkan kenarın başlangıç ve bitiş genişliği:

- **Birim:** çizimin uzunluğu (metre; yerel projenin birimi okumada ve yazmada). Birim değişince ölçeklenir
  (`units::scale_entity`), çizim ölçeklenince de (`transform`).
- **Açık çizgi:** son köşeden kenar çıkmaz, son çift `[0, 0]`'dır. Kapalı çizgide son çift kapanan kenarındır.
- **Hiçbiri:** alan yoksa ya da hepsi sıfırsa genişlik yoktur; çizgi kalınlığıyla çizilir.
- **Sabit genişlik:** bütün çiftler aynı sayıdır; ayrı bir alan değildir (DXF'te 43 olarak yazılır).
- **Kalınlıktan ayrı:** `lineWeight` kâğıt mm'sidir ve kalır; genişlikli kenarlar genişliğiyle, genişliksizler kalınlığıyla çizilir.

**Çizim** (`geom::band`): AutoCAD'in dolgusu açıkken çizdiği gibi. Genişlikli her kenar başlangıcından bitişine şerittir, uçlarında
düz kesilir: düz kenar yamuk, yay genişliği açı boyunca doğrusal değişen halka dilimi. İki genişlikli kenarın buluştuğu köşede dış
yandaki boşluk gönyeyle kapanır; gönye yarım genişliğin dört katından uzağa giderse pah kırılır. Genişliksiz kenarlar çizginin
kendisidir. Şerit, tablo çerçevesi ve kılavuz ok başı gibi nesnenin renginde doludur (`MIXED` kaydı, `leader_symbols_of`);
genişlik nesnenin görünüşüdür, sembolün ve işleyicinin yerine geçer. Nesnenin kutusu şeridi kapsar.

**Düzenlemelerde taşınması** (`ops::widths`, kotun kurallarıyla, ADR 0142): çekirdek geometriyi genişliksiz hesaplar; uygulama
yazılan her kenara kaynağın genişliğini verir:

1. kenar kaynağın bir kenarı boyunca uzanıyorsa (uçları ve ortası üstünde) o kenarın genişliği, uçlarında doğrusal: kesilen parça,
   eklenen köşe, budanan ve kırılan çizginin parçası, ters çevrilen çizgi (genişlikleri yer değiştirir);
2. nesnenin kendi köşeleri taşınmışsa (tutamaç, Esnet, Öznitelikler, Ötele'nin kopyası) aynı sıradaki kenarınki;
3. uç kaynağın bir köşesindeyse oradan çıkan kenarın başlangıcı (ya da oraya gelen kenarın bitişi): Köşe sil'in birleştirdiği iki kenar;
4. uç kaynağın bir kenarı üstündeyse o kenarın orada genişliği: köşe yuvarlamanın ve pahın yeni kenarı;
5. Ötele'de kaynakların en yakın noktasınınki.

Taşınamayan genişlik `widths_lost` uyarısıdır. Geometriyle yazılan genişlik olduğu gibi yazılır; hepsi sıfır “genişlik yok” demektir.

### 3. Komutlar

- `cad.polyline.create`: `closed` ve `widths` (kenar başına, `bulges` gibi; kapalıda kapanan kenar dahil). Yanlış sayı, sonlu
  olmayan ya da eksi genişlik `invalid_polyline`.
- `cad.entities.edit`, `cad.entities.create` ve onlardan yazan anlamsal komutlar: `EntityGeometry::Polyline`'ın `closed` ve
  `widths`'i. Kurallara uymayan `invalid_polyline` (`path_extras_problem`: alanın ikisi de olmaz; kapalı ya da genişlikli çizgi tek
  parçalı; genişlik köşe başına bir çift, sonlu, eksi değil; açık çizginin son çifti sıfır).
- `file.import.dxf`: `closedPolylines` (`area` ya da `line`); verilmezse CAD projesinde kapalı çizgi, öbürlerinde alan. Yanıtta
  kullanılan değer. Masaüstünün içe aktarma penceresinde “Kapalı çoklu çizgiler” (Projeye göre, Alan, Kapalı çizgi).

### 4. Dosyalar

- **KCAD v2 şema 46** (`docs/specs/kcad-v2.md`): çoklu çizginin `closed` (yalnız `true`) ve `widths`'i; yazıcı 46'yı yalnız bir
  çoklu çizgi kapalı ya da genişlikliyken yazar, öbür çizimler eskisiyle bayt bayt aynıdır; eski okuyucu reddeder. Tipli sütunlarda
  yol bitleri 13 ve 14. `FORMATS_VERSION` 59. Bağımsız Python okuyucusu ve yazıcısı, örnek `polyline-extras.kcad` ve on iki bozuk dosya.
- **DXF okuma:** LWPOLYLINE'ın 43'ü ve köşelerin 40 ve 41'i, POLYLINE'ın ve VERTEX'lerin 40 ve 41'i genişlik olur; bloğun ve dosya
  biriminin ölçeğiyle büyür. Eşit olmayan ölçekte genişlik alınmaz (söylenir). Kapalı LWPOLYLINE ve POLYLINE `closedPolylines`'a göre
  alan ya da kapalı çizgi olur; KentOS'un “line” işaretlediği her zaman kapalı çizgidir; alan için yeterli köşesi olmayan da kapalı
  çizgidir. Alan olarak alınan genişlikli çizginin genişliği söylenir.
- **DXF yazma:** kapalı çizgi kapalı LWPOLYLINE ve KENTOS verisinde `line` işaretidir; genişlik bütün kenarlarda aynıysa 43, değilse
  köşe başına 40 ve 41. Köşe kotları için 3B POLYLINE yazılan genişlikli çizginin genişliği yazılmaz (söylenir).
- **GeoJSON, Shapefile, KML, GPX:** kapalı çizgi son noktası ilk noktası olan çizgi olarak yazılır; genişlik bu biçimlerde yoktur
  (söylenir).
- **Sunucu (PostGIS, CLAUDE.md §15):** kapalı ya da genişlikli çoklu çizginin kaynağı, kenarları düz olsa da tanımıdır
  (`cad_definition`): geometri kaynak olsaydı `closed` ve `widths` kaybolurdu. İzdüşümü kapalı çizgide kapanan kenarıyla, başına
  dönen çizgidir.

### 5. Sonraya kalanlar

- **Kapalı eğri:** kapalı eğrinin (Catmull–Rom) Patlat'ı ve tam elipsin ötelemesi bir CAD çiziminde kapalı çizgi vermelidir; çekirdek
  bugün alan veriyor, çünkü web'in kaydedilmiş çağrıları (`tests/calls.rs`) onu bekliyor. Değişiklik web'le birlikte, NURBS
  maddesiyle (`CAD-DATA-15`…`17`) yapılır.
- **Birleştir ve Çizgiye çevir:** başına dönen zincirin Birleştir'i ve alanın Çizgiye çevir'i bugünkü gibi alan ve ilk noktası
  yinelenen açık çizgi verir; kapalı çizgi seçeneği ayrı iştir.
- **Köşe tablosunda genişlik:** kenar kenar başlangıç ve bitiş genişliği komutla ve Python'la yazılır; Öznitelikler sabit genişliği
  yazar. Köşe tablosunun genişlik sütunları ayrı iştir.
- **Web:** okuma, çizim ve araçlar TODOS.md §0.2'de.

## Sonuçlar

- CAD çiziminin kapalı çizgisi çizgi kalır; CBS alanı ancak istenirse olur.
- Genişlikli çizgi AutoCAD'deki gibi görünür ve DXF'e gidip gelir; kâğıt kalınlığı ayrı kalır.
- Eski çizimler değişmez: şema 46 yalnız yeni alanları kullanan çizimde yazılır.
- Web'in okuyucusu ve çizimi bu alanları bilmez; web dönünce TODOS.md §0.2'deki satırla gelir.

## Doğrulama

- Çekirdek: `ops::widths` ve `geom::band` testleri; kapalı yolun her işlemdeki durumu.
- KCAD: `python3 scripts/fixtures/kcad_v2_reference.py --check` (648 dosya), Rust kodeğinin `fixtures.rs`'i.
- Komutlar: `fixtures/commands/v1/cad.polyline.create.json`'un kapalı ve genişlikli durumları bağımsız başvurudan
  (`scripts/fixtures/polyline_extras_cases.py --check`).
- Düzenlemeler ve içe aktarma: `cad.entities.edit`'in genişlik taşıyan ve `widths_lost` söyleyen durumları
  (`edit_command_cases.py --check`), `cad.entities.create`'inkiler (`create_command_cases.py --check`), `file.import.dxf`'in
  `closedPolylines` durumları (`dxf_import_command_cases.py --check`).
- Masaüstü: `exchange::tests::closed_polylines_go_in_as_the_window_says` (CBS projesinde alan ve genişliğin kaybı söylenir; Kapalı
  çizgi seçilince dosya yeniden okunur, kapalı çizgiler genişlikleriyle; CAD projesinde kendiliğinden çizgi, Alan seçilince alan).
  Resimler `KENTOS_SHOTS_ONLY=cokluc-cizim,cokluc-oznitelikler,cokluc-genislik,cokluc-kapat cargo test -p kentos-desktop
  tools_screens -- --ignored --nocapture` (`.run/shots/arac-cokluc-*`) ve `exchange::tests::unit_screens`'in
  `aktar-*-28-dxf-kapali-cizgiler`, `aktar-*-29-dxf-kapali-cizgi-secilen`'i.
