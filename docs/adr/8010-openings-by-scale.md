# 3010 — Kapı ve pencere ölçeğe göre: parçalar duvarın ayrıntı düzeyiyle çizilir

Tarih: 2026-11-07. Durum: kabul (M11, ADV-15; plan S4).

## Sorun

- **Kapı ve pencere her ölçekte bütün parçalarıyla çiziliyordu:** kasa kesitleri, kanat, açılma yayı, pencerenin
  yüzleri, cam, kayıt, denizlik, eşik, pervaz.
  - Aynı kat 1:200, 1:100 ve 1:50 paftalarında aynı 9 çizgiyle basılıyordu.
  - 1:200'de 5 cm'lik kasa kesiti kâğıtta 0,25 mm'dir. 0,25 mm'lik kalemle çizilince duvar ucuyla birleşip kara leke
    oluyordu (`.run/gui-evidence/mimari-m11/m11-once.png`).
- **Duvar ölçeği izliyordu, açıklık izlemiyordu.** Duvarın üç ayrıntı düzeyi var: Şematik, Uygulama, Ayrıntılı (ADR 0339
  §4). Paftada düzey haritanın ölçeğinden ya da görünüş tarifinden gelir, ekranda yakınlıktan. Kapı ve pencere bunu
  izlemiyordu.

## Araştırma (7 Kasım 2026)

- **TMMOB Mimarlar Odası İstanbul Büyükkent Şubesi, *Mimari Proje Çizim ve Sunuş Standartları*.** Sayfa Chrome ile
  basıldı: `.run/gui-evidence/mimari-m11/referans/mimarist-cizim-ve-sunus-standartlari.pdf`. Kesitler:
  `referans/mimarist-standartlar.png`.
  - §9: ön projeler genellikle 1/100'dür (1/200 ya da 1/50 olabilir). Kesin projeler 1/100, uygulama projeleri 1/50,
    sistem detayları 1/20 çizilir.
  - §14.3.3, ön proje planları: "Kapıların açılış yönleri belirtilir."
  - §14.5.3, uygulama projesi planları: "Bütün doğramalar detayına uygun ve şematik olarak çizilir, açılan kanatları
    belirtilir".
  - §14.5.4, uygulama projesi kesitleri: "Parapet- Denizlik detaylarına uygun çizilir."
- **TMMOB Mimarlar Odası, *Mimarlık Hizmetleri Şartnamesi ve En Az Bedel Tarifesi*.** 28.12.2011 tarihli, 42/31 sayılı
  MYK kararıyla kabul edildi; dayanağı 3458 sayılı Kanun.
  - Fikir projesi, "başka bir ölçekte istemedikçe" 1/200 çizilir.
  - "Sistem ve montaj detayları, (cephe, çatı, temel, merdiven, kapı, pencere, ıslak hacimler vb) en az plan, kesit ve
    görünüş olarak ifade edilir."
- **2026 Dijital Hazırlama Yönetmeliği.** EK-4 4.22 çizgiler için TS EN ISO 128-2'yi esas alır. ISO 128-23 Tablo 1 01.1:
  kapı ve pencerenin *basitleştirilmiş gösterimi* ince sürekli çizgiyle yapılır (ADR 8009).
- **Revit 2024 yardımı** (Revit-Customize, GUID-0BB861D5):
  - "View scales are organized under the detail level headings Coarse, Medium, or Fine."
  - Görselin yazısı: "A custom door created in the Family Editor can display differently at coarse, medium, and fine
    detail levels." (`referans/revit-kapi-ayrinti-duzeyi.png`)
  - Görünüşün kendi ayrıntı düzeyi ölçeğinkini geçer.
- **Doğrulanamayan kaynak.** Bir üniversite ders notunun ("Plan ve kesitte kapı ve pencere", OMÜ) arama özetine göre
  1/200'de kapının "taradığı alan" da gösterilir. Belgenin sunucusuna ulaşılamadı, bu yüzden karar ona dayanmıyor.

## Karar

