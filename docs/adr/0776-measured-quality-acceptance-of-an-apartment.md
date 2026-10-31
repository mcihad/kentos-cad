# ADR 0776: Bir konut çiziminin baştan sona ölçülen kalite kabulü

- **Durum:** kabul edildi (2026-10-31); `plan/architecture-advanced.md` ADV-07'nin ilk ölçülen dilimi **[~]**.
- **İş:** plan/architecture-advanced.md §3 `ADV-07` (“Konut çizimini baştan sona ölçülen bir kalite teslimine
  dönüştür”). Yeni özellik değil, ölçülen kabul; bulunan küçük kusurlar testleriyle düzeltildi, büyükleri listelendi.
- **Web:** web dururken (ADR 0228) yalnız masaüstü; web'e gerekenler TODOS.md §0.2'de.
- **`.kcad`:** şema değişmez. `FORMATS_VERSION` değişmez. Yeni komut yok; değişen komut cevabı yok (§2'nin
  düzeltmeleri cevabı doğru yapar, biçimini değiştirmez).
- **Derleme:** `main` 26cd67cc1 (ADV-01/02/03 birleşmiş; ADV-04 ae598d632 henüz değil). Apple M5, 32 GB, macOS.
- **Bağımlılık:** yeni kitaplık yok.

## Bağlam

ADV-01…03 birer birer gösterildiğinde iyi görünüyordu; bu ADR aynı 2+1 daireyi baştan sona çizer ve ölçer: dış kontur,
bölmeler, tipli kapı ve pencereler, mahaller, donatı, ölçü, bağlı döşeme, tasarım değişikliği, çizelge, pafta. Üç yol
aynı işi yapar: (1) masaüstünde gerçek klavye ve fare olaylarıyla (iz oynatıcısı, ADR 0018/0021), (2) Python, MCP ve
masaüstünün ajan bağlantısıyla aynı anlamsal komutlar, (3) büyük koordinat, ölçek ve performans ölçümleri. Plan
uyarısı aynen geçerlidir: tek otomatik sahne görüntüsü GUI kabulü değildir.

