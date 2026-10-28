# ADR 0730: Ölçü dizisinin kalanı — çoklu pafta, sığmayan yazının yeri, yuvarlama kuralı, kopyalama

- **Durum:** kabul edildi (28 Ekim 2026), uygulandı (29 Ekim 2026); ARCH-DIM-01'in son açık maddeleri.
- **Bağlam:** `plan/architecture.md` ARCH-DIM-01; ADR 0695 (kalıcı duvar ölçü dizisi, “Sonraki dilimler”), ADR 0700, ADR 0729
  (cephe ve mahal tarifi), ADR 0323 (ölçekli açıklamalar), ADR 0321 (ölçünün yazısı), ADR 0149 (tek gösterim kuralı),
  ADR 0337 (kopyada kısıtlar).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2'ye satırı yazıldı.

## Bağlam

ADR 0695'in açık bıraktığı dört iş: **çoklu pafta** (aynı dizinin 1:50 ve 1:100 paftada aynı kâğıt boyuyla), **çakışma ve
sığmayan yazıyı açıklayan yerleştirme** (ADR 0729 §6 sığmayanı söyler ve tehlike renginde çizer ama yerinde bırakır), planın
kabulündeki **toplam ile parçaların yuvarlama kuralı** ve **tam grafik kopyalama** (bugün otomatik üye kopyalanınca
`dimension_set_owned` ile reddedilir).

Araştırma (üreticilerin güncel belgeleri; yerelde çalıştırılmadı):

