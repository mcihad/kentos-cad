# ADR 0854 — Tanım değişimini geçmişte yok sayma

Tarih: 2026-11-02. CAD-ADV-06 arayüz kabulünde bulunan hata.

Gerçek **Başka çizimden yeniden tanımla** testinde tanımlar doğru değişiyor,
tek Undo ve kaydet–aç da geçiyordu; ancak işlem geçmişi **Çizimde değişiklik
yok** diyordu. Mevcut günlük biçimi blok kimliklerini nesne kimliği listelerine
ve `ProjectPart` listesine yazmıyor. Bu listelerin boşluğu, çizimin değişmediği
anlamına gelmez. Kaydedilmiş çağrının başlangıç/son revision'ı zaten ayrıdır.

Masaüstü özetinde ayrıntılı nesne/proje parçası/dosya bilgisi varsa aynı kalır.
Hiçbiri yokken revision değişmişse **Çizim güncellendi** denir. Revision aynı
olan gerçek no-op eski **Çizimde değişiklik yok** metnini kullanır. Geri alındı
son eki de korunur. Bilinmeyen değişimin türü veya kaç blok olduğu tahmin edilmez;
KCAD, komut, Python ve MCP şemalarına alan eklenmez.

Regresyon testi gerçek masaüstü dosyadan yeniden tanımlama handler'ını kullanır:
nesne listesi aynı kalırken blok tanımı değişir; aynı dosyanın ikinci kez
alınması no-op'tur; gerçek geri alma ilk satırın durumunu günceller. Test eski
metinde başarısızdı. 1604 masaüstü testi (256 mevcut ignored), derleme ve
Clippy geçti. Gerçek sağ tuş menüsünde tanım güncellemesi **Çizim güncellendi**,
aynı dosyanın değişikliksiz tekrarı **Çizimde değişiklik yok** dedi; gerçek tek
Undo ilk satırı **Çizim güncellendi · geri alındı** yaptı. Ekranlar 207–209
repo dışında `/tmp/kentos-cad-adv04-2026-11-01/` dizinindedir. Ortak komut
algoritması/format değişmediğinden önceki 440 Python kabulü yeniden koşulmadı.
Bu metin düzeltmesi blok değişim kimliklerini günlük formatına eklemiş sayılmaz.
