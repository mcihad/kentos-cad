# ADR 1011: Konaklar arası kabulde tarama, pafta, yarıda çöken betik ve durdurulan iş

- **Durum:** kabul edildi (2026-11-06). Kapsam: plan/cad-advanced.md `CAD-ADV-20` kabulü “aynı genel detayı GUI, Python
  ve MCP üzerinden oluştur, ilişkili ölçü/tarama ekle, parametre değiştir ve pafta çıkar … İptal, tekrar gönderim,
  yanlış belge/revizyon ve script çökmesinde yarım iş kalmasın.” ADR 1004'ün açık bıraktığı pafta, tarama, iptal ve
  çökme parçaları.
- **Bağlam belgesi:** ADR 1004 (parametreli plaka senaryosu), ADR 0253 §9 (işler), ADR 1090 (`cad.sheets.details`).

## 1. Karar

`scripts/fixtures/acceptance_detail.py` (KentOS kodu yok) senaryoya şunları ekler; Python
(`python/tests/test_acceptance_detail.py`) ve MCP (`crates/native/mcp/tests/acceptance_detail.rs`) aynı dosyayı oynatır:

- **İlişkili tarama:** dört çizginin çevrelediği yüze `cad.entities.hatch` (`way: lines`). Beklenen alan her adımda
  plakanın w × h'sidir (8 × 4 = 32 … 9 × 5 = 45): parametre değişince, geri alınınca, kaydedip açınca tarama izler.
- **Yarıda çöken betik:** iki adım (genişlik 14, sonra olmayan nesneyi taşıma). Python'da adımlar `doc.group` içinde
  koşar ve betik sonda kendi istisnasını atar; grup hepsini geri alır. MCP betik koşturmaz: modelin aynı adımları tek
  `cad.changeset.apply`'dır ve son adımı reddedilir. İkisinde de revizyon değişmez, genişlik 10 kalır.
- **Durdurulan iş:** genişlik 30 iş olarak başlatılır ve bakılmadan durdurulur; iş sonucu ancak bakılınca yazdığı için
  (ADR 0253 §9) durdurma her zaman zamanındadır: `cancelled`, hiçbir şey yazılmaz.
- **Pafta:** son adımda plakanın 10 × 6 m'lik kapsamı 1:50 detay paftası olur (`cad.sheets.details`); çıktıda görünümün
  ölçeği ve detayın paydası 50. İlk denemedeki 1:20 (550 × 400 mm) A3 şablonunun 370 × 208 mm'lik alanına sığmadığı için
  komut `detail_does_not_fit` ile reddetti; ölçek senaryoda düzeltildi, kural değişmedi.

Masaüstü (GUI) aynı adımları Parametreler penceresiyle ADR 1004'te görüntülemişti; bu ADR GUI'ye yeni pencere eklemez.

## 2. Kanıt

MCP testi ve Python testi (maturin ile yeniden derlenen modülle) geçti; çöken betik, durdurulan iş ve tarama adımları
ilk koşuda tuttu.

## 3. Sınırlar

Eklenti (`.kek`) katkısının ve AI sağlayıcı seçiminin kabulü açık. PDF yayını bu senaryoda değil (ADR 1090'ın kabul
testlerinde). Yanlış belge ADR 1004'te revizyon çatışmasıyla sınandı; iki ayrı açık belge arasında karışma ayrıca
sınanmadı.
