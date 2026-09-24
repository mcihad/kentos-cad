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
├── theme/               renk belirteçleri (Tokens), yazı ayarı ve tip ölçeği, iced teması
├── style/               iced stil fonksiyonları: button, container, text, field
├── icon/                16×16 ızgarada çizilmiş vektör ikon seti
├── label.rs             tip ölçeğine bağlı hazır metin biçimleri
├── snapshot.rs          ekransız görüntü (`snapshot` özelliği)
├── widget/              uygulama çerçevesi
│   ├── ribbon/          şerit: sekmeler, gruplar, düğmeler, alanlar
│   ├── app_menu.rs      uygulama menüsü (Office "Dosya" menüsü gibi)
│   ├── dock.rs          yan panel yuvası: açılıp kapanan paneller, sürüklenen kenar
│   ├── sash.rs          boyutlandırma tutamağı (sürükle, çift tıkla sıfırla)
│   ├── table.rs         veri tablosu: sıralama, çoklu seçim, yatay kaydırma
│   ├── tree_view.rs     ağaç tablo: sınırsız derinlik, üç durumlu onay kutusu
│   ├── context_menu.rs  sağ tık menüsü ve menü düğmesi: alt menü, kısayol, klavye
│   ├── inspector.rs     nesne inceleyici: arama, kategoriler, geri alma, yardım
│   ├── date_picker.rs   tarih, tarih-saat ve saat seçicileri (açılır takvim)
│   ├── select.rs        aranabilir, açılır seçim kutusu
│   ├── query_builder.rs sorgu oluşturucu
│   ├── toolbar.rs       araç çubuğu: arama, eylemler, anahtarlar
│   ├── segmented.rs     parçalı seçim
│   ├── property_grid.rs salt okunur özellik ızgarası
│   ├── command_line.rs  komut kutusu: geçmiş, istem, otomatik tamamlama
│   ├── status_bar.rs    durum çubuğu: menülü göstergeler ve anahtarlar
│   ├── navigation_bar.rs, dialog.rs, overlay.rs, tip.rs
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
├── sample.rs            örnek veri ve öznitelik şemaları
├── settings.rs          kalıcı ayarlar: tema ve yazı (~/.config/kentos-cad/ayarlar)
├── snapshot.rs          `snapshot` alt komutu ve senaryolar
└── view/                bileşenlerin yerleşimi
    └── gallery/         bileşen kataloğu (Galeri sekmesi)
```

Vitrindeki **Galeri** sekmesi kütüphanenin kataloğudur: renkler, yazı, ikonlar,
düğmeler, veri bileşenleri, çerçeve, öznitelikler ve CBS/CAD bileşenleri; her
biri canlı örneği, modül yolu ve kullanım koduyla.

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
  rengi ve görünürlüğü ayrıdır.
- **Yan panel.** Sol kenarı sürüklenerek genişletilir, çift tık varsayılan
  genişliğe döndürür; paneller başlıklarına tıklanınca açılıp kapanır. Düzen
  ayar dosyasında saklanır.
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
  eski satırlar soluklaşır. F2 bütün geçmişi açar.
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
cargo run -- snapshot --yardim   # senaryolar, girdiler ve galeri sayfaları
```

`KENTOS_SNAPSHOT_BACKEND=tiny-skia` yazılım çiziciyi zorlar.

## İlkeler

- **Renkler temadan gelir.** Bileşenler renkleri `Tokens::of(&theme)` ile okur;
  uygulama yalnızca `theme::theme(Mode::Dark)` verir, renk taşımaz.
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
