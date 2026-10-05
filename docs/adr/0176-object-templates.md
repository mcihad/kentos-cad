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
- **Yazı şablonu:** yazının kâğıttaki yüksekliği (mm, Yazı'nın Yükseklik'i gibi: yazı her ölçekte baskıda aynı boyda çıkar), hizası,
  zemini. **Blok şablonu:** yerleştirilecek bloğun adı.
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
- İstemin başında şablonun adı görünür (“Parsel sınırı · Kapalı alan: …”); komut geçmişi de şablonu böyle yazar.
- Şablondan şablona geçilirken döndürülecek renk ve kalınlık ilk şablondan önceki değerlerdir. Şeridin Renk'i paletin dışındaki
  bir şablon rengini kendi yazısıyla gösterir (“#7A5C3E”).
- Komutu `template.draw`'dır (“Şablonla çiz”): şablonun kimliğiyle çizer; kimliksiz (komut satırı, Komut ara) Stil yöneticisini
  Şablon türüyle açar. Kitaplıkta bulunmayan ya da kuralına uymayan şablon söylenir, başlamaz.

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
- **Ayrıntılar (5 Ekim, bu ADR'nin varsayılanları):** grup şablonu sıradan bir şablondur, `members` listesi olan: kendi aracıyla
  çizilen ana nesne kendi katmanına, görünüşüyle, öznitelikleri ve etiketiyle yazılır (tek şablon gibi); üyelerin nesneleri onunla
  aynı geri alma adımındadır (adımın adı aracınkidir). Üye, kitaplıktaki bir şablonun kimliği ve kuralıdır; üyenin şablonu nesnenin
  katmanını, görünüşünü, özniteliklerini ve etiketini verir, aracı ve yöntemi kullanılmaz:
  - **Aynı geometri** (`same`): çizilen geometri üyenin katmanına.
  - **Ötelenmiş** (`offset`): çizgide, çoklu çizgide ve kapalı alanda; uzaklık (m, sıfırdan büyük) ve yan: sol, sağ ya da iki yan
    (çizim yönüne göre); Ötele'nin kuralıyla.
  - **Köşelere nokta** (`vertices`): üye nokta şablonudur; Köşelere nokta'nın kuralıyla (ADR 0152): ortak köşe bir kez, noktası olan
    köşe atlanır; adlar üyenin ad dizisinden (şablon başına sürer, §3b), kod üyenin.
  - **Ağırlık merkezine** (`centroid`): alanın ağırlık merkezine, çizginin ortasına (etiketin yeri, ADR 0175); üye nokta şablonuysa
    nokta (adı dizisinden), yazı şablonuysa yazı: metni üyenin etiketidir, her çizimde Artır'ın kuralıyla bir artar (101, 102 …;
    ADR 0145), yüksekliği, hizası ve zemini üyenin.
- Grup şablonunun aracı Çizgi, Çoklu çizgi, Kapalı alan, Dikdörtgen ya da Döndürülmüş dikdörtgendir. Üyenin şablonu kitaplıkta yoksa,
  kendisi grup şablonuysa ya da kurala uymuyorsa (köşelere nokta üyesi nokta şablonu değil; ağırlık merkezi üyesi nokta ya da yazı
  şablonu değil ya da etiketsiz yazı şablonu) grup başlamaz, söylenir. Üyelerin katmanları grup seçilince bulunur ya da açılır (ana
  katmanla aynı “Katman ekle” adımında); kilitli üye katmanı grubu başlatmaz.
- Bir üyenin nesnesi yapılamazsa (öteleme şekil vermezse) o üye atlanır ve söylenir; ana nesne ve öbür üyeler yazılır.

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
2. Komutlar: oluşturma komutlarının `symbol`'ü (ve eksikse `label`'ı); ortak durumlar. **Tamam (5 Ekim):** `cad.polygon.create`,
   `cad.line.create`, `cad.polyline.create`, `cad.circle.create` ve `cad.arc.create` `label` ve `symbol`, `cad.point.create` `symbol`,
   `cad.entities.create`'in nesneleri `symbol` alır; değerler olduğu gibi yazılır, sembolün kimliği kitaplıkta aranmaz (kitaplıklar ev
   sahibinindir; kitaplıkta bulunmayan sembolün nesnesi çizimde düz görünüşle, rengi ve kalınlığıyla çizilir, kaybolmaz). Her komutun dosyasında bir ortak durum (`create_command_cases.py`
   üretir); TS sözleşmeleri, katalog ve Python SDK'sı yeniden üretildi.
