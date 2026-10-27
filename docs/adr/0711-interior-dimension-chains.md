# ADR 0711: Kesen çizgiyle iç ölçü zinciri

- **Durum:** kabul edildi (27 Ekim 2026); ARCH-DIM-01'in kesen çizgi dilimi.
- **Bağlam:** `plan/architecture.md` ARCH-DIM-01; ADR 0249 (ilişkili ölçü), ADR 0263 (kenar boyunca yer), ADR 0320
  (ölçünün başvuru kuralları), ADR 0340 (duvarın çizgileri), ADR 0343 (kolon), ADR 0695/0700 (duvar ölçü dizileri).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2'ye satırı yazıldı.

## Bağlam

Plan çiziminde en çok konan ölçü iç ölçü zinciridir: bir odanın içinden geçen tek çizgide sırayla duvar kalınlıkları ve
odaların net genişlikleri. KentOS'ta bunu yapmak için her yüz çifti için ayrı Doğrusal ölçü ya da Zincir ölçüyle yüzleri tek
tek tıklamak gerekiyordu; ADR 0695/0700'ün ölçü dizileri tek duvarın kendi boyunu ölçer, odayı enine kesmez.

Araştırma (üretici belgeleri; yerelde çalıştırılmadı):

- [Archicad 29 — Automatic Interior Dimensioning](https://help.graphisoft.com/AC/29/INT/_AC29_Help/070_Documentation/070_Documentation-50.htm):
  elemanları kesen geçici bir çizgi çizilir, sonra zincir bir tıklamayla yerleştirilir; çizgiyi kesen duvarlar, kolonlar,
  kirişler ölçülür. Ayarlar: kolonun ekseni ya da uçları (yuvarlak kolon her zaman merkezinden), “All Skins” ile duvarın her
  katmanı ya da yalnız dış yüzleri.
- [Revit 2026 — Add Aligned Dimensions to Walls Automatically](https://help.autodesk.com/cloudhelp/2026/ENU/Revit-DocumentPresent/files/GUID-C26D95E4-F272-4C67-8C1D-BC26C0493B75.htm):
  Entire Walls seçeneğinde açıklıklar, kesişen duvarlar ve akslar dizinin noktalarına eklenebilir.

KentOS kararı: Archicad'in akışı (kesen çizgi, sonra zincirin yeri) ve seçenekleri (dış yüzler ya da bütün katmanlar, kolonun
uçları ya da merkezi). Ölçüler sıradan, seçilebilir Doğrusal ölçülerdir; her ucu kestiği yüze bağlıdır. Mimar duvarı
kaydırınca değer değişir, duvar uzayıp kısalınca zincir yerinde kalır: yeni “kesen çizgi” kuralı bunu tutar. Alınmayanlar:
yay yüzler (atlanır, söylenir), açıklıklar ve akslar, kirişler ve döşeme kenarları, zincirin üyeliğinin sonradan değişmesi
(kesen çizgiye sonradan konan duvar zincire katılmaz; ADR 0695'in kalıcı dizi tarifi bu dilimde yoktur).

## Karar

### 1. Duvar yüzünde ve kolonda yer

- Duvarın çizgilerinin (ADR 0340 §5: 0 sol yüz, son sağ yüz, aradakiler bileşen sınırları) kenarları üstünde `along`
  yerleri (ADR 0263): noktası, kenetin Yakın/Dik yeri ve kenarın doğrusu (`edge_line`); yapı aynı kaldıkça aynı yer.
- Kolon (ADR 0343): `center` merkezi; dikdörtgen kolonun köşeleri `vertex { 0, 0, k }` (sırası `column_corners`'ınki) ve
  kenarları üstünde `mid`, `along`. Yuvarlak kolonun yalnız merkezi vardır.

### 2. Kesen çizgi kuralı (`cross`, KCAD şema 118)

Doğrusal ölçünün yeni kuralı: `c` noktası kesen çizginin üstündedir, kesen çizgi `c`'den ölçünün doğrultusunda geçer. Düz
kenar anan bir nokta o kenarın doğrusunun kesen çizgiyi kestiği yerdir; başka bir yer anan nokta (kolonun merkezi) yerini
izler. Noktalar kaydıkça `offset` ölçü çizgisini dünyada yerinde tutacak kadar değişir. Kenar kesen çizgiye paralel olursa
nokta yerinde kalır, başvurusu kopar (`no_such_place`); elle taşınan nokta başvurusunu bırakır ve kural düşer (ADR 0320 §1).
Kural yalnız Doğrusal ölçüde, `a` ile `b` bağlıyken, `c` bağsız olur. Şema 118 isteğe bağlıdır: yalnız bu kuralı taşıyan
dosya yazar.

### 3. Hesap: `ops::interior_dims`

- Kesen çizgi (`from` → `to`, en az 1 mm) her duvarın seçilen çizgilerinin (dış yüzler: ilk ve son; bütün katmanlar: hepsi)
  düz kenarlarını ve kolonların dış çizgisini keser; kesişim kenarın ve kesen çizginin içindedir (uçlar dahil), paralel
  kenar kesmez. Yay kenar ölçülmez, sayılır.
- Kolon: uçları seçilince dikdörtgenin kenar kesişimleri; merkezi seçilince (ya da yuvarlak kolonda) kesen çizgi kolonu
  kesiyorsa merkezi. Başka bir duvarın ya da kolonun gövdesinin içinde (sınırından 1e-9 m'den fazla) kalan kesişim görünmez
  yüzdür, atılır (köşe ve T birleşimlerinde, duvara gömülü kolonda).
- Noktalar kesen çizgi boyunca sıralanır (eşitte çizimin sırası, sonra çizgi ve kenar); 1e-9 m'den yakın ikincisi atılır.
  Ardışık her çift bir ölçüdür; en az iki nokta gerekir.

### 4. Komut: `cad.dimensions.interior` v1

- Girdi: `from`, `to`, isteğe bağlı `at` (zincirin çizgisi bu noktadan kesen çizgiye paralel geçer; yoksa kesen çizginin
  üstünde), `faces` (`outer` varsayılan, `skins`), `columns` (`ends` varsayılan, `centers`, `none`), `total` (toplam ölçü
  bir sıra ötede, kâğıtta 8 mm, `at`'ın yanında), `level` (yalnız o mimari katın duvar ve kolonları; yoksa hepsi),
  `layerId`, `dimensionStyle`, `height`, `mask`, görünüş, `expectedRevision`.
- Gizli ya da dondurulmuş katmandaki duvar ve kolon kesilmez (ADR 0278); kilitli katmandaki ölçülür (değişmez).
- Ölçüler `cad.entities.create` yoluyla tek adım “İç ölçü zinciri”; her biri Doğrusal, açısı kesen çizginin, başvuruları
  kestiği yüzlerin yerleri ve kuralı `cross`. Çıktı `EntitiesCreated`, plan `EntitiesCreatePlan`.
- Retler: sonsuz değer `not_finite`; kısa kesen çizgi ve ikiden az nokta `no_dimension`; olmayan kat `level_not_found`;
  katman `layer_not_found`, `layer_locked`; stil `unknown_style`. Uyarı: yay yüz atlandı `arch_curved_face_skipped`.

### 5. Masaüstü: İç ölçü zinciri aracı

Mimari › Belgele'de, Duvar ölçüleri'nin yanında (`tool.interiorDimensions`). İlk tıklama kesen çizginin başı; imleç
ilerledikçe kesilen yüzler işaretli, zincir değerleriyle önizlenir (komutun planı). İkinci tıklama kesen çizgiyi bitirir
(yüz kesmiyorsa nedeni söylenir, çizgi yeniden çizilir); zincir imleçle paralel kayar, üçüncü tıklama yerleştirir, Enter
zinciri kesen çizginin üstüne koyar. Yazılan zincir seçilir, araç sonraki zincir için açık kalır. Ayarlar: Ölçü katmanı,
Ölçü stili, Duvarlarda (Dış yüzler / Bütün katmanlar), Kolonlarda (Kenarlar / Merkezler / Ölçme), Toplam ölçü, Kat.
Esc ve Geri al bir adım geri; yazılan zincir tek Geri al.

## Sınırlar

- Üyelik kalıcı değildir: zincir yazıldıktan sonra kesen çizgiye konan duvar zincire katılmaz; kaldırılan duvarın ölçüsü
  kopuk bağ olarak kalır (ADR 0249 §3).
- Yay yüz, açıklık, aks, kiriş ve döşeme kenarı ölçülmez; duvar yolunun ekseni (referans çizgisi) bu dilimde ölçü noktası
  değildir.
- Eğik kesen çizgide değerler kesen çizgi boyuncadır (Archicad'deki gibi); yüze dik mesafe değildir.

## Doğrulama

Bu dilimde koşulanlar: 44 ortak durum Rust ve Python koşucularında; KCAD takımı (218); çekirdeğin `interior_dims` (5) ve
`subref` (10); belge ve araç ölçü testleri; `kentos-mcp`; masaüstü geçmiş başlıkları; ilgili `cad.dimensions`, `cad.snap`,
`cad.walls`, `cad.columns`, `cad.openings`, `cad.arch`, `cad.grids` ortak durumları. Web'in TS sütun okuyucusu kuralın yeni
bitini bilmez (ADR 0228; TODOS.md §0.2).

- `python3 scripts/fixtures/interior_dims_cases.py --check`: `cad.dimensions.interior.json`'un durumları (oda sırası, T ve
  köşe birleşimi, bütün katmanlar, kolonun uçları ve merkezi, yuvarlak kolon, gömülü kolon, eğik kesen çizgi, kat, gizli ve
  kilitli katman, toplam, `at`, yay yüz uyarısı, plan, retler) ve izleme durumları (duvar kaydırılınca değer, uzatılınca
  yerinde kalan çizgi, paralel olan kenar, silinen duvar) KentOS kodu olmadan.
- `cargo test -p kentos-geometry-core --test all interior_dims` ve `subref`; `cargo test -p kentos-kcad --test all
  dimension_cross`; `KENTOS_CASES_ONLY=cad.dimensions.interior cargo test -p kentos-native-application --test all
  every_command_case`; Python koşucusu; `cargo test -p kentos-mcp`.
- `cargo test -p kentos-interaction --test all interior_dims` (üç tıklama, Enter, Esc, ayarlar, tek geri alma).
- Gerçek GUI resimleri `.run/shots/arac-ic-olcu-*`.
