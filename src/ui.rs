use super::AppState;
use bevy::prelude::*;

pub const MAIN_MENU_STYLE: Style = {
    let mut style = Style::DEFAULT;
    // style.flex_direction = FlexDirection::Column;
    style.justify_content = JustifyContent::Center;
    style.align_items = AlignItems::Center;
    style.width = Val::Percent(100.);
    style.height = Val::Percent(100.);
    style
    // gap: Size::new(Val::Px(8.0), Val::Px(8.0)),
};

pub const BUTTON_STYLE: Style = {
    let mut style = Style::DEFAULT;
    // style.justify_content = JustifyContent::Center;
    // style.align_items = AlignItems::Center;
    style.width = Val::Percent(50.);
    style.height = Val::Percent(50.);
    style
};

pub const NORMAL_BUTTON_COLOR: Color = Color::rgb(1., 1., 1.);
pub const HOVERED_BUTTON_COLOR: Color = Color::rgb(0.75, 0.75, 0.75);
pub const PRESSED_BUTTON_COLOR: Color = Color::rgb(0.5, 0.5, 0.5);

#[derive(Component)]
pub struct ReplayButton;

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (build_replay_button,))
            .add_systems(
                Update,
                interact_with_replay_button.run_if(in_state(AppState::GameOver)),
            )
            .add_systems(OnEnter(AppState::GameOver), show_replay_button)
            .add_systems(OnExit(AppState::GameOver), hide_replay_button);
    }
}

pub fn build_replay_button(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn(NodeBundle {
            style: MAIN_MENU_STYLE,
            ..default()
        })
        .with_children(|parent| {
            // === Replay Button ===
            parent.spawn((
                ButtonBundle {
                    style: BUTTON_STYLE,
                    image: asset_server.load("art/Replay.png").into(),
                    background_color: NORMAL_BUTTON_COLOR.into(),
                    visibility: Visibility::Hidden,
                    ..default()
                },
                ReplayButton {},
            ));
        });
}

fn show_replay_button(mut replay_button_query: Query<&mut Visibility, With<ReplayButton>>) {
    if let Ok(mut replay_button_visibility) = replay_button_query.get_single_mut() {
        *replay_button_visibility = Visibility::Visible;
    }
}

fn hide_replay_button(mut replay_button_query: Query<&mut Visibility, With<ReplayButton>>) {
    if let Ok(mut replay_button_visibility) = replay_button_query.get_single_mut() {
        *replay_button_visibility = Visibility::Hidden;
    }
}

fn interact_with_replay_button(
    mut button_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<ReplayButton>),
    >,
    mut app_state_next_state: ResMut<NextState<AppState>>,
) {
    if let Ok((interaction, mut background_color)) = button_query.get_single_mut() {
        match *interaction {
            Interaction::Pressed => {
                *background_color = PRESSED_BUTTON_COLOR.into();
                app_state_next_state.set(AppState::Playing);
            }
            Interaction::Hovered => {
                *background_color = HOVERED_BUTTON_COLOR.into();
            }
            Interaction::None => {
                *background_color = NORMAL_BUTTON_COLOR.into();
            }
        }
    }
}
