# ADR 0269: Kenet türlerinin kapsamı, Ekleme noktası ve yayın çeyreği

- **Durum:** kabul edildi (2026-10-14). [plan/cad.md](../../plan/cad.md) §7 `CAD-INPUT-05`: “Endpoint, midpoint, center, quadrant,
  intersection, tangent, perpendicular, nearest, insertion, geometric center gibi snap türlerinin mevcut kapsamını tek tek doğrula.”
  Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0029 (kenet deposu), 0163 (kenet ekleri: Ağırlık merkezi, Uzantı, Paralel, Karelaj; katmanın kendi türleri),
  0059 (tek seferlik kenet), 0144 (blok), 0184 (tablo).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

Kenet türleri depoda tek kuralla çalışır (`store::snap`), ama hangi türün hangi nesnede ne verdiği bir yerde yazılı değildi ve bağımsız
bir başvuruyla tek tek sınanmamıştı. Denetim üç eksik buldu:

1. **Ekleme noktası yoktu.** Yazının, blok yerleştirmesinin noktası “Nokta” (node) türüyle geliyordu: kullanıcı noktaları
   kapatmadan yazıların noktalarını kapatamıyordu, tablonun ekleme noktası hiç yoktu. AutoCAD'in INS'i ayrı türdür.
2. **Yayın çeyrekleri yoktu.** Dairenin dört çeyreği vardı; yayın (ve çoklu çizginin yaylı kenarının) üstündeki çeyrekleri yoktu.
3. **Dar yayın merkezi bulunamıyordu.** Depo imlecin çevresindeki nesneleri kutularıyla arar; 180°'den dar bir yayın kutusu merkezini
   içermeyebilir, imleç merkezdeyken yay aday olmaz.

## Karar

### 1. Ekleme noktası (`insertion`)

- Yeni tür `SnapKind::Insertion` (bit 13; `SnapKind::ALL`'ın sonunda, eski bitler değişmez), ağırlığı uç nokta ve Nokta gibi 1.
- Verenler: yazı (çok satırlı ve eğri boyunca yazı dahil) ve blok yerleştirmesi (`p`), tablonun ekleme noktası (köşesiyle aynı yerde
  ekleme noktası olarak).
- Ayar `snap.insertion` (masaüstünün; açık başlar, bugün Nokta'yla gelenler gelmeye devam eder), komut `draft.snap.insertion` (Kenet
  hücresinin menüsünde ve tek seferlik kenette), işareti üst üste iki kare (AutoCAD'in işareti).
- **Katmanın kendi türleri** (ADR 0163 §4) `.kcad`'de adlarıyla durur: Ekleme noktası katmanın listesinde Nokta ile gelir (Çeyrek'in Uç
  nokta ile gelmesi gibi); şema değişmez.
- Web'in donmuş TypeScript yanıtları (`fixtures/geometry/v1/store-v1.json`) Nokta'yı soruyordu: deponun testi TypeScript'in Nokta'sını
  çekirdeğin Nokta'sı ve Ekleme noktası sayar.

### 2. Yayın çeyreği ve eğrinin üstünden merkez

- **Çeyrek:** yayın ve yaylı kenarın, dairesinin dört çeyreğinden (doğu, kuzey, batı, güney; tam değerleriyle) yayın üstünde kalanları.
- **Merkez eğrinin üstünden:** yalnız Merkez istendiğinde (tek seferlik Merkez; AutoCAD'in CEN'i gibi), imleç dairenin ya da yayın
  üstündeyken merkez gelir; eğriler arasında imlece yakın olan seçilir. Sürekli kenette (başka türler de açıkken) bu yol **kapalıdır**:
  yaya tıklamak yayın üstünde kalır (kırıklı yarıçap, yay uzunluğu gibi araçlar yaydaki noktayı bekler). Doğrudan merkezin yanındaki
  imleç her zaman merkezi verir.
- Bir izin beklentisi (`fixtures/geometry/v1/store-v1.json`'un “imleç 83”ü) yayın daha yakın çeyreğine güncellendi: 47 numaralı yayın
  0°'deki çeyreği (merkez + r, tam), imlece eski cevaptan yakındır.

### 3. Kapsam matrisi

| Tür | Verenler |
|---|---|
| Uç nokta | çizginin, çoklu çizginin, alanın köşeleri; yayın, eliptik yayın, eğrinin (Catmull–Rom ve NURBS) uçları; kılavuzun kolu; ölçünün çizgisinin uçları; resmin, rasterin, tablonun köşeleri |
| Orta nokta | düz kenarın ortası; yaylı kenarın ve yayın, yayın üstündeki ortası; NURBS'ün uzunluğa göre ortası; resmin kenar ortaları |
| Merkez | dairenin, yayın, yaylı kenarın merkezi; elipsin merkezi; tek seferlik Merkez'de eğrinin üstünden (§2) |
| Çeyrek | dairenin dördü; yayın ve yaylı kenarın üstündekiler (§2); elipsin ve eliptik yayın üstündeki eksen uçları |
| Nokta | noktanın (çok noktalı nesnenin her noktası); ölçünün ölçülen noktaları; Catmull–Rom eğrisinin iç noktaları |
| Ekleme noktası | yazının, blok yerleştirmesinin, tablonun ekleme noktası (§1) |
| Kesişim | her iki kenarın (düz, yay, elips, NURBS) kesişimi |
| Dik | son noktadan her kenara, elipse ve NURBS'e dikmenin ayağı |
| Teğet | son noktadan daireye, yaya, yaylı kenara ve elipse teğetin değme noktası |
| En yakın | her kenarın ve eğrinin imlece en yakın noktası (başka kenet yoksa) |
| Ağırlık merkezi | kapalı alanın ve kapalı çoklu çizginin (ADR 0163) |

Eliptik yayın orta noktası ve Catmull–Rom eğrisinin teğeti bu matrisin dışındadır; gerekirse `CAD-INPUT-06`'nın işidir.

## Sonuçlar

- Her tür her nesne türünde bağımsız başvuruyla sınanır; yazı ve blok noktaları Nokta'dan ayrı açılıp kapanır.
- Yayın çeyreği ve dar yayın merkezi (tek seferlik Merkez'le) bulunur; sürekli kenetin davranışı yaylarda değişmez.

## Doğrulama

- Bağımsız başvuru `python3 scripts/fixtures/snap_coverage_cases.py --check` (`fixtures/snap/v1/coverage.json`, 42 durum: on bir tür, on üç
  nesne türü, yokluklar; kesirler ve Python'un `math`'i, KentOS kodu olmadan); çekirdekte
  `cargo test -p kentos-geometry-core --test all snap`.
- `cargo test -p kentos-geometry-core --test store` (donmuş TypeScript yanıtları; Nokta ve Ekleme noktası eşliği).
- Masaüstü: `cargo test -p kentos-desktop input_scenes` (tek seferlik Merkez yayın üstünden, yazının Ekleme noktası), Kenet hücresinin
  menüsü (`snap_tests`), izler (`usage-blocks-view`'ın yerleştirme noktaları `insertion`).
- Resimler: `KENTOS_SHOTS_ONLY=kenet-merkez,kenet-ekleme cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`.
