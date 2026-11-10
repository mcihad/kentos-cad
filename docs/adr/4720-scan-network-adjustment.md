# 4720 — Tarama ağı dengelemesi: üç ve daha çok taramanın birlikte kaydı

Tarih: 2026-11-10. plan/map-cad.md MAP-13, dokuzuncu tur. Durum: taslak. MAP-13'ün açığı “çoklu tarama demeti”.

İlgili:
- [ADR 1800](1800-scan-registration-check.md), [ADR 2300](2300-scan-registration-robust-weights.md),
  [ADR 2700](2700-scan-registration-rejections.md): iki taramanın kaydı (nokta-düzlem ICP, ağırlık, ayıklama);
- [ADR 3100](3100-apply-a-transform-to-a-point-cloud.md): Dönüşüm uygula (taramayı matrisiyle yeni dosyaya taşır);
- [ADR 2160](2160-network-similarity-article-15-1-c.md) ve sonrası: GNSS ağının dengelemesi (bazlar ölçü, noktalar bilinmeyen).

## Dayanak

- **Leica Cyclone REGISTER 360** (alt ajanın notu `.run/research/map-cad/uygulama/MAP-13.9.md`, belgeler ve ekran
  resimleri 10 Kasım 2026'da okundu): taramalar bağlarla (link) bir demet olur; her bağın hatası ve örtüşmesi renkle
  gösterilir (yapılı bağda 15 mm altı ve %20 üstü örtüşme yeşil); Constraint List her bağın hatasını sıralar; demet
  birlikte en iyilenir, döngüler kapanır.
- **FARO SCENE, CloudCompare, TBC:** taramanın konumu nesnede (konum + eksen + açı ya da 4 × 4 matris) tutulur;
  E57 konumu kuaterniyonla saklar, noktaları yeniden yazmaz.
- **Jeodezik ağ dengelemesi** (KentOS'un GNSS ağı, ADR 2160 vd.): bazlar ölçü, noktalar bilinmeyen, sabit noktalar
  datum; döngüler kapanmamayı söyler. Tarama ağı aynı kurgudur: bağın kaydı ölçü, taramanın düzeltmesi bilinmeyen.
- **Mevzuat** (alt ajanın notu `.run/research/map-cad/mevzuat/MAP-13.9.md`): TKGM'nin Yersel LİDAR Yöntemi ile
  Kadastral Detay Ölçmeleri esasları (2018) oturum başına en az iki kontrol noktası, en çok 100 m menzil, en az 100
  nokta/m² ister; kontrol BÖHHBÜY m. 46(4)'e göre ±15 cm. **Çok taramalı kaydın artığına resmî Türk metninde sayısal
  sınır yok;** kayıt artığı (iç uyum) doğruluk diye sunulmaz.

## Karar

1. **`cad.pointclouds.network` v1** (yeni komut, salt okunur): `uids` (en az üç tarama), `fixed` (en az biri, hepsi
   değil), `links` (çiftler; verilmezse taramaların sırasıyla plan kutuları örtüşen her çift), kaydın kendi seçenekleri
   (`spacing`, `reach`, `iterations`, `tolerance`, `neighbours`, `robust`, `reject`, `drawing`; her bağda aynı).
2. **Bağ = ölçü:** her bağ `cad.pointclouds.register`'ın kaydettiği gibi kaydedilir (birincisi sabit, ikincisi
   hareketli). Cevabın dönmesi (ω, φ, κ) ve ötelemesi ağın merkezine (taramaların kutularının ortalarının ortalaması)
   taşınır: t + (R − I)(merkez − c). Ağırlığı kaydın son eşleşmelerinin normal matrisi N'dir (ADR 1800'ün Low
   doğrusallaştırması), merkeze Jᵀ·N·J ile taşınır (J birim matris, sol alt bloğunda [d]×, d = merkez − c). Altıdan az
   eşleşmeli bağ reddedilir (altı bilinmeyen).
3. **Dengeleme** (`kentos_pointcloud::ops::network`, ortak çekirdek): sabit taramanın düzeltmesi sıfır; öbürlerinin
   x = [ω, φ, κ, tx, ty, tz]. Bağın artığı r = x_b − x_a − m; Σ rᵀ·N·r en küçüktür. Normal denklemler taramaların
   sırasıyla kurulur, Cholesky ile çözülür; büyük köşegenin 10⁻¹²'sinden küçük pivot ağı belirsiz sayar (ret).
   Birinci derece: dönmeler vektör gibi toplanır; kayıtlı taramalar arasındaki küçük düzeltmelerde geçerlidir.
4. **Döngüler:** sabit taramalar tek düğüm; bağların sırasıyla genişlik öncelikli ağaç; ağaç dışı her bağ bir döngü
   kapatır, kapanmaması a'nın yol toplamı + m − b'ninkidir. Cevapta döngünün taramaları, dönme ve öteleme uzunluğu.
