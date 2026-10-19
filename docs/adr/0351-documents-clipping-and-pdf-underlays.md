# ADR 0351: Belge sekmeleri, kırpma ve PDF altlığı

- Durum: Kabul edildi (19 Ekim 2026)
- Kapsam: plan/cad.md CAD-REF'in son sekiz maddesi tek grupta (sahibin isteği: “8 maddenin hepsini beraber yap çünkü testler
  uzun”): `CAD-REF-01` (birden çok belge, sekmeler), `02` (kapanan belgenin geç sonuçları), `03` (Python ve ajanın hedef belgesi),
  `04` (belgeler arası yapıştırmada ad çakışması), `12` (blok ve dış başvuru kırpma), `15` (kaynağı yerinde düzenleme), `17` (PDF
  altlığı), `18` (altlıktan alınan ölçünün doğruluğu).
- İlgili: ADR 0348 (dış başvurular), ADR 0349 (arama yolları), ADR 0350 (resim altlığı, paket), ADR 0309 (pano komutları), ADR 0137
  (ajan bağlantısı), ADR 0132 (Python konsolu), ADR 0315 (blokta katman kuralı), ADR 0263 (blok içi yol), ADR 0265 (yeni şemanın
  kuralı).

## Bağlam

Masaüstü bir anda tek çizim açabiliyordu: başka bir dosyayı açmak ekrandakini bırakmaktı. Dış başvurunun kaynağını düzenlemek için
çizimi kapatıp kaynağı açmak gerekiyordu. Python konsolu ve ajan bağlantısı “ekrandaki çizimde” çalışıyordu: kullanıcı başka bir
çizime geçseydi ajanın yazması yeni çizime giderdi. Pano yapıştırması aynı adlı bloğu ve stili hep hedef çizimdekiyle eşliyordu.
Blok ve dış başvuru yerleştirmesi kırpılamıyordu (AutoCAD'in XCLIP'i). PDF altlık olarak kullanılamıyordu.

## Karar

### 1. Kırp (`cad.inserts.clip`, `CAD-REF-12`)

Yerleştirmenin (`insert`) kırpma sınırı `clipping` alanıdır: `boundary` köşeleri **tanımın koordinatlarında** (yerleştirme
taşınınca, döndürülünce, ölçeklenince, aynalanınca sınır onunla gider), `inverted` (dışını göster), `frame` (`hide`: çizilmez,
`show`: yalnız ekranda, `plot`: içerikle çizilir ve yazdırılır). `.kcad` şema 85.

Komutun işlemleri: `set` (sınır çizimde verilir: `rectangle` iki köşeyle, eksenlere paralel, köşeleri sol alttan saat yönü tersine;
ya da `polygon`, kapanış köşesi düşer; her köşe yerleştirmenin yerleşiminin tersiyle tanıma taşınır: `x = M⁻¹(X − p) + taban`,
`M = R(dönüş)·S(ölçek, Y ölçeği)·ayna`), `remove`, `frame`, `invert`. Sınır en az 3, en çok 10 000 köşeli, alanı olan ve kendini
kesmeyen bir halkadır: iki kenar ortak köşeleri dışında buluşmaz, komşu kenar diğerinin üstüne geri katlanmaz; kesişen ilk çift
(önce en küçük ilk numara, sonra en küçük ikinci) söylenir. Kilitli katmandakiler atlanır ve sayılır (ADR 0278). `remove`, `frame`
ve `invert` kırpılmamış yerleştirmeyi geçer; hiçbiri kırpılmamışsa `not_clipped`. Sınır yerleştirmede çizilecek bir şey
bırakmıyorsa yazılır ve `clip_empty` uyarısı verilir. Tek adım “Kırp”.

