# ADR 9230: Parçalardan oluşan tarama sınırı birleştirilip alınır

- **Durum:** kabul edildi (2026-11-25). Plan: [plan/render.md](../../plan/render.md) RM-22, 4. adım.
- **İstek:**
  - planın RM-22 maddesi: “okunamayan tarama sınırları … ele alınır”;
  - sahibin 23 Kasım isteği: “veri kaybolmasın”;
  - 25 Kasım: “Devam et, asla durma, bitene kadar tüm maddeler.”
- **Sözleşmeler:** değişmedi. `.kcad`, komut ve ayar aynı.

## §1 Bulgu

10 DWG örneğinin açılış raporlarından ([ADR 9229](9229-dgn-line-styles-said-by-what-they-draw.md) §3) iki YILTAŞ
dosyasında 130'ar tarama “sınırı okunamadı ya da alanı yok” nedeniyle alınmıyordu.

**Atlanan taramaların hepsi aynı biçimde:**

- Dört sınır yolu var. Her biri tek bir doğru kenar ve “kapalı değil” bayraklı (92 = 33: 1 dış sınır, 32 kapalı
  değil).
- Dördü birlikte bir kareyi kapatıyor. Kenarların uçları birebir aynı sayılar.
- Okuyucu her yolu ayrı halka sayıyordu. İki noktalı halka geçersiz olduğundan dördü de düşüyor, tarama hiç
  alınmıyordu.

**Nerede, neyi etkiliyor** (YILTAŞ İRTİFAK 2007):

| Katman | Sayı | Durum |
|---|---|---|
| KOLON TARAMA | 118 | Kolon taramaları; katman dosyada kapalı (renk −248), kaynak CAD'de de görünmez |
| DIS MERDİVEN | 12 | Dış merdivenin 9 × 9 cm'lik dikmeleri; katman açık, görünür |

Çizimdeki 5 253 taramadan 146'sının en az bir yolu “kapalı değil” bayraklı. Kalan 16'sı tek yollu; o yol kendi başına
halka olduğu için zaten alınıyordu.

## §2 Karar

**“Kapalı değil” bayraklı ve uçları buluşmayan yollar, kendi başlarına halka değil, birlikte kapanan bir sınırın
parçalarıdır.** Okuyucu onları uç uca ekler (`crates/shared/formats/src/dxf/emit.rs`, `stitch`):

- Bir parça, uçlarından biri zincirin ucuna değdiği yerde eklenir; gerekirse ters çevrilir. Değme, parçaların
  kutusunun milyonda biri içinde aranır.
- Zincir iki ucundan da büyür; kapanınca ya da ekleyecek parça kalmayınca biter.
- Birden çok parçadan kurulan halkanın tam eğri sınırı yoktur
  ([ADR 0328](0328-hatch-source-boundary-and-face-ties.md) §1); taraması halkasıyla çizilir.
- **Hiçbir parçanın değmediği tek parça** eskisi gibi kendi halkasıdır, son kenarıyla kapanır, tam sınırı korunur.
  Şimdiye dek alınan taramaların çıktısı değişmez.

## §3 Kanıt

**Ölçüm:** YILTAŞ İRTİFAK'ın raporunda “Tarama (HATCH) · 130 · sınırı okunamadı ya da alanı yok” satırı kalktı.
130 taramanın hepsi alınıyor.

**Sınama:** `kentos-formats` `dxf::a_boundary_of_pieces_is_joined_into_its_ring`.

- Girdi: dört tek doğrulu, “kapalı değil” bayraklı yol bir kareyi kapatıyor; biri ters yönde.
- Sonuç: bir tarama, köşeleri karenin dört köşesi, deliksiz, raporda tarama satırı yok.
- Bütün sınamalar geçti: 221 + 141 + 63 + 6.

**Gerçek pencere** (`.run/gui-evidence/render/rm22/once-sonra-tarama-parca.png`, YILTAŞ İRTİFAK, iki adımda
yakınlaştırılmış dış merdiven): önce boş, sonra iki dikmenin taraması.

## Uygulama

- `crates/shared/formats/src/dxf/emit.rs`: `hatch` parçaları ayırır, `stitch` birleştirir; tek yolların halkası
  `take` ile eskisi gibi.
- `crates/shared/formats/tests/all/dxf.rs`: sınama.
