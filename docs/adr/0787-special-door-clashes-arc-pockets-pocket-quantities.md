# ADR 0787: Özel kapıların çarpışma denetimi, yaylı duvarda cep ve cebin metrajdan düşülmesi

- **Durum:** kabul edildi (2026-10-31).
- **İş:** plan/architecture-advanced.md §3 `ADV-03`'ün “Açık” satırı (ADV-03c): döner ve katlanır kapının çarpışma denetimi,
  yaylı duvarda cep, cebin metrajdan düşülmesi; ADR 0777'nin “Sınırlar”ının üstüne.
- **Web:** web dururken (ADR 0228) yalnız masaüstü; web'e gerekenler TODOS.md §0.2'de.
- **`.kcad`:** şema değişmez. **Sözleşme:** `WallRow.pocketsVolume` (yalnız komutun çıktısı).
- **Bağımlılık:** yeni kitaplık yok.

## Bağlam

ADR 0777 döner, cebe giren ve katlanır kapıyı çizdi; döner kapının tamburu ve katlanır kapının panelleri için “süpürme
alanı” da hesaplandı (`geom::opening::swing_zones`), ama ADR 0772'nin Kapılar denetimi yalnız kanatlı kapının kanadına
bakıyordu: tamburu kolona giren döner kapı, panelleri toplandığı yerde kapıya çarpan katlanır kapı, duvar yüzünde kayarken
dik duvara giren raylı sürme kapı denetimde görünmüyordu. Cebe giren kapı yalnız düz duvarda denenmişti: yaylı duvarda
kanat kirişin üstünde düz çizilir, yayın içindeki cebe uymazdı. Cebin duvardaki boşluğu metrajda düşülmüyordu: duvarın
hacmi cebin hacmi kadar fazla çıkıyordu.

