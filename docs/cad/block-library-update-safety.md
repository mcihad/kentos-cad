# Kitaplık güncellemesinde mevcut değerleri koruma

`cad.blocks.edit` ile bir `kentos.blocks` kitaplığını `operation="import"`
ve `replace=True` vererek mevcut tanımların yerine alabilirsiniz. Önce
`answer(..., op="plan")` kullanın; başarılı planı mevcut
`cad.Expected(plan=plan.plan_id)` ile uygulayın. MCP'de aynı alanlar ve
`expected: {plan: ...}` geçerlidir.

Yeni tanım sahnedeki bir parametre değerini kabul etmiyorsa
`invalid_parameter_value`, taşınmış özniteliğin konumunu kabul etmiyorsa
`attribute_not_movable` döner. Yol `library` olur. Bu hata çizimi kısmen
güncellemez; mevcut verinizi kaydedebilirsiniz. Uyumlu kitaplık kullanın
veya değiştirmek istediğiniz örnek değerini/özel konumunu önce bilinçli
düzenleyin. Program uyumsuz değerlerinizi kendiliğinden silmez.

`cad.file.import_.block(..., mode="blocks", replace=True)` ile dosyadan
yeniden tanımlamada da mevcut yerleştirmeler korunur; uyumsuzluk
`block_refused`, yol `path` ile bildirilir. Bu işlem de önce planlanabilir.

Uyumlu geometri değişiminde örnek değerleri korunur ve tek Undo ile eski
tanımlara dönülür. Bu işlem adı eşleşen tanımları değiştirir; aile/yayın
sürümü izleyen seçilmiş yükseltme yerine geçmez.
[Karar ve kabul kapsamı](../adr/0852-block-library-replacement-validates-scene-inserts.md).

Bir kitaplıktaki ana/çocuk tanımlar birlikte doğrulanır ve uygulanır.
Ana tanım yeni eklenen çocuğu yerleştirebilir; çocuğun parametresi yeniden
adlandırılmışsa aynı kitaplıktaki ana tanımın iç içe yerleştirme değerleri de
yeni adla verilmelidir. Kitaplıkta ana/çocuk sırası nedeniyle geçerli bir son
grafik reddedilmez. Bilinmeyen çocuk, döngü veya uyumsuz parametre yine reddedilir;
kitaplığın bir kısmı sessizce atlanmaz. Hem kitaplık hem dosyadan güncelleme
tek Undo/Redo'dur. [Birlikte uygulama kararı](../adr/0853-related-block-library-definitions-apply-together.md).
