# ADR 1111: Paketin yazı tipi, SHX, çizgi tipi, desen ve baskı stili raporu

- **Durum:** kabul edildi (6 Kasım 2026). Kapsam: plan/cad-advanced.md `CAD-ADV-14`'ün açık bıraktığı “paketin
  stil/desen ayrıntı raporu” (ADR 1060 §12) ve aynı yerin “dış başvurunun çizimde karşılığı olmayan yazı stilini
  bırakması yüklemede ayrı söylenmiyor” notu. İlke “Performance First”; alt madde tek parçada.
- **Bağlam belgesi:** ADR 0350 §4 (paket ve `paket.json`), 0506 (Paketle), 0324 §5 (yazı tipi raporu, `fonts::resolve`),
  0313 (projenin çizgi tipleri), 0930 (SHX şekil kaynağı kitaplıkta gömülü), 0331 (projenin tarama desenleri), 0609
  (baskı profili), 0348 §2 (dış başvurunun stilleri ada göre eşlenir), 1060 §6 (kaynağın bulunması).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

Paket, dış başvuru kaynaklarını bayt bayt kopyalıyor, KentOS'un kendi yüzlerinde olmayan yazı tipini `left`'te
(`substituted`) söylüyordu. Ama çizimin ve dış başvurularının kullandığı çizgi tiplerini, SHX şekil dosyalarını, tarama
desenlerini ve baskı profillerini, her birinin pakette **nasıl** bulunduğunu (KentOS'la mı gelir, proje dosyasına mı
gömülü, yalnız kopyalanan bir kaynak dosyada mı, hiçbir yerde mi) söyleyen yapılı bir bölüm yoktu. İki sessiz kayıp
koddan doğrulandı:

1. Dış başvurunun katmanları ve parçaları kaynağın çizgi tipini **adıyla** taşır (`linePattern`); kaynağın çizgi tipi
   tanımı çizime alınmaz. Çizimde o adda tip yoksa çizim en yakın yerleşik tiple çizer; bunu kimse söylemiyordu.
2. Dış başvurunun kaynağındaki yazı stili çizimde aynı adla yoksa yazılar stilsiz (Standart) gelir (ADR 0348 §2):
   yüklemede de pakette de söylenmiyordu.

### Araştırma

- AutoCAD eTransmit: geçerli çizim ve başvurduğu destek dosyaları (yazı tipleri, SHX, CTB/STB baskı stili tabloları,
  dış başvurular) pakete kendiliğinden eklenir; rapor dahil edilenleri ve **tanınıp dışarıda bırakılanları** ayrı
  listeler; yazı tipleri ayrı seçenektir ve lisans uyarısı vardır; rapor `.txt` olarak pakettedir
  (help.autodesk.com/cloudhelp/2023/ENU/AutoCAD-DidYouKnow/files/GUID-DD451786-5D5A-42A5-92D5-943CE9D66016.htm;
  “Files Table Tab (Create Transmittal Dialog Box)”, AutoCAD LT 2026,
  help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-LT/files/GUID-A329E883-840A-4584-A59A-06468DEB6137.htm).
- Kurumların yayımladığı eTransmit raporları (davranış örneği, resmî belge değil): `simplex.shx`, `txt.shx` gibi SHX
  yazı tipleri “dışarıda bırakılan” listesinde; alıcıya CTB'yi baskı stili arama yoluna kopyalaması söyleniyor; kök
  çizimin kullanmadığı bindirme başvurusunun baskı stili tablosu pakete girmiyor; eksik yazı tipli stiller yedek yüze
  çevriliyor.
- **KentOS kararı:** dosya kopyalamak yerine **ne gerektiği ve nasıl bulunduğu** yapılı söylenir. KentOS'un yüzleri
  uygulamayla gelir (lisanslı yazı tipi kopyalanmaz); çizgi tipleri, desenler, SHX şekil kaynağı ve baskı profilleri
  proje dosyasının içindedir (ADR 0350 §4); yalnız dış başvurunun kaynak dosyasında olan ama çizimde olmayan şey ve
  hiçbir yerde olmayan şey bulgudur, uyarıyla söylenir.

## Karar

### 1. Rapor (`file.package` cevabının `resources`'ı, `paket.json`'un `resources`'ı)

Satır başına `kind`, `name`, `how`, isteğe bağlı `file`, `users` (sıralı: `stil:Ad`, `katman:Yol / Ad`,
`çizgi tipi:Ad`, `pafta:Ad`, `xref:Ad`), `objects` (onunla çizilen nesne sayısı) ve `note`. Satırlar türe (font,
shapeFile, lineType, hatchPattern, plotStyle) sonra ada göre (karakter sırası) dizilir; aynı çizim her zaman aynı
raporu verir.

`how`: `builtIn` (KentOS'la gelir), `embedded` (proje dosyasında), `carried` (yalnız pakete kopyalanan dış başvuru
kaynağında; bu çizim onsuz çizer), `substituted` (yerine başka yüzle çizilir), `missing` (hiçbir yerde).

