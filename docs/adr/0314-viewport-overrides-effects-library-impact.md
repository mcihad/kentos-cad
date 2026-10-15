# ADR 0314: Görünüm katmanları, değişikliğin etkisi, kitaplık API'si ve etki raporu

- **Durum:** kabul edildi (2026-10-16). `plan/cad.md` §9 `CAD-STYLE-12`, `13`, `14`, `15`. Sahibin isteği: “cad-style maddelerini
  bitir ama her maddeyi ayrı ayrı yapmak yerine 3 veya 4'er yaptıktan sonra test et”. Bu dört madde dördüncü ve son gruptur.
  Kapsamı ben belirledim.
- **Bağlam belgesi:**
  - ADR 0311 (katman durumları), 0312 (görünüşün kaynağı), 0313 (çizgi tipleri);
  - 0164 ve 0251 (pafta kitabı), 0092 (stil kitaplığı, `.kstil`), 0144 (bloklar);
  - 0026 ve 0144 §5 (bulut eşitlemesi), CLAUDE.md §9.2.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

- **Paftanın haritası bütün katmanları modeldeki gibi çiziyordu.** Haritanın bir katman durumu (`layers: theme`) olabiliyordu,
  ama AutoCAD'in görünüm (viewport) geçersiz kılmaları yoktu: bir paftada bir katmanı dondurmak, başka renkte, başka çizgi
  tipinde ya da kalınlıkta basmak, yazdırmamak. Bunu yapmanın tek yolu modeldeki katmanı değiştirmekti; bu da öbür paftaları
  bozuyordu.
- **Değişikliğin anlamı dağınıktı.** Hangi değişiklik geri alma adımı, hangisi adımın dışında ama çizimi kaydedilmemiş yapar,
  hangisi yalnız kullanıcının tercihidir, bulutta nasıl gider? Bunlar ADR'lere dağılmıştı. Python ve ajan bunu bir yerden
  okuyamıyordu.
- **Stil kitaplığı ve blok kitaplığı arayüzdeydi.** Projenin kitaplığını (semboller, çizimler, şablonlar) değiştirmek, `.kstil`
  almak ve vermek, nesnelere sembol vermek yalnız Stil yöneticisinden yapılıyordu. Blok kopyalamak, blokları dosyaya vermek ve
  dosyadan almak hiç yoktu. Python ve MCP bunların hiçbirini yapamıyordu.
- **Bir katmanın ya da stilin değişmesinin neye dokunacağı bilinmiyordu.** Kaç nesne izler, kaçı kendi değerini korur, blok
  içinde ne değişir, hangi katman durumu ya da pafta eski görünüşü saklar?

## Karar

### 1. Haritanın kendi katmanları (`CAD-STYLE-12`)

Paftanın harita öğesi (`MapItem`) iki şey taşır:

