# ADR 0116: Masaüstünde Model tasarımcısı

- **Durum:** kabul edildi (2026-09-27): çekirdek ve pencere.
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** docs/specs/model-designer.md; PROCESSING.md §7; ADR 0084 (İşlemler), 0088 (Sahneden seç).
- **Kaynak:** web'in `processing/model.ts`, `processing/modelEdit.ts`, `ui/processing/model/designerPlan.ts`; ortak durumlar `fixtures/processing/v1/designer.json`.

## Bağlam

Web'de İşlemler'in modelleri bir tasarımcıda kurulur: girdiler, adımlar ve bağlantılar diyagramda, ayarlar sağ sütunda. Masaüstü hazır modeli çalıştırıyordu, ama model kuramıyor ve düzenleyemiyordu. “Yeni model…” masaüstüne taşınmamıştı.

Web ajanı tasarımcının davranışını, web kodunu görmemiş biri için yazdı: `docs/specs/model-designer.md`. Kuralları da DOM'suz bir plana ayırdı ve ortak durumlarla sabitledi.

Masaüstünün modeli bu iş için yetmiyordu:

- Girdileri Rust `ParamDef`'iydi. Web ise modelin girdilerini kendi JSON biçiminde tutar. Tasarımcı bu JSON'u düzenler, kaydedilen model aynı JSON'la geri okunur.
- Adımların değerleri sırayla tutuluyordu, ama JSON'dan okunurken sıra kayboluyordu. serde_json'un kendi haritası anahtarları sıralar; JavaScript yazıldığı sırayla tutar. Kenar yazısının ilk parametresi bu sıraya bağlıdır.
- Veriyle yazılmış görünürlük kuralı yoktu (`visibleWhen: { param, equals }`).

## Karar

Çekirdek `kentos-processing`'tedir; saf Rust'tır, arayüz bilmez:

