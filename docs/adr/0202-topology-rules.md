# ADR 0202: Topoloji kuralları ve denetimi

- **Durum:** kabul edildi (2026-10-07). Kapsam sahibin kararlarıdır (7 Ekim): katman içi ve katmanlar arası kurallar; bulgular alt
  panelin Topoloji sekmesinde; bulgudan düzeltme; kurallar ve istisnalar projede. Düzenlerken canlı denetim ve kayıtta uyarı kapsam
  dışıdır. Madde tek parçada biter.
- **Bağlam belgesi:** TODOS.md `GIS-04` (ve araştırma notu), ADR 0201 (Geçerliliği denetle ve Onar: sorunların tanımı ve onarım),
  ADR 0148 (Topolojik temizlik), ADR 0162 (Çakışma denetimi ve komşulara köşe), ADR 0177 (projede adlı listeler: katman durumları),
  ADR 0178 (alt panelin Arama sekmesi, çizimdeki işaret), ADR 0143 ve 0174 (çok parçalı nesneler), ADR 0025 (`.kcad` v2).

## Bağlam

Çizimde yerinde onaran araçlar (Topolojik temizlik, Çizimi temizle, Onar) ve tek tek denetleyen araçlar (Geçerliliği denetle) vardır;
bir projenin katmanlarına “parseller çakışmamalı, binalar parselin içinde kalmalı” gibi kurallar koyup hepsini birden denetleyen, bulguları
satır satır gösteren, bulgudan nesneye giden ve bulguyu düzelten ya da bilerek bırakılmış diye işaretleyen bir yer yoktur. ArcGIS'te
Topology kuralları, Error Inspector ve Mark as Exception; QGIS'te Topology Checker ve Geometry Checker; Netcad'de topoloji kontrolleri
bu işi yapar.

## Karar

### 1. Kurallar ve tolerans

Projenin topoloji ayarı (`ProjectSettings.topology`): **Tolerans**, **kurallar** (sırasıyla) ve **istisnalar**.

- **Tolerans** metredir (0,000001–1; yazılmamışsa 0,001). Toleransın içinde kalan iki yer aynı yerdir; toleranstan dar parçalar sayısal
  gürültüdür (§2). Pencerede projenin uzunluk birimiyle yazılır.
- **Kural**: kimlik (projede bir kez), tür, katman, katmanlar arası kuralda öbür katman (kendi katmanı olamaz), değer isteyen türde değer
  (uzunluk metre, açı radyan; yazılmamışsa türün varsayılanı). Katman bir katmandır (grup değil); kural katmanın bütün nesnelerine bakar
  (gizli ve kilitli de). Katmanı ya da öbür katmanı projede olmayan kural denetlenmez ve söylenir.
- **Nesnelerin türü** ADR 0201 §1'in sınıflarıdır: alan (kapalı alan parçalarıyla ve delikleriyle, daire, tam elips, kapalı eğri), çizgi
  (çizgi, çoklu çizgi parçalarıyla, yay, elips yayı, açık eğri), nokta (çok noktalı da); elips ve eğri 0,1 mm'lik sınırıyla. Kuralın
  bakmadığı türler (yazı, ölçü, blok …) atlanır.

On üç tür. Katman içi:

| Tür | Adı | Bakar | Değer |
|---|---|---|---|
| `mustNotOverlap` | Çakışmamalı | alan | |
| `mustNotHaveGaps` | Boşluk olmamalı | alan | |
| `mustNotHaveSlivers` | İnce alan olmamalı | alan | en az genişlik, 0,1 m |
| `mustNotHaveDuplicates` | Yinelenmemeli | çizgi, nokta | |
| `mustNotHaveDangles` | Sarkan uç olmamalı | çizgi | |
| `mustNotHaveShortEdges` | Kısa kenar olmamalı | çizgi, alan | en kısa kenar, 0,05 m |
| `mustNotHaveSmallAngles` | Küçük açı olmamalı | çizgi, alan | en küçük açı, 5° |
| `mustBeValid` | Geçerli olmalı | çizgi, alan | |
| `mustNotHaveMissingVertices` | Ortak sınırda köşe eksik olmamalı | alan | |

Katmanlar arası (katman ve öbür katman):

