# ADR 0179: Veri karşılaştırma

- **Durum:** kabul edildi (2026-10-05). Sıra sahibin kararıdır: TODOS.md §16.0'ın yirmi birinci işi `HYB-21`; sahibin 5 Ekim kararıyla
  madde tek parçada biter (alt adım yok). Ayrıntılar bu ADR'nin varsayılanlarıdır. Ad Netcad'in “Veri Karşılaştır”ından; ArcGIS'in
  Feature Compare ve Detect Feature Changes'i, QGIS'in Detect dataset changes'i takma adlardır.
- **Bağlam belgesi:** TODOS.md `HYB-21`, `SYNC-11`, `GIS-12`; [araştırma kaydı](../research/2026-10-01-netcad-arcgis-qgis.md);
  CLAUDE.md §5 (koordinat sistemi sorulur, dönüşüm sessiz yapılmaz), §7 (geometrik alan, kayıtlı alan ve gösterim ayrıdır), §23.3
  (toleranslar açık ve ayrıdır); ADR 0159 (Kenar eşleme: çekirdekte eşleme, iki platformda pencere).

## Bağlam

Harita ve kadastro işinde aynı verinin iki sürümü sık karşılaştırılır: yüklenicinin teslim ettiği parseller ile belediyenin elindeki,
güncellemeden önceki ve sonraki çizim, iki ekibin aynı bölgeyi sayısallaştırması. Bugün KentOS'ta iki katmanı ya da iki çizimi
karşılaştıran bir iş yoktur; kullanıcı farkı gözle arar.

## Karar

### 1. Girdiler

- **Eski** ve **Yeni** birer katman, grup (altındaki bütün katmanlar) ya da bütün çizimdir. Yeni açık çizimdendir; Eski açık çizimden
  ya da başka bir çizim dosyasından (`.kcad`, v1 ve v2; Dosyadan…) seçilir. Başka çizim açık çizimin yerine geçmez, yalnız okunur.
- İki tarafın koordinat sistemi aynı olmalıdır: başka çizimin SRID'i (ya da tanımı) açık çizimden başkaysa karşılaştırma yapılmaz ve
  bu söylenir; dönüşüm yapılmaz (CLAUDE.md §5).
- Açık çizimde aynı katman iki tarafta birden olamaz (bir taraf öbürünü içeriyorsa reddedilir).

### 2. Eşleme

- **Konumla** (varsayılan): aynı türden iki nesne, konum farkları (§3) arama uzaklığı içindeyse adaydır. Adaylar konum farkı
  küçükten büyüğe, eşitlikte eski nesnenin, sonra yeni nesnenin sırasıyla birer birer eşlenir; her nesne en çok bir eş alır.
- **Anahtar alanla:** seçilen özniteliği (ör. ParselNo) aynı değeri (uçlarındaki boşluklar kırpılmış, harfler olduğu gibi) taşıyan
  iki nesne eşleşir, türleri farklı olsa da. Anahtarı boş olan ya da anahtarını iki taraftan biri yineleyen nesne eşleşmez; “Anahtar
  sorunu” olarak ayrıca listelenir.
- Eşi olmayan yeni nesne **Eklenen**, eşi olmayan eski nesne **Silinen**dir.

### 3. Konum farkı ve geometri

- Her nesnenin tanım noktaları vardır: nokta (çok noktalıda bütün noktaları); çizgi: iki ucu (yönsüz); çoklu çizgi: köşeleri ve
  yaylı kenarların orta noktaları, yönsüz, parçaları sırasıyla; alan: halkanın köşeleri ve yaylı kenarların orta noktaları (başlangıcı
  ve yönü serbest), delikleri ve parçaları sırasıyla; daire: merkezi; yay: merkezi, başlangıç ve bitiş noktası; yazı: yerleşim noktası;
  blok yerleştirmesi: yerleşim noktası. Öbür türlerin (elips, eğri, yardımcı çizgi, ışın, ölçü, tarama, kılavuz) tanım noktası sınır
  kutusunun merkezidir.
