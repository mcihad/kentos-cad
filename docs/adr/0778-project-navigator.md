# ADR 0778: Proje gezgini: katlar, görünümler, paftalar, çizelgeler ve bir nesnenin göründüğü yerler

- **Durum:** kabul edildi (31 Ekim 2026); `plan/architecture-advanced.md` ADV-06'nın ikinci teslimi **[~]**.
- **Bağlam:** ADV-06 ("Mimari görünüşler, proje gezgini ve koordineli yayın"), dayanak A8–A9. Görünüş tarifi ADR 0774, pafta
  seti ADR 0347, paftanın katı ADR 0716, kat görünümünün kuralı ADR 0715, etkin kat ADR 0713, çizelge ADR 0344 ve 0733,
  modüllerin yüzeyleri ADR 0802.
- **Web:** ertelendi (ADR 0228); satırı TODOS.md §0.2'de.
- **Şema:** değişmez. Yeni komut `cad.navigator.read` v1 (salt okunur).

## Araştırma (üreticilerin resmî belgeleri)

- Revit 2026, [Project Browser](https://help.autodesk.com/cloudhelp/2026/ENU/Revit-GetStarted/files/GUID-C8D3E5A6-02A5-43A9-AFFC-D49DD27398B1.htm)
  ve [Organizing Views in the Project Browser](https://help.autodesk.com/cloudhelp/2017/ENU/Revit-DocumentPresent/files/GUID-96F9CDB5-C46D-4597-943B-DF231E8EC688.htm):
  görünüşler (türe ve kata göre), çizelgeler, paftalar (paftanın altında üzerindeki görünüşler) tek ağaçta; bir öğeye çift basmak
  onu açar. Seçili elemanın “hangi paftada” olduğu ağaçta değil, ayrı komutlardadır.
- Archicad 30, Navigator: Project Map (katlar), View Map (kayıtlı görünümler), Layout Book (yerleşimler ve üzerlerindeki
  çizimler), Publisher Sets; aynı paletin dört yüzü, öğeye basmak onu açar; Archicad 19 yardımı [Navigator Palette](https://help.graphisoft.com/AC/19/INT/AC19Help/02_Interaction/02_Interaction-15.htm).
- Sonuç: KentOS **tek panelde** dört bölüm (Katlar, Görünümler, Paftalar, Çizelgeler) ve ikisinde de olmayan bir beşinci
  bölüm: tek seçili nesnenin göründüğü yerler (onu çizen görünümler ve paftalar, onu listeleyen çizelgeler, onu adlandıran
  etiketler). Ağaç bir okumadır; satırı basmak var olan işi yapar (etkin kat, görünümü göster, paftayı aç, seç ve göster).
  Bağlar yeni bir veri değildir: pafta setinin (ADR 0347), tarifin (ADR 0774) ve çizelgenin üyeliğinin (ADR 0344) okunuşudur.

## Karar

### §1 Komut: `cad.navigator.read` v1

- Girdi `{ uid? }`. Ret: `entity_not_found` (`uid`), `book_unreadable` (pafta setininki).
- `levels`: projenin katları kendi sıralarında: kimlik, ad, kot, üzerindeki nesne sayısı (nesnenin katı kat görünümünün
  kuralıyla: açıklık duvarının, etiket nesnesinin, ilişkili tarama mahalinin katında; `Entity::level`), tarifi o katı çizen
  görünümler, o katı çizen haritalar (çerçevenin katı: bağlı görünümün tarifinin katı, yoksa haritanın kendi katı).
- `unleveled`: katsız ya da projede olmayan bir kattaki nesnelerin sayısı.
- `views`: adlı görünümler: kararlı kimlik, ad, tarifinin katı, sürümü ve ölçeği, ona bağlı haritalar (görünüm silinse de
  çerçeve onun kimliğini taşır).
- `sheets`: kitabın sırasıyla paftalar: kimlik, ad, seri, haritalar (bağlı görünüm, `missing`, kat, ölçek); `cad.sheets.set`'in
  çerçevelerinden.
- `schedules`: modeli okuyan tablolar (`TableSource::Schedule`): kimlik, neyi listelediği, listelediği ayrı nesne sayısı
  (`members`'ın tekilleri).
- `object` (`uid` verilince): türü; katı (projede olmayan katta yok); onu çizen görünümler: görünümün alanının kutusu
  (dört köşesi görünümün dönüşüyle) nesnenin kutusuyla kesişir **ve** tarifi (yoksa hiçbir şey) onu bırakmaz (kat, evre,
  gizlenen tür: ADR 0774 §4'ün kuralı, `sheets_set::recipe_draws`); onu çizen haritalar (`cad.sheets.set`'in `shows`'u); onu
  listeleyen çizelgeler; onu adlandıran etiketler. Kutu kesişimidir: fazladan listelenebilir, eksik listelenmez.
- Çizimi ve kitabı değiştirmez; revizyon aynı kalır.

### §2 Ortak kural

- Haritanın çizdiği nesne kuralı tek yerde: `sheets_set::recipe_draws(doc, level, phase, hidden)`; pafta setinin `drawn_by`'ı
  ve gezginin görünüm testi onu kullanır.

### §3 Masaüstü: Gezgin paneli

- `Panel::Navigator` (“Gezgin”), Katmanlar'ın yuvasında bir sekme; kentos-cad ve KentOS Mimari'de (Harita ve GIS'te yok,
  `module_surfaces.rs`); yerleşimde `dockTab: navigator`. Komut `arch.navigator` (Mimari › Proje ▾, takma adları GEZGIN,
  PROJECTBROWSER) paneli öne getirir.
- Bölümler katlanır (▾/▸) ve sayılarını gösterir. Seçili nesne bölümü tek nesne seçiliyken **en üsttedir**.
- Satırlar: kat → etkin kat (Katsız da); görünüm → göster (`show_named_view`); pafta ve harita → paftayı aç; çizelge, etiket ve
  nesnenin kendisi → seç ve göster.
- Cevap çizimin revizyonu ya da seçim değişince yeniden okunur (panelin önbelleği), her karede değil.

## Doğrulama

- `scripts/fixtures/navigator_cases.py --check`: 21 ortak durum (`fixtures/commands/v1/cad.navigator.read.json`), KentOS kodu
  olmadan; ADR 0347'nin çerçeve kuralları (`sheet_set_cases.py`) ve ADR 0774'ün tarifi (`view_recipe_sheet_cases.py`) üzerine:
  katların sayıları, görünümlerin ve haritaların bağları, dönük görünümün kutusu, gizlenen tür, iki evre, projede olmayan kat,
  silinen görünüm, tarif değişimi, çizelgenin üyeliği, etiket, iki okuma aynı, ret. Nesnelerin kutuları çerçeve ve görünüm
  kenarlarından en az 0,5 m uzakta tutulur (başvuru denetler).
- `KENTOS_CASES_ONLY=cad.navigator cargo test -p kentos-native-application --test all -- every_command_case capabilit`: 21/21,
  yetenekler.
- `KENTOS_CASES_ONLY=cad.navigator .run/py/bin/python -m unittest python.tests.test_command_cases`: Python'dan aynı durumlar.
- `cargo test -p kentos-desktop --lib -- navigator module_surface ribbon`: ağacın okunuşu, satırların eylemleri (etkin kat,
  pafta), seçili kapının yerleri, önbelleğin seçimle yenilenmesi, modüllerin panelleri.
- Resimler: `KENTOS_SHOTS_ONLY=gezgin-agac,gezgin-nesne cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored
  --nocapture` (`.run/shots/arac-gezgin-*`, 1440×900 ve 1100×650, açık ve koyu): ADR 0773'ün 2+1 planı, iki tarifli görünüm,
  iki bağlı pafta ve kapı çizelgesi.

## Açık

- Görünüşün nesneyi kırpmasının (kırpma sınırı) ve kalem setinin gezgine yansıması; görünüş şablonu ve türetilmiş çizim
  (ADV-06'nın kalanı). Gerçek pencerede elle kabul yapılmadı.