| Tür | Adı | Katman | Öbür katman |
|---|---|---|---|
| `mustNotOverlapWith` | … ile çakışmamalı | alan | alan |
| `mustBeCoveredBy` | … içinde kalmalı | alan, çizgi, nokta | alan |
| `boundaryMustBeCoveredBy` | Sınırı … sınırlarında olmalı | alan | alan sınırı, çizgi |
| `mustBeOnEndOf` | … çizgilerinin ucunda olmalı | nokta | çizgi |

### 2. Kuralların tanımları

Ölçüler: bir **parça** örtüşmenin verdiği tek alandır (dış halka ve delikleri); A net alanı, P bütün halkalarının çevresi, **ortalama
genişlik** w = 2A/P (uzun şeritte genişliği, karede kenarın yarısı). **Sayılan parça** w > t olandır (t tolerans). **Ağırlık merkezi**
parçanın alanının merkezidir. Örtüşmeler çekirdeğindir (`geom::overlay`, 1 µm; yaylar kesin).

Çiftler nesnelerin sırasıyla (i < j; katmanlar arasında önce katmanın nesnesi) ve yalnız kutuları kesişenler (t kadar büyütülerek)
denenir.

1. **Çakışmamalı.** Her alan çifti için A ∩ B'nin sayılan parçaları: bulgu “Çakışma”, nesneler (A, B), ölçü sayılan parçaların alanı,
   yer en büyük sayılan parçanın ağırlık merkezi (eşitse ilki), şekil sayılan parçalar.
2. **Boşluk olmamalı.** Katmanın alanlarını t + 1 m'den geniş saran kutudan alanların birleşimi çıkarılır; kutunun kenarına değmeyen her
   sayılan parça (kendi içindeki adalar delik olarak) bir “Boşluk”tur. Nesneler boşluğa sınır veren alanlardır: boşluğun sınırı alanların
   kenarlarından gelir; bir sınır parçası, ortası bir alanın sınırının t içinde kalan alanındır, alanın **ortak sınırı** bu parçaların
   uzunluğudur. Nesneler ortak sınırı uzundan kısaya (eşitse katmandaki sırasıyla); ölçü boşluğun alanı, yer ağırlık merkezi. Alanların
   birleşiminin dışı (katmanın dış sınırı) boşluk değildir.
3. **İnce alan olmamalı** (değer g, varsayılan 0,1 m). Her alanın her parçası (çok parçalıda ayrı ayrı): w < g ise “İnce alan”, ölçü w,
   yer parçanın ağırlık merkezi, şekil parça.
4. **Yinelenmemeli.** Çizgilerde: bir çizginin **kenarı** (yazıldığı gibi köşeden köşeye, yay kenarı bütün; uzunluğu 2t'den fazla) bütün
   noktalarıyla öbür çizginin t içindeyse **yinelenen kenar**dır; her çizgi çifti için ikisinin yinelenen kenarları: bulgu “Yinelenen
   kenar”, nesneler (A, B), ölçü A'nın ve B'nin yinelenen kenar uzunluklarının büyüğü, yer en uzun yinelenen kenarın ortası (A'nınkiler
   önce), şekil yinelenen kenarlar. Kesişen iki çizgi kesişimin çevresinde 2t / sin θ boyunca birbirinin t içindedir; bu yüzden ölçüt
   kenarın bütünüdür. Noktalarda: iki nokta nesnesinin birer noktası t içindeyse “Yinelenen nokta”, ölçü en yakın ikisinin uzaklığı, yer
   B'nin o noktası.
5. **Sarkan uç olmamalı.** Her çizgi parçasının iki ucu (başı ve sonu t içindeyse kapalıdır, ucu yoktur): uca t içinde katmanın başka bir
   çizgisi, bir alanın sınırı ya da aynı çizginin ucun kendi kenarı ve ona komşu kenar dışındaki bir kenarı yoksa “Sarkan uç”; ölçü en
   yakın böyle yerin uzaklığı (yoksa boş), yer uç.
6. **Kısa kenar olmamalı** (değer k, varsayılan 0,05 m). Yazıldığı gibi kenarlar: çizginin, çoklu çizginin (parçalarıyla), kapalı alanın
   (halkaları ve parçalarıyla) köşeden köşeye kenarları ve yay nesnesi; uzunluk (yayda yay uzunluğu) < k ise “Kısa kenar”, ölçü uzunluk,
   yer kenarın ortası (yayda yayın ortası). Daire, elips ve eğri bakılmaz.
7. **Küçük açı olmamalı** (değer α, varsayılan 5°). İki kenarın buluştuğu her köşe (halkanın bütün köşeleri; çoklu çizginin iç köşeleri,
   başı ve sonu t içindeyse başı da): köşeden önceki kenara ve sonraki kenara giden doğrultular arasındaki açı (yayda köşedeki teğet;
   0–180°) < α ise “Küçük açı”, ölçü açı, yer köşe. Sıfır uzunluklu kenara komşu köşe bakılmaz (Geçerli olmalı onu bulur).
8. **Geçerli olmalı.** ADR 0201 §6'nın sorunları, adlarıyla ve yerleriyle (yinelenen köşe, alanı sıfır olan halka, kendini kesen halka,
   kendini kesen yol, dış halkanın dışına taşan delik, örtüşen delikler).
