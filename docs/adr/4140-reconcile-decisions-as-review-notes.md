# 4140 — Detay eşleştirmenin kararları ayrı inceleme kaydı olarak

Tarih: 2026-11-09. plan/map-cad.md MAP-04, sekizinci tur. Durum: taslak. ADR 1760'ın ve 3740'ın “Sonra”sındaki “Ayrı
inceleme kaydı (kararların öznitelik değil kayıt olarak tutulması)”.

İlgili:
- [ADR 1000](1000-review-notes-and-taking-revision-differences.md) §2: inceleme notları; tasarımdan ayrı, nesneye
  damgasıyla bağlı;
- [ADR 1760](1760-asset-candidate-reconciliation.md): Detay eşleştirme (`map.features.reconcile`) ve kararları;
- [ADR 2540](2540-reconcile-field-rules-and-entry.md): sınıf katmanının alan kuralları.

## Dayanak

- **ArcGIS Data Reviewer**,
  [Fields in the Reviewer table](https://desktop.arcgis.com/en/arcmap/10.6/extensions/data-reviewer/fields-in-the-reviewer-table.htm)
  (ArcMap 10.6; 9 Kasım 2026'da okundu):
  - inceleme sonuçları nesne sınıfından ayrı bir tabloda tutulur;
  - her kaydın sürekli bir kimliği (RECORDID) ve oturumu (SESSIONID) vardır;
  - kayıt, nesneyi kendi sınıfındaki kimliğiyle (OBJECTID) ve denetimin koştuğu sınıfı (ORIGINTABLE) adlandırır;
  - hatanın tanımı (REVIEWSTATUS), notlar, gözden geçiren (REVIEWTECHNICIAN) ve kaydın yazıldığı tarih (REVIEWDATE)
    kayıttadır;
  - yaşam döngüsü evresi ve durumu (LIFECYCLEPHASE, LIFECYCLESTATUS) ayrı alanlardır.
- **ArcGIS Pro 3.6**,
  [Error results and their life cycle](https://pro.arcgis.com/en/pro-app/3.6/help/data/validating-data/results-and-life-cycle-phases.htm)
  (9 Kasım 2026'da okundu):
  - veri kalitesi gereklerini karşılamayan nesneler Reviewer hata sonucu olarak saklanır;
  - sonuç, sorunun bilgisini ve isteğe bağlı bir geometriyi taşır;
  - evreler İnceleme, Düzeltme, Doğrulamadır; iş akışı kimin ne zaman düzelttiğini ve doğruladığını kaydeder.
  - Sayfa, Reviewer çalışma alanına dayalı iş akışının kullanımdan kalkacağını söyler; KentOS'un dayandığı yalnız
    “sonuç nesneden ayrı bir kayıttır” ilkesidir.
- **KentOS bugün:**
  - Detay eşleştirme kararları yazılan varlığın öznitelikleriydi (`İnceleme`, `Gözden geçiren`, ADR 1760 §5). Kimin
    hangi adayı neyle değiştirdiği nesnenin verisine karışıyordu; reddedilen adayın kararı yalnız işaretindeydi;
  - çizimde zaten nesneden ayrı, damgalı bir inceleme kaydı vardır: inceleme notları (ADR 1000 §2), listesi
    `cad.review.notes.list`, penceresi İnceleme notları.

## Karar

1. **Karar, inceleme notudur.** Yeni bir depo kurulmaz; ADR 1000'in notlarına isteğe bağlı yapılandırılmış `decision`
   eklenir (şema 215):
   - `command` (`map.features.reconcile`), `candidate`, `source`;
   - verildiği gibi `accept`, `class`, `fields`, `reject` (neden);
   - `asset`: adayın grubunun yazıldığı ya da bağlandığı varlık, retle yok.
2. **Hangi karar not olur:** bir şey söyleyen (kabul, sınıf, dolu alan ya da ret) ve adayı bu çalıştırmada işaretlenen
   her karar, girdinin sırasıyla. Çözülmüş (`resolved`), kullanıcının; çizimin notlarının ardından numaralanır.
   - Yazarı `reviewer` (kırpılmış), zamanı yeni girdi `reviewedAt` (verildiği gibi, kırpılmış). Masaüstü o anın
     UTC zamanını verir.
   - Adaya ve varlığa damgalarıyla, adımın bıraktığı hâliyle bağlıdır. Varlık sonra değişirse not eskimiş görünür.
   - Metni: “Detay eşleştirme, Ortofoto adayı: kabul edildi.”, “… sınıfı “Direk” olarak düzeltildi.”, “… alanları
     verildi (KapiNo = 14).”, “… reddedildi (Gölge; nesne yok).”. 200 karakteri aşan alan ya da neden kararda kalır.
   - Notlar aynı geri alma adımındadır (“Detay eşleştirme”). Çıktının `notes`'u numaralarıdır.
3. **Varlık artık `İnceleme` ve `Gözden geçiren` taşımaz.** `Kaynaklar`, `Adaylar`, `Yayılım`, `Örtüşme` köken
   olarak kalır. Adayın `Eşleştirme` işareti kalır: adayın kendi durumudur, sonraki çalıştırma onu okumaz.
4. **Boş alan değeri** karar sayılmaz: varlığa yazılmadığı gibi nota da yazılmaz.
5. **Yeni retler** (`invalid_input`), nota sığmayan girdi:
   - kaynağın adı 120 karakteri aşıyor ya da denetim karakteri taşıyor;
   - gözden geçirenin adı (120) ya da zaman (64) kısa etiket değil;
   - ret nedeni 4000 karakteri aşıyor ya da denetim karakteri taşıyor;
   - alanın adı (64) ya da dolu değeri nota sığmıyor.

   Notlar 10 000'i aşacaksa hiçbir şey yazılmaz (`too_many_review_notes`, ADR 1000'in sınırı).
6. **`.kcad` şema 215:** notun `decision`'ı. Not adayına ve varlığına bağlı olmalı; karar bir şey söylemeli; kabul ile
   ret birlikte ve varlıklı ret olmaz. 214 ve öncesi alanı tanımaz. Bağımsız yazıcı ve okuyucu, örnek ve üç bozuk
   dosya. Çok yeni dosya artık `schema-version-216`.
7. **Masaüstü:**
   - Detay eşleştirme Eşleştir'de zamanı verir. Durum çubuğu söyler: “5 varlık yazıldı, 11 aday işaretlendi; 2 karar
     1–2 numaralı inceleme notları olarak kaydedildi.”;
   - İnceleme notları kararları listeler. Seçili notun ayrıntısı kaydı söyler: “Kayıt: Detay eşleştirme kararı · kaynak
     Ortofoto · aday ve yazılan varlık bağlı”.
8. **Kapsam işareti:** **Harita + CBS** (inceleme notları ortak, Detay eşleştirme Harita'nın).

## Sınırlar

- Komutun sürümü 1 kaldı. Girdiye `reviewedAt`, çıktıya `notes` eklendi; varlığın `İnceleme` ve `Gözden geçiren`
  öznitelikleri artık yazılmıyor. Eski çizimlerdeki bu öznitelikler yerinde kalır ve kopyalanmaz.
- Kayıtlı kararlar sonraki çalıştırmada kendiliğinden yeniden uygulanmaz. Adayın işareti onu okumamaya yeter; katalog ya
  da model değiştiğinde kararın yeniden uygulanması ayrı bir karardır.
- Yaşam döngüsü evreleri yok: not çözülmüş yazılır. Açık, Yanıtlandı ve Çözüldü ADR 1000'in durumlarıdır.

## Kanıt

- **Sözleşme:** `review_notes.rs`'nin birim testi; kararın söyledikleri, bağları, kabul ile ret, varlıklı ret.
- **`.kcad`:** `scripts/fixtures/kcad_v2_reference.py` ile `tools/kcad/kcad.py` (bağımsız yazıcı ve okuyucu) 1727
  dosyada uyuşur; `kentos-kcad`'in fixture testi (246 geçerli dosya) geçer.
- **Bağımsız başvuru** (`scripts/fixtures/map_features_reconcile_cases.py`; KentOS kodu yok), 44 durum:
  - kararlı durumların notları (`$any` kimlik ve damgayla), çıktının `notes`'u, varlıkta `İnceleme` yok;
  - çizimin 3 numaralı notunun ardından 4–7 numaralı dört not, kırpılmış gözden geçiren ve zamanla;
  - boş alan değeri not yazmaz;
  - beş yeni ret.

  Rust ve Python geçer.
- **Python** (`python/tests/test_reconcile_review_notes.py`):
  - dört karar 4–7 numaralı çözülmüş notlar, kararlar verildiği gibi, ret yalnız adaya bağlı;
  - varlıkta öznitelik yok;
  - dosya bağımsız okuyucuyla şema 215;
  - değişen varlığın notu eskir;
  - tek geri alma notları da götürür.
- **MCP** (`crates/native/mcp/tests/reconcile_review_notes.rs`): `cad.review.notes.list` dört kararı geri verir;
  65 karakterlik zaman `reviewedAt` yoluyla reddedilir.
- **Masaüstü** (`features_reconcile_window_tests::eşleştir_records_the_decisions_as_review_notes`):
  - Kabul ve Düzelt'ten sonra Eşleştir iki çözülmüş not yazar, gözden geçirenle ve zamanla;
  - İnceleme notları onları güncel listeler, kayıt satırını söyler;
  - geri alma notları da götürür.

  Detay eşleştirme ve inceleme testleri ile çapraz denetimler geçer. Resimler `.run/gui-evidence/eslestirme-kaydi-4140/`.

## Sonra

- Model değişince kayıtlı kararların yeniden uygulanması: [ADR 4540](4540-reapply-recorded-reconcile-decisions.md).
  Katalog değişikliğinin geçiş raporu (ADR 1760 Sonra'nın kalanı).
- AI çıkarımı; gerçek çıkarımın adaylarıyla ölçüm (MAP-04'ün kalanı, dış kaynak bekler).
- Web (TODOS §0.2).
