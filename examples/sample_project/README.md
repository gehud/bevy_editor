# Sample project

A small Bevy game for testing the editor against a real project: its own reflected components
(`Spinner`, `Health`, `Unit` with a nested `Stats` struct and a `Team` enum), a hierarchy, and a
system that animates it.

- `cargo run -p sample_project` runs the game.
- `cargo run --example open_sample_project` opens it in the editor: the editor loads this
  folder as its project and spawns the same scene (`src/scene.rs` is shared with the example).
- `cargo run -- examples/sample_project` opens the folder with the plain editor binary (the
  scene and the sample's compiled-in assets are not loaded).

In the editor example, open the **Assets** pane to browse:

| Source | Files |
| --- | --- |
| **Project Assets** | `models/low-poly-dragon.gltf`, `textures/stone-checker.png` |
| **Embedded Assets (read-only)** → `sample_project` | `textures/team-banner.png`, `audio/notification.ogg` |

The dragon and both textures are used in the scene. The notification sound is included as an
audio sample and does not play automatically.

Keep `src/scene.rs` to `bevy` APIs only, since the editor example compiles it too.
