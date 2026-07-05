/// A single selectable entry: the stable `value` used by callers and the
/// human-facing `label` shown in the menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

impl SelectOption {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
        }
    }
}
