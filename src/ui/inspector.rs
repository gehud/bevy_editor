//! The Components panel: the primary selected entity's components, edited through reflection.
//!
//! The cards are rebuilt only when the shown entity, its set of components, a card's collapsed
//! state or the filter change. Each field widget carries a [`FieldBinding`] (component type plus
//! reflection path); every frame [`refresh_inspector_fields`] copies the world's values into the
//! widgets (except the one being typed into), and edits are written back through reflection.
//!
//! - Numbers, `bool`s, `String`s, `Vec2`/`Vec3`/`Vec4` and `Quat`s (as Euler angles in degrees)
//!   are editable; structs and tuple structs nest; other values (enums, lists, handles, …) are
//!   shown read-only. Immutable components are shown read-only.
//! - `Name` is edited as text.
//! - Click a card header to collapse it (remembered per component type).
//! - The filter field hides cards whose name doesn't contain its text.

use std::any::TypeId;

use bevy::{
    ecs::{
        component::{ComponentId, ComponentInfo, Components},
        world::EntityRef,
    },
    feathers::{
        controls::{
            FeathersCheckbox, FeathersNumberInput, FeathersTextInput, FeathersTextInputContainer,
            NumberInputValue,
        },
        palette,
        theme::ThemeToken,
        tokens as feathers_tokens,
    },
    input::keyboard::{Key, KeyboardInput},
    input_focus::{FocusLost, FocusedInput, InputFocus},
    picking::events::PointerClick,
    platform::collections::HashSet,
    prelude::*,
    reflect::{ReflectPath, ReflectRef, TypeRegistry},
    text::EditableText,
    ui::{Checked, InteractionDisabled},
    ui_widgets::{Button, ScrollArea, ValueChange},
};

use crate::{editing::Selection, ui::icons};

use super::{
    style::{Weight, colors, icon_sq, ink, text},
    widgets::{filter_bar, wide_button},
};

/// Nesting depth below which structs are no longer expanded.
const MAX_DEPTH: usize = 4;
/// Longest read-only value text, in characters.
const MAX_TEXT: usize = 80;

/// Components panel contents: filter, "Add Component" and the component cards.
pub fn components_content() -> impl Scene {
    bsn! {
        Node {
            flex_grow: 1.0,
            min_height: px(0),
            flex_direction: FlexDirection::Column,
            row_gap: px(8),
            padding: UiRect::all(px(6)),
        }
        Children [
            Node { flex_direction: FlexDirection::Column, row_gap: px(6), width: percent(100) }
            Children [
                @filter_bar()
                InspectorFilter
                --
                @wide_button(icons::PACKAGE_PLUS, "Add Component")
            ]
            --
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                width: percent(100),
                flex_grow: 1.0,
                min_height: px(0),
                overflow: Overflow::scroll_y(),
            }
            ScrollArea
            InspectorBody
        ]
        BackgroundColor(colors::PANE_BG)
    }
}

/// The container the cards are rebuilt into.
#[derive(Component, Clone, Default)]
pub(super) struct InspectorBody;

/// Marks the Components panel's filter bar (the text input is inside it).
#[derive(Component, Clone, Default)]
pub(super) struct InspectorFilter;

/// Components panel state.
#[derive(Resource, Default)]
pub struct InspectorState {
    /// Component types whose cards are collapsed.
    collapsed: HashSet<TypeId>,
    /// The filter text, trimmed and lowercased.
    filter: String,
}

/// What the cards were last built from.
#[derive(PartialEq, Default)]
pub(super) struct Shown {
    entity: Option<Entity>,
    /// The listed components and whether each card is collapsed.
    cards: Vec<(ComponentId, bool)>,
}

/// Copies the Components filter field's text into [`InspectorState::filter`].
pub(super) fn read_inspector_filter(
    inputs: Query<(Entity, &EditableText), Changed<EditableText>>,
    filter_bars: Query<(), With<InspectorFilter>>,
    parents: Query<&ChildOf>,
    mut state: ResMut<InspectorState>,
) {
    for (input, text) in &inputs {
        if parents
            .iter_ancestors(input)
            .any(|ancestor| filter_bars.contains(ancestor))
        {
            let filter = text.value().to_string().trim().to_lowercase();
            if state.filter != filter {
                state.filter = filter;
            }
        }
    }
}

