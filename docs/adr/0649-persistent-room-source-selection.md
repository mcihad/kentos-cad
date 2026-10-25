# 0649 — Mahalın kalıcı kaynak seçimi

25 Ekim 2026. **Bu dilim uygulandı ve doğrulandı.** ARCH-SPACE-01'in seçilmiş kaynak kapsamı; ana madde [~].

- `sources` yoksa eski tüm uygun duvar/kolon araması aynıdır. Verilirse boş
  olmayan, tekrarsız, sınırlı duvar/kolon UUID listesi mahalda saklanır. Kat ve
  atanmamış kapsamı listeyle birlikte uygulanır; uygun olmayan kaynak açık ret.
- Plan/execute/follower/yeniden bulma/ölçü/etiket/çizelge aynı listeyi kullanır.
  Kaynak silinmesi/tür veya kat değişimi son sınırı ve `sourceUnavailable`
  uyarısını korur; komşu duvara sessiz geçiş yapılmaz. Tek undo/redo bağı korur.
- Yeni oluşturma ve toplu bulmada görünür Tüm uygun kaynaklar / Seçilenler
  tercihi. Geçici sahne seçimi komutun içinde okunmaz; araç UUID'leri verir.
  Kaynak listesi özelliklerde görünür; açık değiştirme/temizleme yolu gerekir.
- Python/MCP aynı create/discover/edit sözleşmesini kullanır. KCAD 105 yalnız
  yeni alana/uyarıya opt-in olur; eski dosyanın şema/baytları korunur.
- Böl/merge çizilmiş sınıra dönüştüğünde kaynak listesi kaldırılır. Kopya/UUID
  yeniden eşleme davranışı ayrıca sınanır; dış referans sessizce farklı nesneye
  bağlanamaz.
- Sanal ayırıcı açık rolü bu listeden ayrı sonraki iştir; bu dilim düz çizgiyi
  duvar diye yorumlamaz. Ana plan maddesi bütün kapılar bitmeden kapanmaz.

Kabul kapıları: bağımsız geometri/kimlik korunumu ve eski/yeni regresyon;
follower kaynak kaybı/geri kazanma, farklı kat/aynı kot/uzak koordinat,
toplu/tek mahal, ret/boş tekrar/tek undo; Python/MCP/KCAD; gerçek fare/klavye,
önce/sonra görüntüler ve bağımsız kaydedilmiş dosya kontrolü.

Gerçek GUI'de bulunan eksik dispatcher kaydı, kırpılmış uyarı ve kaynak
sayısını örten özellik eylemi düzeltildi. 30 gerçek dosya ve 53 ham görüntü
[kabul kaydında](../baseline/2026-10-25-architecture-space-sources/README.md).
63 bağımsız ortak durum üç yöntem/beş birim/iki başlangıç ve iki kaynak
alt kümesini, kısmi kopya uyarılarını kapsar. Eski dosya ve genel kapsam
semantiği taşınmaz; yeni seçim/şema isteğe bağlıdır.