- **Parçanın düzeyi** (`opening::Part::shown_at`, `opening::pieces_at`). Düzey duvarınkidir (`wall::View`): paftada
  haritanın ölçeğinden ya da görünüş tarifinin kendi ayrıntısından gelir, ekranda yakınlıktan (ADR 0339 §4, 0696).

  | Düzey | Ölçek | Kapı | Pencere |
  |---|---|---|---|
  | Şematik, 1:150'den küçük | 1:200 | kanat | cam (sürmede kanatlar) |
  | Şematik | 1:100 | ve açılma yayı | ve iki yüz çizgisi |
  | Uygulama | 1:50 | ve kasa kesitleri, eşik, ray, pervaz | ve kasa kesitleri, kayıt, denizlik, pervaz |
  | Ayrıntılı | 1:20 | 1:50 ile aynı | 1:50 ile aynı |

  - Kullanım alanı her ölçekte çizilir; boşluğun ve nişin dış çizgisi ile bacanın çaprazı da öyle.
  - **1:200'de yay çizilmez.** Plan (ADV-15) bu ölçek için "boşluk + kanat" diyor. Açık çizilen kanat, Mimarlar
    Odası'nın ön projede istediği açılış yönünü (menteşe tarafını ve açıldığı yüzü) zaten gösterir.
    - Eşik 1:150'dir (`SWING_UP_TO`) ve yalnız şematik düzeyde geçerlidir.
    - Görünüş tarifi Uygulama derse 1:200'de de yay ve kasa çizilir.
    - Ekranda yakınlık yalnız düzeyi seçer; ölçek projenin çizim ölçeğidir (ADR 0696). 1:100 projede uzaklaşınca kasa
      kaybolur, yay kalır. Yay ekranda ancak projenin çizim ölçeği 1:200 ise gider.
  - **Pervaz 1:50'den itibaren çizilir, 1:20'yi beklemez.** Pervaz yalnız kullanıcı genişliğini verince çizilir. Web
    çizimi düzey seçmediği için hep Uygulama'dadır (aşağıda); pervaz 1:20'ye bırakılsaydı webde hiç görünmezdi.
- **Kasası çizilmeyen pencere delik boyunca uzanır.** Kasa kesitleri gizlenince pencerenin yüzleri, camı ve sürme
  kanatları, kasa ve boşluk payı sıfırmış gibi duvar ucundan duvar ucuna çizilir.
  - Önce: çizilmeyen kasa her uçta kendi genişliği kadar boşluk bırakıyordu. Pencere 120'de bu 6 cm'dir, 1:100'de
    0,6 mm; ilk denemenin resminde görüldü.
  - Kullanım alanı ışığın genişliğini korur.
  - Kapının kanadı ve menteşesi yerinde kalır: kanadın gerçek genişliği değişmez.