- **Katman durumu:** `layers: { type: "theme", name }`. Var olan alan. Haritayı projenin adlı bir katman durumuyla (ADR 0311)
  çizer: durumun sakladığı düğümlerde görünürlük, dondurma, yazdırma ve stil durumunkidir. Katman durumunun görünüm kapsamı budur
  (`CAD-STYLE-04`'ün açık bıraktığı).
- **Katman başına geçersiz kılma:** `layerOverrides`, katman başına en çok bir satır. Her satırda:
  - `layer`: katmanın kimliği;
  - `frozen`: bu haritada çizilmez;
  - `plot`: bu haritada yazdırılır ya da yazdırılmaz; yoksa katmanınki;
  - `color`: `#rrggbb` ya da tema adı (`fg`, `fg-dim`, `ink`, `paper`);
  - `lineType`: yerleşik tip. Verilince katmanın deseni düşer;
  - `linePattern`: projenin LIN desenli tipinin adı (ADR 0313);
  - `lineWeight`: kalınlık, µm'de, 0 ile 20 mm arası.

Kurallar:

- **Sıra:** önce katman durumu uygulanır, sonra geçersiz kılma. Uygulamanın tek yeri `kentos_plot::layers::map_layers`'tır.
  Masaüstünün pafta kipi, PDF ve başsız kitap PDF'i aynı işlevi çağırır.
- **Model değişmez:** geçersiz kılma yalnız o haritanın çizdiği düğümlerin kopyasındadır. Masaüstünde stilli çizimin önbelleği
  haritanın kendi anahtarıyla tutulur (`StyledAt`'in `own_key`'i), model görünümünün çizimine karışmaz. Etiketler de dondurulan
  katmanı atlar.
- **Biçim:** pafta kitabı KCAD'de opaktır (ADR 0251). Şema değişmez; okumayan eski sürüm alanı korur.
- **Doğrulama:** pafta çekirdeğinin `validate_book`'u yapar. Retler:
  - aynı katman iki kez (`duplicate`);
  - renk değil (`bad_color`);
  - çizgi tipi değil (`bad_value`);
  - kalınlık aralık dışında (`out_of_range`);
  - durumun adı boş.

  Yol `ops[i].sheets[k].items[j].kind.layerOverrides[n].<alan>`'dır.
- **Komut:** yazma `cad.sheets.edit` ile olur (öğenin özellikleri). Python, MCP ve ajan aynı komutu kullanır.
- **Arayüz:** pafta tasarımcısının Denetçi'sinde haritanın Katmanlar bölümü var. İçinde:
  - Katman durumu seçimi (Tüm katmanlar ya da projenin durumları);
  - katman ekleme;
  - satır başına Dondur, Yazdır, Renk, Çizgi tipi (yerleşik dört tip ve projenin desenleri) ve Kalınlık;
  - Kaldır.

  “katmana göre” seçimi geçersiz kılmayı kaldırır.

### 2. Değişikliğin etkisi tek tabloda (`CAD-STYLE-13`)

`kentos_contracts::change_effects::CHANGE_EFFECTS` 16 satırdır. Her satır şunları söyler:

- değişikliği;
- onu yapan komutu;
- geri almayı: `step` (geri alma adımı), `outside` (adımın dışında, çizim kaydedilmemiş olur) ya da `none` (tercih);
- kirliliği;
- yazdığı proje parçalarını.

Parçalar: nesneler, bloklar, katman ağacı, ayarlar, kitaplık, pafta kitabı, tercihler. Her parçanın bulutta nasıl gittiği
`EffectPart::cloud`'dadır:

- nesneler ve bloklar `project.changes`'le, tek tek sürümle (ADR 0026, 0144 §5);
- katmanlar, ayarlar, kitaplık ve pafta `ProjectPatch`'in kendi alanıyla, bütün olarak;
- tercihler eşitlenmez.

Satırlar:

| Değişiklik | Komut | Geri alma | Parçalar |
|---|---|---|---|
| Nesnenin görünüşü | `cad.entities.set` | adım | nesneler |
| Çizim sırası | `cad.entities.order` | adım | nesneler |
| Özellik kopyala | `cad.entities.match` | adım | nesneler |
| Katmanın rengi, tipi, deseni, kalınlığı | `cad.layers.edit` | adım | katmanlar |
| Katmanın işleyicisi | `cad.layers.renderer` | adım | katmanlar |
| Görünürlük, kilit, dondurma, yazdırma, ad | `cad.layers.set` | dışında | katmanlar |
| Katman durumu kaydetmek | `cad.layers.states` | dışında | ayarlar |
| Yazı ve ölçü stilleri | `cad.styles.edit` | adım | nesneler, ayarlar |
| Çizgi tipleri, LTSCALE | `cad.linetypes.edit` | dışında | ayarlar |
| Standardı uygula | `cad.standards.apply` | adım | nesneler, katmanlar, ayarlar |
| Projenin kitaplığı | `cad.library.edit` | dışında | kitaplık |
| Kitaplıktan sembol vermek | `cad.library.edit` | adım | nesneler |
| Blok tanımları | `cad.blocks.edit` | adım | bloklar |
| Haritanın katmanları | `cad.sheets.edit` | dışında | pafta |
| Proje ayarları | `cad.settings.set` | dışında | ayarlar |
| Görünüm kipleri | — | yok | tercihler |

Tablonun yayımlanması ve sınanması:

- **Katalog:** tablo kataloğun `effects`'inde yayımlanır. Python ve ajan oradan okur.
- **Uygulamanın testi:** `change_effects.rs` her satırın komutunu kaydedilmiş bir çizimde çalıştırır. Geri almanın, kirliliğin ve
  değişen parçaların tablodakiyle aynı olduğunu denetler.
- **Testin bulduğu sapma:** standardı uygulamanın ayarları da yazdığı tabloda eksikti; tablo düzeltildi.
- **Bulut testi:** katmanın görünüşünün ve kitaplığın `ProjectPatch`'le gittiğini sınar.

### 3. Kitaplık ve blok kitaplığı API'si (`CAD-STYLE-14`)

**`cad.library.edit`** projenin kitaplığını değiştirir. İşlemleri:

- `create`, `update` (alan yaması, `null` alanı kaldırır; kimlik ve tür değişmez), `duplicate`, `rename`, `move` (kategori);
- `remove`: kullanılmıyorsa. Kullananlar nesnenin ve katmanın sembolü, şablon ve sembolün çizimidir;
- `import`: `.kstil` metni. Var olan kimlik kalır ya da `conflict: replace` ile yerine geçer;
- `export`: öğeler, kullandıkları çizimlerle `.kstil` metni olarak; çizim değişmez;
- `assign`: sembolü nesnelere (`uids`) ya da katmanın tek sembolüne verir.

Kitaplığın yazılması geri alma adımı değildir (ADR 0092), çizimi kaydedilmemiş yapar. Sembol vermek tek adımdır: nesnelere “Sembol ata” (`cad.entities.set`'in yolu;
kilitli katmandaki nesneler sayıyla atlanır, ADR 0278), katmana “Katman stili” (`cad.layers.renderer`'ın yolu, tek sembollü
işleyicinin türüne uyan yuvası; öbür yuvalar kalır).

Bağlantılı kararlar:

- **Kod ayrımı:** `.kstil`'in kuralları masaüstünün stil crate'inden yeni `kentos-style-library`'ye ayrıldı (kitaplık, dosya,
  nesne şablonu). Uygulama katmanı onu çizim hattını bilmeden kullanır (`pnpm arch:deps`: 41 crate). `kentos-native-style`
  modülleri yeniden dışa aktarır; çağıranlar değişmedi.
