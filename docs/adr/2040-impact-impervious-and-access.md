# 2040 — Alternatif etkisinde geçirimsiz alan ve erişim, değerleri kullanıcının

Tarih: 2026-11-07. plan/map-cad.md MAP-10, ikinci tur. Durum: taslak. Plan satırı: “Donatı, kullanım alanı, nüfus/kapasite,
yaya erişimi, arazi eğimi, altyapıya yakınlık, geçirimsizlik ve bilinen risk katmanları için kaynaklı karşılaştırma
üret. Ölçüm birimi, analiz tarihi ve varsayımlar sonuçla taşınsın.” Kabulde de: “Yanlış birimli veya eksik risk
verisiyle kesin uygunluk sonucu verilmesin.” İlgili: [ADR 1440](1440-design-alternative-impact.md) (Alternatif etkisi).

## Dayanak

- **ArcGIS Urban:** plan panosu alanı ve türetilmiş ölçüleri senaryolar arasında yan yana gösterir, tek puan vermez
  (ADR 1440'ın kaydı).
- **Geçirimsizlik payı:** kullanım türüne göre geçirimsiz yüzey payı yerel yönetmelik ve kurumların tablosudur.
  Rasyonel yöntemin akış katsayıları gibi değerler kaynaktan kaynağa değişir. KentOS bir tablo uydurmaz: pay
  kullanıcınındır.
- **Erişim uzaklığı:** yeşil alana erişim için kullanılan uzaklık (örneğin 300–500 m) plan ve kurum kararıdır. KentOS
  uzaklık önermez.
- **KentOS bugün:** Alternatif etkisi alan, uzunluk, inşaat alanı ve kullanıma göre alanları veriyordu. Erişim ve
  geçirimsizlik ADR 1440'ta açık kalmıştı.

## Karar

1. **`cad.scenarios.impact`'e iki isteğe bağlı girdi** (komut sürümü değişmez; verilmezse ölçü yoktur):
   - **`impervious`:** kullanım kodu → geçirimsizlik payı (0–1).
     - Ölçü `impervious`, “Geçirimsiz alan” (m²): Σ alan × kullanımının payı.
     - Yöntem verilen payları yazar.
     - Alanı olup payı verilmeyen kullanım veri eksiğidir: “Geçirimsizlik payı verilmemiş kullanım: sanayi, yesil”.
   - **`access`:** `distance` (m), `from` ve `to` kullanımları.
     - Ölçü `access`, “Erişen alan (konut → park, yesil, 40 m)” (m²): ağırlık merkezi (`shape_centroid`) en yakın
       `to` alanına en çok `distance` uzaklıktaki `from` alanlarının toplamı.
     - Uzaklık çekirdeğin kesin uzaklığıdır (`spatial_query::distance`; yaylar dahil, merkez alanın içindeyse 0).
     - Ölçü `accessShare`, “Erişen alanın payı” (%): erişen alanın `from` alanlarının toplamına oranı.
     - `from` alanı olup `to` alanı olmayan taraf veri eksiğidir.
   - **Ret** `invalid_input`: 0–1 dışındaki pay (`impervious.<kullanım>`); 0'dan büyük olmayan uzaklık
     (`access.distance`); boş `from` ya da `to`.
2. **Tek puan yok:** ölçüler öbürlerinin yanında yöntemleriyle durur. Varsayımlar değişmez: değerler kullanıcınındır ve
   yöntem satırında görünür.
3. **Masaüstü** (Alternatif etkisi):
   - “Ölçüler (değerler sizin)” bölümü:
     - Geçirimsizlik payları: “konut 0,6; park 0,1” gibi; okunamayan değer söylenir;
     - Erişim kimden / kime;
     - Erişim uzaklığı (m).
   - Boş bırakılan ölçü hesaplanmaz.
   - Yüzde, bir ondalıkla “%44,4” diye yazılır.
   - Gövde artık kayar: kısa pencerede de alt düğmeler görünür.

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/scenarios_impact_cases.py` iki ölçüyü elle aldı. Durum sayısı 21'den 30'a çıktı;
  KentOS kodu yok.
  - Dikdörtgenlerin ağırlık merkezi ortasıdır; kutuya uzaklık içeride 0, dışarıda kutuya olan uzaklıktır.
  - Yeni durumlar:
    - paylar ve geçirimsiz alan: mevcut durum 3690 m², alternatif 2535 m²;
    - payı verilmeyen kullanım;
    - 40 m'de erişim: mevcut durum 0 m², alternatif 1600 m² (%44,4);
    - 50 m'de erişim: ikisi de %100;
    - erişilecek kullanım hiçbir tarafta yok: iki eksik;
    - ikisi birlikte;
    - üç ret.

  Rust ve Python geçer.
- **Masaüstü** (`impact_window_tests::the_impervious_shares_and_the_access_typed_give_the_case_s_measures`):
  - formdan yazılan paylar ve erişim ortak durumun ölçülerini verir;
  - okunamayan pay söylenir.

  Resimler `.run/gui-evidence/alternatif-etkisi-1440/` altındadır.

## Sonra

- Arazi eğimi (yüzeyle), altyapıya yakınlık ve bilinen risk katmanları.
- Ağ üzerinden yaya erişimi (yol ağıyla).
- Damganın çizimde saklanması.
- Zarf ve harita gösterimi.
