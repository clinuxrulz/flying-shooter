use crate::args::Args;
use bevy::{prelude::*, render::camera::ScalingMode};
use bevy_asset_loader::prelude::*;
use bevy_egui::{
    egui::{self, Align2, Color32, FontId, RichText},
    EguiContexts, EguiPlugin,
};
use bevy_ggrs::{ggrs::DesyncDetection, prelude::*, *};
use bevy_matchbox::prelude::*;
use bevy_roll_safe::prelude::*;
use clap::Parser;
use crate::components::*;
use crate::input::*;
use bevy_prototype_lyon::prelude::*;
use virtual_joystick::*;
use bevy_round_ui::prelude::*;

const THRUST_ACCELERATION: f32 = 0.2;

// The first generic parameter, u8, is the input type: 4-directions + fire fits
// easily in a single byte
// The second parameter is the address type of peers: Matchbox' WebRtcSocket
// addresses are called `PeerId`s
pub type Config = bevy_ggrs::GgrsConfig<u8, PeerId>;

#[derive(States, Clone, Eq, PartialEq, Debug, Hash, Default)]
enum GameState {
    #[default]
    AssetLoading,
    Matchmaking,
    InGame,
}

#[derive(States, Clone, Eq, PartialEq, Debug, Hash, Default)]
enum RollbackState {
    /// When the characters running and gunning
    #[default]
    InRound,
    /// When one character is dead, and we're transitioning to the next round
    RoundEnd,
}

#[derive(Resource, Clone, Deref, DerefMut)]
struct RoundEndTimer(Timer);

#[derive(Resource, Default, Clone, Copy, Debug)]
struct Scores(u32, u32);

impl Default for RoundEndTimer {
    fn default() -> Self {
        RoundEndTimer(Timer::from_seconds(1.0, TimerMode::Repeating))
    }
}

#[derive(Resource, Debug, Clone)]
pub struct GameConfig {
    pub room_url: String,
}

use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(inline_js =
    "export function url_params() {
        let result = [];
        for (let x of new URLSearchParams(window.location.search).entries()) {
            if (x.length < 2) {
                continue;
            }
            result.push(x[0] + \",\" + x[1]);
        }
        return result;
    }"
)]
extern "C" {
    fn url_params() -> Vec<String>;
}

