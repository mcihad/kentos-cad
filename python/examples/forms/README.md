# Form örnekleri

Her örnek başsız bir çizimde (`kentos.cad.Document`) bir işi baştan sona yapar ve `main()` sonuçlarını döndürür;
`python/tests/test_examples.py` hepsini çalıştırıp sonuçlarını denetler. Örneklerin hiçbiri bulut, ağ ya da eklenti istemez.

| Örnek | Kapsam |
|---|---|
| `building.py` | Alanlar, alanlardan form, görünme ve zorunluluk kuralları, hesap, yayın, formla yazma, Kayıtları denetle, Hesapları yenile (ADR 0401–0421) |
| `kinds.py` | Alt türler ve değer bileşimleri; yazmaların reddi ve formun daralan seçenekleri (ADR 0422, 0423) |
| `owners.py` | İlişki, geometrisiz kayıtlar, formun ilişkili kayıtlar öğesi, alt kayıtlarıyla çoğaltma, silmede izleme (ADR 0405, 0409, 0410, 0415) |
| `address.py` | Başka katmandan değer listesi ve zincir (il, ilçe), etiketin kodla yazılması (ADR 0404) |

    .run/py/bin/python python/examples/forms/building.py
