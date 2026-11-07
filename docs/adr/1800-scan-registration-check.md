# 1800 — Tarama kaydı denetimi: nokta-düzlem ICP, dönüşüm tarifi, güç ve güvenilir kapsam (`cad.pointclouds.register`)

Tarih: 2026-11-07. plan/map-cad.md MAP-13, ikinci dilim. Durum: taslak. İlgili:
- [ADR 1500](1500-scan-overlap-check.md): Tarama uyumu (`cad.pointclouds.overlap`). Bu ADR oradaki “Sonra: kayıt ve
  ICP'nin kendisi (dönüşüm tarifi, güvenilir kapsam)” maddesini açar.
- [ADR 0552](0552-point-cloud-areas-meshes-and-virtual-clouds.md): bulutun tam çözünürlükte okunması (`pointcloud_areas::Scan`).

## Dayanak (7 Kasım 2026'da okundu)

- **Plan (MAP-13):**
  - Otomatik kayıt/ICP sonucunda kontrol noktası, örtüşme, artık, güvenilir kapsam ve dönüşüm tarifi raporlansın.
  - Kontrol bilinmiyorsa bulut “ölçüye kesin hazır” sayılmasın.
  - Kabul satırı: kayıt bağımsız kontrolle değerlendirilsin.
- **CloudCompare ICP** ([wiki](https://www.cloudcompare.org/doc/wiki/index.php?title=ICP),
  [Registration](https://cloudcompare.org/doc/wiki/index.php/Registration)):
  - ICP en çok yineleme sayısında ya da iki yineleme arasındaki RMS farkı eşiğin altına inince durur.
  - “Final overlap” kısmi örtüşmeyi söyler.
  - Son RMS √(ΣDᵢ²/N)'dir ve yalnız kullanılan noktalardan hesaplanır (forum yöneticisinin uyarısı).
  - Sonuç 4×4 matris olarak yazılır.
- **Leica Cyclone REGISTER 360** ([hata raporlama](https://rcdocs.leica-geosystems.com/cyclone-register-360/2025.0/error-reporting)):
  - bulutlar arası hata, hedef hatası, örtüşme yüzdesi ve “strength” bağlantı başına raporlanır;
  - Leica, kaydın kalitesini yalnız bulutlar arası ve hedef hatasıyla değerlendirmemeyi öğütler.
- **Low (2004)**, Linear Least-Squares Optimization for Point-to-Plane ICP Surface Registration (UNC TR04-004,
  [PDF](https://comp.nus.edu.sg/~lowkl/publications/lowk_point-to-plane_icp_techrep.pdf)):
  - küçük açıda sin θ ≈ θ, cos θ ≈ 1 alınınca nokta-düzlem en küçük karesi altı bilinmeyenli doğrusal bir sisteme
    döner;
  - çözüm üç açı ve üç ötelemedir.
  - PCL'nin `TransformationEstimationPointToPlaneLLS`'i aynı yöntemdir
    ([PCL](https://pointclouds.org/documentation/classpcl_1_1registration_1_1_transformation_estimation_point_to_plane_l_l_s.html)).
- **Mevzuat:** BÖHHBÜY LiDAR kaydına ölçüt vermez (ADR 1500). Kontrol noktalarında ürünün KOH_Z'si m. 91(2)'yle
  denetlenir. Şeritler arası sınır `lidar-uyum` paketinin örnek değeridir (USGS LBS QL2, bağlayıcı değil).

## Karar

1. **`cad.pointclouds.register` v1, salt okunur** (Harita + CBS: nokta bulutu bağlam sekmesi). Birinci bulut sabittir,
   ikincisi ona oturtulur. Bulut taşınmaz.
2. **Okuma ve seyreltme:**
   - İki bulut, kutularının buluştuğu yerde (erişim kadar genişletilmiş) tam çözünürlükte bir kez okunur.
   - Her biri voksel başına bir noktaya indirilir: vokseldeki noktaların ortalaması (`spacing`, öntanımlı 0,5 m).
     Vokseller anahtar sırasıyla dizilir.
   - Bir bulut 400 000 voksel noktasından çok verirse `too_many_cloud_points`.
3. **Normal:**
   - Sabit bulutun bir noktadaki normali, `neighbours` (öntanımlı 8) en yakın noktasının kovaryansının en küçük öz
     değerinin öz vektörüdür; yukarı döndürülür.
   - Orta öz değer en küçüğün dört katından küçükse normal yoktur (düzlem tanımlı değil).
4. **ICP adımı (Low 2004):**
   - Her hareketli nokta, erişim içindeki (öntanımlı 1 m) en yakın sabit noktayla eşlenir (eşitlikte küçük sıra).
     O noktanın normali yoksa eşleşme yoktur.
   - Artık r = (p − q)·n ve satır a = [(p − c) × n, n] ile (Σ a aᵀ) x = −Σ a r çözülür.
   - Çözüm normal matrisin öz vektörleriyle yapılır; en büyüğün 10⁻¹²'sinin altındaki öz değerler dışarıda kalır.
     Böylece düz zeminde belirsiz yönler kendiliğinden kaymaz.
   - Adımın dönüklüğü Rz(κ)·Ry(φ)·Rx(ω), merkez c (sabit noktaların ortalaması) çevresinde, önceki dönüşümün önüne
     konur.
   - Durma: bir adım hiçbir noktayı tolerans (öntanımlı 0,1 mm) kadar taşımayınca (öteleme ve açılar × hareketli
     noktaların RMS yarıçapı) ya da yineleme sınırında (öntanımlı 30, en çok 200).
   - En yakın aramalar kesindir: vokselin iki katı ızgarada halka halka.
5. **Cevap:**
   - Dönüşüm tarifi: p ↦ R(p − merkez) + merkez + öteleme. Dönüklük [ω, φ, κ] radyandır. Aynısı projenin
     koordinatlarında satır sıralı 4×4 matristir (CloudCompare'in verdiği biçim).
   - Okunan ve kullanılan noktalar, adım sayısı ve yakınsama.
   - Eşleşme ve örtüşme payı; önce ve sonra nokta-düzlem RMS'i, sonraki işaretli ortalama (yukarı normalle: hareketli
     bulut üstteyse artı).
   - **Güç:** açılar RMS yarıçapla metreye çevrilmiş normal matrisin √(λmin/λmax)'ı. **Yatay güç:** plan
     ötelemelerinin küçük öz değeri ile kot ötelemesininkinin oranının karekökü. 0,1'in altında not düşülür: düz zemin
     yatay kaymayı belirlemez; yatay için eğimli yüzey, yapı ya da kontrol gerekir. Cyclone'un “strength”inin
     karşılığıdır ama tanımı KentOS'undur.
   - **Güvenilir kapsam:** hareketli noktaların plan hücreleri (`cell`, öntanımlı 2 m). `least` (öntanımlı 4)
     eşleşmeli hücreler kapsanmış, RMS'i paketin şeritler arası değerini aşmayanlar güvenilir sayılır. En kötü 20
     hücre verilir.
   - **Kontrol:** her kontrol noktasında sabit bulutun ve hareketli bulutun önce/sonra düzlemi çıkarılır (ADR 1500'ün
     düzlemi, `radius` içinde). KOH_Z'ler m. 91(2)'nin sınırıyla verilir.
   - **Notlar:** dönüşüm uygulanmadı; RMS yalnız eşleşen noktalarındır; sınır örnek değerdir; düz zemin; yineleme
     sınırı; kontrolsüz kayıt ölçüye kesin hazır değil.
6. **Retler:**
   - `invalid_input`: iki ayrı bulut değil, aralık dışı sayı, ölçek 0, kural paketi;
   - `invalid_uid`, `entity_not_found`, `not_a_pointcloud`;
   - `layer_not_found` (`control`), `too_many_cloud_points`.

   Uyarı: `cloud_source_unreadable`.
7. **Masaüstü: Tarama kaydı…** (`pointcloud.register`):
   - **Simge:** `cloudRegister`, kesikli taramanın düz taramanın üstüne oturtulması. Seçenek sayfası
     `.run/gui-evidence/tarama-kaydi-1800/secenekler.png`.
   - **Yer:** Nokta bulutu bağlam sekmesinde ve iki şerit yerinde Tarama uyumu'nun altında.
   - **Form:** sabit ve oturtulacak tarama bir satırda; voksel, erişim ve en çok adım bir satırda; kontrol katmanı,
     çevre ve ölçek bir satırda.
   - **Özet:** adımlar ve RMS, dönüşüm (öteleme cm, açılar projenin biriminde: grad ve cc), güç, güvenilir kapsam,
     üç KOH_Z, notlar.
   - **Tablo:** en kötü hücreler ya da kontrol noktaları; CSV kaydet matrisi de yazar.

## Kanıt

- **Ortak durumlar:** bağımsız `scripts/fixtures/pointcloud_register_cases.py`, 31 durum; KentOS kodu yok.
  - Voksel, normal, Low adımı, güç, kapsam ve kontrol elle hesaplanır. numpy yalnız öz değerler içindir; en yakın
    aramalar tam taramadır.
  - Bulutlar sentetik metin bulutlarıdır: eğim, tümsek, sırt ve beşik çatı ±1 cm gürültüyle iki kez örneklenir.
    İkincisi ω 0,02°, φ −0,03°, κ 0,05° ve (0,30; −0,20; 0,06) m kaydırılmıştır. Ayrıca iki düz örnek vardır.
  - Kaydırılmış tarama 4 adımda yakınsar: öteleme (−0,302; 0,197; −0,060) m, RMS 8,4 → 1,5 cm, kontrol KOH_Z
    5,0 → 0,5 cm.
  - Düz zeminde kot 5 cm bulunur ama yatay güç 0,006'dır: plan kayması gürültüye kalır, yineleme sınırına varılır.
    Bu durumda matrisin ötelemesi uygulamalar arasında 10⁻⁵ m içinde tutar (kötü koşullu çözüm, merkezden 4,4 milyon
    metre); öbür durumlar 10⁻⁷ içindedir.
  - Öbür durumlar:
    - örtüşmeyen bulutlar, tek yineleme, 1 m voksel, dar erişim;
    - sınıf süzgeci, ters sıra;
    - başka paket, 16 komşu ve 4 m hücre, 1/500;
    - 17 ret.

  Rust ve Python geçer. Izgaranın kesinliği ve Jacobi öz ayrışımı birim testlerle doğrulanır.
- **Masaüstü** (`register_window_tests`, 2 test):
  - pencere ortak durumla aynı sonucu verir; KOH_Z 5 cm'den 1 cm'nin altına iner;
  - düz çiftte yatay güç 0,1'in altındadır;
  - CSV matrisi yazar, çizime hiçbir şey yazılmaz;
  - Harita ve CBS sunar.

  Resimler `.run/gui-evidence/tarama-kaydi-1800/` altındadır. Ortak masaüstü denetimleri 133 testle geçer.
- **MCP:** `kentos-mcp` testleri geçer. Soket testi macOS'un uzun `TMPDIR`'iyle düşer, `TMPDIR=/tmp` ile geçer; 30
  Ekim'den beri böyledir.

## Otomasyon

```python
from kentos import cad
doc = cad.Document.open("seritler.kcad")
r = cad.pointclouds.register(doc, uids=[sabit, oturtulacak], control="gnss")
print(r.translation, r.rotation, r.rms_before, r.rms_after, r.horizontal, r.control_after.koh_z)
```

MCP: `{"tool": "cad.pointclouds.register", ...}`; salt okunurdur.

## Sonra

- **Uygulama:** dönüşümün buluta yazılması (bulutun yerleşim dönüşümü; `.kcad` ve çizici), geri alınarak.
- **Çoklu tarama:** demet (bundle) dengelemesi, bağlantı ağacı, kayıt kimliği.
- **Sağlamlık:** kaba hatalı eşleşmeyi atan ağırlıklar (Huber, kırpılmış ICP), kenar ve örtülme ayıklaması.
- **Sınıf düzeltme:** kalıcı kayıt kimliği (dosya özeti + kayıt konumu) ve kaynak sınıfından ayrı düzeltme katmanı.
- **Ölçüm:** gerçek veriyle süre ve bellek raporu.
