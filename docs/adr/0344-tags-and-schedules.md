# ADR 0344: Etiketler ve çizelgeler

- **Durum:** kabul edildi (2026-10-18). [plan/cad.md](../../plan/cad.md) §14 `CAD-ARCH-20`, `21`, `22` ve `CAD-ARCH-39`'un mahal
  etiketi; `CAD-ARCH-01`'in etiket ve çizelge kısmı (sahibin sözü: “Devam arch ile”; yedi grubun altıncısının ilk yarısı, 6a). Kapsamı
  ben belirledim. Kesit, görünüş ve detay göstergeleri, görünümler ve döşeme, tavan, mobilya yerleşimi 6b'dir (ADR 0345).
- **Bağlam belgesi:** ADR 0341 (açıklık), 0342 (mahal), 0343 (aks, kolon), 0184 ve 0327 (tablo, blok çizelgesi), 0325 (alanlar),
  0228 (web'e ara), CLAUDE.md §9.2.
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

Mimari plan nesnelerini etiketlerle okur: kapının ve pencerenin numarası ve türü, mahalin numarası, adı ve alanı, kolonun etiketi.
Etiket elle yazılmış bir yazı olursa kapı yeniden numaralanınca eskir. Çizelge de aynı sorunu taşır: kapı ve pencere listesi,
mahal listesi çizimden okunmalı ve kapı değişince değişmelidir. Çizelgede bazı hücreler kaynağın özelliğidir (numara, tür,
denizlik): orada düzenlenince kapı değişir; bazıları hesaptır (adet, alan, yükseklik): düzenlenemez.

## Karar

### 1. Etiket (`CAD-ARCH-20`, `39`)

Yeni tür `tag` (`.kcad` şema 80, `FORMATS_VERSION` 87, sütunlu kayıtta tür 28, deponun paketinde tür 35, sunucuda göç 0027):

- `of`: etiketlenen nesnenin kalıcı kimliği (açıklık, mahal, kolon, aks, duvar, merdiven, rampa, blok yerleştirmesi).
- `at`: etiketin yeri (ortası).
- `content`: şablon; `{number}`, `{type}`, `{width}`, `{height}`, `{sill}`, `{head}`, `{name}`, `{usage}`, `{area}`, `{perimeter}`,
  `{level}`, `{tag}`, `{place}`, `{label}`, `{style}`, `{risers}`, `{slope}` ve `{attr:Ad}`; satırlar `\n`. Yoksa türün varsayılanı:
  açıklıkta `{number}`, mahalde `{number}\n{name}\n{area}`, kolonda `{tag}`, akstaki `{label}`, duvarda `{style}`, merdivende
  `{risers}`, rampada `{slope}`, blokta `{attr:NO}`.
- `shape`: `circle`, `hexagon`, `rectangle`, `rounded`, `none` (yoksa türün: açıklıkta altıgen, mahalde dikdörtgen, ötekilerde
  daire); `size`: yazının kâğıttaki yüksekliği, mm (yoksa 2.5); `leader`: etiketten nesnenin bağ noktasına çizgi.
- `text` ve `anchor` belgenin yazdığıdır: şablonun nesnenin şimdiki değerleriyle doldurulmuşu ve nesnenin bağ noktası (açıklığın
  ortası, mahalin etiket noktası, kolonun merkezi, öteki nesnelerin ortası). Nesne yok olursa `broken: targetGone`, yazı kalır.
- Belge izler (`kentos_domain::tag_follow`): bir adımın sonunda etiketlenen nesnesi eklenen, değişen ya da silinen her etiketin
  yazısı ve bağ noktası yeniden yazılır; nesne yer değiştirdiyse etiket de bağ noktasının kaydığı kadar kayar (elle konan yeri korunur).
- Değerlerin yazımı (çekirdeğin `geom::tag`, bağımsız başvuru `tag_cases.py`): uzunluklar projenin biriminde ve basamağında,
  alanlar m² ve alan basamağında, sayılar en kısa; boş değer boş yazılır.

### 2. Denetim (`CAD-ARCH-20`)

Salt okunur `cad.tags.check`: açıklıkların yinelenen numaraları (aynı numara iki açıklıkta), türü olmayan açıklık, duvarı kopuk
açıklık (`hostGone`, `offWall`, `overlap`), nesnesi olmayan etiket, etiketsiz açıklık ve mahal; her bulgu kodu, sözü, nesnelerin
kimlikleri ve yeriyle. Kodlar `tag_duplicate_number`, `tag_missing_type`, `tag_host_broken`, `tag_target_gone`, `tag_untagged`.

### 3. Çizelge (`CAD-ARCH-21`, `22`)

Tablonun yeni kaynağı `TableSource::Schedule`:

- `of`: `openings`, `spaces`, `columns`, `walls`.
- `columns`: sütunlar sırasıyla; her biri bir alan (`number`, `type`, `form`, `width`, `height`, `sill`, `head`, `wall`, `level`,
  `phase`, `count`, `area`, `perimeter`, `volume`, `name`, `usage`, `tag`, `place`, `length`, `thickness`, `attr`) ve başlığı.
- `filter`: katlar, evreler, biçimler (kapı, pencere, boşluk), katmanlar.
- `sort`: sütunlar ve yönleri; `group`: aynı sütun değerli satırlar bir satır, `count` sayısıyla; `total`: toplam satırı.
- Belge çizelgeyi her adımda yeniden yazar (blok çizelgesi gibi, `kentos_domain::table_cells`); satırların nesneleri `members`
  (satır başına kalıcı kimlikler) belgenin yazdığıdır.
- **Düzenlenebilir ve hesaplanan hücre:** sütunun alanı kaynağın özelliğiyse (numara, tür, genişlik, denizlik, lento, ad,
  kullanım, etiket, kalınlık, öznitelik) hücre düzenlenebilir; duvarın türü stilidir, çizelgede değişmez. `count`, `area`,
  `perimeter`, `volume`, `height`, `length`, `wall`, `level`, `phase`, `place`, `form` hesaptır. Hesaplanan sütunların veri
  hücreleri gri (`#7f8790`), sayılar sağa, toplam kalın (belgenin yazdığı biçim); `cad.tables.edit`'in hücre, satır, sütun ve
  biçim düzenlemeleri çizelgede reddedilir (`schedule_cell`), boy ve genişlikler düzenlenir.
- Satırların sırası: sıralama anahtarları sırayla, sayılar değerle, sözler doğal sırayla, boşlar sonda; eşitlikte çizimin sırası.
  Grup: toplanmayan bütün sütunlarda aynı olan satırlar ilkinin yerinde tek satır, toplananlar eklenir. Başlık: sütunun adı,
  yoksa alanın adı ve birimi (`Genişlik (m)`, `Alan (m²)`, `Hacim (m³)`). Toplam: ilk sütun toplanmıyorsa `Toplam`, adedin
  boş toplamı 0.
- Belge çizelgeyi yalnız adımın dokunduğu türler onun okuduklarındansa yeniden yazar (açıklıkta açıklık ve duvar, mahalde
  mahal, kolonda kolon ve aks, duvarda duvar ve açıklık): büyük çizimde her adımda bütün çizelgeler sayılmaz.

### 4. Komutlar

- `cad.tags.create` (“Etiket”): `tags` (her biri `of`, `at`, `content`, `shape`, `size`, `leader`) ve ya da `all` (türler, katmanlar,
  şablon, çerçeve, yükseklik; etiketi olmayan görünen nesnelere). `at` verilmezse mahalde bağ noktası, ötekilerde onun 4 yazı
  yüksekliği üstü; çerçeve verilmezse açıklıkta altıgen, mahalde dikdörtgen, ötekilerde daire. Kodlar `no_objects`, `invalid_uid`,
  `not_finite`, `entity_not_found`, `not_taggable`, `invalid_tag`, kalınlık, sürüm ve katman.
- `cad.tags.edit` (“Etiketi düzenle”): şablon (null türün şablonu), çerçeve, yazı yüksekliği (null 2,5 mm), kol; yazılar
  nesnelerinden yeniden yazılır; hepsi ya da hiçbiri, kilitli katman reddeder (`not_a_tag`, `nothing_to_set`, `invalid_tag`).
- `cad.tags.check` (salt okunur, yukarıda).
- `cad.schedules.create` (“Çizelge”): tür, sütunlar (yoksa türün sütunları), süzgeç, sıralama, grup, toplam, yazı yüksekliği
  (yoksa projenin tablo yüksekliği çizim ölçeğinde), sol üst köşe, katman (`invalid_schedule`, `level_not_found`).
- `cad.schedules.edit` (“Çizelgeyi düzenle”): `cell` (tablonun satırı, 0 başlık; sütun; değer: uzunluk projenin biriminde, nokta ya da
  virgülle; boş isteğe bağlı alanı kaldırır) satırın nesnelerine aynı adımda yazar, belge çizelgeyi ve etiketleri izletir;
  `set` sütunları, süzgeci (null kaldırır), sıralamayı, grubu, toplamı değiştirir (`not_schedule`, `cell_out_of_range`,
  `cell_computed`, `schedule_group_number`, `invalid_cell_value`, `layer_locked`).
- Python `kentos.cad.tags`, `.schedules`; MCP; `cad.tags.edit` ve `cad.schedules.edit` yıkıcı listede. Ortak durumlar
  `fixtures/commands/v1/cad.{tags,schedules}.*.json` (162 durum) `scripts/fixtures/tag_command_cases.py`'den, KentOS kodu olmadan;
  kurulum kalıcı kimlik veremediğinden açıklıklar ve ilk etiket her durumun başında komutla yazılır.

### 5. Masaüstü

- **Etiket** (`tool.tag`): nesneye tıkla (imlecin altındaki etiketlenebilir nesne vurgulanır), sonra etiketin yerine; Enter
  nesnenin yanına koyar. Şablon (Ş; `\n` satır), Biçim (B: türe göre, daire, altıgen, dikdörtgen, yuvarlak köşeli, çerçevesiz),
  Kol (K). Önizleme komutun planından: çerçeve, kol ve soluk yazı.
- **Hepsini etiketle** (`tool.tagAll`): Açıklık (A), Mahal (M), Kolon (K), Aks (S), Duvar (D) aç/kapat, Enter.
- **Etiketleri denetle** (`arch.tagsCheck`): bulgular kendi penceresinde, yalnız sıfır olmayan türleri sayan özetle; Göster
  nesnelerine yakınlaştırır.
- **Kapı ve pencere çizelgesi**, **Mahal çizelgesi** (`tool.scheduleOpenings`, `tool.scheduleSpaces`): çizelge planından
  kesikli olarak imleçten sarkar, tıklama yerleştirir. Tür (T: açıklık, mahal, kolon, duvar), Grup (G: türün paylaşılan
  sütunları ve Adet; toplam açılır), Toplam (O), Sırala (S: ilk sütuna göre). Tabloyu düzenle'de çizelgenin hücreleri Kaydet'te
  `cad.schedules.edit` ile nesnelere, boylar `cad.tables.edit` ile, tek adımda yazılır; reddedilen değer pencerede söylenir.
- Öznitelikler'de etiketin satırları: nesnesi, şablonu, yazısı, çerçevesi, yazı yüksekliği, kolu, kopuksa durumu
  (`cad.tags.edit` ile); kopuk etiketin uyarı üçgeni. Mimari sekmesinde Etiket ve Çizelge panelleri; seçim süzgecinde Etiket.
- Görünüş: etiketin yazısı, tablonun hücreleri gibi, “Kaybolmasın”da büyütülmez (gerçek boy kuralı): çerçevesi yazısına göre
  ölçülüdür.

## Sonuçlar

- Etiketin ve çizelgenin yazısı kaynaktan türer; elle yazılmış eski numara kalmaz.
- Çizelgede düzenleme kaynağa yazar; hesaplanan değer düzenlenemez.
- Denetim bulguları söyler, düzeltmez.

## Doğrulama

- Çekirdek: `python3 scripts/fixtures/tag_cases.py --check` ve `cargo test -p kentos-geometry-core --test all tags_draw` (çerçeve,
  satırlar, kol; yazı tipinin ölçülmüş ilerlemeleriyle).
- Komutlar: `python3 scripts/fixtures/tag_command_cases.py --check`.
- Resimler: `KENTOS_SHOTS_ONLY=mimari-etiket-serit,mimari-etiket-plan,mimari-etiket-arac,mimari-etiket-oznitelikler,mimari-etiket-denetim,cizelge-arac,cizelge-izleme
  cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`.
- `.kcad` şema 80: `fixtures/kcad/v2/tags.json` ve bozuk örnekleri (`python3 scripts/fixtures/kcad_v2_reference.py --check`).
