# ADR 0689: Numarasız mahalın görünür denetimi

- Durum: bu denetim dilimi uygulandı ve doğrulandı (26 Ekim 2026).
- Kapsam: ARCH-SPACE-02'nin boş/yinelenen numara denetimi. Kat öneki,
  ad/kullanım listeleri ve genel mimari kabul matrisi ayrı açık işlerdir.

## Mevcut eksik ve karar

Etiketli fakat numarasız mahal, eski `cad.tags.check` içinde bulgu üretmez.
Beş etiketli mahalın üçü numarasız, ikisi aynı numaralı sentetik belge eski
gerçek pencerede yalnız bir yinelenen numara bulgusu verir. Ayrı bir kimlik
veya geometri sistemi kurmak yerine mevcut salt okunur denetim genişler.

`missingSpaceNumbers=true`, numarası olmayan her mahalı ayrı
`tag_missing_space_number` bulgusuyla raporlar. Mesaj mahal adı ve katını;
mevcut bulgu biçimi kalıcı UUID ve gösterilecek noktayı taşır. Gizli/kilitli
katmanlar dahil, yalnız açık `layers` süzgeci sınırlar. Kat atanmamış mahal
ayrıca belirtilir. Etiket varlığı numara varlığı sayılmaz. Boş değer, mevcut
mahal sözleşmesinde numara alanının yokluğudur; boş/yalnız boşluk metnin
reddi değiştirilmez.

Yok/null/false aynı eski bulguları ve sırayı verir. Var olan yinelenen
numaralar tam metin karşılaştırmasıdır: Z-001, Z-1, z-001 ve sondan boşluklu
metinler birleştirilmez; sayısal dönüşüm yapılmaz. Açıklık ve mahal numaraları
ayrı gruplardır. Gruplar ilk görünüm, üyeler belge sırasındadır; yeni eksik
numaralar yinelenen gruplardan sonra, diğer eski bulgulardan önce gelir.
Grup araması ilk sırasını koruyan indeks kullanır; belgeye indeks kaydetmez.

Masaüstü mevcut “Etiketleri denetle” penceresinde numarasız mahal seçeneğini
açık başlatır; görünür seçenek kapatılıp yeniden denetlenebilir. Plan,
validate, Python ve MCP aynı sorguyu kullanır. Otomatik numaralama/onarım,
undo, alan/birim/ölçek veya etiket değişikliği yapılmaz. Düzeltme mevcut
mahal özellikleri ve açık numaralandırma komutuyla gerçekleşir.

## Birincil kaynak karşılaştırması

[Revit LT 2026 Room Instance Properties](https://help.autodesk.com/cloudhelp/2026/ENU/RevitLT-ArchDes/files/GUID-21326970-0037-41C6-A996-980C24EE019F.htm)
numara, ad ve kullanımı ayrı kimlik alanları olarak tanımlar; aynı numara
için uyarı verir. [Revit Room Tags](https://help.autodesk.com/view/RVT/2025/ENU/?guid=GUID-6DB755B9-1830-4CCA-A358-511AC9D48FC6)
mahal ile etiketini ayrı fakat bağlı nesneler olarak açıklar. Bu karşılaştırma
eksik denetimin gerekçesidir; KentOS'un numaralandırma veya yerleştirilmemiş
mahal modelini bu ürünlerle eşitlemez. KentOS'un mevcut tam metin/UUID ve
numaralandırma çakışma kuralları korunur.

## Kabul kapıları

- Önceki 21 ortak denetim vakası aynen korunmalı. Etiketli, kat atanmamış,
  gizli ve kilitli mahaller, açık/boş katman listeleri ve retler; yok/null/
  false/true ile validate/plan/execute karşılaştırılmalı.
- Python/MCP aynı bulguları vermeli; sorgu modele/geçmişe yazmamalı.
  Özellikte numara ver/boşalt, tek undo/redo, gerçek KCAD kayıt-aç aynı
  kimlik ve geometriyi korumalı.
- Gerçek GUI eski/yeni, seçenek/yenile/Göster/kapat, numara düzeltme,
  tema/yazı/pencere boyları; ham ekran ve bağımsız tam kayıtlar incelenmeli.
- Derleme/lint, sözleşme ve SDK üretim sapması, bağımsız dosya başvuruları;
  yoğun belge sorgusunun doğruluk ve süre ölçümü gerekli.

Sonuçlar [kabul kaydında](../baseline/2026-10-26-architecture-space-identity/README.md):
233 ortak durum/323 Python, native ve üç gerçek MCP/iki desktop testi,
clippy/build/WASM/1170 bağımsız KCAD; 18 ham ekran ve 12 tam gerçek kayıt.
Tek Enter/boşalt/undo/redo/Escape/yeniden açma mevcut geometri/ayar/UUID'leri
korur. 10.000 mahal+20.000 çizgide debug sorgu ölçümleri ve sınırları kayıttadır.
ARCH-SPACE-02'nin kat öneki/ad/kullanım listeleri, Koru/Yenile yönetimi ve
genel tam mimari kabul matrisi açık kalır; ana madde tamamlandı sayılmaz.
