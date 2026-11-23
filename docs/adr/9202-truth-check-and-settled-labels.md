# ADR 9202: Çizim alanının doğruluk denetçisi; görünüm durunca yazılar eski yerleşimde kalıyordu

- **Durum:** kabul edildi (2026-11-23). Plan: [plan/render.md](../../plan/render.md) RM-02.
- **İstek:** sahibin sözü (23 Kasım 2026): “zaman zaman özellikle büyük veri olan projelerde yakınlaştırma uzaklaştırma
  yaptığımızda bazı katmanlar ekrana yapışıyor ve bir sıkıntılara yol açıyor objeler ekranda sabit kalıyor yapışık
  kalıyor”. Bulgu: [ADR 9200](9200-render-engine-research.md) §1.2.
- **Sözleşmeler:** yok. Çizim alanına ölçüme özel `Viewport::fresh` eklendi; ayar, komut ve `.kcad` değişmedi.
- **Bağımlılık:** yok.

## §1 Sorun

Çizim alanı, bir önceki karenin işini beş ayrı yoldan yeniden kullanıyor:

- saklanan resim (0120);
- kaydırmada taşınan resim (1067);
- tekerlekte ölçeklenen resim (1068);
- hareket boyunca yazıların eski yerleşimi (0597, 0626 §7);
- iced Canvas önbellekleri.

Her yolun kendi anahtarı var. Anahtarı bir durumu kaçıran yol, artık doğru olmayan bir resmi gösterir. Sahibin
“ekrana yapışıyor” dediği budur. Ama hangi yolun, hangi anahtarın kaçırdığını göstermek için ekranın neyi göstermesi
gerektiğini bilen bir şey yoktu.

## §2 Karar: doğruluk denetçisi

`perf::oracle` (`apps/desktop/src/shell/perf/oracle.rs`) aynı dosyayı iki uygulamada açar:

- birincisi uygulamanın kendisidir, kısayollarıyla;
- ikincisi her kareyi taze çizer (`Viewport::fresh`): saklanan, taşınan ya da ölçeklenen resim yok, hareket boyunca
  tutulan yazı yok.

İkisi de aynı girdiyi alır:

- imleç;
- tek tek tekerlek çentikleri ve dinlenme;
- tek karede on dört çentiklik patlamalar;
- orta tuşla kaydırma;
- “Tümünü göster”;
- aynısı dönük görünümde.

Her kareden sonra çizim alanları piksel piksel karşılaştırılır. Bir pikselin rengi öbür resimde bir piksel yakınında
varsa eşleşir, çünkü kaydırmanın resmi tam piksellerle taşınır (1067). Alanın binde birinden çoğu eşleşmeyen kare
farklıdır. Böyle bir karenin üç resmi kaydedilir: kısayollu, taze ve fark.

**Ölçüt:** hareket sürerken yazıların eski yerleşimi tasarımdır (0597) ve tuval motoruna kadar kalır (RM-13).
**Durmuş bir görünümde fark sıfır olmalıdır.**

## §3 Bulgu: görünüm durunca yazılar eski yerleşimde kalıyordu

İlk koşum, dört dosya, dosya başına 174 kare:

| Dosya | Farklı kare | Durmuş görünümde |
|---|---|---|
| Suşehri DXF | 0 | 0 |
| NCZ | 0 | 0 |
| DWG | 0 | 0 |
| sivas.kcad (1 M nesne) | 36 | **4 / 4** |

sivas.kcad'deki farklar tekerlek karelerinde, alanın %4–7'si kadar ve hepsi yazılarda
(`.run/gui-evidence/render/rm02-once/sivas/`). Kısayollu resimde yazılar daha az ve daha küçük; taze resim görünümün
ölçeğine göre yerleştirilmiş yazıları gösteriyor. Görünüm durduktan sonraki dört dinlenme karesinin dördünde de durum
aynıydı.

**Kök neden.** Hareket sürerken yazılar son yerleşimlerinde tutulur. Ölçeği çeyreği geçmeyen bir tekerlek için son
yerleşim kullanılır, büyüyen yazılar o yerleşimin ekran boylarında kalır (1064 §6). Görünüm durunca yazılar aynı
kamerada yeniden yerleşir. Fakat yazı resminin anahtarı (`Labels::key`) yalnız kamerayı ve görünüşü içeriyordu.

Son tekerlek karesiyle dinlenme karesinin kamerası aynı olduğu için anahtar değişmiyor ve resim yeniden çizilmiyordu:

