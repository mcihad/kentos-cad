# ADR 0326: Çok kollu kılavuz: kollar, blok içerik, bağlı uçlar ve kılavuz stilleri

- **Durum:** kabul edildi (2026-10-17). [plan/cad.md](../../plan/cad.md) §11 `CAD-ANNO-25`, `26`, `27` (sahibin sözü: “Bitir o
  zaman”; 3–4 maddelik gruplar). Kapsamı ben belirledim; kaynak modeli ADR 0264 §5'tedir.
- **Bağlam belgesi:** ADR 0146 (kılavuz), 0205 §7 (ok uçları), 0264 §5 (çok kollu kılavuzun kaynak modeli), 0249 ve 0263 (alt
  nesne başvurusu), 0175 (bağlı yazı), 0183 ve 0322 (stiller), 0323 (ölçekli açıklamalar).
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

Kılavuz (ADR 0146) tek oklu bir nottur. Mimari ve harita çiziminde bir not birden çok yeri gösterir (“Ø150 PVC” üç boruyu), kapı
ve pencere numaraları balon bloklarıyla yazılır, notlar bir hizaya dizilir. AutoCAD bunu MULTILEADER'la, MLEADERALIGN,
MLEADERCOLLECT ve kılavuz stilleriyle (MLEADERSTYLE) yapar. KentOS'ta DXF'in MULTILEADER'ının yalnız ilk kolu geliyordu; okun
ucu gösterdiği nesneye bağlı değildi, kapı taşınınca ok boşta kalıyordu.

## Karar

### 1. Kollar (`CAD-ANNO-25`)

Kılavuzun yeni `arms`'ı: ilk kol bugünkü `pts`'tir (ucu ilk köşe, kolun başladığı yer son köşe); her ek kol `pts`'iyle kendi
ucundan kırıklarından geçerek kılavuzun **son köşesine**, yani ortak kolun başına gider. Ok başı, boyu, not, yükseklik ve stil
bütün kolların ortağıdır. Her kolun ok başı ve çizgisi ilk kolunkiyle aynı kuralla çizilir (`geom::leader::layout`, kolun
noktaları ve ortak köşe); seçme, kenet ve tutamaçlar her kolun köşelerindedir; uzunluk kolların toplamıdır. Not çok satırlı
olabilir (satır sonu; satırlar notun hizasıyla). Taşıma, döndürme, ölçekleme ve aynalama kolları da taşır.

### 2. Blok içerik (`CAD-ANNO-25`)

Kılavuzun yeni `blocks`'u: notun yerinde duran blok yerleştirmeleri, kalıcı kimlikleriyle, sırayla (AutoCAD'in blok içerikli
MULTILEADER'ı; balon, kapı numarası). Yerleştirmeler çizimin olağan nesneleridir (öznitelikleri, Öznitelikler'i, kenetiyle);
kılavuz onları ortak kolun ucunda bir sıra hâlinde tutar: ilkinin tanımının kutusunun kola bakan kenarı kolun ucunda, kutusunun
dikey ortası kolun doğrultusunda, her biri bir öncekinin kutusunun ardında; dönüşleri notun doğrultusudur. Blokları olan
kılavuzun notu yoktur; kolu notlu kılavuzunki gibi çizilir.

Belge her adımı kaydetmeden önce (bağlı yazılar gibi): kılavuz değişince blokları yerlerine yazılır; kılavuz silinince blokları
da silinir; bir blok silinince kılavuzun listesinden düşer. Bloğu elle taşımak bağı koparmaz; kılavuz bir sonraki değişiminde onu
yerine koyar.

### 3. Bağlı uçlar (`CAD-ANNO-26`)

Her kolun ucu bir nesnenin yerine bağlanabilir (`tip`; ilk kolunki kılavuzun `tip`'i, ek kolunki kendi `tip`'i): ilişkili
ölçünün başvurusu (`SubRef`, ADR 0249) ve kuralıyla. Nesne değişince uç yerin gittiği yere gider; nesne silinince ya da yer
bulunamazsa uç yerinde kalır ve bağ nedeniyle kopar (`broken`). Ucu kendi başına taşınan kolun bağı düşer. Kapı taşınınca
etiketi ve balonu kapıyı gösterir.

### 4. Kılavuz stilleri (`CAD-ANNO-25`)

