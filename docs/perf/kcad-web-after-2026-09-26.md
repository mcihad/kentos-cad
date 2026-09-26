# KCAD v2 web'de kaydet ve aç: after (2026-09-26, 35caac8)

11th Gen Intel(R) Core(TM) i5-11300H @ 3.10GHz, 8 iş parçacığı, 15 GB; Ubuntu 26.04.1 LTS (7.0.0-34-generic); Google Chrome 154.0.8037.57, başsız, WebGL2 (ANGLE (Intel, Mesa Intel(R) Iris(R) Xe Graphics (TGL GT2), OpenGL ES 3.2)); Vite geliştirme sunucusu, biçim modülü `--profile wasm` (rustc 1.96.0 (ac68faa20 2026-05-25)); Node v24.16.0. 3 koşu, her hücre ortanca (aralık). Betik `apps/web/scripts/perf/kcad.mjs`; toplam 1 dk. Hedefler [ADR 0005](../adr/0005-performance-acceptance-targets.md)'tedir (taslak); bu rapor yalnız kayıttır.

Çizim: parsel başına 20 köşeli bir alan, üç öznitelik (Ada, Parsel, Nitelik) ve etiket, tek katman (`crates/shared/kcad/tests/measure.rs` ile aynı). “Kaydet” uygulamanın kendi Farklı kaydet'idir (`files.saveAs()`, bellekteki bir dosyaya): çizimin alınması, biçim işçisinde kodlama ve doğrulama, dosyaya yazma. “Aç” uygulamanın Aç'ıdır (`files.open()`): dosyanın okunması, işçide çözme, sayfada denetim ve belgenin değişmesi; “çizildi” ilk karenin çizildiği andır. “En uzun görev” sayfanın ana iş parçacığını kesintisiz tutan en uzun görevdir (Long Tasks API): kullanıcının donma olarak gördüğü süre. Bellek, işlem sürerken Chrome'un bütün süreçlerinin PSS toplamındaki en yüksek artıştır (50 ms örnekleme); biçim işçisi kayıttan sonra durdurulur, açma kendi işçisini yeniden başlatır.

| Parsel | Dosya | Kaydet | Kayıtta en uzun görev | Kayıtta bellek artışı | Aç | Aç: çizildi | Açışta en uzun görev | Açışta bellek artışı | Çizimle Chrome |
|---|---|---|---|---|---|---|---|---|---|
| 1 | 0.0 MB | 3 (2–3) ms | 0 (0–0) ms | 0 (0–0) MB | 54 (41–56) ms | 87 (74–90) ms | 0 (0–0) ms | 7 (7–8) MB | 625 (591–628) MB |
| 25 000 | 12.0 MB | 310 (306–318) ms | 0 (0–0) ms | 103 (101–103) MB | 449 (407–461) ms | 494 (452–506) ms | 79 (76–89) ms | 186 (185–194) MB | 819 (743–837) MB |
| 50 000 | 24.1 MB | 565 (553–579) ms | 91 (90–99) ms | 160 (159–162) MB | 761 (746–807) ms | 847 (830–892) ms | 177 (177–194) ms | 270 (259–271) MB | 958 (932–962) MB |
| 100 000 | 48.3 MB | 1 094 (1 074–1 097) ms | 187 (185–189) ms | 322 (318–372) MB | 1 601 (1 540–1 650) ms | 1 781 (1 720–1 825) ms | 347 (333–363) ms | 523 (523–526) MB | 1 160 (1 159–1 160) MB |
| 200 000 | 96.6 MB | 2 168 (2 151–2 242) ms | 361 (360–384) ms | 663 (641–734) MB | 3 313 (3 107–3 359) ms | 3 704 (3 507–3 745) ms | 716 (637–720) ms | 850 (832–927) MB | 1 644 (1 633–1 646) MB |

