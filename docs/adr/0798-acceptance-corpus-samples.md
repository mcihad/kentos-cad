# ADR 0798: Kabul korpusunun öbür dört örneği

- **Durum:** kabul edildi (2026-11-01); `plan/architecture-advanced.md` ADV-07'nin beşinci ölçülen dilimi (ADV-07e) **[~]**.
- **İş:** ADV-07'nin “Kabul korpusu”: boş projeden 2+1 dairenin (ADR 0776) yanında ölçülü değişiklik, iki katlı konut,
  eğrisel/şaftlı zor örnek ve DXF altlığı üzerinde tadilat. Her biri üç yoldan; bulunan kusurlar testleriyle düzeltildi,
  düzeltilmeyenler listelendi.
- **Web:** web dururken (ADR 0228) yalnız masaüstü, Python ve MCP.
- **`.kcad`:** şema değişmez. `FORMATS_VERSION` değişmez. Yeni komut yok. Komut kataloğunda iki açıklama değişti
  (`cad.slabs.edit`'in Kes'i, `cad.walls.convert`'in `keepSources`'u); şema biçimi aynı, TS ve Python yeniden üretildi.
- **Derleme:** `main` b3ba51862 üzerinde; Apple M5, macOS 26 (Darwin 25.6), rustc 1.96.0.
- **Bağımlılık:** yeni kitaplık yok. Tadilatın DXF altlığını GDAL 3.13.3'ün `ogr2ogr`'u yazar (yalnız fixture'ı yazarken).

## Bağlam

ADR 0776 2+1 daireyi baştan sona üç yoldan ölçtü; planın kabul korpusunun öbür dört örneği açık kaldı. Bu ADR onları
aynı düzenle yapar: (1) masaüstünde gerçek klavye ve fare olaylarıyla iz ve bölüm bölüm ölçü (`trace_metrics.py`),
(2) aynı anlamsal işi Python, MCP ve masaüstünün ajan bağlantısıyla, beklenenleri KentOS kodu olmadan hesaplanmış
senaryodan; üç ev sahibinin kaydettiği çizimler bağımsız KCAD okuyucusuyla karşılaştırılır, (3) performans sahneleri.
Etiket motorunun artımlı yerleşimi (ADR 0786'nın açık noktası) bu dilime sığmadı (§5).

Başvurulan kaynaklar (31 Ekim – 1 Kasım 2026'da okundu):

- **Autodesk Revit API, `Element.DemolishedPhaseId` / `CreatedPhaseId`:** her öğe bir evrede yapılır, bir evrede yıkılır;
  oda evreye aittir. <https://rvtdocs.com/2025/75c9d2c7-a402-ea8b-9e7c-f8bc3510bbd5>
- **Autodesk Community, Revit Architecture forumu (oda ve evre başlıkları):** odanın sınırı, odanın evresinde var olan
  (o evrede henüz yıkılmamış) duvarlardır; yıkılan duvar sonraki evrenin odasını sınırlamaz.
  <https://forums.autodesk.com/t5/revit-architecture-forum/show-existing-rooms-in-quot-show-previous-and-demo-quot-phase/m-p/9944039>,
  <https://forums.autodesk.com/t5/revit-architecture-forum/spaces-are-being-subtracted-from-room-placement/m-p/11082363>
- **GDAL DXF sürücüsü:** başlık şablonuyla AutoCAD 2004 (AC1018) yazar, çizgileri LWPOLYLINE olarak; şablonun
  `$INSUNITS`'i 1'dir (inç), veri metre olsa da. Gerçek dünyanın birimi yanlış söyleyen DXF'idir.

KentOS kararı: KentOS'un dört evresi (var olan, yıkılacak, yeni, geçici) Revit'in iki evreli (mevcut, yeni yapım) en
yaygın düzenine eşlenir: **önce** (var olan ve yıkılacak oda) ve **sonra** (yeni oda) (§3, K1).

## Karar

### 1. Dört örnek: senaryo, iz ve çizim

`scripts/fixtures/acceptance_corpus.py` (KentOS kodu olmadan; kesirler, düzlemin formülleri) dört senaryoyu
(`fixtures/acceptance/v1/<id>.json`), dört izi (`fixtures/interaction/v1/acceptance-<id>.json`) ve izlerin çizimlerini
yazar; `--check` güncelliği, `--files` altlığı GDAL'la yeniden yazar, `--compare D` ev sahiplerinin çizimlerini okur.

| Örnek | Senaryo | Ne sınar |
|---|---|---|
| `change` Ölçülü değişiklik | 22 adım (12 yazan, geri alınıp yinelenen) | var olan 2+1'de Yatak 1 net 3.50 → 3.80 ve Bu ölçüyü koru; Salon 4.50 → 4.80, bağ Bölme 3'ü götürür, Bölme 2'nin ucu izler, kapılar yerinde kalır; dış duvar 25 → 30 cm iç yüzü yerinde (odalar aynı, bağlı döşeme 11.6 × 8.6); kapı Bölme 1'in yüzünden 0.10, pencere doğu yüzden 0.60 (ölçü dizisi izler); üç ret (`nothing_to_set`, `invalid_wall`, `invalid_opening`) |
| `house` İki katlı konut | 22 adım | iki kat; zemin katın yüksekliğine bağlı merdiven (⌈3.00/0.175⌉ = 18 rıht, 17 × 0.28); 1. katta merdiven boşluğu (boşluk mahal); iki bağlı döşeme; bağlı döşemeden Kes reddi (`slab_bound`), Ayır, sonra merdiven boşluğunun iç noktasından Kes (83.25 m²); iki kat paftası; kullanılan katın kaldırılması reddi (`level_in_use`); kat 3.00 → 3.24, merdiven 18 rıhtla 0.18 |
| `curved` Eğrisel ve şaftlı | 20 adım | doğusu yarım daire cumba (büküm 1, R 4), ortalı 25 cm duvar; yaydaki pencere (10 + 2π), yay boyunca taşınması; şaftlı L duvar ve boşluk mahal; cumbada döner merdiven (yürüme yarıçapı 1.50, dönüş 4.76/1.5 rad); merdivenin eni ve merkez direği; yarım daireli bağlı döşeme; iki ret (`invalid_wall` yayda Kaydır, `invalid_opening` yaydaki açıklığa mesafe), Metraj'ın döşemesi şaftın düşüldüğü net alan |
| `renovation` Tadilat | 20 adım | GDAL'ın DXF'i metre olarak (başlığı inç), kilitli altlık, Çiftler ve Kaynağı koru ile var olan duvarlar (her duvar çiftinin ortasında, örtüştükleri boyca, 0.15000000000000036 en DXF'in iki sayısının farkı), var olan oda ve açıklıklar, orta duvarın yıkımı, yeni bölme ve kapı, yeni odalar; evreye göre Metraj; Evre görünümü; yeni bölmenin Mesafe'si (yeni odalar değişir, var olanlar değişmez); iki ret (`invalid_file_path`, kilitli altlığın çizgilerini silerek dönüştürme `layer_locked`) |

Senaryo biçimine iki ek (dört koşucu da destekler): `files` (senaryonun dosyaları, koşucu mutlak yolu `$ad` ile
bağlar) ve sorgu adımında `as` (çıktının `uids`'i adlara; tadilatın seçtiği çizgiler). Cevap beklentileri de bağlanır.

### 2. Üç yol: Python, MCP, masaüstünün ajan bağlantısı

Koşucular tek bir `replay`'e indi ve korpusu da oynatır: `python/tests/test_acceptance_corpus.py`
(`test_acceptance_flat.replay`), `crates/native/mcp/tests/acceptance_flat.rs`
(`every_corpus_sample_is_drawn_changed_and_reopened_as_its_scenario_says`), `apps/desktop/src/acceptance_flat_tests.rs`
(aynı ad; `CORPUS`, `opened_with`, `drawn_with`). Her yazan adım geri alınıp yinelenir (adı beklenen: `Duvar`,
`Duvarı düzenle`, `Açıklığı düzenle`, `Kapı`, `Mahal`, `Merdiven`, `Döşeme`, `Döşemeyi düzenle`, `Etiket`, `Çizelge`,
`Otomatik duvar ölçüleri`, `Mimari nesneyi düzenle`, `Çizgiden duvar`, `DXF: renovation-altlik.dxf`); retler
yazmaz; kaydet-aç'tan sonra her bağlı nesne aynı kimlikle aynıdır. `KENTOS_ACCEPTANCE_OUT=D` ile üç ev sahibi
`D/<id>/{python,mcp,desktop}.kcad` yazar; `acceptance_corpus.py --compare D` bunları `tools/kcad/kcad.py` ile okur,
odaların alanını kendi halkalarından ve bükümlerinden (yay parçası r²(θ − sin θ)/2) hesaplar: dört örnekte üç çizim
türleri, duvarları, açıklıkları ve odalarıyla aynı ve plana eşit.

### 3. Bulunan ve düzeltilen kusurlar

- **K1 (yüksek) — Evre odanın sınırını değiştirmiyordu.** Yeni odalar yıkılacak duvarla, var olan odalar yeni
  bölmeyle sınırlanıyordu: tadilatta yeni Salon 40.95 yerine 33.95, var olan Salon 28.00 yerine 21.00 m². Mahalin
  kaynakları artık evresinde duranlardır (`space_follow::phase_bounds`): var olan ve yıkılacak oda var olan ve yıkılacak
  duvarlarla (kolon, ayırıcı) sınırlanır, yeni oda var olan ve yenilerle; evresiz oda, evresiz duvar ve geçici oda eskisi
  gibi her kaynağı okur. Kaynak görünümünün önbellek anahtarı evre sınıfını taşır (`phase_key`). Bir ortak durumun
  beklentisi bu kuralla değişti (`cad.spaces.edit.json` › “Değiştir: kat, evre ve yükseklik”: yeni bölmenin içindeki oda
  var olan evreye alınınca 8 × 5 m'nin bütünü); bağımsız başvuru (`space_command_cases.py`) aynı kuralı taşır.
  Testler: `space_phase_tests.rs`, tadilat senaryosu üç ev sahibinde.
- **K2 (orta) — Döşemeden iç noktayla Kes bütün döşemeyi götürüyordu.** `subtract`'ın `seed`'i `boundary` verilmezse
  Dış yüzler'i (yapının dış konturu) buluyordu; bağlı döşeme o kontur olduğundan kesik hep `slab_cut_missed` idi. Kes'in
  iç noktası artık öntanımlı olarak odadır (`SlabBoundary::Room`, iç yüzler); Ekle eskisi gibi. Test: konut senaryosu.
- **K3 (orta) — Kilitli altlıktan duvar yapılamıyordu.** Çizgiden duvar Kaynağı koru'yla da kilitli katmanın
  çizgilerini atlıyordu (bütünü kilitliyse `layer_locked`). Kaynaklar korunup duvarlar başka katmana (`layerId`)
  yazılınca çizgiler yalnız okunur: kilit sorulmaz (`walls_from_lines::check`). Kaynakları silen dönüştürme kilitte
  yine reddedilir. Test: tadilat senaryosu (ret ve dönüştürme).
- **K4 (orta) — Çizgiden duvar aracı duvarları altlığın katmanına yazıyordu.** Aracın girdisinde `layerId` yoktu;
  duvarlar kaynağın (altlığın, çoğunlukla kilitli) katmanına düşüyordu. Kaynağı koru açıkken duvarlar etkin katmana
  yazılır (`interaction::wall_from_lines`); kaynak silinirken eskisi gibi. Test: tadilat izi.
- **K5 (orta) — Kapının boşluğuna tıklamak duvarı seçiyordu.** Duvarın gövdesi açıklıkların içinden geçen alan olarak
  seçilir (ADR 0776 B3); açıklığın parçaları pervaz ve kanat çizgileridir, ortasında hiçbiri yoktur. Açıklık artık
  pervazları arasındaki boşlukta alan olarak bulunur, daha küçük olduğu için duvardan önce (`store/pick.rs`).
  Test: `wall_pick.rs` › `a_click_in_an_openings_gap_picks_the_opening_before_its_wall`; değişiklik izi kapıya
  tıklayarak geçer.

Oynatıcıya bir adım eklendi: `clickLock` (seçili duvarın geçici ölçüsünün kilit karesine tıklar; ADR 0770 §3) ve
`Session::temporary_locks`. `trace_metrics.py` onu tık sayar.

### 4. Gerçek girdiyle izler ve bölüm bölüm ölçü

İzler üç çeşitte (US, Türkçe Q, 2×) `every_trace_passes` ile geçer; resimler `kentos-cad kullan acceptance-<id>` ile
1440×900 ve 1100×650'de, koyu ve açık temada (`.run/shots/kullanim/masaustu-acceptance-<id>-*`, 60 resim).

**Ölçülü değişiklik** (önce 2+1'in planı ADR 0776'nın iziyle, sonra değişiklikler):

| Bölüm | Araç yolu | Tık | İmleç | Tuş | Seçenek | Değer | Tekrar | Odak | Kurtarma | Resim | Adım |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Var olan plan — Dış kontur: Dikdörtgen duvar, İç net, 25 cm | şerit | 0 | 0 | 6 | 1 | 3 | 0 | 1 | 0 | 1 | 14 |
| Var olan plan — Bölmeler: dört Ölçülü bölme | şerit | 14 | 4 | 12 | 5 | 5 | 0 | 7 | 0 | 1 | 47 |
| Var olan plan — Kapılar ve pencereler | şerit | 10 | 10 | 6 | 1 | 1 | 0 | 2 | 0 | 1 | 32 |
| Var olan plan — Mahaller: Mahal, adı yazılıp odaya tıklanarak | şerit | 5 | 0 | 12 | 5 | 5 | 0 | 10 | 0 | 1 | 34 |
| Değişiklik 1: Yatak 1 3.50 → 3.80, kilitle (Bu ölçüyü koru) | — | 3 | 1 | 2 | 0 | 1 | 0 | 0 | 0 | 1 | 8 |
| Değişiklik 2: Salon 4.50 → 4.80; bağ Bölme 3'ü götürür | — | 2 | 1 | 2 | 0 | 1 | 0 | 0 | 0 | 1 | 7 |
| Değişiklik 3: Yatak 1 kapısı Bölme 1'in yüzünden 0.10 | — | 2 | 1 | 2 | 0 | 1 | 0 | 0 | 0 | 0 | 6 |
| Değişiklik 4: Yatak 2 penceresi doğu yüzden 0.60 | — | 2 | 1 | 2 | 0 | 1 | 0 | 0 | 0 | 1 | 7 |
| Kaydet ve aç | Ctrl+S, Ctrl+O | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 1 |
| **Toplam** |  | 38 | 18 | 44 | 12 | 18 | 0 | 20 | 0 | 7 | 156 |

**İki katlı konut:**

| Bölüm | Araç yolu | Tık | İmleç | Tuş | Seçenek | Değer | Tekrar | Odak | Kurtarma | Resim | Adım |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Zemin kat: dış kontur | şerit | 0 | 0 | 6 | 1 | 3 | 0 | 1 | 0 | 0 | 13 |
| Zemin kat: bölme | şerit | 2 | 1 | 6 | 2 | 2 | 0 | 1 | 0 | 0 | 16 |
| Zemin kat: merdiven kat yüksekliğine | şerit | 2 | 1 | 7 | 2 | 2 | 0 | 2 | 0 | 1 | 18 |
| Zemin kat: mahaller | şerit | 2 | 0 | 6 | 2 | 2 | 0 | 4 | 0 | 0 | 15 |
| 1. kat: Üst kat (Ctrl+↑), Yalnız etkin kat, dış kontur | şerit | 0 | 0 | 7 | 1 | 3 | 1 | 1 | 0 | 0 | 15 |
| 1. kat: bölmeler | şerit | 6 | 2 | 10 | 4 | 4 | 2 | 3 | 0 | 1 | 32 |
| 1. kat: mahaller, merdiven boşluğu (V) | şerit | 3 | 0 | 10 | 5 | 3 | 0 | 6 | 0 | 1 | 28 |
| Kaydet ve aç | Ctrl+S, Ctrl+O | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 1 |
| **Toplam** |  | 15 | 4 | 52 | 17 | 19 | 3 | 18 | 0 | 3 | 138 |

**Eğrisel ve şaftlı:**

| Bölüm | Araç yolu | Tık | İmleç | Tuş | Seçenek | Değer | Tekrar | Odak | Kurtarma | Resim | Adım |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Kontur: Çoklu çizgi, doğuda teğet yay | şerit | 0 | 0 | 8 | 3 | 4 | 0 | 1 | 0 | 1 | 18 |
| Çizgiden duvar: ortalı 25 cm | şerit | 1 | 0 | 5 | 1 | 1 | 0 | 2 | 0 | 0 | 10 |
| Bölme: batı yüzden 4.00 | şerit | 2 | 1 | 6 | 2 | 2 | 0 | 1 | 0 | 0 | 16 |
| Şaft: L duvar | şerit | 0 | 0 | 8 | 1 | 4 | 0 | 1 | 0 | 0 | 16 |
| Mahaller: Yatak, Salon, şaft boşluk (V) | şerit | 3 | 0 | 9 | 4 | 3 | 0 | 5 | 0 | 1 | 25 |
| Döner merdiven: cumbada, merkez, başlangıç, yön | şerit | 3 | 3 | 6 | 2 | 2 | 0 | 2 | 0 | 1 | 20 |
| Kaydet ve aç | Ctrl+S, Ctrl+O | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 1 |
| **Toplam** |  | 9 | 4 | 42 | 13 | 16 | 0 | 12 | 0 | 3 | 106 |

**Tadilat:**

| Bölüm | Araç yolu | Tık | İmleç | Tuş | Seçenek | Değer | Tekrar | Odak | Kurtarma | Resim | Adım |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Çizgiden duvar: Çiftler, Kaynağı koru, kilitli altlık | şerit | 1 | 0 | 7 | 3 | 1 | 0 | 1 | 0 | 1 | 16 |
| Mahaller: Salon ve Oda | şerit | 2 | 0 | 6 | 2 | 2 | 0 | 3 | 0 | 0 | 15 |
| Yeni bölme: batı yüzden 3.00 | şerit | 2 | 1 | 6 | 2 | 2 | 0 | 2 | 0 | 1 | 17 |
| Kaydet ve aç | Ctrl+S, Ctrl+O | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 1 |
| **Toplam** |  | 5 | 1 | 19 | 7 | 5 | 0 | 6 | 0 | 2 | 49 |

Okunuşu:

- **Bir ölçülü değişiklik 6–8 adım, 2–3 tık, bir değer:** duvarı seç, imleci yanına götür, sayıya tıkla, yaz, Enter;
  bağ için bir tık daha (kilit). Elle onarılan ilişki 0: bağ Bölme 3'ü, ölçü dizisi pencereleri, odalar duvarları izler.
- **Tekrar yazılan değer konutta 3:** Dikdörtgen duvar ve Ölçülü bölme duvar genişliğini paylaşır (K7); üst katta 25 ve
  15 cm yeniden yazılır. İzin ilk sürümünde yazılmadı ve üst katın dış duvarı 15 cm oldu: kullanıcı hatası değil,
  aracın belleğinin sonucu.
- **Kurtarma 0, odak değişimi her mahalde iki:** ADR 0776'daki gibi (ad komut satırında, oda çizimde).
- **Tadilatın evreleri GUI'de verilemedi** (K6): iz duvarları, odaları ve yeni bölmeyi çizer ama evre yazmaz.

### 5. Performans

`apps/desktop/src/perf/corpus.rs` (release, 1440×900, 13 px, koyu, wgpu başsız; “GPU” geri okumayı da içerir). Her
örnek anlamsal akışla çizilir, 1 ve 20 kopya (onarlı satırlar). Kaydırma ve seçme UI p50 / p95 · GPU p95 (ms); tasarım
değişikliği örneğin kendi Mesafe'si (komut p50 · kare UI p95, ms); bellek sürecin bütünü (aynı süreçte sırayla).

| Örnek | Kopya | Nesne | Kaydırma | Seçme tıklaması | Tasarım değişikliği | Metraj p50 | Bellek çizilince / tepe / sonra (MB) |
|---|---|---|---|---|---|---|---|
| change | 1 | 35 | 0.21 / 0.27 · 4.67 | 0.28 / 0.32 · 4.73 | 1.07 · 2.82 | 0.75 | 378 / 725 / 725 |
| change | 20 | 700 | 0.37 / 0.44 · 4.93 | 0.28 / 0.33 · 4.87 | 7.50 · 12.00 | 18.45 | 726 / 747 / 741 |
| house | 1 | 24 | 0.20 / 0.24 · 4.75 | 0.25 / 0.33 · 4.69 | 0.44 · 1.58 | 0.43 | 738 / 749 / 749 |
| house | 20 | 480 | 0.19 / 0.24 · 4.57 | 0.27 / 0.31 · 4.77 | 2.69 · 4.78 | 20.86 | 739 / 749 / 749 |
| curved | 1 | 15 | 0.19 / 0.24 · 4.61 | 0.26 / 0.29 · 3.62 | 0.63 · 1.81 | 0.40 | 739 / 751 / 750 |
| curved | 20 | 300 | 0.20 / 0.28 · 4.72 | 0.30 / 0.39 · 4.80 | 6.24 · 9.05 | 13.66 | 740 / 752 / 752 |
| renovation | 1 | 25 | 0.21 / 0.26 · 6.07 | 0.28 / 0.39 · 7.29 | 0.44 · 1.20 | 0.37 | 742 / 757 / 752 |
| renovation | 20 | 500 | 0.20 / 0.24 · 4.67 | 0.27 / 0.30 · 6.06 | 0.80 · 1.98 | 18.73 | 743 / 757 / 753 |

Bütün etkileşimler 60 Hz bütçesinin (16,7 ms) ve 100 ms geri bildirim hedefinin içinde. 20 kopyalı değişiklikte yeni
tasarım değişikliği 1 dairenin 7–10 katı (bağlı ve izleyen nesneler); ADR 0782'den sonra karesel değil.

### 6. Bulgu listesi ve yapılmayanlar

| # | Önem | Bulgu | Durum |
|---|---|---|---|
| K1 | yüksek | Evre odanın sınırını değiştirmiyordu | düzeltildi, testli |
| K2 | orta | Döşemeden iç noktayla Kes bütün döşemeyi götürüyordu | düzeltildi, testli |
| K3 | orta | Kaynağı koru'yla da kilitli altlıktan duvar yapılamıyordu | düzeltildi, testli |
| K4 | orta | Çizgiden duvar aracı duvarları altlığın katmanına yazıyordu | düzeltildi, testli |
| K5 | orta | Kapının boşluğuna tıklamak duvarı seçiyordu | düzeltildi, testli |
| K6 | orta | Duvar, Ölçülü bölme, Mahal ve Çizgiden duvar araçlarında Evre yok; tadilatın evresi ancak sonradan Öznitelikler'den | açık |
| K7 | düşük | Dikdörtgen duvar ve Ölçülü bölme duvar genişliğini paylaşır; katlar arasında 25/15 cm yeniden yazılır | açık |
| K8 | düşük | `cad.settings.set`'in `arch`'ı mimari ayarların bütünüdür: yalnız `phaseView` gönderen katları siler ve `level_in_use` alır | açık (API'nin kuralı; belgelenmeli ya da yama yapılmalı) |
| K9 | düşük | Cumbadaki Salon'un etiketi döner merdivenin altında kalıyor (resim `acceptance-curved-03-merdiven`) | açık |
| K10 | bilgi | Duvarın ucu izlerken üstündeki açıklık dünyada yerinde kalır (`at` değişir); bu ADR beklentileri buna göre | davranış |

Yapılmayanlar (dürüstçe):

- **Etiket motorunun artımlı yerleşimi (ADR 0786'nın açık noktası) yapılmadı.** Bağlam bu paketin ilk maddesiyle doldu.
  Önerilen yol: `Store`'un yerleşimi önceki yerleşimi ve her etiketin aday kutularını tutar; değişikliğin dokunduğu
  nesnelerin ve kutularıyla kesişen yerleşmiş etiketlerin adayları yeniden yerleştirilir (QGIS PAL'ın etiket önbelleği
  gibi), `label_memo` tarzı rastgele düzenleme özelliği testiyle tam yerleşime eşitlik, 100 dairede değişiklik karesi
  önce/sonra.
- **Gerçek bir ofisin DWG'si yok;** altlık GDAL'ın yazdığı DXF'tir (üçüncü tarafın, KentOS'un değil). DWG okuyucusu
  bu dilimde sınanmadı.
- **Gerçek pencerede elle kabul, acemi kullanıcı ve araç bulma süresi ölçülmedi** (ADR 0776 gibi).
- **GUI izleri anlamsal akışların hepsini yapmaz:** Öznitelikler hücresine oynatıcı yazmaz (dış duvarın kalınlığı,
  merdivenin eni, evre), açıklıklar konuttaki ve cumbadaki izde çizilmedi, paftalar GUI'de oynatılmadı.
- Eklenti (plugins.md) yolu yok.

## Doğrulama

- `python3 scripts/fixtures/acceptance_corpus.py --check`; `--compare .run/acceptance` (üç ev sahibinin çizimleri,
  `KENTOS_ACCEPTANCE_OUT=.run/acceptance` ile yazılır); `--files` altlığı GDAL'la yazar.
- `.run/py/bin/python -m unittest python/tests/test_acceptance_corpus.py python/tests/test_acceptance_flat.py`;
  `cargo test -p kentos-mcp --test acceptance_flat`; `cargo test -p kentos-desktop --lib acceptance_flat`;
  `KENTOS_TRACES_ONLY=acceptance-change,acceptance-house,acceptance-curved,acceptance-renovation cargo test -p kentos-desktop --lib every_trace_passes`.
- `python3 scripts/acceptance/trace_metrics.py fixtures/interaction/v1/acceptance-<id>.json --markdown`;
  `./target/debug/kentos-cad kullan acceptance-<id>`.
- `cargo test --release -p kentos-desktop --lib perf::corpus -- --ignored --nocapture --test-threads=1`.
- Bu dalda koşulanlar (1 Kasım 2026): `kentos-geometry-core` (548 + testler), `kentos-domain` (306 + testler),
  `kentos-interaction` (823 + 71), uygulamanın ortak durumları (`cad.walls`, `cad.slabs`, `cad.spaces`, `cad.openings`,
  `file.import`, `cad.tags`, `cad.schedules` süzgeciyle), `kentos-commands` katalog, MCP kabul testleri, masaüstünün
  bütün izleri üç çeşitte, `acceptance_flat` ve `opening` süzgeçli masaüstü testleri, Python kabul testleri; clippy
  `-D warnings` dokunulan yedi crate'te. Koşulmayanlar: bütün takımlar (yalnız hedefli testler kuralı), Python'un bütün
  takımı.

## Belgelere eklenecek satırlar

- **CLAUDE.md §0** (ADR 0776'nın maddesinin altına):
  `- **Kabul korpusunun öbür dört örneği (1 Kasım 2026; [ADR 0798](docs/adr/0798-acceptance-corpus-samples.md), ADV-07 [~]).**`
  `  Ölçülü değişiklik, iki katlı konut, eğrisel/şaftlı örnek ve GDAL'ın DXF altlığında tadilat; her biri senaryo (`fixtures/acceptance/v1/<id>.json`, 20–22 adım, beklenenler `acceptance_corpus.py`'de KentOS kodu olmadan), Python, MCP ve masaüstünün ajan bağlantısıyla aynı (`--compare` üç çizimi bağımsız okur), gerçek girdiyle iz (`acceptance-<id>.json`, oynatıcıya `clickLock`) ve `perf::corpus`. Düzeltilen: mahalin sınırı evresinde duranlar (`space_follow::phase_bounds`; var olan oda yeni duvarla, yeni oda yıkılacakla sınırlanmaz), döşemeden iç noktayla Kes öntanımlı oda, Kaynağı koru'yla kilitli altlıktan Çizgiden duvar (araç duvarları etkin katmana yazar), açıklığın boşluğuna tıklamak açıklığı seçer. Açık: araçlarda Evre, duvar genişliğinin araçlar arası paylaşılması, `settings.set`'in `arch`'ı bütün, etiket motorunun artımlı yerleşimi.`
- **CLAUDE.md §2** (komut listesine):
  `python3 scripts/fixtures/acceptance_corpus.py --check; KENTOS_ACCEPTANCE_OUT=.run/acceptance .run/py/bin/python -m unittest python/tests/test_acceptance_corpus.py; cargo test -p kentos-mcp --test acceptance_flat; cargo test -p kentos-desktop --lib acceptance_flat; python3 scripts/fixtures/acceptance_corpus.py --compare .run/acceptance; KENTOS_TRACES_ONLY=acceptance-change,acceptance-house,acceptance-curved,acceptance-renovation cargo test -p kentos-desktop --lib every_trace_passes   # kabul korpusunun dört örneği üç yoldan ve gerçek girdiyle; süreler cargo test --release -p kentos-desktop --lib perf::corpus -- --ignored --nocapture --test-threads=1 (ADR 0798)`
- **plan/architecture-advanced.md ADV-07 Durum'u:** “…kabul korpusunun öbür dört örneği ([ADR 0798](../docs/adr/0798-acceptance-corpus-samples.md): ölçülü değişiklik, iki katlı konut, eğrisel/şaftlı, DXF altlığında tadilat; evreye göre mahal sınırı, döşemenin iç noktayla kesilmesi, kilitli altlıktan duvar, açıklığın boşluğuna tıklama düzeltildi). Açık: etiket motorunun artımlı yerleşimi, araçlarda Evre, gerçek pencerede elle kabul, gerçek ofis DWG'si.”
- **TODOS.md** (ADV-07 satırı/§0.2): “ADV-07e (ADR 0798) [~]: korpusun dört örneği üç yoldan geçti; açık: etiket motorunun artımlı yerleşimi (ADR 0786), Duvar/Ölçülü bölme/Mahal/Çizgiden duvar araçlarına Evre seçeneği (K6), duvar genişliğinin araçlara ayrı belleği (K7), `cad.settings.set`'in `arch` yaması (K8), cumbadaki etiketin merdivenden kaçması (K9). Web'e: `clickLock` adımı web oynatıcısında yok; evreye göre mahal sınırı web'in mahal bulucusunda yok.”
