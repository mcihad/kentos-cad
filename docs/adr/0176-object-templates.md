# ADR 0176: Nesne şablonları

- **Durum:** kabul edildi (2026-10-05). Sıra sahibin kararıdır: TODOS.md §16.0'ın on dokuzuncu işi `HYB-19`. Ayrıntılar bu ADR'nin
  varsayılanlarıdır. Arayüzdeki ad sahibin seçimidir (5 Ekim): “Nesne şablonu” (TODOS'taki “çizim kalemi”; “kalem” Netcad'de çıktının
  kalınlık tablosu anlamına da geldiği için, `CAD-08`). Kurumsal şablon takımlarının içeriği (BÖHYY menüleri) sahibin tarifini bekler
  [M]; bu ADR şablonu, saklanmasını ve kullanılmasını kurar.
- **Bağlam belgesi:** TODOS.md `HYB-19`, `CAD-09` (proje şablonları, standart katman ve stil); [araştırma kaydı](../research/2026-10-01-netcad-arcgis-qgis.md);
  ADR 0089 (yeni nesnelerin rengi ve kalınlığı), ADR 0092 (stil yöneticisi ve kitaplık: sistem, Kitaplığım, proje; `.kstil`), ADR 0067
  (standart katmanlar aynı adımda açılır), ADR 0152 (ölçü noktası: ad ve kod), ADR 0144 (blok); Netcad Sayısallaştırma Sihirbazı,
  ArcGIS feature templates ve group templates, QGIS default values.

## Bağlam

Yeni nesne bugün etkin katmana, şeridin Özellikler panelindeki renk ve kalınlıkla çizilir (ADR 0089); sembolü, öznitelikleri ve etiketi
sonradan Öznitelikler'den verilir. Harita mühendisliğinde her nesne türünün (parsel sınırı, bina, yol kenarı, poligon noktası, kot
noktası…) katmanı, rengi, kalınlığı, sembolü ve öznitelikleri bellidir; çizen kişi her nesne için bunları yeniden seçer. Netcad'in
Sayısallaştırma Sihirbazı, ArcGIS'in nesne şablonları ve QGIS'in varsayılan değerleri bu işi tek tıklamaya indirir.

## Karar

### 1. Nesne şablonu

Nesne şablonu, bir nesne türünün çizim reçetesidir:

