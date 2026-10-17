# ADR 0325: Alanlar (yazıdaki bağlı değerler) ve yazının komutları

- **Durum:** kabul edildi (2026-10-17). [plan/cad.md](../../plan/cad.md) §11 `CAD-ANNO-22`, `23`, `24` (sahibin sözü: “Anno
  bitene kadar devam et”, “Bitir o zaman”; 3–4 maddelik gruplar). Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0175 (nesneye bağlı yazı), 0182 (çok satırlı yazı), 0145 (Bul ve değiştir, Artır), 0214 (`@` değerleri),
  0317 (blok öznitelikleri), 0324 (yazının düzeni).
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

Bir mahal etiketi “Salon 24,35 m²” yazar; oda büyüyünce elle düzeltilir. AutoCAD bunu alanlarla (FIELD) çözer: yazının bir parçası
bir kaynağa bağlıdır, kaynak değişince değeri yenilenir. KentOS'ta bağlı yazı (ADR 0175) bütün yazıyı bir nesnenin etiketiyle
yazar; bir yazının içindeki tek bir değer bağlanamıyordu. Yazının komutları da araçlardaydı: Bul ve değiştir `cad.entities.edit`'in
hazır yazıyla `replaceText`'i, numaralama, birleştirme ve satırlara ayırma yoktu.

## Karar

### 1. Alan (`CAD-ANNO-22`)

Yazının yeni `fields`'ı (`kentos_contracts::TextField`, çok satırlı yazının alanlarıyla birlikte `Paragraph`'ta): her biri
yazının harflerinden bir aralık (`start`, `end`; Unicode skaler değerleri, biçim dilimleri gibi), sırayla ve ayrık; o aralığın
harfleri alanın **son yazılan değeridir** (önbellek). Kaynağı tiplidir; serbest ifade değildir: belge onu her adımda ifade motorunu
ve başka katmanlara bakan işlevleri bilmeden yeniden hesaplar.

- `object`: kalıcı kimliğiyle bir nesne ve değeri: `area` (alan, delikler düşülmüş), `perimeter` (kapalı nesnenin çevresi),
  `length` (çizginin, yayın, yolun, eğrinin uzunluğu), `layer` (katmanının adı), `label` (etiketi), `attribute` (`name` adlı
  özniteliği; bloğun yerleştirmesinde öznitelik değeri), `text` (bir yazının harfleri: çapraz başvuru), `x`, `y` (noktanın,
  yazının, bloğun, tablonun yerleştirme noktası, ötekilerde etiketinin yeri).
- `project`: `name` (projenin adı), `scale` (çizim ölçeği, `1:500`), `unit` (çizim birimi), `crs` (`EPSG:5254`, tanımın adı ya da
  `Yerel`), `variable` (`name` adlı proje değişkeni, ADR 0214 §4).

Tarih ve saat kaynağı **yoktur**: değeri her açılışta değişirdi, belge ve dosya aynı girdiyle aynı sonucu vermezdi. Paftanın adı,
numarası ve sayfası paftanın kendi ifadelerindedir (`@pafta_adi`, `@sayfa`, `@proje_adi`; pafta çekirdeği): çizimin yazısı
paftayı bilmez.

Biçim (`format`): `decimals` (yoksa projenin uzunluk ya da alan basamağı), `unit` (`mm`, `cm`, `m`, `km` uzunlukta ve yerde;
`m2`, `donum`, `ha` alanda; yoksa çizim birimi ve projenin alan birimi), `comma` (ondalık virgül), `prefix`, `suffix`. Sayı
projenin gösterim kuralıyla yazılır (`display::fixed`, ADR 0149). Metinler olduğu gibi; değişkenin sayısı basamak verilmezse
en kısa yazımıyla, evet/hayır `Evet`/`Hayır`. Alanın değeri değerin türüne uymayan birimle (alanı santimetreyle) ve projenin
değeri birimle yazılmaz (`text_fields_problem`).

### 2. Yenileme, döngü ve kopuk kaynak (`CAD-ANNO-23`)

Belge her adımı kaydetmeden önce (`commit`, bağlı yazılar, ilişkili taramalar ve ölçülerden sonra; `kentos_domain::text_fields`):

- Elle düzenlenmiş yazı (harfleri değişmiş, alanları aynı): harflerin başta ve sonda aynı kalan kısmı bulunur; düzenlemenin
  dokunmadığı alanlar kayarak kalır, dokunduğu alan kalkar (değeri yazı olarak kalır).
- Kaynağı adımda eklenen, değişen ya da silinen yazılar, adımda eklenen ya da alanları değişen yazılar ve projenin ya da bir
  katman adının alanı olan bütün yazılar yeniden yazılır: değeri değişen alanın aralığı yeni harflerle yer değiştirir, sonraki
  dilimler ve alanlar kayar.
- Harflerin yerine harf koymanın tek kuralı: aralıktan önceki yer kalır, sonraki kayar, içindeki yeni harflerin içinde kalır;
  iki harfin arasına konan harfler o yerde başlayan aralığı iter, orada biten aralığı uzatmaz.
- Bir yazıyı okuyan yazı ondan sonra yazılır; değişen yazıyı okuyanlar onu izler. Birbirini halka hâlinde okuyan yazıların o
  alanları kopuktur (`cycle`).
