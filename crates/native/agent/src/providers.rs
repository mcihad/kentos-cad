//! The providers an agent can talk to and what the settings window offers for each (docs/adr/8056 §4): a Turkish name,
//! the default address (and the mainland China one where there is one), the wire format, suggested models, the API key's
//! environment variable and a one-line privacy note. The values are the research note's
//! (docs/research/2026-11-13-yapay-zeka-saglayicilari.md, “Karşılaştırma tablosu”, “Gizlilik bilgisi”); the model ids
//! are only suggestions, an agent's model always comes from its settings (the note's trap 40).

use serde::{Deserialize, Serialize};

/// A provider, or a kind of server, an agent talks to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProviderKind {
    #[serde(rename = "anthropic")]
    Anthropic,
    #[serde(rename = "openai")]
    OpenAi,
    #[serde(rename = "xai")]
    Xai,
    #[serde(rename = "deepseek")]
    DeepSeek,
    #[serde(rename = "gemini")]
    Gemini,
    #[serde(rename = "kimi")]
    Kimi,
    #[serde(rename = "glm")]
    Glm,
    #[serde(rename = "mistral")]
    Mistral,
    #[serde(rename = "qwen")]
    Qwen,
    #[serde(rename = "openrouter")]
    OpenRouter,
    #[serde(rename = "ollama")]
    Ollama,
    #[serde(rename = "lmstudio")]
    LmStudio,
    /// Any server speaking the Chat Completions format at an address the user gives.
    #[serde(rename = "openai-compatible")]
    OpenAiCompatible,
}

/// The format a provider is spoken to in (the research note's four adapters).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Wire {
    /// `POST /v1/messages`, named server-sent events.
    AnthropicMessages,
    /// `POST /v1/responses`, stateless (`store: false`), encrypted reasoning items echoed back.
    OpenAiResponses,
    /// `POST /chat/completions` with a quirk profile per provider.
    ChatCompletions,
    /// `POST /v1beta/models/{model}:streamGenerateContent?alt=sse`, thought signatures echoed back.
    GeminiNative,
}

/// What the settings window shows and pre-fills for a provider.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Preset {
    pub kind: ProviderKind,
    /// Its name in the window.
    pub name: &'static str,
    /// The address requests go to, up to the API's version (`https://api.openai.com/v1`); none: the user gives it.
    pub base_url: Option<&'static str>,
    /// The mainland China address, where the provider has a separate one.
    pub china_base_url: Option<&'static str>,
    pub wire: Wire,
    /// The strongest suggested model.
    pub flagship_model: Option<&'static str>,
    /// The low-cost suggestion, where the provider has one.
    pub low_cost_model: Option<&'static str>,
    /// Every suggested model, the flagship first.
    pub models: &'static [&'static str],
    /// The environment variable an API key may come from (none for local servers).
    pub key_env: Option<&'static str>,
    /// Whether a request goes nowhere without a key.
    pub needs_key: bool,
    /// A server on this computer (plain HTTP to `localhost` allowed, no key).
    pub local: bool,
    /// Whether the provider lists its models (`GET /models`); otherwise the suggestions are the list.
    pub lists_models: bool,
    /// One line on what the provider does with what it is sent.
    pub privacy: &'static str,
    /// The provider's page that says so.
    pub privacy_url: Option<&'static str>,
    /// What to know when setting it up, when there is something.
    pub note: Option<&'static str>,
}

impl ProviderKind {
    /// Every kind, in the order the settings window lists them.
    pub const ALL: [ProviderKind; 13] = [
        ProviderKind::Anthropic,
        ProviderKind::OpenAi,
        ProviderKind::Gemini,
        ProviderKind::Xai,
        ProviderKind::DeepSeek,
        ProviderKind::Mistral,
        ProviderKind::Kimi,
        ProviderKind::Glm,
        ProviderKind::Qwen,
        ProviderKind::OpenRouter,
        ProviderKind::Ollama,
        ProviderKind::LmStudio,
        ProviderKind::OpenAiCompatible,
    ];

