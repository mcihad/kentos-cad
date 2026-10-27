# ADR 0720: Etkin kat dosyada, kullanılan katın silinmesi ve araçların katı izlemesi

- **Durum:** kabul edildi (27 Ekim 2026); plan/architecture.md ARCH-UX-04'ün kalan alt maddeleri ve kabulü.
- **Bağlam:** ADR 0713 (etkin kat), ADR 0715 (kat görünümü), ADR 0716 (paftada kat), ADR 0717 (Kat ver), ADR 0718 (kat
  planı paftaları), ADR 0339 §3 (kat silme reddi), ADR 0265 (yeni şemanın kuralı), ADR 0351 (belge sekmeleri).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2'ye satırı yazıldı.

## Bağlam

ADR 0713 etkin katı oturumun durumu yaptı: çizim her açılışta zemin katta, Bütün katlar görünümüyle başlıyordu. Çok
katlı bir projede 2. katta çalışılıp kaydedilen dosya yeniden açılınca kullanıcı katı ve görünümü her seferinde yeniden
seçiyordu; birden çok çizim sekmede açıkken etkin kat sekmeler arasında taşınıyordu (sekmenin değil uygulamanın
durumuydu). Proje ayarları'nda bir kat silinirken yalnız onu anan duvarlar sorulur, mahal, kolon, merdiven, rampa, aks,
mahal ayırıcı ve paftanın kat haritası unutulurdu (silinen kat onları katı olmayan bir kata bırakırdı); ret de katın
kimliğini söylüyor ve neyin andığını göstermiyordu. Kat durum çubuğundan değiştirilince çalışan aracın ayarları
(merdivenin kat yüksekliği, mahal araçlarının Kat'ı) ve önizlemesi fare kımıldayana dek eski katta kalıyordu.

Araştırma (üretici belgeleri; yerelde çalıştırılmadı):

- [Revit 2014 — Specifying the Starting View for a Model](https://help.autodesk.com/cloudhelp/2014/ENU/Revit/files/GUID-622E667E-FB0B-47E1-8F66-E237A70771BD.htm):
  başlangıç görünümünün varsayılanı “<Last Viewed>”dir; model son bakılan görünümle açılır.
- [Revit API — StartingViewSettings.ViewId](https://rvtdocs.com/2025/aaa6f49c-faeb-851e-45e9-d3d5799c1753): görünüm
  belirtilmemişse Revit dosya kaydedildiğinde açık olan görünümleri açar.
- [Archicad 29 — Navigate Among Stories](https://help.graphisoft.com/AC/29/INT/_AC29_Help/050_ViewsVB/050_ViewsVB-6.htm)
  ve [Archicad 18 — Navigate Among Stories](https://help.graphisoft.com/AC/18/INT/AC18Help/03_2_Views_Virtual_Building/03_2_Views_Virtual_Building-7.htm):
  Go Up a Story / Go Down a Story geçerli katın bir üstüne ya da altına geçer; topluluk yanıtlarına göre kısayolları
  Ctrl+↑ / Ctrl+↓ (macOS'ta ⌘).
- [Archicad 29 — Story Settings](https://help.graphisoft.com/AC/29/INT/_AC29_Help/140_UserInterfaceDialogBoxes/140_UserInterfaceDialogBoxes-33.htm):
  kat silinince ana katı o olan bütün elemanlar da silinir ve geri alma kuyruğu boşalır.
- [Revit 2023 — About Deleting Levels](https://help.autodesk.com/cloudhelp/2023/ENU/Revit-Model/files/GUID-118C3D05-EE56-45DA-9A1D-00C5BB4DEC86.htm):
  kat silinince ona bağlı görünümler ve barındırdığı elemanlar (kapı, mobilya, mahal) silinir; uyarı listeyi gösterir,
  vazgeçilebilir.

KentOS kararı: dosya, kaydedildiği andaki etkin katla ve kat görünümüyle açılır (Revit'in son görünümü); bu bir çizim
düzenlemesi değildir. Kat silmede iki üretici de nesneleri siler; KentOS hiçbir nesneyi sessizce silmez ya da katsız
bırakmaz: anılan kat kalır, neyin andığı sayılarıyla söylenir ve tek tıkla seçilir (Archicad'in önce Relink Home Story
öğüdü gibi, Kat ver ADR 0717 ile). Üst kat / Alt kat Archicad'in Go Up/Down a Story'si gibi Ctrl+↑ / Ctrl+↓'dur.

## Karar

### 1. Model (KCAD şema 120, opt-in)

- `ArchSettings`'e `activeLevel` (çizimin açılacağı kat: projenin bir katının kimliği; `null` Katsız; yoksa zemin kat)
  ve `levelView` (`active`, `below`; yoksa Bütün katlar) eklenir. Kat görünümü sözleşmeye taşındı (`LevelView`).
- İkisi de yalnız katı olan projede yazılır; `activeLevel` projenin bir katını anmalı (`invalid_arch`, yol
  `arch.activeLevel` / `arch.levelView`). `cad.settings.set`'te `levelView: all` ayar değildir, yazılmaz.
- Şema 120 yalnız biri varken yazılır; eski dosyaların baytları aynıdır, 119 bunları `unknown_field` ile reddeder.
  Python/MCP bir dosyanın hangi katta açılacağını `cad.settings.set` ile söyleyebilir.

### 2. Masaüstü: kaydetme, açma ve sekmeler

- Etkin kat ve kat görünümü her açık çizimin kendisinindir: sekmeye bırakılan çizim onları saklar, öne gelince geri
  alır.
- Kaydet (yerel dosya ve bulut dosya projesi) ve kurtarma kopyası, yazdığı anlık görüntünün ayarlarına ekrandaki katı ve
  görünümü yazar (`with_level_state`); belgeye dokunmaz: geri alma adımı yoktur, çizim kaydedilmemiş olmaz. Zemin kat
  ve Bütün katlar hiçbir şey yazmaz, böylece katı değiştirilmemiş çizimin baytları aynı kalır.
- Açılış dosyanın söylediği katta ve görünümde başlar; dosya projede olmayan bir katı anıyorsa zemin kata dönülür.
  Varsayılan dışındaysa söylenir: “Çizim kaydedildiği katta açıldı: etkin kat 1. kat, kat görünümü …”.
- Proje ayarları formu açılış katını ve görünümünü olduğu gibi korur; formda silinen katı anan açılış katı düşer.

### 3. Kullanılan katın silinmesi

- `cad.settings.set`, girdinin kaldırdığı bir katı bir duvar, mahal, mahal ayırıcı, kolon, aks, merdiven ya da rampa
  kendi katı olarak anıyorsa ya da bir paftanın haritası gösteriyorsa reddeder (`level_in_use`): katın adı, türlere göre
  sayılar (“2 duvar, 1 mahal ve 1 kolon anıyor, 1 pafta haritası gösteriyor”) ve nesnelerin kalıcı kimlikleri
  (`uids`). Projenin zaten olmayan bir katı anan nesne başka bir değişikliği engellemez (önceden engelliyordu).
- Proje ayarları › Mimari bunu Kaydet'ten önce sorar: satırların altında uyarı ve **Nesneleri seç (n)**, Kaydet bekler.
  Nesneleri seç onları seçer, pencereyi Vazgeç gibi kapatır ve söyler; Öznitelikler'in ortak Kat'ı (ADR 0717) ya da
  Denetim'in Kata ver'i onlara başka kat verir, sonra kat silinir. Komut yine de reddederse nesneler seçilir.
- Etkin kat silinirse zemin kata dönülür; yeniden adlandırılan kat nesnelerini (kimlikle bağlı) korur, adı her yerde
  yenilenir.

### 4. Araçlar katı hemen izler

- `Tool::level_changed` (varsayılan boş) ve `Session::level_changed`: etkin kat ya da kat görünümü değişince çalışan
  araç bağlamı yeniden görür; merdiven ve rampa kat yüksekliğini, mahal, toplu mahal, mahal ayırıcı, iç ölçü zinciri ve
  çizgiden duvar Kat'ını hemen alır, önizlemeleri yenilenir. Fare çizimin üstündeyse orada yeniden görülür.

### 5. Üst kat / Alt kat

- `view.levels.up` (Ctrl+↑) ve `view.levels.down` (Ctrl+↓): etkin kat kotça bir üstteki ya da alttaki kat olur (eşit
  kotlularda projenin sırası); en üstte ya da altta değişmez ve söylenir; Katsız'dan zemin kata gelinir. Durum
  çubuğunun Kat menüsünde ve Görünüm › Kat görünümü ▾'de; manifestte oturum düzeyi (`view.*`).
- Simgeler seçenek sayfasından A (kalın etkin kat çizgisi, kesikli hedef kat ve ok).

## Sınırlar

- Veritabanı (canlı) bulut projesi açılış katını taşımaz; dosya projesi taşır.
- Kat silinirken nesneleri başka kata taşıyıp katı tek adımda silen bir yol yoktur (iki adım: Kat ver, sonra sil).
- Bloğun içindeki mimari nesneler kat silmede sayılmaz.

## Doğrulama

- `python3 scripts/fixtures/settings_set_command_cases.py --check` (104 durum, 13'ü yeni: her türün andığı kat,
  bütün katlar, üst katın duvarı, pafta haritası, duvar ve harita birlikte, zaten olmayan kat, açılış katı ve görünümü,
  Katsız, Bütün katlar yazılmaz, bilinmeyen ve kaldırılan açılış katı, katsız projede görünüm); Rust ve Python koşucuları.
- KCAD: `level-state.kcad`, `level-state-unassigned.kcad`, `broken/level-state-in-schema-119.kcad`,
  `broken/level-state-{unknown-level,without-levels,view-type}.kcad`, `broken/schema-version-121.kcad` bağımsız
  yazıcıyla; `kentos-kcad` (220), Python okuyucusu.
- Sözleşme testi, etkileşim (775; kat değişince merdiven), uygulama (241 + 67), masaüstü (1352; kaydet/aç, Katsız,
  sekmeler, kat silme ve Nesneleri seç, yeniden adlandırma, Üst/Alt kat, merdiven), MCP (açık kat, ret ve kimlikler,
  açılış katı dosyada), Python (370; açık kat, ret, açılış katı dosyada), clippy.
- Gerçek GUI resimleri: `KENTOS_SHOTS_ONLY=kat-dosya-acilis,kat-dosya-menu,kat-silme-pencere,kat-silme,kat-merdiven-izler,kat-gorunumu-serit
  cargo test -p kentos-desktop --bin kentos-cad tools_screens::screens -- --ignored --nocapture`.