- **Dışa aktarılan metin kanoniktir:** `canonical_json`, aynı kitaplık her zaman aynı baytlar.

**`cad.blocks.edit`'e üç işlem:**

- **`duplicate` (Blok kopyala):** aynı içerik, taban ve öznitelikler yeni adla ve kimlikle; yerleştirmeler eskisinde kalır.
- **`export`:** verilen bloklar ya da hepsi, yerleştirdikleriyle birlikte, `kentos.blocks` metni olarak (kanonik).
  Masaüstünde `.kblok` dosyası.
- **`import`:** `kentos.blocks` metni. Çizimde olan ad kalır ve söylenir; `replace` ile tanım yerinde değişir, kimliği kalır.
  Dosyanın iç yerleştirmeleri çizimin aynı adlı bloğuna bağlanır. Hepsi tek adımdır.

Yeni ret kodları:

- blok dosyası verilmedi (`no_library`) ya da okunamadı (`invalid_library`);
- kitaplık için `no_item`, `no_kstil`, `no_targets`, `invalid_library_item`, `invalid_kstil`, `library_item_exists`,
  `library_item_not_found`, `library_item_in_use`, `not_a_symbol`.

Arayüz ve otomasyon:

- **Bloklar paneli:** Kopyasını oluştur (ad “… kopya”, boş olan ilk ad), Dışa aktar… (blok ya da hepsi) ve Blokları al….
  Üçü de komutla yazar (`blocks_files.rs`).
- **Python ve MCP:** ikisinde de var. İkisi de var olanı değiştirdiği için MCP'nin yıkıcı listesindedir.

### 4. Değişikliğin etkisi raporu (`CAD-STYLE-15`)

Salt okunur **`cad.styles.impact`** dört hedeften birini değiştirmenin neye dokunacağını söyler: katman, çizgi tipi, yazı stili,
ölçü stili.

