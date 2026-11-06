# 1262 — Topolojik temizliğin taşınma raporu (`cad.topology.clean`)

Tarih: 2026-11-06. plan/map-cad.md MAP-07 (alt madde: büyük toplu snap/temizliğin taşınma raporu). Durum: taslak.
İlgili: [ADR 0148](0148-topology-cleanup.md) (Topolojik temizliğin kuralları), [ADR 1260](1260-topology-quality-review-and-repair-recipe.md)
(kalite profili, onarım tarifinin taşınma raporu).

## Dayanak

- **AutoCAD Map 3D, Drawing Cleanup:** etkileşimli kipte bulunan her hata listelenir, tek tek ya da bir işin bütün
  hataları için düzelt, işaretle ya da atla seçilir; otomatik kipte sonuç komut satırında özetlenir; işaretler
  temizlikten sonra da çizimde kalabilir; ayarlar profil olarak saklanır
  ([To Clean Up Drawing Data](https://help.autodesk.com/cloudhelp/2026/ENU/MAP3D-Use/files/GUID-F0A6B62A-03BF-4839-BA51-A6E703049326.htm),
  [Drawing Cleanup Errors](https://help.autodesk.com/cloudhelp/2018/ENG/MAP3D-Use/files/GUID-49EEF7EA-F08B-450C-9C51-990991BC881D.htm),
  [Error Markers](https://help.autodesk.com/cloudhelp/2027/ENU/MAP3D-Use/files/GUID-1F33941D-D6A7-43E4-B6CF-D98D07B204D9.htm);
  6 Kasım 2026'da okundu).
- **ArcGIS Pro, Integrate:** küme toleransındaki köşeleri ortak kabul edip girdiyi yerinde değiştirir; istenmeyen
  taşınmayı azaltmak için tolerans küçük tutulmalı, girdinin kopyası üzerinde çalışılmalıdır; araç taşınmayı
  raporlamaz, doğrulama önce/sonra karşılaştırmayla yapılır
  ([Integrate](https://pro.arcgis.com/en/pro-app/latest/tool-reference/data-management/integrate.htm)).
- **KentOS bugün:** Topolojik temizlik aracı (ADR 0148) önizlemede sayıları ve yalnız en büyük kaymayı söyler; komutu
  yoktur (yazma `cad.entities.edit`'in `topology` işlemiyle), Python ve MCP temizliği çalıştıramaz, taşınmanın
  toplamı, nesne başına özeti ve kurumun kabul eşiğiyle karşılaştırması görünmez. MAP-07 bunu ister: “taşınan
  toplam/maksimum mesafe raporlansın”, büyük toplu temizlik ölçüm koordinatlarını sessiz yuvarlamasın.

## Karar

1. **`cad.topology.clean` v1** (Harita + CBS): ADR 0148'in temizliği komut olarak. Girdi: `uids` (yoksa görünen,
   kilitsiz katmandaki bütün çizgi, çoklu çizgi, yay ve alan), `tolerance` (en az 1 µm), dört iş (`ends`, `vertices`,
   `extend`, `trim`; verilmezse aracın ilk değerleri). Çekirdek ve nesne/geometri dönüşümü aracınkiyle aynıdır:
   `object`/`geometry_of` uygulama katmanına (`topology_clean.rs`) taşındı, araç oradan alır.
2. **Rapor** (plan ve yazmanın cevabı): her taşınma (nesne, iş, nereden, nereye, uzaklık); değişen her nesnenin
   katmanı, alanı (alan) ya da boyu (öbürleri) önce ve sonra, bit bit yerinde kalan kaynak köşeleri, taşınma sayısı,
   en uzak ve toplamı; bütün taşınmaların sayısı, en uzağı, toplamı; alanların toplamındaki değişim; düzeltilen,
   dayanak ve verilip düzeltilemeyen (`skipped`: kilitli, gizli ya da türü katılmayan) nesne sayıları. Köşe her zaman
   var olan bir köşeye gider (ADR 0148 §4): koordinat üretilmez, yuvarlanmaz.
3. **Kabul eşiği:** kalite profili (ADR 1260 §2) varsa eşik ve onu aşan taşınma sayısı (`beyond`) söylenir. Plan
   raporlamayı sürdürür; `execute` ve `validate` `clean_beyond_acceptance` ile bütün temizliği reddeder (onarım
   tarifinin `repair_beyond_acceptance`'ı gibi). Toleransı kabul eşiğine indirmek ya da köşeyi ölçüyle düzeltmek kullanıcıya
   kalır; eşik sessizce kırpılmaz.
4. **Pencere Taşınma raporu…** (`topology.cleanReport`, ikon `cleanReport`): Harita'da Plan › Ada ve parsel, CBS'de
   Veri › Topoloji (taşan liste) panelinde Topolojik temizliğin yanında; Yol ve Mimari sunmaz. Kapsam aracınki
   (seçim ya da bütün çizim), tolerans ve işler aracın hatırladığı değerlerle başlar. Form: Tolerans ve kapsam bir
   satırda, dört iş anahtarı yan yana (sahibin 6 Kasım isteği: form her zaman alt alta değil, düzgün ve dengeli).
   Özet kutusu ve kitin veri tablosu (ADR 0938: Nesne, Katman, Taşınan, En uzak, Toplam, Alan/boy farkı). **CSV
   kaydet** her taşınmayı `;` ile, Y ve X önce/sonra, uzaklık hesaplandığı gibi (yuvarlanmadan) yazar. **Uygula** tek
   geri alma adımı “Topolojik temizlik”; eşik aşılıyorsa kapalıdır, komut da reddeder.

## Kanıt

- 20 ortak durum, bağımsız `scripts/fixtures/topology_clean_cases.py`: temizlik ADR 0148'in bağımsız başvurusuyla
  (`topology_cases.py`), rapor elle (uzaklık hypot, alan rasyonel sayılarla shoelace: harita koordinatında kayan
  noktalı shoelace basamak kaybeder, yay boyu r·θ). Bütün çizim, Köşeler açık, yalnız seçim, skipped, hiçbir şey yok,
  işler kapalı, eşik içi/dışı plan, uygula ve geri al, eşik reddi (execute ve validate), tolerans, kimlik ve sürüm
  retleri. Rust ve Python geçer.
- Masaüstü `clean_report_window_tests.rs`: rapor ve tek adım, seçim ve Köşeler, eşikte reddedilen Uygula, CSV, Harita
  ve CBS sunar. Resimler `.run/gui-evidence/tasinma-raporu-1262/`. Aracın etkileşim izleri (`topology`) değişmeden geçer.

## Sonra

- Taşınmaları sahnede işaretli tutmak ve tablodan nesneye gitmek (AutoCAD Map'in işaretleri gibi), tek tek kabul/ret.
- Temizlik ayarlarını kalite profiline bağlamak; büyük veride (50 000 nesne) raporun ölçümü.
- Ortak sınırın bütün yetkili komşularda atomik düzenlenmesi (MAP-07'nin kalanı).