- **Ad, kategori, açıklama:** kitaplık öğesinin alanları (ad, kategori yolu, açıklama, etiketler).
- **Araç:** hangi çizim aracıyla çizileceği: Nokta, Çizgi, Çoklu çizgi, Kapalı alan, Dikdörtgen, Döndürülmüş dikdörtgen, Daire, Yazı,
  Blok ekle; aracın yöntemi verilebilir (Daire'nin 2 nokta, 3 nokta, teğet yöntemleri).
- **Katman:** katmanın yolu ve adı (“Kadastro / Parsel”); çizimde yoksa şablonun verdiği görünüşle açılır (ADR 0067'nin standart
  katmanları gibi). Katman kimlikle değil adla bulunur: şablon projeler arasında taşınır.
- **Görünüş:** renk, kalınlık ve sembol (kitaplıktaki bir sembolün kimliği); verilmeyen katmanınki kalır.
- **Öznitelikler ve etiket:** nesneye yazılacak öznitelikler (ad ve metin değer) ve etiket. Noktada ad ve kod Nokta aracının Ad ve Kod'u
  gibidir (ADR 0152): ad her noktada artar.
- **Yazı şablonu:** yazının yüksekliği, hizası, zemini. **Blok şablonu:** yerleştirilecek bloğun adı.
- **Çizgi tipi ve tarama:** nesnenin kendi çizgi tipi yoktur; çizgi tipi şablonun katmanınındır. Alanın tarama görünüşü sembolündür
  (sembolün desenli dolgusu); ayrı bir tarama nesnesi grup şablonunun işidir (§5).
- Kodda adı `ObjectTemplate`'tir (kitaplık öğesinin türü `template`).

### 2. Saklama: stil kitaplığında bir öğe türü

Nesne şablonu stil kitaplığının üçüncü öğe türüdür (`kind: "template"`, sembol ve varlığın yanında). Böylece kitaplığın olanları şablona
da geçer:

- **Üç kaynak:** sistem şablonları (KentOS'la gelir, kopyalanır, değiştirilmez; BÖHYY takımı sahibin tarifiyle buraya girer),
  Kitaplığım (kullanıcının, projeler arasında) ve proje şablonları (proje dosyasında, projeyi açan herkes görür).
- **Kategoriler, arama, kopyalama** kaynaklar arasında; Stil yöneticisi şablonları “Şablon” türüyle listeler.
- **Kurumsal dağıtım:** şablon takımı `.kstil` dosyasıyla dışa ve içe aktarılır; dosya şablonun kullandığı sembolleri ve varlıkları da
  taşır. Şablonlu dosya `.kstil` sürüm 2'dir; yazıcı 2'yi yalnız şablon varken yazar.
- **Proje dosyası:** projenin kitaplığı `.kcad`'de zaten saydam öğeler olarak durur (`ProjectStyles`); belge şeması değişmez. Bulutta
  da projenin kitaplığıyla gider.
- Şablonun kuralları (alanlar, denetim, iletiler) iki platformda tek tanımla, ortak `fixtures/style/v1/object-templates.json` ile sınanır.

### 3. Şablonla çizmek

- **Şablon seçilince** katmanı etkin olur; çizimde yoksa şablonun görünüşüyle, yolundaki gruplarla açılır (“Katman ekle” adımı:
  kullanıcı nereye çizeceğini Katmanlar panelinde görür). Katman adla bulunur: o adda birden çok katman varsa şablonun yolundaki, yoksa
  ağaçta ilk olan. Kilitli katmana şablon başlamaz, söylenir.
- Şeridin rengi ve kalınlığı şablonun olur (verilmeyen “katmana göre”); aracı yöntemiyle başlar. Çizilen her nesne şablonun sembolünü,
  özniteliklerini ve etiketini de alır; nokta şablonunun adı her noktada artar.
- **Şablon bırakılınca** (Esc ile araçtan çıkmak, başka araç ya da şablon) renk ve kalınlık önceki değerlerine döner; etkin katman
  şablonun katmanı kalır (kullanıcı orada çiziyordu). Son komutu yinele şablonu yineler.
- Ürün komutları değerleri açıkça alır (CMD-07): oluşturma komutlarının girdisi `symbol` alır, eksik olanlar `label` da; öznitelikleri
  zaten alırlar. Adım aracın adımıdır.
- İstemin başında şablonun adı görünür (“Parsel sınırı · Kapalı alan: …”).

### 4. Şablonlar paneli ve düzenleyici

- **Şablonlar paneli:** sağ dokta Katmanlar, İşlemler ve Bloklar'ın yanında; kategorilere göre şablonlar (resimleri, adları), arama,
  son kullanılanlar; tıklama şablonla çizer. Şeridin Giriş sekmesinde Şablonlar açılır listesi.
- **Şablon düzenleyici:** ad, kategori, araç, katman (var olan katmanlardan seçilir ya da yazılır), renk, kalınlık, sembol (Stil
  yöneticisinin seçme kipi), öznitelik tablosu, etiket; önizleme. “Seçili nesneden şablon” seçili nesnenin katmanını, görünüşünü,
  özniteliklerini ve etiketini şablona alır.

### 5. Grup şablonu

- Grup şablonu bir çizimden birkaç nesne yazar: üyeleri şablonlardır, her biri aynı çizimden kendi nesnesini kendi kuralıyla alır: aynı
  geometri (başka katmana), ötelenmiş çizgi (iki yana, uzaklıkla), köşelere nokta (ADR 0152), ağırlık merkezine nokta ya da yazı.
  Hepsi tek adımda yazılır.

### 6. Var olan nesneye şablon uygulamak

- “Şablonu uygula” seçili nesnelere şablonun katmanını, görünüşünü, özniteliklerini ve etiketini `cad.entities.set` ile tek adımda verir
  (Özellik kopyala gibi); nesnenin türü şablonun aracına uymuyorsa söylenir ve dokunulmaz.

### 7. Kapsam dışı

- BÖHYY ve öbür kurumsal takımların içeriği: sahip tarif eder [M].
- Nesne ile şablon arasında kalıcı ilişki (şablon değişince nesnelerin değişmesi): şablon bir reçetedir, nesne yazılınca kendi başınadır.
- Öznitelik ifadeleri ve alan doğrulaması (tipli öznitelikler `GIS-*`).

## Adımlar

1. Model: şablonun tanımı ve kuralları iki platformda (`model/objectTemplate.ts`, `kentos_native_style::object_template`; ortak
   `fixtures/style/v1/object-templates.json`), kitaplıkta üçüncü öğe türü, `.kstil` sürüm 2, Kitaplığım ve projenin kitaplığı; Stil
   yöneticisinde Şablon türü (kart, ayrıntılar, tür süzgeci). **Tamam (5 Ekim):** 47 ortak kural durumu; kitaplıkta `template`, kopya
   projeye kullanıcının sembolünü ve varlıklarını taşır; `.kstil` okuyucusu 2'yi okur, yazıcı 2'yi yalnız şablon varken yazar, dışa
   aktarma şablonun sembolünü de alır, içe aktarma varlık, sembol, şablon sırasıyla ve kopyada şablonu yeni sembol kimliğine çevirir
   (`kstil.json` web'den yeniden kaydedildi, iki platformda); şablonun resmi kendi sembolü ya da görünüşü (`style/templateSymbol.ts`,
   `object_template::preview_symbol`).
2. Komutlar: oluşturma komutlarının `symbol`'ü (ve eksikse `label`'ı); ortak durumlar.
3. Şablonla çizmek: şablonun katmanı, rengi ve kalınlığı, araçların yazdığı sembol, öznitelik ve etiket; ortak iz.
4. Şablonlar paneli ve şeridin listesi; Şablon düzenleyici, Seçili nesneden şablon; resimler.
5. Grup şablonu; ortak iz.
6. Şablonu uygula; ortak iz.

Her adım iki platformda, ortak fixture'larla, kendi commit'inde ilerler.

## Doğrulama

- Şablonun kuralları ortak durumlarla; `.kstil` sürüm 2'nin okuma ve yazması iki platformda aynı dosyayla.
- Şablonla çizim ve grup şablonu ortak izlerle; komutların yeni alanları ortak komut durumlarıyla.
