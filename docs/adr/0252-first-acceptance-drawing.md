# ADR 0252: İlk uçtan uca kabul çizimi; pafta işlemleri katalogda, DXF ve PDF başsız sunucuda

- **Durum:** kabul edildi (2026-10-13). [plan/cad.md](../../plan/cad.md) §24.1'in on ikinci işi (TODOS.md `CADP-12`; plan §23.1
  `K01`–`K04`, `K11`, `K12`; `CAD-SHEET-06` ve `07`'nin ilk dilimi). Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0130 (başsız komut sunucusu), 0133 (MCP), 0137 (ajan bağlantısı), 0164 (pafta düzeni), 0245 (Python ve
  MCP'de çizimin birimi), 0249 (ilişkili ölçü), 0250 (anlamsal Buda, Uzat, Köşe yuvarla, Pah), 0251 (pafta kitabı projenin dosyasında).
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

Planın §23.1'i bir çizimin baştan sona her yoldan aynı sonucu vermesini ister: arayüzde fareyle ve klavyeyle, yazılı komutla,
Python'dan ve bir ajanın MCP'sinden. İlk kabul çizimi bir odadır:

- K01: yerel, milimetre CAD projesinde 4000 × 3000 mm kapalı alan; 12 m², 14 m; kaydet ve aç.
- K02: odanın çevresinde 200 mm duvar; eksenler 100 mm ötelenir, köşeler birleşir, taşan uçlar budanır.
- K03: alt kenara ilişkili ölçü; köşe 4500'e çekilince ölçü izler, tek geri alma ikisini birlikte döndürür.
- K04: ölçülen kenara köşe eklenir; bağ kaymaz.
- İlişkili tarama: oda değişince izler.
- K11: 5000 mm kaynak çizgi 1:50 A3 paftada 100 mm.
- K12: iki pafta kaydedilir; başka ortamda açılınca kitap, harita, antet eksiksiz.
- PDF ve DXF.

Başlarken eksikler:

- Python'un ve MCP'nin DXF'i yoktu; masaüstünün DXF yazıcısının girdisini kabuk kuruyordu.
- Pafta işlemleri katalogda değildi (`CAD-SHEET-06`): betik pafta ekleyemiyordu.
- Paftanın PDF'i masaüstünün kabuğundaydı: haritaların vektörleri, stil motorunun katman kurulumu, tabloların ve lejantların girdileri,
  yazı tiplerinin baytları.
- Köşe yuvarla'nın yarıçap 0 ile iki çizgiyi birleştirmesi çizgilerden birinin yönünü çeviriyordu.

## Karar

### 1. Ortak senaryo: `fixtures/acceptance/v1/room.json`

- Senaryo veridir (`kentos.acceptance` v1). Adımları:
  - `command` ve `input`: katalogun komutu; `$ad` yer tutucularıyla, `as` sonucun kimliğini (ya da `created` listesini) adlar;
  - `undo`: geri alınan adımın adı;
  - `reopen`: kaydet ve aç;
  - `export`: `dxf` ya da `pdf`.
- Beklentiler:
  - `objects`: nesnelerin alanlarının alt kümesi;
  - `measures`: alan ve uzunluk;
  - `count`: nesne sayısı;
  - `sheets`: kitabın paftaları;
  - `dxf`, `pdf`: dosyanın gerçekleri (aşağıda).
- Beklenen değerleri `scripts/fixtures/acceptance_room.py` KentOS kodu olmadan hesaplar. Aynı betik izin çizimini
  (`fixtures/interaction/v1/acceptance-room.kcad`) de yazar. Hoşgörü 1e-9.
