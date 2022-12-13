// NOTE current implementation of NPC proximity does not take into
//     account lengths when several objects are considered to be in proximity
//     now, the "closest object" is the latest object detected to be in proximity
// OPINION it's not worth playing with entity visibility or despawning
//     when you can just hide them under main menu canvas, will see
// NOTE you cannot trigger state "enter" using pop(), but can using set(state)
// TODO add Unload component to PauseScreen and DialogWindow

#![allow(dead_code, unused_imports)]
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::ecs::schedule::ShouldRun;
use bevy::prelude::*;
use bevy::sprite::MaterialMesh2dBundle;
use bevy::utils::{HashMap, HashSet};
use std::borrow::BorrowMut;
use std::f32::consts::{PI, SQRT_2};

mod unused_systems;
use crate::unused_systems::*;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum AppState {
    MainMenu,
    InGame,
    PauseScreen,
    DialogWindow,
}

fn main() {
    #[derive(SystemLabel)]
    enum Label {
        SetupCamera,
        SpawnPlayer,
        NextToNPCEventHandler,
        AwayFromNPCEventHandler,
        NextToObjectWatcher,
        SpawnNPCs,
    }

    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(Color::DARK_GRAY))
        .add_startup_system(set_up_camera)
        .insert_resource(ProximityToObjResource::default())
        .insert_resource(NearestNPCinProximity { value: vec![] })
        .add_event::<NextToObjEvent>()
        .add_event::<AwayFromObjEvent>()
        .add_system(away_from_npc_event_handler.label(Label::AwayFromNPCEventHandler))
        .add_system(next_to_npc_event_handler.after(Label::AwayFromNPCEventHandler))
        // .add_state(AppState::MainMenu)
        .add_state(AppState::InGame)
        .add_system_set(
            SystemSet::on_enter(AppState::InGame)
                .with_system(spawn_player)
                .label(Label::SpawnPlayer),
        )
        // NOT see SystemSet with spawn_player (above)
        .add_system_set(
            SystemSet::on_enter(AppState::InGame)
                .with_system(spawn_npcs)
                .label(Label::SpawnNPCs),
        )
        .add_system_set(SystemSet::on_update(AppState::InGame).with_system(next_to_obj_watcher))
        // move player only when InGame
        .add_system_set(SystemSet::on_update(AppState::InGame).with_system(player_movement))
        .add_system(pause_screen_trigger)
        .add_system_set(SystemSet::on_enter(AppState::PauseScreen).with_system(setup_pause_screen))
        .add_system_set(SystemSet::on_exit(AppState::PauseScreen).with_system(close_pause_screen))
        .add_system(dialog_window_trigger)
        .add_system_set(
            SystemSet::on_enter(AppState::DialogWindow).with_system(setup_dialog_window),
        )
        .add_system_set(SystemSet::on_exit(AppState::DialogWindow).with_system(close_dialog_window))
        .add_system(button_main_menu_trigger)
        .add_system_set(
            SystemSet::on_exit(AppState::InGame).with_system(despawn_all_recursive::<LevelUnload>),
        )
        .add_system_set(SystemSet::on_enter(AppState::MainMenu).with_system(setup_main_menu))
        .add_system_set(
            SystemSet::on_exit(AppState::MainMenu)
                .with_system(despawn_all_recursive::<MainMenuUnload>),
        )
        // .add_plugin(LogDiagnosticsPlugin::default())
        // .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_system(bevy::window::close_on_esc)
        .run();
}

fn set_up_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

#[derive(Component)]
pub struct LevelUnload;

#[derive(Component)]
pub struct MainMenuUnload;

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
    // model: SpriteBundle,
    _identity: Player,
    _unload: LevelUnload,
}

#[derive(Component)]
struct NPC;

#[derive(Component)]
struct InProximity {
    edge_distance: f32,
}

#[derive(Bundle)]
struct NPCBundle {
    name: Name,
    in_proximity: InProximity,
    model: SpriteBundle,

    _identity: NPC,
    _unload: LevelUnload,
}