Başvurulan kaynaklar (31 Ekim 2026'da okundu):

- **Autodesk AutoCAD LT 2026, Lineweight Settings Dialog Box:** model alanında kalınlık pikselle gösterilir ve
  yakınlaştırmayla değişmez; pafta alanında gerçek birimlerle gösterilir, yakınlaştırmayla değişir; LWDISPLAY yalnız
  ekranı etkiler, çıktıyı değil.
  <https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-LT/files/GUID-1BB43E62-DF93-494E-ACF2-55824ACD5130.htm>
- **Autodesk Revit, Thin Lines / Maintaining Line Thickness when Zooming:** model çizgileri görünüş ölçeğine bağlı
  kalınlıktadır; Thin Lines yakınlaşınca çizgileri ince tutar, yazdırmayı etkilemez.
  <https://help.autodesk.com/cloudhelp/2014/ENU/Revit/files/GUID-DF7D7ED0-20F5-4CFD-A415-2023E732B09B.htm>

KentOS kararı: ölçek ve kalınlık kuralı (ADR 0626, 0696) bu ADR'de değişmez; derin yakınlıkta bulunan sonuç (§3) bulgu
olarak yazıldı, “Görünüm › Çizgi kalınlığı” kapalıyken milimetrenin doğru çizildiği gösterildi.

## Karar

### 1. Masaüstünde gerçek girdiyle 2+1 akışı ve ölçümü

`fixtures/interaction/v1/acceptance-flat.json` (167 adım; `scripts/fixtures/acceptance_flat.py` yazar, çizimi
`acceptance-flat.kcad`: boş metre/1:100 CAD projesi, zemin kat 3.06 m) deneyimli bir kullanıcının yolunu oynatır:
araçlar şeritten (`run`), değerler komut satırından, duvarlar gövdelerine tıklanarak. İz üç çeşitte (US, Türkçe Q, 2×
ekran) `every_trace_passes` ile geçer; her bölümün sonunda nesne sayısı ve duvarların yeri denetlenir, tasarım
değişikliğinden sonra Bölme 3'ün ekseni 8.325 m'dedir. Oynatıcıya bir adım eklendi: `clickNumber` (seçili duvarın ya da
kolonun geçici ölçüsüne, ekranda durduğu yerde tıklar; ADR 0710).

Bölüm bölüm ölçüm (`scripts/acceptance/trace_metrics.py --markdown`; tık, imleç, tuş, komut satırına yazılan seçenek
harfi ve değer, aynı seçenek için tekrar yazılan değer, çizim ile komut satırı arasında odak değişimi, kurtarma):

| Bölüm | Araç yolu | Tık | İmleç | Tuş | Seçenek | Değer | Tekrar | Odak | Kurtarma | Resim | Adım |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Dış kontur: Dikdörtgen duvar, İç net, 25 cm | şerit | 0 | 0 | 6 | 1 | 3 | 0 | 1 | 0 | 1 | 14 |
| Bölmeler: dört Ölçülü bölme | şerit | 14 | 4 | 12 | 5 | 5 | 0 | 7 | 0 | 1 | 47 |
| Kapılar ve pencereler | şerit | 10 | 10 | 6 | 1 | 1 | 0 | 2 | 0 | 1 | 32 |
| Mahaller: Mahal, adı yazılıp odaya tıklanarak | şerit | 5 | 0 | 12 | 5 | 5 | 0 | 10 | 0 | 1 | 34 |
| Donatı: Mutfak dizisi kuzey duvarı boyunca | şerit | 3 | 1 | 2 | 0 | 0 | 0 | 0 | 0 | 1 | 8 |
| Ölçü: Duvar ölçüleri güney duvarında | şerit | 2 | 1 | 2 | 0 | 0 | 0 | 0 | 0 | 0 | 6 |
| Döşeme: kaynağa bağlı | şerit | 1 | 0 | 3 | 1 | 0 | 0 | 2 | 0 | 0 | 7 |
| Etiketler: Hepsini etiketle | şerit | 0 | 0 | 2 | 0 | 0 | 0 | 1 | 0 | 0 | 4 |
| Çizelgeler | şerit | 2 | 0 | 1 | 0 | 0 | 0 | 1 | 0 | 1 | 6 |
| Tasarım değişikliği: Yatak 1 net 3.50 → 3.60 geçici ölçüden | — | 2 | 1 | 2 | 0 | 1 | 0 | 0 | 0 | 2 | 8 |
| Kaydet ve aç | Ctrl+S, Ctrl+O | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 1 |
| **Toplam** |  | 39 | 17 | 48 | 13 | 15 | 0 | 24 | 0 | 8 | 167 |

Okunuşu:

- **Araç bulma süresi** bir kişi olmadan ölçülemez; yolu (şerit) ve adım sayısı ölçüldü. Acemi kullanıcı akışı
  kaydedilmedi (§5).
- **Mahal adı başına iki odak değişimi:** ad komut satırına yazılır, oda çizimde tıklanır (Mahaller: 10).
- **Tekrar yazılan değer 0:** Ölçülü bölme Kalınlık, Uzaklık ve Uçlar'ı bölmeler arasında tutar (ilk ölçümde iz
  gereksiz yere yeniden yazıyordu; araç bunu istemiyor).
- **Kurtarma 0:** izin ilk sürümünde (2 cm/piksel) Bölme 3'ün ortasına tıklamak duvarı değil, yüzü bölmenin
  yüzünde duran odayı seçti, duvara Sıradakini seç (Shift+Boşluk) ile varıldı; 3 cm/pikselde aynı tık duvarı
  seçiyordu: sonuç yakınlığa bağlıydı (B4, düzeltildi). İz 3 cm/pikselde oynar: 1100×650'de plan çizim alanına sığar.
- **Elle onarılan ilişki 0:** bölmeler, açıklıklar, mahaller, ölçü dizisi, döşeme, etiketler ve çizelgeler tasarım
  değişikliğini kendiliğinden izler (anlamsal akışta sayılarla, §2).
- Dikdörtgen duvarın köşeleri yazılarak verildi: izin ilk sürümünde 2 cm'lik pikselde fareyle 0,0'a tıklamak duvarı 1 cm kaydırıyordu
  (ızgara ve kenet kapalı); bu ekranın doğasıdır, bulgu değil.
