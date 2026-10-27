# ADR 0719: Niş, baca ve tesisat boşluğu

- **Durum:** kabul edildi (27 Ekim 2026); plan/architecture.md ARCH-ELEM-02'nin bütünü.
- **Bağlam:** ADR 0341 (açıklık), ADR 0340 (duvar birleşimi), ADR 0342 (metraj), ADR 0695 (duvar ölçü dizisi),
  ADR 0704 (merdivenin plan kesmesi), ADR 0265 (yeni şemanın kuralı).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2'ye satırı yazıldı.

## Bağlam

Açıklık (ADR 0341) duvarı her zaman bir yüzden öbürüne keser. Duvarın bir yüzünden oyulan niş, duvarın içindeki baca ve
kesme düzleminin üstünde kalan tesisat boşluğu bu modelle çizilemiyordu; kapı boşluğunu başka anlamda kullanmak planı
ve metrajı yanlış gösterirdi. Türkiye'deki konut projelerinde pencere altı kalorifer nişi, mutfak/banyo bacası ve kombi
için tesisat boşluğu çok yaygındır.

Araştırma (üretici belgeleri; yerelde çalıştırılmadı):

- [Archicad 29 — Openings](https://help.graphisoft.com/AC/29/INT/_AC29_Help/040_ElementsVB/040_ElementsVB-258.htm):
  açıklık aracı duvarı ya da döşemeyi keser; sınırlı derinlikli açıklıkla duvar nişi ya da döşeme oyuğu yapılır.
- [Archicad 29 — Use Limited Opening to Create Niche or Recess](https://help.graphisoft.com/AC/29/INT/_AC29_Help/040_ElementsVB/040_ElementsVB-262.htm):
  “Limit One Side” sınırlı yüzü elemanın başvuru çizgisine oturtur; “Limit Both Sides” başvurudan ötelemeli ve boylu
  bir oyuk verir; Flip yönü çevirir.
- [Archicad 29 — Opening Display on Floor Plan](https://help.graphisoft.com/AC/29/INT/_AC29_Help/040_ElementsVB/040_ElementsVB-260.htm):
  açıklığın kesilen ve kesilmeyen hâlleri için ayrı plan simgesi vardır; kesilmeyen elemandaki açıklık simgesel çizilir.
- [Revit 2024 — Cut a Rectangular Opening in a Wall](https://help.autodesk.com/cloudhelp/2024/ENU/RevitLT-ArchDes/files/GUID-DE386617-D2BB-4F71-96B1-51D157DAEAA7.htm):
  Wall Opening duvarı boydan boya dikdörtgen keser (düz ve eğri duvarda).
- [Revit 2023 — About the View Range](https://help.autodesk.com/cloudhelp/2023/ENU/RevitLT-DocumentPresent/files/GUID-58711292-AB78-4C8F-BAA1-0855DDB518BF.htm):
  kesme düzlemi neyin kesilmiş, neyin izdüşüm çizileceğini belirler; düzlemin üstündeki elemanlar pencere, dolap ve
  genel model dışında planda görünmez.
- [Vectorworks 2021 — Creating wall recesses](https://app-help.vectorworks.net/2021/eng/VW2021_Guide/Walls/Creating_wall_recesses.htm):
  duvara kapalı bir şekille oyuk (recess) açılır.

KentOS kararı: Archicad'in sınırlı açıklığı gibi niş ve baca açıklığın kendisidir (duvarına bağlı, onunla gider,
çevrilir, taşınır, numaralanır), ayrı nesne değildir; ama kapıdan ayrı biçimlerdir. Yüz başvuru çizgisi yerine duvarın
yüzüdür (kullanıcının gördüğü). Kesme düzlemi kuralı Revit'in görünüm aralığının sade hâlidir: kesmeyi kesen kesilir,
kesmeyen kesik çizgiyle görünür.

## Karar

### 1. Model (KCAD şema 119, opt-in)

- `OpeningForm`'a `niche` (Niş: duvarın bir yüzünden `depth` içeri) ve `flue` (Baca: duvarın içinde, yüzünden `inset`
  uzakta, yoksa ortada, `depth` enine) eklenir. Yüz, açıklığın `side`'ıdır (Çevir yüzü değiştirir).
- `depth` (0 < d ≤ 10 m) niş ve bacada zorunlu, `inset` (> 0) yalnız bacada; kapı, pencere ve boşluk ikisini de
  taşımaz (`invalid_opening`). Biçim değişince derinlik biçimin ilk türünden gelir; duvarı geçen biçime dönünce düşer.
- Duvarın kalınlığına sığmayan niş (öbür yüze varan) ya da baca (bir yüze varan) yazılmaz (`recess_too_deep`); duvar
  sonradan incelirse açıklık `tooDeep` kırığını alır, uyarı üçgeniyle görünür, duvarı kesmez.
- Türler: Niş 60 (0,60 × 0,10 m, 0,90–1,50), Kalorifer nişi 100 (1,00 × 0,10 m, 0,10–0,80), Baca 20 (0,20 × 0,20 m,
  ortada, tam boy), Tesisat boşluğu 20 (boşluk, 2,30–2,50).
- Şema 119 yalnız niş, baca, derinlik, yüzden uzaklık, `tooDeep` ya da nişleri anan bir çizelge süzgeci varken yazılır;
  eski dosyaların baytları aynıdır, 118 bunları `bad_value` ile reddeder.

### 2. Planın kesmesi

- `PLAN_CUT_HEIGHT` = 1,10 m (KentOS'un varsayılanı, merdiven aracının kesmesiyle aynı; üreticinin değeri değildir).
- Boşluk, niş ve baca, alt kotu (yoksa döşeme) kesmenin altında ve üst kotu (yoksa tam boy) üstündeyse kesilir; değilse
  duvar bütün kalır ve açıklık kesik çizgiyle çizilir: boşluk pervazları ve çaprazı, niş yanları ve arkası, baca boşluğu
  ve çaprazı. Kapı ve pencere eskisi gibi her zaman kesilir (yüksek pencerenin kesik çizimi sınırdır).
- Kesilen niş duvarın yüz çizgisini, içindeki katman sınırlarını ve dolguyu pervazlar arasında derinliği kadar keser;
  arkası (bir katman sınırına denk gelmiyorsa) çizilir, kalan duvar dolgusu yerine konur. Kesilen baca yüzleri kesmez,
  iç çizgileri ve dolguyu keser, iki yanı ve çaprazı çizilir. Yaylı duvarda arka yay duvarın merkezine göredir.

### 3. Aynı duvardaki açıklıklar

Duvar boyunca örtüşen iki açıklık kotları ayrıysa ve plan en çok birini kesiyorsa yan yana durur (pencere altı kalorifer
nişi, kapı üstü tesisat boşluğu); kotları örtüşürse ya da ikisi de kesilirse `openings_overlap` (ikincisinde sözü kotu
kesmenin dışına almayı söyler). Belgenin izleyicisi `overlap` kırığını aynı kuralla verir.

### 4. Metraj ve ölçü

- Duvarın net yüzeyinden yalnız duvarı geçen açıklıklar düşülür; niş ve bacanın satırı derinliğini taşır, toplam açıklık
  alanına girmez; niş ya da baca taşıyan duvarın kuralı bunu söyler. Metraj penceresinde Derinlik sütunu.
- Duvar ölçü dizisi nişi yalnız kendi yüzünde ölçer, bacayı ölçmez.

### 5. Araç ve Öznitelikler

- Mimari › Yerleştir'de (ve duvar bağlam sekmesinde) Boşluk ▾: Boşluk, Niş, Baca. Niş ve Baca açıklık aracıdır: imlecin
  yanı yüzdür; ayarlarda Tür, Numara (başta Yok), Genişlik, Derinlik, Yüz, (baca) Yüzden uzaklık, Alt kot, Üst kot.
  Önizleme komutun planıdır; etiket yüzü, derinliği duvara göre ve planda kesilip kesilmediğini söyler; sığmayan niş
  imlecin yanında reddedilir.
- Öznitelikler: Biçim (Niş/Baca), Yüz, Derinlik, Yüzden uzaklık (Ortada), Alt kot, Üst kot ve salt okunur Planda.
- Simgeler seçenek sayfasından A (oyuk ve çaprazlı kare).

## Sınırlar

- Kesme yüksekliği proje ayarı değildir; kapı ve pencere kotlarına göre kesik çizilmez.
- Planda ikisi de kesilen, kotları ayrı iki açıklık aynı yerde duramaz.
- 3B katı, kesit/görünüşte niş ve baca bu aşamada yoktur; döşemeyi geçen şaft ADR 0708'in mahal boşluğudur.

## Doğrulama

- `python3 scripts/fixtures/opening_cases.py --check` (41 durum, 16'sı yeni: niş sol/sağ yüz, katman sınırında ve
  ortasında, eğik ve kapalı duvar, kapıyla, kesmenin üstü/altı, duvardan derin, baca ortada/yüzden/katmanlı/yüksekte,
  tesisat boşluğu, alt-üst kotlu boşluk) ve çekirdeğin `openings` testi; yaylı duvarda niş testi.
- `opening_command_cases.py --check`: `cad.openings.create` 112 (33 yeni), `.edit` 102 (17 yeni) durum; Rust ve Python
  koşucuları; `dimension_sets_command_cases.py` (2 yeni), `space_command_cases.py` (2 yeni metraj durumu).
- KCAD: `recesses.kcad`, `broken/recesses-in-schema-118.kcad`, `broken/schema-version-120.kcad` bağımsız yazıcıyla;
  `kentos-kcad` (219), Python okuyucusu ve taşıma testi.
- Etkileşim (4 yeni araç testi, 774), uygulama (241), çekirdek (534 + 283), belge, biçimler (düz dışa aktarma testi),
  masaüstü (1348, Öznitelikler testi), Python (368), MCP (78), clippy.
- Gerçek GUI resimleri: `KENTOS_SHOTS_ONLY=nis-oda,nis-ayrintili,nis-pencere-alti,nis-arac,nis-derin,nis-oznitelikler,baca-oznitelikler,nis-serit
  cargo test -p kentos-desktop --bin kentos-cad tools_screens::screens -- --ignored --nocapture`.