/// A component's card title: its short type name.
fn card_title(info: &ComponentInfo, registry: &TypeRegistry) -> String {
    info.type_id()
        .and_then(|type_id| registry.get(type_id))
        .map(|registration| {
            registration
                .type_info()
                .type_path_table()
                .short_path()
                .to_owned()
        })
        .unwrap_or_else(|| info.name().shortname().to_string())
}

/// Rebuilds the cards when the shown entity or its components change.
#[expect(clippy::too_many_arguments, reason = "a Bevy system")]
pub(super) fn sync_inspector(
    mut commands: Commands,
    body: Single<Entity, With<InspectorBody>>,
    selection: Res<Selection>,
    entities: Query<EntityRef, Without<Node>>,
    components: &Components,
    registry: Res<AppTypeRegistry>,
    state: Res<InspectorState>,
    mut last: Local<Option<Shown>>,
) {
    let registry = registry.read();
    let entity = selection.primary().and_then(|e| entities.get(e).ok());

    let mut cards: Vec<(ComponentId, &ComponentInfo, String)> = Vec::new();
    if let Some(entity) = entity {
        for &id in entity.archetype().components() {
            let Some(info) = components.get_info(id) else {
                continue;
            };
            let title = card_title(info, &registry);
            if title.to_lowercase().contains(&state.filter) {
                cards.push((id, info, title));
            }
        }
        // `Name` and `Transform` come first, as in other editors; the rest keep their order.
        let rank = |info: &ComponentInfo| match info.type_id() {
            Some(t) if t == TypeId::of::<Name>() => 0,
            Some(t) if t == TypeId::of::<Transform>() => 1,
            _ => 2,
        };
        cards.sort_by_key(|(_, info, _)| rank(info));
    }
    let is_collapsed =
        |info: &ComponentInfo| info.type_id().is_some_and(|t| state.collapsed.contains(&t));
    let shown = Shown {
        entity: entity.map(|e| e.id()),
        cards: cards
            .iter()
            .map(|(id, info, _)| (*id, is_collapsed(info)))
            .collect(),
    };
    if last.as_ref() == Some(&shown) {
        return;
    }

    let scenes: Vec<Box<dyn Scene>> = match entity {
        None => vec![Box::new(note("Nothing selected"))],
        Some(_) if cards.is_empty() => vec![Box::new(note("No matching components"))],
        Some(entity) => cards
            .into_iter()
            .map(|(_, info, title)| card(entity, info, title, is_collapsed(info), &registry))
            .collect(),
    };
    commands
        .entity(*body)
        .despawn_related::<Children>()
        .apply_scene(bsn! { Children [ {scenes} ] });
    *last = Some(shown);
}

/// A dim one-line message in place of the cards.
fn note(message: &'static str) -> impl Scene {
    bsn! {
        @text(message, 12.0, Weight::Regular, ink::DIM)
        Node { margin: UiRect::axes(px(6), px(4)) }
    }
}

// --- Cards ------------------------------------------------------------------------------------

/// Identifies the component type a card header collapses.
#[derive(Component, Clone, Copy)]
struct CardToggle(TypeId);

impl Default for CardToggle {
    fn default() -> Self {
        Self(TypeId::of::<()>())
    }
}

