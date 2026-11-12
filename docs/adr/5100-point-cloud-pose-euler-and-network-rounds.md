# 5100 — Bulutun konumunun dosyasız nesnede tutulması, Euler açılarıyla giriş ve tarama ağında dış döngü

Tarih: 2026-11-11. plan/map-cad.md MAP-13, onuncu tur. Durum: taslak. ADR 4720'nin “Sonra”sı: dönüşümün dosyasız
nesnede tutulması; Euler açılarıyla giriş (sırası artık kaynaklı); ağda dış döngü ve kontrol noktaları.

İlgili:
- [ADR 3100](3100-apply-a-transform-to-a-point-cloud.md), [ADR 3500](3500-point-cloud-inverse-and-history.md),
  [ADR 3900](3900-point-cloud-undo-last-transform.md), [ADR 4300](4300-point-cloud-axis-angle.md): Dönüşüm uygula
  (matris, ters, geçmiş, son adımı geri alma, eksen ve açı);
- [ADR 1800](1800-scan-registration-check.md): tarama kaydı, tarifi p ↦ R(p − c) + c + t;
- [ADR 4720](4720-scan-network-adjustment.md): tarama ağı dengelemesi (tek tur).

## Dayanak

Araştırma notları: `.run/research/map-cad/uygulama/MAP-13.10.md` (bu tur; resimler `MAP-13.10-resim/`),
`R11-MAP-13-euler-acilari.md`, `MAP-13.9.md`; mevzuat `.run/research/map-cad/mevzuat/R11-MAP-13.md`, `MAP-13.9.md`.

- **OGC GeoPose 1.0** (OGC 21-056r11, 08.09.2023; https://docs.ogc.org/is/21-056r11/21-056r11.html, 11 Kasım 2026'da
  bakıldı; resim `02-ogc-geopose-basic-ypr-A13.jpg`). Requirement 13 `/req/basic-ypr/angles` ve Conformance test A.13:
  iç çerçeve yalnız dönmedir; Doğu-Kuzey-Yukarı (ENU: X doğu, Y kuzey, Z yukarı) çerçeve **dönmüş yerel eksenler z, y ve
  x çevresinde, bu sırayla**, art arda üç kez döndürülür (alışılmış yaw, pitch, roll); birim derece, açılar işaretli
  gerçek sayı. D.24: yerel teğet sistem sağ el kuralıyla kurulmuş Öklid sistemidir.
- **CloudCompare'in kaynağı** (`libs/qCC_db/include/ccGLMatrixTpl.h`, master, satır 565–609, 11 Kasım 2026; resim
  `01-cloudcompare-initFromParameters.jpg`): `initFromParameters(phi, theta, psi, t3D)`, belge yorumunda “Tait-Bryan
  Z1Y2X3”. Sütunlar: R11 = c2·c1, R21 = c2·s1, R31 = −s2; R12 = s3·s2·c1 − c3·s1, R22 = s3·s2·s1 + c3·c1, R32 = s3·c2;
  R13 = c3·s2·c1 + s3·s1, R23 = c3·s2·s1 − s3·c1, R33 = c3·c2 (c1, s1 φ'nin; c2, s2 θ'nın; c3, s3 ψ'nin kosinüs ve
  sinüsü); dördüncü sütun öteleme, M·X = R·X + T. Bu, R = Rz(φ)·Ry(θ)·Rx(ψ)'dir.
- **İkisi aynı matristir:** iç z → y′ → x″ sırası sabit eksenlerde x, y, z sırasıyla aynıdır ve R = Rz(yaw)·Ry(pitch)·
  Rx(roll) verir. GeoPose'un ENU'su KentOS'un kod eksenleriyle (x doğu, y kuzey, z yukarı) aynıdır; arayüzün Türk
  ölçmecilik sırası (Y sağa, X yukarı) koordinat yazımıdır, eksenlerin anlamını değiştirmez.
