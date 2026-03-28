//! Design tokens used by editor themes.
//!
//! The term "design token" is commonly used in UX design to mean the smallest unit of a theme,
//! similar in concept to a CSS variable. Each token represents an assignment of a color or
//! value to a specific visual aspect of a widget, such as background or border.

use crate::theme::EditorThemeToken;

/// Window background
pub const WINDOW_BG: EditorThemeToken = EditorThemeToken::new("editor.window.bg");

// Border
pub const BORDER: EditorThemeToken = EditorThemeToken::new("editor.border");

pub const PANE_BG: EditorThemeToken = EditorThemeToken::new("pane.bg");
pub const PANE_TAB_ACTIVE: EditorThemeToken = EditorThemeToken::new("pane.tab.active");

/// Focus ring
pub const FOCUS_RING: EditorThemeToken = EditorThemeToken::new("editor.focus");

// Normal buttons

/// Regular button background
pub const BUTTON_BG: EditorThemeToken = EditorThemeToken::new("editor.button.bg");
/// Regular button background (hovered)
pub const BUTTON_BG_HOVER: EditorThemeToken = EditorThemeToken::new("editor.button.bg.hover");
/// Regular button background (disabled)
pub const BUTTON_BG_DISABLED: EditorThemeToken = EditorThemeToken::new("editor.button.bg.disabled");
/// Regular button background (pressed)
pub const BUTTON_BG_PRESSED: EditorThemeToken = EditorThemeToken::new("editor.button.bg.pressed");
/// Regular button text
pub const BUTTON_TEXT: EditorThemeToken = EditorThemeToken::new("editor.button.txt");
/// Regular button text (disabled)
pub const BUTTON_TEXT_DISABLED: EditorThemeToken = EditorThemeToken::new("editor.button.txt.disabled");

// Primary ("default") buttons

/// Primary button background
pub const BUTTON_PRIMARY_BG: EditorThemeToken = EditorThemeToken::new("editor.button.primary.bg");
/// Primary button background (hovered)
pub const BUTTON_PRIMARY_BG_HOVER: EditorThemeToken =
    EditorThemeToken::new("editor.button.primary.bg.hover");
/// Primary button background (disabled)
pub const BUTTON_PRIMARY_BG_DISABLED: EditorThemeToken =
    EditorThemeToken::new("editor.button.primary.bg.disabled");
/// Primary button background (pressed)
pub const BUTTON_PRIMARY_BG_PRESSED: EditorThemeToken =
    EditorThemeToken::new("editor.button.primary.bg.pressed");
/// Primary button text
pub const BUTTON_PRIMARY_TEXT: EditorThemeToken = EditorThemeToken::new("editor.button.primary.txt");
/// Primary button text (disabled)
pub const BUTTON_PRIMARY_TEXT_DISABLED: EditorThemeToken =
    EditorThemeToken::new("editor.button.primary.txt.disabled");

// Slider

/// Background for slider
pub const SLIDER_BG: EditorThemeToken = EditorThemeToken::new("editor.slider.bg");
/// Background for slider moving bar
pub const SLIDER_BAR: EditorThemeToken = EditorThemeToken::new("editor.slider.bar");
/// Background for slider moving bar (disabled)
pub const SLIDER_BAR_DISABLED: EditorThemeToken = EditorThemeToken::new("editor.slider.bar.disabled");
/// Background for slider text
pub const SLIDER_TEXT: EditorThemeToken = EditorThemeToken::new("editor.slider.text");
/// Background for slider text (disabled)
pub const SLIDER_TEXT_DISABLED: EditorThemeToken = EditorThemeToken::new("editor.slider.text.disabled");

// Checkbox

/// Checkbox background around the checkmark
pub const CHECKBOX_BG: EditorThemeToken = EditorThemeToken::new("editor.checkbox.bg");
/// Checkbox border around the checkmark (disabled)
pub const CHECKBOX_BG_DISABLED: EditorThemeToken = EditorThemeToken::new("editor.checkbox.bg.disabled");
/// Checkbox background around the checkmark
pub const CHECKBOX_BG_CHECKED: EditorThemeToken = EditorThemeToken::new("editor.checkbox.bg.checked");
/// Checkbox border around the checkmark (disabled)
pub const CHECKBOX_BG_CHECKED_DISABLED: EditorThemeToken =
    EditorThemeToken::new("editor.checkbox.bg.checked.disabled");
