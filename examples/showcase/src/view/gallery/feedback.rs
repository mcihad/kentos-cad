//! Geri bildirim sayfası: bildirimler, ilerleme ve görevler, onay kutusu,
//! uyarı şeridi, boş ve hata durumları.

use iced::widget::{Row, button, column, container, row};
use iced::{Center, Element, Fill};

use kentos_rc::icon::Icon;
use kentos_rc::label;
use kentos_rc::style;
use kentos_rc::theme::typography;
use kentos_rc::widget::progress::{self, State, Task, TaskList, Tint};
use kentos_rc::widget::{Banner, Confirm, EmptyState};

use super::{entry, pressed};
use crate::app::Showcase;
use crate::gallery::{Demo, sample_toasts};
use crate::message::Message;

impl Showcase {
    pub(super) fn feedback_page(&self) -> Vec<Element<'_, Message>> {
        let samples = sample_toasts();
        let all = samples.len();

        let buttons = samples
            .into_iter()
            .enumerate()
            .map(|(index, (name, _))| demo_button(name, index))
            .chain(std::iter::once(demo_button("Hepsi birden", all)));

        let bars = column![
            bar_row("Oranı bilinen", progress::bar(Some(0.62)).into(), "%62"),
            bar_row("Oranı bilinmeyen", progress::bar(None).into(), "sürüyor"),
            bar_row(
                "Bitti",
                progress::bar(Some(1.0)).tint(Tint::Success).into(),
                "%100",
            ),
            bar_row(
                "Başarısız",
                progress::bar(Some(0.7)).tint(Tint::Danger).into(),
                "%70",
            ),
        ]
        .spacing(12);

        let spinners = row![
            progress::spinner().size(12.0),
            progress::spinner(),
            progress::spinner().size(20.0),
            progress::spinner().size(28.0).tint(Tint::Muted),
        ]
        .spacing(20)
        .align_y(Center);

        let tasks = TaskList::new()
            .push(
                Task::new("GeoJSON olarak dışa aktar")
                    .detail("Türkiye.geojson: 38 / 60 öğe")
                    .running(Some(0.63))
                    .on_cancel(pressed("İptal et")),
            )
            .push(
                Task::new("Uzamsal dizin oluştur")
                    .detail("Katmanlar taranıyor")
                    .on_cancel(pressed("İptal et")),
            )
            .push(
                Task::new("Paftaları içe aktar")
                    .detail("48 pafta, 12 saniye.")
                    .state(State::Done)
                    .on_dismiss(pressed("Listeden kaldır")),
            )
            .push(
                Task::new("DXF olarak dışa aktar")
                    .detail("Karayolları: çizgi tipi DXF'te karşılanamadı.")
                    .state(State::Failed)
                    .on_retry(pressed("Yeniden dene"))
                    .on_dismiss(pressed("Listeden kaldır")),
            );

