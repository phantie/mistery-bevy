// NOTE current implementation of NPC proximity does not take into
//     account lengths when several objects are considered to be in proximity
//     now, the "closest object" is the latest object detected to be in proximity
// OPINION it's not worth playing with entity visibility or despawning
//     when you can just hide them under main menu canvas, will see
// NOTE you cannot trigger state "enter" using pop(), but can using set(state)
// NOTE many unknowns about screen resolutions now
// NOTE for now leaving a state uses simple entity despawn everywhere,
//      for nicer transitions (for ex. animations), functionality must be expanded
// NOTE found easier way to scale objects with scale factor, but it's far from over
// NOTE WindowMode::Fullscreen does not support properly scale UI,
//      and messes up UI of the operating system (at least on Mac it does)
// NOTE spawning UI entities using some hierarchy is tempting, but I get crashes
//      trying to spawn SpriteBundle (for ex.), with TextBundle it does not

#![allow(dead_code, unused_imports)]
use bevy::ecs::event::ManualEventReader;
use bevy::ecs::schedule::ShouldRun;
use bevy::prelude::*;
use bevy::sprite::MaterialMesh2dBundle;
use bevy::utils::{HashMap, HashSet};
use bevy::window::PresentMode;
use bevy::window::WindowResized;
use float_to_int::*;
use std::borrow::BorrowMut;
use std::f32::consts::{PI, SQRT_2};
use Val as FlexVal;
use Val::{Percent, Px};

mod unused_systems;
use crate::unused_systems::*;

const PACKAGE_NAME: &'static str = "mistery";

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
    width: u16,
    height: u16,
}

#[derive(Debug, PartialEq)]
struct ScreenResolution {
    ratio: ScreenResolutionRatio,
    // TODO find a better name, because it's confusing,
    // due to similiraty with "scale factor"
    scale: u16,
}

impl ScreenResolution {
    fn new(width: u16, height: u16, scale: u16) -> Self {
        Self {
            ratio: ScreenResolutionRatio { width, height },
            scale,
        }
    }

    fn width(&self) -> u16 {
        self.ratio.width * self.scale
    }

    fn height(&self) -> u16 {
        self.ratio.height * self.scale
    }
}

#[derive(Resource, Debug, Default)]
struct CurrentScreenResolution {
    value: Option<ScreenResolution>,
}

impl Into<ScreenResolution> for (u16, u16) {
    fn into(self) -> ScreenResolution {
        Into::into(&self)
    }
}

impl Into<ScreenResolution> for &(u16, u16) {
    // removed restrictions on ratios
    fn into(self) -> ScreenResolution {
        let (width, height) = *self;
        assert!(width > 0 && height > 0, "invalid resolution");

        let (width_reduced, height_reduced) = {
            let ratio = num_rational::Ratio::new(width, height);
            (*ratio.numer(), *ratio.denom())
        };

        ScreenResolution::new(width_reduced, height_reduced, width / width_reduced)
    }
}

