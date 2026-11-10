# 3960 — Boru hattı denetiminde et kalınlığı: örtü dış tepeden, açıklık dıştan dışa

Tarih: 2026-11-09. plan/map-cad.md MAP-16, yedinci tur. Durum: taslak. ADR 2360'ın “Sonra”sındaki “Et kalınlığı:
malzeme ve dış çap özniteliği”, ADR 2760 ve 3160'ın “kesişen borunun dış duvarı ve et kalınlığı”nın ölçü kısmı.

İlgili:
- [ADR 1640](1640-gravity-pipe-check.md): boru hattı denetimi (örtü tepeden, iç çapla);
- [ADR 2360](2360-pipe-clearance-to-another-network.md): başka ağa açıklıklar (iç çaplarla).

## Dayanak

- **Civil 3D 2021, Part Properties Tab (Pipe Properties Dialog Box)**
  ([belge](https://help.autodesk.com/cloudhelp/2021/ENU/Civil3D-UserGuide/files/GUID-A9581C87-F561-4475-8B36-2D4D3B9C97A0.htm),
  9 Kasım 2026'da okundu): Minimum Cover ve Maximum Cover borunun bütün boyunca, “borunun dış tepesinden başvuru
  yüzeyine” ölçülür. Sayfa uçları ayrıca anmaz.
- **Civil 3D 2020, About the Cover And Gradient Pipe Rule**
  ([belge](https://help.autodesk.com/cloudhelp/2020/ENG/Civil3D-UserGuide/files/GUID-39F96537-BBF1-4DAF-9CF0-8DC469C089F1.htm),
  9 Kasım 2026'da okundu):
  - en çok örtü yalnız denetimdir, parçayı oynatmaz; aşılınca kural ihlali olur;
  - yüzey seçilmemişse yüzey kotu 0 sayılır (KentOS'ta yüzeysiz örtü ölçülmez, ADR 1640).
- **KentOS bugün** (ADR 1640, 2360): çaplar iç çaptır, et bilinmediği için katılmaz. Örtü iç tepeden (akar + iç çap),
  açıklıklar iç yüzeylerden ölçülüyordu; ikisi de duvar kadar fazla çıkıyordu.
- Et kalınlığının değeri (malzeme, sınıf) borunundur. KentOS değer vermez, özniteliği okur.

## Karar

1. **`diameter.wall`** (`cad.pipes.check`, komut sürümü değişmez): her borunun et kalınlığının özniteliği, çapın
   biriminde, 0 ya da büyük. Diğer ağın çapında da (`clearance.diameter.wall`) verilebilir.
2. **Örtü:** et sorulunca borunun dış tepesinden ölçülür: akar + iç çap + et (Civil 3D'nin tanımı). Yüzeyden boru boyunca
   her üçgen kenarı kesişiminde (ADR 1640), ya da bacaların kapak kotundan uçlarda. Kot birimi önerisi de dış tepeyle
   denetlenir.
3. **Açıklıklar** (ADR 2360): dıştan dışa ölçülür.
   - Düşey açıklık: üstteki borunun dış altı (akar − et) eksi alttakinin dış tepesi (akar + iç çap + et). Hangisinin
     üstte olduğu eskisi gibi iç boşluklarının ortasıyla bulunur.
   - Yatay açıklık: eksenler arası en kısa uzaklık eksi iki yarıçap ve iki et.
4. **Et kalınlığı olmayan boru:** et sorulmuş ama borunun özniteliği yoksa (ya da sayı değil, 0'dan küçükse) yeni bulgu
   `noWall` (“Et kalınlığı yok”, çapın bulgularından sonra) söylenir. Bu borunun örtüsü iç tepeden, açıklığı etsiz
   ölçülür. Diğer ağın etsiz borusu etsiz sayılır; bulgu denetlenen ağın borularınındır.
5. **Cevap:** `coverTo` et sorulunca `outside`, çapla `crown`, yoksa `invert`. Her borunun satırında okunan et (`wall`,
   metre).
6. **Ret:** boş et özniteliği (`invalid_pipe_check`, `diameter.wall` ya da `clearance.diameter.wall`): “Et kalınlığının
   özniteliği boş; özniteliğin adını yazın.”
7. **Masaüstü** (Boru hattı denetimi):
   - Çap satırında ve Diğer ağın çapı satırında **Et** kutusu;
   - pencere açılınca ve diğer ağ seçilince borularda “Et kalınlığı” ya da “Et” özniteliği varsa o yazılır;
   - özet örtünün ve açıklığın neye göre ölçüldüğünü söyler.
8. Profil çiziminde kesişen borunun dış duvarı (ADR 2760) bu ADR'de çizilmez (Sonra).
9. **Kapsam işareti:** ADR 1640'taki gibi **Harita + Yol + CBS**.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/pipes_check_cases.py`; mpmath, KentOS kodu yok):
  - kendi çiziminde borular `Et` taşır (mm; P7'de yok);
  - örtü dış tepeden, düşey ve yatay açıklık dıştan dışa elle;
  - beş yeni durum (49): et kalınlığıyla örtü ve `noWall`, et sorulmadan aynı çizimde iç tepe, dıştan dışa açıklıklar,
    iki ret;
  - eski 44 durum ve ortak çizim bayt bayt aynı; hat profilinin durumları değişmedi;
  - Rust ve Python geçer.
- **Python** (`python/tests/test_pipe_walls.py`), testin kendi hesabıyla:
  - her etli borunun en az ve en çok örtüsü iç tepedekinden tam etin kadar az, yeri aynı;
  - P7 `noWall` ile söylenir, örtüsü değişmez;
  - her kesişimin ve yakın geçişin açıklığı iki borunun etleri toplamı kadar az.
- **MCP** (`crates/native/mcp/tests/pipe_walls.rs`): dıştan dışa açıklık durumunun etleri, örtüleri, bulguları ve
  açıklıkları; boş özniteliğin reddi.
- **Masaüstü** (`pipes_check_window_tests::the_walls_are_read_from_the_pipes_and_the_answers_are_the_shared_cases`):
  - Et iki ağda da borulardan bulunur;
  - cevaplar durumlarınki; Et boşaltılınca iç tepe.

  Pencerenin öbür testleri ve çapraz denetimler geçer. Resimler `.run/gui-evidence/et-kalinligi-3960/`.

## Sonra

- Hat profilinde kesişen borunun dış duvarı ve taraması (ADR 2760 Sonra): profil komutu et kalınlığını okumalı.
  [ADR 4360](4360-crossing-pipe-outer-wall-and-live-check.md)'te.
- Et yerine dış çap özniteliği (bazı kataloglar dış çapla yazar). [ADR 4780](4780-pipe-outer-diameter-own-walls-and-sewer-rules.md)'de.
- “Yüzey kaplaması” (planın satırı): anlamı belirsiz, kaynak beklenir.
- Boru hattı denetiminin açık pencerede ağ değişince yeniden okunması (ADR 3560 Sonra; ADR 4360'ta).
- Web (TODOS §0.2).
