//! Widget kind classification for InputCoordinator hierarchy.

/// Classifies widgets as composite parents or atomic leaves.
///
/// Composite parents contain children whose rects lie inside the parent's rect.
/// Atomic leaves are individual interactive elements with no sub-widgets.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum WidgetKind {
    // --- Composite (parent — has children): ---
    /// Full-screen or large overlay that blocks interaction with content behind it.
    Modal,
    /// Small transient floating panel.
    Popup,
    /// Inline dropdown list attached to a trigger widget.
    Dropdown,
    /// Right-click or long-press context menu.
    ContextMenu,
    /// Horizontal or vertical bar of grouped controls.
    Toolbar,
    /// Collapsible side panel.
    Sidebar,
    /// Generic container / card.
    Panel,
    /// Canvas-like panel whose internal input dispatch is managed externally.
    /// The coordinator does NOT recurse into its children.
    /// `is_over_ui()` returns `false` when the cursor is over a BlackboxPanel.
    BlackboxPanel,
    /// Window decoration container: titlebar with tabs + min/max/close buttons.
    Chrome,
    /// Single tab in a chrome tab strip (composite — can have a close-button child).
    ChromeTab,

    // --- Atomic (leaf — no children): ---
    /// Clickable button.
    Button,
    /// Boolean checkbox (checked / unchecked).
    Checkbox,
    /// Toolbar time display with hover-only behavior.
    Clock,
    /// X-glyph dismiss button for modals and panels.
    CloseButton,
    /// Color swatch square that opens a color picker (atomic leaf).
    ColorSwatch,
    /// Trigger button for a dropdown menu (atomic leaf — trigger only, not the menu).
    DropdownTrigger,
    /// Selectable item inside a menu or list (non-interactive).
    Item,
    /// Single radio option (part of a group).
    Radio,
    /// Draggable thumb of a scrollbar.
    ScrollbarHandle,
    /// Non-draggable track of a scrollbar (click-to-jump area).
    ScrollbarTrack,
    /// Directional chevron for toolbar overflow navigation.
    ScrollChevron,
    /// Non-interactive visual divider.
    Separator,
    /// Draggable slider thumb.
    Slider,
    /// iOS-style on/off toggle switch.
    Toggle,
    /// Text-with-background popup overlay. No internal interaction; hover tracked
    /// by the coordinator for fade-out logic.
    Tooltip,
    /// Draggable handle / grip — composite widgets register one of these for their
    /// header drag zones.
    DragHandle,
    /// Click-swallowing backdrop under a modal / menu (the dimmed scrim, "click outside
    /// closes"). Blocks everything below it like the DOM's fixed overlay; default arrow
    /// cursor (it is not a control).
    Backdrop,
    /// Inert surface that swallows pointer events: a dialog card, a menu panel, a message
    /// bubble. Default arrow cursor; clicks on it reach no widget below.
    Surface,
    /// Inline link or nick mention inside text (`<a>`): hand cursor, clickable.
    Link,
    /// Draggable splitter between panes: column-resize cursor when taller than wide,
    /// row-resize otherwise. Register it with `Sense::DRAG` so a press captures the pointer.
    Splitter,
    /// Static selectable text (label, message body): I-beam cursor.
    Text,
    /// Escape hatch — any widget that doesn't fit the above categories.
    Custom,
}

impl WidgetKind {
    /// The cursor the role implies (the DOM's UA defaults: `button`/`a` → pointer, text →
    /// I-beam, splitters → resize, everything else the arrow). `rect` picks a splitter's
    /// axis. A widget can override it with `InputCoordinator::set_widget_cursor`.
    pub fn cursor(self, rect: crate::types::Rect) -> crate::CursorIcon {
        use crate::CursorIcon as C;
        use WidgetKind::*;
        match self {
            Button | Checkbox | CloseButton | ColorSwatch | DropdownTrigger | Item | Radio | Toggle | ChromeTab | ScrollChevron | Link => C::PointingHand,
            Splitter => {
                if rect.height >= rect.width {
                    C::ResizeHorizontal
                } else {
                    C::ResizeVertical
                }
            }
            Text | Custom => C::Text,
            Slider | DragHandle => C::Default,
            _ => C::Default,
        }
    }

    /// Interactive role: takes clicks / keyboard focus as a control (a DOM `button`, `a`,
    /// `input`). Backdrops and surfaces swallow clicks but are not controls.
    pub const fn is_interactive(self) -> bool {
        use WidgetKind::*;
        matches!(self, Button | Checkbox | CloseButton | ColorSwatch | DropdownTrigger | Item | Radio | Toggle | ChromeTab | ScrollChevron | Link | Slider | Splitter)
    }

    /// `true` for kinds that act as parent containers.
    pub const fn is_composite(self) -> bool {
        use WidgetKind::*;
        matches!(
            self,
            Modal | Popup | Dropdown | ContextMenu | Toolbar | Sidebar
                | Panel | BlackboxPanel | Chrome | ChromeTab
        )
    }

    /// `true` for leaf kinds that cannot have registered children.
    pub const fn is_atomic(self) -> bool {
        !self.is_composite()
    }

    /// `true` only for `BlackboxPanel`.
    pub const fn is_blackbox(self) -> bool {
        matches!(self, WidgetKind::BlackboxPanel)
    }

    /// Whether widgets of this kind can have children registered under them.
    /// All composites except `BlackboxPanel` allow children.
    pub const fn allows_children(self) -> bool {
        self.is_composite() && !self.is_blackbox()
    }

    /// Whether widgets of this kind block events from reaching lower z layers
    /// when they are hit (modal / context-menu behaviour).
    pub const fn blocks_lower_layers(self) -> bool {
        use WidgetKind::*;
        matches!(self, Modal | ContextMenu | Popup | Backdrop)
    }
}
