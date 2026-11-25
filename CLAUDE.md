# Bevy Editor

An editor for the Bevy game engine: scene editing, asset inspection, and (planned) custom editor
plugins. The UI is built with `bevy_feathers` and follows a Figma design.

## Repository layout

- `bevy/` is a git submodule holding **our Bevy fork** (`github.com/gehud/bevy`, branch
  `editor/0.20`, based on tag `v0.20.1`). Engine and widget changes go there.
- `src/` is the editor binary (crate `bevy_editor`).
- `crates/bevy_ui_svg/` renders SVG icons in `bevy_ui` at each node's exact pixel size (Bevy has
  no SVG support, and down-scaled bitmaps look jagged).
- `assets/` holds editor-only assets loaded from disk: `icons/*.svg`, `shaders/infinite_grid.wesl`.

## Build, run, test

```sh
git submodule update --init      # first checkout
cargo run                        # dev build, dynamic linking via the default `dev` feature
cargo test                       # dock layout unit tests
cargo build --release --no-default-features   # release without dynamic linking
cargo fmt && cargo clippy --all-targets
```

- `BEVY_EDITOR_SCREENSHOT=<path.png> cargo run` saves a screenshot after 120 frames and exits
  (`src/dev.rs`). Use it to check UI changes visually against the design.
- A clean build compiles all of Bevy (~7 min). `.cargo/config.toml` uses `rust-lld` on Windows.

## Cargo setup (important)

- The root is a workspace with `members = ["crates/*"]` that **excludes `bevy/`** (it has its
  own workspace). Do not add `"."` to `members`: that triggers a "multiple workspace roots" error.
- `bevy` comes from the submodule path, and `[patch.crates-io]` points `bevy` at it. When adding
  a third-party crate that depends on Bevy sub-crates (`bevy_ecs`, `bevy_app`, …), add those to
  `[patch.crates-io]` too, then check `cargo tree -d` shows no duplicate `bevy_*` crates.
- Enabled Bevy features: `bevy_feathers`, `system_font_discovery` (needed for the Windows
  caption-button icon font).

## Working on the fork

- Commit fork changes on `editor/0.20` inside `bevy/` and **push the fork first**, then
  `git add bevy` in the root repo. Otherwise the submodule points at a commit nobody can fetch.
- Keep fork changes small and upstreamable. Current fork changes (all in `bevy_feathers`):
  Fira Sans Medium/SemiBold fonts, `text_input_placeholder` for text inputs, and 33 design icons
  as PNG (`constants::icons::*`). The editor no longer uses those PNGs; it uses SVGs instead.
- Generic widget/engine features belong in the fork; editor-specific UI belongs in `src/`.

## Design source

- Figma file "Bevy Editor (Copy)": https://www.figma.com/design/85RYQVygSU1Wy1Gau9W3r2/Bevy-Editor--Copy-?node-id=9744-8817
  (frame `9744:8817`). The full frame is too large for one design-context request; fetch
  sections by node id (see `get_metadata` on `9744:8818`).
- The feathers palette matches the design exactly; use `feathers::palette` / `style::colors`
  rather than new hex values where a match exists.
- Icons: download SVGs from the design into `assets/icons/`, add a constant to `ui/icons.rs`, and
  show them with `style::icon(path, w, h, color)`. That uses `SvgIcon { mask: true }`, which
  renders a white silhouette at the node's exact size and tints it with the design's color.
  Multi-color art uses `mask: false`; `trim: true` crops away empty canvas margins (used for the
  Bevy logo, taken from `bevy/assets/branding/bevy_bird_dark.svg`). Name icons by appearance,
  not meaning (feathers convention).

## Architecture (`src/`)

- `main.rs`: borderless window (`decorations: false`), adds `EditorUiPlugin`, `EditingPlugin`,
  `EditorCameraPlugin`, `InfiniteGridPlugin`, `DevPlugin`.
- `editing.rs`: `EditorOnly` marker (editor entities hidden from scene views; UI nodes count as
  editor-only automatically) and the `Selection` resource (multi-selection, in selection order;
  the last entity is the primary one). **Mark every editor-owned entity
  that has a `Transform` with `EditorOnly`.**
- `grid.rs`: infinite ground grid (custom `Material`; the vertex shader recenters a unit quad
  under whichever camera renders it, so it works in every viewport; `NoFrustumCulling`).
- `editor_camera.rs`: one `EditorCamera` per viewport panel, created when a `ViewportSlot` node
  appears and rendered into it through `ViewportNode`. Fly-cam: hold RMB (WASD/QE, Shift, scroll
  changes speed), MMB to pan, scroll to dolly. `look_along` snaps a camera to an axis view,
  orbiting around the ground point it was looking at (unit-tested). Cameras pause while their viewport is hidden;
  that check runs after UI layout.
- `ui/mod.rs`: root layout (title bar, dock, status bar, resize handles), theme tokens, plugins
  (`FeathersPlugins`, `bevy_ui_svg::SvgPlugin`, `DockPlugin`).
