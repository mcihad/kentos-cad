# 8013 — Ölçeğe göre çizgi grubu

Tarih: 2026-11-08. Durum: kabul (ADV-15; plan S4). Sahibin onayı: 1:200 0,25/0,13 · 1:100 0,35/0,18 · 1:50 0,5/0,25 ·
1:20 0,7/0,35 mm ("Uygun", 8 Kasım 2026).

## Sorun

- **Kalem her ölçekte aynıydı.** Duvar aracı 0,5 mm, açıklık aracı 0,25 mm yazıyor. Pafta 1:200 de olsa 1:20 de olsa
  aynı kalınlıkla basılıyordu.
  - 1:200'de 0,5 mm'lik duvar çizgisi binaya göre çok kalındı: 25 cm'lik duvar kâğıtta 1,25 mm, iki yüzünün çizgileri
    arasında 0,75 mm boşluk kalıyordu (0,25 mm'lik çizgiyle 1 mm). Kapı, pencere ve duvar ucu kara bir lekeye
    dönüyordu.
  - 1:20'de ise aynı 0,5 mm, ayrıntının büyüklüğüne göre cılızdı.
- ISO 128-23 hangi ölçekte hangi grubun kullanılacağını söylemez; seçim çizenindir. Bunun için sahibin kararı istendi
  (M12 sonu), kararı yukarıdadır.

## Araştırma (8 Kasım 2026)

- **TS 88-23 ISO 128-23** (TSE 2003, "Teknik resim – Gösterişle ilgili genel prensipler – Bölüm 23: İnşaat teknik
  resminde çizgiler"), **Tablo 2**; İTÜ, Ecem Edis, *Teknik Çizim İlkeleri* (2013, s. 3: "Farklı çizgi gruplarında
  çizgi tipleri için kalınlıklar (mm olarak)"): `.run/gui-evidence/mimari-cizgi-grubu/referans/itu-tablo2-cizgi-gruplari.png`.

  | Çizgi grubu | Dar | Geniş | Çok geniş | Grafik semboller |
  |---|---|---|---|---|
  | 0,25 | 0,13 | 0,25 | 0,5 | 0,18 |
  | 0,35 | 0,18 | 0,35 | 0,7 | 0,25 |
  | 0,5 | 0,25 | 0,5 | 1 | 0,35 |
  | 0,7 | 0,35 | 0,7 | 1,4 | 0,5 |

  - Tablo 1 (aynı not, s. 2): dar sürekli çizgi "Kapıların, pencerelerin, merdivenlerin, donanımlarının vb.
    sadeleştirilmiş gösterilişi", geniş sürekli çizgi "Kesit taramaları parçaların görünen çevre çizgileri".
  - 2026 Dijital Hazırlama Yönetmeliği EK-4 4.22 çizgiler için TS EN ISO 128-2'yi esas alır (ADR 8009).
- **Revit 2024 yardımı, *Specify Line Weights* ve *Add View Scales for Model Line Weights*:**
  - "Model line weights are associated to a scale." "The width of a particular line weight can change as the scale
    changes."
  - "The width used for a particular line weight will be applied to the associated scale and larger scales until the
    next range is reached." Örnek: "lines using line weight 5 will be printed at .5mm for all scales from 1:100 to
    1:51."
  - Model çizgileri (kapı, pencere, duvar) ölçeğe bağlıdır; açıklama çizgilerinin tek takımı vardır.
- **Mimarlar Odası standardı** çizgi kalınlığını ölçeğe bağlamaz; yalnız vaziyet planında "yükseldikçe kalınlaşan
  çizgiler" der.

## Karar

- **Mimari ayarına `lineGroups` eklendi** (`ArchSettings::line_groups`, `.kcad` şema 218): `byScale` ya da `own`.
  - Yoksa ya da `own` ise her nesne her ölçekte kendi kalemindedir: eski çizimler değişmez.
  - Yeni Mimari projesi `byScale` yazar.
- **Grup ölçeğe göre seçilir** (`kentos_contracts::LINE_GROUPS`, `line_group`):

  | Ölçek | Grup (geniş çizgi) | Dar çizgi |
  |---|---|---|
  | 1:20 ve büyük | 0,7 | 0,35 |
  | 1:21–1:50 | 0,5 | 0,25 |
  | 1:51–1:100 | 0,35 | 0,18 |
  | 1:101 ve küçük | 0,25 | 0,13 |

  - Ara ölçek Revit'in kuralıyla bir küçük ölçeğin grubunu alır: 1:75 1:100 gibi, 1:150 ve 1:500 1:200 gibi çizilir.
- **Nesnenin kalemi 1:50'deki genişliği sayılır** (`LINE_GROUP_BASE` 0,5). Pay = grubun geniş çizgisi / 0,5:
  1:200'de 0,5, 1:100'de 0,7, 1:50'de 1, 1:20'de 1,4.
  - Duvar aracının 0,5 mm'si grubun geniş çizgisi, açıklığın 0,25 mm'si dar çizgisi olur.
  - M10'un parça payı (yay, cam, malzeme sınırı yarısı) bunun üstüne biner: camın 0,125'i 1:200'de 0,0625 olur.
  - TS 88-23'ün 0,13 ve 0,18'i yuvarlanmış değerlerdir; KentOS 0,125 ve 0,175 basar (fark 0,005 mm).
- **Yalnız modelin çizgileri** (`Store::takes_line_group`): duvar, açıklık, kolon, merdiven, rampa, döşeme ve mobilya.
  Duvarın içindeki malzeme taraması duvarın parçasıdır, onunla birlikte gruba uyar. Aks, etiket, görünüş işareti, mahal
  sınırı ve yazısı, ölçü, yazı ve bağımsız tarama nesneleri kendi kaleminde kalır (Revit'in açıklama çizgileri gibi).
- **Nerede uygulanır:** duvarların görünüşü payı taşır (`wall::View::pens`, `ProjectSettings::pens_at`). Depo modelin
  parçalarının `weight_scale`'ini onunla çarpar; ekran, pafta ve PDF aynı yoldan çizer.
  - Ekranda pay çizimin ölçeğinden, paftada haritanın ölçeğinden gelir. Ölçek değişince depo görünüşe bağlı nesneleri
    yeniden açar (`set_wall_view`).
- **DXF nesnenin kendi kalınlığını yazar** (`pens: 1`): DXF'te kalınlık nesnenindir; AutoCAD'de ölçeğe göre kalınlığı
  çizim stili verir.
- **Proje ayarları › Mimari › Çizgi kalınlıkları:** "Ölçeğe göre" ya da "Nesnenin kendi kalınlığı". İkincisi hiçbir şey
  yazmaz. Açıklama grupları ve hangi çizgilerin kendi kaleminde kaldığını söyler.
- **Betik ve ajan:** `cad.settings.set`'in `arch` girdisi `lineGroups`'u alır; komutun özeti onu anar.
- **Sınama değişiklikleri:**
  - M11'in kapı-pencere paftası testi parçaları kalınlığa göre sayıyordu. Artık "nesnenin kendi kalınlığı"yla
    basılıyor: amacı hangi parçanın hangi ölçekte çizildiği, kalınlık bu ADR'nin testidir.
  - Masaüstünün duvar ayrıntısı paftası testi (`wall_detail_sheet_tests`) çizgiyi en çok 0,5 mm sayıyordu; sınır artık
    ölçeğin grubudur (1:20'de 0,7). 1:100'de Ayrıntılı duvarın beş çizgisi artık ayrı basılıyor: 0,35 ve 0,175 mm'lik
    çizgiler 2 ve 3 cm'lik sıvanın kenarını (kâğıtta 0,2 ve 0,3 mm) yüzünden ayırıyor. Önce 0,5 mm'lik yüz onu
    yutuyordu (3 koşu yerine 5 okundu).

## Ölçüm

- **PDF** (`headless/scales.rs::the_lines_print_by_the_line_group_of_their_maps_scale`): aynı kat 1:200, 1:100, 1:50
  ve 1:20 paftada, çizgi grubuyla ve her nesne kendi kalemiyle basıldı. Pafta çerçevesi ayrıca basılıp çıkarıldı, yalnız
  binanın çizgileri sayıldı (mm):

  | Ölçek | Duvar (8 çizgi) | Kapı kanadı, pencere | Yay, cam |
  |---|---|---|---|
  | 1:200 | 0,5 → 0,25 | 0,25 → 0,125 | 0,125 → 0,063 |
  | 1:100 | 0,5 → 0,35 | 0,25 → 0,175 | 0,125 → 0,088 |
  | 1:50 | 0,5 | 0,25 | 0,125 |
  | 1:20 | 0,5 → 0,7 | 0,25 → 0,35 | 0,125 → 0,175 |

  - Resimler: `.run/gui-evidence/mimari-cizgi-grubu/cizgi-grubu-once.png` (önce),
    `cizgi-grubu-once-sonra.png` (aynı kapı ve pencere, üstte önce altta sonra).
- **Gerçek pencere** (`fixtures/interaction/real/cizgi-grubu.txt`): aynı çizim dosyayla açıldı, 1:200 paftası %234'te.
  Önceki derlemede duvar 0,5 mm; yeni projede 0,25 mm. Proje ayarları › Mimari › Çizgi kalınlıkları "Nesnenin kendi
  kalınlığı" yapılıp kaydedilince yeniden 0,5 mm. Resim: `cizgi-grubu-gercek.png`.
- **Testler:** `contracts::arch::plan_cut_tests::the_line_group_goes_by_the_scale_only_when_asked`,
  `store/walls.rs::a_line_group_weighs_the_models_lines_only`, `kcad/tests/all/line_groups.rs`, Proje ayarları
  `the_line_groups_are_written_only_by_the_scale`, yeni proje, PDF.

## Sonraki adımlar

- Web: `io/kcad.ts` şema 218, web çiziminin payı (`pens`) projeden alması, Proje ayarları › Mimari'de seçim (TODOS.md).
- Çok geniş çizgi (taramasız kesit, 1,0 mm grubu) ve grafik semboller sütunu (yazı) bugün kullanılmıyor; taramasız
  kesit çizimi geldiğinde ele alınır.
