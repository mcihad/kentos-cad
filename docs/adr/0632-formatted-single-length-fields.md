# 0632 — Tek uzunluk alanında gösterilen birimle düzenleme

25 Ekim 2026 · ARCH-UX-01 takip bulgusu · Durum: uygulandı ve gerçek GUI kabulü tamamlandı.

## Sorun ve sınır

ARCH-PERF-01 gerçek pencere denemesinde Kalınlık alanı `0.200 m` gösterirken
`0.300 m` girdisini reddetti; `0.300m` kabul edildi. `Format::length` sayı ile
birimin arasına boşluk koyuyor. Aynı sorun ondalık `in`/`ft` gösteriminde de var:
tek uzunluk okuyucusu yalnız imperial işaretlerini biliyordu.

Koordinat söz diziminde boşluk iki sayıyı ayırır. Ortak point-input v1 fixture'ı
`5 m` nokta girdisini bilerek reddeder. Bu nedenle koordinatların iç `length` /
`whole_len` / `pair` okuyucuları değiştirilmez. Yeni davranış yalnız ortak
**tek uzunluk alanı** okuyucusu `parse_length` içindedir.

## Davranış

- Önce mevcut sayısal, parantezli ve `'` / `"` imperial yolları aynen denenir.
- Tek sayının ardından tam bir birim adı gelirse `mm`, `cm`, `m`, `km` kabul edilir;
  arada JS boşluğu bulunabilir. Normal boşluk, NBSP ve dar NBSP gösterimleri aynı
  fiziksel değere gider. Ondalık virgül alanın mevcut kuralıyla korunur.
- Ondalık `in` ve `ft` tam son ekleri, boşluklu veya bitişik, bu alanlarda kabul
  edilir. Mevcut `Len::Inches` ve aynı dönüşüm kullanılır; yeni bir ölçek yoktur.
- `1 2`, `1 2 m`, `1 mm extra`, `1 min`, `1 inchworm`, `1 feet`, `1 m2` uzunluk
  sayılmaz. İçteki bütün boşluklar silinmez; iki sayı yanlışlıkla birleştirilmez.
- Nokta/koordinat ve iki uzunluklu alanların v1 söz dizimi aynen kalır. Bu dilim
  onlara `ft`/`in` veya boşluklu metrik uzantısı getirmez. Birim seçimi, fiziksel
  SI model, import, çizim/yazı ölçeği ve render hesabı değişmez.
- Native araçlar ortak `Context::typed_length` üzerinden kullanır. Anlamsal
  komutlar/Python/MCP sayısal metre parametrelerini kullanmayı sürdürür; yeni
  sözleşme veya UI'da hesaplanan geometri eklenmez. Web UI ADR 0228 gereği ertelenir.

## Kabul

Regresyon testi ilk gerçek girdi `0.300 m` için eski kodda başarısız oldu.
Düzeltme sonrasında aynı test ve bütün eski point-input fixture'ları geçti.
Beş proje biriminde gösterilen kalınlık metnini geri okuma, açık mm/m/in/ft,
salt taslak, ilk köşenin korunumu ve üretilen normal duvarın gerçek genişliği
native etkileşim testiyle denetlendi. Gerçek GUI Enter/blur/iptal/ret, tek undo/redo
ve kaydetme tamamlandı. Bağımsız okuyucu eski 100013 nesnenin bütün alanlarının
korunduğunu doğruladı; eklenen tek duvar 0.25 m. [Kabul kaydı](../baseline/2026-10-25-architecture-input/README.md)
ekranları, test sonuçlarını ve deneme sırasındaki odak hatasının geri alınmasını içerir.
