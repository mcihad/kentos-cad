# ADR 9234: Tıklamanın isabeti en yakından aranır

- **Durum:** kabul edildi (2026-11-25). Plan: [plan/render.md](../../plan/render.md) RM-08, 1. adım.
- **İstek:**
  - planın RM-08 maddesi: “En uzak görünümde tıklamanın isabet sorgusu en yakını önce arayacak”;
  - sahibin “LOD ve performans” isteği;
  - 25 Kasım: “Devam et, asla durma, bitene kadar tüm maddeler.”
- **Sözleşmeler:** değişmedi. Tıklamanın seçtiği nesne aynı; eşitlikte belge sırası aynı.

## §1 Bulgu

[ADR 9206](9206-selection-without-whole-drawing-scans.md) §3, seçim karesinin arayüz payını 64,9 ms'den 2,3 ms'ye
indirmişti. Kalan 37 ms isabet sorgusuydu (SİVAS AVM DWG, 601 619 nesne, release, UHD 770).

`Store::hit` imlecin çevresinde toleransın 1,5 katı kadar bir kutu arıyordu:

- **Uzak görünümde** tolerans dünyada metrelerce. Kutu nesnelerin sekizde birinden fazlasını kapsayınca
  `Store::candidates` bütün nesneleri belge sırasıyla geziyordu.
- Her biri için görünürlük ve kenara uzaklık hesaplanıyordu.
- **Bedeli:** tıklama ve gezinme kartının her imleç hareketi.

## §2 Karar

**İsabet en yakından aranır** (`crates/shared/geometry-core/src/store/pick.rs`, `hit`):

- Önce imlecin çevresinde küçük bir kutu (erişimin 64'te biri) aranır; her turda kutu dört katına büyür, en çok
  erişime kadar.
- **Durma kuralı:** bulunan en yakın kenar kutunun yarı genişliğinden uzak değilse cevap odur.
  - Ondan yakın ya da ona eşit uzaklıktaki her nesnenin kutusu o uzaklığın içindedir, yani küçük kutudadır.
  - Küçük kutunun adayları da belge sırasıyla gelir, eşitlik eskisi gibi bozulur.
- **Kenar yoksa** son tur bugünkü sorgunun kendisidir; imleci içeren en küçük alan oradan çıkar.
- **Kötü durum:** seyrek bir yerde birkaç küçük arama ve ardından eskisiyle aynı tam arama.

`hits` (Sıradakini seç, bütün adaylar sıralı) değişmedi; isteğe bağlı ve her imleç hareketinde çalışmıyor.

## §3 Kanıt

**Sınama:** `kentos-geometry-core` `store::pick::tests::the_nearest_first_pick_is_the_whole_reach_s_pick`.

- Kurulum: 1 600 çizgi, noktalar, üst üste iki eş çizgi ve iç içe alanlar.
- 2 500 tık ve üç tolerans (yakın, orta, uzak görünüm): her birinde cevap tam erişimin cevabıyla aynı.
- Üst üste iki çizgide öndeki seçilir.

Deponun bütün sınamaları (97), sahne sorgusu (9 + 8), masaüstü seçim sınamaları (110) ve çağrı fikstürleri geçti.

**Ölçüm** (release, 600 000 çizgi, uzak görünüm toleransı 600 m, 50 tık): tık başına tam erişim 6,18 ms, en yakından
0,005 ms. Cevaplar aynı.

## Uygulama

- `crates/shared/geometry-core/src/store/pick.rs`: `hit` turlarla, `hit_within` eski gövde; sınama.
