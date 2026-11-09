# 3039 — Tadilat grafik dili: açıklığın evresi, evreye göre kesim

Tarih: 2026-11-09. Durum: kabul (M39; ADV-17). 1. bölüm: açıklığın kendi evresi ve evre görünümüne göre duvar birleşimi
ve kesimi. 2. bölüm (araçlarda evre, tadilat görünümü ve lejantı, tadilat takımı, DXF evre katmanları) ayrı eklerle.

## Sorun

- **Kapı ve pencerenin kendi evresi yoktu.** Evre (Mevcut, Yıkılacak, Yeni, Geçici) duvarda, mahalde, kolonda,
  merdivende, rampada ve blokta vardı (ADR 0346); açıklık evresini duvarından okuyordu (`Entity::phase`). Tadilatın en sık
  iki işi bu yüzden çizilemiyordu: **mevcut duvara açılan yeni kapı** ve **kapatılan mevcut pencere**.
- **Gizlenen nesne birleşmeye ve kesmeye devam ediyordu.** Evre görünümü nesneyi yalnız gizliyordu (`set_phase_hidden`);
  duvar birleşimi ve açıklık kesimi görünümden habersizdi. Mevcut durum planında yeni kapının boşluğu duvarda delik
  olarak kalırdı; Yeni durum planında yıkılan duvarın birleştiği duvarların yüzünde boşluk kalıyordu (ölçüldü: iki yatak
  odası arasındaki duvar yıkılınca alttaki ve üstteki duvarın yüzü T birleşiminin yerinde kesik).
- **Paftadaki harita da aynı.** Bir harita kendi evresini gösterebiliyordu (ADR 0774 §3), ama kesim çizimin evresine göre
  yapılıyordu.

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

### 1. Açıklığın kendi evresi

- `OpeningFields.phase` (isteğe bağlı): yoksa duvarınki. `Entity::phase` önce açıklığın kendisininkini okur; açıklığın
  etiketi de onu. Çizelgenin evre süzgeci (`schedule_takes`) ve "Evre" sütunu (`arch_value("phase")`) da.
- `cad.openings.edit` `set` `phase`; `null`: duvarınki.
- Öznitelikler'de açıklığın **Evre** satırı: "Duvarınki (Mevcut)", Mevcut, Kapatılacak, Yeni, Geçici. Satır ana satırlar
  arasındadır (Numara, Tür, Genişlik, Evre).
- **Sözcük:** açıklığın kendi "Yıkılacak"ı **"Kapatılacak"** yazılır (`ArchPhase::opening_label`). Duvarı kalan bir
  pencere yıkılmaz, boşluğu kapatılır (Mimarlar Odası'nın "iptal"i). Duvarından aldığı evre duvarın sözcüğüyle kalır:
  yıkılan duvarın kapısı duvarla gider.
- `.kcad` şema 218: açıklığın `phase`'i. Yoksa dosya eski şemasıyla bayt bayt aynı; şema 217 dosyasında `unknown_field`.
  Bağımsız Python okuyucusu ve yazıcısı aynı kuralla.

### 2. Evre görünümüne göre birleşim ve kesim

Görünümün gizlediği duvar başka duvarla birleşmez, gizlediği açıklık duvarı kesmez. Bu, tasarım seçeneklerinin
kuralının aynısı (ADR 0788 §4) ve aynı yoldan: `Store::set_option_out`.

| Görünüm | Mevcut açıklık | Kapatılacak açıklık | Yeni açıklık |
|---|---|---|---|
| Mevcut durum | keser | keser | gizli, kesmez (duvar bütün) |
| Yıkım planı | keser | kesikli, keser | gizli, kesmez |
| Yeni durum | soluk, keser | gizli, kesmez (duvar dolu) | keser |

- **Ekranda** `Spatial::sync_phases` görünümün gizlediklerini bir kez bulur; duvar ve açıklıkları seçeneklerle birlikte
  dışarıda bırakır.
- **Paftada** (PDF ve ekrandaki pafta) kendi evresini gösteren harita, evresi çizimden farklıysa kendi kopyasından
  çizilir: `Document::with_phase_view`. Kopyanın anahtarı evrenin adını taşır. Aynı evredeki ya da evresiz harita
  çizimin kendisinden çizilir.
- `viewport_scope::phase_out`: kapsamlı pafta deposu da aynı kuralla.

## Doğrulama

- Etkileşim (`crates/native/interaction/tests/all/phase_cuts.rs`): 6 m mevcut duvar, 1,5 m'de yeni 90'lık kapı, 4 m'de
  kapatılacak 120'lik pencere. Duvar yüzündeki boşluklar santimetreyle:
  - Tümü: kapı (1,05–1,95) ve pencere (3,40–4,60).
  - Mevcut durum: yalnız pencere.
  - Yıkım planı: yalnız pencere.
  - Yeni durum: yalnız kapı.
  - Kapının evresi kaldırılınca duvarınkini (Mevcut) alır ve Mevcut durumda da keser.
  - Yıkılan köşe duvarı Yeni durumda birleşmez: komşu duvarın dış yüzü 6,15 → 6,00 m'de biter.
- Pafta (`book.rs` `phase_tests`, `sheets.rs` `a_map_with_its_own_phase_draws_a_copy_cut_as_it_stands`): çizim Mevcut
  durumdayken Yeni durum haritası kapıyı keser, pencereyi kapatır. Aynı evredeki ya da evresiz harita kopya açmaz;
  çizim değişmez.
- Öznitelikler (`an_openings_evre_row_writes_its_own_phase`): satır, sözcükler, tek adım, Duvarınki ile geri alma.
- Ortak komut durumları: `cad.openings.edit`'e iki yeni durum (kendi evresi yeni; kapatılacak, sonra kaldırılır); bağımsız
  başvuru `scripts/fixtures/opening_command_cases.py`.
- KCAD 218: `opening-phases` örneği ve şema 217'de bozuk dosyası; çok yeni şema örneği 219'a kaydı (geçerli örnek 249).
- Gerçek pencere (1920×1080, rölöve daire: bütün duvarlar Mevcut; iz `fixtures/interaction/real/tadilat-evre.txt`):
  - İki yatak odası arasındaki duvar Yıkılacak yapıldı.
  - Salon ile yatak odası 1 arasına kapı kondu ve Evre'si Yeni yapıldı.
  - Salonun batı penceresi Kapatılacak yapıldı.
  - Mevcut durumda yeni kapı yok ve duvar bütün.
  - Yıkım planında duvar kesikli, kapı yok.
  - Yeni durumda kapı var, pencere kapalı, yıkılan duvarın birleşim yerlerinde komşu yüzler kesintisiz.
  - ÖNCE (M38 ikilisi) açıklığın Evre satırı yoktu ve yeni kapı Mevcut durumda da görünüyordu.
- Testler: 9 paket 4222 geçti; masaüstü 2033 geçti (tek kırmızı işten önce de kırmızı olan
  `module_surface_tests::each_program_docks_its_own_panels`).

## Sınırlar

- Metraj evreye göre ayrılmıyor: mevcut duvarın net yüzü bütün açıklıklarını düşüyor (yeni açılan kapının kırım metrajı
  yok).
- Mahal evre görünümüne göre yeniden bulunmuyor: Yeni durumda birleşen iki odanın sınırları, mahaller yeniden çizilene
  ya da evre verilene dek eski yerinde.
- Parçasal yıkım (duvarın bir kesimini yıkmak) duvarı bölmekle yapılır (Duvarı böl); ayrı bir araç yok.