/// One component's card: a header (click to collapse) and its fields.
fn card(
    entity: EntityRef,
    info: &ComponentInfo,
    title: String,
    collapsed: bool,
    registry: &TypeRegistry,
) -> Box<dyn Scene> {
    let type_id = info.type_id();
    let reflected = type_id.and_then(|t| {
        let reflect = registry.get_type_data::<ReflectComponent>(t)?;
        Some((t, reflect.reflect(entity)?))
    });

    let fields: Vec<Box<dyn Scene>> = match reflected {
        None => vec![Box::new(bsn! {
            @text("Not reflected", 11.0, Weight::Regular, ink::DIM)
        })],
        Some((t, _)) if t == TypeId::of::<Name>() => {
            let binding = FieldBinding::new(entity.id(), t, String::new(), FieldKind::Name);
            vec![text_field(None, binding, true)]
        }
        // Its matrix is unreadable as fields: show it decomposed, read-only.
        Some((t, _)) if t == TypeId::of::<GlobalTransform>() => {
            ["Translation", "Rotation", "Scale"]
                .into_iter()
                .enumerate()
                .map(|(part, caption)| {
                    let axes = (0..3)
                        .map(|axis| {
                            let kind = FieldKind::GlobalPart { part, axis };
                            FieldBinding::new(entity.id(), t, String::new(), kind)
                        })
                        .collect();
                    vector_field(Some(caption.to_owned()), axes, false)
                })
                .collect()
        }
        Some((t, value)) => {
            let cx = FieldCx {
                entity: entity.id(),
                component: t,
                editable: info.mutable(),
            };
            struct_fields(value.as_partial_reflect(), "", &cx, 0)
        }
    };

    let is_transform = type_id
        .is_some_and(|t| t == TypeId::of::<Transform>() || t == TypeId::of::<GlobalTransform>());
    let kind_icon = if is_transform {
        icons::MOVE_3D_SMALL
    } else {
        icons::BOX
    };
    let chevron = if collapsed {
        icons::CHEVRON_RIGHT
    } else {
        icons::CHEVRON_DOWN
    };
    let toggle = CardToggle(type_id.unwrap_or(TypeId::of::<()>()));

    let mut children: Vec<Box<dyn Scene>> = vec![Box::new(bsn! {
        Button
        Node {
            height: px(26),
            width: percent(100),
            padding: UiRect::horizontal(px(8)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            border_radius: BorderRadius::top(px(5)),
        }
        BackgroundColor(colors::ITEM_BG)
        toggle
        on(toggle_card)
        Children [
            @icon_sq(chevron, 15.0, ink::C4C4)
            Pickable::IGNORE
            --
            Node { column_gap: px(5), align_items: AlignItems::Center, min_width: px(0) }
            Pickable::IGNORE
            Children [
                @icon_sq(kind_icon, 13.0, ink::C4C4)
                Pickable::IGNORE
                --
                @text(title, 12.0, Weight::Medium, ink::DCDC)
                TextShadow { offset: Vec2::new(0.0, 1.0), color: Color::srgba(0.0, 0.0, 0.0, 0.2) }
                Pickable::IGNORE
            ]
            --
            @icon_sq(icons::ELLIPSIS, 15.0, ink::C4C4)
            Pickable::IGNORE
        ]
    })];
    if !collapsed {
        children.push(Box::new(bsn! {
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(4),
                padding: UiRect { left: px(7), right: px(7), top: px(5), bottom: px(7) },
            }
            Children [ {fields} ]
        }));
    }

    Box::new(bsn! {
        Node {
            width: percent(100),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            overflow: Overflow::clip(),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(5)),
        }
        BackgroundColor(colors::PANE_BG)
        BorderColor { top: colors::ITEM_BORDER, bottom: colors::ITEM_BORDER, left: colors::ITEM_BORDER, right: colors::ITEM_BORDER }
        BoxShadow(vec![ShadowStyle {
            color: Color::srgba(0.0, 0.0, 0.0, 0.88),
            x_offset: px(0),
            y_offset: px(0),
            spread_radius: px(0),
            blur_radius: px(1),
        }])
        Children [ {children} ]
    })
}

fn toggle_card(
    mut click: On<PointerClick>,
    toggles: Query<&CardToggle>,
    mut state: ResMut<InspectorState>,
) {
    let Ok(CardToggle(type_id)) = toggles.get(click.entity) else {
        return;
    };
    click.propagate(false);
    if click.button == PointerButton::Primary && !state.collapsed.remove(type_id) {
        state.collapsed.insert(*type_id);
    }
}

// --- Fields -----------------------------------------------------------------------------------

/// What fields are built for.
struct FieldCx {
    entity: Entity,
    component: TypeId,
    editable: bool,
}

/// Ties a field widget to a value in the world: `path` inside the `component` of `entity`.
#[derive(Component, Clone)]
pub(super) struct FieldBinding {
    entity: Entity,
    component: TypeId,
    /// A reflection path such as `.translation.x`; empty for the component itself.
    path: String,
    kind: FieldKind,
}

impl Default for FieldBinding {
    fn default() -> Self {
        Self::new(
            Entity::PLACEHOLDER,
            TypeId::of::<()>(),
            String::new(),
            FieldKind::ReadOnly,
        )
    }
}

impl FieldBinding {
    fn new(entity: Entity, component: TypeId, path: String, kind: FieldKind) -> Self {
        Self {
            entity,
            component,
            path,
            kind,
        }
    }
}

/// How a field shows and edits its value.
#[derive(Clone, Copy, PartialEq, Debug)]
enum FieldKind {
    Number(NumberKind),
    /// One Euler angle (XYZ order, in degrees) of a `Quat`.
    EulerAxis(usize),
    Bool,
    Text,
    /// The `Name` component.
    Name,
    /// One axis of the translation (`part` 0), Euler rotation in degrees (1) or scale (2) of a
    /// `GlobalTransform`.
    GlobalPart {
        part: usize,
        axis: usize,
    },
    ReadOnly,
}

