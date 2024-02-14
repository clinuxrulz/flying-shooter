use bevy::{app::{Plugin, Startup, Update}, asset::AssetServer, ecs::{entity::Entity, system::{Commands, Query, Res}}, hierarchy::BuildChildren, math::{Vec2, Vec3}, prelude::default, render::color::Color, transform::components::Transform, ui::{node_bundles::{ImageBundle, NodeBundle}, BackgroundColor, PositionType, Style, UiImage, Val}};
use bevy::ecs::component::Component;
use bevy_ggrs::LocalPlayers;
use bevy::prelude::DespawnRecursiveExt;

use crate::components::Player;

pub struct RadarPlugin;

impl Plugin for RadarPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app
            .add_systems(Startup, setup_radar_ui)
            .add_systems(Update, update_radar_ui);
    }
}

#[derive(Component)]
struct Radar;

#[derive(Component)]
struct Blip {
    pub index: usize,
}

fn setup_radar_ui(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    commands
        .spawn((
            Radar,
            ImageBundle {
                style: Style {
                    width: Val::Px(150.0),
                    height: Val::Px(150.0),
                    position_type: PositionType::Absolute,
                    right: Val::Percent(5.0),
                    top: Val::Percent(5.0),
                    ..default()
                },
                image: UiImage {
                    texture: asset_server.load("radar.png"),
                    ..default()
                },
                ..default()
            },
        ));
}

fn update_radar_ui(
    mut commands: Commands,
    local_players: Option<Res<LocalPlayers>>,
    players: Query<(&Transform, &Player)>,
    radar: Query<(Entity,&Radar)>,
    mut blips: Query<(Entity,&Blip,&mut Style)>,
) {
    let Some(local_players) = local_players else { return; };
    let mut index: usize = 0;
    let mut local_transform: Transform = Transform::IDENTITY;
    let mut local_player_found = false;
    for local_player in &local_players.0 {
        for (transform, player) in &players {
            if player.handle == *local_player {
                local_transform = *transform;
                local_player_found = true;
                break;
            }
        }
        if local_player_found {
            break;
        }
    }
    if !local_player_found {
        for (blip_entity, _, _) in &blips {
            commands.entity(blip_entity).despawn_recursive();
        }
        return;
    }
    for (transform, player) in &players {
        if local_players.0.contains(&player.handle) {
            continue;
        }
        let p1 = local_transform.translation;
        let p2 = crate::math::finite_cube_point_to_closest_visible_location(p1, transform.translation);
        let d1 = p2 - p1;
        let mut radius = 75.0 * d1.normalize().dot(local_transform.rotation.mul_vec3(Vec3::Z)).acos().abs() / std::f32::consts::PI;
        if !radius.is_finite() {
            radius = 0.0;
        }
        let angle: f32;
        if radius >= 74.9 {
            angle = 0.0;
        } else if radius >= 1.0 {
            let d2 = Vec2::new(
                local_transform.rotation.mul_vec3(Vec3::X).dot(d1),
                local_transform.rotation.mul_vec3(Vec3::Y).dot(d1),
            );
            angle = std::f32::consts::PI - d2.y.atan2(d2.x);
        } else {
            angle = 0.0;
        }
        let blip_pos = Vec2::new(
            75.0 + angle.cos() * radius,
            75.0 - angle.sin() * radius,
        );
        let mut has_blip: bool = false;
        for (_, blip, mut blip_style) in &mut blips {
            if blip.index != index {
                continue;
            }
            has_blip = true;
            blip_style.left = Val::Px(blip_pos.x - 5.0);
            blip_style.top = Val::Px(blip_pos.y - 5.0);
            break;
        }
        if !has_blip {
            for (radar_entity, _) in &radar {
                commands
                    .entity(radar_entity)
                    .with_children(|parent| {
                        parent.spawn((
                            Blip { index: index, },
                            NodeBundle {
                                style: Style {
                                    width: Val::Px(10.0),
                                    height: Val::Px(10.0),
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(blip_pos.x - 5.0),
                                    top: Val::Px(blip_pos.y - 5.0),
                                    ..default()
                                },
                                background_color: BackgroundColor(Color::RED),
                                ..default()
                            }
                        ));
                    });
                break;
            }
        }
        index += 1;
    }
    for (blip_entity, blip, _) in &blips {
        if blip.index >= index {
            commands.entity(blip_entity).despawn_recursive();
        }
    }
}
