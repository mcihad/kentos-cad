# ADR 0309: Pano komutları — kopya, taban noktası, yapıştırma ve blok olarak yapıştırma

- **Durum:** kabul edildi (2026-10-15). `plan/cad.md` §8 `CAD-EDIT-22`: “COPYBASE, PASTECLIP/PASTEORIG/PASTEBLOCK
  eşdeğerlerinde belge birimi, kaynak katman/stil/blok ad çakışmasını planlı çöz.” Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0056 (pano ve Yapıştır aracı), 0074 (yapıştırma komutlarla), 0193 (çizimler arası alışverişin
  kuralları), 0199 §7 (katmanı nesneleriyle alma), 0253 (komut protokolü), CLAUDE.md §9.2 (araç = komut).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

Pano oturumun nesne listesiydi: Panoya kopyala yalnız nesneleri ve kutularının sol alt köşesini tutuyordu, Yapıştır onları
`cad.entities.create` ve `.set` ile yazıyordu. Bu yüzden:

- **Katmanlar:** başka çizimden gelen nesnenin katmanı yoksa ya da kilitliyse nesne sessizce etkin katmana gidiyordu.
- **Bloklar ve stiller:** taşınmıyordu. Blok yerleştirmesi bu çizimde olmayan bir tanımı gösterebiliyor, yazı bu çizimde
  olmayan bir stile bağlı kalabiliyordu.
- **Birim:** milimetre çizimden metre çizime geçen nesnenin birimi söylenmiyordu.
- **Python ve ajan:** aynı işi yapamıyordu; pano yalnız arayüzdeydi.
- **Eksik komutlar:** AutoCAD'in COPYBASE'i (taban noktasıyla kopyala) ve PASTEBLOCK'u (blok olarak yapıştır) yoktu.

## Karar

### 1. Kopya: `cad.entities.copy`

Salt okunur komut, çizim değişmez. Girdisi nesnelerin kalıcı kimlikleri (`uids`) ve isteğe bağlı taban noktası (`base`).
Cevabı bir **kopya** (`Clip`):

- **Nesneler:** çizimdeki sıralarıyla (çizim sırası korunur), 1, 2 … diye numaralı; her birinin kaynaktaki kimliği
  (`uids`).
- **Katman ağacı:** nesnelerin katmanları ve üstlerindeki gruplar, kaynağın görünüşüyle.
- **Bloklar:** yerleştirmelerin tanımları ve onların yerleştirdikleri.
- **Stiller ve kitaplık:** nesnelerin ve blok parçalarının adını verdiği yazı ve ölçü stilleri, kullandıkları kitaplık
  öğeleri (semboller, resimleri).
- **Birim ve taban noktası:** kaynağın çizim birimi; taban noktası verilen nokta ya da nesnelerin kutusunun sol alt köşesi
  (blok yerleştirmesi kendi noktasıyla).

Kopyalanan nesneler arasındaki bağlar kopyada kalır (bağlı yazı, ilişkili tarama, çizelgenin nesneleri); kopyalanmayan
nesnelere bağlar düşer. Kurallar alışverişin seçim kuralıdır (`kentos_domain::exchange::selection`, ADR 0193 §1).

Retler: `no_entities`, `invalid_uid`, `not_finite` (taban noktası), `entity_not_found`.

Kopya, ev sahibinin panosunun tuttuğudur. Python ve ajan onu olduğu gibi `cad.entities.paste`'e verir.

### 2. Yapıştırma: `cad.entities.paste`

Kopyayı bir çizime (aynısına ya da başkasına) tek geri alma adımında (“Yapıştır”) yazar. Taban noktası `at`'e gider; `at`
yoksa kopyalandığı yere (Özgün koordinatlara yapıştır).

- **Katmanlar:** her nesne kendi katmanının **yolundaki** katmana gider. Adlar büyük küçük harf ve Türkçe harfler
  gözetilmeden karşılaştırılır (“Kadastro / Bina” ile “KADASTRO / BİNA” aynıdır). Bu çizimde olmayan yol açılır: kaynağın
  görünüşüyle, gruplarıyla, katmanı alma kuralıyla (`layer_take`, ADR 0199 §7).
- **Bloklar:** bu çizimde aynı adda tanım varsa bu çizimin tanımı kullanılır (rapor “bu çizimin tanımıyla” der);
  öbürleri eklenir.
