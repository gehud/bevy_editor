/// Size constants
pub mod size {
    use bevy::ui::Val;

    pub const GAP: Val = Val::Px(6.0);

    pub const BORDER_RADIUS: f32 = 5.0;
    pub const BORDER_THICKNESS: Val = Val::Px(1.0);

    /// Common row size for buttons, sliders, spinners, etc.
    pub const ROW_HEIGHT: Val = Val::Px(24.0);

    /// Width and height of a checkbox
    pub const CHECKBOX_SIZE: Val = Val::Px(18.0);

    /// Width and height of a radio button
    pub const RADIO_SIZE: Val = Val::Px(18.0);

    /// Width of a toggle switch
    pub const TOGGLE_WIDTH: Val = Val::Px(32.0);

    /// Height of a toggle switch
    pub const TOGGLE_HEIGHT: Val = Val::Px(18.0);
}
