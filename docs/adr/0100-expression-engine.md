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

### 2. Sütunla değerlendirme (uygulandı)

- **Derleme** (`program.rs`): ifade bir kez, düz bir komut listesine derlenir. Her komut bir yazmaca yazar; işlenenleri kendinden önceki yazmaçlar ya da sabitlerdir.
  - **Sabitler katlanır:** `yuvarla(2.5)`, `'P' || '-'` derlenirken hesaplanır. Fırlatan bir sabit (`doldur('x', 1e12)`) onu okuyan işlemi de fırlatmış yapar, eskisinde olduğu gibi.
  - **Aynı iş bir yazmaçtır:** bir alan, bir değişken ya da tekrarlanan bir alt ifade, ifadede kaç kez geçerse geçsin nesne başına bir kez hesaplanır.
  - Katlamadan artan sabitler atılır. Yazmaçların başında sabitler durur; bunlar çalıştırma başına bir kez doldurulur.
- **Değerlendirme** (`exec.rs`): nesneler 256'lık partilerle değerlendirilir.
  - Bir yazmaç, partinin her nesnesi için bir değerdir; sütunlar hâlinde tutulur: tür, sayı, metin. Bir komut parti üzerinde tek bir döngüdür.
  - Sayılar `f64` sütunlarında kalır. Bir komutun yaptığı metin yazmacın kendi arabelleğine yazılır.
  - Nesne başına bellek ayırma, trait çağrısı ya da özyineleme yoktur. Nesnelerin değerleri `Source` üzerinden parti başına bir çağrıyla gelir.
- **Hızlı yollar, kuralların yerine geçmeden:** işleçler ve sayı işlevleri önce partinin tamamında, her değer sonlu bir sayıymış gibi çalışır. Sonra dilin kuralları (`scalar.rs`) öyle olmayan nesnelere uygulanır: boş, metin, doğru/yanlış, ∞. Kurallar karar verir; hızlı döngü yalnız aynı sonucu verdikleri yerde işi kısaltır.
  - Sabit bir metne `=` (`Nitelik = 'Arsa'`): sabit boş ya da sayı değilse, metin değerlerde iki metnin karşılaştırılmasıdır.
  - Metin olarak yazılmış sayılar (öznitelikler) tek geçişte okunur: dilbilgisi denetimi ve değer bir arada. 15 anlamlı basamağa ve double'ın tam tuttuğu bir 10 kuvvetine kadar değer, iki tam double'ın tek bir doğru yuvarlanan çarpımı ya da bölümüdür (Clinger'in hızlı yolu); bu, çözümleyicinin verdiği double'ın ta kendisidir. Gerisi çözümleyiciye gider.
  - `yuvarla(x, 2)` gibi sabit basamakla yuvarlamada 10 kuvveti bir kez hesaplanır.
- **Metin:** metin işlevleri aynı sonuçları bir arabelleğin sonuna yazar (`js::text::push_*`, `js::number::push_*`). Eski işlevler olduğu gibi kaldı; test onları başvuru alır.
  - Türkçe sıralama iki metnin ağırlıklarını toplamadan, düzey düzey yürüyerek karşılaştırır. Eski toplayan yolla 50 000 çift bir birim testinde karşılaştırılır.
  - Tamsayılar (|x| < 10¹⁵) doğrudan basamaklarıyla yazılır.
- **Tek nesne** (`walk.rs`): `Expr::evaluate(&dyn Scope)` bir parti kurmaz. Ağacı aynı kurallarla (`scalar`) yürür; iki sonlu sayı ve `ve`/`veya` doğrudan, soldaki yapılmış metne birleştirme yerinde yapılır.
  - Bir argümanın parçası olan sonuç (`kırp`, `eğer`, `varsayılan`) o argümandan, ödünç alındıysa ödünç olarak alınır; bellek yalnız ifadenin yaptığı metin için ayrılır.
  - Stil motoru ve masaüstü İşlemler bugün bu yoldan gider. Sütun yoluna geçmeleri sahipleriyle konuşulur.
- **Web sınırı değişmedi:** `exprEvaluate` aynı tabloyu alıp aynı sütunu döndürür. Tablonun metinleri ASCII ise UTF-16 uzunlukları bayt uzunluğudur; yuvalar artık dilim yerine bayt aralığı (8 bayt) tutar.
- **Doğrulama:**
  - Eski ağaç değerlendiricisi testte donmuş bir kopya olarak durur (`tests/reference`, 23cc8f7'nin kodu, kendi tablosuyla).
  - `tests/differential.rs` web'in üretecini (`cases.ts`) Rust'ta yeniden kurar: geçerli ve bozuk kaynaklar, Türkçe harfler, emoji, Yunanca sigma, sonlu olmayan geometri değerleri, eşit sayılar, fırlatan metinler, parça döndüren metin işlevleri.
  - Her kaynakta şunlar karşılaştırılır: hata iletisi ve konumu, okunan alanlar ve değişkenler; beş biçimin her birinde her değerin türü, biti biti sayısı, metni ve UTF-16 uzunluğu; tek nesne yolunun değerleri.
  - Hata ayıklamada her koşuda 20 000 kaynak koşulur; `EXPRESSION_DIFF_CASES=300000` ile iki farklı tohumda 600 000 kaynak temiz geçti.
  - **Tuzak denetimi:** motora tek tek 22 hata yerleştirildi, testler hepsini yakaladı. Hatalar: karşılaştırma yönleri, eşitlik toleransı, doğruluk, eksi, metin eşitliği, metinden sayıda çıkarma, sabit basamaklı yuvarlama, tek nesne yolunun birleştirmesi ve argüman parçası, hızlı sayı okuma sınırı, yerinde doldurma, tamsayı yazımı, `başlar`, ASCII bölme, `Bool` biçimi, sabit numaralama, fırlatan sabit, Türkçe küçük harf, sıralama.
  - `fixtures/expression/v1` iki platformda değişmeden geçer. Stil motorunun dondurulmuş katmanları ve İşlemler'in ortak durumları da yeni motorla geçer.
- **WASM:** başlangıç WASM'ı 1 207 133 → 1 252 250 bayt, gzip 421 397 → 437 961 (+16,5 KB). Nedeni derleyici, komut döngüleri, tek nesne yürüyüşü ve arabelleğe yazan metin işlevleridir.

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
