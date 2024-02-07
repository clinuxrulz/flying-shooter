use crate::{args::Args, fps_plugin::FpsPlugin, pbr_material::CustomStandardMaterial};
use bevy::{prelude::*, utils::HashMap};
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

// The first generic parameter, u8, is the input type: 4-directions + fire fits
// easily in a single byte
// The second parameter is the address type of peers: Matchbox' WebRtcSocket
// addresses are called `PeerId`s
pub type Config = bevy_ggrs::GgrsConfig<[u8; 3], PeerId>;

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
    }

    export function go_fullscreen() {
        document.body.querySelector(\"canvas\").requestFullscreen();
    }
    "
)]
extern "C" {
    fn url_params() -> Vec<String>;
    fn go_fullscreen();
}

pub fn run_game() {
    let args = Args::parse();
    info!("Args: {args:?}");

    let default_room_url = "wss://dune-breezy-honeysuckle.glitch.me/?next=2";

    #[allow(unused_mut)]
    let mut game_config = GameConfig {
        room_url: default_room_url.into(),
    };

    /*
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
    }*/

    App::new()
        .insert_resource(args)
        .insert_resource(game_config)
        .add_state::<GameState>()
        .add_loading_state(
            LoadingState::new(GameState::AssetLoading).continue_to_state(GameState::Matchmaking),
        )
        .add_collection_to_loading_state::<_, ImageAssets>(GameState::AssetLoading)
        .add_collection_to_loading_state::<_, ModelAssets>(GameState::AssetLoading)
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
            FpsPlugin,
            MaterialPlugin::<CustomStandardMaterial>::default(),
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
        .insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.0)))
        .init_resource::<RoundEndTimer>()
        .init_resource::<Scores>()
        //
        .add_systems(
            OnEnter(GameState::Matchmaking),
            (
                setup,
                start_matchbox_socket,
            ),
        )
        .insert_resource(LocalInputs::<Config>(HashMap::from_iter(vec![(0, [0, 0, 0])].drain(0..))))
        .add_systems(
            PreUpdate,
            read_local_inputs.run_if(in_state(GameState::Matchmaking)),
        )
        .add_systems(
            Update,
            (
                swap_standard_material,
                button_system,
            ),
        )
        .add_systems(
            Update,
            (
                (
                    camera_follow,
                    move_skybox_with_camera.after(camera_follow),
                ),
                (
                    move_players,
                    reload_bullet,
                    fire_bullets.after(move_players).after(reload_bullet),
                    move_bullet.after(fire_bullets),
                    wait_for_players,
                ).run_if(in_state(GameState::Matchmaking)),
                (
                    handle_ggrs_events,
                    update_score_ui,
                ).run_if(in_state(GameState::InGame)),
            ),
        )
        //
        /*
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
        */
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
    //Thrust,
}

/// Marker component to identify round buttons
#[derive(Component)]
pub struct RoundButton;

#[derive(AssetCollection, Resource)]
struct ImageAssets {
    #[asset(path = "HDR_blue_nebulae-1.png")]
    sky: Handle<Image>,
}

#[derive(AssetCollection, Resource)]
struct ModelAssets {
    #[asset(path = "low_poly_x-wing.glb#Scene0")]
    xwing: Handle<Scene>,
}

#[derive(Resource)]
struct ModelAssets2 {
    bullet_mesh: Handle<Mesh>,
    bullet_material: Handle<StandardMaterial>,
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    button_style: Res<ButtonStyle>,
    images: Res<ImageAssets>,
    models: Res<ModelAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let camera_init_pos = Vec3::new(0.0, 3.0, -10.0);
    // camera
    commands.spawn(Camera3dBundle {
        transform:
            Transform::from_translation(camera_init_pos.clone())
                .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        projection: Projection::Perspective(PerspectiveProjection {
            far: 100_000.0,
            ..default()
        }),
        ..default()
    });

    // skybox
    let material = materials.add(StandardMaterial {
        base_color_texture: Some(images.sky.clone()),
        unlit: true,
        ..default()
    });
    let sphere_mesh: Handle<Mesh> = meshes.add(shape::UVSphere {
        radius: -90_000.0,
        ..default()
    }.into());
    commands.spawn((
        Skybox,
        PbrBundle {
            mesh: sphere_mesh,
            material,
            transform: Transform::from_translation(camera_init_pos),
            ..default()
        }
    ));