/// Checkbox border around the checkmark
pub const CHECKBOX_BORDER: EditorThemeToken = EditorThemeToken::new("editor.checkbox.border");
/// Checkbox border around the checkmark (hovered)
pub const CHECKBOX_BORDER_HOVER: EditorThemeToken =
    EditorThemeToken::new("editor.checkbox.border.hover");
/// Checkbox border around the checkmark (disabled)
pub const CHECKBOX_BORDER_DISABLED: EditorThemeToken =
    EditorThemeToken::new("editor.checkbox.border.disabled");
/// Checkbox check mark
pub const CHECKBOX_MARK: EditorThemeToken = EditorThemeToken::new("editor.checkbox.mark");
/// Checkbox check mark (disabled)
pub const CHECKBOX_MARK_DISABLED: EditorThemeToken =
    EditorThemeToken::new("editor.checkbox.mark.disabled");
/// Checkbox label text
pub const CHECKBOX_TEXT: EditorThemeToken = EditorThemeToken::new("editor.checkbox.text");
/// Checkbox label text (disabled)
pub const CHECKBOX_TEXT_DISABLED: EditorThemeToken =
    EditorThemeToken::new("editor.checkbox.text.disabled");

// Radio button

/// Radio border around the checkmark
pub const RADIO_BORDER: EditorThemeToken = EditorThemeToken::new("editor.radio.border");
/// Radio border around the checkmark (hovered)
pub const RADIO_BORDER_HOVER: EditorThemeToken = EditorThemeToken::new("editor.radio.border.hover");
/// Radio border around the checkmark (disabled)
pub const RADIO_BORDER_DISABLED: EditorThemeToken =
    EditorThemeToken::new("editor.radio.border.disabled");
/// Radio check mark
pub const RADIO_MARK: EditorThemeToken = EditorThemeToken::new("editor.radio.mark");
/// Radio check mark (disabled)
pub const RADIO_MARK_DISABLED: EditorThemeToken = EditorThemeToken::new("editor.radio.mark.disabled");
/// Radio label text
pub const RADIO_TEXT: EditorThemeToken = EditorThemeToken::new("editor.radio.text");
/// Radio label text (disabled)
pub const RADIO_TEXT_DISABLED: EditorThemeToken = EditorThemeToken::new("editor.radio.text.disabled");

// Toggle Switch

/// Switch background around the checkmark
pub const SWITCH_BG: EditorThemeToken = EditorThemeToken::new("editor.switch.bg");
/// Switch border around the checkmark (disabled)
pub const SWITCH_BG_DISABLED: EditorThemeToken = EditorThemeToken::new("editor.switch.bg.disabled");
/// Switch background around the checkmark
pub const SWITCH_BG_CHECKED: EditorThemeToken = EditorThemeToken::new("editor.switch.bg.checked");
/// Switch border around the checkmark (disabled)
pub const SWITCH_BG_CHECKED_DISABLED: EditorThemeToken =
    EditorThemeToken::new("editor.switch.bg.checked.disabled");
/// Switch border around the checkmark
pub const SWITCH_BORDER: EditorThemeToken = EditorThemeToken::new("editor.switch.border");
/// Switch border around the checkmark (hovered)
pub const SWITCH_BORDER_HOVER: EditorThemeToken = EditorThemeToken::new("editor.switch.border.hover");
/// Switch border around the checkmark (disabled)
pub const SWITCH_BORDER_DISABLED: EditorThemeToken =
    EditorThemeToken::new("editor.switch.border.disabled");
/// Switch slide
pub const SWITCH_SLIDE: EditorThemeToken = EditorThemeToken::new("editor.switch.slide");
/// Switch slide (disabled)
pub const SWITCH_SLIDE_DISABLED: EditorThemeToken =
    EditorThemeToken::new("editor.switch.slide.disabled");

// Color Plane

/// Color plane frame background
pub const COLOR_PLANE_BG: EditorThemeToken = EditorThemeToken::new("editor.colorplane.bg");