**Kurallar:**

- **İzleyenler:** kendi değeri olmayan (katmanda dört görünüş değerinin hepsi kendisinde değil) nesneler izler. Kimlikleri en çok
  1000 tanedir, sayı bütündür.
- **Kendi değerini koruyanlar:** özelliğe göre sayılır (renk, çizgi tipi ve desen, kalınlık, saydamlık; stillerde stilden farklı
  alanlar).
- **Kilitli katman:** katmanın görünüşü kilitli katmana da yansır. Bu yüzden katman ve çizgi tipinde `locked` 0'dır, pencere
  bunu söyler. Yazı ve ölçü stilinin izlemesi kilitli katmandakini değiştirmez; onlar sayılır.
- **Bloklar:**
  - katmanın blok parçaları: o katmanda, `look.byLayer`'ı boş olmayanlar;
  - yerleştirmeler: o blokların ya da onları içeren blokların çizimdeki yerleştirmeleri.
- **Çizgi tipi:** kendi deseni olan nesneler ve deseni kullanan katmanlardaki, kendi tipi olmayan nesneler.
- **Katman durumları:** saklanan düğüm stili bugünkünden farklı olanlar. Uygulanınca eski görünüşü geri getirirler.
- **Pafta haritaları:** katmanı geçersiz kılanlar.
- **Dış başvurular:** her zaman 0. KentOS dış başvuru taşımaz; cevap bunu açıkça söyler.

**Pencere:** Değişikliğin etkisi. Satırlar Türkçe adlarla, altta tek not; Nesneleri seç izleyenleri seçer. Açıldığı yerler:

- Katmanlar'ın sağ tık menüsü (Değişikliğin etkisi…);
- Çizgi tipleri penceresi (Etkisi…).

## Sonuçlar

- **Paftalar modeli bozmadan ayrışır:** aynı modelin bir paftasında bir katman dondurulabilir, öbüründe gri ve ince basılabilir.
- **Python ve ajan:**
  - bir değişikliğin geri alınıp alınmayacağını, bulutta nasıl gideceğini tablodan okur;
  - değiştirmeden önce etkisini sorar;
  - kitaplığı ve blokları arayüzle aynı komutlarla yönetir.
- **Sınırlar:**
  - geçersiz kılma saydamlık ve işleyici taşımaz;
  - dış başvuru yoktur;
  - kitaplıkta sistemin ve kullanıcının kitaplıkları ev sahibinindir, komut yalnız projeninkini görür.

## Doğrulama

- **Ortak durumlar** (KentOS kodu olmadan yazılmış başvurulardan, kesirlerle):
  - `cad.library.edit` 33 durum (`library_command_cases.py`);
  - `cad.styles.impact` 20 durum (`style_impact_cases.py`);
  - `cad.blocks.edit`'e 6 durum (`blocks_command_cases.py`; toplam 36);
  - `cad.sheets.edit`'e 6 durum (`sheets_edit_command_cases.py`; toplam 26).

  Yetenek testi her yeni kodu bir durumda bulur.
- **Testler:**
  - etki tablosu (`change_effects.rs`);
  - `.kstil` ve `.kblok` gidip gelir, kanonik (`library_files.rs`);
  - haritanın katmanları (`kentos-plot`'un `map_layers.rs`'i);
  - bulut eşitlemesinde kitaplık ve katman.
- **Resimler:**
  - `.run/shots/sheet-desktop/*-0314-harita-katmanlari` (pafta tasarımcısı);
  - `.run/shots/arac-etki-katman-*`, `arac-etki-cizgitipi-*`, `arac-blok-kitapligi-*` (`impact_scenes.rs`).
- **Süreler** (release; 100 000 nesne, 50 katman, beşte biri desenli, 200 blok × 5 parça, 1 000 yerleştirme; `library_files::timing`):

| İş | Sonuç | Süre |
|---|---|---|
| Katmanın etkisi | 1 980 izleyen, 20 blok parçası, 100 yerleştirme | 5,3 ms |
| Çizgi tipinin etkisi (10 katman) | 19 800 izleyen, 200 blok parçası, 1 000 yerleştirme | 17,4 ms |
