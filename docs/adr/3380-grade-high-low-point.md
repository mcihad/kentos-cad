# 3380 — Eğimle kot ver'de yüksek/alçak nokta

Tarih: 2026-11-09. plan/map-cad.md MAP-06, altıncı tur. Durum: taslak. ADR 1290'ın ve ADR 2980'in “Sonra”sındaki
“yüksek/alçak nokta ekleme”.

İlgili:
- [ADR 1290](1290-grade-between-vertices.md): Eğimle kot ver (`cad.entities.grade`);
- [ADR 2980](2980-grade-range-marked-on-the-drawing.md): aralığın çizimde işareti;
- [ADR 0279](0279-grip-menus-and-the-vertex-command.md): Köşe ekle (yayı bölen köşe, çizginin çoklu çizgiye dönmesi);
- [ADR 0337](0337-constraints-in-edits-copies-and-jobs.md): öbür komutların yazdığında kısıtlar.

## Dayanak

- **Civil 3D, Insert High/Low Elevation Point**
  ([To Insert a High or Low Level Point on a Feature Line](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-3BE8349C-02B7-472A-87D6-D7A7FE46D0F0.htm),
  9 Kasım 2026'da okundu):
  - özellik çizgisinde başlangıç ve bitiş noktası seçilir; aralık birden çok kenarı geçebilir;
  - başlangıçtan ileri eğim girilir (artı yukarı, eksi aşağı), sonra bitişten geri eğim;
  - değerler bir kesişme vermezse “No solution found” denir.

  Belge noktanın yatay yerini, kotunu ve aradaki noktaların ne olacağını söylemez.
- **Civil 3D, Grading Elevation Editor**
  ([Grading Elevation Editor Dialog Box](https://help.autodesk.com/cloudhelp/2023/ENU/Civil3D-UserGuide/files/GUID-CD59A8C3-6A59-4655-9CDB-75ECA6C0D31B.htm),
  9 Kasım 2026'da okundu): Grade Back kenarın sonundan başına doğru eğimidir. Geri eğimin yönü buradan alındı: bitişten
  başlangıca doğru, artı yukarı.
- **KentOS bugün** (ADR 1290): Eğimle kot ver aralığa tek eğim verir. Bir hendeğin ya da bordürün iki ucundan suyu
  ortadaki bir noktaya akıtmak (alçak nokta) ya da iki yana dağıtmak (yüksek nokta) için kesişmenin yeri elle
  hesaplanıp Köşe ekle ile köşe konuyor, sonra iki aralık ayrı ayrı eğimleniyordu.

## Karar

1. **`cad.entities.grade`'in `ahead` ve `back`'i** (ek alanlar; komut sürümü 1 kalır):
   - `grade` yerine verilir, ikisi birlikte;
   - `ahead` başlangıçtan aralığın içine, `back` bitişten başlangıca doğru eğimdir (düşey bölü yatay, artı yukarı);
   - bitiş kotu `end`, verilmezse bitiş köşesinin kendi kotu; başlangıç eskisi gibi `start` ya da köşenin kotu.
2. **Yer:** başlangıçtan iki doğrunun kesiştiği yere yol boyunca yatay uzaklık
   s = (bitiş − başlangıç + back·L) / (ahead + back).
   - s aralığın içinde olmalıdır (0 < s < L); başlangıç ve bitiş sayılmaz.
   - s'nin düştüğü kenarda kenar boyunun t kesri alınır. t kenarın bir ucuna 10⁻⁹'dan yakınsa nokta o köşedir ve köşe
     eklenmez. Bu eşik Köşe ekle'nin “mevcut köşeye çok yakın” kuralıdır (`ops::vertex`).
3. **Eklenen köşe:**
   - düz kenarda kirişin t noktası;
   - yaylı kenarda yayın açısının t'si. İki yarı aynı çemberde kalır: bulge'ları tan(tθ/4) ve tan((1 − t)θ/4)'tür;
   - çizgi, Köşe ekle'deki gibi üç köşeli çoklu çizgi olur.
4. **Kotlar:**
   - noktaya kadar köşeler başlangıç + ahead·d alır;
   - sonrakiler bitiş + back·(L − d) alır;
   - noktanın kotu başlangıç + ahead·s'dir;
   - aralığın dışındaki köşeler kotlarını korur.

   Belgede aradakilerin ne olduğu yazmıyor. KentOS onları iki kola göre eğimler: Set Grade/Slope Between Points
   aradaki noktaların kotunu da yazar (ADR 1290), iki eğim de aralığın tamamında geçerli olur.
5. **Türü** (`kind`):
   - `high`: iki eğim de noktaya doğru yükselir;
   - `low`: ikisi de alçalır;
   - `break`: ikisi de değil, bir eğim öbürüne döner.
6. **Taşınanlar:**
   - eklenen köşenin ölçüsü (M) ve çoklu çizginin genişlikleri, Düzenle'nin taşıdığı ortak kurallarla (kenarın üstünde,
     kenar boyunca; ADR 0224, 0258);
   - nesnenin sıralı parçalarını anan kısıtlar, yazıştan sonra ortak kuralla gider (ADR 0337 §1);
   - tek geri alma adımı “Eğimle kot ver”.
7. **Retler:**
   - `grade` ile `ahead`/`back` birlikte, ya da ikisinden biri yalnız: `invalid_input`;
   - bitiş verilmemiş ve bitiş köşesinin kotu yok: `elevation_missing` (`end`);
   - eğimler paralel (ahead = −back), ya da kesişme aralığın dışında, başlangıçta ya da bitişte: `no_intersection`.
     İleti kesişmenin yerini ve aralığın boyunu söyler: “kesişme yol boyunca başlangıçtan −129.434 m'de, aralık 0 ile
     127.956 m arası”.
8. **Cevap** (ek alanlar):
   - `grade` ileri eğimdir, `back` geri eğim;
   - `zs` eklenen köşe dahil aralığın kotlarıdır;
   - `point`: köşe sırası (yazıştan sonra, 0'dan), eklenip eklenmediği, başlangıçtan uzaklığı, yeri, kotu, türü.
9. **Masaüstü** (Eğimle kot ver):
   - “Eğim yerine bitiş kotunu ver” anahtarı yerine **Hedef**: Eğim, Bitiş kotu, Yüksek/alçak nokta;
   - Yüksek/alçak noktada Bitiş kotu, Başlangıçtan eğim (%) ve Bitişten eğim (%) kutuları ve yönü söyleyen yardım satırı;
   - önizleme noktayı söyler: “Yüksek nokta: 2. ile 3. köşe arasına yeni köşe (Y …, X …), başlangıçtan 50.591 m, kot
     852.024 m; 5 köşenin kotu değişecek.”;
   - nokta çizimde aralığın uçlarıyla birlikte halkalanır;
   - kabul iletisi noktayı ve yerini söyler.
10. **Kapsam işareti:** ADR 1290'daki gibi **CAD (Harita + Yol)**; komut Python ve MCP'de.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/grade_cases.py`; yay merkezi, döndürülen nokta ve yarıların bulge'ı betikte,
  KentOS kodu yok):
  - 14 yeni durum (35 durum). Koordinatlar 10⁻⁶ m içinde: aynı nokta 4 400 000 m'de iki ayrı hesapla bulunuyor.
  - Eklenen yeni köşeler:
    - düz kenarda yüksek nokta (geri alma ile);
    - yaylı kenarda yüksek nokta;
    - alçak nokta (plan);
    - eğim değişimi;
    - çizgi çoklu çizgi olur;
    - alanın dış halkası.
  - Ara köşeye düşen kesişme: köşe eklenmez.
  - Retler:
    - `grade` ile birlikte, `ahead` yalnız, `back` yalnız;
    - bitiş köşesi kotsuz;
    - paralel eğimler;
    - kesişme aralığın dışında;
    - kesişme bitişte.
  - Eski durumlar aynı. Rust ve Python geçer.
- **Çekirdek** (`ops::grade::tests`): köşeye düşen kesişme, düz kenarda alçak nokta, yarım dairede iki yarının çemberi
  ve bulge'ı, retler.
- **Python** (`python/tests/test_grade_peak.py`): SDK'nın `ahead`/`back`'i; yeni köşe, kotlar ve geri alma, yayın
  yarıları, köşeye düşen kesişme, düz eğimde `point`in `cad.UNSET` olması, paralel eğimde `no_intersection`.
- **MCP** (`crates/native/mcp/tests/grade_peak.rs`): yaylı kenardaki durumun planı durumun cevabı; yazılıp kaydedilen
  dosyada çoklu çizginin köşeleri, bulge'ları ve kotları durumunki.
- **Masaüstü** (`grade_window_tests::a_high_point_previews_ringed_and_adds_a_vertex`):
  - önizleme durumun noktasıdır ve halkalanır;
  - Uygula köşeyi ekler, tek geri alma kaldırır;
  - paralel eğimde ileti, Uygula kapalı;
  - resimler `.run/gui-evidence/yuksek-nokta-3380/` (açık geniş, koyu dar, Uygula'dan sonra).

  Pencerenin öbür testleri ve çapraz denetimler geçer.

## Sonra

- ~~Eğimle yükselt/alçalt (Raise/Lower by Reference)~~ (ADR 3780), yüzeyden kot alma (ADR 1290 Sonra).
- Noktanın çizimde kotuyla yazılması (etiket).
- Çok parçalı nesne; dönen ve yaylı referansın bağı (MAP-06'nın kalanı).
- Web (TODOS §0.2).