/// The Rust number type behind a number field.
#[derive(Clone, Copy, PartialEq, Debug)]
enum NumberKind {
    F32,
    F64,
    I8,
    I16,
    I32,
    I64,
    Isize,
    U8,
    U16,
    U32,
    U64,
    Usize,
}

/// Runs `$body` with `$t` bound to the integer type of each integer [`NumberKind`].
macro_rules! for_int_kind {
    ($kind:expr, $t:ident => $body:expr, $other:expr) => {
        match $kind {
            NumberKind::I8 => {
                type $t = i8;
                $body
            }
            NumberKind::I16 => {
                type $t = i16;
                $body
            }
            NumberKind::I32 => {
                type $t = i32;
                $body
            }
            NumberKind::I64 => {
                type $t = i64;
                $body
            }
            NumberKind::Isize => {
                type $t = isize;
                $body
            }
            NumberKind::U8 => {
                type $t = u8;
                $body
            }
            NumberKind::U16 => {
                type $t = u16;
                $body
            }
            NumberKind::U32 => {
                type $t = u32;
                $body
            }
            NumberKind::U64 => {
                type $t = u64;
                $body
            }
            NumberKind::Usize => {
                type $t = usize;
                $body
            }
            NumberKind::F32 | NumberKind::F64 => $other,
        }
    };
}

impl NumberKind {
    const ALL: [NumberKind; 12] = [
        NumberKind::F32,
        NumberKind::F64,
        NumberKind::I8,
        NumberKind::I16,
        NumberKind::I32,
        NumberKind::I64,
        NumberKind::Isize,
        NumberKind::U8,
        NumberKind::U16,
        NumberKind::U32,
        NumberKind::U64,
        NumberKind::Usize,
    ];

    /// The kind of number `value` is, if it is one.
    fn of(value: &dyn PartialReflect) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.read(value).is_some())
    }

    /// The value as a number input value: `f32`/`f64` stay as they are, integers become `i64`.
    #[allow(
        clippy::unnecessary_fallible_conversions,
        clippy::useless_conversion,
        reason = "one macro body covers every integer type; only u64/usize can overflow i64"
    )]
    fn read(self, value: &dyn PartialReflect) -> Option<NumberInputValue> {
        match self {
            NumberKind::F32 => value
                .try_downcast_ref::<f32>()
                .map(|v| NumberInputValue::F32(*v)),
            NumberKind::F64 => value
                .try_downcast_ref::<f64>()
                .map(|v| NumberInputValue::F64(*v)),
            kind => for_int_kind!(kind, T => value
                .try_downcast_ref::<T>()
                .map(|v| NumberInputValue::I64(i64::try_from(*v).unwrap_or(i64::MAX))), None),
        }
    }

    /// Stores `input` into `target`, clamping integers into the target type's range.
    fn write(self, target: &mut dyn PartialReflect, input: NumberInputValue) {
        let number = match input {
            NumberInputValue::F32(v) => f64::from(v),
            NumberInputValue::F64(v) => v,
            NumberInputValue::I32(v) => f64::from(v),
            NumberInputValue::I64(v) => v as f64,
        };
        match self {
            NumberKind::F32 => {
                if let Some(v) = target.try_downcast_mut::<f32>() {
                    *v = number as f32;
                }
            }
            NumberKind::F64 => {
                if let Some(v) = target.try_downcast_mut::<f64>() {
                    *v = number;
                }
            }
            kind => for_int_kind!(kind, T => {
                if let Some(v) = target.try_downcast_mut::<T>() {
                    // `as` from a float saturates at the type's bounds.
                    *v = number.round() as T;
                }
            }, ()),
        }
    }
}