fn spawn_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    // asset_server: Res<AssetServer>,
) {
    commands.spawn(PlayerBundle {
        name: "Player".into(),
        model: MaterialMesh2dBundle {
            mesh: meshes.add(shape::Circle::new(50.).into()).into(),
            material: materials.add(ColorMaterial::from(Color::BEIGE)),
            transform: Transform::from_translation(Vec3::ZERO),
            ..default()
        },
        _unload: LevelUnload,
        _identity: Player,
        // model: SpriteBundle {
        //     texture: asset_server.load("character.png"),
        //     transform: Transform {
        //         scale: Vec3::splat(0.12),
        //         ..default()
        //     },
        //     ..default()
        // },
    });
    info!("Spawning a player");
}

fn not_spawned<T: Component>(components: Query<With<T>>) -> ShouldRun {
    components.is_empty().into()
}

fn spawned<T: Component>(components: Query<With<T>>) -> ShouldRun {
    (!components.is_empty()).into()
}

fn despawn_all_recursive<T: Component>(mut commands: Commands, q: Query<Entity, With<T>>) {
    debug!("despawning entities");
    for (i, e) in q.iter().enumerate() {
        debug!("\t{}", i);
        commands.entity(e).despawn();
    }
}

fn hide_all<T: Component>(mut components: Query<&mut Visibility, With<T>>) {
    for mut visibility in components.iter_mut() {
        visibility.is_visible = false;
    }
}

fn show_all<T: Component>(mut components: Query<&mut Visibility, With<T>>) {
    for mut visibility in components.iter_mut() {
        visibility.is_visible = true;
    }
}

fn spawn_npcs(mut commands: Commands) {
    commands.spawn(NPCBundle {
        name: "Joe".into(),
        in_proximity: InProximity {
            edge_distance: 150.,
        },
        model: SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.25, 0.25, 0.75),
                custom_size: Some(Vec2::new(100., 100.)),
                ..default()
            },
            transform: Transform::from_xyz(200., 0., 0.),
            ..default()
        },
        _identity: NPC,
        _unload: LevelUnload,
    });

    commands.spawn(NPCBundle {
        name: "Rue".into(),
        in_proximity: InProximity {
            edge_distance: 150.,
        },
        model: SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.25, 0.25, 0.75),
                custom_size: Some(Vec2::new(100., 100.)),
                ..default()
            },
            transform: Transform::from_xyz(-200., 100., 0.),
            ..default()
        },
        _identity: NPC,
        _unload: LevelUnload,
    });

    commands.spawn(NPCBundle {
        name: "Moe".into(),
        in_proximity: InProximity {
            edge_distance: 150.,
        },
        model: SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.25, 0.25, 0.75),
                custom_size: Some(Vec2::new(100., 100.)),
                ..default()
            },
            transform: Transform::from_xyz(-350., 100., 0.),
            ..default()
        },
        _identity: NPC,
        _unload: LevelUnload,
    });

    info!("Spawning NPC");
}

// TODO "maybe" remove key-value pair when entity despawns
#[derive(Resource)]
struct ProximityToObjResource {
    values: HashMap<Entity, (bool, f32)>,
}

// TODO clean this when entities despawn
// RELATED BUG:
//         go to NPC
//         stay close to it
//         trigger PauseScreen (press M), then MainMenu (press Tab)
//         close MainMenu (press Tab)
//         trigger DialogWindow check (press E)
//         get panic
#[derive(Resource)]
struct NearestNPCinProximity {
    value: Vec<Entity>,
}

impl NearestNPCinProximity {
    // nearest is considered to be the latest one, joining a stack
    fn get(&self) -> Option<&Entity> {
        self.value.last()
    }

    // check if there's "any" npc in proximity
    fn any(&self) -> bool {
        !self.value.is_empty()
    }
}

