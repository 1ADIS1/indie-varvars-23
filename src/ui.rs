use crate::GameManager;

use super::AppState;
use bevy::prelude::*;

pub const MAIN_HUD_STYLE: Style = {
    let mut style = Style::DEFAULT;
    style.flex_direction = FlexDirection::Row;
    // style.justify_content = JustifyContent::Start;
    // style.align_items = AlignItems::Center;
    style.width = Val::Percent(100.);
    style.height = Val::Percent(100.);
    style
};

pub const BUTTON_STYLE: Style = {
    let mut style = Style::DEFAULT;
    style.justify_self = JustifySelf::Baseline;
    style.justify_content = JustifyContent::Center;
    style.align_items = AlignItems::Center;
    style.width = Val::Percent(50.);
    style.height = Val::Percent(50.);
    style
};

pub const SCORE_IMAGE_STYLE: Style = {
    let mut style = Style::DEFAULT;
    style.justify_content = JustifyContent::End;
    // style.align_items = AlignItems::End;
    style.align_self = AlignSelf::Start;
    style.width = Val::Percent(10.);
    style.height = Val::Percent(10.);
    style
};

pub const TEXT_STYLE: Style = {
    let mut style = Style::DEFAULT;
    // style.flex_direction = FlexDirection::Column;
    style.justify_content = JustifyContent::End;
    style.align_items = AlignItems::End;
    // style.justify_self = JustifySelf::Baseline;
    style.width = Val::Percent(100.);
    style.height = Val::Percent(100.);
    style
};

pub const NORMAL_BUTTON_COLOR: Color = Color::rgb(1., 1., 1.);
pub const HOVERED_BUTTON_COLOR: Color = Color::rgb(0.75, 0.75, 0.75);
pub const PRESSED_BUTTON_COLOR: Color = Color::rgb(0.5, 0.5, 0.5);

#[derive(Component)]
pub struct ReplayButton;

#[derive(Component)]
pub struct ScoreText;

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, build_hud)
            .add_systems(Update, update_score_text)
            .add_systems(
                Update,
                interact_with_replay_button.run_if(in_state(AppState::GameOver)),
            )
            .add_systems(OnEnter(AppState::GameOver), show_replay_button)
            .add_systems(OnExit(AppState::GameOver), hide_replay_button);
    }
}

fn build_hud(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn(NodeBundle {
            style: MAIN_HUD_STYLE,
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

            // === Score text ===
            parent.spawn((
                TextBundle {
                    style: TEXT_STYLE,
                    text: Text {
                        sections: vec![TextSection::new(
                            0.to_string(),
                            TextStyle {
                                font: asset_server.load("fonts/Comic Sans MS.ttf"),
                                font_size: 48.0,
                                color: Color::WHITE,
                            },
                        )],
                        alignment: TextAlignment::Center,
                        ..default()
                    },
                    ..default()
                },
                ScoreText {},
            ));

            // === Score image ===
            parent.spawn(ImageBundle {
                style: SCORE_IMAGE_STYLE,
                image: asset_server.load("art/Score.png").into(),
                ..default()
            });
        });
}

// Updates score text, if the player completed the planet.
pub fn update_score_text(
    mut score_text_query: Query<&mut Text, With<ScoreText>>,
    game_manager: Res<GameManager>,
) {
    if game_manager.is_changed() {
        if let Ok(mut score_text) = score_text_query.get_single_mut() {
            score_text.sections[0].value = format!("{}", game_manager.score.to_string());
        }
    }
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