- **font:** projenin her yazı stili, stilin adıyla; `file` adlandırdığı dosya, yoksa KentOS yüzü. Dosya adlandırmayan,
  KentOS yüzünün ailesini ya da eşdeğerini (Arial/Helvetica/Liberation Sans → Arimo, Courier New/Courier → Courier
  Prime; ADR 0324 §5'in kuralı, aile ve ardından yalnız ağırlık/biçim eki) adlandıran `builtIn`; SHX ya da başka aile
  `substituted` (ayrıca `left`'te, `package_left`). `objects`: stili kullanan çizim yazıları. Dış başvurunun kaynağının
  kendi yazılarının kullandığı, çizimde aynı adla (büyük küçük harf bir) olmayan stil (Standart hariç) o başvurunun
  `carried` satırıdır: “bu çizimde yok, yazıları Standart ile çizilir” (ADR 1060 §12'nin notu).
- **lineType:** projenin her çizgi tipi (`embedded`); her katmanın (deseni, yoksa yerleşik tipi) ve her nesnenin (çizimin,
  blokların ve dış başvuru parçalarının kendi deseni ya da tipi) adlandırdığı: projede varsa `embedded` (projenin
  adıyla), dört yerleşik tipten biriyse `builtIn`, yalnız kopyalanan bir kaynağın ayarlarında varsa `carried`, değilse
  `missing`.
- **shapeFile:** projenin çizgi tiplerinin şekil öğelerinin SHX dosyası (adı küçük harfle): kitaplıkta `shx:<ad>`
  kaynağı varsa `embedded`, yoksa `missing` (“çizgi tipinin şekilleri çizilemez”).
- **hatchPattern:** projenin her deseni (`embedded`); her taramanın: dolu, çizgili, çapraz, geçişli `builtIn`; desen
  adıyla projede varsa `embedded`, değilse çizgileri taramanın kendisinde olduğundan `embedded` (“desenin çizgileri
  taramanın kendisinde”). KCAD yazıcısı çizgisiz desenli taramayı reddettiğinden `missing` ve `carried` yalnız
  kaydedilmemiş bozuk çizimde olur (kural yine yazılıdır).
- **plotStyle:** pafta kitabının her baskı profili (`embedded`), bir paftanın profili (kendi kopyası, yoksa kitabınki)
  `pafta:` kullanıcısıyla; ikisinde de olmayan profil kimliğiyle `missing`. Profilin adlı stilleri bütündür: pafta
  çekirdeğinin kitap denetimi atanan ya da varsayılan stili tablosunda olmayan profili zaten reddeder (ADR 0609);
  kitap okunamıyorsa bu da `missing` satırıdır (“pafta kitabı okunamadı”), atlanmaz. İkili CTB/STB okunmaz (ADR 1090
  §1'in sınırı), KentOS profili `kentos.plot/1` olarak kitaptadır.

### 2. Bulgular

`missing` ve `carried` satırları `package_resources` uyarısıdır (yeni kod): sayıları ve her birinin adı, türü ve hâli.
Paket yine yapılır; hiçbir şey sessizce düşmez. `substituted` yazı tipleri ADR 0350'nin `package_left` uyarısında kalır.

### 3. Dış başvuruların kaynakları

Rapor, paketin kopyaladığı (ADR 1060 §6'nın kuralıyla bulunan, `leaveOut`'ta olmayan) her kaynağı bir kez okur
(`xrefs::source_drawing`: KCAD, DXF, DWG; PDF sayfasının kendi stili yoktur). Kaynağı bulunamayan başvurunun gerekleri
`carried` olamaz: `missing`'dir.

### 4. Arayüz

Masaüstünde Paketle bittiğinde ileti rapor özetini söyler (“Yazı tipi, SHX, çizgi tipi, tarama deseni ve baskı stili
raporu: N kaynak, K tanesi pakette tanımlı değil”), bulgular uyarı satırlarıdır; ayrıntı `paket.json`'dadır. Yeni ikon
gerekmedi.

### 5. Komut ve otomasyon

Yeni komut yok: `file.package` v1 cevabına `resources` (`PackageResource`, `PackageResourceKind`,
`PackageResourceHow`) ve `package_resources` uyarısı eklendi; yetenekler, katalog özeti, TS türleri ve Python SDK
güncellendi. `paket.json` `resources`'ı yalnız rapor boş değilse taşır (boş çizimin manifesti aynı kalır).

### 6. Performans

Rapor planda bir kez kurulur: katman ağacı ve nesneler bir kez gezilir; her kopyalanan kaynak bir kez okunur (paket onu
zaten özetle okuyor). Sıcak yola dokunulmadı.

## Uygulama

- Sözleşme: `crates/shared/contracts/src/file_package.rs`, `codes.rs` (`PACKAGE_RESOURCES`).
- Başsız: `crates/native/headless/src/package_resources.rs` (rapor), `package.rs` (kaynakların yeri, manifest).
- Uygulama: `crates/native/application/src/xrefs.rs` (`source_drawing`).
- Masaüstü: `apps/desktop/src/exchange/package.rs`.

## Doğrulama

- **Bağımsız başvuru:** `python3 scripts/fixtures/package_resource_cases.py --check` — `fixtures/commands/v1/
  file.package.resources.json` (24 durum, `host: headless-file`) ve kaynak çizim `fixtures/package/v1/kaynaklar/
  kadastro.kcad` (bağımsız KCAD v2 yazıcısıyla). Kurallar ADR'den yazıldı; KentOS kodu yok. Başsız sunucunun
  `tests/headless/package_resources.rs`'i her durumu proje dosyası olarak yazıp açar ve paketi planlar.
- Kabul testi `xref_network.rs` paketin cevabında `resources`'ı denetler.
- Sonuçlar: plan/cad-advanced.md CAD-ADV-14'ün 6 Kasım 2026 paragrafı.

## Web'e ertelenenler

TS türleri üretildi; web'in Paketle iletisinde rapor özeti.
