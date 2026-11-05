# 0951 — Şev uçlarının teşhisi: kesişmeler, duyarlılık ve neden buluşmadığı

Tarih: 2026-11-05. Durum: kabul (Y45). Kapsam ROAD-COR-10.

## Sorun

Kesit çözücüsü (ADR 0903 §3–§5) şevi menteşeden dışa doğru sürer ve araziyle ilk buluştuğu yere şev ucu koyar.
Buluşmazsa yanı "açık" (`Kind::Open`) bırakır ve koridor raporu "şev buluşmuyor" der. Bu tek kelime üç ayrı durumu
saklıyordu:

- menteşede arazi yok;
- şev araziye varmadan arazi bitiyor (yüzeyin kenarı ya da bir boşluk);
- arazi sürüyor ama kesit şeridi şeve yetmiyor.

Her birinin çaresi başka: arazi ölçmek, yüzeyin sınırını genişletmek, şeridi açmak. Buluşan şevde de iki şey
görünmüyordu:

- **Şev araziyi birden çok kez kesiyor mu?** İlk kesişmenin ötesinde arazi yeniden şevin altına iniyorsa şev ucu bir
  tümseğe takılmış olabilir.
- **Şev ucu arazinin kot hatasına ne kadar duyarlı?** Şev araziye yatık bir açıyla varıyorsa küçük bir kot hatası şev
  ucunu metrelerce kaydırır; kamulaştırma sınırı da onunla kayar.

ROAD-COR-10: "Daylight solver hangi terrain branch/sidewall aradığını açık alsın; no-hit, multiple-hit, iteration
limit ve missing terrain ayrı sonuç olsun."

## Karar (`kentos_road::section::template::daylights`)

Her yan için şev, çözücüdeki gibi kurulur: menteşe, yarmada hendek, sonra şev. f(u) = şev − arazi, arazinin sürdüğü
yere kadar, dışa doğru yürünür. Sonuç `Daylight`:

- **`Hit`:** şev ucu (ilk sıfır, çözücününkiyle aynı); şerit boyunca kesişme sayısı (işaret değişimi ya da tam
  çizginin üstündeki nokta); bir sonraki kesişmenin uzaklığı; duyarlılık.
- **`Hinge`:** menteşede arazi yok.
- **`Gap { at }`:** arazi şevden önce bitiyor.
- **`Swath { end }`:** şerit bitiyor.

**Duyarlılık = 1/|m − g|.** m şevin, g arazinin şev ucundaki eğimidir. Arazi δz yükselirse f'nin sıfırı
δz/|f′| = δz/|m − g| kayar: arazinin her metre kot hatası şev ucunu o kadar metre oynatır. Şev araziye dik vardıkça
küçük, paralel vardıkça sonsuza gider (şev araziyle aynı eğimde yatıyorsa çıktıda yoktur).

**Şerit ucundaki kıl payı.** Yüzeyin kesiti şeridin ucunda 5e-15 m'lik bir "yüzey dışı" parça ya da 5,999999999999998'de
biten bir arazi bırakabiliyor. Şerit ucunun 1 µm yakınındaki kesilme, ve oraya düşen şev başlangıcı, şeridin sonu
sayılır, boşluk değil. 6 m'lik şeritte hendeğin şev başını tam 6 m'ye ittiği kesitlerde Rust "boşluk", bağımsız
başvuru "şerit bitti" diyordu; ikisi de artık şerit sonu der.

## Komut: `road.corridors.daylights`

- **Girdi:** koridor ve isteğe bağlı `sensitive` (m/m, 0'dan büyük). Sınır projenindir; KentOS sayı vermez.
- **Çıktı:** her kurulan kesitin iki yanı (tür, işaretli şev ucu, yarma mı, kesişme, sonraki, duyarlılık); sayılar
  (varan, menteşede yok, arazi bitiyor, şerit yetmiyor, çok kesişme, sınırı aşan, kurulmayan); en duyarlı şev ucu.
- **Uyarılar:**
  - `corridor_daylight_crossings`: şev araziyi birden çok kez kesiyor;
  - `corridor_daylight_sensitive`: sınırı aşan şev uçları, km'leriyle.
- **Retler:** `not_a_road_project`, `unknown_corridor`, `invalid_daylight_check`.

## Pencere: Koridor → Görünüş: Şev uçları

- **Resim:**
  - üstte iki yanın duyarlılığı km boyunca, sınır kesikli;
  - altta sol ve sağ şerit, her kesit kendi rengiyle: varıyor, çok kesişme, duyarlı, arazi bitiyor, şerit yetmiyor,
    menteşede yok.
- **Cümle:** sayılar ve en duyarlı şev ucu.
- **Tablo:** yalnız bakılması gereken yanlar.

Pencere yazılan koridoru okur (`preview_of`).

## Ortak durumlar ve ölçüm

- `scripts/fixtures/road_daylight_cases.py`: 10 durum, tam kesirli ve **başka bir yoldan**. Şevin ışını her arazi
  parçasıyla ayrı ayrı kesilir, kesişmeler sıralanır, arazinin kesintisiz uzandığı yer ayrıca bulunur. Durumlar:
  - düz, yükselen ve hendekli arazi;
  - tümsek (üç kesişme);
  - teğete yakın (arazi 0,48, şev 0,5 eğimli: 50 m/m);
  - menteşede arazi yok, boşluk, kısa şerit;
  - şeve değip ayrılan köşe (tam olsun diye ikilik tabanda tam sayılarla: 2 kesişme, sonraki 8,64 m).

  Rust 1e-9 içinde aynı bulur. Her durumda teşhisin şev ucu, kurulan kesitin şev ucudur.
- `scripts/fixtures/road_corridor_daylights_command_cases.py`: 23 durum. İstasyonlar koridorun, arazi, eksen ve
  deverli tip kesit kesitlerin başvurusundan. Rust ve Python 1e-6 içinde aynı.
- **Ortak çizimin ana koridoru** (1/1 ve 1,5/1 şevler, 30 m şerit): 74 yan varıyor, 2 yanda menteşede arazi yok, çok
  kesişme yok; en duyarlı şev ucu km 1+450 sol, 1,94 m/m.
- **Aynı koridor 12/1 yatık şevlerle, 60 m şeritle:** 63 yan varıyor, 4 yanda arazi önce bitiyor, 7 yanda şerit
  yetmiyor, 5 yanda şev araziyi birden çok kez kesiyor. En duyarlı km 1+337.864 sağ: 46,32 m/m (1 cm'lik kot hatası
  şev ucunu 46 cm kaydırır). Önceden bunların hepsi ya "şev buluşmuyor" ya da hiçbir şey demiyordu.

## Açık

- Çözücünün ilk kesişme yerine seçilebilir bir kesişme ("en dıştaki", "yarmada en yakın sırt") alması: bugün yalnız
  söyleniyor, değiştirilmiyor.
- Duyarlılığı arazinin kendi doğruluğuyla (ADR 0945'in NMAD'i) çarpıp şev ucunun belirsizlik bandını kamulaştırma
  sınırına taşımak.
