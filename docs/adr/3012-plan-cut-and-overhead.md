# 3012 — Kesme yüksekliği ve üstteki çizgiler

Tarih: 2026-11-08. Durum: kabul (M13, ADV-15; plan S4). Üç adımda yapılır; bu belge her adımda genişler.

## Sorun

- **Pencere her zaman kesilmiş çiziliyordu.** Denizliği 1,60 m olan bir banyo vasistası da planda duvarı kesiyor ve düz
  çizgiyle basılıyordu.
  - Planın kesme yüksekliği 1,10 m'dir (ADR 0719). Boşluk, niş ve baca yalnız bu yüksekliği geçiyorsa kesiliyordu;
    pencere ve kapı her durumda kesiliyordu.
  - Ölçüm (1:50 PDF, `headless/scales.rs`): denizliği 0,9 ve 1,6 olan iki pencere duvarı iki yerden kesiyordu. Duvar
    çizgisi (0,5 mm) 9 yol, pencere çizgileri 0,25 mm'de 10 yol, kesikli yol hiç yok.
- **Kesme yüksekliği sabitti** (1,10 m, `opening.rs`): ofisin ya da projenin kendi değeri verilemiyordu.
- **Üstteki çizgiler yoktu.** Üst katın çıkması, kiriş izi ve asma tavan alt katın planında kesikli görünmüyordu.

## Araştırma (8 Kasım 2026)

- **İTÜ, Ecem Edis, *Teknik Çizim İlkeleri*** (2013, s. 1): "Bir binada herhangi bir katın plan çiziminde, çizimin kat
  döşemesinin yaklaşık 1,20-1,50 m üzerinden yapıldığı kabul edilir. Bu kabul, kattaki pencere boşluklarının
  yerlerinin gösterilmesine olanak tanımaktadır." (`.run/gui-evidence/mimari-m13/referans/`)
- **TMMOB Mimarlar Odası İstanbul Büyükkent Şubesi, *Mimari Proje Çizim ve Sunuş Standartları*** §14.5.3 o, uygulama
  projesi planları: "Tavandaki kirişlerin sarkıntıları, nervür ve kasetler nokta nokta (ifade edecek kadar)
  gösterilir."
- **Revit 2023 yardımı, *About the View Range*:**
  - "The horizontal planes that define the view range are Top, Cut Plane, and Bottom."
  - Kesme düzleminin üstündeki öğeler için: "These elements are not displayed in a plan view unless their categories
    are window, casework, or generic model."
  - Pencere kesme düzleminin üstündeyse kesilmez, görünen çizgiyle çizilir (`referans/revit-GUID-8B8DB6E2…png`,
    `revit-GUID-68FEDE8B…png`).
  - Metrik şablonlarda kesme düzlemi 1200 mm'dir (Autodesk Community; resmî değer değil).
- **2026 Dijital Hazırlama Yönetmeliği** EK-4 4.22 → TS EN ISO 128-2; ISO 128-23 Tablo 1: kesikli ince çizgi (02.1)
  gizli ve görünmeyen çizgiler içindir.

## Karar

### Adım 1: kesme düzleminin üstündeki pencere

- **Pencere de yüksekliğine göre kesilir** (`OpeningForm::cut_by_height`, `geom::opening::cut_in_plan`):
  - denizliği kesme yüksekliğinin altında, lentosu üstünde olan pencere kesilir ve düz çizilir;
  - bütünüyle üstte kalan pencere (vasistas, yüksek pencere) duvarı kesmez; bütün parçaları kesikli çizilir.
  - Kapı yerden başladığı için her zaman kesilir.
  - Yüksekliği verilmemiş pencere yerden tavana sayılır ve eskisi gibi kesilir. Hazır pencere tiplerinin denizliği
    0,9 m'dir; onlar değişmedi.
- **Panel satırları ayrı bir yoklamaya bağlandı** (`OpeningForm::hole_or_recess`): açıklık aracının Yüz, Alt ve Üst
  satırları yalnız boşluk, niş ve bacada çıkar. `cut_by_height` pencereyi kapsayınca pencere panelinde nişin "Yüz"
  satırı belirecekti.
- **Bilgi satırları pencereyi de söyler:** aracın yerleştirme bilgisi ve Öznitelikler'in "Planda" satırı ("Kesilir" /
  "Kesik çizgi") artık pencerede de görünür.
- **Çakışma kuralı** (ADR 0719 §3) aynıdır ama sonucu değişir. Kesmenin üstündeki pencerenin altına, kesmeyi geçen bir
  niş konabilir: yükseklikleri çakışmaz ve plan yalnız nişi keser.
  - Başvuru betiği (`scripts/fixtures/opening_command_cases.py`) aynı kurala geçti.
  - Bir durum yeniden üretildi: "yüksek pencerenin altında kesmeyi kesen niş" artık eklenir (önce reddediliyordu).

## Ölçüm

- **Adım 1, 1:50 PDF** (`headless/scales.rs::a_window_over_the_cut_leaves_its_wall_whole_on_paper`):
  - Aynı duvarda denizliği 0,9 ve 1,6 olan iki pencere çizildi, pafta PDF'e basılıp geri okundu.

    | | Duvar 0,5 mm | Pencere 0,25 mm düz / kesikli | 0,125 mm düz / kesikli |
    |---|---|---|---|
    | Önce | 9 | 10 / 0 | 2 / 0 |
    | Sonra | 5 | 6 / 4 | 1 / 1 |

  - Duvar yalnız alçak pencerede kesilir. Yüksek pencerenin kasa kesitleri ve camı kesikli çizilir; yüzleri duvarın
    yüzleriyle çakışır.
  - Resim: `.run/gui-evidence/mimari-m13/m13-pencere.png` (solda önce, sağda sonra).
- **Testler:** `opening_builds::a_window_over_the_cut_leaves_its_wall_whole_and_is_drawn_dashed`; başvuru
  fikstürleri ve `cad.openings.create` durumları.

## Sonraki adımlar

- Adım 2: kesme yüksekliği proje ayarı olur (Proje ayarları › Mimari). Yeni Mimari projesinde 1,20 m yazılır (İTÜ'nün
  alt sınırı, Revit'in metrik şablonu); ayarı olmayan eski çizimler 1,10'da kalır.
- Adım 3: üst katın döşeme çıkması alt katın planında kesikli çizilir.
