# 4360 — Hat profilinde kesişen borunun dış duvarı; denetimin açık pencerede yeniden okunması

Tarih: 2026-11-10. plan/map-cad.md MAP-16, sekizinci tur. Durum: taslak. ADR 3960'ın “Sonra”sındaki “hat profilinde
kesişen borunun dış duvarı ve taraması (profil komutu et kalınlığını okumalı)” ve ADR 3560'ın “Boru hattı
denetimi'nin kendi sonucunun ağ değişince kendiliğinden yeniden okunması (pencere açıkken)”.

İlgili:
- [ADR 2760](2760-crossing-pipes-in-the-pipe-profile.md): hat profilinde kesişen borular (iç duvar elipsi);
- [ADR 3960](3960-pipe-wall-thickness.md): et kalınlığı (`diameter.wall`), örtü ve açıklıklar dıştan;
- [ADR 0328](0328-hatch-source-boundary-and-face-ties.md): taramanın kesin sınırı (elips yayları).

## Dayanak

- **Autodesk, Civil 3D 2015 kılavuzu, Profile Tab (Pipe Style Dialog Box)**
  ([belge](https://help.autodesk.com/cloudhelp/2015/ENU/Civil3D-UserGuide/files/GUID-9E09CD1C-60C1-4A42-8B58-A6283400272E.htm),
  10 Kasım 2026'da okundu):
  - **Crossing Pipe Hatch Options**, kesişen borunun hangi kısmının taranacağı: iç duvarlara kadar, dış duvarlara kadar,
    ya da yalnız iç ve dış duvar arasındaki duvar alanı (Hatch Walls Only);
  - **Pipe Wall Sizes:** iç ve dış duvar ya borunun gerçek boyutlarıyla (Use Part Dimensions) ya da kullanıcının
    değerleriyle çizilir.
- **KentOS bugün:**
  - Kesişen boru akarından yukarı iç çap elipsidir (ADR 2760).
  - Et kalınlığı denetimde okunur, profilde okunmaz (ADR 3960 §8).
  - Profilin girdisindeki kesişen ağın çap sorusu denetiminkiyle aynı türdür (`PipeDiameter`, `wall` alanıyla); profil
    bu alanı yok sayıyordu.
  - Boru hattı denetimi sonucunu açılışta ve form değişince okur. Pencere açıkken ağ değişirse (geri al, betik,
    buluttan gelen değişiklik) eski sonuç kalıyordu.

## Karar

1. **`cad.pipes.profile`'ın `crossing.diameter.wall`'u okunur** (komut sürümü değişmez). Kesişen borunun cevabındaki
   `wall`: okunan et (metre), sorulmuş ve borunun özniteliği varsa (0 ya da büyük). Denetimin kuralıyla okunur
   (çapın biriminde). Özniteliği olmayan borunun `wall`'u yazılmaz, dış duvarı çizilmez.
2. **Çizim** (Civil 3D'nin “Use Part Dimensions” ve “Hatch Walls Only”'si):
   - akar kotu ve çapı bilinen, eti 0'dan büyük her kesişen borunun iç elipsinin çevresinde aynı merkezli bir dış elips:
     yatayda çap + iki et, düşeyde (çap + iki et) × abartı;
   - iki elips arası dolu tarama. Sınırı kesin: dış elips, sonra delik olarak iç elips; her biri tek noktalı bir
     döngüde tam elips (ADR 0328);
   - yazı dış duvarın tepesinin yarım yazı yüksekliği üstünde;
   - dış duvarın altı (akar − et) ve tepesi (akar + çap + et) gösterilen kotlara katılır; kıyas kotu ve çerçeve onları
     da içine alır.
3. **Masaüstü** (Boru hattı denetimi): Diğer ağın Et kutusu doluysa (ADR 3960 onu borulardan bulur) profil dış duvarları
   çizer. İleti çizilen dış duvarların sayısını söyler: “…, 3 kesişen borunun dış duvarı; …”.
4. **Denetimin yeniden okunması:** pencere açıkken çizim değişince (her mesajdan sonra `follow_document`) sonuç ağdan
   yeniden okunur. Betik ya da geri al ile değişen bir akar kotu, sonraki mesajda tabloda görünür.
5. **Kapsam işareti:** ADR 2760'taki gibi **Harita + Yol + CBS**.

## Sınırlar

- Hattın kendi borularının dış duvarı profilde çizilmez (Civil 3D'nin boru biçimindeki iç ve dış duvarı); tepe çizgisi
  iç tepedir.
- Taramanın türü dolu ve sabittir; Civil 3D'nin üç tarama seçeneğinden yalnız duvar alanı.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/pipes_profile_cases.py`; mpmath, KentOS kodu yok), 5 yeni durum (52):
  - et kalınlığıyla (W1 ve W3 5 mm, W2 8 mm; W7 kotsuz, yalnız listede);
  - abartısız (iki çember);
  - plan;
  - yağmursuyunun yaylı Q3'ünde W4;
  - et sorulmuş, borularda yok.

  Nesne sayısı ve kotlar dış duvarla. Eski 47 durum değişmedi (yalnız ekleme). `road_project_cases.py --check` güncel.
  Rust ve Python geçer.
- **Python** (`python/tests/test_pipe_profile_walls.py`), testin kendi geometrisiyle:
  - her kesişen boruda aynı merkezli iki elips, yarı eksenleri d/2 ve d/2 + w (yatay), abartıyla düşey;
  - iki döngülü dolu tarama;
  - yazı dış tepenin üstünde.
- **MCP** (`crates/native/mcp/tests/pipe_profile_walls.rs`): iki durumun etleri, nesne sayısı, kıyas ve tepe kotu.
  Hat profilinin ve et kalınlığının MCP testleri geçer.
- **Masaüstü** (`pipes_profile_walls_tests`):
  - pencereden profil: altı elips (üç iç, üç dış), dış elips etin kadar geniş; üç dolu halka; ileti;
  - Et boşken dış duvar yok;
  - pencere açıkken betik P1'in alt akarını indirince sonraki mesajda eğimi büyür.

  Boru hattı denetimi testleri ve çapraz denetimler geçer. Resimler `.run/gui-evidence/dis-duvar-4360/` (profil, W2
  yakın).

## Sonra

- Hattın kendi borularının dış duvarı ve taraması; taramanın türünün seçilmesi.
- Et yerine dış çap özniteliği (ADR 3960 Sonra).
- “Yüzey kaplaması”nın anlamı (kaynak beklenir); EK-1 değerleri (sahibin PDF'i); EPANET/SWMM.
- Web (TODOS §0.2).
