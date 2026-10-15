# ADR 0311: Katmanlar: dondurma, yalıtım, durum dosyaları, bulma ve çevirme

- **Durum:** kabul edildi (2026-10-15). `plan/cad.md` §9 `CAD-STYLE-01`, `02`, `04`, `05`. Sahibin isteği: “cad-style maddelerini bitir
  ama her maddeyi ayrı ayrı yapmak yerine 3 veya 4'er yaptıktan sonra test et”. Bu dört madde bir gruptur. Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0248 (katman, stil ve ayar komutları), 0177 (katman yönetimi ekleri), 0260 §4 (Yazdır), 0020 (ağacın kendi
  değişiklikleri), 0211 (katman süzgeci, nesne düzeyinde), 0244 (CAD işlem manifesti), CLAUDE.md §9.2.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

ADR 0248 katman ağacının, durumların, stillerin ve ayarların komutlarını getirdi ama dört eksik bıraktı:

- **Dondurma:** AutoCAD'in freeze ve thaw'u yoktu. DXF'in dondurulmuş katmanı gizli okunuyordu.
- **Yalıtımı kaldır:** yalnız kabuğun belleğindeydi (`App::isolated_layers`). Python ve ajan yapamıyordu.
- **Durumları taşımak:** katman durumları bir çizimden ötekine taşınamıyordu (AutoCAD'in LAS dosyası).
- **Toplu iş:** katmanları adla ve özellikle bulmanın (AutoCAD'in katman süzgeçleri) ve bir ofis standardına çevirmenin (LAYTRANS)
  yolu yoktu.

## Karar

### 1. Dondur (`CAD-STYLE-02`)

Katman ağacının düğümünde `frozen` (`.kcad` şema 55, yalnız `true` yazılır; grubun dondurulması altındakileri de kapsar).

- **Dondurulmuş katman:**
  - çizilmez, seçilmez, kenetlenmez;
  - Tümünü göster ve kapsam onu saymaz;
  - paftada ve PDF'te çizilmez.
  - Belgenin kuralında görünürlüğün parçasıdır: `LayerTree::is_visible` dondurulmuşu görünmez sayar. Bu yüzden seçim, kenet, etiket,
    İşlemler ve DXF'in “görünenler”i ayrı bir kural bilmeden uyar.
- **Gözle farkı:**
  - Hepsini göster gözleri açar ama dondurulmuşu çözmez. Göz (`is_on`) ile dondurma ayrı bayraktır.
  - Etkin katman dondurulamaz; dondurulmuş katman etkin olamaz.
- **Komut:** `cad.layers.set`'e üç işlem:
  - `frozen` (`layer`, `value`);
  - `thawAll`;
  - `unisolate` (§2).
- **Yeni retler:**
  - `active_frozen`: etkin katmanı ya da onu içeren grubu dondurmak;
  - `layer_frozen`: dondurulmuş katmanı etkin yapmak.
- **DXF:** 70 grubunun 1. biti dondurmadır.
  - Okuyucu önceden onu gizlilik sayıyordu; artık `frozen` olarak alır.
  - Yazıcı dondurulmuş katmanı o bitle yazar; görünürlüğü yalnız gözüdür.

### 2. Kalıcı ve geçici (`CAD-STYLE-02`)

| Bayrak | Nerede | Düzenleme | Geri alma |
|---|---|---|---|
| Göz (`visible`), kilit, Yazdır, Dondur, ad | proje (dosyada) | evet | adım değil (ADR 0020) |
| Etkin katman | proje (dosyada) | hayır | adım değil |
| Yalıt'ın gizledikleri | oturum (belgede, dosyada değil) | — | — |
| Katmanlar panelinin araması ve Süzgeç ▾'i | oturum (kabukta) | — | — |

- **Yalıt (`isolate`):** gizlediği düğümleri belgeye not eder (`LayerTree::isolated`). İkinci bir yalıtım notu büyütür.
- **Yalıtımı kaldır (`unisolate`):** notlu olup hâlâ gizli olanları gösterir, notu siler. Elle gösterilenler olduğu gibi kalır; not
  yoksa bir şey değişmez.
- **Planın kopyası** notu taşır: plan yazmadan ne olacağını söyler.
- **Kabuğun eski belleği** kalktı: Katmanı yalıt aracı, Yalıtımı kaldır ve Katmanlar'ın “Yalnızca bunu göster”i aynı komutu kullanır.

### 3. Katman durumları: dondurma, yazdırma, dosya (`CAD-STYLE-04`)

- **Saklananlar:** durum düğümü her kayıtta düğümün kendi `frozen` ve `plot`'unu da saklar (şema 55).
  - Uygulama onları görünürlük gibi, ağacın kendi değişikliği olarak yazar.
  - Etkin katmanı ya da onu içeren grubu dondurmak atlanır.
  - Eski durum (alanlar yoksa) dondurmaya ve yazdırmaya dokunmaz.
  - Çıktıda `frozen` ve `plot` sayılır.
- **`export`:** durumları (`states`, verilmezse hepsi) `kentos.layer-states` sürüm 1 dosyası olarak verir; çizim değişmez. Her düğüm
  ağaçtaki yoluyla (`path`: “Mimari / A-DUVAR”) yazılır.
- **`import`:** dosyanın durumlarını ekler.
  - **Eşleme:** düğüm kimliğiyle, yoksa yoluyla bulunur. Adlar Türkçe katlanır, ilk eşleşen alınır. Hiçbiriyle bulunamayan kimliğiyle
    kalır ve uygulanınca `missing` sayılır; kimliği de yolu da yoksa düşer.
  - **Aynı adlı durum:** atlanır (`skipped`), ya da `replace` ile üzerine yazılır (kimliği kalır, `replaced`). Yenisi ilk boş `durum-n`
    kimliğini alır (`added`).
  - **Retler:**
    - `no_file`: dosya verilmedi;
    - `invalid_states`: biçim ya da sürüm başka, adlar iki kez, boş düğüm (`layer_states_file_problem`).
  - **Yol yalnız dosyadadır:** projenin durumunda yol `layer_states_problem` ile reddedilir, KCAD'de bilinmeyen alandır.
- **Masaüstü:** Katman durumları penceresinde Dışa aktar… ve İçe aktar… (`.json`). İçe aktarmada aynı adlılar atlanır ve söylenir.
- **Viewport kapsamı** (bir durumun paftanın bir görünümüne uygulanması) `CAD-STYLE-12` ile gelir: görünüm başına katman ayarı
  onundur.

### 4. Katmanları bul (`CAD-STYLE-05`)

Salt okunur `cad.layers.find`: AutoCAD'in katman süzgeçleri tek sorgu.

- **`name`:** AutoCAD'in `wcmatch` örüntüsü (`kentos_contracts::pattern_matches`).
  - `*` herhangi bir dizi, `?` tek karakter, `#` rakam, `@` harf, `.` ikisi de değil;
  - `[..]` ve `[~..]` kümeler (`a-z` aralıkları);
  - `` ` `` sonraki karakter olduğu gibi, `,` seçenekler, başta `~` değili.
  - Harfler Türkçe katlanır (I ı, İ i); okunmayan örüntü `invalid_pattern`.
- **Özellikler:**
  - `group` (yalnız altındakiler);
  - `color`, `lineType`, `lineWeight` (katmanın kendi stili);
  - `on`, `frozen`, `locked`, `plot` (üstündeki gruplarla birlikte);
  - `empty` (nesnesiz; servis katmanı boş sayılmaz).
- **Cevap:** yalnız katmanlar, ağacın sırasıyla. Her biri yolu, nesne sayısı ve özellikleriyle gelir.
- **Katmanlar paneli:**
  - Katman ara'ya `*`, `?`, `#`, `@`, `[`, `~` ya da `,` içeren metin örüntü olarak okunur, düz metin önceki gibi adın parçası.
  - Yanındaki Süzgeç ▾ (huni) görünenleri, gizlileri, dondurulmuşları, kilitlileri, yazdırılmayanları ve nesnesizleri süzer. Menü açık
    kalır. Süzgeç vurgu renginde görünür; Süzgeci temizle hepsini kaldırır.
  - Süzme komutun kuralıyla (`layers_find::find`) yapılır; gruplar yalnız ad aramasında kendi adlarıyla bulunur.

### 5. Katmanları çevir (`CAD-STYLE-05`)

`cad.layers.translate`, AutoCAD'in LAYTRANS'ı. Kurallar (`rules`): `from` bir ad örüntüsü, `to` standart katmanın adı, isteğe bağlı
`color`, `lineType`, `lineWeight`.

- **Kaynak:** nesne tutan her katman (grup ve servis katmanı değil) adına uyan ilk kuralı alır. Adı kuralın `to`'su olan katman
  standart katmanın kendisidir (katlanarak).
- **Standart katman:** `to` adlı ilk nesne tutan katmandır.
  - Yoksa ve kuralı bir katman alırsa `parent` grubunun sonuna (verilmezse en üste) eklenir. Stili ilk kaynağınkinin üstüne kuralın
    verdikleridir.
  - Var olanı kuralın verdiği stili alır (`restyled`).
- **Yazma:** kaynakların nesneleri `cad.entities.set` ile standart katmana gider, kaynaklar silinir. Etkin katman kaynaksa standart
  katman etkin olur. Hepsi tek adım “Katmanları çevir”; gizli standart katmana taşınma `layer_hidden` ile söylenir.
- **Hep ya da hiç:** kilitli bir kaynak ya da standart katman hiçbir şeyi yazdırmaz (`layer_locked`). Servisten çizilen standart
  katman `service_layer` alır.
- **Plan önizlemedir:**
  - kaynak, kural, standart katman ve nesne sayısı (`translations`);
  - eklenecek (`created`) ve stili değişecek (`restyled`) katmanlar.
- **Masaüstü:** Katmanları çevir penceresi (Katmanlar'ın ⋯ menüsü ve şeritte Katmanlar ▾).
  - Kurallar tablosu: Kaynak, Standart katman, Renk, Kalınlık; Kural ekle, satırı sil.
  - Önizleme her değişiklikte komutun planıyla yeniden okunur; Çevir yazar.
- **Retler:**
  - kuralın kendisi: `no_rules`, `empty_name`, `invalid_pattern`, `invalid_color`, `invalid_line_weight`;
  - sürüm;
  - çizime göre: `layer_not_found` ve `not_a_group` (`parent`), `service_layer`, `layer_locked`.

### 6. Ağacın komutları kataloğun ürün komutlarıdır (`CAD-STYLE-01`)

Katman ve grup oluşturma, adlandırma, taşıma, silme, birleştirme ve temizleme ADR 0248'den beri `cad.layers.edit` ve
`cad.layers.set`'tedir: katalogda, Python'da ve MCP'de. Bu madde onları denetledi ve eksiklerini (dondurma, yalıtımı kaldırma, çevirme)
tamamladı.

### 7. Araçlar ve arayüz

- **Katmanı dondur** (`tool.layerFreeze`; LAYFRZ): tıklanan nesnenin katmanı. Etkin katmanın reddi komutun sözüyle söylenir.
- **Bütün katmanları çöz** (`layer.thawAll`; LAYTHW).
- **Katmanları çevir…** (`layer.translate`; LAYTRANS).
- **Şeritteki yerleri:** üçü de iki türde Giriş › Katmanlar ▾'de, ikisi Katmanlar panelinin ⋯ menüsünde de.
- **Katmanlar panelinin sağ tıkı:** Dondur / Çöz ve Bütün katmanları çöz.
- **Satır:** dondurulmuş düğümün yanında kar tanesi ve ipucu; dondurulmuş grubun altındakiler soluk.
- **İkonlar:** `freeze`, `layerFreeze`, `layersThawAll` ve `layersTranslate` `catalog/v1/additions.json`'da; Çöz web setinin `sun`'ı.

### 8. Python ve MCP

- **Python:** `kentos.cad.layers.find` ve `.translate`, `.set`'in yeni işlemleri, `.states`'in `export` ve `import`'u (SDK yeniden
  üretildi).
- **MCP:** araçlar katalogdan gelir; `cad.layers.translate` yıkıcıdır.
- **Manifest:** dört komut ve araç `semantic`.

## Performans

Sürüm derlemesinde, 500 katmanlı ve 100 000 nesneli çizimde (katmanların yarısı `eski-N`), bu makinede:

| İş | Sonuç | Süre |
|---|---:|---:|
| Katmanları bul (`eski-1*`, nesnesi olan) | 55 katman | 0,8 ms |
| Katmanları çevir, plan (5 kural) | 150 katman, 30 000 nesne | 119 ms |
| Katmanları çevir, yazma | 150 katman, 30 000 nesne | 103 ms |

**Ek düzeltme (belgede).** İlk ölçümde çevirme 5,9 s sürdü. Nedeni katman silmeydi: her silme nesnelerini bulmak için ve
`LayerRemove` uygulanırken katmanın nesnesi kaldı mı diye (`holds_objects`) bütün çizimi iki kez dolaşıyordu (100 000 nesnede
silme başına 36 ms).

- **Düzeltme:** ikisi de deponun katman dizininden (`Store::layer_slots`) okunuyor; silme mikro saniyelere indi.
- **Etkisi:** Katman sil, Katmanları birleştir ve Kullanılmayanları temizle de aynı işlevle hızlandı.
- **Taşıma:** standart katman başına tek `cad.entities.set` çağrısıyla yapılıyor.
- **Bulma:** nesne sayıları da dizinden (`Document::count`).

```text
cargo test --release -p kentos-native-application --test all layers_find_cmd::timing -- --ignored --nocapture
```

## Doğrulama

- **Ortak durumlar:** bağımsız başvurular KentOS kodu olmadan yazıldı.
  - `layer_tree_command_cases.py` yazar:
    - `cad.layers.set.json` 42 durum: dondurma, çözme, etkin katmanın retleri, Hepsini göster'in dondurulmuşu bırakması, yalıtımı
      kaldırma;
    - `cad.layers.states.json` 30 durum: dondurma ve yazdırma, eski durum, dışa ve içe aktarma, yolla eşleme, retler.
  - `layer_state_cases.py`: `fixtures/layers/v1/states.json`, dondurulan ve yazdırılmayan düğümler.
  - `layer_find_cases.py`, `wcmatch`'i belgesinden yazar ve iki dosya üretir:
    - `cad.layers.find.json` 28 durum;
    - `cad.layers.translate.json` 28 durum.
- **KCAD:** `frozen.kcad` (dondurulmuş katman ve grup, yazdırılmayan katman, dondurmayı saklayan ve saklamayan durum) ve dört bozuk
  örnek bağımsız Python yazıcısıyla yazıldı. Bağımsız okuyucu ve Rust kodeği (`tests/all/frozen.rs`) aynı sonucu verir.
  `schema-version-55.kcad` artık `-56`'dır.
- **Resimler** (`apps/desktop/src/layer_style_scenes.rs`, `.run/shots/arac-katman-*`):
  - dondurulmuş katman ve grup;
  - örüntüyle arama ve Dondurulmuşlar süzgeci;
  - Katmanları çevir'in önizlemesi;
  - Katman durumları'nın dışa ve içe aktarması.