/// Converts `snake_case` field names to "Title Case" captions.
fn caption(name: &str) -> String {
    name.split('_')
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Fields for the members of a struct, tuple struct or tuple; one read-only field otherwise.
fn struct_fields(
    value: &dyn PartialReflect,
    path: &str,
    cx: &FieldCx,
    depth: usize,
) -> Vec<Box<dyn Scene>> {
    let members: Vec<(String, &dyn PartialReflect)> = match value.reflect_ref() {
        ReflectRef::Struct(s) => (0..s.field_len())
            .filter_map(|i| Some((s.name_at(i)?.to_owned(), s.field_at(i)?)))
            .collect(),
        ReflectRef::TupleStruct(s) => (0..s.field_len())
            .filter_map(|i| Some((i.to_string(), s.field(i)?)))
            .collect(),
        ReflectRef::Tuple(t) => (0..t.field_len())
            .filter_map(|i| Some((i.to_string(), t.field(i)?)))
            .collect(),
        _ => return vec![field(None, value, path.to_owned(), cx, depth)],
    };
    if members.is_empty() {
        return vec![Box::new(bsn! {
            @text("No fields", 11.0, Weight::Regular, ink::DIM)
        })];
    }
    // A lone tuple field ("0") needs no caption.
    let lone_index = members.len() == 1 && members[0].0 == "0";
    members
        .into_iter()
        .map(|(name, member)| {
            field(
                (!lone_index).then(|| caption(&name)),
                member,
                format!("{path}.{name}"),
                cx,
                depth,
            )
        })
        .collect()
}

/// The widget for one value.
fn field(
    caption: Option<String>,
    value: &dyn PartialReflect,
    path: String,
    cx: &FieldCx,
    depth: usize,
) -> Box<dyn Scene> {
    let bind = |path: String, kind| FieldBinding::new(cx.entity, cx.component, path, kind);

    let vector_axes = if value.try_downcast_ref::<Vec2>().is_some() {
        2
    } else if value.try_downcast_ref::<Vec3>().is_some()
        || value.try_downcast_ref::<Vec3A>().is_some()
    {
        3
    } else if value.try_downcast_ref::<Vec4>().is_some() {
        4
    } else {
        0
    };
    if vector_axes > 0 {
        let axes = ["x", "y", "z", "w"][..vector_axes]
            .iter()
            .map(|axis| bind(format!("{path}.{axis}"), FieldKind::Number(NumberKind::F32)))
            .collect();
        return vector_field(caption, axes, cx.editable);
    }
    if value.try_downcast_ref::<Quat>().is_some() {
        let axes = (0..3)
            .map(|axis| bind(path.clone(), FieldKind::EulerAxis(axis)))
            .collect();
        return vector_field(caption, axes, cx.editable);
    }
    if let Some(kind) = NumberKind::of(value) {
        return number_field(caption, bind(path, FieldKind::Number(kind)), cx.editable);
    }
    if value.try_downcast_ref::<bool>().is_some() {
        return bool_field(caption, bind(path, FieldKind::Bool), cx.editable);
    }
    if value.try_downcast_ref::<String>().is_some() {
        return text_field(caption, bind(path, FieldKind::Text), cx.editable);
    }

    let nested = matches!(
        value.reflect_ref(),
        ReflectRef::Struct(_) | ReflectRef::TupleStruct(_) | ReflectRef::Tuple(_)
    );
    if nested && depth < MAX_DEPTH {
        // A newtype shows its single field in place.
        if let ReflectRef::TupleStruct(s) = value.reflect_ref()
            && s.field_len() == 1
            && let Some(inner) = s.field(0)
        {
            return field(caption, inner, format!("{path}.0"), cx, depth + 1);
        }
        let members = struct_fields(value, &path, cx, depth + 1);
        return captioned(
            caption,
            Box::new(bsn! {
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(4),
                    width: percent(100),
                    padding: UiRect::left(px(8)),
                    border: UiRect::left(px(1)),
                }
                BorderColor::all(colors::TREE_GUIDE)
                Children [ {members} ]
            }),
        );
    }
    read_only_field(caption, bind(path, FieldKind::ReadOnly))
}

/// Small caption above a field.
fn field_caption(caption: String) -> impl Scene {
    bsn! {
        Node { padding: UiRect::vertical(px(3)) }
        Children [
            @text(caption, 10.0, Weight::Regular, ink::D1D1.with_alpha(0.8))
            TextShadow { offset: Vec2::new(0.0, 1.0), color: ink::SHADOW }
        ]
    }
}

/// `content` under an optional caption.
fn captioned(caption: Option<String>, content: Box<dyn Scene>) -> Box<dyn Scene> {
    match caption {
        None => content,
        Some(caption) => Box::new(bsn! {
            Node { flex_direction: FlexDirection::Column, row_gap: px(2), width: percent(100) }
            Children [
                @field_caption(caption)
                --
                @{content}
            ]
        }),
    }
}

