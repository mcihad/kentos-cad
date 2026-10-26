# 0665 — Host ilişkisini koruyan açıklık dağıtımı

25 Ekim 2026 · ARCH-OPEN-02 · Uygulama ve kabul çalışması sürüyor.

## Komut ve hesap

- Mevcut `cad.openings.edit` v1'e ek `array` işlemi gelir. Eski set/move/flip/
  rehost/replaceType/renumber/copy girdileri ve davranışları korunur. Yeni bir
  nesne veya KCAD dosya sürümü yoktur. Tek kaynak açıklık gerekir; kaynak yerinde,
  kimliği ve numarasıyla kalır. `count` yalnız **yeni kopyaların** sayısıdır.
- `layout.kind=spacing`: `distance` metre; `reference=clear` net pervaz arası,
  `centers` merkez arası. `direction=start/end` analitik host yönünü seçer.
  Net aralıkta adım en + boşluk, merkez aralığında doğrudan distance'dır.
- `layout.kind=fit`: `startMargin/endMargin` duvarın uçlarından net paylardır.
  İlk/son kopyanın pervazı pay sınırına gelir, merkezler eşit aralıktadır.
  Bir kopya kullanılabilir aralığın ortasına gelir. Kaynak bu dağıtıma dahil
  edilmez; kopyayla çakışırsa bütün işlem reddedilir.
- Ölçüler düz kiriş boyunca veya render kırıkları boyunca değil, mevcut
  duvarın analitik yol uzunluğu boyunca hesaplanır. Mevcut Start/Center/End
  ankraj korunur. Her kopya normal, ayrı host ilişkili açıklıktır.
- `numbering` yoksa yeni kopyalar numarasızdır; kaynak numarası çoğalmaz.
  Varsa `prefix/start/step` yalnız yeni kopyaları sıralar. Aynı çizimde aynı
  biçimdeki (door/window/bare ayrı) açıklığın numarasıyla çakışma bütün işlemi
  reddeder. Eski ilgisiz yinelenmeler yeniden numaralanmaz. Önek en çok 128
  Unicode karakter, adım pozitif, count 1–10000; numara hesabı u64'tür.
- Ondalıklı tam sığma kontrolü mevcut `SPAN_TOLERANCE=1e-9 m` pervaz
  politikasıyla yapılır. `20−17.8−2=0.2` aralığında iki 0.1 m açıklık,
  çıkarma yuvarlaması nedeniyle reddedilmez; gerçek 1 µm eksiklik reddedilir.
  Negatif kullanılabilir aralık toleransla kabul edilmez.
- Sonluluk, adet, pay/mesafe, sığma, köşe, diğer açıklıklarla çakışma ve mevcut
  kilit/revision kontrolleri yazmadan çalışır. Bir hata hiçbir kopya yazmaz.
  Normal edit uygulayıcısı bütün grubu tek transaction/journal/undo ile yazar;
  yeni kimlikler, kaynak nitelik/stil/katman bilgisi korunur; ekler ve bağımsız
  etiket bağlantıları eski copy işlemi gibi kopyalanmaz.

## Görünür araç

`tool.openingArray` Mimari ve Açıklık bağlamından açılır. Tek seçili açıklık
kaynak olur veya kullanıcı sahneden seçer. Kopya adedi, dağıtım, aralık türü,
yön, uç payları ve isteğe bağlı numaralar araç ayarlarında görünür. Kaynak
korunacağı ve sayının yeni kopya olduğu açıklanır. Önizleme aynı native
komutun planıdır; sahneye tıklamak kaynak seçer, Enter/Onayla bütün grubu
uygular. İptal ve ayar düzenleme belgeye yazmaz. Eski revision ile onay
reddedilir; önizleme yenilenir, uygulama tekrar açık onay ister.

## Kabul kapıları

- Bağımsız Python başvurusunda önceki 45 edit vakası korunarak 40 dizi vakası.
- Analitik düz/yaylı duvar, üç ankraj, iki yön; net/merkez aralığı; tek/çok adet;
  fit uç payları; Unicode/varsayılan numara; çakışma/köşe/numara/sonluluk retleri;
  planın yazmaması, kimlikler ve kaynak korunumu, tek undo/redo.
- Etkileşim ayarları, kaynak seçme, yazmasız önizleme/iptal, çakışmadan sonra
  düzeltip uygulama ve eski revision onayı.
- Python ve MCP aynı komutun tipli girdisi, plan/execute ve işlem korumaları.
- Gerçek GUI ayar/önizleme/onay/ret/undo/redo/çizelge/kaydet/aç görüntüleri.
  Çalıştırılan komutlar, kapsam ayrımı, ham günlükler, bağımsız dosya denetimi ve
  gerçek görüntüler [kabul kaydındadır](../baseline/2026-10-25-architecture-opening-arrays/README.md).
  Tam gerçek boyut/font/tema matrisi açık olduğundan ARCH-OPEN-02 [~] kalır.

Model birimi, çizim/pafta ölçeği, FP64, kenet açıklığı, import ve kamera/render
hattı değişmez. Web arayüzü ADR 0228 uyarınca ertelenmiştir; ortak sözleşme,
katalog ve Python SDK çıktısı güncellenir. Otomatik TS çıktıları geçici dizine
üretilir, yalnız bu işleme ait sözleşme farkları kaynak olarak alınır.

## Büyük yazı ve dar panel takibi

Gerçek koyu tema / 18 px etkileşim, özellik düzenleme ve gezinme takibi
[ayrı kabul kaydındadır](../baseline/2026-10-25-architecture-opening-array-dark18/README.md).
312 px panelde yeni aracın aralık/yön/fit etiketleri sadeleştirildi; wire
değerleri ve hesap aynı kaldı. Tam 1440×900 / 1100×650 matrisi halen açık.
