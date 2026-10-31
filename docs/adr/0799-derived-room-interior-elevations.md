# ADR 0799: Türetilmiş çizim: oda iç görünüşü

- **Durum:** kabul edildi (31 Ekim 2026); `plan/architecture-advanced.md` ADV-06'nın altıncı teslimi (ADV-06f) **[~]**.
- **Bağlam:** ADV-06'nın “Kesit/görünüş işareti ile türetilmiş çizim ayrımı arayüzde de açık olsun. İlk gerçek üretim, mevcut
  kot/yükseklik ve host bilgilerinin yeterli olduğu kapı-pencere açılımı/oda iç görünüşü … olabilir. Desteklenmeyen elemanlar
  gösterilip raporlansın; yalnız 2B plandan bütün çatıyı/taşıyıcıyı bilen sahte otomatik kesit üretilmesin.” maddesi.
  Görünüm işaretleri ADR 0345, pafta seti (otomatik kesit/görünüşü yalnız tasarlamıştı) ADR 0347, duvar ADR 0339, açıklık
  ADR 0341, mahal ADR 0342, döşeme ADR 0722, türetim kaydı ADR 0633, araç = komut CLAUDE.md §9.2.
- **Web:** ertelendi (ADR 0228); satırı aşağıda.
- **Şema:** değişmez. Türetim kaydı katman düğümünün `derivation`'ında (ADR 0633 §2, `.kcad` şema 106, opt-in); geçici şema
  numarası alınmadı (§4).

## Araştırma (üreticilerin resmî belgeleri)