/// Adds [`InteractionDisabled`] to `scene` unless `editable`.
fn disabled_unless(editable: bool, scene: impl Scene) -> Box<dyn Scene> {
    if editable {
        Box::new(scene)
    } else {
        Box::new(bsn! { @{scene} InteractionDisabled })
    }
}

fn number_field(caption: Option<String>, binding: FieldBinding, editable: bool) -> Box<dyn Scene> {
    let input = bsn! {
        @FeathersNumberInput
        Node { width: percent(100), height: px(22) }
        binding
        on(number_changed_f32)
        on(number_changed_f64)
        on(number_changed_i64)
    };
    captioned(caption, disabled_unless(editable, input))
}

/// A row of number inputs with colored X/Y/Z/W sigils.
fn vector_field(
    caption: Option<String>,
    axes: Vec<FieldBinding>,
    editable: bool,
) -> Box<dyn Scene> {
    let sigils: [(&'static str, ThemeToken, Color); 4] = [
        ("X", feathers_tokens::TEXT_INPUT_X_AXIS, palette::X_AXIS),
        ("Y", feathers_tokens::TEXT_INPUT_Y_AXIS, palette::Y_AXIS),
        ("Z", feathers_tokens::TEXT_INPUT_Z_AXIS, palette::Z_AXIS),
        ("W", feathers_tokens::TEXT_INPUT_LABEL_BG, ink::DIM),
    ];
    let inputs: Vec<Box<dyn Scene>> = axes
        .into_iter()
        .zip(sigils)
        .map(|(binding, (label, sigil, color))| {
            let input = bsn! {
                @FeathersNumberInput {
                    @sigil_color: sigil,
                    @label_text: label,
                }
                Node {
                    flex_grow: 1.0,
                    flex_basis: px(0),
                    min_width: px(0),
                    height: px(22),
                    border: UiRect::left(px(2)),
                }
                BorderColor::all(color)
                binding
                on(number_changed_f32)
                on(number_changed_f64)
                on(number_changed_i64)
            };
            disabled_unless(editable, input)
        })
        .collect();
    captioned(
        caption,
        Box::new(bsn! {
            Node { width: percent(100), column_gap: px(8), padding: UiRect::left(px(1)) }
            Children [ {inputs} ]
        }),
    )
}

fn bool_field(caption: Option<String>, binding: FieldBinding, editable: bool) -> Box<dyn Scene> {
    let caption = caption.unwrap_or_default();
    let checkbox = disabled_unless(
        editable,
        bsn! {
            @FeathersCheckbox
            binding
            on(bool_changed)
        },
    );
    Box::new(bsn! {
        Node { column_gap: px(6), align_items: AlignItems::Center, padding: UiRect::vertical(px(2)) }
        Children [
            @{checkbox}
            --
            @text(caption, 11.0, Weight::Regular, ink::D1D1)
        ]
    })
}

fn text_field(caption: Option<String>, binding: FieldBinding, editable: bool) -> Box<dyn Scene> {
    let input = disabled_unless(
        editable,
        bsn! {
            @FeathersTextInput
            binding
            on(text_keys)
            on(text_focus_lost)
        },
    );
    captioned(
        caption,
        Box::new(bsn! {
            @FeathersTextInputContainer
            Node {
                height: px(22),
                width: percent(100),
                flex_grow: 0.0,
                padding: UiRect::horizontal(px(6)),
            }
            Children [ @{input} ]
        }),
    )
}

fn read_only_field(caption: Option<String>, binding: FieldBinding) -> Box<dyn Scene> {
    captioned(
        caption,
        Box::new(bsn! {
            @text("", 11.0, Weight::Regular, ink::C2C2)
            Node { padding: UiRect::left(px(2)) }
            binding
        }),
    )
}

// --- Reading and writing values ---------------------------------------------------------------

/// A field's current value in the world.
#[derive(PartialEq, Debug)]
enum FieldValue {
    Number(NumberInputValue),
    Bool(bool),
    Text(String),
}

/// The value at `path` inside `root` (`root` itself for an empty path).
fn at_path<'a>(root: &'a dyn PartialReflect, path: &str) -> Option<&'a dyn PartialReflect> {
    if path.is_empty() {
        Some(root)
    } else {
        path.reflect_element(root).ok()
    }
}

