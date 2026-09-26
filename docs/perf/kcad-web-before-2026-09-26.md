# KCAD v2 web'de kaydet ve aç: before (2026-09-26, 82dcbb9)

11th Gen Intel(R) Core(TM) i5-11300H @ 3.10GHz, 8 iş parçacığı, 15 GB; Ubuntu 26.04.1 LTS (7.0.0-34-generic); Google Chrome 154.0.8037.57, başsız, WebGL2 (ANGLE (Intel, Mesa Intel(R) Iris(R) Xe Graphics (TGL GT2), OpenGL ES 3.2)); Vite geliştirme sunucusu, biçim modülü `--profile wasm` (rustc 1.96.0 (ac68faa20 2026-05-25)); Node v24.16.0. 3 koşu, her hücre ortanca (aralık). Betik `apps/web/scripts/perf/kcad.mjs`; toplam 3 dk. Hedefler [ADR 0005](../adr/0005-performance-acceptance-targets.md)'tedir (taslak); bu rapor yalnız kayıttır.

Çizim: parsel başına 20 köşeli bir alan, üç öznitelik (Ada, Parsel, Nitelik) ve etiket, tek katman (`crates/shared/kcad/tests/measure.rs` ile aynı). “Kaydet” uygulamanın kendi Farklı kaydet'idir (`files.saveAs()`, bellekteki bir dosyaya): çizimin alınması, biçim işçisinde kodlama ve doğrulama, dosyaya yazma. “Aç” uygulamanın Aç'ıdır (`files.open()`): dosyanın okunması, işçide çözme, sayfada denetim ve belgenin değişmesi; “çizildi” ilk karenin çizildiği andır. “En uzun görev” sayfanın ana iş parçacığını kesintisiz tutan en uzun görevdir (Long Tasks API): kullanıcının donma olarak gördüğü süre. Bellek, işlem sürerken Chrome'un bütün süreçlerinin PSS toplamındaki en yüksek artıştır (50 ms örnekleme); biçim işçisi kayıttan sonra durdurulur, açma kendi işçisini yeniden başlatır.

| Parsel | Dosya | Kaydet | Kayıtta en uzun görev | Kayıtta bellek artışı | Aç | Aç: çizildi | Açışta en uzun görev | Açışta bellek artışı | Çizimle Chrome |
|---|---|---|---|---|---|---|---|---|---|
| 1 | 0.0 MB | 2 (2–2) ms | 0 (0–0) ms | 0 (0–0) MB | 21 (21–22) ms | 25 (24–32) ms | 0 (0–0) ms | 7 (7–7) MB | 620 (588–623) MB |
| 25 000 | 12.0 MB | 1 641 (1 635–1 677) ms | 115 (108–116) ms | 316 (284–331) MB | 1 071 (1 059–1 209) ms | 1 195 (1 149–1 290) ms | 297 (283–338) ms | 226 (220–230) MB | 816 (736–854) MB |
| 50 000 | 24.1 MB | 3 122 (3 093–3 194) ms | 251 (250–264) ms | 563 (562–592) MB | 2 149 (2 133–2 201) ms | 2 333 (2 328–2 397) ms | 588 (580–613) ms | 419 (399–431) MB | 965 (945–966) MB |
| 100 000 | 48.3 MB | 6 436 (6 333–6 553) ms | 591 (589–595) ms | 1 139 (1 109–1 224) MB | 4 364 (4 327–4 461) ms | 4 721 (4 689–4 844) ms | 1 244 (1 223–1 247) ms | 950 (913–964) MB | 1 264 (1 158–1 267) MB |
| 200 000 | 96.6 MB | 14 417 (14 203–14 552) ms | 1 227 (1 214–1 239) ms | 2 192 (2 191–2 325) MB | 10 150 (9 386–10 827) ms | 10 878 (10 105–11 544) ms | 3 263 (2 430–3 347) ms | 1 684 (1 681–1 753) MB | 1 776 (1 632–1 777) MB |

