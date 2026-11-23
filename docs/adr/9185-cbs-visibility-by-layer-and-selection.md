# ADR 9185: CBS'nin görünürlüğü katmana ve seçime göre; alt şeritte CAD terimi yok

- **Durum:** kabul edildi (2026-11-23); adım VY-84. Plan: [plan/veri.md](../../plan/veri.md) VY-84.
- **İstek:** sahibin sözü (23 Kasım 2026): “büyük arayüz işi (3, 12, 13, 15) ile devam”. Değerlendirmenin 12. maddesinin
  kalanı (ADR 9127 §4):
  - proje türü, yetki, etkin katmanın geometrisi, seçim ve sağlayıcı yeteneği için görünürlük matrisi;
  - CBS'nin alt şeridindeki CAD terimleri (Kalınlık, Model sekmesi);
  - raster, nokta bulutu ve ağ işleme için, uygun katman seçilince bağlamsal araçlar.

  Maddenin kabulü:
  - boş CBS, yalnız nokta, çizgi, poligon, raster, nokta bulutu, salt okunur tablo, düzenlenen katman ve çoklu seçim
    için görünürlük matrisi;
  - CAD projesinden CBS'ye dönünce CAD'e özgü panel, komut ya da eski bağlamsal sekme geri gelmemeli;
  - varsayılan CBS oturumunda arama, kısayol ve sağ tıkla bina IFC komutuna ulaşılamamalı;
  - paylaşılan CAD çalışma alanında ters yönde regresyon sınaması.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok; şeride bir bağlamsal sekme
  eklendi (`ctx.network`).

## §1 Bağlam ve araştırma

- Bağlamsal sekmeler yalnız seçimden açılıyordu: seçilen nesnelerin türü (nokta, çizgi, alan, raster, nokta bulutu…)
  sekmeyi seçiyordu (ADR 0622 §4). Etkin katman bir raster ya da nokta bulutu katmanıyken, bir nesne seçilmeden o
  türün araçlarına ulaşmak için Analiz'i ya da Menülerde ara'yı bilmek gerekiyordu. Ağ analizinin araçları yalnız
  Analiz'deydi.
- CBS projesinin alt şeridinde CAD'in iki alışkanlığı vardı:
  - **Kalınlık lambası:** çizgi kalınlığını gösterme düğmesi;
  - **“Model” sekmesi:** çizimin, paftaların yanındaki sekmesi.
- Komut kararı ADR 9127'den beri tektir (`App::offers`): Menülerde ara, komut satırı, kısayol, hızlı erişim, F1 ve
  çizimin menüleri ona bakar, `run` aynı kararla reddeder. Raporun “duvar” sızıntısı (CBS projesinde arama ve
  çalıştırma) orada kapandı.