- **Gerçek boyutlu yazılar:** katmanı (`labels/layer.rs`) önbelleğini temizlemedi.
- **Öbür boylar:** Canvas önbelleği aynı kaldı.

Sonuç olarak tekerleğin başladığı görünüm için yapılmış yerleşim, o görünümün yazı boylarıyla, bir sonraki harekete
kadar ekranda kaldı. Bu yalnız büyük projede görünüyor: küçük çizimde tekerlek kareleri hızlı ve bir sonraki kare
zaten yeniden yerleştiriyor. Bir milyon nesnede ise kare 100–200 ms sürüyor.

**Düzeltme:**

- Yerleşimler sayılır. Sayaç `Spots`'ta tutulur, `get` her yeni yerleşimde bir artırır.
- Yazı resminin anahtarına yerleşimin sayısı ve görünümün hareket edip etmediği eklendi.
- Durunca yapılan yerleşim yeni bir anahtardır; resim onunla çizilir.

## §4 Planda olmayan kararlarım

1. **Karşılaştırma bir piksel komşulukla yapıldı.** Kaydırmanın resmi tam piksellerle taşındığı için tam karşılaştırma
   her kenarda yarım piksellik farkı yapışma sanırdı.
2. **İki uygulama, iki ekransız çizici.** Tek çizicide sırayla çizmek, taze karenin kısayollu uygulamanın saklanan
   resmini bozmasına yol açardı; bozmasa da hatanın birikmesini gizlerdi. İki wgpu aygıtı aynı süreçte sırayla
   kullanıldı, sorun çıkmadı.
3. **Hareket süren karelerdeki fark ölçütte değil.** Yazıların hareket boyunca tutulması 0597'nin kararıdır. Yazılar
   GPU'ya geçince (RM-13) bu fark da kalkacak ve ölçüt bütün karelere genişleyecek.

## §5 Kanıt

**Sınamalar:**

- `labels::tests::a_settled_wheel_draws_the_placing_made_where_it_stopped`: çeyreği geçmeyen bir tekerlek çentiği son
  yerleşimle çizilir; görünüm aynı kamerada durunca yerleşim bir artar.
- `labels::tests` (0597'nin sınamaları) değişmedi ve geçiyor.

**Denetçi, düzeltmeden sonra** (aynı dört dosya, dosya başına 174 kare):

| Dosya | Farklı kare | Durmuş görünümde |
|---|---|---|
| Suşehri DXF | 0 | 0 |
| NCZ | 0 | 0 |
| DWG | 0 | 0 |
| sivas.kcad | 32, hepsi tekerlek dönerken (0597) | **0 / 4** (önce 4 / 4) |

**Gerçek pencerede yeniden üretilemedi, nedeni ölçüldü.** Ortam: `.run/gui-evidence/render/rm02/{once,sonra}-pencere`,
sivas.kcad yakında, tek çentik ve dinlenme, varsayılan ayarlar. Önce ile sonra arasındaki fark çizim alanında 28–157
piksel; bu, iki sürümün arayüz farkı.

- **Neden:** hata, tekerlekten sonraki ilk kare hareketin 150 ms'lik süresi içinde çizildiğinde çıkar. Xvfb'de
  lavapipe ve hata ayıklama derlemesiyle ilk kare bu süreyi kaçırıyor, yazılar zaten yeniden yerleşiyor.
- **Hatanın görüldüğü yer:** release derlemesi ve gerçek GPU (UHD 770) bu süre içinde kalıyor. Sahibin makinesi ve
  denetçinin ortamı budur.
- **Sahibin ayarında görünmez:** gerçek boyutlu yazılarda yerleşim ölçekten bağımsızdır (0626 §6), eski yerleşim
  yenisiyle aynıdır.

## Uygulama

- **Denetçi:**
  - `apps/desktop/src/shell/perf/oracle.rs` (yeni), `perf/mod.rs`.
  - `apps/desktop/src/shell/viewport/{mod.rs,state.rs}`:
    - `Viewport::fresh`;
    - `Program::keep` → `FrameInput::keep_picture`;
    - taze çizimde `moving()` yanlıştır ve ölçeklenen resmin anahtarı yoktur.
- **Yazılar:** `apps/desktop/src/cad/labels/mod.rs`: `Kept`'e yerleşim sayısı, `Spots::get`, yazı resminin anahtarı.
  Sınama: `labels/tests.rs`.
- **Tekrar üretmek:**

  ```text
  KENTOS_PERF_FILES=<dosyalar> cargo test --release -p kentos-desktop --lib perf::oracle::oracle -- --ignored \
    --nocapture --test-threads=1
  ```
