# 3039 — Tadilat grafik dili: tadilat durumu, açıklığın tadilat durumu, görünüme göre kesim

Tarih: 2026-11-09. Durum: kabul (M39; ADV-17).
- 1. bölüm: açıklığın kendi tadilat durumu; tadilat görünümüne göre duvar birleşimi ve kesimi.
- §0: kavramın adı "Evre"den "Tadilat durumu"na geçti.
- 2. bölüm ayrı eklerle gelecek: araçlarda tadilat durumu, tadilat projesi görünümü ve lejantı, tadilat takımı, DXF
  katmanları.

## Sorun

- **Kapı ve pencerenin kendi tadilat durumu yoktu.** Tadilat durumu duvarda, mahalde, kolonda, merdivende, rampada ve
  blokta vardı (ADR 0346). Açıklık onu duvarından okuyordu (`Entity::phase`). Tadilatın en sık iki işi bu yüzden
  çizilemiyordu: **mevcut duvara açılan yeni kapı** ve **kapatılan mevcut pencere**.
- **Gizlenen nesne birleşmeye ve kesmeye devam ediyordu.** Görünüm nesneyi yalnız gizliyordu (`set_phase_hidden`);
  duvar birleşimi ve açıklık kesimi görünümden habersizdi.
  - Mevcut durum planında yeni kapının boşluğu duvarda delik olarak kalırdı.
  - Yeni durum planında yıkılan duvarın birleştiği duvarların yüzünde boşluk kalıyordu (ölçüldü: iki yatak odası
    arasındaki duvar iptal edilince alttaki ve üstteki duvarın yüzü T birleşiminin yerinde kesikti).
- **Paftadaki harita da aynı.** Bir harita kendi görünümünü gösterebiliyordu (ADR 0774 §3), ama kesim çizimin
  görünümüne göre yapılıyordu.