- **Araştırma.** ArcGIS Pro 3.8'in proje kurma öğreticisi
  ([create-a-project](https://pro.arcgis.com/en/pro-app/latest/get-started/create-a-project.htm), okundu 23 Kasım 2026):
  bağlamsal sekmeler, programın onları kullanabileceği bir durumdayken şeritte görünür. Öğreticideki örnekte Graphics
  ve Graphics Layer sekmeleri, İçindekiler bölmesinde bir grafik katmanı olduğu için gelir: sekme seçimden değil,
  katmandan doğar.

## §2 Karar

1. **Katmanın bağlamsal sekmesi** (`modes.rs`, `App::layer_tab`). Hiçbir şey seçili değilken etkin katman karar verir:
   - nesnelerinin hepsi raster olan katman: Raster sekmesi;
   - hepsi nokta bulutu olan katman: Nokta bulutu sekmesi;
   - bir ağın kenar ya da düğüm katmanı (`networks[].edges`, `junctions`): yeni **Ağ** sekmesi (yalnız CBS'de).
     Panelleri Ağ analizi (Ağlar, En kısa yol, Hizmet alanı, Şebeke izleme) ve Katman'dır (Tablo, Katman stili, Katman
     süzgeci).
   - Başka her katman: sekme yok. Bir vektör katmanı ilk nesnesinde anlaşılır; büyük katman bir şey ödetmez.
2. **Seçim önce gelir.** `App::context_tab` önce seçimin sekmesini, yoksa katmanınkini verir. Şerit, tuş ipuçları ve
   komutun yerini gösteren arama aynı sekmeyi kullanır.
   - Katmanın sekmesi öbürlerinin yanında, katmanın adıyla durur (“Raster · Ortofoto 2024”); öne kendiliğinden
     geçmez, bir tıkla açılır.
   - Başka katman etkin olunca kapanır ve seçili sekme yeniden görünür.
3. **Alt şerit.**
   - CBS projesinde Kalınlık lambası gösterilmez (`shows_aid`). Çizgi kalınlığı Görünüm › Görünüş'te kalır: yakın
     bakışta çizgileri ince görmek CBS'de de gerekir (çizim ölçeğindeki sembollerle).
   - Çizimin sekmesi CBS'de “Harita”, CAD'de “Model”dir (`kentos_sheet_ui::status::model_name`). Pafta sekmesindeki geri
     dönüş düğmesi de aynı adı taşır (“Haritaya dön”).
   - Orto, Dik açı ve Kutupsal kalır: kadastro çiziminde de kullanılır.
4. **Değişmeyenler.** Bütün modüllü programda harita ve yol işinin CAD projeleri binanın komutlarını sunmaya devam eder
   (ADR 9127 kuralı CBS'ye daraltılmıştı). Raporun sızıntısı CBS projesindeydi: “Yol ekseni” CBS kadastro ağacının
   katmanıdır.

## §3 Kanıt

- **Masaüstü** (`tests/gis_visibility_tests.rs`):
  - `the_contextual_tab_follows_the_selection_and_the_active_layer`:
    - boş CBS'de CBS'nin dokuz sekmesi var, bağlamsal sekme yok;
    - nokta, çizgi, çoklu çizgi ve alan seçimi kendi sekmesini, karışık seçim Seçim'i açıyor;
    - raster katmanı boşken sekme yok, rasterle Raster var ama öne geçmiyor; tıklanınca açılıyor, başka katmanda
      kapanıyor;
    - seçim katmanın sekmesinin önüne geçiyor;
    - nokta bulutu katmanı Nokta bulutu'nu, ağın katmanı Ağ'ı açıyor; Ağ'ın dört aracı sunuluyor.
  - `a_read_only_table_offers_no_drawing_and_one_in_its_session_its_own_geometry`: salt okunur tabloda çizim araçları
    kapalı ve nedenini söylüyor, Tablo ve Katman stili açık. Yazılabilir tablo oturum kapalıyken çizilmiyor; oturumda
    alan çiziliyor, nokta çizilmiyor (tablonun geometrisi).
  - `a_cad_project_left_for_a_cbs_project_brings_nothing_of_its_own_back`: CAD'de ölçü seçiliyken Ölçü sekmesi
    açıktı. CBS projesi açılınca bağlamsal sekme yok, öndeki sekme CBS'nin; Mimari sekmesi, Duvar ve IFC yok. Kalınlık
    lambası yok, Çizgi kalınlığı Görünüm'de; çizimin sekmesi Harita.
  - `the_buildings_ifc_is_out_of_reach_in_cbs_and_in_reach_in_a_buildings_cad_project`:
    - CBS'de iki IFC komutu sunulmuyor, sağ tuşun menülerine girmiyor, aramada çıkmıyor;
    - çalıştırılınca reddediliyor ve nedenini söylüyor;
    - binanın CAD projesinde IFC sunuluyor, menüde ve aramada var; Kalınlık ve Model duruyor, ağ katmanı Ağ sekmesini
      açmıyor.
- Durum çubuğunun sınamaları CBS'nin lambalarına göre güncellendi: Kalınlık'sız son lamba Oto. seçim. Mesajın hücresi
  kipin adına yer açıldığı için daha erken bitiyor.
- **Resimler:** `.run/gui-evidence/kullanim-9185` (Git'e girmez).

## §4 Kalan

- Yetki ve sağlayıcı yeteneğine göre şeridin kendisi: bugün komut açık görünür, çalıştırmada nedenini söyler.
- Kayıtlı panel düzeninde CBS'ye özgü öntanım.
- Gerçek pencerede deneme; web (TODOS.md §0.2).