Projenin adlı kılavuz stilleri (`leaderStyles`): ok başı, ok boyu, notun kâğıttaki yüksekliği (mm), zemin. Kılavuzun
`leaderStyle`'ı izlediği stildir; stil değişince kılavuz eski değerdeki alanlarında yenisini alır (ADR 0183'ün kuralı). Stiller
`cad.styles.edit`'in `leader` türüyledir (oluştur, güncelle, ad, sil, dışa ve içe aktar).

### 5. Komut (`CAD-ANNO-27`)

`cad.leaders.edit`:

- `addArm`: kılavuza ucu `pts[0]` olan bir kol; isteğe bağlı `tip` bağı.
- `removeArm`: `arm` (0 ilk kol) kalkar; ilk kol kalkınca ilk ek kol ilk kol olur; tek kol kalkmaz.
- `align` (MLEADERALIGN): ilk kılavuz yerinde kalır; diğerlerinin son köşeleri (notları ve kolları onlarla) onun son köşesinden
  `angle` doğrultusundaki doğrunun üstüne izdüşümleriyle iner; `spacing` verilirse doğru boyunca ilkinden başlayarak eşit
  aralıkla, izdüşüm sırasıyla dizilir. Uçlar yerinde kalır.
- `collect` (MLEADERCOLLECT): blok içerikli kılavuzlar ilkinde toplanır: diğerlerinin kolları ilkinin ek kolları, blokları
  ilkinin bloklarının ardına; diğerleri silinir, blokları kalır.
- `content`: notu yazar ya da (null) kaldırır; blokları olan kılavuza not yazılmaz.
- `look`: kılavuzların ok başı, ok boyu, yüksekliği, zemini (stilden ayrılır); `style` bir stili izletir ya da (null) bırakır.
- `bind`: `arm`'ın ucunu `tip` başvurusuna bağlar (uç yerin noktasına gider) ya da (null) serbest bırakır.

Kilitli katmandaki kılavuz reddedilir. Deponun paketli kaydında kolu ya da bloğu olan kılavuz 24. türdür (15'inki, ardından
kolların noktaları ve blok sayısı); kolsuz kılavuz web'in paketlediği 15. tür olarak kalır. Arayüzde CAD › Açıklama › Kılavuz'da **Kol ekle**, **Kol sil**, **Hizala**, **Topla**,
Öznitelikler'de kılavuzun stili, kol sayısı, uçların bağı; DXF'in MULTILEADER'ı bütün kollarıyla ve blok içeriğiyle gelir, kollar
LEADER olarak gider, bütünü KENTOS verisinde. `.kcad` şema 67 (`FORMATS_VERSION` 76).

Arayüzde ayrıca **Blok içerik** (seçili kılavuz seçili yerleştirmeleri içerik alır) ve **Kılavuz stilleri** penceresi; Kol ekle'de
ucun kenetlendiği yer (köşe, uç, merkez) bağ olur. Öznitelikler'de Kılavuz stili (menü), Kollar, her bağlı ucun durumu ve Bağı çöz,
Blok içerik sayısı.

## Sonuçlar

- Bir not birden çok yeri gösterir; borular, kapılar taşınınca okları izler.
- Blok içerik kılavuzun içinde değil, kılavuzun tuttuğu yerleştirmelerdir: öznitelikleri, Öznitelikler'i ve kenetiyle çizimin
  nesneleridir; elle taşınan blok kılavuz bir sonraki değişiminde yerine döner.
- DXF'in MULTILEADER'ının blok içeriği ve DXF'e uçların bağları ve blok içerik yazılmaz (söylenir); kollar iki yönde.

## Doğrulama

- `scripts/fixtures/leader_command_cases.py --check`: `cad.leaders.edit`'in 39 ortak durumu KentOS kodu olmadan (kollar, hizalama,
  toplama, blok içeriğin yerleri, bağlar ve izleme, silmeler, retler); kılavuz stilleri `style_edit_command_cases.py`'nin dört
  durumunda.
- KCAD: `multileader.kcad` ve beş bozuk dosya; bağımsız yazıcı ve okuyucu. DXF: `a_leaders_arms_read_back_as_they_were`.
- Resimler: `KENTOS_SHOTS_ONLY=kilavuz-kollar,kilavuz-hizala,kilavuz-stilleri cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`.
