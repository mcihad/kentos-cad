# 3016 — Mimari ikon seti

Tarih: 2026-11-08. Durum: kabul (M16, ADV-10; plan S16).

## Sorun

- **Mimari sekmesinde on komut üç ikonu paylaşıyordu** (`catalog/v1/additions.json`):
  - `space`: Mahal, Toplu mahal bul, Mahal ayırıcı, İç nokta seç (M17'den beri Mahali yeniden sınırla), Mahal kaynakları;
  - `wall`: Duvar, Dikdörtgen duvar, Ölçülü bölme;
  - `tag`: Etiket ve Mahal numarala.
  - Mahal ▾ açılınca üç satırda aynı resim vardı. DESIGN.md §6: "bir açılır listede aynı simge iki kez yoktur."
- **Üç mimari araç genel CAD aracının ikonunu ödünç alıyordu:** Duvar ölçüleri Ölçülendirme'nin (`dimension`), Açıklık dağıt
  Dizi'nin (`array`), Duvarı kaydır Paralel kaydır'ın (`edgeShift`).
- **Yedi çizelge ikonu aynı tablonun üstüne küçük bir işaret koyuyordu** (Metraj, Kapı ve pencere, Mahal, Mahal kaplama,
  Donatı, Bölüm, Revizyon listesi). İşaret 16 px'te 3–4 pikseldi, ayırt edilmiyordu.
- **Düzenle paneli Duvar'ın ikonuyla katlanıyordu:** dar pencerede Duvar ve Düzenle aynı resimdi.
- **İkon turunda Mimari yoktu** (`icon_tour.rs`, 37 sahne).

## Araştırma (8 Kasım 2026)

- **Revit 2023 yardımı, *Parts of the User Interface*** (resmî resim:
  `.run/gui-evidence/mimari-m16/referans/revit-2023-build-room-area-paneli.png`): Room & Area panelinde Room, Room Separator ve
  Tag Room'un her birinin kendi ikonu vardır. Oda kutusu ortaktır, ayıran şey üstündeki işarettir (çarpı, ayırıcı çizgi, etiket).
- **IEC 80416-1:2008** (EN 80416-1:2009, *Basic principles for graphical symbols for use on equipment — Part 1*): simge temel bir
  ızgarada, sabit çizgi kalınlığıyla çizilir. TS karşılığı aramada doğrulanamadı. KentOS'ta karşılığı DESIGN.md §6: 20×20
  ızgara, 1,4 px çizgi, tıklanan noktalar 3×3 dolu kare.
- **ISO 4157-2:1998** (EN ISO 4157-2, yapı resimlerinde oda adları ve numaraları; TS karşılığı doğrulanamadı): oda numarası
  odanın içine yazılır. Mahal numarala'nın ikonu bunu gösterir.
- **ISO 129:1985 Şekil 5, ISO 2595:1973** (yapı resimleri): ölçü çizgisinin ucu ok yerine 45° eğik çizgi olabilir. Mimari çizimin
  alışkanlığı budur; İç ölçü zinciri de eğik çizgiyle çizilmişti. Duvar ölçüleri genel ölçünün okları yerine eğik çizgi alır.

## Karar

- **On yeni ikon** (`catalog/v1/additions.json` › `icons`):

  | Komut | İkon | Ne gösterir |
  |---|---|---|
  | Dikdörtgen duvar | `wallRectangle` | çift çizgili kapalı duvar, iki karşı köşe tıklanır |
  | Ölçülü bölme | `wallPartition` | odada bölme duvar ve duvarın yüzünden ona ölçü oku |
  | Toplu mahal bul | `spaceDiscover` | duvarların kapattığı üç oda, her birinde adı |
  | Mahal ayırıcı | `spaceSeparator` | iki oda arasındaki açıklığı kapatan kesikli çizgi, iki tık |
  | Mahali yeniden sınırla (M17'ye dek İç nokta seç) | `spaceRebound` | odanın içinde tık, çevresinde bulunan sınır kesikli |
  | Mahal kaynakları | `spaceSources` | seçilen duvarlar kalın, kolon dolu, sınırın kalanı kesikli |
  | Mahal numarala | `spaceNumber` | odanın içinde "01" |
  | Duvar ölçüleri | `wallDimensions` | açıklıklı duvarın üstünde eğik çizgili ölçü zinciri |
  | Açıklık dağıt | `openingArray` | duvarda eşit aralıklı üç açıklık, altında eşit aralık işareti |
  | Duvarı kaydır | `wallShift` | duvarın eski yeri kesikli, yenisi düz, aradaki dik ok |

- **Çizelgeler yeniden çizildi.** Solda listelenen şeyin resmi, sağda liste çizgileri:
  - Metraj Σ, Kapı ve pencere çizelgesi kapı ve kanat yayı, Mahal çizelgesi adlı oda;
  - Mahal kaplama çizelgesi boya rulosu, Donatı çizelgesi klozet planı, Bölüm çizelgesi iki bölümlü yapı;
  - Revizyon listesi revizyon üçgeni.
  - Malzeme lejandı zaten ayırt ediliyordu, değişmedi.
- **Ailenin yüzü değişmedi:** Duvar (`wall`), Mahal (`space`) ve Etiket (`tag`) kendi ikonlarını korur. Bölünmüş düğmenin yüzü
  ilk aracınkidir.
- **Panelin ikonu ilk komutununkidir** (DESIGN.md §6): Düzenle `wall` → `wallJoins` (Duvar birleşimi), Belgele `tag` →
  `wallDimensions`. Proje paneli `archSettings`'te kaldı: ilk öğesi Kat ▾, onun ok ikonu bir projeyi anlatmaz.
- **Bekçiler** (`apps/desktop/src/arch_icons.rs`):
  - `no_two_of_mimari_s_commands_share_an_icon`: Mimari sekmesinin komutlarından ikisi aynı ikonu taşımaz.
  - `no_two_of_mimari_s_panels_share_an_icon`: iki panel aynı ikonla katlanmaz.
  - `the_building_s_own_commands_borrow_no_other_tool_s_icon`: binanın kendi araçları (yalnız mimari sekmelerde duranlar)
    başka bir çizim aracının ikonunu taşımaz.
    - Seçim süzgecinin ikonu türünündür (Duvarları seç: duvar); ödünç sayılmaz.
    - Paftanın Detay paftaları penceresi Detay işaretinin ikonunu taşır (ADR 1112'nin kararı); bu da ödünç sayılmaz.
- **İkon tablosu** (`arch_icons::icon_table`, elle çalıştırılır): Mimari sekmesi ve beş bina bağlam sekmesinin 101 komutu.
  Her satırda yeri, ikonu 16, 20 ve şeridin büyük 28 px'inde, adı ve ikonun adı; açık ve koyu tema.
- **İkon turuna 15 Mimari sahnesi eklendi:** Duvar ▾, Boşluk ▾, Aks ▾, Merdiven ▾, Mahal ▾, Donatı ▾, Düzenle ▾, Ölçü ▾,
  Etiket ▾, Çizelge ▾, İşaret ▾, Belgele ▾, Kat ▾, Evre ▾, Proje ▾.

## Ölçüm

- **Önce:** üç ikonu paylaşan 10 komut, ödünç ikonlu 3 araç, aynı ikonla katlanan 2 panel. **Sonra:** 0, 0, 0 (üç bekçi geçer).
- **İkon tablosu:** önce `.run/gui-evidence/mimari-m16/once/`, sonra `sonra/` (açık ve koyu). Çizilen 17 ikonun önce → sonra
  büyütmesi: `m16-ikonlar-once-sonra.png`.
- **İkon turu:** 15 sahne, iki temada 30 resim (`.run/shots/ikon-turu/mimari-*.png`); özet `ikon-turu-mimari.png`.
- **Gerçek pencere** (`fixtures/interaction/real/mimari-ikonlar.txt`, 1440×900, sahibin ayarlarının kopyası): Mahal ▾, Düzenle ▾
  ve Çizelge ▾ fareyle açıldı, her satır kendi ikonuyla. Resim: `m16-gercek-menuler.png`.

## Sonraki adımlar

- Birbirine benzeyen öbür aileler (araştırma A1 §5): Mahal ile Mahali böl; Tek duvara çevir, Duvarı böl ve Duvarı uzat; Duvar
  birleşimi ile Birleşimleri denetle; Tipten ayır, Tipe uygula ve Tipe döndür; Döşeme deseni ve Tavan ızgarası. Paylaşılan ikon
  değiller, bekçiden geçerler. Yine de 16 px'te birbirine yakınlar.
- Yeni araçlar (Oda M21, Çatı, Kesit üret) aynı ızgarada çizilir ve bekçiden geçer.