    // Spawn Virtual Joystick at horizontal center
    create_joystick(
        &mut commands,
        asset_server.load("knob.png"),
        asset_server.load("outline.png"),
        None,
        None,
        None,
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
                width: Val::Px(100.0),
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
        });
    
    // load player to use while waiting for players
    commands
        .spawn((
            Player { handle: 0 },
            BulletReady(true),
            FaceDir(0.0),
            Speed(10.0),
            Acceleration(Vec3::ZERO),
            SceneBundle {
                scene: models.xwing.clone(),
                ..default()
            },
        ));
    
    
    commands.spawn((
        AwaitingPlayersRoot,
        NodeBundle {
            z_index: ZIndex::Global(i32::MAX),
            style: Style {
                position_type: PositionType::Absolute,
                top: Val::Percent(1.0),
                padding: UiRect::all(Val::Px(4.0)),
                ..default()
            },
            ..default()
        },
    )).with_children(|parent| {
        parent
            .spawn(
                TextBundle {
                    text: Text::from_section(
                        "Awaiting Players",
                        TextStyle {
                            font_size: 16.0,
                            color: Color::WHITE,
                            ..default()
                        }
                    ),
                    ..default()
                }
            );
    });

    // bullet
    let bullet_mesh = meshes.add(shape::Box::new(0.3, 0.3, 2.0).into());
    let bullet_material = materials.add(StandardMaterial {
        base_color: Color::BLUE,
        unlit: true,
        ..default()
    });

    commands.insert_resource(ModelAssets2 {
        bullet_mesh,
        bullet_material,
    });

    // fullscreen button
    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn(ButtonBundle {
                    style: Style {
                        width: Val::Px(140.0),
                        height: Val::Px(32.0),
                        border: UiRect::all(Val::Px(5.0)),
                        justify_content: JustifyContent::Center,
                        align_content: AlignContent::Center,
                        ..default()
                    },
                    border_color: BorderColor(Color::BLACK),
                    background_color: NORMAL_BUTTON.into(),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "Fullscreen",
                        TextStyle {
                            font_size: 20.0,
                            color: Color::rgb(0.9, 0.9, 0.9),
                            ..default()
                        }
                    ));
                });
        });
}

const NORMAL_BUTTON: Color = Color::rgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::rgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::rgb(0.35, 0.75, 0.35);

fn button_system(
    mut interaction_query: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    _text_query: Query<&mut Text>,
) {
    for (interaction, mut color, mut border_color, _children) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                go_fullscreen();
                *color = PRESSED_BUTTON.into();
                border_color.0 = Color::RED;
            }
            Interaction::Hovered => {
                *color = HOVERED_BUTTON.into();
                border_color.0 = Color::WHITE;
            }
            Interaction::None => {
                *color = NORMAL_BUTTON.into();
                border_color.0 = Color::BLACK;
            }
        }
    }
}

fn move_skybox_with_camera(
    camera: Query<&Transform, (With<Camera>,Without<Skybox>)>,
    mut skybox: Query<&mut Transform,With<Skybox>>,
) {
    for mut skybox_transform in &mut skybox {
        for camera_transform in &camera {
            skybox_transform.translation = camera_transform.translation;
        }
    }
}

