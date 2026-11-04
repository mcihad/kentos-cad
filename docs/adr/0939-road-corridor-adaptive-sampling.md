# 0939 — Koridorun uyarlanır örneklenmesi: düşey kiriş yüksekliği ve ölçülerek sıklaştırma

Tarih: 2026-11-04. Durum: kabul (Y33.1 çekirdek, Y33.2 komut ve kayıt; Y33.3 pencere). Kapsam ROAD-COR-08, ROAD-COR-09.

## Sorun

Koridor kesitleri belli km'lerde kurar, araları düz birleştirir:

- öznitelik çizgileri noktadan noktaya düz gider;
- üst yüzey iki kesit arasında bir üçgen şerididir (ADR 0905).

Bugünkü sıklık beş kaynaktan gelir:

- aliymanda sabit aralık;
- kurpta ve klotoidde aralık, isteğe bağlı olarak kiriş sapmasıyla (eksenin orta ordinatı);
- geometri noktaları;
- boykesitin PVI'leri;
- deverin kritik km'leri.

Bunlar aradaki gerçek kesitin düz birleştirmeden ne kadar saptığını hiçbir yerde ölçmüyor. Bilinmeyen dört şey var:

- **Düşey kurp:** düz birleştirmenin ondan ne kadar saptığı.
- **Dış kenar:** eksenin orta ordinatı tutsa bile dış kenar daha çok sapar, `(R+o)/R` katı.
- **Arazi:** şevin buluştuğu araziyi hiçbir önsel kural bilemez. Yarma–dolgu geçişi gibi biçim değişiminin iki kesit
  arasında nerede olduğu da bilinmez.

ROAD-COR-09 bunu ister: "uyarlanır örnekleme konum, teğet, enine eğim ve yüzey sapmasını doğrulasın; sabit 20 m aralık
tasarım güvencesi sayılmasın".

## Dayanak (araştırma 4 Kasım 2026)

**Civil 3D, Frequency to Apply Assemblies:**

- Kurplar "By Curvature" seçeneğinde orta ordinat mesafesiyle (mid-ordinate distance) bölünür; kurp aralığıyla ya da
  ikisiyle birlikte de bölünebilir.
- Boykesit kurplarının ayrı bir aralığı vardır.
- Düşey geometri noktalarına, tepe ve dip noktalarına ve dever kritik noktalarına kesit konabilir.
- Aynı yerde iki kural varsa sık olan kazanır.
- Uygulamada 0,02 m orta ordinatla aliymanda 10 m aralık birlikte kullanılır (Arkance notu).

**OpenRoads Designer, Linear Template:**

- "Densify Horizontal Curves" ek noktaları "yatay kurptan kiriş sapmasına göre" hesaplar; değer kiriş yüksekliğidir,
  öntanımlısı 0,01.
- "Densify Vertical Curves" aynısını boykesitten yapar; öntanımlısı 0,1.
- Bentley'in fikir panosundaki bir isteğe göre aralık ve iki sıklaştırma birbirini hesaba katmadan ayrı ayrı uygulanır.
- Kurum önerileri (Iowa DOT 21B-270, FHWA Roadway Designer kılavuzu): yatayda 0,1, düşeyde 0,05 kiriş yüksekliği.
  Değer küçüldükçe yüzey büyür.

**Bulunmayan:** iki programda da aradaki gerçek kesiti kurup ölçen bir doğrulama bulunamadı. Arazi ve biçim değişimi
için yalnız aralık ve kullanıcının elle eklediği km'ler var.

**Kaynaklar:**

