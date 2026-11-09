# 4240 — Sembolü karardan ver

Tarih: 2026-11-09. plan/map-cad.md MAP-09, sekizinci tur. Durum: taslak. ADR 1420'nin ve 3840'ın “Sonra”sındaki
“MPYY sembolünün karar kodundan seçilmesi (sembol değişikliği kararı değiştirmez)”.

İlgili:
- [ADR 1420](1420-zoning-plan-typed-check.md): İmar planı denetimi; `imar-plani` profili, karar tipli değerlerdedir;
- [ADR 3840](3840-zoning-check-in-processing-and-error-layer.md): imar denetimi İşlem araç kutusunda;
- docs/STYLE.md §8: MPYY sistem kitaplığı (yönetmelik eklerinin gösterimleri).

## Dayanak

- **QGIS, Categorized Renderer**
  ([Vector Properties](https://docs.qgis.org/3.40/en/docs/user_manual/working_with_vector/vector_properties.html),
  9 Kasım 2026'da okundu):
  - sembolün görünüşü bir alanın ya da ifadenin ayrık değerlerini yansıtır;
  - sınıflar değerlerden kurulur, her nesne sınıfının sembolüyle çizilir;
  - hiçbir sınıfa düşmeyen nesneler varsayılan “all other values” sınıfına düşer.

  Yön veriden sembole doğrudur; sembolden veri okunmaz.
- **MPYY sistem kitaplığı** (docs/STYLE.md §8): yönetmelik eklerinin lejantları kademe kademe sembol olarak vardır.
  Uygulama imar planının 208, nazım imar planının 148 alan (dolgu) sembolü vardır.
- **KentOS bugün:**
  - `imar-plani` profilinin karar alanı kararı tipli değerlerde tutar (Kullanım: 17 kod; ADR 1420).
  - Profilin kullanım kodları MPYY lejantlarından kabadır. Örneğin MPYY'de Yerleşik ve Gelişme konut alanı vardır,
    profilde tek Konut alanı.
  - Karar ile sembol arasında bağ yoktu; sembol elle veriliyordu ve kararla çelişebiliyordu.

## Karar

1. **KentOS'un tablosu** (`catalog/v1/profiles/imar-plani-semboller.json`, `scripts/cad/zoning_symbols.py`'nin
   kitaplıktan yazdığı veri):
   - Kademe (uip: MPYY › Uygulama imar planı, nip: MPYY › Nazım imar planı) başına, kullanımın adıyla aynı yazılan tek
     alan sembolü. Aynı yazım: Türkçe küçük harf, “î” “i” sayılır, ad alan lejantın sondaki “ (…)”'su atılır.
   - Birebir eşi olmayan kullanım açık kalır: UİP'de 8 kullanım eşlenir (ticaret, sağlık, sosyal, kültür, resmî
     kurum, teknik altyapı, park, meydan), NİP'te 5 (sanayi, sosyal, kültür, spor, teknik altyapı). KentOS tahmin
     etmez.
   - Tablo kademenin seçilebilir alan sembollerini de taşır.
2. **`cad.zoning.symbolize` v1** (“Sembolü karardan ver”):
   - **Kapsam:** karar alanları (`uids`, yoksa profilin karar katmanının bütün alanları).
   - **Plan ve kademe:** alanın `Plan`'ı plan sınırını adlandırır; adlandırmıyorsa ve tek plan sınırı varsa o plandır.
     Kademe planın `Seviye`'sidir.
   - **Sembol:** kullanıcının seçimi (`symbols`: kademe, kullanım, sembol) önce gelir, yoksa KentOS'un tablosu.
   - **Durum,** bu sırayla: `noPlan`, `noLevel`, `noUse`, `unknownUse`; sonra sembolü yoksa `unmapped`, zaten oysa
     `same`, değilse `set` (önceki sembol `was`).
   - **Yazılan:** yalnız nesnenin kitaplık sembolü (`symbol`), tek geri alma adımı “Sembolü karardan ver”.
   - **Karar hiç yazılmaz.** Sembolü “Park” olup kararı ticaret olan alan Ticaret alanı sembolünü alır; sembolden karar
     okunmaz. Sonra değişen sembol de kararı değiştirmez.