9. **Ortak sınırda köşe eksik olmamalı.** İki alandan birinin yazılmış bir köşesi öbürünün sınırının t içinde ama öbürünün hiçbir köşesinin
   t içinde değilse (komşunun kenarında T birleşimi) “Komşuda eksik köşe”; nesneler (köşesi eksik olan, köşenin sahibi), yer köşe. Her iki
   yön ayrı ayrı denenir.
10. **… ile çakışmamalı.** Katmanın her alanı ve öbür katmanın her alanı için 1. kural.
11. **… içinde kalmalı.** Öbür katmanın alanları birleşir (U). Alan: A − U'nun sayılan parçaları, “Dışarıda kalan”, ölçü alanları, yer en
    büyüğünün ağırlık merkezi. Çizgi: U'nun dışında kalan parçaları (ADR 0201'in Kırp'ının kuralı; sınır üstü içeridedir) t'den uzunsa,
    ölçü uzunlukları, yer en uzununun ortası. Nokta: U'nun dışında ve sınırından t'den uzaksa, ölçü uzaklığı, yer nokta. Öbür katmanda alan
    yoksa her nesne bütünüyle dışarıdadır.
12. **Sınırı … sınırlarında olmalı.** Katmanın her alanının sınırının, öbür katmanın alan sınırlarının ve çizgilerinin t içinde olmayan
    kısımları (bir kenardan öbürüne süren kısımlar birleşir) t'den uzunsa “Sınırda olmayan kenar”, nesne başına bir bulgu: ölçü
    uzunlukların toplamı, yer en uzun kısmın ortası, şekil kısımlar.
13. **… çizgilerinin ucunda olmalı.** Öbür katmanın çizgilerinin parça uçları (kapalı parçada başı); katmanın her noktası en yakın uca
    t'den uzaksa “Çizgi ucunda değil”, ölçü en yakın ucun uzaklığı (uç yoksa boş), yer nokta.

Bulgular kuralların sırasıyla, kuralın içinde çekirdeğin sırasıyla (nesneler, çiftler, köşeler) gelir.

### 3. Bulgular ve istisnalar

- **Bulgu**: kural, sorun (anahtarı ve adı), nesneler, yer, kapsam (sorunun şeklinin, yoksa yerin kutusu), ölçü (türüyle: alan, uzunluk,
  açı, uzaklık; yoksa boş), şekil (çizimde gösterilen bölge ya da kenarlar), düzeltmeler (§4) ve istisna olup olmadığı.
- **İstisna**: kuralın kimliği, nesnelerin kalıcı kimlikleri (ADR 0014; bulgunun sırasıyla) ve yer. Bulgu, aynı kuralın, aynı nesnelerin
  aynı sırayla ve yeri t içinde olan bir istisnası varsa istisnadır; nesne düzenlenip yer değişince bulgu yeniden açılır. Kural silinince
  istisnaları da gider; silinen nesnenin istisnası kalır, bir şeye uymaz.
- Denetim çizimi değiştirmez; bulgular oturumdadır, kaydedilmez. Denetimden sonra çizim değişince sekme sonuçların eski olabileceğini
  söyler; Denetle yeniler.

### 4. Düzeltmeler

