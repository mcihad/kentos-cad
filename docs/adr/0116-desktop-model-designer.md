# ADR 0116: Masaüstünde Model tasarımcısı

- **Durum:** çekirdek kabul edildi (2026-09-27); pencere sürüyor.
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

Pencere (tuval, parçalar, ayarlar, taslak ve geri alma, kaydetme ve kapatma sorusu, Sahneden seç) bu ADR'nin ikinci dilimidir.

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

## Doğrulama

- **`cargo test -p kentos-processing`:** `designer` durumları 6 grup, süreçlerin ortak durumları (`cases`) değişmeden geçer.
- **Dikilen hatalar yakalanıyor:** yeni adımın yeri ve sütunlara dizmenin satırı birer piksel kaydırılınca `designer` düşer.
- **`cargo test -p kentos-desktop processing`:** masaüstünün İşlemler'i, hazır modelin penceresi ve çalışması.
- **Temiz denetimler:** `cargo clippy -p kentos-processing -p kentos-desktop --all-targets -- -D warnings`; `pnpm arch:deps`.
