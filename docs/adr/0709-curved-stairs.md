# ADR 0709: Döner merdiven

- **Durum:** kabul edildi (27 Ekim 2026); ARCH-STAIR-01'in döner merdiven dilimi.
- **Bağlam:** `plan/architecture.md` ARCH-STAIR-01; ADR 0343 (merdiven), ADR 0704 (yükseklikten tasarım, `fit`,
  kesme), ADR 0705 (korkuluk), ADR 0265 (yeni şema kuralı).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2'ye satırı yazıldı.

## Bağlam

ADR 0343 ve 0704'ün merdiveni yalnız düz kollardan (düz, L, U) oluşuyordu; konutta ve dubleks katlarda sık görülen
döner (spiral) merdiven KentOS'ta çizilemiyor, yay ve radyal basamaklar elle çiziliyordu. Elle çizilen bir merdivenin
rıht sayısı, yüksekliği, kesmesi ve korkuluğu ise komutla ve Öznitelikler'le izlenmiyordu.

Araştırma (üretici belgeleri; yerelde çalıştırılmadı):

- [Revit 2020 — Center-Ends Spiral run](https://help.autodesk.com/cloudhelp/2020/ENU/Revit-Model/files/GUID-4E05115C-84C9-4930-95FB-E8B91219B1E6.htm)
  ve [Run bileşeni seçenekleri](https://help.autodesk.com/cloudhelp/2020/ENU/Revit-Model/files/GUID-26053199-6B08-44B9-907C-F5D9AA7CEC81.htm):
  önce merkez, sonra başlangıç tıklanır; dönüş yönü imlecin hareketinden gelir; bitiş tıklaması rıhtları belirler; kol
  360°'den küçüktür; konum çizgisi kolun ortası (Run: Center) seçilebilir; ters yön için Flip.
- [Archicad 25 — Stair Improvements](https://help.graphisoft.com/AC/25/INT/_AC25_Help/005_NewFeatures/005_NewFeatures-5.htm):
  basamaklar yürüme çizgisi boyunca dizilir; derinlik yay boyunca (Along Arc) ya da kiriş boyunca (Along Chord) ölçülür.

KentOS kararı: Revit'in merkez–başlangıç–yön akışı ve 360°'den küçük kol alınır; yürüme çizgisi merdivenin ortasıdır
ve basamak derinliği onun **yayı boyunca** ölçülür (Archicad'in Along Arc'ı). Yeni bir nesne türü yoktur: döner merdiven
ADR 0343'ün merdivenidir, iki noktalı yürüme çizgisi bir yaydır. ADR 0704'ün `fit`'i yayı son rıhtta bitirir (Revit'te
bitişin rıhtları belirlemesinin karşılığı). Alınmayanlar: merkez direği ve taşıyıcı, iç kenarda yönetmelik basamak
denetimi (yalnız bilgi olarak gösterilir), birden çok yaylı kol, yay–düz karışık kol (balıksırtı/dönen sahanlık), 3B.

## Karar

### 1. `StairFields.bulges`, KCAD şema 117

Merdivenin yeni isteğe bağlı alanı `bulges`: iki büküm; ilki yürüme çizgisinin yayınınki (dönüşün dörtte birinin
tanjantı, saat yönünün tersine artı, sıfır değil, sonlu), ikincisi 0. Yalnız iki noktalı yürüme çizgisinde; yürüme
çizgisinin yarıçapı genişliğin yarısından büyük olmalıdır (iç kenar merkezi geçmez). Bunlar sözleşmenin `problem()`'idir:
`invalid_stair`, yol `bulges`. KCAD şema 117 opt-in'dir: yazıcı 117'yi yalnız bir merdiven döner iken seçer, öteki
bütün dosyaların baytları aynıdır; 116'da `bulges` taşıyan merdiven `bad_value` olur (`stair-arcs.kcad`,
`broken/stair-arcs-in-schema-116.kcad`, `broken/schema-version-118.kcad`). Bağımsız `tools/kcad/kcad.py` aynı kuralı okur.

### 2. Çizim

Çekirdek `geom::arch`: `Walk::curved` yayın merkezini, yarıçapını ve dönüşünü tutar; noktası `merkez + r·(cos, sin)`,
doğrultusu yayın teğetidir. Döner merdivende dönüş, dolayısıyla sahanlık yoktur. Rıhtlar yürüme çizgisi boyunca basamak
aralığıyla, her biri yarıçap doğrultusunda genişliği boyunca (radyal); bant iki eş merkezli yaydır (`stair_band`,
büküm b, 0, −b, 0); kesme çizgisi, kesmenin üstünde kesikli rıhtlar ve basamak numaraları düz merdivenin kuralıyla;
ok yay boyunca; korkuluklar yanların 5 cm içinde eş merkezli yaylar, kesmede düz merdivenin kuralıyla bölünür
(`stair_railing_arcs`). Ayna yayı ters çevirir (bükümün işareti), döndürme ve taşıma korur.

### 3. `cad.stairs.create`

`NewStair.bulges` (yeni, isteğe bağlı). Sonsuz büküm `not_finite` (`stairs[i].bulges[k]`); yay kuralı `invalid_stair`.
`fit: true` döner merdivende yayı merkezinin çevresinde başlangıçtan `(rıht − 1) × basamak` kadar yürütür
(`stair_fit_arc`): yeni bitiş ve büküm yazılır; rıhtlar tam bir turdan uzunsa `invalid_stair` (`fit`). Uyarılar
(`stair_short`, `stair_long`, `stair_step_rule`, `stair_rise_level`) yayın uzunluğuyla aynıdır. Python `kentos.cad`
ve MCP aynı komut ve alanlardır. Yeni ret ya da uyarı kodu yoktur.

### 4. Araç: Döner merdiven

Mimari › Yerleştir'deki Merdiven ▾ bölünmüş düğmesinde (`tool.stairCurved`); Merdiven aracında da Biçim (B) Döner'e
geçer. Akış: merkeze tıklanır (yürüme çizgisinin çemberi ve iki yanı kesikli imleçle gelir), yürüme çizgisinin başına
tıklanır (merkeze uzaklığı yarıçap, çıkış buradan), imleç merkezin çevresinde merdivenin döneceği yana götürülüp
tıklanır. Dönüş imlecin her hareketinin kendi açısıyla birikir: yarım turu geçen imleç yönünü korur (300° sola, 60°
sağa değil). Yay rıhtlara göre (öntanımlı) son rıhtta biter; Tıklanan yerde seçilirse imlecin açısında biter ya da
dönüş projenin açı düzeninde yazılır (eksi saat yönünde). Önizleme komutun planıdır; etiket rıhtları, 2R+G'yi, yarıçapı,
dönüşü (sola/sağa) ve iç kenardaki basamağı (`basamak × (r − genişlik/2) / r`; bilgi, denetim değil) söyler; ret
kırmızı yazılır, hiçbir şey yazılmaz. Seçili döner merdivenin Öznitelikler'inde Biçim (Döner, 273° sola), Yarıçap ve
İç basamak görünür; sahanlık satırı yoktur. Ayarlar paneli Sahanlık'ı göstermez, Son kol'un yerini Yay alır.

### 5. Ayar panelinin seçim değerleri (aynı dilimde bulunan hata)

Ayar paneli bir seçimin yazılan değerini (`right`, `clicked`, `fit`) gönderir; Merdiven, Rampa ve Mutfak dizisi
bunu Türkçe büyük harfle (`RİGHT`, `CLİCKED`, `FİT`) karşılaştırıp reddediyordu: panelden Korkuluk: Sağ, Son kol:
Tıklanan yerde/Rıhtlara göre ve Yan: Sağ uygulanamıyordu. İki biçim de kabul edilir. `setting_choices` testi
mimari araçların panelde sunulan her seçimini uygular; düzeltme geri alınınca düşer.

## Sınırlar

- Merkez direği, taşıyıcı ve basamak bindirmesi çizilmez; iç kenardaki basamak yalnız bilgidir, yönetmelik denetimi
  değildir.
- Bir merdivende tek yay vardır; yay ile düz kolun birleştiği (dönen sahanlıklı ya da balıksırtı) merdiven ve iki
  yaylı kol açıktır.
- Kat görünümü (üst kattaki parça), 3B ve pafta kabulü ADR 0704'teki gibi açıktır.

## Doğrulama

- `python3 scripts/fixtures/arch_cases.py --check`: çekirdeğin 10 döner merdiven durumu (rıhtlar, radyal rıht
  çizgileri, bant ve bükümleri, uydurma, kesikli parçalar, korkuluk yayları; iki yön, yol uzun/kısa, bir turdan uzun,
  Blondel dışı) KentOS kodu olmadan; `cargo test -p kentos-geometry-core --test all arch`.
- `python3 scripts/fixtures/arch_command_cases.py --check`: yeni `cad.stairs.create.curved.json`'un 26 durumu
  (yükseklikten biten yay, iki yön, kesme ve korkuluk, 270°, uyarılar, kat, düz ile birlikte, eğik kiriş, ayna,
  9 ret) KentOS kodu olmadan; uydurulan bitiş çekirdeğin kayan nokta adımlarıyla ve bağımsız formülle 1e-9 içinde.
  `KENTOS_CASES_ONLY=cad.stairs cargo test -p kentos-native-application --test all every_command_case`; Python koşucusu;
  `cargo test -p kentos-mcp`.
- KCAD: `stair_arcs` testi, `schemas` kuralı, bağımsız `tools/kcad/kcad.py` (1251 dosya); Python taşıma testi.
- `cargo test -p kentos-interaction --test all spiral_stair` (imleçle dönen ve son rıhtta biten yay, tek geri alma;
  saat yönünde; yazılan 270° ve imleçle 300°; dar yarıçapın reddi; Merdiven'in Biçim'i) ve `setting_choices`.
- Gerçek GUI resimleri: `KENTOS_SHOTS_ONLY=doner-merdiven,doner-merdiven-yazildi,doner-merdiven-dar,doner-merdiven-serit
  cargo test -p kentos-desktop --bin kentos-cad tools_screens::screens -- --ignored --nocapture`;
  `.run/shots/arac-doner-merdiven-*`.