Bulgudan düzeltme `cad.entities.edit`'in yeni `topologyFix` işlemiyle yazılır (adım “Topoloji düzelt”, tek geri alma adımı; kilitli
katmandaki nesneye dokunan düzeltme reddedilir). Düzeltmeden sonra denetim kendiliğinden yinelenir.

| Bulgu | Düzeltme | Ne yapar |
|---|---|---|
| Çakışma | Birinci nesneden çıkar, İkinci nesneden çıkar | A − B (ya da B − A) A'nın (B'nin) yerine; hiçbir şey kalmayacaksa sunulmaz |
| Boşluk | Komşuya kat | boşluk ortak sınırı en uzun komşuyla birleşir |
| İnce alan | Komşuya kat, Sil | parça ortak sınırı en uzun komşuyla birleşir ve nesneden çıkar; Sil parçayı (tek parçaysa nesneyi) siler |
| Yinelenen kenar | Yineleneni sil | 2t'den uzun bütün kenarları öbürünün üstünde olan çizgi silinir (ikisi de öyleyse B); değilse sunulmaz |
| Yinelenen nokta | Yineleneni sil | B tek noktaysa silinir |
| Sarkan uç | Ucu en yakın çizgiye taşı | çizgi ya da çoklu çizginin ucu §2.5'in en yakın yerine taşınır |
| Kısa kenar | Köşeyi sil, Sil | kenarın sonundaki köşe (açık yolun son kenarında başındaki) silinir, iki kenar düz kenar olur; en az köşe kalmayacaksa sunulmaz; iki köşeli çizgi ve yay nesnesinde Sil |
| Küçük açı | Köşeyi sil | köşe silinir (aynı kural) |
| Geçerlilik sorunu | Onar | ADR 0201'in Onar'ı nesnenin yerine; kendini kesen yolda ve hiçbir şey kalmayacaksa sunulmaz |
| Komşuda eksik köşe | Komşuya köşe ekle | köşe, komşunun en yakın kenarına tam o yerde eklenir (yay kenarı aynı çemberde ikiye bölünür) |
| Dışarıda kalan | Dışarıda kalanı kes | A ∩ U (çizgide içeride kalan parçalar) nesnenin yerine; noktada sunulmaz |
| Çizgi ucunda değil | En yakın uca taşı | nokta en yakın uca taşınır |

Sarkan uç'un ve Sınırda olmayan kenar'ın ayrıntılı onarımı Topolojik temizlik'tir (ADR 0148); sınır için düzeltme yoktur.

### 5. Topoloji sekmesi

Alt panelin yeni sekmesi **Topoloji** (Arama ile Uyarılar arasında):

