# 3780 — Referanstan kot ver (Raise/Lower by Reference)

Tarih: 2026-11-09. plan/map-cad.md MAP-06, yedinci tur. Durum: taslak. ADR 1290'ın, 2980'in ve 3380'in “Sonra”sındaki
“eğimle yükselt/alçalt (Raise/Lower by Reference)”.

İlgili:
- [ADR 0142](0142-vertex-elevation.md): köşe kotları, Kot ver (Sabit, Artır, Sıfırla);
- [ADR 1290](1290-grade-between-vertices.md): Eğimle kot ver (`cad.entities.grade`);
- [ADR 2180](2180-grade-range-picked-on-the-drawing.md): aralığın sahneden seçilmesi;
- [ADR 3380](3380-grade-high-low-point.md): yüksek/alçak nokta, Hedef satırı.

## Dayanak

- **Civil 3D, Raise/Lower by Reference**
  ([To Raise or Lower Feature Line Level by Reference](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-89B13DF8-1005-433E-A8F7-AAFA42A2857C.htm),
  9 Kasım 2026'da okundu):
  - önce referans noktası seçilir: çizimde herhangi bir yer (nesne kenetiyle), bir COGO noktası ya da yüzey kotu;
    komut satırı referansın kotunu söyler;
  - sonra yükseltilecek özellik çizgisi ve onun üstünde bir nokta seçilir; komut noktanın referansa uzaklığını, kotunu
    ve eğimini söyler;
  - eğim (grade), oran (slope) ya da kot farkı (difference) girilir, değer referansla nokta arasına uygulanır; komut
    noktanın aldığı kotu söyler;
  - referans etkin kalır: başka bir özellik çizgisi seçilip aynı referansa göre kot verilebilir.

  Belge şunları söylemez: değerin işaretinin yönü, çizginin öbür noktalarının ne olacağı.
- **Civil 3D, Raise/Lower**
  ([To Raise or Lower Feature Lines](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-D2C13F0E-8774-4561-9536-0A0A8F66CFC6.htm),
  9 Kasım 2026'da okundu): seçili özellik çizgilerinin kotu girilen bir kot farkıyla yükseltilir ya da alçaltılır.
  [Grading Command Reference](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-4D4405BF-D71C-4A35-A0ED-82D8434571D2.htm)
  (aynı gün okundu) ikisini ayrı komut olarak sayar: RaiseLowerFeatures ve RaiseLowerFeaturesByRef.
- **KentOS bugün:**
  - Kot ver'in Artır'ı nesnenin bütün kotlarını yazılan farkla kaydırır (ADR 0142); farkın kendisi elle hesaplanıyordu;
  - bir bordürü kapının eşiğinden %2 aşağıda bitirmek için köşenin eşiğe uzaklığı ölçülüyor, istenen kot ve fark elle
    bulunup Artır'a yazılıyordu.

## Karar

1. **`cad.entities.raise` v1** (yeni komut, “Referanstan kot ver”):
   - girdi: nesne (çizgi, çoklu çizgi, alan), köşe `at` (0'dan; çizginin uçları, çoklu çizginin ilk parçası, alanın dış
     halkası), referans (`x`, `y`, `z`) ve `grade` ya da `difference`, ikisinden biri;
   - köşenin yeni kotu referansın kotu artı eğim × köşenin referansa yatay uzaklığı (düz doğru, √(Δx² + Δy²)), ya da artı
     kot farkı;
   - **yön:** eğim referanstan köşeye doğrudur, artı yukarı; kot farkı köşenin kotu eksi referansınkidir. Belge işareti
     söylemiyor; KentOS ADR 3380'in “artı yukarı” kuralını sürdürür.
2. **Bütün nesne kayar:** köşenin farkı (yeni − eski) nesnenin öbür bütün kotlarına eklenir: alanın delikleri, her
   parçanın halkası ve delikleri. Nesnenin kendi eğimleri değişmez.
   - Belge öbür noktaları söylemiyor. Komutun adı ve Raise/Lower'ın tanımı (seçili çizgilerin kotu bir farkla) KentOS'a
     bütünün kaymasını verir.
   - Seçilen köşe hesaplanan kotun kendisini alır, toplamdan gelen yuvarlama artığı yoktur.
   - Kotsuz köşeler kotsuz kalır (Kot ver'in Artır'ı gibi).
3. **Cevap:** yatay uzaklık, köşenin önceki ve yeni kotu, fark (`by`), referanstan köşeye eğim (köşe referansın tam
   üstündeyse yok), kayan kot ve kotsuz köşe sayısı. Plan aynı cevabı verir, hiçbir şey yazmaz.
4. **Retler** (sırasıyla): `invalid_revision`, `revision_conflict`, `invalid_uid`, `entity_not_found`, `not_a_path`
   (nokta da: kotunu Kot ver yazar), `wrong_place` (köşe yok), `invalid_input` (eğim ile kot farkı birlikte ya da
   hiçbiri; sonlu olmayan sonuç), `elevation_missing` (köşenin kotu yok: fark bulunamaz), `layer_locked`.
5. **Geri alma:** tek adım “Referanstan kot ver”.
6. **Masaüstü** (Eğimle kot ver penceresi, yeni Hedef **Referanstan**; şerit yeri Çizgi ve Alan sekmelerinin Köşe
   paneli, Eğimle kot):
   - Hedef pencerenin ilk satırına çıktı: yapılacak iş altındaki satırları belirler;
   - satırlar: Referans Y, X ve Sahneden seç; Referans kotu; Köşe (1'den); Fark (Eğim | Kot farkı); Eğim (%) ya da Kot
     farkı (m);
   - **Sahneden seç iki tıktır:** önce referans (tıklanan yerin yakınındaki kotlu nokta ya da köşe, seçme açıklığı
     içinde, uzamsal dizinden), sonra köşe. Kotlu bir şeyin yanına tıklanmazsa yer alınır, kotun yazılacağı söylenir;
   - önizleme: “Referanstan 104.403 m; 4. köşe 851.250 m → 851.838 m (%2.000); 4 kot +0.588 m kayacak, 1 kotsuz köşe
     kotsuz kalır.”;
   - referans ve köşe çizimde halkalanır, aralarına çizgi çekilir;
   - Uygula'dan sonra referans bir sonraki nesne için kalır (Civil 3D'deki gibi), Vazgeç onu bırakır;
   - oran (yatay:düşey) pencerede yok, eğim yüzdeyle yazılır (Eğimle kot ver'deki gibi).
7. **MCP:** komut var olanı değiştirdiği için yıkıcı listesindedir. Eğimle kot ver de köşelerin kotunu üzerine
   yazdığı hâlde listede değildi; ikisi birlikte eklendi.
8. **Manifest:** Eğimle kot ver aracının `via`'sı iki komuttur. Manifestin Python adı artık SDK'nınki gibi ayrılmış
   sözcüğe alt çizgi ekler (`entities.raise_`, `entities.break_`, `forms.import_`); önceki satırlar bu yüzden düzeldi.
9. **Kapsam işareti:** ADR 1290'daki gibi **CAD (Harita + Yol)**; komut Python ve MCP'de.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/raise_cases.py`; uzaklık, yeni kot ve kayan kotlar elle, KentOS kodu yok):
  - 27 durum:
    - eğimle (artı, eksi, sıfır) ve kot farkıyla;
    - çizginin iki ucu;
    - iki parçalı, delikli alan (kotsuz delik kotsuz kalır);
    - iki parçalı çoklu çizgi;
    - referansın üstündeki köşe (eğim yok);
    - plan ve doğrulama;
    - retler: iki ya da hiç değer, sonlu olmayan sonuç, köşe dışı, kotsuz köşe, hiç kotu olmayan çizgi, yazı, nokta,
      kilitli katman, olmayan nesne, UUID, sürüm, çatışma;
  - Rust ve Python geçer; yetenek testi geçer.
- **Çekirdek** (`entities_raise::tests`): bütün kotlar aynı farkla kayar, köşe sonucun kendisini alır; delikteki ve
  kotsuz köşelerin sayısı.
- **Python** (`python/tests/test_raise_by_reference.py`, beklentiler testin kendi aritmetiğiyle): bordürün dördüncü
  köşesi eşikten %2; delikli ve iki parçalı alan kot farkıyla; referansın üstünde eğim `cad.UNSET`; kotsuz köşe
  `elevation_missing`, hiçbir şey yazılmaz.
- **MCP** (`crates/native/mcp/tests/raise_by_reference.rs`): araç yıkıcı diye listelenir; alanın planı durumun cevabı;
  yazılıp kaydedilen dosyada dış halkanın, deliğin ve öbür parçanın kotları durumunki. MCP'nin genel testleri geçer.
- **Masaüstü** (`grade_reference_tests`):
  - önizleme durumun cevabı, Uygula tek adım, geri alma;
  - iki tık: referans çizginin kotlu ucundan, sonra köşe; kotsuz yere tıklama söylenir;
  - referans bir sonraki nesne için kalır, Vazgeç bırakır;
  - kotsuz köşe söylenir, Uygula kapalı;
  - halkalar ve çizgi.

  Eğimle kot ver'in öbür testleri ve çapraz denetimler geçer. Resimler `.run/gui-evidence/referanstan-kot-3780/`
  (pencere açık geniş, koyu dar; Uygula'dan sonra Koordinat listesi).

## Sonra

- Referansın yüzeyden kotu (Civil 3D'deki gibi yüzey kotu); ~~yüzeyden kot alma (ADR 1290 Sonra)~~:
  [ADR 4180](4180-elevations-from-surface.md).
- Köşe yerine çizginin üstünde herhangi bir nokta (bugün köşe; kenarın içi için önce Köşe ekle).
- Noktanın kotu referanstan (bugün Kot ver).
- Çok parçalı nesne; dönen ve yaylı referansın bağı (MAP-06'nın kalanı).
- Web (TODOS §0.2).