3. Şablonla çizmek: şablonun katmanı, rengi ve kalınlığı, araçların yazdığı sembol, öznitelik ve etiket; ortak iz. İki parçada:
   - 3a **Tamam (5 Ekim):** katmanın kuralı iki platformda (`kentos_interaction::templates::find_layer`, `templateLayer`; bağımsız
     başvurusu `scripts/fixtures/template_layer_cases.py`, 22 ortak durum `fixtures/style/v1/template-layers.json`), açılışı tek adım
     “Katman ekle” (`open_layer`); şablonun koşusu (masaüstü `templates.rs`, `Context::template`, `Session::runs`; web
     `app/objectTemplates.ts`, `tools/templateStamp.ts`, `DraftingSettings.template`, `ToolManager`'ın `activate(id, template)`'i);
     masaüstünün taslak rengi her rengi taşır (`DraftColor`); Kapalı alan, Çoklu çizgi, Çizgi, Dikdörtgen, Döndürülmüş dikdörtgen,
     Daire ve `cad.entities.create` ile yazan araçlar damgayı yazar; Stil yöneticisinde Şablonla çiz; ortak iz `template-draw.json`
     (`templates.kcad`; iz biçimine `template` eylemi, `activeLayer`, `currentColor`, `currentWeight` ve nesnenin `symbol`, `color`,
     `lineWeight`, `layer` beklentileri). Web'de kitaplığın proje bölümü artık açılan çizimi izler (masaüstününki gibi).
   - 3b: nokta, yazı ve blok şablonları: Nokta'nın Ad ve Kod'u şablondan, ad dizisi şablon başına sürer, bırakılınca aracın kendi
     değerleri döner; Yazı'nın yüksekliği (kâğıtta mm), hizası ve zemini, bırakılınca aracınkiler döner; Blok ekle'nin bloğu adıyla
     (çizimde yoksa şablon başlamaz, söylenir); nesneden şablonda yazının yerdeki yüksekliği çizim ölçeğiyle mm'ye çevrilir; ortak iz.
     **Tamam (5 Ekim):** şablonun koşusu aracın kendi seçeneklerini verir ve sonunda geri alır (masaüstü `templates.rs`'in `seed_tool`
     ve `give_tool_back`'i, `Memory`'nin alanlarıyla; web `tools/templateSeeds.ts`, `TemplateRun.seed`, `TextTool.useOptions`): nokta
     şablonu Kod'u her zaman (yoksa kodsuz), Ad'ı ilk adı varsa verir, adsız şablon Nokta'nın kendi dizisine dokunmaz; şablonun sonraki
     adı çizim boyunca kimliğiyle saklanır, başka çizim açılınca şablonlar ilk adlarından başlar (`template_names`,
     `ToolManager.templateNames`; web'in iz oynatıcısı aynı tarayıcıda izden ize geçerken bunu yakaladı); yazı şablonu yazı bölümü varsa yüksekliği, hizayı
     ve zemini verir; blok şablonu bloğunu adıyla bulur, bulamazsa katmana dokunmadan söyler (“Çizimde “Vana” bloğu yok: …”). Nokta aracı
     da artık damgayı yazar (sembol, öznitelikler; adı yoksa şablonun etiketi), iki platformda. Nesneden şablonun yazı yüksekliği
     `height × 1000 / plotScale` (`from_object`, `templateFromObject`; başvuru tam kesirle, iki yeni durum 1:500 ve 1:2000);
     düzenleyicilerde “Yükseklik (mm)”. Ortak iz `template-tools.json` (`template-tools.kcad`, 1:500), `shot` adımlarıyla iki platformda
     resimlenir.
4. Şablonlar paneli ve şeridin listesi; Şablon düzenleyici, Seçili nesneden şablon; resimler. 3b'den önce yapılır (sahibin 5 Ekim
   sorusu üzerine: şablon panel ve düzenleyici olmadan kullanılamıyor). Üç parçada: 4a Şablonlar paneli (sağ dokta Katmanlar, İşlemler ve
   Bloklar'ın yanında; kategorilere göre şablonlar resim ve adlarıyla, arama, son kullanılanlar; tıklama şablonla çizer; satırın menüsü),
   4b Şablon düzenleyici ve Seçili nesneden şablon (nesneden şablonun kuralı ortak durumlarla), 4c şeridin Giriş sekmesinde Şablonlar
   listesi.
   - 4a **Tamam (5 Ekim):** listenin kuralı iki platformda (`object_template::listed`, `style/templateList.ts`; elle yazılmış ortak
     durumlar `fixtures/style/v1/template-list.json`: gruplar kategori yollarının Türkçe sırasıyla, kategorisizler sonda; aramada ad,
     açıklama, kategori, araç ve katman); panel masaüstünde `templates_panel.rs`, web'de `ui/templates/TemplatesPanel.ts`: resim, ad ve
     “araç · katman”, son kullanılan beş şablon önce (oturumun; web `ToolManager.recentTemplates`), çizilen şablon işaretli, tıklama
     çizer, ↓ ve Enter, grup açılıp kapanır, satırın menüsü (Şablonla çiz, Stil yöneticisinde göster, Kitaplığıma ve Projeye kopyala,
     Sil sorarak); `template.panel` (“Şablonlar”), kimliksiz `template.draw` paneli açar; dok sekmesi `templates` (`fixtures/shell/v1/
     layout.json`); ikonlar `templates`, `templateDraw`; ortak iz `template-draw.json` paneli de resimler.
   - 4b-1 **Tamam (5 Ekim):** düzenleyicinin iki kuralı iki platformda: Seçili nesneden şablon (`object_template::from_object`,
     `templateFromObject`; araç nesnenin türünden, katman yolu ve görünüşüyle, nesnenin rengi, kalınlığı, sembolü, öznitelikleri ve
     etiketi; noktanın etiketi ilk adı, Kod'u kodu; yazının yüksekliği, hizası, zemini; bloğun adı; başka tür reddedilir; bağımsız
     başvuru `template_from_object_cases.py`, 14 ortak durum) ve formun kuralı (`template_form`, `model/templateForm.ts`: alanlardan
     öğe ve şablon, sorunlar alanların sırasıyla, ve geri; başvuru `template_form_cases.py`, 24 ortak durum).
   - 4b-2 **Tamam (5 Ekim):** Şablon düzenleyici iki platformda (masaüstü `template_editor.rs`, web `ui/templates/TemplateEditor.ts`): ad,
     kategori, açıklama ve şablonun resmi; araç ve yöntem; katman (gruplar, ad, Çizimden seçme, açılacağı görünüş); renk, kalınlık,
     sembol (Stil yöneticisinin seçme kipi, masaüstünde `PickTarget::Template`, pencere geri gelir); öznitelik tablosu, etiket; noktanın,
     yazının ve bloğun alanları; yeni şablonun yeri (Kitaplığım, Proje). Sorunlar canlı, Kaydet sorun varken kapalı; gövde kayar, düğmeler
     görünür. Girişler: `template.new` (Yeni şablon…), `template.fromSelection` (Seçili nesneden şablon…), panelin araç çubuğu ve satır
     menüsünde Düzenle (sistem şablonunda Kopyasını düzenle, kopya Kitaplığım'a), Stil yöneticisinde Düzenle; ikonlar `templateNew`,
     `templateFromSelection`.
   - 4c **Tamam (5 Ekim):** şeridin üç Giriş sekmesinde (türsüz, CAD, CBS) Katmanlar'dan sonra Şablonlar paneli (web
     `ui/ribbon/fields.ts`'in `templateField`'ı, `panels.ts`'in `templates` paneli; masaüstü `ribbon_panels.rs`'in `templates_group`'u):
     çizilen şablonun adı ya da “Şablonla çiz”; açılır listede son kullanılanlar, sonra kategoriler, her şablon aracının ikonu ve adıyla,
     çizilen işaretli, seçim şablonla çizer; altında Yeni şablon…, Nesneden şablon ve Şablonlar; köşedeki düğme paneli açar. Pencere
     daraldıkça alan kısalır, düğmeler ikona iner, sonra panel tek düğmeye katlanır (menüsünde şablonlar ve üç komut). Şablonlar paneli
     1440 px'te Özellikler'i bir kademe küçülttüğü için o kademede alanların adlarının yerini ikonları alır (Renk, Tip, Kalınlık ve yeni
     `plotScale` ikonuyla Ölçek; web'de `dropdown--glyph`, masaüstünde KentOS UI `Choice::label_icon`): web'de “Katmana göre” o kademede
     hiçbir alana sığmıyordu. Resimler: masaüstü `templates_panel::tests::ribbon_screens` (`.run/shots/sablon-serit-*`), web
     `shots.mjs templates`.
5. Grup şablonu; ortak iz. Dört parçada: 5a model (şablonun `members`'ı, kuralları ve sorunları iki platformda,
   `object-templates.json`'a durumlar; düzenleyicinin formunda üyeler, `template-form.json`); 5b çekirdek (üyelerin geometrisi:
   `ops::template_members`, aynı, ötelenmiş, ağırlık merkezi; bağımsız başvuru ve ortak durumlar); 5c grup şablonuyla çizmek (üyelerin
   katmanları, ana nesneyle tek adım, köşelere nokta ve yazının artışı; ortak iz); 5d Şablon düzenleyicide üyeler tablosu, resimler.
   - 5a **Tamam (5 Ekim):** şablonun `members`'ı iki platformda (web `TemplateMember`, `GROUP_TOOLS`, `CLOSED_TOOLS`, `MEMBER_RULES`,
     `MEMBER_SIDES`; masaüstü `object_template`'in aynı adlı sabitleri): kendi kuralları `templateIssues` ve `template_issues`'ta (yalnız
     grup araçlarında, liste, her üyenin şablonu, kuralı, ötelemede uzaklığı ve şekle uyan yanı, başka kuralda uzaklık ve yan yok;
     `object-templates.json`'a 14 durum); kitaplıktaki çözümü `memberIssues` ve `member_issues` (kitaplıkta olmayan, bozuk ya da grup
     olan üye şablonu, köşelere nokta üyesi nokta şablonu, ağırlık merkezi üyesi nokta ya da etiketli yazı şablonu; yeni ortak dosya
     `template-groups.json`, 11 durum, elle); düzenleyicinin formunda üye satırları (`MemberRow`; boş satır düşer, şablon ve kural
     gerekir, ötelemede uzaklık ve yan; `template_form_cases.py`'ye 6 + 1 durum). Düzenleyiciler formu bütün tuttuğu için var olan
     grup şablonunun üyeleri düzenlemede korunur; üyelerin tablosu 5d'de.
   - 5b **Tamam (5 Ekim):** üyelerin geometrisi çekirdekte (`ops::template_members`): `member_offsets` öteleme üyesinin paralelleri
     (Ötele'nin kuralı `geom::offset`'in işaretli uzaklığıyla: açık şekilde sol artı, kapalı şekilde iç, halkanın yönü işaretli
     alanından, yaylı halkada yaylarıyla; iki yanda önce sol ya da iç; retler: bilinmeyen yan, sıfırdan büyük olmayan uzaklık, çok
     parçalı alan, çizgi, çoklu çizgi ve alan dışındaki şekil, şekle uymayan yan); ağırlık merkezi etiketin yeri (`entity_anchor`).
     İşlemler adlarıyla `templateMemberOffsets` ve `templateMemberCentroid`, web'de `model/ops/templateMembers.ts`. Bağımsız başvuru
     `scripts/fixtures/template_member_cases.py` (kesin kesirlerle, Ötele'nin dört uzaklıklık köşe sınırı ve yarım çember dahil; 21
     öteleme ve 7 ağırlık merkezi durumu, `fixtures/template-members/v1/cases.json`); iki platform 1e-9 m içinde geçer.
6. Şablonu uygula; ortak iz.

Her adım iki platformda, ortak fixture'larla, kendi commit'inde ilerler.

## Doğrulama

- Şablonun kuralları ortak durumlarla; `.kstil` sürüm 2'nin okuma ve yazması iki platformda aynı dosyayla.
- Şablonla çizim ve grup şablonu ortak izlerle; komutların yeni alanları ortak komut durumlarıyla.
