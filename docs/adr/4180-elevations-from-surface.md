# 4180 — Yüzeyden kot ver

Tarih: 2026-11-09. plan/map-cad.md MAP-06, sekizinci tur. Durum: taslak. ADR 1290'ın ve 3780'in “Sonra”sındaki
“yüzeyden kot alma”.

İlgili:
- [ADR 1290](1290-grade-between-vertices.md): Eğimle kot ver;
- [ADR 3780](3780-raise-lower-by-reference.md): Referanstan kot ver;
- [ADR 1520](1520-map-project-terrain-surfaces.md): Harita projesinin arazi yüzeyleri.

## Dayanak

- **Autodesk Civil 3D**,
  [To Set Feature Line Elevations From a Surface](https://help.autodesk.com/cloudhelp/2016/ENU/Civil3D-UserGuide/files/GUID-07357A77-3973-4D5D-AED9-8A2AB6327D25.htm)
  (2016; 9 Kasım 2026'da okundu):
  - Elevations From Surface'ta önce yüzey seçilir, sonra nesne (tek, çok ya da bir kısmı);
  - nesne altındaki yüzeyin kotlarını alır, her köşeye kot verilir;
  - “Insert Intermediate Grade Break Points” seçilirse nesnenin yüzeyin TIN çizgilerini kestiği yerlere ara kırık
    noktaları eklenir;
  - yüzeyden alınan kotlar dinamik değildir.
- **Autodesk Civil 3D**,
  [Set Elevations From Surface Dialog Box](https://help.autodesk.com/cloudhelp/2021/ENU/Civil3D-UserGuide/files/GUID-02FFD5F9-190D-484F-B12C-BF0428359526.htm)
  (2021; 9 Kasım 2026'da okundu):
  - seçenekler yüzey, ara kırık noktaları (TIN çizgisinin kesildiği her yere bir kot noktası) ve yüzeye göreli kottur;
  - göreli kot yüzeye bağlıdır, yüzey değişince güncellenir, yukarı ya da aşağı bir aralıkla verilebilir.
  - İki sayfa da yüzeyin dışındaki köşeyi söylemez.
- **KentOS bugün:** Eğimle kot ver ve Referanstan kot ver kotu bir köşeden, bir eğimden ya da bir referanstan verir.
  Harita ve Yol projelerinin arazi yüzeyleri (ADR 1520) kurulur ve sorgulanır, ama bir nesneye kotunu vermez.

## Karar

1. **`cad.entities.drape` v1** (yeni komut, “Yüzeyden kot ver”):
   - girdi: nesneler (`uids`, 1–10 000; nokta, çizgi, çoklu çizgi, alan), Harita ya da Yol projesinin yüzeyi
     (`surface`: kimliği ya da adı) ve `breaks`;
   - her köşe (çizginin iki ucu, çoklu çizginin ve alanın her parçası ve deliği) yüzeyin altındaki kotunu alır. Kot,
     yüzeyin kendi sorgusuyla (köşenin düştüğü üçgenin düzlemi) okunur; eski kotu değişir;
   - yüzeyin dışındaki köşe kendi kotunu korur. Belge bunu söylemez; KentOS yüzeyin olmadığı yere kot uydurmaz.
2. **Ara kırık noktaları** (`breaks`): düz bir kenarın yüzeyin üçgenlerinin kenarlarını uçları arasında kestiği her
   yere, kenar boyunca sırayla, yüzeyin kotuyla bir köşe eklenir. Ucuna 10⁻⁹ m'den yakın kesişim o uçtur; aynı yere
   düşen kesişimler (üçgen köşesinden geçen kenar) bir kez sayılır. Eklenen köşenin bükümü 0'dır.
   - Yay bölünmez: yayın TIN kenarlarıyla kesişimi doğrusal değildir; köşeleri kot alır.
   - Çizgi iki ucunu tutar: köşe eklenirse çoklu çizgi olurdu.
   - Ölçülü (M) ya da genişlikli çoklu çizgi bölünmez: ölçü ve genişlik köşe başınadır, eklenen köşeye
     uydurulamaz.
   - Bu nesneler cevapta `unsplit`'tir; nokta da öyle.
3. **Kotlar yüzeye bağlı kalmaz**, Civil 3D'deki gibi. Göreli kot ve aralık bu dilimde yoktur.
4. **Cevap:** her nesnenin kot alan, yüzey dışında kalan ve eklenen köşe sayısı, toplamlar ve yüzeyin kimliği. Plan
   aynı cevabı verir, yazmaz.
5. **Retler**, sırasıyla:
   - `invalid_revision`, `revision_conflict`;
   - `not_a_road_project` (Harita ya da Yol projesi değil), `unknown_terrain`;
   - `invalid_input` (nesne yok, 10 000'den çok, biri iki kez);
   - `invalid_uid`, `entity_not_found`;
   - `not_a_path` (köşesi olmayan nesne: yazı, daire …);
   - `terrain_unbuildable`;
   - yeni `off_surface`: nesnelerin hiçbir köşesi yüzeyde değil;
   - `layer_locked`.
6. **Geri alma:** tek adım, “Yüzeyden kot ver”.
7. **Masaüstü** (Eğimle kot ver, yeni Hedef **Yüzeyden**):
   - satırlar Yüzey (projenin yüzeyleri, ilki seçili) ve Ara kırık noktaları (“Üçgen kenarlarını kestiği yerlere köşe
     ekle”);
   - önizleme: “8 köşe yüzeyden kot alır; 4 ara kırık noktası eklenir.”;
   - yüzeyi olmayan projede neden söylenir, Uygula kapalıdır;
   - beş hedef yan yana sığsın diye pencere 680 piksel genişliğindedir.
8. **MCP:** komut köşelerin kotunu üzerine yazdığı için yıkıcı listesindedir.
9. **Kapsam işareti:** **CAD (Harita + Yol)**; komut Python ve MCP'de.

## Sınırlar

- Göreli kot (yüzeye bağlı, aralıklı) yok.
- Yayın TIN'le kesişimi bölünmez. Çizgi çoklu çizgiye çevrilmez.
- Nesnenin bir kısmına (Civil 3D'nin Partial'ı) değil bütününe uygulanır.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/drape_cases.py`; KentOS kodu yok), 21 durum:
  - dört köşeli yüzey: Delaunay köşegeni boş çember sınamasıyla burada seçilir (AC), iki düzlem ve kenar kesişimleri
    kesirlerle;
  - durumlar: köşeler ve yüzey dışı; ara kırık noktaları (çoklu çizgiye 1, alanın halkasına 2, deliğine 2; yay, nokta,
    çizgi ve ölçülü çizgi bölünmez); yüzey adıyla; kotlu çizginin kotları; kapalı çizginin kapanış kenarı; ikinci
    parça; yüzey dışındakiyle birlikte; plan; doğrulama ve on bir ret.

  Rust ve Python geçer; yetenekler testi her kodu sayar.
- **Python** (`python/tests/test_drape_from_surface.py`), testin kendi düzlemleriyle:
  - içerideki nokta üçgeninin düzleminin kotunu alır, dışarıdaki kendi kotunu korur;
  - çizgi AC'yi kestiği (45, 36)'ya AC'nin kotuyla köşe alır;
  - tek geri alma.
- **MCP** (`crates/native/mcp/tests/drape_from_surface.rs`): araç yıkıcı işaretli; nokta ABC düzleminin kotunu,
  çizgi AC'de köşesini alır; geri alma kotu kaldırır. MCP'nin ana testleri geçer.
- **Masaüstü** (`grade_surface_tests`, 3 test):
  - delikli alan ortak durumun halkalarını yazar, ileti doğrudur, tek geri alma;
  - ara kırık noktası olmadan yalnız kotlar değişir;
  - yüzeysiz projede önizleme yok.

  Eğimle kot ver testleri ve çapraz denetimler geçer. Resimler `.run/gui-evidence/yuzeyden-kot-4180/`.

## Sonra

- Göreli kot ve nesnenin bir kısmı: [ADR 4580](4580-drape-relative-height-and-range.md) (fark ve köşe aralığı); yüzeye
  bağlı (dinamik) kot açık.
- Çok parçalı nesne; dönen ve yaylı referansın bağı (MAP-06'nın kalanı).
- Web (TODOS §0.2).