- **`model.rs`:**
  - Girdiler `ModelInput`'tur: web'in `ParamDef` JSON'u, yazıldığı gibi. Model onlardan parametre üretir (`def`).
  - `Model::to_json` web'in `ProcessingModel`'ini yazar. Tam sayılar JavaScript'teki gibi yazılır (`40`, `40.0` değil).
  - `Model::from_text` ve `Deserialize`, adımların değerlerini yazıldıkları sırayla okur (serde'nin harita ziyaretçisi). Bunun için `kentos-processing` çalışma alanının `serde`'sini kullanır; kilide yeni paket girmedi.
- **`web_param.rs`:** web'in `ParamDef` JSON'undan parametre. `visibleWhen` veriyle `{ param, equals }` yazılır; `types::ShownWhen` kodla yazılan kuralın yanında bunu da taşır.
- **`model_edit.rs`:** web'in `modelEdit.ts`'i:
  - kimlik türetme (`slug`), yeni model ve kopya;
  - girdi ekleme ve silme, adım ekleme (seçili kutuya kendiliğinden bağlanır) ve silme;
  - “Yeni model girdisi yap”, başlık, kaynak atama, model çıktısı;
  - uygun kaynaklar (döngüsüz), kenarlar, sütunlara dizme.
  JavaScript nesnesinin anahtar sırası korunur: yeniden atanan parametre yerinde kalır.
- **`designer.rs`:** web'in `designerPlan.ts`'i:
  - bütün sözler;
  - durum satırı, kaydedilen ad, kaynağın adı, kutunun ikinci satırı;
  - telin menüsü (`connect_choices`);
  - yeni kutunun yeri, geri almanın birleşmesi;
  - diyagramın geometrisi: eğri, noktalar, hedefteki kenar yazıları, sığdırma, yakınlaştırma ve tabanı, ızgara.

### Pencere

Masaüstünün penceresi `apps/desktop/src/processing/designer/`'dadır, web'in `ModelDesigner`'ı gibidir:

- **`mod.rs`:** taslak, kaydedilen ve karşılaştırılan hâli, seçim ve pencerenin kendi geri alması. Geri alma 60 adım tutar; aynı alana 1,2 saniye içinde yazılanlar tek adımdır. Diyagramın görünümü, taşınan araç, telin menüsü ve sorular da buradadır.
- **`update.rs`:** her değişiklik tek yoldan geçer: geri alma adımı, değişiklik, denetim.
  - Kaydet (Ctrl+S) modeli Kitaplığa ve `islemler.json`'a yazar; boş ad “Adsız model” olur.
  - Kapat, × ve Esc kaydedilmemiş değişikliği sorar; Modeli sil de sorar.
  - Sahneden seç: tasarımcı çekilir, nokta çizimde gösterilir, pencere aynı taslakla geri gelir (ADR 0088).
- **`canvas.rs`:** diyagram Iced tuvalinde çizilir:
  - noktalı zemin, eğri bağlantılar ve hedefteki yazıları;
  - kutular: ikon, ad, ikinci satır; sorunlu kutunun kenarı kesikli, seçilinin vurgulu;
  - kutu 10 piksellik ızgarada taşınır; porttan tel çekilir;
  - zemin sürüklenerek kaydırılır, tekerlekle yakınlaştırılır; çift tık da çalışır;
  - araç kutusundan getirilen araç buraya bırakılır.
- **`palette.rs`:** Girdi ekle ve araçlar: arama ve kategoriler; araç tıklanır ya da tuvale sürüklenir.
- **`inspector.rs`:** modelin, girdinin ve adımın ayarları.
  - Her parametrenin kaynağı listeden seçilir: Aracın varsayılanı, Sabit değer, model girdileri, önceki adımların çıktıları ya da Yeni model girdisi yap.
  - Sabit değer, aracın penceresinin kendi denetimiyle girilir (`fields.rs`).
- **`view.rs`:** pencere en çok 1400 × 880'dir, pencerenin %92'sini geçmez. Alt çubukta Düzenle, durum, Kapat, Kaydet ve çalıştır… ve Kaydet vardır; sorular pencerenin üstünde açılır.
- **Açıldığı yerler:**
  - İşlemler sekmesinde Yeni model… ve modelin satırındaki düzenle düğmesi;
  - modelin penceresinde Modeli düzenle ya da Kopyasını düzenle;
  - `processing.newModel` komutu.
  Hazır model kopyası olarak açılır. Kullanıcının modelleri araç kutusundan çalışır.

Kitaplıkta modelin `Serialize`'ı adımların değerlerini sırasıyla yazar. `islemler.json`, web'in `kentos.processing.v1`'i gibidir.

KentOS UI'a eklenenler:

- `TreeView::flat` ve `Node::heading`: ok sütunu olmayan, başlıklı liste (palet).
- `icon::draw(frame, icon, color, at)`: tuvale, yeri verilerek çizilen ikon. Iced'in `with_clip`'i koordinatları kaydırmaz; ikon bu yüzden yerini kendisi alır.
- Menünün iki satırlı öğesinde seçili radyo noktası.
- `Tokens::info`: girdilerin mavisi.

## Sonuçlar

- `crates/native/processing/tests/designer.rs`, `designer.json`'un bütün bölümlerini oynatır:
  - sözler;
  - girdi türleri;
  - diyagram sayıları;
  - tür uyumu;
  - kimlikler;
  - adım adım düzenlemeler (her adımdan sonra modelin JSON'u ve sorunları);
  - son modellerde durum, sıra, kenarlar ve yazıları, kutu satırları, kaynaklar, tel menüleri, kapladığı alan;
  - yeni kutunun yeri, başlık, kaydedilen ad, birleşme;
  - geometri.
- Aynı dosyayı web de oynatır.
- Hazır model girdilerini web'in tanımıyla tutar.
- Çalıştırıcı parametreleri modelin girdilerinden üretir.
- Masaüstünde model kurulur, kaydedilir, yeniden açılır ve silinir.
- Farklar:
  - Web'in taşınan araç hayaleti sayfanın her yerinde görünür; masaüstünde yalnız tuvalin üstünde.
  - Kullanıcının modelleri şeritte ve komut satırında henüz yoktur; araç kutusundan çalışır.

## Doğrulama

- **`cargo test -p kentos-processing`:** `designer` durumları 6 grup, süreçlerin ortak durumları (`cases`) değişmeden geçer.
- **Dikilen hatalar yakalanıyor:** yeni adımın yeri ve sütunlara dizmenin satırı birer piksel kaydırılınca `designer` düşer.
- **`cargo test -p kentos-desktop processing`:** masaüstünün İşlemler'i, hazır modelin penceresi ve çalışması. Tasarımcının 11 testi:
  - modeli parçalardan kurup kaydetmek ve yeniden açmak;
  - yazının tek geri alma adımı olması; Ctrl+Z ve Ctrl+Y;
  - kaydedilmemiş değişiklik sorusu (Vazgeç, Kaydetmeden kapat, Kaydet ve kapat);
  - hazır modelin kopyası olarak açılması;
  - telin menüsü;
  - sabit değerin aracın denetimiyle girilmesi;
  - başlangıç noktasının çizimden gösterilmesi;
  - tuşlar ve Modeli sil;
  - sayı girdisinde yazılanın korunması.
  `processing::memory` testi, kaydedilen modelin değer sırasıyla geri okunduğunu doğrular.
- **Resimler:** `cargo test -p kentos-desktop processing::designer::tests::screens -- --ignored --nocapture`. Web'in 13 sahnesi, iki boyut ve iki temayla `.run/shots/model-*`'a yazılır ve web'in resimleriyle karşılaştırılmıştır.
- **Temiz denetimler:** `cargo clippy -p kentos-processing -p kentos-desktop --all-targets -- -D warnings`; `pnpm arch:deps`.
