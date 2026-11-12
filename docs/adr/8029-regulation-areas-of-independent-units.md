# 8029 — Bağımsız bölümün yönetmelik alanları (PAİY Madde 4)

Tarih: 2026-11-08. Durum: kabul (M29, ADV-22; plan S22).

## Sorun

- **Brüt yanlış adlanıyordu.** Bölüm çizelgesindeki (ADR 0795) "Brüt", bölümün odalarının toplamıyla ortak alan payının
  toplamıydı (ADR 0784 §2). Yönetmeliğin brüt alanı bambaşka bir şeydir: bölümün dış konturunun çevrelediği alan.
  Mimarın imza atacağı bir çizelgede yanlış terim, yanlış bir alanı resmî alan gibi gösteriyordu.
- **Yönetmeliğin alanları yoktu.** Planlı Alanlar İmar Yönetmeliği 4 (ğ)'nin BB brüt, genel brüt ve net alanı
  hesaplanmıyordu. Ruhsat paftasındaki metrekare cetveli ve kat irtifakı bu alanlara dayanır (PAİY 57/2-a).
- **ADV-22:** "Türk alan motoru (PAİY Md. 4): bağımsız bölümün net, brüt, genel brüt ve toplam brüt alanı … kontur
  kuralları." M29'un kabulü: "PAİY 4 alanları çizelgede".

## Araştırma (8 Kasım 2026)

- **Planlı Alanlar İmar Yönetmeliği, Madde 4 (ğ) 1–4** (mevzuat.gov.tr, `MevzuatMetin/yonetmelik/7.5.23722.pdf`, s. 1–2;
  `referans/paiy-4g1-brut.png`, `referans/paiy-4g2-4-genel-net-toplam.png`):
  - 1) Brüt: "Şaftlar, ışıklıklar, hava bacaları, galeri boşlukları hariç, bağımsız bölümün dış konturlarının
    çevrelediği alan (Dış cephe haricindeki bölümlerdeki dış kontur, duvar orta aksı olarak belirlenir …)".
  - 2) Genel brüt: "Bağımsız bölüm toplam brüt alanına … ortak alanlardan düşen payların da eklenmesi".
  - 3) Net: "… duvarlar arasında kalan temiz alanı (Bu alana; kapı ve pencere eşikleri, 2.5 santimetreyi geçmemek
    koşuluyla sıva payları, kolonlar, … tesisat ve hava bacaları ile ışıklıklar, … asansör ve galeri boşlukları, …
    merdivenlerin altlarında 1.80 metre yüksekliğinden az olan yerler … dâhil edilmez. Açık çıkmalar, balkonlar, …
    teraslar … ile ortak alanlar bağımsız bölüm net alanı içinde değerlendirilmez …)".
  - 4) Toplam brüt: brüt ve bölümün eklentilerinin brüt alanı.
  - Birden çok katta yer alan bölümün alanları birlikte değerlendirilir.
- **Araştırma notları** (W4 §2, `docs/research/2026-11-07-mimari/W4-mevzuat-standart.md`) aynı tanımları ve metrekare
  cetvelinin içeriğini (PAİY 57/2-a) veriyor.
- **KentOS'ta parçalar hazırdı:**
  - Mahalin üç ölçüsü: iç yüz (net), duvar ekseni ve dış yüz (ADR 0342 §1); tohumdan yeniden bulunur.
  - Bina programının grupları (bağımsız bölüm, ortak alan, dış alan) ve tam kesirli pay kuralı (`kentos.share/1`).
  - Bölüm çizelgesi.
- **KentOS duvar stillerinde sıva duvarın içindedir** (Dış duvar 25'te iç sıva 2 cm, İç duvar 15'te iki yüzde 2 cm).
  Mahalin iç yüzü sıvalı yüzdür. Yönetmelik sıvayı (en çok 2,5 cm) net alanın dışında tutar; bu yüzden ayrıca düşülmez.

## Karar

