# 1500 — Tarama uyumu denetimi: şeritler arası fark ve kontrol noktaları (`cad.pointclouds.overlap`)

Tarih: 2026-11-06. plan/map-cad.md MAP-13 (alt maddeler: kayıt ve şerit uyumunda kontrol noktası, örtüşme, artık ve
güvenilir kapsam; global kontrol bilinmiyorsa bulut “ölçüye kesin hazır” sayılmaz; ölçüm tam kaynaktan; eğimli zemin,
örtülmüş yol ve çoklu tarama). Durum: taslak. İlgili: [ADR 0552](0552-point-cloud-areas-meshes-and-virtual-clouds.md) (bölge ve profil:
tam çözünürlükte okuma), [ADR 1480](1480-orthophoto-accuracy-check.md) (kural paketi biçimi).

## Dayanak

- **Türk mevzuatı:** BÖHHBÜY (RG 26.06.2018/30460 Mük.) LiDAR'ı yalnız tanımlar (m. 4/h); şerit uyumu ya da nokta
  bulutu doğruluğu için ölçüt vermez (metin harita.gov.tr'den okundu). Harita ürününün düşey doğruluğu için m. 91(2)
  geçerlidir: KOH_Z ≤ 1,33·KOH_X,Y(S). Türk kurumlarının LiDAR şartnamelerinde sayısal şerit uyumu ölçütü bulunamadı
  (arama 6 Kasım 2026).
- **USGS Lidar Base Specification:** göreli doğruluk iki ölçüyle verilir.
  - **Şerit içi:** düz yüzeyde tekrarlanabilirlik ≤ 6 cm.
  - **Şeritler arası:** örtüşmede RMSDz ≤ 8 cm (QL2).

  Değerler v1.3'ü anan USGS görev belgelerinden alındı
  ([USGS revizyon geçmişi](https://www.usgs.gov/ngp-standards-and-specifications/LiDAR-base-specification-revision-history)).
  2024 sürümü denetim noktası sayısını 30'a çıkardı ve ölçüm hatasını RMSE'ye kattı. Türkiye'de bağlayıcı değildir.
- **Trimble Business Center, Leica Cyclone, TerraScan/TerraMatch:** kayıt (registration) ve şerit uyumu ayrı bir iştir.
  Sonucu örtüşen yüzeylerin farkı ve kontrol noktalarıyla raporlanır (plan/map-cad.md [S14]).
- **KentOS bugün:** LAS/LAZ/COPC ve metin bulutları tam çözünürlükte okunur; bölge, profil ve zemin süzgeci vardır
  (ADR 0552). İki taramanın uyumunu ölçen bir iş yoktur.

## Karar

1. **Kural paketi veridir:** `catalog/v1/rules/lidar-uyum.json`.
   - **İçerik:** şeritler arası RMSDz sınırı (örnek: USGS QL2'nin 8 cm'i, “Türk mevzuatı değil, kurum kendi değerini
     verir” notuyla) ve m. 91(2)'nin KOH_Z katsayıları.
   - **Başka paket:** komut başka bir paket de alır (`rules`).
2. **`cad.pointclouds.overlap` v1, salt okunur (Harita + CBS).**
   - **Girdi:** iki bulut (`uids`).
   - **Okuma:** bulutlar tam çözünürlükte bir kez okunur (`pointcloud_areas::Scan`); `classes` ile sınıf süzülebilir.
   - **Hücre farkı:**
     - her bulutun bir hücredeki (`cell`, ilk değer 2 m) noktalarına en küçük kareler düzlemi uydurulur, böylece eğim
       fark sayılmaz;
     - düzlem hücre merkezine göre yazılır, merkezdeki yükseklik Cramer kuralıyla bulunur;
     - fark ikinci − birincidir;
     - noktası `least`'ten (ilk değer 4) az ya da noktaları bir doğru üzerinde olan hücre sayılmaz.
   - **Rapor:** örtüşen hücreler ve alanı, RMSDz, ortalama (sistematik kayma), standart sapma, en büyük fark, en kötü
     20 hücre.
   - **Kontrol noktaları:** kontrol katmanı (`control`) verilirse, her bulutun kontrol noktası çevresindeki (`radius`)
     düzleminin noktaya farkı alınır; her bulutun KOH_Z'si m. 91(2) sınırıyla karşılaştırılır.
   - **Kontrolsüz bulut:** kontrol noktası yoksa “bulut ölçüye kesin hazır sayılmaz” notu verilir.
   - **Taşıma yok:** hiçbir bulut taşınmaz, dönüşüm uygulanmaz.
3. **Pencere Tarama uyumu…** (`pointcloud.overlap`, ikon `cloudOverlap`; seçenekleri sahibine önceden gönderildi):
   - **Yeri:** Nokta bulutu bağlam sekmesi (Harita, Yol, CBS), Yol'un Arazi sekmesi, CBS'nin Analiz › Arazi paneli.
   - **Form:** iki tarama bir satırda; hücre ve en az nokta bir satırda; kontrol katmanı, çevre ve ölçek bir satırda.
   - **Özet:** RMSDz sınırla, sistematik kayma, iki taramanın KOH_Z'si.
   - **Tablolar:** En kötü hücreler ve Kontrol noktaları kayan veri tablolarında; CSV kaydet.

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/pointcloud_overlap_cases.py` 26 durum üretir. Düzlem uydurma ve istatistik
  elle yapılır; KentOS kodu kullanılmaz. Dört sentetik metin bulutu da betikte üretilir:
  - eğimli zemin üzerinde A ve 4 cm kaymış B;
  - 25 cm kaymış C;
  - yolda aracı olan D.

  Durumlar şunları kapsar:
  - A–B geçer, A–C ve A–D kalır; D'de en kötü hücreler aracın yeridir;
  - sıranın işareti çevirmesi, hücre boyu 1 ve 5 m, en az nokta, kontrol çevresi;
  - 1/500 ölçek, başka paket, sınıf süzgeci;
  - retler.

  Rust ve Python geçer.
- **Masaüstü:** `overlap_window_tests.rs` sınar:
  - GNSS katmanının tahminini, A–B'nin geçmesini (4 cm kayma) ve A–C'nin kalmasını;
  - CSV'yi, salt okunurluğu, Harita ve CBS'yi.

  Resimlerde D'nin araçlı hücreleri ve araç altında kalan K5 kontrol noktası (ikinci taramanın KOH_Z'sini aşırıyor)
  görülür. Resimler `.run/gui-evidence/tarama-uyumu-1500/`, menüdeki yeri dahil.

## Sonra

- Kayıt ve ICP'nin kendisi (dönüşüm tarifi, güvenilir kapsam), şerit içi tekrarlanabilirlik.
- Kaydın kalıcı kimliği (dosya özeti + kayıt konumu); kaynak sınıfı ile kullanıcı düzeltmesinin ayrı tutulması.
- Kesit ve düzlemde düzenleme, semantik çıkarım (bordür, yol kenarı); gerçek veriyle süre ve bellek raporu.