fn main() {
    let screen_resolution = ScreenResolution::new(16, 9, 80);

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            window: WindowDescriptor {
                title: "Mistery".into(),
                width: screen_resolution.width().into(),
                height: screen_resolution.height().into(),
                present_mode: PresentMode::AutoVsync,
                resizable: false,
                // mode: WindowMode::SizedFullscreen,
                ..default()
            },
            ..default()
        }))
        .insert_resource(ClearColor(Color::DARK_GRAY))
        .add_startup_system(set_up_camera)
        .add_startup_system(init_screen_resolution)
        // .insert_resource(CurrentScreenResolution {value: Some(screen_resolution)})
        .insert_resource(CurrentScreenResolution::default())
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
        .add_system_set(
            SystemSet::on_update(AppState::InGame)
                .with_system(next_to_obj_watcher)
                // move player only when InGame
                .with_system(player_movement),
        )
        .add_system_set(
            SystemSet::on_exit(AppState::InGame)
                .with_system(despawn_all::<LevelUnload>)
                .with_system(reset_resource::<ProximityToObjResource>)
                .with_system(reset_resource::<NearestNPCinProximity>),
        )
        .add_system(keyboard_pause_screen_trigger)
        .add_system_set(SystemSet::on_enter(AppState::PauseScreen).with_system(setup_pause_screen))
        .add_system_set(
            SystemSet::on_exit(AppState::PauseScreen).with_system(despawn_all::<PauseScreen>),
        )
        .add_system(keyboard_dialog_window_trigger)
        .add_system_set(
            SystemSet::on_enter(AppState::DialogWindow).with_system(setup_dialog_window),
        )
        .add_system_set(
            SystemSet::on_exit(AppState::DialogWindow).with_system(despawn_all::<DialogWindow>),
        )
        .add_system(keyboard_main_menu_trigger)
        .add_system_set(SystemSet::on_enter(AppState::MainMenu).with_system(setup_main_menu))
        .add_system_set(SystemSet::on_exit(AppState::MainMenu).with_system(despawn_all::<MainMenu>))
        .add_system(keyboard_settings_trigger)
        .add_system_set(SystemSet::on_enter(AppState::Settings).with_system(setup_settings))
        .add_system_set(SystemSet::on_exit(AppState::Settings).with_system(despawn_all::<Settings>))
        // .add_plugin(bevy::diagnostic::LogDiagnosticsPlugin::default())
        // .add_plugin(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default())
        .add_system(bevy::window::close_on_esc)
        .add_system(window_fullscreen)
        .add_system(on_window_resize)
        .run();
}

fn set_up_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

#[derive(Component)]
pub struct LevelUnload;

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

trait TransformFromXY {
    fn from_xy(x: f32, y: f32) -> Self;
}

impl TransformFromXY for Transform {
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
    debug!("Spawning a player");
}

fn not_spawned<T: Component>(components: Query<With<T>>) -> ShouldRun {
    components.is_empty().into()
}

fn spawned<T: Component>(components: Query<With<T>>) -> ShouldRun {
    (!components.is_empty()).into()
}

fn despawn_all<T: Component>(mut commands: Commands, q: Query<Entity, With<T>>) {
    let type_name = {
        let fully_qualified_type_name = std::any::type_name::<T>();
        fully_qualified_type_name
            .strip_prefix(&format!("{}::", PACKAGE_NAME))
            .unwrap()
    };
    debug!(
        "despawning {}x entities with {} component",
        q.iter().enumerate().count(),
        type_name
    );
    q.for_each(|e| commands.entity(e).despawn());
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

    debug!("Spawning NPC");
}

fn reset_resource<T: Resource + Default>(mut commands: Commands) {
    commands.insert_resource(T::default());
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

        debug!("Next to NPC {}", name.value);
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

        debug!("Away from NPC {}", name.value);
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
                custom_size: Some(Vec2::new(100., 100.)),
                ..default()
            },
            transform: Stacking::PauseScreen.into(),
            ..default()
        },
        _ps: PauseScreen,
    });
}

#[derive(Component)]
struct DialogWindow;

#[derive(Bundle)]
struct DialogWindowBundle {
    sprite: SpriteBundle,
    _dw: DialogWindow,
}

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
                custom_size: Some(Vec2::new(800., 200.)),
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
                font_size: 100.,
                color: Color::WHITE,
            },
        ) // Set the alignment of the Text
        .with_text_alignment(TextAlignment::TOP_CENTER)
        // Set the style of the TextBundle itself.
        .with_style(Style {
            position_type: PositionType::Absolute,
            position: UiRect {
                bottom: Px(150.),
                left: Px(300.),
                ..default()
            },
            ..default()
        }),
        DialogWindow,
    ));
}

#[derive(Component)]
struct MainMenu;

#[derive(Bundle)]
struct MainMenuBundle {
    sprite: SpriteBundle,

