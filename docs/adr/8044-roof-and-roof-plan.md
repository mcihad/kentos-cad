# ADR 8044: Çatı ve çatı planı — saçak çizgisinden çatı yüzleri, mahya, kırma, dere; çatı planı

- **Durum:** sürüyor (10 Kasım 2026); `plan/architecture-advanced.md` ADV-20, M44. 1. bölümün ilk adımı yapıldı:
  çekirdek geometri (§2). Sırada çatı nesnesi, plan çizimi ve araç (§1, §3, §4).
- **Bağlam belgesi:** ADR 8040 (düşey model: kat yükseklikleri, döşemenin katmanları), ADR 8042–8043 (kesit ve görünüş:
  "çizilmeyen: çatı"), ADR 8008 (EK-4 katman anahtarı), ADR 8011 (ölçeğe göre ayrıntı).

## Bağlam

KentOS'ta çatı nesnesi yok: döşemenin "Teras" rolü düz çatıyı karşılıyor, eğimli çatı ise elle çiziliyor. Kesit ve
görünüş (M42–M43) çatıyı "çizilmeyen: çatı" diye söylüyor; ruhsat takımında çatı planı paftası yok. PAİY 57(2)(a) mimari
projede çatı planını ister.

## Araştırma

### Türkiye mevzuatı

- **PAİY Md. 40:** (2) çatı eğimi idarece, yöresel mimari ve iklime göre belirlenir; (4) mahya yüksekliği en çok 5,00 m;
  (5) merdiven evi, ışıklık ve baca çatı örtüsünü en çok 0,60 m aşabilir; (6) düz çatı parapeti en çok 1,10 m, kullanılan
  terasta 1,10 m parapet ya da korkuluk; (7) çatı arasında bağımsız bölüm yapılamaz.
- **PAİY Md. 42:** saçak en çok 1,00 m (plan aksini söylemiyorsa), parsel sınırını aşamaz; çıkma varsa çıkmadan 0,50 m.
- **Sivas İmar Yönetmeliği:** 40 — mahya 4,00 m'de kaldıkça eğim en çok %40; en az iki yöne eğim (bina derinliği 11,50
  m'ye kadar tek yöne izin); 5 kata kadar kırmızı kiremit; konutta tonoz çatı yasak; çatı penceresi en çok 1,20 × 1,50 m,
  cepheye en az 1 m, birbirine en az 2 m; 42/5 — saçak en çok 1,00 m ve **kar siperliği zorunlu**.
- **Mimarlar Odası, Mimarlık Hizmetleri Şartnamesi:** 14.3.3 (u) ön projede "Çatı planında; eğimler, su toplama yerleri,
  dereler, tesisat ve asansör çıkıntıları, bacalar ve çatıya çıkış delikleri gösterilir. Gerekli kotlandırma ve
  açıklamalar yapılır." 14.5.3 (w) uygulama projesinde "Çatı planı çizilir. Meyiller, su toplama yerleri, dereler ...
  gösterilir ve gerekli kotlar verilir", (x) "Yağmur iniş boruları gerçek boyutlarında çizilir ve ölçüleri yazılır."
- **EK-2 Tablo 2.3** (katman öğeleri): Çatı CATI, Çatı Kaplama CKAP, Çatı Kaplama Tahtası CKTA, Çatı Taşıyıcı CTAS,
  Oluk OLUK, Yağmur İniş Borusu YIBO, Baca BACA, Harpuşta HARP, Çörten CORT.

### Başka programlar (yalnız görünüş için)

- **Revit, Roof by Footprint** (Revit 2024 Get Started, *Revit Roof Sample File*): taslağın her kenarı "Defines slope"
  ya da değil; dört kenar eğim verirse kırma, karşılıklı iki kenar beşik, bir kenar tek eğim, hiçbiri düz çatı
  (`.run/gui-evidence/mimari-m44/referans/revit-cati-turleri.png`). Kenar başına ayrı eğim verilebilir.

## Karar

### 2. Çekirdek: saçak çizgisinden yüzler (`geom::roof`)

- Çatı saçak çizgisinden (kapalı, en az üç nokta) ve **her kenarın eğiminden** (yüzde; yok: kalkan) kurulur, Revit'in
  "Defines slope"u gibi. Eğimli kenarın düzlemi kenarın çizgisinden içeri doğru o eğimle yükselir; bir noktada çatı,
  oradaki eğimli düzlemlerin en alçağıdır (alt zarf). Dışbükey saçak çizgisinde bu, düz iskeletin (straight skeleton)
  çatısıdır.
- **Yüz:** her eğimli kenar için, düzleminin hiçbir öbüründen yüksek olmadığı yer (alanı varsa). **Hatlar:** kenarlar
  verildiği sırayla, eğimliyse saçak, değilse kalkan; sonra yüzlerin saçakta olmayan kenarları bir kez, yüksek ucundan:
  alçak ucu saçağın dışa dönen köşesindeyse kırma, içe dönendeyse dere, değilse mahya (yatay ya da iki iç nokta arasında
  eğik). Mahya yüksekliği yüzlerin köşelerinin en yükseği.
- Saat yönünde verilen saçak çizgisi ters çevrilir, kenarlar eğimlerini korur.
- **Henüz yok:** içe dönen köşesi olan saçak çizgisi (L, T): düz iskeleti bölünme olayları ister; reddedilir (§9).
- **Bağımsız başvuru:** `scripts/fixtures/roof_cases.py` (KentOS kodu olmadan, tam kesirlerle) 9 durum: 12 × 9 m kırma
  (%33: mahya 3 m, 1,485 m yüksek), beşik (%40: mahya kalkandan kalkana, 1,80 m), tek eğim (%10: 0,90 m), düz, ön %30 arka
  %45 (mahya dik yüze kayar), saat yönünde beşik, kare kırma (mahyasız tepe), 6-8-10 kenarlı beşgen (iki eğik mahya, beş
  kırma), üçgen. İlk beşgen seçimi bir iç teğet çemberli beşgendi: bütün kırmalar tek tepe noktasında buluşuyor, iç
  noktaları birleştiren hattı sınamıyordu; değiştirildi.

## Doğrulama

- `geom::roof` birim testleri 6 (kırma, beşik, tek eğim, düz ve saat yönü, içe dönen köşe reddi, planın okları ve
  yazıları); `tests/all/roofs.rs` başvurunun 9 durumu 1e-7 içinde.

## Sonraya kalan

- §9: içe dönen köşeli saçak (L, T) ve dereler.
