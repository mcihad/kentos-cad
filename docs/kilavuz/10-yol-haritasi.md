# KentOS CAD kılavuzu — §10 Yol haritası

> [CLAUDE.md](../../CLAUDE.md)'nin 10. bölümüdür; 4 Kasım 2026'da kılavuz küçültülürken buraya taşındı. Koddaki ve ADR'lerdeki “CLAUDE.md §10…” atıfları bu dosyayı gösterir; bölüm numaraları korunur.

## 10. Yol haritası

Tek ayrıntılı yol haritası [TODOS.md](../../TODOS.md)'dir. Buraya ikinci checkbox
listesi, eski Faz A–F sırası veya her tamamlanan commit'in dökümünü eklemeyin.
Yapılmış işin durumunu §1'de kısa tutun; kanıtı test/ADR/ölçümde saklayın.
Bir maddenin baştan sona nasıl yapıldığı (sıra, denetimler, resimler, commit, paralel ajanların
numaraları ve çakışan dosyaları): [docs/MADDE-TARIFI.md](../../docs/MADDE-TARIFI.md).

### 10.1 Devir notu (3 Ekim 2026)

İşi devralan için kalınan yer; bir sonraki devirde bu bölümü yenileyin.

- Bitti (3 Ekim, sahibin isteği: “hibrit mod iptal … sahne kurma CAD ve gis için ayrı … proje oluşturma sihirbazla … menüleri bu
  yapılara göre düzenleyelim”): proje türleri ([ADR 0165](../../docs/adr/0165-project-types-cad-gis.md), TODOS.md `PRJ-01`). 1. adım
  (Hibrit kalkar) iki platformda: sözleşmede `hybrid` yalnız okunur (`Workspace::LegacyHybrid`, `ProjectSettings::project_type`),
  belge ve v1 göçü onu yazmaz; türsüz proje CBS gösterilir ve kullanıcının açtığında bir kez sorulur (web `ProjectTypeDialog.ts`,
  masaüstü `project/ask_type.rs`); ayarlarda emekli değer (`RETIRED_VALUES`); envanterde `layout.ribbon` CBS'nin, `ribbonByMode`
  her türün; pafta profillerinde hibrit satırı yok; arayüzde “Çalışma modu” yerine “Proje türü”, bulut kataloğunun etiketi “İş
  türü”. 2. adımın yerel sistemi (SRID 0, “Yerel (koordinat sistemi yok)”) ve çizim birimi (`settings.drawingUnit`, `.kcad`
  şema 11; geometri metrede, birim yalnız okuma ve yazmada: `Formatter.toMetres`, `Format::to_metres`, `Context::typed_point`,
  `typed_length`, `point_from_text_in`; Hesap pencereleri ve İşlemler metrede) bitti; DXF iki yönde birimiyle (2c,
  `formats::units`) bitti. 3. adımdan ileri TM izdüşümü (`geodesy::tm_forward`, PROJ başvurusuyla), 81 il, ana görünüm, CAD'in
  katman şablonu ve web'in sihirbazı (`ui/settings/NewProjectWizard.ts`, kuralları `model/newProjectWizard.ts`, ortak
  `fixtures/project/v1/wizard.json`), masaüstünün sihirbazı (`project/wizard/`, kuralları `kentos_project::wizard`, KentOS UI
  `Wizard::rail`) ve yeni projelerin birimi (`newProjects.drawingUnit`) bitti; 3. adım tamam. 4. adım (eksen ve açı düzeni) bitti:
  kutupsal giriş türün düzeniyle (`point_input::polar_point_in`, `fixtures/point-input/v1`'in `convention`'ı), eksen adları ve
  doğrultular biçimlendiricide (`Format::axes`, `Formatter.axes`), yeni CAD projesi derece. 5. adım (sahne) bitti: CAD'de koordinat
  ekseni simgesi (`map_marks.rs`, `drawUcsIcon`) ve koyu zemin (“Türe göre”, web'de `data-canvas`), durum çubuğunda ölçek seçici
  (`screen_scale.rs`, `scaleSelector.ts`). 6. adım (her türün kendi şeridi) bitti: CAD Dosya, Giriş, Ekle, Açıklama, Değiştir,
  Görünüm, Yönet, Çıktı; CBS Dosya, Giriş, Harita, Veri, Düzenle, Analiz, Ölçme, Görünüm, Çıktı (web `app/ribbon.ts`'in
  `CAD_RIBBON_TABS`, `GIS_RIBBON_TABS`; envanterin `ribbonByMode`'u; ortak `fixtures/shell/v1/ribbon.json`). Resimlerde türü olan
  sahne `make_cad` (masaüstü testleri) ya da `shots.mjs`'in `type` alanıyla kurulur; web'in resim grubu `types`.

- Bitti (30 Eylül): yazı ekleri ([ADR 0145](../../docs/adr/0145-text-extras.md), TODOS.md `CAD-15`, `.kcad` şema 7),
  1–5. adımlar iki platformda: sözleşme ve şema 7; çekirdek ve çizim; komutlar; araçlar ve arayüz (Yazı'nın
  Hiza, Genişlik, Zemin ve Artır seçenekleri, Öznitelikler'in satırları, komut satırının “Diğer” çipi, Okunur yap,
  Bul ve değiştir, Metin dosyası yerleştir); biçimler (DXF okuma ve yazma, NCZ çapaları; `texts.dxf` ve
  `dxf-write/texts` bağımsız denetimle). Ondan önce blok (ADR 0144, `CAD-04`), çok parçalı alan (ADR 0143,
  `CAD-14`) ve köşe kotu (ADR 0142, `CAD-13`).
- Bitti (1 Ekim): kılavuz ([ADR 0146](../../docs/adr/0146-leader.md), TODOS.md `CAD-16`, `.kcad` şema 8), 1–5. adımlar iki
  platformda: sözleşme ve şema 8; yerleşim ve çizim; komutlar; Kılavuz aracı, Öznitelikler ve notun yerinde
  düzenlenmesi (ortak iz `leader.json`); DXF LEADER (bağlı MTEXT'iyle) ve MULTILEADER okuma, LEADER ve MTEXT yazma
  (`leaders.dxf`, `dxf-write/leaders` bağımsız denetimle). Arayüzdeki adı “Kılavuz”dur (sahibin seçimi).
- Bitti (1 Ekim): yeni ölçü türleri ([ADR 0147](../../docs/adr/0147-new-dimension-kinds.md), TODOS.md `CAD-17`, `.kcad`
  şema 9), 1–5. adımlar iki platformda: Koordinat, Yay uzunluğu, Kırıklı yarıçap, Semt, Eğim; ölçü değerinin zemini;
  Açı'nın yaydan ve daireden yolları, döndürülmüş doğrusal, Hızlı ölçü; hepsi şeritte Ölçülendirme ▾ (büyük düğme, her
  türün ikonu); yerleşimler bağımsız başvurularla (`fixtures/dimension/v1`), ortak izler; DXF koordinat (DIMENSION 6),
  ARC_DIMENSION ve LARGE_RADIAL_DIMENSION iki yönde, semt ve eğim hizalı ölçü olarak KentOS verisiyle, zemin DIMTFILL
  (`dimension-kinds.dxf`, `dxf-write/dimensions` bağımsız denetimle). Grupların anlamı AutoCAD'in kendi dosyasından
  doğrulandı; ezdxf'in ve LibreDWG'nin kırıklı yarıçap adları yanlıştır (ADR 0147 §10.5).
- Bitti (1 Ekim, sahibin bildirimi üzerine): ölçü doğruluğu ve gösterim yuvarlaması
  ([ADR 0149](../../docs/adr/0149-measurement-accuracy-and-display-rounding.md), TODOS.md `NUM-11`): tek gösterim kuralı
  (`display::fixed`, `core/displayNumber.ts`), 224 durumluk bağımsız ölçü denetimi (eğri uzunluğu ve basık elips düzeltildi),
  hesaplarda eğri ve elipsin 0,1 mm'lik açık sınırla temsili (`geom::curve_outline`).
- Bitti (1 Ekim): topolojik temizlik ([ADR 0148](../../docs/adr/0148-topology-cleanup.md), TODOS.md `HYB-01`), 1–3. adımlar
  iki platformda: çekirdek `ops::topology` (bağımsız başvuru `topology_cases.py`, 74 ortak durum); `cad.entities.edit`'in
  `topology` işlemi; Topolojik temizlik aracı (Değiştir › Nesne ▾; tolerans, Uçlar, Köşeler, Uzat, Buda; önizleme ve
  imlecin yanında sayılar; ortak iz `topology.json`).
- Sürmekte: TODOS.md §16'nın sırası (sahibin kararı, 1 Ekim): önce hibrit (§16.0), sonra CAD (§16.1), sonra CBS (§16.2).
  `HYB-02` çizgi ağından toplu alan ([ADR 0151](../../docs/adr/0151-polygonize.md)) bitti: çekirdek `ops::polygonize` (bağımsız
  başvuru `polygonize_cases.py`, 58 ortak durum), `cad.entities.create`'in `polygonize` işlemi, Toplu alan aracı iki
  platformda (ortak iz `polygonize.json`). `HYB-03` ölçü noktası ([ADR 0152](../../docs/adr/0152-survey-points.md)) bitti (2 Ekim):
  `#ad` dilbilgisi ve `ops::vertex_points` bağımsız başvurusuyla, `cad.entities.create`'in `vertexPoints` işlemi, Nokta'nın Ad, Kod
  ve Kot'u, aynı yer sorusu, `#ad` çözümü (adlı noktanın yeri araca tıklanmış gibi verilir) ve Köşelere nokta iki platformda
  (ortak izler `survey-points.json`, `vertex-points.json`; iz biçimi etiketi, öznitelikleri ve kotu da denetler). Sıradaki
  `HYB-04` nokta editörü ([ADR 0153](../../docs/adr/0153-point-editor.md)) bitti (2 Ekim): çekirdek (doğal sıra, tablo, çift
  noktalar, bağlı köşeler, bağımsız başvurusuyla); alt panelin Noktalar sekmesi (sıralama, süzgeç, seçim, Göster); yerinde
  düzenleme, Bağlı çizgiler izler, Satır ekle, Sil (`edits.json`); İşlemler ▾ ve satırın menüsü: Yeniden adlandır, Sıralı
  numara ver, Katmana taşı (`batch.json`), Çift noktaları ayıkla ve Çiftleri göster (`dedupe.json`), Dışa aktar (tablonun
  satırları, sırasıyla) ve İçe aktar (aynı adlar için ardından Çift noktaları ayıkla, Aynı ad; `dedupe.json`'un `imports`'u).
  `HYB-05` vektör oturtma ([ADR 0156](../../docs/adr/0156-vector-fit.md)): 1. adım (çözüm: `ops::fit`, bağımsız başvuru `fit_cases.py`)
  ve 2. adım (nesnelerin dönüşmesi: `ops::warp`, `warp_cases.py`) ve 3. adım (`cad.entities.transform`'un `similarity`, `affine`,
  `projective` türleri, adım “Oturt”), 4.1 (Vektör oturtma penceresi iki platformda: Kullan sütunu, Adla eşle, canlı çözüm,
  Uygula, rapor; sahne `vector-fit.kcad`) ve 4.2 (Parametrelerle: `scale_turn`, `fit_parameters.py`) tamam. 4.2'de bulunan
  çizim hattı sınırı da kapandı ([ADR 0157](../../docs/adr/0157-styled-far-tiles.md), `REN-07`'nin eki): stilli çizimin konumları
  karolarının merkezine göre, iki platformda ve üç çizim hattında (wgpu, WebGPU, WebGL2); en derin yakınlıkta milimetre altı
  `REN-16`'da açık. Kauçuk levha ([ADR 0158](../../docs/adr/0158-rubber-sheet.md)): 1. adım (ince plaka eğrisi `ops::rubber`,
  `rubber_cases.py`), 2. adım (nesneler: yalnız köşeler, `rubber_warp_cases.py`) ve 3. adım (`cad.entities.transform`'un
  `rubbersheet` türü, adım “Kauçuk levha”; durumlar levhanın Python ikiziyle bit bit, `sheet_f64.py`) ve 4. adım (Vektör
  oturtma'nın dördüncü dönüşümü, Sabit, yerel düzeltmeler) tamam. Kenar eşleme ([ADR 0159](../../docs/adr/0159-edgematch.md), `HYB-05`'in
  kalanı): 1. adım (çekirdek `ops::edgematch`, `edgematch_cases.py`), 2. adım (`cad.entities.edit`'in `edgematch` işlemi, adım
  “Kenar eşle”) ve 3. adım (pencere iki platformda, sahne `edgematch.kcad`) tamam; `HYB-05` bitti. `HYB-06` topolojik düzenleme
  ([ADR 0160](../../docs/adr/0160-topological-editing.md)): 1. adım (çekirdek `ops::topology_edit`: `apply`, `changes`;
  `topology_edit_cases.py`), 2. adım (durum çubuğunda Topoloji ve Noktalar da, tutamaçlar komşularla tek adımda, önizleme;
  ortak iz `topology-edit.json`) ve 3. adım (tutamaç menüsü, Esnet, kartta köşe sayısı) tamam; `HYB-06` bitti. `HYB-07` izleyerek
  çizim ([ADR 0161](../../docs/adr/0161-trace-drawing.md)): 1. adım (çekirdek `ops::trace` ve `ops::join::chain`, `trace_cases.py`) ve 2. adım
  (yol aracında İzle iki platformda, ortak iz `trace-draw.json`), 3. adım (Birleştir'in Zincir seçeneği, ortak iz `join-chain.json`)
  ve 4. adım (yol aracında Akış ve Adım boyu, ortak iz `stream-draw.json`) tamam; `HYB-07` bitti. `HYB-08` bitişik alan ve çakışma
  denetimi ([ADR 0162](../../docs/adr/0162-adjoining-areas.md)): 1. adım (çekirdek `ops::adjoin`, `adjoin_cases.py`) ve 2. adım (Çakışma kipi
  iki platformda, durum çubuğunda Çakışma; ortak iz `overlap.json`) , 3. adım (`cad.entities.create`'in `adjoin` işlemi, Bitişik alan aracı
  iki platformda; ortak iz `adjoin.json`) ve 4. adım (Topoloji açıkken komşulara köşe: çekirdek `junctions`, araçlar iki platformda; ortak iz
  `junctions.json`) tamam; `HYB-08` bitti. Sahibin isteğiyle araya girenler de bitti: PR 16 (KentOS UI) birleşti, alt panelin Python
  konsolu KentOS UI'ın `PythonRepl` ve `PythonEditor`'ü oldu (ADR 0132'nin eki), nokta editörünün arama kutusu düzeldi. `HYB-09`
  kenet ekleri ([ADR 0163](../../docs/adr/0163-snap-additions.md)): 1. adım (çekirdek: dört tür, `snap_ex`, çizilmekte olan yol, katman
  maskesi; `snap_cases.py`) ve 2. adım (ayarlar, türlerin komutları, Kenet hücresinin menüsü, karelaj aralıkları, ölçek aralığı, yol
  araçlarının taslağı, işaretler; ortak iz `snap-additions.json`), 3. adım (Uzantı ve Paralel'in alınması, yazılan mesafe; ortak iz
  `snap-acquire.json`) ve 4. adım (katman başına kenet, `.kcad` şema 10; ortak iz `layer-snap.json`) tamam; `HYB-09` bitti. `HYB-10`
  sayısallaştırma kilitleri ([ADR 0166](../../docs/adr/0166-digitizing-locks.md); sahibin “Todos ile devam”ı, 4 Ekim): 1. adım (çekirdek
  `tools::locks`: kilitli nokta, imlecin kuralı kilitlerle, açı ve sapma doğrultuları, Dik kapat, `<açı`; `lock_cases.py`) ve 2. adım
  (Uzunluk, Açı ve Sapma iki platformda: oturumun kilitleri, değer kartında Tab ve `<açı`, çipler, kılavuzlar, Kilit ▸, `draft.lock.*`;
  ortak iz `locks.json`) ve 3a (Nesneye paralel ve dik: `edge_direction`, kenar bekleyişi, `draft.lock.parallel` ve `.perpendicular`; ortak iz
  `lock-edges.json`), 3b (Dik açı `drafting.rightAngle`, Dik kapat (D); ortak iz `right-angle.json`) ve 4. adım (Referans noktası, Yapım
  kipi; ortak iz `lock-reference.json`) tamam; `HYB-10` bitti. `HYB-11` ikinci koordinat sistemi ve dönüştürücü
  ([ADR 0167](../../docs/adr/0167-second-crs-and-converter.md)): 1. adım (çekirdek `crs::transform`, geri TM, DMS; PROJ başvurusu
  `crs_transform_cases.py`) ve 2. adım (proje ayarı `secondSrid`, `.kcad` şema 12, `display.geographic`; okunuş `kentos_interaction::second`
  ve `model/secondCrs.ts`; durum çubuğunda iletiyle aynı yeri paylaşan hücre, sistem düğmelerinin sağ tık menüsünde İkinci sistem ▸,
  Proje ayarları'nda alan, Koordinat oku'nun ikinci satırı; ortak iz `second-crs.json`) ve 3. adım (Mesafe ölç ve Alan hesapla'nın ikinci
  sistemin düzlemindeki uzunluğu, alanı ve çevresi: çekirdek `crs::measure`, PROJ başvurusu `crs_measure_cases.py`) ve 4. adım
  (Koordinat dönüştür penceresi, `crs.transform`: tek nokta, Çizimden, liste, panoya ve CSV; okuma ve yazma `crs_convert_cases.py`)
  tamam; `HYB-11` bitti. `HYB-12` özel koordinat sistemi ([ADR 0168](../../docs/adr/0168-custom-crs-and-datum-transforms.md)):
  1a (çekirdek: projenin datumu, başlangıç enlemli TM, yerel sistem, projenin datum seçimleri; PROJ başvurusu
  `crs_custom_cases.py`) ve 1b (WKT ve PROJ okuma ve yazma, `crs::text`, `crs::wkt`; PROJ başvurusu `crs_text_cases.py`)
  ve 2. adım (NTv2 ızgarası `crs::ntv2`, PROJ'un hgridshift'i birebir; başvuru `ntv2_cases.py`) ve 3a (sözleşme
  `kentos_contracts::crs`, ayarlar `customCrs`, `secondCustomCrs`, `datumTransforms`, `ProjectSettings::sanitized`; `.kcad` şema 13 ve
  bağımsız Python okuyucu ve yazıcısı) ve 3b (çözücü `kentos_project::systems`, `model/projectCrs.ts`, başvuru `project_crs_cases.py`;
  ikinci sistem tanımlar ve seçimlerle; tanımın adı; tanımlı proje sistemli sayılır), 3c (dönüştürücüde projenin tanımları ve seçimleri)
  ve 3d (cihazın ızgara kitaplığı, `grids.rs`, `app/gridLibrary.ts`; Proje ayarları'nda Izgaralar) ve 4a (Proje ayarları'nda Datum
  dönüşümleri, form kuralları `choice_form`) ve 4b (Özel koordinat sistemi penceresi: masaüstü `project/custom_crs.rs` Proje ayarları'nın
  üstünde, web `ui/settings/CustomCrsDialog.ts`; form kuralları `definition_form`, `model/definitionForm.ts`, ortak durumlar
  `definition-form.json`; listelerde tanımın satırı, “Özel sistem…” ve Düzenle) ve 4c (pencerede WKT ve PROJ'dan al, kopyala, Kayıttakini
  seç, Deneme noktası; okuma kuralları `definition_form::read`, `readDefinition`, ortak durumlar `definition-text.json`) ve 4d (Ortak
  noktalardan hesapla: `fit_plane`, `fitPlane`, ortak durumlar `definition-fit.json`; Hesap pencerelerinin tablosuyla) tamam; `HYB-12` bitti.
  Sıradaki `HYB-13` saha verisi ([ADR 0169](../../docs/adr/0169-field-data.md); sahibin kararları 4 Ekim: dört total station biçimi, GPX ve
  NMEA, toleranslar proje ayarı ve değerlerini sahip verir, k = 0,13, alete okunan biçimlerle): 1. adım (çekirdek indirgeme
  `survey::fieldbook`, CSV karne `formats::field`, iki platformda; başvurular `field_reduce_cases.py`, `field_csv_cases.py`) ve 2. adım
  (Leica GSI-8 ve GSI-16, `formats::field::gsi`; başvuru `field_gsi_cases.py`) ve 3a (Proje ayarları › Ölçme: k ve toleranslar, `.kcad`
  şema 14, `survey_form`; Kutupsal alım k'yı uygular) ve 3b (Karne editörü iki platformda, `calc.fieldbook`; çekirdekte durumlar ve
  tolerans denetimi, `field::sniff`, `readFieldBook`) ve 3c (Kutupsal alım'a aktar: `polar_transfer`, `fieldPolar`, Geri bakış) tamam;
  3. adım bitti. 4a (Poligon hesabı'na aktar: `traverse_transfer`, `fieldTraverse`, Poligon bölümü) ve 4b (toleranslar: kenarın iki
  yönden farkı, poligonun kapanmaları, `.kcad` şema 15; `BookLeg.over`, `survey::traverse::closure`) tamam; 4. adım bitti. 5. adım
  dört parçada, her biri üreticinin belgesinden bağımsız başvurusuyla ve aynı karnenin örneğiyle (`field_samples.py`): 5a Sokkia SDR2x ve
  SDR33 (`formats::field::sdr`, başvuru `field_sdr_cases.py`; Sokkia'nın “Interfacing with the SOKKIA SDR Electronic Field Book”u) ve 5b
  Topcon GTS-7 (`formats::field::gts7`, kesin değerler `field::exact`; başvuru `field_gts7_cases.py`, Topcon Link Reference Manual'ın Ek C'si)
  ve 5d Nikon RAW (`formats::field::nikon`; başvuru `field_nikon_cases.py`, Nivo kılavuzu) ve 5c Trimble JobXML (`formats::field::jobxml`;
  başvuru `field_jobxml_cases.py`, şema 5.3; XML roxmltree ile, sahibin 4 Ekim onayıyla biçim çekirdeğinde de, derinlik koruması
  `formats::xml`) tamam; 5. adım bitti. 6. adım GNSS: 6a okuyucular (`formats::gnss`, `readGnss`; GPX 1.1 ve NMEA 0183, başvurular
  `gnss_gpx_cases.py`, `gnss_nmea_cases.py`) ve 6b GNSS içe aktar (`file.import.gnss`; kurallar `kentos_interaction::gnss`,
  `model/gnssImport.ts`, başvuru `gnss_import_cases.py`; pencere `exchange/gnss_import.rs`, `ui/io/GnssImportDialog.ts`; içe aktarma
  yolundan tek adım, yeni ya da var olan katman) tamam; 6. adım bitti. 7. adım cihaza gönderme: 7a yazıcılar (`formats::field::write`,
  `writeField`; başvuru `field_write_cases.py`) ve 7b Cihaza gönder penceresi (`field.send`; `exchange/field_send.rs`,
  `ui/io/FieldSendDialog.ts`; seçili noktalar, Aplikasyon, Nokta editörü) tamam; Sokkia SDR33 yazımı sahibin bir SDR33 dosyasını ya da
  Sokkia'nın belgesini bekliyor (iş kaydının kodları). `HYB-13` bundan başka bitti. `HYB-14` zemin, elipsoit ve düzlem
  ([ADR 0171](../../docs/adr/0171-ground-ellipsoid-plane.md); sahibin kararları 4 Ekim: `geographiclib-rs`, projenin ortalama elipsoit
  yüksekliği, “Uzunlukları projeksiyona indir” kapalı başlar): 1. adım (çekirdek `crs::ground`: noktanın ve çizginin ölçeği,
  `geodesy::tm_scale`, yükseklik çarpanı, jeodezik uzunluk, köşe çokgeni ve yayların alanı; başvuru `ground_cases.py`, PROJ ve
  GeographicLib'in C'si, alanlar eşit alanlı izdüşümde; GeographicLib libm'li kopyayla, `scripts/vendor/geographiclib.py`) ve 2. adım
  (Ölçme'de Ortalama elipsoit yüksekliği, `.kcad` şema 16 `groundHeight` ve `reduceToGrid`; yükseklik yazılıyken Mesafe ölç ve Alan
  hesapla'nın elipsoit ve zemin satırları, `kentos_interaction::ground`, `model/groundMeasures.ts`; ortak iz `ground-measures.json`)
  tamam; 3. adım (Hesap pencerelerinde “Uzunlukları projeksiyona indir”) iki parçada: 3a çekirdek (`crs::ground::Grid`,
  `grid_factor`; `survey::polar`, `survey::traverse` isteğe bağlı `grid` ile, iki ve üç geçiş; başvuru `ground_survey_cases.py`)
  ve 3b (Ölçme'de anahtar: yükseklik, sistem ve noktanın tek ölçeğiyle açılır, `survey_grid`, `why_not_grid`, `surveyGrid`,
  `whyNotGrid`; pencerelerin tabloları, özetleri ve raporları iki platformda) tamam; `HYB-14` bitti (5 Ekim). `HYB-15` köşe tablosu
  ([ADR 0172](../../docs/adr/0172-vertex-table.md)): 1. adım (çekirdek `ops::vertex_table`, başvuru `vertex_table_cases.py`), 2. adım
  (düzenlenen Koordinat listesi iki platformda: web `VertexTable.ts`, `vertexEdit.ts`, masaüstü `vertices/`; `edits.json`) ve 3. adım
  (Topoloji açıkken komşular: `neighbours_in`, `neighboursOf`; takma adlar) tamam; `HYB-15` bitti (5 Ekim). `HYB-16` biçim değiştirme,
  sürdürme ve delikler ([ADR 0173](../../docs/adr/0173-reshape-continue-holes.md)): 1. adım (`cad.entities.edit`'in beş işlemi; çekirdek
  `ops::holes`, `ops::reshape_by`; başvuru `reshape_cases.py`) ve 2. adım (Delik ekle yol aracının `Hole` biçimi, Deliği sil ve
  Deliği doldur iki platformda: masaüstü `kentos_interaction::holes`, web `holeTools.ts`; deponun `holes_at`'i; ortak iz `holes.json`)
  ve 3. adım (Sürdür: `ops::continuation`, yol aracının `Continue` biçimi, web `continueTool.ts`; ortak iz `continue.json`) ve 4. adım
  (Biçim değiştir: yol aracının `Reshape` biçimi, `kentos_interaction::reshape_by`, web `reshapeTool.ts`; ortak iz `reshape.json`) tamam;
  `HYB-16` bitti (5 Ekim). Sıradaki `HYB-17` çok parçalı çizgi ve çok noktalı nesne
  ([ADR 0174](../../docs/adr/0174-multi-part-lines-and-points.md)): 1. adım (sözleşme: çoklu çizginin ve noktanın `parts`'ı, `PointPart`;
  `.kcad` şema 17, `FORMATS_VERSION` 28; kodek, sütunlar, bağımsız Python okuyucu ve yazıcısı, `multi-part-lines.kcad`; iki belgede
  miras parçalar düşer) ve 2. adım (hesap: çekirdeğin `Shape`'i ve `area_parts`'ı üç türde, deponun `MARKERS` kaydı ve paket türleri 16, 17,
  düzenlemeler ve `MULTI_PART_REFUSED` retleri, `EntityGeometry`'nin `parts`'ı, kotlar, iki çizici) ve 3. adım (GeoJSON'ın MultiLineString ve
  MultiPoint'i, Shapefile'ın PolyLine ve MultiPoint kaydı tek nesne, `tools/formats/gis.py` de; GeoJSON'a ve DXF'e yazım, `export/cizgiler`,
  `dxf-write/parts`; EWKB'nin ve sunucunun MultiLineString ve MultiPoint'i) ve 4. adım (`partsJoin` ve `partsSplit`'in çizgi ve noktası:
  `kentos_interaction::line_parts`, web `tools/lineParts.ts`; ortak komut durumları) ve 5. adım (Öznitelikler'de Parça ve Nokta sayısı, kartta
  Parça ve Nokta, Koordinat listesinde parçalar ve noktalar; `usage-line-parts`) tamam; `HYB-17` bitti (5 Ekim). `HYB-18` etiketleri yazıya
  çevirme ([ADR 0175](../../docs/adr/0175-labels-to-text.md)): 1. adım (çekirdek `ops::label_text`: paftanın kuralı, `spot`, `fill_template`; `Store::label_texts`;
  başvuru `label_text_cases.py`; paftayla karşılaştırma `mapLabels.test.ts`; çizimin şablonu iki platformda çekirdeğin kuralıyla) ve 2. adım
  (`cad.entities.create`'in `labels` işlemi, adım “Etiketleri yazıya çevir”) ve 3. adım (araç iki platformda: web `labelsToTextTool.ts`,
  masaüstü `kentos_interaction::labels_to_text`; önizlemede soluk yazılar `drawTextGhost`, `labels::ghost`; standart yazı katmanı türüne göre adıyla aynı
  adımda; ortak iz `labels-to-text.json`) tamam; 4. adım (nesneye bağlı yazı) üç parçada: 4a (yazının `labelOf` ve `labelScale`'i, `.kcad`
  şema 18) ve 4b (çekirdeğin `label_text_of`'u, iki belgede kayıt öncesi izleme: web `model/linkedTexts.ts`, masaüstü `kentos_domain` `linked.rs`;
  ortak belge durumları `linked-texts.json`) ve 4c (komutlarda bağ, `invalid_link`, `link_not_found`, Bağı kopar; araçta Nesneye bağlı (B);
  çizimde etiketin yerini alma, deponun `set_text_labelled`'ı; ortak iz `labels-linked.json`; Öznitelikler'de Bağlı nesne; DXF penceresinin
  notu) tamam; `HYB-18` bitti (5 Ekim). Sıradaki `HYB-19` nesne şablonları ([ADR 0176](../../docs/adr/0176-object-templates.md); TODOS'taki “çizim
  kalemleri”, ad sahibin seçimi): şablon stil kitaplığının üçüncü öğe türü, şablonla çizmek, Şablonlar paneli, grup şablonu, Şablonu uygula;
  BÖHYY takımı sahibin tarifini bekler. 1. adım (şablonun kuralları `model/objectTemplate.ts`, `kentos_native_style::object_template`, ortak
  `object-templates.json`; kitaplıkta `template`, `.kstil` sürüm 2; Stil yöneticisinde Şablon türü) ve 2. adım (oluşturma komutlarının
  `symbol`'ü, eksik olanlarda `label`; ortak komut durumları) ve 3a (şablonla çizmek: katmanın kuralı `templates::find_layer`,
  `templateLayer`, başvuru `template_layer_cases.py`; koşu masaüstünde `templates.rs`, web'de `app/objectTemplates.ts`; damga
  `Context::template`, `tools/templateStamp.ts`; `template.draw`; ortak iz `template-draw.json`) ve 4a (Şablonlar paneli: masaüstü
  `templates_panel.rs`, web `ui/templates/TemplatesPanel.ts`; liste kuralı `object_template::listed`, `style/templateList.ts`, ortak
  `template-list.json`; `template.panel`) ve 4b (Şablon düzenleyici: masaüstü `template_editor.rs`, web `ui/templates/TemplateEditor.ts`;
  kuralları `template_form`, `model/templateForm.ts`; Seçili nesneden şablon `object_template::from_object`, `templateFromObject`; ortak
  `template-form.json`, `template-from-object.json`) ve 4c (şeridin Giriş'inde Şablonlar paneli: web `templateField`, masaüstü
  `templates_group`; dar kademede Özellikler'in alanları adları yerine ikonlarıyla, KentOS UI `Choice::label_icon`, web `dropdown--glyph`)
  ve 3b (nokta, yazı ve blok şablonları: masaüstü `seed_tool`, `give_tool_back`, web `tools/templateSeeds.ts`; ad dizisi şablon başına;
  bloğu olmayan şablon başlamaz; nesneden şablonda yazı kâğıtta mm; ortak iz `template-tools.json`) ve 5a (grup şablonunun modeli:
  `members`, `templateIssues`/`template_issues`, kitaplıkta `memberIssues`/`member_issues`, formun `MemberRow`'u; ortak
  `template-groups.json`) ve 5b (üyelerin geometrisi: çekirdek `ops::template_members`, işlemler `templateMemberOffsets` ve
  `templateMemberCentroid`, web `model/ops/templateMembers.ts`; başvuru `template_member_cases.py`) ve 5c (grup şablonuyla çizmek: web
  `tools/templateMembers.ts`'in `withMembers`'ı grup araçlarının yazma yerlerinde, masaüstünde `with_tool`'un belge grubu ve
  `template_members.rs`; belgenin `nextSlot`/`next_slot`'u; ortak iz `template-group.json`) ve 5d (düzenleyicide Grup üyeleri tablosu)
  ve 6. adım (Şablonu uygula: kural `templateApplication`, `object_template::application`, ortak `template-apply.json`; `cad.entities.set`'in
  `template` işlemi; web `applyTemplate`, masaüstü `apply_template`; `template.apply`, Şablonlar panelinde ve Stil yöneticisinde “Seçili
  nesnelere uygula”; ortak iz `template-apply.json`, izlerin `applyTemplate` eylemi) tamam; `HYB-19` bitti (5 Ekim; BÖHYY takımı sahibin
  tarifini bekler). Sıradaki `HYB-20` katman yönetimi ekleri ([ADR 0177](../../docs/adr/0177-layer-management-extras.md)): nesneden katman
  işlemleri, Katmanı eşle ve Katmana kopyala, Kopyasını oluştur ve Katmanları birleştir, Katman durumları (`.kcad` şema 19),
  Kullanılmayanları temizle, Katman listesi. 1. adım (nesneden katman işlemleri: web `tools/layerTools.ts`, masaüstü
  `kentos_interaction::layer_tools`; iki belgede `isolateLayers`/`isolate_layers`; Yalıtımı kaldır `layer.unisolate`; şeridin yerleşik
  panelinin ▾'i `under`; izlerin `hiddenLayers` ve `lockedLayers`'ı; ortak iz `layer-by-object.json`) ve 2. adım (Katmanı eşle ve
  Katmana kopyala: web `tools/layerMoveTool.ts`, masaüstü `kentos_interaction::layer_move`; ortak iz `layer-move.json`) ve 3. adım
  (Kopyasını oluştur ve Katmanları birleştir: web `app/layerActions.ts`, `ui/layers/MergeLayersDialog.ts`, masaüstü `layer_merge.rs`;
  ortak iz `layer-merge.json`) ve 4a (katman durumları: `ProjectSettings.layerStates`, `.kcad` şema 19, `layer-states.kcad`) tamam;
  kalanı tek parçada (sahibin 5 Ekim kararı: “her maddeyi tek parçada bitir”, bundan sonraki maddeler alt adımlara bölünmez): Katman
  durumları penceresi ve Katmanlar panelinin Katman durumları ▾'i (web `app/layerStates.ts`, `ui/layers/LayerStatesDialog.ts`, masaüstü
  `layer_states.rs`; kurallar `kentos_domain::layer_states`, `model/layerStates.ts`), Kullanılmayanları temizle (kural
  `kentos_domain::layer_purge`, `model/layerPurge.ts`: kalanların kullanmadığı en büyük küme; web `ui/layers/PurgeDialog.ts`, masaüstü
  `layer_purge.rs`), Katman listesi (pano ve CSV; web `app/layerList.ts`, masaüstü `layer_list.rs`); ortak durumlar `fixtures/layers/v1`
  bağımsız Python başvurularıyla, ortak izler `layer-states.json` ve `layer-purge.json`, izlerin `layers` beklentisi; web kaydının
  katman durumlarını ve katmanın kendi kenetini düşürmesi düzeldi (`io/kcad.ts` `projectHead`); `HYB-20` bitti (5 Ekim). `HYB-21`
  veri karşılaştırma ([ADR 0179](../../docs/adr/0179-data-compare.md)) tek parçada bitti (5 Ekim): çekirdek `ops::compare` (işlem
  `dataCompare`, bağımsız başvuru `compare_cases.py`, ortak durumlar `fixtures/compare/v1`), web `app/dataCompare.ts`,
  `ui/data/DataCompareDialog.ts`, masaüstü `data_compare.rs`; ortak iz `data-compare.json`. `HYB-22` veride arama
  ([ADR 0178](../../docs/adr/0178-data-search.md); sahibin izniyle bir kezlik alt ajanın dalında yapıldı, `main`'e birleşti) tek parça, iki
  platformda bitti (5 Ekim): çekirdek `text::edit::matches` ve `ops::data_search` (bağımsız başvuru `data_search_cases.py`, ortak
  `fixtures/search/v1`), web `ui/bottom/SearchPanel.ts` ve `model/dataSearch.ts`, masaüstü `search/` ve `kentos_interaction::data_search`;
  ortak iz `data-search.json` (oynatıcılarda `panel` eylemi ve `panel`, `search`, `mark` beklentileri). `HYB-23` kayıtlı ölçü (COGO)
  öznitelikleri ([ADR 0180](../../docs/adr/0180-recorded-measurements.md)) tek parçada bitti (6 Ekim): çekirdek `ops::cogo` (işlemler
  `cogoMeasure`, `cogoCheck`, `cogoRecord`; bağımsız başvuru `cogo_cases.py`, ortak `fixtures/cogo/v1`), Çizgi aracının kaydı (web
  `drawTools.ts`'in `typedText`'i, masaüstü `line.rs`'in `typed`'ı), web `app/cogo.ts` ve `ui/cogo/CogoCheckDialog.ts`, masaüstü
  `cogo.rs`; ortak iz `cogo.json`. `HYB-25` gezinme ekleri ([ADR 0181](../../docs/adr/0181-overview-and-magnifier.md)) tek parçada bitti
  (6 Ekim): çekirdek `store::overview` (Genel bakışın kapsamı ve resmi, web'e `overviewExtent`, `overviewPicture`) ve `tools::navigation`
  (kartların yeri, eşleme; web `viewport/navigation.ts`), bağımsız başvuru `navigation_cases.py` (ortak `fixtures/navigation/v1`, resim
  piksel piksel); büyüteç çizim hatlarında ikinci kamera (web `FrameState.lens`, masaüstü `Renderer::prepare_lens`/`draw_lens`, aynı
  ilkel türüyle); kartlar web `viewport/navigationCards.ts`, masaüstü `navigation_cards.rs`; yerleşimde `overview`, `magnifier`,
  `magnifierZoom`; ortak iz `overview.json`. §16.0 bitti; §16.1 (CAD) başladı: `CAD-18` kroki yazımı **[M]** (gösterim kurallarını
  sahip tarif edecek) beklerken `CAD-20` çok satırlı yazı ([ADR 0182](../../docs/adr/0182-paragraph-text.md)) tek parçada bitti (6 Ekim):
  sözleşmenin `Paragraph`'ı (`boxWidth`, `lineSpacing`, `runs`; `.kcad` şema 20, `FORMATS_VERSION` 30), çekirdek `text::paragraph`
  (satırlar, sarma, kalın tablo, düzenleyicinin dilimleri, `corner_box`; bağımsız başvuru `paragraph_cases.py`, ortak
  `fixtures/text/v1/paragraph.json`), deponun `LABEL_LINE`, `LABEL_PARAGRAPH_MASK`, `LABEL_PIECE_LINE` kayıtları; araç web
  `tools/paragraphTool.ts`, masaüstü `kentos_interaction::paragraph`; düzenleyici web `ui/shell/ParagraphEditor.ts`, masaüstü
  `paragraph_editor.rs`; DXF MTEXT biçimleriyle iki yönde (`dxf/strings.rs`'in `mtext_content`'i, `writer/entities/paragraph.rs`;
  `mtext.dxf`, `dxf-write/paragraphs`); `invalid_paragraph`; ortak iz `paragraph-text.json` (oynatıcılarda `paragraph` eylemi).
  `CAD-21` yazı ve ölçü stilleri ([ADR 0183](../../docs/adr/0183-text-and-dimension-styles.md)) tek parçada bitti (6 Ekim): sözleşmenin
  `TextFace`, `DimensionLook`, `TextStyleDef`, `DimensionStyleDef`'i ve kuralları (`annotation.rs`; bağımsız başvuru
  `annotation_style_cases.py`, ortak `fixtures/text/v1/styles.json`), `.kcad` şema 21 (`FORMATS_VERSION` 31), `cad.entities.edit`'in
  `textStyle` ve `dimensionStyle` işlemleri; pencere masaüstünde `annotation_styles.rs`, web'de `ui/annotation/StylesDialog.ts`
  (kaydetme `kentos_interaction::style_tables`, `app/styleTables.ts`); araçların Stil'i `kentos_interaction::styles`,
  `tools/styleOption.ts` (stil adının adımında Boşluk harftir: `takes_words`, `takesWords`); DXF'te `dxf/styles.rs`,
  `writer/styles.rs`, içe aktarmanın birleştirmesi `exchange/apply.rs`, `io/apply.ts`; ortak izler `text-styles.json`,
  `dimension-styles.json` (oynatıcılarda `face`, `look`). `CAD-22` tablo ([ADR 0184](../../docs/adr/0184-tables.md)) tek parçada bitti
  (6 Ekim; sahibin eki: kalın çerçeve): sözleşmenin `TableEntity`'si ve kuralları (`table.rs`), `.kcad` şema 22 (`FORMATS_VERSION` 32),
  çekirdek `geom::table`, `ops::table`, `ops::table_edit` (bağımsız başvuru `table_cases.py`, ortak `fixtures/table/v1/cases.json`),
  dosya okuyucusu `formats::table_file` (`table_file_cases.py`, `fixtures/table/v1/files`); pencereler masaüstünde `tables/`, web'de
  `ui/table/`, kuralları `kentos_interaction::table` ve `app/tables.ts`; yerleştirme `kentos_interaction::table_place`,
  `tools/tablePlaceTool.ts`; DXF adsız blok ve KENTOS verisi (`dxf-write/tables`); ortak iz `table.json` (oynatıcılarda `table`
  beklentisi, ok tuşları). `CAD-19` koordinat yazımı ([ADR 0185](../../docs/adr/0185-coordinate-labels.md)) tek parçada bitti (6 Ekim; ikonlar
  sahibin seçimi, yeni ikonlarda seçenekler sunulur): çekirdek `ops::coordinate_labels` (yerler ve adlar koordinat çizelgesiyle ortak, şablon,
  yerleşim; bağımsız başvuru `coordinate_label_cases.py`, ortak `fixtures/coordinate-labels/v1/cases.json`), `cad.entities.create`'in
  `coordinates` işlemi; araçlar masaüstünde `kentos_interaction::coordinate_labels`, web'de `tools/coordinateLabelTool.ts`; Çizelge Tablo
  ekle'nin yerleştirmesiyle (`ViewChange::PlaceTable`; web'in tablo yapma yardımcıları `tools/newTable.ts`); ortak iz `coordinate-labels.json`
  (oynatıcılar `{`, `}`, `|`, `=`'i yazar). `CAD-23` tarama ekleri ([ADR 0186](../../docs/adr/0186-hatch-extras.md)) tek parçada bitti (6 Ekim;
  ikonları sahibin seçimi): sözleşmenin `hatch` modülü (desen ve degrade türleri, `PatternLine`, `HatchGradient`, `HatchAssoc`), `.kcad`
  şema 23 (`FORMATS_VERSION` 33), çekirdek `geom::hatch_pattern` (kitaplık, boyalar, parçalar, `carried`), `ops::hatch_region`,
  `tools::hatch` (bağımsız başvuru `hatch_pattern_cases.py`, ortak `fixtures/hatch/v1/cases.json`); stil motorunda `hatchFill`'in
  `stagger`'ı ve `gradientFill` (çerçevesi partide çapaya göre), WGSL sözleşmesi 4. sürüm; ilişkili taramanın izlenmesi masaüstünde
  `kentos_domain` `hatch_ties.rs`, web'de `model/hatchTies.ts` (ortak `hatch-ties.json`); araçlar `kentos_interaction::hatch_options`,
  `hatch_selected`, web `tools/hatchOptions.ts`, `tools/hatchSelectedTool.ts`; Öznitelikler `properties::hatch_rows`,
  `ui/properties/hatchRows.ts`; Desen menülerinde 56 × 24'lük geniş örnekler (sahibin seçimi; KentOS UI menüsünün `preview`'ü ve
  `Glyph::wide`'ı, web `iconPreview`); DXF `dxf/emit/pattern.rs` ve yazıcı (`hatch-patterns.dxf`, `dxf-write/hatches`); ortak izler `hatches.json`,
  `hatch-patterns.json` (oynatıcılarda taramanın `pattern` ve `assoc` beklentisi). `CAD-25` seçim ekleri
  ([ADR 0187](../../docs/adr/0187-selection-extras.md)) tek parçada bitti (6 Ekim; ikonlar sahibin seçtikleri): çekirdekte `Store::hits` (`hit` aynı
  puandan) ve `store/polygon.rs` (`in_polygon`, `ring_problem`; bağımsız başvuru `selection_cases.py`, ortak `fixtures/selection/v1`);
  seçimin önceki seti ve çipi (`Selection`'ın `previous`, `Cycle`'ı; web `model/selection.ts`), süzgeç (`drafting.selectFilter`;
  `kentos_interaction::selectable`, `tools/selectable.ts`), araçlar `select_polygon.rs`, `select_similar.rs`, web `PolygonSelectTool`,
  `tools/selectSimilarTool.ts`; komutlar `selection_commands.rs`, `app/selectionCommands.ts`; çip `selection_chip.rs` (KentOS UI menüsünün
  `highlight`'ı, `beside`'ın menü iletimi), `ui/shell/SelectionChip.ts`; ortak iz `selection-extras.json` (oynatıcılarda `cycle`). `CAD-26`
  nokta hesaplayıcı ekleri ([ADR 0188](../../docs/adr/0188-point-calculator-extras.md)) tek parçada bitti (6 Ekim; ikonlar önerilen A seçenekleri,
  sahip uyurken): çekirdek `tools::point_calc` (`route_of`, `station`, `reading`, `km_value`, `km_text`, `slope`, `bisector`; eğride ayak
  eğrinin kendisinden; bağımsız başvuru `point_calc_cases.py`, ortak `fixtures/point-calc/v1`), masaüstünde `point_calc.rs` ve
  `point_calc/route.rs`, web'de `tools/pointCalc.ts` ve `tools/pointCalcRoute.ts`; adlı nokta `session::named_point_at`, `namedPointAt`;
  KentOS UI menüsünün kısayol sütunu ölçülen genişlikle; ortak iz `point-calc-extras.json` (oynatıcılarda `%`). `CAD-27` Km yaz
  ([ADR 0189](../../docs/adr/0189-stationing.md)) tek parçada bitti (6 Ekim; ikon sahip uyurken seçenek sayfasının B'si): çekirdek `ops::stationing`
  (bağımsız başvuru `stationing_cases.py`, ortak `fixtures/stationing/v1`), `CreateOperation::Stations`, masaüstü
  `kentos_interaction::station_labels`, web `tools/stationLabelTool.ts`; ortak iz `stationing.json`. Sahibin 6 Ekim gecesi kararı: CAD-36
  bitene kadar durmadan sırayla (`CAD-24` [M] sahibin tarifini bekler). `CAD-28` Orta hat ([ADR 0190](../../docs/adr/0190-centerline.md)) tek
  parçada bitti (6 Ekim; ikon seçenek sayfasının D'si): çekirdek `ops::centerline` (güzergâhın `Walk::edges`, `vertex_lengths`; bağımsız
  başvuru `centerline_cases.py`, ortak `fixtures/centerline/v1`), `CreateOperation::Centerline`, masaüstü `kentos_interaction::centerline`,
  web `tools/centerlineTool.ts`; ortak iz `centerline.json`. `CAD-29` Paralel kaydır ([ADR 0191](../../docs/adr/0191-parallel-shift.md)) tek
  parçada bitti (7 Ekim; ikon seçenek sayfasının B'si): çekirdek `ops::edge_shift` (`shifted`, `for_area`, `pick`; bağımsız başvuru
  `edge_shift_cases.py`, ortak `fixtures/edge-shift/v1`), `EditOperation::EdgeShift` (köşeler kotlarını yerlerinden alır), alan biriminden
  m²'ye `Format::area_to_square_metres`, `Formatter.areaToSquareMetres`; masaüstü `kentos_interaction::edge_shift`, web
  `tools/edgeShiftTool.ts`; ortak iz `edge-shift.json`. `CAD-30` Resim nesnesi ([ADR 0192](../../docs/adr/0192-image-object.md)) tek
  parçada bitti (7 Ekim; ikon seçenek sayfalarının A'ları): sözleşmenin `ImageEntity`'si ve kuralları (`image.rs`), `.kcad` şema 24
  (`FORMATS_VERSION` 34), çekirdek `geom::image`, `ops::image` (bağımsız başvuru `image_cases.py`, ortak `fixtures/image/v1`); stil
  motorunun `MODE_IMAGE`'i, stilli çizim hattının 5. sürümü (`imageFs`), resim dokuları `styled/pictures.rs`, web `render/pictures.ts`;
  masaüstü `kentos_interaction::image_insert`, `::image_clip`, `pictures.rs`; web `tools/imageTools.ts`, `tools/pictureFile.ts`,
  `ui/properties/imageRows.ts`; ortak izler `image-insert.json`, `image-clip.json` (oynatıcılarda resmin gösterilen kısmı). `CAD-31`
  çizimler arası alışveriş ([ADR 0193](../../docs/adr/0193-drawing-exchange.md)) tek parçada bitti (7 Ekim; ikonlar önerilen seçenekler):
  kurallar belge düzeyinde, sözleşmenin JSON'u üzerinde `kentos_domain::exchange` ve `model/exchange.ts` (bağımsız başvuru
  `exchange_cases.py`, ortak `fixtures/exchange/v1`); masaüstü `drawing_exchange.rs` (`NewLayer`'ın `snap`'i), web
  `app/drawingExchange.ts`, `ui/io/TakeFromDialog.ts`; ortak izler `take-from.json`, `block-insert-file.json` (web'in oynatıcısı dosya
  okuyan komutun aracını bekler). `CAD-32` hizala ve dağıt ([ADR 0194](../../docs/adr/0194-align-and-distribute.md)) tek parçada bitti
  (7 Ekim; ikonlar seçenek sayfasının A'ları): çekirdek `ops::arrange` (kutular deponun kuralıyla, `block::pieces_bounds`; bağımsız
  başvuru `arrange_cases.py`, ortak `fixtures/arrange/v1`), sözleşmenin `Transform::Arrange` ve `ArrangeMode`'u, komut iki platformda
  (`too_few_objects`), araç masaüstünde `kentos_interaction::align_distribute`, web'de `tools/alignDistributeTool.ts`; ortak iz
  `align-distribute.json`. `CAD-33` görünüm kipleri ([ADR 0195](../../docs/adr/0195-view-modes.md)) tek parçada bitti (7 Ekim; ikonlar
  önerilen seçenekler, Tek renk'in ikinci seçenek): ayarlar `graphics.colorMode`, `.fills`, `.areaEdges`, `.transparency`,
  `.highlightColor`, `.highlightWidth`; stil çekirdeğinin `build::View`'u (`BatchSink::hide`), renk kuralı masaüstünde
  `kentos_native_style::color`'ın `ViewColors`'ı, web'de `render/color.ts`; vurgu masaüstünde wgpu'nun ikinci çerçeve bağlaması, web'de
  `widenLines`; ortak durumlar `batches.json`'un görünüş durumları, ortak iz `view-modes.json`. `CAD-34` eğri boyunca yazı
  ([ADR 0196](../../docs/adr/0196-text-along-curve.md)) tek parçada bitti (7 Ekim; ikonlar önerilen seçenekler, Doğrultuya döndür'ün ikincisi):
  sözleşmenin `TextPath`'i ve `text_path_problem`'i (`invalid_path`), `.kcad` şema 25 (`FORMATS_VERSION` 35), çekirdek `text::along`
  (bağımsız başvuru `text_along_cases.py`, ortak `fixtures/text/v1/along.json`), deponun harf başına `LABEL_LINE` kayıtları
  (`TextPlace::by_records`), DXF `writer/entities/curved.rs` ve `emit.rs`'in `curved_of`'u (`dxf-write/curved`); araçlar masaüstünde
  `kentos_interaction::text_along`, web'de `tools/textAlongTool.ts`; ortak iz `text-along.json` (oynatıcılarda `curve` beklentisi).
  `CAD-35` çizim ekleri ([ADR 0197](../../docs/adr/0197-drawing-extras.md)) tek parçada bitti (7 Ekim; ikonlar önerilenler, Menzil
  halkaları'nınki 16 px'te Halka'ya benzediği için yelpaze): çekirdek `tools::drawing_extras` (bağımsız başvuru
  `drawing_extras_cases.py`, ortak `fixtures/drawing-extras/v1/cases.json`), `cad.entities.create`'in `tangentLine`, `fourthCorner`,
  `rangeRings` adımları; araçlar masaüstünde `kentos_interaction::drawing_extras`, web'de `tools/drawingExtrasTools.ts`; ortak iz
  `drawing-extras.json`; 7 Ekim eki (sahibin bildirimi): Menzil halkaları bir takımdan sonra kapanır, ilk değerleri 5 m ve 3 halka. `CAD-36`
  plan yolu çizimi ([ADR 0198](../../docs/adr/0198-plan-roads.md)) tek parçada bitti (7 Ekim; geometri genel, değer tablosu **[M]** sahibin
  tarifini bekler): çekirdek `ops::road` (bağımsız başvuru `plan_road_cases.py`, ortak `fixtures/plan-road/v1/cases.json`; köşeler
  `ops::reshape::corners_of_path_where` ve `turns_inward` ile), `cad.entities.create`'in `planRoad`, `cad.entities.edit`'in
  `roadJunctions` ve `medianClose` adımları; araçlar masaüstünde `kentos_interaction::plan_road`, web'de `tools/planRoadTools.ts`; ortak iz
  `plan-road.json`. §16.1'in sahibin sırası (CAD-27 … CAD-36) bitti; `CAD-18` ve `CAD-24` **[M]** sahibin tarifini bekler.
  §16.2 CBS başladı (sahibin seçimi “GIS-01”, kapsamı üç kararı: kaynaklar klasörler ve KentOS projeleri, değerler metin ve şema
  türü verir, form şemadan): `GIS-01` katman alanları, öznitelik tablosu ve veri kaynakları
  ([ADR 0199](../../docs/adr/0199-layer-fields-and-feature-table.md)) tek parçada bitti (7 Ekim; ikonlar sahibin seçtikleri): sözleşmenin
  `fields`'ı (bağımsız başvuru `layer_field_cases.py`), `.kcad` şema 26 (`FORMATS_VERSION` 36), belgenin “Alanlar” adımı, komutların
  alan denetimi, çekirdek `ops::feature_table` (`feature_table_cases.py`); masaüstü `features/`, `layer_fields.rs`, `sources/`, web
  `ui/bottom/FeatureTable.ts`, `ui/layers/LayerFieldsDialog.ts`, `ui/sources/SourcesPanel.ts`; katmanı nesneleriyle alma `layerTake`
  (`exchange_cases.py`'nin `layers`'ı), klasör listesi `source_list_cases.py`; ortak iz `feature-table.json` (adım düzeyinde
  `featureTable` beklentisi); KentOS UI'da yalnız ikonu kalan dok sekmesi 28 px. `GIS-02` mekânsal ve öznitelik sorgusu
  ([ADR 0200](../../docs/adr/0200-spatial-and-attribute-queries.md); kapsam sahibin kararları: dört araç ailesi, İşlemler araçları, ilişkiler
  sonraya; ikonlar sahibin seçtikleri) tek parçada bitti (7 Ekim): çekirdek `ops::spatial_query`, deponun `relate_pairs`'i,
  `ops::statistics` (bağımsız başvuru `spatial_query_cases.py`, ortak `fixtures/spatial-query/v1` ve `fixtures/processing/v1/queries.json`);
  beş araç web'de `processing/builtin/`, masaüstünde `kentos-processing`'in `builtin/`'ında; İşlemler'e dosya parametresi, tablo çıktısı,
  `refused` ve katman alanları denetimi (`writeCheck.ts`, `writes.rs`); KentOS UI'ın parçalı seçimi `fill_widths` ile. `GIS-03` geometri
  işlemleri ([ADR 0201](../../docs/adr/0201-geometry-operations.md); kapsam sahibin kararları: dört grubun hepsi, İşlemler araçları, sonuç yeni
  katmana; simgeler sahibin seçtikleri; çizimdeki Kes ve Birleştir'le karışmasın diye adlar Kırp ve Gruplayarak birleştir) tek parçada bitti
  (7 Ekim): çekirdek `ops::geoprocess` (`buffer`, `clip`, `validity`, `reproject`, `calls`) ve `statistics::apportion` (bağımsız başvuru
  `geoprocess_cases.py`, ortak `fixtures/geoprocess/v1` ve `fixtures/processing/v1/geometry.json`: yeni nesneler ölçüleriyle `addedShapes`,
  araçların varsayılanları); on bir araç web'de `processing/builtin/geometry/`, masaüstünde `kentos-processing`'in `builtin/geometry/`'sinde;
  web'de `RunJob.crs`, masaüstünde araç `kentos_project::systems`'ten (işlem grubu artık `project`'e bağlanır); sonuç tablolarında sayı
  sütunları sağda; KentOS UI'ın simge çizicisi `stroke-opacity` okur. `GIS-04` topoloji kuralları
  ([ADR 0202](../../docs/adr/0202-topology-rules.md); kapsam sahibin kararları: katman içi ve katmanlar arası kurallar, alt panelde sekme,
  bulgudan düzeltme, kurallar ve istisnalar projede; simgeler sahibin seçtikleri) tek parçada bitti (7 Ekim): sözleşmenin `topology`'si,
  `.kcad` şema 27 (`FORMATS_VERSION` 37), çekirdek `ops::topology_rules` (`check`, `fix`; bağımsız başvuru `topology_rules_cases.py`,
  ortak `fixtures/topology-rules/v1`), `cad.entities.edit`'in `topologyFix`'i; sekme web'de `ui/bottom/TopologyPanel.ts` (`topologyRun.ts`,
  `topologyPlan.ts`), masaüstünde `topology/`; pencere `ui/topology/TopologyRulesDialog.ts`, `topology/rules.rs`; bulgunun işareti
  `drawProblemMark`, `marks.rs`; ortak iz `topology-rules.json` (oynatıcılarda `topology` beklentisi, sekmede `pick`). `GIS-05` ağ
  dengelemesi ve kot ağı ([ADR 0203](../../docs/adr/0203-network-adjustment.md); kapsam sahibin kararları: yatay ağ ve kot ağı, önsel doğruluklar
  Proje ayarları › Ölçme'de, aynı adlı noktalar güncellenir, yenileri eklenir; prizmatik aplikasyon ve kanava iptal; simgeler sahibin
  seçtikleri) tek parçada bitti (8 Ekim): sözleşmenin önsel doğrulukları (`SurveySigmas`, `SIGMA_DEFAULTS`), `.kcad` şema 28
  (`FORMATS_VERSION` 38), çekirdek `survey::adjust` (`horizontal`, `levelling`, `linalg`, `chi2`; bağımsız başvuru `network_adjust_cases.py`,
  ortak `fixtures/network-adjust/v1`), Karne editörünün satırları `survey::fieldbook`'un `network_rows`, `level_rows`'u;
  `cad.entities.edit`'in `networkAdjust`, `levelAdjust`'ı, `cad.entities.create`'in `networkAdjust`'ı; pencereler web'de
  `ui/calc/NetworkDialog.ts`, masaüstünde `calc/network/`; sahne `fixtures/interaction/v1/network-adjust.kcad`. `GIS-08` raster
  katmanları ([ADR 0204](../../docs/adr/0204-raster-layers.md); kapsam sahibin kararları: GeoTIFF ile dünya dosyalı PNG ve JPEG kendi okuyucumuzla,
  bütün dönüşümleriyle oturtma, bantlar, rampa ve gölgeli kabartma, bağlı dosya ve küçüklerin gömülmesi; ilke “Performance First”; simgeler
  sahibin seçtikleri) tek parçada bitti (8 Ekim): sözleşmenin `raster`'ı, `.kcad` şema 29 (`FORMATS_VERSION` 39), biçim çekirdeği
  `formats::raster` (`tiff`, `geotiff`, `codec`, `png`, `source`, `style`, `pyramid`, `write`, `warp`, `place`; bağımsız başvurular
  `raster_cases.py`, `raster_georef_cases.py`, ortak `fixtures/raster/v1`), geometri çekirdeğinin `geom::raster` ve `ops::georef`'i, stilli
  çizimin 6. sürümü; masaüstünde `rasters/` ve `calc/raster_fit.rs`, çizim hattı `styled/raster_tiles.rs`; web'de `io/rasterWorker.ts`,
  `render/rasterService.ts`, `render/rasterPass.ts`, `ui/raster/`; sahne `fixtures/interaction/v1/rasters.kcad`. `GIS-09` nokta bulutu
  ([ADR 0207](../../docs/adr/0207-point-clouds-and-large-data.md); kapsam sahibin kararları: LAS, LAZ, COPC ve XYZ; LAZ için `laz`; 2B plan, kat kat
  ve renk kipleriyle; dört işlem grubu; dosya, URL ve sanal bulut; simgeler sahibin seçtikleri) tek parçada, **yalnız masaüstünde** bitti
  (8 Ekim; sahibin kararı: “Nokta bulutu çok ağır bir iş … Sadece masaüstü ile kalalım”, web'de düğmeler notuyla bekler, web'in modülü
  TODOS.md `GIS-09`'un alt maddesi): sözleşmenin `pointcloud`'u ve rasterin `url`'si, `.kcad` şema 31 (`FORMATS_VERSION` 41), çekirdek
  `kentos-pointcloud` (`source`, `las`, `chunks`, `copc`, `text`, `crs`, `index`, `write`, `vpc`, `look`, `nodes`, `place`, `ops`; bağımsız
  başvurular `pointcloud_cases.py`, `pointcloud_index_cases.py`, `pointcloud_ops_cases.py`), geometri çekirdeğinin `geom::pointcloud`'u,
  noktaların çizim hattı (`styled/points.rs`, `shaders/wgsl/points`, stilli çizimin 7. sürümü); masaüstünde `pointclouds/`, İşlemler'in
  `builtin/pointcloud/`'u ve `files`'ı, rasterin adresi (`rasters/tiles.rs`'in `Origin::Url`'si); komut durumları `fixtures/commands/v1/desktop`;
  sahne `fixtures/interaction/v1/pointclouds.kcad`; süreler `perf::clouds`. Sahip aksini söyleyene dek her modül iki platformda, gerektiğinde
  bulut tarafıyla yapılır; platform sorulmaz, ikonları sorulmadan seçilir (sahibin kararları, 8 Ekim akşamı). `GIS-10` harita servisleri
  ve altlıklar ([ADR 0208](../../docs/adr/0208-map-services-and-basemaps.md); kapsam sahibin sözleri: sayılanların hepsi, Google ve vektör karolar,
  bütün kimlik doğrulama türleri, GeoJSON; iki platform ve bulut) tek parçada bitti (9 Ekim): sözleşmenin `service`'i (`ServiceLayer`,
  `FeatureFeed`, `ServiceConnection`), `cad_layers` (`cad.layers.service`), `.kcad` şema 32 (`FORMATS_VERSION` 42); çekirdek
  `kentos-services` ve geometri çekirdeğinin `geom::tiles`'ı (bağımsız başvurular `service_tiles_cases.py` pyproj'la,
  `service_mvt_cases.py` GDAL'la, `service_caps_cases.py` OWSLib'le, `service_rules_cases.py`, `layer_service_command_cases.py`); WASM
  `crates/wasm/services-wasm`; masaüstünde `services/` (hub, `net`, `cache`, `secrets`, `window`, `feed_window`, `connections`, `info`,
  `overlay`), çizim hattında `styled/service_tiles.rs`; web'de `io/services/` (`worker.ts`, `feedWorker.ts`, `fetch.ts`),
  `render/serviceHub.ts`, `render/servicePass.ts`, `ui/services/`, `ui/bottom/ServiceInfoPanel.ts`; sunucuda `http/proxy.rs`; Python
  `python/kentos/services.py`; süreler `perf::services`. `GIS-11` ağ analizi ([ADR 0209](../../docs/adr/0209-network-analysis.md); kapsamı
  sahibin sözüyle ben belirledim, “Biliyorsun önceliğimiz performans”) tek parçada bitti (9 Ekim): sözleşmenin `network`'ü ve
  `cad_networks`'ü (`cad.network.define`), `.kcad` şema 33 (`FORMATS_VERSION` 43); çekirdek `ops::network` (`input`, `graph`, `search`,
  `route`, `area`, `closest`, `trace`, `check`, `session`; bağımsız başvuru `network_cases.py`, kurallar `network_rules_cases.py`, form
  `network_form_cases.py`), tamponların birleşimi `buffer::union_pieces` ve `overlay::overlay_buffers`; web'de ağın işçisi `io/network/`,
  `app/networks.ts`, araçlar `tools/network*Tool.ts`, `ui/networks/`; masaüstünde `networks/` (ağın iş parçacığı `engine.rs`) ve
  `kentos_interaction::network`; İşlemler'in Ağ analizi kategorisi iki platformda (`network_processing_cases.py`); Python `kentos.network`;
  ortak izler `network-*.json`, sahne `networks.kcad`; süreler `network::timing`, `network_timing`. Açık kalanlar TODOS.md `GEO-01`,
  `GEO-02`. Raster analizi kümesi (`GIS-31`…`GIS-36`) başka bir ajandadır (sahibin kararı, 9 Ekim; onun ADR'leri 0230'dan başlar), bu
  sırada atlanır; dosyalarına dokunulmaz. `GIS-12` zaman ve senaryolar ([ADR 0210](../../docs/adr/0210-temporal-layers-and-scenarios.md);
  kapsamı ben belirledim) tek parçada bitti (9 Ekim): sözleşmenin `temporal`'ı (`LayerTime`, `ScenarioInfo`, `scenarios_problem`),
  `cad_layers`'ın `LayersTime`'ı ve `cad_scenarios`'ı, `.kcad` şema 34 (`FORMATS_VERSION` 44); çekirdek `time` ve deponun zamanları
  (bağımsız başvurular `temporal_cases.py`, `temporal_rules_cases.py`, `temporal_command_cases.py`); web'de `app/timeSlider.ts`,
  `app/timeCommands.ts`, `app/scenarios.ts`, `ui/time/`, `tools/timeVersionTool.ts`; masaüstünde `temporal/` ve
  `kentos_interaction::time_version`, stilli çizimin önbelleği pencerenin farkını parçalarla kurar; sunucu ağacın kurallarını denetler;
  Python `kentos.temporal`; komut durumlarında `$layer:K`; ortak izler `time-*.json`, `scenarios.json`, sahne `temporal.kcad`; süreler
  `time::timing`, `style::perf::temporal`, `scripts/perf/time.test.ts`. `GIS-13` ve `GIS-14` mevzuatla düzenlenen işlerdir (sona kalır).
  `GIS-15` katman süzgeci ([ADR 0211](../../docs/adr/0211-layer-filter.md); kapsamı ben belirledim) tek parçada bitti (9 Ekim): sözleşmenin
  `layer_filter`'ı (`LayerFilter`, `filters_problem`), `cad_layers`'ın `LayersFilter`'ı, `.kcad` şema 35 (`FORMATS_VERSION` 45);
  değerlendirme `kentos_native_application::layer_filter` ve web `model/layerFilter.ts` (liste bir kez kimliklere çevrilir, koşul
  İfadeyle seç'in sütun motoruyla yalnız listedekilere), deponun süzgeç işareti (`set_filtered`, `view_shown`), masaüstünde `Spatial`'ın
  `sync_filters`'ı, web'de `PickIndex`'in `markWhole`'u ve `model/idMarks.ts`; web'de `app/layerFilterCommands.ts`,
  `ui/layers/LayerFilterDialog.ts`; masaüstünde `layer_filters.rs`; sunucu ağacın kuralını denetler (`http::filter_tests`); Python
  `kentos.cad.layers.filter`, MCP'de araç; komut durumları `layer_filter_command_cases.py`; ortak iz `layer-filter.json` (yeni beklenti
  `layerCounts`); süreler `perf layer_filter`, `scripts/perf/filter.test.ts`.
  Paralel dal `gis-31-36-raster-analysis` (sahibin sözü, 9 Ekim:
  “yeni bir branch açarak GIS-31 ve GIS-36 aralığını yapacağız”, “yüksek performans ilk önceliğimiz”; ADR numaraları 0231–0236 bu aralığa
  ayrıldı): `GIS-31` yüzey analizi ([ADR 0231](../../docs/adr/0231-raster-analysis-and-surface.md)) tek parçada bitti (9 Ekim): çekirdek
  `kentos-raster` (`job`, `terrain`, `relief`, `insolation`, `contours`, `out`, `par`; bağımsız başvurular `terrain_cases.py` gdaldem'le,
  `contour_cases.py` gdal_contour'la), WASM `crates/wasm/raster-wasm`; İşlemler'in `builtin/surface/`'u iki platformda, ev sahibinin
  `Files::open_raster`'ı, katman parametresinin `above`'u; masaüstünde `DesktopFiles::open_raster`, `rasters/tiles.rs`'in `open_reader`'ı; web'de
  `io/rasterAnalysis*.ts`, `processing/rasterHost.ts`, `app/rasterAnalysis.ts`; ortak durumlar `fixtures/processing/v1/surface.json`
  (`surface_processing_cases.py`); süreler ADR'nin Doğrulama'sında. `GIS-32` interpolasyon ve yoğunluk
  ([ADR 0232](../../docs/adr/0232-interpolation-and-density.md)) tek parçada bitti (9 Ekim): geometri çekirdeğinde `predicates::incircle` ve
  `geom::delaunay` (bağımsız başvuru `delaunay_cases.py`); raster çekirdeğinde `points`, `grid`, `index`, `solve`, `interp` (`natural`,
  `spline`, `kriging`), `density`, `from_points` (bağımsız başvurular `interpolation_cases.py`, `bessel_k0.py`), WASM `PointAnalysis`;
  İşlemler'in `builtin/interpolation/`'u iki platformda, katman parametresinin `below`'u, `Beside::named`, web RunContext'in `project`'i;
  ortak durumlar `fixtures/processing/v1/interpolation.json` (`interpolation_processing_cases.py`). `GIS-33` raster işlemleri
  ([ADR 0233](../../docs/adr/0233-raster-operations.md)) tek parçada bitti (9 Ekim): raster çekirdeğinde `dd`, `stats`, `inputs`, `areas`,
  `calc`, `reclass`, `focal`, `resample`, `ops` (`OpsSpec::reads`: hesaplayıcının okuduğu rasterler, rasterler açılmadan), `out`'un her
  örnek türü (bağımsız başvuru `raster_ops_cases.py`, gdalwarp'la çapraz denetim); ifade dilinin matematik işlevleri; WASM `OpsOpening`,
  `OpsAnalysis`, `opsReads`; İşlemler'in `builtin/raster_ops/`'u ve `rasterOps/`'u iki platformda, girdinin özetinde raster adları
  (`features::raster_order`, `rasterRun`), `RunResult.above`; ortak durumlar `fixtures/processing/v1/raster-ops.json`
  (`raster_ops_processing_cases.py`). `GIS-34` raster ve vektör dönüşümü
  ([ADR 0234](../../docs/adr/0234-raster-vector-conversion.md)) tek parçada bitti (9 Ekim): raster çekirdeğinde `vector` (`label`, `rings`,
  `thin`, `simplify`, `capture`, `work`), `rasterize`, `inputs`'un `Raw`'ı; geometri çekirdeğinde `ops::contour_elevations` (bağımsız
  başvuru `raster_vector_cases.py`, GDAL'la çapraz denetim); WASM `OpsAnalysis`'in nesneleri; İşlemler'in `builtin/raster_vector/`'u ve
  `rasterVector/`'u iki platformda, `Patch.zs`; ortak durumlar `fixtures/processing/v1/raster-vector.json`
  (`raster_vector_processing_cases.py`), taranmış paftanın sahnesi `scanned.kcad` (`scanned_scene.py`). `GIS-35` hidroloji
  ([ADR 0235](../../docs/adr/0235-hydrology.md); sahibin sözü “Bu konu çok önemli iyi araştır”) tek parçada bitti (9 Ekim): raster çekirdeğinde
  `hydro` (`surface`, `heap`, `fill`, `tiled`, `flow`, `accum`, `basins`, `streams`; bağımsız başvuru `hydrology_cases.py`, GRASS'la
  çapraz denetim), `ops`'un sekiz türü, `vector`'ün `fields` ve `numbers`'ı; WASM `featureFields`, `featureNumbers`; İşlemler'in
  `builtin/hydrology/`'si iki platformda; ortak durumlar `fixtures/processing/v1/hydrology.json` (`hydrology_processing_cases.py`),
  resimlerin sahnesi `hydrology.kcad` (`hydrology_scene.py`). `GIS-36` uzaklık ve maliyet
  ([ADR 0236](../../docs/adr/0236-distance-and-cost.md)) tek parçada bitti (9 Ekim): raster çekirdeğinde `distance` (`edt`, `network`, `mod`;
  bağımsız başvuru `distance_cases.py`, GRASS'ın r.cost ve r.grow.distance'ı ve GDAL'ın Proximity'siyle çapraz denetim), `ops`'un dört
  türü, `from_points`'in `PointTool::Distance`'ı; WASM notlarında `distance`; İşlemler'in `builtin/distance/`'ı iki platformda, web
  koşucusunun yerleşimi görünen girdiyle; ortak durumlar `fixtures/processing/v1/distance.json` (`distance_processing_cases.py`),
  resimlerin sahnesi `distance.kcad` (`distance_scene.py`). Dal 9 Ekim'de `main`'e birleşti (sahibin sözü: “36 bitince main ile
  birleştir ve main push yap”); raster çözümleme kümesi bitti. `GIS-16` etiket motoru ([ADR 0212](../../docs/adr/0212-label-engine.md); sahibin
  sözü “bu konu çok önemli, QGIS falan nasıl çözüyor incele ve ona göre yap”: QGIS'in PAL'ı, Maplex ve MapLibre'nin çakışma dizini
  araştırıldı) tek parçada bitti (10 Ekim): sözleşmenin `labels`'ı ve `cad_labels`'ı, `.kcad` şema 36 (`FORMATS_VERSION` 46);
  çekirdek `labels` (`engine`, `geom`, `style`, `text`) ve deponun `placing`'i, adaylar gruplar hâlinde gerektikçe (bağımsız başvuru
  `label_engine_cases.py`, 43 durum; komut durumları `label_command_cases.py`); metinler `kentos_native_application::label_texts` ve web
  `model/labelTexts.ts`; Etiketleri yazıya çevir motorun yerleşimiyle (`ops::label_text`, ADR 0175'in eki); web'de `app/labelCommands.ts`,
  `ui/labels/`, `tools/labelTools.ts`, `viewport/placedLabels.ts`; masaüstünde `labelling/`, `kentos_interaction::label_tools`; pafta PDF'ine
  zeminler ve çağrı çizgileri; sunucuda `label_pins` sütunu ve göç 0015 (tür denetimi kılavuz, tablo, resim, raster ve nokta bulutunu da
  alır); Python ve MCP; ortak iz `label-tools.json` (yeni beklenti `labelPins`); süreler ADR'nin Doğrulama'sında. `GIS-17` ek işleyiciler
  ([ADR 0213](../../docs/adr/0213-additional-renderers.md); kapsamı ben belirledim) tek parçada bitti (10 Ekim): stil çekirdeğinde dokuz tür
  (`model.rs`, `rules.rs`, `thematic.rs`, `dots.rs`, `charts.rs`, `groups.rs`, `heat.rs`, `inverted.rs`; kurulumun `build_layer_in`'i ve
  `ViewFrame`'i, `BatchSink::fan`; bağımsız başvuru `renderer_cases.py`, 124 durum); işleyici KCAD'de opak, şema değişmez; komut
  `cad.layers.renderer` iki platformda, başsız sunucuda, Python'da ve MCP'de (`renderer_command_cases.py`, 45 durum); masaüstünde stilli
  sahnenin `ViewBuilt`'i, resim kaynağının `put_made` ve `keep_made`'i, Katman stili'nin `thematic.rs`'i; web'de `ui/style/thematicPanels.ts`,
  `style/thematic.ts`; KentOS UI'da açıklamalı radyo satırlarının ikonları; ortak `legend.json` ve `batches.json`; resimlerin sahnesi
  `renderers.kcad` (`renderer_scene.py`); süreler `renderer_timing` ve `scripts/perf/renderers.test.ts`. `GIS-18` ifade dili ekleri
  ([ADR 0214](../../docs/adr/0214-expression-extras.md); kapsamı ben belirledim, `regex-lite` sahibin onayıyla) tek parçada bitti (10 Ekim):
  çekirdekte `dates`, `patterns`, `arrays`, `compound`, `resolve` ve `world` (`roles`, `aggregate`, `spatial`, `Session`), `library/extras.rs`'in
  71 işlevi; `compile_with` başka katmana bakanı şema izin vermedikçe reddeder (`WORLD_REFUSED`); sözleşmenin `variables`'ı, `.kcad` şema 37
  (`FORMATS_VERSION` 47); `kentos_project::variables` ve `variable_form`, web `model/projectVariables.ts` ve `model/variableForm.ts`; masaüstünün
  İşlemler'i sütun motorunda (`EntityObjects`, `evaluate_in`, `runner::world_layers`); web'de `exprCompileIn`, `evaluateExpressionIn`,
  `ExprWorld`, `RunJob.variables` ve `leftOut`; Proje ayarları › Değişkenler iki platformda (`project/variables.rs`, `variablesSection`);
  pafta `ProjectInfo.variables`; Python `ProjectVariable`; bağımsız başvurular `expression_extras.py` (190 kaynak), `variable_form_cases.py`,
  `spatial_query_cases.py`'nin ifade durumları; süreler `measures_the_additions` ve `scripts/perf/expressionExtras.test.ts`. `GIS-19`
  yakınlık analizi ([ADR 0215](../../docs/adr/0215-proximity-analysis.md); kapsamı ben belirledim) tek parçada bitti (10 Ekim): çekirdek
  `ops::proximity` (en yakın noktalar, ortak kenar, içlerin örtüşmesi kutularla daraltılarak) ve `store::proximity` (`nearest`, `neighbors`),
  `store::rtree`'nin en iyi önce araması (`PackedTree::nearest`, `Nearer`); İşlemler'in Yakınlık kategorisi iki platformda (web
  `processing/builtin/proximity/`, masaüstü `kentos-processing`'in `builtin/proximity/`); bağımsız başvuru `proximity_cases.py` (9 durum);
  pencereler `processing::proximity_tests`, `shots.mjs proximity`; süreler `proximity::timing` ve `scripts/perf/proximity.test.ts`. `GIS-20`
  anahtarla veri aktarma ([ADR 0216](../../docs/adr/0216-data-transfer.md); kapsamı ben belirledim) tek parçada bitti (10 Ekim): İşlemler'in
  Alan eşleme parametresi (web `fieldPlan.ts`'in `fieldMapView`'i ve `paramFields.ts`'in `fieldMapField`'ı, masaüstü `kentos_processing::field_map`
  ve `fields.rs`'in `field_map`'i), `allowEmpty`, `FeatureSet.layerId`, `RunContext.fields`, şeklin yaması (`Patch.geometry`); Veri aktar
  web `processing/builtin/transfer.ts`, masaüstü `builtin/transfer.rs`, kaynağın satırları Anahtarla birleştir'le ortak (`joinSource.ts`,
  `join_source.rs`); bağımsız başvurular `transfer_cases.py` (10 durum) ve `field_map_cases.py` (27 durum); pencereler
  `processing::transfer_tests`, `shots.mjs transfer`; süreler `transfer_timing` ve `scripts/perf/transfer.test.ts`. Masaüstünde dosyanın
  satırları kopyalanmadan okunur, çalıştırıcı yamanın parçalarını taşır. `GIS-21` katman bölme ve birleştirme
  ([ADR 0217](../../docs/adr/0217-layer-split-and-merge.md); kapsamı ben belirledim) tek parçada bitti (11 Ekim): çekirdek `ops::order` (işlem
  `orderBy`, doğal sıra `text::natural`'ın `tokens_cmp`'siyle); İşlemler'in Katmanlar parametresi (`layers`), aracın katmanları
  (`RunResult.layers`, `ToolLayer`; çalıştırıcı tek adımda açar) ve `like`, masaüstünde `Patch.layer`; beş araç web
  `processing/builtin/layers/`, masaüstü `builtin/layers/`; Yinelenenleri sil değer sınıfları, ön süzgeç ve masaüstünde özetlenen şekille;
  bağımsız başvuru `layer_ops_cases.py` (16 durum); pencereler `processing::layer_tools_tests`, `shots.mjs layertools`; süreler
  `layer_timing` ve `scripts/perf/layers.test.ts`. `GIS-22` geometri dönüştürme
  ([ADR 0218](../../docs/adr/0218-geometry-conversion.md); kapsamı ben belirledim) tek parçada bitti (11 Ekim): çekirdek `ops::convert`
  (`points`: merkez, GEOS'un iç noktası yaylarıyla, köşeler; `paths`: parça, halkalar, alan, köşe ekle; `table`: satırlar ve şekiller;
  web'e toplu `calls`); on araç web `processing/builtin/convert/`, masaüstü `builtin/convert/`, kategori `conversion`; ortak durumlarda
  hesaplanan alanlar için `toleranceFields`; çalıştırıcı geçmiş kaydında dosya satırlarını tutmuyor, hesap değerleri ödünç alıyor, büyük
  değerler ayrı iş parçacığında bırakılıyor; bağımsız başvuru `convert_cases.py` (23 durum, iç noktalar GEOS'la çapraz); pencereler
  `processing::convert_tests`, `shots.mjs geoconvert`; süreler `convert_timing` ve `scripts/perf/convert.test.ts`. `GIS-23` geometri
  üretimi ([ADR 0219](../../docs/adr/0219-geometry-generation.md); kapsamı ben belirledim) tek parçada bitti (11 Ekim): çekirdek `ops::generate`
  (`voronoi`: çevrel merkezlerden hücreler, aynı çemberdekiler bir merkez, çerçeveye ortak kesişimlerle; `bounding`: kesin zarf, dönen
  kumpasla en küçük kutu, sabit tohumlu Welzl; `concave`: JTS'in aşındırması, delikler büyükten küçüğe; `pole`: etiket motorunun
  polylabel'ı; `points`: eğrilerin 0,1 mm'lik noktaları ve eksen uçları; web'e toplu `calls`); beş araç web `processing/builtin/generate/`,
  masaüstü `builtin/generate/`, kategori `generation`, birim `%`; web'de çekirdeğe yalnız geometri gider (`entityGeometry`); bağımsız başvuru
  `generate_cases.py` (20 durum, kesirler ve GEOS); pencereler `processing::generate_tests`, `shots.mjs generate`; süreler
  `generate_timing` ve `scripts/perf/generate.test.ts`. `GIS-24` örnekleme ve ağ üretimi
  ([ADR 0220](../../docs/adr/0220-sampling-and-grids.md); kapsamı ben belirledim) tek parçada bitti (11 Ekim): çekirdek `ops::sampling` (`grid`,
  `cover`, `inside`, `regular`, `random`, `scatter`, `walk`, `along`, `cut`, `subdivide`, `calls`; web'e ağlar ve noktalar düz sayı
  dizileriyle); `WindingIndex::owned`; beş araç web `processing/builtin/sampling/`, masaüstü `builtin/sampling/`, kategori `sampling`;
  Alt bölümlere ayır'ın alanı genel overlay yerine doğru boyunca kesilir (`cut`: overlay'le aynı alanlar ve köşeler, 10 000 köşede 297 ms
  yerine 2 ms); bağımsız başvuru `sampling_cases.py` (29 durum, GEOS ve kesirler); pencereler `processing::sampling_tests`,
  `shots.mjs sampling`; süreler `sampling_timing` ve `scripts/perf/sampling.test.ts`. `GIS-25` kartografik genelleştirme
  ([ADR 0221](../../docs/adr/0221-cartographic-generalization.md); kapsamı ben belirledim) tek parçada bitti (11 Ekim): çekirdek `ops::generalize`
  (`smooth`, `orthogonal`, `building`, `coverage`: kapsama grafı köşe numaralarıyla, `simplify`: Visvalingam–Whyatt sayım ızgarasıyla,
  `parts`, `aggregate`: kümeler ve kapanış ayrı, `eliminate`, `calls`; web'e toplu işlemler, değişmeyen nesnenin şekli dönmez); yedi araç web
  `processing/builtin/generalize/`, masaüstü `builtin/generalize/` (kümeler iş parçacıklarında), kategori `generalization`; şeridin ▾ listesi
  İşlemler kategorileriyle başlıklı (`headed_menu_of`, `overflowRows`); kesişim çekirdeğinde üç sağlamlık kuralı (`geom::arrangement`'in
  `Circles`'ı, yakın kesişimlerin kesmesi, `balance`; ADR 0201'in eki); bağımsız başvurular `generalize_cases.py` (22 durum) ve
  `closing_cases.py` (45 blok; 2 700 rastgele blokla zorlandı); pencereler `processing::generalize_tests`, `shots.mjs generalize`; süreler
  `generalize_timing` ve `scripts/perf/generalize.test.ts`. `GIS-26` KML/KMZ, GPX ve içe aktarmada açık dönüşüm
  ([ADR 0222](../../docs/adr/0222-kml-gpx-and-explicit-transformation.md); kapsamı ben belirledim) tek parçada bitti (11 Ekim): biçim çekirdeğinde
  `kml` (`read`, `style`, `write`), `gnss::gpx_write`, `reproject` (içe aktarılanın projenin sistemine çevrilmesi), `export_geom` (GeoJSON,
  KML ve GPX'in ortak geometrisi, biçimin sözleriyle), `zip::write`, `geojson::write_wgs84`, GPX ve NMEA okuyucularının çizgileri
  (`GnssLine`); `FORMATS_VERSION` 49; WASM `readKml`, `writeKml`, `writeGpx`, `writeGeoJsonWgs84`, `reprojectImport`; koordinat sistemi
  sorusunun çevirme onayı iki platformda (`CrsQuestion::convertible`, `way`, `ready`); masaüstü `exchange/wgs84_export.rs`, web
  `ui/io/Wgs84ExportDialog.ts`; GNSS'in çizgileri `kentos_interaction::gnss::place_all`, `model/gnssImport.ts`'in `placeGnssAll`'ı;
  bağımsız başvurular `kml_cases.py` (7 örnek, 4 taşıma, yazıcının 3 dosyası) ve `wgs84_write_cases.py` (GPX ve GeoJSON), GNSS
  başvurularının çizgileri; pencereler `exchange::kml_tests`, `shots.mjs kml`; süreler `kml_timing` ve `scripts/perf/kml.test.ts`.
  `GIS-27` tablo alışverişi ([ADR 0223](../../docs/adr/0223-table-exchange.md); kapsamı ben belirledim) tek parçada bitti (11 Ekim): biçim
  çekirdeğinde `xml_scan` (akış okuyucusu: önekler önbellekte, etiketin türü baytından), `table_file`'ın `Limits`'i ve `read_with`'i
  (Tablo ekle 10 001 × 101, İşlemler 250 001 × 501, sessiz kısaltma kalktı), `xlsx`, `ods`, `dates` (openpyxl'in tarih kuralı),
  `workbook` (XLSX, ODS, CSV yazıcısı; hücreler dilim dilim, büyük parçalar `zip::Pack::Fast`'la), `coords::NOT_TEXT`; geometri
  çekirdeğinde `ops::convert::table::point_rows` (`convertPointRows`); `FORMATS_VERSION` 50; WASM `readTableFile(bytes, processing)`,
  `writeWorkbook`; İşlemler'in dosya alanında Sayfa listesi ve ayrı iş parçacığında okuma (masaüstü `FileRead`, `Sheet`), Tablodan nokta iki
  platformda, sonuç tablolarında Kaydet ▾ (`SaveTableAs`); masaüstü `table_write.rs`, web `ui/bottom/featureExport.ts`; Öznitelik tablosunun
  Dışa aktar ▾'si (`workbook_sheet`, `featureSheet`); bağımsız başvurular `table_file_cases.py`, `workbook_cases.py`, `table_points_cases.py`,
  `feature_export_cases.py`; pencereler `processing::table_points_tests`, `shots.mjs tableexchange`; süreler `workbook_timing`,
  `table_points_timing`, `scripts/perf/workbook.test.ts`, `scripts/perf/convert.test.ts`. `GIS-28` doğrusal referanslama
  ([ADR 0224](../../docs/adr/0224-linear-referencing.md); kapsamı ben belirledim) bitti (11 Ekim; iki commit: sahibin isteğiyle yarıda
  bırakılıp WIP olarak gitti, kalanı ikinci commit'te): sözleşmenin `ma`, `mb`, `ms`'i, `.kcad` şema 39
  (`FORMATS_VERSION` 51); düzenlemelerde M kotun kurallarıyla (`measure_lost`, `invalid_measures`); Shapefile'ın M'si; çekirdek `ops::lrs`
  (`Walk`, `create`, `calibrate`, `overlay`, `area`, `index`, `calls`; bağımsız başvuru `lrs_cases.py`, 59 durum); İşlemler'in
  `builtin/lrs/`'ı iki platformda (`lrs_processing_cases.py`, 14 durum; masaüstünde pencereden `processing::lrs_tests`); Öznitelikler,
  köşe tablosu ve kartta M; Km yaz'ın Ölçü (M)'si (`measure_stations`, `stationingMeasure`; ortak iz `stationing-measure.json`; hattın
  rotası web'de `model/ops/lrs.ts`'in `routeOf`'u, masaüstünde `kentos_native_application::geometry::route`); masaüstünün komut
  satırında yazılan tuş kendi seçeneğini seçer (KentOS UI `keyed`); resimlerin yolu `lrs_scene.py`; süreler `lrs::timing` ve
  `scripts/perf/lrs.test.ts`. `GIS-29` nesne ekleri ([ADR 0225](../../docs/adr/0225-object-attachments.md); kapsamı ben belirledim) tek
  parçada bitti (12 Ekim): sözleşmenin `attachment`'ı ve `layer_info`'su, `cad.layers.info`, `cad.entities.set`'in `attachments`'ı,
  `.kcad` şema 40 (`FORMATS_VERSION` 52), sunucuda göç 0016; çekirdek `kentos_expression::template` (`template_cases.py`),
  `kentos_formats::exif` (`exif_cases.py`), `kentos_interaction::photos` (`photo_import_cases.py`, PROJ) ve `::identify`,
  `kentos_native_application::info_templates` ve `layers_info` (`layer_info_command_cases.py`); masaüstünde `attachments.rs`,
  `photo_view.rs` (kendi wgpu boru hattı: yeni bağımlılık yok), `object_info.rs`, `info_card.rs`, `layer_info.rs`,
  `exchange/photo_import.rs`, `properties/rows/attachments.rs`; web'de `app/attachments.ts`, `app/objectActions.ts`, `ui/attachments/`,
  `ui/shell/InfoCard.ts`, `ui/layers/LayerInfoDialog.ts`, `ui/io/PhotoImportDialog.ts`; ortak iz `identify.json` (yeni beklenti
  `infoCard`, sahne `attachments_scene.py`); masaüstü testleri `attachments_tests`; süreler `exif::timing`, `info_templates`,
  `a_card_opens_in_a_frame`, `scripts/perf/attachments.test.ts`. `GIS-30` sembol katmanı ekleri
  ([ADR 0226](../../docs/adr/0226-symbol-layer-extras.md); kapsamı ben belirledim) tek parçada bitti (12 Ekim): sözleşmenin `layer_compose`'u
  (`LayerNode.compose`), `cad.layers.compose`, `.kcad` şema 41 (`FORMATS_VERSION` 53); stil çekirdeğinde `style/generate.rs` ve efekt grupları
  (`compile.rs`); çizim alanı `shaders/wgsl/compose` (sözleşmenin 2. sürümü), masaüstü `crates/render/wgpu/src/compose.rs`, web
  `render/composePlan.ts` ve iki arka ucun `composer.ts`'i (WebGL2'nin ikizi `composeShaders.ts`): paketli haritalar, x boyunca sütunlara
  ve y boyunca satırlara, bulanıklığın dokunuşları ikişer, grubun efektleri tek geçişte, çizim resminin geçişleri tek render geçişinde; ilk
  uygulama 19 ms'ydi, zaman damgalarıyla geçiş geçiş ölçülerek 6 ms'ye indi; tasarımcıda `generator.rs`, `layerForms.ts` (Adım ekle'de
  ikonlar, sahibin isteği), Katman stili'nde `layer_style/compose.rs`, `composeSection`; bağımsız başvurular `geometry_generator_cases.py`,
  `effects_cases.py` (gerçek GPU ve tarayıcıyla), `layer_compose_command_cases.py`; ortak iz `symbol-extras.json`; süreler ADR'nin
  Doğrulama'sında. Ardından sahibin bildirimleriyle iki düzeltme (12 Ekim): masaüstünde ikonlar kendi piksellerinde SVG belgesi
  olarak resvg ile çizilir (ADR 0054'ün eki); kendi hedefine çizen alanın resmi tam piksellere oturur, tasarımcının önizlemesinde
  ince çizgi ve dalga düzgün (ADR 0023'ün eki). Devir notu ve açık iş (yarı saydam çizginin köşesi iki kez boyanır):
  [docs/devir/2026-10-12-gis-30.md](../../docs/devir/2026-10-12-gis-30.md). `GIS-39` grafikler ve alan istatistikleri
  ([ADR 0227](../../docs/adr/0227-charts-and-field-statistics.md); kapsamı ben belirledim) tek parçada bitti (12 Ekim): çekirdek `ops::charts`
  (`data`, `fields`, `specs`, `model`, `ticks`, `fonts`, `layout`, `svg`, `work`, `json`; bağımsız başvuru `chart_cases.py`, 94 durum),
  `ops::statistics`'in `figures_of`'u ve hızlandırılması (ayırmasız `read_number`, Clinger'in `to_f64`'ü; sonuçlar bit bit aynı); WASM
  `ChartWork`; web'de `app/charts.ts`, `model/chartData.ts`, `model/ops/charts.ts`, `ui/bottom/StatisticsPanel.ts`, `ChartPanel.ts`,
  `ui/charts/`; masaüstünde `kentos_interaction::chart_data`, `statistics/`, `charts/` (şekiller tiny-skia ile ekranın pikselinde, PNG
  resvg ile); İşlemler penceresinde Grafik; ortak iz `charts.json` (oynatıcılarda `chartPress`, `chart` ve `statistics`), sahne
  `chart_scene.py`; süreler ADR'nin Doğrulama'sında (ilk ölçümde üç iş bütçeyi aşıyordu). Aynı commit'te sahibin bildirimiyle CAD'in
  koordinat ekseni simgesi her zaman sol altta (0,0 ekrandayken orijine gidiyordu; ADR 0165'in 12 Ekim eki, resmi `tools_screens`'in
  `cad-eksen-simgesi`'si) ve macOS'ta düşen üç denetim: pafta ve vitrinin birer testi (Ctrl yerine platformun komut tuşu,
  `Modifiers::COMMAND`) ve web'in iz oynatıcısının Ctrl'li tıklaması (macOS'ta Chrome onu sağ tık sayar; `tracePlayer.mjs` orada ⌘ basar;
  Veride ara'nın ve Topoloji kuralları'nın izleri). Ardından (12 Ekim) Grafik'in Tablo görünümü dar panelde sayılarını gösterir
  (nokta editörünün `ptable` kuralları ezildi) ve sahibin bildirimiyle ifadeler çizim biriminde (ADR 0165'in “ifadeler çizim
  biriminde” eki): mm ya da cm projesinde etiketin metni ve koşulu, bilgi kartı, katman süzgeci, tablo süzgeci, oluşturucu ve
  İstatistik/Grafik'in Alan ve Uzunluk'u çizim biriminde ve karesinde (Öznitelikler gibi); İşlemler, ağ maliyetleri ve semboller
  metrede. Birim derlemenin şemasında (`Schema::unit`, web bağlamının `unit`'i), çözümleyici geometri değerlerini ve başka katmana
  bakan işlevlerin ölçülerini çevirir (`resolve`); okuma birimi `expression_unit::reading_unit`, web `readingUnit`; birimli
  durumlar `geometry.json`'un `units`'inde; 13 Ekim'de sahibin bildirimiyle katmanın işleyicisinin ifadeleri de (Kategorili, Aralıklı,
  Kurallı, tematik işleyiciler: Katman stili penceresi, çizim alanı, pafta haritası ve PDF) okuma sayılır ve projenin biriminde
  okunur (ADR 0165'in 13 Ekim eki: `Program::read_in`, `BuildOptions::reading_unit`, `Look::reading_unit`, `ExprScope::unit`);
  sembolün kendi ifadeleri, İşlemler, ağ maliyetleri ve pafta tabloları metrede kalır. Sahibin kararı geldi (12 Ekim): **web katmanına ara**
  ([ADR 0228](../../docs/adr/0228-web-pause-desktop-and-cloud-first.md)). Web'in son hâli `1aad00d0`'dır; bundan sonra masaüstü ve
  gerektiğinde bulut. `GIS-40` adres arama ([ADR 0229](../../docs/adr/0229-address-search.md); kapsamı
  sahibin kurallarıyla ben belirledim) tek parçada, yalnız masaüstünde bitti (12 Ekim): yeni komutların arayüz kaydı
  `catalog/v1/additions.json` (komutlar, ikonlar, şerit yerleri; sözleşmenin kuralı `ui_catalog`, masaüstünün okuyucusu `catalog.rs`'in
  `place_additions`'ı; `catalog/README.md`); sözleşmenin `AddressLocator`'ı ve `cad_addresses`'i (`cad.addresses.define`), `.kcad` şema
  42 (`FORMATS_VERSION` 54); çekirdek `ops::geocode` (`fold`, `parse`, `similar`, `locator`, `forward`, `reverse`, `format`, `check`;
  harf maskesiyle kesin ön süzgeç, adaylar iki aşamada; bağımsız başvuru `geocode_cases.py`), sağlayıcılar `kentos_services::geocode`
  (`providers`, `request`, `read`, `score`; `geocode_online_cases.py`); masaüstünde `address/` (sekme, adres iş parçacığı `engine.rs`,
  sağlayıcıların ev sahibi `online.rs`, pencereler `data.rs` ve `providers.rs`), araç `kentos_interaction::address_reverse`, alanların
  tahmini `address_form`; İşlemler'in `kentos_processing::address`'i ve `builtin/address/`'i (ortak durumlar `address.json`); Python
  `kentos.address`; ortak izler `address-search.json`, `address-reverse.json` (yeni beklenti `address`), sahne `address_scene.py`; süreler
  ADR'nin Doğrulama'sında. Sağlayıcılarla canlı deneme yapılmadı. Ardından sahibin talimatı (12 Ekim): “Plan/CAD.md dosyası ile
  devam edelim”; planın §24.1'deki on iki iş TODOS.md §16.5'te (`CADP-01`…`CADP-12`). `CADP-01` CAD işlem manifesti
  ([ADR 0244](../../docs/adr/0244-cad-operation-manifest.md)) bitti (12 Ekim): eşleme `catalog/v1/operations.json`, üreteç
  `scripts/cad/manifest.py`, çıktı `docs/cad/manifest.{json,md}` (654 satır; CBS'ye özgü olmayan 449 satırın 243'ü Python ve AI için
  boşluk). `CADP-02` ([ADR 0247](../../docs/adr/0247-sheet-commands-on-the-desktop.md)) bitti: pafta komutlarının masaüstündeki yerleri
  `equivalents.json`'da, Ad ver yerinde (liste, ağaç, F2); `kentos-sheet-ui` masaüstü takımında. Sahibin izniyle (12 Ekim) iki paralel ajan: `CADP-03` (dal `cad-units`, ADR 0245; PR #21 birleşti: Python ve MCP'de yeni
  çizimin birimi ve sihirbazın parametreleri, cevaplarda `units`) ve `CADP-04` (dal `cad-offset`,
  ADR 0246; PR #22 birleşti: `cad.entities.offset`, araç komutun planıyla); ağır işler `/tmp/kentos-heavy.sh` kilidiyle sırayla. PR'leri ana ajan birleştirir; birleştirirken eşleme ve manifest
  yeniden yazılır.
  Paralel dal `gis-37-38-42-43` (sahibin sözü, 10 Ekim: “GIS-37, 38, 42, 43 maddelerini tamamla”, “Yeni branch içinde yap bunları”;
  ADR numaraları 0237, 0238, 0242, 0243): `GIS-37` çok ölçütlü uygunluk ([ADR 0237](../../docs/adr/0237-multi-criteria-suitability.md)) tek
  parçada bitti (10 Ekim): raster çekirdeğinde `suitability` (`pairwise`, `roc`), `ops`'un altı türü, `reclass::parse_with`; WASM
  notlarında `suit`, `roc()`; İşlemler'in `builtin/suitability/`'ı iki platformda; İşlemler'in iki yeni parametre türü (`rasterValues`,
  `rasterPairs`; masaüstünde `raster_rows`, web'de `fieldPlan.ts`'in satırları), tablo veren çalıştırmadan sonra tablo görünür olur;
  bağımsız başvuru `suitability_cases.py` (50 durum), ortak durumlar `fixtures/processing/v1/suitability.json`
  (`suitability_processing_cases.py`), resimlerin sahnesi `suitability.kcad` (`suitability_scene.py`); CBS'nin Raster sekmesinde Uygunluk
  analizi paneli, sekme 1100 px'e sığsın diye İnterpolasyon ve Yoğunluk tek panelde. `GIS-38` mekânsal istatistik
  ([ADR 0238](../../docs/adr/0238-spatial-statistics.md)) tek parçada bitti (10 Ekim): geometri çekirdeğinde `ops::spatial_stats` (`kdtree`,
  `centers`, `nearest`, `weights`, `autocorrelation`, `clusters`, `calls`; bağımsız başvuru `spatial_stats_cases.py`, 68 durum); web'in
  `model/ops/spatialStats.ts`'i; İşlemler'in `builtin/stats/`'u iki platformda; yeni katmanın görünüşünde kategorili görünüş
  (`NewLayerStyle.renderer`); ortak durumlar `fixtures/processing/v1/spatial-stats.json` (`spatial_stats_processing_cases.py`; yeni
  katmanın yeri `layerAbove`, karşılaştırıcılarda `r`, `major`, `ratio` geometri alanı), resimlerin sahnesi `spatial-stats.kcad`
  (`spatial_stats_scene.py`); CBS'nin Analiz sekmesinde (birleşimde Yakınlık'la Analiz paneline katıldı: ayrı panel 1100 px'te
  sığmıyordu). `GIS-42` uzaktan algılama
  ([ADR 0242](../../docs/adr/0242-remote-sensing.md)) tek parçada bitti (10 Ekim): raster çekirdeğinde `remote` (`spectral`, `classify`,
  `cluster`, `accuracy`, `work`; işin okuma geçişleri, `OpsFinished::Report`, notlarda `remote`, sonuca göre görünüş); bağımsız başvuru
  `remote_cases.py` (46 durum, Brovey GDAL'la bire bir); raster okuyucusunda ExtraSamples'ın 0'ı artık alfa değil (`raster_cases.py`'ye
  GDAL'ın alfa okuması ve iki dosya); İşlemler'in `builtin/remote/`'u iki platformda; ortak durumlar `fixtures/processing/v1/remote.json`
  (`remote_processing_cases.py`, `remoteOf`), resimlerin sahnesi `remote.kcad` (`remote_scene.py`); CBS'nin Raster sekmesinde Uzaktan
  algılama paneli (altı araç, ikisi ▾'de), sekme 1100 px'e sığsın diye Raster ve vektör ile Taranmış harita tek panelde. `GIS-43` mesh ve
  çok boyutlu veri ([ADR 0243](../../docs/adr/0243-mesh-and-multidimensional-data.md); sahibin sözü “o tamamen bitecek”) tek parçada bitti
  (10 Ekim): sözleşmenin `RasterDataset`'i ve görünüşün `edges`'i, `.kcad` şema 38 (`FORMATS_VERSION` 48; `main`'in şema 37'si proje değişkenlerinindir); biçim çekirdeğinde
  `multidim` (`netcdf`, `cf`, `ugrid`, `sms`, `mesh`, `cube`, `series`, `write`; bağımsız başvuru `multidim_cases.py` kendi NetCDF yazıcısı
  `netcdf_classic.py`'yle, GDAL ve QGIS'in MDAL'ıyla çapraz denetim), raster çekirdeğinde `multidim` (`points`, `profile`, `series`,
  `calc`); WASM `NetcdfFile`, `smsToUgrid`, `CubeOpening`, `MultidimAnalysis`; İşlemler'in `builtin/multidim/`'i iki platformda,
  `Files::open_cube`; masaüstünde `rasters/multidim.rs`, web'de `ui/raster/MultidimDialog.ts` ve çözümleme işçisinin birleştirilmiş
  okumaları (`readRuns`); ortak durumlar `fixtures/processing/v1/multidim.json` (`multidim_processing_cases.py`), resimlerin sahnesi
  `multidim.kcad` (`multidim_scene.py`). Yolda düzelenler: NetCDF okuyucusu `vsize`'ı işaretli okuyup 2–4 GiB'lık değişkeni reddediyordu;
  zaman sürgüsünün yazıları çekirdekten ve çubuk kendi genişliğine sığar (ADR 0210'un eki). Dal 10 Ekim'de `main`'e birleşti (sahibin
  sözü: “Görevin bitince github main ile birleştir ve main üzerine push yap”).
  `GIS-06` ve `GIS-07` mevzuatla
  düzenlenen işlerdir: yol haritasının en sonuna kalır, sahiple ayrı çalışma ister; §16.2 sırasında atlanır (sahibin kararı, 7 Ekim).
  `HYB-24` canlı GNSS ertelendi (sahibin kararı, 5 Ekim: elde alıcı yok); sıra gelince atlanır; sahip cihazı bulunca söyleyecek.
  4 Ekim: derleme ve test süreleri
  ([ADR 0170](../../docs/adr/0170-build-and-test-times.md)). Sahibin sorusu üzerine (4 Ekim) pyproj'la rastgele fark testi eklendi (`crs_sweep.py`; PROJ'un kendi `+towgs84`
  yolu dahil, bilinen tek fark TUREF'e 0,1 mm, ADR 0168 Doğrulama). 3 Ekim:
  pafta düzeni dalı (PR #17, [ADR 0164](../../docs/adr/0164-sheet-layouts.md)) sahibin sözüyle `main`'e birleşti; birleştirmeden
  sonraki ayrı işler `docs/sheet/integration.md` §6'da. 2 Ekim: web'in klasik arayüzü kaldırıldı, iki platformda yalnız şerit var (sahibin
  kararı, [ADR 0155](../../docs/adr/0155-web-ribbon-only.md)). PDF, yazdırma ve pafta çıktısı (§16.4) en
  sondadır, zamanını sahip söyleyecek. İşler ADR 0142–0148'deki gibi: önce ADR ve adımları, sonra adım adım iki platformda,
  ortak fixture'larla.
- Web `pnpm e2e` duman testi 2 Ekim'de bütünüyle geçiyor (203 denetim). ADR 0143'ten önce de düşen
  üç testten araç kutusununki araç kutusuyla kalktı (ADR 0155); nokta hesaplayıcının “yan nokta 30/5”
  adımı ve “Layers panel: counts follow add, undo and redo” araç kutusu kalkınca iki koşuda geçti (kök
  nedenleri araştırılmadı). Önceki dilimlerin bozduğu dört denetim aynı gün düzeltildi: Uyarılar
  sekmesinin yeri (Noktalar), gizli “Diğer” çipi, DXF içe aktarmada Blokları patlat, CAD modunun
  Ölçme sekmesi.
- Çalışma düzeni: çekirdek, masaüstü ve web aynı oturumda (29 Eylül akşamından beri alt ajan yok); her
  iş iki platformda kullanılarak resimlenir, commit'lenir; main belli noktalarda (tamamlanan adımlardan sonra) push'lanır (sahibin isteği, 30 Eylül).
  12 Ekim'den beri (ADR 0228) yalnız çekirdek, masaüstü ve gerektiğinde bulut. Resimler masaüstünün, izler masaüstünde oynatılır.
  Yalnız masaüstünde biten her madde TODOS.md §0.2'ye web'de gerekenleri yazar.
