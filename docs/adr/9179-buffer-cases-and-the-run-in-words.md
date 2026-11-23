# ADR 9179: Tampon'un durumları ayrı sınanır; iş kaydı çalıştırmayı sözle tutar

- **Durum:** kabul edildi (2026-11-23); adım VY-78. Plan: [plan/veri.md](../../plan/veri.md) VY-78.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”; 23 Kasım: “küçük kalanlarla”.
  Değerlendirmenin 11. maddesinin kalanı ([ADR 9144](9144-processing-inputs-follow-the-drawing.md) §4):
  - “İş kaydı girdi, seçim, ölçü birimi, algoritma, süre, hedef ve yazılan sayıyı içersin.”
  - “Coğrafi CRS, metre kullanan CRS, iptal, hatalı geometri ve mevcut çıktı adı ayrı test edilsin.”
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok; ortak durumlar değişmedi.

## §1 Bağlam ve araştırma

- **İşlemler'in Geçmiş kartı** aracın adını, saatini, özetini, okunan ve yazılan kayıt sayısını ve süreyi
  gösteriyordu. Girdinin ne olduğu, uzaklığın birimi ve sonucun hangi katmana yazıldığı yoktu.
- **Var olan sınamalar:**
  - coğrafi projede metre isteyen araç çalışmadan reddedilir (ADR 1135, `metres_in_a_geographic_project…`);
  - Durdur arka plandaki çalıştırmayı bitirir (`durdur_ends_a_background_run…`).

  Ama Tampon'un durumları raporun sorduğu gibi bir arada sınanmıyordu. Kendini kesen alan ve var olan sonuç adı hiç
  sınanmamıştı.
- **Hatalı geometri.** Tampon kendini kesen alanı çizildiği hâliyle alıyor ve bir şey söylemiyordu. CLAUDE.md §23.3
  gizli onarımı yasaklar; ama sessiz geçmek de kişiyi sonuca körü körüne güvendirir.
- **Araştırma.** QGIS 3.44 kullanıcı kılavuzunun “The Processing Toolbox” bölümüne bakıldı
  ([işlem geçmişi](https://docs.qgis.org/3.44/en/docs/user_manual/processing/toolbox.html), okundu 23 Kasım 2026).
  Masaüstü CBS'lerin işlem geçmişi her çalıştırmanın aracını ve parametrelerini, girdileri ve çıktılarıyla birlikte
  tutar; çalıştırma oradan aynı değerlerle yeniden açılır.

## §2 Karar

1. **İş kaydı çalıştırmayı sözle tutar** (`RunRecord::words`).
   - Kayıt hazırlanmış işten yazılır, bu yüzden seçim o anki hâliyle kalır. Her parametre bir satırdır:
     - nesne girdisi çözüldüğü gibi: “Nesneler: 1 çizgi; ‘Yol ekseni’ katmanında”, seçimde katmanın adıyla
       (ADR 9157);
     - sayı birimiyle: “Uzaklık: 10 m”;
     - seçenek kendi adıyla: “Yan: İki yan”;
     - yazılan katman yeni mi var mı: “Çıktı katmanı: ‘Tampon’ (yeni katman)” ya da “… (var olan katmana yazıldı)”.
   - Durdurulan ve hata veren çalıştırma da girdisini tutar. Hiç hazırlanmayanın satırı yoktur.
   - Geçmiş kartı bu satırları özetin altında gösterir. Algoritma kartın başlığı, süre ve yazılan sayı kartın
     altıdır.
2. **Hatalı girdi söylenir, onarılmaz** (`invalid_note`, Tampon'da).
   - Girdinin geometrisi çekirdeğin geçerlilik denetiminden geçmezse çalıştırma şunu yazar: “1 nesnenin geometrisi
     geçersiz (kendini kesen halka): tamponları çizildiği hâliyle alındı, onarılmadı. Geçerliliği denetle ile
     yerlerini görün; gerekiyorsa Onar ile düzeltip yeniden çalıştırın.”
   - Sonuç yine çizilir; geçerli girdide bu söz çıkmaz.
3. **Var olan sonuç adı** o katmana yazar. Aracı ikinci kez aynı adla çalıştırmak aynı katmana ekler (bugünkü kural,
   `resolve_layer`); kayıt bunu “var olan katmana yazıldı” diye söyler.

## §3 Kanıt

- **Masaüstü** (`gis/processing/buffer_cases_tests.rs`). Fikstür TM36 ya da EPSG:4326'da bir GIS çizimi:
  “Yol ekseni”nde 100 m'lik doğu–batı ekseni, “Alanlar”da kendini kesen bir alan (papyon).
  - **Metre:** 10 m tek koridor çiziyor (kutusu 120 × 20 m). Kayıt girdiyi, “Uzaklık: 10 m”yi ve “‘Tampon’ (yeni
    katman)”ı söylüyor.
  - **Coğrafi:** çalıştırmadan önce reddediliyor, hiçbir şey yazılmıyor.
  - **Durdur:** arka plandaki çalıştırma iptal; çizim olduğu gibi; kayıt iptal ve girdisini tutuyor.
  - **Papyon:** tamponu çiziliyor ve “geçersiz (kendini kesen halka) … onarılmadı … Geçerliliği denetle” deniyor.
    Yolun geçerli çizgisinde bu söz yok.
  - **Var olan ad:** ikinci çalıştırma (20 m) aynı “Tampon” katmanına yazıyor (iki koridor, tek katman). Kayıt “var
    olan katmana yazıldı” ve “Uzaklık: 20 m” diyor.
  - İşlemler, şerit, geçmiş, katalog ve ayar sınamaları geçiyor (286). İşlem çekirdeğinin bütün sınamaları ve ortak
    durumları geçiyor.
- **Resim:** `.run/gui-evidence/kullanim-9179/01-is-kaydi-{light,dark}.png`. Geçmiş (3): her kartta girdi,
  uzaklık, yan, halka sayısı, çıktı katmanı (yeni ya da var olan) ve kayıt sayısı görünüyor; haritada iki koridor ve
  papyonun tamponu var.

## §4 Kalan

- Girdiden türetilen sonuç adı (“Yol ekseni — tampon 10 m”). Ortak durumlarla birlikte yapılmalı.
- Geçersiz girdi sözünün öbür geometri araçlarına (kesişim, kırp, birleştir…) yayılması.
- Çalıştırmanın uyarılarının da kayıtta tutulması.
- Gerçek pencerede deneme; web (TODOS.md §0.2).
