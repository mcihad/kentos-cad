# Örnek Python işlem aracı: Alan dilimleri

KentOS ile gelen örnek eklenti (CLAUDE.md §26, ADR 9310). İşlemler'e **Alan dilimleri** aracını ekler: kapalı alanları
alanlarına göre dilimlere ayırır, dilimi yeni katmana yazar, dilimlerin sayısını tablo olarak verir. Kendi aracınızı
yazarken bu klasörü kopyalayıp başlayın.

## Bir işlem aracı iki dosyadır

- **Tanım:** `tools/alan-dilimleri.tool.json` (`kentos.processingTool` 1). Adı, açıklaması, parametreleri ve
  çıktıları. KentOS aracı kodu çalıştırmadan buradan okur; pencere, komut satırı, model, Python ve MCP aynı tanımı
  gösterir.
  - `modules`: aracın çıktığı modüller (`cbs`, `harita`, `mimari`, `yol`); her biri manifestin `modules`'ünde olmalı.
    Yazılmazsa manifestinkiler geçerlidir. Başka modülün projesi aracı listelemez, çalıştırmaz.
  - `category`: araç kutusundaki yeri (`analysis`, `geometry`, `annotation` …); yazılmazsa Eklentiler.
  - `capabilities.runtimes`: `desktop-python` her zaman; `headless-python` ile MCP'de ve Python kütüphanesinde de.
- **İş:** `src/ornek_isleme/logic.py`. `@ext.processing("alan-dilimleri")` ile işaretli fonksiyon, parametreleri
  tanımdaki adlarıyla alır; girdiler veri olarak gelir (`ctx.read_features`), çıktılar tanımdaki adlarıyla döner.
  KentOS dönen her şeyi tanıma göre denetler, tek geri alma adımında yazar.

Manifest (`manifest.json`) aracı `entrypoints.processing` ile bağlar; Python projesi `pyproject.toml` ve `uv.lock`'tur.

## Denemek

```bash
PYTHONPATH=src python -m unittest discover -s tests
kentos kek verify paket/eklentiler/ornek-isleme
kentos kek pack paket/eklentiler/ornek-isleme
```

Paket, KentOS’ta Eklentiler penceresinin “Paket yükle…” düğmesiyle kurulur.