- Çubuk: **Denetle**, **Kurallar…**, **Kural** listesi (Bütün kurallar ya da tek kural, bulgu sayısıyla), **Açık / İstisna / Hepsi**;
  sonunda bulgu sayısı, **Düzelt ▾** (seçili tek bulgunun düzeltmeleri, nesnesiyle ya da taşıma uzaklığıyla: “Birinci nesneden çıkar
  (#2)”) ve **İstisna yap** ya da **İstisnayı kaldır** (seçili bulgular). Dar panelde çubuğun sonu ikinci satıra iner.
- Tablo: Sıra, Katman, Kural, Sorun, Nesneler (`#id`), Ölçü (projenin biçimleriyle).
- Satıra tıklamak satırı seçer, bulgunun nesnelerini seçer, kapsamına yakınlaşır (Seçime yakınlaştır'ın kuralıyla) ve çizimde bulguyu
  gösterir: şekli kırmızı (bölge yarı saydam dolgulu, kenarlar kalın), yeri Arama'nın işaretiyle ve sorunun adıyla. Ctrl ve Shift
  satırları seçime katar (yakınlaşmadan); İstisna yap hepsine uygulanır.
- Boş durumlar: kural yok (Kurallar… düğmesi), henüz denetlenmedi, bulgu yok (kaç kural, kaç nesne), süzgeçte bulgu yok; denetimden sonra
  çizim değiştiyse uyarı satırı.

### 6. Kurallar penceresi

**Topoloji kuralları**: Tolerans; kural tablosu (Katman, Kural, Öbür katman, Değer, İstisna sayısı); **Kural ekle** (etkin katmanda
Çakışmamalı) ve **Sil** (seçili kural); Kaydet ve Vazgeç. Katman listeleri katmanlardır (gruplar değil); Öbür katman yalnız katmanlar arası türde, Değer yalnız
değer isteyen türde açıktır (uzunluk projenin biriminde, açı projenin açı biriminde). Kaydet doğrular: tolerans aralıkta, değer sıfırdan
büyük (açı 90°'den küçük), öbür katman seçilmiş ve katmandan başka. Silinen kuralların istisnaları gider. Kaydetmek proje ayarıdır, geri
alma adımı değildir (katman durumları gibi).

### 7. Proje dosyası

`.kcad` belge şeması 27 (`docs/specs/kcad-v2.md`): ayarların `topology` alanı (`tolerance` isteğe bağlı, `rules`, `exceptions`); istisnanın
nesneleri 16 baytlık kimliklerdir (§6.3), yeri x, y. Kuralı ya da istisnası olan proje 27 yazar; öbür çizimler şemalarını ve baytlarını
korur. Okuyucu ve yazıcı reddeder: boş ya da yinelenen kural kimliği, bilinmeyen tür, eksik katman, katmanlar arası türde öbür katmanın
yokluğu ya da katmanla aynılığı, öbür katmanı istemeyen türde öbür katman, değer istemeyen türde değer, sonlu ve sıfırdan büyük olmayan
değer ve tolerans, bilinmeyen kurala bağlı istisna, boş nesne listesi.

### 8. Komutlar ve şerit

- `topology.check` **Topolojiyi denetle** (TOPOLOJIDENETLE, VALIDATETOPOLOGY): sekmeyi açar ve denetler.
- `topology.rules` **Topoloji kuralları…** (TOPOLOJIKURALLARI): pencereyi açar.
- Şerit: CBS'de Analiz sekmesinde Karşılaştırma'nın yanında (verinin denetimleri bir arada; Düzenle'nin paneli 3000 px'lik pencereye de
  sığmazdı), CAD'de Yönet sekmesinde Temizlik'in yanında **Topoloji** paneli.

## Uygulama

- **Sözleşme** `crates/shared/contracts/src/topology.rs`: `TopologySettings` (tolerans, kurallar, istisnalar; `sanitized`,
  `tolerance_holds`, `value_holds`), `TopologyRule`, `TopologyRuleKind` (on üç tür; `between`, `value`, `default_value`),
  `TopologyException`; `ProjectSettings.topology`, `EditOperation::TopologyFix`; `FORMATS_VERSION` 37.
- **Çekirdek** `crates/shared/geometry-core/src/ops/topology_rules/`: `mod.rs` (türler, sorunlar ve düzeltmelerin adları, nesnelerin
  sınıfları, çiftler, `check`, istisnaların eşlenmesi), `areas.rs` (çakışma, boşluk ve ortak sınır, ince alan, eksik köşe, içinde kalma,
  sınırda olma), `lines.rs` (yinelenen kenar ve nokta, sarkan uç, kısa kenar, küçük açı, uçta olma), `fixes.rs` (`fix`: on iki düzeltme
  şekil olarak), `calls.rs` (web'in işlemleri `topologyCatalog`, `topologyCheck`, `topologyFix`); `validity::Kind::of_key`.
- **`.kcad`** şema 27: kodek `crates/shared/kcad` (`decode.rs`'in `topology_settings`'i, `encode.rs`), `docs/specs/kcad-v2.md`, bağımsız
  okuyucu `tools/kcad/kcad.py` ve yazıcı `scripts/fixtures/kcad_v2_reference.py`; örnek `fixtures/kcad/v2/topology.kcad`, on bir bozuk
  dosya (`broken/topology-*.kcad`); web `io/kcad.ts`, `model/snapshot.ts`, `model/projectSettings.ts`.
- **Web**: kurallar `model/topologyRules.ts`, çekirdeğin çağrıları `model/ops/topologyRules.ts`; denetim, düzeltme ve istisnalar
  `ui/bottom/topologyRun.ts`, sekmenin kuralları `ui/bottom/topologyPlan.ts`, görünüşü `ui/bottom/TopologyPanel.ts`; pencere
  `ui/topology/TopologyRulesDialog.ts`; bulgunun çizimdeki işareti `viewport/overlay.ts`'in `drawProblemMark`'ı (`model/selection.ts`'in
  `problem`'i); komutlar `app/commands.ts`, şerit `app/ribbon.ts`, simgeler `ui/icons.ts` (`topologyCheck`, `topologyRules`,
  `topologyException`, `topologyFix`; sahibin seçtikleri).
- **Masaüstü** `apps/desktop/src/topology/`: `mod.rs` (denetim, düzeltme, istisnalar, izlerin denetimleri), `plan.rs` (web'in
  `topologyPlan`'ı sözcüğü sözcüğüne), `view.rs` (sekme; dar panelde iki satırlık çubuk), `rules.rs` (pencere); bulgunun işareti
  `marks.rs`, sekme `bottom.rs`, yerleşimde `bottomTab`'ın `topology`'si (`fixtures/shell/v1/layout.json`, `log.json`).
- **İz oynatıcıları** iki platformda: alt panelin sekmesinde `pick` (Düzelt ▾), `row`, `ctrl`, `press`; `topology` beklentisi (sayı
  satırı ve satırlar).

## Doğrulama

- Bağımsız başvuru `scripts/fixtures/topology_rules_cases.py` (ADR'den, KentOS kodu olmadan, çekirdeğin örtüşmesine dayanmadan: eksenlere
  paralel alanlar koordinatlarının hücreleriyle kesin kesirlerde, iki dairenin örtüşmesi kapalı biçimde, kök gerektiğinde mpmath 50 basamak):
  `fixtures/topology-rules/v1/cases.json`, 17 durum: çakışma (gürültü payı ve parçalar), boşluk ve ortak sınırları, ince alan, yinelenen
  kenar ve nokta, sarkan uç, kısa kenar, küçük açı, geçerlilik, eksik köşe, … ile çakışmamalı, içinde kalma, sınırda olma, uçta olma,
  istisnalar ve daireler; her bulgunun nesneleri, yeri, ölçüsü ve düzeltmeleri, düzeltmelerin şekilleri. Çekirdeğin
  `tests/all/topology_rules.rs`'i ve web'in `model/ops/topologyRules.test.ts`'i (WASM) aynı durumları oynatır; ikisi de geçer.
- Komut: `fixtures/commands/v1/cad.entities.edit.json`'da `topologyFix` (yerinde güncelleme ve silme, adım “Topoloji düzelt”, geri alma)
  iki platformda.
- `.kcad`: `crates/shared/kcad/tests/all/topology.rs` (şema 27 yalnız kuralı olan projede, kurallar ve istisnalar yazıldığı gibi geri
  gelir, yazıcı okuyucunun reddettiğini reddeder) ve bağımsız Python okuyucusu ile yazıcısı, örnek dosyalar bayt bayt.
- Ortak iz `fixtures/interaction/v1/topology-rules.json` (sahne `topology-rules.kcad`: altı kural, 18 nesne): denetim 19 bulgu, satırın
  seçimi, Düzelt ▾'in Birinci nesneden çıkar'ı (14 bulgu; çakışma ve dört eksik köşe kalkar), Ctrl ile iki satır ve İstisna yap, İstisna
  ve Hepsi süzgeçleri; masaüstünde ve web'de üç kipte (us, tr-q, hidpi) geçer.
- Masaüstünün testleri (`topology/tests.rs`: sekme ve satırlar, satırın nesneleri ve işareti, düzeltmenin tek geri alma adımı, istisnaların
  projede kalması, pencerenin reddi ve yazması, değerin projenin açı biriminde okunması) ve web'in `topologyPlan.test.ts`'i,
  `topologyRules.test.ts`'i. Resimler `topoloji-*` (masaüstünde `topology::tests::screens`, web'de `shots.mjs topologyrules`; 1440 × 900
  ve 1100 × 650, iki tema).
- Süre (`topology_rules::timing`, release): 100 × 100 parselde (10 000 alan) Çakışmamalı 0,06 s, Boşluk olmamalı 0,15 s, Ortak sınırda
  köşe eksik olmamalı 0,09 s; İnce alan, Kısa kenar, Küçük açı ve Geçerli olmalı birer 0,01 s'den az.