- Mahal adları izde küçük harfle girer: oynatıcı harfleri Shift'siz yazar (ADR 0018'in kuralı); gerçek klavyede
  “Salon” yazılır. Oynatıcının sınırı, ürünün değil.

### 2. Aynı anlamsal akış: Python, MCP ve masaüstünün ajan bağlantısı

`fixtures/acceptance/v1/flat.json` (27 adım; beklenen her değer `acceptance_flat.py`'de KentOS kodu olmadan planın
sayılarından kesirle hesaplanır: beş odanın net alanı, değişiklikten sonra Yatak 1 16.20 / Yatak 2 11.70 m², ölçü
dizisinin yedi parçası ve 11.00 m toplamı, bağlı döşeme 11.5 × 8.5 = 97.75 m², duvarların yüzeyi 169.728 m²).
Üç koşucu aynı dosyayı oynatır: `python/tests/test_acceptance_flat.py`, `crates/native/mcp/tests/acceptance_flat.rs`,
`apps/desktop/src/acceptance_flat_tests.rs` (ajan bağlantısı; geri alma Ctrl+Z ile). Her yazan adımdan sonra koşucu
geri alır ve yineler: adım tektir, adı beklenendir (`Duvar`, `Açıklık tipi`, `Kapı`, `Mahal`, `Mutfak dizisi`,
`Otomatik duvar ölçüleri`, `Döşeme`, `Etiket`, `Çizelge`, `Duvarı düzenle`), nesne sayısı ve bağlı nesneler geri gelir.
İki ret aynı koddur (`opening_off_wall`, `layer_locked`) ve hiçbir şey yazmaz. Kaydet-aç'tan sonra her bağlı nesne
aynı kimlikle aynıdır. `KENTOS_ACCEPTANCE_OUT` ile üçü son çizimi yazar; `acceptance_flat.py --compare` onları
bağımsız KCAD okuyucusuyla (`tools/kcad/kcad.py`) okur, oda alanlarını kendi halkalarından hesaplar: üç çizim türleri,
duvarları, açıklıkları ve odalarıyla aynıdır.

Bu akışta bulunup düzeltilenler:

- **B1 (orta) — Python ve MCP'nin ölçüsü mahal ve döşemede alan vermiyordu.** `Document.measure` /
  `drawing.measure` / ajanın `measure`'ı odada ve döşemede `area: null` döndürüyordu; ajan alanı ancak
  `cad.spaces.quantities` ile bulabiliyordu. Başsız sunucunun ölçüsü artık halkaları tek alan nesnesi olarak ölçer
  (`kentos_headless::query::item`): delikler düşülmüş alan ve halkaların uzunluğu, Metraj'ın okuduğu. Test: üç koşucu
  mahal ve döşeme alanlarını ölçüyle denetler.
- **B2 (düşük) — Metraj'ın boş toplamları `-0.0` diyordu.** Rust'ın f64 toplamı -0'dan başlar; yüksekliği olmayan
  duvarlarda `wallNetArea` ve her duvarın `openingsArea`'sı JSON'da `-0.0` çıkıyordu (`spaces_quantities.rs`'in
  toplamları +0). Test: `quantities_zero.rs`.

### 3. Büyük koordinat, ölçek ve yakınlaştırma regresyonları

`apps/desktop/src/quality_scenes.rs`:

- **Test** `a_millimetre_far_from_the_origin_is_drawn_picked_and_kept_through_a_pan`: E 487 000, N 4 420 000'de 0,1
  mm/piksel yakınlıkta bir milimetre on pikseldir (1e-4 piksel içinde), ekran dünyaya 1 µm içinde döner, 1 mm aralıklı
  iki çizgiden yakın olanı seçilir, bir kilometrelik kaydırma ve geri dönüş görünümü bit bit aynı bırakır.
- **Test** `the_unit_the_drawing_scale_and_the_screens_zoom_stay_apart`: dairede ekran yakınlığı (1:20 … 1:500) ve
  kaydırma hiçbir nesneyi ve ayarı değiştirmez; çizim ölçeği 1:100 → 1:50 duvar, açıklık, mahal, döşeme ve donatıya
  dokunmaz; birim m → cm “boylar korunur” hiçbir koordinatı değiştirmez (ADR 0626 aynen).
