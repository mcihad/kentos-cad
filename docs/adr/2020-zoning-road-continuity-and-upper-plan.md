# 2020 — İmar planı denetiminde yol devamlılığı ve üst plan kapsaması

Tarih: 2026-11-07. plan/map-cad.md MAP-09, ikinci tur. Durum: taslak. Plan satırı: “Alan sınırı, karar çelişkisi, eksik
plan notu, kopuk yol devamlılığı ve üst/alt plan tutarsızlığı sahnede bulunabilsin.” İlgili:

- [ADR 1420](1420-zoning-plan-typed-check.md): `imar-plani` profili ve `cad.zoning.check`. Bu ADR oradaki “Açık: üst plan
  sınırı kapsaması, yol devamlılığı” maddelerini kapatır;
- [ADR 0202](0202-topology-rules.md) §2: topoloji kuralları, “Sarkan uç olmamalı” ve “… içinde kalmalı”.

## Dayanak

- **Mekânsal Planlar Yapım Yönetmeliği:** alt kademe plan üst kademe plana uygun hazırlanır; uygulama imar planı nazım
  planın içindedir (ADR 1420'nin kaydı, Bakanlığın plan standartları sayfası).
- **QGIS planx_mpyy_studio** (ikincil, ADR 1420): plan kademesi ve topoloji denetimlerinin arasında yol ağının sürekliliği
  vardır.
- **KentOS bugün:**
  - Denetim üst planın kademesine bakıyordu (`levelOrder`), alt planın üst planın sınırı içinde kalıp kalmadığına
    bakmıyordu.
  - Profildeki yol aksı sınıfı (Yollar) hiç denetlenmiyordu.

## Karar

1. **`outsideUpper`:** üst planı plan sınırları arasında olan bir planın, üst planın sınırı dışında kalan parçası.
   - Topoloji çekirdeğinin “… içinde kalmalı” kuralı her alt/üst çifti için ayrı çalışır. Bir plan bir çiftin alt
     planı, öbür çiftin üst planı olabilir.
   - Parça, denetimin toleransından darsa sayılmaz (ADR 0202 §2). Bulgu iki planı ve dışarıdaki alanı söyler.
   - Kademe sırasından bağımsızdır: kademe yanlış olsa da sınır denetlenir.
2. **`roadGap`:** profilin Yollar katmanı çizimdeyse, yol aksının (çizgi ya da çoklu çizgi) ne başka bir yola ne bir
   plan sınırının kenarına toleransta değen ucu.
   - “Sarkan uç olmamalı” kuralı bağlantı olarak kendi katmanının alanlarını alır. Bu yüzden plan sınırları bu denetim
     için yolların katmanına konur.
   - Bulgu yolu kimliğiyle (YolKimlik) ve ucun yeriyle söyler: “(Y 487050.000, X 4420045.000)”.
   - İki ucu da kopuk yol iki bulgudur, baştaki önce.
   - Yollar katmanı yoksa yol denetlenmez. `missingLayer` yalnız plan sınırı, karar alanı ve plan notu için kalır:
     eski çizimlerin bulguları değişmez.
3. **Sıra:** `outsideUpper` `levelOrder`'dan sonra, `roadGap` `outside`'dan sonra gelir (17 tür). Komut sürümü değişmez:
   yeni türler yalnız yeni durumlarda görünür.
4. **Masaüstü** (İmar planı denetimi):
   - “Üst plan dışına taşma” Plan ve kademe grubunda, “Kopuk yol” Alan grubundadır;
   - yol, Nesne sütununda kimliğiyle görünür;
   - Seç ve göster yolu seçer.

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/zoning_check_cases.py` iki kuralı elle aldı. Durum sayısı 28'den 34'e çıktı;
  KentOS kodu yok.
  - Alt planın dışarıdaki parçası dikdörtgenlerin koordinat sıkıştırmasıyla kesin hesaplanır.
  - Yolun ucunun öbür yolların doğru parçalarına ve plan sınırlarının kenarlarına uzaklığı elle hesaplanır.
  - Yeni durumlar:
    - üst plan dışına taşan alt plan (1000 m²);
    - birbirine ve plan sınırına bağlı yollar;
    - kopuk yol;
    - iki ucu kopuk yol;
    - plan sınırına toleransta yaklaşan yol (bağlı) ve toleranstan uzak kalan yol (kopuk).
  - Karışık planın bulguları arasına UIP-3'ün `outsideUpper`'ı girdi (17 bulgu).

  Rust ve Python geçer.
- **Masaüstü** (`zoning_window_tests::a_broken_road_and_a_plan_outside_its_upper_plan`): kopuk yol Alan grubunda,
  yeriyle söylenir ve seçilir; üst plan dışına taşma 1000 m² ile Plan ve kademe grubundadır. Resimler
  `.run/gui-evidence/imar-denetimi-1420/` altındadır (`kopuk-yol-*`).

## Sonra

- Resmî PlanGML şeması ve eşleme (Bakanlığın XSD'si edinilince).
- Plan değişikliği taslağı.
- Sembolün karardan seçilmesi (MPYY lejantı).
- Yol genişliğinin karar alanı ve parselle tutarlılığı.