/// Short text for a value that can't be edited.
fn describe(value: &dyn PartialReflect) -> String {
    let mut text = match value.reflect_ref() {
        ReflectRef::Enum(e) if e.field_len() == 0 => e.variant_name().to_owned(),
        ReflectRef::List(list) => format!("{} items", list.len()),
        ReflectRef::Array(array) => format!("{} items", array.len()),
        ReflectRef::Map(map) => format!("{} entries", map.len()),
        ReflectRef::Set(set) => format!("{} items", set.len()),
        _ => format!("{value:?}"),
    };
    if text.chars().count() > MAX_TEXT {
        text = text.chars().take(MAX_TEXT - 1).collect::<String>() + "…";
    }
    text
}

/// Rounds degrees so that converting through a `Quat` doesn't show float noise.
fn round_degrees(radians: f32) -> f32 {
    (radians.to_degrees() * 1000.0).round() / 1000.0
}

fn read_value(
    scene: &Query<EntityRef, Without<Node>>,
    registry: &TypeRegistry,
    binding: &FieldBinding,
) -> Option<FieldValue> {
    let entity = scene.get(binding.entity).ok()?;
    match binding.kind {
        FieldKind::Name => {
            return Some(FieldValue::Text(entity.get::<Name>()?.as_str().to_owned()));
        }
        FieldKind::GlobalPart { part, axis } => {
            let transform = entity.get::<GlobalTransform>()?.compute_transform();
            let value = match part {
                0 => transform.translation[axis],
                1 => {
                    let (x, y, z) = transform.rotation.to_euler(EulerRot::XYZ);
                    round_degrees([x, y, z][axis])
                }
                _ => transform.scale[axis],
            };
            return Some(FieldValue::Number(NumberInputValue::F32(value)));
        }
        _ => {}
    }
    let component = registry
        .get_type_data::<ReflectComponent>(binding.component)?
        .reflect(entity)?;
    let value = at_path(component.as_partial_reflect(), &binding.path)?;
    Some(match binding.kind {
        FieldKind::Number(kind) => FieldValue::Number(kind.read(value)?),
        FieldKind::EulerAxis(axis) => {
            let (x, y, z) = value.try_downcast_ref::<Quat>()?.to_euler(EulerRot::XYZ);
            FieldValue::Number(NumberInputValue::F32(round_degrees([x, y, z][axis])))
        }
        FieldKind::Bool => FieldValue::Bool(*value.try_downcast_ref::<bool>()?),
        FieldKind::Text => FieldValue::Text(value.try_downcast_ref::<String>()?.clone()),
        FieldKind::ReadOnly => FieldValue::Text(describe(value)),
        FieldKind::Name | FieldKind::GlobalPart { .. } => unreachable!(),
    })
}

/// Copies the world's values into the field widgets, except the one being edited.
#[expect(clippy::too_many_arguments, reason = "a Bevy system")]
pub(super) fn refresh_inspector_fields(
    mut commands: Commands,
    fields: Query<(
        Entity,
        &FieldBinding,
        Option<&NumberInputValue>,
        Has<Checked>,
    )>,
    mut inputs: Query<&mut EditableText, With<Node>>,
    mut labels: Query<&mut Text, With<Node>>,
    scene: Query<EntityRef, Without<Node>>,
    parents: Query<&ChildOf>,
    registry: Res<AppTypeRegistry>,
    focus: Res<InputFocus>,
) {
    let registry = registry.read();
    let editing = |field: Entity| {
        focus
            .get()
            .is_some_and(|f| f == field || parents.iter_ancestors(f).any(|a| a == field))
    };
    for (field, binding, number, checked) in &fields {
        if editing(field) {
            continue;
        }
        let Some(value) = read_value(&scene, &registry, binding) else {
            continue;
        };
        match value {
            FieldValue::Number(value) => {
                if number != Some(&value) {
                    commands.entity(field).insert(value);
                }
            }
            FieldValue::Bool(value) => {
                if value != checked {
                    if value {
                        commands.entity(field).insert(Checked);
                    } else {
                        commands.entity(field).remove::<Checked>();
                    }
                }
            }
            FieldValue::Text(value) => {
                if let Ok(mut input) = inputs.get_mut(field) {
                    if input.value() != value.as_str() {
                        input.editor_mut().set_text(&value);
                    }
                } else if let Ok(mut label) = labels.get_mut(field)
                    && label.0 != value
                {
                    label.0 = value;
                }
            }
        }
    }
}

