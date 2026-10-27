# ADR 0710: Duvarın geçici ölçüsü ve kaydırılması

- **Durum:** kabul edildi (27 Ekim 2026); ARCH-EDIT-01'in duvar dilimi.
- **Bağlam:** `plan/architecture.md` ARCH-EDIT-01; ADR 0701 (açıklığın geçici ölçüsü), ADR 0340 (duvar birleşimleri ve
  düzenlemeleri), ADR 0341 (açıklık duvarıyla gider), ADR 0700 (duvar kenarının yüzü).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2'ye satırı yazıldı.

## Bağlam

Mimar odayı çoğunlukla net ölçüyle kurar ve düzeltir: “salon 4,20 olsun”, “banyo 1,80'e insin”. KentOS'ta bir duvarın karşı
duvara net mesafesini görmek için ölçü koymak, değiştirmek için de duvarı `Taşı` ile kaydırıp köşedeki ve T ile bağlı
duvarları tek tek uzatıp kısaltmak gerekiyordu; açıklık için (ADR 0701) olan doğrudan düzenleme duvarda yoktu.

Araştırma (üretici belgeleri; yerelde çalıştırılmadı):

- [Revit 2023 — About Temporary Dimensions](https://help.autodesk.com/cloudhelp/2023/ENU/RevitLT-DocumentPresent/files/GUID-74FFB00B-FB29-44B7-8FEA-613DBE010AD4.htm):
  eleman seçilince en yakın dik/paralel elemanlara geçici ölçüler görünür; sayı değiştirilince seçili eleman taşınır.
- [Autodesk Learn — Wall layout, design, and editing with Revit](https://www.autodesk.com/learn/ondemand/curated/wall-layout-design-and-editing-with-revit/aeN9Ju7PDgSABryW6ofnn):
  duvar seçilip geçici ölçünün sayısına tıklanır, değer yazılır, duvar kayar; ona bağlı duvarlar birleşimlerini korur.

KentOS kararı: Revit'in “seç, sayıya bas, yaz” akışı duvarda da; referans **paralel en yakın duvarın bakan yüzü**,
mesafe **net** (yüzden yüze) ve kenara diktir. Değer yazılınca yalnız seçili duvarın o kenarı kendine dik kayar; karşıdaki
duvar sabit kalır. Revit'teki gibi bağlı duvarlar izler: köşedeki duvarın ucu, T ile kenara saplanan duvarların uçları ve
çok kenarlı duvarın bitişik köşeleri. Açıklıklar yerinde kalır. Ölçü nesnesi yaratılmaz; birleşimler (ADR 0340) her zamanki
gibi duvarlardan çizilir. Alınmayanlar: referansın elle başka elemana çekilmesi, kilitli (sabit) referans, duvarın boyunun
geçici ölçüsü ve ucunun taşınması, yay kenar, kolona ve aksa mesafe.

## Karar

### 1. Ortak hesap: `ops::wall_shift`

- **Aralıklar:** bir duvarın düz kenarının her yanında, aynı kattaki (ya da ikisi de katsız) duvarların düz kenarlarından
  (duvarın kendi öbür kenarları da) ona 0,001° içinde paralel olup kenar boyunca 1e-9 m'den çok örtüşenlerin en yakını;
  mesafe kenarın o yandaki yüzünden (hizaya göre: sol yüz `öteleme + w/2` ortada, `öteleme` solda, `öteleme + w` sağda; sağ
  yüz ondan genişlik kadar) öbürünün kenara bakan yüzüne diktir; ölçünün yeri örtüşmenin ortası. Eşit mesafede çizimde önce
  gelen duvar, sonra küçük kenar.
- **Kaydır:** kenarın doğrusu `by` kadar soluna (eksi: sağına) kayar. İki kenar arasındaki köşe öbür kenar boyunca kayar
  (yay ya da aynı doğrultudaki bitişik kenar, ya da oradaki başka duvar ucu reddedilir). Yol ucu: birleşimi `none` değilse,
  ucu oraya (1 mm) gelen tek açık duvarın son kenarı boyunca kayar ve o uç da aynı noktaya gider (köşe); iki uç tek doğruda
  buluşuyorsa o doğru boyunca kayar (onlar yerinde); birden çok başka duvar reddedilir; hiçbiri yoksa ucun içinde durduğu
  duvarın kenarı boyunca (T: o duvar yerinde), o da yoksa kenara dik. Ucu aynı doğrultuda (1°) duvara uç uca bağlı duvar
  reddedilir. Kenara T ile saplanan (ucu kenarın gövdesinde, uçlarından uzak) açık duvarların uçları kendi son kenarları
  boyunca kayar. Değişen her kenar 1 mm'den uzun kalır ve yönünü korur; yoksa reddedilir, hiçbir şey kaymaz.
- **Mesafe:** sol yanda `aralık − mesafe`, sağ yanda `mesafe − aralık` kadar Kaydır.

### 2. Komutlar

- **`cad.walls.edit`'in yeni işlemleri:** `shift { edge?, by }` ve `distance { edge?, side, distance }` (edge yoksa 0). Tek
  duvar (`invalid_wall`, `uids`); sonsuz değer `not_finite`; eksi mesafe, olmayan kenar, yay, paralel duvarı olmayan yan ve
  §1'in retleri `invalid_wall`; sıfır kaydırma `nothing_to_set`; izleyen duvar kilitli katmandaysa (adlandırılan duvarların
  kilidinden sonra) `layer_locked`. Değişen duvarlar tek adım “Duvarı düzenle”; çıktının `walls`'ı kaydırılan duvar,
  yeni `joined`'ı izleyenler (kaydırılanın başındaki, sonundaki, sonra çizimin sırasıyla). Açıklıklar ADR 0341 §3'teki gibi
  pervazlarından yeni yola okunur: başı kayan duvardaki açıklığın `at`'ı değişir, yeri değişmez; planın izleyen önizlemesi
  de bunu gösterir.
- **Salt okunur `cad.walls.distances` v1:** duvarların bir kenarındaki iki yan aralığı: mesafe, referans duvarın kimliği,
  kenarı ve yüzü, ölçünün iki ucu. Python ve MCP aracı katalogdan; ajan önce mesafeyi okur, sonra `distance` ile yazar.

### 3. Masaüstü

Tek duvar seçiliyken (komut çalışmazken) imlecin en yakın olduğu düz kenarın iki yanındaki net mesafeler, aralığı dik geçen
ölçülerle görünür; karşı yüz işaretlidir. Sayıya basmak sayının yerinde değeri hazır bir yazı kutusu açar (ADR 0060); Enter
`cad.walls.edit` `distance` ile yazar (tek adım, izleyen duvarlar söylenir), Esc ve okunamayan değer hiçbir şey yazmaz.
Açıklığın geçici ölçüsüyle aynı seçim katmanındadır.

## Sınırlar

- Referans kullanıcı tarafından değiştirilemez ve kilitlenemez; duvarın boyu, kolona/aksa mesafe ve yay kenar açıktır.
- Köşede ikiden çok duvar, yay uçlu bağlı duvar ve uç uca aynı doğrultudaki duvar kaydırılmaz (söylenir); bunlar birlikte
  taşınmalı ya da önce ayrılmalıdır.
- UCS'e göre ölçü yoktur: mesafe kenara diktir, eksenlerden bağımsızdır.

## Doğrulama

- `python3 scripts/fixtures/wall_shift_cases.py --check`: `cad.walls.distances.json`'un 22 ve `cad.walls.edit.shift.json`'un
  32 durumu (köşe ve T izleme, kapalı dikdörtgenin kenarı, tek doğrudaki iki duvar, L duvar, kat ve `none`, sola hizalı
  duvarın yüzleri, eşit mesafe, örtüşmeyen ve 0,01° eğik duvar, açıklıkların `at`'ı ve pervazları, plan, doğrula, 13 ret)
  KentOS kodu olmadan; birleşmeyenlerin uyarıları `wall_join_cases.py`'nin kesirli çözücüsünden.
  `KENTOS_CASES_ONLY=cad.walls cargo test -p kentos-native-application --test all every_command_case`; Python koşucusu;
  `cargo test -p kentos-mcp`.
- `cargo test -p kentos-interaction --test all wall_distance` (seçili duvarın aralığı, sayıya basıp yazınca köşelerin
  izlemesi ve tek geri alma, Esc ve okunamayan değer, çok kenarlı duvarda imlecin kenarı).
- Gerçek GUI resimleri: `KENTOS_SHOTS_ONLY=duvar-mesafe,duvar-mesafe-yazi-kutusu,duvar-mesafe-kaydirildi,duvar-mesafe-oda,duvar-mesafe-oda-genisledi
  cargo test -p kentos-desktop --bin kentos-cad tools_screens::screens -- --ignored --nocapture`; `.run/shots/arac-duvar-mesafe-*`.