        vec![
            entry(
                "Bildirimler",
                "kentos_rc::widget::Toaster",
                "Alanın sağ alt köşesinde üst üste dizilen kısa iletiler. İkonun ve alttaki kalan \
             süre çizgisinin rengi önem düzeyidir. Bilgi ve başarı 5, uyarı 8 saniyede kapanır; \
             eylemli bildirim en az 8 saniye durur, hata kendiliğinden kapanmaz. İmleç \
             üzerindeyken süre durur. En fazla üç bildirim görünür, eskiler sayılır; aynı \
             bildirim yinelenirse sayısı artar. Düğmelerle deneyin: bildirimler bu sayfanın \
             sağ alt köşesinde açılır.",
                Row::with_children(buttons).spacing(6).align_y(Center),
                Some(
                    "self.toasts.push(\n    \
                 Toast::success(\"Çizim silindi\").action(\"Geri al\", Message::UndoDelete),\n);\n\n\
                 Toaster::new(map, &self.toasts, Message::ToastClosed)\n\n\
                 // update\n\
                 Message::ToastClosed(id) => self.toasts.dismiss(id),",
                ),
            ),
            entry(
                "İlerleme çubuğu",
                "kentos_rc::widget::progress::bar",
                "İnce çubuk: oranı bilinen işte dolar, bilinmeyende üzerinde bir parça soldan \
             sağa kayar. Renk işin durumunu söyler: süren iş vurgu, biten iş yeşil, başarısız \
             iş kırmızı.",
                bars,
                Some(
                    "progress::bar(Some(0.62))\nprogress::bar(None) // oranı bilinmeyen iş\nprogress::bar(Some(1.0)).tint(Tint::Success)",
                ),
            ),
            entry(
                "Dönen gösterge",
                "kentos_rc::widget::progress::spinner",
                "Süren işin küçük göstergesi: çember üzerinde sekiz nokta, öndeki en koyu. Durum \
             çubuğunda ve görev satırında kullanılır.",
                spinners,
                Some("progress::spinner().size(12.0)"),
            ),
            entry(
                "Görev listesi",
                "kentos_rc::widget::TaskList",
                "Arka plandaki işler durumlarıyla: sürüyor, sırada, bitti, başarısız, iptal edildi. \
             Süren ve sıradaki iş durdur düğmesiyle iptal edilir; başarısız iş yeniden denenir; \
             biten iş listeden kaldırılır. Vitrinde dışa aktarma ve dizin oluşturma buraya düşer: Yönet \
             sekmesi ya da uygulama menüsü.",
                container(tasks)
                    .width(typography::scaled(420.0))
                    .padding(1)
                    .style(style::container::bordered),
                Some(
                    "TaskList::new()\n    \
                 .push(Task::new(\"GeoJSON olarak dışa aktar\")\n        \
                 .detail(\"Türkiye.geojson: 38 / 60 öğe\")\n        \
                 .running(Some(0.63))\n        \
                 .on_cancel(Message::Cancel(id)))\n    \
                 .push(Task::new(\"DXF olarak dışa aktar\").state(State::Failed).on_retry(Message::Retry(id)))",
                ),
            ),
            entry(
                "Onay kutusu",
                "kentos_rc::widget::Confirm",
                "Başlık soru olarak yazılır; onay düğmesi işin adını taşır (\"Tamam\" değil, \
                 \"Tümünü sil\"). Yıkıcı işte onay düğmesi kırmızıdır; geri alınabiliyorsa bu \
                 söylenir. overlay::modal ile ortalanır; Enter onaylar, Esc vazgeçer. Vitrinde \
                 Çizimler katmanının menüsündeki \"Çizimleri temizle\" ve çizim varken çıkış \
                 onay ister.",
                confirm_sample(),
                Some(
                    "overlay::modal(\n    \
                     Confirm::new(\"Bütün çizimler silinsin mi?\", Message::Clear, Message::Cancel)\n        \
                     .message(\"Çizimler katmanındaki 5 öğe silinecek.\")\n        \
                     .confirm(\"Tümünü sil\")\n        \
                     .destructive(),\n    \
                     Message::Cancel,\n)",
                ),
            ),
            entry(
                "Uyarı şeridi",
                "kentos_rc::widget::Banner",
                "Bir alanın üstünde süren bir durumu anlatır: olay değil, hâl. Bildirimden farkı \
                 kapatılana ya da durum değişene kadar yerinde kalmasıdır. Vitrinde örnek veri \
                 silinmeye çalışılınca haritanın üstünde salt okunur şeridi açılır.",
                banners(),
                Some(
                    "Banner::warning(\"Örnek veri katmanları salt okunur.\")\n    \
                     .action(\"Çizimlere geç\", Message::ActivateDrawings)\n    \
                     .on_dismiss(Message::BannerClosed)",
                ),
            ),
            entry(
                "Boş ve hata durumları",
                "kentos_rc::widget::EmptyState",
                "İçeriği olmayan alanın ortasında ne olduğu ve ne yapılabileceği; hata durumunda \
                 neyin yapılamadığı ve nasıl düzeltileceği. Hata özür dilemez, neyin olduğunu \
                 açıkça söyler. Vitrinde bütün katmanlar gizliyken harita ve satırı olmayan \
                 öznitelik tablosu (filtre, arama, boş katman) böyle görünür.",
                empty_states(),
                Some(
                    "EmptyState::new(Icon::Layers, \"Haritada görünür katman yok\")\n    \
                     .description(\"Bütün katmanlar gizli.\")\n    \
                     .primary(\"Tümünü göster\", Message::ShowAll)\n\n\
                     EmptyState::error(\"parseller.csv okunamadı\")\n    \
                     .description(reason)\n    \
                     .primary(\"Yeniden dene\", Message::Retry)",
                ),
            ),
        ]
    }
}

