#![allow(dead_code, unused_imports)]
use std::borrow::BorrowMut;
use std::f32::consts::SQRT_2;

// TODO fix pause screen might not be in front of the whole scene

use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::sprite::MaterialMesh2dBundle;

mod unused_systems;
use crate::unused_systems::*;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum AppState {
    // MainMenu,
    InGame,
    PauseScreen,
}

fn main() {
    #[derive(SystemLabel)]
    enum Label {
        SetupCamera,
        SpawnPlayer,
    }

    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(Color::DARK_GRAY))
        .add_startup_system(set_up_camera)
        .add_startup_system(spawn_player)
        .add_state(AppState::InGame)
        // move player only when InGame
        .add_system_set(SystemSet::on_update(AppState::InGame).with_system(player_control))
        .add_system(pause_screen_trigger)
        .add_system_set(SystemSet::on_enter(AppState::PauseScreen).with_system(setup_pause_screen))
        .add_system_set(SystemSet::on_exit(AppState::PauseScreen).with_system(close_pause_screen))
        // .add_plugin(LogDiagnosticsPlugin::default())
        // .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_system(bevy::window::close_on_esc)
        // .add_system(change_player_name)
        // .add_system(debug_player)
        .run();
}

fn set_up_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

#[derive(Debug, Component)]
struct Player;

#[derive(Debug, Component)]
struct Name {
    pub value: String,
}

impl Name {
    fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
        }
    }

    fn set(&mut self, value: impl Into<String>) {
        self.value = value.into();
    }
}

impl<T> From<T> for Name
where
    T: Into<String>,
{
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

#[derive(Bundle)]
struct PlayerBundle {
    name: Name,
    model: MaterialMesh2dBundle<ColorMaterial>,
    _p: Player,
}

fn spawn_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(PlayerBundle {
        name: "Player".into(),
        _p: Player,
        model: MaterialMesh2dBundle {
            mesh: meshes.add(shape::Circle::new(50.).into()).into(),
            material: materials.add(ColorMaterial::from(Color::BEIGE)),
            transform: Transform::from_translation(Vec3::ZERO),
            ..default()
        },
    });

    info!("Spawning a player");
}

// fn debug_player(
//     query: Query<&Name>,
// ) {
//     let player_name = query.single();
//     info!("Player name: {:?}", player_name);
// }

// fn change_player_name(mut commands: Commands, query: Query<Entity, With<Name>>) {
//     commands
//         .entity(query.single())
//         .remove::<Name>()
//         .insert(Name::from("Alex"));
// }

// fn change_player_name(mut query: Query<&mut Name, With<Player>>) {
//     let mut name = query.single_mut();
//     name.set("Alex");
// }

fn player_control(
    time: Res<Time>,
    keys: Res<Input<KeyCode>>,
    mut query: Query<&mut Transform, With<Player>>,
) {
    let mut transfrom = query.single_mut();
    let mut translation = transfrom.translation.borrow_mut();

    let multiplier = 250.;
    let magnitude = multiplier * time.delta_seconds();
    let diagonal_magnitude = magnitude / SQRT_2;

    let up = keys.any_pressed([KeyCode::W, KeyCode::Up]);
    let left = keys.any_pressed([KeyCode::A, KeyCode::Left]);
    let down = keys.any_pressed([KeyCode::S, KeyCode::Down]);
    let right = keys.any_pressed([KeyCode::D, KeyCode::Right]);

    if up && left {
        translation.y += diagonal_magnitude;
        translation.x -= diagonal_magnitude;
    }
    if up && right {
        translation.y += diagonal_magnitude;
        translation.x += diagonal_magnitude;
    }
    if down && left {
        translation.y -= diagonal_magnitude;
        translation.x -= diagonal_magnitude;
    }
    if down && right {
        translation.y -= diagonal_magnitude;
        translation.x += diagonal_magnitude;
    }
    if up && !(left || right) {
        translation.y += magnitude;
    }
    if left && !(up || down) {
        translation.x -= magnitude;
    }
    if down && !(left || right) {
        translation.y -= magnitude;
    }
    if right && !(up || down) {
        translation.x += magnitude;
    }
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
                custom_size: Some(Vec2::new(100.0, 100.0)),
                ..default()
            },
            ..default()
        },
        _ps: PauseScreen,
    });
}

fn pause_screen_trigger(keys: Res<Input<KeyCode>>, mut app_state: ResMut<State<AppState>>) {
    if keys.just_pressed(KeyCode::M) {
        match app_state.current() {
            AppState::InGame => {
                app_state.push(AppState::PauseScreen).unwrap();
            }
            AppState::PauseScreen => {
                app_state.pop().unwrap();
            }
        }
    }
}

fn close_pause_screen(mut commands: Commands, query: Query<Entity, With<PauseScreen>>) {
    commands.entity(query.single()).despawn();
}
