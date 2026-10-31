use serde::Serialize;

/// Where an event belongs after observation. `None` stays in the log only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Config,
    Env,
    Decision,
    Failure,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Config => "config",
            Self::Env => "env",
            Self::Decision => "decision",
            Self::Failure => "failure",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "config" => Some(Self::Config),
            "env" => Some(Self::Env),
            "decision" => Some(Self::Decision),
            "failure" => Some(Self::Failure),
            _ => None,
        }
    }
}

/// Signals the caller supplies. The engine does not infer these with a model.
#[derive(Debug, Clone)]
pub struct Signals<'a> {
    pub failure: bool,
    pub decision: bool,
    pub event_kind: Option<&'a str>,
    pub config_key: Option<&'a str>,
    pub config_value: Option<&'a str>,
    pub stage: Option<&'a str>,
}

/// Assign a belonging. Explicit flags win, then an `event_kind` that names a
/// class, then the presence of an exact config pair, then a stage.
pub fn classify(signals: Signals<'_>) -> Option<Kind> {
    if signals.failure {
        return Some(Kind::Failure);
    }
    if signals.decision {
        return Some(Kind::Decision);
    }
    if let Some(kind) = signals.event_kind.and_then(Kind::parse) {
        return Some(kind);
    }
    if has_text(signals.config_key) && has_text(signals.config_value) {
        return Some(Kind::Config);
    }
    if has_text(signals.stage) {
        return Some(Kind::Env);
    }
    None
}

fn has_text(value: Option<&str>) -> bool {
    value.is_some_and(|text| !text.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{classify, Kind, Signals};

    fn signals() -> Signals<'static> {
        Signals {
            failure: false,
            decision: false,
            event_kind: None,
            config_key: None,
            config_value: None,
            stage: None,
        }
    }

    #[test]
    fn failure_wins_over_a_config_pair() {
        let mut input = signals();
        input.failure = true;
        input.config_key = Some("port");
        input.config_value = Some("4080");
        assert_eq!(classify(input), Some(Kind::Failure));
    }

    #[test]
    fn named_event_kind_assigns_a_class() {
        let mut input = signals();
        input.event_kind = Some("decision");
        assert_eq!(classify(input), Some(Kind::Decision));
    }

    #[test]
    fn harness_event_names_do_not_assign_a_class() {
        let mut input = signals();
        input.event_kind = Some("tool_result");
        assert_eq!(classify(input), None);
    }

    #[test]
    fn stage_is_env_when_nothing_more_specific_is_present() {
        let mut input = signals();
        input.stage = Some("exploring");
        assert_eq!(classify(input), Some(Kind::Env));
    }
}