pub fn run_game() {
    let args = Args::parse();
    info!("Args: {args:?}");

    let default_room_url = "ws://127.0.0.1:3536/extreme_bevy?next=2";

    #[allow(unused_mut)]
    let mut game_config = GameConfig {
        room_url: default_room_url.into(),
    };

    {
        let url_params2 = url_params();
        for x in url_params2 {
            let y: Vec<&str> = x.split(",").collect();
            if y.len() != 2 {
                continue;
            }
            if y[0] == "room_url" {
                game_config.room_url = y[1].into();
            }
        }
    }

    App::new()
        .insert_resource(args)
        .insert_resource(game_config)
        .add_state::<GameState>()
        .add_loading_state(
            LoadingState::new(GameState::AssetLoading).continue_to_state(GameState::Matchmaking),
        )
        .add_collection_to_loading_state::<_, ImageAssets>(GameState::AssetLoading)
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    // fill the entire browser window
                    fit_canvas_to_parent: true,
                    ..default()
                }),
                ..default()
            }),
            GgrsPlugin::<Config>::default(),
            EguiPlugin,
            ShapePlugin,
            VirtualJoystickPlugin::<String>::default(),
            RoundUiPlugin,
        ))
        .init_resource::<ButtonStyle>()
        .add_ggrs_state::<RollbackState>()
        .rollback_resource_with_clone::<RoundEndTimer>()
        .rollback_resource_with_copy::<Scores>()
        .rollback_component_with_clone::<Transform>()
        .rollback_component_with_copy::<BulletReady>()
        .rollback_component_with_copy::<Player>()
        .rollback_component_with_copy::<MoveDir>()
        .rollback_component_with_copy::<FaceDir>()
        .rollback_component_with_copy::<Velocity>()
        .rollback_component_with_copy::<Acceleration>()
        .rollback_component_with_clone::<Sprite>()
        .rollback_component_with_clone::<GlobalTransform>()
        .rollback_component_with_clone::<Handle<Image>>()
        .rollback_component_with_clone::<Visibility>()
        .rollback_component_with_clone::<InheritedVisibility>()
        .rollback_component_with_clone::<ViewVisibility>()
        .checksum_component::<Transform>(checksum_transform)
        .checksum_component::<FaceDir>(checksum_face_dir)
        .checksum_component::<Velocity>(checksum_velocity)
        .checksum_component::<Acceleration>(checksum_acceleration)
        .insert_resource(ClearColor(Color::rgb(0.53, 0.53, 0.53)))
        .init_resource::<RoundEndTimer>()
        .init_resource::<Scores>()
        .add_systems(
            OnEnter(GameState::Matchmaking),
            (setup, start_matchbox_socket.run_if(p2p_mode)),
        )
        .add_systems(
            Update,
            (
                (
                    wait_for_players.run_if(p2p_mode),
                    start_synctest_session.run_if(synctest_mode),
                )
                    .run_if(in_state(GameState::Matchmaking)),
                (camera_follow, update_score_ui, handle_ggrs_events)
                    .run_if(in_state(GameState::InGame)),
            ),
        )
        .add_systems(ReadInputs, read_local_inputs)
        .add_systems(Update, handle_button_interactions)
        .add_systems(OnEnter(RollbackState::InRound), spawn_players)
        .add_systems(
            GgrsSchedule,
            (
                move_players,
                reload_bullet,
                fire_bullets.after(move_players).after(reload_bullet),
                move_bullet.after(fire_bullets),
                kill_players.after(move_bullet).after(move_players),
            )
                .run_if(in_state(RollbackState::InRound))
                .after(apply_state_transition::<RollbackState>),
        )
        .add_systems(
            GgrsSchedule,
            round_end_timeout
                .run_if(in_state(RollbackState::RoundEnd))
                .ambiguous_with(kill_players)
                .after(apply_state_transition::<RollbackState>),
        )
        .run();
}

/// Resource containing material handles for the different button states
#[derive(Resource)]
pub struct ButtonStyle {
    pub width: f32,
    pub height: f32,
    pub default: Handle<RoundUiMaterial>,
    pub hover: Handle<RoundUiMaterial>,
    pub press: Handle<RoundUiMaterial>,
    pub default_2: Handle<RoundUiMaterial>,
    pub hover_2: Handle<RoundUiMaterial>,
    pub press_2: Handle<RoundUiMaterial>,
}

impl FromWorld for ButtonStyle {
    fn from_world(world: &mut World) -> Self {
        let cell = world.cell();
        let mut materials = cell
            .get_resource_mut::<Assets<RoundUiMaterial>>()
            .expect("Failed to get Assets<RoundRectMaterial>");

        let width = 100.;
        let height = 100.;
        let offset = 5.;
        let border_radius = RoundUiBorder::all(100.);

        Self {
            width,
            height,
            default: materials.add(RoundUiMaterial {
                background_color: Color::hex("#F76161").unwrap(),
                border_color: Color::hex("#A53A3D").unwrap(),
                border_radius: border_radius.into(),
                size: Vec2::new(width, height),
                offset: RoundUiOffset::bottom(offset).into(),
            }),
            hover: materials.add(RoundUiMaterial {
                background_color: Color::hex("#F61A39").unwrap(),
                border_color: Color::hex("#A0102A").unwrap(),
                border_radius: border_radius.into(),
                size: Vec2::new(width, height),
                offset: RoundUiOffset::bottom(offset).into(),
            }),
            press: materials.add(RoundUiMaterial {
                background_color: Color::hex("#A0102A").unwrap(),
                border_color: Color::NONE,
                border_radius: border_radius.into(),
                size: Vec2::new(width, height),
                offset: RoundUiOffset::top(offset).into(),
            }),
            default_2: materials.add(RoundUiMaterial {
                background_color: Color::hex("#6161F7").unwrap(),
                border_color: Color::hex("#3A3DA5").unwrap(),
                border_radius: border_radius.into(),
                size: Vec2::new(width, height),
                offset: RoundUiOffset::bottom(offset).into(),
            }),
            hover_2: materials.add(RoundUiMaterial {
                background_color: Color::hex("#1A39F6").unwrap(),
                border_color: Color::hex("#102AA0").unwrap(),
                border_radius: border_radius.into(),
                size: Vec2::new(width, height),
                offset: RoundUiOffset::bottom(offset).into(),
            }),
            press_2: materials.add(RoundUiMaterial {
                background_color: Color::hex("#102AA0").unwrap(),
                border_color: Color::NONE,
                border_radius: border_radius.into(),
                size: Vec2::new(width, height),
                offset: RoundUiOffset::top(offset).into(),
            }),
        }
    }
}

