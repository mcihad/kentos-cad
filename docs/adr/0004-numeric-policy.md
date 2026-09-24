# ADR 0004: Kadastral sayısal politika sözleşmesi

- **Durum:** kabul edildi (sözleşme ve çekirdek). **Onaylı resmî yuvarlama politikası henüz yoktur.**
- **Tarih:** 2026-09-23
- **Bağlam belgesi:** CLAUDE.md §23, §19 Faz A

## Bağlam

Alan, koordinat ve hisse gibi mülkiyeti etkileyen değerlerde sessiz yuvarlama, kaydırma ya da yaklaşık sonucun kesin gibi kaydı yasaktır (§23). Bugün TypeScript'te alanlar `number` (f64) olarak hesaplanıyor ve `ctx.format` ile gösteriliyor. Bu gösterim içindir; kaynak ya da resmî değer değildir.

## Karar

- **Taşıma:** kesin değerler ondalık metin (`DecimalString`, "748.5151") ya da kesin kesir (`ShareValue`, pay/payda) olarak taşınır. API'de, WASM sınırında, IndexedDB'de ya da dosyada JavaScript `Number`'ına dönüşmez.
- **Kabul edilen metin:** isteğe bağlı eksi işareti, başında sıfır olmayan rakamlar, isteğe bağlı kesir kısmı.
  - Artı işareti, üs (`1e5`), `NaN`, sonsuz ve 28'den fazla anlamlı basamak kontrollü hatayla reddedilir.
  - Yuvarlanıp kabul edilmez.
- **Yuvarlama** yalnızca `NumericPolicy` ile yapılır. Politika `id`, `version`, `status` (`draft` | `approved`), `quantity`, `unit`, `scale`, `rounding` ve `remainder` taşır.
  - Yedi açık kip var: `half_even`, `half_away_from_zero`, `half_toward_zero`, `away_from_zero`, `toward_zero`, `toward_positive`, `toward_negative`.
  - Kütüphanelerin varsayılan yuvarlaması (`toFixed`, `Math.round`, SQL `round`, rust_decimal'ın varsayılanları) iş kuralı olarak **kullanılmaz**.
- **Hisse:** tam kesir olarak tutulur; 1/3 hiçbir zaman 0,3333'e zorlanmaz. Toplamın tam 1 olup olmadığı kesin kesir toplamıyla denetlenir. Taşma olursa sonuç yanlış olmaz, hata verilir.
- **Dağıtım** (alan, hisse):
  - Parçaların toplamı tutmazsa ve artık kuralı `reject` ise işlem `needs_rule` hatasıyla durur; artık kimseye verilmez.
  - `largest_remainder` kuralında artık birer birim, en çok eksik kalan parçaya verilir; eşitlikte giriş sırası geçerlidir. Verilen artık kaydedilir.
  - Artığı rastgele son parsele vermek yoktur.
- **Uygulama:** `crates/shared/geometry-core/src/numeric.rs` (rust_decimal 1.43.0 ve taşma denetimli i128 kesirler). Aynı kod native ve WASM olarak çalışacak. Sözleşme tipleri `crates/shared/contracts/src/numeric.rs`'dedir.
- **Onay:** bu tarihte **onaylı politika yok.** Kesin kadastral işlemler (resmî alan yazımı, ifraz dağıtımı) kurum kuralı doğrulanıp `approved` sürüm girilene kadar açılmaz. Önizleme gösterilebilir.

## Bağımsız doğrulama

Beklenen sonuçlar KentOS kodu kullanılmadan, Python'un kesin `decimal` ve `fractions` modülleriyle üretildi (`scripts/fixtures/numeric_reference.py`):

- **`fixtures/numeric/v1/rounding.json`:** 883 durum; 7 kip × 5 ölçek; tam yarım, eşiğin iki yanı, negatif, büyük koordinat, ret durumları.
- **`shares.json`:** kesin toplamlar.
- **`distribution.json`:** artık kuralları.

Geometri için de ayrı bir referans var: `fixtures/geometry/v1/reference.json`. TM parselin ondalık metinden kesin alanını ve π ile yazılan yay alanlarını taşır. TypeScript, native Rust ve WASM her durumun sınırı içinde kalmalıdır.

## Sonuçlar

- **Gösterim ayrı kalır:** `ctx.format` yalnızca sunum yapar. Onun çıktısı hesaba geri girmez.
- **Alan anlamları ayrı kalır:** hesaplanan alan, tapu kaydındaki alan ve yuvarlanmış gösterim ayrı alanlardır.
- **Yeni gereksinim:** resmî politika tanımı ve onay akışı (kim onaylar, nerede saklanır) Faz B'de proje ve tenant verisiyle gelir.