- `ui/dock/`: the docking layout.
  - `layout.rs`: pure data model (splits plus tab-stack leaves), with unit tests.
  - `view.rs`: rebuilds only the chrome (splits, splitters, tab bars) when `DockLayout` changes,
    spawned immediately with `apply_scene` so the dock never draws empty for a frame.
    **Panel contents are spawned once** (`PanelContent`) and re-parented into the active leaf's
    `LeafContentSlot`, or into a hidden stash, so their state survives layout changes.
  - `interact.rs`: tab clicks, splitter drags (edit sizes without change detection, update
    `flex_grow` directly), and tab/grip drag-and-drop with a drop preview.
  - Only horizontal tab bars exist, and every leaf shows one (no inlined panel headers).
- `ui/panels.rs`: `PanelKind` registry (title, content scene) and the default layout, which
  reproduces the Figma arrangement (with a Console placeholder tab next to Assets).
- Panel contents: `hierarchy.rs` (Scene Tree: live world hierarchy; rebuilt only when a row
  snapshot changes, and never while a rename field is open or a row is dragged; name filter
  (matches plus dimmed ancestors, fully expanded); Ctrl/Shift
  multi-select, expand/collapse, visibility toggle, Add Entity, Delete, F2/double-click rename
  of the whole selection, drag-and-drop reorder/reparent keeping world transforms; root order
  lives in `SceneTreeState::root_order`),
  `project_files.rs` (Project Files), `viewport.rs` (toolbar, view, demo
  scene), `axis_gizmo.rs` (orientation gizmo per viewport, UI nodes positioned from the camera
  rotation each frame; clicking an axis calls `look_along`), `asset_browser.rs`, `inspector.rs` (Components: the primary selection's components
  through reflection; cards rebuilt only when the entity/component set/collapse/filter change,
  values refreshed into the widgets every frame via `FieldBinding` (component `TypeId` +
  reflect path), edits written back with `ReflectComponent::reflect_mut`), plus `title_bar.rs`,
  `status_bar.rs`, `window_frame.rs` (resize zones, maximized state from winit).
- `ui/style.rs`: colors, text/icon helpers, `HoverBackground`. `ui/widgets.rs`: search fields,
  wide buttons, dividers. `ui/icons.rs`: icon paths.

## Current state

- The Scene Tree and the Components panel show the real world. Project Files and the asset
  browser still show the design's **sample data**; next step: bind them to disk. Inspector
  gaps: enums, lists, maps and handles are read-only; "Add Component" and the card "…" menu do
  nothing; no undo.
- Not done: layout persistence, the "+" button in tab bars, closing tabs, orthographic axis
  views, and window corner rounding.
- Known issue: when a viewport changes size, its render looks stretched for one frame (Bevy
  updates the camera's target size a frame after `ViewportNode` resizes the image).

## Bevy 0.20 / BSN gotchas

- UI is authored with `bsn!` / `bsn_list!` scenes. `{expr}` embeds values, `@func(..)` includes a
  scene function, `@{scene}` includes a scene value, `Children [ {vec} ]` adds a list.
  `--` separates entities.
- Enum or tuple values like `DragsPanel(Some(x))` can misparse inside `bsn!`. Build the
  component in a `let` and list the variable as an entry.
- Components with asset handles need `#[derive(FromTemplate)]` (not `Default`) so `bsn!` can take
  a path string for the handle (see `SvgIcon`).
- `Button` from the prelude is deprecated: use `bevy::ui_widgets::Button`. Give buttons a `Node`,
  or they are not laid out.
- Picking events are concrete types (`PointerClick`, `PointerPress`, `PointerDrag…`), not
  `Pointer<Click>`. `event.entity` is the current target while an event bubbles.
- `ui_widgets::Button` stops `PointerClick`, `PointerPress`, `PointerRelease` and
  `PointerDragEnd` from bubbling, but **not** `PointerDragStart`/`PointerDrag`. A drag handler
  on a button's parent must check `original_event_target()`, or it starts drags that never end.
- `ViewportNode::camera` is `Option<Entity>`. `Window` has no maximized getter: read
  `WINIT_WINDOWS` on the main thread (`NonSendMarker`).
- `commands.entity(e).apply_scene(..)` spawns immediately. `queue_*` variants may land a frame
  later, which causes visible flicker for UI rebuilds.
- The feathers text-input container has `flex_grow: 1`. Override it in column layouts.
- A root node with `height: percent(100)` still grows to fit its content unless it has
  `min_height: px(0)` (Taffy's automatic minimum size). The editor root sets it.
- `overflow: scroll` alone doesn't react to the mouse wheel: add `ui_widgets::ScrollArea`.
- `NumberInputValue` is immutable: insert a new one to change the shown value, and only when it
  differs (the widget rewrites its text on every insert).
- UI images are drawn with bilinear filtering and no mipmaps: never show a bitmap much smaller
  than its pixel size (use SVG via `SvgIcon` instead).

## Workflow

- Commit directly to `dev` and push to `origin dev` (no feature branches). Commit only when asked.
- Verify UI changes with the screenshot hook. For interaction paths that can't be clicked from a
  script, a temporary system that mutates `DockLayout` on a fixed frame works; remove it afterwards.
