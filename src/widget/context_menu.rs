use std::sync::Arc;

use bevy::{
    app::{App, Plugin, Update},
    asset::{self, AssetServer},
    camera::{NormalizedRenderTarget, visibility::Visibility},
    color::{Alpha, Color},
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
        AlignItems, BoxShadow, FlexDirection, GlobalZIndex, JustifyContent, Node, OverrideClip,
        PositionType, ShadowStyle, UiRect, UiScale, ZIndex, percent, px,
        widget::{ImageNode, Text},
    },
    ui_widgets::{
        MenuItem, MenuLayout, MenuPopup,
        popover::{Popover, PopoverAlign, PopoverPlacement, PopoverSide},
    },
    utils::default,
};
use variadics_please::{all_tuples, all_tuples_enumerated};

use crate::{
    theme::{
        RoundedCorners, ThemeBackgroundColor, ThemeBorderColor, ThemeFontColor, ThemeImageColor,
        constants::fonts::REGULAR,
        tokens::{BORDER, BUTTON_BG_HOVER, BUTTON_TEXT, PANE_TAB_ACTIVE, TEXT_MAIN, WINDOW_BG},
    },
    window::EditorWindow,
};

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum ContextMenuMark {
    #[default]
    None,
    Checked,
}

#[derive(Component, Clone)]
pub enum ContextMenuItem {
    Option {
        mark: ContextMenuMark,
        label: String,
        callback: Arc<dyn Fn(&mut DeferredWorld, Entity) + Send + Sync>,
    },
    Submenu {
        label: String,
        menu: ContextMenu,
    },
    Separator,
}

impl Into<ContextMenuItem> for () {
    fn into(self) -> ContextMenuItem {
        ContextMenuItem::Separator
    }
}

impl<L: Into<String>> Into<ContextMenuItem> for (L,) {
    fn into(self) -> ContextMenuItem {
        ContextMenuItem::Option {
            mark: default(),
            label: self.0.into(),
            callback: Arc::new(|_, _| {}),
        }
    }
}

impl<L: Into<String>> Into<ContextMenuItem> for (ContextMenuMark, L) {
    fn into(self) -> ContextMenuItem {
        ContextMenuItem::Option {
            mark: self.0,
            label: self.1.into(),
            callback: Arc::new(|_, _| {}),
        }
    }
}

impl<L: Into<String>, C: Fn(&mut DeferredWorld, Entity) + Send + Sync + 'static>
    Into<ContextMenuItem> for (L, C)
{
    fn into(self) -> ContextMenuItem {
        ContextMenuItem::Option {
            mark: default(),
            label: self.0.into(),
            callback: Arc::new(self.1),
        }
    }
}

impl<L: Into<String>, C: Fn(&mut DeferredWorld, Entity) + Send + Sync + 'static>
    Into<ContextMenuItem> for (ContextMenuMark, L, C)
{
    fn into(self) -> ContextMenuItem {
        ContextMenuItem::Option {
            mark: self.0,
            label: self.1.into(),
            callback: Arc::new(self.2),
        }
    }
}

macro_rules! impl_into_context_menu {
    ($(($n:tt, $I:ident)),*) => {
        impl<$($I: Into<ContextMenuItem>),*> Into<ContextMenu> for ($($I,)*) {
            fn into(self) -> ContextMenu {
                ContextMenu::new()$(.with(self.$n.into()))*
            }
        }
    };
}

all_tuples_enumerated!(impl_into_context_menu, 0, 16, I);

#[derive(Component, Clone, Default)]
pub struct ContextMenu {
    pub items: Vec<ContextMenuItem>,
}

