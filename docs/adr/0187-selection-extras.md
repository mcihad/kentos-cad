# ADR 0187: Seçim ekleri

- **Durum:** kabul edildi (2026-10-06). Sıra sahibin kararıdır: TODOS.md §16.1, `CAD-23`'ün ardından `CAD-25` (`CAD-24` sahibin
  tarifini bekler); sahibin 5 Ekim kararıyla madde tek parçada biter. Ayrıntılar bu ADR'nin varsayılanlarıdır. Örnekler AutoCAD'in
  seçim döngüsü (Shift+Boşluk ve seçim listesi), WPolygon ve CPolygon'u, Previous'u ve SELECTSIMILAR'ı; ArcGIS'in üst üste binen
  nesnelerin seçim çipi ve seçilebilir katmanları; QGIS'in Select by Polygon'u; Netcad'in Obje Seçim Aracı ve İleri Seçim Süzgeçleridir.
- **Bağlam belgesi:** TODOS.md `CAD-25` (ilgili `CAD-10`); ADR 0029 (seçim ve kenet), ADR 0141 (Çitle, Daireyle ve İçeren alanı seç),
  ADR 0056 (Enter ve Boşluk son komutu yineler), ADR 0165 §6 (ortak araçlar iki türün şeridinde).

## Bağlam

Seç'te tıklama en özgül nesneyi alır: önce noktalar ve kenarlar, sonra imlecin içinde kaldığı en küçük alan; sürüklemek pencere ya da
kesişim kutusu çizer, Shift ekler ve çıkarır. Seç ▾'de Çitle, Daireyle ve İçeren alanı seç; Düzen'in Seçim bölümünde Tümünü seç,
Seçimi kaldır ve Ters çevir; İşlemler'de İfadeyle seç vardır. Eksik olanlar: üst üste binen nesnelerde istenene ulaşmak (ortak sınırdaki
parsel ve ada, köşedeki nokta), düzensiz bir alanla seçmek, bir alanın dışında kalanları seçmek, yanlış bir tıklamayla kaybedilen seçimi
geri getirmek, bir örneğe benzeyenleri seçmek ve yalnız bazı nesne türlerini seçmek.

## Karar

### 1. Sıradakini seç

- Tıklanan yerde seçilebilecek nesneler (adaylar) çekirdeğin sırasıyladır (`Store::hits`): imlece seçim açıklığı içinde gelen noktalar,
  yazılar ve kenarlar uzaklığa göre, sonra içinde kalınan alanlar küçükten büyüğe (iç seçimi kapalı katmanlarınki değil); eşitlerde
  belge sırası. İlk aday, tıklamanın bugün seçtiğidir (`hit`); ikisi tek kuraldan gelir.
- Tıklama birden çok aday bulunca seçilen nesne seçilir ve tıklanan yerin yanında çip açılır: “1/3 ▾”. **Shift+Boşluk** sıradakini
  seçer (sonuncudan sonra ilkine döner). Çipe tıklamak adayların listesini açar: türün ikonu, tür ve katman (“Kapalı alan · Parsel”),
  seçili olan işaretli; listede üzerine gelinen aday çizimde vurgulanır, tıklanan seçilir. Seçimde yalnız o anki aday değişir: Shift'le
  eklenmiş bir adayın yerine sıradaki girer, öbür seçililer kalır.
- Boşluk Enter'dır ve son komutu yineler (ADR 0056); çip açıkken de öyle kalır: ortak sınırlarda neredeyse her tıklama birden çok aday
  bulur, Boşluk'u döngüye vermek seç-ve-yinele alışkanlığını bozardı. Döngü AutoCAD'in eski tuşu Shift+Boşluk'tadır.
- Çip, seçim başka bir yoldan değişince, bir komut başlayınca ve Esc'le kapanır; görünüm değişince tıklanan yerle birlikte kayar.
  Shift'li tıklama bir nesneyi seçimden çıkarınca çip açılmaz.

### 2. Çokgenle seç

`tool.selectPolygon` (Seç ▾, Düzen › Seçim): çokgenin köşelerine tıklanır (kenet, orto ve kutupsal izleme geçerli), Geri (G) son köşeyi
bırakır, Enter ya da sağ tık bitirir. Kip: **İçindekiler** (İ; tümüyle içinde kalanlar, başlangıç), **Kesişenler** (K; içinde kalanlar
ve dokunanlar), **Dışındakiler** (D; çokgene hiç dokunmayan görünen nesnelerin hepsi); kip oturum boyunca hatırlanır. Çokgen en az üç
köşelidir ve kendini kesemez; kesen çokgen söylenir, araç bekler. Bulunanlar seçimin yerine geçer, Shift basılıysa eklenir; araç Seç'e
döner. Kural çekirdektedir (`Store::in_polygon`); yalnız görünen nesneler sayılır:

