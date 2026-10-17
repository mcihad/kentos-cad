# ADR 0327: Tablonun biçimi, formülleri ve alanları; dosyaya bağlı çizelge; açıklamaların DXF biçimleri

- **Durum:** kabul edildi (2026-10-17). [plan/cad.md](../../plan/cad.md) §11 `CAD-ANNO-28`, `29`, `30` (sahibin sözü: “Bitir o
  zaman”; 3–4 maddelik gruplar). Kapsamı ben belirledim. Bu grupla plan/cad.md §11 (CAD-ANNO) biter.
- **Bağlam belgesi:** ADR 0184 (tablo), 0223 (tablo dosyaları), 0325 (yazının alanları), 0317 (blok çizelgesi), 0147 ve 0205
  (ölçünün DXF'i), 0146 ve 0326 (kılavuz), 0253 (komut protokolü).
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

Tablo (ADR 0184) tek satırlık hücrelerden, birleşik alanlardan, sütun hizasından ve bir başlık satırından oluşuyordu. Mimari
ve harita çiziminde tablolar hesap taşır (malzeme listesinin tutarı, alan çizelgesinin toplamı), bir hücre çizimdeki bir sayıyı
gösterir (kapı sayısı), tablo bir Excel dosyasından gelir ve dosya değişince güncellenir. AutoCAD bunu tablonun hücre
biçimiyle, formüllerle, alanlarla ve veri bağlantısıyla (DATALINK) yapar. Tabloyu güncelle bir dosyayı yeniden seçtiriyordu;
elle düzeltilen hücre dosyanınkiyle eziliyordu. DXF'e açıklamalar tek biçimde gidiyordu: başka programda düzenlenebilsin mi,
KentOS'un çizdiği gibi mi görünsün, seçilemiyordu.

## Karar

### 1. Hücre biçimi (`styles`)

Tablonun yeni `styles`'ı sıralı bir listedir: her biçim bir aralık (`row`, `col`, `rows`, `cols`) ve isteğe bağlı `align`
(sol, orta, sağ), `bold`, `color` ve `fill` (`#rrggbb`) ile `decimals` (en çok 10). Bir hücrenin biçimi, aralığı onu tutan
biçimlerin sırayla birleşimidir: sonraki öncekinin verdiği alanı ezer (`TableEntity::cell_style`). Hiza ve kalınlık yerleşimin
(`geom::table`: hizalı yer, kalın yazı tipiyle genişlik, Yazıya sığdır kalını ölçer), renk ve dolgu çizimin işidir: dolgu
hücrenin dikdörtgeni olarak yazıların ve çizgilerin altına, koyu zeminde rengi koyulaştırılarak (`fill_cell`); yazının rengi
nesnenin rengi yerine. `decimals` yalnız hesaplanan değerlere uygulanır (formül, alan); yazılmış sayı yazıldığı gibi kalır:
gösterim kaynağı değiştirmez. Hiçbir şey vermeyen biçim, tablonun dışındaki aralık ve renk olmayan renk reddedilir. Biçimler
satır ve sütun eklenip silinince kayar, daralır ya da düşer.

### 2. Formüller (`formulas`)

`=` ile başlayan hücre bir formüldür; tablo onu `formulas`'ta (`row`, `col`, `formula`) tutar, hücrenin yazısı formülün
sonucudur (önbellek). Dil (`kentos_contracts::table_formula`): sayılar, A1 başvuruları (`$` sabitlemesiyle), aralıklar
(`A1:C3`), `+ - * / ^`, ayraçlar ve yedi işlev, İngilizce ve Türkçe adlarıyla: SUM TOPLA, AVERAGE ORTALAMA, MIN, MAX MAK,
COUNT SAY, ROUND YUVARLA, ABS MUTLAK; bağımsız değişkenler `,` ya da `;` ile. Bir hücrenin sayısı yazısının sayı olarak
okunuşudur (nokta ondalık); boş hücre aritmetikte 0, işlevlerde atlanır; yazı aritmetikte `#DEĞER!`, aralıkta atlanır (elektronik
tablolar gibi). Hatalar: `#AD?` (okunamayan), `#BAŞV!` (tablonun dışı ya da silinmiş), `#DEĞER!`, `#SAYI/0!`, `#SAYI!` (sonlu
olmayan), `#DÖNGÜ!` (birbirini okuyan formüller). Sonuç, elektronik tabloların gösterdiği gibi 15 anlamlı basamakla yazılır:
en kısa yazımı (üssüz) sıfırdan uzağa yarımla 15 anlamlı basamağa yuvarlanır, sondaki sıfırlar düşer (`0.1+0.2` `0.3`'tür,
`0.30000000000000004` değil; 17 Ekim eki, sahibin bildirimi); hesap tam değerle sürer. Biçimin basamağı varsa bu yazımdan
sıfırdan uzağa yarımla yuvarlanır; YUVARLA da aynı kuralla. Hesap derinliğine bakmayan yinelemeli bir sırayla yapılır,
döngüdeki her hücre `#DÖNGÜ!` olur. Satır ve sütun eklenip silinince başvurular kayar; silinen hücreyi gösteren başvuru `#BAŞV!`,
silinen satırları içine alan aralık daralır (`shift`). Birleşik alanın içinde formül olmaz; formül alanın sol üst hücresine yazılır.

### 3. Hücre alanları (`fields`)

Bir hücre ADR 0325'in kaynaklarından birine bağlanabilir (`fields`: `row`, `col`, `source`, `format`, `broken`): nesnenin
alanı, uzunluğu, katmanı, özniteliği … ya da projenin adı, ölçeği, değişkeni. Projenin değerlerine **Blok sayısı** eklenir
(`blockCount`, bloğun adıyla): çizimdeki yerleştirmelerinin sayısı (bloğun içindekiler değil). Değer hücrenin yazısıdır; kaynağı
gidince hücre `####` olur ve `broken` nedenini söyler. Bir hücre hem formül hem alan olamaz; formül alanın değerini okuyabilir
(“=B2*C2”).

### 4. Hesaplanan tablolar ve blok çizelgesi

Belge her adımı kaydetmeden önce (bağlı yazılar ve yazının alanları gibi, ADR 0175 ve 0325), hesaplanan hücresi olan tabloları
yeniden yazar (`kentos_domain::table_cells::follow_tables`): önce blok çizelgesinin satırları, sonra alanların değerleri, sonra
formüller. Yalnız adımın değiştirdiği tablolar ile alanı ya da çizelgesi çizimi okuyanlar bakılır; değişmeyen yazılmaz. Belgenin
deposu bunları bir dizinde tutar (`computed`): tabloya dokunmayan adım hiçbir tabloyu gezmez.

Blok çizelgesi (`table.blockSchedule`, “Blok çizelgesi tablosu”): kaynağı `blocks` olan tablo (`names`: sayılan bloklar, yoksa
yerleştirilen her blok; `by`: bir özniteliğin değerine göre satırlar). Satırları Blok, [öznitelik], Adet ve sonda Toplam'dır
(ADR 0317'nin çizelgesiyle aynı sayım); kapı silinince çizelge ve kapı sayısını gösteren alan aynı adımda değişir. Tabloyu
güncelle onu `live` söyler.

### 5. Dosyaya bağlı tablo (`CAD-ANNO-29`)

Dosyadan gelen tablonun kaynağı artık dosyayı tutar: `path` (çizimin klasörüne göre ya da mutlak), `digest` (son okunan
baytların SHA-256'sı), `copy` (o baytların verdiği hücreler: gömülü kopya) ve `overrides`: elle değişen hücreyle ne yapılacağı.
Bir hücre, dosyanın kopyasındakinden farklıysa elle değişmiştir (formülün ve alanın değil). Tabloyu güncelle:

- dosya aynı baytlarsa `unchanged`, tablo yazılmaz;
- değiştiyse dosyanın hücreleri yazılır; `keep`'te (varsayılan) elle değişen hücre, dosyada o hücre değişmedikçe korunur,
  `replace`'te dosyanınki yazılır; rapor korunan ve üzerine yazılan hücreleri sayar (`kept`, `replaced`);
- dosya yolunda değilse `missing`, okunamıyorsa `unreadable` (uyarı; tablo olduğu gibi kalır); bağı olmayan dosya tablosu
  `unlinked`. Masaüstü bu ikisinde dosyayı sorar ve onu bağlar.

Dosyaya bağla (`table.link`): seçili tabloyu dosyaya bağlar ve hücrelerini ondan yazar. Kaynağı bırak (`unlink`) tabloyu elle
yazılmış yapar. Biçimler, formüller ve alanlar güncellemede yeni satır ve sütunların içinde kalır.

### 6. Komut: `cad.tables.edit` v1

Tabloyu düzenle, güncelle, bağla ve bırak tek anlamsal komuttur (ADR 0250, 0253): `edit` (`setCell` — `=` ile formül —,
`insertRows`, `deleteRows`, `insertColumns`, `deleteColumns`, `merge`, `unmerge`, `style`, `clearStyle`, `field`, `unfield`,
`rowHeight`, `columnWidth`, `fit`, `columnAlign`, `header`, `grid`, `frame`, `unlink`; sırayla, tek adım), `refresh`
(kimlikler, `base` klasör), `link` (`path`, `sheet`, `base`, `overrides`) ve `unlink`. Hücreler ve birleşik alanlar
çekirdeğin kurallarıyla (`ops::table_edit`) değişir, biçimler, formüllerin başvuruları ve alanlar satır ve sütunlarla kayar.
Retler: `invalid_uid`, `entity_not_found`, `not_a_table`, `layer_locked`, `no_entities`, `cell_out_of_range`, `invalid_table`,
`not_finite`, `no_source`, `source_missing`, `source_unreadable` (yeni kodlar `cell_out_of_range`, `no_source`; `source_missing`
ve `source_unreadable` bağlamada ret, güncellemede uyarıdır, `source_unlinked` uyarı). Tabloyu düzenle penceresi değişikliklerini bu komutla önizler
(`plan`) ve yazar; ızgara hücrenin gösterilen değerini, biçimin dolgu, renk, kalınlık ve hizasını gösterir, formül çubuğu
formülü. Pencerenin ikinci araç satırı: Kalın, Yazı rengi ▾, Dolgu ▾, Basamak ▾, Biçimi temizle, Alan ▾ (Blok sayısı),
Alanı kaldır. Izgaranın yüksekliği pencereye göre (1100×650'de düğmeler pencerede). Python `kentos.cad.tables.edit`, MCP aracı
(yıkıcı listede), manifestte `table.edit`, `table.update`, `table.link` anlamsal.

### 7. KCAD şema 68

Tablonun `styles`, `formulas`, `fields`'ı (JSON'larıyla), dosya kaynağının yanında `link`'i (`path`, `digest`, `copy`,
`overrides`; `keep` yazılmaz) ve blok çizelgesinin kaynağı olarak `schedule`'ı (eski okuyucunun `source`'u bilinmeyen türle
okumaması için ayrı alan). Yazıcı 68'i yalnız bunlardan biri varken yazar; şema 2–67 yükünde bilinmeyen alandır. Sütunlarda
tablonun 12. seçeneği; çizelgede 10. seçenek yok. Bağımsız Python yazıcısı ve okuyucusu, örnek `table-cells.kcad`, on üç bozuk
dosya (docs/specs/kcad-v2.md “Şema 68”). `FORMATS_VERSION` 77. Çizim hattının paketinde biçimli tablo 25. türdür (18'inki ve
biçimlerin JSON'u).

### 8. Açıklamaların DXF biçimleri (`CAD-ANNO-30`)

DXF dışa aktar'da Açıklamalar: **Ölçüler** ve **Kılavuzlar** her biri Yerel nesne ya da Blok görünüşü; **KentOS verisini
yaz** (açık başlar). `DxfWriteInput.annotations` ve `file.export.dxf`'in `annotations`'ı (`dimensions`, `leaders`,
`metadata`):

- Yerel nesne: ölçü DIMENSION, kılavuz LEADER ve notu MTEXT; başka programlar düzenler.
- Blok görünüşü: KentOS'un çizdiği gibi adsız blok (`*U`) ve onu başlangıçta yerleştiren bir INSERT; her programda aynı
  görünür, düzenlenemez. KentOS verisi açıkken INSERT nesnenin JSON'unu taşır ve KentOS'a ölçü ya da kılavuz olarak geri okunur.
- KentOS verisi kapalıyken tablonun, ölçünün ve kılavuzun KentOS verisi yazılmaz: başka programın görmediği veri dosyaya
  girmez; KentOS'a DXF'in taşıdığı kadarıyla gelir (blok biçimi blok olarak).
- Tablo her zaman blok görünüşüyle yazılır. AutoCAD'in ACAD_TABLE'ı ve MULTILEADER'ı yazılmaz: ikisinin de nesne sözlüğü ve
  bağlam verisi geniştir ve elimizde onları doğrulayacak bir AutoCAD yok (DXF başvurusu ve ezdxf ikisini farklı okur); yanlış
  yazılmış bir ACAD_TABLE dosyayı AutoCAD'de açılmaz yapar. Okuma ADR 0184 ve 0326'daki gibidir.

Pencerenin raporu seçilen biçimi ve verinin yazılıp yazılmadığını söyler (“… adsız blok (*U) olarak yazılır … KentOS'a blok
olarak geri okunur.”).

## Doğrulama

- `python3 scripts/fixtures/table_command_cases.py --check`: `fixtures/commands/v1/cad.tables.edit.json`'ın 45 durumu (hücreler,
  formüller ve hataları, YUVARLA'nın yarımı, başvuruların kayması, biçimler ve Yazıya sığdır, alanlar ve blok sayısı, silinen
  kapının aynı adımda izlenmesi, bağlı dosyanın üç güncellemesi ve iki kuralı, bağlama ve bırakma, retler) KentOS kodu olmadan,
  formül dili ve yuvarlama başvurunun kendi yazdığı hesapla; masaüstünün işleyicisi ve yetenek testi (`capabilities.rs`) aynı
  durumları geçer.
- `python3 scripts/fixtures/kcad_v2_reference.py --check`: şema 68'in örneği ve bozuk dosyaları bağımsız okuyucuyla; Rust
  kodeği aynı dosyaları okur ve aynı baytları yazar (`cargo test -p kentos-kcad`).
- `cargo test -p kentos-formats --test all dxf_write`: açıklamaların dört biçiminin yazılıp KentOS'a geri okunması
  (`annotations_come_back_by_the_form_and_data_they_were_written_with`); `dxf_write_reference.py --check` öbür örneklerin
  baytlarının değişmediğini denetler.
- Süre (`cargo test --release -p kentos-native-application --test all tables_cmd::timing -- --ignored --nocapture`, bu makinede):
  100 000 parselli çizimde 1001 satırlık, 1001 formüllü (sonuncusu 1000 hücrenin TOPLA'sı) tabloda bir hücrenin düzenlenmesi ve
  bütün formüllerin hesabı 3,6 ms, bütün tabloya biçim 3,2 ms, tabloya dokunmayan bir silme adımı 0,02 ms.
- Resimler: `KENTOS_SHOTS_ONLY=tablo-formul,tablo-baglanti,dxf-bicimler cargo test -p kentos-desktop tools_screens -- --ignored
  --nocapture` (`.run/shots/arac-tablo-formul-*`, `-baglanti-*`, `arac-dxf-bicimler-*`). Resim düzeni artık pencerenin boyunu
  uygulamaya da söyler (`app.window_size`): pencereye göre boyu değişen alanlar 1100×650'de gerçek boylarıyla çizilir.

## Sonuçlar

- Web'in paketleyicisi 25. türü, `cad.tables.edit`'in TypeScript işleyicisi, şema 68'in TS okuyucusu ve DXF penceresinin
  Açıklamalar'ı TODOS.md §0.2'de.
- ACAD_TABLE ve MULTILEADER yazımı bir AutoCAD ile sınanabildiğinde ayrı bir karar olur.
- Tablonun çok satırlı hücresi, hücrede blok ve hücreler arası tablolar (başka tabloya başvuru) kapsam dışıdır.