- [AutoCAD LT 2018 — About Using Annotative Objects to Represent Multiple Annotation Scales](https://help.autodesk.com/cloudhelp/2018/ENU/AutoCAD-LT/files/GUID-590DE319-FD23-45FE-8430-D3EF4AB989D2.htm):
  tek açıklama nesnesi birden çok ölçekte temsil edilir; ölçeği atanmamış görünüm çerçevesinde nesne çizilmez.
- [Revit 2024 — Change Text Size](https://help.autodesk.com/cloudhelp/2024/ENU/Revit-GetStarted/files/GUID-E1664999-4EEB-4526-9096-1B92646218FD.htm):
  açıklamanın basılı boyu tipinde sabittir, görünümün ölçeğiyle model boyu değişir.
- [Archicad 28 — Scale](https://help.graphisoft.com/AC/28/INT/_AC28_Help/020_Configuration/020_Configuration-39.htm): ölçü ve
  okları “kâğıt boyu” elemanlardır, proje ölçeğinden bağımsız belirtilen boyda basılır.
- [Graphisoft 29 — Linear Dimension Tool Settings](https://help.graphisoft.com/MEP/29/INT/_MEP_Designer_Help/150_UserInterfaceToolSettings/150_UserInterfaceToolSettings-19.htm)
  (Archicad'in ölçü aracıyla ortak): yazı yerleşim yöntemi **Classic** (yer yoksa yana, o da yoksa yukarı sıra sıra),
  **Flexible** (yana; olmazsa dikey; “Allow below dimension line”, “Allow adjacent texts to shift”), **Centered**, **Off**.
- [Archicad 19 — Associative Dimensions](https://help.graphisoft.com/AC/19/INT/AC19Help/04_Documentation/04_Documentation-40.htm):
  ilişkili ölçü bağlı olduğu eleman değişince güncellenir. Ölçünün elemanıyla birlikte kopyalanınca kopyaya bağlanması için
  üreticilerin resmî bir kural sayfası bulunamadı (Revit'in “bazı ölçüler başvurusunu bulamadığı için kopyalanmadı” uyarısı
  yalnız kullanıcı forumlarında); KentOS'un kuralı aşağıdadır.

KentOS kararı: ölçekler ADR 0323'ün ölçekli açıklama modelidir (her ölçeğin kendi ölçü çizgisi ve yazı yeri), tarif onları
kendi hesabıyla yazar; yazı yerleşimi Archicad'in Classic/Flexible davranışının basit ve belirli bir biçimidir; yuvarlama
farkı dağıtılmaz, söylenir; kopya, bütün kaynaklarıyla seçilmiş dizinin tarifini kopyalara taşır.

## Karar

### 1. Çoklu pafta: tarifin `scales`'ı

`DimensionSet.scales`: üyelerin de çizildiği pafta ölçekleri (1:n'nin n'si). Her biri sonlu, 1 ile 1 000 000 arasında,
artan sırada, ikisi aynı ölçek değil (`same_scale`), en çok 32. Boş: tarif üyelerin ölçekli temsilini yönetmez (eski
davranış; kullanıcı üyeyi ADR 0323 ile kendisi ölçekli yapabilir).

Doludur ise üyelerin `annotative`'ini tarif yazar: `at` çizimin çizim ölçeği `P`'dir (planın ölçeği), `scales` tarifin
ölçeklerinden `P` ile aynı olmayanlardır; her birinde `offset` o ölçekteki planın ölçü çizgisi, `textShift` o ölçekteki
yazının yeri (§2; `Keep`'te yok: ADR 0323'ün kuralıyla kendi yerinin ölçeklisi). Paftanın haritası kendi ölçeğindeki temsili
çizer (`Document::at_scale`); tarifin ölçeklerinde olmayan ölçekteki paftada üyeler çizilmez (ADR 0323). Tarif ölçeklerini
yönetirken üyenin `annotative`'ini başka bir düzenleme değiştiremez (`a`, `b`, `offset` gibi tarifin alanıdır).

**Bir ölçekteki plan.** `k = s / P`. Tarifin sıra yerleri (`at` ve `rowPlaces`'in noktaları) başvuru çizgisinden enine
uzaklıkları `k` katına çıkarılarak taşınır: `q' = q + n̂·(k − 1)·δ(q)`, `δ` noktanın başvuru çizgisinden `n̂` yönündeki işaretli
uzaklığı. Başvuru çizgisi tek duvar tarifinde ölçülen yüz kenarı (`n̂` çerçevenin normali), `span`'lı tarifte gövdelerin
sıralar yanındaki en dış yüzü (`o = body.1`, yan `+1`; `o = body.0`, yan `−1`; `n̂ = n`). Sıra aralığı `gapMm × s / 1000`.
Aynı planlayıcı bu yerlerle koşar; aralıklar ölçekten bağımsızdır, her üye kendi aralığının o ölçekteki `offset`'ini alır.
Böylece sıraların kâğıttaki düzeni her paftada aynıdır.

### 2. Sığmayan yazının yeri: tarifin `tight`'ı

`DimensionSet.tight`: `keep` (yok sayılan; ADR 0729 §6: yerinde, tehlike renginde) ya da `shift`. Komutun öntanımlısı
`keep`'tir (eski betikler aynı sonucu alır); Duvar ölçüleri aracı yeni tarifi `shift` ile yazar.

Her ölçekte ayrı hesaplanır (`h_s = h·s/P`). Ölçü `i`'nin ölçülen boyu `m`, yazısı `Format::dimension_value` ile okuyucunun
gördüğü metin, genişliği çizim yazı tipinin ilerlemeleriyle `w = width_em(metin)·h_s`; **sığmaz**: `w + h_s > m`.

Planın çerçevesinde (`d` doğrultu; tek duvarda ölçülen kenarın yönü; `t = p·d`, `o = p·n`) her yazı bir dikdörtgendir: boyunca
`[c − w/2, c + w/2]` (`c` iki noktanın `t`'lerinin ortası), enine ölçü çizgisinden okuma yönünün yukarısına `[λ, λ + h_s]`
(`λ` = ölçünün yazı boşluğu `text_gap·h_s`; ortalı yazıda `−0,35·h_s`). Okuma yönü ölçü çizgisinin yönüdür, 90°'den büyükse
ters çevrilir (çekirdeğin kuralı). Pay `m₀ = h_s/2`: iki yazı boyunca bundan yakınsa ve enine örtüşüyorsa çakışır; bir yazı
başka bir sıranın ölçü çizgisini enine içine alıyor ve boyunca örtüşüyorsa çakışır (kendi sırasının çizgisi hariç).

Önce sığan yazılar ve kullanıcının elle taşıdığı yazılar (yerleriyle) yerleşir. Sonra sığmayanlar, sıraların sırasıyla ve
sıra içinde `c`'ye göre, şu adaylardan ilk çakışmayanı alır:

1. **sağ:** `c' = t₁ + m₀ + w/2` (aralığın büyük ucunun ötesi), aynı bant;
2. **sol:** `c' = t₀ − m₀ − w/2`;
3. **üst:** yerinde, okuma yönünün yukarısına `h_s·(1 + g)` (`g` yazı boşluğu çarpanı);
4. **alt:** yerinde, aşağıya `min(0, λ) − λ − h_s − g·h_s`.

Hiçbiri olmazsa yerinde kalır (tehlike renginde). Yerleşen her yazı sonrakilerin engelidir. Kayma okuma yönünde
`(boyunca, enine)` olarak `textShift`'e yazılır; sıfırsa yok.

**Elle taşınan yazı.** Üye kendi ölçeğinde son yazdığı yeri `shift` olarak tutar. Ölçünün `textShift`'i bununla aynıysa (ikisi
yok sayılır) yazı tarifindir ve yeniden yerleşir; değilse kullanıcı taşımıştır, yeri korunur ve engel sayılır.
`cad.dimensionsets.edit`'in `resetText` işlemi (`row` ile bir sıranın, onsuz bütün üyelerin) yazısını tarifin son yerine
döndürür; `tight` `keep`'e dönünce tarifin yerleri kalkar, kullanıcının taşıdıkları kalır. Ölçeklerin yazı yerleri her zaman
tarifindir.

### 3. Toplam ile parçaların yuvarlama kuralı

Her değer kendi ölçüsünden, ölçünün görünüşüyle ve tek gösterim kuralıyla (ADR 0149) yuvarlanır; toplam da öyle. Fark
parçalara dağıtılmaz. Toplam sırası olan ve sırası baştan sona kesintisiz (her ölçünün ikinci yeri sonrakinin ilki; ilk ve
son yer toplamınkiler) bir sıranın yazılı değerleri aynı sayıda ondalıklı düz sayılarsa (önek, sonek, tolerans, ayak-inç,
elle yazı yok) toplanır; yazılı toplamdan farklıysa plan `rounding`'de sırayı, parçaların toplamını ve toplamı söyler ve
`dimension_set_rounding` uyarısını verir. Hesap ondalıkların tam sayısıyladır (kesin).

### 4. Kopyalama

Bir tarif, bütün kaynakları (`sources()`) ve bütün etkin üyeleri kopyalanan nesneler arasındaysa **bütünüyle seçilmiştir**.

- `cad.entities.transform` (kopya), `cad.entities.array` ve pano (`cad.entities.copy` → `cad.entities.paste`) bütünüyle
  seçilmiş her tarifi kopyalarla yeniden kurar: yeni kimlik; kaynaklar, üyeler ve bastırılmış aralıkların kopyalanan
  başvuruları kopyalara; `at` ve `rowPlaces` dönüşümle; `span`'ın doğrultusu dönüşümle ve okunur yöne (ADR 0729); katmanı
  kopyaların katmanı (Katmana kopyala). Aynalayan dönüşümde çekirdek duvarın yolunu ters çevirir (ADR 0339 §1): gövde yolun
  aynı yanında kalır, tek duvar tarifinin yüzü (`face`) aynı kalır ve aynı fiziksel yüzü ölçer; çok kenarlı duvarda kenar
  numarası yeni yolda aynı kenara çevrilir (açık yolda `k → n − 2 − k`, kapalıda `(n − 2 − k) mod n`). Aynalanan kopyada
  açıklıklar pervazlarından yeniden okunur (`at`, ters yönde `L − at`), aynı yerde kalır. Kopyalanmayan başvurular
  (seçilmemiş aks, kapı) olduğu gibi kalır. Tarif aynı adımda planlanır: üyeler kopyalar olarak kalır, kimlikleri aynı aralığın
  kopyasıyla korunur, geometrileri ve bağları plandan yazılır.
- Kısmen seçilmiş tarif (üyelerinden biri seçili ama bir kaynağı ya da üyesi değil) eskisi gibi reddedilir
  (`dimension_set_owned`), ileti neyin eksik olduğunu sayar.
- Yalnız kaynaklar seçiliyse kopyalar tarifsizdir (eskisi gibi).
- Blok tanımı ve dosyaya blok yazma tarifi taşıyamaz: kısmen ya da bütünüyle seçilmiş tarif reddedilir (değişmedi).
- Kopyasız `cad.entities.transform` (Taşı, Döndür, Ölçekle, Aynala) bütünüyle seçilmiş tarifin `at`'ını, `rowPlaces`'ini ve
  doğrultusunu aynı dönüşümle taşır; sıralar kaynaklarla birlikte gider.
- Sunucuya tarif ve üyeleri olağan `project.changes` v2 ile gider (ADR 0729 §8).

### 5. Sunucu

`scales`'lı tarifte sunucu, `project.changes` v2'de tarifi planlarken her üyenin `annotative`'ini aynı kuralla denetler
(`kentos_formats::dimension_spans::scales_agree`: `at` çizim ölçeği, ölçekler tarifinkiler sırasıyla, her birinde kendi
noktası ve dönüşü yok, `offset` = `scaled_offset(offset, δ, s, P)`; `δ` sıranın başvuru çizgisinden uzaklığı: cephede
`row_delta`, tek duvarda ölçülen yüz kenarından). Uymayan üye `features[k].entity.annotative` yoluyla atomik rettir. Yazı
yerleri (`textShift`, üyenin `shift`'i) ADR 0695'teki gibi istemcinindir.

Sunucunun tek duvar denetimi, kabul testinin ikinci tarifiyle (açıklıksız kuzey duvarı) bulunan bir eksiği de kapattı: ADR
0695'in 27 Ekim ekindeki kural (sıraların sırasıyla, önce çizilenle aynı iki yeri ölçen aralık çizilmez: açıklıksız kenarın
Açıklıklar sırası ve Toplam) sunucuda yoktu; masaüstünün yazdığı tek üyeli tarif “üyelik eksik” diye reddediliyordu. Sunucu
artık aynı kuralı aynı yerlerle (`source`, `place`) uygular.

### 6. Dosya

`scales`, `tight` ya da üyenin `shift`'i olan tarif KCAD **şema 129** ister; yoksa yazıcı eski şemayı seçer, eski dosyaların
baytları değişmez.

### 7. Masaüstü

Duvar ölçüleri aracı: **Pafta ölçekleri** (boş ya da “50 100” gibi; çizim ölçeği kendiliğinden), **Sığmayan yazı**
(Kaydır/Yerinde; yeni tarif Kaydır). Önizleme yazıları yerleştirilmiş çizer; yerleştirilemeyenler tehlike renginde.
Öznitelikler'de dizinin Pafta ölçekleri, Sığmayan yazı ve “Yazıları tarife döndür”. Pafta tasarımcısında aynı dizi 1:50 ve
1:100 haritasında kendi temsiliyle çizilir.

## Sınırlar

- Yazı yerleşimi uzatma çizgilerini ve okları engel saymaz; sıraların ölçü çizgilerini ve yazılarını sayar. Kılavuzlu yazı
  (Archicad'in Flexible'ındaki komşu yazıyı kaydırma) yoktur.
- Elle taşınan yazı yalnız tarifin kendi ölçeğinde korunur; ölçeklerin yazı yerleri tarifin hesabıdır.
- Blok ve WBLOCK tarifi taşımaz.

## Doğrulama

- **Ortak durumlar** (`scripts/fixtures/dimension_layout_cases.py --check`, KentOS kodu olmadan yazılmış başvuru):
  `cad.dimensionsets.create.layout` 22, `cad.dimensionsets.edit.layout` 23, `cad.entities.transform.recipes` 20,
  `cad.entities.array.recipes` 21 durum (ölçekler ve temsillerin `offset`'i, Kaydır'ın dört adayı ve elle taşınan yazı,
  yuvarlama, kopya/dizi/pano/taşı/döndür/aynala; aynalamada ters yol ve açıklığın `L − at`'i). Bütün ortak komut
  durumları (`every_command_case`) ve yetenek testi geçer; Python'un tipli katmanı da hepsini geçer (`test_command_cases`).
- **Çekirdek:** `ops::dimension_layout`'un 4 birim testi (ölçekli uzaklık, aday sırası ve elle taşınan engel, boş yer
  yokken yerinde, kesin yuvarlama toplamı).
- **Uygulama:** `dimension_layout.rs`'in 8 testi (pafta ölçeği ve temsil, ölçekte olmayan paftada çizilmeme, temsillerin
  başka düzenlemeyle değişmemesi, Kaydır ve elle taşınan yazı, Yazıları tarife döndür, Yerinde'ye dönüş, yuvarlama uyarısı,
  kopya, dizi, pano, taşıma, yarım tur, aynalı kopyada yüz, L duvarda yerinde aynalamada kenar `1 → 0`, KCAD örneği).
- **Dosya:** KCAD şema 129 örneği (`dimension-set-scales.json`, komutların çizimi: 1:100 ve 1:200'lü cephe, elle taşınmış
  bir yazı, 1:100'lü tek duvar tarifi) bağımsız Python yazıcısıyla (`kcad_v2_reference.py --check`, 1325 dosya) ve
  Rust'ta (`kentos-kcad` 234 test; şema 128 reddeder, üçü de tek başına şema 129 ister). `tools/kcad/kcad.py validate`.
- **Sunucu** (geçici PostGIS 18, 55432; `KENTOS_TEST_DB=required`): `project_dimension_sets`'in 9 testi; yeni test iki
  tarifin (cephe ve tek duvar) temsillerini kabul eder, birinin 1:200 sırası 0,5 m kayınca yoluyla reddeder.
- **Araç ve masaüstü:** Duvar ölçüleri'nin 19 testi (Kaydır/Yerinde, Pafta ölçekleri, yuvarlama satırı), Öznitelikler'in
  testi (Pafta ölçekleri, Sığmayan yazı, Yazıları tarife döndür, Yok ile kaldırma).
- **PDF** (`dimension_set_sheet_tests.rs`, kabul odasının 1:50 ve 1:100 A3 paftaları, `pdftotext -bbox`): Pafta ölçekleri
  yokken 1:100 sayfada yazı kutusu 1,5 mm, sıra aralığı 3,75 mm (1:50'de 3,0 mm ve 7,50 mm); `scales: [100]` ile 1:100'de
  de 3,0 mm ve 7,50 mm, Açıklıklar sırasının yazısı iki sayfada da duvar yüzünden 14,6 mm aşağıda.
- **Python:** `test_wall_dimension_sets.py`'nin yeni testi (temsillerin `offset`'i iki kat, kaldırma, geri alma, bağımsız
  okuyucuyla dosya). **MCP:** `kentos-mcp`'nin `dimension_sets` ve `mcp` testleri (33).
- **Gerçek arayüz** (`dimension_layout_scenes.rs`, iki tema, 1440×900 ve 1100×650; `.run/shots/arac-olcu-dizisi-*`):
  Kaydır, Yerinde, Pafta ölçekleri, Öznitelikler, yuvarlama, Kopyala (uzak ve yakın), Dizi, Aynala.
- **Katalog:** ADR 0731'in paylaşılan biçimiyle yeniden yazıldı (4,25 MB); Python SDK'sı ve TypeScript türleri
  üretildi; manifest değişmedi (yeni komut yok).