- **Konum farkı:** iki nesnenin tanım noktaları aynı yapıdaysa (aynı sayıda yol ya da halka, her birinde aynı sayıda nokta) karşılıklı
  noktaların en büyük uzaklığı (yönsüz yolda iki yönün, halkada bütün başlangıç ve iki yönün en iyisi); yapı farklıysa sınır
  kutularının merkezleri arasındaki uzaklık. Türleri farklı iki nesnenin konum farkı yoktur.
- **Geometrisi aynı:** türü ve yapısı aynı, konum farkı toleransı aşmıyor, uzunluk ölçüleri (dairenin ve yayın yarıçapı, yazının
  yüksekliği) en çok tolerans kadar farklı, öbür değerleri (yazının metni, dönüklüğü, hizası, genişlik çarpanı; bloğun kendisi,
  ölçeği, dönüklüğü, aynası; öbür türlerin bütün geometrisi) aynı. Aksi hâlde geometrisi değişmiştir.
- Tolerans ve arama uzaklığı metredir; ikisi ayrıdır (CLAUDE.md §23.3): arama uzaklığı neyin eş olabileceğini, tolerans eşin aynı
  sayılıp sayılmayacağını söyler.

### 4. Öznitelikler

- İki nesnenin öznitelik alanlarının birleşimi karşılaştırılır; değeri farklı alanlar (yalnız bir tarafta olan alan, öbüründe boş
  sayılır) “değişen alanlar”dır, adlarının harf kodlarının sırasıyla (çizim alanlara kendi sırasını vermez: masaüstü onları sıralı
  tutar, dosya başka sırayla yazabilir); pencerenin alan listesi de bu sıradadır. Karşılaştırılmayacak alanlar seçilebilir
  (ör. güncelleme tarihi); anahtar alanla eşlemede anahtar alan karşılaştırılmaz (kırpılmış değeri zaten aynıdır). Değerler metin
  olarak, olduğu gibi karşılaştırılır.

### 5. Sonuç

- Her eş ya da eşsiz nesne bir satırdır: Durum (Eklenen, Silinen, Geometrisi değişti, Öznitelikleri değişti, Geometrisi ve
  öznitelikleri değişti, Aynı, Anahtar sorunu), Tür, Anahtar, eski ve yeni nesnenin katmanı, konum farkı (m), değişen alanlar. Satırlar
  yeni tarafın sırasıyla, sonra silinenler eski tarafın sırasıyla. Özet her durumun sayısını söyler. “Yalnız farklar” (varsayılan açık)
  aynıları listeden çıkarır.
- Satıra tıklanınca açık çizimdeki nesnesi (yenisi, yoksa eskisi) seçilir ve görünüm ona gider; başka çizimdeki eski nesne için
  yalnız görünüm onun yerine gider.
