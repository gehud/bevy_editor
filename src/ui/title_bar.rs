//! Custom window title bar: logo, menus, document tabs, play controls and caption buttons.
//!
//! The OS title bar is disabled (`Window::decorations = false`), so this bar also moves the
//! window when dragged and provides minimize / maximize / close.

use bevy::{
    app::AppExit,
    feathers::constants::icons,
    picking::events::{PointerClick, PointerPress},
    prelude::*,
    text::{FontSourceTemplate, FontWeight},
    ui_widgets::Button,
};

use super::style::{HoverBackground, Weight, colors, icon, icon_sq, ink, label, text};

/// Marks title-bar areas that start a window drag when pressed.
#[derive(Component, Clone, Default)]
pub struct DragsWindow;

#[derive(Component, Clone, Copy, Default, PartialEq)]
enum CaptionButton {
    #[default]
    Minimize,
    Maximize,
    Close,
}

pub fn title_bar() -> impl Scene {
    bsn! {
        Node {
            height: px(34),
            width: percent(100),
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
        }
        DragsWindow
        on(start_window_drag)
        Children [
            // Left: logo, menus, document tabs.
            Node { column_gap: px(15), align_items: AlignItems::Center }
            DragsWindow
            Children [
                @logo()
                --
                @menus()
                --
                @document_tabs()
            ]
            --
            // Right: view selector, play controls, caption buttons.
            Node { column_gap: px(10), align_items: AlignItems::Center }
            DragsWindow
            Children [
                Node { column_gap: px(5), align_items: AlignItems::Center }
                Children [
                    @view_select()
                    --
                    @play_controls()
                ]
                --
                @caption_buttons()
            ]
        ]
    }
}

fn logo() -> impl Scene {
    bsn! {
        Node { column_gap: px(6), align_items: AlignItems::Center, padding: UiRect::left(px(12)) }
        DragsWindow
        Children [
            // The logo artwork sits inside a 19.43 x 20 frame with a small inset.
            Node { width: px(19.427), height: px(20), flex_shrink: 0.0 }
            Children [
                Node {
                    position_type: PositionType::Absolute,
                    left: percent(3.15),
                    top: percent(15.38),
                    width: percent(93.71),
                    height: percent(69.23),
                }
                ImageNode { image: "bevy-logo.png" }
            ]
            --
            @text("Bevy", 13.5, Weight::Bold, ink::ECEC)
        ]
    }
}

fn menus() -> impl Scene {
    let items = ["File", "Edit", "Debug", "Project", "Help"]
        .into_iter()
        .map(|title| bsn! { @menu_title(title) })
        .collect::<Vec<_>>();
    bsn! {
        Node { column_gap: px(16), align_items: AlignItems::Center }
        Children [ {items} ]
    }
}

fn menu_title(title: &'static str) -> impl Scene {
    bsn! {
        @text(title, 13.0, Weight::Regular, ink::ECEC)
        TextShadow { offset: Vec2::new(0.0, 1.0), color: ink::SHADOW }
    }
}

