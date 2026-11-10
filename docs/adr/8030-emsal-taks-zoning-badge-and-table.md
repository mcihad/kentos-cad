# 3030 — Emsal ve TAKS: imar durumu, canlı rozet ve tablo

Tarih: 2026-11-08. Durum: kabul (M30, ADV-22; plan S22).

## Sorun

- **Projede imar durumu yoktu.** Parselin alanı, planın emsali (KAKS) ve TAKS'ı hiçbir yere yazılmıyordu. Mimar katlar
  alanını, emsal harici payı ve taban alanını elle topluyor, oranları hesap makinesiyle buluyordu.
- **Sınır çizerken görünmüyordu.** Bir mahal büyüdüğünde ya da bir oda otopark olduğunda emsalin nereye gittiği ancak
  proje bitince, elle yapılan hesapla anlaşılıyordu.
- **Onaylı projenin istediği tablo yoktu.** Planlı Alanlar İmar Yönetmeliği 57/3, parsel alanının, emsale konu alanın,
  yapı inşaat alanının, taban alanının, TAKS'ın ve KAKS'ın (emsal) ruhsat eki mimari projede gösterilmesini zorunlu
  tutar.
- **Plan M30'un kabulü:** "Canlı rozet ve tablo".

## Araştırma (8 Kasım 2026)

- **Planlı Alanlar İmar Yönetmeliği** (mevzuat.gov.tr, `MevzuatMetin/yonetmelik/7.5.23722.pdf`; kesitler
  `.run/gui-evidence/mimari-m30/m30-once-sonra.png` altında):
  - 4 (şş) Katlar alanı: "… kullanılabilen bütün katların, katlar alanına dâhil edilmeyen alanları çıktıktan sonraki
    alanlar toplamı". Madde 21 aynı tanımı verir.
  - 4 (tt) KAKS (Emsal): "Yapının inşa edilen tüm kat alanlarının toplamının imar parseli alanına oranı".
  - 4 (bbbb) Taban alanı: yapının tabii ya da tesviye edilmiş zemin üzerinde kalan kısmının parseldeki izdüşümü.
  - 4 (cccc) TAKS: taban alanının imar parseli alanına oranı.
  - 4 (oooo) Yapı inşaat alanı: ışıklıklar ve avlular hariç; balkonlar, teraslar, açık çıkmalar ve ortak alanlar dahil
    bütün katların alanı.
  - 5/8: "22 nci maddeyle … getirilebilecek emsal harici tüm alanların toplamı; parselin toplam emsale esas alanının
    % 30'unu aşamaz." Yangın merdiveni, korunumlu koridor ve yangın güvenlik holünün 6 m²'si bu %30'un dışındadır.
  - 22/1: katlar alanına dahil edilmeyen kullanımlar (otopark, sığınak, kazan dairesi, su deposu, jeneratör, trafo,
    asansör ve tesisat yerleri, kapıcı dairesi, çocuk oyun yeri, mescit, galeri ve atriumlar …).
  - 57/3: onaylı mimari projede gösterilmesi zorunlu değerler (yukarıda).
- **W4 §3** (`docs/research/2026-11-07-mimari/W4-mevzuat-standart.md`) aynı maddeleri ve imar tablosunun içeriğini verir.
- **KentOS'ta parçalar hazırdı:** mahalin üç ölçüsü (iç yüz, eksen, dış yüz; ADR 0342) ve M29'un yönetmelik brütü
  (dış cephede dış yüz, komşuyla eksen; ADR 8029); bina programının kullanım listesi (ADR 0784); her adımda kendini
  yeniden yazan tablolar (ADR 0327, 0344).

## Karar

- **İmar durumu** (`ArchSettings.zoning`: `parcelArea`, isteğe bağlı `far` ve `coverage`, ikisi de "1" birimli oran):
  Proje ayarları › Mimari › İmar durumu. Parsel 0'dan büyük ve en çok 100 km²; emsal (0, 50]; TAKS (0, 1].
