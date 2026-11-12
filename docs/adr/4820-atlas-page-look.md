# 4820 — Atlas sayfasının görünüşü: değişen sayfanın çizdiği de değişti mi

Tarih: 2026-11-10. plan/map-cad.md MAP-18, dokuzuncu tur. Durum: taslak. ADR 4000 ve 4400'ün “Sonra”sındaki “atlas
sayfalarının özetiyle görünüşü değişti ayrımı”. Maddenin sıradaki işinin ikinci parçasıdır. Birinci parça, total
station ve nivelman kalitesi (önce dengeleme doğruluklarının projede saklanması), projenin kaydına yeni bir bölüm (`.kcad`
şeması) ister; açık kalır (Sonra).

İlgili:
- [ADR 1320](1320-sheet-sources-and-update-publication.md): pafta yayınında aynı çizen sayfa baytı baytına kopyalanır;
- [ADR 4000](4000-atlas-update-from-its-record.md): atlasın kaynak kaydı, “değişti” kaynakların değişmesidir;
- [ADR 4400](4400-atlas-update-pdf-of-changed-pages.md): yalnız değişen sayfaların güncelleme PDF'i.

## Dayanak

- **QGIS QgsRenderChecker** ([API](https://api.qgis.org/api/classQgsRenderChecker.html), 10 Kasım 2026'da okundu): çizilen
  resmi beklenenle karşılaştırır (farklı piksel sayısı, renk payı). QGIS atlası ve ArcGIS harita serisi değişen sayfayı
  söylemez (ADR 3600, 4000).
- **diff-pdf** ([kaynak](https://github.com/vslavik/diff-pdf)): iki PDF'i sayfa sayfa çizerek karşılaştırır.
- **BÖHHBÜY m. 76** (araştırma notu `.run/research/map-cad/mevzuat/MAP-18.9.md`): paftayı idarenin kontrol mühendisi
  imzalar, yetkilisi onaylar; imza çizilmiş sayfaya aittir. Görünüşü değişen imzalı sayfanın ne olacağını yönetmelik
  yazmaz; KentOS değişikliği söyler, onay kararı vermez.
- **KentOS bugün:**
  - ADR 4000: değişti = sayfanın kaynaklarından birinin içerik özeti değişti. Çizilmeyen bir öznitelik (formun gizlediği
    malik) değişince de sayfa “değişti”.
  - PDF'in sayfa akışı, yazı tipinin alt kümesi bütün belgeden kurulduğu için başka sayfanın yazısıyla değişir. Sayfa
    baytları görünüşün kanıtı olamaz.

## Karar

1. **Sayfanın görünüşü (`look`):** sayfalar yapılırken (plan ve execute PDF'i kurar) her sayfa için SHA-256. Özetin
   içeriği:
   - paftanın o sayfadaki görüntü listesi, yayının kendi damgaları dışarıda: gün, kullanıcı ve KentOS'un boş `kaynak`
     değişkenine yazdığı çizim durumu notu. Kullanıcının kendi `kaynak` metni içeride kalır;
   - sayfanın haritalarının yolları ve yazıları;
   - tabloları, koordinat listeleri, lejantları, haritaların kalite notları;
   - her sayfada duran haritalar.

   Taşınmayan öznitelik (formun `redact`'ı) zaten çizilmez. Sayfa baytları ve yazı tipi alt kümesi özete girmez.
2. **Kayıt:** `pages[].look` cevapta ve `<ad>.kaynaklar.json`'da. Güncelleme PDF'inin (ADR 4400) yapmadığı sayfa
   önceki kaydın görünüşünü taşır: kaynakları değişmemiştir.
3. **Değişikliklerde:** önceki kayıt sayfanın görünüşünü biliyor ve bu çalışma sayfayı yaptıysa her `changed` sayfanın
   `changes.pages[].look`'u:
   - `same`: yalnız çizmediği değişti;
   - `changed`: çizdiği değişti.

   Görünüş kaydı olmayan eski kayıtta `look` yazılmaz. Kural ADR 4000'in “değişti”sini değiştirmez: sayfa yine kaynağıyla
   değişir, `look` nedenini ayırır.
4. **Masaüstü** (Atlası PDF yap): yazıldı iletisi “önceki atlasa göre değişen sayfalar: görünüşü değişen n, görünüşü aynı
   kalan m (yalnız çizmedikleri değişti)” der. Önizleme sayfaları çizmez; görünüş yazılırken bulunur.
5. **Kapsam işareti:** ADR 4000'deki gibi **CAD + CBS** (pafta).

## Sınırlar

- Görünüş yayının damgalarını ayırır: aynı içerik başka gün yazılınca da `same`'dir, oysa basılan gün değişir. Damga
  olarak yalnız gün, kullanıcı ve KentOS'un çizim durumu notu ayrılır.
- Özet sayısal bir karşılaştırmadır, piksel farkı değildir: görünmeyecek kadar küçük bir koordinat değişimi de
  `changed`'dir. Renk ya da piksel payı yoktur.
- Önizleme (masaüstünün planı) sayfaları çizmez; görünüş PDF yazılırken ya da başsız plan PDF'i kurarken bulunur.
- Güncelleme PDF'i görünüşü aynı sayfaları da yazar: seçim kaynağa göredir (ADR 4400).
- Total station ve nivelman kalitesi bu dilimde yok.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/atlas_cases.py`; KentOS kodu yok), 3 yeni durum (47). Kural paftanın
  kendisinden: nesne notu sayfanın parselinin Ada, Parsel ve Durum'unu basar, malik taşınmaz, atlas haritası
  etiketsizdir.
  - malik değişince parseli listeleyen her sayfa `changed`, görünüşü `same`;
  - adası değişince yalnız kendi sayfası `changed` görünüşlü, komşularınınki `same`;
  - planda da aynı.

  Özetin değeri `$any` değildir, durumda yazılmaz: yalnız ayrım denetlenir. Eski 44 durum bayt bayt aynı (bu Mac'in
  PROJ'unun oynattığı iki durum dahil, ADR 4000). Bozma denemesi: bir `same` `changed` yazılınca koşucu yakalar.
- **Python** (`python/tests/test_atlas_look.py`), testin kendi okumasıyla:
  - kayıtta her sayfanın 64 hanelik görünüşü;
  - aynı çizim iki kez aynı görünüşler;
  - malikte değişen sayfaların hepsi `same`;
  - adada yalnız parselin kendi sayfası `changed`.

  ADR 4000'in Python testi geçer.
- **MCP** (`crates/native/mcp/tests/atlas_look.rs`): ajandan gizlenen malik yerine taşınmayan `Sır` ile hepsi `same`;
  `Ada` ile yalnız kendi sayfası `changed`. Atlasın öbür MCP testleri geçer.
- **Masaüstü** (`exchange::atlas_look_tests`): ikinci yazışın iletisi “görünüşü değişen 0, görünüşü aynı kalan 6”, kayıtta
  her sayfanın görünüşü. Atlas pencere testleri geçer. Resimler `.run/gui-evidence/atlas-gorunus-4820/` (önizleme ve ileti).

## Sonra

- Total station ve nivelman kalitesi: yatay ağ, kot ağı ve poligon dengelemelerinin doğrulukları ve kapanmaları projenin
  kaydında (yeni `.kcad` bölümü), paftanın kalite notunda; BÖHHBÜY m. 28, 38–40 sınırları kural paketi olarak.
- Önizlemede görünüş (değişen sayfaları çizerek); güncelleme PDF'inde görünüşü aynı sayfaları dışarıda bırakma seçeneği.
- Web (TODOS §0.2).