3. **Retler**, sırasıyla:
   - `invalid_revision`, `revision_conflict`;
   - `invalid_input`: seçimin kademesinin sembolleri yok, kullanımı listede değil, aynı kademe ve kullanım iki kez;
   - yeni `unknown_plan_symbol`: seçimin sembolü kademenin alan sembollerinden değil; çizgi sembolü ya da başka kademenin
     sembolü olabilir;
   - `layer_not_found`: karar katmanı yok;
   - `uids[i]`: `invalid_input` (iki kez), `invalid_uid`, `entity_not_found`, yeni `not_a_decision` (karar
     katmanında alan değil);
   - `layer_locked`: sembolü değişecek alan kilitli katmanda; hiçbir şey yazılmaz.
4. **Masaüstü:** İmar planı denetiminde **Sembolleri karardan ver…** penceresi açar.
   - Tablo, çizimin karar alanlarının her kademe ve kullanımı için bir satırdır: Kademe, Kullanım, Sembol (seçilmedi),
     Kaynak (KentOS / Seçildi), Alan, Verilecek. Kitin veri tablosudur.
   - Basılan satırın sembolü “Seçili satır” bölümündeki listeden, kademenin alan sembollerinden seçilir.
   - Özet satırı sayıları söyler: verilen, zaten öyle olan, seçilmemiş ve atlanan alanlar.
   - **Sembolleri ver** kendi bölümündedir; pencere kapanır, ileti kararların değişmediğini söyler.
5. **MCP:** komut sembollerin üstüne yazdığı için yıkıcı listesindedir.
6. **Kapsam işareti:** **Harita + CBS**, imar denetimi gibi. Bakanlığın MPYY/PlanGML şemasına ya da mevzuata uygunluk
   iddiası taşımaz.

## Sınırlar

- Kullanıcının seçimi projede saklanmaz: her çalıştırmada verilir. Pencere, açıkken seçimleri tutar.
- Çevre düzeni ve mekânsal strateji planı kademelerinin tablosu yok (açık kalır).
- İmar denetimi sembol ile kararın uyuşmazlığını bulgu olarak söylemez; komutun cevabı önceki sembolü (`was`) verir.
- Yapı nizamı ve yoğunluk gösterimleri (TAKS, KAKS …) karar alanına konmaz.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/zoning_symbols_cases.py`; KentOS kodu yok), 26 durum:
  - tablo kitaplığın öğelerinden ad eşitliğiyle betikte yeniden kurulur, katalogdakiyle karşılaştırılır;
  - durumlar:
    - bütün alanlar;
    - kullanıcının konut seçimi; tablonun önüne geçen park seçimi;
    - seçili alanlar; çizim sırası;
    - zaten doğru sembol (yazılmaz);
    - NİP kademesi; çevre düzeni planı;
    - tek plan sınırlı çizim;
    - atlananlar; plan ve doğrulama;
    - 14 ret.

  Rust ve Python geçer; yetenekler testi her kodu sayar.
- **Python** (`python/tests/test_zoning_symbols.py`), testin kendi kitaplık okumasıyla:
  - karar sembolü verir: park, yanlış sembollü ticaret, açık konut;
  - sonra değişen sembol hiçbir özniteliği değiştirmez;
  - tek geri alma.
- **MCP** (`crates/native/mcp/tests/zoning_symbols.rs`): araç yıkıcı işaretli; tablo, kullanıcının seçimi, kararlar
  değişmez, geri alma. MCP'nin ana testleri geçer.
- **Masaüstü** (`zoning_symbols_window_tests`, 2 test):
  - satırlar ve sayılar ortak durumunkiler;
  - konut seçimi ortak durumun cevabını verir, yazılan semboller durumunkiler, öznitelikler aynı, tek geri alma.

  İmar denetimi, İşlem aracı ve çapraz denetimler geçer. Resimler `.run/gui-evidence/sembol-karar-4240/`.

## Sonra

- Kullanıcının kademe ve kullanım seçimlerinin projede saklanması; imar denetiminde sembol ile karar uyuşmazlığı.
- Plan değişikliği taslağı; hata katmanının GeoPackage'a yazılması; resmî şema ve eşleme (MAP-09'un kalanı).
- Web (TODOS §0.2).