fn document_tabs() -> impl Scene {
    bsn! {
        Node { height: px(26), column_gap: px(4), align_items: AlignItems::Center }
        Children [
            // Inactive tab.
            Node {
                width: px(175),
                height: percent(100),
                padding: UiRect::axes(px(7), px(4)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                border_radius: BorderRadius::all(px(4)),
            }
            HoverBackground { normal: Color::NONE, hover: colors::ITEM_BG }
            BackgroundColor(Color::NONE)
            Children [
                Node { column_gap: px(5), align_items: AlignItems::Center }
                Children [
                    @tab_marker(colors::TAB_MARKER_YELLOW)
                    --
                    @icon_sq(icons::CALENDAR_SEARCH, 12.0, ink::DIM)
                    --
                    @label("Schedule Explorer", ink::DIM)
                ]
                --
                Node { width: px(10), height: px(10), align_items: AlignItems::Center, justify_content: JustifyContent::Center }
                Children [ @icon_sq(icons::X_SMALL, 5.833, ink::E6) ]
            ]
            --
            // Active tab; the dot marks unsaved changes.
            Node {
                width: px(165),
                height: percent(100),
                padding: UiRect::axes(px(7), px(4)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
            }
            BackgroundColor(colors::ITEM_BG_ACTIVE)
            BorderColor { top: colors::TAB_BORDER, bottom: colors::TAB_BORDER, left: colors::TAB_BORDER, right: colors::TAB_BORDER }
            Children [
                Node { column_gap: px(5), align_items: AlignItems::Center }
                Children [
                    @tab_marker(colors::ACCENT)
                    --
                    @icon_sq(icons::FILE_SMALL, 12.0, ink::E6)
                    --
                    @text("Main scene", 12.0, Weight::Medium, ink::DBDB)
                ]
                --
                Node { width: px(10), height: px(10), align_items: AlignItems::Center, justify_content: JustifyContent::Center }
                Children [ @icon_sq(icons::CIRCLE_FILLED, 6.0, ink::D9D9) ]
            ]
            --
            @icon_sq(icons::PLUS, 18.0, ink::E6)
        ]
    }
}

fn tab_marker(color: Color) -> impl Scene {
    bsn! {
        Node { width: px(2.5), height: px(12), border_radius: BorderRadius::all(px(5)) }
        BackgroundColor(color)
    }
}

fn view_select() -> impl Scene {
    bsn! {
        Button
        Node {
            width: px(130),
            height: px(22),
            padding: UiRect { left: px(9), right: px(7), top: px(4), bottom: px(4) },
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(5)),
        }
        BackgroundColor(colors::ITEM_BG)
        BorderColor { top: colors::ITEM_BORDER, bottom: colors::ITEM_BORDER, left: colors::ITEM_BORDER, right: colors::ITEM_BORDER }
        Children [
            @label("Scene View", ink::DADA)
            --
            @icon_sq(icons::CHEVRON_DOWN, 12.0, ink::E6)
        ]
    }
}

fn play_controls() -> impl Scene {
    bsn! {
        Node {
            height: px(22),
            padding: UiRect::horizontal(px(6.667)),
            column_gap: px(8.889),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(5)),
        }
        BackgroundColor(colors::ITEM_BG)
        BorderColor { top: colors::ITEM_BORDER, bottom: colors::ITEM_BORDER, left: colors::ITEM_BORDER, right: colors::ITEM_BORDER }
        Children [
            Button
            Node
            Children [ @icon_sq(icons::PLAY, 13.333, ink::E6) ]
            --
            Button
            Node { width: px(13.333), height: px(13.333), align_items: AlignItems::Center, justify_content: JustifyContent::Center }
            Children [ @icon(icons::PAUSE, 8.578, 11.174, ink::E6) ]
        ]
    }
}

fn caption_buttons() -> impl Scene {
    bsn! {
        Node { width: px(138), height: px(32), border_radius: BorderRadius::top_right(px(7)) }
        Children [
            @caption_button(CaptionButton::Minimize, "\u{E921}", colors::CAPTION_HOVER)
            --
            @caption_button(CaptionButton::Maximize, "\u{E922}", colors::CAPTION_HOVER)
            --
            @caption_button(CaptionButton::Close, "\u{E8BB}", colors::CAPTION_CLOSE_HOVER)
            Node { border_radius: BorderRadius::top_right(px(7)) }
        ]
    }
}

/// A Windows-style caption button. The glyphs come from the system icon font
/// (Segoe Fluent Icons on Windows 11, Segoe MDL2 Assets on Windows 10).
fn caption_button(kind: CaptionButton, glyph: &'static str, hover: Color) -> impl Scene {
    bsn! {
        Button
        Node {
            width: px(46),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
        BackgroundColor(Color::NONE)
        HoverBackground { normal: Color::NONE, hover: hover }
        kind
        on(caption_button_click)
        Children [
            Text(glyph)
            TextFont {
                font: FontSourceTemplate::Families("Segoe Fluent Icons, Segoe MDL2 Assets"),
                font_size: FontSize::Px(10.0),
                weight: FontWeight::NORMAL,
            }
            TextColor(ink::E6)
        ]
    }
}

fn caption_button_click(
    click: On<PointerClick>,
    buttons: Query<&CaptionButton>,
    mut window: Single<&mut Window>,
    mut maximized: ResMut<WindowMaximized>,
    mut exit: MessageWriter<AppExit>,
) {
    let Ok(button) = buttons.get(click.entity) else {
        return;
    };
    match button {
        CaptionButton::Minimize => window.set_minimized(true),
        CaptionButton::Maximize => maximized.toggle(&mut window),
        CaptionButton::Close => {
            exit.write(AppExit::Success);
        }
    }
}

/// Whether the editor window is maximized. Tracked here because `Window` has no getter for it.
#[derive(Resource, Default)]
pub struct WindowMaximized(bool);

impl WindowMaximized {
    fn toggle(&mut self, window: &mut Window) {
        self.0 = !self.0;
        window.set_maximized(self.0);
    }
}

/// On an empty part of the title bar: drag moves the window, double-click maximizes it.
fn start_window_drag(
    press: On<PointerPress>,
    drag_areas: Query<(), With<DragsWindow>>,
    mut window: Single<&mut Window>,
    mut maximized: ResMut<WindowMaximized>,
) {
    if press.button != PointerButton::Primary || !drag_areas.contains(press.original_event_target())
    {
        return;
    }
    if press.count >= 2 {
        maximized.toggle(&mut window);
    } else {
        window.start_drag_move();
    }
}