- **Türk resmî metinleri** (mevzuat notu R11-MAP-13): TKGM yersel LİDAR esasları D-6 dönüşüm parametrelerinin teslimini
  ister ama açıların sırasını yazmaz; BÖHHBÜY m. 6 “üç eksendeki dönüklükler”; TUCBS_OG `omega/fi/kappa`
  `DonuklukAcisi` sırasız. Resmî bir Türk Euler sözleşmesi yok; KentOS GeoPose'u izler, fotogrametrinin ω-φ-κ'sını ve
  sırası yazılı olmayan roll-pitch-yaw kiplerini açmaz.
- **Taramanın konumu dosyanın yanında:** E57 her taramanın (data3D) `pose`'unu birim kuaterniyon ve ötelemeyle tutar,
  noktalar taramanın kendi koordinatında kalır; RIEGL RiSCAN PRO tarama başına SOP 4 × 4'ünü, PTX başlığı tarama başına
  4 × 4'ü, FARO SCENE kayıt raporu tarama başına konum + eksen + açıyı yazar (MAP-13.9, R11 notları). Konum
  **dosya (tarama) başınadır**.
- **Dış döngü:** Pulli 1999 (çok görüşlü kayıt: çift kayıtlar kısıt olur, çoklu görüş adımı dağıtır, yeniden
  eşleştirmeyle yinelenir); Leica Cyclone REGISTER 360'ın demeti bağ oluştukça yeniden en iyilemesi (“Always
  bundle-optimize upon link creation”). Bu turda ölçülen: tek turda dengelenen B taraması nesnede konumlanıp ağ yeniden
  çözülünce hâlâ 1,3 mm ve 2,3·10⁻⁴ rad düzeltme alıyordu (ilk turun 361 mm'sinden sonra).
- **Kontrol noktaları** (mevzuat MAP-13.9): oturum başına en az iki kontrol noktası, BÖHHBÜY m. 46(4) ±15 cm; TBC'nin
  Georeference'ı ve Leica'nın “Snap Bundle to Applied Control”u kontrolü demete uygular. Bu turda yapılmadı (Sonra).

## Karar

1. **Konum dosya başına, sözleşmede** (`CloudSource.pose`, `.kcad` şema 251): dosyanın noktalarını çizime götüren katı
   dönüşüm, satır satır 16 sayı (sol üst 3 × 3 dönme, dördüncü sütun öteleme, son satır 0 0 0 1); yoksa dosyanın
   kendi yeri. Kural (`pose_problem`): sayılar sonlu, son satır 0 0 0 1, RᵀR = I 10⁻⁹ içinde, determinant artı; ölçek,
   kayma ya da yansıma “uzunlukları değiştirir: Dönüşüm uygula ile dosyaya yazın” diye reddedilir. Bulutun `bounds`'u
   dosyaların konumdaki kutularının (başlık kutusunun sekiz köşesi taşınarak) birleşimidir; dosyanın `bounds`'u
   başlığınki kalır. Şema 251'i yazıcı bir dosyanın konumu varken seçer; 248 ve öncesi alanı `unknown_field` sayar. (249
   ve 250 başka işlerin; birleştirmede numaralanır; bilinmeyen sürüm örneği `schema-version-252.kcad`.)
