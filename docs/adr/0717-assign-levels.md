# ADR 0717: Kat ver

- **Durum:** kabul edildi (27 Ekim 2026); ARCH-UX-04'ün dördüncü dilimi.
- **Bağlam:** ADR 0713 (etkin kat), ADR 0715 (nesnenin katı, kat görünümü), ADR 0716 (paftada kat), ADR 0703 (mimari
  denetim), ADR 0641 (mahalın açık katı), ADR 0278 (kilitli katman), ADR 0286 (çoklu seçimde Öznitelikler).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2'ye satırı yazıldı.

## Bağlam

Katlar (ADR 0713) gelmeden çizilmiş duvarlar, mahaller, kolonlar ve merdivenler katsızdır. Kat görünümü (ADR 0715) ve
paftada kat (ADR 0716) onları her katta gösterir; Denetim (ADR 0703) her birini “katı yok” diye sayar ama düzeltmesi
yoktu. Kat ancak Öznitelikler'de, nesne nesne ve tek türde verilebiliyordu: duvarla mahalı birlikte seçmek Kat satırını
gizliyordu. Eski bir planı katlara oturtmak onlarca tıklama istiyordu.

Araştırma (üretici belgeleri; yerelde çalıştırılmadı):

- [Archicad 29 — Home Story](https://help.graphisoft.com/AC/29/INT/_AC29_Help/040_ElementsVB/040_ElementsVB-4.htm):
  her eleman bir ana kata bağlıdır; seçili elemanın ana katı Tracker'dan değişir; Relink Home Story ana katı değiştirir,
  belge “The element will stay in place even as its Home Story changes” der.
- [Revit 2023 — Properties Palette](https://help.autodesk.com/cloudhelp/2023/ENU/Revit-GetStarted/files/GUID-A764EA7A-FE26-469B-857C-F3A70812FC34.htm):
  birden çok eleman seçilince palet hepsinin ortak örnek özelliklerini gösterir.

KentOS kararı: Archicad'in Relink Home Story'si gibi kat verme yer değiştirmez (KentOS planı iki boyutludur; kat nesnenin
bilgisidir). Revit'in ortak özellikleri gibi karışık mimari seçimde tek Kat satırı. Katsızların hepsine birden kat
vermek Denetim'dendir: üreticilerde katsız eleman olmadığından bunun karşılığı yoktur, KentOS'un eski çizimleri içindir.

## Karar

### 1. Komut: `cad.levels.assign` (“Kat ver”)

- Girdi `level` (projenin katı, yoksa `level_not_found`) ve isteğe bağlı `uids`. `uids` verilirse onlar: duvar, mahal,
  aks, kolon, merdiven, rampa ve mahal ayırıcı; kapı, pencere, etiket, kaplama, çizgi ve mahal ayırıcı olmayan çoklu
  çizginin kendi katı yoktur (`not_levelled`, ADR 0715 §1). `uids` yoksa Denetim'in `arch_level_missing` dediği her nesne:
  katsız ya da projede olmayan bir kattaki duvar, mahal, kolon, merdiven, rampa ve mahal ayırıcı; akslar katsız kalabilir
  (her katın akslarıdır).
- Zaten o kattaki nesne değişmez; kilitli katmandakiler atlanır ve sayılır (`layer_locked` uyarısı), hepsi kilitliyse
  ret (ADR 0278); değişecek nesne yoksa `nothing_to_assign`. Retlerin sırası: `no_entities`, `invalid_uid`,
  `entity_not_found`, `not_levelled`, `level_not_found`, `invalid_revision`, `revision_conflict`, `layer_locked`,
  `nothing_to_assign`.
- Tek geri alma adımı “Kat ver”; yerler değişmez. Mahalın sınırı aynı adımda yeni katının duvarları arasında yeniden
  bulunur (ADR 0641; katsız mahal bütün katların duvarlarına bakar). Kapanmayan mahal ya da duvarları katından ayrılan
  mahal son sınırıyla, `lostEnclosure` ile kalır ve `arch_space_broken` uyarılır. Uyarı, izleyicilerin komut adımı biterken
  çalıştığı için geçmişsiz okuma kopyasında önceden hesaplanır; plan ve yazma aynı uyarıyı verir.
- Plan ve cevap kat verilenleri çizimin sırasıyla verir: `uid`, `id`, `kind`, `from` (eski katı).
- Python `kentos.cad.levels.assign`, MCP aracı (yıkıcı listesinde), komut geçmişinde “Kat ver”.

### 2. Masaüstü

- **Denetim:** Katlar grubunun her bulgusunun Düzelt'i “Katını Zemin kat yap”: denetim etkin katı verir (`cad.arch.audit`
  girdisinin yeni `level`'i; yoksa zemin kat — kotu 0 olan, yoksa ilk kat; projede olmayan `level_not_found`). Kat
  bulgusu varken çubukta **Kata ver (n) ▾**: başlığı “Katsız n nesne”, **Seç ve göster** (hepsi seçilir, görünüm onlara
  gelir: önizleme), sonra projenin katları (etkin kat işaretli); seçilen kat hepsine tek adımda verilir, nesneler seçili
  kalır, denetim yinelenir. Mahal ayırıcı da kat denetimine girdi.
- **Öznitelikler:** karışık ya da tek türde birden çok mimari nesne seçiliyken Ortak özellikler'de **Kat** (ortak kat ya
  da Çeşitli); seçilen kat hepsine tek adımda. Seçimde katı olmayan bir nesne varsa satır yoktur.

### 3. Ortak durumların koşucusu

Masaüstünün ortak durum koşucusu (`run_op`) yüzlerce komutu tek `match`'te çalıştırıyordu; debug derlemede her kolun
girdisi ve cevabı aynı çerçevede yer tuttuğundan, bütün durumlar birlikte koşunca bir DWG okuması test iş parçacığının 2 MiB
yığınını aşıyordu (2,2 MB ile geçiyordu). Her komut kendi çerçevesinde (`run_typed`, `#[inline(never)]`) çalışır; ürün
kodu değişmez.

## Sınırlar

- Bloklar, çizgiler ve yazılar katsızdır; kat verilmez (katman ayrımı sürer).
- Kat nesnenin bilgisidir; kotu değiştirmez, nesneyi taşımaz.
- Denetim'in toplu kat vermesi bütün katsızlaradır; bir kısmına vermek için seçip Öznitelikler'in Kat'ı kullanılır.

## Doğrulama

- `python3 scripts/fixtures/levels_assign_cases.py --check` ve `KENTOS_CASES_ONLY=cad.levels.assign`: 32 ortak durum KentOS
  kodu olmadan (katsızlar, iki kat, plan, doğrulama, aks, taşınan duvar, mahalın kapanması ve kaybı, ayırıcı, olmayan
  kattaki rampa, kilitli, yinelenen kimlik, her ret), Rust ve Python.
- `python3 scripts/fixtures/arch_audit_cases.py --check` ve `KENTOS_CASES_ONLY=cad.arch.audit`: 41 durum; kat bulgularının
  düzeltmesi, ayırıcı, girdinin katı ve retleri, Düzelt ve yeniden denetle.
- Yetenek testi (`capabilities`): tür denemesi ve her kodun bir durumda geçmesi. Bütün ortak durumlar varsayılan yığınla,
  uygulama (241+67), etkileşim (770), masaüstü (1346) ve MCP takımları; değişen crate'lerde clippy.
- `cargo test -p kentos-desktop --bin kentos-cad kata_ver several_architectural_objects_take_one_level`: Denetim'in
  Seç ve göster'i ve tek adımda kat vermesi, geri alma; Öznitelikler'in ortak Kat'ı.
- Gerçek GUI resimleri: `KENTOS_SHOTS_ONLY=denetim-katlar,denetim-kata-ver,denetim-katsizlar,denetim-kat-verildi
  cargo test -p kentos-desktop --bin kentos-cad tools_screens::screens -- --ignored --nocapture`.
