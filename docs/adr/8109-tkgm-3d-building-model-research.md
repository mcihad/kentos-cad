# ADR 8109: Kat irtifakı için TKGM'nin üç boyutlu sayısal yapı modeli — araştırma ve karar: biçim standardı okunana kadar ertelenir

- **Durum:** karar (17 Kasım 2026): **ertelendi.** Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-19 ("Kat irtifakı için
  3B yapı modeli (araştırma önce)"), B23. Araştırma yapıldı (§1); kod yazılmadı ("kaynak okunmadan kod yazılmaz"). Yeniden
  açılma koşulu §3'te.
- **Bağlam:** araştırma notu [2026-11-15-bim-ifc-yonetmelik.md](../research/2026-11-15-bim-ifc-yonetmelik.md) §9; bina
  programının bağımsız bölümleri ve eklentileri (ADR [0773](0773-building-program-units-and-design-brief.md)), katlar ve
  mahaller; EK-6'nın bağımsız bölüm alanları (IFC'ye yazılan).
- **Web, `.kcad`, komut, bağımlılık:** değişmedi.

## §1. Araştırma (17 Kasım 2026)

| Kaynak | Ne okundu | Ne okunamadı |
|---|---|---|
| TKGM Mevzuat Bilgi Sistemi (<https://mevzuat.tkgm.gov.tr>, duyurular ve arama) | Kadastro Dairesi Başkanlığının "Kat İrtifakı ve Kat Mülkiyeti Tesisi İşlemlerinde Üç Boyutlu Sayısal Yapı Modeli Hakkında Genelge" konulu **2025/2** sayılı genelgesi, 13.02.2025 tarihli ve 15554789 sayı ile yayımlandı | Belgenin kendisi: bağlantısı bu ortamdan açılmadı (sitenin oturumunu ister) |
| Lexpera, genelgenin derlenmiş metninin açık kısmı | Kabul ve yürürlük 13.02.2025, son değişiklik ve güncel hâl 25.02.2026. Amaç: kat irtifakı ve kat mülkiyeti işlemlerinde üç boyutlu sayısal yapı modelinin nasıl üretileceği; belediyeler ve öbür ruhsat idareleri, kadastro ve tapu müdürlükleri. Dayanak: 4 sayılı Cumhurbaşkanlığı Kararnamesi 480/1-ç, 634 sayılı Kat Mülkiyeti Kanunu 12 ve 14, Planlı Alanlar İmar Yönetmeliği. Tanımlar: model Genel Müdürlüğün internet sitesinde yayımlanan standartlara uygun, onaylı mimari proje esas alınarak düzenlenir; **yapı aplikasyon projesinin ekidir ve o projenin müellifinin sorumluluğunda** üretilir. Otomatik Doğrulama Uygulaması: Genel Müdürlüğün web arayüzü; modelin geometrik ve öznitelik denetimini yapar, tapu, kadastro ve MAKS verisiyle kendiliğinden bütünleştirir. Doğrulama Kodu: denetimler başarılıysa verilen altı haneli harf ve rakam | 4–8. maddeler (genel hükümler, ruhsat idaresinin, kadastro ve tapu müdürlüğünün görevleri), biçim, koordinat sistemi, ayrıntı düzeyi, bağımsız bölüm ve eklenti kimlikleri, geçiş hükmü: abonelik ister |
| TKGM'nin sitesindeki değişiklik genelgesi (2026-02 tarihli PDF bağlantısı) | — | Bağlantı sitenin ana sayfasına döndü |
| FIG 2022 ve 2024 bildirileri (Aslan, Cankurt vd.; Bayramoğlu, Dursun vd.), ISPRS 2021 | TKGM'nin CityGML'i bağımsız bölüm için genişlettiği (kat bilgisi, bağımsız bölüm numarası, cephe bilgisi), modellerin LoD 0–2 düzeylerinde üretildiği, kontrolü tamamlanmış GML'in TKGM'nin 3B şehir modelleri sisteminde tutulduğu | İkincil kaynak: değerleri ADR'nin kararına dayanak yapılmadı |
| Basın (2024–2025) | Zorunluluğun 1.1.2025'te başladığı, modelin ruhsat aşamasında üretileceği ve TAKBİS'le bütünleşeceği | İkincil kaynak |

## §2. Karar

- **K197 — Ertelendi:** TKGM'nin modeli için yayımladığı standart (biçim şeması, ayrıntı düzeyi, bağımsız bölüm ve eklenti
  kimlikleri, koordinat sistemi) ve genelgenin 4–8. maddeleri birincil kaynaktan okunamadı. Değer uydurulmaz: KentOS bu
  modeli yazmaz, biçimini tahmin etmez.
- **K198 — Kimin işi:** genelge modeli yapı aplikasyon projesinin eki ve o projenin müellifinin sorumluluğunda sayıyor;
  mimari projenin müellifinin değil. KentOS'ta yapılırsa harita modülünün (yapı aplikasyonu) işidir; mimari model onu
  besler: katların kotları ve yükseklikleri, bağımsız bölümlerin ve eklentilerinin mahalleri ve numaraları, dış kontur.
  Mimari IFC ([ADR 8047](8047-ifc-building-export.md)) ve EK-6'nın bağımsız bölüm alanları ayrı yükümlülüktür, bu modelin yerini tutmaz.
- **K199 — Yeniden açılma koşulu:** TKGM'nin standart belgesi (GML şeması ya da teknik dokümanı) ve genelgenin tam metni
  okunduğunda (TKGM'den resmî kopya, kurumun verdiği örnek dosya ya da sahibin sağladığı metin) madde açılır: önce bir
  örnek dosyanın Otomatik Doğrulama Uygulamasından geçmesi kabul kapısı olur.

## §3. Sıradaki adımlar

- Genelgenin tam metnini ve TKGM'nin standart belgesini sahipten ya da kurumdan almak (B17'nin pilotuyla birlikte
  sorulabilir).
- Okununca: harita modülünde "3B sayısal yapı modeli" dışa aktarımı, mimarinin bağımsız bölüm ve kat verisinden; doğrulama
  kodunun projede saklanması.
