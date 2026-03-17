use std::sync::Arc;

use bevy::{
    app::{App, Plugin, Update},
    asset::AssetServer,
    camera::NormalizedRenderTarget,
    color::{Alpha, Color},
    ecs::{
        component::Component,
        entity::{ContainsEntity, Entity},
        error::Result,
        hierarchy::ChildOf,
        message::MessageReader,
        observer::On,
        system::{Commands, EntityCommands, Query, Res, SystemState},
        world::{DeferredWorld, World},
    },
    math::Vec2,
    picking::{
        Pickable,
        events::{Click, Out, Over, Pointer, Press},
        pointer::PointerButton,
    },
    text::TextFont,
    ui::{
        AlignItems, AlignSelf, BoxShadow, ComputedNode, FlexDirection, JustifyContent, Node,
        PositionType, ShadowStyle, UiGlobalTransform, UiRect, UiScale, percent, px,
        widget::{ImageNode, Text},
    },
    utils::default,
};

use crate::{
    theme::{
        RoundedCorners, ThemeBackgroundColor, ThemeBorderColor, ThemeImageColor, ThemeTextColor,
        ThemedText,
        constants::fonts::REGULAR,
        tokens::{
            BORDER, BUTTON_TEXT, BUTTON_TEXT_DISABLED, PANE_TAB_ACTIVE, TEXT_DIM, TEXT_MAIN,
            WINDOW_BG,
        },
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
        enabled: bool,
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

#[derive(Component, Clone, Default)]
pub struct ContextMenu {
    pub items: Vec<ContextMenuItem>,
}

impl ContextMenu {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_item(mut self, item: ContextMenuItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn with_option(
        mut self,
        enabled: bool,
        mark: ContextMenuMark,
        label: impl Into<String>,
        callback: impl Fn(&mut DeferredWorld, Entity) + Send + Sync + 'static,
    ) -> Self {
        self.items.push(ContextMenuItem::Option {
            enabled,
            mark,
            label: label.into(),
            callback: Arc::new(callback),
        });
        self
    }

    pub fn with_submenu(mut self, label: impl Into<String>, menu: ContextMenu) -> Self {
        self.items.push(ContextMenuItem::Submenu {
            label: label.into(),
            menu,
        });
        self
    }

    pub fn with_separator(mut self) -> Self {
        self.items.push(ContextMenuItem::Separator);
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

        let root = commands
            .spawn((
                ChildOf(editor_window.root()),
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

        spawn_menu(
            &mut commands,
            &asset_server,
            root,
            click.pointer_location.position / ui_scale.0,
            target,
            menu,
        )?;

        break;
    }

    state.apply(world);

    Ok(())
}

#[derive(Component)]
struct OpenedSubmenu {
    root: Option<Entity>,
}

fn spawn_menu<'a>(
    commands: &'a mut Commands,
    asset_server: &AssetServer,
    root: Entity,
    position: Vec2,
    target: Entity,
    menu: &ContextMenu,
) -> Result<EntityCommands<'a>> {
    let context_menu = commands
        .spawn((
            ChildOf(root),
            OpenedSubmenu { root: None },
            Node {
                position_type: PositionType::Absolute,
                flex_direction: FlexDirection::Column,
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
        .observe(|mut trigger: On<Pointer<Press>>| {
            trigger.propagate(false);
        })
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
            enabled,
            mark,
            label,
            callback,
        } => spawn_option(
            commands,
            asset_server,
            root,
            context_menu,
            target,
            enabled,
            mark,
            label,
            callback,
        )?,
        ContextMenuItem::Submenu { label, menu } => spawn_submenu(
            commands,
            asset_server,
            root,
            context_menu,
            target,
            label,
            menu,
        )?,
        ContextMenuItem::Separator => spawn_separator(commands, context_menu)?,
    };

    if matches!(item, ContextMenuItem::Submenu { .. })
        || matches!(item, ContextMenuItem::Option { enabled, .. } if enabled)
    {
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
    enabled: bool,
    mark: ContextMenuMark,
    label: String,
    callback: Arc<dyn Fn(&mut DeferredWorld, Entity) + Send + Sync>,
) -> Result<EntityCommands<'a>> {
    let text_color = if enabled { TEXT_MAIN } else { TEXT_DIM };

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
                .spawn((
                    Node {
                        align_self: AlignSelf::Start,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        width: px(16),
                        height: px(16),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .with_children(|commands| {
                    let mark_node = commands
                        .spawn((
                            Node {
                                width: percent(100),
                                height: percent(100),
                                ..default()
                            },
                            Pickable::IGNORE,
                        ))
                        .id();

                    if matches!(mark, ContextMenuMark::Checked) {
                        commands.commands_mut().entity(mark_node).insert((
                            ImageNode::new(
                                asset_server
                                    .load("embedded://bevy_editor/assets/widget/icons/check.png"),
                            ),
                            ThemeImageColor(text_color.clone()),
                        ));
                    }
                });

            commands
                .spawn((
                    Pickable::IGNORE,
                    Node {
                        align_self: AlignSelf::Start,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                ))
                .with_children(|commands| {
                    commands.spawn((
                        Pickable::IGNORE,
                        Text::new(label),
                        TextFont {
                            font: asset_server.load(REGULAR),
                            font_size: 12.0,
                            ..default()
                        },
                        ThemeTextColor(text_color),
                    ));
                });
        })
        .observe(
            move |_: On<Pointer<Over>>,
                  submenus: Query<&OpenedSubmenu>,
                  mut commands: Commands|
                  -> Result {
                let submenu = submenus.get(context_menu)?;
                if let Some(root) = submenu.root {
                    commands.entity(root).despawn();
                }

                Ok(())
            },
        )
        .observe(
            move |_: On<Pointer<Click>>, mut world: DeferredWorld, mut commands: Commands| {
                if enabled {
                    callback(&mut world, target);
                    commands.entity(root).despawn();
                }
            },
        )
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
    menu: ContextMenu,
) -> Result<EntityCommands<'a>> {
    let item =
        commands
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
                commands.spawn((
                    Node {
                        align_self: AlignSelf::Start,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        width: px(16),
                        height: px(16),
                        ..default()
                    },
                    Pickable::IGNORE,
                ));

                commands
                    .spawn((
                        Node {
                            align_self: AlignSelf::Start,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        Pickable::IGNORE,
                    ))
                    .with_children(|commands| {
                        commands.spawn((
                            Pickable::IGNORE,
                            Text::new(label),
                            TextFont {
                                font: asset_server.load(REGULAR),
                                font_size: 12.0,
                                ..default()
                            },
                            ThemeTextColor(TEXT_MAIN),
                        ));
                    });

                commands
                    .spawn((
                        Node {
                            align_self: AlignSelf::End,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            width: px(16),
                            height: px(16),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ))
                    .with_children(|commands| {
                        commands.spawn((
                            Pickable::IGNORE,
                            ImageNode::new(asset_server.load(
                                "embedded://bevy_editor/assets/widget/icons/chevron_right.png",
                            )),
                            ThemeImageColor(BUTTON_TEXT),
                        ));
                    });
            })
            .observe(
                move |trigger: On<Pointer<Over>>,
                      computed_nodes: Query<&ComputedNode>,
                      ui_global_transforms: Query<&UiGlobalTransform>,
                      asset_server: Res<AssetServer>,
                      mut submenus: Query<&mut OpenedSubmenu>,
                      mut commands: Commands|
                      -> Result {
                    let item_translation = ui_global_transforms.get(trigger.entity)?.translation;
                    let item_size = computed_nodes.get(trigger.entity)?.size();
                    let menu_size = computed_nodes.get(context_menu)?.size();

                    let submenu_position = Vec2::new(
                        item_translation.x + menu_size.x / 2.0,
                        item_translation.y - item_size.y / 2.0,
                    );

                    let new_submenu = spawn_menu(
                        &mut commands,
                        &asset_server,
                        root,
                        submenu_position,
                        target,
                        &menu,
                    )?
                    .id();

                    let mut submenu = submenus.get_mut(context_menu)?;

                    if let Some(old_submenu) = submenu.root.replace(new_submenu) {
                        commands.entity(old_submenu).despawn();
                    }

                    Ok(())
                },
            )
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