/// Button actions for handling click events
#[derive(Component, Debug, PartialEq, Eq)]
pub enum ButtonAction {
    Fire,
    Thrust,
}

/// Marker component to identify round buttons
#[derive(Component)]
pub struct RoundButton;

const MAP_SIZE: i32 = 41;
const GRID_WIDTH: f32 = 0.05;

#[derive(AssetCollection, Resource)]
struct ImageAssets {
    #[asset(path = "bullet.png")]
    bullet: Handle<Image>,
}

fn synctest_mode(args: Res<Args>) -> bool {
    args.synctest
}

fn p2p_mode(args: Res<Args>) -> bool {
    !args.synctest
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, button_style: Res<ButtonStyle>,) {
    // Horizontal lines
    for i in 0..=MAP_SIZE {
        commands.spawn(SpriteBundle {
            transform: Transform::from_translation(Vec3::new(
                0.,
                i as f32 - MAP_SIZE as f32 / 2.,
                0.,
            )),
            sprite: Sprite {
                color: Color::rgb(0.27, 0.27, 0.27),
                custom_size: Some(Vec2::new(MAP_SIZE as f32, GRID_WIDTH)),
                ..default()
            },
            ..default()
        });
    }

    // Vertical lines
    for i in 0..=MAP_SIZE {
        commands.spawn(SpriteBundle {
            transform: Transform::from_translation(Vec3::new(
                i as f32 - MAP_SIZE as f32 / 2.,
                0.,
                0.,
            )),
            sprite: Sprite {
                color: Color::rgb(0.27, 0.27, 0.27),
                custom_size: Some(Vec2::new(GRID_WIDTH, MAP_SIZE as f32)),
                ..default()
            },
            ..default()
        });
    }

    let mut camera_bundle = Camera2dBundle::default();
    camera_bundle.projection.scaling_mode = ScalingMode::FixedVertical(10.);
    commands.spawn(camera_bundle);

    // Spawn Virtual Joystick at horizontal center
    create_joystick(
        &mut commands,
        asset_server.load("knob.png"),
        asset_server.load("outline.png"),
        None,
        None,
        Some(Color::ORANGE_RED.with_a(0.3)),
        Vec2::new(75., 75.),
        Vec2::new(150., 150.),
        VirtualJoystickNode {
            dead_zone: 0.,
            id: "UniqueJoystick".to_string(),
            axis: VirtualJoystickAxis::Both,
            behaviour: VirtualJoystickType::Fixed,
        },
        Style {
            width: Val::Px(150.),
            height: Val::Px(150.),
            position_type: PositionType::Absolute,
            left: Val::Percent(10.),
            bottom: Val::Percent(10.),
            ..default()
        },
    );

    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Px(250.0),
                height: Val::Px(100.0),
                position_type: PositionType::Absolute,
                right: Val::Percent(10.),
                bottom: Val::Percent(10.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ..default()
        })
        .with_children(|p| {
            p.spawn((
                MaterialNodeBundle {
                    material: button_style.default.clone(),
                    style: Style {
                        width: Val::Px(100.),
                        height: Val::Px(100.),
                        ..default()
                    },
                    ..default()
                },
                ButtonAction::Fire,
                Interaction::default(),
            ));
            p.spawn((
                MaterialNodeBundle {
                    material: button_style.default_2.clone(),
                    style: Style {
                        width: Val::Px(100.),
                        height: Val::Px(100.),
                        margin: UiRect::left(Val::Px(50.0)),
                        ..default()
                    },
                    ..default()
                },
                ButtonAction::Thrust,
                Interaction::default(),
            ));
        });
}

