// TODO fix pause screen might not be in front of the whole scene

#![allow(dead_code, unused_imports)]
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::sprite::MaterialMesh2dBundle;
use bevy::utils::HashMap;
use std::borrow::BorrowMut;
use std::f32::consts::SQRT_2;

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
        .add_startup_system(spawn_npcs)
        .insert_resource(ProximityToNPCsResource::default())
        .add_system(next_to_obj_watcher)
        .add_event::<NextToObjEvent>()
        .add_event::<AwayFromObjEvent>()
        .add_system(next_to_npc_event_handler)
        .add_system(away_from_npc_event_handler)
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

#[derive(Component)]
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

#[derive(Component)]
struct NPC;

#[derive(Component)]
struct InProximity {
    edge_distance: f32,
}

// #[derive(Component)]
// struct TalkTo;

// #[derive(Component)]
// struct Interact;

#[derive(Bundle)]
struct NPCBundle {
    name: Name,
    model: SpriteBundle,
    _n: NPC,
    in_proximity: InProximity,
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

fn spawn_npcs(mut commands: Commands) {
    commands.spawn(NPCBundle {
        name: "Joe".into(),
        in_proximity: InProximity {
            edge_distance: 150.,
        },
        _n: NPC,
        model: SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.25, 0.25, 0.75),
                custom_size: Some(Vec2::new(100., 100.)),
                ..default()
            },
            transform: Transform::from_xyz(200., 0., 0.),
            ..default()
        },
    });

    commands.spawn(NPCBundle {
        name: "Rue".into(),
        in_proximity: InProximity {
            edge_distance: 150.,
        },
        _n: NPC,
        model: SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.25, 0.25, 0.75),
                custom_size: Some(Vec2::new(100., 100.)),
                ..default()
            },
            transform: Transform::from_xyz(-200., 100., 0.),
            ..default()
        },
    });

    commands.spawn(NPCBundle {
        name: "Moe".into(),
        in_proximity: InProximity {
            edge_distance: 150.,
        },
        _n: NPC,
        model: SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.25, 0.25, 0.75),
                custom_size: Some(Vec2::new(100., 100.)),
                ..default()
            },
            transform: Transform::from_xyz(-350., 100., 0.),
            ..default()
        },
    });

    info!("Spawning NPC");
}

#[derive(Resource)]
struct ProximityToNPCsResource {
    values: HashMap<Entity, bool>,
}

impl Default for ProximityToNPCsResource {
    fn default() -> Self {
        Self {
            values: HashMap::new(),
        }
    }
}

struct NextToObjEvent {
    entity: Entity,
}
struct AwayFromObjEvent {
    entity: Entity,
}

fn next_to_obj_watcher(
    rel_obj_transforms: Query<(Entity, &Transform, &InProximity)>,
    player_transform: Query<&Transform, With<Player>>,
    mut next_to_obj: ResMut<ProximityToNPCsResource>,
    mut ev_next_to_obj: EventWriter<NextToObjEvent>,
    mut ev_away_from_obj: EventWriter<AwayFromObjEvent>,
) {
    let player_transform = player_transform.single();

    fn is_in_proximity(
        player_transform: &Transform,
        rel_obj_transform: &Transform,
        in_proximity: &InProximity,
    ) -> bool {
        let next_to_when_distance_is_less_than = in_proximity.edge_distance;

        rel_obj_transform
            .translation
            .distance(player_transform.translation)
            < next_to_when_distance_is_less_than
    }

    for (entity, obj_transform, in_proximity) in &rel_obj_transforms {
        let next_to = next_to_obj.values.get(&entity);

        if is_in_proximity(&player_transform, obj_transform, in_proximity) {
            match next_to {
                None | Some(false) => {
                    next_to_obj.values.insert(entity, true);
                    ev_next_to_obj.send(NextToObjEvent { entity });
                }
                Some(true) => (),
            }
        } else {
            match next_to {
                Some(true) => {
                    next_to_obj.values.insert(entity, false);
                    ev_away_from_obj.send(AwayFromObjEvent { entity });
                }
                Some(false) => (),
                // if you never've been close don't send AwayFrom event
                // not sure if the same should apply to NextTo event
                None => (),
            }
        }
    }
}

fn next_to_npc_event_handler(
    mut ev_next_to_obj: EventReader<NextToObjEvent>,
    query: Query<(Entity, &Name), With<NPC>>,
) {
    for ev in ev_next_to_obj.iter() {
        for (entity, name) in &query {
            if entity == ev.entity {
                info!("Next to NPC {}", name.value);
            }
        }
    }
}

fn away_from_npc_event_handler(
    mut ev_away_from_obj: EventReader<AwayFromObjEvent>,
    query: Query<(Entity, &Name), With<NPC>>,
) {
    for ev in ev_away_from_obj.iter() {
        for (entity, name) in &query {
            if entity == ev.entity {
                info!("Away from NPC {}", name.value);
            }
        }
    }
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