- Dört koşucu aynı dosyayı oynatır:
  - Python: `python/tests/test_acceptance.py`, `kentos.cad.Document` ile.
  - MCP: `crates/native/mcp/tests/acceptance.rs`, `Server::handle`'ın `tools/call`'larıyla.
  - Masaüstü: `apps/desktop/src/acceptance_tests.rs`:
    - komutlar ajan bağlantısıyla (`desktop.attach`'in yolu, “Ajan: …” satırlarıyla);
    - geri alma Ctrl+Z'nin `undo`'suyla;
    - kayıt ve açılış uygulamanın kendisininkiyle;
    - DXF Dışa aktar › DXF penceresinden;
    - PDF pafta kipinin Dışa aktar › PDF'inden.
  - Arayüz izi: `fixtures/interaction/v1/acceptance-room.json`. Aynı oda fareyle ve klavyeyle çizilir: komut satırına yazılan
    koordinatlar, kenetle bağlanan ölçüler, tutamaçlar, Ctrl+Z, Ötele, Köşe yuvarla, Buda, kaydet ve aç. Resimleri
    `kentos-cad kullan acceptance-room`.

### 2. `cad.sheets.edit` v1: pafta çekirdeğinin işlemleri katalogda

- Girdi:
  - `ops`: pafta çekirdeğinin işlemleri (`kentos.sheet/1` `Op`, `op` ile adlanan nesneler); pafta tasarımcısının yaptıklarının
    aynısı;
  - `expectedRevision`.
- İşlemlerin biçimi:
  - Sözleşmede işlemler opak JSON'dur: pafta çekirdeği sözleşmelere bağlıdır, sözleşme onun türlerini anamaz.
  - Kitabı nasıl opak tutuyorsa (ADR 0251) işlemleri de öyle tutar.
  - Çekirdek okur, denetler ve uygular (`kentos_sheet::ops::apply`).
- Uygulama:
  - İşlemler sırayla, tek değişiklik olarak uygulanır.
  - Bir ret hiçbir şey yazdırmaz. Çekirdeğin kodu `ops[i]` yoluyla döner.
  - Boş kalan kitap kitap değildir.
- Geri alma:
  - Kitap kâğıdındır (ADR 0251 §3): değişiklik çizimi kirli yapar, çizimin geri alma adımı değildir (`undo: none`).
  - Cevabın `inverse`'ü değişikliği geri alan işlemlerdir; bu komuta yeniden verilir.
  - Pafta tasarımcısının kendi Geri al'ı aynı terstir.
- Cevap kitabın paftalarını sayar: kimlik, ad, sayfanın milimetre boyu, öğe sayısı, haritalar ve ölçekleri (`SheetInfo`). Aynı
  liste başsız sunucunun özetinde `sheets`'tir: Python `Document.sheets()`, MCP `drawing.summary`, masaüstünün ajan bağlantısı.
- Masaüstünde ajanın değiştirdiği kitabı pafta kipi gösterir (`sheet_book.rs`'in göstericisi kitabı göstericiyle karşılaştırır).
- Ortak durumlar: `fixtures/commands/v1/cad.sheets.edit.json`, 20 durum, `scripts/fixtures/sheets_edit_command_cases.py` ile.
- MCP'de yıkıcı araçtır: işlemleri pafta ve öğe siler.
- Manifestte (ADR 0244) pafta kipinin kitabı değiştiren komutları artık `semantic`, `cad.sheets.edit` ile:
  - görünüm ve seçim `session`;
  - dosyalar ve pencereler `host`;
  - ön denetim `query`;
  - CAD tarafının boşlukları 214'ten 156'ya iner.

### 3. Kâğıt üstünde çizim: `kentos-plot`

- Yeni saf crate `crates/native/plot` (`deps.mjs` grubu `plot`). Bağımlılıkları: paylaşılan çekirdek, belge, uygulama, araç oturumu,
  proje ve stil motoru; Iced, wgpu, pencere yok.
- Masaüstünün kabuğundan oraya taşındı:
  - `layers`: katmanın stil motorundan bütün olarak kurulması, gösterilen katmanlar, `Look`; masaüstünün sahnesi önbelleğini bunun
    üstünde tutar;
  - `vectors`: motorun partilerinin PDF'in yollarına çevrilmesi (eski `map_vectors.rs`);
  - `maps`: bir haritanın içeriği; `MapLabel` renklerini `#rrggbb` olarak taşır;
  - `inputs`: paftanın çizimden okudukları (eski `sheet_inputs.rs`);
  - `fonts`: çizim yazı tiplerinin baytları; masaüstü aynılarını Iced'e yükler.
- İşaret şekillerinin yolları (`shape_path`, `shape_outline`) çizim hattından stil motoruna geçti (`kentos_native_style::shapes`):
  atlas da PDF de aynı kuraldan.
- `book::book_pdf` kitabı penceresiz PDF yazar:
  - masaüstünün Dışa aktar › PDF'inin girdileri: proje, sistemi, tarih, kullanıcı, tablolar, koordinat listeleri, lejantlar;
  - aynı display listeleri;
  - her yerleşik harita gösterilen katmanların ölçeğindeki yollarıyla;
  - TM sistemde GeoPDF.
- Başsız ortamın yapamadıkları söylenir, uydurulmaz (cevabın `notes`'u):
  - Haritaların yazıları başsız çıktıda henüz yoktur: masaüstünün yazı düzeni Iced'in metin sistemiyle ölçer. Ortak ölçümle yazı
    düzeni ayrı iştir (TODOS.md).
  - Vektörle yazılamayan harita (desenli ya da resimli dolgu, resimli işaret) boş harita kutusudur; masaüstü onu resim olarak yazar.
  - SVG resmi çekirdeğin kendi bulgusuyla (`svg_not_in_pdf`) boş kutudur.
- Masaüstünde dosya, yazdığı anın projesini alır: Kaydet ve Yazdır, yazmadan önce paftaya projenin bağlamını verir (ad, tarih, sistem).
  Pafta kipi çizimi izlememiş olsa da (ajan kitabı değiştirmiş olabilir) antet boş kalmaz.

### 4. DXF başsız sunucuda

- DXF yazıcısının girdisini kuran kural `kentos_headless::export`'ta: katmanlar yollarıyla, nesneler ağacın sırasıyla, kendi yazısı
  olmayan ölçünün değeri görünüşüyle, bloklar, yerel projenin birimi, stiller. Masaüstünün DXF dışa aktarı da bunu kullanır.
- Sayıların biçimlendirmesi (`Format`) bunun için araç oturumundan proje modeline taşındı (`kentos_project::format`); araç
  oturumu onu yeniden dışa aktarır.
- Yüzler:
  - `Session::export_dxf`;
  - Python `Document.export_dxf` (`Exported`: yol, bayt, nesne, katman, yazıcının raporu);
  - MCP `drawing.exportDxf`.
  - Masaüstünün açık çizimini ise kullanıcı dışa aktarır (`desktop_user`).

### 5. PDF başsız sunucuda

- Yüzler:
  - `Session::export_pdf(yol, paftalar)`;
  - Python `Document.export_pdf` (`Printed`: yol, bayt, paftalar, bulgular, notlar);
  - MCP `drawing.exportPdf`.
- Dosya geçici adla yazılır, geri okunmadan yerine konmaz (`files::write_bytes`, `.kcad`'in kuralı).

### 6. Köşe yuvarla'nın iki çizgisi yönlerini korur

- `cad.entities.fillet` ve `.chamfer`'ın `lines` köşesi çizgileri kısaltır ya da uzatır, yönlerini çevirmez.
- Kesilen uç değişir, öbür uç yerinde kalır. Önce bir çizgi, uç köşeye taşınınca baştan sona çevriliyordu.
- Bağımsız başvuru (`edge_command_cases.py`'nin `along`'u), ortak durumlar ve `corner-tools` izi buna göre.

### 7. Dosyaların bağımsız okunması

- Koşucular dosyaları KentOS kodu olmadan okur:
  - Python `python/tests/acceptance_files.py`;
  - Rust ikizi `crates/native/plot/tests/support/files.rs`, MCP'nin ve masaüstünün testleri `#[path]`'le kullanır.
- DXF: grup kodları.
  - `$INSUNITS` 4.
  - Türlerin sayıları.
  - Odanın kapalı LWPOLYLINE'ı milimetrede.
  - On üç LINE uçlarıyla sırasıyla.
  - İki ölçünün ölçümü 4000.
  - Taramanın açısı 45°, aralığı 100 mm ve sınırı.
  - Hoşgörü 1e-6 mm.
- PDF:
  - Flate akışları açılır.
  - Sayfalar `/Kids` sırasıyla.
  - `MediaBox` milimetrede.
  - İçerik akışının çizilen düz parçaları `cm`'yi izleyerek.
- PDF'in gerçekleri:
  - Her sayfa 420 × 297 mm.
  - 5000 mm'lik çizgi haritasının koyduğu yerde: 1:50'de (165, 122)–(265, 122) mm, 100 mm; 1:100'de 50 mm. Hoşgörü 0,01 mm.
- Antetin sözcükleri (proje, pafta, ölçek) poppler'ın `pdftotext`'iyle okunur; yoksa söylenir ve atlanır.
- Masaüstü 1:50 sayfasında ölçünün değerini de arar.

## Uygulama

- Senaryo ve başvurular:
  - `scripts/fixtures/acceptance_room.py`, `fixtures/acceptance/v1/room.json`;
  - `fixtures/interaction/v1/acceptance-room.{json,kcad}`;
  - `scripts/fixtures/sheets_edit_command_cases.py`, `fixtures/commands/v1/cad.sheets.edit.json`.
- Sözleşme: `crates/shared/contracts/src/cad_sheets.rs` (`SheetsEdit`, `SheetsEdited`, `SheetInfo`, `SheetMapInfo`); katalog.
- Uygulama: `crates/native/application/src/sheets_edit.rs` (`sheets_of`, `sheets_in`); kodlar `no_operations`,
  `invalid_operation`, `book_unreadable`.
- Başsız sunucu: `export.rs`, `Session::export_dxf`, `Session::export_pdf`, özetin `sheets`'i, `files::write_bytes`.
- Çizim: `crates/native/plot` (`book`, `fonts`, `inputs`, `layers`, `maps`, `vectors`); `kentos_native_style::shapes`;
  `kentos_project::format`; `kentos_sheet::text::fold`.
- Python: `Document.export_dxf`, `.export_pdf`, `.sheets`, `Exported`, `Printed`; `kentos.cad.sheets.edit`.
- MCP: `drawing.exportDxf`, `drawing.exportPdf`.
- Masaüstü:
  - `exchange/dxf_export.rs` ortak kuralla;
  - `sheet_pdf.rs` ve `sheets.rs` `kentos-plot` ile;
  - `labels.rs`'in `MapLabel`'ı çekirdeğinki;
  - Kaydet ve Yazdır paftaya projeyi verir.

## Doğrulama

- Python koşucusu:

  ```bash
  .run/py/bin/python -m unittest discover -s python/tests -p "test_acceptance.py"
  ```

- MCP koşucusu: `cargo test -p kentos-mcp --test acceptance`.
- Masaüstü koşucusu ve dosyaları: `KENTOS_ACCEPTANCE_OUT=.run/shots/kabul cargo test -p kentos-desktop acceptance_tests`.
- Arayüz izi ve resimleri:
  - `KENTOS_TRACES_ONLY=acceptance-room cargo test -p kentos-desktop every_trace_passes`;
  - `./target/debug/kentos-cad kullan acceptance-room`.
- Çizimin kendi testi: `cargo test -p kentos-plot`. Başsız kitap PDF'i senaryonun paftalarıyla bağımsız okunur.
- `cad.sheets.edit` iki koşucuda:
  - `python3 scripts/fixtures/sheets_edit_command_cases.py --check`;
  - `cargo test -p kentos-native-application`;
  - Python'un `test_command_cases.py`'si.
- `python3 scripts/fixtures/acceptance_room.py --check`, `python3 scripts/fixtures/edge_command_cases.py --check`,
  `python3 scripts/cad/manifest.py --check`, `pnpm arch:deps`.
- Okuyucuların yanlışı yakaladığı denendi: sayfadaki çizginin ucu 0,05 mm kaydırılınca, DXF'in çizgisi 0,01 mm ve ölçümü 4500
  yazılınca düşer.

## Sonuçlar

- Oda dört yoldan aynı sonucu veriyor: aynı koordinatlar, ölçüler, kitap, DXF ve PDF gerçekleri.
- Betik ve ajan pafta ekleyip PDF ve DXF yazabiliyor.
- Açık kalanlar:
  - Haritaların yazıları başsız PDF'te yok: ortak ölçümle yazı düzeni (`CAD-SHEET-06`'nın kalanı).
  - Başsız PDF'te vektörü olmayan haritanın resmi ve SVG resimler (çizim hattı ister).
  - Pafta işlemlerinin kullanışlı sarmalayıcıları (şablondan pafta, haritayı nesneye sığdır): bugün işlemler çekirdeğin tam JSON'uyla
    verilir (`CAD-SHEET-07`).
  - 1:100'de 2,5 mm'lik yazının 1,25 mm'ye inmesi (ölçeğe göre açıklama, K10).
  - Uygulama içi yapay zekâ panelinin yolu `CADP-10`'la eklenir.
