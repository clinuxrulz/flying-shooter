use crate::{components::{FaceDir, Player}, game::{ButtonAction, Config}};
use bevy::{prelude::*, utils::HashMap};
use bevy_ggrs::{LocalInputs, LocalPlayers};
use virtual_joystick::*;

const INPUT_UP: u8 = 1 << 0;
const INPUT_DOWN: u8 = 1 << 1;
const INPUT_LEFT: u8 = 1 << 2;
const INPUT_RIGHT: u8 = 1 << 3;
const INPUT_FIRE: u8 = 1 << 4;

const ROTATE_SPEED: f32 = 100.0;
const PITCH_SPEED: f32 = 100.0;
const ROLL_SPEED: f32 = 100.0;

pub fn read_local_inputs_prematch(
    mut commands: Commands,
    keys: Res<Input<KeyCode>>,
    players: Query<&Player>,
    mut joystick: EventReader<VirtualJoystickEvent<String>>,
    interaction_query: Query<(&Interaction, &ButtonAction)>,//, Changed<Interaction>>,
) {
    {
        let mut local_inputs = HashMap::new();
        let handle: usize = 0;
        let handle = &handle;
        {
            let mut input: [u8; 3] = [0u8; 3];
            for j in joystick.read() {
                if j.get_type() != VirtualJoystickEventType::Drag {
                    continue;
                }
                for player in &players {
                    if player.handle != *handle {
                        continue;
                    }
                    let axis = j.axis();
                    input[1] = ((axis.x * 100.0).round() as i8) as u8;
                    input[2] = ((axis.y * 100.0).round() as i8) as u8;
                }
            }
            if keys.any_pressed([KeyCode::Up, KeyCode::W]) {
                input[0] |= INPUT_UP;
            }
            if keys.any_pressed([KeyCode::Down, KeyCode::S]) {
                input[0] |= INPUT_DOWN;
            }
            if keys.any_pressed([KeyCode::Left, KeyCode::A]) {
                input[0] |= INPUT_LEFT
            }
            if keys.any_pressed([KeyCode::Right, KeyCode::D]) {
                input[0] |= INPUT_RIGHT;
            }
            if keys.any_pressed([KeyCode::Space, KeyCode::Return]) {
                input[0] |= INPUT_FIRE;
            }
            for (interaction, action) in &interaction_query {
                if *interaction == Interaction::Pressed {
                    match action {
                        ButtonAction::Fire => input[0] |= INPUT_FIRE,
                    }
                }
            }
            local_inputs.insert(*handle, input);
        }
    
        commands.insert_resource(LocalInputs::<Config>(local_inputs));
    }
}

pub fn read_local_inputs(
    mut commands: Commands,
    keys: Res<Input<KeyCode>>,
    local_players: Res<LocalPlayers>,
    players: Query<(&Player, &FaceDir)>,
    mut joystick: EventReader<VirtualJoystickEvent<String>>,
    interaction_query: Query<(&Interaction, &ButtonAction)>,//, Changed<Interaction>>,
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
    let min_diff = ROTATE_SPEED * time.delta_seconds() * 10.0;
    for handle in &local_players.0 {
        let mut input: [u8; 3] = [0u8; 3];
        for j in joystick.read() {
            if j.get_type() != VirtualJoystickEventType::Drag {
                continue;
            }
            let axis = j.axis();
            if axis.x == 0.0f32 && axis.y == 0.0f32 {
                continue;
            }
            let target_face_dir = axis.y.atan2(axis.x);
            for (player, face_dir) in &players {
                if player.handle != *handle {
                    continue;
                }
                let face_dir = face_dir.0;
                let diff = angle_diff(face_dir, target_face_dir);
                if diff.abs() > min_diff {
                    if diff < 0.0f32 {
                        input[0] |= INPUT_RIGHT;
                    } else if diff > 0.0f32 {
                        input[0] |= INPUT_LEFT;
                    }
                }
            }
        }
        if keys.any_pressed([KeyCode::Up, KeyCode::W]) {
            input[0] |= INPUT_UP;
        }
        if keys.any_pressed([KeyCode::Down, KeyCode::S]) {
            input[0] |= INPUT_DOWN;
        }
        if keys.any_pressed([KeyCode::Left, KeyCode::A]) {
            input[0] |= INPUT_LEFT
        }
        if keys.any_pressed([KeyCode::Right, KeyCode::D]) {
            input[0] |= INPUT_RIGHT;
        }
        if keys.any_pressed([KeyCode::Space, KeyCode::Return]) {
            input[0] |= INPUT_FIRE;
        }
        for (interaction, action) in &interaction_query {
            if *interaction == Interaction::Pressed {
                match action {
                    ButtonAction::Fire => input[0] |= INPUT_FIRE,
                }
            }
        }
        local_inputs.insert(*handle, input);
    }

    commands.insert_resource(LocalInputs::<Config>(local_inputs));
}

pub fn fire(input: [u8; 3]) -> bool {
    input[0] & INPUT_FIRE != 0
}

pub fn angular_thrust_pitch(input: [u8; 3]) -> f32 {
    if input[0] & INPUT_DOWN != 0 {
        return -PITCH_SPEED;
    } else if input[0] & INPUT_UP != 0 {
        return PITCH_SPEED;
    }
    let joystick_y = ((input[2] as i8) as f32) / 100.0;
    return joystick_y * PITCH_SPEED;
}

pub fn angular_thrust_roll(input: [u8; 3]) -> f32 {
    if input[0] & INPUT_LEFT != 0 {
        return -ROLL_SPEED;
    } else if input[0] & INPUT_RIGHT != 0 {
        return ROLL_SPEED;
    }
    let joystick_x = ((input[1] as i8) as f32) / 100.0;
    return joystick_x * ROLL_SPEED;
}
