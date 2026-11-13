# Yapay zekâ sağlayıcıları: uygulama içi ajan için API araştırması

Tarih: 13 Kasım 2026. Konu: KentOS CAD masaüstü uygulamasına gelecek sohbet ajanı. Kullanıcı ayarlardan sağlayıcı seçer, kendi API anahtarını girer, bir ajanı varsayılan yapar; ajan sohbet penceresinde çizim araçlarını çağırır (çizimi oku, komut planla, uygula). Çağrılar SDK'sız, düz HTTPS ile Rust'tan yapılacak (`reqwest` 0.13 + rustls, `tokio`), akış (SSE) ve araç çağrısı (tool/function calling) zorunlu.

## Yöntem ve sınırlar

- Bütün kaynaklar 13 Kasım 2026'da okundu. Sayfaların çoğu tarih göstermiyor; gösterenlerin tarihi yanında yazılı. Kaynak kısaltmaları ([A1], [O4] …) en sondaki listededir.
- Belgeler mümkün olduğunda sağlayıcının kendi Markdown sürümünden (`.md`, `llms.txt`) indirilip aranarak okundu; Alibaba, LM Studio ve bazı tekil sayfalar özetleyici okuma aracıyla okundu.
- **[T*]** kaynakları topluluk raporudur (forum, GitHub, üçüncü taraf kod). Resmî belge değildir; yalnız resmî belgenin sustuğu yerde anıldı ve “doğrulanmadı” diye işaretlendi. Bunların sayfaları açılmadı, arama sonucu özetinden alındı.
- Hiçbir sağlayıcıya anahtarlı istek atılmadı; davranış belgeden yazıldı. Tek istisna: Z.ai'de anahtarsız bir `GET /models` denemesi (aşağıda).
- Fiyatlar yalnız okunan sayfada görüldüyse yazıldı (USD / 1M belirteç = token).
- Belgeler birçok yerde değişmiş: OpenAI belgeleri `developers.openai.com`'a, Anthropic belgeleri `platform.claude.com`'a, Moonshot belgeleri `platform.kimi.ai`'ye taşınmış; Google ise `generateContent`'i “legacy” sayıp Interactions API'yi varsayılan yapmış. Bellekteki 2025 bilgisine güvenilmemeli.

## Özet

- **Dört tel biçimi yeter:** Anthropic Messages (Anthropic), OpenAI Responses (OpenAI, xAI), OpenAI Chat Completions (DeepSeek, Kimi, Z.ai GLM, Mistral, Qwen, OpenRouter, Ollama, LM Studio) ve Gemini yerel `generateContent` (Gemini).
- **OpenAI için Chat Completions artık yetmiyor:** GPT-5.4'ten beri Chat Completions, `reasoning_effort` `none` değilken araç çağrısını desteklemiyor; `gpt-6-astra` ve `gpt-6.1-sol` araç çağrısı için Responses istiyor [O1][O2][O3]. xAI de Chat Completions'ı “legacy” yaptı [X1][X2].
- **Gemini için yerel protokol:** OpenAI uyumlu uç hâlâ “beta” [G8]; düşünce imzalarının o uçta hangi alanda taşındığı güncel resmî sayfada yazmıyor, topluluk 400 ve kararsızlık bildiriyor [T1][T3][T4]. Yerel `generateContent` imza kurallarını açıkça belgeliyor [G6].
- **En büyük ortak tuzak, düşünce/akıl yürütme artıklarının geçmişe geri gönderilmesi:** Anthropic `thinking` blokları + `signature`, Gemini `thoughtSignature`, OpenAI/xAI şifreli `reasoning` öğeleri, DeepSeek/Kimi/GLM/Qwen `reasoning_content`, Mistral `thinking` parçaları, OpenRouter `reasoning_details`. Anthropic değiştirilmiş bloğu, Gemini ve DeepSeek eksik gönderimi 400 ile reddeder [A4][G6][D4]; diğerlerinde kayıp sessizdir (kalite düşer).
- **Zorla araç çağırma taşınabilir değil:** Claude Opus 5.5 / Sonnet 5.5 `tool_choice` `any`/`tool`'u reddediyor [A4], GLM yalnız `auto` [Z11], Qwen `required`'ı desteklemiyor [Q2], Ollama `tool_choice` desteklemiyor [L1]. Ajan döngüsü `auto` üzerine kurulmalı.
- **Örnekleme parametreleri gönderilmemeli:** Anthropic `2026-10-21` sürümünde `temperature`/`top_p`/`top_k` 400 [A2]; Kimi K3'te sabit, başka değer hata [K4]; Gemini 3.6+ ve 4'te yok sayılıyor [G5]; xAI akıl yürütme modellerinde `stop` ve cezalar hata [X6].
- **Akış ayrıştırıcısı ortak olmalı ama sağlayıcı farklarını bilmeli:** adlandırılmış olaylar (Anthropic) / yalnız `data:` (diğerleri), `: keep-alive` ve `: OPENROUTER PROCESSING` yorum satırları, `[DONE]` olan/olmayan sonlar, HTTP 200 içinde akış ortası hata, parçalı ya da tek parça araç argümanları [A10][D5][R2][X5].

## Karşılaştırma tablosu

| Sağlayıcı | Taban URL (uç) | Kimlik | Önerilen tel biçimi | Araç çağrısı | Akış | Model listesi | Önerilen modeller (amiral / ucuz) |
|---|---|---|---|---|---|---|---|
| Anthropic | `https://api.anthropic.com/v1/messages` | `x-api-key` ya da `Authorization: Bearer`; `anthropic-version: 2026-10-21` | Anthropic Messages | `tool_use` (`input` nesne) → `tool_result` blokları | Adlandırılmış SSE; son `message_stop` | `GET /v1/models` | `claude-opus-5-5` / `claude-haiku-5-5` |
| OpenAI | `https://api.openai.com/v1/responses` | `Authorization: Bearer` | OpenAI Responses (`store: false`) | `function_call` öğesi (`arguments` metin, `call_id`) → `function_call_output` | Tipli SSE (`response.*`); son `response.completed` | `GET /v1/models` | `gpt-6-astra` / `gpt-6-luna` |
| DeepSeek | `https://api.deepseek.com/chat/completions` | `Authorization: Bearer` | Chat Completions + `reasoning_content` geri gönderimi | OpenAI `tool_calls`; `strict` beta `/beta` ucunda | `data:` SSE, `: keep-alive`, `data: [DONE]` | `GET /models` | `deepseek-v4-pro` / `deepseek-flash` |
| Google Gemini | `https://generativelanguage.googleapis.com/v1beta/models/{model}:streamGenerateContent?alt=sse` | `x-goog-api-key` | Gemini yerel `generateContent` (ileride Interactions) | `functionCall` (`id`, `args` nesne) + `thoughtSignature` → `functionResponse` | `data:` SSE; `[DONE]` yok | `GET /v1beta/models` | `gemini-4-carbon` / `gemini-4-hydrogen` |
| Moonshot Kimi | `https://api.moonshot.ai/v1/chat/completions` (Çin: `api.moonshot.cn`) | `Authorization: Bearer` | Chat Completions | `tool_calls` (kimlik ör. `search:0`); `tool` mesajında `name` | `data:` SSE | `GET /v1/models` (yetenek bayraklı) | `kimi-k3.1-pro` / ucuz sınıf yok (`kimi-k3.1-pro-highspeed` hızlı) |
| Z.ai GLM | `https://api.z.ai/api/paas/v4/chat/completions` (Çin: `open.bigmodel.cn/api/paas/v4`) | `Authorization: Bearer` | Chat Completions + `tool_stream: true` | `tool_calls`; `tool_choice` yalnız `auto` | `data:` SSE, `data: [DONE]` | Belgelenmemiş | `glm-5.3` / `glm-5.3-flash` |
| Mistral | `https://api.mistral.ai/v1/chat/completions` | `Authorization: Bearer` | Chat Completions | `tool_calls`; kimlik 9 karakter (topluluk) | `data:` SSE, `data: [DONE]` | `GET /v1/models` | `mistral-large-4` / `mistral-small-2603` |
| xAI (Grok) | `https://api.x.ai/v1/responses` | `Authorization: Bearer` | OpenAI Responses (Chat Completions “legacy”) | `function_call` → `function_call_output`; akışta tek parça | Tipli SSE | `GET /v1/models` | `grok-4.7` / `grok-4.3` |
| Alibaba Qwen | `https://{WorkspaceId}.ap-southeast-1.maas.aliyuncs.com/compatible-mode/v1` | `Authorization: Bearer` (anahtar bölgeye özgü) | Chat Completions | `tool_calls`; `required` yok | `data:` SSE | Doğrulanamadı | `qwen3.8-max` / `qwen3.8-flash` |
| OpenRouter | `https://openrouter.ai/api/v1/chat/completions` | `Authorization: Bearer` | Chat Completions | Standartlaştırılmış OpenAI biçimi | SSE + `: OPENROUTER PROCESSING`, `data: [DONE]` | `GET /api/v1/models?supported_parameters=tools` | Kullanıcı seçer |
| Ollama | `http://localhost:11434/v1/chat/completions` (bulut: `https://ollama.com/v1`) | Yerelde yok (bulut: Bearer) | Chat Completions | `tools` var, `tool_choice` yok | SSE (`/v1`); yerel `/api/*` NDJSON | `GET /api/tags`, `GET /v1/models` | Yerel model |
| LM Studio | `http://localhost:1234/v1/chat/completions` | Varsayılan yok (0.4.0+: Bearer) | Chat Completions | `tools`; “default” modda kırılgan | SSE | `GET /v1/models` | Yerel model |

## Sağlayıcılar

### 1. Anthropic (Messages API)

**Uç ve kimlik.** `POST https://api.anthropic.com/v1/messages` [A1]. Başlıklar: `x-api-key: <anahtar>` (eski ama destekleniyor) ya da `Authorization: Bearer <anahtar>`; `anthropic-version` zorunlu, güncel örnek değer `2026-10-21`; `content-type: application/json`; tek çalışma alanına bağlı olmayan anahtarda `anthropic-workspace-id` zorunlu [A1]. SDK bu başlıkları kendisi koyar; biz elle koymalıyız.

**Sürüm.** `2026-10-21` sürümü `temperature`, `top_p`, `top_k` parametrelerini kaldırdı: gönderilirse her modelde 400. Ayrıca `assistant` mesajında `document` bloğu 400 [A2]. Önceki sürüm `2023-06-01`, SSE olaylarını adlandırılmış yaptı ve `data: [DONE]`'ı kaldırdı [A2]. Sayfa en yeni sürümü öneriyor [A2].

**İstek.** `model`, `max_tokens` (zorunlu), `messages` (`role`: `user` | `assistant`; `content` metin ya da blok dizisi), `system` (üst düzey; metin ya da `text` blokları), `tools`, `tool_choice`, `thinking`, `output_config` (`effort`: `low|medium|high|xhigh|max`; `format`), `stop_sequences`, `stream`, `metadata.user_id`, üst düzey `cache_control` [A3].
- Araç tanımı: `name` (`^[a-zA-Z0-9_-]{1,128}$`), `description`, `input_schema` (kök `type: "object"`), isteğe bağlı `strict`, `cache_control`, `eager_input_streaming` [A3]. `strict: true`, dilbilgisi kısıtlı örneklemeyle girdinin şemaya uymasını ve adın geçerli olmasını garanti eder; desteklenen JSON Schema alt kümesi Structured Outputs sayfasında [A17].
- `tool_choice`: `{"type":"auto"}` (varsayılan), `{"type":"any"}`, `{"type":"tool","name":"…"}`, `{"type":"none"}`; paralel çağrıyı kapatan `disable_parallel_tool_use` bu nesnenin içindedir, üst düzey değil [A3][A8].
- **Claude Fable 5.2, Opus 5.5, Sonnet 5.5 ve Fable 5.1 zorla araç çağrısını (`any`, `tool`) 400 ile reddeder;** yalnız `auto` ve `none` kabul [A4].
- Önceden doldurma (assistant prefill) 4.6 ve sonrası modellerde yok: konuşma `user` mesajıyla bitmeli, yoksa 400 [A4].

**Yanıt ve araç sonucu.** `content` blokları: `text`, `thinking` (`thinking`, `signature`), `redacted_thinking`, `tool_use` (`id` ör. `toolu_01A09q90qw90lq917835lq9`, `name`, `input` **nesne**) [A6][A9]. `stop_reason`: `end_turn`, `max_tokens`, `stop_sequence`, `tool_use`, `pause_turn` (sunucu aracı döngü sınırı; içeriği aynen geri gönderip sür), `refusal`, `model_context_window_exceeded` [A5].
- Sonuç bir sonraki `user` mesajında: `{"type":"tool_result","tool_use_id":"…","content":<metin ya da text/image/document blokları>,"is_error":true}` (`is_error` isteğe bağlı) [A7].
- Sıra kuralları: sonuçlar araç çağıran asistan mesajının hemen ardından gelir, arada mesaj olamaz; `tool_result` blokları içerik dizisinin başında, varsa metin sonra; paralel çağrıların bütün sonuçları **tek** `user` mesajında [A7][A8].

