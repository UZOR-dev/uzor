//! Semantic accessibility role.

/// Semantic role of an accessibility node — modeled 1:1 on a subset of
/// AccessKit's own `accesskit::Role` enum (accesskit 0.19-shaped, from
/// memory of its public docs — NOT fetched fresh in this pass; re-verify
/// exact upstream member names against whichever `accesskit` version is
/// pinned before writing the H7 adapter that maps this onto
/// `accesskit::Node`). No `accesskit` dependency here — this type is pure
/// uzor and must stay that way per the master plan's H0/H7 split.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum A11yRole {
    Button,
    CheckBox,
    RadioButton,
    Switch,
    Slider,
    TextInput,
    Tab,
    TabList,
    MenuItem,
    ListItem,
    Group,
    Dialog,
    Tooltip,
    Link,
    Unknown,
}

impl A11yRole {
    /// The role a registered widget kind implies; `None` for structure the tree
    /// leaves out (surfaces, scrims, text, separators, scrollbars, panels).
    pub fn for_kind(kind: crate::input::WidgetKind) -> Option<Self> {
        use crate::input::WidgetKind as K;
        Some(match kind {
            K::Button | K::CloseButton | K::DropdownTrigger | K::ColorSwatch | K::ScrollChevron => Self::Button,
            K::Checkbox => Self::CheckBox,
            K::Radio => Self::RadioButton,
            K::Toggle => Self::Switch,
            K::Slider => Self::Slider,
            K::ChromeTab => Self::Tab,
            K::Item => Self::MenuItem,
            K::Link => Self::Link,
            K::Modal => Self::Dialog,
            K::Tooltip => Self::Tooltip,
            _ => return None,
        })
    }
}