- **Parça adları ayrıştı** (M10'un rolleri, ADR 8009):
  - `Frame` ikiye ayrıldı: `Frame` kasanın uçlardaki kesitleri, `Face` pencerenin ışık boyunca iki yüz çizgisidir.
  - `Glass`'tan `Mullion` (kayıt) ayrıldı.
  - Kalemler değişmedi: yüz bütün kalemi, kayıt yarısını alır.
- **Çizim yolları:**
  - Depo, açıklığı duvar gibi görünüşe bağlı nesnelerden sayar. Görünüş değişince `set_wall_view` açıklıkları da yeniden
    açar.
  - `expanded_at`, başka ayrıntıdaki bir görünüşe (bölünmüş çizim alanı, pafta haritası) açıklığı o ayrıntıda verir.
    Ekran, pafta ve PDF aynı yoldan geçer.
  - DXF'te `openings::plain_pieces` duvarlarla aynı görünüşü alır (`DxfWriteInput.wallDetail` ve ölçek).
- **Değişmeyenler:**
  - Parçalama (Patlat) bütün sembolü verir.
  - Yakalama ve kırpma kenarları (`ops::edges`), araç anahatları ve genel bakış bütün parçaları kullanır. Gizlenen kasa
    köşeleri duvar ucundan en çok kasa genişliği kadar uzaktadır. Şematik düzeye geçilen yakınlıkta (1:75'ten küçük) bu
    2–3 pikseldir.
- **Web:** web çizimi duvarın görünüşünü seçmez; depo varsayılanı Uygulama ve 1:100'dür. Açıklıklar webde bugünkü gibi
  bütün parçalarıyla çizilir.

## Ölçüm

- **Üç pafta ve PDF** (`crates/native/headless/tests/headless/scales.rs`):
  - Yeni Mimari projesinde 5 × 4 m dikdörtgen duvar (0,5 mm), bir kapı ve bir pencere (0,25 mm) çizildi.
  - Aynı kat `cad.sheets.levels` ile 1:200, 1:100 ve 1:50 paftalara kondu.
  - Her pafta `file.export.pdf` ile yazıldı ve KentOS'un PDF okuyucusuyla geri okundu.
  - Yol sayıları `pdftocairo -svg` ile de sayıldı:

    | Pafta | 0,25 mm (önce → sonra) | 0,125 mm (önce → sonra) | Kapı ve pencere çizgisi |
    |---|---|---|---|
    | 1:200 | 9 → 3 | 2 → 1 | 9 → 2: kanat, cam |
    | 1:100 | 9 → 5 | 2 → 2 | 9 → 5: kanat, yay, iki yüz, cam |
    | 1:50 | 9 → 9 | 2 → 2 | 9 → 9 |

  - 0,25 mm'deki 2 yol paftanın kendisinindir. 0,5 mm'deki 9 duvar yolu her ölçekte aynıdır.
  - Resimler: `.run/gui-evidence/mimari-m11/m11-once.png` ve `m11-sonra.png`. Üst sıra kâğıttaki gerçek boyut, alt sıra
    kapı ve pencere yakından.
- **Gerçek pencere** (Xvfb 1440×900, gerçek fare ve klavye olayları; ADR 8000):
  - `fixtures/interaction/real/kapi-pencere-pafta.txt` el hareketlerini sırayla yapar:
    - yeni proje, 11 × 8 m dikdörtgen duvar, Kapı ve Pencere araçlarıyla birer tık;
    - Model'in yanındaki + ile Mimari pafta;
    - görünüm penceresinin Ölçek (1:...) alanına 50, sonra 200 yazılır;
    - her ölçekte Sığdır ve kapının üstüne sekiz tekerlek yaklaşma.
  - Aynı senaryo M11'den önceki ikiliyle de koşuldu: `.run/gui-evidence/mimari-m11/m11-gercek-pafta.png` (üstte önce,
    altta sonra).
    - 1:200'de önce yay ve kasa vardı; sonra kapı tek kanat, pencere tek cam çizgisi.
    - 1:100'de kasa kesitleri gitti; pencere duvar ucundan ucuna.
    - 1:50 aynı kaldı.
  - Ekranda yakınlık (`kapi-pencere-olcek.txt`): Home 1:62 Uygulama; iki tekerlek uzaklaşma 1:113 Şematik; beş yaklaşma
    1:25 Ayrıntılı.
    - 1:113'te önce ve sonra resimleri piksel piksel aynıdır: 6 cm'lik kasa kesiti orada 2 pikselden küçüktür ve duvar
      ucunun kalın çizgisiyle birleşir.
    - Fark kâğıtta görünür; ekranın şematiğe geçtiği yakınlıkta zaten görünmez.
- **Testler:**
  - `store::walls::a_door_and_a_window_follow_the_stores_view`: deponun görünüşü 1:200 / 1:100 / 1:50 olunca kapı
    1 / 2 / 4, pencere 1 / 3 / 5 parça; başka bir görünüşün parçaları (`expanded_at`) deponunkini değiştirmez.
  - `opening_builds::a_door_and_a_window_show_their_parts_by_the_scale`:
    - Pervazlı, eşikli, kalın kanatlı bir kapı ile kayıtlı, denizlikli bir pencere dört ölçekte parça parça denetlenir.
    - Kasasız çizilen pencerenin yüzleri ve camı x = 1–2'dir (duvar ucundan ucuna); kasalı çizilince 1,05–1,95.
    - Görünüşün ayrıntısı ölçeği geçer. 1:150'de yay durur.
  - `openings::a_larger_scale_draws_every_piece_a_smaller_one_does_and_more`: başvuru fikstürlerinin 81 kapı ve
    penceresi denetlenir.
    - Büyük ölçek küçüğün her parçasını çizer; 1:50 ve 1:20 hepsini.
    - Kanat ve cam her ölçektedir.
    - 1:200'de yay, yüz ve kasa yoktur. Çerçevesiz sürme, cep ve katlanır kapılar her ölçekte aynıdır.
  - `walls::a_door_and_a_window_go_out_as_the_scale_draws_them` (DXF düz parçaları):
    - kapı 1 / 2 / 4, pencere 1 / 3 / 5 parça;
    - 1:100'de pencerenin üç çizgisi duvar ucundan ucuna.

- **Türkçe:** Proje ayarları › Mimari'nin ve duvar stili bileşeninin ayrıntı seçimi "Standart" yazıyordu; sözleşme ve
  Görünüş tarifi aynı düzeye "Uygulama" der. İkisi de "Uygulama" oldu (Türk pratiğinde 1/50 uygulama projesidir).

## Kalanlar

- 1:20'nin kendi çizimi: kasa profili (kasa dişi, conta) ve kanadın kasaya oturuşu (ADV-15 "1:20: profil").
- Ölçeğe göre çizgi grubu → M12. ISO 128-23 §5 genişliği ölçeğe göre seçtirir, Tablo 2 grupları verir (0,25–1); hangi
  ölçeğe hangi grubun düştüğünü söylemez.
- Kolon, merdiven, döşeme, mobilya ve taramaların düzeyi → M12.
- Web çiziminin duvar görünüşünü ölçeğe göre seçmesi (TODOS).
- 1:200'de kalın kanadın (bant) tek çizgiye inmesi.
