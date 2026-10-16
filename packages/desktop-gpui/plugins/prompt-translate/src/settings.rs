//! The settings record `translate` (`shared/settings.ts`): nine fields with defaults, and the
//! send-key rule.

use bachuc_plugin_host::SendKey;
use serde_json::{Value, json};

pub(crate) const DEFAULT_TRANSLATE_MODEL: &str = "google/gemini-2.5-flash-lite";
pub(crate) const DEFAULT_ENHANCE_MODEL: &str = "google/gemini-2.5-flash-lite";

/// The Caveman modes in the order of `client/modes.ts`. `value` is the stored string.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Mode {
    #[default]
    FollowAgent,
    Lite,
    Full,
    Ultra,
    WenyanLite,
    WenyanFull,
    WenyanUltra,
}

impl Mode {
    pub(crate) const ALL: [Mode; 7] = [
        Mode::FollowAgent,
        Mode::Lite,
        Mode::Full,
        Mode::Ultra,
        Mode::WenyanLite,
        Mode::WenyanFull,
        Mode::WenyanUltra,
    ];

    pub(crate) fn value(self) -> &'static str {
        match self {
            Mode::FollowAgent => "follow-agent",
            Mode::Lite => "lite",
            Mode::Full => "full",
            Mode::Ultra => "ultra",
            Mode::WenyanLite => "wenyan-lite",
            Mode::WenyanFull => "wenyan-full",
            Mode::WenyanUltra => "wenyan-ultra",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Mode::FollowAgent => "Default",
            Mode::Lite => "Lite",
            Mode::Full => "Full",
            Mode::Ultra => "Ultra",
            Mode::WenyanLite => "Wenyan Lite",
            Mode::WenyanFull => "Wenyan Full",
            Mode::WenyanUltra => "Wenyan Ultra",
        }
    }

    /// `None` for a value outside the seven.
    pub(crate) fn parse(value: &str) -> Option<Mode> {
        Mode::ALL.into_iter().find(|mode| mode.value() == value)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum RewriteMode {
    #[default]
    Automatic,
    CmdEnter,
}

impl RewriteMode {
    pub(crate) fn value(self) -> &'static str {
        match self {
            RewriteMode::Automatic => "automatic",
            RewriteMode::CmdEnter => "cmd-enter",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<RewriteMode> {
        [RewriteMode::Automatic, RewriteMode::CmdEnter]
            .into_iter()
            .find(|mode| mode.value() == value)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ChineseScript {
    #[default]
    SkillDefault,
    Simplified,
}

impl ChineseScript {
    pub(crate) fn value(self) -> &'static str {
        match self {
            ChineseScript::SkillDefault => "skill-default",
            ChineseScript::Simplified => "simplified",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<ChineseScript> {
        [ChineseScript::SkillDefault, ChineseScript::Simplified]
            .into_iter()
            .find(|script| script.value() == value)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Provider {
    #[default]
    OpenRouter,
    Vercel,
}

impl Provider {
    pub(crate) fn value(self) -> &'static str {
        match self {
            Provider::OpenRouter => "openrouter",
            Provider::Vercel => "vercel",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Provider> {
        [Provider::OpenRouter, Provider::Vercel]
            .into_iter()
            .find(|provider| provider.value() == value)
    }
}

/// `TranslateSettings`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Settings {
    pub(crate) translate: bool,
    pub(crate) enhance_shortcut: bool,
    pub(crate) rewrite_mode: RewriteMode,
    pub(crate) match_reply_language: bool,
    pub(crate) caveman_mode: Mode,
    pub(crate) chinese_script: ChineseScript,
    pub(crate) provider: Provider,
    pub(crate) translate_model: String,
    pub(crate) enhance_model: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            translate: true,
            enhance_shortcut: true,
            rewrite_mode: RewriteMode::default(),
            match_reply_language: true,
            caveman_mode: Mode::default(),
            chinese_script: ChineseScript::default(),
            provider: Provider::default(),
            translate_model: DEFAULT_TRANSLATE_MODEL.into(),
            enhance_model: DEFAULT_ENHANCE_MODEL.into(),
        }
    }
}

impl Settings {
    /// Reads a stored record. A missing field, or one the schema would reject, takes its
    /// default (`translateSettings.schema.parse`, with `.default()` on every field). A model is
    /// trimmed and must not be empty.
    pub(crate) fn decode(values: &Value) -> Settings {
        let defaults = Settings::default();
        let flag =
            |key: &str, default: bool| values.get(key).and_then(Value::as_bool).unwrap_or(default);
        let text = |key: &str| values.get(key).and_then(Value::as_str);
        let model = |key: &str, default: &str| {
            text(key)
                .map(str::trim)
                .filter(|model| !model.is_empty())
                .unwrap_or(default)
                .to_string()
        };
        Settings {
            translate: flag("translate", defaults.translate),
            enhance_shortcut: flag("enhanceShortcut", defaults.enhance_shortcut),
            rewrite_mode: text("rewriteMode")
                .and_then(RewriteMode::parse)
                .unwrap_or(defaults.rewrite_mode),
            match_reply_language: flag("matchReplyLanguage", defaults.match_reply_language),
            caveman_mode: text("cavemanMode")
                .and_then(Mode::parse)
                .unwrap_or(defaults.caveman_mode),
            chinese_script: text("chineseScript")
                .and_then(ChineseScript::parse)
                .unwrap_or(defaults.chinese_script),
            provider: text("provider")
                .and_then(Provider::parse)
                .unwrap_or(defaults.provider),
            translate_model: model("translateModel", &defaults.translate_model),
            enhance_model: model("enhanceModel", &defaults.enhance_model),
        }
    }

    /// The whole record, as `settings.save` writes it.
    pub(crate) fn to_value(&self) -> Value {
        json!({
            "translate": self.translate,
            "enhanceShortcut": self.enhance_shortcut,
            "rewriteMode": self.rewrite_mode.value(),
            "matchReplyLanguage": self.match_reply_language,
            "cavemanMode": self.caveman_mode.value(),
            "chineseScript": self.chinese_script.value(),
            "provider": self.provider.value(),
            "translateModel": self.translate_model,
            "enhanceModel": self.enhance_model,
        })
    }
}

/// Automatic rewrites on Enter and the send button; Cmd/Ctrl+Enter sends as typed. Cmd+Enter
/// mode rewrites only on Cmd/Ctrl+Enter. The button, dictation, and older hosts report no key
/// (`rewritesOnSendKey`).
pub(crate) fn rewrites_on_send_key(send_key: Option<SendKey>, mode: RewriteMode) -> bool {
    match mode {
        RewriteMode::Automatic => send_key != Some(SendKey::ModEnter),
        RewriteMode::CmdEnter => send_key == Some(SendKey::ModEnter),
    }
}

#[cfg(test)]
mod tests {
    use super::{Mode, RewriteMode, Settings, rewrites_on_send_key};
    use bachuc_plugin_host::SendKey;
    use serde_json::json;

    #[test]
    fn automatic_rewrites_on_enter_and_the_button_cmd_enter_mode_rewrites_only_on_cmd_enter() {
        let automatic = RewriteMode::Automatic;
        assert!(rewrites_on_send_key(Some(SendKey::Enter), automatic));
        assert!(rewrites_on_send_key(None, automatic));
        assert!(!rewrites_on_send_key(Some(SendKey::ModEnter), automatic));
        let cmd = RewriteMode::CmdEnter;
        assert!(rewrites_on_send_key(Some(SendKey::ModEnter), cmd));
        assert!(!rewrites_on_send_key(Some(SendKey::Enter), cmd));
        assert!(!rewrites_on_send_key(None, cmd));
    }

    #[test]
    fn settings_defaults_fill_missing_fields() {
        let defaults = Settings::decode(&json!({}));
        assert_eq!(defaults, Settings::default());
        assert_eq!(
            defaults.to_value(),
            json!({
                "translate": true,
                "enhanceShortcut": true,
                "rewriteMode": "automatic",
                "matchReplyLanguage": true,
                "cavemanMode": "follow-agent",
                "chineseScript": "skill-default",
                "provider": "openrouter",
                "translateModel": "google/gemini-2.5-flash-lite",
                "enhanceModel": "google/gemini-2.5-flash-lite",
            })
        );
        let partial = Settings::decode(&json!({ "translate": false, "enhanceShortcut": false }));
        assert!(!partial.translate && !partial.enhance_shortcut);
        assert!(partial.match_reply_language);
        assert_eq!(partial.caveman_mode, Mode::FollowAgent);
        // A stored value round-trips, a blank model takes its default, a model is trimmed.
        let stored = Settings::decode(&json!({
            "rewriteMode": "cmd-enter",
            "cavemanMode": "wenyan-ultra",
            "provider": "vercel",
            "translateModel": "  a/b  ",
            "enhanceModel": "   ",
        }));
        assert_eq!(stored.rewrite_mode, RewriteMode::CmdEnter);
        assert_eq!(stored.caveman_mode, Mode::WenyanUltra);
        assert_eq!(stored.translate_model, "a/b");
        assert_eq!(stored.enhance_model, "google/gemini-2.5-flash-lite");
        assert_eq!(Settings::decode(&stored.to_value()), stored);
        // A value outside the schema takes the default.
        assert_eq!(
            Settings::decode(&json!({ "cavemanMode": "nope" })).caveman_mode,
            Mode::FollowAgent
        );
    }

    #[test]
    fn the_seven_modes_keep_the_order_of_modes_ts() {
        let labels: Vec<_> = Mode::ALL.iter().map(|mode| mode.label()).collect();
        assert_eq!(
            labels,
            [
                "Default",
                "Lite",
                "Full",
                "Ultra",
                "Wenyan Lite",
                "Wenyan Full",
                "Wenyan Ultra"
            ]
        );
        assert!(
            Mode::ALL
                .iter()
                .all(|mode| Mode::parse(mode.value()) == Some(*mode))
        );
    }
}