/// Updates button materials when their interaction changes
#[allow(clippy::type_complexity)]
fn handle_button_interactions(
    mut interaction_query: Query<
        (&Interaction, &mut Handle<RoundUiMaterial>, &ButtonAction),
        (Changed<Interaction>),
    >,
    button_style: Res<ButtonStyle>,
) {
    for (interaction, mut material, button_action) in &mut interaction_query {
        *material = match *interaction {
            Interaction::Pressed => if *button_action == ButtonAction::Fire { button_style.press.clone() } else { button_style.press_2.clone() },
            Interaction::Hovered => if *button_action == ButtonAction::Fire { button_style.hover.clone() } else { button_style.hover_2.clone() },
            Interaction::None => if *button_action == ButtonAction::Fire { button_style.default.clone() } else { button_style.default_2.clone() },
        };
    }
}

fn spawn_players(
    mut commands: Commands,
    players: Query<Entity, With<Player>>,
    bullets: Query<Entity, With<Bullet>>,
) {
    info!("Spawning players");

    for player in &players {
        commands.entity(player).despawn_recursive();
    }

    for bullet in &bullets {
        commands.entity(bullet).despawn_recursive();
    }

    let make_ship_path = || {
        let mut path_builder = PathBuilder::new();
        let scale = 0.5;
        path_builder.move_to(Vec2::new(0.5, 0.0) * scale);
        path_builder.line_to(Vec2::new(-0.5 ,0.3) * scale);
        path_builder.line_to(Vec2::new(-0.25, 0.0) * scale);
        path_builder.line_to(Vec2::new(-0.5, -0.3) * scale);
        path_builder.close();
        let path = path_builder.build();
        return path;
    };

    let p1_path = make_ship_path();
    let p2_path = make_ship_path();

    // Player 1
    commands
        .spawn((
            Player { handle: 0 },
            BulletReady(true),
            MoveDir(Vec2::X),
            FaceDir(0.0),
            Velocity(Vec2::ZERO),
            Acceleration(Vec2::ZERO),
            ShapeBundle {
                path: p1_path,
                spatial: SpatialBundle {
                    transform: Transform::from_translation(Vec3::new(-2., 0., 100.)),
                    ..default()
                },
                ..default()
            },
            Stroke::new(Color::BLACK, 0.05),
            Fill::color(Color::BLUE),
        ))
        .add_rollback();

    // Player 2
    commands
        .spawn((
            Player { handle: 1 },
            BulletReady(true),
            MoveDir(-Vec2::X),
            FaceDir(std::f32::consts::PI),
            Velocity(Vec2::ZERO),
            Acceleration(Vec2::ZERO),
            ShapeBundle {
                path: p2_path,
                spatial: SpatialBundle {
                    transform: Transform::from_translation(Vec3::new(2., 0., 100.)),
                    ..default()
                },
                ..default()
            },
            Stroke::new(Color::BLACK, 0.05),
            Fill::color(Color::BLUE),
        ))
        .add_rollback();
}

fn start_matchbox_socket(mut commands: Commands, game_config: Res<GameConfig>) {
    //let room_url = "ws://127.0.0.1:3536/extreme_bevy?next=2";
    info!("config {:?}", game_config);
    let room_url = game_config.room_url.clone();
    info!("connecting to matchbox server: {room_url}");
    commands.insert_resource(MatchboxSocket::new_ggrs(room_url));
}

