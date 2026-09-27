# ADR 0100: İfade motoru — kendi crate'i, sütunla değerlendirme, tipli alanlar

- **Durum:** kabul edildi (2026-09-27), dilim dilim uygulanıyor. Aşağıda uygulanmış olanla planlanan ayrı yazılmıştır.
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** [docs/PROCESSING.md](../PROCESSING.md) §5 (dil), [docs/STYLE.md](../STYLE.md) (veriye bağlı değerler, kurallar); ADR 0008 “İfade dili” (TypeScript'ten Rust'a taşıma, `fixtures/expression/v1`); TODOS.md `JOB-11`, `DOM-09`–`DOM-13`.
- **Kaynak:** sahibin 27 Eylül kararları:
  - “Çok karmaşık olmayan yüksek hızlı bir expression motoru, uzunluk alan gibi değerlere doğrudan erişebilen ama yüksek hızlı hesaplamalar da yapabilen.”
  - “Özniteliklere erişebilen, şimdiki gibi sadece internal özniteliklere değil; user defined öznitelik olacak, onlara da erişebilen.”
  - İfade alanında kod renklendirme ve kod tamamlama; en sonda isteğe bağlı küçük bir akış (düğüm) görünümü.

## Bağlam

- **Dil nerede:** İşlem araçlarının ve stil motorunun ifade dili (`Nitelik = 'Arsa' ve $alan > 500`) ADR 0008'den beri `crates/shared/style-core/src/expr`'deydi.
- **Kimler kullanıyor:**
  - web'in stil motoru ve İşlemler'i, WASM'daki `exprCompile`, `exprEvaluate` ve `exprCatalog` üzerinden;
  - masaüstünün İşlemler'i (`kentos-processing`, ADR 0084);
  - yakında masaüstünün stil pencereleri (`crates/native/style`).
- **Dondurulmuş yanıtlar:** `fixtures/expression/v1/cases.json`, 400 kaynak. Aynı değer, aynı metin, aynı yerde aynı hata.
- **Değerlendirme:** ifade ağaç olarak, nesne nesne dolaşılıyordu:
  - her nesnede `&dyn Scope` çağrıları, `Value` kopyaları ve özyineleme vardı;
  - öznitelikler hep metindi, sayı her kullanımda metinden okunuyordu;
  - geometri değerleri (`$alan`, `$uzunluk`, `$y`, `$x`) çağıranın hazırladığı ölçü kaydından geliyordu.
- **Maliyet:** 10⁶ nesnede native tek bir `yuvarla($alan, 2)` 56 ms, web yolunda (tablo ve okuma dahil) 75 ms tutuyordu (aşağıda “Ölçüm”).

## Karar

### 1. Kendi crate'i (uygulandı)

- **Yer:** `crates/shared/expression` (`kentos-expression`), §14'ün saf kütüphanesi: DOM, Iced, SQLx, ağ ya da çalışma zamanı bilmez. Bağımlılık yönü denetiminde `shared` grubundadır; hem makine hem wasm32 için derlenir. Tek bağımlılığı `kentos-geometry-core`'dur (jsmath; sonraki dilimlerde geometri değerleri). Yeni dış bağımlılık yoktur.
- **Taşınan:** dilin bütünü değişmeden taşındı:
  - `lexer`, `parser`, `library`, `value`, `rows`;
  - dilin dayandığı JavaScript anlamı `js/`: sayıdan metne, UTF-16 metin, Türkçe sıralama tablosu;
  - dondurulmuş yanıtların native testi `tests/cases.rs`.
- **Eski yollar çalışır:** `kentos_style_core::expr` ve `kentos_style_core::js` bu crate'in yeniden dışa aktarımıdır. Bütün çağıranlar değişmeden derlenir: stil motoru, `svg-core`, `kentos-processing` ve masaüstünün menüleri.
  - `rows::As::convert` açıktır (ADR 0084), nesneleri tek tek değerlendiren çağıran için.
  - WASM bağlayıcısı (`geometry-wasm`) crate'i doğrudan kullanır.
