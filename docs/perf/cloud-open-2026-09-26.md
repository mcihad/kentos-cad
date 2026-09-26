# Masaüstü: büyük veritabanı projesinin açılışı, ölçüm (26 Eylül 2026)

- **Ne:** masaüstünün bulut istemcisi (`crates/native/cloud`, ADR 0040) gerçek sunucu ve yerel PostGIS'le.
  - Sayfa sayfa açılış (`open`, 2000'lik sayfalar): eşitlemenin istediği, her nesnenin sürümüyle.
  - Tek dosyalık görüntü (`GET …/snapshot`, ADR 0033): karşılaştırma için.
- **Ölçüm:** `apps/api/src/http/native_tests.rs` `opening_a_large_project_measured` (yok sayılır, elle çalıştırılır).
- **Ortam:** Intel Core i5-11300H, Linux, PostgreSQL/PostGIS 18 aynı makinede docker'da, sürüm derlemesi, commit `dc73077`.
- **Çizim:** örnek çizim (13 nesne) ve eklenen noktalar.

| Nesne | İçe aktarım (`project.import`) | Açılış (sayfalar) | Görüntü (tek dosya) | Görüntü boyu |
|---:|---:|---:|---:|---:|
| 10 013 | 0,2 s | 0,1 s | 0,0 s | 1,0 MB |
| 100 013 | 1,9 s | 1,8 s | 0,4 s | 9,7 MB |

- 100 bin nesnede sayfa sayfa açılış 1,8 saniyedir; ilk açılış ve olay kaydı silinince yeniden açılış bu yoldan geçer. Sonraki açılışlar yerel kopyadan gelir (100 bin parsel 0,4 s, `replica-desktop-2026-09-26.md`).
- Görüntü dört kat hızlı, ama nesne sürümlerini taşımaz. Bugünkü sürede sayfaları değiştirmeye gerek görülmedi.

Çalıştırmak için:

```text
KENTOS_TEST_DB=required cargo test --release -p kentos-api --bin kentosd native_tests::opening_a_large -- --ignored --nocapture
```
