//! The models a run can use, and the one chosen.

use serde_json::Value;

/// The model a run uses until one is chosen.
pub(crate) const DEFAULT_MODEL: &str = "sonnet";

/// The models on offer, in order: each family's alias, which the runtime
/// keeps pointed at the family's latest version.
const FAMILIES: [(&str, &str); 4] = [
    ("fable", "Fable"),
    ("sonnet", "Sonnet"),
    ("opus", "Opus"),
    ("haiku", "Haiku"),
];

/// A model a run can use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    /// What a run is started with: an alias (`sonnet`) or a full model id.
    pub value: String,
    /// Its name for people, with its version (`Sonnet 5`).
    pub name: String,
    /// One line on what it is for.
    pub description: String,
}

impl Model {
    fn alias(value: &str, name: &str) -> Self {
        Self {
            value: value.to_owned(),
            name: name.to_owned(),
            description: String::new(),
        }
    }
}

/// What the app hears about the models from outside.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelsNews {
    /// The runtime listed the models it can run.
    Listed(Vec<Model>),
    /// The choice kept from an earlier launch.
    Saved(String),
}

/// The models on offer and the one chosen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Models {
    list: Vec<Model>,
    chosen: String,
}

impl Default for Models {
    /// The aliases every runtime takes, which stand in until it lists its own.
    fn default() -> Self {
        Self {
            list: FAMILIES
                .iter()
                .map(|&(value, name)| Model::alias(value, name))
                .collect(),
            chosen: DEFAULT_MODEL.to_owned(),
        }
    }
}

impl Models {
    /// The models on offer, in the order they are shown.
    pub fn list(&self) -> &[Model] {
        &self.list
    }

    /// What a run is started with.
    pub fn chosen(&self) -> &str {
        &self.chosen
    }

    /// The chosen model's name: the listed one's, else the value as it is.
    /// A trailing note in brackets is left off.
    pub fn chosen_name(&self) -> &str {
        let name = (self.list.iter())
            .find(|model| model.value == self.chosen)
            .map_or(self.chosen.as_str(), |model| model.name.as_str());
        name.split_once(" (").map_or(name, |(name, _)| name)
    }

    /// Chooses `value`. A value that is not listed is kept: the list may
    /// not have arrived, and a full model id is a model too.
    pub fn choose(&mut self, value: &str) {
        if !value.trim().is_empty() {
            value.trim().clone_into(&mut self.chosen);
        }
    }

    /// Takes in `news`. An empty list keeps the one there is.
    pub fn hear(&mut self, news: ModelsNews) {
        match news {
            ModelsNews::Listed(list) if list.is_empty() => {}
            ModelsNews::Listed(list) => self.list = list,
            ModelsNews::Saved(value) => self.choose(&value),
        }
    }
}

/// The families `specular-helper` listed, when `line` is its `models` line.
/// Pinned versions and `default` are left off.
pub fn parse_models(line: &str) -> Option<Vec<Model>> {
    let message = serde_json::from_str::<Value>(line.trim()).ok()?;
    let is = |key: &str, want: &str| message.get(key).and_then(Value::as_str) == Some(want);
    if !is("type", "helper") || !is("event", "models") {
        return None;
    }
    let text = |model: &Value, key: &str| {
        (model.get(key).and_then(Value::as_str))
            .unwrap_or_default()
            .to_owned()
    };
    let listed: Vec<Model> = (message.get("models")?.as_array()?.iter())
        .map(|model| Model {
            value: text(model, "value"),
            name: text(model, "displayName"),
            description: text(model, "description"),
        })
        .filter(|model| !model.value.is_empty() && !model.name.is_empty())
        .collect();
    let families = (FAMILIES.iter())
        .filter_map(|&(alias, _)| listed.iter().find(|model| model.value == alias))
        .cloned()
        .collect();
    Some(families)
}