- Archicad, [Interior Elevations (IE)](https://help.graphisoft.com/AC/27/INT/_AC27_Help/050_ViewsVB/050_ViewsVB-48.htm) ve
  [Create Interior Elevation Group](https://help.graphisoft.com/AC/27/INT/_AC27_Help/050_ViewsVB/050_ViewsVB-50.htm): iç
  görünüş grubu bir dikdörtgen, çoklu çizgi ya da bölgeye (Zone) boşlukla tıklanarak kurulur; çoklu çizginin **her parçası
  ayrı bir iç görünüş** olur, bakış parçaya diktir. “Detect and Fit to Zones” açıkken düşey aralık **bölgenin yüksekliğidir**.
  Görünüşün durumu Auto-Rebuild Model (açıldığında plan değişmişse kendiliğinden yeniden kurulur), Manual-Rebuild (yalnız
  “Rebuild from Model” ile) ya da Drawing (çizgi ve dolgulara çevrilmiş, modele bağsız). Graphisoft'un destek yazısı çok
  sayıda Auto-Rebuild görünüşün dosyayı yavaşlattığını, Manual-Rebuild'in o yüzden seçildiğini söyler
  ([Interior elevation creates 2D elements](https://support.graphisoft.com/hc/en-us/articles/32855105904017-Interior-elevation-creates-2D-elements-instead-of-original-elements)).
- Revit, [About Elevation Views](https://help.autodesk.com/cloudhelp/2023/ENU/Revit-DocumentPresent/files/GUID-819DC7D0-FDAC-420E-ABDD-2E944211F331.htm):
  iç görünüş, mutfak ve banyo gibi iç duvarların ayrıntılı görünüşüdür; işaretin oku en yakın duvara döner, kırpma bölgesi
  görünüşün sınırıdır ve görünüş modelin canlı bir görüntüsüdür (3B modelden).
- Sonuç: KentOS'un 3B modeli yok (plan/3d.md). İç görünüş planın bildiklerinden **2B çizim olarak türetilir**: Archicad'in
  Manual-Rebuild durumuna denk; bağ kaydı tutulur, model değişince **eski** denir, yeniden üretmek açık bir eylemdir (Yeniden
  üret). Archicad gibi her düz yan bir görünüş; düşey aralık önce mahalin yüksekliği. Revit/Archicad'in modelde çözdüğü (kolonun
  kesiti, donatının görünüşü, döşemenin ve tavanın kesiti) KentOS'un modelinde yüksekliği olmadığından çizilmez, listelenir.

## Karar

### 1. İşaret başvurudur, türetilmiş çizim çizimdir

Görünüm işareti (Kesit, Görünüş, Detay; ADR 0345) yalnız bir hedefe bakar; işaret koymak çizim üretmez ve bunu iddia etmez.
Türetilmiş çizim ayrı bir komutun ürünüdür: Mimari › Belgele ▾'deki **Oda iç görünüşü** (`tool.roomElevation`), İşaret ▾'in
içinde değil, Başvurular'ın yanındadır. Çizimin nesneleri kendi katmanındadır; Öznitelikler onları “Türetilmiş çizim · oda iç
görünüşü” diye söyler. Bir görünüm işareti iç görünüşün katmanına ya da adlı görünüşüne bağlanabilir; bu bağ çizimi değiştirmez.

### 2. Neyin çizildiği (yalnız modelin bildiği)

- **Yan:** mahalin dış halkalarının her düz kenarı, parça parça 0'dan numaralı (harfleri A, B …). Odanın içinden bakılır:
  halka saat yönünün tersindeyse çizim kenarın sonundan başına, saat yönündeyse başından sonuna gider. Mahalin yöntemi Net
  olmalı (halka duvarların iç yüzleri); eksen ya da brüt mahal `elevation_space_not_net`. **Yaylı yan açılmaz** (`curvedSide`).
- **Duvar:** mahalle aynı kattaki duvarın düz parçası yana paralelse (sinüs ≤ 1e-6) ve sol ya da sağ yüzü (öteleme + hizaya
  göre 0, w/2, w; sağ yüz w eksik) 1 mm içinde yandaysa, parçanın uçlarının izdüşümü kadar yanı örter. Örtülmeyen kısımlar açık
  (`open`, `elevation_side_open`): ayırıcı ya da boşluk; orada duvar çizilmez.
- **Yükseklik:** mahalinki (döşemeden tavana); yoksa yanın duvarlarının en alçağı (duvarın kendi yüksekliği, yoksa katın);
  yoksa bilinmez: yalnız döşeme çizgisi (`elevation_height_unknown`). Ondan alçak bir duvarın üstü kendi uzunluğunca çizilir.
- **Açıklık:** yanın duvarlarındaki açıklığın, yanın yüzündeki iki pervaz noktası (sol yüzde `place[0]`, `place[2]`; sağda
  `place[1]`, `place[3]`) izdüşer; denizlikten (yoksa döşeme) lentoya dikdörtgen. Niş yalnız kendi yüzünden görünür. Lentosu
  olmayan açıklığın pervazları yanın yüksekliğine kadar **kesikli** (`headUnknown`, `elevation_head_unknown`). Baca duvarın
  içindedir, görünmez (`flue`); duvarına uymayan açıklık çizilmez (`brokenOpening`). Numaralı açıklığın numarası ortasında.
- **Çizilmeyen:** mahalin boşlukları (`hole`), mahalle aynı katta olup bir noktası mahalin içinde kalan kolon, merdiven,
  rampa ve blok (dolap, mobilya: `fixture`); `elevation_not_drawn` ve `leftOut` listesi. Döşeme ve tavanın kesiti çizilmez:
  mahalin yüksekliği net yüksekliktir, döşeme kalınlığı görünüşe kesit olarak girmez (sonraya kalan).
- **Her yanın nesneleri:** çerçeve (0..L × 0..H kapalı çoklu çizgi; yükseklik bilinmezse döşeme çizgisi), alçak duvarların
  üstleri, açıklıklar (x0'a, sonra kimliğe göre), numaraları, başlık “101 Salon — A” döşemenin 4h altında (üst orta), en ölçüsü
  2h altta ve yükseklik ölçüsü 2h sağda; h = 2,5 mm × çizim ölçeği. Yanlar başlangıçtan (verilmezse mahalin kutusunun 3 m sağı,
  altı) soldan sağa, her biri öncekinin L + aralık (1 m) ötesinde.

### 3. Komutlar

`cad.elevations.edit` (tek geri alma adımı “Oda iç görünüşü”, plan aracın önizlemesidir):

- `derive` (space, sides, at, gap, name): nesneler yeni katmana (“ic-gorunus-k”, adı “101 Salon iç görünüşü”, katmanların
  kuralıyla tekil) **İç görünüşler** grubunda (çizimde yoksa “ic-gorunusler” aynı adımda açılır; kilitliyse `layer_locked`).
- `refresh` (layer): kayıttaki mahal, yanlar, başlangıç ve aralıkla yeniden çizer; aynı roldeki nesne aynı kimlikle kalır
  (değişen üzerine yazılır, silinmiş olan eski kimliğiyle yeniden yapılır), artık çizilmeyen silinir, yeni rol eklenir.
  Hiçbir şey değişmiyorsa `nothing_to_set`; model aynıyken elle değiştirilmiş nesneler yeniden yazılır ve
  `elevation_edited_overwritten` söylenir; mahali yoksa `elevation_source_gone`.
- `release` (layer): türetim kaydı kalkar, nesneler sıradan çizim olarak kalır.

`cad.elevations.query` (salt okunur): iç görünüşler katman ağacının sırasıyla; her birinin mahali, adı, yanları, durumu
`current` (yeniden çizilse aynı ve nesnelerinin hepsi yerinde), `stale` (model değişti ya da nesnesi silindi), `gone` (mahali
yok), silinmiş (`missing`) ve model aynıyken elle değişmiş (`edited`) nesneleri, türetildiği sürüm. `layer` ile bir katman,
`uid` ile nesnenin üyesi olduğu iç görünüş. `cad.datasets.describe` iç görünüş katmanının eskiliğini bu kuralla söyler.

Ret ve uyarı kodları `codes.rs`'te (`elevation_*`); katalogda ve `capabilities.rs`'te. Python (`kentos.cad.elevations`) ve MCP
aynı komutlar; MCP'de `cad.elevations.edit` yıkıcı sayılır (yeniden üretmek nesnelerin üstüne yazar ve siler).

### 4. Kayıt: var olan türetim kaydı, şema değişmeden

ADR 0633 §2'nin katman türetim kaydı (`LayerDerivation`) aynı soruyu cevaplar: “bu katman neyden üretildi, hâlâ güncel mi?”.
Kaydın `tool`'u `cad.elevations.edit`, `parameters`'ı `ElevationRecord`: mahalin kimliği, istenen yanlar, başlangıç, aralık,
her nesnenin rolü ve kimliği (`0/frame`, `2/opening/<uid>`, `3/title` …) ve çizilenin SHA-256 özeti. Kayıt `.kcad` şema 106'da
zaten opt-in olarak taşınır; yeni şema, yeni ayar ya da Python/KCAD okuyucu değişikliği gerekmedi. `inputs` boş bırakılır:
katmanın içerik özeti (ADR 0511) iç görünüş için kaba olurdu (duvar katmanında ilgisiz bir çizgi değişse “eski” derdi);
eskilik nesne düzeyinde, yeniden çizilenin özetiyle ölçülür. Rol anahtarı nesnelerin kimliklerini yeniden üretmede korur.

### 5. Masaüstü

- **Oda iç görünüşü** aracı: mahalin içine tıklanır (mahal vurgulanır), sonra ilk yanın sol alt köşesi (Enter: mahalin
  yanına); önizleme komutun planıdır ve etiketi “Türetilmiş çizim · 4 yan: A, B, C, D · çizilmeyen: baca, kolon, donatı …”
  der. Yanlar (Y): harfler (A,C) ya da hepsi; Aralık (R).
- **Öznitelikler:** iç görünüşün nesnesi seçilince “İç görünüş: …” bölümü: Tür, Kaynak (101 Salon (mahal)), Yanlar, Durum
  (Güncel / Eski: model değişti, yeniden üretin / Kaynağı yok; silinmiş ve elle değişmiş sayıları) ve listesinde **Yeniden
  üret** (yalnız eskiyken ya da elle değişmişken) ile **Bağı kopar (çizim kalır)**; türetildiği sürüm.
- Şerit: Mimari › Belgele ▾ (`scripts/ui/ribbon.py`), komut kaydı ve ikon `catalog/v1/additions.json`.

## Doğrulama

- Ortak durumlar KentOS kodu olmadan ADR'nin kuralından: `python3 scripts/fixtures/room_elevation_cases.py --check`
  (`fixtures/commands/v1/cad.elevations.edit.json` 35 ve `cad.elevations.query.json` 22 durum): dört yanlı oda, yan seçimi,
  isimli katman, ikinci üretim, duvarsız ve yüksekliksiz mutfak, yaylı yan, brüt mahal, boşluk ve merdiven, saat yönünde halka,
  alçak duvar, retler; kapı taşınınca, başlık silinince ve elle taşınınca, mahalin yüksekliği değişince, kapı silinince yeniden
  üretim; bağı koparma; okumanın durumları. Masaüstünde `KENTOS_CASES_ONLY=cad.elevations cargo test -p kentos-native-application
  --test all every_command_case`; Python'da `test_command_cases`.
- Resimler `derived_elevation_scenes.rs` (`.run/shots/arac-ic-gorunus-*`).
- Gerçek pencerede elle denenmedi.

## Sonraya kalan

Kapı ve pencerenin kanat/açılış çizimi (bugün yalnız boşluğu), döşeme ve tavanın kesiti, duvar kaplamasının taraması, yaylı
duvarın açılımı, kolon ve donatının görünüşü (yükseklikleri modelde yok), sınırlı 2,5B kesit, iç görünüşün kendiliğinden
yeniden üretilmesi (bilerek yok: §Araştırma), paftaya yerleştirme ve görünüm işaretinin iç görünüşü kendiliğinden hedeflemesi.

## Belgelere eklenecek satırlar

- **CLAUDE.md §0** (Kırpma çerçevesi satırının ardına):

  - **Türetilmiş çizim: oda iç görünüşü (31 Ekim 2026; [ADR 0799](docs/adr/0799-derived-room-interior-elevations.md), ADV-06 [~]).**
    Bir mahalin her düz yanı odanın içinden, yalnız modelin bildiğiyle 2B çizilir: yüzü o yanda olan duvarlar, yükseklik (mahalin,
    yoksa duvarların en alçağı), açıklıklar denizlik ve lentolarıyla (lentosuz kesikli), numaralar, başlık ve iki ölçü; kendi
    katmanında (İç görünüşler grubu), tek adım “Oda iç görünüşü”. Kayıt katmanın türetim kaydında (ADR 0633; mahal, yanlar, rol →
    kimlik, özet; şema değişmez). `cad.elevations.edit` (derive, refresh: aynı kimliklerle, release) ve `cad.elevations.query`
    (current, stale, gone; missing, edited); yaylı yan, boşluk, kolon, merdiven, rampa, donatı, baca, kopuk açıklık çizilmez,
    `leftOut`'ta. İşaret başvurudur, çizim üretmez: araç İşaret ▾'de değil, Belgele ▾'de. Öznitelikler'de İç görünüş bölümü
    (Durum, Yeniden üret, Bağı kopar). 35 + 22 ortak durum (`room_elevation_cases.py`). Kanat çizimi, döşeme/tavan kesiti,
    2,5B kesit ve gerçek pencere kabulü açık.

- **CLAUDE.md §2** (komutlar):

  ```
  python3 scripts/fixtures/room_elevation_cases.py --check; KENTOS_CASES_ONLY=cad.elevations cargo test -p kentos-native-application --test all every_command_case; KENTOS_SHOTS_ONLY=ic-gorunus-arac,ic-gorunus-cizildi,ic-gorunus-yakin,ic-gorunus-eski,ic-gorunus-guncel cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture   # oda iç görünüşü: 35 + 22 ortak durum KentOS kodu olmadan, resimler .run/shots/arac-ic-gorunus-* (ADR 0799)
  ```

- **TODOS.md §0.2** (Web'e ertelenenler): “ADR 0799 oda iç görünüşü: `cad.elevations.edit` ve `.query`'nin web işleyicileri, Oda
  iç görünüşü aracı, Öznitelikler'in İç görünüş bölümü; ortak durumlar `fixtures/commands/v1/cad.elevations.*.json`.”

- **plan/architecture-advanced.md ADV-06** `_Durum_` paragrafında “Açık: türetilmiş çizim, yayın/revizyon birlikteliği …”
  yerine: “Oda iç görünüşü türetilmiş çizim olarak ([ADR 0799](../docs/adr/0799-derived-room-interior-elevations.md)): modelin
  bildiği duvar, yükseklik ve açıklıklarla, eskiliği ve yeniden üretimi; bilinmeyenler listelenir. Açık: kanat çizimi, döşeme/tavan
  kesiti, sınırlı 2,5B kesit, yayın/revizyon birlikteliği (paftanın yayın kaydı ister), gerçek pencerede elle kabul.”
