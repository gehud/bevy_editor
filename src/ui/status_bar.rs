//! Footer: VCS branch, project build, error / warning counters.

use bevy::{feathers::constants::icons, prelude::*};

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
            @label("bevy-editor-2025-03 (e4f5g6h)", ink::A4A4)
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
