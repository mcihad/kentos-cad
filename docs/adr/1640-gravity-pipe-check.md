# 1640 — Boru hattı denetimi: cazibeli hattın eğimi, örtüsü, birimi ve bağlantısı (`cad.pipes.check`)

Tarih: 2026-11-07. plan/map-cad.md MAP-16, ilk dilim. Kabul satırı: “bağlantısız hat, ters eğim, yetersiz örtü ve
yanlış birimi yakala”. Durum: taslak. İlgili ADR'ler:
- [ADR 0209](0209-network-analysis.md): ağlar ve Denetle;
- [ADR 0557](0557-network-terminals-and-subnetworks.md): altyapı ağının terminalleri ve alt ağları;
- [ADR 1520](1520-map-project-terrain-surfaces.md): Harita projesinde arazi yüzeyi.

## Dayanak

- **Civil 3D, boru ağı kuralları:**
  - **Cover and Slope:** en az/en çok örtü ve en az/en çok eğim. Kural akış yönünde, yukarıdan aşağı uygulanır. En çok
    eğim ve en az örtü yalnız denetimdir: aşılırsa nesnede kural ihlali gösterilir, parça değiştirilmez.
  - **Pipe Drop Across Structure:** bir bacaya giren ve çıkan boruların düşüşü; tepe, akar ya da eksenden ölçülür.

  Kaynaklar:
  - [About Pipe Network Part Rules](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-4FBF470E-16EC-497A-82A9-8E84580F3963.htm);
  - [Pipe Drop Across Structure Rule](https://help.autodesk.com/cloudhelp/2015/ENU/Civil3D-UserGuide/files/GUID-92C122FE-ADB6-4D8E-BC0E-24AC927DA584.htm).

  Kural değerleri kullanıcının kural kümesindedir. LandXML'den alınan ağa kural uygulanmaz.
- **Türk kaynakları:**
  - Atıksu Toplama ve Uzaklaştırma Sistemleri Hakkında Yönetmelik (RG 06.01.2017/29940) okundu.
    - Ana metin (md. 11, 17) borunun üstüne 30 cm dolgu ve %92–95 sıkıştırma ister.
    - Eğim, örtü ve çap sayıları EK-1'dedir. EK-1'in bakanlık sunucusundaki PDF'i
      (webdosya.csb.gov.tr) sertifika doğrulamasından geçmedi. Güvenlik denetimi atlatılmadı, EK-1 okunmadı.
  - İller Bankası'nın kanalizasyon şartnamesine erişilemedi (arama 7 Kasım 2026).
  - Ders notları (Balıkesir Üniversitesi, kanalizasyon boykesiti):
    - kanal eğimi tek yönlüdür;
    - yol eğimi ters ise en az eğim alınır.
  - DWA-A 110 ve EN 752'de en az eğim çapa göre değişir; tek bir değer yoktur.

  **Karar:** KentOS sınır değeri koymaz. Sınırlar kullanıcının (idarenin şartnamesi, projenin kendisi) değeridir.
  Verilmeyen sınır denetlenmez.
- **KentOS bugün:**
  - Altyapı ağları vardır (Ağlar, ADR 0209/0557): kenar katmanları, süzgeçler, bacalar, yön alanı, Denetle'nin
    kopuk parçaları.
  - Arazi yüzeyi Harita'da da vardır (ADR 1520).
  - Boruyu bir cazibeli hat olarak (kot, eğim, örtü) denetleyen bir şey yoktu.

## Karar

1. **`cad.pipes.check` v1, salt okunur:**
   - **Borular:** ağın kurulan kenarları (katmanları ve süzgeçleri; ağı İşlemler'in araçları kurar).
   - **Girdiler:**
     - `inverts`: akar kotları iki öznitelikten (`fields`: çizimin başı ve sonu, aralarında boy boyunca doğrusal) ya da
       köşe kotlarından (`heights`);
     - `diameter`: iç çap özniteliği, birimi (mm, cm, m) ve isteğe bağlı kataloğu;
     - `ground`: arazi yüzeyi (`surface`, Harita ya da Yol) ya da bacaların kapak kotu özniteliği (`structures`);
     - `minCover`, `maxCover` (m) ve `minSlope`, `maxSlope` (m/m). Hepsi isteğe bağlıdır.
   - **Akış:** çizim yönündedir; ağın yön alanı “ters” diyorsa tersinedir.
2. **Bulgular**, borunun sırasında ve türlerin sırasında:
   - **Bağlantı:**
     - `detached`: en uzun parçaya bağlı değil (parçalar ağın düğümlerinden).
     - `noStructure`: ağın baca katmanı varken bacasız uç.
   - **Kot:**
     - `noInvert`: akar kotu yok.
     - `invertUnit`: kot yazıldığı gibi örtü sınırlarına uymuyor; /100 (cm) ya da /1000 (mm) okununca uyuyor.
     - `invertIsDepth`: değer derinlik olarak (örtü = değer − çap) uyuyor.
     - İkisi de yalnız iki örtü sınırı ve uçta zemin varken denetlenir.
   - **Eğim**, (yukarı − aşağı) / boy:
     - `reverseSlope` (< 0), `flat` (= 0), `lowSlope`, `steepSlope`.
     - `outletAbove`: bir düğümde çıkan borunun akarı, giren boruların en alçağından yüksek. Civil 3D'nin düşüş
       kuralının sorusudur; düşüş istenmez.
   - **Çap:**
     - `noDiameter`: çap yok.
     - `diameterUnit`: katalogda yok, başka birimde var.
     - `notInCatalog`: katalogda yok.
   - **Örtü**, tepe üstü (akar + iç çap). Et kalınlığı bilinmez, eklenmez. Çap istenmemişse örtü akardan ölçülür.
     - Yüzeyle: düz boruda köşeler ve yüzeyin her üçgen kenarı kesişimi ölçülür. Zemin de tepe de aralarında
       doğrusal olduğundan en az örtü kesindir. Yaylı boruda yalnız köşeler ölçülür (`curved`).
     - Kapak kotuyla: iki uçta ölçülür.
     - Bulgular: `noGround`, `offSurface`, `aboveGround` (< 0), `shallow`, `deep`.
   - Birim hatası önerilir, hiçbir değer düzeltilmez. Çizime bir şey yazılmaz.
3. **Retler:**
   - `invalid_pipe_check` (yeni): boş öznitelik adı; sıfır ya da eksi sınır; en az > en çok; 200'den uzun katalog ya
     da sıfır çap; 100 000'den çok boru.
   - `unknown_network`: ağın kimliği ya da adı, Türkçe büyük-küçük harf ayrımsız.
   - `not_a_pipe_network` (yeni): yol ağı.
   - Yüzey sorulursa: `not_a_road_project`, `unknown_terrain`, `terrain_unbuildable`.
   - `invalid_network`, `tools_unavailable`.
4. **Masaüstü, Boru hattı denetimi penceresi:**
   - **Form:**
     - Ağ;
     - Akar kotu: Öznitelik (Baş, Son) ya da Köşe kotları;
     - Çap: öznitelik, birim, katalog;
     - Zemin: Yüzey, Kapak kotu ya da Yok;
     - Örtü (m) ve Eğim (‰): en az, en çok; boş bırakılan denetlenmez.
   - **Sonuç** formu izler:
     - özet: bulgu sayıları türlere göre;
     - Tümü | Bulgulu süzgeci;
     - kaydırılan tablo: boru, uzunluk, eğim ‰, çap, en az örtü, bulgular;
     - Bulgulu boruları seç ve CSV kaydet.
   - **Şerit:** Harita ve Yol'da Arazi › Drenaj › Boru denetimi; CBS'de Analiz › Ağ analizi menüsü.
   - Açılışta ilk altyapı ağı seçilir; öznitelik adları borulardan tahmin edilir; sınır boştur.

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/pipes_check_cases.py` 30 durum üretir.
  - Bağımsız hesap:
    - zemin, her karesi tek düzlem olan kapalı biçimli bir kafestir;
    - örtü köşelerde ve kafes çizgisi kesişimlerinde ölçülür;
    - parçalar ortak uçlardan bulunur;
    - kurallar elle yazılmıştır; KentOS kodu kullanılmaz.
  - Atıksu ağı:
    - dokuz boru;
    - bir boruda kot cm yazılmış, birinde derinlik yazılmış;
    - bir çap m yazılmış, biri katalogda yok;
    - ters ve sıfır eğim; yüzeyin dışına çıkan ve tek başına kalan boru.
  - Yağmursuyu ağı:
    - köşe kotları;
    - yön alanıyla ters akan boru;
    - yaylı boru;
    - kapak kotu olmayan ızgara.
  - Öbür durumlar:
    - sınırsız, çapsız, katalogsuz, zeminsiz;
    - adla, CBS projesinde kapak kotuyla, Yol projesinde;
    - 16 ret.
  - Durumların sınırları (0,8–3 m, ‰9–50) durumların kendi değerleridir, yönetmelik değeri değildir.

  Rust ve Python (.kcad gidiş-dönüşüyle) geçer.
- **Masaüstü** (`pipes_check_window_tests`):
  - pencerenin cevabı ortak durumunkidir;
  - sayı olmayan sınır söylenir;
  - yağmursuyu ağı kapak kotunu okur;
  - CSV yazılır;
  - Harita, Yol ve CBS sunar; Mimari sunmaz.

  Resimler `.run/gui-evidence/boru-denetimi-1640/` altındadır.

## Sonra

- EK-1'in değerleri okunduğunda sürümlü bir kural paketi. Kullanıcı yine başka değer verebilir.
- Hat profili:
  - zemin, akar, tepe ve örtünün birlikte çizimi;
  - plan değişince denetimin güncellenmesi.
- Basınçlı su (EPANET) ve SWMM adaptörleri: girdi eşleme, çözücü sürümü, yakınsamama.
- Bacada istenen düşüş, disiplinler arası yatay ve düşey açıklık.
- Tasarlanan, uygulanan ve ölçülen durumların ayrı revizyonları.