impl Default for ProximityToObjResource {
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
    mut next_to_obj: ResMut<ProximityToObjResource>,
    mut ev_next_to_obj: EventWriter<NextToObjEvent>,
    mut ev_away_from_obj: EventWriter<AwayFromObjEvent>,
) {
    let player_transform = player_transform.single();

    for (entity, obj_transform, in_proximity) in &rel_obj_transforms {
        let next_to = next_to_obj.values.get(&entity);

        let distance_to_object = obj_transform
            .translation
            .distance(player_transform.translation);

        if distance_to_object < in_proximity.edge_distance {
            match next_to {
                None | Some((false, _)) => {
                    next_to_obj
                        .values
                        .insert(entity, (true, distance_to_object));
                    ev_next_to_obj.send(NextToObjEvent { entity });
                }
                Some((true, _)) => (),
            }
        } else {
            match next_to {
                Some((true, _)) => {
                    next_to_obj
                        .values
                        .insert(entity, (false, distance_to_object));
                    ev_away_from_obj.send(AwayFromObjEvent { entity });
                }
                Some((false, _)) => (),
                // if you never've been close don't send AwayFrom event
                // not sure if the same should apply to NextTo event
                None => (),
            }
        }
    }
}

fn next_to_npc_event_handler(
    mut ev_next_to_obj: EventReader<NextToObjEvent>,
    npcs: Query<(Entity, &Name), With<NPC>>,
    mut nearest_npc_in_proximity: ResMut<NearestNPCinProximity>,
) {
    for ev in ev_next_to_obj.iter() {
        let entity = ev.entity;
        let name = npcs.get_component::<Name>(entity).unwrap();

        nearest_npc_in_proximity.value.push(entity);

        info!("Next to NPC {}", name.value);
    }
}

fn away_from_npc_event_handler(
    mut ev_away_from_obj: EventReader<AwayFromObjEvent>,
    npcs: Query<(Entity, &Name), With<NPC>>,
    mut nearest_npc_in_proximity: ResMut<NearestNPCinProximity>,
) {
    for ev in ev_away_from_obj.iter() {
        let entity = ev.entity;
        let name = npcs.get_component::<Name>(entity).unwrap();

        let idx = nearest_npc_in_proximity
            .value
            .iter()
            .position(|&e| e == entity)
            .unwrap();
        nearest_npc_in_proximity.value.remove(idx);

        info!("Away from NPC {}", name.value);
    }
}

fn player_movement(
    time: Res<Time>,
    keys: Res<Input<KeyCode>>,
    mut query: Query<&mut Transform, With<Player>>,
) {
    let mut transform = query.single_mut();

    let multiplier = 250.;
    let magnitude = multiplier * time.delta_seconds();
    let diagonal_magnitude = magnitude / SQRT_2;

    let up = keys.any_pressed([KeyCode::W, KeyCode::Up]);
    let left = keys.any_pressed([KeyCode::A, KeyCode::Left]);
    let down = keys.any_pressed([KeyCode::S, KeyCode::Down]);
    let right = keys.any_pressed([KeyCode::D, KeyCode::Right]);

    // if left {
    //     transform.rotation = Quat::from_rotation_y(PI);
    // } else if right {
    //     transform.rotation = Quat::default();
    // }

    let mut translation = transform.translation.borrow_mut();

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

#[derive(Component)]
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
            transform: Stacking::PauseScreen.into(),
            ..default()
        },
        _ps: PauseScreen,
    });
}

fn pause_screen_trigger(keys: Res<Input<KeyCode>>, mut app_state: ResMut<State<AppState>>) {
    if keys.just_pressed(KeyCode::M) {
        match app_state.current() {
            AppState::InGame | AppState::DialogWindow => {
                app_state.push(AppState::PauseScreen).unwrap();
            }
            AppState::PauseScreen => {
                app_state.pop().unwrap();
            }
            AppState::MainMenu => (),
        }
    }
}

fn close_pause_screen(mut commands: Commands, query: Query<Entity, With<PauseScreen>>) {
    commands.entity(query.single()).despawn();
}

#[derive(Component)]
struct DialogWindow;

#[derive(Bundle)]
struct DialogWindowBundle {
    sprite: SpriteBundle,
    _dw: DialogWindow,
}

#[derive(Component)]
struct NameDialogText;

