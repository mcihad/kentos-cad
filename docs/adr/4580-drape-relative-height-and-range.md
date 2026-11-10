# 4580 — Yüzeyden kot ver'de yüzeye göreli kot ve köşe aralığı

Tarih: 2026-11-10. plan/map-cad.md MAP-06, dokuzuncu tur. Durum: taslak. ADR 4180'in “Sonra”sındaki “Göreli kot (yüzeye
bağlı, aralıklı); nesnenin bir kısmı”.

İlgili:
- [ADR 4180](4180-elevations-from-surface.md): Yüzeyden kot ver (`cad.entities.drape`);
- [ADR 1290](1290-grade-between-vertices.md) ve [ADR 2180](2180-grade-range-picked-on-the-drawing.md): Eğimle kot
  ver'in köşe aralığı ve sahneden seçilmesi;
- [ADR 2980](2980-grade-range-marked-on-the-drawing.md): aralığın çizimde işareti.

## Dayanak

- **Autodesk Civil 3D 2024**, [To Set Feature Line Elevations From a Surface](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-07357A77-3973-4D5D-AED9-8A2AB6327D25.htm)
  (10 Kasım 2026'da okundu):
  - yüzey seçilir; ara kırık noktaları seçilirse nesnenin üçgen çizgilerini kestiği yerlere kot noktası konur;
  - özellik çizgisinin yüzeye göreli kalması için “Relative Elevation to Surface” işaretlenir ve isteğe bağlı olarak
    artı ya da eksi bir fark girilir; bu işaretliyken yüzeyden alınan kotlar dinamiktir;
  - “Partial” ile nesnenin kot verilecek bir kısmı tanımlanır.
- **Autodesk Civil 3D 2023**, [About Relative Feature Lines](https://help.autodesk.com/cloudhelp/2023/ENU/Civil3D-UserGuide/files/GUID-62AEE70C-34BA-4B1D-8BD0-E94E4A8C4FF0.htm)
  (10 Kasım 2026'da okundu):
  - özellik çizgisinin kotları yüzeyden alınabilir, yüzeye göreli de olabilir;
  - göreli oluşturulan çizgi yüzey değişince kendiliğinden güncellenir, kot farklarını korur;
  - yüzey güncellenince kot noktaları üçgen çizgilerine göre eklenmez, silinmez;
  - sonradan göreli yapılan çizginin kotları yüzeye göre yeniden kurulmaz;
  - yüzeylerin birbirinin çizgisine dayanması döngüsel bağımlılık sorunu doğurur.
- **KentOS bugün** (ADR 4180): her köşe yüzeyin kotunu tam alır. Bir fark verilemez (bordür üstü zeminin 15 cm
  üstündedir), nesnenin yalnız bir kısmına kot verilemez.

## Karar

1. **`cad.entities.drape`** (sürüm aynı, ek alanlar):
   - `offset` (m): alınan her kot (köşe ve ara kırık noktası) yüzeyin kotu artı bu farktır; eksi yüzeyin altıdır.
     Yüzeyin dışındaki köşe kendi kotunu korur, fark eklenmez;
   - `range` (`from`, `to`; 0'dan, `from` < `to`): tek nesnenin bu köşeleri kot alır ve sayılır. Çizgide uçlar, çoklu
     çizgide ve alanda dış halka (ilk parçanınki) kullanılır;
   - aralıkla ara kırık noktası yalnız aradaki kenarlara girer; kapanış kenarı aralıkta değildir. Delikler ve öbür
     parçalar olduğu gibi kalır.
2. **Kotlar yüzeye bağlı kalmaz** (ADR 4180 gibi). Civil 3D'nin dinamik göreli çizgisi bu dilimde yoktur: yüzey
   değişince kotu yeniden vermek kalıcı bir bağ ister (Sonra).
3. **Retler** (yerleri ADR 4180'in sırasında):
   - `invalid_input`: sonlu olmayan fark (`offset`: “Yüzeyle kot farkı sonlu bir sayı olmalı (m).”); birden çok nesneye
     aralık (`range`);
   - `not_a_path`: noktaya aralık (`range`);
   - yeni sırada `wrong_place`: köşelerin dışında ya da başı sonundan önce olmayan aralık (`range`; Eğimle kot ver'in
     sözüyle);
   - `off_surface`: aralığın hiçbir köşesi yüzeyde değil.
4. **Masaüstü** (Eğimle kot ver › Hedef Yüzeyden):
   - **Kot farkı (m)** kutusu: “Yüzeyin kotuna eklenir: artı yüzeyin üstü, eksi altı.”;
   - **Aralık** anahtarı: “Yalnız başlangıç ile bitiş köşesi arası”. Açılınca Başlangıç ve Bitiş köşesi (Sahneden seç
     ile) gelir; aralık çizimde Eğimle kot ver'inki gibi işaretlenir (ADR 2980);
   - özet farkı söyler: “… yüzeyin kotunu artı 0.150 m alır”.
5. **Kapsam işareti:** ADR 4180 gibi **CAD (Harita + Yol)**.

## Sınırlar

- Göreli kot dinamik değildir: yüzey değişince nesne yeniden kot almaz.
- Aralık köşe numarasıyladır; köşe arası bir yerden başlamaz (Civil 3D'nin Partial'ı herhangi iki nokta alabilir).
- Aralık dış halkaya verilir; delik ya da ikinci parça seçilemez.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/drape_cases.py`; KentOS kodu yok, kesirlerle), 13 yeni durum (34):
  - 0,15 m fark (ara kırık noktaları dahil; yüzey dışındakiler farksız);
  - eksi 0,5 m;
  - çoklu çizgide 0–1 aralığı (köşegeni kesen kenar bölünür, son köşe kotsuz) ve 1–2 aralığı (bölünmez, ilk köşe
    kotsuz);
  - farkla alanın 0–2 aralığı (kapanış kenarı ve delik olduğu gibi);
  - iki parçalı çizginin ilk parçası; plan;
  - altı ret.

  Eski 21 durum aynı. Rust ve Python geçer. Bozma denemesi: ara kırık noktası fark almayınca koşucu yakalar.
- **Python** (`python/tests/test_drape_relative_range.py`), testin kendi düzlemleriyle: noktanın üstte ve altta kotu;
  aralığın köşeleri ve (45, 36)'daki kırık noktası farkla, son köşe kotsuz; kesen kenar aralık dışındayken kırık
  noktası yok; alanın aralığında delik ve kapanış kenarı olduğu gibi; dört ret.
- **MCP** (`crates/native/mcp/tests/drape_relative_range.rs`): noktanın ABC düzleminin 0,15 m üstü; çizginin 0–1
  aralığı iki köşe ve bir kırık noktası, son köşe kotsuz; iki nesneye aralık `range`'de reddedilir.
- **Masaüstü** (`grade_surface_relative_tests`): Kot farkı 0,15, Aralık 1–2 ve ara kırık noktalarıyla önizleme 2 köşe,
  1 kırık noktası; aralık çizimde işaretli; Uygula köşeleri yüzeyin 0,15 m üstüne yazar, son köşe kotsuz; tek geri
  alma. Eğimle kot ver testleri ve çapraz denetimler geçer. Resimler `.run/gui-evidence/yuzeye-goreli-kot-4580/`.

## Sonra

- Yüzeye bağlı (dinamik) göreli kot: yüzey değişince kotların farkla yeniden verilmesi, kalıcı bağ ve şema.
- Çok parçalı nesnede aralığın parçası; dönen ve yaylı referansın bağı (MAP-06'nın kalanı).
- Web (TODOS §0.2).
