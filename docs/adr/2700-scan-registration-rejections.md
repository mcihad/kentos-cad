# 2700 — Tarama kaydında kenar ve örtülme ayıklaması

Tarih: 2026-11-08. plan/map-cad.md MAP-13, dördüncü tur. Durum: taslak. Plan satırı: “Otomatik kayıt/ICP sonucunda
kontrol noktası, örtüşme, artık, güvenilir kapsam ve dönüşüm tarifi raporlansın”; kabul: “eğimli zemin, bordür, örtülmüş
yol ve çoklu tarama içeren veriyle işle”. ADR 2300'ün “Sonra”sındaki “kenar ve örtülme ayıklaması”.

İlgili:
- [ADR 1800](1800-scan-registration-check.md): tarama kaydı denetimi (nokta-düzlem ICP);
- [ADR 2300](2300-scan-registration-robust-weights.md): kaba hatayı atan ağırlıklar.

## Dayanak

- **PCL, `CorrespondenceRejectorSurfaceNormal`**
  ([belge](https://pointclouds.org/documentation/classpcl_1_1registration_1_1_correspondence_rejector_surface_normal.html),
  8 Kasım 2026'da okundu): eşleşmeyi iki noktanın yüzey normallerinin uyuşmasına göre atar. Kaynak ve hedef bulutun
  normallerini ister. Eşiği kullanıcı verir; belgeye göre eşik açının kosinüsüdür.
- **PCL, `BoundaryEstimation`**
  ([belge](https://pointclouds.org/documentation/classpcl_1_1_boundary_estimation.html), 8 Kasım 2026'da okundu):
  - noktanın komşuları normaline dik yerel düzleme izdüşürülür (u-v koordinatları), açı ölçütüyle sınır noktası
    olup olmadığına karar verilir;
  - öntanımlı açı eşiği π/2'dir (`setAngleThreshold`);
  - komşular k en yakın ya da yarıçapla aranır.
- **Örnek veri ölçüsü:** 8 komşulu ölçüt seyrek vokselde neredeyse her noktayı kenar sayıyordu (A–B taramasında 2378
  eşleşmenin 1892'si). Rastgele 7 yönün en büyük boşluğu çoğu zaman 90°'yi aşar; komşular yarıçapla aranınca kenar,
  eşleşmenin uzanabileceği uzaklıkta kapsamın bitmesi demek olur.
- **KentOS bugün** (ADR 1800, 2300):
  - her hareketli nokta erişim içindeki en yakın sabit noktayla eşlenir; ağırlıklar artığın büyüklüğüne bakar;
  - bir taramanın görüp öbürünün görmediği yüzeyler kalıyordu: yalnız birinde olan duvarın ayağı, bordür arkasındaki
    gölgeye düşen kaldırım. Bunlar erişim içinde yanlış eşleşir, artıkları da ağırlıkları her zaman uyandıracak kadar
    büyük olmaz.

## Karar

1. **`cad.pointclouds.register`'a `reject`** (istenince; komut sürümü değişmez). Her adımda, ağırlıklardan önce:
   - **`edges`:** sabit noktası bulutunun kenarında olan eşleşme dışarıda kalır. Kenar ölçütü:
     - noktanın erişim içindeki sabit komşuları (kendisi hariç) normaline dik düzleme izdüşürülür;
     - u = n × e (e düşey eksen, |n_z| ≥ 0,9 ise doğu), v = n × u;
     - yönlerin (atan2) en büyük boşluğu, dolanan boşluk dahil, 90°'den büyükse ya da ikiden az yön varsa kenardır
       (PCL'in açı ölçütü ve öntanımlı eşiği).

     Komşuların yarıçapı erişimdir: eşleşme o kadar uzanabilir, kenar orada sabit taramanın bitmesidir.
   - **`angle`:** sabit normal ile hareketli noktanın kendi normali arasındaki açı bu dereceden büyükse eşleşme dışarıda
     kalır. Hareketli normal hareketli voksel noktalarından aynı kuralla kurulur ve bulutla döner. Açı doğruların
     açısıdır (|cos|); hareketli normali olmayan nokta da bırakılır. Açı kullanıcınındır (0° ile 90° arasında): KentOS
     değer vermez, PCL de vermez.
2. **Cevapta `rejected`:** son adımda kenarda ve açıyla bırakılan eşleşmeler; RMS, örtüşme ve kapsam kalan
   eşleşmelerden; bir not bunu söyler.
3. **Retler** (`invalid_input`): hiçbir şey istemeyen `reject`; 0° ile 90° dışında `reject.angle`.
4. **Masaüstü** (Tarama kaydı):
   - Ayıkla satırı: Kenardaki eşleşmeler, Normal açısı (°) ve açının kutusu; açı ilk açılışta 30°, kutu işaretlenene
     kadar kullanılmaz;
   - özet satırı bırakılanları sayar.
5. **Yoğunluk koşulu:** kenar ölçütü voksellerin çoğu dolu olduğunda anlamlıdır. Voksel başına 1,5 nokta olan seyrek
   taramada boş vokseller boşluk sayılır, eşleşmelerin yarısından çoğu atılır; sonuç yine oturur (ortak durumda), ama
   vokseli taramanın yoğunluğuna göre seçmek gerekir.
6. **Kapsam işareti:** ADR 1800'deki gibi **Harita + CBS**.

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/pointcloud_register_cases.py`, numpy yalnız öz değerler için, KentOS kodu yok.
  - Yeni bulutlar:
    - kayit-f: ikinci tarama ve yalnız onda olan 20 m × 3 m duvar;
    - kayit-h ve kayit-i: zeminin kuzey şeridi, y = 20 m'de 15 cm bordür, m²'de 24 nokta; sabit taramada bordür arkası
      1,5 m gölge.
  - Duvar: düz ICP kuzeyde 5 cm, kotta 1,4 cm kayar. 30° 91 eşleşmeyi atar, dönüşüm temiz kayda 1,4 mm içinde.
  - Bordür: düz ICP kuzeyde 2 cm, kotta 1 cm kayar, ω'yu altı kat yanlış bulur. Kenar 616 eşleşmeyi atar, dönüşüm
    gerçeğe 4 mm içinde.
  - 60°, Huber ile birlikte, kenar ve açı birlikte; seyrek taramada kenar ölçütü; temiz taramada açı (8 eşleşme);
    üç ret.
  - 52 durum (40'tan); eski 40 aynı. Rust ve Python geçer; Rust ile Python kenar ve açı kararlarında aynı.
- **MCP** (`crates/native/mcp/tests/register_rejections.rs`): dört durumun dönüşümü, eşleşmesi ve sayıları
  durumunkiler; boş ayıklama reddedilir.
- **Masaüstü** (`register_window_tests.rs`): Ayıkla satırının açısı (30°, 60°) ve kenarı durumların dönüşümünü ve
  sayılarını verir. Çapraz denetimler geçer. Resimler `.run/gui-evidence/tarama-ayiklama-2700/` (dizin işleri
  bitince çekilir).

## Sonra

- Dönüşümün buluta uygulanması yapıldı: [ADR 3100](3100-apply-a-transform-to-a-point-cloud.md) (Dönüşüm uygula,
  Tarama kaydı'nda Buluta uygula).
- Çoklu tarama demeti; gerçek veriyle süre, bellek ve yoğunluk ölçümü (MAP-13'ün öbür maddeleri).
- Web (TODOS §0.2).
