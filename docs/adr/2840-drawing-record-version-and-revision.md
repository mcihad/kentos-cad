# 2840 — Çizim kaydında işlem sürümü ve çıktı revizyonu

Tarih: 2026-11-08. plan/map-cad.md MAP-20, dördüncü tur. Durum: taslak. ADR 1940'ın “Sınırlar”ındaki “işlem sürümü
kayıtta yok”, “çıktı revizyonu yok”; planın satırı: “Girdi kimlik/özetleri, işlem sürümü, birim/CRS/epok, parametreler
ve çıktı revizyonu kaydedilsin.”

İlgili:
- [ADR 0911](0911-road-project-tree-views-and-rebuild.md): yol çiziminin kaydı (komut, girdi, kaynakların damgaları, nesneler);
- [ADR 1940](1940-project-graph.md): iş grafiği; tarif;
- [ADR 2440](2440-rebuild-as-a-job-progress-and-stop.md): yeniden üretim bir iş.

## Dayanak

- **STAC Processing uzantısı** (ADR 1940'ın okuması): işlenmiş verinin `processing:software` (yazılım ve sürümü),
  `processing:version` ve `processing:datetime` alanları. Sürüm, ürünün hangi işlemle yapıldığını söyler.
- **ArcGIS Pro'nun geoprocessing geçmişi** (ADR 1940'ın okuması): aracın adı, parametreleri, ortamı, zamanı ve durumu
  projede saklanır. Aynı aracı yeniden koşmak geçmişten başlar.
- **KentOS bugün** (ADR 1940):
  - çizim kaydı komutu, saklanan girdiyi, kaynakların damgalarını, nesneleri ve nesnelerin damgasını tutuyordu;
  - tarifteki `version` çizimi *yeniden çizecek* sürümdü, kaydın kendisi sürüm tutmuyordu;
  - çizimin yazıldığı revizyon yoktu. Çıktıyı yalnız nesnelerin damgası tanımlıyordu.
- **Revizyon:** belgenin revizyonu her düzenlemede, geri almada ve ileri almada değişir. Bir komutun adımı açıkken
  revizyon, adımın kaydedilince yapacağı revizyondur (ADR 0253 §1). Kayıt aynı adımda yazıldığı için yazımın
  revizyonunu doğrudan bilir; ikinci bir yazım gerekmez.

## Karar

1. **Kayıtta iki alan** (`RoadDrawingDef`, isteğe bağlı):
   - `version`: çizen komutun sürümü (bugün her çizim komutu v1);
   - `revision`: çizimin yazımının yaptığı revizyon, komutların cevaplarındaki gibi ondalık rakamlar.

   Yol komutu çizip kaydettiğinde ikisi de yazılır; yeniden üretim kaydı yeniler. Eski kayıtta yoktur, tahmin
   edilmez: “her çizim komutu v1'di” çıkarımı kayda yazılmaz.
2. **Doğrulama:** sürüm en az 1; revizyon boş olmayan, en çok 20 haneli, 64 bite sığan rakamlar. Değilse kayıt
   reddedilir (`drawings[i].version`, `drawings[i].revision`).
3. **`.kcad` şema 209:**
   - yazıcı bir kayıt iki alandan birini taşıyınca 209'u seçer;
   - 208 ve öncesi iki alanı `bad_value` sayar (Rust okuyucusu ve bağımsız okuyucu aynı);
   - örnek `map-drawing-provenance.json/.kcad`; bozuk dosyalar: 208'de alanlar, 0 sürüm, rakam olmayan revizyon;
   - “daha yeni şema” dosyası `schema-version-210.kcad` oldu.
4. **İş grafiği:**
   - tarif kaydın sürümünü (`made`) ve revizyonunu (`revision`) taşır; `version` yine yeniden çizecek sürümdür;
   - masaüstünde **Revizyon** sütunu; sürüm şimdikinden farklıysa Komut hücresi “(vN çizdi)” der.
5. **Kapsam işareti:** ADR 1940'taki gibi **Harita + Yol**.

## Kanıt

- **Sözleşme** (`kentos-contracts`, `a_drawing_record_is_judged`): 0 sürüm ve dört bozuk revizyon reddedilir; alanları
  olmayan kayıt geçer.
- **`.kcad`:**
  - bağımsız başvuru yazıcısı (`scripts/fixtures/kcad_v2_reference.py`) ve bağımsız okuyucu (`tools/kcad/kcad.py`):
    1701 dosya, `expected.json`'daki her dosya aynı sonucu verir;
  - Rust okuyucusu ve yazıcısı aynı (`kentos-kcad`, 242 test): örnek okunur, yazıcının baytları başvurununki.
- **Ortak durumlar:** `road.project.graph` (`scripts/fixtures/road_project_graph_cases.py`):
  - adımda çizilen her çizimden sonra bir `captureRevision` adımı;
  - tarif `made: 1` ve yakalanan revizyonu söyler;
  - yeniden üretimden ve kendiliğinden yeniden üretimden sonra yeni revizyon;
  - 21 durum, 13'ü yeni adımlarla. Bütün ortak durumlar Rust'ta geçer; ilgili durumlar Python'da da.
- **MCP** (`crates/native/mcp/tests/drawing_provenance.rs`): sıfır hattı çizilir; grafiğin tarifi sürüm 1'i ve
  komutun cevabındaki revizyonu söyler; `.kcad` olarak kaydedilip okununca dosya şema 209'dur ve kayıt ikisini
  tutar.
- **Masaüstü** (`project_graph_window_tests.rs`):
  - Revizyon sütunu çizimin yazıldığı revizyonu gösterir;
  - yeniden üretimden sonra yeniden üretimin revizyonunu;
  - Proje ağacı paneli testleri aynen geçer.

  Resimler `.run/gui-evidence/islem-surumu-2840/`.

## Sonra

- Tariften başka projede üretim; bellek ve CPU bütçesi (MAP-20'nin öbür maddeleri).
- Bir çizim komutu v2 olduğunda eski sürümle çizilmiş kaydın “sürümü eski” diye işaretlenmesi.
- Web (TODOS §0.2).
