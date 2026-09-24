# ADR 0003: `CadDocument.transact` ya hep ya hiç çalışır

- **Durum:** kabul edildi, uygulandı (Faz A1)
- **Tarih:** 2026-09-23
- **Bağlam belgesi:** CLAUDE.md §13.1, §20 Faz A

## Bağlam

`transact(label, fn)` birkaç düzenlemeyi tek geri alma adımında topluyordu. `fn` hata fırlatınca `finally` bloğu o ana kadar uygulanmış op'ları yine de commit ediyordu. Sonuçları:

- belge kısmen değişmiş kalıyor, `dirty` işaretleniyor ve yarım işlem geri alma yığınına giriyordu;
- iç içe bir çağrının hatası dıştaki işlemin yarım işini de kalıcı yapıyordu;
- İşlem çalıştırıcısı (`processing/runner.ts`) hatayı yakalayıp "hata" raporluyordu, ama yarım uygulanan değişiklik belgede duruyordu.

Sunucu tarafında (CLAUDE.md §15) her yazma tek mantıksal commit olacak. İstemcideki işlem de aynı anlamı taşımalı ki yerel önizleme ile sunucu commit'i aynı sonucu versin.

## Karar

- **Hata olursa:** `fn` hata fırlatırsa, işlemin uyguladığı op'lar ters sırayla geri alınır (`rollback`) ve hata çağırana gider.
  - Commit yapılmaz, `dirty` değişmez, geri alma ya da yineleme yığınına dokunulmaz.
  - Görünümler geri almayı olaylarla (`changed`, `attrs`) öğrenir.
- **İç içe çağrı bir kayıt noktasıdır (savepoint):** iç işlemin hatası yalnızca kendi op'larını geri alır.
  - Hatayı yakalayan kod devam ederse, dış işlem geri kalanıyla commit edilir.
  - Hata dış işleme ulaşırsa, dış işlem her şeyi geri alır.
- **Grup içinde:** `beginGroup` içinde başarısız olan işlem gruba hiçbir şey katmaz. `cancel()` grubun yaptıklarını geri alır; yığınları ve `dirty` durumunu değiştirmez.
- **Kimlikler:** varlık kimlikleri geri alınan işlemlerde de yeniden kullanılmaz (`nextId` geri sarılmaz). Yerel geçici kimlik, sunucu kimliğiyle karıştırılmamalıdır (§15).

## Sonuçlar

- Çalıştırıcıda hata veren bir `apply` artık belgeyi değiştirmez.
- Yeni geri alma kodu sonradan eklenmedi: mevcut `invert` ve `applyAll` kullanılıyor. Ters op'lar, uygulanmamış bir op'u da zararsızca geri çevirir (silinmemiş varlığı yeniden yazmak aynı değeri yazar).
- Testler: `apps/web/src/model/document.test.ts`:
  - başarısız işlemin tam geri alınması;
  - iç içe kayıt noktası;
  - grup içinde başarısız işlem ve grup iptali.