**Düşünme.** Claude 5 ve sonrası modellerde düşünme hep açık (uyarlamalı); `display` varsayılanı `"omitted"` (boş `thinking` alanı + `signature`). Özeti görmek için `thinking: {"type":"adaptive","display":"summarized"}` [A9]. Opus 5.5'te `{"type":"disabled"}` 400; Sonnet 5.5'te en düşük ayar `{"type":"between_tools"}`; 4.7 ve sonrası `enabled` + `budget_tokens` kabul etmez [A4]. Varsayılan efor: Opus 5.5 `medium`, Sonnet 5.5 `high` [A12]. **Araç kullanımında asistan turundaki bütün `thinking` ve `redacted_thinking` blokları (boş olanlar dahil) değiştirilmeden geri gönderilmeli; değişirse 400** [A4]. İmza, bloğu üretildiği önek (sistem, araçlar, önceki mesajlar) ile bağlar; geçmiş değişirse “bound to a different conversation” 400'ü gelir [A4].

**Akış.** `"stream": true`; adlandırılmış SSE (`event:` + `data:`). Sıra: `message_start` (boş `content`) → her içerik bloğu için `content_block_start`, `content_block_delta`…, `content_block_stop` → `message_delta` (`stop_reason`, **birikimli** `usage`) → `message_stop`. Arada `ping` gelebilir; hata `event: error` + `{"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}`; bilinmeyen olay türleri sessizce geçilmeli [A10].
- Delta türleri: `text_delta` (`text`), `input_json_delta` (`partial_json`; parçalar biriktirilip `content_block_stop`'ta ayrıştırılır; varsayılan olarak her anahtar–değer tamamlanınca gönderilir, bu yüzden araçlarda olaylar arası bekleme olabilir), `thinking_delta`, `signature_delta` (blok kapanmadan hemen önce) [A10]. `redacted_thinking` ve sunucu aracı sonuçları deltasız, tek parça gelir [A10]. Araç başına `eager_input_streaming` ile ince taneli akış [A10].
- Kopan akış: 4.6 ve sonrası modellerde yarım yanıt assistant mesajına konamaz; yarım metni içeren ve “devam et” diyen bir `user` mesajıyla yeniden istenir; `tool_use` ve `thinking` blokları kısmen kurtarılamaz [A10].

**Hatalar ve hız sınırı.** Gövde: `{"type":"error","error":{"type":"…","message":"…"},"request_id":"req_…"}` [A4]. 400 `invalid_request_error` (kendi koyduğunuz harcama sınırı da 400), 401 `authentication_error`, 402 `billing_error`, 403 `permission_error`, 404 `not_found_error`, 409 `conflict_error`, 413 `request_too_large` (Messages için 128 MB), 429 `rate_limit_error`, 500 `api_error`, 504 `timeout_error`, 529 `overloaded_error` [A4][A1]. Kademe harcama tavanı 429'unda `retry-after` yok ve `error.details.error_code` = `enforced_spend_limit_reached`; ani artış (ramp) 429'u `ramp_limit_exceeded` [A11]. Başlıklar: `retry-after` (saniye); `anthropic-ratelimit-requests-limit|remaining|reset`, `anthropic-ratelimit-tokens-*`, `anthropic-ratelimit-input-tokens-*`, `anthropic-ratelimit-output-tokens-*` (reset değerleri RFC 3339 zaman damgası); her yanıtta `request-id` [A11][A1].

**Modeller.** `claude-opus-5-5` (çoğu iş için önerilen başlangıç; $4 / $20), `claude-fable-5-2` (en zorlu akıl yürütme ve uzun ajan işleri; $8 / $40), `claude-sonnet-5-5` ($2 / $10), `claude-haiku-5-5` ($0,10 / $0,50'den başlayan). Hepsi 1M bağlam, 128K çıktı, görsel girdi ve araç kullanımı [A12]. Liste: `GET /v1/models` (`after_id`, `before_id`, `limit`; `data[].id`, `display_name`, `created_at`, `max_input_tokens`, `max_tokens`, `capabilities`, `line`; sayfalama `has_more`, `first_id`, `last_id`) [A13][A12].

**Önbellek, görsel.** Üst düzey `cache_control` otomatik önbelleği açar (kesme noktası son uygun bloğa kendiliğinden kayar); ya da bloklara açık `cache_control` (en çok 4 kesme noktası); TTL 5 dk varsayılan, `"ttl": "1h"` iki kat yazma fiyatı; 5.x modellerde en az 512 belirteç; önek sırası `tools` → `system` → `messages`; `usage.cache_creation_input_tokens` / `cache_read_input_tokens` [A14]. Önbellekten okunan belirteçler ITPM sınırına sayılmaz [A11]. Görsel: `image` bloğu, `source.type: "base64"`, `media_type` `image/jpeg|image/png|image/gif|image/webp`; en çok 8000×8000 px; istekte 20'den çok görsel varsa daha sıkı sınır (her kenar ≤3000 px önerilir) [A15]. `tool_result` içine görsel konabilir — CAD ekran görüntüsünü araç sonucu olarak vermek için uygun [A7].

**Veri.** Saklanan veri açık izin olmadan model eğitiminde kullanılmaz; konuşma içeriği varsayılan olarak saklanmaz (30 gün saklama isteyen “Covered Models” hariç); standart ticari saklama politikası ayrı sayfada [A16].

### 2. OpenAI (Responses ve Chat Completions)

**Uç ve kimlik.** `https://api.openai.com/v1` + `/responses` ya da `/chat/completions`; `Authorization: Bearer $OPENAI_API_KEY` [O12]. `platform.openai.com/docs/...` adresleri `developers.openai.com/api/docs/...`'a yönleniyor.

**Hangisi ve neden.** OpenAI: “Chat Completions desteklenmeye devam ediyor, ama bütün yeni projeler için Responses öneriliyor” [O2]. Bizim için belirleyici olanlar:
- GPT-5.4'ten itibaren Chat Completions, `reasoning_effort` `none` dışındayken araç çağrısını desteklemiyor [O2].
- GPT-6 Astra ve GPT-6.1 Sol araç çağrısı için Responses API istiyor [O1]; `gpt-6.1-sol` ve `gpt-6.1-terra` model sayfaları “Tool calling requires the Responses API” diyor; `gpt-6-luna`'da Chat Completions araç çağrısı yalnız `reasoning_effort: "none"` ile [O3]. `gpt-6-astra`'nın efor değerleri `low…max`, `none` yok [O3].
- Akıl yürütme modelleri Responses'ta daha iyi çalışıyor; Responses durumsuz da kullanılabiliyor (`store: false` + şifreli akıl yürütme öğeleri) [O2].
Sonuç: OpenAI için **Responses, `store: false`**.

**Responses isteği.** `model`, `input` (öğe dizisi), `instructions` (sistem yönergesi), `tools`, `tool_choice`, `parallel_tool_calls` (varsayılan `true`), `max_output_tokens`, `reasoning` (`effort` …), `store` (**varsayılan `true`**), `include` (ör. `"reasoning.encrypted_content"`), `stream`, `previous_response_id`, `prompt_cache_key`, `temperature` [O4]. `previous_response_id` üst düzey `instructions`'ı taşımaz; her istekte yeniden gönderilir [O2].
- Araç tanımı Responses'ta “iç etiketli”: `{"type":"function","name":"…","description":"…","parameters":{…},"strict":true}`; Chat Completions'ta “dış etiketli”: `{"type":"function","function":{"name","description","parameters","strict"}}` [O2][O8]. İşlev adı: `a-z`, `A-Z`, `0-9`, `_`, `-`, en çok 64 karakter [O8].
- `strict`: Responses'ta verilmezse katı moda çevirmeyi dener, olmazsa katı olmayana düşer ve araçta `strict: false` döner; Chat Completions'ta varsayılan katı değil [O2]. Katı mod koşulları: her nesnede `additionalProperties: false`, `properties`'teki her alan `required`'da; isteğe bağlı alan `null` tür seçeneğiyle [O1]. Alt küme: string `pattern`, `format` (`date-time`, `time`, `date`, `duration`, `email`, `hostname`, `ipv4`, `ipv6`, `uuid`); sayı `multipleOf`, `maximum`, `exclusiveMaximum`, `minimum`, `exclusiveMinimum`; dizi `minItems`, `maxItems`; `anyOf`, tanımlar ve özyineleme destekli. Desteklenmeyen: `allOf`, `not`, `dependentRequired`, `dependentSchemas`, `if`/`then`/`else`. Kök nesne olmalı, `anyOf` olamaz; en çok 5000 özellik, 10 iç içe düzey, 1000 enum değeri [O5].
- `tool_choice`: `"auto"`, `"required"`, `"none"`, `{"type":"function","name":"…"}`, `{"type":"allowed_tools","mode":"auto",…}` [O1]; Chat Completions'ta `"none"|"auto"|"required"`, işlev nesnesi ve `allowed_tools` [O8].

**Yanıt ve araç sonucu.** `output` öğeleri: `reasoning`, `function_call` (`id` ör. `fc_…`, `call_id` ör. `call_…`, `name`, `arguments` **JSON metni**), `message` [O1]. Sonuç yeni bir girdi öğesidir: `{"type":"function_call_output","call_id":"…","output":"…"}` [O1]. Akıl yürütme modellerinde araç çağrılı yanıtlardaki `reasoning` öğeleri araç çıktılarıyla birlikte geri gönderilmeli [O1]; `store: false` kullanımında her `reasoning` öğesi `encrypted_content` taşır ve aynen yeniden gönderilir [O2][O4]. Yeni modeller, önek değişikliğinden (ör. araç sonucunu kısaltmak, eski turları özetlemek) sonraki reasoning öğelerini düşürür; düşürülenler yanıttaki `input_transformations`'ta görünür [O16].

**Chat Completions (karşılaştırma).** `messages` (`developer`/`system`, `user`, `assistant` + `tool_calls`, `tool` + `tool_call_id`), `max_completion_tokens` (`max_tokens` kullanımdan kalktı, o-serisi ile uyumsuz), `parallel_tool_calls` (varsayılan `true`), `stream_options`, `store` [O8].

**Akış.** Responses: tipli SSE olayları; `response.created`, `response.in_progress`, `response.output_item.added` (işlev çağrısında `item.type: "function_call"`, `arguments: ""`), `response.function_call_arguments.delta` (`item_id`, `output_index`, `delta`), `response.function_call_arguments.done`, `response.output_item.done`, `response.output_text.delta`, `response.reasoning_summary_text.delta`, `response.completed`, `response.incomplete`, `response.failed`, `error`. Argüman parçaları `output_index`'e göre birleştirilir [O1][O6]. Chat Completions: `chat.completion.chunk`; `choices[].delta.tool_calls[]` = `{index, id, type, function{name, arguments}}`; `finish_reason`: `stop`, `length`, `tool_calls`, `content_filter`, `function_call` (eski); `usage` yalnız `stream_options: {"include_usage": true}` ile, `data: [DONE]`'dan önceki ek parçada [O7][O8].

**Hatalar ve hız sınırı.** 401 (geçersiz anahtar, kuruluş üyeliği yok, IP izinli değil), 403 (desteklenmeyen ülke/bölge), 429 (`Rate limit reached` — `Retry-After`'a uy; `rate_limit_error` + `slow_down`; faturalama: `credit_balance_exhausted`, `organization_spend_limit_exceeded`, `project_spend_limit_exceeded`, `organization_usage_limit_exceeded`; bunlarda `error.type` `insufficient_quota` olabilir, `error.code`'a bakılmalı), 500, 503 (`service_unavailable_error` + `server_is_overloaded`) [O9]. Başlıklar: `x-ratelimit-limit-requests`, `x-ratelimit-limit-tokens`, `x-ratelimit-remaining-requests`, `x-ratelimit-remaining-tokens`, `x-ratelimit-reset-requests` (ör. `1s`), `x-ratelimit-reset-tokens` (ör. `6m0s`), proje kapsamlı `x-ratelimit-*-project-tokens`; `Retry-After` 429 ve 503'te olabilir [O10].

**Modeller.** Amiral `gpt-6-astra` ($10 / $50; 1.050.000 bağlam; 128.000 çıktı), dengeli `gpt-6.1-sol` ($2 / $10), hızlı/görsel `gpt-6.1-terra`, en verimli `gpt-6-luna` ($0,10 / $0,50) [O3][O11]. Liste: `GET /v1/models` → `data[].id`, `created`, `owned_by`, `shutdown_date`; yetenek bilgisi yok [O12].

**Önbellek, görsel.** Örtük önbellek varsayılan; en kısa önbelleklenebilir önek GPT-5.6 ve sonrası için 1.024 belirteç; `prompt_cache_key`; `prompt_cache_retention` `in_memory` ya da `24h`; açık kesme noktası modu isteğe bağlı [O13]. Görsel: Responses'ta `{"type":"input_image","image_url":"<URL ya da data URL>","detail":"auto"}`; Chat Completions'ta `{"type":"image_url","image_url":{"url":"…","detail":"…"}}`; PNG, JPEG, WEBP, animasyonsuz GIF; istek başına en çok 512 MB [O14][O8].

**Veri.** API verisi 1 Mart 2023'ten beri açıkça izin verilmedikçe eğitimde kullanılmıyor; kötüye kullanım izleme kayıtları en çok 30 gün saklanıyor [O15]. Responses varsayılan olarak saklanır; `store: false` ile kapatılır [O2].

### 3. DeepSeek

**Uç ve kimlik.** OpenAI biçimi `https://api.deepseek.com` (`/chat/completions`), Anthropic biçimi `https://api.deepseek.com/anthropic`; `strict` (beta) için `https://api.deepseek.com/beta` [D1][D2]. `Authorization: Bearer <anahtar>` [D3]. Model tablosu Responses API ve Anthropic API desteğini de “Yes” gösteriyor [D1].

**Modeller.** `deepseek-flash` (DeepSeek-V4.1-Flash-1017; görsel girdi var; eşzamanlılık 2500) ve `deepseek-v4-pro` (DeepSeek-V4-Pro-0813; görsel yok; eşzamanlılık 500). İkisi de 1M bağlam, en çok 384K çıktı, düşünme varsayılan açık; eski `deepseek-v4-flash` adı hâlâ kabul ediliyor [D1]. Fiyat (yoğun olmayan / yoğun saat): flash girdi (önbellek kaçırma) $0,15 / $0,3, çıktı $0,6 / $1,2; pro $0,66 / $1,32 ve $1,98 / $3,96; yoğun saatler hafta içi 01:00–04:00 ve 06:00–10:00 UTC [D1]. Sayfada `deepseek-chat` / `deepseek-reasoner` adları artık yok.

**İstek ve düşünme.** `thinking: {"type":"enabled"|"disabled"}` (OpenAI SDK'da `extra_body`), `reasoning_effort` `low|high|max` (`none` kapatır); varsayılan açık, efor `high`; uyum için `minimal`→`low`, `medium` ve `xhigh`→`high` eşlenir [D4][D3]. Düşünme modunda `temperature`, `presence_penalty`, `frequency_penalty` hata vermez ama etkisiz [D4]. `max_tokens` 1–393216; varsayılan düşünmesiz 8K, düşünmeli 64K (`max` eforla 128K) [D3].
- **Araçlı isteklerde önceki bütün turların `reasoning_content`'i (araç çağırmayan turlar dahil) geri gönderilmeli; gönderilmezse 400.** Araçsız isteklerde gönderilse de yok sayılır [D4].

**Araç.** OpenAI biçimi (`tools`, `tool_calls`, `role: "tool"`). `strict` beta: bütün işlevlerde `strict: true`; her nesnede bütün özellikler `required` ve `additionalProperties: false`; türler `object`, `string`, `number`, `integer`, `boolean`, `array`, `enum`, `anyOf`; string'de `pattern` ve `format` (`email`, `hostname`, `ipv4`, `ipv6`, `uuid`) var, `minLength`/`maxLength` yok [D2]. Chat Completions, modelin üretmediği araç çağrılarını konuşmanın ortasına eklemeyi desteklemez; bunun için Anthropic ya da Responses biçimi gerekir [D2].

**Akış.** Yalnız `data:` SSE, `data: [DONE]` ile biter [D3]. Yanıt beklerken akışta SSE yorumu `: keep-alive`, akışsız istekte boş satırlar gelebilir; elle ayrıştıran bunları atlamalı [D5]. `finish_reason`: `stop`, `length`, `content_filter`, `tool_calls`, `insufficient_system_resource`, `aborted` [D3]. `usage`'da `prompt_cache_hit_tokens`, `prompt_cache_miss_tokens` [D3][D8].

**Hatalar ve sınır.** 400 Invalid Format, 401 Authentication Fails, 402 Insufficient Balance, 422 Invalid Parameters, 429 Rate Limit Reached, 500 Server Error, 503 Server Overloaded [D6]. Sınır hesap düzeyinde eşzamanlı istek sayısı; aşımda 429; `user_id` parametresi son kullanıcıları ayırır [D5]. Hız sınırı başlıkları belgelenmemiş.

**Liste, önbellek, görsel.** `GET /models` (`id`, `object`, `owned_by`) [D7]. Diskte bağlam önbelleği herkese varsayılan açık, “en iyi çaba”; kullanılmayan önbellek birkaç saat–birkaç gün içinde silinir [D8]. Görsel girdi yalnız `deepseek-flash`'ta [D1][D9].

**Veri.** Gizlilik politikası (son güncelleme 10 Şubat 2026): kişisel veri Çin Halk Cumhuriyeti'nde toplanır, işlenir ve saklanır; kullanıcı girdisi modelleri eğitmek ve geliştirmek için kullanılabilir, buna itiraz (opt-out) hakkı tanınır; açık platformla geliştirilen uygulamalar politikanın kapsamı dışında [D10]. Ayrı bir açık platform veri politikası bu çalışmada bulunamadı.

### 4. Google Gemini

**Hangi API.** Google'ın belge dizini: varsayılan Interactions API; `generateContent` mevcut entegrasyonlar için destekleniyor ama “legacy” [G1]. Interactions sayfası: “legacy sayılsa da `generateContent` tam destekli” [G2].

**Yerel `generateContent`.** `POST https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent`; akış `:streamGenerateContent?alt=sse`; anahtar `x-goog-api-key` başlığıyla (örneklerde `key=` sorgu parametresi de var) [G3][G4]. Kararlı `v1` referansı da yayında [G1].
- İstek: `contents` (`role`: `user` | `model`; `parts`: `text`, `inline_data`, `functionCall`, `functionResponse`; parçada `thoughtSignature`), `systemInstruction`, `tools: [{"functionDeclarations":[…]}]`, `toolConfig.functionCallingConfig` (`mode`: `AUTO` — yalnız işlev bildirimi varken varsayılan, `ANY`, `NONE`, `VALIDATED`; `allowedFunctionNames`), `generationConfig` (`maxOutputTokens`, `stopSequences` en çok 5, `thinkingConfig` → `thinkingLevel`, `includeThoughts`) [G3][G4]. Gemini 3.6+ ve 4'te `temperature` ve `topP` yok sayılıyor [G5].
- İşlev bildirimi: `name` (a-z, A-Z, 0-9, `_`, `:`, `.`, `-`; en çok 128), `description` (zorunlu), `parameters` (OpenAPI 3.0 alt kümesi `Schema`) **ya da** `parametersJsonSchema` (JSON Schema; kök nesne) [G3]. `ANY` modunda ve karmaşık şemalarda bildirimler kısıtlı çözümleme için bir durum makinesine derlenir; sınır aşımı `400 INVALID_ARGUMENT` [G4].
- Yanıt: `candidates[].content.parts[]` içinde `functionCall` (`id`, `name`, `args` **nesne**). Gemini 3 her `functionCall`'a benzersiz `id` veriyor; `functionResponse`'a aynı `id` konmalı [G4][G3]. Sonuç `user` rollü içerikte `{"functionResponse":{"id","name","response":{…}}}` [G4][G3]. Yerleşik araçlarla birlikte kullanılırsa `functionCall` hep son parça olmayabilir; parçalar sırayla taranmalı [G4].
- `finishReason`: `STOP`, `MAX_TOKENS`, `SAFETY`, `RECITATION`, `OTHER`, `MALFORMED_FUNCTION_CALL`, `UNEXPECTED_TOOL_CALL`, `TOO_MANY_TOOL_CALLS`, `MISSING_THOUGHT_SIGNATURE`, `CONTINUATION` ve görsel/güvenlik değerleri [G3].

**Düşünce imzaları (zorunlu).** Gemini 4 ve 3'te işlev çağrısında `thoughtSignature` geri gönderilmezse doğrulama hatası (4xx) [G6]. Kurallar [G6]:
- Geçerli turdaki her adımın ilk `functionCall` parçası kendi imzasını taşımalı; eksikse 400 (“Function call `FC1` in the `1.` content block is missing a `thought_signature`”).
- Paralel çağrıda imza yalnız ilk çağrıda; geri gönderim sırası “FC1+imza, FC2, FR1, FR2” — iç içe (“FC1, FR1, FC2, FR2”) 400.
- Modelin üretmediği (istemcinin eklediği ya da başka modelden gelen) çağrılar için sahte imza `"skip_thought_signature_validator"` ya da `"context_engineering_is_the_way_to_go"`.
- Akışta, çağrı içermeyen yanıtta imza boş metinli bir parçada gelebilir; `finish_reason` gelene kadar okunmalı.

**Akış (yerel).** `alt=sse` ile her olay `data: <GenerateContentResponse>`; `[DONE]` yok; `usageMetadata` yanıtta [G3]. Referansta argümanı parçalayan bir alan yok (`functionCall.args` nesne); çağrının tek parça geldiği sonucu buradan çıkarıldı, ayrıca doğrulanmadı.

**Interactions API (özet).** `POST https://generativelanguage.googleapis.com/v1beta/interactions` (v1 sürümü de var) [G2][G1]. Alanlar: `model`, `input`, `tools` (`{"type":"function","name","description","parameters"}`), `generation_config` (içinde `tool_choice`, `thinking_level`, `max_output_tokens` …), `stream`, `store`, `previous_interaction_id` [G7][G2]. Yanıt `steps` dizisi (`function_call`, `function_result`, `thought`, `model_output` …); sonuç `{"type":"function_result","call_id":"…","name":"…","result":[{"type":"text","text":"…"}]}` [G7]. Akış olayları `interaction.created`, `step.start`, `step.delta` (argümanlar `partial_arguments` ile parçalı), `step.stop`, `interaction.status_update`, `interaction.completed` [G7]. Varsayılan `store=true` (ücretli katmanda 55 gün, ücretsizde 1 gün saklama); `store=false` ile durumsuz [G2]. Durumsuz modda düşünce imzaları ayrı `thought` adımlarında taşınır, tek tek parçalara iliştirilmez [G6i].

**OpenAI uyumlu uç.** `https://generativelanguage.googleapis.com/v1beta/openai/` + `chat/completions`; `Authorization: Bearer GEMINI_API_KEY`; akış, işlev çağrısı, görsel, model listeleme destekli; **“OpenAI kütüphaneleri desteği hâlâ beta”** [G8]. `reasoning_effort` eşlemesi: Gemini 4'te `minimal`/`low`→`low`, `medium`, `high`, `xhigh` (yalnız `gemini-4-hydrogen`); Gemini 3 ve 4'te düşünme kapatılamaz; Gemini'ye özgü alanlar `extra_body.google` (ör. `thinking_config`, `cached_content`) [G8]. İmzalar uyumlu uçta “destekleniyor” deniyor [G8], ama alan adı güncel resmî sayfalarda yok. Topluluk raporlarına göre imza `tool_calls[].extra_content.google.thought_signature`'da geliyor ve standart OpenAI istemcileri bilinmeyen alanı atınca sonraki turda 400 alınıyor [T1][T2]; aynı modelin doğrudan uyumlu uçta çok turlu araç kullanımında kararsız davrandığı [T3] ve Vertex uyumlu ucunda yeni Flash modellerde araç çağrısının çalışmadığı [T4] bildiriliyor — doğrulanmadı.

**Hatalar ve sınır.** `generateContent` hata gövdesi: `{"error":{"code":400,"message":"…","status":"INVALID_ARGUMENT","details":[…]}}` [G10]. `429 RESOURCE_EXHAUSTED` ve `503 UNAVAILABLE` üstel geri çekilmeyle yeniden denenir; 400, 402, 403 denenmez [G5]. Interactions hata kodları küçük harfli: `invalid_request`, `authentication` (401), `payment_required` (402), `permission_denied` (403), `model_not_found` (404), `rate_limit_exceeded` / `quota_exceeded` (429), `service_unavailable` (503), `deadline_exceeded` (504) … [G9]. Sınırlar RPM, TPM, RPD ve **API anahtarı başına değil proje başına** [G11]; hız sınırı başlıkları belgede görülmedi.

**Modeller.** `gemini-4-carbon` (en güçlü akıl yürütme), `gemini-4-argon` (Carbon'dan hızlı, tur başına daha az belirteç), `gemini-4-hydrogen` (hızlı, gündelik kodlama ve ajan işleri). Üçü de 1.048.576 bağlam (girdi+çıktı ortak), çıktı “65K / 1M” (uzun çözümleme), metin/görsel/ses/video/PDF girdi, işlev çağrısı ve düşünme [G12]. Daha eski ucuz seçenekler: `gemini-3.8-flash`, `gemini-3.5-flash-lite`, `gemini-3.1-flash-lite` [G12]. Fiyatlar bu çalışmada okunmadı. Liste: `GET https://generativelanguage.googleapis.com/v1beta/models` (`pageSize`, `pageToken` → `nextPageToken`; `name` = `models/{model}`, `supportedGenerationMethods`) [G13].

**Önbellek, görsel.** Örtük önbellek Gemini 2.5 ve sonrası için varsayılan açık, tasarruf garantisi yok; açık önbellek Gemini 4'te desteklenmiyor [G14]. Görsel: `inline_data` (`mime_type`, `data`); satır içi verilerle istek en çok 20 MB, büyükler için Files API [G15].

**Veri.** Ücretsiz (Unpaid) hizmette — AI Studio ve ücretsiz kota dahil — içerik Google ürünlerini geliştirmek için kullanılır, insan değerlendiriciler okuyabilir. Ücretli hizmet (faturalı Cloud projesi) için istem ve yanıt ürün geliştirmede kullanılmaz, yalnız kötüye kullanım tespiti için sınırlı süre kaydedilir. AEA, İsviçre ve Birleşik Krallık'taki kullanıcılara yalnız ücretli hizmetle uygulama sunulabilir [G16].

### 5. Moonshot Kimi

**Uç ve kimlik.** Belge alanı `platform.kimi.ai` (`platform.moonshot.ai` buraya yönleniyor). Uluslararası: `https://api.moonshot.ai/v1` (`/chat/completions`, `/responses`), Anthropic biçimi `https://api.moonshot.ai/anthropic` (`/messages`) [K1]; Çin: `https://api.moonshot.cn/v1`, `https://api.moonshot.cn/anthropic` [K2]. `Authorization: Bearer $MOONSHOT_API_KEY`; `thinking` gibi Kimi'ye özgü alanlar OpenAI SDK'da `extra_body` ile [K1].

**Modeller.** `kimi-k3.1-pro` (amiral; 1M bağlam; görsel anlama), `kimi-k3.1-pro-highspeed` (aynı zekâ, 2–3 kat hız), `kimi-k3` (önceki amiral). `kimi-k2.7-code` ve `kimi-k2.6` 1 Kasım'dan sonra kaydolanlara kapalı, Aralık 2026 sonunda kaldırılması bekleniyor; `kimi-k2-*`, `moonshot-v1-*`, `kimi-latest` kullanımdan kalktı [K3]. Fiyat sayfası okunmadı.

**Parametreler (K3 serisi).** `thinking` desteklenmiyor; `reasoning_effort` `low|high|max` (varsayılan `max`), efor değiştirmek önek önbelleğini bozar; `tool_choice` `auto|none|required`; **`temperature` 1.0, `top_p` 0.95, `n` 1, `presence_penalty`/`frequency_penalty` 0'da sabit — başka değer hata döner, hiç gönderilmemeli** [K4]. Belirli işlevi zorlayan `tool_choice` nesnesi düşünme açıkken 400 (`tool_choice 'specified' is incompatible with thinking enabled`) [K5].

**Düşünme.** `reasoning_content` alanı `content` ile aynı düzeyde; araç döngüsünde ve çok turlu konuşmada asistan mesajı `reasoning_content` dahil **olduğu gibi** geri gönderilmeli (K3'te “Preserved Thinking” hep açık); `max_tokens >= 16000` ve akış öneriliyor; `reasoning_content` + `content` toplamı `max_tokens`'a sayılır; akışta `reasoning_content` hep `content`'ten önce gelir [K6].

**Araç ve akış.** OpenAI biçimi. Belge örneğinde çağrı kimliği `"search:0"` biçiminde; `tool` mesajı `role`, `tool_call_id` ve `name` taşır; her `tool_call`'a bir `tool` mesajı düşmeli, sayı ya da kimlik tutmazsa hata; `finish_reason=tool_calls` iken `content` boş olmayabilir. Akışta ilk parçada `id` ve `function.name`, sonra yalnız `arguments`; birden çok çağrıda `index` [K7]. Kullanım bilgisi için `stream_options={"include_usage": True}`; örnek kod `usage`'ı hem `chunk.usage`'da hem `choices[0].usage`'da arıyor [K8].

**Hatalar ve sınır.** Gövdede `error.type`: 400 `invalid_request_error`, `content_filter`; 401 `invalid_authentication_error`, `incorrect_api_key_error`; 403 `permission_denied_error`; 404 `resource_not_found_error`; 429 `engine_overloaded_error` (`Retry-After`), `exceeded_current_quota_error`, `rate_limit_reached_error` (eşzamanlılık / RPM / TPM / TPD); 499 / 500 / 503 / 504 [K9]. Sınır aşımında 429 ile `X-RateLimit-Limit`, `X-RateLimit-Remaining`, `X-RateLimit-Reset` [K10].

**Liste ve önbellek.** `GET /v1/models` → `context_length`, `supports_image_in`, `supports_video_in`, `supports_reasoning` [K11]. Önek önbelleği kendiliğinden; önbellek yazımı ayrı ücretli; TTL `5m` (varsayılan) ya da `1h` (`prompt_cache_options`) [K12].

**Veri.** Platform gizlilik politikası (Moonshot AI Pte. Ltd., Singapur) bilgilerin hizmeti geliştirmek, “makine öğrenmesi dahil temel teknolojiyi eğitmek ve iyileştirmek” için kullanılabileceğini söylüyor [K13]. Kimi yardım merkezindeki API veri güvenliği sayfasının ise API verisinin eğitimde kullanılmadığını söylediği bildiriliyor [K14, açılmadı]. Çelişkili; kurum verisi gönderilecekse yazılı teyit gerekir.

### 6. Z.ai / Zhipu GLM

**Uç ve kimlik.** Uluslararası `https://api.z.ai/api/paas/v4/chat/completions` [Z1]; Çin anakarası (BigModel) `https://open.bigmodel.cn/api/paas/v4/chat/completions` [Z2]. `Authorization: Bearer <anahtar>`; örnekte ayrıca `Accept-Language: en-US,en` [Z1]. GLM Coding Plan'ın ayrı ucu var; 16 Ekim 2026'dan beri anahtarları standart API anahtarından ayrı [Z1][Z3].

**Modeller.** `glm-5.3` (yalnız metin; 1M bağlam; 128K çıktı; düşünme yalnız `enabled`; $1,4 / $4,4), `glm-5.3-flash` ($0,15 / $0,50) ve `glm-5.3-flashx` ($0,37 / $1,25); Flash ailesi görsel/video/dosya girdi alır, 1M bağlam [Z4][Z5][Z6]. Eski: GLM-5.2, 5.1, 5, 4.7, 4.6 [Z6].

**İstek.** OpenAI benzeri; `thinking: {"type":"enabled","clear_thinking":…}`; `reasoning_effort` (GLM-5.3: `low|high|max`, varsayılan `max`); `tools` en çok 128 işlev; **`tool_choice` yalnız `auto`**; `tool_stream` (bool, varsayılan `false`) araç çağrısı akışı için; `do_sample`; `max_tokens` [Z7][Z11]. GLM-5.3'te düşünme kapatılamaz: `disabled` yerine `enabled` + `reasoning_effort: "low"` [Z4].

**Yanıt ve düşünme.** `reasoning_content`; `finish_reason`: `stop`, `tool_calls`, `length`, `sensitive`, `model_context_window_exceeded`, `network_error`; akış `data: [DONE]` ile biter; `usage.prompt_tokens_details.cached_tokens` [Z7]. “Interleaved thinking” varsayılan: araçla kullanımda düşünce blokları açıkça korunup araç sonuçlarıyla birlikte geri gönderilmeli. “Preserved thinking” Coding Plan ucunda varsayılan açık, standart uçta kapalı; korunan `reasoning_content` blokları özgün sırayla, değiştirilmeden gönderilmeli [Z8]. `tool_stream=True` ile akışta `delta.reasoning_content`, `delta.content`, `delta.tool_calls` [Z12].

**Hatalar.** HTTP durumu + gövdede iş kodu: `{"error":{"code":"1214","message":"…"}}`. 401 (1000–1005), 400 (1210–1215 parametre, 1261 istem çok uzun, 1301 güvensiz içerik), 429 (1113 bakiye, 1302 hız sınırı, **1305 aşırı yük**, 1308 ve sonrası kullanım sınırları), 500 (1200, 1230, 1234 ağ hatası) [Z9]. Aşırı yükün 429 ile gelmesi, “429 = bekle” mantığıyla uyumlu ama hız sınırından ayırt etmek için gövde kodu okunmalı.

**Liste.** Model listeleme ucu belgede bulunamadı. Anahtarsız `GET https://api.z.ai/api/paas/v4/models` 401 döndü; uç var olabilir ama belgelenmemiş (doğrulanmadı). Statik liste kullanılmalı.

**Veri.** Gizlilik politikası (JINGSHENG HENGXING TECHNOLOGY PTE. LTD., Singapur) bilgileri hizmeti işletmek, geliştirmek ve iyileştirmek için kullandığını; saklama süresinin amaca bağlı olduğunu söylüyor [Z10]. API girdisinin eğitimde kullanılıp kullanılmadığı açıkça yazmıyor (doğrulanmadı).

### 7. Mistral

**Uç ve kimlik.** `POST https://api.mistral.ai/v1/chat/completions`, `Authorization: Bearer $MISTRAL_API_KEY` [M1]; bölgesel uçlar `https://api.eu.mistral.ai`, `https://api.us.mistral.ai` (%10 ek ücret) [M2]. Belge dizininde ayrıca bir Responses API sayfası var [M10].

**Modeller.** `mistral-large-4` (GA; 1M bağlam; çok kipli; takma ad `mistral-large-latest`), `mistral-small-2603` (Mistral Small 4; 256k), `ministral-14b-2512` / `ministral-8b-2512` / `ministral-3b-2512` (256k). `mistral-medium-3-5` 31 Ekim 2026'da kullanımdan kalktı. Mistral üzerinden üçüncü taraf modeller de sunuluyor (`zai-glm-5-3`, `deepseek-v4-1-flash-1017`) [M3][M4]. Fiyat sayfası okunmadı.

**İstek.** `tool_choice` `auto|none|any|required` ya da `{"type":"function","function":{"name":"…"}}`; `parallel_tool_calls` (varsayılan `true`); `reasoning_effort` (`none`…`xhigh`, modele göre; Large 4 `low|medium|high`); `prompt_cache_key` (önbellekten okunan belirteç %10 fiyat); `response_format` (`json_object`, `json_schema`); `temperature` önerisi 0.0–0.7, varsayılanı modele göre değişir [M1][M6]. Akış `data: [DONE]` ile biter [M1].

**Yanıt ve araç.** `tool_calls[]` (`id`, `type`, `function{name, arguments}`, `index`); OpenAPI'ye göre `arguments` **nesne ya da metin** olabilir; `finish_reason`: `stop`, `length`, `model_length`, `error`, `tool_calls`; araç mesajı `role: "tool"`, `name`, `content`, `tool_call_id` [M5][M4]. Belge örneğindeki çağrı kimliği 9 karakter (`D681PevKs`) [M4]; topluluk kaynaklarına göre Mistral kimliğin `^[a-zA-Z0-9]{9}$` olmasını şart koşuyor ve başka sağlayıcıdan gelen kimlikler hata veriyor [T5][T6] (resmî belgede bulunamadı).

**Akıl yürütme.** Düşünen modellerde `message.content` metin değil **parça dizisi**: `{"type":"thinking","thinking":[{"type":"text","text":"…"}]}` + `text` parçası; çok turlu kullanımda ThinkChunk dahil asistan mesajı aynen geri gönderilmeli [M6].

**Sınır, hata, liste.** Sınırlar çalışma alanı düzeyinde: saniyedeki istek (RPS) ve dakikadaki belirteç [M7]. Hata gövdesi için yalnız OpenAPI'deki 422 `HTTPValidationError` görüldü; diğer kodlar ve başlıklar belgede ayrıntılanmamış (doğrulanmadı). Liste: `GET /v1/models` [M5].

**Veri.** ZDR ücretli planlarda ayrı bir denetim; eğitimden çıkma ayrı ayar: Admin → Privacy → “Anonymous improvement data”; API ve Vibe ayarları ayrı [M8][M9]. Okunan sayfa varsayılanın ne olduğunu söylemiyor.

### 8. xAI (Grok)

**Uç ve kimlik.** Belgeler şirketi “SpaceXSI (xAI)” diye anıyor. `https://api.x.ai/v1` (`/responses`, `/chat/completions`); bölgesel uç ör. `https://us.api.x.ai/v1` [X3]; `Authorization: Bearer`.

**Hangisi.** Responses öneriliyor; Chat Completions “legacy” (karşılaştırma tablosunda “Deprecated”): akıl yürütme içeriği döndürmez, yeni özellikler önce Responses'a gelir. Responses varsayılan saklanır (`store` varsayılan `true`, 30 gün) [X1][X2].

**Modeller.** `grok-4.7` (500k bağlam; 200k altı istemde $2 / $6, önbellekli girdi $0,50), `grok-4.6`, `grok-4.5`, `grok-4.3` (1M; $1,25 / $2,50), `grok-4.20-0309-reasoning` ve `-non-reasoning` (1M; $1,25 / $2,50), `grok-build-0.1` (256k; $1 / $2); 200k ve üstü istemde fiyat iki katı; takma adlar `<modelname>` / `<modelname>-latest` önerilir [X4]. Liste: `GET /v1/models` (`context_length` …), `GET /v1/models/{model_id}` [X9].

**Araç ve akıl yürütme.** `tool_choice` `auto|required|none`; paralel çağrı varsayılan açık (`parallel_tool_calls: false` kapatır); istek başına en çok 350 araç; **akışta işlev çağrısı parçalanmadan tek parçada gelir**; sonuç `{"type":"function_call_output","call_id":"…","output":"…"}` [X5]. `grok-4.7`/`4.6`/`4.5` `reasoning_effort` destekler (varsayılan `high`, kapatılamaz; `xhigh` 4.6 ve sonrası); akıl yürütme modellerinde `presencePenalty`, `frequencyPenalty`, `stop` hata döndürür; şifreli akıl yürütme `include: ["reasoning.encrypted_content"]` ile, `grok-4.7` her yanıtta zaten döndürür [X6].

**Hata ve sınır.** 400, 401, 403, 404, 405, 415, 422, 429, 5xx [X7]. Sınır modele göre RPS ve TPM; aşımda 429; önbellekli belirteçler de TPM'e sayılır [X8]. Başlık adları belgede görülmedi. Görsel (Chat Completions): `jpg/jpeg`, `png`; `detail` `auto|low|high` [X1].

**Veri.** Açık izin olmadan API girdi/çıktısıyla eğitim yapılmaz; istek ve yanıtlar kötüye kullanım denetimi için 30 gün şifreli saklanır; ekip düzeyinde ZDR var (açıkken durumlu Responses gibi özellikler kapanır) [X10].

### 9. Alibaba Qwen (Model Studio / DashScope)

**Uç ve kimlik.** OpenAI uyumlu uç artık çalışma alanına özel: Singapur `https://{WorkspaceId}.ap-southeast-1.maas.aliyuncs.com/compatible-mode/v1`, ABD (Virginia) `…us-east-1…`, Pekin `https://{WorkspaceId}.cn-beijing.maas.aliyuncs.com/compatible-mode/v1`, Hong Kong `…cn-hongkong…`, Tokyo `…ap-northeast-1…`; eski `dashscope` alan adları hâlâ kullanılabilir [Q1][Q2] (arama sonucuna göre Singapur için `https://dashscope-intl.aliyuncs.com/compatible-mode/v1`). Anahtar bölgeye özgü; yanlış bölge → 401 `invalid_api_key`. `Authorization: Bearer $DASHSCOPE_API_KEY` [Q1]. (Uyumluluk sayfası 13 Ekim 2026 tarihli ve Pekin satırında Singapur adresini yinelemiş; doğru Pekin alanı işlev çağrısı sayfasında [Q2].)

**Modeller.** Önerilen liste (12 Kasım 2026): `qwen4-max-preview` (Token Plan), `qwen3.7-plus`, `qwen3.8-flash` [Q3]; işlev çağrısı örnekleri `qwen3.8-max` kullanıyor [Q2]. Düşünme varsayılan açık: `qwen3.8-max`, `qwen3.8-flash`, `qwen3.7-plus` ve diğerleri [Q4]. Bağlam pencereleri ve fiyatlar resmî sayfada okunamadı.

**Düşünme.** `enable_thinking` (Python SDK'da `extra_body`), `thinking_budget`, `preserve_thinking` (geçmiş `reasoning_content`'in okunması); çıktı `reasoning_content`; bazı açık kaynak modeller düşünme modunda yalnız akışla çalışır (“parameter.enable_thinking only support stream call”) [Q4].

**Araç.** OpenAI biçimi; `tool_choice` `auto` (varsayılan), `none`, belirli işlev nesnesi; **`"required"` Qwen modellerinde desteklenmiyor**; `parallel_tool_calls` varsayılan `true`; akışta ilk parçada `index`, `id`, `name`, sonrakilerde `arguments` parçaları (Omni'de ad ve argüman ayrı parçalarda); sonuç `role: "tool"` + `tool_call_id`, içerik metin ya da `text`/`image_url` dizisi; sonuçları özetletirken `tool_choice` kaldırılmalı, yoksa yeni çağrılar gelebilir [Q2]. Uyumluluk sayfası (13 Ekim) “`tools` `stream=True` ile kullanılamaz” diyor, işlev çağrısı sayfası (11 Kasım) akışlı araç çağrısını anlatıyor; güncel olan ikincisi.

**Akış, hata, liste.** `stream_options={"include_usage": True}` ile `usage`, `choices` dizisi boş son parçada [Q1]. Hatalar: 400 `InvalidParameter`, 401 `InvalidApiKey`/`invalid_api_key`, 403 `AccessDenied`, 404 `ModelNotFound`, 429 `Throttling.RateQuota`, `Throttling.AllocationQuota`/`insufficient_quota`, 5xx `InternalError`/`ServiceUnavailable`; içerik denetimi `DataInspectionFailed`/`data_inspection_failed`; JSON gövde şeması sayfada yok [Q5]. OpenAI uyumlu `GET …/models` ucu doğrulanamadı; statik liste.

**Veri.** Arama sonuçlarındaki resmî sayfalara göre API girdi/çıktısı eğitimde kullanılmıyor; uluslararası modun merkezi Singapur; ZDR kurumsal müşterilere [Q6, sayfa açılmadı].

### 10. OpenRouter

**Uç ve kimlik.** `https://openrouter.ai/api/v1` + `/chat/completions`, `/responses`, `/messages` [R1][R2]. `Authorization: Bearer <OPENROUTER_API_KEY>` (anahtar `sk-or-v1-` ile başlar); Anthropic SDK'lı istemciler `x-api-key`; isteğe bağlı `HTTP-Referer` ve `X-OpenRouter-Title` (`X-Title` de kabul) [R1].

**Araç.** Araç arayüzünü modeller ve sağlayıcılar arasında standartlaştırır (OpenAI biçimi); `tools` her istekte (sonuçların gönderildiği istek dahil) bulunmalı; araç destekleyen modeller `supported_parameters=tools` süzgeciyle bulunur [R7][R4].

**Akış.** SSE; bağlantı zaman aşımına düşmesin diye yorum satırları `: OPENROUTER PROCESSING`; elle ayrıştıran `:` ile başlayan satırları atlamalı [R2]. Chat Completions'ta her akış `[DONE]`'dan önce `usage` taşıyan ek parçayla biter; bu parçada (OpenAI'nin boş `choices`'ının aksine) bir `choice` vardır ve `finish_reason` iki kez görünür [R2]. Yanıt başladıktan sonraki hata HTTP 200 içinde gelir: Chat Completions'ta üst düzey `error` alanı ve `finish_reason: "error"` taşıyan parça; Responses'ta `response.failed`. İlk ve tek olay bu hata olabilir [R2].

**Hata.** `{"error":{"code":…,"message":"…","metadata":{…}}}`; 400, 401, 402 (kredi yetersiz), 408, 429, 502 (model/sağlayıcı hatası), 503 (uygun sağlayıcı yok); `Retry-After` 429, 503 ve uçuştaki bütçe kaynaklı 402'de [R3][R2].

**Akıl yürütme ve liste.** `reasoning` parametresi (`effort`, `max_tokens`, `enabled`, `exclude`); Chat Completions'ta önceki akıl yürütme `message.reasoning` (metin) ya da `message.reasoning_details` (dizi; şifreli/özetli türler için) olarak geri verilir — araç çağrısında özellikle önemli [R5]. Liste: `GET https://openrouter.ai/api/v1/models`; `supported_parameters=tools` süzgeci; modelde `reasoning` nesnesi (`supported_efforts`, `default_effort` …) [R4][R5].

**Veri.** OpenRouter, kullanıcı açıkça seçmedikçe (özel kayıt; ürün geliştirme izni varsayılan kapalı) istem ve yanıtları saklamaz; CSAM taraması istisna; alttaki sağlayıcının kayıt politikası ayrı ve yönlendirme ayarlarıyla süzülebilir [R6][R8].

### 11. Ollama (yerel)

**Uç.** Yerel `http://localhost:11434/v1/` (`/chat/completions`, `/responses` — yalnız durumsuz, `/models`); anahtar “gerekli ama yok sayılır”. Bulut: `https://ollama.com/v1`, `OLLAMA_API_KEY` [L1].

**Destek.** `/v1/chat/completions`'ta `tools` var, **`tool_choice` yok**; `stream_options.include_usage` var; `reasoning_effort`/`reasoning` değerleri modele göre [L1]. Yerel `/api/chat` ucu SSE değil **NDJSON** (`application/x-ndjson`) akıtır; orada araç çağrısı `arguments` **nesne**, sonuç `{"role":"tool","tool_name":"…","content":"…"}`; akışta `thinking`, `content`, `tool_calls` toplanıp sonuçlarla birlikte geri gönderilir [L2][L3].

**Hata.** `{"error":"…"}`; 400, 404, 429, 500, 502; akış ortasındaki hata ayrı bir NDJSON satırıdır, HTTP durumu değişmez [L4].

**Bağlam (önemli).** Varsayılan bağlam VRAM'e göre: <24 GiB → 4k, 24–48 GiB → 32k, ≥48 GiB → 256k; bulut modelleri en büyük bağlamda [L5]. OpenAI uyumlu API'den bağlam boyu ayarlanamaz; `Modelfile`'da `num_ctx` ya da sunucuda `OLLAMA_CONTEXT_LENGTH` gerekir [L1][L5]. 4k bağlam, araç şemaları + çizim özeti için yetersiz kalabilir.

**Liste.** `GET /api/tags` ya da `GET /v1/models` [L6][L1].

### 12. LM Studio (yerel)

**Uç.** `http://localhost:1234/v1`: `GET /v1/models`, `POST /v1/responses`, `POST /v1/chat/completions`, `POST /v1/embeddings`, `POST /v1/completions` (eski); ayrıca Anthropic uyumluluğu (Messages) bölümü var [L7].

**Kimlik.** Varsayılan kimlik doğrulama yok; 0.4.0 ve sonrasında Server Settings'ten açılır, `Authorization: Bearer <token>` [L9].

**Araç.** OpenAI biçimi; yanıtta `tool_calls`, `finish_reason: "tool_calls"`, `arguments` JSON metni; akışta `delta.tool_calls` parçalı. “Native” destekli modeller (uygulamada çekiç rozeti) daha iyi; “default” destekte LM Studio kendi sistem istemini ekler, `tool` mesajlarını `user`'a çevirir, çıktıyı ayrıştıramazsa çağrı `message.content`'e düz metin olarak düşer. `tool_choice` ve paralel çağrı belgede geçmiyor [L8].

## Soyutlama önerisi

### Dört protokol uyarlayıcısı

| Uyarlayıcı | Sağlayıcılar | Gerekçe |
|---|---|---|
| `anthropic_messages` | Anthropic | Yerel; düşünme imzaları, `tool_result` blok kuralları ve adlandırılmış SSE başka hiçbir biçime birebir oturmuyor [A4][A7][A10]. |
| `openai_responses` | OpenAI, xAI | OpenAI amiral modelleri araç için Responses istiyor [O1][O3]; xAI'de Chat Completions eski ve akıl yürütme döndürmüyor [X2]. `store: false` + `include: ["reasoning.encrypted_content"]`; dönen `reasoning` öğeleri aynen geri gönderilir [O2][X6]. |
| `openai_chat` | DeepSeek, Kimi, Z.ai GLM, Mistral, Qwen, OpenRouter, Ollama, LM Studio | Hepsinin birincil, belgeli yolu Chat Completions. Farklar bir **sağlayıcı profili** tablosuyla yönetilir (aşağıda). |
| `gemini_native` | Gemini | `streamGenerateContent?alt=sse`. İmza kuralları açık [G6]; uyumlu uç beta ve imza alanı belgesiz [G8][T1]. Google `generateContent`'i “legacy ama tam destekli” sayıyor; Interactions API ikinci aşamada eklenebilir (argüman akışı `step.delta` ile parçalı, imzalar ayrı `thought` adımlarında) [G1][G2][G7]. |

Ek notlar:
- **Gemini OpenAI uyumlu ucu araçlı ajan için önerilmez:** resmî olarak beta [G8]; imza alanı güncel resmî belgede yok, yalnız topluluk raporunda (`extra_content.google.thought_signature`) [T1][T2]; çok turlu araç kullanımında 400 ve kararsızlık raporları var [T3][T4]. En fazla “deneysel” seçenek olarak, araçsız sohbete bırakılabilir.
- DeepSeek, Kimi, OpenRouter, Ollama ve LM Studio ayrıca Anthropic ve/veya Responses uyumlu uç sunuyor [D1][K1][R1][L1][L7]; ama her uyum katmanının kendi boşlukları var (ör. Ollama'da durumlu Responses yok [L1]). Başlangıçta yalnız Chat Completions yolu kullanılmalı; DeepSeek'te modelin üretmediği araç çağrısını geçmişe eklemek gerekirse Anthropic/Responses ucu çare [D2].

### Sağlayıcı profili (Chat Completions ve diğerleri için)

Her sağlayıcı için kodda sabit bir profil:

| Alan | Örnek değerler |
|---|---|
| `max_tokens_field` | `max_tokens` (Anthropic, DeepSeek, Kimi, GLM, Mistral), `max_completion_tokens` (OpenAI CC), `max_output_tokens` (Responses), `generationConfig.maxOutputTokens` (Gemini) [A3][D3][O8][O4][G3] |
| `system_placement` | üst düzey `system` (Anthropic), `instructions` (Responses), `systemInstruction` (Gemini), `system`/`developer` mesajı (CC) [A3][O4][G3][O8] |
| `sends_sampling` | varsayılan **hayır**; yalnız profilde açıkça izinliyse `temperature` [A2][K4][G5][X6] |
| `tool_choice_values` | Anthropic 5.5 ailesi: `auto`, `none`; GLM: `auto`; Qwen: `auto`, `none`, işlev; Ollama: yok [A4][Z11][Q2][L1] |
| `reasoning_echo` | `thinking_blocks` (Anthropic), `reasoning_items` (OpenAI/xAI), `thought_signature` (Gemini), `reasoning_content` (DeepSeek/Kimi/GLM/Qwen), `content_chunks` (Mistral), `reasoning_details` (OpenRouter) |
| `tool_call_id_rule` | olduğu gibi; Mistral: 9 alfasayısal (topluluk) [T5][T6] |
| `tool_msg_needs_name` | Kimi, Mistral: `name` alanı da gönderilir [K7][M4] |
| `stream_usage` | `stream_options.include_usage` gerekir (OpenAI CC, Kimi, Qwen, Ollama) [O8][K8][Q1][L1] |
| `extra_fields` | SDK'ların `extra_body`'si düz HTTP'de gövdenin üst düzey alanıdır: DeepSeek `"thinking": {…}` [D3], Qwen `"enable_thinking"` [Q4], GLM `"tool_stream": true` [Z7]. İstisna: Gemini uyumlu ucun REST örneği gövdeye gerçekten `"extra_body": {"google": {…}}` yazıyor [G8] |
| `list_models` | uç + sayfalama + yetenek alanları (aşağıda) |

### Ortak iç konuşma modeli

- Mesaj = rol + blok dizisi. Bloklar: `Text`, `Image { media_type, bytes }`, `ToolCall { iç_kimlik, sağlayıcı_kimliği, ad, argüman_json_metni }`, `ToolResult { iç_kimlik, içerik: metin/görsel, hata_mı }`, `Reasoning { sağlayıcı, model, görünen_metin?, opak: serde_json::Value }`.
- Argümanlar iç modelde **JSON metni** olarak tutulur ve ayrıca ayrıştırılır: OpenAI/DeepSeek/Kimi metin verir, Anthropic/Gemini/Ollama-yerel nesne verir, Mistral ikisinden biri [O1][A6][G3][M5].
- **Sağlayıcıdan gelen asistan turu, opak alanlarıyla birlikte ham hâliyle de saklanır** ve aynı sağlayıcı + modele geri gönderilirken yeniden kurulmaz, aynen kullanılır. Gerekçe: Anthropic imzayı önekle bağlıyor ve değişmiş bloğu reddediyor [A4]; OpenAI önek değişince reasoning öğelerini düşürüyor [O16]; GLM blokların özgün sırada ve değiştirilmeden gelmesini istiyor [Z8].
- **Sağlayıcı ya da model değişince opak akıl yürütme artıkları silinmeli.** Gemini'de başka modelden gelen çağrılar için sahte imza belgeli [G6]; DeepSeek araçlı istekte bütün turların `reasoning_content`'ini istiyor [D4] ve başka sağlayıcının turunda bu alan yok — ne olacağı belgelenmemiş. En güvenli yol: ajan değişince geçmişi özetleyip yeni oturum açmak.
- Araç çağrısı kimlikleri iç kimlikle eşlenir; aynı sağlayıcıya özgün kimlik geri verilir; Mistral'e giderken 9 alfasayısal kimlik üretilir [T5].
- Geçmiş kırpma/özetleme yalnız tur sınırında ve araç çağrısı–sonuç çiftleri bölünmeden yapılmalı: Anthropic sonucun çağrıdan hemen sonra gelmesini [A7], Kimi her çağrıya bir sonuç düşmesini [K7], Gemini paralel çağrı sonuçlarının sırayla gelmesini [G6] şart koşuyor.

### Araç şeması ilkeleri

- Ad: `^[a-zA-Z0-9_-]{1,64}$` — OpenAI'nin 64 sınırı en dar [O8]; Anthropic 128 ve `-`/`_` [A3]; Gemini 128 ama `.` ve `:` da serbest [G3]; Kimi harf/rakam/`-`/`_` öneriyor [K7]. Nokta, iki nokta, boşluk kullanılmamalı.
- Şema: kök `type: "object"`; bütün alanlar `required`; her nesnede `additionalProperties: false`; `enum`, `minimum`/`maximum`, `items` serbest. Kaçınılacaklar: `allOf`, `not`, `if`/`then`/`else`, `dependent*` (OpenAI katı mod) [O5]; `minLength`/`maxLength` (DeepSeek katı mod) [D2]; derin iç içe yapı ve büyük birleşimler (Gemini `ANY`/`VALIDATED` durum makinesi sınırları) [G4]. Bu biçim, OpenAI ve DeepSeek katı moduna ve Anthropic `strict`'ine hazırdır [O1][D2][A17].
- İsteğe bağlı alanlar: OpenAI katı modu `null` tür seçeneğini öneriyor [O1]; aynı yazımın her sağlayıcıda (özellikle Gemini OpenAPI alt kümesinde) kabul edildiği doğrulanmadı. Gemini'ye `parameters` yerine `parametersJsonSchema` gönderilmeli; böylece OpenAPI alt kümesine çeviri gerekmez [G3].
- `strict` yalnız OpenAI Responses (varsayılan dener), Anthropic ve DeepSeek `/beta`'da var [O2][A17][D2]. Her durumda Rust tarafı argümanı şemaya karşı doğrulamalı; hatalıysa çağrıyı çalıştırmadan hata sonucu (`is_error: true` ya da hata metni) döndürmeli.
- Araç sayısı: en dar belgelenen sınır GLM'nin 128 işlevi [Z7]; xAI 350 [X5]. Araç tanımları her istekte belirteç harcar [O1][A6]; araç listesi kısa ve sabit tutulmalı ki önek önbelleği tutsun (Kimi'de efor değişimi bile önbelleği bozuyor [K4]).

### `tool_choice` ve paralel çağrılar

- Taşınabilir değerler yalnız `auto` (ve `none`). “Önce çizimi oku” gibi zorunlu adımlar `tool_choice` ile değil sistem istemi ve döngü mantığıyla sağlanmalı [A4][Z11][Q2][L1][K5].
- Paralel çağrı neredeyse her yerde varsayılan açık [A8][O1][X5][M1][Q2]. Çizimi değiştiren araçlar sırayla, modelin verdiği sırada yürütülmeli; sonuçlar sağlayıcının istediği biçimde toplu döndürülmeli (Anthropic tek `user` mesajı [A8], Gemini çağrılardan sonra sırayla [G6], CC'de çağrı başına bir `tool` mesajı [K7]). Paralel çağrıyı kapatmak (`disable_parallel_tool_use`, `parallel_tool_calls: false`) her modelde garanti değil; Qwen sayfası Kimi K3'ün `false`'a rağmen çoklu çağrı döndürebileceğini söylüyor [Q2].

### Akış ayrıştırıcısı

- Tek bir SSE satır ayrıştırıcısı (`event:`, `data:`, çok satırlı `data:`, `:` ile başlayan yorumları atla, boş satırda olayı bitir) + sağlayıcıya özgü olay çözücüleri. Yorum satırları: DeepSeek `: keep-alive` [D5], OpenRouter `: OPENROUTER PROCESSING` [R2].
- Bitiş işaretleri: Anthropic `message_stop` (ve `[DONE]` yok) [A2][A10]; Responses `response.completed` / `response.incomplete` / `response.failed` [O6]; CC `data: [DONE]` [O8][D3][Z7][M1]; Gemini'de bağlantı kapanışı + `finishReason` [G3][G6].
- Akış ortası hata (HTTP 200 içinde): Anthropic `event: error` [A10]; OpenAI `error` ya da `response.failed` [O6]; OpenRouter üst düzey `error` + `finish_reason: "error"` [R2]; Ollama NDJSON `{"error":…}` satırı [L4].
- Araç argümanları: Anthropic blok `index`'iyle `partial_json` [A10]; Responses `output_index`/`item_id` ile `delta` [O1]; CC `tool_calls[].index` ile `arguments` parçaları, `id` ve `name` yalnız ilk parçada [K7][Q2]; xAI ve (okunduğu kadarıyla) Gemini yerel çağrıyı tek parçada verir [X5][G3]. Biriktirici iki durumu da kaldırmalı; JSON yalnız blok/akış bitince ayrıştırılmalı.
- Kullanım (`usage`): Anthropic `message_delta`'da birikimli [A10]; OpenAI CC son ek parçada, `choices` boş [O8]; OpenRouter son parçada bir `choice` ile [R2]; Kimi'de `choices[0].usage` de olabilir [K8]; Gemini `usageMetadata` [G3]. Çözücü `choices[0]`'ın varlığını varsaymamalı.
- Düşünce akışı: Anthropic `thinking_delta` + `signature_delta` [A10]; DeepSeek/Kimi/GLM/Qwen `delta.reasoning_content` (içerikten önce) [D4][K6][Z12][Q4]; Responses `response.reasoning_summary_text.delta` [O6]; Mistral `thinking` parçası [M6]. Arayüzde “düşünüyor” göstergesi için ayrı kanal.
- Uzun düşünme: Kimi `max_tokens >= 16000` [K6], DeepSeek düşünmede 64K varsayılan [D3]; zaman aşımları buna göre (dakikalar) ayarlanmalı ve akış hep açık kullanılmalı [A4][K6].

### Durma nedenleri → ortak sınıf

| Ortak sınıf | Anthropic | OpenAI Responses | OpenAI-CC ailesi | Gemini |
|---|---|---|---|---|
| Bitti | `end_turn`, `stop_sequence` | `response.completed` (çağrısız) | `stop` | `STOP` |
| Araç | `tool_use` | `function_call` öğeleri | `tool_calls` | `functionCall` parçaları |
| Uzunluk | `max_tokens` | `response.incomplete` | `length` | `MAX_TOKENS` |
| Bağlam taştı | `model_context_window_exceeded` | — | GLM `model_context_window_exceeded`, Mistral `model_length` | — |
| Ret/içerik | `refusal` | — | `content_filter`, GLM `sensitive` | `SAFETY`, `PROHIBITED_CONTENT` … |
| Hata/kesinti | `event: error` | `response.failed` | DeepSeek `insufficient_system_resource`/`aborted`, GLM `network_error`, Mistral `error`, OpenRouter `error` | `MALFORMED_FUNCTION_CALL`, `MISSING_THOUGHT_SIGNATURE` |
| Sürdür | `pause_turn` | — | — | `CONTINUATION` |

Kaynaklar: [A5][O6][O7][D3][Z7][M5][R2][G3].

### Hata sınıflandırması

| Sınıf | Davranış | Sağlayıcı işaretleri |
|---|---|---|
| Kimlik | Yeniden deneme yok; ayarlara yönlendir | 401 her yerde; Qwen'de yanlış bölge anahtarı da 401 [Q1]; Anthropic'te bazı kuruluş ayarlarında `Origin` başlıklı (tarayıcı) istek “CORS requests are not allowed” 401'i alır; `reqwest` bu başlığı kendiliğinden göndermez [A4] |
| İzin/bölge | Yok | 403 (OpenAI desteklenmeyen ülke [O9], Kimi IP izin listesi [K9]) |
| Geçersiz istek | Yok; ayrıntıyı göster | 400/422 (Anthropic imza/prefill/`tool_choice` hataları [A4]; DeepSeek `reasoning_content` eksik [D4]; Gemini imza eksik [G6]) |
| Kota/bakiye bitti | Yok; kullanıcıya söyle | Anthropic 429 + `enforced_spend_limit_reached`, `retry-after` yok [A11]; OpenAI 429 + `credit_balance_exhausted` vb. [O9]; DeepSeek/OpenRouter 402 [D6][R3]; Kimi `exceeded_current_quota_error` [K9]; GLM 1113 [Z9]; Gemini `quota_exceeded` [G9] |
| Hız sınırı | `retry-after`/`Retry-After` kadar bekle, yoksa üstel + rastgele | 429 (başlıklar [A11][O10][K10]) |
| Aşırı yük | Üstel geri çekilme | Anthropic 529 [A4], OpenAI 503 `server_is_overloaded` [O9], DeepSeek 503 [D6], Kimi 429 `engine_overloaded_error` [K9], GLM 429 + 1305 [Z9], Gemini 503 `UNAVAILABLE` [G5] |
| İçerik denetimi | Yok; mesajı göster | Kimi `content_filter` [K9], GLM 1301 [Z9], Qwen `DataInspectionFailed` [Q5] |
| Çok büyük | Görseli küçült/geçmişi kırp | Anthropic 413 [A4], GLM 1261 [Z9], Kimi “Input token length too long” [K9] |

### Model listesi

- Listeleme ucu olanlar: Anthropic (yetenek bilgili) [A13], OpenAI (yeteneksiz) [O12], DeepSeek [D7], Gemini (`models/` önekli ad, `supportedGenerationMethods`) [G13], Kimi (bağlam ve görsel/akıl yürütme bayrakları) [K11], Mistral [M5], xAI [X9], OpenRouter (`supported_parameters=tools`) [R4], Ollama [L6], LM Studio [L7]. Z.ai ve Qwen için belgeli uç bulunamadı.
- Listeler çoğunlukla “araç destekliyor mu?” sorusunu cevaplamıyor (OpenAI, DeepSeek, Mistral). Öneri: sağlayıcı başına küçük, elle bakımlı “araçla sınanmış modeller” listesi + listeleme ucundan gelenler + serbest model kimliği girişi. Kimlikler sık değişiyor (Kimi ve Mistral'de son iki ayda kullanımdan kalkanlar var [K3][M3]); 404/`model_not_found` hatası “model listesini yenile” önerisine bağlanmalı.

### Yerel sunucular

- Yalnız `http://localhost` / `127.0.0.1` için düz HTTP'ye izin verilmeli; anahtar alanı isteğe bağlı (LM Studio varsayılan kimliksiz [L9], Ollama anahtarı yok sayar [L1]).
- Ollama'da `tool_choice` gönderilmemeli [L1]; bağlam varsayılanı küçükse (4k) kullanıcı `OLLAMA_CONTEXT_LENGTH` için uyarılmalı [L5].
- LM Studio'da “default” araç desteğinde bozuk çağrı `content`'e düşer [L8]; içerikte araç çağrısına benzeyen JSON görülürse kullanıcıya “bu model araç kullanımını yerel olarak desteklemiyor” denmeli.

### Gizlilik bilgisi (ayarlar ekranı için)

Sağlayıcı seçilirken veri politikası bağlantısı ve tek cümlelik özet gösterilmeli: Gemini ücretsiz katmanda içerik ürün geliştirmede kullanılıyor [G16]; DeepSeek veriyi ÇHC'de işliyor ve girdiyi eğitimde kullanabiliyor [D10]; Kimi ve Z.ai metinleri çelişkili/belirsiz [K13][K14][Z10]; OpenAI Responses varsayılan saklıyor (`store: false` gönderilmeli) [O2]; xAI 30 gün saklıyor [X10].

## Tuzaklar

1. **Anthropic `anthropic-version: 2026-10-21` ile `temperature`, `top_p`, `top_k` gönderilirse her modelde 400** [A2].
2. **Anthropic 5.5 ailesi zorla araç çağrısını reddeder** (`tool_choice` `any`/`tool` → 400) [A4].
3. **Anthropic'te prefill yok** (4.6+): konuşma `user` ile bitmeli [A4].
4. **Anthropic düşünme blokları** (boş olanlar ve `redacted_thinking` dahil) araç turunda aynen geri gönderilmeli; geçmişi düzenlemek imzayı geçersiz kılar [A4].
5. Anthropic `tool_result` blokları `user` mesajının **başında**, araç çağrısının **hemen ardından**, paralel sonuçlar **tek** mesajda [A7][A8].
6. Anthropic `input_json_delta` varsayılan olarak anahtar–değer tamamlanınca gelir; olaylar arası uzun bekleme normal [A10]. `ping` ve bilinmeyen olay türleri yok sayılmalı [A10].
7. Anthropic kademe harcama tavanı 429'u yeniden denemeyle çözülmez (`retry-after` yok, `enforced_spend_limit_reached`) [A11].
8. **OpenAI Chat Completions, GPT-5.4+'ta `reasoning_effort` `none` değilken araç çağıramaz; `gpt-6-astra`/`gpt-6.1-sol`/`gpt-6.1-terra` araç için Responses ister** [O1][O2][O3].
9. OpenAI Responses **varsayılan saklar** (`store: true`); gizlilik için `store: false` + `include: ["reasoning.encrypted_content"]` ve `reasoning` öğelerini geri gönder [O2][O4].
10. OpenAI Responses `strict` vermezsen katı moda çevirmeyi dener ve sessizce katı olmayana düşebilir (`strict: false` döner); CC'de varsayılan katı değil [O2].
11. OpenAI araç tanımı Responses'ta düz (`name` üst düzeyde), CC'de `function` içinde; sonuç Responses'ta `function_call_output` + `call_id`, CC'de `role: "tool"` + `tool_call_id` [O2][O1][O8].
12. OpenAI CC'de `usage` yalnız `stream_options.include_usage` ile ve `choices`'ı boş son parçada [O8]; OpenRouter'da o parçada bir `choice` var, `finish_reason` iki kez görünür [R2].
13. OpenAI 429'un birçok anlamı var: hız (`Retry-After`), `slow_down`, kredi/harcama/kullanım sınırı; ayırt etmek için `error.code` [O9].
14. **DeepSeek araçlı istekte bütün geçmiş turların `reasoning_content`'ini ister; yoksa 400** [D4]. Araçsız istekte yok sayılır [D4].
15. DeepSeek Chat Completions modelin üretmediği araç çağrısının geçmişe eklenmesini desteklemez [D2]; akışta `: keep-alive` yorumları, akışsızda boş satırlar gelir [D5].
16. DeepSeek `finish_reason` `insufficient_system_resource` ve `aborted` gibi standart dışı değerler taşır [D3]; düşünme modunda `temperature` sessizce etkisiz [D4].
17. **Gemini 3/4'te `thoughtSignature` geri gönderilmezse 400**; paralel çağrıda imza yalnız ilkinde; sonuçlar çağrılardan sonra sırayla, iç içe değil [G6].
18. Gemini istemciye özgü (modelin üretmediği) çağrılar için sahte imza ister (`skip_thought_signature_validator`) [G6]; akışta imza boş metinli parçada gelebilir [G6].
19. Gemini işlev şeması `parameters`'ta OpenAPI alt kümesi, `parametersJsonSchema`'da JSON Schema; ikisini karıştırma [G3]. `ANY`/`VALIDATED` modunda karmaşık şema 400 [G4].
20. Gemini 3.6+ ve 4'te `temperature`/`topP` yok sayılır; Gemini 3/4'te düşünme kapatılamaz [G5][G8].
21. Gemini sınırları **proje** başına; aynı projedeki başka anahtarlar aynı kotayı tüketir [G11]. Ücretsiz katman verisi ürün geliştirmede kullanılır; AEA/İsviçre/BK kullanıcılarına yalnız ücretli hizmet [G16].
22. Gemini OpenAI uyumlu uç beta; imza alanı (`extra_content.google.thought_signature`, topluluk raporu) standart istemcilerce atılır → 400 [G8][T1][T2].
23. **Kimi K3'te `temperature`, `top_p`, `n`, cezalar sabit; başka değer hata** [K4]. Belirli işlevi zorlamak düşünme açıkken 400 [K5].
24. Kimi `tool` mesajında `name` da istiyor; çağrı kimliği `search:0` gibi; çağrı ile sonuç sayısı tutmazsa hata [K7]. Aşırı yük 429 (`engine_overloaded_error`) [K9].
25. Kimi K3'te `reasoning_effort` değiştirmek önek önbelleğini bozar; seviyeyi oturum başında sabitle [K4].
26. **GLM'de `tool_choice` yalnız `auto`**; araç argümanı akışı için `tool_stream: true` gerekir; GLM-5.3'te düşünme kapatılamaz [Z11][Z12][Z4].
27. GLM aşırı yükü (1305) ve bakiye bitişini (1113) 429 ile döndürür; ayırt etmek için gövdedeki iş kodu okunmalı [Z9].
28. GLM “Preserved thinking” standart uçta varsayılan kapalı, Coding Plan ucunda açık; aynı model iki uçta farklı davranır [Z8].
29. **Mistral araç çağrısı kimliği 9 alfasayısal karakter olmalı** (topluluk; başka sağlayıcıdan gelen `toolu_…`/`call_…` kimlikleri hata verir) [T5][T6][M4].
30. Mistral'de düşünen modelin `content`'i metin değil parça dizisi (`thinking` + `text`) [M6]; `arguments` nesne de olabilir, metin de [M5]; `tool_choice` `any` ve `required` eşanlamlı [M1].
31. xAI akışında araç çağrısı tek parça gelir [X5]; akıl yürütme modellerinde `stop`, `presencePenalty`, `frequencyPenalty` hata [X6]; Chat Completions akıl yürütme döndürmez [X2].
32. **Qwen'de `tool_choice: "required"` desteklenmiyor**; sonuçları özetletirken `tool_choice` kaldırılmazsa yeni çağrılar gelebilir [Q2]. Anahtar bölgeye özgü, yanlış bölge 401 [Q1].
33. Qwen belgeleri kendi içinde çelişiyor (akışta araç çağrısı: eski sayfa “olmaz”, yeni sayfa “olur”); güncel tarihli sayfa esas alınmalı [Q1][Q2].
34. OpenRouter akış ortası hatayı HTTP 200 içinde `finish_reason: "error"` ile verir; ilk ve tek olay olabilir [R2]. `tools` her istekte gönderilmeli [R7].
35. OpenRouter'da akıl yürütme `reasoning_details` olarak geri verilmezse araç döngüsünde akıl yürütme bağlamı kaybolur [R5].
36. **Ollama'da `tool_choice` yok; yerel `/api/*` NDJSON akıtır (SSE değil); varsayılan bağlam 4k olabilir ve OpenAI API'sinden değiştirilemez** [L1][L3][L5].
37. LM Studio “default” araç desteğinde bozuk çağrı düz metin olarak `content`'e düşer [L8].
38. Sağlayıcılar arası araç adı kesişimi `^[a-zA-Z0-9_-]{1,64}$` [O8][A3][G3].
39. Çoğu Chat Completions uyarlaması `reasoning_content`'i standart dışı alan olarak taşır; OpenAI SDK tipleri bu alanı tanımaz (Kimi belgesi `hasattr` önerir) [K6]. Kendi serde tiplerimizde bu alan açıkça olmalı.
40. Model kimlikleri hızla değişiyor: DeepSeek eski `deepseek-chat`/`deepseek-reasoner` adlarını listeden kaldırdı, Kimi ve Mistral birçok modeli kullanımdan kaldırdı [D1][K3][M3]. Sabit varsayılan model kimliği koda gömülmemeli; ayardan gelmeli.

## Doğrulanamayanlar ve açık sorular

- **Gemini OpenAI uyumlu uçta düşünce imzasının alanı:** güncel resmî belgede yok; `extra_content.google.thought_signature` yalnız topluluk raporu [T1][T2]. Uyumlu uçta araç argümanlarının parçalı mı tek parça mı aktığı da belgelenmemiş.
- **Gemini yerel akışta argümanların tek parça geldiği**, referansta parçalı alan olmamasından çıkarıldı; denenmedi [G3].
- **Mistral 9 karakter kimlik kuralı** resmî belgede bulunamadı; yalnız örnek kimlik ve topluluk kodu [M4][T5][T6]. Mistral hata kodları ve hız sınırı başlıkları belgede ayrıntılı değil.
- **Qwen:** bağlam pencereleri, fiyatlar ve OpenAI uyumlu model listeleme ucu doğrulanamadı; veri politikası sayfası açılmadı (arama özeti) [Q6].
- **Z.ai:** model listeleme ucu belgelenmemiş (anahtarsız istek 401); API verisinin eğitimde kullanımı açıkça yazmıyor [Z10].
- **Kimi:** veri politikası çelişkili (platform gizlilik metni eğitimden söz ediyor, yardım sayfasının aksini söylediği bildiriliyor; ikincisi açılmadı) [K13][K14]. Fiyat ve ucuz model sınıfı okunmadı; akışın `[DONE]` ile bittiği Kimi belgesinde görülmedi.
- **Gemini, Kimi, Mistral fiyatları** bu çalışmada okunmadı.
- **xAI ve Gemini hız sınırı başlık adları** belgede görülmedi.
- **Sağlayıcı değiştirilen konuşmada** (ör. Claude turlarından sonra DeepSeek'e araçlı istek) geçmişte `reasoning_content` olmayan turların ne yapacağı belgelenmemiş [D4]; denenmeli ya da oturum yenilenmeli.
- **Responses `function_call_output`'ta görsel** (CAD ekran görüntüsünü araç sonucu olarak verme) OpenAI ve xAI için bu çalışmada doğrulanmadı; Anthropic `tool_result` içinde görseli açıkça destekliyor [A7], Qwen `tool` içeriğinde `image_url` kabul ediyor [Q2].
- **İsteğe bağlı alanların `null` tür seçeneğiyle** (`["string","null"]`) yazımının bütün sağlayıcılarda kabul edildiği doğrulanmadı.
- Bu belgede hiçbir istek gerçek anahtarla sınanmadı; uyarlayıcılar yazılırken her sağlayıcı için en az bir araçlı, akışlı, çok turlu uçtan uca deneme gerekir.

## Kaynaklar

Bütün sayfalar 13 Kasım 2026'da okundu. Tarih gösteren sayfaların tarihi parantez içinde.

**Anthropic**
- [A1] API overview — https://platform.claude.com/docs/en/api/overview
- [A2] API versions — https://platform.claude.com/docs/en/api/versioning
- [A3] Messages API reference — https://platform.claude.com/docs/en/api/messages
- [A4] Errors — https://platform.claude.com/docs/en/api/errors
- [A5] Stop reasons — https://platform.claude.com/docs/en/build-with-claude/handling-stop-reasons
- [A6] Tool use overview — https://platform.claude.com/docs/en/agents-and-tools/tool-use/overview
- [A7] Handle tool calls — https://platform.claude.com/docs/en/agents-and-tools/tool-use/handle-tool-calls
- [A8] Parallel tool use — https://platform.claude.com/docs/en/agents-and-tools/tool-use/parallel-tool-use
- [A9] Thinking — https://platform.claude.com/docs/en/build-with-claude/thinking
- [A10] Streaming messages — https://platform.claude.com/docs/en/build-with-claude/streaming
- [A11] Rate limits — https://platform.claude.com/docs/en/api/rate-limits
- [A12] Models overview — https://platform.claude.com/docs/en/about-claude/models/overview (→ /docs/en/models/overview)
- [A13] List Models — https://platform.claude.com/docs/en/api/models/list
- [A14] Prompt caching — https://platform.claude.com/docs/en/build-with-claude/prompt-caching
- [A15] Vision — https://platform.claude.com/docs/en/build-with-claude/vision
- [A16] API and data retention — https://platform.claude.com/docs/en/manage-claude/api-and-data-retention
- [A17] Strict tool use — https://platform.claude.com/docs/en/agents-and-tools/tool-use/strict-tool-use

**OpenAI** (`platform.openai.com/docs/…` → `developers.openai.com/api/docs/…`)
- [O1] Function calling — https://developers.openai.com/api/docs/guides/function-calling
- [O2] Migrate to the Responses API — https://developers.openai.com/api/docs/guides/migrate-to-responses
- [O3] Model sayfaları — https://developers.openai.com/api/docs/models/gpt-6-astra , …/gpt-6.1-sol , …/gpt-6.1-terra , …/gpt-6-luna
- [O4] Create a response — https://developers.openai.com/api/reference/resources/responses/methods/create
- [O5] Structured outputs (Supported schemas) — https://developers.openai.com/api/docs/guides/structured-outputs
- [O6] Responses streaming events — https://developers.openai.com/api/reference/resources/responses/streaming-events
- [O7] Chat Completions streaming events — https://developers.openai.com/api/reference/resources/chat/subresources/completions/streaming-events
- [O8] Create chat completion — https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create
- [O9] Error codes — https://developers.openai.com/api/docs/guides/error-codes
- [O10] Rate limits — https://developers.openai.com/api/docs/guides/rate-limits
- [O11] Models — https://developers.openai.com/api/docs/models
- [O12] List models — https://developers.openai.com/api/reference/resources/models/methods/list
- [O13] Prompt caching — https://developers.openai.com/api/docs/guides/prompt-caching
- [O14] Images and vision — https://developers.openai.com/api/docs/guides/images-vision
- [O15] Your data — https://developers.openai.com/api/docs/guides/your-data
- [O16] Preserved reasoning — https://developers.openai.com/api/docs/guides/preserved-reasoning

**DeepSeek**
- [D1] Models & Pricing — https://api-docs.deepseek.com/quick_start/pricing
- [D2] Tool Calls — https://api-docs.deepseek.com/guides/tool_calls
- [D3] Create Chat Completion — https://api-docs.deepseek.com/api/create-chat-completion
- [D4] Thinking Mode — https://api-docs.deepseek.com/guides/thinking_mode
- [D5] Rate Limit & Isolation — https://api-docs.deepseek.com/quick_start/rate_limit
- [D6] Error Codes — https://api-docs.deepseek.com/quick_start/error_codes
- [D7] Lists Models — https://api-docs.deepseek.com/api/list-models
- [D8] Context Caching — https://api-docs.deepseek.com/guides/kv_cache
- [D9] Vision — https://api-docs.deepseek.com/guides/vision
- [D10] Privacy Policy (Last Update: Feb 10, 2026) — https://cdn.deepseek.com/policies/en-US/deepseek-privacy-policy.html

**Google Gemini** (`ai.google.dev/gemini-api/docs/…` ile `aistudio.google.com/docs/…` aynı içerik)
- [G1] Belge dizini (API seçimi notu) — https://aistudio.google.com/llms.txt
- [G2] Interactions API — https://aistudio.google.com/docs/interactions-overview
- [G3] streamGenerateContent reference — https://aistudio.google.com/api-reference/generate-content/stream-generate-content
- [G4] Function calling (generateContent) — https://aistudio.google.com/docs/function-calling?generate-content=true
- [G5] API troubleshooting — https://aistudio.google.com/docs/troubleshooting
- [G6] Thought signatures (generateContent) — https://aistudio.google.com/docs/thought-signatures?generate-content=true ; [G6i] Interactions sürümü — https://aistudio.google.com/docs/thought-signatures
- [G7] Function calling (Interactions) — https://aistudio.google.com/docs/function-calling ; Migrating to the Interactions API — https://aistudio.google.com/docs/migrate-to-interactions
- [G8] OpenAI compatibility — https://ai.google.dev/gemini-api/docs/openai
- [G9] API errors (Interactions) — https://aistudio.google.com/docs/api-errors
- [G10] Error response format (tam belge dökümündeki bölüm) — https://aistudio.google.com/llms-full.txt
- [G11] Rate limits — https://aistudio.google.com/docs/rate-limits
- [G12] Models — https://ai.google.dev/gemini-api/docs/models ; model sayfaları …/models/gemini-4-carbon , …/gemini-4-argon , …/gemini-4-hydrogen
- [G13] List models — https://aistudio.google.com/api-reference/models/list-models
- [G14] Context caching (generateContent) — https://aistudio.google.com/docs/caching?generate-content=true
- [G15] Image understanding — https://ai.google.dev/gemini-api/docs/image-understanding
- [G16] Terms of service — https://aistudio.google.com/docs/terms

**Moonshot Kimi** (`platform.moonshot.ai` → `platform.kimi.ai`)
- [K1] API Overview — https://platform.kimi.ai/docs/api/overview
- [K2] API Overview (Çin) — https://platform.moonshot.cn/docs/api/overview (→ platform.kimi.com)
- [K3] Model List — https://platform.kimi.ai/docs/models
- [K4] Model Parameter Reference — https://platform.kimi.ai/docs/api/models-overview
- [K5] Tool Choice — https://platform.kimi.ai/docs/guide/use-tool-choice
- [K6] Thinking Models — https://platform.kimi.ai/docs/guide/use-thinking-models
- [K7] Tool Calls — https://platform.kimi.ai/docs/guide/use-kimi-api-to-complete-tool-calls
- [K8] Streaming — https://platform.kimi.ai/docs/guide/utilize-the-streaming-output-feature-of-kimi-api
- [K9] Common Error Codes — https://platform.kimi.ai/docs/api/errors
- [K10] Limits — https://platform.kimi.ai/docs/pricing/limits
- [K11] List Models — https://platform.kimi.ai/docs/api/list-models
- [K12] Context caching — https://platform.kimi.ai/docs/guide/context-caching
- [K13] OpenPlatform privacy policy — https://platform.kimi.ai/docs/agreement/userprivacy
- [K14] API data security (açılmadı; arama sonucu) — https://www.kimi.com/help/kimi-api/api-data-security

**Z.ai / Zhipu**
- [Z1] API Introduction — https://docs.z.ai/api-reference/introduction
- [Z2] BigModel 对话补全 (OpenAPI `servers: https://open.bigmodel.cn/api/`) — https://docs.bigmodel.cn/api-reference/模型-api/对话补全
- [Z3] Belge dizini (API Key Management Guide notu) — https://docs.z.ai/llms.txt
- [Z4] GLM-5.3 — https://docs.z.ai/guides/llm/glm-5.3
- [Z5] GLM-5.3-Flash/FlashX — https://docs.z.ai/guides/vlm/glm-5.3-flash
- [Z6] Pricing — https://docs.z.ai/guides/overview/pricing
- [Z7] Chat Completion reference — https://docs.z.ai/api-reference/llm/chat-completion
- [Z8] Thinking Mode — https://docs.z.ai/guides/capabilities/thinking-mode
- [Z9] Errors — https://docs.z.ai/api-reference/api-code
- [Z10] Privacy Policy — https://docs.z.ai/legal-agreement/privacy-policy
- [Z11] Function Calling — https://docs.z.ai/guides/capabilities/function-calling
- [Z12] Tool Streaming Output — https://docs.z.ai/guides/capabilities/stream-tool

**Mistral**
- [M1] Chat Completion API reference — https://docs.mistral.ai/api/endpoint/chat
- [M2] Regional inference — https://docs.mistral.ai/inference/regional-inference
- [M3] Models Overview — https://docs.mistral.ai/models
- [M4] Function Calling — https://docs.mistral.ai/inference/conversations/function-calling
- [M5] OpenAPI spec — https://docs.mistral.ai/openapi.yaml
- [M6] Reasoning — https://docs.mistral.ai/inference/conversations/reasoning
- [M7] Workspace usage and limits — https://docs.mistral.ai/admin/workspaces/usage-limits
- [M8] Zero data retention — https://docs.mistral.ai/admin/monitor-comply/zero-data-retention
- [M9] Training opt-out (3 Eylül 2026 tarihli) — https://help.mistral.ai/en/articles/455207-can-i-opt-out-of-my-input-or-output-data-being-used-for-training
- [M10] Belge dizini (Responses API sayfası) — https://docs.mistral.ai/llms.txt

**xAI**
- [X1] Chat Completions (Legacy) — https://docs.x.ai/developers/model-capabilities/legacy/chat-completions
- [X2] Comparison with Chat Completions API — https://docs.x.ai/developers/model-capabilities/text/comparison
- [X3] Regional Endpoints — https://docs.x.ai/developers/advanced-api-usage/regions
- [X4] Models — https://docs.x.ai/developers/models
- [X5] Function Calling — https://docs.x.ai/developers/tools/function-calling
- [X6] Reasoning — https://docs.x.ai/developers/model-capabilities/text/reasoning
- [X7] Debugging Errors — https://docs.x.ai/developers/debugging
- [X8] Rate limits — https://docs.x.ai/developers/rate-limits
- [X9] Models reference — https://docs.x.ai/developers/rest-api-reference/inference/models
- [X10] Security FAQ — https://docs.x.ai/developers/faq/security

**Alibaba Qwen**
- [Q1] OpenAI compatible – Chat (13 Ekim 2026) — https://www.alibabacloud.com/help/en/model-studio/compatibility-of-openai-with-dashscope
- [Q2] Function calling (11 Kasım 2026) — https://www.alibabacloud.com/help/en/model-studio/qwen-function-calling
- [Q3] Models (12 Kasım 2026) — https://www.alibabacloud.com/help/en/model-studio/models
- [Q4] Deep thinking (11 Kasım 2026) — https://www.alibabacloud.com/help/en/model-studio/deep-thinking
- [Q5] Error codes (12 Kasım 2026) — https://www.alibabacloud.com/help/en/model-studio/error-code
- [Q6] Security certifications and privacy (açılmadı; arama sonucu) — https://help.aliyun.com/en/model-studio/privacy-notice

**OpenRouter**
- [R1] Authentication — https://openrouter.ai/docs/api_reference/authentication ; API Reference overview — https://openrouter.ai/docs/api_reference/overview
- [R2] Streaming — https://openrouter.ai/docs/api_reference/streaming
- [R3] Errors and Debugging — https://openrouter.ai/docs/api_reference/errors-and-debugging
- [R4] Models — https://openrouter.ai/docs/guides/overview/models
- [R5] Reasoning Tokens — https://openrouter.ai/docs/guides/best-practices/reasoning-tokens
- [R6] Data Collection — https://openrouter.ai/docs/guides/privacy/data-collection
- [R7] Tool calling — https://openrouter.ai/docs/guides/features/tool-calling
- [R8] Provider Logging — https://openrouter.ai/docs/guides/privacy/provider-logging

**Yerel sunucular**
- [L1] Ollama OpenAI compatibility — https://docs.ollama.com/api/openai-compatibility
- [L2] Ollama Tool calling — https://docs.ollama.com/capabilities/tool-calling
- [L3] Ollama Streaming — https://docs.ollama.com/api/streaming
- [L4] Ollama Errors — https://docs.ollama.com/api/errors
- [L5] Ollama Context length — https://docs.ollama.com/context-length
- [L6] Ollama List models — https://docs.ollama.com/api/tags
- [L7] LM Studio OpenAI Compatibility — https://lmstudio.ai/docs/developer/openai-compat
- [L8] LM Studio Tool Use — https://lmstudio.ai/docs/developer/openai-compat/tools
- [L9] LM Studio Authentication — https://lmstudio.ai/docs/developer/core/authentication

**Topluluk (resmî değil, sayfalar açılmadı; arama sonucu özetinden)**
- [T1] Gemini 3 Pro Preview, OpenAI uyumluluğu ve thought_signature — https://discuss.ai.google.dev/t/openai-api-compatibility-broken-due-to-thought-signature-on-gemini-3-pro-preview/109823
- [T2] Continue PR #8865 (`extra_content.google` imzasını geri gönderme) — https://github.com/continuedev/continue/pull/8865
- [T3] Gemini 3 Flash: OpenRouter çalışıyor, doğrudan uyumlu uç çok turlu araçlarda kararsız — https://discuss.ai.google.dev/t/same-gemini-3-flash-via-openrouter-works-direct-gemini-openai-compat-api-gives-unstable-hallucination-like-behavior-on-multi-turn-tools/142695
- [T4] Gemini 3.6 Flash / 3.5 Flash-Lite, Vertex OpenAI uyumlu uçta araç çağrısı — https://discuss.ai.google.dev/t/tool-calling-doesnt-work-for-new-gemini-3-6-flash-and-gemini-3-5-flash-lite-models-on-vertex-ai-enterprise-agents-platform-via-openai-compatibility-api/175801
- [T5] Vercel AI SDK issue #11802 (Mistral tool call id 9 karakter) — https://github.com/vercel/ai/issues/11802
- [T6] vLLM Mistral tokenizer/tool parser (kimliği 9 karaktere indirme) — https://docs.vllm.ai/en/latest/api/vllm/tokenizers/mistral/
