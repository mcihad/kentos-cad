# 0991 — Boykesitin varsayılan yeri: çizimin altı

Tarih: 2026-11-06. Durum: kabul (Y85). Kapsam ROAD-SEC-11 ve ADR 0990 §Açık ("paftanın varsayılan yeri").

## Sorun

Boykesitin ve pafta görünümlerinin yeri iki pencerede iki ayrı tahmindi:

- Yol paftaları: sabit "0; -500".
- Boykesit: PI'ların en alçağının 300 m altı.

Y84'ün ilk pafta resminde (1:2000, 40 kat abartma) harita kadar yüksek bir çerçeve oradan plan nesnelerinin (nokta,
some kutusu, PROJE SONU yazısı) üstüne çıktı. Bu nesneler boykesit haritasında, şev satırlarının üstünde göründü.
Tahmin aksın yeriyle de değişmiyordu: başka koordinatlardaki bir projede "0; -500" çizimden kilometrelerce uzaktı.

## Araştırma (6 Kasım 2026)

- **Civil 3D:** görünümün sol alt köşesini (origin) kullanıcı çizimde seçer
  ([To Create a Single Profile View](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-B557BB49-012A-4CDD-9890-B6DC7B7A8B3E.htm)).
  Autodesk'in plan–profil alıştırması noktayı proje alanının dışından seçtirir; görünümler oradan dizilir.
- **Türkiye mevzuatı / KGM:** modelde boykesitin yeri için bir hüküm yok. Pafta düzeni ADR 0980–0981'de.
- **Matematik:** yok. Çizimin kutusu (Tümünü göster'in kutusu) ve görünümün yüksekliği.

## Karar

- **Yer alanı boş başlar,** ipucu "Çizimin altı". Yazılırsa eskisi gibi o yer kullanılır.
- **Boşsa:**
  1. Görünüm önce sıfırda planlanır, hiçbir şey yazılmaz (`road_profiles_draw::plan`, `road_sheets::preview`).
  2. En yükseğinin sol alt köşesinin ne kadar üstüne çıktığı okunur. Paftada KGM satırları (ADR 0989) da eklenir.
  3. Sol alt köşe çizimin kutusunun sol kenarına ve kutunun altına konur: tepe, kutunun 20 kâğıt mm altında kalır
     (çizim ölçeğinde; 1:1000'de 20 m).
- **Boş çizimde** başlangıcın altı.
- **Tek kural** `apps/desktop/src/road_place.rs` (`under_drawing`); iki pencere onu çağırır.
- **Komutlar değişmedi:** `at` yine zorunludur. Yeri pencere seçer, Python ve MCP kullanıcısı yeri kendi verir.

## Ölçüm

- **Testler:**
  - `an_empty_place_puts_the_views_under_the_drawing`: önizlemede en yüksek görünümün tepesi kutunun tam 20 m altında;
    paftaların yazdığı nesnelerin kutusu çizimin kutusunun altında.
  - `an_empty_place_draws_the_profile_under_the_drawing`: boykesitin nesneleri çizimin altında, sol kenardan
    başlıyor.
- **Resimler** `.run/gui-evidence/yol-y85/`:
  - Modelde plan üstte, boşlukla altında görünüm (`y85-model-*.png`).
  - Y84'ün resim testi artık "0; -3000" yazmadan aynı paftayı veriyor.
- **Bulgu** (`y85-sizinti.png`): plan haritasının kendi some kutusu (S-D, S-X, S-Y) ve PROJE SONU yazısı
  haritanın alt kenarından boykesit haritasına taşıyor. Bu kayma modelde değil, plan haritasının yerleşiminde; Y86'da.

## Açık

- **Enkesit penceresi** hâlâ "PI'ların 600 m altı" tahminini kullanıyor; enkesit ızgarasının büyüme yönü ayrıca
  okunmalı.
- **Görünümü çizimde tıklayarak yerleştirme** yok.