    /// The id the agents' file keeps (`anthropic`, `openai-compatible` …).
    pub fn id(self) -> &'static str {
        match self {
            ProviderKind::Anthropic => "anthropic",
            ProviderKind::OpenAi => "openai",
            ProviderKind::Xai => "xai",
            ProviderKind::DeepSeek => "deepseek",
            ProviderKind::Gemini => "gemini",
            ProviderKind::Kimi => "kimi",
            ProviderKind::Glm => "glm",
            ProviderKind::Mistral => "mistral",
            ProviderKind::Qwen => "qwen",
            ProviderKind::OpenRouter => "openrouter",
            ProviderKind::Ollama => "ollama",
            ProviderKind::LmStudio => "lmstudio",
            ProviderKind::OpenAiCompatible => "openai-compatible",
        }
    }

    pub fn from_id(id: &str) -> Option<ProviderKind> {
        ProviderKind::ALL.into_iter().find(|k| k.id() == id.trim())
    }

    /// What the settings window offers for it.
    pub fn preset(self) -> Preset {
        match self {
            ProviderKind::Anthropic => Preset {
                kind: self,
                name: "Anthropic (Claude)",
                base_url: Some("https://api.anthropic.com/v1"),
                china_base_url: None,
                wire: Wire::AnthropicMessages,
                flagship_model: Some("claude-opus-5-5"),
                low_cost_model: Some("claude-haiku-5-5"),
                models: &[
                    "claude-opus-5-5",
                    "claude-sonnet-5-5",
                    "claude-haiku-5-5",
                    "claude-fable-5-2",
                ],
                key_env: Some("ANTHROPIC_API_KEY"),
                needs_key: true,
                local: false,
                lists_models: true,
                privacy: "API verisi açık izin olmadan model eğitiminde kullanılmaz; içerik varsayılan olarak saklanmaz \
                          (bazı modeller 30 gün saklar).",
                privacy_url: Some(
                    "https://platform.claude.com/docs/en/manage-claude/api-and-data-retention",
                ),
                note: None,
            },
            ProviderKind::OpenAi => Preset {
                kind: self,
                name: "OpenAI",
                base_url: Some("https://api.openai.com/v1"),
                china_base_url: None,
                wire: Wire::OpenAiResponses,
                flagship_model: Some("gpt-6-astra"),
                low_cost_model: Some("gpt-6-luna"),
                models: &["gpt-6-astra", "gpt-6.1-sol", "gpt-6.1-terra", "gpt-6-luna"],
                key_env: Some("OPENAI_API_KEY"),
                needs_key: true,
                local: false,
                lists_models: true,
                privacy: "API verisi açık izin olmadan eğitimde kullanılmaz; kötüye kullanım kayıtları en çok 30 gün \
                          saklanır. Yanıtlar sağlayıcıda saklanmasın diye istekler saklamasız (store: false) gider.",
                privacy_url: Some("https://developers.openai.com/api/docs/guides/your-data"),
                note: None,
            },
            ProviderKind::Xai => Preset {
                kind: self,
                name: "xAI (Grok)",
                base_url: Some("https://api.x.ai/v1"),
                china_base_url: None,
                wire: Wire::OpenAiResponses,
                flagship_model: Some("grok-4.7"),
                low_cost_model: Some("grok-4.3"),
                models: &["grok-4.7", "grok-4.6", "grok-4.3"],
                key_env: Some("XAI_API_KEY"),
                needs_key: true,
                local: false,
                lists_models: true,
                privacy: "Açık izin olmadan eğitimde kullanılmaz; istek ve yanıtlar kötüye kullanım denetimi için 30 gün \
                          şifreli saklanır.",
                privacy_url: Some("https://docs.x.ai/developers/faq/security"),
                note: None,
            },
            ProviderKind::DeepSeek => Preset {
                kind: self,
                name: "DeepSeek",
                base_url: Some("https://api.deepseek.com"),
                china_base_url: None,
                wire: Wire::ChatCompletions,
                flagship_model: Some("deepseek-v4-pro"),
                low_cost_model: Some("deepseek-flash"),
                models: &["deepseek-v4-pro", "deepseek-flash"],
                key_env: Some("DEEPSEEK_API_KEY"),
                needs_key: true,
                local: false,
                lists_models: true,
                privacy: "Veriler Çin Halk Cumhuriyeti'nde işlenir ve saklanır; girdiler model eğitiminde kullanılabilir \
                          (itiraz hakkı var).",
                privacy_url: Some(
                    "https://cdn.deepseek.com/policies/en-US/deepseek-privacy-policy.html",
                ),
                note: Some(
                    "Sohbetin ortasında başka bir ajandan DeepSeek'e geçilirse önceki turların düşünce metni olmadığı \
                     için istek reddedilebilir; o durumda yeni bir sohbet başlatın.",
                ),
            },
            ProviderKind::Gemini => Preset {
                kind: self,
                name: "Google Gemini",
                base_url: Some("https://generativelanguage.googleapis.com/v1beta"),
                china_base_url: None,
                wire: Wire::GeminiNative,
                flagship_model: Some("gemini-4-carbon"),
                low_cost_model: Some("gemini-4-hydrogen"),
                models: &["gemini-4-carbon", "gemini-4-argon", "gemini-4-hydrogen"],
                key_env: Some("GEMINI_API_KEY"),
                needs_key: true,
                local: false,
                lists_models: true,
                privacy: "Ücretsiz katmanda içerik Google ürünlerini geliştirmede kullanılır ve insanlarca okunabilir; \
                          faturalı projede kullanılmaz.",
                privacy_url: Some("https://aistudio.google.com/docs/terms"),
                note: Some(
                    "Sınırlar anahtar başına değil proje başınadır; aynı projedeki başka anahtarlar aynı kotayı kullanır.",
                ),
            },
            ProviderKind::Kimi => Preset {
                kind: self,
                name: "Moonshot Kimi",
                base_url: Some("https://api.moonshot.ai/v1"),
                china_base_url: Some("https://api.moonshot.cn/v1"),
                wire: Wire::ChatCompletions,
                flagship_model: Some("kimi-k3.1-pro"),
                low_cost_model: None,
                models: &["kimi-k3.1-pro", "kimi-k3.1-pro-highspeed"],
                key_env: Some("MOONSHOT_API_KEY"),
                needs_key: true,
                local: false,
                lists_models: true,
                privacy: "Platformun gizlilik metni verinin eğitimde kullanılabileceğini, yardım sayfası aksini söylüyor; \
                          kurum verisi göndermeden önce yazılı teyit alın.",
                privacy_url: Some("https://platform.kimi.ai/docs/agreement/userprivacy"),
                note: Some(
                    "Çin anakarası hesabının anahtarı yalnız api.moonshot.cn adresinde çalışır.",
                ),
            },
            ProviderKind::Glm => Preset {
                kind: self,
                name: "Z.ai GLM",
                base_url: Some("https://api.z.ai/api/paas/v4"),
                china_base_url: Some("https://open.bigmodel.cn/api/paas/v4"),
                wire: Wire::ChatCompletions,
                flagship_model: Some("glm-5.3"),
                low_cost_model: Some("glm-5.3-flash"),
                models: &["glm-5.3", "glm-5.3-flash", "glm-5.3-flashx"],
                key_env: Some("ZAI_API_KEY"),
                needs_key: true,
                local: false,
                lists_models: false,
                privacy: "Gizlilik politikası API girdisinin eğitimde kullanılıp kullanılmadığını açıkça söylemiyor; kurum \
                          verisi göndermeden önce teyit alın.",
                privacy_url: Some("https://docs.z.ai/legal-agreement/privacy-policy"),
                note: Some(
                    "GLM Coding Plan anahtarı standart API anahtarından ayrıdır ve bu adreste çalışmaz.",
                ),
            },
            ProviderKind::Mistral => Preset {
                kind: self,
                name: "Mistral",
                base_url: Some("https://api.mistral.ai/v1"),
                china_base_url: None,
                wire: Wire::ChatCompletions,
                flagship_model: Some("mistral-large-4"),
                low_cost_model: Some("mistral-small-2603"),
                models: &["mistral-large-4", "mistral-small-2603"],
                key_env: Some("MISTRAL_API_KEY"),
                needs_key: true,
                local: false,
                lists_models: true,
                privacy: "Eğitimden çıkma ayrı bir ayardır (Admin → Privacy); sıfır saklama ücretli planlarda ayrıca açılır.",
                privacy_url: Some(
                    "https://help.mistral.ai/en/articles/455207-can-i-opt-out-of-my-input-or-output-data-being-used-for-training",
                ),
                note: None,
            },
            ProviderKind::Qwen => Preset {
                kind: self,
                name: "Alibaba Qwen",
                // The international (Singapore) address on the dashscope domain, which the note says still works; a
                // workspace's own address (https://{WorkspaceId}.ap-southeast-1.maas.aliyuncs.com/compatible-mode/v1)
                // may be typed instead.
                base_url: Some("https://dashscope-intl.aliyuncs.com/compatible-mode/v1"),
                // The Beijing region's dashscope address.
                china_base_url: Some("https://dashscope.aliyuncs.com/compatible-mode/v1"),
                wire: Wire::ChatCompletions,
                flagship_model: Some("qwen3.8-max"),
                low_cost_model: Some("qwen3.8-flash"),
                models: &["qwen3.8-max", "qwen3.7-plus", "qwen3.8-flash"],
                key_env: Some("DASHSCOPE_API_KEY"),
                needs_key: true,
                local: false,
                lists_models: false,
                privacy: "Sağlayıcının sayfalarına göre API girdisi eğitimde kullanılmıyor (doğrulanmadı); uluslararası mod \
                          Singapur'da işler.",
                privacy_url: Some("https://help.aliyun.com/en/model-studio/privacy-notice"),
                note: Some(
                    "Anahtar bölgeye özgüdür; başka bölgenin adresi anahtarı reddeder (401). Model Studio çalışma \
                     alanınızın kendi adresini de yazabilirsiniz.",
                ),
            },
            ProviderKind::OpenRouter => Preset {
                kind: self,
                name: "OpenRouter",
                base_url: Some("https://openrouter.ai/api/v1"),
                china_base_url: None,
                wire: Wire::ChatCompletions,
                flagship_model: None,
                low_cost_model: None,
                models: &[],
                key_env: Some("OPENROUTER_API_KEY"),
                needs_key: true,
                local: false,
                lists_models: true,
                privacy: "İstem ve yanıtlar siz açmadıkça saklanmaz; isteği yanıtlayan alttaki sağlayıcının kendi \
                          politikası ayrıca geçerlidir.",
                privacy_url: Some("https://openrouter.ai/docs/guides/privacy/data-collection"),
                note: Some("Model listesinde yalnız araç kullanabilen modeller gösterilir."),
            },
            ProviderKind::Ollama => Preset {
                kind: self,
                name: "Ollama (yerel)",
                base_url: Some("http://localhost:11434/v1"),
                china_base_url: None,
                wire: Wire::ChatCompletions,
                flagship_model: None,
                low_cost_model: None,
                models: &[],
                key_env: None,
                needs_key: false,
                local: true,
                lists_models: true,
                privacy: "Model bu bilgisayarda çalışır; istekler yazdığınız adresin dışına çıkmaz.",
                privacy_url: None,
                note: Some(
                    "Varsayılan bağlam küçük olabilir (4 bin belirteç); araç tanımları sığmazsa Ollama'yı \
                     OLLAMA_CONTEXT_LENGTH ile daha büyük bağlamla başlatın.",
                ),
            },
            ProviderKind::LmStudio => Preset {
                kind: self,
                name: "LM Studio (yerel)",
                base_url: Some("http://localhost:1234/v1"),
                china_base_url: None,
                wire: Wire::ChatCompletions,
                flagship_model: None,
                low_cost_model: None,
                models: &[],
                key_env: None,
                needs_key: false,
                local: true,
                lists_models: true,
                privacy: "Model bu bilgisayarda çalışır; istekler yazdığınız adresin dışına çıkmaz.",
                privacy_url: None,
                note: Some(
                    "Araç kullanımını yerel olarak destekleyen bir model seçin; desteklemeyen modelde araç çağrısı düz \
                     metin olarak gelir ve çalışmaz.",
                ),
            },
            ProviderKind::OpenAiCompatible => Preset {
                kind: self,
                name: "OpenAI uyumlu sunucu",
                base_url: None,
                china_base_url: None,
                wire: Wire::ChatCompletions,
                flagship_model: None,
                low_cost_model: None,
                models: &[],
                key_env: None,
                needs_key: false,
                local: false,
                lists_models: true,
                privacy: "Verinin nasıl işlendiği, adresini yazdığınız sunucunun politikasına bağlıdır.",
                privacy_url: None,
                note: Some(
                    "Sunucunun /v1 ile biten adresini yazın (örneğin http://localhost:8000/v1).",
                ),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_has_a_preset_its_id_reads_back_and_keys_are_named_for_cloud_providers_only() {
        for kind in ProviderKind::ALL {
            let p = kind.preset();
            assert_eq!(p.kind, kind);
            assert_eq!(ProviderKind::from_id(kind.id()), Some(kind));
            assert_eq!(
                serde_json::to_value(kind).unwrap(),
                serde_json::Value::String(kind.id().into())
            );
            assert_eq!(p.key_env.is_some(), p.needs_key, "{kind:?}");
            assert!(
                !p.local
                    || p.base_url
                        .is_some_and(|u| u.starts_with("http://localhost"))
            );
            assert!(
                p.flagship_model
                    .is_none_or(|m| p.models.first() == Some(&m)),
                "{kind:?}"
            );
            assert!(
                p.low_cost_model.is_none_or(|m| p.models.contains(&m)),
                "{kind:?}"
            );
            assert!(!p.privacy.is_empty());
        }
        assert_eq!(ProviderKind::ALL.len(), 13);
        assert_eq!(
            ProviderKind::Anthropic.preset().wire,
            Wire::AnthropicMessages
        );
        assert_eq!(ProviderKind::Xai.preset().wire, Wire::OpenAiResponses);
        assert_eq!(ProviderKind::Gemini.preset().wire, Wire::GeminiNative);
        assert_eq!(
            ProviderKind::Glm.preset().china_base_url,
            Some("https://open.bigmodel.cn/api/paas/v4")
        );
    }
}
