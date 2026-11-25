# ADR 9229: Okunmayan DGN çizgi stilleri, çizimde neyi etkilediğiyle söylenir

- **Durum:** kabul edildi (2026-11-25). Plan: [plan/render.md](../../plan/render.md) RM-22, 3. adım.
- **İstek:**
  - planın RM-22 maddesi: “… vekil grafikler ele alınır”;
  - sahibin 23 Kasım isteği: “veri kaybolmasın”;
  - 25 Kasım: “Devam et, asla durma, bitene kadar tüm maddeler.”
- **Sözleşmeler:** değişmedi. Yalnız açılış raporunun bir satırı değişti.

## §1 Bulgu

SİVAS AVM DWG'sinin açılış raporu “DWG nesnesi · 486 531 · DGN çizgi stili bileşenleri ve vekil (proxy) nesneler
okunmadı” diyordu. Sayı çizimin bütün nesnelerine yakın. Rapor, çizilen bir şeyin görünüşünü yitirip yitirmediğini
söylemiyordu.

Kayıtlar türlerine göre sayıldı:

- **482 389 DGN çizgi stili bileşeni ve 4 142 tanım.** Vekil nesne yok.
- **Tanımlar 4 142 çizgi tipine ait:** “{ Diamond }”, “{ Cable Tele }”, “{ Tapered Dash }” gibi 9 stil. Dış
  referanslar defalarca bağlandıkça her bağlamada bir kopyası oluşmuş
  (`02-11-030_500-BLOK 9_(otopark)_r2$0$1+1A$0${ Diamond }` gibi).
- **Kim kullanıyor:** bu çizgi tipleriyle 32 923 nesne çizilmiş. Hepsi hiçbir yerden yerleştirilmeyen blok
  tanımlarında. Hiçbir katman bu tipleri kullanmıyor.
- **Sonuç:** AVM'de görünen bir kayıp yok, ama rapor bunu bilmenin yolunu vermiyordu.

**Vekil çizim nesneleri** (ACAD_PROXY_ENTITY): elimizdeki 10 DWG örneğinin hiçbirinde yok. Kod çözücü onları
grafikleriyle yazıyor; KentOS'un DXF okuyucusu bu türü tanımıyor, rapor “desteklenmeyen tür” der. Örnek dosya gelince
ele alınır.

## §2 Karar

Köprü, okunmayan nesneleri atmadan önce sayar (`crates/native/dwg/src/lib.rs`, `Unread`, `unread`):

- **DGN çizgi stili kayıtları** (tanımlar ve bileşenler) ve **vekil nesneler** ayrı sayılır.
- **Stilin çizgi tipi:** bir tanım, biçimlendirdiği çizgi tipinin uzantı sözlüğündedir; oradan çizgi tipi bulunur.
- **Kullanan nesneler:** çizgi tipini kendisi alan ya da BYLAYER ile katmanından alan nesneler sayılır, iki gruba
  ayrılır:
  - çizimin gösterdikleri: model uzayı, paftalar ve bunların herhangi bir derinlikte yerleştirdiği bloklar;
  - hiçbir yerden yerleştirilmeyen bloklardakiler.

**Rapor:**

- **Önce:** “DWG nesnesi · 486 531 · DGN çizgi stili bileşenleri ve vekil (proxy) nesneler okunmadı: KentOS onları
  çizmez; çizgiler çizgi tipinin kesik deseniyle çizilir.”
- **Sonra:** “DGN çizgi stili · 486 531 · 4142 çizgi tipinin DGN çizgi stili kayıtları okunmadı; çizimde bu çizgi
  tipleriyle çizilen nesne yok; 32923 nesne hiç yerleştirilmemiş bloklarda”.
- **Çizimde kullanan varsa:** “bu çizgi tipleriyle çizilen n nesne çizgi tipinin kesik deseniyle çizilir”.
- **Vekil nesneler** ayrı satırdadır: “Vekil nesne (ACAD_PROXY_OBJECT) · okunmadı: başka bir uygulamanın çizilmeyen
  verisi”.

**DGN çizgi stilinin kendisi** (simgeleri, sivrilen kesikleri) çizilmiyor. Bir çizimde bu çizgi tipleriyle çizilen
nesne çıkarsa, rapor artık onu sayısıyla söyler; o zaman ayrı bir iş olur.

## §3 Kanıt

**Sınama:** `kentos-dwg` `tests::dgn_line_styles_say_what_they_draw`. Kurulan çizimde:

- model uzayında çizgi tipiyle bir, katmanından bir nesne;
- hiç yerleştirilmeyen bir blokta bir nesne;
- sayım {2 kayıt, 1 çizgi tipi, 2 çizilen, 1 kullanılmayan}, notun tam metni;
- blok yerleştirilince çizilen 3, kullanılmayan 0;
- `drop_unread` saydığı kayıtları atar.

`kentos-dwg` bütün sınamalar (8 + 7); `kentos-native-application` alım sınamaları (14 + 5).

**Gerçek pencere:** `.run/gui-evidence/render/rm22/once-sonra-rapor.png`. Açılış raporunun satırı ve ayrıntısı, önce
ve sonra.

**Öbür 10 DWG örneğinde** (`~/Downloads`) DGN çizgi stili ve vekil nesne yok. Raporlarının “alınmadı” satırları
RM-22'nin sonraki adımlarına bulgu oldu:

- iki YILTAŞ dosyasında 130'ar okunamayan tarama sınırı;
- TEKNOKENT'te görüş doğrultusuna dik 28 çember ve 303 boş yazı;
- üç dosyada OLE nesnesi.

## Uygulama

- `crates/native/dwg/src/lib.rs`: `Unread` ve `said`, `unread` (çizgi tipleri, katmanlar, blokların erişilebilirliği);
  köprü sayar, rapor yazar; sınama.
- Aynı dosyanın yorumlarından başka program adları çıkarıldı.