- **Stiller:** aynı adda yazı ya da ölçü stili varsa bu çizimin stili kullanılır; olmayanlar projeye eklenir.
- **Kitaplık:** bu çizimde olmayan öğeler eklenir.
- **Nesneler:** yeni kalıcı kimlik alır. Bağları düşer, başka çizimden katman alındığındaki gibi: bağlı yazı düz yazı, ilişkili
  tarama düz tarama, çizelge kaynaksız gelir. Çizimler arasında kimliklerin eşlenmesi ayrı bir iştir.
- **Birim:** uzunluklar her çizimde metredir (ADR 0165); nesneler gerçek boylarıyla gelir. Kaynağın birimi bu çizimin
  değilse `units_differ` uyarısı bunu söyler.
- **Katmanların durumu:** nesnenin gideceği katman kilitliyse **yapıştırmanın hepsi reddedilir** (`layer_locked`; eski
  araç nesneyi sessizce etkin katmana koyuyordu). Gizli katman uyarıyla yazılır (`layer_hidden`).

**Blok olarak yapıştır** (`asBlock`): kopyanın nesneleri tek yeni blok olur. Adı verilen addır, alınmışsa “Ad (2)” …
Tabanı nesnelerin sol alt köşesidir. Yerleştirme etkin katmanda, kopyanın taban noktası `at`'e gelecek yerdedir; blok ve
yerleştirme tek adımdadır. Kural dosyadan blok eklemenin kuralıdır (`file_block`, ADR 0193 §3).

Cevabı yeni nesnelerin kimlikleri ve rapordur: açılan katman ve grupların yolları, eklenen bloklar, bu çizimin tanımıyla
kalan bloklar, eklenen stiller, blok olarak yapıştırmada bloğun adı.

Retler, bu sırayla:
- `no_entities` (boş kopya);
- `invalid_clip` (kimlikler nesnelerle eşleşmiyor ya da bir nesnenin katmanı veya bloğu kopyada yok);
- `not_finite`;
- `empty_name` (`asBlock` boş);
- `invalid_revision`, `revision_conflict`;
- `layer_locked`.

### 3. Araçlar ve komutlar

- **Panoya kopyala** (Ctrl+C) ve **Kes** (Ctrl+X): `cad.entities.copy` ile kopyalar; Kes ardından `cad.entities.delete`
  ile “Kes” adımında siler, kilitli katmandakileri atlar.
- **Yapıştır** (Ctrl+V): ADR 0056'nın aracıdır; hayalet imleci taban noktasından izler. Tıklanan ya da yazılan noktaya
  `cad.entities.paste` ile yazar.
- **Özgün koordinatlara yapıştır** (Ctrl+Shift+V): `at`'siz yapıştırma.
- **Taban noktasıyla kopyala** (`tool.copyBase`, Ctrl+Shift+C; COPYBASE): önce nesneler (seçim ya da seçilerek), sonra
  tıklanan ya da yazılan taban noktası. Kopya ev sahibinin panosuna gider (`ViewChange::Copied`), çizim değişmez.
- **Blok olarak yapıştır** (`edit.pasteBlock`, Ctrl+Alt+V; PASTEBLOCK): Yapıştır aracı `asBlock` ile; bloğun adı “Pano”.
- **Yazılanlar:** yapıştırmanın neler getirdiğini komut satırı söyler: “4 nesne yapıştırıldı: “Kadastro” grubu ve 2 katman
  açıldı (Kadastro / Parsel, Kadastro / Bina); 1 stil eklendi (Başlık).”
- **Yeri:** iki yeni komutun kaydı `catalog/v1/additions.json`'dadır (ikonlar `copyBase`, `pasteBlock`). Şeritte CAD'de ve
  CBS'de Giriş › Pano'dadırlar.

### 4. Python ve MCP

- **Python:** `kentos.cad.entities.copy` ve `.paste` (SDK katalogdan üretildi).
- **MCP:** araçlar katalogdan gelir; `cad.entities.paste` yıkıcı listededir. Bir ajan bir çizimden kopyalayıp başka bir
  çizime (başka bir tutamaca) yapıştırabilir.
- **Manifest:** Panoya kopyala, Kes, Yapıştır, Özgün koordinatlara yapıştır, Taban noktasıyla kopyala ve Blok olarak
  yapıştır `semantic` düzeydedir (`catalog/v1/operations.json`).

