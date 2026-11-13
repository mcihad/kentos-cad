//! The agents' file in a temporary folder (never the user's own): readable by the user alone, exactly one default, a
//! session key never written, a file that does not read set aside and not written over, no key in `Debug`.

use std::path::PathBuf;

use kentos_agent::{AgentProfile, KeySource, ProfileField, ProfileStore, ProviderKind};

/// A fresh folder of the test's own under the system's temporary folder.
fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kentos-agent-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

const STORED: &str = "sk-ant-STORED-0001";
const TYPED: &str = "sk-SESSION-0002";

#[test]
fn the_file_is_the_users_alone_keeps_one_default_and_never_a_session_key() {
    let dir = folder("dosya");
    let file = dir.join("yapay-zeka-ajanlari.json");
    let store = ProfileStore::at(&file);
    assert_eq!(store.path(), Some(file.as_path()));
    assert!(store.list().is_empty() && store.default_profile().is_none());
    let claude = store
        .add(AgentProfile {
            key: KeySource::Stored,
            ..AgentProfile::new(ProviderKind::Anthropic, "  Claude ", "claude-opus-5-5")
        })
        .unwrap();
    assert!(
        !claude.id.is_empty() && claude.default,
        "the first agent is the default"
    );
    assert_eq!(claude.name, "Claude");
    store.set_key(&claude.id, STORED).unwrap();
    let deepseek = store
        .add(AgentProfile::new(
            ProviderKind::DeepSeek,
            "DeepSeek",
            "deepseek-flash",
        ))
        .unwrap();
    assert_eq!(deepseek.key, KeySource::Session);
    store.set_key(&deepseek.id, TYPED).unwrap();
    assert_eq!(store.key_for(&deepseek).as_deref(), Some(TYPED));
    assert_eq!(store.key_origin(&claude), Some(KeySource::Stored));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&file).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains(STORED), "a kept key is in the file");
    assert!(!text.contains(TYPED), "a session key never is: {text}");
    // Exactly one default, moved and kept when it is removed.
    let defaults = |s: &ProfileStore| s.list().iter().filter(|p| p.default).count();
    store.set_default(&deepseek.id).unwrap();
    assert_eq!(
        (defaults(&store), store.default_profile().unwrap().id),
        (1, deepseek.id.clone())
    );
    store
        .add(AgentProfile {
            default: true,
            ..AgentProfile::new(ProviderKind::Ollama, "Yerel", "llama4")
        })
        .unwrap();
    assert_eq!(defaults(&store), 1);
    assert_eq!(store.default_profile().unwrap().name, "Yerel");
    let local = store.default_profile().unwrap();
    assert_eq!(store.remove(&local.id), Ok(true));
    assert_eq!(defaults(&store), 1, "another agent took the default");
    assert_eq!(store.remove("yok"), Ok(false));
    assert!(store.set_default("yok").is_err());
    // Read again: the kept key comes back, the session's does not.
    let again = ProfileStore::at(&file);
    assert_eq!(again.list(), store.list());
    assert_eq!(again.key_for(&claude).as_deref(), Some(STORED));
    // (The machine's own DEEPSEEK_API_KEY may answer here; the typed key never does.)
    assert_ne!(again.key_for(&deepseek).as_deref(), Some(TYPED));
    // A kept key becomes this session's: out of the file, still at hand.
    store
        .update(AgentProfile {
            key: KeySource::Session,
            ..claude.clone()
        })
        .unwrap();
    assert!(!std::fs::read_to_string(&file).unwrap().contains(STORED));
    assert_eq!(store.key_for(&claude).as_deref(), Some(STORED));
    store.forget_key(&claude.id).unwrap();
    assert_ne!(store.key_for(&claude).as_deref(), Some(STORED));
    // No key in Debug.
    let shown = format!("{store:?} {again:?}");
    assert!(!shown.contains(STORED) && !shown.contains(TYPED), "{shown}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_file_that_does_not_read_is_set_aside_and_not_written_over() {
    let dir = folder("bozuk");
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("yapay-zeka-ajanlari.json");
    std::fs::write(&file, b"{bozuk").unwrap();
    let store = ProfileStore::at(&file);
    assert!(store.list().is_empty());
    let aside = store.take_set_aside().expect("set aside");
    assert_eq!(std::fs::read(&aside).unwrap(), b"{bozuk");
    assert!(store.take_set_aside().is_none(), "said once");
    store
        .add(AgentProfile::new(
            ProviderKind::Mistral,
            "Mistral",
            "mistral-large-4",
        ))
        .unwrap();
    assert_eq!(ProfileStore::at(&file).list().len(), 1);
    assert_eq!(
        std::fs::read(&aside).unwrap(),
        b"{bozuk",
        "the set-aside file is kept as it was"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_agent_says_what_it_lacks_and_is_refused_until_it_has_it() {
    let store = ProfileStore::in_memory();
    let bare = AgentProfile::new(ProviderKind::OpenAiCompatible, " ", "");
    let fields: Vec<ProfileField> = bare.problems().into_iter().map(|(f, _)| f).collect();
    assert_eq!(
        fields,
        [
            ProfileField::Name,
            ProfileField::Model,
            ProfileField::BaseUrl
        ]
    );
    assert!(store.add(bare).is_err());
    let plain = AgentProfile {
        base_url: Some("sunucu:8000".into()),
        ..AgentProfile::new(ProviderKind::OpenAiCompatible, "Sunucu", "m")
    };
    assert_eq!(plain.problems()[0].0, ProfileField::BaseUrl);
    let ok = AgentProfile {
        base_url: Some(" http://localhost:8000/v1/ ".into()),
        ..AgentProfile::new(ProviderKind::OpenAiCompatible, "Sunucu", "m")
    };
    let kept = store.add(ok).unwrap();
    assert_eq!(kept.base_url.as_deref(), Some("http://localhost:8000/v1"));
    assert_eq!(kept.endpoint().as_deref(), Some("http://localhost:8000/v1"));
    // An environment agent takes no typed key.
    let env = store
        .add(AgentProfile {
            key: KeySource::Environment,
            ..AgentProfile::new(ProviderKind::Gemini, "Gemini", "gemini-4-carbon")
        })
        .unwrap();
    assert!(store.set_key(&env.id, "x").is_err());
    assert!(store.set_key(&kept.id, "  ").is_err());
}