- **Mahalin emsal sınıfı** (`EmsalCount`: Emsale dahil, Emsal harici (%30 içinde), Emsal harici (%30'a sayılmaz)):
  - Kullanım listesindeki değer (`SpaceBrief.emsal`); yoksa dış mahal (balkon, teras) %30 içinde;
  - yoksa adından ve kullanımından (`emsal_by_use`): yangın merdiveni, yangın güvenlik holü ve korunumlu koridor
    %30'un dışında; otopark, sığınak, mescit, galeri, atrium, kapıcı, tesisat, kazan, ısı merkezi, su deposu, jeneratör,
    trafo, enerji odası, kömürlük, hidrofor, çocuk oyun ve kütüphane %30 içinde; öbürleri emsale dahil.
- **Ölçü** (`Document::zoning_report`): her mahalin yönetmelik brütü (M29'un konturu, oda oda: dış yüz çizgisi eksi o
  kattaki öbür mahallerin eksen çizgisi; ortak duvar-cephe köşesinin örtüşmesi iki komşu arasında yarı yarıya).
  - Katlar alanı = emsale dahil mahaller + %30'u aşan emsal harici. Sınır = 0,30 × parsel × emsal (5/8).
  - Taban alanı = zemin katın (adında "Zemin" geçen; yoksa kotu ±0,00'a en yakın) emsale sayılan ve dış olmayan
    mahalleri; katsız projede bütün mahaller.
  - Yapı inşaat alanı = bütün mahaller (boşluklar hariç). Kat kat: emsale dahil, harici ve toplam.
- **Canlı rozet** (`zoning_badge`, durum çubuğu): imar durumu varken.
  - Yer varken "Emsal 0,30/1,50 · TAKS 0,45/0,40"; daralınca sınırsız; lambalar sıkışınca yalnız ikon (Kat hücresi
    gibi). Sınır aşılınca kırmızı; yalnız ikonda kırmızı "!".
  - İpucu bütün sayıları verir; tıklanınca Proje ayarları › Mimari açılır.
  - Rapor çizimin her sürümünde bir kez hesaplanır, her karede değil.
- **Öznitelikler › İmar durumu** (seçim yokken): Parsel alanı, Emsal "0,30 / 1,50", TAKS "0,45 / 0,40 · aşıldı".
  1440 genişlikte rozet ikona indiğinde sayılar burada görünür.
- **Emsal ve TAKS tablosu** (`arch.zoningTable`; Mimari › Belgele › Metraj ▾; EMSAL, TAKS, KAKS, IMARTABLOSU):
  - Yeni tablo kaynağı `TableSource::Zoning {}` (KCAD 222). Hücreleri belge yazar: her adımda (mahal değişince; duvar
    taşınınca mahaller aynı adımda izler), program ve kat değişince, Proje ayarları kaydedilince ve geri alma, yineleme
    ve geri sarmadan sonra (görünüm işaretleri ve etiketler gibi, ayrı geri alma adımı olmadan).
  - Satırlar 57/3'ün sırasıyla: Parsel alanı; Emsal (KAKS); Emsale konu alan; Emsal harici, %30 içinde; (varsa) %30'u
    aşan; (varsa) %30 dışında; TAKS; Taban alanı; Yapı inşaat alanı. Sütunlar İmar durumu, Proje, Kalan.
  - Altında kat kat: Kat, Emsale dahil, Emsal harici, Kat alanı, Toplam.
  - Sınırı aşan Proje ve Kalan hücresi kırmızı (`OVER_COLOR`); kalan eksi işaretli.
  - Alanlar projenin alan birimi ve basamağıyla, oranlar iki basamakla. İmar durumunun vermediği "—".
  - Hücreler elle düzenlenmez (çizelge gibi); dışa aktarma satırları yeniden okur.
  - İmar durumu yokken araç nereye yazılacağını söyler ve Proje ayarları › Mimari'yi açar.
- **Yeni ikon `zoningTable`:** kesikli parsel sınırı, içinde bina izi, yanında tablo satırları. Rozet de bu ikonu taşır.
- **KCAD şema 222:** imar durumu, kullanım listesinde emsal sınıfı ya da Emsal ve TAKS tablosu şema 222 ister; eski
  sürüm `UnknownField` ile reddeder. Python okuyucusu aynı kodu verir.
- KentOS bir onay vermez: değerler çizimin ölçüsüdür; planın kendi notları ayrıca okunur.

## Ölçüm

- **Test** (`paiy_area_tests::emsal_and_taks_read_the_rooms_against_the_parcel`): 15 × 6 m zemin kat, orta oda Otopark.
  - 300 m², emsal 1,50, TAKS 0,40: emsale dahil 60, harici 30 (sınır 135), emsal 0,20; taban 90, TAKS 0,30.
  - 80 m², emsal 1,00: sınır 24, aşan 6 emsale eklenir, katlar alanı 66, emsal 0,825; TAKS 1,125 aşılmış.
- **Test** (`paiy_area_tests::an_emsal_and_taks_table_follows_the_zoning_and_the_rooms`):
  - İmar durumu yokken araç uyarır ve Proje ayarlarını açar.
  - Tablo imleçten yerleşir (`tablePlace`): 450 / 60 / 390; 135 / 30 / 105; TAKS 0,40 / 0,30; taban 120 / 90 / 30;
    yapı inşaat alanı 90; Zemin kat 60 / 30 / 90.
  - Parsel 200 m²: TAKS 0,45 ve taban 90 / −10 kırmızı.
  - x = 5'teki duvar 1 m doğuya: Zemin kat 66 / 24 / 90 aynı adımda. Tek geri alma duvarı ve tabloyu geri getirir.
- **Gerçek pencere** (`fixtures/interaction/real/emsal-taks.txt`, 1440 × 900; M29'un binası):
  - İmar durumu yazıldı (300; 1,5; 0,4), kaydedildi: Öznitelikler › İmar durumu 300,00 m², 0,30 / 1,50, 0,30 / 0,40.
  - Metraj ▾'de yeni satır ve ikon; arama "emsal ve taks" ile tablo imleçten yerleşti.
  - Parsel 200'e inince tablo kendiliğinden yenilendi (TAKS ve taban kırmızı), rozette kırmızı "!".
- **Gerçek pencere** (`fixtures/interaction/real/emsal-taks-genis.txt`, 1920 × 1080; testin sakladığı `imar.kcad`):
  rozet "Emsal 0,30/1,50 · TAKS 0,45/0,40" kırmızı; Öznitelikler "0,45 / 0,40 · aşıldı".
- Resim: `.run/gui-evidence/mimari-m30/m30-once-sonra.png`.
- **KCAD:** `zoning.kcad` (imar durumu ve tablo); kırık `zoning-in-schema-221`, `zoning-no-parcel` ve
  `zoning-table-in-schema-221` (imar durumu olmadan da tablo şema 222 ister). 241 geçerli dosya; 1701 dosya Python
  okuyucusuyla tutarlı.

## Sonraki adımlar

- Kullanım listesinde emsal sınıfını seçtiren arayüz (bugün komut ve dosya; varsayılan ad ve kullanımdan).
- Mahal başına emsal sınıfı (aynı kullanımın bir odası dahil, öbürü harici).
- 22. maddenin öbür sınırları: açık balkon ve çıkmaların katlar alanının %20'si, depoların bağımsız bölüm net alanının
  %50'si, kapıcı dairesinin 70 m²'si.
- Emsal verilmeyen parselde katlar alanı = TAKS × kat adedi (21/4); KAKS verilip TAKS verilmeyende TAKS ≤ %60 (5/6).
- Birden çok bina (57/3: her binanın ve hepsinin emsale konu alanı), bodrum katların taban alanına katılması (4/cccc).
- Vaziyet planı (M32): parsel çizimi ve bahçe mesafeleri.
