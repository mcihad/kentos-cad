# ADR 0342: Mahal ve metraj

- **Durum:** kabul edildi (2026-10-18). [plan/cad.md](../../plan/cad.md) §14.3 `CAD-ARCH-23`–`30`, `CAD-ARCH-01`'in mahal kısmı ve
  `CAD-ARCH-08`'in mahal kalanı (sahibin sözü: “arch ile devam edelim”, “Devam et”; yedi grubun dördüncüsü). Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0339 (duvar), 0340 (birleşimler), 0341 (açıklık), 0328 (çizgilerle kapanan yüz, `FaceIndex`), 0199 (katman
  alanları), 0227 (tablo çıktıları), 0228 (web'e ara), CLAUDE.md §9.2.
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

Mahal (oda), sınırı ve alanı olan bir mimari nesnedir: numarası, adı, kullanımı, katı ve kaplamaları vardır; sınırı elle çizilir ya da
duvarlardan bulunur ve duvar değişince izler. “Alan” tek belirsiz bir sayı değildir: iç yüzlerden net, duvar ekseninden ya da dış
yüzlerden brüt ölçülür; hangi yöntemle ölçüldüğü mahalin alanıdır. Metraj (alan, çevre, duvar ve açıklık miktarları) kaynak geometriden
hesaplanır ve nereden geldiğini söyler. Ölçme yöntemi bir ofisin ya da kurumun kuralı olabilir; KentOS bir yöntemin yürürlükteki bir
mevzuata uyduğunu söylemez.

## Karar

### 1. Kaynak modeli (`CAD-ARCH-23`, `24`, `25`)

- **Yeni tür `space`** (`kentos_contracts::SpaceEntity`, `.kcad` şema 78, `FORMATS_VERSION` 85, sütunlu kayıtta tür 23, deponun paketinde
  tür 30, sunucuda göç 0025). Kaynak:
  - `parts`: sınır; her parça bir dış halka (`outer`: noktalar ve kenar başına büküm) ve delikleri (`holes`); birden çok parça bağımsız
    parçalardır (bir mahalin iki ayrı bölümü);
  - `bound`: `drawn` (sınır olduğu gibi) ya da `walls` (`seed`: içindeki nokta; sınırı belge duvarlardan yazar, §3);
  - `method`: `net` (duvarların iç yüzleri), `centerline` (duvar eksenleri), `gross` (dış yüzler): halkaların neyi gösterdiği. Çizilen
    sınırda kullanıcı söyler; duvarlardan bulunan sınır yöntemle bulunur (§2);
  - `number` (mahal no), `name`, `usage` (kullanım), `arch` (kat, evre, durum, disiplin: duvarınkiyle aynı `ArchInfo`), `height`
    (tavan yüksekliği; yoksa katın), `finishes` (`floor`, `wall`, `ceiling`: kaplamalar), `broken` (`lostEnclosure`: duvarlar artık
    tohumu kapatmıyor; sınır son hâliyle kalır);
  - kullanıcı alanları katmanın tipli alanlarıdır (ADR 0199): mahalin öznitelikleri katmanın şemasıyla denetlenir.
- **Projenin ölçme yöntemi** `settings.arch.spaceMethod` (yoksa `net`): yeni mahalin varsayılanı; ofisin ya da kurumun profili. Proje
  ayarları › Mimari'de seçilir. Bir yöntemin mevzuata uyduğu söylenmez (`CAD-ARCH-30`).

### 2. Geometri (`geom::space`)

- **Duvarlardan sınır:** çizimin duvarları birleşimleriyle çözülür (ADR 0340), açıklıksız (kapı boşluğu mahali açmaz); her duvarın iki
  yüzü ve uçları bir çizgi işi olur; `FaceIndex` tohumu çevreleyen yüzü delikleriyle bulur (ADR 0328). Bulunamazsa sınır yoktur.
- **Yönteme göre kaydırma:** net yüzün her kenarı, üstünde durduğu duvar yüzü varsa duvarın içine kaydırılır: `centerline`'da
  duvarın yarı kalınlığı, `gross`'ta tam kalınlığı kadar; duvar yüzünde olmayan kenar (çizilmiş sınır, duvar ucu) kaymaz. Dış halka
  saat yönü tersine, delikler saat yönünde düzenlenir ve her kenar sağına kayar. Ardışık iki kaymış kenarın doğruları kesiştiği yer
  yeni köşedir; paralel kalan iki kenar (aynı doğrultuda kalınlığı değişen duvar) aralarına kısa bir basamak kenarı alır. Yaylı kenar
  merkezine göre kayar (yarıçap kayma kadar değişir); köşesi komşusunun teğet doğrusuyla kesişimdir (yay duvarlar teğet birleştiğinde
  kesin).
- **Ölçüler:** alan halkaların kesin alanı (yaylar dahil), delikler düşülür; çevre bütün halkaların uzunluğu; hacim alan × yükseklik.
- **Çizim:** mahal sınırı ince kesikli çizgi ve kendi rengiyle çok açık dolgu; içinde etiketi: numara, ad ve alan (iki basamak, m²)
  ulaşılamazlık kutbunda, kâğıtta 2,5 mm yüksekliğinde. Seçim sınırından ve içinden.

### 3. Duvarla güncellenmek (`CAD-ARCH-26`, `08`)

- **Belge izler** (`kentos_domain::space_follow`): bir adımın sonunda, açıklıklardan sonra, duvarı ya da duvarın açıklığı değişen
  adımda duvarlardan bulunan her mahalin sınırı aynı tohumla yeniden yazılır (bir duvarın eski ve yeni kutusuna değen mahaller);
  tohum artık kapanmıyorsa `broken: lostEnclosure`, sınır son hâliyle kalır; yeniden kapanınca kalkar. Mahalin etiketi ve alanı
  sınırdan çizilir, aynı adımda değişir; metraj ve tablolar sorulduğunda kaynaktan hesaplanır.
- Mahalin kendisini taşımak (Taşı) `walls` mahalinin tohumunu da taşır; yeni yerde yeniden bulunur.

### 4. Komutlar (`CAD-ARCH-23`, `25`, `27`, `28`, `29`)

- **`cad.spaces.create`** (“Mahal”): `seed` (duvarlardan) ya da `parts` (çizilen sınır), `method` (yoksa projenin), `number`, `name`,
  `usage`, `arch`, `height`, `finishes`, öznitelikler, katman. Kodlar: `no_objects`, `not_finite`, `space_not_enclosed` (tohumu duvar
  kapatmıyor), `invalid_space` (halka en az üç köşe, sıfırdan büyük alan, en çok 10 000 köşe), `level_not_found`, katman ve sürüm.
- **`cad.spaces.edit`** (“Mahali düzenle”), hepsi ya da hiçbiri: `set` (alanlar), `merge` (verilen mahaller birleşir; ilki kimliğini
  korur, öbürleri silinir; sınırları birleşir ve `drawn` olur), `split` (bir çizgiyle iki ya da daha çok parçaya; en büyük parça
  kimliğini korur, öbürleri yeni kimliklerle aynı alanları taşır), `rebound` (duvarlardan yeniden bul; `drawn`'ı `walls` yapar).
  Cevap `kept`, `added` ve `removed` kimlikleriyle söyler (`CAD-ARCH-27`). Kodlar: `not_a_space`, `too_few_objects`,
  `spaces_not_touching` (birleşmeyen mahaller), `split_missed` (çizgi mahali bölmüyor), `space_not_enclosed`, `invalid_space`,
  `nothing_to_set`, `level_not_found`.
- **`cad.spaces.quantities`** (“Metraj”, salt okunur): mahaller (numara, ad, kullanım, kat, yöntem, alan, çevre, yükseklik, hacim;
  duvarlardan bulunanda üç yöntemin alanı ayrı), duvarlar (stil, uzunluk, kalınlık, yükseklik, brüt yüzey, açıklıkların alanı, net
  yüzey, hacim) ve açıklıklar (tür, biçim, numara, genişlik, yükseklik, alan); süzgeçler `levels`, `phases`, `usages`, `layers`; her
  satır kaynağının kalıcı kimliğiyle (`uid`; duvarın satırı açıklıklarının kimlikleriyle) ve kuralıyla (`rule`: kısa formül). Toplamlar
  satırların sırasıyla toplanır (ortak durumlar başvuruyla 1e-9 içinde). Masaüstünde Metraj penceresi aynı satırları panoya ve CSV, XLSX
  ya da ODS'ye (ADR 0223; üç sayfa) verir: aynı sayılar. Çizime tablo olarak yerleştirmek çizelgelerle 6. gruptadır.

### 5. Masaüstü

- **Mahal** (`tool.space`): duvarlarla çevrili bir yere tıklama mahali yazar (önizleme komutun planından); Yöntem (Y), Numara (N: her
  mahalde bir artar), Ad (A), Kullanım (K); Çiz (Ç) sınırı köşelere tıklatır, Enter kapatıp yazar, G son köşeyi geri alır. **Mahali böl** (`tool.spaceSplit`: mahal, sonra çizgi),
  **Mahalleri birleştir** (`arch.spaceMerge`: seçili mahaller), **Metraj** (`arch.quantities`).
- Öznitelikler'de Numara, Ad, Kullanım, Yöntem, Kat, Evre, Yükseklik, Kaplamalar; Alan, Çevre, Hacim; Sınır (Duvarlardan ya da
  Çizilmiş; Duvarlardan yeniden bul); Durum. Mahalin etiketi (numara, ad, alan) tek çok satırlı yazıdır, satırları birlikte büyür. Seçim süzgecinde Mahal;
  kapanmayan mahal uyarı üçgeniyle. Şeritte Mimari › Mahal paneli.

## Sonuçlar

- Mahal kendi türüdür; alanı yöntemiyle birlikte anlamlıdır; duvarlardan bulunan sınır duvarı izler, kaybolursa nedeniyle kalır.
- Mahal etiketinin stili ve çizelgeler 6. gruptur; bu ADR'nin metrajı onların kaynağıdır.

## Doğrulama

- Çekirdek: `cargo test -p kentos-geometry-core --test all spaces` (`space_cases.py`: bölge, kaydırma, ölçüler).
- Komutlar: `python3 scripts/fixtures/space_cases.py --check` (11 durum), `space_command_cases.py --check` (`cad.spaces.create` 27,
  `.edit` 34, `.quantities` 23 durum; duvar taşınınca ve kalınlaşınca güncellenmek, kapanmayan mahal dahil).
- `.kcad`: `python3 scripts/fixtures/kcad_v2_reference.py --check`, `cargo test -p kentos-kcad`.
- Resimler: `KENTOS_SHOTS_ONLY=mahal-serit,mahal-oda,mahal-arac,mahal-brut,mahal-oznitelikler,mahal-bol,mahal-metraj,mahal-metraj-duvar,mahal-kayip
  cargo test -p kentos-desktop tools_screens -- --ignored --nocapture` (`.run/shots/arac-mahal-*`).
