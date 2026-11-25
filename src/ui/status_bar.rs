//! Footer: VCS branch, project name, error / warning counters.
//!
//! The project name is real ([`ProjectRoot`]); the branch and counters are the design's sample.

use bevy::prelude::*;

use crate::{ProjectRoot, ui::icons};

use super::style::{icon, icon_sq, ink, label};

pub fn status_bar() -> impl Scene {
    bsn! {
        Node {
            height: px(24),
            width: percent(100),
            flex_shrink: 0.0,
            padding: UiRect::horizontal(px(8)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
        }
        Children [
            Node { column_gap: px(5), align_items: AlignItems::Center }
            Children [
                @icon_sq(icons::GIT_PULL_REQUEST_ARROW, 12.0, ink::E6.with_alpha(0.8))
                --
                @label("feature-branch (a1b2c3d)", ink::E6.with_alpha(0.8))
            ]
            --
            @label("No project", ink::A4A4)
            ProjectLabel
            --
            Node { column_gap: px(8), align_items: AlignItems::Center }
            Children [
                Node { column_gap: px(4.643), align_items: AlignItems::Center }
                Children [
                    @icon_sq(icons::CIRCLE_X, 11.122, ink::E6.with_alpha(0.8))
                    --
                    @label("0", ink::E6.with_alpha(0.8))
                ]
                --
                Node { column_gap: px(4.643), align_items: AlignItems::Center }
                Children [
                    @icon(icons::TRIANGLE_ALERT, 11.13, 10.118, ink::E6.with_alpha(0.8))
                    --
                    @label("1", ink::E6.with_alpha(0.8))
                ]
            ]
        ]
    }
}

/// Marks the status bar's project name.
#[derive(Component, Clone, Default)]
pub(super) struct ProjectLabel;

pub(super) fn update_project_label(
    project: Option<Res<ProjectRoot>>,
    mut labels: Query<(Ref<ProjectLabel>, &mut Text)>,
) {
    // `name` touches the file system: only look it up when something changed.
    let project_changed = project.as_ref().is_some_and(|p| p.is_changed());
    for (marker, mut label) in &mut labels {
        if marker.is_added() || project_changed {
            label.0 = project
                .as_ref()
                .map_or_else(|| "No project".to_owned(), |p| p.name());
        }
    }
}
