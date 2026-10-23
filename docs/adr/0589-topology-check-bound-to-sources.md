# ADR 0589: Kaynağa bağlı topoloji denetimi, istisnanın geometri bağı ve düzeltmenin önizlemesi

- **Durum:** kabul edildi (2026-10-22). `plan/gis.md` CBS-14: topoloji bulgularını artımlı ve kaynak revizyonuna bağlı izle.
- **Bağlam belgesi:**
  - ADR 0202: topoloji kuralları, bulgular, düzeltmeler ve istisnalar (MEVCUT-02; değişmez);
  - ADR 0524: bulgunun kalıcı anahtarı ve artımlı denetim;
  - ADR 0510: bağlı katmanın alan sınırlı parçası;
  - ADR 0578: parçanın bandı;
  - ADR 0585: alanın dışındaki komşular.
- **Web:** ertelendi (ADR 0228), TODOS.md §0.2.
- **Sonraya kalan:**
  - yazmanın denetimin damgasını sorması (“bu yazma şu denetimden geçti”): damga cevapta var, `cad.sources.sync` onu
    henüz istemez;
  - kaynaklar arası kuralda (başka katman) bandın komşuları: bugün yalnız kuralın kendi katmanlarının bandı okunur;
  - dışarıdaki satırı değiştiren düzeltme: bugün önizlenmez, yapılmaz (§4).

## Bağlam

ADR 0524'ten sonra masaüstünün Topoloji sekmesi bulguları kalıcı anahtarlarla tutuyor ve yalnız değişeni yeniden
denetliyordu. Üç eksik vardı:

1. **Bağlı parçanın komşuları.** Alan sınırlı bir parçanın nesneleri yalnız birbirleriyle denetleniyordu. Alanın hemen
   dışındaki satırla çakışma görünmüyordu ve parça “temiz” sayılıyordu.
2. **İstisna geometriye bağlı değildi.** İstisna kural, nesneler ve yerle tutuluyordu. Nesneler değişse de istisna
   bulguyu gizlemeye devam ediyordu: yeni bir hata eski kararın arkasında kalabiliyordu.
3. **Düzeltmenin etkisi bilinmiyordu.** Düzelt ▾ yalnız düzeltmenin adını söylüyordu; neyi ne kadar değiştireceği,
   hangi kaynağa yazılacağı görünmüyordu.

Ayrıca denetim yalnız masaüstünün sekmesindeydi. Python ve ajan aynı denetimi komutla yapamıyordu.

## Karar

### 1. `cad.topology.check`: salt okunur denetim

Projenin kurallarını çizim üzerinde çalıştırır. Girdi:

- `rules`: yalnız bu kimlikli kurallar (yoksa hepsi); bilinmeyen kimlik `topology_rule_not_found`;
- `limit`: listenin sınırı (varsayılan 1000, en çok 100 000; sayılar hep bütün bulgularındır);
- `preview`: bir bulgunun bir düzeltmesinin önizlemesi (§4).

Denetlenen nesneler kuralların andığı katmanlarınkidir, çizimdeki sırayla. Bağlı bir katmanın parçası varsa:

- kaynağın, alanı kısa kenarının %15'i kadar büyüyen bandındaki (ADR 0578) satırlardan katmanın tutmadığı her biri komşu
  olarak eklenir. Dosya, katmanın son eşleştiği sürümden okunur; veritabanı bugünkü hâliyle;
- yalnız dışarıdaki satırlar arasındaki bulgu parçanın değildir, bırakılır. Parçanın nesnesiyle dışarıdaki satır
  arasındaki bulgu verilir, dışarıdaki satırın kimliği `outside`'da;
- parçanın bir nesnesi bandın dışına uzanıyorsa, ya da band okunamıyorsa (dosya yok, dosya değişti, veritabanı cevap
  vermedi) denetim **eksiktir**: `complete` yanlış, `incomplete` katmanı ve nedeni söyler, `topology_incomplete` uyarısı.
  Eksik denetim bulgusuz olsa da “temiz” sayılmaz.

Her bulgu ADR 0524'ün kalıcı anahtarıyla, kuralıyla, türüyle, sorunuyla, nesnelerinin kalıcı kimlikleriyle, yeriyle,
ölçüsüyle ve sunduğu düzeltmelerle verilir.

Cevabın **damgası** denetimin gördüğü durumdur:

- `revision`: çizimin sürümü;
- `rules`: kuralların, toleransın ve istisnaların kanonik JSON'unun SHA-256'sının ilk 8 baytı;
- `sources`: her bağlı katmanın eşleştiği kaynak sürümü ve okunan bandın özeti.

Bunlardan biri değişince eski denetimin sonucu geçersizdir.

### 2. Masaüstü aynı çekirdekle

Topoloji sekmesi Denetle'yi ve Değişenleri denetle'yi aynı toplayıcıyla yapar (`topology_check::gather`, `run`). Bağlı
parçanın dışarıdaki satırı Nesneler sütununda “dışarıda” yazar. Eksik denetim sekmenin üstünde uyarı şeridiyle ve
günlükte söylenir; bulgu yoksa “Bulgu yok ama denetim eksik” der.

### 3. İstisnanın geometri bağı (`.kcad` şema 99)

İstisna artık nesnelerinin geometrisinin özetini taşır: `digest`. Özet, bulgunun nesne sırasıyla her şeklin kanonik
JSON'unun SHA-256'sının ilk 8 baytıdır, 16 küçük onaltılık hane.

- Özet tutan istisna bulguyu istisna yapar.
- Nesnelerden biri değişince istisna **düşer**: bulgu açık olur, `lapsed` işaretlidir, `exception_lapsed` uyarısı
  söylenir. Sekmede sorun “(istisna düştü)” diye yazılır.
- Özeti olmayan istisna (şema 98 ve öncesi) eskisi gibi kural, nesneler ve yerle tutar, düşmez.

`cad.topology.exceptions` istisnayı komutla yazar:

- `mark`: verilen anahtarlı bulgular bugünkü geometrinin özetiyle istisna olur. Düşmüş istisna yerine yenisi yazılır.
  Zaten istisna olan değişmez;
- `unmark`: istisnası olan ya da düşmüş bulgunun istisnası kalkar;
- anahtarlar `cad.topology.check`'in bugünkü anahtarlarıdır. Biri yoksa hiçbiri yazılmaz (`finding_not_found`);
- istisnalar projenin ayarıdır: çizimi kirletir, geri alma adımı değildir. `expectedRevision` tutmazsa
  `revision_conflict`.

Sekmenin İstisna yap ve İstisnayı kaldır düğmeleri bu komutla yazar, ardından denetim sessizce yinelenir.

### 4. Düzeltmenin önizlemesi

`preview` bir bulgunun bir düzeltmesinin ne yapacağını söyler, çizime dokunmadan:

- değişen her nesnenin kimliği ve katmanı; alanın önce ve sonraki alanı ya da çizginin uzunluğu; silinen nesne;
- dokunulan bağlı katmanların tablosu ve yazılacak satır sayısı.

Bulgu düzeltmeyi sunmuyorsa ya da düzeltme alanın dışındaki bir satırı değiştirecekse `fix_unavailable`. Masaüstünde
Düzelt ▾ her düzeltmenin altında önizlemesini ikinci satırda gösterir; yapılamayan düzeltme kapalıdır ve nedenini söyler.

## Doğrulama

- `scripts/fixtures/topology_check_cases.py`, KentOS kodu olmadan:
  - bulgular topology_rules_cases.py'nin başvurusundan (ADR 0202);
  - anahtarlar ADR 0524'ün FNV-1a kuralıyla Python'da yeniden hesaplanır;
  - bant ve dışarıdaki satırlar ADR 0578'in kuralından; parçanın dosyasını (`topoloji-parca.gpkg`, değişen sürümü
    `topoloji-parca-2.gpkg`) GDAL'ın OGR'ı yazar;
  - `cad.topology.check` için 23, `cad.topology.exceptions` için 21 ortak durum
    (`fixtures/commands/v1/cad.topology.{check,exceptions}.json`).
- `.kcad` şema 99: kodek, `tools/kcad/kcad.py` ve bağımsız yazıcı (`kcad_v2_reference.py`) `topology-digests.kcad`'i ve
  üç bozuk örneği (şema 98'de özet, büyük harf, kısa) aynı okur.
- Masaüstünde `topology::tests`: dışarıdaki komşu ve önizleme, banttan taşan parça, düşen istisna. Resimler
  `.run/shots/topoloji-{duzelt,dustu,parca,parca-duzelt,eksik}-*`.
- ADR 0524'ün artımlı denetim testleri (tam denetimle aynı bulgular) değişmeden geçer.
