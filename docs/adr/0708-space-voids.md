# ADR 0708: Mahal boşluğu — şaft ve döşeme boşluğu

- **Durum:** kabul edildi (27 Ekim 2026); ARCH-ELEM-01'in ilk dilimi.
- **Bağlam:** `plan/architecture.md` ARCH-ELEM-01; ADR 0342 (mahal ve metraj), ADR 0679 (mahal ayırıcı), ADR 0703
  (mimari denetim), ADR 0265 (yeni şema kuralı).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2'ye satırı yazıldı.

## Bağlam

Konut planlarının neredeyse hepsinde tesisat şaftı, asansör boşluğu ya da merdiven/galeri boşluğu vardır: planda
çapraz çizgiyle gösterilir, döşemesi yoktur, alanı oda alanlarına katılmaz. KentOS'ta şaft duvarlarla çevrilince
Mahal aracı onu bir oda gibi buluyor, Metraj alanını oda toplamına katıyor, denetim de “çok küçük/dar mahal” diyordu;
çaprazı elle çizgiyle çizmek gerekiyordu.

Araştırma (üretici belgeleri; yerelde çalıştırılmadı):

- [Revit LT 2024 — Cut a Shaft Opening](https://help.autodesk.com/cloudhelp/2024/ENU/RevitLT-ArchDes/files/GUID-FA0CF7B6-D3E1-4F58-AE02-A19457BBCD02.htm):
  şaft açıklığı planda çizilerek (çizgi ya da duvar seçerek) kurulur; isteğe bağlı sembolik çizgiler (çapraz) eklenir,
  açıklık alt ve üst kısıt arasındaki her katın döşemesini keser ve sembolik çizgileri o katlarda görünür.
- [Archicad 29 — Area List Add-on](https://help.graphisoft.com/AC/29/INT/_AC29_Help/930_ExtraTopics/930_ExtraTopics-1.htm):
  net alan bölgenin ölçülen net alanıdır; bölgenin içindeki elemanların düşülmesi ayrı bir ayardır (Subtract from
  Zones), kendiliğinden değildir.

KentOS kararı: Revit'in “çaprazla gösterilen döşeme boşluğu” anlamı ve Archicad'in bölgeyle çalışan alan hesabı
birleştirilir: boşluk ayrı bir alan motoru ya da dosya türü değil, mahalin (ADR 0342) bir işaretidir; duvarlardan
bulunur ya da çizilir, mahal gibi numaralanır, etiketlenir, metrajda ayrı toplanır. Alınmayanlar: katlar arası şaft
(her katta ayrı boşluk), döşemenin kendisi (kalınlık, kot), boşluğun içinde kalan odanın alanından kendiliğinden
düşülmesi (duvarı olmayan boşluk Mahal ayırıcı ile ayrılır), boşluğun kat görünümleri.

## Karar

### 1. `SpaceFields.void`, KCAD şema 116

Mahalin yeni isteğe bağlı alanı `void` (yalnız `true` yazılır): mahal bir boşluktur. `cad.spaces.create`'in yeni
mahali (`NewSpace.void`) ve `cad.spaces.edit`'in `set`'i (`SpaceChange.void`; `false` yeniden oda yapar) yazar;
birleştirmede ilk mahalinki kalır. KCAD şema 116 opt-in'dir: yazıcı 116'yı yalnız bir mahal boşlukken seçer,
öteki bütün dosyaların baytları aynıdır; 115'te `void` `unknown_field` olur (`space-voids.kcad`,
`broken/space-voids-in-schema-115.kcad`, `broken/schema-version-117.kcad`).

### 2. Çizim: çapraz

Çekirdek `geom::space::void_cross`: her parçanın en uzun kenarı doğrultusundaki çevreleyen dikdörtgeninin iki
köşegeni, parçanın içinde kalan kısımlarıyla (delikler dışarıda). Dikdörtgen şaftta köşeden köşeye uzanır; L
boşlukta kırpılır. Mahalin parçalarına (`pieces`) eklenir: ekranda, paftada ve DXF'in mimari bloklarında aynı çizgiler.
Mahal aracının önizlemesi de çaprazı gösterir.

### 3. Metraj ve denetim

`cad.spaces.quantities`'in satırı `void` taşır, kuralına “boşluk: alanı mahal toplamına katılmaz” eklenir;
`totals.spaceArea` boşlukları dışarıda bırakır, yeni `totals.voidArea` onları toplar (sıfırsa yazılmaz).
Metraj penceresinde boşluk varken “Boşluk” sütunu ve özet satırında “boşluklar ayrı” görünür; CSV/XLSX aynı
sütunu taşır. `cad.arch.audit`'in küçük/dar mahal bulgusu boşluğa verilmez (kopukluk bulgusu verilir).

### 4. Araç ve Öznitelikler

Mahal aracında Boşluk (V) açılıp kapanır, ayarlar panelinde “Boşluk (şaft)”; önizleme etiketi “Boşluk · alan —
alanı mahal toplamına katılmaz” der. Seçili mahalin Öznitelikler'inde Boşluk satırı (Evet/Hayır) `cad.spaces.edit`
ile yazar.

## Sınırlar

- Katlar arası şaft tek nesne değildir; her katın boşluğu ayrı mahaldir.
- Duvarı olmayan bir boşluk bir odanın içine çizilirse odanın alanı kendiliğinden düşmez; Mahal ayırıcı ile ayrılır.
- Mahal çizelgesi (ADR 0344) boşluğu satır olarak gösterir; çizelgenin toplam satırının boşluğu ayırması açıktır.
- Döşemenin kendisi (kontur, kalınlık, kot), balkon ve korkuluk/parapet ARCH-ELEM-01'in sonraki dilimleridir.

## Doğrulama

- `python3 scripts/fixtures/space_command_cases.py --check`: yeni durumlar (çizilmiş şaft, duvarlarla çevrili
  asansör boşluğu ile oda, `void: false`, odayı boşluk yapıp geri almak, boşlukla birleştirme) metrajın ayrı
  toplamıyla; `scripts/fixtures/arch_audit_cases.py --check`: boşluk işaretli şaft küçük/dar bulgusu vermez.
  `KENTOS_CASES_ONLY=cad.spaces,cad.arch.audit cargo test -p kentos-native-application --test all every_command_case`;
  Python koşucusu.
- KCAD: `space_voids` testi, `schemas` kuralı, bağımsız `tools/kcad/kcad.py` (1249 dosya); Python taşıma testi.
- `cargo test -p kentos-geometry-core --test all a_void_is_crossed` (dikdörtgen, L, delik);
  `cargo test -p kentos-interaction --test all space_void` (Boşluk ile çizilen şaft, önizlemede çapraz, metrajın
  ayrı toplamı).
- Gerçek GUI resimleri: `KENTOS_SHOTS_ONLY=saft-arac,saft-yazildi,saft-metraj,saft-oznitelikler cargo test -p
  kentos-desktop --bin kentos-cad tools_screens::screens -- --ignored --nocapture`; `.run/shots/arac-saft-*`.
