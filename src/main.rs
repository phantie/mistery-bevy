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

type Rational32 = num_rational::Ratio<u32>;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum AppState {
    MainMenu,
    InGame,
    PauseScreen,
    Settings,
    DialogWindow,
}

#[derive(SystemLabel)]
enum Label {
    SetupCamera,
    SpawnPlayer,
    NextToNPCEventHandler,
    AwayFromNPCEventHandler,
    NextToObjectWatcher,
    SpawnNPCs,
}

#[derive(Debug, PartialEq)]
struct ScreenResolutionRatio {
    width: u32,
    height: u32,
}

#[derive(Resource, Debug, PartialEq)]
struct ScreenResolution {
    ratio: ScreenResolutionRatio,
    scale: u32,
}

impl ScreenResolution {
    fn new(width: u32, height: u32, scale: u32) -> Self {
        Self {
            ratio: ScreenResolutionRatio { width, height },
            scale,
        }
    }
}

impl TryInto<ScreenResolution> for (u32, u32) {
    type Error = ();

    fn try_into(self) -> Result<ScreenResolution, Self::Error> {
        let (width, height) = self;

        let ratio = Rational32::new(width, height);

        let (width_reduced, height_reduced) = (*ratio.numer(), *ratio.denom());

        match (width_reduced, height_reduced) {
            (16, 9) | (16, 10) | (4, 3) => Ok(ScreenResolution::new(
                width_reduced,
                height_reduced,
                (width as f32 / width_reduced as f32) as u32,
            )),
            _ => Err(()),
        }
    }
}

// impl Default for ScreenResolution {
//     fn default() -> Self {

//     }
// }

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(Color::DARK_GRAY))
        .add_startup_system(set_up_camera)
        .insert_resource(ProximityToObjResource::default())
        .insert_resource(NearestNPCinProximity::default())
        .add_event::<NextToObjEvent>()
        .add_event::<AwayFromObjEvent>()
        .add_system(window_scaling)
        .add_system(away_from_npc_event_handler.label(Label::AwayFromNPCEventHandler))
        .add_system(next_to_npc_event_handler.after(Label::AwayFromNPCEventHandler))
        // .add_state(AppState::MainMenu)
        .add_state(AppState::InGame)
        .add_system_set(
            SystemSet::on_enter(AppState::InGame)
                .with_system(spawn_player)
                .label(Label::SpawnPlayer),
        )
        .add_system_set(
            SystemSet::on_enter(AppState::InGame)
                .with_system(spawn_npcs)
                .label(Label::SpawnNPCs),
        )
        .add_system_set(SystemSet::on_update(AppState::InGame).with_system(next_to_obj_watcher))
        // move player only when InGame
        .add_system_set(SystemSet::on_update(AppState::InGame).with_system(player_movement))
        .add_system_set(
            SystemSet::on_exit(AppState::InGame).with_system(despawn_all_recursive::<LevelUnload>),
        )
        .add_system_set(
            SystemSet::on_exit(AppState::InGame)
                .with_system(clean_resource::<ProximityToObjResource>),
        )
        .add_system_set(
            SystemSet::on_exit(AppState::InGame)
                .with_system(clean_resource::<NearestNPCinProximity>),
        )
        .add_system(keyboard_pause_screen_trigger)
        .add_system_set(SystemSet::on_enter(AppState::PauseScreen).with_system(setup_pause_screen))
        .add_system_set(SystemSet::on_exit(AppState::PauseScreen).with_system(close_pause_screen))
        .add_system(keyboard_dialog_window_trigger)
        .add_system_set(
            SystemSet::on_enter(AppState::DialogWindow).with_system(setup_dialog_window),
        )
        .add_system_set(SystemSet::on_exit(AppState::DialogWindow).with_system(close_dialog_window))
        .add_system(keyboard_main_menu_trigger)
        .add_system_set(SystemSet::on_enter(AppState::MainMenu).with_system(setup_main_menu))
        .add_system_set(
            SystemSet::on_exit(AppState::MainMenu)
                .with_system(despawn_all_recursive::<MainMenuUnload>),
        )
        .add_system(keyboard_settings_trigger)
        .add_system_set(SystemSet::on_enter(AppState::Settings).with_system(setup_settings))
        .add_system_set(
            SystemSet::on_exit(AppState::Settings)
                .with_system(despawn_all_recursive::<SettingsUnload>),
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

impl PlayerBundle {
    fn new(model: MaterialMesh2dBundle<ColorMaterial>) -> Self {
        Self {
            name: "Player".into(),
            model: model,
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
        }
    }
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

trait TransformExt {
    fn from_xy(x: f32, y: f32) -> Self;
}

impl TransformExt for Transform {
    fn from_xy(x: f32, y: f32) -> Self {
        Self::from_xyz(x, y, 0.)
    }
}

impl NPCBundle {
    fn new(name: impl Into<Name>, transform: Transform) -> Self {
        Self {
            name: name.into(),
            in_proximity: InProximity {
                edge_distance: 150.,
            },
            model: SpriteBundle {
                sprite: Sprite {
                    color: Color::rgb(0.25, 0.25, 0.75),
                    custom_size: Some(Vec2::new(100., 100.)),
                    ..default()
                },
                transform,
                ..default()
            },
            _identity: NPC,
            _unload: LevelUnload,
        }
    }
}

fn spawn_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    // asset_server: Res<AssetServer>,
) {
    let shape = shape::Circle::new(50.);
    let material = ColorMaterial::from(Color::BEIGE);

    commands.spawn(PlayerBundle::new(MaterialMesh2dBundle {
        mesh: meshes.add(shape.into()).into(),
        material: materials.add(material),
        // transform: Transform::from_translation(Vec3::ZERO),
        ..default()
    }));
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
    commands.spawn(NPCBundle::new("Joe", Transform::from_xy(200., 0.)));
    commands.spawn(NPCBundle::new("Rue", Transform::from_xy(-200., 100.)));
    commands.spawn(NPCBundle::new("Moe", Transform::from_xy(-350., 100.)));

    info!("Spawning NPC");
}

trait ResourceClean: Resource {
    fn clean(&mut self);
}

impl ResourceClean for ProximityToObjResource {
    fn clean(&mut self) {
        self.values.clear()
    }
}

impl ResourceClean for NearestNPCinProximity {
    fn clean(&mut self) {
        self.value.clear()
    }
}

fn clean_resource<T: ResourceClean>(mut resource: ResMut<T>) {
    resource.clean()
}

#[derive(Resource, Default)]
struct ProximityToObjResource {
    values: HashMap<Entity, (bool, f32)>,
}

#[derive(Resource, Default)]
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
    Settings,
    MainMenu,
}