/// Changes the value a binding points at, through reflection.
fn write_field(
    commands: &mut Commands,
    binding: &FieldBinding,
    write: impl FnOnce(&mut dyn PartialReflect) + Send + 'static,
) {
    let FieldBinding {
        entity,
        component,
        path,
        ..
    } = binding.clone();
    commands.queue(move |world: &mut World| {
        let registry = world.resource::<AppTypeRegistry>().clone();
        let registry = registry.read();
        let Some(reflect) = registry.get_type_data::<ReflectComponent>(component) else {
            return;
        };
        let Ok(mut entity) = world.get_entity_mut(entity) else {
            return;
        };
        let Some(mut value) = reflect.reflect_mut(&mut entity) else {
            return;
        };
        let root = value.as_partial_reflect_mut();
        let target = if path.is_empty() {
            Some(root)
        } else {
            path.as_str().reflect_element_mut(root).ok()
        };
        if let Some(target) = target {
            write(target);
        }
    });
}

fn apply_number(
    source: Entity,
    value: NumberInputValue,
    fields: &Query<&FieldBinding>,
    commands: &mut Commands,
) {
    let Ok(binding) = fields.get(source) else {
        return;
    };
    match binding.kind {
        FieldKind::Number(kind) => write_field(commands, binding, move |t| kind.write(t, value)),
        FieldKind::EulerAxis(axis) => {
            let NumberInputValue::F32(degrees) = value else {
                return;
            };
            write_field(commands, binding, move |target| {
                if let Some(rotation) = target.try_downcast_mut::<Quat>() {
                    let (x, y, z) = rotation.to_euler(EulerRot::XYZ);
                    let mut angles = [x, y, z];
                    angles[axis] = degrees.to_radians();
                    *rotation = Quat::from_euler(EulerRot::XYZ, angles[0], angles[1], angles[2]);
                }
            });
        }
        _ => return,
    }
    // Show the new value right away (it also arrives while dragging, before the world changes).
    commands.entity(source).insert(value);
}

fn number_changed_f32(
    change: On<ValueChange<f32>>,
    fields: Query<&FieldBinding>,
    mut commands: Commands,
) {
    apply_number(
        change.source,
        NumberInputValue::F32(change.value),
        &fields,
        &mut commands,
    );
}

fn number_changed_f64(
    change: On<ValueChange<f64>>,
    fields: Query<&FieldBinding>,
    mut commands: Commands,
) {
    apply_number(
        change.source,
        NumberInputValue::F64(change.value),
        &fields,
        &mut commands,
    );
}

fn number_changed_i64(
    change: On<ValueChange<i64>>,
    fields: Query<&FieldBinding>,
    mut commands: Commands,
) {
    apply_number(
        change.source,
        NumberInputValue::I64(change.value),
        &fields,
        &mut commands,
    );
}

fn bool_changed(
    change: On<ValueChange<bool>>,
    fields: Query<&FieldBinding>,
    mut commands: Commands,
) {
    let Ok(binding) = fields.get(change.source) else {
        return;
    };
    let value = change.value;
    write_field(&mut commands, binding, move |target| {
        if let Some(v) = target.try_downcast_mut::<bool>() {
            *v = value;
        }
    });
    if value {
        commands.entity(change.source).insert(Checked);
    } else {
        commands.entity(change.source).remove::<Checked>();
    }
}

/// Applies a text field's contents.
fn commit_text(
    field: Entity,
    fields: &Query<(&FieldBinding, &EditableText)>,
    commands: &mut Commands,
) {
    let Ok((binding, input)) = fields.get(field) else {
        return;
    };
    let text = input.value().to_string();
    match binding.kind {
        FieldKind::Name => {
            commands.entity(binding.entity).try_insert(Name::new(text));
        }
        FieldKind::Text => write_field(commands, binding, move |target| {
            if let Some(v) = target.try_downcast_mut::<String>() {
                *v = text;
            }
        }),
        _ => {}
    }
}

/// Enter applies the text.
fn text_keys(
    mut key: On<FocusedInput<KeyboardInput>>,
    fields: Query<(&FieldBinding, &EditableText)>,
    mut focus: ResMut<InputFocus>,
    mut commands: Commands,
) {
    if key.input.state.is_pressed() && key.input.logical_key == Key::Enter {
        key.propagate(false);
        commit_text(key.focused_entity, &fields, &mut commands);
        // Leaving the field lets the refresh show the value as stored.
        focus.clear();
    }
}

/// Clicking elsewhere applies the text.
fn text_focus_lost(
    lost: On<FocusLost>,
    fields: Query<(&FieldBinding, &EditableText)>,
    mut commands: Commands,
) {
    commit_text(lost.entity, &fields, &mut commands);
}
