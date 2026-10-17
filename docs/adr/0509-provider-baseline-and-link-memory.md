# ADR 0509: Sağlayıcının ölçüm tabanı ve bağlamada bellek

- **Durum:** kabul edildi (2026-10-18). `plan/gis.md` CBS-42'nin ilk dilimi:
  - büyük GeoPackage'da sağlayıcının ölçüm tabanı;
  - bağlamanın veriyle orantısız belleğinin düzeltilmesi.
- **Bağlam belgesi:** ADR 0502 (GeoPackage), 0503 (bağlı katman), [docs/perf/provider-2026-10-18.md](../perf/provider-2026-10-18.md).
- **Sonraya kalan:**
  - bağlı katmanın görünür küme ve sayfa sayfa okunması (bugün çalışma kopyası bütün tabloyu alır);
  - çizim alanında 1 milyon nesnede kenet, seçim ve kare süreleri;
  - soğuk önbellek ve yavaş ağ (PostGIS) ölçümleri;
  - 10 milyon satırlık ve 20 GB'lık stres.

## Bağlam

Planın aday bütçeleri 1 milyon nesne ve 10 milyon köşe içindir: ilk tablo sayfası ≤1 s, bellek veriyle sınırsız
büyümez. Ölçülmemişti. İlk ölçüm, bağlamanın 100 000 parselde 1,3 GB yediğini gösterdi (parsel başına ~12 KB). Bu
gidişle 1 milyon parsel ~12 GB isterdi.

## Karar

1. **Ölçüm:** `provider_timing` (uygulamanın testlerinde, elle koşulur).
   - Sentetik 1 000 000 parsellik GeoPackage'ı bir kez `.run/perf/`'e yazar.
   - Açma, sayma, ilk sayfa, anahtarla sonraki sayfa, mekânsal dizinle görünen kutu ve bağlama işlerinin sürelerini ve
     sürecin belleğini yazar.
   - Ortam ve sonuçlar `docs/perf/provider-2026-10-18.md`'de.
2. **Katmanın özeti akışla:** `sources::layer_digest` nesneleri kimlik sırasıyla tek tek kanonik JSON'a yazıp
   SHA-256'ya katar. Baytlar `digest_of`'un listedekiyle aynıdır, dolayısıyla özet değişmez; bütün listenin JSON'u
   kurulmaz.
3. **Dosyanın SHA-256'sı akışla:** bağlı katmanın ve GeoPackage içe aktarmanın özeti dosya belleğe okunmadan hesaplanır
   (`sha256_reader_hex`).
4. **Yeni katmana bağlamada satırlar bir kez:** dosyadan yeni katmana bağlamada tablonun yalnız tanımı, sistemi,
   yazılabilirliği ve özeti okunur (`load_meta`); satırları içe aktarma okur. Var olan katmana bağlama ve veritabanı
   eskisi gibi bütün satırları okur (karşılaştırmak için). Okunan nesneler bir kez daha kopyalanmaz.

## Doğrulama

- Bağlı katmanın bütün ortak durumları ve uygulamanın bağlı katman testleri değişmeden geçer: özetler aynı.
  `bound_sources_cases.py` dosyanın kendi tablolarından ve hashlib'den denetler.
- 100 000 parselde bağlama 2,47 s, 1 309 MB'tan 1,34 s, 323 MB'a indi (tepe 1 425 MB'tan 451 MB'a).
- 1 milyon parselde: ilk sayfa 0,1 ms, kutu sorgusu 0,6 ms, sayma 41 ms. Bağlama 13,7 s ve 3 GB'tır; bu yolda görünür
  küme yoktur.
