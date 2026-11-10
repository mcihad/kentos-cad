# 4540 — Detay eşleştirmede kayıtlı kararların yeni çıkarıma yeniden uygulanması

Tarih: 2026-11-10. plan/map-cad.md MAP-04, dokuzuncu tur. Durum: taslak. ADR 4140'ın “Sonra”sındaki “model değişince
kayıtlı kararların yeniden uygulanması”.

İlgili:
- [ADR 1760](1760-asset-candidate-reconciliation.md): Detay eşleştirme (`map.features.reconcile`), iki adayın bir grup
  olduğu kural (nokta yarıçapla, çizgi Hausdorff uzaklığıyla);
- [ADR 2140](2140-area-candidates-by-overlap.md): alanların örtüşme oranıyla gruplanması;
- [ADR 4140](4140-reconcile-decisions-as-review-notes.md): kararların adaya ve varlığa bağlı çözülmüş inceleme notu olarak
  kaydı.

## Dayanak

- **ArcGIS Pro**, [Error results and their life cycle](https://doc.esri.com/en/arcgis-pro/latest/help/data/validating-data/results-and-life-cycle-phases.html)
  (10 Kasım 2026'da okundu):
  - evreler İnceleme, Düzeltme ve Doğrulamadır; Düzeltme'de sonuç ya nesne düzeltilerek giderilir ya da istisna olarak
    işaretlenir;
  - istisna, otomatik bir denetimin ya da görsel incelemenin hata diye işaretlediği, önce hata sanılan nesnedir;
  - bir sonucu istisna olarak işaretlemek, nesnenin gelecekte yeni bir hata olarak işaretlenmemesini sağlar: gözden
    geçirenin kararı bir sonraki çalıştırmada da geçerlidir.
- **KentOS bugün** (ADR 4140): kararlar adaya bağlı inceleme notudur. Ama model yeniden çalışınca adaylar yeni
  nesnelerdir (yeni kimlik): aynı direk yine ağaç okunur, aynı gölge yine aday olur, kapının numarası yine eksiktir;
  gözden geçiren aynı kararları yeniden verir.
- ArcGIS kararı aynı nesnede tutar; yeni çıkarımda aynı nesne yoktur. Kararın yeni adaya **konum yakınlığıyla**
  taşınması KentOS'un kararıdır. Yeni bir eşik yoktur: komutun kendi gruplama kuralı (ADR 1760, 2140) ve kullanıcının
  verdiği yarıçap ile örtüşme oranı kullanılır.

## Karar

1. **`map.features.reconcile`'e `reapply`** (sürüm aynı, ek alan). Açıkken, bu çalıştırmada kararı verilmemiş ve daha
   önce eşleştirilmemiş her aday kayıtlı bir kararı alabilir:
   - kayıtlı karar, bu komutun çözülmüş bir inceleme notundaki karardır (`decision.command` =
     `map.features.reconcile`); profilde olmayan bir sınıfı adlandıran karar alınmaz;
   - kararın adayı çizimde, aynı kaynağın (adıyla) katmanında, eşleştirilmiş (işaretli) ve aynı türde (nokta, çizgi,
     alan) olmalıdır;
   - yakınlık, iki adayın bir grup olduğu kuraldır: noktalar yarıçap içinde, çizgilerin Hausdorff uzaklığı yarıçap
     içinde, alanların örtüşme oranı en küçük oranın üstünde;
   - birden çok karar uyarsa en yakını (alanlarda en büyük örtüşme), eşitlikte numarası küçük not;
   - alınan karar (kabul, sınıf, alanlar, ret) bu çalıştırmanın kararlarına eklenir; çalıştırmada verilen karar önce
     gelir.
2. **Çıktı `reapplied`:** her adayın aldığı notun numarası. Eşleştir'in yazdığı yeni notun metni “Önceki karar (not N)
   yeniden uygulandı.” ile biter; not, ADR 4140'taki gibi yeni adaya (ve varlığına) bağlıdır.
3. **Masaüstü** (Detay eşleştirme):
   - formun son satırı **Önceki kararlar**: “Yakındaki yeni adaylara yeniden uygula” anahtarı;
   - özet: “N adaya önceki karar yeniden uygulandı (inceleme notu …).”;
   - anahtar oturumundur, projeye yazılmaz.
4. **Kapsam işareti:** ADR 1760 gibi **Harita**.

## Sınırlar

- Konum yakınlığı aynılığın kanıtı değildir: yeni aday, eski adayın yanında başka bir nesne olabilir. Alınan her karar
  planda ve özette görünür, Eşleştir'den sonra notta yazılıdır ve tek adımda geri alınır.
- Eski adayın karardan sonra değişip değişmediğine (notun bayatlığına) bakılmaz.
- Yeniden uygulanan kararın notu da kayıtlı bir karardır; sonraki çıkarım onu da alabilir.
- Katalog (profil) değişikliğinin geçiş raporu yoktur (ayrı iş).

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/map_features_reconcile_cases.py`; KentOS kodu yok, alanlar shapely ile):
  kararlı çalıştırmanın ardından çizim, dosyanın tutacağı biçimde (adaylar işaretli, yazılan varlıklar sınıf
  katmanlarında, dört not adaylarına ve varlıklarına bağlı) ve yeni çıkarımın beş noktası; 4 yeni durum (48):
  - plan: dört yakın aday notları 2, 4, 3 ve 1'i alır; ağaç yine direğe düzeltilir ve yazılmış direk olur (Zaten var),
    bilinmeyen yine reddedilir, kapı numarasını alır, rögar kabul edilir; yarıçapın dışındaki aday karar almaz;
  - eşleştir: dört aday işaretlenir; dört yeni not (5–8) önceki notu anar, adaya ve varlığa bağlıdır; tek geri alma;
  - yeniden uygulama istenmeden: ağaç ağaç kalır, kapının numarası eksiktir;
  - çalıştırmada verilen karar kayıtlı karardan önce gelir.

  Eski 44 durum aynı. Rust ve Python geçer. Bozma denemesi: yarıçap on katına çıkınca uzaktaki aday da reddi alır,
  koşucu yakalar.
- **Python** (`python/tests/test_reapply_recorded_decisions.py`), testin kendi kuralıyla (düz uzaklıkla aynı kaynağın en
  yakın eşleştirilmiş adayının notu): alınan kararlar; ret, sınıf ve alanların etkisi; verilen kararın önceliği;
  yeni notların sözü; tek geri alma.
- **MCP** (`crates/native/mcp/tests/reapply_recorded_decisions.rs`): `reapply` olmadan karar alınmaz; ile notlar 2, 4,
  3, 1; uzaktaki aday almaz; yeni notlar 5–8 önceki notu anar.
- **Masaüstü** (`features_reconcile_reapply_tests`): anahtar kapalıyken ağaç ağaç, bilinmeyen okunmaz; açıkken dört
  karar, direk Zaten var, kapının alanı eksik değil; Eşleştir'in dört notunun sözü; tek geri alma. Pencere testleri ve
  çapraz denetimler geçer. Resimler `.run/gui-evidence/onceki-kararlar-4540/`.

## Sonra

- Katalog (profil) değişikliğinin geçiş raporu (ADR 1760 Sonra'nın kalanı).
- AI çıkarımı; gerçek çıkarımın adaylarıyla ölçüm (dış kaynak bekler).
- Web (TODOS §0.2).
