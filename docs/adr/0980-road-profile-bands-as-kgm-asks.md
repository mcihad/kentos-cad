# 0980 — Boykesit çiziminde KGM'nin bantları

Tarih: 2026-11-06. Durum: kabul (Y73, Y74). Kapsam ROAD-SEC-11'in boykesit paftası, ROAD-DEVER-10 (dever çizelgesinin çizimde
gösterimi); ADR 0902 §8 ve 0978'in devamı.

## Sorun

`road.profiles.draw` çerçevenin altına KentOS'un bantlarını çiziyordu: Km, Siyah kot, Kırmızı kot, Fark, Eğim. KGM'nin
istediği yatay geometri ve dever bantları yoktu, sıra da KGM'ninkinden farklıydı. ADR 0978 bunu açık bırakmıştı.

## Araştırma (6 Kasım 2026)

### Türkiye mevzuatı

- **KGM 2019, Karayolları Kesin ve Ön Projeleri Mühendislik Hizmetleri Teknik Şartnamesi** (s. 6/64):

  > "Profiller alt kısımda sırası ile Kırmızı Kot, Siyah Kot, Kilometre, Yatay Geometri, Düşey Geometri ve Dever Bandı
  > bilgileri olacak şekilde hazırlanacaktır."

  Aynı yerde öteki istekler:
  - eğim yüzde, üç ondalık (ADR 0978);
  - yatay 1/1000, düşey 1/100–1/1000;
  - profil paftaya sığmazsa kot taşıma çizgisi;
  - mevcut yolun ekseni profilde gösterilir;
  - some bilgileri R, sapma açısı **grad**, T, L, dever S, genişleme, Lr/Lt/La.
- **KGM'nin örnek paftaları** (şartnamenin EK-10–13'ü, s. 45–48; Sivas, 16. Bölge): Listeyi birebir izlemez.
  - Profilin üstünde şev, platform ve üstyapı satırları, altında hafriyat ve dever satırları ve km şeridi vardır.
  - Dever satırında sol yan düz, sağ yan kesiklidir; değerler `+% 2.00` biçimindedir.
- **Kırmızı kotun tanımı:** KGM Karayolu Teknik Şartnamesi 2013 (Kısım 100) kırmızı kotu bitmiş kaplama üstü olarak
  tanımlar.
- **Okunamayanlar:** KGM'nin bir "Çizim Standartları" belgesi bulunamadı; yazı yükseklikleri yalnız "teknik resim
  kaideleri"ne bırakılmıştır. Büyük Ölçekli Harita ve Harita Bilgileri Üretim Yönetmeliği (RG 26.06.2018, 30460 Mükerrer)
  profil çizimi kuralı içermez.

### Türk uygulaması

- **İTÜ (Kesten), SÜBÜ (Özden) ve MEB MEGEP "Yol Projesi":**
  - Kırmızı ve siyah kot iki ondalıklı yazılır ve **bant boyunca dik** durur; km `0+000.00` biçimindedir.
  - Yatay geometride aliyman düz çizgidir; kurp **sağa dönüşte yukarı, sola dönüşte aşağı** bir basamaktır. Yanına R,
    T, L ve sapma açısı yazılır.
  - Düşey geometride her doğrunun işaretli eğimi yazılır; her PVI'de KM, KOT, ŞK (kurp boyu) ve K bulunur.
- **Netcad NETPRO:** Varsayılan bantları KM, Z/Arazi-Z, Düşey-Z ve Aramesafe'dir.
  - **Tuzak:** Netcad'in ve KGM EK-11'in "A=" değeri klotoid parametresi değil, **grad cinsinden sapma açısıdır**.
    Araştırmada denetlendi: L/R radyandan grada çevrilince tam o sayılar çıkar.

### Yabancı yazılımlar

- **Civil 3D'nin bant türleri:** Profile Data, Vertical Geometry, Horizontal Geometry, Superelevation, Sectional Data,
  Pipe, Pressure, Cant, Speed.
- **Superelevation bandı:** En çok sekiz çizgi (sol/sağ × iç/dış × şerit/banket) kaydırılarak çizilir; kritik
  noktaları adlıdır.

## Karar

- **Seçenek:** `road.profiles.draw` iki isteğe bağlı girdi alır:
  - `bands: "kgm"`;
  - `superelevation`: boykesitin aksındaki dever çizelgesinin kimliği ya da adı.