- **"Evre" sözcüğü yerinde değildi** (sahibin bildirimi, 9 Kasım: "Evre mantıksız bir kavram, daha düzgün bir kavram
  kullan"). Revit'in "Phase"inin çevirisiydi; Türkiye'deki tadilat işinin dili değil (§0).

## Araştırma (9 Kasım 2026)

### Türkiye mevzuatı ve meslek kuralı

- **Planlı Alanlar İmar Yönetmeliği (RG 3.7.2017/30113; mevzuat.gov.tr 7.5.23722, güncel metin):**
  - 4(1)(y) **esaslı tadilat:** taşıyıcı unsuru etkileyen ya da inşaat alanını, emsale konu alanı, taban alanını,
    bağımsız bölüm sayısını, ortak alanları, bağımsız bölüm alanını, kullanım amacını ya da **ruhsat eki projeleri**
    değiştiren işler; ruhsata tabidir.
  - 4(1)(dddd) **tadilat projesi:** ruhsat eki onaylı projelerde istenen değişiklik ve ilavelerle ilgili uygulama
    projelerinin bütünü.
  - 4(1)(i) **basit tamir ve tadil:** esaslı tadilat olmayan, taşıyıcı sistemi, bağımsız bölümün **dış cephesini**,
    ıslak hacimlerin yerini ve sayısını değiştirmeyen işler (doğrama tamiri dahil).
  - Sonuç: dış duvardaki pencereyi kapatmak cepheyi değiştirir, basit tamir değildir; tadilat projesi gerekir.
  - 58(2)–(4): ruhsatlı yapıda değişiklikte mimari proje yeniden düzenlenir; tadilat projeleri idarece mühürlenir; ilave
    ve tadilat projeleri 5 takım, 57. maddeye göre onaylanır.
- **Mimarlar Odası, "Mevzuat — Çizim tekniği ile ilgili sorular ve cevaplar":**
  - "İlave tadilat projelerinin çizim tekniği nasıl olmalı? Projede mevcut, iptal ve ilaveler üç farklı teknikle ifade
    edilmelidir."
  - Örnek tadilat lejantı: **mevcut** düz kontur, **ilave** eğik tarama, **iptal** kesikli kontur.
  - "Koçanda kayıtlı olan kısım mevcut tekniğinde, kayıtlı olmayan rölevesi yapılacak olan kısım yeni proje tekniğinde
    çizilmelidir."

### Başka uygulamalar (yalnız görünüş için, kural değil)

- Revit: kapının kendi "Phase Created/Demolished"ı vardır. Yıkılan kapının yeri duvarla doldurulur (infill). Görünümün
  süzgeci gizlediği öğeyi birleşimden de çıkarır.
- Archicad: Renovation Status (Existing / To Be Demolished / New) ve süzgeç.

## Karar

### 0. Kavram: Tadilat durumu (eski adı "Evre")

Mimarlar Odası'nın üç sözcüğü kullanılır: **mevcut, iptal, ilave**. "İptal" duvarda da (kaldırılır) açıklıkta da
(boşluğu kapatılır) doğrudur, açıklığa ayrı sözcük gerekmez. Dosyalardaki ve komutlardaki adlar değişmedi (`phase`:
`existing`, `demolish`, `new`, `temporary`; `phaseView`); yalnız kullanıcının gördüğü sözcükler değişti:

| Eski | Yeni |
|---|---|
| Evre (Öznitelikler, mahal ayırıcı, oda modülü, çizelge sütunu ve süzgeci) | Tadilat durumu |
| Mevcut, Yıkılacak, Yeni, Geçici | Mevcut, **İptal**, **İlave**, Geçici |
| Şeritte Evre ▾; Evre: Tümü / Mevcut durum / Yıkım planı / Yeni durum | **Tadilat ▾**; Tadilat: Tümü / Mevcut durum / Yıkım planı / Yeni durum |
| Durum satırında "Evre: Yeni durum." | "Tadilat görünümü: Yeni durum." |
| Görünüş tarifinin ve pafta tarifinin evresi | Tadilat görünümü |
| Komut iletileri ("kat, evre, …", "kat, evre, durum ya da disiplin") | "kat, tadilat durumu, …", "kat, tadilat durumu, yapım durumu ya da disiplin" |
| Ruhsat takımı penceresinde "Evre ve planların ölçeği" (ön proje, ruhsat, uygulama) | **Proje aşaması** ve planların ölçeği |
| Dalga çizgisi stilinde "Evre" (Çizgi başından) | **Başlangıç** |

Görünüm adları (Tümü, Mevcut durum, Yıkım planı, Yeni durum) yerleşik Türkçe olduğu için kaldı. Desen kesiklerinin
"evresi" (taramada kesiğin kayması) başka bir kavramdır, dokunulmadı.

### 1. Açıklığın kendi tadilat durumu

- `OpeningFields.phase` (isteğe bağlı): yoksa duvarınki. `Entity::phase` önce açıklığın kendisininkini okur; açıklığın
  etiketi de onu. Çizelgenin süzgeci (`schedule_takes`) ve "Tadilat durumu" sütunu (`arch_value("phase")`) da.
- `cad.openings.edit` `set` `phase`; `null`: duvarınki.
- Öznitelikler'de açıklığın **Tadilat durumu** satırı: "Duvarınki (Mevcut)", Mevcut, İptal, İlave, Geçici. Satır ana
  satırlar arasındadır (Numara, Tür, Genişlik, Tadilat durumu).
- `.kcad` şema 218: açıklığın `phase`'i. Yoksa dosya eski şemasıyla bayt bayt aynı; şema 217 dosyasında `unknown_field`.
  Bağımsız Python okuyucusu ve yazıcısı aynı kuralla.

### 2. Tadilat görünümüne göre birleşim ve kesim

Görünümün gizlediği duvar başka duvarla birleşmez, gizlediği açıklık duvarı kesmez. Bu, tasarım seçeneklerinin
kuralının aynısı (ADR 0788 §4) ve aynı yoldan: `Store::set_option_out`.

| Görünüm | Mevcut açıklık | İptal açıklık | İlave açıklık |
|---|---|---|---|
| Mevcut durum | keser | keser | gizli, kesmez (duvar bütün) |
| Yıkım planı | keser | kesikli, keser | gizli, kesmez |
| Yeni durum | soluk, keser | gizli, kesmez (duvar dolu) | keser |

- **Ekranda** `Spatial::sync_phases` görünümün gizlediklerini bir kez bulur; duvar ve açıklıkları seçeneklerle birlikte
  dışarıda bırakır.
- **Paftada** (PDF ve ekrandaki pafta) kendi tadilat görünümünü gösteren harita, görünümü çizimden farklıysa kendi
  kopyasından çizilir: `Document::with_phase_view`. Kopyanın anahtarı görünümün adını taşır. Aynı görünümdeki ya da
  görünümsüz harita çizimin kendisinden çizilir.
- `viewport_scope::phase_out`: kapsamlı pafta deposu da aynı kuralla.

## Doğrulama

- Etkileşim (`crates/native/interaction/tests/all/phase_cuts.rs`): 6 m mevcut duvar, 1,5 m'de ilave 90'lık kapı, 4 m'de
  iptal 120'lik pencere. Duvar yüzündeki boşluklar santimetreyle:
  - Tümü: kapı (1,05–1,95) ve pencere (3,40–4,60).
  - Mevcut durum: yalnız pencere.
  - Yıkım planı: yalnız pencere.
  - Yeni durum: yalnız kapı.
  - Kapının kendi tadilat durumu kaldırılınca duvarınkini (Mevcut) alır ve Mevcut durumda da keser.
  - İptal köşe duvarı Yeni durumda birleşmez: komşu duvarın dış yüzü 6,15 → 6,00 m'de biter.
- Pafta (`book.rs` `phase_tests`, `sheets.rs` `a_map_with_its_own_phase_draws_a_copy_cut_as_it_stands`): çizim Mevcut
  durumdayken Yeni durum haritası kapıyı keser, pencereyi kapatır. Aynı görünümdeki ya da görünümsüz harita kopya
  açmaz; çizim değişmez.
- Öznitelikler (`an_openings_renovation_row_writes_its_own_phase`): satır, sözcükler, tek adım, Duvarınki ile geri alma.
- Ortak komut durumları: `cad.openings.edit`'e iki yeni durum (kendi tadilat durumu ilave; iptal, sonra kaldırılır);
  bağımsız başvuru `scripts/fixtures/opening_command_cases.py`. §0'ın sözcükleri 19 başvurunun durum adlarında ve
  iletilerinde; 27 durum dosyası yalnız bu sözcüklerle değişti. Beş dosyanın üreticisi bugünkü dosyayı yeniden
  üretmiyordu: başka işlerden kalan bir sapma. O dosyalarda yalnız sözcükler değiştirildi.
- KCAD 218: `opening-phases` örneği ve şema 217'de bozuk dosyası; çok yeni şema örneği 219'a kaydı (geçerli örnek 249).
- Gerçek pencere (1920×1080, rölöve daire: bütün duvarlar Mevcut; iz `fixtures/interaction/real/tadilat-durumu.txt`):
  - İki yatak odası arasındaki duvar İptal yapıldı.
  - Salon ile yatak odası 1 arasına kapı kondu ve İlave yapıldı.
  - Salonun batı penceresi İptal yapıldı.
  - Mevcut durumda yeni kapı yok ve duvar bütün.
  - Yıkım planında duvar kesikli, kapı yok.
  - Yeni durumda kapı var, pencere kapalı, iptal duvarın birleşim yerlerinde komşu yüzler kesintisiz.
  - ÖNCE (M38 ikilisi) açıklığın bu satırı yoktu ve yeni kapı Mevcut durumda da görünüyordu.
- Testler: 9 paket 4222 geçti; masaüstü 2033 geçti (tek kırmızı işten önce de kırmızı olan
  `module_surface_tests::each_program_docks_its_own_panels`).

## Sınırlar

- Metraj tadilat durumuna göre ayrılmıyor: mevcut duvarın net yüzü bütün açıklıklarını düşüyor (ilave kapının kırım
  metrajı yok).
- Mahal görünüme göre yeniden bulunmuyor: Yeni durumda birleşen iki odanın sınırları, mahaller yeniden çizilene ya da
  tadilat durumu verilene dek eski yerinde.
- Parçasal iptal (duvarın bir kesimini kaldırmak) duvarı bölmekle yapılır (Duvarı böl); ayrı bir araç yok.