fn setup_dialog_window(
    mut commands: Commands,
    npcs: Query<(Entity, &Name), With<NPC>>,
    nearest_npc_in_proximity: Res<NearestNPCinProximity>,
    asset_server: Res<AssetServer>,
) {
    let entity = nearest_npc_in_proximity.get().unwrap().clone();
    let name = npcs.get_component::<Name>(entity).unwrap();

    commands.spawn(DialogWindowBundle {
        sprite: SpriteBundle {
            sprite: Sprite {
                color: Color::OLIVE,
                custom_size: Some(Vec2::new(800.0, 200.0)),
                ..default()
            },
            transform: Stacking::DialogWindow.from_xy(0., -200.),
            ..default()
        },
        _dw: DialogWindow,
    });

    let text = format!("I'm {}", name.value);

    commands.spawn((
        // Create a TextBundle that has a Text with a single section.
        TextBundle::from_section(
            // Accepts a `String` or any type that converts into a `String`, such as `&str`
            text.as_str(),
            TextStyle {
                font: asset_server.load("fonts/OpenSans.ttf"),
                font_size: 100.0,
                color: Color::WHITE,
            },
        ) // Set the alignment of the Text
        .with_text_alignment(TextAlignment::TOP_CENTER)
        // Set the style of the TextBundle itself.
        .with_style(Style {
            position_type: PositionType::Absolute,
            position: UiRect {
                bottom: Val::Px(150.0),
                left: Val::Px(300.0),
                ..default()
            },
            ..default()
        }),
        NameDialogText,
    ));
}

fn close_dialog_window(
    mut commands: Commands,
    dialog_window: Query<Entity, With<DialogWindow>>,
    name_dialog_text: Query<Entity, With<NameDialogText>>,
) {
    commands.entity(dialog_window.single()).despawn();
    commands.entity(name_dialog_text.single()).despawn();
}

fn dialog_window_trigger(
    keys: Res<Input<KeyCode>>,
    mut app_state: ResMut<State<AppState>>,
    nearest_npc_in_proximity: Res<NearestNPCinProximity>,
) {
    if keys.just_pressed(KeyCode::E) {
        match app_state.current() {
            AppState::InGame => {
                if nearest_npc_in_proximity.any() {
                    app_state.push(AppState::DialogWindow).unwrap();
                }
            }
            AppState::DialogWindow => {
                app_state.pop().unwrap();
            }
            AppState::PauseScreen | AppState::MainMenu => (),
        }
    }
}

#[derive(Component)]
struct MainMenu;

#[derive(Bundle)]
struct MainMenuBundle {
    sprite: SpriteBundle,

    _identity: MainMenu,
    _unload: MainMenuUnload,
}

enum Stacking {
    InGame,
    DialogWindow,
    PauseScreen,
    MainMenu,
}

impl Stacking {
    fn sorting(self) -> f32 {
        let z = match self {
            Self::InGame => 0,
            Self::DialogWindow => 1,
            Self::PauseScreen => 2,
            Self::MainMenu => 3,
        } as f32;
        z
    }

    fn from_xy(self, x: f32, y: f32) -> Transform {
        Transform::from_xyz(x, y, self.sorting())
    }
}

impl Into<Transform> for Stacking {
    fn into(self) -> Transform {
        self.from_xy(0., 0.)
    }
}

fn setup_main_menu(mut commands: Commands) {
    commands.spawn(MainMenuBundle {
        sprite: SpriteBundle {
            sprite: Sprite {
                color: *Color::INDIGO.as_rgba().set_a(0.5),
                custom_size: Some(Vec2::new(400.0, 300.0)),
                ..default()
            },
            transform: Stacking::MainMenu.into(),
            ..default()
        },
        _identity: MainMenu,
        _unload: MainMenuUnload,
    });
}

fn button_main_menu_trigger(keys: Res<Input<KeyCode>>, mut app_state: ResMut<State<AppState>>) {
    if keys.just_pressed(KeyCode::Tab) {
        debug!("current state {:?}", app_state.current());
        match app_state.current() {
            // unless its an initial state, make it possible to trigger only from PauseScreen
            AppState::PauseScreen => {
                app_state.replace(AppState::MainMenu).unwrap();
            }
            AppState::MainMenu => {
                app_state.replace(AppState::InGame).unwrap();
            }
            state @ (AppState::InGame | AppState::DialogWindow) => {
                warn!("can't go to the main menu from {:?}", state);
            }
        }
    }
}