İçerik çekirdekte kesilir (`kentos_geometry_core::block_clip`; `Blocks::expand` yerleştirmenin parçalarını yerleştirdikten sonra):
dolu alan alan olarak, çizgi, yay, açık yol ve kapalı eğrilerin sınırları yol olarak kesilir (ortaları sınırın içinde kalan
parçalar tutulur, `inverted`'da dışındakiler); yazı, nokta, tarama ve resim kutusunun ortasına göre ya bütün ya hiç. `plot`
çerçevesi bir parça daha olarak eklenir; `show` çerçevesi masaüstünün işaret katmanında çizilir (`Blocks::clip_ring`), yazdırılmaz.
İç içe bloğun kendi kırpması yok sayılır; Patlat kırpmayı uygulamaz (bütün içerik patlar).

Masaüstünde **Kırp** aracı (`tool.xclip`, CAD › Ekle › Dış başvuru; ikon: kesikli çerçevenin üstünde dolu çerçeve ve köşede kırpma
işareti): yerleştirmeye tıklanır (tek seçili yerleştirme hazır alınır), sınır Dikdörtgen (D) ya da Çokgen (Ç) çizilir, Ters (T),
Çerçeve (E: Gizli → Ekranda → Yazdır), Kaldır (K). Araç `cad.inserts.clip` ile yazar.

### 2. PDF altlığı (`CAD-REF-17`)

PDF dosyası bir dış başvurunun kaynağı olabilir: `cad.xrefs.edit`'in `attach`'ı `.pdf` yolu ve `page` (1'den; yoksa 1) alır.
Sayfa kendi okuyucumuzla okunur (`kentos_formats::pdf`; üçüncü taraf kütüphanesi yok, yalnız var olan `miniz_oxide`): çapraz
başvuru tablosu ve akışı (PNG öngörücüsüyle), `Prev` zinciri ve artımlı güncelleme, nesne akışları, bozuk adreste dosyanın
metninden nesneyi yeniden bulma; Flate, LZW, ASCIIHex, ASCII85, RunLength; sayfa ağacının miras aldıkları; içerik akışının yolları
(`m l c v y h re`, boyamaları), grafik durumu (`q Q cm w d gs`, gri, RGB, CMYK, ICC), form XObject'ler (matrisleri ve yerleştirildikleri
grafik durumuyla), yazılar (`Tf Tm Td TD T* Tj TJ ' "`, `Tz Ts Tr`; WinAnsi, MacRoman, Differences, ToUnicode haritası, Type0 iki
baytlık kodlar) ve isteğe bağlı içerik grupları (katmanlar; varsayılan görünümde kapalı olan gizli). Eğriler 0,01 pt içinde
düzleştirilir. Resimler, gölgelemeler ve harfe çevrilemeyen kodlar sayılır, çizilmez (`pdf_left_out` uyarısı). Şifreli dosya
reddedilir (`import_unreadable`).

Okunan sayfa kaynak çizime çevrilir (`kentos_native_application::pdf_underlay`): kâğıt metresi (1 pt = 0,0254/72 m), sayfanın
görünen sol alt köşesi başlangıç; beyaz olmayan dolgu alan, çizgi çizgi ya da çoklu çizgi (rengi, kâğıt kalınlığı mm, kesik
çizgisi), yazı yazı (yüksekliği, dönüşü); katmanlar “PDF” (bir gruba girmeyen varsa) ve sayfanın grupları. Sonrası dış başvurunun
kendisidir (ADR 0348): tanım, “Dış başvuru: ad” grubu, yeniden yükleme (içerikten türeyen kimliklerle aynı yerel kimlikler),
Yolu değiştir (PDF'e sayfa korunur, çizime sayfa düşer), Paketle. Yerleştirme, ölçek ve dönüş ekleme penceresinden; kırpma
`cad.inserts.clip` ile.

Altlığın görünüşü kaynağın `underlay`'idir: `fade` (%0–80, parçaların saydamlığı en az bu), `monochrome` (renkler ve katman
renkleri yerine yerleştirmenin rengi), `plot` (false: pafta ve PDF'te çizilmez; `kentos_plot::layers::plot_phasing_of`).
`cad.xrefs.edit`'in `underlay` işlemi yazar (`not_an_underlay`, `invalid_underlay`), tek adım “PDF altlığının görünüşü”. Kontrast
ayarı yoktur: altlık vektördür, resmi alınmaz; soldurma ve tek renk onun yerini tutar. Masaüstünde Dış başvuru ekle PDF seçilince
Sayfa alanını gösterir; Dış başvurular'da PDF altlığı seçilince Soldur (%), Tek renk ve Yazdır.

### 3. Belge sekmeleri (`CAD-REF-01`, `02`)

Birden çok çizim açık olabilir; biri öndedir (uygulamanın `document`'ı, bütün araçlar, paneller ve komutlar onu okur). Arkadaki
çizim sekmesinde kendine ait olanlarıyla bekler: belgesi (geri alma geçmişiyle), kamerası, seçimi ve görünüm geçmişi
(`doc_tabs.rs`). Öne gelen çizim açılan bir çizim gibi gösterilir (`show_document`: geometri deposu yeniden kurulur, çizimin
önbellekleri ve geç cevapları bırakılır), sonra kamerası, seçimi ve görünümleri geri verilir. Komutlar: **Yeni sekmede aç**
(`file.openTab`), **Sonraki çizim** (`file.nextTab`, Ctrl+Tab), **Çizimi kapat** (`file.closeTab`; kaydedilmemiş değişikliği varsa
sorar; sağındaki, yoksa solundaki sekme öne gelir). Görünüm › Çizimler paneli; iki ya da daha çok çizim açıkken çizimin üstünde
sekme şeridi (kaydedilmemiş noktası, ×, +). Aç ekrandaki çizimin yerine açar, olduğu gibi.

Sekme değişmez: Blok düzenleyici açıkken, konsolda kod çalışırken, bir çizim açılırken. Bulut projesi sekmede bekletilmez
(bağlantısı ekrandaki çizimi izler); kendi sekmesinde açılır. Kapanan ya da arkada bekleyen çizimin geç sonuçları kendi oturum
numarasıyla bulunur ya da bırakılır: kaydın bitişi arkadaki çizime yazılır, İşlemler'in ve Python'un sonuçları oturuma bakar.
Pencere kapanırken arkadaki sekmelerde kaydedilmemiş çizim varsa söylenir ve pencere kapanmaz.

### 4. Python ve ajanın hedef belgesi (`CAD-REF-03`) ve belgeler arası yapıştırma (`04`)

Ajan bağlantısının her çağrısı çizimini `document` ile adlandırabilir (hello'nun verdiği oturum numarası); `hello` açık çizimleri
(`documents`: numara, ad, yol, önde mi) sayar. Adlandırılan çizim arkadaysa çağrı onun üstünde, sekmesinde çalışır; kapandıysa
`document_closed`. `document` vermeyen çağrı öndeki çizimindir (eski istemciler). MCP'nin `desktop.attach`'ı bağlandığı çizimin
numarasını tutar (ya da `document` ile seçilir) ve her çağrıya ekler: kullanıcı başka sekmeye geçse de ajan kendi çiziminde kalır.
Python konsolunun çalıştırması başladığı çizimdedir; çalışırken sekme değişmez.

`cad.entities.paste`'in `conflicts`'i: `keep` (varsayılan; bu çizimin aynı adlı bloğu ve stili kullanılır) ya da `rename`: kopyanın
aynı adlı bloğu (blok anahtarıyla) ve yazı ve ölçü stili (katlanmış adla) ilk boş “Ad (2)”, “Ad (3)” … adıyla gelir; yeniden
adlandırılan blok yeni kimlik alır ve yerleştirmeleri (iç içe olanlar da) onu izler. `report.renamed` ve `names_renamed` uyarısı
(birim uyarısından önce). Birimler zaten metrede karşılaştırılır (`units_differ`, ADR 0309).

### 5. Kaynağı yerinde düzenleme (`CAD-REF-15`)

Dış başvurunun içeriği kaynağınındır: `cad.blocks.edit` dış başvuru tanımında içeriği değiştiren işlemleri (güncelle, yeniden tanımla,
taban noktası, öznitelikler, birim, eşitle, alanlar, kart, dinamik, kaldır) `xref_read_only` ile reddeder; ad değişir, kopyası ve
dışa aktarması alınır. Blok düzenleyici dış başvuruyu açmaz. Dış başvurular › **Kaynağı aç** kaynağı (arama kuralıyla bulunan
dosyayı) yeni sekmede açar: değişiklikler o dosyanındır, kendi geri almasıyla kaydedilir; bu çizim onları Yeniden yükle ile alır
(dosya değişince bildirim kartı söyler, ADR 0349 §3). PDF altlığının kaynağı düzenlenmez.

### 6. Altlıktan alınan ölçü (`CAD-REF-18`)

Kenet altlığın çizgilerine de olur (dönüşüm yerleştirmeninkidir: kenetin noktası parçanın yerleştirilmiş yeridir), ama sonuç kâğıttan
okunmuştur. `cad.snap.find` bir PDF altlığının parçasına düşen kenette `underlay_measure` uyarısını verir; `cad.dimensions.create`
altlığın parçasına bağlanan ölçüde aynı uyarıyı verir. Doğruluk kâğıdınkidir, kanıtlanmış CAD geometrisi diye sunulmaz.

Bu işte bulunan iki eski hata da düzeldi: blok parçasının yazısı, parçanın katmanı gizliyken de çiziliyordu (`store::labels`
artık `piece_shown`'a bakar); bağlı ölçünün blok içi yolu (`path`) yazmadan önceki denetimde yok sayılıyordu (`create.rs`'in
`placed_points`'i `blocks::place_point` ile).

### 7. Dosya

`.kcad` şema 85 (`FORMATS_VERSION` 92): yerleştirmenin `clipping`'i (JSON'u) ve dış başvurunun `page` ve `underlay`'i. Yazıcı 85'i
yalnız kırpılmış bir yerleştirme ya da PDF altlığı varken yazar; başka her çizim eskisiyle bayt bayt aynıdır. Tipli sütunlarda
yerleştirmenin dördüncü isteğe bağlı alanı.

## Sonuçlar

- Masaüstünde birden çok çizim açılır; ajan ve Python kendi çizimlerinde kalır.
- Blok ve dış başvuru kırpılır; PDF sayfası ölçekli altlık olur, soldurulur, tek renk ve yazdırılmaz yapılabilir.
- Dış başvurunun kaynağı ayrı sekmede düzenlenir, burada salt okunurdur.
- Web'de karşılıkları yoktur (ADR 0228); TODOS.md §0.2'de.

## Doğrulama

- Ortak durumlar KentOS kodu olmadan: `python3 scripts/fixtures/inserts_clip_cases.py --check` (39 durum; ters dönüşüm, kesişen
  kenarlar kesirlerle), `python3 scripts/fixtures/pdf_cases.py --check` (12 PDF ve 14 sayfa Python'un kendi modelinden; poppler'ın
  pdfinfo ve pdftotext'i dosyaları okur), `python3 scripts/fixtures/xref_command_cases.py --check` (PDF altlığının 12 durumu),
  `python3 scripts/fixtures/clipboard_command_cases.py --check` (ad yenileme), `python3 scripts/fixtures/blocks_command_cases.py
  --check` (salt okunur dış başvuru), `python3 scripts/fixtures/snap_find_cases.py --check` (altlıktaki kenet),
  `python3 scripts/fixtures/kcad_v2_reference.py --check` (şema 85'in örneği ve 9 bozuk dosyası).
- Rust: `cargo test -p kentos-formats --test all pdf`, `cargo test -p kentos-native-application --test all xrefs::`,
  `cargo test -p kentos-desktop -- doc_tabs:: an_agent_stays`.
- Resimler: `KENTOS_SHOTS_ONLY=kirp,kirp-ters,pdf-ekle,pdf-altlik,pdf-altlik-pencere,belge-sekmeleri cargo test -p kentos-desktop
  tools_screens -- --ignored --nocapture`.