- **İçinde:** nesnenin her kenarı çokgenin içinde ya da sınırındadır: kenar çokgenin sınırını kestiği ve değdiği yerlerden parçalanır, her
  parçanın ortası içeride ya da sınırdadır (sınır 1 µm); nokta içeride ya da sınırda. Yazının, tablonun ve kılavuz notunun gövdesi dört
  kenarıyla, kılavuzun ok başı uçlarıyla sayılır; çok parçalı nesnenin bütün parçaları, bloğun bütün parçaları içinde olmalıdır. Yardımcı
  çizgi ve ışın hiçbir zaman içinde değildir.
- **Dokunan:** bir kenarı çokgenin sınırını keser ya da ona değer, bir noktası içeride ya da sınırdadır, ya da çokgen kapalı alanın,
  dairenin ya da taramanın içindedir (deliğinde değil): pencerenin ve dairenin kuralı (ADR 0029, 0141).
- **Dışında:** görünen ve dokunmayan.

### 3. Önceki seçim

`edit.previousSelection` (Düzen › Seçim): oturum, yeni bir seçimin yerini aldığı ya da kaldırılan son boş olmayan seçimi tutar. Önceki
seçim onu geri getirir: çizimde hâlâ olan ve görünen nesneleri, sırasıyla. Yeniden basmak bir önceki hâle döner (geri getirilen seçim de
bir seçimin yerini alır). Shift'le eklemek ve çıkarmak öncekini değiştirmez; silinen ve geri alınan nesneler düşer. Süzgeç (§5)
uygulanmaz: geri gelen, o zaman seçilmiş olandır.

### 4. Benzerini seç

`tool.selectSimilar` (Seç ▾, Düzen › Seçim): seçili nesneler varsa onlar, yoksa tıklanan nesne örnektir; görünen bütün nesnelerden en az
bir örneğe ölçütlerde eşit olanlar seçilir. Ölçütler: **Tür** (T; bloğun yerleştirmesinde bloğun kendisi de), **Katman** (K), **Renk**
(R; nesnenin kendi rengi, katmanına göre olan ayrı bir değer), **Sembol** (S; nesnenin kendi sembolü). Dördü de açık başlar ve oturum
boyunca hatırlanır; hepsi kapalıyken bütün görünen nesneler benzerdir. Araç seçtikten sonra açık kalır: bir ölçütü değiştirmek aynı
örneklerle yeniden seçer, başka bir nesneye tıklamak onu örnek yapar; Enter, Esc ya da sağ tık bitirir. Shift basılıysa sonuç seçime
eklenir. Kural iki platformda aynı ortak durumlarla sınanır (`fixtures/selection/v1/similar.json`, bağımsız Python başvurusuyla).

### 5. Seçim süzgeci

Durum çubuğunda **Süzgeç** hücresi (`edit.selectFilter`, oturumun ayarı `drafting.selectFilter`, kapalı başlar): açıkken yalnız
işaretli türlerin nesneleri seçilir. Türler hücrenin sağ tık menüsünde ve Düzen › Seçim › Seçim süzgeci ▾'de tek tek işaretlenir
(`edit.selectFilter.point` … `edit.selectFilter.table`, on altı tür, hepsi işaretli başlar); bir türü işaretlemek ya da kaldırmak
süzgeci açar. Süzgeç Seç'in tıklamasına ve üzerine gelmesine, pencere ve kesişim kutusuna, Çitle, Daireyle, Çokgenle ve İçeren alanı
seç'e, Benzerini seç'e, Tümünü seç ve Ters çevir'e ve seçim yokken nesne seçen komutların seçme adımına (Taşı, Kopyala … ortak
tabanı) uygulanır. Tıklanan yerdeki adaylardan süzgeçten
geçen ilki seçilir. Süzgecin dışarıda bıraktığı söylenir (“Seçim süzgeci 4 nesneyi dışarıda bıraktı.”). Önceki seçim, İfadeyle seç,
Veride ara'nın sonuçları, komutların kendi sonuçlarını seçmesi, komutların tek nesne isteyen adımları (Buda'nın kesen kenarı,
Ötele'nin nesnesi) ve İşlemler'in Sahneden seç'i (alanın türleri kendi süzgecidir) süzülmez. Kenet süzülmez.