fn swap_standard_material(
    mut commands: Commands,
    mut material_events: EventReader<AssetEvent<StandardMaterial>>,
    entites: Query<(Entity, &Handle<StandardMaterial>)>,
    standard_materials: Res<Assets<StandardMaterial>>,
    mut custom_materials: ResMut<Assets<CustomStandardMaterial>>,
) {
    for event in material_events.read() {
        let handle = match event {
            AssetEvent::Added { id } => id,
            AssetEvent::LoadedWithDependencies { id } => id,
            _ => continue,
        };
        if let Some(material) = standard_materials.get(*handle) {
            let custom_mat_h = custom_materials.add(CustomStandardMaterial {
                base_color: material.base_color,
                base_color_texture: material.base_color_texture.clone(),
                emissive: material.emissive,
                emissive_texture: material.emissive_texture.clone(),
                perceptual_roughness: material.perceptual_roughness,
                metallic: material.metallic,
                metallic_roughness_texture: material.metallic_roughness_texture.clone(),
                reflectance: material.reflectance,
                normal_map_texture: material.normal_map_texture.clone(),
                flip_normal_map_y: material.flip_normal_map_y,
                occlusion_texture: material.occlusion_texture.clone(),
                double_sided: material.double_sided,
                cull_mode: material.cull_mode,
                unlit: material.unlit,
                fog_enabled: material.fog_enabled,
                alpha_mode: material.alpha_mode,
                depth_bias: material.depth_bias,
                depth_map: material.depth_map.clone(),
                parallax_depth_scale: material.parallax_depth_scale,
                parallax_mapping_method: material.parallax_mapping_method,
                max_parallax_layer_count: material.max_parallax_layer_count,
                diffuse_transmission: material.diffuse_transmission,
                specular_transmission: material.specular_transmission,
                thickness: material.thickness,
                ior: material.ior,
                attenuation_distance: material.attenuation_distance,
                attenuation_color: material.attenuation_color,
                opaque_render_method: material.opaque_render_method,
                deferred_lighting_pass_id: material.deferred_lighting_pass_id,
            });
            for (entity, entity_mat_h) in entites.iter() {
                if entity_mat_h.id() == *handle {
                    let mut ecmds = commands.entity(entity);
                    ecmds.remove::<Handle<StandardMaterial>>();
                    ecmds.insert(custom_mat_h.clone());
                }
            }
        }
    }
}