impl ContextMenu {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(mut self, item: impl Into<ContextMenuItem>) -> Self {
        self.items.push(item.into());
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
                align_items: AlignItems::Center,
                border: UiRect::all(px(1)),
                padding: UiRect::vertical(px(6)),
                left: px(position.x),
                top: px(position.y),
                row_gap: px(6),
                border_radius: RoundedCorners::All.to_border_radius(6.0),
                ..default()
            },
            ThemeBorderColor(BORDER),
            ThemeBackgroundColor(WINDOW_BG),
            BoxShadow::from(ShadowStyle {
                blur_radius: px(3),
                x_offset: px(0),
                y_offset: px(0),
                color: Color::BLACK.with_alpha(0.8),
                ..default()
            }),
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
    let mut menu_item = match item.clone() {
        ContextMenuItem::Option {
            mark,
            label,
            callback,
        } => spawn_option(
            commands,
            asset_server,
            root,
            context_menu,
            target,
            mark,
            label,
            callback,
        )?,
        ContextMenuItem::Submenu { label, menu } => {
            spawn_submenu(commands, asset_server, root, context_menu, target, label)?
        }
        ContextMenuItem::Separator => spawn_separator(commands, context_menu)?,
    };

    if !matches!(item, ContextMenuItem::Separator) {
        menu_item
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
            });
    }

    Ok(())
}

fn spawn_option<'a>(
    commands: &'a mut Commands,
    asset_server: &AssetServer,
    root: Entity,
    context_menu: Entity,
    target: Entity,
    mark: ContextMenuMark,
    label: String,
    callback: Arc<dyn Fn(&mut DeferredWorld, Entity) + Send + Sync>,
) -> Result<EntityCommands<'a>> {
    let item = commands
        .spawn((
            ChildOf(context_menu),
            Node {
                margin: UiRect::horizontal(px(6)),
                border_radius: RoundedCorners::All.to_border_radius(4.0),
                padding: UiRect::horizontal(px(8)).with_top(px(4)).with_bottom(px(4)),
                column_gap: px(6),
                ..default()
            },
        ))
        .with_children(|commands| {
            commands
                .spawn(Node {
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                })
                .with_children(|commands| {
                    let mark_node = commands
                        .spawn(Node {
                            width: px(16),
                            height: px(16),
                            ..default()
                        })
                        .id();

                    if matches!(mark, ContextMenuMark::Checked) {
                        commands.commands_mut().entity(mark_node).insert((
                            ImageNode::new(
                                asset_server
                                    .load("embedded://bevy_editor/assets/widget/icons/check.png"),
                            ),
                            ThemeImageColor(BUTTON_TEXT),
                        ));
                    }
                });

            commands
                .spawn((Node {
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },))
                .with_children(|commands| {
                    commands.spawn((
                        Pickable::IGNORE,
                        Text::new(label),
                        TextFont {
                            font: asset_server.load(REGULAR),
                            font_size: 12.0,
                            ..default()
                        },
                        ThemeFontColor(TEXT_MAIN),
                    ));
                })
                .observe(
                    move |trigger: On<Pointer<Click>>,
                          mut world: DeferredWorld,
                          mut commands: Commands| {
                        callback(&mut world, target);
                        commands.entity(root).despawn();
                    },
                );
        })
        .id();

    Ok(commands.entity(item))
}

fn spawn_submenu<'a>(
    commands: &'a mut Commands,
    asset_server: &AssetServer,
    root: Entity,
    context_menu: Entity,
    target: Entity,
    label: String,
) -> Result<EntityCommands<'a>> {
    let item = commands
        .spawn((
            ChildOf(context_menu),
            Node {
                margin: UiRect::horizontal(px(6)),
                border_radius: RoundedCorners::All.to_border_radius(4.0),
                ..default()
            },
        ))
        .with_children(|commands| {
            commands
                .spawn((Node {
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    margin: UiRect::horizontal(px(6)),
                    padding: UiRect::horizontal(px(8)).with_top(px(4)).with_bottom(px(4)),
                    ..default()
                },))
                .with_children(|commands| {
                    commands.spawn((
                        Pickable::IGNORE,
                        Text::new(label),
                        TextFont {
                            font: asset_server.load(REGULAR),
                            font_size: 12.0,
                            ..default()
                        },
                        ThemeFontColor(TEXT_MAIN),
                    ));
                });
        })
        .id();

    Ok(commands.entity(item))
}

fn spawn_separator<'a>(
    commands: &'a mut Commands,
    context_menu: Entity,
) -> Result<EntityCommands<'a>> {
    let item = commands
        .spawn((
            ChildOf(context_menu),
            Node {
                width: percent(100),
                height: px(1),
                ..default()
            },
            ThemeBackgroundColor(BORDER),
        ))
        .id();

    Ok(commands.entity(item))
}