/// Onay kutusu örneği; bir kaplama olmadan, yerinde.
fn confirm_sample<'a>() -> Element<'a, Message> {
    Confirm::new(
        "Bütün çizimler silinsin mi?",
        pressed("Tümünü sil"),
        pressed("Vazgeç"),
    )
    .message("Çizimler katmanındaki 5 öğe silinecek.")
    .detail("Silinen çizimler bildirimdeki Geri al ile geri getirilebilir.")
    .confirm("Tümünü sil")
    .destructive()
    .into()
}

/// Üç önem düzeyinde uyarı şeridi.
fn banners<'a>() -> Element<'a, Message> {
    column![
        banner(
            Banner::info(
                "Bu çizim KentOS CAD 0.1 ile kaydedilmiş; açılırken yeni sürüme yükseltildi."
            )
            .on_dismiss(pressed("Kapat")),
        ),
        banner(
            Banner::warning(
                "Örnek veri katmanları salt okunur: yalnızca Çizimler katmanı düzenlenir."
            )
            .action("Çizimlere geç", pressed("Çizimlere geç"))
            .on_dismiss(pressed("Kapat")),
        ),
        banner(
            Banner::error(
                "Altlık harita sunucusuna ulaşılamıyor; önbellekteki paftalar gösteriliyor."
            )
            .action("Yeniden bağlan", pressed("Yeniden bağlan")),
        ),
    ]
    .spacing(8)
    .into()
}

/// Uyarı şeridi örneği: ince kenarlı bir alanın üstünde.
fn banner<'a>(banner: Banner<'a, Message>) -> Element<'a, Message> {
    container(column![
        banner,
        container(label::caption("…")).padding([10, 12])
    ])
    .padding(1)
    .width(Fill)
    .style(style::container::bordered)
    .into()
}

/// Boş durum ve hata durumu yan yana.
fn empty_states<'a>() -> Element<'a, Message> {
    let stage = |state: EmptyState<'a, Message>| {
        container(state)
            .width(Fill)
            .height(typography::scaled(250.0))
            .style(style::container::field_box)
    };

    row![
        stage(
            EmptyState::new(Icon::Search, "\"Kadıköy\" için kayıt yok")
                .description(
                    "Arama bütün alanlarda, büyük küçük harf ve Türkçe karakter ayırmadan yapılır.",
                )
                .primary("Aramayı temizle", pressed("Aramayı temizle")),
        ),
        stage(
            EmptyState::error("parseller.csv okunamadı")
                .description(
                    "12. satırda 5 sütun bekleniyordu, 4 var. Dosyayı düzeltip yeniden deneyin \
                     ya da başka bir dosya seçin.",
                )
                .primary("Yeniden dene", pressed("Yeniden dene"))
                .secondary("Başka dosya", pressed("Başka dosya")),
        ),
    ]
    .spacing(12)
    .into()
}

/// Ad, çubuk ve değerden oluşan örnek satırı.
fn bar_row<'a>(name: &'a str, bar: Element<'a, Message>, value: &'a str) -> Element<'a, Message> {
    row![
        label::muted(name).width(typography::scaled(120.0)),
        container(bar).width(typography::scaled(280.0)),
        label::mono_caption(value),
    ]
    .spacing(12)
    .align_y(Center)
    .width(Fill)
    .into()
}

fn demo_button<'a>(name: &'a str, index: usize) -> Element<'a, Message> {
    button(label::body(name))
        .on_press(Message::Gallery(Demo::Notify(index)))
        .padding([4, 12])
        .style(style::button::secondary)
        .into()
}
