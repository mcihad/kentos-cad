# 2440 — Yeniden üretim bir iş: ilerleme, durdurma; GUI, Python ve MCP'de aynı sonuç

Tarih: 2026-11-08. plan/map-cad.md MAP-20, üçüncü tur. Durum: taslak. Plan satırları:
- “Her ağır işlem UI iş parçacığından ayrı yürüsün; ilerleme, bellek/CPU/VRAM bütçesi ve iptal kanalı olsun.”
- Kabul: “aynı küçük üretim işini GUI, Python ve MCP ile yap; … ağır işi iptal et.”

İlgili:
- [ADR 0253](0253-command-protocol.md) §9: komutların iş olarak koşması (kopya üzerinde hesap, sonra kısa yazma; durdurma);
- [ADR 0911](0911-road-project-tree-views-and-rebuild.md) ve [ADR 1720](1720-map-project-tree-and-rebuild.md): proje
  ağacı ve yeniden üretim;
- [ADR 1940](1940-project-graph.md): iş grafiği. “Sonra”sındaki “ağır işlerde ilerleme ve iptal” maddesini bu ADR
  yeniden üretim için kapatır.

## Dayanak

- **QGIS, PyQGIS Developer Cookbook › Tasks**
  ([belge](https://docs.qgis.org/3.40/en/docs/pyqgis_developer_cookbook/tasks.html), 8 Kasım 2026'da okundu):
  - ağır iş arka planda, iş parçacığında koşar ki arayüz yanıt vermeye devam etsin;
  - görev ilerlemesini `setProgress` ile söyler, `isCanceled()`'ı düzenli yoklayıp temiz biçimde bırakır;
  - arayüze dokunan iş `finished()`'dadır, o da hep ana iş parçacığından çağrılır.

  KentOS'un yolu aynıdır: hesap kopyada ve başka iş parçacığında; ilerleme ve durdurma döngüde; yazma pencerenin iş
  parçacığında.
- **KentOS bugün:**
  - iş altyapısı vardı (ADR 0253 §9). Bir komut okuma kopyasında başka bir iş parçacığında hesaplanır. Durdurma
    istenirse kopyanın sonucu atılır. Komut isterse `ExecutionContext::stopped` ile döngüsünde durur, `progress` ile
    ilerlemesini söyler;
  - İşlemler'in toplu çalıştırması bu yoldan koşuyordu: ilerleme çubuğu ve Durdur;
  - proje ağacının **Hepsini yeniden üret**'i ise `road.project.rebuild`'i pencerenin kendi iş parçacığında koşuyordu.
    Koridor, yüzey ve pafta çizimleri büyüdükçe pencere donuyordu, durdurulamıyordu;
  - komut ilerlemesini söylemiyor, durdurmayı dinlemiyordu.

## Karar

1. **`road.project.rebuild` iş olarak:**
   - ilerlemesini çizim çizim söyler (`progress(k, n)`, sonunda `n/n`);
   - her çizimden önce durdurmayı dinler. Durdurulursa `cancelled` döner.
   - İş okuma kopyasında koştuğu için durdurulan yeniden üretim **hiçbir şey yazmaz**: geri alma adımı yoktur, çizim
     aynıdır.
2. **Masaüstü** (proje ağacı paneli, Yol'da “Yol projesi”, Harita'da “Proje ağacı”):
   - **Hepsini yeniden üret** ve satırın ↻'si yeniden üretimi bir iş olarak başlatır;
   - panelin başında ilerleme çubuğu, “Yeniden üretiliyor…” ve **Durdur** görünür. Durdur'dan sonra “Durduruluyor…”
     yazar;
   - pencere iş sürerken kullanılabilir. Panel işi saatle (120 ms) yoklar. İş bitince yazma, pencerenin iş
     parçacığında kısa bir adımda olur (ADR 0253 §9);
   - durdurulan iş için ileti: “Yeniden üretim durduruldu; hiçbir şey yazılmadı.”;
   - iş başladığı çizim kapanırsa durdurulur.
3. **Eşdeğerlik:** masaüstünün işi, Python'un işi (`doc.start(…)`, `job.cancel()`, `job.wait()`) ve düz bir çağrı
   (Python, MCP) aynı komutu ve aynı hesabı koşar. Sonuç aynıdır: aynı çizimler, her birinin sildiği nesne sayısı.
   MCP çağrıları eşzamanlıdır; MCP'de iş yoktur (`jobs.*` sunucu işleridir).
4. **Sınır:** bellek, CPU ve VRAM bütçesi bu dilimde yoktur. Durdurma çizimler arasında duyulur; bir çizimin kendi
   hesabının içinde değil.
5. **Kapsam işareti:** ADR 1720'deki gibi **Harita + Yol**.

## Kanıt

- **Rust** (`kentos-native-application`, `road_project_rebuild::tests`). Ortak durumun projesinde koridor ve
  korkuluk çizilir, okudukları tip kesitin adı değişir; iki çizim eskir:
  - baştan durdurulan yeniden üretim `cancelled` döner, çizim aynıdır;
  - ilerleme çizim çizim gider ve 1'de biter; cevap ortak durumun iki çizimidir (cz-1, cz-2);
  - iş olarak:
    - durdurulan iş `cancelled`, çizim aynı;
    - koşan iş `done`, ilerleme 1, ağaç güncel, geri alma adımı “Yeniden üret”.
- **Python** (`python/tests/test_rebuild_job.py`):
  - durdurulan işte çizimin baytları aynıdır;
  - koşan iş `done`, ilerleme 1. Düz çağrının kopyada yeniden ürettiğini yeniden üretir (çizimler ve sildikleri).
- **MCP** (`crates/native/mcp/tests/rebuild_equivalence.rs`): proje `.kcad` olarak açılır, aynı üç adım koşulur.
  Yeniden üretimin çizimleri ve sildikleri ortak durumunkidir, ağaç güncel kalır.
- **Masaüstü** (`road_project_panel_tests::a_rebuild_runs_as_a_job_and_durdur_writes_nothing`):
  - Hepsini yeniden üret bir iş başlatır;
  - Durdur'dan sonra ileti, iki eskimiş çizim ve değişmeyen geri alma listesi;
  - yeniden başlatılan iş iki çizimi tek adımda günceller.
  - Eski panel testleri işi bitene dek yoklar ve aynen geçer.

  Çapraz masaüstü denetimleri 137 geçer. Resimler `.run/gui-evidence/yeniden-uretim-isi-2440/` (çalışırken ve bitince).
- **Ortak durumlar:** `road.project.*` durumları aynen geçer (eşzamanlı yol değişmedi).

## Sonra

- Bellek ve CPU bütçesi; bir çizimin kendi hesabının içinde durdurma (koridorun kesitleri arasında).
- Çizim kaydında işlem sürümü ve çıktı revizyonu (`.kcad` şema 207).
- ~~Tariften başka bir projede yeniden üretim.~~ ADR 3240 (Tariften üret).
- Atlas sayfalarının denetimi.