- **`Document::paiy_areas`** (`kentos_domain::paiy_areas`): bina programının her grubu için, katlarıyla birlikte:
  - **BB brüt alanı:** grubun mahallerinin dış yüz çizgileri birleştirilir, o kattaki öbür bütün mahallerin (öbür
    grupların, atanmamışların, boşlukların) duvar ekseni çizgileri çıkarılır.
    - İki odası arasındaki duvar bölümündür; paylaştığı duvar ekseninden kesilir; dış cephede dış yüze kadar gider.
    - Şaft ve galeri boşluk mahalidir; eksenine kadar çıkar.
    - Ortak duvarın dış duvara değdiği köşede iki komşu da dış duvarın dış yarısına uzanır. Bu örtüşme ikisi arasında
      yarı yarıya bölünür. Eksen duvarın ortası olduğundan bölme kesindir; ilk ölçümde iki komşu da 30,02 m²
      sayıyordu.
  - **BB net alanı:** grubun mahallerinin iç yüz alanları; boşluk ve dış mahaller (balkon, teras) dışarıda. Kolonlar
    mahalin kendi kolon kuralıyla. Sıva payı duvar stilinin içinde.
  - **Ortak alan payı:** ortak alanların brüt alanları programın kuralıyla paylaştırılır (`kentos.share/1`; eşit,
    ağırlık; "net" kuralında ağırlık bölümün brüt alanıdır).
  - **BB genel brüt alanı:** brüt ve pay. Yönetmelik toplam brütü (eklentiler dahil) alır; eklenti modeli M31'dedir,
    o güne dek toplam brüt brüttür.
  - Tohumsuz (çizgiyle çizilmiş) mahal başka çizgiye ölçülemez: üç ölçüde de kendi halkaları kullanılır.
- **Bölüm çizelgesinin dört yeni sütunu** (`ScheduleField::PaiyNet`, `PaiyGross`, `PaiyShare`, `GeneralGross`): "BB net
  alanı", "BB brüt alanı", "Ortak alan payı", "BB genel brüt alanı". Yalnız bölüm çizelgesinde; toplamda toplanır; pay
  ve genel brüt yalnız bağımsız bölümde. Başlıkta alan birimi (m²).
