use crate::{components::{FaceDir, Player}, game::Config};
use bevy::{prelude::*, utils::HashMap};
use bevy_ggrs::{LocalInputs, LocalPlayers};
use virtual_joystick::*;

const INPUT_UP: u8 = 1 << 0;
const INPUT_DOWN: u8 = 1 << 1;
const INPUT_LEFT: u8 = 1 << 2;
const INPUT_RIGHT: u8 = 1 << 3;
const INPUT_FIRE: u8 = 1 << 4;

const ROTATE_SPEED: f32 = 0.1;

pub fn read_local_inputs(
    mut commands: Commands,
    keys: Res<Input<KeyCode>>,
    local_players: Res<LocalPlayers>,
    players: Query<(&Player, &FaceDir)>,
    mut joystick: EventReader<VirtualJoystickEvent<String>>,
    time: Res<Time>
) {
    let mut local_inputs = HashMap::new();

    let angle_diff = |a: f32, b: f32| -> f32 {
        //     double diff = ( angle2 - angle1 + 180 ) % 360 - 180;
        // return diff < -180 ? diff + 360 : diff;
        let diff = (b - a + std::f32::consts::PI) % (2.0 * std::f32::consts::PI) - std::f32::consts::PI;
        if diff < -std::f32::consts::PI {
            diff + 2.0 * std::f32::consts::PI
        } else {
            diff
        }
    };
    let min_diff = time.delta_seconds() * ROTATE_SPEED * 10.0;
    for handle in &local_players.0 {
        let mut input = 0u8;
        for j in joystick.read().take(1) {
            let axis = j.axis();
            if axis.x == 0.0f32 && axis.y == 0.0f32 {
                continue;
            }
            let target_face_dir = (-axis.y).atan2(-axis.x);
            for (player, face_dir) in &players {
                if player.handle != *handle {
                    continue;
                }
                let face_dir = face_dir.0;
                let diff = angle_diff(face_dir, target_face_dir);
                if diff < -min_diff {
                    input |= INPUT_LEFT;
                } else if diff > min_diff {
                    input |= INPUT_RIGHT;
                }
            }
        }
        if keys.any_pressed([KeyCode::Up, KeyCode::W]) {
            input |= INPUT_UP;
        }
        if keys.any_pressed([KeyCode::Down, KeyCode::S]) {
            input |= INPUT_DOWN;
        }
        if keys.any_pressed([KeyCode::Left, KeyCode::A]) {
            input |= INPUT_LEFT
        }
        if keys.any_pressed([KeyCode::Right, KeyCode::D]) {
            input |= INPUT_RIGHT;
        }
        if keys.any_pressed([KeyCode::Space, KeyCode::Return]) {
            input |= INPUT_FIRE;
        }
        local_inputs.insert(*handle, input);
    }

    commands.insert_resource(LocalInputs::<Config>(local_inputs));
}

pub fn rotate_by(input: u8) -> f32 {
    if (input & INPUT_LEFT) != 0 {
        return ROTATE_SPEED;
    }
    if (input & INPUT_RIGHT) != 0 {
        return -ROTATE_SPEED;
    }
    return 0.0;
}

pub fn thrust(input: u8) -> bool {
    input & INPUT_UP != 0
}

pub fn direction(input: u8) -> Vec2 {
    let mut direction = Vec2::ZERO;
    if input & INPUT_UP != 0 {
        direction.y += 1.;
    }
    if input & INPUT_DOWN != 0 {
        direction.y -= 1.;
    }
    if input & INPUT_RIGHT != 0 {
        direction.x += 1.;
    }
    if input & INPUT_LEFT != 0 {
        direction.x -= 1.;
    }
    direction.normalize_or_zero()
}

pub fn fire(input: u8) -> bool {
    input & INPUT_FIRE != 0
}
