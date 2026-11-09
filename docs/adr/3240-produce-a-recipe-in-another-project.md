# 3240 — Tariften başka projede üretim

Tarih: 2026-11-09. plan/map-cad.md MAP-20, beşinci tur. Durum: taslak. ADR 2440, 2840 ve 1940'ın “Sonra”sındaki
“tariften başka bir projede yeniden üretim”. Planın kabulü: “yeniden açıp aynı tariften üret”.

İlgili:
- [ADR 1720](1720-map-project-tree-and-rebuild.md): Harita projesinde proje ağacı ve yeniden üretim;
- [ADR 1940](1940-project-graph.md): İş grafiği ve Tarifi kaydet…;
- [ADR 2440](2440-rebuild-as-a-job-progress-and-stop.md): yeniden üretim bir iş, ilerleme ve durdurma;
- [ADR 2840](2840-drawing-record-version-and-revision.md): çizim kaydında işlem sürümü ve revizyon.

## Dayanak

- **QGIS, grafik modelleyici**
  ([The graphical modeler](https://docs.qgis.org/latest/en/docs/user_manual/processing/modeler.html), 9 Kasım 2026'da
  okundu):
  - model `.model3` dosyasına ya da projeye gömülerek saklanır;
  - girdileri çalıştırılırken kullanıcının verdiği parametrelerdir.

  Aynı iş başka projede, girdileri o projenin katmanları olarak yeniden koşar.
- **ArcGIS Pro, geoprocessing geçmişi**
  ([Geoprocessing history](https://doc.esri.com/en/arcgis-pro/latest/help/analysis/geoprocessing/basics/geoprocessing-history.html)):
  - her aracın girdisi, çıktısı, parametreleri, ortamı, zamanı ve durumu kaydedilir;
  - kayıt aynı ayarlarla yeniden açılır;
  - geçmiş projesiyle saklanır. Başka yerde kullanmak için Python betiğine çevrilir ya da modele eklenir.
- **KentOS bugün:**
  - İş grafiği'nin Tarifi kaydet…'i her çizimin tarifini yazar (ADR 1940): komut, sürüm, saklanan girdi (ana kaynak
    kimliğiyle), kaynak düğümleri (kimlik ve ad);
  - tarif yalnız okunuyordu. Başka projede aynı çizimleri üretmek için her komut elle yeniden koşulmalıydı;
  - kimlikler projeye özgüdür (`t-5`), adlar ise insanın verdiğidir (“Düz”). Çizim komutları kaynağı kimlikle ya da
    adla bulur (büyük küçük harf ayrımsız).

## Karar

1. **`road.project.produce` v1** (Tariften üret; Harita ve Yol; tek geri alma adımı “Tariften üret”):
   - **girdi:** `path` (tarifin mutlak `.json` yolu: İş grafiği'nin graf biçimi), `drawings` (yalnız bunlar, tarifteki
     kimlikleri; verilmezse tarifin bütün çizimleri), `expectedRevision`;
   - **her çizim** tarifteki komutuyla, tarifin sakladığı girdiyle çizilir; tarifin sırasıyla.
2. **Kaynaklar adlarıyla:**
   - girdideki bir yazı tarifin bir kaynak düğümünün (yüzey, aks, boykesit, kesit takımı, dever, tip kesit, koridor,
     otokorkuluk, menfez, istinat duvarı, kavşak, platform) kimliğiyse o düğümün adı olur;
   - komut adı bu projede kendi kuralıyla bulur;
   - katman (`layerId`) kimliğiyle kalır. Aynı şablondan açılan projelerde katman kimlikleri aynıdır; yoksa komutun
     reddi söyler.

   Tarifin kimliği bu projede başka bir kaynağın kimliği olsa da ad seçer.
3. **Sürüm:** tarifteki sürüm bu KentOS'un çizdiği sürüm değilse, ya da komut çizen bir komut değilse ret
   (`invalid_project_recipe`, `path`'te).
4. **Var olan çizim:** projede aynı komutla, aynı ana kaynaktan, aynı katmana çizilmiş çizim varsa çizim `left`'te
   `exists` ile kalır: “Projede bu çizim var (aynı komut, ana kaynak ve katman); Yeniden üret ile güncellenir.”
   Tarif iki kez uygulanınca ikincisi hiçbir şey yazmaz.
5. **Hepsi ya da hiçbiri:**
   - **adıyla istenen çizimler** hepsi çizilir ya da hiçbiri. Komutun reddi tarifin çizimini adlayarak gelir: “Tarifin
     “cz-3” çizimi (Sıfır hattı): “Ova” kimlikli ya da adlı yüzey projede yok.” (kod ve yol komutunki);
   - **bütün çizimler istenince** burada çizilmeyen çizim (denetimi reddeder) `left`'te `refused` ile, komutun sözüyle
     kalır; ötekiler çizilir. Denetimden sonra gelen bir ret her durumda bütün yazmayı geri alır.
6. **Kayıt:** her çizim bu projenin çizimi olarak kaydedilir: yeni kimlik (`cz-n`), kaynak damgaları, nesneleri, işlem
   sürümü ve revizyonu. Proje ağacında ve İş grafiği'nde görünür, yeniden üretilebilir. Cevapta her çizimin tarifteki
   kimliği (`from`), buradaki kimliği, ana kaynağı (tür, kimlik, ad) ve nesneleri vardır. Plan nesne yazmaz.
7. **Sistem:** tarif başka koordinat sistemindeki (ya da yerel) bir projede kaydedilmişse uyarı `recipe_other_system`:
   “Tarif TUREF / TM36 (EPSG:5256) projesinde kaydedilmiş; bu proje TUREF / TM30 (EPSG:5254). Girdiler aynen
   alınır.”
8. **İş:** ilerleme çizim çizimdir, durdurma çizimler arasında duyulur (ADR 2440). Durdurulan üretim hiçbir şey
   yazmaz.
9. **Ortak parça:** yeniden üretimin çizim komutlarını koşan parçası (`run_drawing`: denetim, eski nesnelerin
   silinmesi, yazma) ortaklaştırıldı. Yeniden üretim kendi nesnelerini siler, üretim hiçbir şey silmez.
10. **Masaüstü:** İş grafiği penceresinde **Tariften üret…** (Tarifi kaydet…'in yanında):
    - tarif dosyası seçilir;
    - tablonun üstünde ne üretildiği (“Tariften üretildi: 2 çizim, 3 nesne: cz-1 (Düz), cz-2 (Teras).”) ve ne
      bırakıldığı nedeniyle yazılır; tablo yeniden okunur.
11. **Kapsam işareti:** ADR 1720 ve 1940'taki gibi **Harita + Yol** (proje ağacı olan projeler).

## Sınırlar

- Katmanlar adla eşlenmez. Tarifin katmanı bu projede yoksa çizim reddedilir ya da bırakılır.
- Tarif bir çizimin girdisini taşır, kaynağın kendisini taşımaz. Yüzeyin noktaları, aksın geometrisi bu projenindir;
  aynı adlı başka bir yüzey başka bir çizim verir. Bu kasıtlıdır: tarif “nasıl”, proje “neyle”.
- Bellek ve CPU bütçesi, çizimin kendi hesabının içinde durdurma bu ADR'de yoktur.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/road_project_produce_cases.py` → `fixtures/commands/v1/road.project.produce.json`,
  31 durum; tarifler `fixtures/recipes/v1` altında elle; KentOS kodu yok):
  - sıfır hatlarının çizgi sayıları road_terrain_zeroline_cases.py'den: Düz'ün üstünde Tümsek bir kapalı çizgi, Teras'ın
    üstünde Set iki açık çizgi;
  - iki sıfır hattı adlarıyla (a-5 → t-5, Set adıyla), plan, doğrulama, yalnız biri;
  - ikinci kez (ikisi de `exists`), projede var olan çizim bırakılır, başka katmandaki aynı çizim yine üretilir;
  - tarifin katmanına, gizli katmana (komutun uyarısı), Harita projesi;
  - başka sistem ve yerel proje uyarıları;
  - kimliği çakışan tarif (ad seçer), büyük harfli adlar;
  - eksik yüzey (hepsi istenince `refused`, adıyla istenince ret), eksik, kilitli ve grup katman (adıyla: hiçbiri
    yazılmaz);
  - dosya ve tarif retleri, mimari proje, revizyon.
- **Rust ve Python ortak durumları** geçer. `road.project.rebuild`, `graph`, `query` ve sıfır hattı durumları da ortak
  parçadan sonra aynen geçer.
- **Rust birim testleri** (`road_project_produce::tests`, 3 test): durdurulan üretim hiçbir şey yazmaz; ilerleme 1'de
  biter; kayıt üzerinden üretilen çizimler proje ağacında güncel, İş grafiği'nde tarifleriyle, tek geri alma “Tariften
  üret” hepsini geri alır.
- **Bağımsız Python testi** (`python/tests/test_recipe_produce.py`):
  - A projesinin yüzeyleri başka kimliklerle (a-5 …); sıfır hatları A'da çizilir, grafik Python'un `json`'uyla yazılır;
  - B'de tariften üretilen çizgiler başvurunun sayısı ve plan uzunluklarıyla ve A'nın çizgileriyle koordinat koordinat
    aynıdır;
  - B'nin grafiği çizimleri B'nin kimlikleriyle tutar.
- **MCP** (`crates/native/mcp/tests/recipe_produce.rs`): `.kcad` olarak açılan projede ortak durumun cevabı; grafik iki
  güncel çizim; ikinci çağrı iki `exists`; eksik yüzeyli çizim adıyla istenince ret ve hiçbir şey yazılmaz.
- **Masaüstü** (`project_graph_window_tests::tariften_uret_draws_the_recipe_in_another_project`):
  - Harita projesinde Tarifi kaydet…, Yol projesinde Tariften üret: sıfır hattı orada güncel çizim, pencerenin satırı,
    ileti;
  - ikinci kez bırakılır, bir geri alma çizimi götürür;
  - resimler `.run/gui-evidence/tariften-uret-3240/` (açık geniş, koyu dar).

  Çapraz denetimler geçer.

## Sonra

- Katmanların da adla eşlenmesi ya da tarife katman tanımlarının (ad, stil) girmesi.
- Bellek ve CPU bütçesi; bir çizimin kendi hesabının içinde durdurma (koridorun kesitleri arasında).
- Ölçüm arşivinin zincire bağlanması; atlas sayfalarının denetimi.
- Web (TODOS §0.2).