5. **Cevap:** ağın merkezi; her taramanın düzeltmesi, Dönüşüm uygula'ya verilecek matrisi (p ↦ R(p − merkez) + merkez +
   öteleme, R = Rz(κ)·Ry(φ)·Rx(ω)) ve serbest taramada **yatay gücü** (normal matrisin tersindeki öteleme bloğundan
   √(σz²/σh²); Tarama kaydı'nın `horizontal`'ı gibi, 0,1'in altı zayıf); her bağın eşleşmesi, örtüşmesi, kayıt sonrası
   RMS'i, gücü, ölçüsü, artığı ve artığın eşleşmelerin normalleri boyunca kayması (√(rᵀ·N·r / eşleşme)); döngüler;
   ağın RMS kayması (bütün bağların toplamı). Notlar: ağ özeti, birinci derece, döngülerin anlamı, zayıf bağ ve
   taramalar, “ağın mutlak konumu sabit taramalarındır, kontrol noktalarıyla Tarama kaydı'nda ayrıca denetlenir”.
6. **Düz zemin:** gürültü normal matrisi tekil olmaktan çıkarır, yatay düzeltme anlamsızdır; ağ reddetmez, yatay gücü
   ve notu söyler (Tarama kaydı'nın davranışı). Tam tekil ağ reddedilir.
7. **Retler:** `invalid_uid`, `entity_not_found`, `not_a_pointcloud` (`uids[i]`); `invalid_input` (üçten az tarama, aynı
   tarama iki kez; sabit yok, hepsi sabit, sabit ağda değil; bağ iki kez, kendine, taraması ağda değil; sabit taramaya
   bağlanmayan tarama; altıdan az eşleşmeli bağ; belirsiz ağ); kaydın kendi seçeneklerinin retleri.
8. **Masaüstü:** Nokta bulutu sekmesinde Tarama kaydı'nın altında **Tarama ağı** (pencere “Tarama ağı dengelemesi”;
   yeni ikon `cloudNetwork`, seçenek kâğıdı `.run/gui-evidence/ikon-tarama-agi/`). Form: Taramalar (Seçili bulutları al),
   Sabit tarama, Voksel/Erişim/En çok adım, Ağırlık (Eşit | Huber; ilk değer Huber), Kenardaki eşleşmeleri ayıkla.
   Özet notlar; Uygulanacak tarama ve **Dönüşüm uygula…** (taramayı seçer, Dönüşüm uygula'yı matrisiyle açar, tarama
   “Ağda dengelenen taramalar” katmanına yeni dosyayla yazılır); Taramalar (ω φ κ cc'de, öteleme mm, yatay güç),
   Bağlar, Döngüler tabloları.
9. **Kapsam işareti:** **Harita + CBS** (nokta bulutu bağlam sekmesi), ADR 1800'deki gibi.

## Sınırlar

- Tek tur: bağlar taramaların bugünkü yerinden bir kez kaydedilir, ağ bir kez çözülür. Dengelenen yerden bağları
  yeniden kaydeden dış döngü (çok görüşlü ICP) yok.
- Dönüşüm nesnede tutulmaz: tarama ancak Dönüşüm uygula ile yeni dosyaya taşınır (dosyasız nesnede dönüşüm MAP-13'ün
  açığı olarak kalır).
- Kontrol noktaları ağa ölçü olarak girmez; ağın mutlak doğruluğu Tarama kaydı'nın kontrol denetimiyle ayrıca sınanır.
- Bağın ağırlığı ICP'nin normal matrisidir; ölçeği eşleşme sayısıyla büyür, varyans bileşeni kestirilmez.

## Kanıt

- **Çekirdek** (`kentos_pointcloud::ops::network` testleri, 4): tutarlı üçgen gözlemlerini aynen verir ve kapanır;
  0,03 m'lik kapanmama eşit ağırlıkla üçe paylaşılır; bağlanmayan tarama ve tekil ağ reddedilir; merkez değişince
  hareket aynıdır.
- **Bağımsız başvuru** (`scripts/fixtures/pointcloud_network_cases.py`; KentOS kodu yok), 22 durum: yedi sentetik
  tarama (eğim, tümsek, sırt ve çatılı zeminin parçaları, her biri bilinen dönme ve ötelemeyle); bağlar
  pointcloud_register_cases.py'nin kaydıyla, ağ elle (numpy'la normal denklemler, Cholesky kuralı, döngüler). Bilinen
  kaymalar bağımsız doğrudur: Huber ağırlığıyla her taşınan tarama 1 cm içinde yerine döner; ağırlıksızda çatı
  kenarındaki eşleşmeler 2,5 cm saptırır (3 cm içinde). Üç, dört tarama; iki sabit; verilen zincir (döngüsüz, artıksız);
  sıra değişimi; düz zemin (yatay güç < 0,1, not); 14 ret. Rust ve Python geçer; yetenek testi geçer. Bozma denemesi:
  bir düzeltme ve bir döngü kapanması 1 mm oynatılınca tam o iki durum yakalanır.
- **Python** (`python/tests/test_pointcloud_network.py`, testin kendi okumalarıyla): taşınan taramalar 1 cm içinde yerine
  döner; döngüde artıklar kapanmamayı siler (r_ab + r_bc − r_ac = −w, 10⁻⁹); zincirde düzeltmeler toplanır; iki tarama
  reddedilir.
- **MCP** (`crates/native/mcp/tests/pointcloud_network.rs`): üç durum.
- **Masaüstü** (`scan_network_window_tests`): Huber ve eşit ağırlık durumlarınki; tablolar; Dönüşüm uygula seçilen
  taramayı matrisiyle açar; iki tarama söylenir. Resimler `.run/gui-evidence/tarama-agi-4720/` (şeritteki yeri dahil).

## Sonra

- Dengelenen yerden bağları yeniden kaydeden dış döngü; kontrol noktalarının ağa ölçü olarak girmesi.
- Dönüşümün dosyasız nesnede tutulması (taramanın konumu nesnede, çizim ve ölçü onunla).
- Euler açılarının sırası artık kaynaklı: OGC GeoPose 1.0 (ENU, z→y′→x″) ve CloudCompare'in kaynağı (Rz·Ry·Rx);
  Dönüşüm uygula'ya açıyla giriş.
- Web (TODOS §0.2).
