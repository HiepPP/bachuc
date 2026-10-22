//! The settings record `translate` (`shared/settings.ts`): the provider and the model.

use serde_json::{Value, json};

pub(crate) const DEFAULT_MODEL: &str = "google/gemini-2.5-flash-lite";

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Settings {
    pub(crate) provider: Provider,
    pub(crate) model: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            provider: Provider::default(),
            model: DEFAULT_MODEL.into(),
        }
    }
}

impl Settings {
    /// Reads a stored record. A missing field, or one the schema would reject, takes its
    /// default. A model is trimmed and must not be empty.
    pub(crate) fn decode(values: &Value) -> Settings {
        let defaults = Settings::default();
        let text = |key: &str| values.get(key).and_then(Value::as_str);
        Settings {
            provider: text("provider")
                .and_then(Provider::parse)
                .unwrap_or(defaults.provider),
            model: text("model")
                .map(str::trim)
                .filter(|model| !model.is_empty())
                .unwrap_or(&defaults.model)
                .to_string(),
        }
    }

    /// The whole record, as `settings.save` writes it.
    pub(crate) fn to_value(&self) -> Value {
        json!({ "provider": self.provider.value(), "model": self.model })
    }
}

#[cfg(test)]
mod tests {
    use super::{Provider, Settings};
    use serde_json::json;

    #[test]
    fn defaults_fill_missing_and_invalid_fields() {
        let defaults = Settings::decode(&json!({}));
        assert_eq!(defaults, Settings::default());
        assert_eq!(
            defaults.to_value(),
            json!({ "provider": "openrouter", "model": "google/gemini-2.5-flash-lite" })
        );
        assert_eq!(
            Settings::decode(&json!({ "provider": "nope", "model": "   " })),
            Settings::default()
        );
    }

    #[test]
    fn a_stored_record_round_trips_with_a_trimmed_model() {
        let stored = Settings::decode(&json!({ "provider": "vercel", "model": "  a/b  " }));
        assert_eq!(stored.provider, Provider::Vercel);
        assert_eq!(stored.model, "a/b");
        assert_eq!(Settings::decode(&stored.to_value()), stored);
    }
}