2. **`cad.pointclouds.place` v1** (yeni yazan komut, tek geri alma adımı “Bulutu konumla”; `cad.entities.edit`'in
   `pointCloudPlace` işlemiyle yazar): `uid`, `place` = `matrix` (16 sayı) | `axisAngle` (ADR 4300'ün eksen ve açısı) |
   `euler` (`yaw` Z çevresinde, `pitch` dönmüş Y çevresinde, `roll` iki kez dönmüş X çevresinde, derece, her biri sağ el
   kuralıyla) | `remove`; `center` (yoksa bulutun çizimdeki kutusunun ortası), `translation` (yoksa 0). Harita her
   dosyanın konumundan **sonra** uygulanır (`Affine::then`, sabit sıra: (i, j) = Σₖ sonra(i, k)·önce(k, j)); Tarama
   kaydı'nın ve Tarama ağı'nın matrisi böylece taramayı çizimde gösterildiği yerden taşır. `remove` konumları kaldırır,
   cevabın matrisi ilk dosyanın konumunun tersidir; konumu olmayan bulutta hiçbir şey yazılmaz. Cevap: alınan matris,
   her dosyanın sonraki konumu, kutu, kutunun ortasının kayması, dönme açısı, `changed`. Retler: `invalid_uid`,
   `entity_not_found`, `not_a_pointcloud`, `layer_locked` (`uid`); `not_finite`, `invalid_input` (`place`);
   `invalid_revision`, `revision_conflict`.
3. **Euler açılarının matrisi** (`Affine::euler`, ortak çekirdek): CloudCompare'in sütunlarıyla aynı çarpımlar,
   soldan sağa (c1, s1 sapmanın; c2, s2 yunuslamanın; c3, s3 yatışın): satırlar [c2·c1, s3·s2·c1 − c3·s1,
   c3·s2·c1 + s3·s1], [c2·s1, s3·s2·s1 + c3·c1, c3·s2·s1 − s3·c1], [−s2, s3·c2, c3·c2]; merkez ve öteleme eksen ve
   açınınki gibi (p ↦ R(p − c) + c + t). Arayüz adları: **Sapma** (Z), **Yunuslama** (Y), **Yatış** (X); pencerede
   sıra, eksenler ve sağ el kuralı yazılır.
4. **Dönüşüm uygula** (`pointcloud.transform`): Dönüşüm seçimine **Euler açıları** (Sapma (Z), Yunuslama (Y), Yatış (X);
   merkez ve öteleme eksen ve açınınkiler). Konumu nesnede tutulan bulut çizimde gösterildiği yerden taşınır: önce
   konumu, sonra matris dosyaya yazılır (`konum.then(matris)`), geçmişe (Dönüşümler) ikisi de yazılır, yazılan bulutun
   konumu yoktur. Dosyaları farklı konumlu sanal bulut ve konumlu bulutta Son dönüşümü geri al reddedilir.
5. **Okuyanlar konumu izler:** komutların ortak okuyucusu (`pointcloud_areas::Scan`: bölge, profil, uyum, kayıt, ağ,
   yol arazisi) ve `cad.pointclouds.query` dosyanın noktasını konumundan geçirir, dosya ve COPC düğümü kutusunu konumda
   (sekiz köşe) sınar; masaüstünün nokta bulutu hizmeti konumlu dosyanın düğümlerini çizimdeki yerinde çözer (merkez
   taşınır, merkezden uzaklıklar döner; yüksekliğe göre görünüş oradaki kotla boyar); çizici konumlu dosyanın
   sekizli ağacından düğümleri görünümün dosyaya geri götürülmüş kutusuyla seçer; XYZ sor konumlu dosyanın noktasını
   orada okur. Dosyanın kendi noktalarını okuyan öbür İşlemler araçları (Seyrelt, Zemin süzgeci, Sınıfla, Kırp, Karola,
   Rasterleştir, Sınır çıkar, Birleştir, alan istatistiği) konumlu bulutu nedeni ve çözümüyle reddeder.
6. **Masaüstü:** Nokta bulutu sekmesinde Tarama ağı'nın yanında **Bulutu konumla** (pencere “Bulutu konumla”, yeni ikon
   `cloudPlace`, seçenek kâğıdı `.run/gui-evidence/ikon-bulutu-konumla/`): Bulut, Dönüşüm (Matris | Eksen ve açı | Euler
   açıları), alanlar, Merkez, Öteleme; özet bulutun şimdiki yerini ve haritanın ne yaptığını (komutun cevabı, yazmadan)
   söyler; tablo dosya başına dönme ve ortanın kayması; Konumla ve Konumu kaldır. Tarama ağı'na **Nesnede tut**: seçilen
   taramanın matrisi aynı komutla yazılır, ağ oradan yeniden çözülür. Öznitelikler'de **Konum** satırı.
