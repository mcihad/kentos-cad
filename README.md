# kentos-rc

**KentOS Rust Components**: [iced](https://github.com/iced-rs/iced) 0.14 üzerine
kurulu, CBS ve CAD uygulamaları için bileşen kütüphanesi.

```sh
cargo run                       # vitrin uygulaması: KentOS CAD
cargo test --workspace          # kütüphane ve vitrin testleri
cargo doc -p kentos-rc --open   # API belgeleri
cargo run -- snapshot ekran.png # pencere açmadan ekran görüntüsü
```

## Yapı

```
src/                     kentos-rc kütüphanesi
├── attribute/           öznitelik veri modeli (arayüzden bağımsız)
│   ├── value.rs         Value (metin, sayı, evet/hayır, tarih, saat, başvuru), ObjectId
│   ├── field.rs         Field ve FieldKind: tür, birim, zorunluluk, kodlu değer, aralık
│   ├── query.rs         Query: "öznitelikle seç" ve tablo filtresi koşulları
│   ├── time.rs          Date, Time, DateTime (harici bağımlılık olmadan)
│   └── text.rs, number.rs  Türkçe arama, sıralama ve sayı yazımı
├── theme/               renk belirteçleri (Tokens), vurgu rengi (Accent), yazı ayarı, hareket, iced teması
├── style/               iced stil fonksiyonları: button, container, text, field
├── icon/                16×16 ızgarada çizilmiş vektör ikon seti
├── label.rs             tip ölçeğine bağlı hazır metin biçimleri
├── snapshot.rs          ekransız görüntü (`snapshot` özelliği)
├── widget/              uygulama çerçevesi
│   ├── ribbon/          şerit: sekmeler, gruplar, düğmeler, alanlar
│   ├── app_menu.rs      uygulama menüsü (Office "Dosya" menüsü gibi)
│   ├── dock.rs          yan panel yuvası: açılıp kapanan paneller, sürüklenen kenar
│   ├── floating.rs      kayan araç pencereleri: sürükle, yakala, daralt, boyutlandır
│   ├── tabs.rs          belge sekmeleri: kapatma, sürükleyerek sıralama, taşma listesi
│   ├── docking.rs       sekmeli yuva: alanlar, yığınlar, sürükle-bırak, yüzen paneller
│   ├── viewports.rs     görünüm alanları: 1–4 görünüm, etkin görünüm, büyütme, bölme
│   ├── number.rs        sayı girişi: birim, ifade, sürükleme; vektör, açı ve kadran
│   ├── color.rs         renk seçici (HSV, onaltılık, saydamlık) ve renk rampası
│   ├── switch.rs, radio.rs  anahtar ve radyo grubu
│   ├── range.rs         çift uçlu aralık kaydırıcısı, isteğe bağlı histogramla
│   ├── chips.rs         etiket girişi: Enter/virgülle ekleme, öneri tamamlama
│   ├── form.rs          form düzeni: etiket sütunu, bölümler, yardım ve hata
│   ├── toast.rs         bildirimler: önem düzeyi, eylem, üst üste dizilme, süre
│   ├── progress.rs      ilerleme çubuğu, dönen gösterge, iptal edilebilen görev listesi
│   ├── notice.rs        uyarı şeridi, boş ve hata durumları
│   ├── wizard.rs        adımlı sihirbaz: adım göstergesi, seçenek satırları
│   ├── properties.rs    solunda bölüm listesi olan özellikler penceresi
│   ├── severity.rs      geri bildirimin önem düzeyleri (bilgi, başarı, uyarı, hata)
│   ├── sash.rs          boyutlandırma tutamağı (sürükle, çift tıkla sıfırla)
│   ├── table.rs         veri tablosu: sıralama, çoklu seçim, yatay kaydırma, sanal satırlar
│   ├── tree_view.rs     ağaç tablo: üç durumlu onay kutusu, sürükleyerek taşıma, adlandırma
│   ├── virtual_list.rs  sanal liste: yalnızca görünen satırları kurar
│   ├── context_menu.rs  sağ tık menüsü ve menü düğmesi: alt menü, kısayol, klavye
│   ├── inspector.rs     nesne inceleyici: arama, kategoriler, geri alma, yardım
│   ├── mini_toolbar.rs  seçimin üstünde beliren, uzaklaştıkça soluklaşan araç çubuğu
│   ├── radial.rs        dairesel menü: yöne göre seçim, basılı tutup bırakma
│   ├── date_picker.rs   tarih, tarih-saat ve saat seçicileri (açılır takvim)
│   ├── select.rs        aranabilir, açılır seçim kutusu
│   ├── query_builder.rs sorgu oluşturucu
│   ├── toolbar.rs       araç çubuğu: arama, eylemler, anahtarlar
│   ├── segmented.rs     parçalı seçim
│   ├── property_grid.rs salt okunur özellik ızgarası
│   ├── command_line.rs  komut kutusu: geçmiş, istem, otomatik tamamlama
│   ├── status_bar.rs    durum çubuğu: menülü göstergeler ve anahtarlar
│   ├── dialog.rs        iletişim kutusu, onay kutusu, kısayol listesi
│   ├── navigation_bar.rs, overlay.rs, tip.rs
└── spatial/             CBS ve CAD (`spatial` özelliği, varsayılan açık)
    ├── projection.rs    LonLat, Bounds, Viewport (Web Mercator)
    ├── feature.rs       Geometry, Feature, Layer, Sublayer, FeatureRef
    ├── measure.rs       jeodezik mesafe, Measurement
    ├── query.rs         öğe seçimi (tıklama, pencere, kesişen) ve nesne yakalama
    ├── selection.rs     çoklu seçim ve seçim yöntemleri (yeni, ekle, çıkar, kesişim)
    ├── tool.rs          araçlar
    ├── draft.rs         çizim araçlarının durum makinesi
    ├── format.rs        Türkçe koordinat, mesafe ve sayı yazımı
    ├── model_space/     etkileşimli harita/çizim alanı
    └── view_cube.rs     wgpu ile çizilen yön küpü

assets/fonts/            gömülü yazı tipleri ve lisansları (SIL Open Font License)

examples/showcase/       KentOS CAD: kütüphanenin vitrin uygulaması
├── app.rs               durum ve güncelleme mantığı
├── message.rs           mesajlar, sekmeler, menü komutları
├── command.rs           komut kataloğu ve yazılanın çözümlenmesi
├── gallery.rs           galeri sayfaları ve örneklerin durumu
├── table.rs             öznitelik tablosunun görünüm modeli: filtre, arama, sıralama
├── layer_tree.rs        katman ağacı: iç içe gruplar ve görünürlük
├── sheets.rs            model ve düzen (pafta) sekmeleri
├── sample.rs            örnek veri ve öznitelik şemaları
├── settings.rs          kalıcı ayarlar: tema, yazı ve yuva (~/.config/kentos-cad/ayarlar)
├── snapshot.rs          `snapshot` alt komutu ve senaryolar
└── view/                bileşenlerin yerleşimi
    └── gallery/         bileşen kataloğu (Galeri sekmesi)
```

Vitrindeki **Galeri** sekmesi kütüphanenin kataloğudur: renkler, yazı, ikonlar,
düğmeler, veri bileşenleri, çerçeve, yerleşim, girdiler, geri bildirim,
öznitelikler ve CBS/CAD bileşenleri; her biri canlı örneği, modül yolu ve
kullanım koduyla.

## Öznitelikler ve seçim

Vitrindeki **Giriş** sekmesi ArcGIS ve AutoCAD'deki iş akışını izler:

- **Seçim.** Seç aracında tıklama öğeyi seçer; soldan sağa sürüklemek pencere
  seçimi (tamamı içeride kalanlar), sağdan sola sürüklemek kesişen seçimdir.
  Shift seçime ekler, Ctrl seçimden çıkarır.
- **Öznitelik tablosu.** Model alanının altında aktif katmanın kayıtları:
  arama, filtre, "yalnızca seçili", sütuna göre sıralama. Satıra tıklamak seçer;
  Shift aralık seçer, Ctrl satırı ekler ya da çıkarır.
- **Öznitelikle seç ve filtre.** Sorgu oluşturucuyla koşullar kurulur; seçimde
  yeni seçim, ekle, çıkar ve kesişim yöntemleri vardır. Eşleşen kayıt sayısı
  yazarken görünür.
- **Katman ağacı.** Katmanlar iç içe gruplarda durur; derinlik sınırsızdır.
  Grubun ve katmanın kendi görünürlüğü vardır; katman, kendisi ve bütün üst
  grupları açıksa çizilir. Katmanlar bir alanın değerine göre alt
  katmanlara ayrılır (şehirler bölgeye, yollar türe göre); her alt katmanın
  rengi ve görünürlüğü ayrıdır. Gruplar ve katmanlar sürüklenerek taşınır,
  F2 ile yerinde adlandırılır; kilitli katmana çizilmez, seçilemeyen katmanın
  öğeleri haritada seçilmez (bkz. [Veri](#veri)).
- **Yuva.** Katmanlar ve Özellikler sağda, Öznitelik tablosu ve Görevler
  altta sekmeli yığınlardadır; sekmeler sürüklenerek başka yığına, kenara ya
  da ortaya (yüzen pencere) taşınır. Yerleşim ayar dosyasında saklanır;
  kapatılan panel Görünüm sekmesindeki Paneller grubundan geri açılır.
- **Bağlam menüleri.** Ağaçtaki gruplara, katmanlara ve alt katmanlara,
  model alanına ve tablo satırlarına sağ tıklanınca ilgili komutlar açılır:
  yakınlaştır, seç, yalnızca bunu göster, opaklık, koordinatı kopyala...
- **Nesne inceleyici.** Seçimin birincil öğesi türlerine göre düzenlenir:
  metin ve uzun metin, tam sayı ve ondalık (birim, artırma okları), evet/hayır
  (parçalı seçim), kodlu değer (aranabilir liste), aralık (kaydırıcı ve sayı),
  tarih (takvim), saat (saat ızgarası), tarih ve saat (ikisi yan yana), nesne
  başvurusu. Başvuru alanı aranabilir listeden (nesne seçici) ya da haritada
  tıklanarak (varlık seçici) doldurulur; başvurulan nesneye gidilebilir.
  Üstte alan araması, kategorili/alfabetik görünüm ve boş alanları gizleme;
  kategoriler daraltılır. Değişen alanlar işaretlenir ve ilk değerine
  döndürülür. Alttaki yardım bölümü alanın türünü, kısıtlarını ve açıklamasını
  gösterir.

## Veri

### Sanal tablo, ağaç ve liste

On binlerce satırlı tablolar ve ağaçlar yalnızca görünen satırlarını kurar,
çizer ve olaylara bağlar; satırlar eşit yükseklikte olduğundan kaydırma hep
akıcıdır.

- **Sanal tablo.** `Table::virtualized(sayı, |i| satır)` satırı sıra
  numarasından ister; başlık, sütun hizası ve satır stilleri sıradan
  tabloyla aynıdır. `reveal(Some(i))` satırı bir kez görünür yapar (ör.
  haritada seçilen öğenin satırı), kullanıcının kaydırmasını geri almaz.
  Vitrindeki öznitelik tablosu ve galerinin 100.000 kayıtlık örneği böyle
  kurulur.
- **Sanal ağaç.** `TreeView::virtualized(sayı, |i| (derinlik, düğüm))` açık
  düğümlerin düzleştirilmiş sırasından yalnızca görünenleri kurar.
- **Sanal liste.** `VirtualList` aynı işi herhangi bir satırla yapar:
  tekerlek, sürüklenen ince kaydırma çubuğu ve `reveal`.

```rust
Table::new(columns)
    .virtualized(self.records.len(), |index| {
        table::Row::new(self.cells(index))
            .selected(self.selection.contains(&index))
            .on_press(Message::RowPressed(index))
    })
    .reveal(self.primary)
```

### Ağaçta taşıma, adlandırma ve satır düğmeleri

- **Sürükleyerek taşıma.** Kimliği (`Node::id`) olan düğümler sürüklenir:
  satırın üst yarısı önüne, alt yarısı ardına, klasörün (`Node::folder`)
  ortası içine bırakır; yer çizgiyle ya da çerçeveyle gösterilir. Düğüm
  altındakilerle birlikte taşınır, kendi altına bırakılamaz; Esc vazgeçer.
  `on_move(kaynak, hedef, yer)` bildirir, uygulama taşımayı reddedebilir.
- **Yerinde adlandırma.** `Node::editor` adın yerine `tree_view::rename`
  kutusunu koyar; uygulama kutuyu `tree_view::RENAME` kimliğiyle odaklar.
  Enter ve kutunun dışına tıklamak kaydeder, Esc vazgeçer; düzenlenen satır
  sürüklenmez, kutuda fareyle metin seçilir.
- **Satır düğmeleri.** `Node::toggle` adın sağına göz, kilit ya da
  seçilebilirlik düğmesi ekler (`Toggle::visible`, `locked`, `selectable`);
  kapalı düğmenin ikonu sönüktür.
- **Vitrinde.** Katman ağacında gruplar ve katmanlar sürüklenerek taşınır;
  F2 (son tıklanan yer ağaçsa) ya da sağ tık menüsündeki "Yeniden adlandır"
  seçili düğümü adlandırır. Kilitli katmana çizilmez, öğeleri silinmez ve
  özellikleri değişmez; seçilemeyen katmanın öğeleri haritada seçilmez.

```rust
TreeView::new([tree_view::Column::new("Katman").width(Fill)])
    .on_move(Message::TreeMoved)
    .push(
        Node::new("Yerleşim")
            .id(group_key)
            .folder()
            .expanded(open, Message::GroupToggled(0))
            .push(
                Node::new("Şehirler")
                    .id(layer_key)
                    .toggle(Toggle::locked(locked, Message::LayerLocked(0)))
                    .toggle(Toggle::selectable(selectable, Message::LayerSelectable(0))),
            ),
    )

// F2: ad yerinde düzenlenir.
node.editor(tree_view::rename(
    &name,
    Message::RenameInput,
    Message::RenameSubmitted,
    Message::RenameCancelled,
))
```

## Kayan araç pencereleri

Ölçüm, koordinata git ve katman stili harita üstünde kayan küçük
pencerelerdir; arkadaki işi kilitlemezler. Ölçüm sürerken haritaya tıklanır,
stil değişiklikleri haritaya anında yansır.

- **Sürükle ve yakala.** Pencere başlığından sürüklenir. Alanın kenarlarına
  ve öbür pencerelere yaklaşınca yakalanır: kenarlardan 8 piksel içeride durur,
  komşusuyla hizalanır ya da 8 piksel arayla yanına oturur. Yakalanan kenarı,
  model alanındaki nesne yakalamasının sarısıyla noktalı bir kılavuz gösterir.
  Ctrl basılıyken yakalama olmaz.
- **Öne gelme.** Tıklanan pencere öne gelir; öndeki pencerenin başlığının
  üstünde, şeritteki seçili sekmede olduğu gibi vurgu çizgisi bulunur.
- **Daraltma.** Başlığa çift tıklamak ya da ˄ düğmesi pencereyi başlığına
  daraltır; başlık yerinde kalır. Başlıktaki bilgi (ölçümde toplam uzunluk)
  daraltılmışken de okunur.
- **Boyutlandırma.** İzin verilen pencereler kenarlarından ve köşelerinden
  boyutlandırılır; sağ alt köşedeki noktalar bunu belli eder. Yalnızca yatay
  kenar sürüklenirse yükseklik içeriğe göre kalır.
- **Kenara tutunma.** Konum pencerenin yakın olduğu kenarlara göre saklanır:
  sağ alta bırakılan pencere, yuvanın alanları genişleyince ya da tablo
  açılınca sağ alt köşeyle birlikte kayar.
- **Vitrinde.** Ölç aracı ölçüm penceresini açar; pencereyi kapatmak araçtan
  çıkar. Koordinata git (`GIT`) enlem ve boylamı doğrular, görünümü ortalar ya
  da süren çizime nokta ekler. Katman stili (`STIL`, katman menüsünde "Stil…")
  rengi, opaklığı ve çizgi kalınlığını değiştirir. Pencereler Görünüm
  sekmesindeki Pencereler grubundan açılıp kapanır.

```rust
use kentos_rc::widget::floating::{self, Floating, Placement, ToolWindow, Windows};

// Durum: açık pencereler, konumları ve sıraları.
self.windows.open(Pane::Measure, Placement::top_left(8.0, 8.0));

Floating::new(map, &self.windows, Message::Window, |pane| match pane {
    Pane::Measure => ToolWindow::new("Ölçüm", self.measure_body())
        .icon(Icon::Measure)
        .meta(total)          // daraltılmışken de görünür
        .resizable(),
    Pane::GoTo => ToolWindow::new("Koordinata git", self.go_to_body()),
})

// update: sürükleme, boyutlandırma, öne gelme, daraltma ve kapatma
Message::Window(event) => self.windows.update(event),
```

## Seçim çubuğu ve dairesel menü

- **Mini araç çubuğu.** `MiniToolbar` içeriği (ör. model alanı) sarar ve
  seçimin ekrandaki kutusunun üst ortasında küçük bir çubuk gösterir; üstte
  yer yoksa altına geçer, alanın kenarlarından taşmaz, seçim görünür alanın
  dışındaysa gizlenir. Office'teki gibi imleç uzaklaştıkça soluklaşır ve
  çizimi kapatmaz, yaklaşınca belirginleşir. Düğmenin adı ve kısayolu
  çubuğun seçimden uzak yanında yazar; açık ayarlar (`active`) vurgulu,
  yıkıcı işler (`danger`) kırmızıdır. Çubuğun altına tıklanmaz.
- **Dairesel menü.** `RadialMenu` içeriği sarar ve imlecin yerinde en çok
  sekiz komutu çevrede, tepeden saat yönünde dizer (Blender'daki pasta,
  Maya'daki işaretleme menüsü gibi): yönler hep aynıdır, kas hafızası
  oluşur. İmleci bir yöne kaydırmak o komutu seçer, tıklamak çalıştırır.
  Menüyü açan tuş (`hold`) ya da fare basılı tutulup bir yöne çekilerek
  bırakılırsa komut hemen çalışır; kısa basış menüyü açık bırakır. Ortaya
  tıklamak, sağ tık ya da Esc kapatır; kenara yakın açılan menü içeri
  kaydırılır.
- **Vitrinde.** Seç aracında seçimin üstünde çubuk belirir: seçime
  yakınlaştır, özellikler, öznitelik tablosu, katmanı kilitle, sil (yalnızca
  Çizimler katmanında) ve seçimi bırak. Boşluk tuşu model alanında araç
  menüsünü açar; komut yazılırken ve pencereler açıkken açılmaz.
- **Hareket.** Çubuğun belirmesi ve menünün açılması kısa geçişlerdir;
  `theme::motion::set_reduced(true)` geçişleri kapatır (hareketi azaltmak
  isteyen kullanıcılar ve ekransız görüntü için).

```rust
let map = MiniToolbar::new(model_space, self.selection_bounds())
    .button(Icon::Target, "Seçime yakınlaştır", Message::FocusSelection)
    .button(Icon::Lock, "Katmanı kilitle", Message::LayerLocked(layer))
    .active(locked)
    .separator()
    .button(Icon::Eraser, "Sil", can_delete.then_some(Message::Delete))
    .shortcut("Delete")
    .danger();

RadialMenu::new(map, self.radial_open, Message::RadialClosed)
    .hold(keyboard::Key::Named(key::Named::Space))
    .item(Icon::Select, "Seç", Message::ToolSelected(Tool::Select))
    .item(Icon::Line, "Çizgi", Message::ToolSelected(Tool::Line))
```

## Yerleşim

### Belge sekmeleri

`Tabs`, aynı alanı paylaşan içerikler içindir: açık çizimler, model ve düzen
görünümleri.

- **Etkin sekme içeriğe bağlanır.** İçerikle aynı zemindedir, kenarında vurgu
  çizgisi taşır; şeridin kenar çizgisi etkin sekmenin altında kesilir.
  `.content(..)` bağlanılan içeriğin rengini verir (ör. harita zemini).
- **Kapatma.** × düğmesi etkin sekmede ve üzerine gelinen sekmede görünür;
  orta tık da kapatır. Kaydedilmemiş sekmede (`.dirty(true)`) yerinde bir nokta
  durur. Kapatılamayan sekmeler `.closable(false)` ile verilir.
- **Sıralama.** Sekmeler sürüklenerek sıralanır; bırakılacağı yer vurgu
  renginde bir çizgiyle gösterilir. `on_reorder(from, to)` şu sırayı verir:
  `tabs.insert(to, tabs.remove(from))`.
- **Taşma.** Sekmeler sığmayınca önce eşit ölçüde daralır, etkin sekme okunur
  kalır. Okunur genişliğin altına inmeleri gerekirse sığmayanlar sağdaki
  listeye taşar; etkin sekme her zaman görünür, şerit yalnızca etkin sekme
  dışarıda kalınca kayar. Uzun başlıkların sonu solar.
- **Altta şerit.** `.bottom()` şeridi içeriğin altına asar (AutoCAD'in Model /
  Düzen sekmeleri gibi).
- **Vitrinde.** Haritanın altında Model ve Düzen sekmeleri: model alanı
  kapanmaz ve yeri değişmez; + yeni düzen açar. Düzen, masanın ortasında A3
  bir kâğıttır: harita çerçevesi kendi görünümüyle gezinilir, altında antet
  kutusu durur. Kâğıt her temada beyazdır; üzerindeki yazılar `themer` ile
  aydınlık temada çizilir. Galerinin Yerleşim sayfasında açık çizimlerle
  kapatma, sıralama ve taşma denenir.

```rust
use kentos_rc::widget::{Tab, Tabs};

Tabs::new(
    self.drawings.iter().map(|d| Tab::new(&d.name).icon(Icon::Document).dirty(d.dirty)),
    self.current,
    Message::DrawingSelected,
)
.on_close(Message::DrawingClosed)
.on_reorder(Message::DrawingMoved)
.on_new(Message::DrawingAdded)
```

### Sekmeli yuva

`DockSpace`, panelleri ortadaki içeriğin (harita, belgeler) kenarlarına
yerleştirir. Yan alanlar tam yüksekliktedir, alt alan ortanın altındadır.
Her alan yığınlardan oluşur; yığın aynı yeri sekmelerle paylaşan
panellerdir.

- **Sürükle ve bırak.** Sekme sürüklenince bırakılacağı yer vurgu rengiyle
  gösterilir, imlecin yanında sekmenin başlığı taşınır. Başka yığının sekme
  şeridine bırakılan panel o yığına, işaretlenen sıraya katılır; gövdenin
  ortasına bırakılan sona katılır. Gövdenin kenarına bırakılan yığını böler.
  Ortanın kenarlarına bırakılan o kenarın alanına yeni yığın olur (alan boşsa
  açılır); ortanın içine bırakılan yüzen pencere olur. Esc sürüklemeyi bırakır.
- **Boyutlandırma.** Alanların ortaya bakan kenarı ve yığınlar arasındaki
  çizgi sürüklenir; orta en az kendi payı kadar kalır.
- **Daraltma ve menü.** Yan alanlardaki ve yüzen yığınlar ⌃ düğmesiyle ya da
  başlığa çift tıkla başlıklarına daralır. ⋯ menüsü yığındaki panelleri
  listeler, paneli yüzdürür ya da yuvaya geri koyar, kapatır. Yer daralınca
  arkadaki sekmelerin yalnızca ikonu kalır.
- **Yüzen pencereler.** Başlığından taşınır, sağ ve alt kenarından
  boyutlandırılır; tıklanan öne gelir.
- **Tembel gövdeler.** `Pane::new(başlık, || gövde)`: gövde yalnızca panel
  görünürken kurulur. Arkaya geçen panelin durumu (kaydırma, açık düğümler)
  saklanır, sekmesi öne gelince geri gelir.
- **Durum uygulamanındır.** `Docks` yerleşimi tutar; bileşen değişiklikleri
  `docking::Event` olarak bildirir, `Docks::update` uygular. Kapanan panel
  yeniden açılınca aynı kenara döner. Yerleşim tek satırlık metne yazılıp
  okunur (`Docks::save`, `Docks::load`).

```rust
use kentos_rc::widget::docking::{DockSpace, Docks, Pane, Side};

let mut docks = Docks::new();
docks.dock(Panel::Layers, Side::Right);
docks.split(Panel::Properties, Side::Right);
docks.dock(Panel::Table, Side::Bottom);

DockSpace::new(map, &self.docks, Message::Dock, |panel| match panel {
    Panel::Layers => Pane::new("Katmanlar", || self.layers())
        .icon(Icon::Layers)
        .actions(self.layer_actions()),
    Panel::Properties => Pane::new("Özellikler", || self.inspector()).scrollable(),
    Panel::Table => Pane::new("Öznitelik tablosu", || self.table()),
})

// update
Message::Dock(event) => self.docks.update(event),

// saklama: "sol 260: ; sag 300: katmanlar* @1.00 / ozellikler* @1.00; alt 220: tablo* @1.00"
let text = self.docks.save(|panel| panel.key().to_owned());
let docks = Docks::load(&text, Panel::parse);
```

### Görünüm alanları

`Viewports` aynı modeli ya da haritayı birden çok görünümde gösterir.

- **Düzen.** Tek, iki (yan yana ya da alt alta), üç (solda büyük) ya da dört
  görünüm; `viewports::arrangements` düzen seçicisidir.
- **Etkin görünüm.** Görünüme tıklamak onu etkin yapar; etkin görünüm vurgu
  renginde çerçevelenir ve uygulama komutları ona uygular.
- **Başlık.** Sol üstteki menüler uygulamanındır (`View::menu`: bakış yönü,
  görsel stil); içeriğin üstünde okunur kalan küçük etiketlerdir. ⤢ ya da
  başlığa çift tık görünümü büyütür, yeniden basınca düzene dönülür.
- **Bölme.** Görünümler arasındaki çizgiler sürüklenir; oranlar `Views`'ta
  saklanır.
- **Vitrinde.** Galerinin Yerleşim sayfasında örnek bir evin üst, ön, sağ ve
  perspektif görünümleri; tel kafes, gizli çizgi ve gölgeli stiller.

```rust
use kentos_rc::widget::viewports::{self, View, Viewports, Views};

Viewports::new(&self.views, Message::Views, |index| {
    View::new(self.scene(index))
        .menu(camera.label(), move || camera_menu(index))
        .menu(style.label(), move || style_menu(index))
})

// update
Message::Views(event) => self.views.update(event),
```

## Girdiler

### Sayı girişi

`NumberInput` birimli bir değer alır; değer uygulamada temel birimde durur,
düzenlenen metni bileşen kendi tutar.

- **Düzenleme.** Alana tıklayınca değer seçili düzenlenir. Enter ya da alandan
  çıkmak onaylar, Esc vazgeçer; ↑ ↓ adım kadar (Shift ×10) değiştirir.
- **İfadeler.** Dört işlem ve parantez (`(3+4)/2`), Türkçe sayılar (1.234,5),
  başka birimde yazılan sayılar (`1 m + 20 cm`, `250mm`) ve bitişik birimler
  (`30°15'20"`, `1 m 20 cm`) kabul edilir. Birimsiz sayı alanın birimindedir.
  Hatalı ifadede kenar kırmızıdır; hatalıyken alandan çıkılırsa değer
  değişmez. `number::evaluate` aynı çözümleyiciyi dışarıya açar.
- **Sürükleme.** Öndeki etiket (X, Y, G…) basılıp yana sürüklenince değer
  piksel başına bir adım değişir; Shift ince, Ctrl kaba ayardır. Alana basıp
  sürüklemek de aynıdır; sürüklemeden bırakmak alanı düzenler.
- **Birimler.** `units::LENGTH` (m; cm, mm, km), `units::MILLIMETRE`,
  `units::ANGLE` (°; ', ", rad, grad), `units::PERCENT`; uygulama kendi
  birimlerini `Unit::new("ft", 0.3048)` ile verir.
- **Vektör ve açı.** `number::vector` iki ya da üç bileşeni eksen renkli
  etiketlerle yan yana dizer. `number::angle` kadran ve derece alanıdır;
  `Dial` CAD'deki gibi doğudan saat yönünün tersine ya da (`.bearing()`)
  pusuladaki gibi kuzeyden saat yönüne ölçer, Shift 15°'lik adımlara oturtur.

```rust
use kentos_rc::widget::NumberInput;
use kentos_rc::widget::number::{self, units};

NumberInput::new(self.width, Message::WidthChanged)
    .label("G")
    .units(units::LENGTH)
    .range(0.0..=10_000.0)
    .step(0.1)

number::vector(self.position, Message::Moved, units::LENGTH, 0.01, &theme)
number::angle(self.rotation, Message::Rotated)
```

### Renk seçici ve rampa

`ColorPicker` alanın altında açılan panelde rengi seçtirir: doygunluk ve
parlaklık düzlemi, ton şeridi, isteğe bağlı saydamlık şeridi, onaltılık giriş
(#RGB, #RRGGBB, #RRGGBBAA), hazır renkler ve uygulamanın verdiği son
kullanılanlar. Panel açıldığındaki renk yanda durur; tıklamak geri döndürür.

`color::ramp` renk rampası düzenleyicisidir: çubuğa tıklamak o noktanın
rengiyle durak ekler, durak sürüklenerek taşınır ve çubuğun altına uzağa
bırakılınca silinir. Seçili durağın rengi ve yeri alttaki satırda düzenlenir;
rampa ters çevrilir ya da hazır rampalardan (Viridis, Magma, Spektral, Arazi…)
biri seçilir. `Ramp::color_at(t)` herhangi bir noktadaki rengi verir.

```rust
use kentos_rc::widget::color::{self, ColorPicker, Ramp};

ColorPicker::new(layer.color, Message::ColorChanged).alpha()
color::ramp(&self.ramp, self.stop, Message::RampChanged)
```

### Temel kontroller ve form

- **Anahtar.** `Switch` hemen uygulanan açık/kapalı ayarlar içindir; düğme
  yeni konumuna kayar, etiket de tıklanır.
- **Radyo grubu.** `RadioGroup` birbirini dışlayan seçenekleri açıklamalarıyla
  dizer; seçilemeyen seçenek sönüktür, kısa seçenekler yan yana durur.
- **Aralık kaydırıcısı.** `RangeSlider` iki tutamakla alt ve üst sınırı
  seçer; aradaki parça sürüklenince aralık bütün olarak kayar. İsteğe bağlı
  histogram verinin dağılımını gösterir (`range::histogram` sayar), seçili
  aralıktaki çubuklar vurgu rengindedir.
- **Etiket girişi.** `ChipInput` yazılanı Enter ya da virgülle etikete
  çevirir, aynısını ikinci kez eklemez (Türkçe harf ayırmadan); boş alanda
  Backspace son etiketi siler, Tab öneriyi tamamlar.
- **Form.** `Form` etiketleri aynı genişlikte bir sütunda alanın ilk
  satırına hizalar; bölüm başlıkları, zorunlu alan yıldızı, yardım ve hata
  satırları vardır.

```rust
Form::new()
    .section("Pafta")
    .field("Ad", name_input).required()
    .help("Antet kutusunda ve sekmede görünür.")
    .error(self.name_error())
    .field("Kâğıt", RadioGroup::new(self.paper, Message::Paper)
        .option(Paper::A4, "A4", "").option(Paper::A3, "A3", "").horizontal())
    .row(Switch::new(self.title_block, Message::TitleBlock).label("Antet kutusunu göster"))

RangeSlider::new(0.0..=16e6, self.population, Message::PopulationRange)
    .histogram(&range::histogram(values, 0.0..=16e6, 32))
```

## Geri bildirim

- **Bildirimler.** Pencerenin sağ alt köşesinde, durum çubuğunun hemen
  üstünde üst üste dizilir; en yenisi köşeye en yakındır. Bütün pencereye
  bağlı olduğu için sekme değişince yer değiştirmez ve çizim alanına girmez. İkonun ve
  alttaki kalan süre çizgisinin rengi önem düzeyidir: bilgi, başarı, uyarı,
  hata. Bilgi ve başarı 5, uyarı 8 saniyede kapanır; eylemli bildirim ("Geri
  al") en az 8 saniye durur, hata kendiliğinden kapanmaz. İmleç üzerindeyken
  süre durur. En fazla üç bildirim görünür; eskiler "2 bildirim daha" olarak
  sayılır ve hepsi birden kapatılır. Aynı bildirim yinelenirse yenisi
  eklenmez, sayısı (×2) artar.
- **İlerleme.** İnce ilerleme çubuğu oranı bilinen işte dolar, bilinmeyende
  üzerinde bir parça kayar; rengi işin durumunu söyler. Dönen gösterge
  çember üzerinde sekiz noktadır. Görev listesi arka plandaki işleri
  durumlarıyla sıralar (sürüyor, sırada, bitti, başarısız, iptal edildi):
  süren ve sıradaki iş durdurulur, başarısız iş yeniden denenir, biten iş
  listeden kaldırılır.
- **Onay kutusu.** Başlık soru olarak yazılır, onay düğmesi işin adını taşır
  ("Tümünü sil"); yıkıcı işte kırmızıdır. Enter onaylar, Esc vazgeçer.
- **Uyarı şeridi.** Bir alanın üstünde süren bir durumu anlatır; bildirimden
  farkı kapatılana ya da durum değişene kadar yerinde kalmasıdır.
- **Boş ve hata durumları.** İçeriği olmayan alanın ortasında ne olduğu ve ne
  yapılabileceği; hata durumunda neyin yapılamadığı ve nasıl düzeltileceği.
- **Adımlı sihirbaz.** Biten adımlar onay işaretiyle, süren adım vurgu
  renginde; içerik sabit yüksekliktedir, kutu adımlar arasında zıplamaz. İleri
  adım tamamlanmadıysa devre dışıdır ve alttaki not nedenini söyler; son düğme
  işin adını taşır ("İçe aktar"). Arkasına tıklamak ilerlemeyi kaybettirmez
  (`overlay::blocking`).
- **Özellikler penceresi.** Solunda bölüm listesi, sağında bölümün içeriği
  (QGIS'in katman özellikleri gibi). Değişiklikler taslakta tutulur: Uygula
  yazar ve açık kalır, Tamam yazar ve kapatır, İptal atar; taslak farklıyken
  altta not görünür.
- **Vitrinde.** Çizim silmek geri alınabilir: bildirimdeki "Geri al" çizimleri
  numaralarıyla geri koyar. Örnek veri silinmeye çalışılınca haritanın
  üstünde salt okunur şeridi açılır; ayarlar yazılamayınca kalıcı hata
  bildirimi çıkar; kopyalamalar bilgi bildirir. "Çizimleri temizle" ve çizim
  varken çıkış onay ister. Bütün katmanlar gizliyken harita, satırı olmayan
  öznitelik tablosu (filtre, arama, seçili yok, boş katman) nedenini ve
  düzeltecek düğmeyi gösterir. Ekle sekmesindeki "Veri içe aktar" (`ICEAKTAR`)
  örnek bir CSV ya da GeoJSON dosyasını adım adım katman olarak ekler: CSV'de
  koordinat sütunları seçilir ve değerleri denetlenir, bozuk dosyada hata
  durumu görünür, metre bekleyen koordinat sistemi reddedilir; iş arka planda
  sürer ve bitince katman ağacın en üstüne eklenir. Katman menüsündeki
  "Özellikler…" adı, görünürlüğü, opaklığı, rengi ve etiketleri düzenler;
  kaynağı ve alanları gösterir.
  Dışa aktarma (uygulama menüsü ya da Yönet sekmesi) ve dizin oluşturma arka
  planda sürer: durum çubuğunda dönen gösterge ve yüzde görünür, tıklanınca
  Görevler paneli açılır; biten iş bildirilir. DXF'e dışa aktarma ilk
  denemede başarısız olur; hata bildirimi ve görev satırı "Yeniden dene"
  sunar.

```rust
use kentos_rc::widget::{Toast, Toaster, Toasts};

self.toasts.push(Toast::success("2 çizim silindi").action("Geri al", Message::UndoDelete));
self.toasts.push(Toast::error("Ayarlar kaydedilemedi").body(reason)); // kendiliğinden kapanmaz

// Bütün pencereyi sarar: bildirim her sekmede aynı köşede durur.
Toaster::new(window, &self.toasts, Message::ToastClosed)
    .padding(Padding { bottom: status_bar::height() + 8.0, ..Padding::new(8.0) })

// update
Message::ToastClosed(id) => self.toasts.dismiss(id),

// Görevler ve durum çubuğundaki gösterge
TaskList::new().push(
    Task::new("GeoJSON olarak dışa aktar")
        .detail("Türkiye.geojson: 27 / 60 öğe")
        .running(Some(0.45))
        .on_cancel(Message::Cancel(id)),
)
Readout::new(row![progress::spinner().size(12.0), label::caption("Dışa aktarılıyor")])
    .on_press(Message::ShowTasks)

// Sihirbaz ve özellikler penceresi; arkasına tıklamak kapatmaz
overlay::blocking(
    Wizard::new("Veri içe aktar", ["Kaynak", "Alanlar", "Koordinat sistemi", "Özet"])
        .current(step)
        .body(self.step_body())
        .hint(problem.unwrap_or(note))
        .back(Message::Back)
        .next(ready.then_some(Message::Next))
        .finish("İçe aktar", ready.then_some(Message::Finish))
        .on_cancel(Message::Cancel),
)
overlay::blocking(
    PropertiesDialog::new("Katman özellikleri")
        .section(Icon::Info, "Genel", selected, Message::Section(Section::General))
        .body("Genel", self.general())
        .dirty(draft != original)
        .on_apply(Message::Apply)
        .on_accept(Message::Accept)
        .on_cancel(Message::Cancel),
)
```

## Şerit

`Ribbon` sekme şeridi ve seçili sekmenin araç gruplarıdır; grup içeriği üç
satırlık ızgaraya oturur.

- **Menülü ve bölünmüş düğme.** `Button::menu(..)` düğmeye menü ekler.
  Eylemi de olan düğme bölünür: büyük düğmede üst kısım, küçükte sol kısım
  eylemi yapar; ok menüyü açar. Eylemsiz düğmenin tamamı menüdür.
- **Galeri.** `ribbon::Gallery` seçeneklerin önizlemelerini (renk, rampa,
  ikon, çizgi) dizer; satır seçili karoyu içerecek kadar kayar, ⌄ bütün
  karoları ızgarada açar.
- **Hızlı erişim.** `Ribbon::quick(..)` uygulama düğmesinin yanına küçük
  ikon düğmeleri koyar; `quick_menu` sonlarına ⌄ menüsü ekler (ör. hangi
  düğmelerin görüneceği).
- **Daraltma.** `Ribbon::collapsible(..)` sekme şeridinin sağ ucuna şeridi
  daraltan düğmeyi koyar; daraltılmış şeritte yalnızca sekmeler görünür.
- **Vitrinde.** Hızlı erişimde dışa aktarma, geri alma, tümünü görme ve
  kısayollar (⌄ menüsünden gizlenir); Ctrl+F1 şeridi daraltır. Yönet
  sekmesindeki "Dışa aktar" bölünmüş düğmedir; Açıklama sekmesinde aktif
  katmanın rengi ve çizgi kalınlığı galeriden seçilir.

```rust
Ribbon::new()
    .quick(Icon::Undo, "Geri al", self.can_undo().then_some(Message::Undo))
    .collapsible(self.ribbon_collapsed, Message::RibbonToggled)
    .group(Group::new("Dışa aktar").push(
        Button::large(Icon::Export, "Dışa aktar")
            .on_press(Message::Export(Format::GeoJson))
            .menu(|| formats_menu()),
    ))
    .group(Group::new("Renk").push(Gallery::new(tiles, selected, Message::ColorPicked)))
```

## Komut kutusu ve durum çubuğu

Pencerenin altı AutoCAD'deki gibi klavyeyle çalışır. Yazı tipinin bir anlamı
vardır: yazılabilen her şey (komutlar, koordinatlar, ölçek) eş aralıklı Plex
Mono ile, uygulamanın yanıtları ve talimatları Plex Sans ile yazılır.

- **Her yerden yazılır.** Bir metin kutusu odakta değilken yazılanlar komut
  kutusuna gider: `l` yazıp Enter'a basmak çizgi aracını seçer.
- **Öneriler.** Yazarken komutlar adlarına, AutoCAD kısaltmalarına (L, PL, ZE)
  ve Türkçe başlıklarına göre önerilir; eşleşen kısım vurgulanır, altta
  komutun ne yaptığı yazar. ↑ ↓ gezinir, Tab tamamlar, Enter çalıştırır. Giriş
  boşken ↑ önceki komutları getirir, ↓ bütün komutları listeler.
- **İstem.** Çizim ve ölçüm sürerken kutu beklenen adımı ve seçenekleri
  gösterir: `CCIZGI  Sonraki noktayı belirtin  [Geri al] [Bitir]`. Seçenekler
  tıklanarak ya da yazılarak ("g", "geri") seçilir. "enlem, boylam" yazmak nokta
  ekler; çizim yokken görünümü oraya ortalar. Boşken Enter çizimi bitirir, etkin
  komut yokken son komutu yineler; Esc etkin komuttan çıkar.
- **Geçmiş.** Yazılan komutlar `›` işaretiyle, hatalar kırmızıyla gösterilir;
  eski satırlar soluklaşır. F2 bütün geçmişi açar (son tıklanan yer katman
  ağacıysa seçili düğümü adlandırır).
- **Durum çubuğu.** Göstergeler tıklanınca yukarı doğru menü açar: koordinat
  biçimi (ondalık derece, DMS, Web Mercator metre) ve kopyalama, seçimle
  yapılacak işler, standart harita ölçekleri (1:1.000 halihazırdan 1:5.000.000'a).
  Anahtarlar açıkken ikonlarıyla vurgulanır. İmleç model alanından çıkınca son
  koordinat soluk kalır; değerler sabit genişliktedir, çubuk kıpırdamaz.

```rust
use kentos_rc::widget::command_line::{self, CommandLine, Prompt};
use kentos_rc::widget::status_bar::{Readout, Toggle};

const CATALOG: &[command_line::Command] = &[
    command_line::Command::new("CIZGI", "Çizgi")
        .aliases(&["LINE", "L"])
        .icon(Icon::Line)
        .description("Art arda doğru parçaları çizer."),
];

CommandLine::new(&self.history, &self.input)
    .id(COMMAND_INPUT)
    .commands(CATALOG.iter().copied())
    .prompt(
        Prompt::new("Sonraki noktayı belirtin")
            .command("CCIZGI")
            .option("Geri al", Message::Undo)
            .option("Bitir", Message::Finish)
            .key("Enter"),
    )
    .on_input(Message::CommandInput)
    .on_submit(Message::CommandSubmitted)
    .on_run(Message::CommandRun)
    .expanded(self.history_open, |_| Message::HistoryToggled)

// Şeritteki "Komut listesi" düğmesi: kutuya odaklanıp bütün komutları açar.
Message::CommandList => command_line::show_commands(COMMAND_INPUT),

StatusBar::new()
    .push(Readout::new(coordinates).icon(Icon::Target).width(214.0).menu(formats))
    .separator()
    .push(Toggle::new("Izgara", grid).icon(Icon::Grid).shortcut("F7").on_press(Message::Grid))
    .spacer()
    .push(Readout::new(label::mono(scale)).menu(standard_scales))
```

## Renkler

- **Temalar.** Dört tema vardır:
  - Koyu: CAD programlarının grafit arayüzü (varsayılan).
  - Aydınlık: kâğıt zeminli.
  - Gece: çok koyu, mavimsi, az parlak yazılı; karanlık odada ekran parlamaz.
    Vurgu biraz kısılır, haritada katman renkleri kısılır.
  - Yüksek karşıtlık: siyah zemin, beyaz yazı, parlak kenarlar. Bölgeler zemin
    tonlarıyla değil kenarlarla ayrılır; yazı ve vurgu en az 7:1
    karşıtlıktadır, vurgu zeminindeki yazı daha okunaklı olan renktir.
  Her temanın kendi harita zemini vardır. Bileşenler temayı iced paletinden
  tanır; bir alt ağaca `iced::widget::themer` ile başka tema verilebilir.
- **Vurgu rengi.** Etkin araç, seçim, odak, birincil düğmeler, öndeki
  pencerenin çizgisi ve haritadaki seçim ve tutamaçlar vurgu rengindedir.
  Sekiz hazır renk vardır: mavi, turkuaz, yeşil, kehribar, turuncu, pembe,
  mor, gri. Her birinin koyu ve aydınlık tema için ayrı tonu seçilmiştir.
  Kendi renginiz `VURGU` komutuyla `#RRGGBB` olarak yazılır; renk temanın
  zemininde okunur kalacak kadar (en az 4:1 karşıtlık) açılır ya da
  koyulaştırılır. Vurgu zeminindeki yazı, rengin açıklığına göre beyaz ya da
  koyudur. iced'in kendi bileşenleri (onay kutusu, kaydırıcı) de aynı rengi
  alır: vurgu, temanın `primary` rengidir.
- **Harita zemini.** Arayüzün temasından bağımsız seçilir: temaya uyan (koyu
  temada arduvaz, aydınlıkta kâğıt, gecede gece haritası, yüksek karşıtlıkta
  siyah), arduvaz, klasik AutoCAD siyahı ya da kâğıt. Koyu arayüzde kâğıt
  zeminli harita da olur.
- **Vitrinde.** Görünüm sekmesindeki Tema grubunda dört temanın önizleme
  karoları, renk düğmeleri ve "Özel renk…"; yanında Harita zemini karoları
  var. `KOYU`, `AYDINLIK`, `GECE`, `KARSITLIK`, `VURGU` ve `ZEMIN` komutları da
  aynı seçenekleri sunar. Seçim ayar dosyasında saklanır (`tema = gece`,
  `vurgu = turuncu`, `harita-zemini = siyah`). Galerinin Renkler sayfası dört
  temayı gerçek bileşenlerle yan yana gösterir.

```rust
use kentos_rc::spatial::model_space::Backdrop;
use kentos_rc::theme::{self, Accent, Mode};

iced::application(App::new, App::update, App::view)
    .theme(|app: &App| theme::theme(app.mode, app.accent)) // Mode::Night, Mode::HighContrast…

Accent::parse("#e8618c"); // Some(Accent::Custom(0xe8618c))

// Bir alt ağaca başka tema: belirteçler onu izler.
themer(Some(theme::theme(Mode::HighContrast, accent)), preview)

ModelSpace::new(viewport, &layers, Message::ModelSpace).backdrop(Backdrop::Black)
```

## Yazı tipleri ve boyut

Yazı ailesi ve boyutu çalışırken değişir; bütün bileşenler, harita etiketleri
dahil, yeni ayarla kurulur. Aileler kütüphaneye gömülüdür (`fonts` özelliği,
varsayılan açık), makinede kurulu olmaları gerekmez:

| Arayüz metni       | Eş aralıklı (koordinat, ölçü, komut) |
|--------------------|--------------------------------------|
| IBM Plex Sans      | IBM Plex Mono                        |
| Inter              | JetBrains Mono                       |
| Plus Jakarta Sans  |                                      |

Hepsi SIL Open Font License ile dağıtılır; kaynakları, lisansları ve
statik kesimlerin nasıl üretildiği `assets/fonts/README.md` dosyasındadır.

Gövde metni 11–18 piksel arasında seçilir, varsayılanı 13'tür. Tip ölçeği
ona göre kurulur (açıklama bir küçük, başlıklar bir ve iki büyük). Metni
taşıyan ölçüler de birlikte büyür: satır yükseklikleri, şerit, menüler,
takvim hücreleri, sütun genişlikleri. Ölçüler tam piksele yuvarlanır;
çizgiler her boyutta keskin kalır. Menü ve liste genişlikleri seçili ailenin
ölçülmüş harf genişliğiyle tahmin edilir.

Vitrinde **Görünüm** sekmesi aileleri kendi yazılarıyla gösterir ve boyutu
değiştirir. `YAZITIPI` ve `PUNTO` komutları da aynı seçenekleri komut
kutusunda sunar; Ctrl +, Ctrl − ve Ctrl 0 boyutu değiştirir. Seçim
`~/.config/kentos-cad/ayarlar` dosyasında saklanır.

```rust
use kentos_rc::theme::typography::{self, Family, Typography};

typography::load(); // gömülü yazı tipleri
typography::set(Typography { family: Family::Inter, size: 14.0, ..Typography::DEFAULT });

iced::application(App::new, App::update, App::view)
    .default_font(typography::ui())
    .run()

// Bileşenler ölçüleri gövde metnine göre büyütür:
container(content).height(typography::scaled(26.0))
```

## Ekransız görüntü

`snapshot` özelliği arayüzü pencere açmadan çizip PNG'ye yazar. Ekran kapalı
ya da kilitliyken, hatta hiç ekran yokken (CI) de çalışır: iced'in ekran dışı
çizicisi kullanılır, önce GPU (wgpu), olmazsa yazılım (tiny-skia). Arayüz
gerçek olaylarla sürülür; bağlam menüsü gibi durumunu kendi tutan bileşenler
de sağ tıkla açılır.

Vitrin bunu bir alt komutla sunar. Senaryo uygulamayı mesajlarla hazırlar;
girdiler ardından verildikleri sırayla uygulanır. Konumlar pencere
koordinatıdır, görüntüdeki piksellerle aynıdır:

```sh
cargo run -- snapshot ekran.png
cargo run -- snapshot menu.png --senaryo agac --sag-tikla 1233,329 --imlec 1100,546
cargo run -- snapshot takvim.png --senaryo yol --tikla 1418,778
cargo run -- snapshot galeri.png --senaryo galeri --sayfa veri --boyut 1440x1500
cargo run -- snapshot secim.png --senaryo secim --tema acik --olcek 2
cargo run -- snapshot oneri.png --senaryo cizim --tikla 800,851 --yaz c
cargo run -- snapshot olcek.png --senaryo cizim --tikla 1255,884
cargo run -- snapshot yazi.png --senaryo secim --yazi inter --esaralikli jetbrains-mono --punto 15
cargo run -- snapshot panel.png --senaryo secim --surukle 1077,400,877,400 --tikla 1140,483
cargo run -- snapshot pencereler.png --senaryo pencereler --bas 400,158 --imlec 406,163
cargo run -- snapshot bildirim.png --senaryo bildirimler --tikla 903,463
cargo run -- snapshot gorevler.png --senaryo gorevler
cargo run -- snapshot onay.png --senaryo onay
cargo run -- snapshot bos.png --senaryo bos-durumlar
cargo run -- snapshot sihirbaz.png --senaryo sihirbaz --sayfa 2
cargo run -- snapshot ozellikler.png --senaryo ozellikler
cargo run -- snapshot duzen.png --senaryo duzen
cargo run -- snapshot yuva.png --senaryo yuva --bas 1150,153 --imlec 700,300
cargo run -- snapshot mini.png --senaryo mini --imlec 437,272
cargo run -- snapshot daire.png --senaryo daire --imlec 600,250
cargo run -- snapshot sekmeler.png --senaryo galeri --sayfa yerlesim
cargo run -- snapshot girdiler.png --senaryo galeri --sayfa girdiler --boyut 1440x1700 --tikla 222,1190
cargo run -- snapshot renk.png --senaryo pencereler --vurgu turuncu --zemin siyah
cargo run -- snapshot mor.png --senaryo secim --tema acik --vurgu "#7c5cff" --zemin arduvaz
cargo run -- snapshot gece.png --senaryo pencereler --tema gece
cargo run -- snapshot karsitlik.png --senaryo secim --tema karsitlik
cargo run -- snapshot --yardim   # senaryolar, girdiler ve galeri sayfaları
```

`KENTOS_SNAPSHOT_BACKEND=tiny-skia` yazılım çiziciyi zorlar. Alt komut
geçişleri kapatır (`theme::motion::set_reduced`); bileşenler son hâlleriyle
çizilir.

## İlkeler

- **Renkler temadan gelir.** Bileşenler renkleri `Tokens::of(&theme)` ile okur;
  uygulama yalnızca `theme::theme(Mode::Dark, Accent::Blue)` verir, renk
  taşımaz.
- **Bileşenler `Message` türünden bağımsızdır** ve yapıcı (builder) desenini
  izler; hepsi `Element`'e dönüşür.
- **Stil fonksiyonları iced imzalarını kullanır**; kentos-rc bileşenleri
  dışında da verilebilir: `button("Kaydet").style(style::button::primary)`.
- **İkonlar düğmenin rengini miras alır**; etkin, üzerine gelinmiş ve devre
  dışı durumlar ikona kendiliğinden yansır.
- **Hesap arayüzden ayrıdır.** `spatial` içindeki veri ve hesap türleri
  (projeksiyon, ölçüm, sorgu, çizim durum makinesi) arayüz olmadan test edilir.
- **Model alanı durum tutmaz.** Görünüm, seçim ve ölçüm uygulamanındır; model
  alanı olanları `model_space::Event` olarak bildirir.
- **Düzenleyiciler değeri değiştirmez.** Nesne inceleyici ve sorgu oluşturucu
  değişikliği mesaj olarak bildirir (`inspector::Action`, `query::Edit`);
  değeri uygulama kendi verisine yazar.
- **Türkçe öncelikli.** Sayılar 15.840.900 ve 1.234,5; tarihler 24.09.2026;
  arama Türkçe harf ve büyük/küçük harf duyarsız; sıralama Türk alfabesine göre.

## Örnekler

```rust
use kentos_rc::icon::Icon;
use kentos_rc::widget::ribbon::{self, Ribbon};

Ribbon::new()
    .application(ribbon::AppButton::new("KentOS CAD").on_press(Message::AppMenu))
    .tabs(Tab::ALL, self.tab, Message::TabSelected)
    .group(
        ribbon::Group::new("Görünüm")
            .push(ribbon::Button::large(Icon::ZoomExtents, "Tümünü\ngör").on_press(Message::FitAll))
            .push(
                ribbon::Stack::new()
                    .push(ribbon::Button::small(Icon::ZoomIn, "Yakınlaştır").on_press(Message::ZoomIn))
                    .push(ribbon::Button::small(Icon::ZoomOut, "Uzaklaştır").on_press(Message::ZoomOut)),
            ),
    )
```

```rust
use kentos_rc::widget::{Inspector, inspector};

// Görünüm: alanlar türlerine göre düzenlenir.
Inspector::new(&self.inspector, Message::Inspector)
    .category("Öznitelikler")
    .field(0, &layer.schema[0], feature.value(0))
    .object(4, &layer.schema[4], feature.value(4), candidates, true)

// Güncelleme: değişiklik uygulamanın verisine yazılır.
Message::Inspector(event) => match self.inspector.update(event) {
    Some(inspector::Action::Change { id, value }) => self.set_attribute(id, value),
    Some(inspector::Action::Pick(id)) => self.start_picking(id),
    Some(inspector::Action::CancelPick) | None => {}
}
```

```rust
use kentos_rc::widget::tree_view::{self, Node, TreeView};
use kentos_rc::widget::{ContextMenu, Menu};

// Ağaç tablo: düğümler iç içe, durumları uygulamanın.
TreeView::new([
    tree_view::Column::new("Ad").width(Fill),
    tree_view::Column::new("Öğe").width(28).align_right(),
])
.push(
    Node::new("Ulaşım")
        .check(group.visible, Message::GroupChecked(id))
        .expanded(group.expanded, Message::GroupToggled(id))
        .push(Node::new("Karayolları").cells([count]))
        .menu(move |_| Menu::new().item("Gruba yakınlaştır", Message::ZoomToGroup(id))),
)

// Herhangi bir öğeye sağ tık menüsü.
ContextMenu::new(model_space, |position| {
    Menu::new()
        .item("Koordinatı kopyala", Message::Copy(position))
        .icon(Icon::Copy)
        .separator()
        .item("Sil", Message::Delete)
        .shortcut("Del")
        .danger()
})
```
