# ADR 1004: Parametreli genel detayın konaklar arası kabulü

- **Durum:** kabul edildi (2026-11-05). Kapsam: plan/cad-advanced.md `CAD-ADV-20` kabulü “aynı genel detayı GUI, Python
  ve MCP üzerinden oluştur, ilişkili ölçü … ekle, parametre değiştir … Kaynak geometri, bağlar, retler ve kalıcı sonuç
  eşdeğer olsun. … tekrar gönderim, yanlış … revizyon … yarım iş kalmasın.”
- **Bağlam belgesi:** ADR 0253 (protokol: istek kimliği, beklenen durum, değişiklik kümesi), ADR 0336/0338
  (parametreler ve kısıtlar), oda kabul senaryosu (`fixtures/acceptance/v1/room.json`, CADP-12).

## 1. Senaryo

`scripts/fixtures/acceptance_detail.py` KentOS kodu kullanmadan `fixtures/acceptance/v1/detail.json`'u yazar: kaba
çizilmiş dört çizgilik plaka (ve kilitli katmanda bir çizgi); kısıtlarla dikdörtgen olur (sol alt köşe sabit,
`genislik` 8 m, `yukseklik` 4 m); alt kenarın uçlarına ilişkili ölçü; genişlik 10 ve `1200cm`; Undo; eski revizyonla
yazım (`revision_conflict`); ikinci adımı kilitli katmana çarpan değişiklik kümesi (`layer_locked`, ilk adım da
yazılmaz); aynı `requestId` ile iki kez gönderilen istek (bir kez uygulanır, ikincisi `replayed`); kaydet–aç; açılan
çizimde genişlik 9. Beklenen her nokta ve uzunluk dikdörtgenin kenarlarından yazılır (ölçü alt kenarı, uzunluğu
genişliği ölçer).

## 2. Konaklar

- **Python** (`python/tests/test_acceptance_detail.py`): yerel motor, `Document.run` ile katalog JSON'u.
- **MCP** (`crates/native/mcp/tests/acceptance_detail.rs`): araç çağrısı ardına araç çağrısı, `requestId`/`expected`
  argümanlarıyla, kaydet–aç `drawing.save`/`drawing.open` ile.
- **Masaüstü** (`review_scenes::detail`): aynı adımlar uygulamanın komut kayıt defterinden; Parametreler penceresi
  açıkken görüntülendi.

İki otomatik konak her adımda aynı beklenen değerleri ilk koşuda tuttu (noktalar 1e-9, uzunluklar 1e-6 içinde); her
ret çizimin revizyonunu değiştirmedi.

## 3. Sınırlar

Bu senaryoda pafta çıktısı, iş iptali ve eklenti/AI sağlayıcısı yoktur: pafta ve PDF/DXF çıktısı oda senaryosunda,
iş iptali `test_protocol`/dizi benimsetme testlerinde ayrı kanıtlıdır; eklenti (`.kek`) ve AI sağlayıcı bağlantısının
bu maddeye göre uçtan uca kabulü açıktır. Masaüstü sahnesi gerçek fare kaydı değildir.