- [Civil 3D Frequency to Apply Assemblies](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-4D57F0BE-4EAF-4CBB-AE80-F9C6F45A7AE1.htm)
- [Arkance: Assembly Frequency](https://emea.support.arkance.world/hc/cs/articles/36468568670482-Features-in-Civil-3D-2015-Assembly-Frequency)
- [OpenRoads Linear Template](https://docs.bentley.com/LiveContent/web/OpenRoads%20Designer-v2024.0.2/Help/en/topics/1296840/GUID-A91AA464-023F-4538-9E0D-8FFF1FAA7B33.html)
- [Iowa DOT 21B-270](https://ia.iowadot.gov/design/dmanual/21B-270.pdf)
- [FHWA Roadway Designer](https://webarchive.library.unt.edu/web/20161031220244/https:/flh.fhwa.dot.gov/resources/cadd/rd/1-CorridorModelingManual.htm)
- [Bentley fikir BCI-I-1129](https://bentleyopencivil.ideas.aha.io/ideas/BCI-I-1129)

## Karar

### §1 Önsel: düşey kurbun kiriş yüksekliği

İkinci türevi her yerde en çok `k` büyüklükte olan bir fonksiyonu yatay `h` adımında düz birleştirmek, en çok `k·h²/8`
sapar. Bu, doğrusal aradeğerlemenin hata sınırıdır. Parabolde `z'' = (g₂−g₁)/L` sabittir, sapma tam olarak budur ve
adımın ortasında olur.

`t` kiriş yüksekliği için adım `h = √(8t/k)`'dir (`sampling::vertical_step`). Düz eğimde (`k = 0`) adım sonsuzdur.

Kurp bu adımla eşit bölünür; km'lerin gerekçesi `Why::Vertical`'dır. Çember düşey kurpta `k` kurbun en dik yerindeki
`1/(R·cos³θ)` değeridir, böylece sınır yine tutar. Asimetrik parabolün iki kolu kendi `k`'larıyla ayrı bölünür.

Bu, OpenRoads'un "Densify Vertical"ının ve Civil 3D'nin boykesit kurp aralığının karşılığıdır. Ancak aralığı kullanıcı
uydurmaz, toleranstan çıkar.

### §2 Sonsal: aradaki kesit ölçülür, gerekirse ikiye bölünür

Her aralığın ortasındaki kesit kurulur ve uçların düz birleştirmesiyle karşılaştırılır (`sampling::stray`). Ölçülen
her kod (eksen, bileşen kenarları, hendek, şev ucu) için:

- **Plan sapması:** noktanın, aynı kodun iki uçtaki noktası arasındaki kirişe uzaklığı. İzdüşüm kirişin uçlarına kırpılır.
- **Kot sapması:** noktanın kirişin o yerdeki kotundan farkı.

Her ikisinin en büyüğü ve onu veren kod tutulur.

Üç kesitin kodları aynı değilse kesit **değişmiştir**. Örnekler:

- yarmadan dolguya geçiş (hendek kodu biter);
- şevin buluşmadığı yan;
- kesitin hiç olmadığı yer.

Değişen kesit hiçbir toleransla "içinde" sayılmaz.

Toleransı aşan ya da değişen aralık ikiye bölünür. İki yarısı sırayla, derinlik öncelikli olarak ölçülür. Bir aralık şu
durumlardan birine kadar bölünür:

- **İçinde:** tolerans tutuyor.
- **En kısa adım:** yarıları en kısa adımın iki katından kısa kalacak. Ortası eklenir ve aralık en kısa adımda bırakılır.
  Değişen kesitte bu, değişimin yeri en kısa adımın iki katı içinde bulundu demektir. Değişmeyen kesitte, en kısa adımın
  çözemediği bir sapma demektir.
- **Bütçe:** kurulan orta sayısı bütçeyi doldurdu. Kalan aralıklar ölçülmeden bırakılır.

En kısa adımın iki katından kısa aralık ölçülmez (`sampling::refine`).

**Bu bir ölçümdür, kesin üst sınır değildir.** Yalnız ortalar ölçülür. Ölçülen iki yer arasına sığan dar bir tümsek
görünmez. Ortak durumlardaki "Dar dalga uçlarda ve ortada sıfır" bunu bilerek gösterir: λ = 3 m'lik dalga 12 m'lik
aralığın uçlarında da ortasında da sıfırdır, sapma 0 ölçülür. Rapor bu yüzden ölçülen en büyük sapmayı yazar,
"en çok" demez.

### §3 Konum, teğet, enine eğim, yüzey

ROAD-COR-09 dört şey sayar. Bu ölçüm dördünü kodlu noktalar üzerinden okur:

- **Konum:** plan sapmasıdır. Dış kenarın sapması da içindedir; bugünkü yatay kiriş sapması yalnız ekseni bölüyordu.
- **Teğet:** ardışık kesitlerin yön değişimi `κ·h`'dir. Orta ordinatı `≈ h·Δθ/8` olan kiriş bunu konum sapmasında taşır.
- **Enine eğim:** dever geçişi kenar kotunu değiştirir; kenar noktalarının kot sapmasında görünür. Kritik km'ler zaten
  kesittir (ADR 0905).
- **Yüzey:** şev ucunun ve hendeğin kotu araziden gelir; onların kot sapmasında görünür.

### §4 Komut ve kayıt (Y33.2)

- **`CorridorDef`** iki isteğe bağlı alan alır (KCAD şema 168):
  - `verticalSag`: düşey kurpların kiriş yüksekliği, m.
  - `refine { plan?, height?, least }`: sapma toleransları ve en kısa adım, m.
- **Hiçbirine öntanımlı değer konmaz.** Yazılmazsa koridor eskisi gibi örneklenir; eski dosyalar bayt bayt aynı kalır.
- **`road.corridors.edit` ile kaldırma:** `verticalSag` 0 ya da `refine.least` 0 alanı kaldırır; bu, `sag` 0'ın
  kuralıyla aynıdır.
- **Sınırlar:**
  - toleranslar 0 < t ≤ 10 m;
  - en kısa adım 0,01–100 m;
  - kurulan ortaların bütçesi, koridorun km sınırından (20 000) kalanıdır.
- **Km'lerin gerekçesi:** eklenen km'ler `refine`, düşey bölmeler `vertical` gerekçesiyle gelir. Rapordaki `sampling`
  şunları taşır:
  - ölçülen orta sayısı;
  - eklenen km;
  - ilk aralıkların (`before`) ve sondakilerin (`after`) en büyük plan ve kot sapması; her biri km'si ve öznitelik
    çizgisinin koduyla;
  - en kısa adımda bulunan kesit değişimleri;
  - bırakılan aralıklar (`change`, `least`, `budget`).
- **Uyarı `corridor_sampling_left`:** değişmeyen kesitin en kısa adımda da toleransa inmediği ya da km sınırının dolduğu
  aralıkları söyler. Kesit değişimlerinin yeri uyarı değildir, bulunmuş bilgidir.

Python başvurusu (`road_corridor_command_cases.py`) kesitleri kendi kesit başvurusuyla, araziyi 40 haneyle kurar.
Yukarıdaki ikiye bölmeyi `road_sampling_cases.refine_with` ile yapar. Düşey bölmeyi boykesit raporunun kurplarından
çıkarır:

- parabolde `|g₂−g₁|/L`;
- asimetrikte iki kolun `r₁`, `r₂`'si;
- çemberde dik ucun `(1+g²)^{3/2}/R`'si.

Yeni ortak durumlar: 11 sorgu ve 7 düzenleme. Rust masaüstü ve Python tipli katman koridorun 102 durumunun hepsinde
aynı sonucu verir.

**Ölçüm (örnek proje, Ana koridor km 0+100–0+260, Tip 1, 25 m aliyman, 10 m kurp; plan ve kot 0,05 m, en kısa adım 0,5 m):**

| | Plan sapması | Kot sapması |
|---|---|---|
| İlk km'ler | 0,679 m | 0,539 m |
| Denetimden sonra | 0,049 m | 0,039 m |

- 47 ara kesit ölçüldü, 21 km eklendi.
- Kesitin kodlarının değiştiği 3 yer en kısa adımda bulundu.
- Yan koridorda (50 m aliyman, kot 0,05 m) en büyük plan sapması 3,351 m'den 0,377 m'ye indi.

## Ortak durumlar

`fixtures/road/v1/sampling.json`, `scripts/fixtures/road_sampling_cases.py` tarafından KentOS kodu olmadan yazılır.
Kesitler kapalı biçimde verilir:

- eksen doğru ya da sola dönen R çemberi;
- normal üstünde ötelemeli kodlar ve kodların km aralıkları;
- kot `z₀ + g·s + c·s²/2 + a·sin(2πs/λ) + dz`;
- kesitsiz aralıklar.

Durumlar:

- 5 düşey adım;
- 7 tek orta sapması: parabolde `c·h²/8 = 0,125`; R 120'de dış kenar eksenin `126/120` katı; kirişin dışına düşen
  izdüşüm;
- 21 sıklaştırma: parabol, iki çember, dalgalı arazi, hendeğin başladığı yer, kesitsiz aralık, bütçe, en kısa adım,
  toleranssız biçim değişimi, eşit olmayan aralıklar, örtüşme sınırı.

Rust (`crates/shared/road/tests/all/sampling.rs`) 1e-9 ile aynısını bulur.

## Açık

- Pencere (Y33.3).
- Çember düşey kurbun sınırı için en dik yer, kurbun uçlarından okunur.