fn wait_for_players(
    mut commands: Commands,
    mut socket: ResMut<MatchboxSocket<SingleChannel>>,
    mut next_state: ResMut<NextState<GameState>>,
    args: Res<Args>,
) {
    if socket.get_channel(0).is_err() {
        return; // we've already started
    }

    // Check for new connections
    socket.update_peers();
    let players = socket.players();

    let num_players = 2;
    if players.len() < num_players {
        return; // wait for more players
    }

    info!("All peers have joined, going in-game");

    // create a GGRS P2P session
    let mut session_builder = ggrs::SessionBuilder::<Config>::new()
        .with_num_players(num_players)
        .with_desync_detection_mode(DesyncDetection::On { interval: 1 })
        .with_input_delay(args.input_delay);

    for (i, player) in players.into_iter().enumerate() {
        session_builder = session_builder
            .add_player(player, i)
            .expect("failed to add player");
    }

    // move the channel out of the socket (required because GGRS takes ownership of it)
    let socket = socket.take_channel(0).unwrap();

    // start the GGRS session
    let ggrs_session = session_builder
        .start_p2p_session(socket)
        .expect("failed to start session");

    commands.insert_resource(bevy_ggrs::Session::P2P(ggrs_session));
    next_state.set(GameState::InGame);
}

fn start_synctest_session(mut commands: Commands, mut next_state: ResMut<NextState<GameState>>) {
    info!("Starting synctest session");
    let num_players = 2;

    let mut session_builder = ggrs::SessionBuilder::<Config>::new().with_num_players(num_players);

    for i in 0..num_players {
        session_builder = session_builder
            .add_player(PlayerType::Local, i)
            .expect("failed to add player");
    }

    let ggrs_session = session_builder
        .start_synctest_session()
        .expect("failed to start session");

    commands.insert_resource(bevy_ggrs::Session::SyncTest(ggrs_session));
    next_state.set(GameState::InGame);
}

fn handle_ggrs_events(mut session: ResMut<Session<Config>>) {
    match session.as_mut() {
        Session::P2P(s) => {
            for event in s.events() {
                match event {
                    GgrsEvent::Disconnected { .. } | GgrsEvent::NetworkInterrupted { .. } => {
                        warn!("GGRS event: {event:?}")
                    }
                    GgrsEvent::DesyncDetected {
                        local_checksum,
                        remote_checksum,
                        frame,
                        ..
                    } => {
                        error!("Desync on frame {frame}. Local checksum: {local_checksum:X}, remote checksum: {remote_checksum:X}");
                    }
                    _ => info!("GGRS event: {event:?}"),
                }
            }
        }
        _ => {}
    }
}

fn move_players(
    mut players: Query<(&mut Transform, &mut Velocity, &mut Acceleration, &mut FaceDir, &Player)>,
    inputs: Res<PlayerInputs<Config>>,
    time: Res<Time>,
) {
    for (mut transform, mut velocity , mut acceleration, mut face_dir, player) in &mut players {
        let (input, _) = inputs[player.handle];
        let rotate_by = rotate_by(input);
        face_dir.0 += rotate_by * time.delta_seconds();
        while face_dir.0 < 0.0 {
            face_dir.0 += 2.0 * std::f32::consts::PI;
        }
        while face_dir.0 >= 2.0 * std::f32::consts::PI {
            face_dir.0 -= 2.0 * std::f32::consts::PI;
        }
        transform.rotation = Quat::from_axis_angle(Vec3::new(0., 0., 1.), face_dir.0);
        if thrust(input) {
            acceleration.0 = Vec2::new(face_dir.0.cos(), face_dir.0.sin()) * THRUST_ACCELERATION;
        } else {
            acceleration.0 = Vec2::ZERO;
        }
        velocity.0 += acceleration.0 * time.delta_seconds();
    }
    for (mut transform, velocity, _, _, player) in &mut players {
        let move_speed = 7.;
        let move_delta = velocity.0 * move_speed * time.delta_seconds();

        let old_pos = transform.translation.xy();
        let limit = Vec2::splat(MAP_SIZE as f32 / 2. - 0.5);
        let new_pos = (old_pos + move_delta).clamp(-limit, limit);

        transform.translation.x = new_pos.x;
        transform.translation.y = new_pos.y;
    }
}