- **Panoya kopyala** (sekmeyle) ve **CSV olarak kaydet…** (ADR 0177 §6'nın biçimi: UTF-8 BOM, noktalı virgül, CR LF, ondalık virgül).
- **Farkları çizime yaz:** “Karşılaştırma” grubunda üç katman: Eklenen (yeşil), Silinen (kırmızı), Değişen (sarı); yeni tarafın
  eklenen ve değişen nesnelerinin, eski tarafın silinen nesnelerinin kopyaları, özellikleriyle; tek geri alma adımı “Veri karşılaştır”.
  Adları alınmışsa ağaçta tek olan yeni adlar verilir. Renkli fark görünümü budur: katmanlar gizlenip gösterilir, silinir.

### 6. Pencere ve yer

- **Veri karşılaştır** (`data.compare`; takma adlar VERIKARSILASTIR, COMPARE, FEATURECOMPARE): Eski (kaynak: Bu çizim ya da
  dosyanın adı; Dosyadan…; katman ya da grup, ya da Bütün çizim), Yeni (bu çizimin katmanı, grubu ya da bütünü), Eşleme (Konumla,
  Anahtar alanla), Anahtar alan (iki tarafın alanları), Arama uzaklığı (varsayılan 1 m), Tolerans (varsayılan 0,001 m), Karşılaştırılmayacak
  alanlar, Yalnız farklar; Karşılaştır; sonuç tablosu ve özeti; Panoya kopyala, CSV olarak kaydet…, Farkları çizime yaz, Kapat.
- Şeritte CBS'nin Analiz sekmesinde ve CAD'in Yönet sekmesinde; Düzen menüsünde; komut arama ve takma adlar.

### 7. Hesap

- Eşleme, konum farkı ve sınıflama ortak çekirdektedir (`kentos_geometry_core::ops::compare`, işlemi `dataCompare`); web WASM ile,
  masaüstü doğrudan çağırır. Bağımsız başvuru `scripts/fixtures/compare_cases.py` (noktalar ve uzaklık kareleri kesin kesirlerle,
  yayın uçları 50 basamaklı mpmath ile) `fixtures/compare/v1/cases.json`'ı yazar; iki platform onu geçer.
- Büyük veride adaylar sınır kutularıyla süzülür (x'e göre sıralı tarama); her eşleşme ve sınıflama nesne başına bir kez yapılır.

## Kapsam dışı

- Koordinat dönüşümü; DXF, Shapefile ve GeoJSON dosyalarıyla doğrudan karşılaştırma (önce içe aktarılır, sonra katmanlar karşılaştırılır).
- Bulut projesinin revizyonları arası karşılaştırma (`SYNC-11`), zamansal katman (`GIS-12`).
- Bölünme ve birleşmenin (bir parselin ikiye ayrılması) ayrıca tanınması: bunlar silinen ve eklenen olarak görünür.

## Uygulama

Tek parçada (sahibin kararı): çekirdek ve bağımsız başvurusu; web `model/ops/compare.ts`, `app/dataCompare.ts`,
`ui/data/DataCompareDialog.ts`; masaüstü `data_compare.rs`; komut, ikon, şerit ve menü; ortak iz `data-compare.json` (sahne
`data-compare.kcad`), `shot` adımlarıyla iki platformda resimler; envanter. **Tamam (5 Ekim):** çekirdek
`kentos_geometry_core::ops::compare` (işlem `dataCompare`; adaylar büyütülmüş kutularla süzülür) ve bağımsız başvuru
`compare_cases.py` (mpmath 60 basamak; beş durum: konumla, dar arama, anahtarla, halkalar ve yaylar, sorunlu anahtarlar); web WASM
ile aynı durumları geçer. Pencere iki platformda: Eski için Bu çizim ya da Dosyadan… (`.kcad`), katman, grup ya da Bütün çizim;
Yeni bu çizimin; Eşleme (Konumla, Anahtar alanla), Anahtar alan, Arama uzaklığı, Tolerans, Karşılaştır; karşılaştırılan
öznitelikler kutularla; Yalnız farklar; satırlar durumlarının renginde, tıklanan satırın nesnesi seçilir ve gösterilir; Panoya
kopyala ve CSV olarak kaydet… (ADR 0177 §6'nın biçimiyle); Farkları çizime yaz. Pencere açılınca Eski ilk (etkin olmayan) katman,
Yeni etkin katmandır. Komut `data.compare` (ikon `dataCompare`): CBS'de Analiz, CAD'de Yönet sekmesinde, Analiz menüsünün
Karşılaştırma bölümünde. Ortak iz `data-compare.json` (`layers` beklentisiyle tek geri alma adımı).

## Doğrulama

- Çekirdeğin kuralı ortak durumlarla, bağımsız başvurudan; iki platform aynı dosyayı geçer.
- Pencere ve Farkları çizime yaz ortak izle (tek geri alma adımı dahil); resimler iki temada, 1440 × 900 ve 1100 × 650.