- Değeri okunamayan alan **kopuk**tur: harfleri `####`, nedeni `broken`'da (`removed` nesne silindi, `noValue` nesnenin bu değeri
  yok, `notNamed` öznitelik ya da değişken yok, `cycle`). Kaynak geri gelirse (geri alma) değer döner.

Değişiklikler adımın içindedir: bir geri alma hepsini geri verir. Proje ayarları geri alma adımı değildir (ADR 0023): proje
alanları bir sonraki adımda ya da Alanları yenile ile yazılır.

### 3. Komutlar (`CAD-ANNO-24`)

- `cad.fields.edit`: `insert` (yazının `start`–`end` harflerinin yerine ya da eşitse o yere alan; değeri hemen yazılır), `remove`
  (alan kalkar, harfleri yazı olarak kalır), `format`, `update` (verilen ya da kilitsiz bütün alanlı yazılar yeniden yazılır).
  Ret kodları `not_a_text`, `invalid_text_field`, `field_not_found`; kopuk alan `field_broken` uyarısı.
- `cad.texts.edit`: `replace` (Bul ve değiştir: Türkçe büyük-küçük harf kuralı, tam sözcük; dilimler ve alanlar kayar; boş kalacak
  yazı `text_left_blank`, kilitli katmandaki `layer_locked` uyarıyla atlanır; verilenlerin hepsi kilitliyse ret), `number`
  (AutoCAD'in TCOUNT'u: sırayla `start + i × step`, en az `digits` basamak, ön ve son ek; yazının yerine, başına ya da sonuna),
  `combine` (TXT2MTXT: yazılar yerleşim noktalarına göre yukarıdan aşağıya, eşitse soldan, satır satır tek yazı; en üsttekinin
  görünüşüyle ve üstünden aynı paydaki üst hizayla; öbürleri silinir), `split` (her paragraf durduğu yerde, biçim dilimleri ve
  alanlarıyla ayrı yazı; boş paragraf atlanır; tek paragraflı, sütunlu, eğri boyunca ve ölçekli yazı `not_splittable`).
- `cad.texts.check` (salt okunur): satırın içinde art arda iki ya da daha çok boşluk (satır başı girintidir), art arda iki kez
  yazılmış sözcük (Türkçe büyük-küçük harf kuralıyla; sayı değil), kapanmayan ya da açılmayan parantez, “ ” ve tek kalan `"`,
  kopuk alan. Sözlükle yazım denetimi yoktur: KentOS'ta Türkçe sözlük yok; gelirse ayrı bir ADR.

Yazının üstü: ilk tabanı noktasının `rise(up, yükseklik, satır, aralık)` altındadır (çekirdeğin `text::paragraph::rise`, ADR 0182),
üstü bir yükseklik yukarıda; `k`'ıncı paragraf `k` satır aralığı aşağıda başlar (kutulu yazıda çekirdeğin sarmasıyla).

### 4. Arayüz

CAD › Açıklama › Yazı'da **Alan ekle** (`text.fieldInsert`: seçili yazının sonuna; kaynak Nesne ya da Proje, nesne seçimden ya da
**Sahneden seç** ile, değer, ad, basamak, birim, ondalık virgül, ön ve son ek; önizleme komutun planı), **Alanları yenile**,
**Numarala** (seçilen yazılar seçim sırasıyla; önizleme), **Birleştir**, **Satırlara ayır**, **Yazı denetimi** (bulgular ve
Göster). Öznitelikler'de yazının **Alan n** satırları (değeri ya da kopukluğunun nedeni; menüde kaynağı, Kaynağı seç, Alanı
kaldır). Bul ve değiştir penceresi Joker kapalıyken `cad.texts.edit` ile yazar; Joker açıkken desen bütün yazıyı aldığı için
`cad.entities.edit`'in `replaceText`'iyle.

### 5. Dosyalar

`.kcad` şema 66 (`FORMATS_VERSION` 75): yazının `fields`'ı JSON'u gibi; sütunlarda yazının 17. seçeneği. DXF'te alanlar KENTOS
verisinde (`fields`): başka programlar harflerini (son değerleri) görür, KentOS bağları geri okur; harflere uymayan veri düşer.

## Sonuçlar

- Mahal etiketi, kapı numarası, parsel yazısı odasını, kapısını, parselini izler; silinen kaynak sessizce eski değeri bırakmaz.
- Kaynak tiplidir: ifadeli alan (`$alan * 2`) yoktur; gerekirse ayrı bir ADR'de ifade motorunun belge adımında çalışmasıyla.
- AutoCAD'in FIELD kodları DXF'e yazılmaz ve okunmaz.

## Doğrulama

- `scripts/fixtures/text_command_cases.py --check`: KentOS kodu olmadan yazılmış başvurudan `cad.fields.edit` (41), `cad.texts.edit`
  (27) ve `cad.texts.check` (20) ortak durumları; izleme (etiket değişince, oda büyüyünce, silinince ve geri alınca, zincir) başka
  komutların adımlarıyla aynı durumlarda.
- KCAD: `text-fields.kcad` ve yedi bozuk dosya; bağımsız yazıcı ve okuyucu.
- Resimler: `KENTOS_SHOTS_ONLY=yazi-alanlar,yazi-alan-ekle,yazi-numarala,yazi-denetimi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`.
