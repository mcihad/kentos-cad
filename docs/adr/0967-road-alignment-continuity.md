# 0967 — Güzergâhta süreklilik: konum, doğrultu, eğrilik

Tarih: 2026-11-05. Durum: kabul (Y61). Kapsam ROAD-ALIGN-05.

## Sorun

ROAD-ALIGN-05: "Doğru→spiral→kurp→spiral→doğru zincirinde konum/teğet/eğrilik süreklilik koşulları ve kalan residual
raporlansın."

Aks raporu iki şey veriyordu:

- **Kalan artık:** Kurbun çıkış klotoidinin integralle bulunan ucu, PI geometrisinin söylediği yerden ne kadar ayrılıyor;
  konumda (m) ve doğrultuda (rad).
- **Eksik:** Elemanların birleştiği yerlerde süreklilik raporlanmıyordu. Eğrilikse hiç söylenmiyordu: klotoidsiz bir
  kurbun teğetle eğrilik sıçramasıyla birleştiği rapordan okunmuyordu.

## Araştırma

- **Geometrik süreklilik (CAD ve eğri tasarımı):** Kabul görmüş sınıflandırma şudur:
  - G0: konum sürekli;
  - G1: teğet doğrultusu sürekli;
  - G2: eğrilik sürekli.
- **Yol tasarımında:** Klotoid, doğru ile daire arasında eğriliği 0'dan 1/R'ye doğrusal götürür. Amacı G2'dir:
  direksiyon açısı sürekli değişir.
- **Klotoidsiz kurp:** Doğru ile daire G1 birleşir, eğrilik 1/R sıçrar. Birçok yönetmelik büyük yarıçapta buna izin
  verir.
- **Kırık:** Kurpsuz bir PI G1'i de bozar.

## Karar

`Alignment::seams` her iki komşu elemanın birleşimini verir:

- sonraki elemanın km'si;
- iki elemanın kimliği;
- konum farkı |önceki elemanın ucu − sonrakinin başı| (m);
- doğrultu farkı |θ_son − θ₀| (rad, ±π içine sarılmış);
- eğrilik farkı |κ_son − κ₀| (1/m).

`road.alignments.*`'ın aks raporunda `seams` alanı olarak yer alır.

**Konum ve doğrultu farkı kuruluş gereği 0'dır.** Zincir her elemanı bir öncekinin hesaplanan ucundan ve doğrultusundan
başlatır. Ölçülen asıl artık, kurp sonunun PI'nın söylediği yerden ayrılmasıdır; bu `residual` ve `residualDirection`
olarak kalır.

**Eğrilik farkı tasarımın özelliğidir, hata değildir:**

- klotoidli kurpta 0 (G2);
- klotoidsiz kurpta 1/R;
- kurpsuz PI'da doğrultu farkı sapma açısıdır.

**Paralel aks:** Kaynağın birleşimlerini taşır. Klotoidli zincirin ötelemesinde konum, doğrultu ve eğrilik sürekli
kalır. Ötelenmiş eğrilik κ/(1 − κ·d), κ sürekliyse süreklidir.

**Güzergâh penceresi** durum satırında söyler:

- kırıkları PI ve açıyla: "p1 kırık 90,000°";
- eğrilik sıçramalarını sayıyla: "2 yerde eğrilik sıçrar (klotoidsiz kurp)";
- ikisi de yoksa: "konum, doğrultu ve eğrilik sürekli".

## Ölçüm

### Bağımsız referans

`scripts/fixtures/road_alignment_command_cases.py` birleşimleri referansın kendi elemanlarından (mpmath, klotoid
serisi) kurar. Rust'ta ve Python'da geçer.

| Aks | Birleşimler | Sonuç |
|---|---|---|
| g-1, R 400, klotoid 80/80 | doğru→giriş→kurp→çıkış→doğru | dört birleşimde fark 0 (referansta konum 6·10⁻³⁶, yuvarlama) |
| g-1, klotoidler kaldırılmış | doğru→kurp→doğru | eğrilik 0,0025 = 1/400 iki yerde; konum ve doğrultu 0 |
| g-3, kurpsuz p1 | doğru→doğru | doğrultu 1,570796 rad (90°), eğrilik 0 |

Yeni düzenleme durumu "Klotoidler kaldırılır: eğrilik 1/R sıçrar" eklendi.

**Rust'ın haklı olduğu bir ret.** Referansa ilk önce "Kurp kaldırılır: PI kırık olur" durumu da eklendi; Rust onu
reddetti: `offset_cusp`, "Sol kenar paralel aksı km 596,000 ile 602,996 yakınında kendini kesiyor". g-1'in 7,5 m'lik
paralel aksı kırığın iç yanında kendini keser; bu ADR 0901'in paralel aks kuralıdır. Referansın düzenleme modeli bu
kuralı bilmiyordu. Rust haklı olduğu için durum çıkarıldı. Kırık, sorgudaki g-3'te
sınanır.

### Pencere testi

`the_window_says_the_chain_s_continuity`, klotoidli aksta "sürekli", klotoidler sıfırlanınca "2 yerde eğrilik sıçrar"
okur.
