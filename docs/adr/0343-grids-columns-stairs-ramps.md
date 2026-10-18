# ADR 0343: Aks, kolon, merdiven ve rampa

- **Durum:** kabul edildi (2026-10-18). [plan/cad.md](../../plan/cad.md) §14.4 `CAD-ARCH-31`–`36` ve `CAD-ARCH-01`'in aks ve
  merdiven kısmı (sahibin sözü: “Devam arch ile”; yedi grubun beşincisi). Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0339 (duvar), 0341 (açıklık), 0342 (mahal), 0249 ve 0320 (ilişkili ölçü), 0228 (web'e ara), CLAUDE.md §9.2.
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

Mimari planın taşıyıcı iskeleti akslardır: her aksın bir etiketi (A, B, … ya da 1, 2, …) ve uçlarında balonu vardır; kolonlar aks
kesişimlerinde durur ve yerlerini “B-3” gibi aks adlarıyla söyler; aks ölçüleri akslar arasıdır. Merdiven ve rampa parametreleriyle
çizilir: merdivenin basamak sayısı, rıht yüksekliği ve basamak derinliği, rampanın yüksekliği ve eğimi. Parametreler birbirine ya da
çizilen yola uymayabilir; bu hesaplanıp söylenir, ama KentOS bir merdivenin ya da rampanın yönetmeliğe uyduğunu söylemez.

## Karar

### 1. Kaynak modeli (`CAD-ARCH-31`, `33`, `34`, `36`)

Dört yeni tür, hepsi bir yol (`pts`, köşe başına büküm `bulges`) ve kendi parametreleriyle (`.kcad` şema 79, `FORMATS_VERSION` 86,
sütunlu kayıtta tür 24–27, deponun paketinde tür 31–34, sunucuda göç 0026). Çizim parametrelerden türer, saklanmaz.

- **`grid` (aks):** `pts` iki nokta (başı ve sonu; büküm yaylı aks), `name` (etiket, boş olmaz; nesnenin kendi `label`ı başka alandır), `bubbles` (`both`, `start`, `end`,
  `none`; balonların uçları), `bubbleSize` (balon çapı, kâğıtta mm; yoksa 8), `arch` (kat, evre).
- **`column` (kolon):** `pts` tek nokta (merkez), `shape` (`rect`, `round`), `width`, `depth` (dikdörtgende; yoksa `width`), `rotation`
  (radyan), `type` (tür adı, serbest metin), `tag` (etiket: S1 …), `fill` (dolu çizim), `grids` (isteğe bağlı iki aksın kalıcı
  kimlikleri: kolon onların kesişiminde durur), `place` (aksların adlarından türeyen konum yazısı, “B-3”; belge yazar), `broken`
  (`gridGone`: bağlı aks silindi ya da akslar kesişmiyor), `arch`.
- **`stair` (merdiven):** `pts` yürüme çizgisi (2 nokta düz, 3 nokta L, 4 nokta U; yolun yönü çıkış yönüdür), `width`, `risers` (rıht
  sayısı), `riserHeight`, `tread` (basamak derinliği), `landing` (dönüşteki sahanlığın boyu; yoksa genişlik), `cut` (kesme çizgisinin
  durduğu rıht; yoksa yok), `numbering` (basamak numaraları), `arch`.
- **`ramp` (rampa):** `pts` orta çizgisi, `width`, `rise` (toplam yükseklik), `slope` (yüzde; yoksa yükseklik ve uzunluktan),
  `base` (başlangıç kotu; yoksa 0), `landing` (dönüşteki sahanlığın boyu; yoksa genişlik), `arch`.

### 2. Geometri (`geom::arch`)

- **Aks:** yolu (çizgi ya da yay) kesikli; seçilen uçlarda yolun uzantısında, ucuna değen daire balonu ve içinde ortalanmış etiket
  (balon çapının yarısı yüksekliğinde). Çap kâğıtta mm × çizim ölçeği.
- **Kolon:** dikdörtgen (merkez, en, boy, dönüş) ya da daire; `fill` açıksa içi dolu; etiket ve konum yazısı sağ üst köşenin yanında.
- **Merdiven:** yürüme çizgisinin iki yanına genişliğin yarısı kadar kayan şerit (köşelerde gönye); her köşe dönüşte bir sahanlıktır
  (yürüme çizgisi boyunca `landing` uzunluğu, köşenin iki yanında yarısı). Basamak çizgileri yürüme çizgisi boyunca başlangıçtan
  `tread` aralıkla, sahanlıkları atlayarak, `risers − 1` basamak (son rıht üst kata çıkar); çıkış oku yürüme çizgisi boyunca, ucunda
  ok başı ve başlangıçta “Ç”. `cut` verilmişse o rıhtın yerinde şeridi çapraz kesen kırık çizgi; sonraki basamaklar çizilmez.
  `numbering` açıksa her basamakta numarası.
- **Rampa:** aynı şerit; sahanlıklar köşelerde; eğimli kollarda yürüme çizgisi boyunca ok; başta ve sonda kot yazısı (`base` ve
  `base + rise`, “+0.00” gibi, iki basamak).
- **Denetimler** (`CAD-ARCH-35`, `geom::arch::checks`): merdivende sığan basamak sayısı `risers − 1`'den azsa `stair_short` (eksik
  basamak sayısı), yol fazla uzunsa `stair_long` (artan uzunluk); `2 × rıht + basamak` 0,60–0,65 m dışındaysa `stair_step_rule` (yalnız
  bilgi: Blondel'in oranı); katın yüksekliği biliniyorsa `risers × riserHeight` ondan farklıysa `stair_rise_level`. Rampada `slope`
  verilmişse ve yükseklik bölü eğimli uzunlukla 0,1 puandan çok farklıysa `ramp_slope_mismatch`. Bunlar komutların uyarısı ve
  Öznitelikler'in Denetim satırıdır; hiçbiri onay ya da ret değildir, sözleri yönetmeliğe uyum söylemez.

### 3. Akslara bağlılık (`CAD-ARCH-32`)

- **Belge izler** (`kentos_domain::arch_follow`): bir adımın sonunda, akslara bağlı her kolonun merkezi iki aksının (doğrusunun ya da
  yayının) kesişimine yazılır (yay için eski merkeze yakın olan), `place` iki aksın etiketlerinden “B-3” olur (birinci aks harfli ise önce,
  ikisi de aynı türdense verildiği sıra); bir aks silinmişse ya da kesişmiyorlarsa `broken: gridGone`, merkez kalır. Aksın etiketi
  değişince (Numarala dahil) bağlı kolonların konum yazısı aynı adımda değişir.
- **Aks ölçüsü:** Hizalı ve Doğrusal ölçü aks uçlarına kenetlenince ilişkili ölçü olur (ADR 0249): aks taşınınca ölçü izler.

### 4. Komutlar

- **`cad.grids.create`** (“Aks”): `grids` (her biri yol, etiket, balonlar) ya da `series` (paralel akslar: ilk aksın başı ve sonu,
  `spacings` aralıklar dizisi, aksların yana doğru yönü `side` sol ya da sağ, `start` ilk etiket); etiket verilmezse sıradaki: harfliler
  A … H, J … N, P … Z, AA (I ve O atlanır), sayılar 1, 2 …. Kodlar `no_objects`, `not_finite`, `invalid_grid` (iki noktadan başka, sıfır
  boy, boş etiket), `level_not_found`, katman ve sürüm.
- **`cad.columns.create`** (“Kolon”): `columns` (her biri `at` merkezi ya da `grids` iki aksı; ölçüler, tür, etiket) ya da
  `intersections` (verilen aksların bütün kesişimlerinde, sıraları aksların sırasıyla); etiket verilmezse `tagStart`'tan artarak.
  Kodlar `not_a_grid`, `grids_parallel`, `invalid_column`.
- **`cad.stairs.create`** (“Merdiven”) ve **`cad.ramps.create`** (“Rampa”): yol ve parametreler; denetimler uyarı. Kodlar
  `invalid_stair`, `invalid_ramp` (2–4 nokta, sıfırdan büyük ölçüler, sayılar).
- **`cad.arch.edit`** (“Mimari nesneyi düzenle”), hepsi ya da hiçbiri: `set` (bu dört türün alanları; nesnenin türüne ait olmayan alan
  `field_not_for_kind`), `renumber` (verilen aksları ya da kolonları verildiği sırayla `start`'tan numaralar), `attach` (kolonu iki aksa
  bağlar), `detach` (bağı koparır, merkez kalır). Kodlar `not_arch_element`, `nothing_to_set`, `field_not_for_kind`, `mixed_kinds`.
- Python `kentos.cad.grids`, `.columns`, `.stairs`, `.ramps`, `.arch`; MCP; `cad.arch.edit` yıkıcı listede.

### 5. Masaüstü

- **Aks** (`tool.grid`): iki tık bir aks; Etiket (E) sıradakini verir; Seri (S) aralıkları yazdırır (“6 6 4.5”), ilk aksın iki noktası
  ve yan tıkla seri yazılır. **Kolon** (`tool.column`): tık merkez (iki aksın kesişimine yakın tık akslara bağlar); Biçim (B),
  En (E), Boy (Y), Döndür (D), Etiket (K); Aks kesişimleri (A) görünen bütün aksların kesişimlerine. **Merdiven** (`tool.stair`) ve
  **Rampa** (`tool.ramp`): yolun noktaları, Enter yazar; Genişlik (G), Rıht sayısı (S), Rıht (R), Basamak (D), Sahanlık (H); rampada
  Yükseklik (Y), Eğim (E). **Aksları numarala** (`tool.gridNumber`): Başlangıç (B), sırayla tıklanan akslar, Enter.
- Öznitelikler'de türlerin satırları ve merdivenle rampanın Denetim satırı; seçim süzgecinde Aks, Kolon, Merdiven, Rampa; bağı kopan
  kolonda uyarı üçgeni. Şeritte Mimari › Taşıyıcı (Aks, Kolon, Aksları numarala) ve Merdiven (Merdiven, Rampa).
- DXF'e ve Patlat'a çizimin düz parçaları; sunucuda izdüşüm: aks çizgi, kolon, merdiven ve rampa alanı.

## Sonuçlar

- Dört tür kendi parametreleriyle saklanır; çizim her zaman kaynaktan türer.
- Kolonun akslara bağı belgede izlenir; aksların kendisi bağımsızdır.
- Merdiven ve rampanın denetimleri bilgi ve uyarıdır; yönetmelik onayı değildir.

## Doğrulama

- Çekirdek: `cargo test -p kentos-geometry-core --test all arch` (`arch_cases.py`: aks balonları, kolon, merdivenin basamakları ve
  denetimleri, rampanın eğimi ve kotları; kesirlerle).
- Komutlar: `python3 scripts/fixtures/arch_command_cases.py --check`.
- `.kcad`: `python3 scripts/fixtures/kcad_v2_reference.py --check`, `cargo test -p kentos-kcad`.
- Resimler: `KENTOS_SHOTS_ONLY=aks-serit,aks-sistem,aks-arac,kolon-arac,kolon-oznitelikler,merdiven-arac,merdiven-denetim,rampa,aks-numarala,aks-izleme
  cargo test -p kentos-desktop tools_screens -- --ignored --nocapture` (`.run/shots/arac-aks-*`, `-kolon-*`, `-merdiven-*`, `-rampa-*`).