fn reload_bullet(
    inputs: Res<PlayerInputs<Config>>,
    mut players: Query<(&mut BulletReady, &Player)>,
) {
    for (mut can_fire, player) in players.iter_mut() {
        let (input, _) = inputs[player.handle];
        if !fire(input) {
            can_fire.0 = true;
        }
    }
}

fn fire_bullets(
    mut commands: Commands,
    inputs: Res<PlayerInputs<Config>>,
    images: Res<ImageAssets>,
    mut players: Query<(&Transform, &Player, &mut BulletReady, &FaceDir)>,
) {
    for (transform, player, mut bullet_ready, face_dir) in &mut players {
        let (input, _) = inputs[player.handle];
        if fire(input) && bullet_ready.0 {
            let move_dir: MoveDir = MoveDir(Vec2::new(face_dir.0.cos(), face_dir.0.sin()));
            let move_dir = &move_dir;
            let player_pos = transform.translation.xy();
            let pos = player_pos + move_dir.0 * PLAYER_RADIUS + BULLET_RADIUS;
            commands
                .spawn((
                    Bullet,
                    *move_dir,
                    SpriteBundle {
                        transform: Transform::from_translation(pos.extend(200.))
                            .with_rotation(Quat::from_rotation_arc_2d(Vec2::X, move_dir.0)),
                        texture: images.bullet.clone(),
                        sprite: Sprite {
                            custom_size: Some(Vec2::new(0.3, 0.1)),
                            ..default()
                        },
                        ..default()
                    },
                ))
                .add_rollback();
            bullet_ready.0 = false;
        }
    }
}

fn move_bullet(mut bullets: Query<(&mut Transform, &MoveDir), With<Bullet>>, time: Res<Time>) {
    for (mut transform, dir) in &mut bullets {
        let speed = 20.;
        let delta = dir.0 * speed * time.delta_seconds();
        transform.translation += delta.extend(0.);
    }
}

const PLAYER_RADIUS: f32 = 0.5;
const BULLET_RADIUS: f32 = 0.025;

fn kill_players(
    mut commands: Commands,
    players: Query<(Entity, &Transform, &Player), Without<Bullet>>,
    bullets: Query<&Transform, With<Bullet>>,
    mut next_state: ResMut<NextState<RollbackState>>,
    mut scores: ResMut<Scores>,
) {
    for (player_entity, player_transform, player) in &players {
        for bullet_transform in &bullets {
            let distance = Vec2::distance(
                player_transform.translation.xy(),
                bullet_transform.translation.xy(),
            );
            if distance < PLAYER_RADIUS + BULLET_RADIUS {
                commands.entity(player_entity).despawn_recursive();
                next_state.set(RollbackState::RoundEnd);

                if player.handle == 0 {
                    scores.1 += 1;
                } else {
                    scores.0 += 1;
                }
                info!("player died: {scores:?}")
            }
        }
    }
}

fn camera_follow(
    local_players: Res<LocalPlayers>,
    players: Query<(&Player, &Transform)>,
    mut cameras: Query<&mut Transform, (With<Camera>, Without<Player>)>,
) {
    for (player, player_transform) in &players {
        // only follow the local player
        if !local_players.0.contains(&player.handle) {
            continue;
        }

        let pos = player_transform.translation;

        for mut transform in &mut cameras {
            transform.translation.x = pos.x;
            transform.translation.y = pos.y;
        }
    }
}

fn round_end_timeout(
    mut timer: ResMut<RoundEndTimer>,
    mut state: ResMut<NextState<RollbackState>>,
    time: Res<Time>,
) {
    timer.tick(time.delta());

    if timer.just_finished() {
        state.set(RollbackState::InRound);
    }
}

fn update_score_ui(mut contexts: EguiContexts, scores: Res<Scores>) {
    let Scores(p1_score, p2_score) = *scores;

    egui::Area::new("score")
        .anchor(Align2::CENTER_TOP, (0., 25.))
        .show(contexts.ctx_mut(), |ui| {
            ui.label(
                RichText::new(format!("{p1_score} - {p2_score}"))
                    .color(Color32::BLACK)
                    .font(FontId::proportional(72.0)),
            );
        });
}
