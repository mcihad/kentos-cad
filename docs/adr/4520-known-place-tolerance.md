# 4520 — Bilinen noktanın kabul noktasına farkı için kullanıcının sınırı

Tarih: 2026-11-10. plan/map-cad.md MAP-03, dokuzuncu tur. Durum: taslak. ADR 4120'nin “Sınırlar”ındaki “Kabul noktasına
uzaklık için tolerans yok. TBC'de olduğu gibi toleransı kullanıcı verir” (MAP-03'ün açığı: “kabul noktası farkının
kullanıcı toleransı”).

İlgili:
- [ADR 1340](1340-gnss-campaign-archive.md): GNSS arşivi, kabul noktaları;
- [ADR 4120](4120-known-point-own-place-in-the-project.md): bilinen noktanın projedeki kendi yeri ve kabul noktasına
  farkı.

## Dayanak

- **Trimble Business Center**, [Run a Point Comparison Report](https://help.fieldsystems.trimble.com/tbc/21022.htm)
  (10 Kasım 2026'da yeniden okundu):
  - raporun ayarlarında yatay ve düşey arama uzaklıkları ile **yatay ve düşey tolerans** vardır; toleranslar, fark
    uzaklığının kırmızıyla vurgulanıp vurgulanmayacağını belirleyen iki nokta arasındaki yatay ve düşey uzaklıklardır;
  - toleransı aşan çift raporda kırmızıyla vurgulanır;
  - değerleri kullanıcı değiştirir.
- **KentOS bugün** (ADR 4120): `map.survey.query`'nin `places`'i bilinen noktanın projedeki yerini ve işaretinin kabul
  noktasına farkını (`dx`, `dy`, `dz`) verir; bir sınır yoktur, fark değerlendirilmez.

## Karar

1. **`map.survey.query`'ye `tolerance`** (sürüm aynı, ek alan; `places` ile): kullanıcının `horizontal` (yatay uzaklık)
   ve `vertical` (kot farkı) sınırları, metre; biri verilmeyebilir. KentOS bir değer önermez, öntanımlı değer yoktur.
2. **Bilinen noktanın yeri:**
   - kabul noktasıyla karşılaştırılan her yer `horizontal`'ını taşır: √(dx² + dy²) (sınır olmadan da);
   - sınırla `beyond`: yatay uzaklık yatay sınırı ya da |dz| düşey sınırı geçiyorsa `true`, değilse `false`;
   - sınırın değerlendireceği bir şey yoksa (kabul noktası yok; kot yok ve yalnız düşey sınır var) `beyond` yazılmaz.
3. **Retler** (`invalid_input`, ötekilerden önce): `places`'siz sınır (`tolerance`): “Kabul farkı sınırı bilinen
   noktaların yerleriyle okunur: places'i de verin.”; boş sınır (`tolerance`); 0'dan büyük olmayan ya da sonlu olmayan
   sınır (`tolerance.horizontal`, `tolerance.vertical`).
4. **Masaüstü** (GNSS arşivi):
   - bilinen nokta varsa araç çubuğunun altında **Kabul farkı sınırı (m)** satırı: **yatay** ve **düşey** kutuları.
     0'dan büyük bir uzunluk olmayan yazı sınır sayılmaz, kutu kırmızıya döner;
   - arşiv sınırlarla okunur; sınırı aşan bilinen noktanın satırı uyarı rengiyle **Sınırı aşıyor**; ipucunda “Kabul farkı
     sınırını aşıyor.” ya da “… içinde.”;
   - sınırlar oturumundur, projeye yazılmaz.
5. **Kapsam işareti:** ADR 4120 gibi **Harita**.

## Sınırlar

- Sınırlar projede saklanmaz (Proje ayarları › Ölçme'ye yazılması `.kcad` şemasını değiştirir; ayrı karar).
- Arama uzaklığı yoktur: karşılaştırma işaretin kendi kabul noktasıyladır, yakın nokta aranmaz (ADR 4120).
- Kabul noktası olmayan bilinen nokta değerlendirilmez.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/map_survey_cases.py`'nin kuralı, `map_survey_network_cases.py`'nin
  durumları; KentOS kodu yok, PROJ'la): 8 yeni `map.survey.query` durumu (36):
  - N1'in farkı (6 mm doğu, 4 mm güney, 12 mm yukarı; yatay √52 ≈ 7,2 mm): yatay 5 mm aşar, 1 cm içinde; düşey 1 cm
    aşar; yatay 1 cm ve düşey 2 cm içinde;
  - dört ret.

  N1'in eski durumu yalnız `horizontal`'ı kazandı; öbür durumlar aynı. Rust ve Python geçer. Bozma denemesi: düşey sınır
  yok sayılınca koşucu yakalar.
- **Python** (`python/tests/test_known_place_tolerance.py`), testin kendi kuralıyla: N1'in yatay uzaklığı ve kot farkı
  kurgusundan, beş sınırda `beyond`; BN1, BN2 değerlendirilmez; dört ret ve komutun hatası.
- **MCP** (`crates/native/mcp/tests/known_place_tolerance.rs`): 5 mm'de N1 aşar, 1 cm'de içinde; BN1 değerlendirilmez;
  0 sınır reddedilir.
- **Masaüstü** (`map_survey_limits_tests`): yazılan sınırlarla N1'in durumu; ipucunun sözü; sınır olmayan yazı yok
  sayılır. GNSS arşivi testleri ve çapraz denetimler geçer. Resimler `.run/gui-evidence/kabul-farki-siniri-4520/`.

## Sonra

- Sınırların projede saklanması (şema).
- GVX oturum korelasyonları; yeni noktaların hızı (MAP-03'ün öbür açıkları).
- Web (TODOS §0.2).