### 6. İki türde

Seçim iki proje türünde ortaktır (ADR 0165 §6): yeni araçlar ve komutlar CAD'in ve CBS'in şeridinde aynı yerdedir (Giriş › Seçim,
Düzen › Seçim), Süzgeç hücresi iki türün durum çubuğundadır.

## Kapsam dışı

Seçim kümelerini adla kaydetmek; süzgeçte katman, renk ya da öznitelik koşulu (İfadeyle seç vardır); Benzerini seç'te öznitelik değeri
ve kalınlık; Daireyle seç'in Dışındakiler kipi; imleç üst üste binen nesnelerin üzerinde dururken açılan liste (AutoCAD'in seçim döngüsü
rozeti).

## Uygulama

Tek parçada (6 Ekim). Çekirdekte `Store::hits` (tıklamanın adayları; `hit` aynı nesne puanından ilkini sıralamadan bulur:
`store/pick.rs`'in `score`'u) ve `Store::in_polygon` ile `ring_problem` (`store/polygon.rs`: kenar sınırla buluştuğu yerlerden
parçalanır, her parçanın ortası içeride ya da sınırda, 1 µm; sınırın içinde kalan alan dokunur), web'e `hits`, `inPolygon` ve
`selectionRingProblem`; masaüstünde `Spatial::hits`, `in_polygon`. Seçim iki platformda önceki seçimi ve çipi tutar (masaüstü
`kentos_interaction::Selection`'ın `previous`, `Cycle`'ı; web `model/selection.ts`'in `previous`, `cycle`'ı; her başka değişiklik çipi
kapatır). Süzgeç oturumun ayarı `drafting.selectFilter`, türleri oturumun (masaüstü `App::select_kinds`, araçlara `Draft::select_kinds`;
web `DraftingSettings.selectKinds`); kuralları `kentos_interaction::selectable` ve `tools/selectable.ts` (Seç, seçim araçları,
değiştirme araçlarının ortak tabanı, Tümünü seç ve Ters çevir onunla). Araçlar: Çokgenle seç (`select_polygon.rs`,
`PolygonSelectTool`), Benzerini seç (`select_similar.rs` kuralıyla, `model/selectSimilar.ts` ve `tools/selectSimilarTool.ts`). Komutlar
masaüstünde `selection_commands.rs`, web'de `app/selectionCommands.ts`; Shift+Boşluk `edit.cycleSelection`'ın kısayolu. Çip masaüstünde
`selection_chip.rs` (KentOS UI'ın menüsüne satır vurgulanınca ileti, `Menu::highlight` ve `on_unhighlight`; `beside` içindekinin
menüsünü iletir), web'de `ui/shell/SelectionChip.ts` (`PopupMenu`'nün `highlight`'ı). Durum çubuğunda Süzgeç hücresi ve sağ tık menüsü
(masaüstü `view.rs`, web `ui/statusbar/selectFilterMenu.ts`); Düzen › Seçim'de yeni komutlar ve Seçim süzgeci ▾. İkonlar sahibin
seçtikleri: Çokgenle seç kesikli çokgen, içinde nesne, köşelerde tutamaç; Benzerini seç tutamaçlı örnek ve kesikli benzeri; Önceki seçim
kesikli kutu ve geri al oku; Seçim süzgeci huni; Sıradakini seç üst üste iki kare ve dönüş oku. Ortak durumlar
`fixtures/selection/v1` (`hits.json`, `polygon.json`, `similar.json`; bağımsız başvuru `scripts/fixtures/selection_cases.py`), ortak iz
`selection-extras.json` (oynatıcılarda `cycle` beklentisi).

## Doğrulama

- `python3 scripts/fixtures/selection_cases.py --check` (tıklamanın adayları, çokgenin üç kipi ve halka sorunları kesirlerle, benzerlik;
  ADR'den, KentOS kodu olmadan); çekirdeğin `tests/all/selection.rs`'i ve web `viewport/selection.wasm.test.ts` (WASM'dan aynı
  dosyalar), iki platformda benzerlik (`select_similar` testleri, `model/selectSimilar.test.ts`).
- Ortak iz `selection-extras.json` iki platformda üç türde.
- Resimler: `(cd apps/web && node scripts/e2e/shots.mjs selecting)`,
  `KENTOS_SHOTS_ONLY=secim-cip,secim-cip-liste,secim-cokgen,secim-cokgen-sonuc,secim-benzeri,secim-suzgec,secim-suzgec-menu cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`
  (`.run/shots/arac-secim-*`).
