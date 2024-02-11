use bevy::{app::{Plugin, Startup, Update}, asset::AssetServer, ecs::system::{Commands, Res}, prelude::default, ui::{node_bundles::ImageBundle, PositionType, Style, UiImage, Val}};

pub struct RadarPlugin;

impl Plugin for RadarPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app
            .add_systems(Startup, setup_radar_ui)
            .add_systems(Update, update_radar_ui);
    }
}

fn setup_radar_ui(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    commands
        .spawn((
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

pub fn update_radar_ui() {
}