- **Eski "Brüt" başlığı "Net + pay" oldu.** Değeri değişmedi (odalar ve ADR 0784'ün payı). Ortak örnekler iki yerde
  yeni başlıkla yazıldı (`unit_schedule_cases.py`, `program_share_cases.py`).
- **Çizelge aracının bölüm hazırı:** No, Ad, Bölüm türü, Kat, BB net, BB brüt, Ortak alan payı, BB genel brüt.
  Gruplu hâli: Bölüm türü, Bina, Mahal sayısı ve dört alan. Komutun (Python, ajan) varsayılan sütunları değişmedi.
- **Metrekare cetveli** (PAİY 57/2-a: "bağımsız bölümler ile ortak alanların brüt inşaat alanı … toplam yapı inşaat
  alanı"):
  - Bölüm çizelgesine **kat kat** satırlar geldi (`byLevel`; `cad.schedules.create`, `cad.schedules.edit`'in `set`'i ve
    çizelgenin kaynağı).
    - Projenin her katı, sırasıyla; her katta odası olan gruplar programın sırasıyla.
    - Satırın katı kendisinindir; mahalleri o kattakilerdir. Yönetmelik brüt ve net alanı o katın payıdır
      (`PaiyGroup::levels`).
  - Birden çok kattaki ortak alan, örneğin merdiven evi, her katta ayrı satırdır. Bölümün bütününe ait değerler (pay,
    genel brüt, kayıtlı alan, hedef) kat satırında boştur; onlar bölüm çizelgesindedir.
  - Çizelge aracının yeni hazırı **Metrekare cetveli** (`tool.scheduleAreas`; Mimari › Belgele › Metraj ▾; METREKARE,
    ALANCETVELI): Kat, No, Ad, Bölüm türü, BB brüt alanı, BB net alanı ve toplam. Toplam binanın brüt alanıdır; bütün
    mahaller bir gruptaysa yapı inşaat alanına eşittir.
  - Yeni ikon `scheduleAreas`: üç katlı bir blok, tablo satırları ve kalın bir toplam çizgisi.
  - **Metraj ▾:** Metraj düğmesi Metraj ile Metrekare cetvelini tutan bir aileye dönüştü. Çizelge ▾ altı satırla doluydu
    (DESIGN.md §7.3.1, `mimari_s_tab_shows_at_most_thirty_buttons_and_its_main_tools_large` yedinciyi reddetti). Alan
    cetveli metrajın yanında aranır.
  - Kat ara toplamı yok: çizelgenin ara toplamı ilk sütunun sıralanmasını ister, kat adının yazı sırası ("1. kat" <
    "Zemin kat") ise katların sırası değildir.
- **KCAD şema 221:** Bu sütunlardan birini taşıyan ya da kat kat (`byLevel`) bir çizelge şema 221 ister; eski sürüm
  `UnknownField` ile reddeder.
  Python okuyucusu da sonraki şemanın sütununu "bilinmeyen alan" sayar (Rust'la aynı kod); 220'nin Kod sütunu da böyle.
- Bir yönetmeliğe uyulduğu söylenmez; çizelge ölçülen alanları yazar.

## Ölçüm

- **Test** (`paiy_area_tests::a_units_schedule_writes_the_regulation_s_net_gross_share_and_general_gross`): 15 × 6 m
  bina, dış duvar 25 cm, x = 5 ve 10'da 15 cm duvar; batı ve doğu odaları bağımsız bölüm, orta merdiven evi ortak alan;
  eşit pay.
  - Başlıklar "BB net alanı (m²)" … "BB genel brüt alanı (m²)".
  - Bölümler: net 25.71 (= 4.675 × 5.5), brüt 30.00 (= 5 × 6), pay 15.00, genel brüt 45.00.
  - Ortak alan: net 26.68 (= 4.85 × 5.5), brüt 30.00.
- **Gerçek pencere** (`fixtures/interaction/real/yonetmelik-alanlari.txt`; testin sakladığı bina, 1440×900):
  - Komut ara "bölüm çizelgesi", önizleme, tık.
  - Toplam satırı: net 78.10, brüt 90.00, pay 30.00, genel brüt 90.00. Genel brüt toplamı 15 × 6'ya eşit; alanın hepsi
    paylaştırılmış.
  - Resim: `.run/gui-evidence/mimari-m29/m29-once-sonra.png`.
- **Test** (`paiy_area_tests::a_metrekare_cetveli_lists_each_storey_s_units_and_common_areas`): iki katlı aynı plan;
  merdiven evi iki katta tek ortak alan. Kat kat cetvel:
  - Zemin kat 1, 2, O; 1. kat 3, 4, O.
  - Her biri 30.00; toplam 180.00.
- **Gerçek pencere** (`fixtures/interaction/real/metrekare-cetveli.txt`; testin sakladığı iki katlı bina):
  - Metraj ▾'de yeni satır ve ikon.
  - Cetvel: Kat, No, Ad, Bölüm türü, BB brüt, BB net; altı satır; toplam 180.00 brüt, 156.20 net.
- **KCAD:** `paiy-areas.kcad` (yönetmelik sütunları ve kat kat), kırık `paiy-areas-in-schema-220`. 240 geçerli dosya.

## Sonraki adımlar

- Metrekare cetvelinde kat ara toplamları (katın sırasıyla) ve gruba alınmamış mahallerin yapı inşaat alanına
  katılması (PAİY 4/oooo).
- Merdiven altında 1,80 m'den alçak yerin net alandan düşülmesi (merdivenin kesitinden).
- Sıvasız duvar stilli projelerde 2,5 cm sıva payının düşülmesi (proje seçeneği).
- Eklentiler ve toplam brüt (M31); emsal ve TAKS (M30).