    _state: MainMenu,
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
        use Stacking::*;
        (match self {
            InGame => 0u8,
            DialogWindow => 1,
            PauseScreen => 2,
            MainMenu => 3,
            Settings => 4,
        })
        .into()
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
                custom_size: Some(Vec2::new(400., 300.)),
                ..default()
            },
            transform: Stacking::MainMenu.into(),
            ..default()
        },
        _state: MainMenu,
    });
}

fn main_menu_trigger(mut app_state: ResMut<State<AppState>>) {
    match app_state.current() {
        // unless its an initial state, make it possible to trigger only from PauseScreen
        AppState::PauseScreen => app_state.replace(AppState::MainMenu),
        AppState::Settings => app_state.replace(AppState::MainMenu),
        AppState::MainMenu => app_state.replace(AppState::InGame),
        state @ (AppState::InGame | AppState::DialogWindow) => {
            warn!("can't go to the main menu from {:?}", state);
            Ok(())
        }
    }
    .unwrap()
}

fn keyboard_main_menu_trigger(keys: Res<Input<KeyCode>>, app_state: ResMut<State<AppState>>) {
    if keys.just_pressed(KeyCode::Tab) {
        debug!("current state {:?}", app_state.current());
        main_menu_trigger(app_state);
    }
}

fn pause_screen_trigger(mut app_state: ResMut<State<AppState>>) {
    match app_state.current() {
        AppState::InGame | AppState::DialogWindow => app_state.push(AppState::PauseScreen),
        AppState::PauseScreen => app_state.pop(),
        AppState::MainMenu | AppState::Settings => Ok(()),
    }
    .unwrap()
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
                app_state.push(AppState::DialogWindow)
            } else {
                Ok(())
            }
        }
        AppState::DialogWindow => app_state.pop(),
        AppState::PauseScreen | AppState::MainMenu | AppState::Settings => Ok(()),
    }
    .unwrap()
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

#[derive(Bundle)]
struct SettingsBundle {
    sprite: SpriteBundle,

    _state: Settings,
}

fn setup_settings(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(SettingsBundle {
        sprite: SpriteBundle {
            sprite: Sprite {
                color: *Color::ORANGE_RED.as_rgba().set_a(0.5),
                custom_size: Some(Vec2::new(350., 250.)),
                ..default()
            },
            transform: Stacking::Settings.into(),
            ..default()
        },
        _state: Settings,
    });

    commands.spawn((
        TextBundle::from_section(
            "Settings",
            TextStyle {
                font: asset_server.load("fonts/OpenSans.ttf"),
                font_size: 80.,
                color: Color::WHITE,
            },
        )
        .with_text_alignment(TextAlignment::TOP_RIGHT)
        .with_style(Style {
            position_type: PositionType::Absolute,
            position: UiRect {
                left: Px(500.),
                top: Px(250.),
                ..default()
            },
            ..default()
        }),
        Settings,
    ));
}

fn settings_window_trigger(mut app_state: ResMut<State<AppState>>) {
    match app_state.current() {
        AppState::MainMenu => app_state.push(AppState::Settings),
        AppState::PauseScreen => app_state.push(AppState::Settings),
        AppState::Settings => app_state.pop(),
        _ => Ok(()),
    }
    .unwrap();
}

fn keyboard_settings_trigger(keys: Res<Input<KeyCode>>, app_state: ResMut<State<AppState>>) {
    if keys.just_pressed(KeyCode::R) {
        settings_window_trigger(app_state);
    }
}

fn init_screen_resolution(
    windows: Res<Windows>,
    mut current_screen_resolution: ResMut<CurrentScreenResolution>,
) {
    let window = windows.get_primary().unwrap();
    current_screen_resolution.value = Some(window.resolution());
    // debug!("{:?}", window.resize_constraints());
    debug!("{:?}", *current_screen_resolution);
    debug!("ScaleFactor {{ value: {:?} }}", window.scale_factor());
}