Başvurulan kaynaklar (31 Ekim 2026'da okundu):

- **Autodesk Revit:** kapı açılım ve kullanım alanı için yerleşik bir kategori yoktur; Interference Check kapının
  açılımını engel saymaz, ofisler ailenin içine saydam “clearance” katı çizer ve çakışmayı Navisworks ya da Solibri'de
  arar (AUGI, “Wish List System”; Revit Ideas, “Clearance zone group in Revit family”).
  <https://forums.augi.com/archive/index.php/t-136939.html>,
  <https://forums.autodesk.com/t5/revit-ideas/clearance-zone-group-in-revit-family/idi-p/14181448>
- **Graphisoft Archicad:** kapı açılımının çakışması kütüphane nesnesinin açılım çizgisinden ve IFC'ye yazılan açılım
  geometrisinden Solibri gibi denetleyicilerle aranır (“Door swing width/depth at ifc-export”, “Door clearance options”).
  Yaylı duvardaki kapının duvarda açtığı boşluk eğrisel ya da radyal olur; “Opening Side in Curved Walls: Radial”
  seçeneği ve boşluk yüzlerinin “düz ya da eğri” bölünmesi (Archicad yardımı, “Model Attributes of Wall Opening”);
  topluluk, duvar yayının içinde düz boşluk istemenin yolunu boş açıklıkla arar (“Straight door opening in curved wall”).
  <https://community.graphisoft.com/t5/Collaboration-with-other/Door-swing-width-depth-at-ifc-export-wallhole-size/td-p/141849>,
  <https://community.graphisoft.com/t5/Documentation/Door-clearance-options/m-p/354672>,
  <https://help.graphisoft.com/AC/20/INT/AC20Help/Appendix_Tools/Appendix_Tools-50.htm>,
  <https://community.graphisoft.com/t5/Modeling/Straight-door-opening-in-curved-wall/td-p/651165>
- ADR 0719 §4: niş ve baca duvarın net yüzeyinden düşülmez, satırı derinliğini taşır, duvarın kuralı bunu söyler.

İki üreticiden çıkan ders: çakışmayı ikinci bir program bulur ve çizimde açıklanmaz. KentOS bunu ADR 0772'nin tek
denetiminde, kapının kendi alanlarından ve düzeltmesiyle yapar; ikinci bir denetim sistemi kurulmaz.

## Karar

### 1. Kapıların çarpışma denetimi

- Kapının **açılırken aldığı alanlar** tek işlevdedir: `geom::opening::door_zones` = `swing_zones` (kanatlı kapının
  kanat dilimleri, döner kapının tamburu, katlanır kapının toplanan panellerinin alanı) + tek kanatlı raylı sürme kapının
  yolu (ADR 0777 §4: kanadın bandı, açıldığı yüzün dışında t/4'ten kanadın kalınlığı derinliğinde, ışığın uzak kenarından
  menteşe yanında bir ışık ötesine kadar). Pencere ve duvarın içinde kayan sürme kapının alanı yoktur.
- `cad.arch.audit`'in Kapılar grubu (`doors`) her kapı türünün alanlarını ADR 0772 §6'nın kuralıyla sınar: başka duvar
  (kendi duvarı değil), kolon, donatı ve öbür kapıların alanları; aynı kat; 1 mm'lik pay. Bulgu yine
  `arch_door_swing_clash` (fiziksel çarpışma), anahtarı `kod:kapı,engel` (ADR 0732'nin kalıcı anahtarı, yinelenende `#n`).
  İleti neyin çarptığını söyler: “… döner kapısının tamburu …”, “… katlanır kapısının panelleri toplanırken …”,
  “… sürme kapısının kanadı kayarken …”; iki kapının biri kanatlı değilse “… kapıları açılırken birbirinin alanına
  giriyor”. Kanatlı kapının iletisi ve yeri değişmez.
- **Düzeltme:** kapı Çevir'le (`cad.openings.edit`'in `flip`'i) önce öbür yüze, sonra öbür menteşe yanına, sonra ikisine
  birden çevrilir; alanları değişen ve bütün engellerden ve öbür kapıların alanlarından ayrılan ilk çevirme bulgunun
  `fix`'idir (“Açılışı öbür yüze çevir”, “Menteşe yanını çevir”, “Açılışı ve menteşe yanını çevir”). Döner kapının tamburu
  çevirmeyle değişmez: düzeltmesi yoktur. İki yana toplanan katlanır kapının ve iki kanatlı kanatlı kapının menteşe yanı
  alanı değiştirmez, denenmez. Kilitli katmandaki kapının düzeltmesi yoktur. Kanatlı kapının çarpışmaları da bu
  düzeltmeyi alır (ADR 0772'nin 22 durumu düzeltmeleriyle yeniden yazıldı).
- Masaüstünün Denetim sekmesi bulguları ve Düzelt'i değişmeden gösterir (tek `cad.changeset.apply`, bulguyu seçince
  önizleme).

### 2. Yaylı duvarda cep

- Cebe giren kapı yaylı duvarda da durur. Her açıklık gibi yeri ve eni duvarın taban çizgisi boyuncadır (ADR 0341); cebin
  boyu (ADR 0777 §2'nin w − lip − fit − kasa + 1 cm'i) da taban çizgisi boyunca, cep yayı izler (kavisli kanat, kavisli
  cep). `opening_pocket_short` yay boyuyla sorulur: kapı ve cebi yayın tek kenarında, uçlarının arasında kalmalı, cep öbür
  açıklığa girmemeli; `opening_pocket_thin` düz duvardaki kuralla.
- Duvarın çizimi (birleşim çözücüsü) düz duvardakinin aynısıdır: yüzler kapının üstünde kesilir, cebin açıldığı pervazda
  duvar cebin iki yanında kalır, cebin uzak ucu kendi çizgisiyle kapanır, iki yanı duvarın merkezi çevresinde yaylar.
- Kanatlar yayın merkezi çevresinde kavisli bantlardır (`arc_pocket_leaves`): kalınlığı kanadınki, orta yarıçapı bandın
  ortasının merkeze uzaklığı, ışığın başından ve sonundan geçen yarıçap çizgileri arasında; iki kanat açıyla bölünür.
  Kasa yaylı duvardaki her kapı gibi kirişin üstündedir. Çizim kapalı çoklu çizgi bükümleriyle: DXF'e `LWPOLYLINE`
  bükümleri, PDF'e yaylarıyla gider.

### 3. Cebin metrajdan düşülmesi

- `cad.spaces.quantities`'in duvar satırına `pocketsVolume` (m³): her cebin boyu × eni × kapının yüksekliği (üst − alt
  kot); yaylı duvarda duvarın ortasına ölçeklenir ((R − σ·m)/R, m duvarın ortasının taban çizgisinin soluna uzaklığı).
  Duvarın hacmi net yüzey × kalınlık eksi bu hacimdir. Yüzeyler cebi düşmez (iki yüzü de yerindedir); duvarın kuralı
  “cebe giren kapıların cebi düşülür (cebin boyu × eni × kapının yüksekliği; yüzeyler cebi düşmez)” der. Üst kotu
  olmayan kapının cebi düşülmez (yüksekliği bilinmez). Cebi olmayan duvarın satırı ve kuralı bayt bayt eskisidir.
- ADR 0719 ile tutarlı: niş yüzeyden düşülmez çünkü duvarın öbür yüzü yerindedir; cep de yüzden düşülmez. Cebin boyu
  ve eni bilindiğinden hacimden düşülür; nişin ve bacanın hacmi bu dilimde düşülmez (sınır).
- Metraj penceresinin Duvarlar sekmesinde **Cep hacmi** sütunu (Hacim'den önce); pano, XLSX, ODS ve CSV aynı sütunu
  yazar.

## Doğrulama

- Bağımsız Python başvuruları (KentOS kodu yok):
  - `scripts/fixtures/door_clash_cases.py`: `cad.arch.audit.doors.special.json`, 22 durum (her durum kendi çiziminde:
    tamburun kolona ve kapıya çarpması, katlanır panellerin kolona ve kapıya, raylı sürmenin duvara çarpması, kilitli
    katman, iki yüzü dolu kapı, başka kat, düzeltmelerin uygulanması ve sonrası). Alanlar ADR'nin kurallarıyla, çakışma
    ayırıcı eksenle 1 mm payla.
  - `scripts/fixtures/opening_types_cases.py`: ADR 0772'nin Kapılar durumları (22) düzeltmeleriyle yeniden yazıldı.
  - `scripts/fixtures/arc_pocket_cases.py`: çekirdeğin çizimi `fixtures/walls/v1/arc-pockets.json` 21 durum (pervazlar,
    kasa ve kavisli kanatlar bükümleriyle, yüz çizgileri, uç çizgileri, cebin iç yayları, gövdenin alanı; saat yönü ve
    tersi, sola/sağa yaslı, dar ve geniş yay, büyük koordinat), `cad.openings.create.arcpocket.json` 23 durum,
    `cad.spaces.quantities.pockets.json` 20 durum. ADR 0777'nin ölçüleri ve retleri `special_doors_command_cases.py`'den.
- Çekirdek: `tests/all/openings.rs`'in `pocket_doors_in_arc_walls_cut_their_walls_as_the_reference_says` (10 nm; gövde
  alanı yayların noktalanmasından ötürü %0,5 içinde).
- Masaüstü resimleri `door_clash_scenes.rs` (iki tema, 1440×900 ve 1100×650): `kapi-carpisma`, `kapi-carpisma-secili`,
  `kapi-carpisma-duzeltildi`, `yayli-cep`, `yayli-cep-arac`, `cep-metraj`.

## Sınırlar [~]

- Üç ve dört kanatlı sürme kapı (çift ray) yapılmadı.
- Döner kapının tamburu çevirmeyle düzelmez; tamburu küçültme ya da kapıyı kaydırma önerisi metindir, düzeltme değil.
- Nişin ve bacanın hacmi düşülmez; cebin hacmi kapının üst kotu yoksa düşülmez.
- Yaylı duvardaki döner ve katlanır kapı kirişin üstünde çizilir (bu dilim yalnız cebi yaya oturttu).
- Gerçek pencerede elle denenmedi; resimler pencere açmadan.

## Belgelere eklenecek satırlar

- **CLAUDE.md §0:** “**Özel kapıların çarpışması, yaylı duvarda cep, cebin metrajı (31 Ekim 2026; [ADR 0787](docs/adr/0787-special-door-clashes-arc-pockets-pocket-quantities.md), ADV-03c [~]).**
  `geom::opening::door_zones` (kanat dilimi, döner tambur, katlanır panellerin alanı, raylı sürmenin yolu); `cad.arch.audit`'in
  Kapılar grubu hepsini sınar (`arch_door_swing_clash`, ileti neyin çarptığını söyler), düzeltmesi Çevir'in ilk temiz
  çevirmesi (öbür yüz, menteşe yanı, ikisi; döner kapıda ve kilitli katmanda yok). Cebe giren kapı yaylı duvarda: cep ve
  `opening_pocket_short` yay boyunca, kanatlar merkez çevresinde kavisli bant (`arc_pocket_leaves`). `cad.spaces.quantities`'in
  duvar satırında `pocketsVolume` (boy × en × kapı yüksekliği, yayda duvarın ortasına ölçekli) hacimden düşülür, yüzeyler
  düşmez; Metraj'da Cep hacmi. 22 + 21 + 23 + 20 ortak durum (`door_clash_cases.py`, `arc_pocket_cases.py`). Üç-dört kanatlı
  sürme, nişin hacmi ve gerçek pencere kabulü açık.”
- **CLAUDE.md §2 (komutlar):** `python3 scripts/fixtures/door_clash_cases.py --check; python3 scripts/fixtures/arc_pocket_cases.py --check; cargo test -p kentos-geometry-core --test all openings; KENTOS_CASES_ONLY=doors.special,arcpocket,quantities.pockets,cad.arch.audit.doors cargo test -p kentos-native-application --test all every_command_case; KENTOS_SHOTS_ONLY=kapi-carpisma,kapi-carpisma-secili,kapi-carpisma-duzeltildi,yayli-cep,yayli-cep-arac,cep-metraj cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture   # ADR 0787`
- **plan/architecture-advanced.md ADV-03 durum satırı:** “… döner/katlanır/raylı sürme kapının çarpışma denetimi düzeltmesiyle,
  yaylı duvarda cep ve cebin metrajdan düşülmesi ([ADR 0787](../docs/adr/0787-special-door-clashes-arc-pockets-pocket-quantities.md)).
  Açık: üç-dört kanatlı sürme, PDF metninin okunması, gerçek pencerede elle kabul.”
- **TODOS.md §0.2 (Web'e ertelenenler):** “ADR 0787: Kapılar denetiminde özel kapıların alanları ve düzeltmesi, yaylı duvarda
  kavisli cep kanadı, Metraj'da Cep hacmi sütunu.”