### 5. Belge

- **`to_snapshot_v2_of`:** `kentos_domain::Document`'in yeni işlevi. Çizimin başını (ayarlar, katmanlar, bloklar,
  kitaplık) yalnız istenen nesnelerle verir.
- **Yeni yer:** katman ve blok yazma yardımcıları (`put_nodes`, `put_blocks`) masaüstünün `drawing_exchange.rs`'inden
  uygulama katmanına (`exchange_write.rs`) taşındı. Başka çizimden al ve Yapıştır aynılarını kullanır.

### 6. Performans

Alışverişin kuralları JSON üzerinde çalışır ve bizim çizimin nesnelerini hiç okumaz. İlk uygulama yine de bütün çizimi
JSON'a çeviriyordu: 100 000 nesneli çizimde tek nesneyi kopyalamak 215 ms, aynı çizime yapıştırmak 900 ms sürüyordu; 1 000
nesneyi yapıştırmak 7,4 s. Üç değişiklik bunu düzeltti:

- **Yalnız başlık:** kurallara bizim çizimin yalnız başı gider (`to_snapshot_v2_of`).
- **Temsilciler:** kurallar nesnelerden yalnız katmanlarını, sembollerini, stillerini, bloklarını ve resimlerini okur. Bu
  yüzden her farklı birleşimden bir temsilci kuralın içinden geçer. Kuralın temsilcilere yaptığı (katman, blok ve stil
  eşlemesi, düşen bağlar) öbür nesnelere tipli olarak uygulanır (`Taught`). Resimler kendi temsilcileridir.
- **Tipli kutu:** kopyanın sol alt köşesi, nesneleri JSON'dan geçirmeden çekirdeğin şekliyle bulunur.

Süreler sürüm derlemesinde, 100 000 çizgilik çizimde, 10 katmanlı bir grupta, bu makinede:

| Nesne | Kopyalama | Boş çizime yapıştırma | Aynı çizime yapıştırma |
|---:|---:|---:|---:|
| 1 | 11,1 ms | 0,4 ms | 4,0 ms |
| 1 000 | 11,7 ms | 6,0 ms | 10,6 ms |
| 10 000 | 19,9 ms | 42,6 ms | 47,7 ms |
| 100 000 | 119,5 ms | 468 ms | 501 ms |

```text
cargo test --release -p kentos-native-application --test all clipboard_cmd::timing -- --ignored --nocapture
```

## Doğrulama

- **Bağımsız başvuru:** `scripts/fixtures/clipboard_command_cases.py` KentOS kodu olmadan yazıldı; alışverişin bağımsız
  başvurusunu (`exchange_cases.py`) kullanır.
  - Kaynak çizim: grup içinde katmanlar, kilitli ve gizli katman, iki blok, iki yazı ve bir ölçü stili.
  - Hedef çizim: milimetre birimli; aynı yolun büyük harfli grubu, başka bir grup, aynı adlı küçük harfli blok ve stil.
  - Ortak durumlar `fixtures/commands/v1/cad.entities.copy.json` (20) ve `cad.entities.paste.json` (22).
  - Kapsananlar: kopyanın parçaları ve bağları, taban noktası, katman yolları ve açılanlar, ad çakışmaları, blok olarak
    yapıştırma ve alınmış ad, birim uyarısı, kilitli ve gizli katman, retler, plan, geri alma.
- **Yetenekler:** `capabilities.rs`'in testi her ret ve uyarı kodunu bir durumda bulur.
- **Araç:** `crates/native/interaction/tests/all/clipboard.rs`:
  - kopya ve kesme, Yapıştır aracının hayaleti ve yeri, yazılan nokta;
  - yolu olmayan katmanın açılması ve ikinci yapıştırmada aynı katman;
  - kilitli katmanın reddi, gizli katmanın uyarısı, tutarsız kopyanın reddi;
  - blok olarak yapıştırma ve alınmış ad, Taban noktasıyla kopyala.
- **Masaüstü:** pano kısayollarının testleri ve izler değişmeden geçer.
- **Resimler:** `apps/desktop/src/clipboard_scenes.rs`, `.run/shots/arac-pano-*`: taban noktası, başka çizime
  yapıştırmanın hayaleti, açılan grup, katmanlar ve stil, blok olarak yapıştırma; 1440×900 ve 1100×650, açık ve koyu.