impl Stacking {
    fn sorting(self) -> f32 {
        let z = match self {
            Self::InGame => 0,
            Self::DialogWindow => 1,
            Self::PauseScreen => 2,
            Self::MainMenu => 3,
            Self::Settings => 4,
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

fn main_menu_trigger(mut app_state: ResMut<State<AppState>>) {
    match app_state.current() {
        // unless its an initial state, make it possible to trigger only from PauseScreen
        AppState::PauseScreen => app_state.replace(AppState::MainMenu).unwrap(),
        AppState::Settings => app_state.replace(AppState::MainMenu).unwrap(),
        AppState::MainMenu => app_state.replace(AppState::InGame).unwrap(),
        state @ (AppState::InGame | AppState::DialogWindow) => {
            warn!("can't go to the main menu from {:?}", state)
        }
    }
}

fn keyboard_main_menu_trigger(keys: Res<Input<KeyCode>>, app_state: ResMut<State<AppState>>) {
    if keys.just_pressed(KeyCode::Tab) {
        debug!("current state {:?}", app_state.current());
        main_menu_trigger(app_state);
    }
}

fn pause_screen_trigger(mut app_state: ResMut<State<AppState>>) {
    match app_state.current() {
        AppState::InGame | AppState::DialogWindow => app_state.push(AppState::PauseScreen).unwrap(),
        AppState::PauseScreen => app_state.pop().unwrap(),
        AppState::MainMenu | AppState::Settings => (),
    }
}

fn keyboard_pause_screen_trigger(keys: Res<Input<KeyCode>>, app_state: ResMut<State<AppState>>) {
    if keys.just_pressed(KeyCode::M) {
        pause_screen_trigger(app_state);
    }
}

fn dialog_window_trigger(
    mut app_state: ResMut<State<AppState>>,
    nearest_npc_in_proximity: Res<NearestNPCinProximity>,
) {
    match app_state.current() {
        AppState::InGame => {
            if nearest_npc_in_proximity.any() {
                app_state.push(AppState::DialogWindow).unwrap();
            }
        }
        AppState::DialogWindow => app_state.pop().unwrap(),
        AppState::PauseScreen | AppState::MainMenu | AppState::Settings => (),
    }
}

fn keyboard_dialog_window_trigger(
    keys: Res<Input<KeyCode>>,
    app_state: ResMut<State<AppState>>,
    nearest_npc_in_proximity: Res<NearestNPCinProximity>,
) {
    if keys.just_pressed(KeyCode::E) {
        dialog_window_trigger(app_state, nearest_npc_in_proximity);
    }
}

#[derive(Component)]
struct Settings;

#[derive(Component)]
struct SettingsUnload;

#[derive(Bundle)]
struct SettingsBundle {
    sprite: SpriteBundle,

    _identity: Settings,
    _unload: SettingsUnload,
}

fn setup_settings(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(SettingsBundle {
        sprite: SpriteBundle {
            sprite: Sprite {
                color: *Color::ORANGE_RED.as_rgba().set_a(0.5),
                custom_size: Some(Vec2::new(350.0, 250.0)),
                ..default()
            },
            transform: Stacking::Settings.into(),
            ..default()
        },
        _identity: Settings,
        _unload: SettingsUnload,
    });

    commands.spawn((
        // Create a TextBundle that has a Text with a single section.
        TextBundle::from_section(
            // Accepts a `String` or any type that converts into a `String`, such as `&str`
            "Settings",
            TextStyle {
                font: asset_server.load("fonts/OpenSans.ttf"),
                font_size: 80.,
                color: Color::WHITE,
            },
        ) // Set the alignment of the Text
        .with_text_alignment(TextAlignment::TOP_RIGHT)
        // Set the style of the TextBundle itself.
        .with_style(Style {
            position_type: PositionType::Absolute,
            position: UiRect {
                left: Val::Px(500.0),
                top: Val::Px(250.0),
                ..default()
            },
            ..default()
        }),
        SettingsUnload,
    ));
}

fn settings_window_trigger(mut app_state: ResMut<State<AppState>>) {
    match app_state.current() {
        AppState::MainMenu => app_state.push(AppState::Settings).unwrap(),
        AppState::Settings => app_state.pop().unwrap(),
        _ => (),
    }
}

fn keyboard_settings_trigger(keys: Res<Input<KeyCode>>, app_state: ResMut<State<AppState>>) {
    if keys.just_pressed(KeyCode::R) {
        settings_window_trigger(app_state);
    }
}

// temporary
fn window_scaling(mut windows: ResMut<Windows>, keys: Res<Input<KeyCode>>) {
    let window = windows.get_primary_mut().unwrap();

    let mut height = window.requested_height();
    let mut width = window.requested_width();

    let scale = 1.05;

    if keys.just_pressed(KeyCode::Equals) {
        height *= scale;
        width *= scale;
        window.set_resolution(width, height);
    }
    if keys.just_pressed(KeyCode::Minus) {
        height /= scale;
        width /= scale;
        window.set_resolution(width, height);
    }
}

mod tests {
    use crate::ScreenResolution;

    #[test]
    fn test_screen_resolution_from_tuple() {
        // input, expected output
        let supported: &[((u32, u32), ScreenResolution)] = &[
            ((1920, 1080), ScreenResolution::new(16, 9, 120)),
            ((1024, 768), ScreenResolution::new(4, 3, 256)),
            // TODO somewhere enforce limit to min and max scale
            ((4, 3), ScreenResolution::new(4, 3, 1)),
        ];

        for (resolution, expected_result) in supported {
            let result: ScreenResolution = (*resolution).try_into().expect(&format!(
                "must be able to convert into ScreenResolution: {:?}",
                resolution
            ));
            assert_eq!(result, *expected_result);
        }

        let unsupported: &[(u32, u32)] = &[(1000, 1000)];
        for resolution in unsupported {
            let result: Result<ScreenResolution, ()> = (*resolution).try_into();
            assert!(
                result.is_err(),
                "must not be able to convert into ScreenResolution: {:?}",
                resolution
            );
        }
    }
}