/// Updates button materials when their interaction changes
#[allow(clippy::type_complexity)]
fn handle_button_interactions(
    mut interaction_query: Query<
        (&Interaction, &mut Handle<RoundUiMaterial>, &ButtonAction),
        Changed<Interaction>,
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
    mut awaiting_players: Query<&mut Visibility, With<AwaitingPlayersRoot>>,
    players: Query<Entity, With<Player>>,
    bullets: Query<Entity, With<Bullet>>,
    models: Res<ModelAssets>,
) {
    info!("Spawning players");

    for mut awaiting_player_visibility in &mut awaiting_players {
        *awaiting_player_visibility = Visibility::Hidden;
    }

    for player in &players {
        commands.entity(player).despawn_recursive();
    }

    for bullet in &bullets {
        commands.entity(bullet).despawn_recursive();
    }

    // Player 1
    commands
        .spawn((
            Player { handle: 0 },
            BulletReady(true),
            FaceDir(0.0),
            Speed(10.0),
            Acceleration(Vec3::ZERO),
            SceneBundle {
                scene: models.xwing.clone(),
                ..default()
            },
        ))
        .add_rollback();
    
    // Player 2
    commands
        .spawn((
            Player { handle: 1 },
            BulletReady(true),
            FaceDir(0.0),
            Speed(10.0),
            Acceleration(Vec3::ZERO),
            SceneBundle {
                scene: models.xwing.clone(),
                transform: Transform::from_translation(Vec3::new(0.0, 2.0, 200.0)).looking_to(Vec3::Z, Vec3::Y),
                ..default()
            },
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
    mut players: Query<(&mut Transform, &mut Speed, &mut Acceleration, &mut FaceDir, &Player)>,
    local_inputs: Option<Res<LocalInputs<Config>>>,
    inputs: Option<Res<PlayerInputs<Config>>>,
    time: Res<Time>,
    _cameras: Query<&mut Transform, (With<Camera>, Without<Player>)>,
) {
    for (mut transform, speed , _acceleration, _face_dir, player) in &mut players {
        let input: [u8; 3];
        if let Some(inputs) = &inputs {
            input = inputs[player.handle].0;
        } else if let Some(inputs) = &local_inputs {
            input = inputs.0[&player.handle];
        } else {
            input = [0; 3];
        }
        let angular_thrust_pitch = angular_thrust_pitch(input);
        if angular_thrust_pitch != 0.0 {
            transform.rotate_local_axis(Vec3::X, angular_thrust_pitch * std::f32::consts::PI / 180.0 * time.delta_seconds());
        }
        let angular_thrust_roll = angular_thrust_roll(input);
        if angular_thrust_roll != 0.0 {
            transform.rotate_local_axis(Vec3::Z, angular_thrust_roll * std::f32::consts::PI / 180.0 * time.delta_seconds());
        }
        let velocity = transform.rotation.mul_vec3(Vec3::Z) * speed.0;
        transform.translation += velocity * time.delta_seconds();
    }
}

fn reload_bullet(
    inputs: Option<Res<PlayerInputs<Config>>>,
    local_inputs: Option<Res<LocalInputs<Config>>>,
    mut players: Query<(&mut BulletReady, &Player)>,
) {
    for (mut can_fire, player) in players.iter_mut() {
        let input: [u8; 3];
        if let Some(inputs) = &inputs {
            input = inputs[player.handle].0;
        } else if let Some(inputs) = &local_inputs {
            input = inputs.0[&player.handle];
        } else {
            input = [0; 3];
        }
        if !fire(input) {
            can_fire.0 = true;
        }
    }
}

fn fire_bullets(
    mut commands: Commands,
    inputs: Option<Res<PlayerInputs<Config>>>,
    local_inputs: Option<Res<LocalInputs<Config>>>,
    models: Res<ModelAssets2>,
    mut players: Query<(&Transform, &Player, &mut BulletReady)>,
    time: Res<Time>,
) {
    for (transform, player, mut bullet_ready) in &mut players {
        let input: [u8; 3];
        if let Some(inputs) = &inputs {
            input = inputs[player.handle].0;
        } else if let Some(inputs) = &local_inputs {
            input = inputs.0[&player.handle];
        } else {
            input = [0; 3];
        }
        if fire(input) && bullet_ready.0 {
            let bullet_transform = *transform * Transform::from_translation(Vec3::new(0.0, 0.0, 2.0));
            let offset_width: f32 = 2.2;
            let offset_height: f32 = 1.0;
            let bullet_offsets: [[f32; 2]; 4] = [
                [-offset_width, -offset_height],
                [-offset_width, offset_height],
                [offset_width, -offset_height],
                [offset_width, offset_height],
            ];
            for bullet_offset in bullet_offsets {
                commands
                    .spawn((
                        Bullet,
                        BirthTime(time.elapsed_seconds()),
                        PbrBundle {
                            transform: bullet_transform * Transform::from_translation(Vec3::new(bullet_offset[0], bullet_offset[1], 0.0)),
                            mesh: models.bullet_mesh.clone(),
                            material: models.bullet_material.clone(),
                            ..default()
                        },
                    ))
                    .add_rollback();
            }
            bullet_ready.0 = false;
        }
    }
}

fn move_bullet(
    mut commands: Commands,
    mut bullets: Query<(Entity, &mut Transform, &BirthTime), With<Bullet>>,
    time: Res<Time>
) {
    const BULLET_DIE_IN_SECONDS: f32 = 10.0;
    for (bullet_entity, mut transform, birth_time) in &mut bullets {
        let bullet_age = time.elapsed_seconds() - birth_time.0;
        if bullet_age >= BULLET_DIE_IN_SECONDS {
            commands.entity(bullet_entity).despawn_recursive();
        } else {
            let speed = 200.;
            let delta = transform.rotation * (Vec3::Z * speed * time.delta_seconds());
            transform.translation += delta;
        }
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
    local_players: Option<Res<LocalPlayers>>,
    players: Query<(&Player, &Transform)>,
    mut cameras: Query<&mut Transform, (With<Camera>, Without<Player>)>,
    time: Res<Time>,
) {
    for (player, player_transform) in &players {
        if let Some(local_players) = &local_players {
            if !local_players.0.is_empty() {
                if !local_players.0.contains(&player.handle) {
                    continue;
                }
            }
        }
        for mut transform in &mut cameras {
            let target = player_transform.transform_point(Vec3::new(0.0, 1.5, -10.0));
            let delta = (target - transform.translation) * (2.5f32 * time.delta_seconds()).min(1.0);
            transform.translation += delta;
            let target_rotation = player_transform.rotation * Quat::from_rotation_y(std::f32::consts::PI);
            transform.rotation = transform.rotation.lerp(target_rotation, (2.5f32 * time.delta_seconds()).min(1.0));
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
                    .color(Color32::RED)
                    .font(FontId::proportional(72.0)),
            );
        });
}