// temporary, for testing
fn window_scaling(mut windows: ResMut<Windows>, keys: Res<Input<KeyCode>>) {
    const SCALE: f64 = 0.1;
    if keys.just_pressed(KeyCode::Equals) {
        let window = windows.get_primary_mut().unwrap();
        window.set_scale_factor_override(Some(window.scale_factor() + SCALE));
    }
    if keys.just_pressed(KeyCode::Minus) {
        let window = windows.get_primary_mut().unwrap();
        window.set_scale_factor_override(Some(window.scale_factor() - SCALE));
    }
}

trait WindowExt {
    const WINDOWED: WindowMode;
    const FULLSCREEN: WindowMode;
    type Error;

    fn is_fullscreen(&self) -> bool;
    fn is_windowed(&self) -> bool;
    fn go_fullscreen(&mut self) -> Result<(), Self::Error>;
    fn go_windowed(&mut self) -> Result<(), Self::Error>;
    fn is_valid_mode(&self) -> bool;
    fn resolution(&self) -> ScreenResolution;
}

impl WindowExt for Window {
    const WINDOWED: WindowMode = WindowMode::Windowed;
    const FULLSCREEN: WindowMode = WindowMode::SizedFullscreen;
    type Error = ();

    fn is_fullscreen(&self) -> bool {
        self.mode() == Self::FULLSCREEN
    }

    fn is_windowed(&self) -> bool {
        self.mode() == Self::WINDOWED
    }

    fn go_fullscreen(&mut self) -> Result<(), Self::Error> {
        if self.is_windowed() {
            self.set_mode(Self::FULLSCREEN);
            Ok(())
        } else {
            Err(())
        }
    }

    fn go_windowed(&mut self) -> Result<(), Self::Error> {
        if self.is_fullscreen() {
            self.set_mode(Self::WINDOWED);
            Ok(())
        } else {
            Err(())
        }
    }

    fn is_valid_mode(&self) -> bool {
        self.is_fullscreen() || self.is_windowed()
    }

    fn resolution(&self) -> ScreenResolution {
        static ERR: &str = "dimention must fit into u16";
        (
            TryIntoInt::<u16>::try_into_int(self.width()).expect(ERR),
            TryIntoInt::<u16>::try_into_int(self.height()).expect(ERR),
        )
            .into()
    }
}

fn window_fullscreen(mut windows: ResMut<Windows>, keys: Res<Input<KeyCode>>) {
    if keys.just_pressed(KeyCode::F) {
        let window = windows.get_primary_mut().unwrap();
        assert!(window.is_valid_mode());

        if window.is_fullscreen() {
            window.go_windowed().unwrap();
        } else if window.is_windowed() {
            window.go_fullscreen().unwrap();
        }
    }
}

fn on_window_resize(
    windows: Res<Windows>,
    mut resize_reader: EventReader<WindowResized>,
    mut current_screen_resolution: ResMut<CurrentScreenResolution>,
) {
    // there supposed to be max 1 window max, iter because to make it's more idiomatic and...
    // NOTE don't know why 2 "resized" events received at the very beginning
    // NOTE changing WindowMode it also sends 2 same events
    for _e in resize_reader.iter() {
        let window = windows.get_primary().unwrap();
        debug!(
            "Resized: {:?}; ScaleFactor {{ value: {} }}",
            window.resolution(),
            window.scale_factor(),
        );
        current_screen_resolution.value = Some(window.resolution());
    }
}

mod tests {
    use crate::ScreenResolution;

    #[test]
    fn test_screen_resolution_from_tuple() {
        // input, expected output
        let cases: &[((u16, u16), ScreenResolution)] = &[
            ((1920, 1080), ScreenResolution::new(16, 9, 120)),
            ((1024, 768), ScreenResolution::new(4, 3, 256)),
            ((1000, 1000), ScreenResolution::new(1, 1, 1000)),
            // TODO somewhere enforce limit to min and max scale
            ((4, 3), ScreenResolution::new(4, 3, 1)),
        ];

        for (resolution, expected_result) in cases {
            let result: ScreenResolution = resolution.into();
            assert_eq!(&result, expected_result);
            assert_eq!(resolution, &(result.width(), result.height()));
        }
    }
}
