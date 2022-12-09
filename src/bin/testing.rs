//! Shows how to render simple primitive shapes with a single color.

use bevy::prelude::*;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum AppState {
    // MainMenu,
    InGame,
    Paused,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_state(AppState::InGame)
        .add_startup_system(set_up_camera)
        .add_system_set(SystemSet::on_enter(AppState::Paused).with_system(setup_pause_screen))
        .add_system_set(SystemSet::on_exit(AppState::Paused).with_system(close_pause_screen))
        .add_system(pause_screen_watch)
        .run();
}

fn set_up_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

#[derive(Debug, Component)]
struct PauseScreen;

#[derive(Bundle)]
struct PauseScreenBundle {
    sprite: SpriteBundle,
    _ps: PauseScreen,
}

fn setup_pause_screen(mut commands: Commands) {
    commands.spawn(PauseScreenBundle {
        sprite: SpriteBundle {
            sprite: Sprite {
                color: Color::SEA_GREEN,
                custom_size: Some(Vec2::new(1000.0, 1000.0)),
                ..default()
            },
            // visibility: Visibility { is_visible: false },
            ..default()
        },
        _ps: PauseScreen,
    });
}

fn pause_screen_watch(
    keys: Res<Input<KeyCode>>,
    // mut query: Query<&mut Visibility, With<PauseScreen>>,
    mut app_state: ResMut<State<AppState>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        match app_state.current() {
            AppState::InGame => {
                app_state.set(AppState::Paused).unwrap();
            }
            AppState::Paused => {
                app_state.set(AppState::InGame).unwrap();
            }
        }
    }
}

fn close_pause_screen(mut commands: Commands, query: Query<Entity, With<PauseScreen>>) {
    let e = query.single();
    commands.entity(e).despawn();
}
