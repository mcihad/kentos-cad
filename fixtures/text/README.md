# Yazı kuralları fixture'ları (`kentos.text-cases` v1)

[ADR 0145](../../docs/adr/0145-text-extras.md) §3: yazının düzenleme kuralları. Dosyaları [`scripts/fixtures/text_cases.py`](../../scripts/fixtures/text_cases.py) kuralların kendisinden, bir uygulamanın çıktısından değil, yazar; `--check` farkı arar.

| Dosya | Kural |
|---|---|
| `v1/increment.json` | Artır: yazının bittiği ASCII rakamları bir artar, en az eski basamak sayısıyla (`A-009` → `A-010`, `99` → `100`); sonu rakam olmayanın sonrakisi yoktur. |
| `v1/pattern.json` | Bul ve değiştir: jokersiz her geçtiği yer (soldan, örtüşmeden; tam sözcük seçeneğiyle), jokerli bütün yazıya uyan kalıp (`*` soldan en kısası; değiştirmedeki `*`'lar sırasıyla); büyük küçük harf Türkçe katlamayla. |
| `v1/readable.json` | Okunur yap: 90°'den büyük, en çok 270° dönük yazı kutusunun ortası çevresinde yarım döner; noktası w·(1 − 2a) boyuna, h·(0,92 − 2b) yukarı kayar. Genişlik (`width`) verilir. |

Koşucular: çekirdek (`crates/shared/geometry-core/tests/text.rs`, çağrı tablosundan) ve web (`apps/web/src/model/textEdit.test.ts`, WASM'dan). Masaüstü aynı işlevleri doğrudan çağırır.
