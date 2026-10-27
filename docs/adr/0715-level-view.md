# ADR 0715: Kat görünümü

- **Durum:** kabul edildi (27 Ekim 2026); ARCH-UX-04'ün ikinci dilimi.
- **Bağlam:** ADR 0713 (etkin kat), ADR 0346 (evre görünümü: gizli, kesikli, soluk), ADR 0341 (açıklık duvarında), ADR 0344
  (etiket nesnesinin), ADR 0345 (mahal kaplaması), ADR 0320 (ölçünün bağları).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2'ye satırı yazıldı.

## Bağlam

Etkin kat (ADR 0713) yeni nesnelerin katını verdi, ama bütün katlar planda üst üste çiziliyordu. Daire yürüyüşüne ikinci kat
eklenince (aynı dış duvarlar, başka bölmeler) plan okunmaz oldu: iki katın etiketleri, kapıları ve mahal alanları üst üste,
alt katın ölçüleri üst katın planında. Ayrıca üst üste duran duvarlarda tıklama belgenin sırasıyla alt katın duvarını
seçiyordu: 1. kat etkinken konan pencereler ve bir kapı **zemin katın duvarına** oturdu (gerçek GUI'de görüldü).

Araştırma (üretici belgeleri; yerelde çalıştırılmadı):

- [Revit 2023 — About Underlays](https://help.autodesk.com/cloudhelp/2023/ENU/Revit-DocumentPresent/files/GUID-F03617DC-541D-49CF-B40C-B2537175F561.htm):
  plan görünümü bir kattır; altlık (underlay) başka bir katı görünümün altında yarım tonla gösterir.
- [Revit 2023 — Options to Control Element Selection](https://help.autodesk.com/cloudhelp/2023/ENU/Revit-GetStarted/files/GUID-756F6241-C5D8-48D5-B703-9363E8C8FD65.htm):
  “Select underlay elements” kapalıyken altlığın elemanları seçilmez ama onlara kenetlenilir ve hizalanılır.
- [Archicad 29 — Virtual Trace: Using References to Edit and Compare Model Views and Drawings](https://help.graphisoft.com/AC/29/INT/_AC29_Help/030_Interaction/030_Interaction-103.htm):
  Trace Reference ikinci bir görünümü etkin olanın altında gösterir; imleç başvurunun elemanlarına duyarlıdır, kenet
  noktaları kullanılır; başvurunun elemanları doğrudan düzenlenmez.

KentOS kararı: Revit'in plan-kat ilişkisi ve altlığı, Archicad'in izleme başvurusu oturumun **Kat görünümü** olarak. Pafta
görünümü (her pafta haritasının kendi katı) bu dilimde yoktur.

## Karar

### 1. Nesnenin katı (`Entity::level`, sözleşme)

Duvar, mahal, aks, kolon, merdiven, rampa ve mahal ayırıcının kendi katı vardır (`own_level`). Açıklık duvarının katında,
etiket nesnesinin katında, mahale bağlı kaplama (taramanın `assoc.outer`'ı) mahalin katındadır. Ölçü, bağlandığı nesnelerin
hepsi aynı kattaysa o kattadır; bağsız, kopuk ya da iki katı birden ölçen ölçü katsızdır. Çizgi, yazı, blok ve öbür nesneler
katsızdır.

### 2. Görünümler

- **Bütün katlar** (varsayılan): her şey eskisi gibi.
- **Yalnız etkin kat:** etkin katın ve katsız nesneler çizilir; öbür katlarınkiler (yukarıdaki kurala göre açıklıkları,
  etiketleri, ölçüleri, kaplamaları) gizlenir: seçilmez, kenetlenmez, etiketleri yerleşmez.
- **Etkin kat ve alt kat:** bunlara ek olarak etkin katın hemen altındaki kat (kotu etkininkinden küçüklerin en yükseği)
  evre görünümünün soluk görünüşüyle (%60 saydam) çizilir, yazıları da soluk; **kenetlenir ama seçilmez** (tıklama, pencere,
  kesişim, çit, çokgen, Tümünü seç, tarama sınırı, budama sınırı onu almaz). En alttaki katta alt kat yoktur.
- Projede olmayan bir katı anan nesne gizlenmez (Denetim söyler). Katsız seçiliyken yalnız katsız nesneler çizilir.
- **Tıklamada etkin kat önce** (her görünümde): üst üste duran nesnelerde etkin katın ve katsız nesneler öbür katlardan önce
  gelir. Kapı ve pencere etkin katın duvarına oturur; düzenleme araçları etkin katın nesnesini alır.

### 3. Masaüstü

- Durum oturumundur (etkin kat gibi), dosyaya yazılmaz; açılış Bütün katlar'dır.
- Durum çubuğunun Kat menüsü: etkin kat seçimi, sonra **Kat görünümü** başlığı altında üç seçim. Kat hücresinin ikonu
  görünümü söyler (üç çizgi: bütün katlar; kalın orta: yalnız etkin kat; altında soluk: alt kat soluk); ipucu da yazar.
- Görünüm sekmesinin Görünüş panelinde (yalnız CAD) **Kat görünümü ▾**; komut adları `view.levels.all`, `.active`,
  `.below` (oturum düzeyi, manifestte `view.*` kuralı). Mimari sekmesi 1440 px'te yazılı kalsın diye oraya konmadı.
- Seçim söylenir: “Kat görünümü: 1. kat ve altında Zemin kat soluk: soluk kat kenetlenir, seçilmez.”; görünümün aldığı
  nesneler seçimden düşer.

### 4. Uygulama

- Çekirdek depoya `set_unpicked` ve `pickable`: seçen sorgular (`hit`, `hits`, `hit_edge`, `in_rect`, `in_fence`,
  `in_circle`, `in_polygon`, `containing`, `holes_at`, `enclosing`, `edges_in`, `extent_outliers`) `pickable`'ı, kenet
  `shown`'u okur. `cad.entities.select` de `pickable` ile (başsız sunucuda küme boştur).
- Etkileşimde `level_view::looks` gizlenenleri, solukları ve başka kattakileri verir; `Spatial::set_level_view` gizlenenleri
  evre görünümünün gizlediklerine katar, solukları `set_unpicked`'e verir; `hits`, `pick` ve `pick_edge` etkin katı öne alır.
- Çizim alanı evre görünüşüne (`Phasing`) gizlenenleri ve solukları katar (`Phasing::fade`); yazılar soluk katta soluk.

## Sınırlar

- Pafta haritaları ve PDF bütün katları çizer; haritanın kendi katı (Revit'in kat planı görünümü) sonraki dilimdir.
- Bloklar, çizgiler ve yazılar katsızdır: her katta görünürler (katman ayrımı sürer).
- Etkin kat ve görünüm çizimle saklanmaz; kat görünümünün seçimi bölünmüş görünümde her görünüm için ayrı değildir.
- Katsız çizilmiş nesnelere toplu kat atama hâlâ açıktır.

## Doğrulama

- `cargo test -p kentos-contracts --test all levels`: duvar, açıklık, mahal, etiket, kopuk etiket, kaplama, ölçünün aynı ve
  farklı katlı bağları, katsız bağ.
- `cargo test -p kentos-interaction --test all level_view`: görünümlerin gizledikleri ve soluk kat (en alt katta yok, Katsız);
  soluk kat kenetlenir, seçilmez; gizli kat ikisini de vermez; pencere seçimi; üst üste duvarda tıklama ve kapı etkin katın
  duvarına. Bütün etkileşim (770), geometri çekirdeği (282) ve masaüstü (1344) takımları, `cad.entities.select/.pick` ortak
  durumları geçti.
- `cargo test -p kentos-desktop --bin kentos-cad active_level`: menüyle görünüm, Kat hücresinin ikonu, seçimden düşme, etkin
  kat değişince gizlenenler.
- Gerçek GUI resimleri: `KENTOS_SHOTS_ONLY=kat-gorunumu-butun,kat-gorunumu-yalniz,kat-gorunumu-alt,kat-gorunumu-zemin,kat-gorunumu-menu,kat-gorunumu-serit
  cargo test -p kentos-desktop --bin kentos-cad tools_screens::screens -- --ignored --nocapture`: iki katlı daire üst üste,
  yalnız 1. kat, 1. kat ve soluk zemin, yalnız zemin (ölçüleriyle), Kat menüsü ve Görünüm şeridinin menüsü.