7. **Tarama ağında dış döngü** (`cad.pointclouds.network`'e `rounds`, 1–10; verilmezse 1, eskisi gibi): ikinci turdan
   başlayarak her bağ, taramalar o ana dek turların haritalarından geçirilmiş olarak (`place`'in tutacağı gibi, hiçbir
   şey yazılmadan) yeniden kaydedilir; bağlar ve ağın merkezi ilk turda bir kez belirlenir. Bir turun haritası taramanın
   o ana dekine eklenir (`then`). Bir taramanın kutusunun hiçbir köşesi kaydın durma toleransı (`tolerance`, öntanım
   0,1 mm) kadar kaymayınca ya da tur sayısı dolunca durulur. Cevaba `rounds`: her turun en büyük köşe kayması;
   taramaların matrisi turların toplamı, dönmesi (Rz·Ry·Rx'ten geri okunan ω, φ, κ) ve ötelemesi bu toplamın merkez
   çevresindeki; bağlar, artıklar, döngüler ve yatay güç son turunkidir; not turları söyler. Retler: tur sayısı 1–10
   dışında. Tarama ağı penceresine **Dış döngü (tur)**.
8. **Kapsam işareti:** **Harita + CBS** (nokta bulutu bağlam sekmesi), ADR 1800'deki gibi.

## Sınırlar

- Konum katıdır (dönme + öteleme); ölçekli ya da kaymalı bir matris yalnız Dönüşüm uygula ile dosyaya yazılır.
- Planda bulutun çerçevesi konumdaki kutunun eksenlere paralel kutusudur (dönmüş dikdörtgen değil).
- Görünüş penceresinin önerdiği kot aralığı dosyanın kök düğümünden, dosyanın kotlarıyla okunur (konumun düşey ötelemesi
  öneriye girmez).
- XYZ sor konumlu dosyada düğümleri, eğikliğin kaydırdığı kadar büyütülmüş bir erimle arar (küçük dönmelerde dar).
- Dış döngü ağın kendi sabit noktasına yakınsar: üç taramada turların kayması 374 mm, 5,2 mm, 0,84 mm, 0,30 mm, 0,31 mm;
  son turlar verinin gürültüsü içinde (vokseller taramayla kayar) 0,1 mm'nin altına inmez, not bunu “durma toleransının
  üstünde” diye söyler. Bilinen kaymalara uzaklık ilk turdaki gibi ~5 mm'de kalır: kalan hata veridendir (nokta gürültüsü
  ±1 cm, voksel, çatı kenarı), tur onu gidermez.
- **Kontrol noktaları ağa ölçü olarak girmiyor** (Sonra). Ağın mutlak konumu sabit taramalarındır; kontrol Tarama
  kaydı'nda ayrıca denetlenir.
- Web (TODOS §0.2): `cad.pointclouds.place`, şema 251'in `pose`'u (web'in `io/columns.ts`'i dahil: kaynak bayrağı 8 ve
  16 sayı), web çizicisinde konumlu düğümler, Dönüşüm uygula'nın Euler açıları, ağın `rounds`'u.

## Kanıt

- **Çekirdek** (`kentos_pointcloud::ops::transform::tests::euler_angles_make_the_map`): her açı tek başına sağ el
  çeyrek dönmesi (sapma +X → +Y, yunuslama +Z → +X, yatış +Y → +Z); üç açı birlikte, sabit eksenlerde önce yatış sonra
  yunuslama sonra sapma ile aynı nokta; katı. Sözleşme testi (`pointcloud::tests::places`): çeyrek dönmenin kutusu,
  ölçek, yansıma ve son satır retleri.
- **`cad.pointclouds.place`** (`scripts/fixtures/pointcloud_place_cases.py`; KentOS kodu yok: Euler üç temel dönmenin
  çarpımıyla, eksen-açı kuaterniyonla, ters Rᵀ ile), **26 durum**: Euler (çeyrek dönme, üç açı, yunuslama 90°, eksi
  yatış), matris (öteleme, ağın matrisi gibi), eksen ve açı (Z, eğik eksen), konumlu bulutta ardışık konum, iki dosyalı
  bulut, Konumu kaldır (ve konumsuz bulutta yazmama), 14 ret; her yazmada Nokta sorgula noktayı çizimdeki yerinde bulur
  ve geri alma kutuyu geri getirir. Yetenek testi geçer. Bozma denemesi: iki değer oynatılınca tam o iki durum
  yakalanır.
- **İşlemler** (`pointcloud_processing_cases.py`, 59 durum; eskiler aynı): Euler açıları (merkez boş ve verilen, ters;
  noktalar bayt bayt), konumlu bulutun birim matrisle dosyaya yazılması (geçmişte konum | matris), konumlu bulutta Son
  dönüşümü geri al ve Seyrelt retleri.
- **`.kcad`** (`kcad_v2_reference.py`): örnek `pointcloud-poses.json/.kcad` (iki dosyalı bulut: çeyrek dönme + kalkma
  ve küçük öteleme), `broken/pointcloud-pose-in-schema-248.kcad` (`unknown_field`), `pose-scaled` ve `pose-fifteen`
  (`bad_value`); öbür 1800 örnek bayt bayt aynı; sütunlu aktarım (`columns`) konumu taşır.
- **Ağın dış döngüsü** (`pointcloud_network_cases.py`, 27 durum; eski 22'si aynı, yalnız son not “Nesnede tut”u anar):
  üç tarama 5 tur (turların kayması küçülür, bilinen kaymalar 1 cm içinde), dört tarama 1 mm toleransla erken durur
  (4 tur: 373, 8,8, 1,4, 0,38 mm), tek tur eskisi gibi, tur 0 ve 11 retleri. Kayıt başvurusu taşınan taramanın
  noktalarını ve kutusunu (`BOXES`) aynı biçimde taşır. Tur durumları göreli tutulur (her sayının büyüklüğünün 10⁻⁷'si,
  en az 1): ikinci turdan sonra iki taraf dönmede 10⁻¹¹ dolayında anlaşır, bu matrislerin öteleme sütununda
  (c + t − R·c, c ≈ 4,4·10⁶ m) 10⁻⁵ m görünür. Bozma denemesi: iki değer oynatılınca tam o iki durum yakalanır.
- **Python** (`python/tests/test_cloud_place.py`: Euler konumu testin kendi kuaterniyon çarpımı q_z·q_y·q_x ile 10⁻¹²
  içinde, merkez + öteleme, kutunun sekiz köşesi, sağ el çeyrek sapma, sorgu noktayı yerinde bulur, Konumu kaldır,
  ölçekli matris reddi; `test_pointcloud_network.py`'ye dış döngü: turların kayması düşer, matris merkezi öteleme kadar
  taşır). Ortak durumlar Python'da da geçer.
- **MCP** (`crates/native/mcp/tests/cloud_place.rs`): Euler konumu durumunki, ardından sorgu noktayı yerinde bulur,
  ölçekli matris reddi; `pointcloud_network` geçer.
- **Masaüstü** (`cloud_place_window_tests`, `scan_network_window_tests`): pencerenin Euler önizlemesi ortak durumunki,
  Konumla yazar ve tek adımda geri alınır, çiziciye dosyanın konumu verilir; Konumu kaldır; ölçekli matris söylenir,
  yazılmaz; Nesnede tut taramanın matrisini nesneye yazar ve ağ yeniden çözülünce B'nin düzeltmesi 361 mm'den 1,3 mm'ye
  iner; 5 tur ortak durumunki. Çapraz denetimler (module, ribbon, history::text, catalog, settings) geçer. Resimler
  `.run/gui-evidence/map-13-10/` (1 Euler penceresi, 2 konumlanmış bulut çizimde — dönmüş noktalar dosyanın yerindekinin
  yanında, 3 ölçekli matris reddi koyu dar, 4 şeritteki yeri, 5 Nesnede tut, 6 dış döngü koyu dar).

## Sonra

- Kontrol noktalarının ağa ölçü olarak girmesi (KOH_Z'nin düzlem kotu düşey gözlem olarak; TKGM oturum başına iki
  kontrol noktası) ve dış döngüde kontrolün sabit tarama yerine datum olması.
- Dosyaları farklı konumlu sanal bulutta Dönüşüm uygula (dosya başına matris); öbür İşlemler araçlarının konumu
  okuması.
- Gerçek veride süre ve bellek ölçümü.
- Web (TODOS §0.2).