- **WASM:** kod aynıdır; paketin yapısı ve işlemleri değişmedi (başlangıç WASM'ı taşımadan sonra 1 207 133 bayt, gzip 421 397).

### 2. Sütunla değerlendirme (plan, dilim 2)

- **Derleme:** ifade bir kez, düz ve tipli bir programa derlenir.
  - Sabitler katlanır.
  - Her alan, değişken ve geometri değeri nesne başına bir kez okunur.
- **Değerlendirme:** nesneler 256'lık partilerle değerlendirilir.
  - Sayılar `f64` sütunlarında kalır.
  - Üretilen metin partinin tek arenasına yazılır.
  - Nesne başına bellek ayırma ve dinamik çağrı yoktur.
- **Doğrulama:** eski ağaç değerlendiricisi yalnız testte kalır. Rastgele kaynak ve nesnelerle yeni motorla karşılaştırılır: değerler, hata iletileri ve konumları, UTF-16 uzunlukları.
- **Tek nesne:** `Expr::evaluate(&dyn Scope)` tek nesne için kalır.

### 3. Şema ve tipli sütunlar (plan, dilim 3)

- **Adların çözümü derlemede:** çağıranın verdiği şemaya göre, bu sırayla:
  - yerleşik değerler: geometri, tür, katman, etiket, numara;
  - kullanıcının tanımladığı tipli alanlar: sayı, metin, doğru/yanlış, tarih;
  - bugünkü metin öznitelikleri.
- **Bilinmeyen ad:** bugünkü gibi metin özniteliğidir, yoksa boştur; `fixtures/expression/v1` değişmez.
- **Sütunlar:** değerlendirme, çağıranın uyguladığı bir arayüzden sütunları parti parti ister. Sayı alanı `&[f64]`'tir ve sıcak yolda metinden okunmaz.
- **Geometri değerleri** yalnız okunduğunda ve nesne başına en çok bir kez hesaplanır: uzunluk, alan, çevre, köşe sayısı, yer noktası, ağırlık merkezi, sınırlar, genişlik ve yükseklik. Hesap çağırandan ya da geometri çekirdeğinin `Shape`'i üstünde bir yardımcıdan gelir.
- **Kapsam dışı:** öznitelik şemasının kendisi (veri modeli, `.kcad`, bulut) ayrı karardır (TODOS.md `DOM-09`–`DOM-11`). Bu crate yalnız arayüzü ve tipli sütunları olan bir test ev sahibini verir.

### 4. Dil ekleri (plan, dilim 4)

- **Eklenenler:**
  - `CASE WHEN … THEN … ELSE … END`;
  - `[NOT] IN`, `[NOT] BETWEEN`, `[NOT] LIKE` / `ILIKE`;
  - `IS [NOT] NULL` (`boş`);
  - birkaç sayı ve metin işlevi.
- **Adlar:** Türkçe adlar önce gelir, QGIS adları takma addır.
- **Durumlar:** `fixtures/expression/v2`'de, sayıların bağımsız bir Python başvurusuyla.

### 5. Düzenleyici hizmetleri ve arayüz (plan, dilim 5–8)

- **Motorun tablolarından:**
  - sözcük türleri ve konumları (renklendirme);
  - bağlama göre tamamlama: alanlar tür ve kaynaklarıyla, değerler, tek satırlık yardımıyla işlevler, sözcükler;
  - imza yardımı, konumlu tanılar, biçimleme.
- **Arayüz:**
  - web'de `apps/web/src/ui/expression/`;
  - masaüstünde KentOS UI'da genel bir kod alanı ve `apps/desktop/src/expression/`.
  - Web ajanı, stil ajanı ve ana oturum kendi pencerelerine yerleştirir.
- **En son:** metinle gidip gelen küçük bir akış görünümü.

## Ölçüm

`docs/perf/expression-native-eski-2026-09-27.md` ve `docs/perf/expression-web-eski-2026-09-27.md`: bugünkü motor, taşımadan sonra aynı kodla, 10⁵ ve 10⁶ nesnede. Yeni motor aynı betiklerle ölçülür (dilim 2).

## Sonuçlar

- İfade dili tek yerdedir ve stil çekirdeğinden bağımsız gelişir. Stil çekirdeği ona bağımlıdır, tersi değil.
- Eski yollar bir geçiş kolaylığıdır. Çağıranlar sahipleriyle anlaşılarak `kentos_expression`'a küçük adımlarla geçer.
