use std::sync::Arc;

use bevy::{
    app::{App, Plugin, Update},
    asset::AssetServer,
    camera::{NormalizedRenderTarget, visibility::Visibility},
    ecs::{
        component::Component,
        entity::{ContainsEntity, Entity},
        error::Result,
        hierarchy::ChildOf,
        message::MessageReader,
        observer::On,
        query::{Added, With},
        schedule::IntoScheduleConfigs,
        system::{Commands, EntityCommands, Query, Res, ResMut, SystemState},
        world::{CommandQueue, DeferredWorld, FromWorld, World},
    },
    input::keyboard::KeyboardFocusLost,
    input_focus::{InputFocus, IsFocused, IsFocusedHelper, tab_navigation::TabIndex},
    log::info,
    math::Vec2,
    picking::{
        Pickable,
        events::{Click, Out, Over, Pointer, Press},
        hover::Hovered,
        pointer::{Location, PointerButton, PointerLocation},
    },
    text::TextFont,
    ui::{
        AlignItems, FlexDirection, GlobalZIndex, JustifyContent, Node, OverrideClip, PositionType,
        UiRect, UiScale, ZIndex, percent, px, widget::{ImageNode, Text},
    },
    ui_widgets::{
        MenuItem, MenuLayout, MenuPopup,
        popover::{Popover, PopoverAlign, PopoverPlacement, PopoverSide},
    },
    utils::default,
};

use crate::{
    theme::{
        RoundedCorners, ThemeBackgroundColor, ThemeBorderColor, ThemeFontColor,
        constants::fonts::REGULAR,
        tokens::{BORDER, BUTTON_BG_HOVER, PANE_TAB_ACTIVE, TEXT_MAIN, WINDOW_BG},
    },
    window::EditorWindow,
};

#[derive(Component, Clone)]
pub enum ContextMenuItem {
    Option {
        label: String,
        callback: Arc<dyn Fn(&mut DeferredWorld, Entity) + Send + Sync>,
    },
    Submenu {
        label: String,
        menu: ContextMenu,
    },
}

impl ContextMenuItem {
    pub fn label(&self) -> &String {
        match self {
            ContextMenuItem::Option { label, .. } => label,
            ContextMenuItem::Submenu { label, .. } => label,
        }
    }
}

#[derive(Component, Clone, Default)]
pub struct ContextMenu {
    pub items: Vec<ContextMenuItem>,
}

impl ContextMenu {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_option(
        mut self,
        label: impl Into<String>,
        callback: impl Fn(&mut DeferredWorld, Entity) + Send + Sync + 'static,
    ) -> Self {
        self.items.push(ContextMenuItem::Option {
            label: label.into(),
            callback: Arc::new(callback),
        });

        self
    }

    pub fn with_submenu(mut self, label: impl Into<String>, menu: impl Into<ContextMenu>) -> Self {
        self.items.push(ContextMenuItem::Submenu {
            label: label.into(),
            menu: menu.into(),
        });

        self
    }
}

pub struct ContextMenuPlugin;

impl Plugin for ContextMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, spawn_menus);
    }
}

fn spawn_menus(
    world: &mut World,
    state: &mut SystemState<(
        MessageReader<Pointer<Click>>,
        Query<(Entity, &ContextMenu)>,
        Query<&EditorWindow>,
        Res<UiScale>,
        Res<AssetServer>,
        Commands,
    )>,
) -> Result {
    let (mut clicks, menus, editor_windows, ui_scale, asset_server, mut commands) =
        state.get_mut(world);

    for click in clicks.read() {
        let NormalizedRenderTarget::Window(window_ref) = click.pointer_location.target else {
            continue;
        };

        let Ok(editor_window) = editor_windows.get(window_ref.entity()) else {
            continue;
        };

        let Some((target, menu)) = menus.iter().find(|(entity, _)| {
            if *entity != click.entity {
                return false;
            }

            if click.button != PointerButton::Secondary {
                return false;
            }

            return true;
        }) else {
            continue;
        };

        spawn_menu(
            &mut commands,
            &asset_server,
            editor_window.root(),
            click.pointer_location.position / ui_scale.0,
            target,
            menu,
        )?;

        break;
    }

    state.apply(world);

    Ok(())
}

fn spawn_menu<'a>(
    commands: &'a mut Commands,
    asset_server: &AssetServer,
    window: Entity,
    position: Vec2,
    target: Entity,
    menu: &ContextMenu,
) -> Result<EntityCommands<'a>> {
    let root = commands
        .spawn((
            ChildOf(window),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                ..default()
            },
            Pickable {
                should_block_lower: false,
                ..default()
            },
        ))
        .observe(|trigger: On<Pointer<Press>>, mut commands: Commands| {
            commands.entity(trigger.entity).despawn()
        })
        .id();

    let context_menu = commands
        .spawn((
            ChildOf(root),
            Node {
                position_type: PositionType::Absolute,
                flex_direction: FlexDirection::Column,
                border: UiRect::all(px(1)),
                padding: UiRect::all(px(6)),
                left: px(position.x),
                top: px(position.y),
                border_radius: RoundedCorners::All.to_border_radius(6.0),
                ..default()
            },
            ThemeBorderColor(BORDER),
            ThemeBackgroundColor(WINDOW_BG),
        ))
        .id();

    for item in menu.items.clone() {
        spawn_menu_item(commands, asset_server, root, context_menu, target, item)?;
    }

    Ok(commands.entity(context_menu))
}

fn spawn_menu_item(
    commands: &mut Commands,
    asset_server: &AssetServer,
    root: Entity,
    context_menu: Entity,
    target: Entity,
    item: ContextMenuItem,
) -> Result {
    let menu_item = commands
        .spawn((
            ChildOf(context_menu),
            Node {
                border_radius: RoundedCorners::All.to_border_radius(4.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                padding: UiRect::horizontal(px(8)).with_top(px(4)).with_bottom(px(4)),
                ..default()
            },
            ThemeBackgroundColor(WINDOW_BG),
        ))
        .observe(|trigger: On<Pointer<Over>>, mut commands: Commands| {
            commands
                .entity(trigger.entity)
                .insert(ThemeBackgroundColor(PANE_TAB_ACTIVE));
        })
        .observe(|trigger: On<Pointer<Out>>, mut commands: Commands| {
            commands
                .entity(trigger.entity)
                .insert(ThemeBackgroundColor(WINDOW_BG));
        })
        .observe(|mut trigger: On<Pointer<Press>>| {
            trigger.propagate(false);
        })
        .id();

    let click_item = item.clone();
    commands.entity(menu_item).observe(
        move |trigger: On<Pointer<Click>>, mut world: DeferredWorld, mut commands: Commands| {
            match &click_item {
                ContextMenuItem::Option { label, callback } => {
                    callback(&mut world, target);
                    commands.entity(root).despawn();
                }
                ContextMenuItem::Submenu { label, menu } => {}
            }
        },
    );

    let display_item = item.clone();
    commands.entity(menu_item).with_children(move |commands| {
        // Label
        commands.spawn((
            Pickable::IGNORE,
            Text::new(display_item.label()),
            TextFont {
                font: asset_server.load(REGULAR),
                font_size: 12.0,
                ..default()
            },
            ThemeFontColor(TEXT_MAIN),
        ));
    });

    Ok(())
}
