# kentos-rc

**KentOS Rust Components**: [iced](https://github.com/iced-rs/iced) 0.14 üzerine
kurulu, CBS ve CAD uygulamaları için bileşen kütüphanesi.

```sh
cargo run                       # vitrin uygulaması: KentOS CAD
cargo test --workspace          # kütüphane ve vitrin testleri
cargo doc -p kentos-rc --open   # API belgeleri
```

## Yapı

```
src/                     kentos-rc kütüphanesi
├── theme/               renk belirteçleri (Tokens), tip ölçeği, iced teması
├── style/               iced stil fonksiyonları: button, container, text, field
├── icon/                16×16 ızgarada çizilmiş vektör ikon seti
├── label.rs             tip ölçeğine bağlı hazır metin biçimleri
├── widget/              uygulama çerçevesi
│   ├── ribbon/          şerit: sekmeler, gruplar, düğmeler, alanlar
│   ├── app_menu.rs      uygulama menüsü (Office "Dosya" menüsü gibi)
│   ├── dock.rs          yan panel yuvası ve başlıklı paneller
│   ├── table.rs         veri tablosu
│   ├── property_grid.rs özellik ızgarası
│   ├── command_line.rs  CAD komut satırı
│   ├── status_bar.rs    durum çubuğu ve anahtarlar
│   ├── navigation_bar.rs, dialog.rs, overlay.rs, tip.rs
└── spatial/             CBS ve CAD (`spatial` özelliği, varsayılan açık)
    ├── projection.rs    LonLat, Bounds, Viewport (Web Mercator)
    ├── feature.rs       Geometry, Feature, Layer, FeatureRef
    ├── measure.rs       jeodezik mesafe, Measurement
    ├── query.rs         öğe seçimi (hit test) ve nesne yakalama
    ├── tool.rs          araçlar
    ├── draft.rs         çizim araçlarının durum makinesi
    ├── format.rs        Türkçe koordinat, mesafe ve sayı yazımı
    ├── model_space/     etkileşimli harita/çizim alanı
    └── view_cube.rs     wgpu ile çizilen yön küpü

examples/showcase/       KentOS CAD: kütüphanenin vitrin uygulaması
├── app.rs               durum ve güncelleme mantığı
├── message.rs           mesajlar, sekmeler, menü komutları
├── command.rs           komut satırı çözümleyicisi
├── sample.rs            örnek veri
└── view/                bileşenlerin yerleşimi
```

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

## Örnek

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
