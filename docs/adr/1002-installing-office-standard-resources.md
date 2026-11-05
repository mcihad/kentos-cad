# ADR 1002: Ofis standardının kaynaklarını kullanıcı eşlemesiyle kurma

- **Durum:** kabul edildi (2026-11-05). Kapsam: plan/cad-advanced.md `CAD-ADV-12`'nin açık işi “Paket güncellemesi
  hangi kaynakları yeniden adlandıracak, birleştirecek veya değiştirecek gösterilsin. Aynı adlı fakat farklı
  içerikli kaynakta kullanıcı eşlemesi korunsun; kullanılan tanım purge edilmesin.” Sahibin sözü (5 Kasım 2026):
  “plan/cad-advanced.md maddelerini tamamlamanı istiyorum”. Maddenin tamamı değildir; kalan kapsam §7'dedir.
- **Bağlam belgesi:** [docs/specs/drawing-standard-package.md](../specs/drawing-standard-package.md) (sürüm 2 paket,
  `cad.standards.resolve/conflicts/check`), ADR 0312 (standart denetimi), ADR 0699 (stil tabloları Undo).

## 1. Araştırma

AutoCAD'in CAD Standards ve Layer Translator akışı bir standardı **denetler ve eşleme tablosuyla** düzeltir; eşlenmeyen
adlandırılmış kaynağı kendiliğinden ezmez ([AutoCAD 2027 — CAD Standards](https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-Core/files/GUID-D64F8076-4978-44B7-B056-D921C77FEA88.htm)).
DesignCenter/Content Browser ile aynı adlı tanım eklendiğinde çizimdeki tanım korunur, yenisi alınmaz; yeniden tanımlama
ayrı ve açık bir eylemdir. KentOS bu iki davranışı tek, önizlemeli komutta birleştirir.

## 2. `cad.standards.install` v1

Kaynak: projeye bağlı standartların `cad.standards.resolve` cevabı (ilk kaynak önceliği). Kapsam: çizgi tipi, tarama
deseni, tam yazı/ölçü stili tanımı ve kılavuz stili. Blok kitaplığı başvuruları ve pafta şablonu kimlikleri burada
kurulmaz (§7).

Her kaynak çizimdeki aynı adlıyla (harf büyüklüğü ve kenar boşluğu sayılmaz) karşılaştırılır; görünüş
`cad.standards.check`'in kuralıyla: ad, kimlik, açıklama ve kaynak alanı dışarıda; sayılar 1e-9 içinde eşit.
Durum `missing`, `same` ya da `differs` (`fields` farklı alanları adlar).

| Durum | Varsayılan | İzinli |
| --- | --- | --- |
| missing | add | add, keepLocal |
| same | keepLocal | keepLocal |
| differs | keepLocal | keepLocal, replace, renameIncoming |

- `replace`: çizimdeki tanımın yerine standarttaki; **çizimdeki ad ve yerel kimlik korunur**, onu adla ya da kimlikle
  kullanan katman/nesne izler.
- `renameIncoming`: standarttaki `newName` ile eklenir; ad çizimde ya da standartta o türde olmamalı (`duplicate_name`).
  Stil kendi kimliğini alır; çizimde o kimlik varsa `kimlik-2`, `kimlik-3` …
- Hiçbir kaynak silinmez, purge yapılmaz.

Denetim sırası: bağlı standart yok (`no_standard`), okunamıyor (`invalid_standard`), seçimin kaynağı yok
(`standard_resource_not_found`, `choices[i].name`), aynı kaynağa iki seçim (`invalid_install_choice`, `choices[i]`),
durumla uymayan işlem (`invalid_install_choice`, `choices[i].action`), yeni ad eksik/kenarı boşluklu ya da yalnız
renameIncoming dışında verilmiş (`invalid_install_choice`, `choices[i].newName`), yeni ad çakışıyor (`duplicate_name`),
beklenen revizyon (`invalid_revision`, `revision_conflict`). Satırlar türe, sonra standartların sırasına göredir.

## 3. Sayaçlar

`added`, `replaced`, `renamed` değişen satırları; `kept` çizimdeki olarak kalanları (aynılar dahil) sayar.

## 4. Undo

Yazı/ölçü/kılavuz stilleri mevcut `with_style_tables` (ADR 0699) ile, çizgi tipi ve tarama tabloları yeni
`Op::PatternTables` ile aynı adımda kaydedilir: tek Undo “Standart kaynaklarını kur” hepsini geri getirir. Proje
ayarlarının geri kalanı Undo dışında kalmayı sürdürür (ADR 0248).

## 5. Masaüstü

Çizim standardı penceresinde bağlı standart varsa **Kaynakları kur…** düğmesi açılır. Pencere kitin veri tablosunda
Tür, Ad, Durum (farklı alanlarıyla, uyarı renginde), İşlem (durumun izin verdiği parçalı seçim) ve Kaynak (standart
kimliği · revizyonu) gösterir; Yeni adla seçilince ad alanı çıkar. Tablo komutun planıdır; **Kur** komutu çalıştırır.

## 6. Doğrulama

`scripts/fixtures/standards_install_cases.py` KentOS kodu kullanmadan 25 ortak durum yazar: varsayılanlar, plan ve
doğrulamanın yazmaması, yerinde değiştirme (ad/kimlik korunur), yeni adla ekleme, kimlik çakışmasında `-2`, atlama,
hiç değişiklik yokken yazmama, Undo/Redo, ilk standart önceliği ve bütün retler. İlk koşuda 25/25 geçti. Rust testi
`no_standard` ve `invalid_standard` retlerini sınar. Gerçek pencerede kurulum önce/sonra görüntülendi (repo dışında).

## 7. Sınırlar (CAD-ADV-12 açık kalanı)

Blok kitaplığı yayınının ve pafta şablonu içeriğinin kurulması, tablo stili, kişisel palet dışında yayımlanan ofis
araç paleti, blok içi/viewport/plot override'ının tam görünüş kökeni ve iki ofis profilinin uçtan uca kabul senaryosu
bu ADR'de yok. Web dururken (ADR 0228) web karşılığı yazılmaz.