- **Varsayılan aynı kaldı:** Bant yoksa ya da `kentos` ise eski bantlar çizilir; eski çizimler bayt bayt aynıdır.
- **Çizilen bantlar, yukarıdan aşağı:**
  - **Kırmızı Kot, Siyah Kot, Kilometre:** Her yuvarlak km'de ve uçlarda bir sütun; değerler bant boyunca dik
    (90°), kotlar iki ondalık, kırmızı kot kırmızı.
  - **Yatay Geometri:**
    - Aksın elemanları bandın ekseninde; aliyman eksende durur.
    - Kurp, sağa dönüşte yukarı, sola dönüşte aşağı bir basamaktır. Klotoid düz ile kurp arasında eğik, iki kurp
      arasında (ove) basamakta kalır.
    - Yazılar: aliyman `L=`, kurp `R= A=…g L=`, klotoid `Kl. A= L=`. (Y73'te `Δ=` yazılmıştı; çizim yazı tipinde Δ
      olmadığı için pafta PDF'e yazılamıyordu, Y76'da KGM'nin yazdığı `A=` oldu, ADR 0982.)
    - "Kl." bilerek eklendi: Türk paftasında "A=" sapma açısı demektir, karışmasın.
  - **Düşey Geometri:** Üç satır yazı vardır ve birbirine binmez:
    - üstte kurpların `PVI ŞK= K=` yazısı;
    - eksenin üstünde eğimler, KGM biçiminde (`+%5.588 L=170.00`);
    - altta PVI'lerin km'si ve kotu.

    Tepe kurbu yukarı, çukur kurbu aşağı bir basamaktır.
  - **Dever:**
    - Sol yan düz, sağ yan kesiklidir (KGM'nin örnek paftası gibi); bandın ekseni %0'dır.
    - Ölçek, aralıktaki en büyük eğim bandın yarısına sığacak biçimde seçilir.
    - Her kırık noktada iki yanın değeri (`-%2.500 +%6.000`) dik yazılır.
    - Çizelge verilmemişse bant "Dever çizelgesi verilmedi" der.
- **Retler:**
  - `superelevation` KentOS bantlarıyla verilirse `invalid_label`;
  - çizelge yoksa `unknown_superelevation`;
  - çizelge başka aksın ise `invalid_label`.
- **Kaynaklar:** Kalıcı çizim (ADR 0911) dever çizelgesini de kaynak sayar; çizelge değişince çizim eskir.
- **Boykesit penceresi:** Çizime satırına "Bantlar" (KentOS, KGM 2019) ve KGM seçiliyken "Dever bandı" (aksın
  çizelgeleri) eklendi.

## Ölçüm

- **Ortak durumlar:** `road.profiles.draw` 43 durum. Yeniler:
  - KGM çizelgesiz;
  - KGM çizelgeli;
  - KGM km 100–300;
  - açıkça `kentos`;
  - üç ret.

  Her KGM çiziminin nesne sayısını bağımsız başvuru ayrı sayar: bant çizgileri ve adları, sütunlar, aralıktaki
  elemanlar, kurplar, eğimler, PVI'ler, deverin kırık noktaları. Rust ve Python tipli katman geçer.
- **Masaüstü testi:** Bant adları KGM sırasıyla yukarıdan aşağı dizilir. Sağ yan kesiklidir. Kalıcı çizim çizelgeyi
  kaynak sayar. Tek geri alma adımıdır.
- **Resimler:** `.run/gui-evidence/yol-y73/` altında.
- **Bulgu:** Yazının dönmesi sözleşmede derecedir; ilk denemede π/2 radyan yazıldı ve değerler yatay kaldı. Resimde
  görüldü ve düzeltildi.

## Y74: plan–profil paftaları

`road.sheets.plan` aynı iki girdiyi alır (`bands`, `superelevation`) ve her boykesit görünümüne geçirir.

- **Aralık:** Görünümler arası aralık KGM bantlarının derinliğine göre açılır: 43 yazı yüksekliği, 2,5 mm'de kâğıtta
  107,5 mm. KentOS bantları 30 mm'dir.
- **Pencere:** Yol paftaları penceresinde "Bantlar" ve "Dever bandı" seçimleri vardır.
- **Ortak durumlar:** `road.sheets.plan` 35 durum. Yeniler "KGM bantlarıyla, 1:1000" ve "KGM bantları ve dever
  çizelgesiyle"; nesne sayılarını başvuru ayrı sayar. Masaüstü testi de geçer.
- **Bulgu (resimle görüldü):** KentOS'un A3 plan–profil şablonunda (`sys:yol-plan-boykesit`) boykesit haritası
  kâğıtta 102 mm yüksektir. KGM bantları buna sığmaz: 1:1000'de Düşey Geometri ve Dever bantları haritanın dışında
  kalır. Komut bunu `road_sheet_outside` uyarısıyla söyler.

  KGM 2019 profil paftalarını "rulo halinde (… 10 veya 20 km'lik kesimler)" ister (s. 13). KGM bantları için bir rulo
  şablonu ayrı bir iştir; plana yazıldı. (Y75 düzeltmesi: o "rulo" Brükner içindir; plan–profil paftası A1'dir, ADR
  0981.)

## Açık

- ~~**KGM rulo şablonu**~~: Y75'te KGM'nin A1 plan–profil paftası oldu (ADR 0981). "Rulo" KGM'de Brükner içindir;
  plan–profil paftası A1'dir.
- **Öteki bant içerikleri:**
  - KGM'nin örnek paftasındaki hafriyat, şev ve platform satırları;
  - kot taşıma çizgisi;
  - mevcut yolun ekseni.
- **Yazı yükseklikleri:** KGM'nin resmî değeri bulunamadı; bantlar `height`'e göre ölçeklenir.