- **Resimler:** `kalite-ayrinti-1mm` (0,1 mm/piksel, kalınlık açık), `kalite-ayrinti-1mm-ince` (Çizgi kalınlığı
  kapalı: iki çizgi on piksel aralıkla keskin), `kalite-ayrinti-1-5`, `kalite-ayrinti-1-1`, `kalite-1-100`,
  `kalite-1-50`, `kalite-1-20` (yazı ve ölçülerin görünürlüğü), `kalite-bilesen-1-20` (T birleşiminde bileşenlerin
  sürekliliği; ADR 0340'ın resmiyle aynı birleşim). Görünürlük ve süreklilik resimle gözle denetlendi, sayıyla değil.

Bu bölümde bulunup düzeltilenler:

- **B3 (orta) — Kalın duvarın gövdesine tıklamak duvarı bulmuyordu.** Duvarın parçaları yüz çizgileridir; 25 cm
  duvarın ortası her iki yüze 12,5 cm'dir ve 1:100'e yakın görünümde seçme açıklığının dışındadır. Ölçülü bölme
  “Bir duvarın düz ya da yaylı kenarına tıklayın” dedi, seçme bir şey bulmadı. Deponun seçmesi duvarı gövdesinin
  içinde de alan olarak bulur (`store/pick.rs`'in `score`'u: dış halka içinde, deliğin dışında; kenar isabetlerinden
  sonra, en küçük alan önce). Test: `geometry-core` `wall_pick.rs`; iz duvar gövdelerine tıklayarak geçer.
- **B4 (orta) — Duvarın gövdesine tıklamak odayı seçebiliyordu.** Odanın sınırı duvarın yüzündedir; duvarın
  gövdesindeki tıkta iki yüz de açıklıktaysa oda kenarıyla, duvar ondan sonra geliyordu ve hangisinin önce geldiği
  yakınlığa bağlıydı. Tık bir duvarın gövdesindeyken odaların kenar isabetleri bütün alanlardan sonraya gider
  (`Store::hits`); gövdenin dışında sıra eskisi gibidir. Test: `wall_pick.rs`. Duvar ölçüleri aracı kendi gövde
  seçmesini (üst üste gövdelerde en yakın merkez) korur: önce yüzün açıklıktaki isabeti (`hit_edge`).
- **B5 (orta) — Döşeme'de “B” (Kaynağa bağlı) komut satırında seçenek değildi.** ADR 0771 harfi aracın girdisine ve
  ayarlarına koymuş, istemine koymamıştı. Komut satırı önerilerini istemin seçeneklerinden yapar ve Enter ilkini
  alır: “B” bir komutun başı sayıldı ve aracı bitirdi. İstem artık “Kaynağa bağlı (B)” der. Test:
  `kentos-interaction` `prompt_settings.rs`; iz döşemeyi komut satırından bağlı yazar.

### 4. Performans: 1, 20 ve 100 daire

`apps/desktop/src/perf/flat.rs` (release, Apple M5, 1440×900, 13 px yazı, koyu tema, wgpu başsız; “GPU” resmin geri
okunmasını da içerir, pencere sunumu yok). Daire anlamsal akışla çizilir, `cad.entities.transform`'un kopyasıyla
onarlı satırlara çoğaltılır:

| Daire | Nesne | Kaydırma (bütün plan) UI p50/p95 · GPU p95 | Kaydırma (tek daire) | Kapı önizlemesi | Seçme tıklaması | Tasarım değişikliği: komut p50 · kare UI p95 | Metraj p50 | Bellek çizildikten sonra / beş değişiklikten sonra |
|---|---|---|---|---|---|---|---|---|
| 1 | 44 | 0.21 / 0.25 · 5.40 | 0.20 / 0.22 · 3.88 | 0.35 / 0.58 · 7.08 | 0.28 / 0.58 · 7.17 | 0.92 · 1.75 | 0.76 | 369 / 717 MB |
| 20 | 880 | 0.36 / 0.42 · 7.19 | 0.39 / 0.44 · 4.08 | 0.37 / 0.42 · 5.24 | 0.49 / 1.16 · 5.68 | 73 · 87 | 17 | 728 / 754 MB |
| 100 | 4400 | 1.07 / 1.16 · 6.63 | 1.13 / 1.29 · 10 | 1.15 / 1.24 · 9.79 | 1.04 / 1.09 · 8.68 | 1862 · 2175 | 116 | 1733 / 1940 MB |

Okunuşu ve hedefler (60 Hz = kare başına 16,7 ms; girdiye geri bildirim 100 ms; hedeftir, ölçülmüş sonuç değildir):

- Kaydırma, Kapı önizlemesi ve tıklayarak seçme 100 dairede de UI iş parçacığında p95 ≈ 1,3 ms, GPU (geri okuma
  dahil) p95 ≈ 10 ms: 60 Hz hedefinin içinde.
- **B6 (yüksek) — Tasarım değişikliği büyüdükçe karesel yavaşlıyor.** Tek bölmenin Mesafe'si 1 dairede 1,2 ms, 20
  dairede 73 ms, 100 dairede 1,86 s; kare 2,2 s donar (100 ms hedefinin 20 katı). Parçalar ölçüldü (aynı duvara bir öznitelik yazmak 20 dairede 31 ms, 100 dairede 794 ms; değişikliğin yalnız planı (okuma kopyasında etkileri) 1 874 ms; Kaydır 1 844 ms. macOS `sample` ile release koşusunda ana iş parçacığının meşgul örneklerinin %75'i malloc, free ve memmove: her adım büyük yapıları kopyalıyor (release sembolsüz, işlevin adı bulunmadı)).
  Düzeltilmedi: izleme geçişlerinin (mahal, ölçü dizisi, etiket, çizelge, bağ) bütün çizimi taradığı yerlerin
  bulunması ayrı iştir; bu ADR yalnız ölçer. İlerleme/iptal yok: komut UI iş parçacığında çalışır.
- **B7 (yüksek) — Değişiklik sırasında bellek ikiye katlanıyor.** Ölçü sürecin bütün belleğidir (aynı süreçte 1, 20,
  100 daire sırayla): 100 daire çizilince 1,73 GB, değişikliğin planı sırasında 3,17 GB, Mesafe ve Kaydır'dan sonra
  2,87 GB; ilk koşuda beş değişiklik ve geri almadan sonra 5,49 GB görüldü, ikincide 1,94 GB: bellek geri
  verilmesi koşudan koşuya değişiyor. Nedeni bulunmadı (okuma kopyası, geri alma kaydı ya da izleme geçişlerinin
  kopyaları adaylar).
- Metraj 100 dairede 117 ms (20'de 17 ms): doğrusaldan biraz hızlı büyüyor.
- Kopyalama 100 daire için 1,8 s (bir adım, 2 200 nesne).

### 5. Yapılmayanlar ve bulgu listesi

| # | Önem | Bulgu | Durum |
|---|---|---|---|
| B1 | orta | Python/MCP ölçüsü mahal ve döşemede alan vermiyordu | düzeltildi, testli |
| B2 | düşük | Metraj'ın boş toplamları `-0.0` | düzeltildi, testli |
| B3 | orta | Kalın duvarın gövdesine tıklamak duvarı bulmuyordu | düzeltildi, testli |
| B4 | orta | Bölmenin ortasına tıklamak yakınlığa göre duvarı ya da yüzü bölmenin yüzünde duran odayı seçiyordu | düzeltildi, testli |
| B5 | orta | Döşeme'nin “B”si istemde yoktu; komut satırı onu komut sayıp aracı bitiriyordu | düzeltildi, testli |
| B6 | yüksek | Tasarım değişikliği 100 dairede 1,86 s, bir öznitelik yazmak 0,79 s; karesel büyüme, iptal yok | açık |
| B7 | yüksek | 100 dairede değişiklik sırasında bellek 1,7 → 3,2 GB; bir koşuda 5,5 GB | açık |
| B8 | orta | Model alanında kalem kalınlığı çizim ölçeğinde dünya birimiyle çiziliyor (1:100'de 0,25 mm kalem 25 mm); derin yakınlıkta 1 mm ayrıntılar banda karışıyor. AutoCAD model alanında pikselle çizer | açık (ADR 0626/0696 kuralı; geçici çözüm Görünüm › Çizgi kalınlığı) |
| B9 | orta | 17 mimari aracın panelinde 56 satır (başlık ve bilgi satırları dahil) komut satırının isteminde yok; o harfler yazılınca komut sayılabilir (B5'in genel hâli) | açık (ölçü: `prompt_settings.rs`'in yok sayılan testi) |
| B10 | düşük | Ölçülü bölmenin ve dikdörtgen duvarın geri alma adı “Duvar”; hangi aracın yazdığı söylenmiyor | açık |
| B11 | düşük | Bağlı duvarda büyük Mesafe'nin reddi `constraint_conflict` ile bağı suçluyor; asıl neden kenarın ters dönmesi | açık |
| B12 | düşük | Geçici ölçünün sayısı odanın etiketinin üstüne biniyor (resim `acceptance-flat-07-gecici-olcu`) | açık |

Yapılmayanlar (dürüstçe):

- **Gerçek pencerede fare ve klavyeyle kabul yok.** İz oynatıcısı gerçek Iced olaylarını uygulamaya verir ama
  pencere, işletim sistemi odağı ve insan yoktur; araç bulma süresi ve acemi kullanıcı akışı ölçülmedi.
- **Pafta GUI'de oynatılmadı:** Çıktı › Kat planları şablonun sorularını bir panelde açar, oynatıcı o paneli
  cevaplamaz; pafta anlamsal akışta (`cad.sheets.levels`) ve ADR 0718'in masaüstü testinde yapılır.
- **Kapı ve pencere tipleri GUI'de penceresinden tanımlanmadı:** iz KentOS'un kendi türlerini kullanır, giriş kapısının
  enini yazar; proje tipleri anlamsal akıştadır.
- **Bu bağı (Bu ölçüyü koru) GUI'de kilit karesiyle verilmedi;** anlamsal akışta `keep` ile verildi.
- Kabul korpusunun öbürleri (iki katlı konut, eğrisel/şaftlı zor örnek, gerçek DWG/DXF altlığında tadilat) yapılmadı.
- Eklenti (plugins.md) yolu bu dilimde yok.
- ADV-04 (bina programı, ae598d632) `main`'de değil: bölüm/hedef alanları bu ölçüme girmedi.

## Doğrulama

- `python3 scripts/fixtures/acceptance_flat.py --check` (senaryo, iz, izin çizimi); `--compare .run/acceptance-flat`
  üç ev sahibinin çizimlerini bağımsız okur.
- `.run/py/bin/python -m unittest python/tests/test_acceptance_flat.py`; `cargo test -p kentos-mcp --test
  acceptance_flat`; `cargo test -p kentos-desktop --lib acceptance_flat quality_scenes`;
  `KENTOS_TRACES_ONLY=acceptance-flat cargo test -p kentos-desktop --lib every_trace_passes`.
- `cargo test -p kentos-geometry-core --test all wall_pick`; `cargo test -p kentos-interaction --test all
  prompt_settings`; `cargo test -p kentos-native-application --test all quantities_zero`.
- `cargo test --release -p kentos-desktop --lib perf::flat -- --ignored --nocapture --test-threads=1`
  (`KENTOS_PERF_PARTS=1` parçalar).
- Bu dalda koşulanlar (31 Ekim 2026): `kentos-interaction` 819 (duvar ölçüleri aracının gövde seçmesi depo kuralına
  uyduruldu: yüzün açıklıktaki isabeti önce, üst üste gövdeler en yakın merkeze), `kentos-geometry-core`,
  `kentos-headless`, `kentos-native-application`, `kentos-mcp`; masaüstü kütüphanesinin 1 490 testi (bütün izler üç
  çeşitte dahil); Python 376. Dalın dokunmadığı, `main`'de (26cd67cc1) düşenler: uygulamada `cad.walls.edit`'in
  yetenek testi (`constraints_relaxed` …), masaüstünde `history::text` TITLES (ADV-03'ün iki komutu yok),
  `temporary_scenes` (ADV-01'in kilit kareleri), `every_program_opens_a_link_of_its_own` ve MCP'nin `desktop.attach`
  testi (uzun çalışma ağacı yolu, Unix soketi SUN_LEN); Python SDK'sı katalogla güncel değil (`sdk.py --check`:
  `types.py`, `walls.py`), yeniden yazılınca kalan 6 düşüş ADV-01/02'nin türleri (`wall` bileşeni, `GapChange`,
  `building-parts.json`).
