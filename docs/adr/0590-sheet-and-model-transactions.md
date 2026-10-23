# ADR 0590: Model ve paftanın birlikte geri alınması

- Tarih: 22 Ekim 2026
- Kapsam: CAD-SHEET-05; ADR 0251 §3 ve ADR 0252'nin işlem kuralını tamamlar.
- Ölçek sorunu kullanıcının kararıyla sonraya bırakıldı; bu karar ölçek davranışını değiştirmez.

## Sahiplik ve işlem sınırı

1. Pafta kitabı ve resimleri projenindir; `.kcad` ve bulut snapshot'ında kalır.
2. Pafta tasarımcısının normal düzenlemeleri kendi geçmişindedir. Tek başına `cad.sheets.edit` çizimin geçmişine eklenmez; kendi `inverse` işlemiyle geri alınır.
3. Açık bir belge grubu (`doc.group`, masaüstü Python çalışması, yerel `begin_group`) içindeki `set_sheets`, modelle aynı grupta bir pafta işlemi kaydeder. Başarısız grup iptali ve savepoint geri dönüşü kitabı ve resimleri de geri getirir. Başarılı grup tek model undo/redo adımıdır; yalnız pafta yazan grup da bir adımdır.
4. `cad.changeset.apply` paftayı da aynı undo/redo adımına alır. Başarısız alt komut mevcut `run_step` ile bütün değişiklikleri geri alır. Doğrudan `cad.sheets.edit` davranışı değişmez.
5. Kitap snapshot'ları `Arc` ile tutulur. Dosya biçimi ve pafta öğe modeli değişmez; undo geçmişi dosyaya taşınmaz.

## Sonraki bağımsız pafta düzenlemesi

- Birleşik adımın undo'su, kitabın adım sonundaki sürümüyle; redo'su adım başındaki sürümüyle karşılaştırılır. Kitap ve resim içerikleri eşleşmeden hiçbir model veya pafta işlemi uygulanmaz, geçmişten adım düşürülmez.
- Sonradan pafta tasarımcısında veya buluttan değişmiş kitap eski snapshot ile ezilmez. Kullanıcıya `sheet_history_conflict` ve sonraki pafta düzenlemesini önce geri alma açıklaması verilir.
- Kitapla ilgisi olmayan model adımları bundan etkilenmez. İlgili kitap durumu tekrar sağlandığında birleşik adım geri alınabilir/yinelenebilir.
- Çizimin kitabı dışarıdan veya birleşik undo ile değişince mevcut `follow_sheet_book` tasarımcıyı aynı kitaba açar ve eski kâğıt undo geçmişini sıfırlar (ADR 0251 §4). İki ayrı otorite oluşmaz.
- DWG/DXF içe aktarmanın mevcut kitap snapshot'ı da aynı korumayı kullanır. Kitap değiştirmeyen stil içe alması sonraki bağımsız pafta düzenlemesini engellemez ve geri alırken ezmez.

## API ve kanıt

- Yeni komut veya dosya alanı yoktur. UI, komut, Python ve MCP aynı native işlem yolunu kullanır.
- Komut özeti paftayı `project` içinde bildirir; birleşik adımda `undo.outside` içinde bildirmez. Yalnız pafta içeren changeset'in undo etiketi de vardır.
- Kabul: model+kitap başarı/undo/redo; yalnız kitap; hata ve iptal; birden çok kitap yazma; grup savepoint'i; resim baytları; bağımsız pafta geçmişi; sonraki kitap değişikliğinde undo/redo'nun bütünüyle reddi; kitap değiştirmeyen içe alma; planın salt okunurluğu; günlük ve özet.
- Proje ayarları ve stil kitaplığının başarılı undo politikası bu pafta kararının kapsamı değildir; mevcut davranışları korunur.

### Doğrulama kaynakları

- Bağımsız beklenen durumlar: `scripts/fixtures/sheet_transaction_cases.py --check`; 22 senaryo aynı
  `fixtures/sheet/v1/model-transactions.json` üzerinden native headless ve gerçek Python `doc.group` ile yürür.
- `crates/native/domain/tests/sheet_history.rs`: gömülü resim baytları, savepoint, iç içe grup, eşit kitap, kitap ve stil içe alma geçmişi.
- Komut protokolü: yalnız paftalı changeset'in özeti, damgaları, gerçek revizyonu ve salt okunur planı; tüm mevcut komut fixture'ları.
- Masaüstü: gerçek geri alma tuşlarının model/pafta kipinde yönlenmesi, sonraki bağımsız paftanın korunması;
  Python konsolunun başarı, hata ve durdurmasında yalnız kitap ve model+kitap birlikte sınanır.
- MCP: birleşik changeset, undo/redo ve sonraki bağımsız düzenlemede `sheet_history_conflict`.
- GPU görüntüleri: `cargo test -p kentos-desktop sheet_book::tests::transaction_screens -- --ignored --test-threads=1 --nocapture`.
  `.run/shots/pafta-islem-{koruma,geri}-{dark-13,light-13,dark-18}.png`: açık/koyu 1440×900 ve 18 px yazıyla 1100×650.
  Üretilen PNG'ler açılıp incelenir; geri alınmış kitaptaki gömülü kırmızı logonun gerçekten çizildiği piksel sayısıyla da denetlenir.
  Örnekteki kaynak harita bağlı değildir; harita yer tutucusu ve iki ön denetim bulgusu örnek verinin beklenen durumudur.
- Web arayüzü duruyor (ADR 0228); yalnız ortak sözleşme/katalog üretimi ve mevcut WASM bağlayıcılarının derlemesi yapılır.
