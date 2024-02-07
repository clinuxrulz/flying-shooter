use crate::{components::Player, game::{ButtonAction, Config}};
use bevy::{prelude::*, utils::HashMap};
use bevy_ggrs::{LocalInputs, LocalPlayers};
use virtual_joystick::*;

const INPUT_UP: u8 = 1 << 0;
const INPUT_DOWN: u8 = 1 << 1;
const INPUT_LEFT: u8 = 1 << 2;
const INPUT_RIGHT: u8 = 1 << 3;
const INPUT_FIRE: u8 = 1 << 4;

const PITCH_SPEED: f32 = 100.0;
const ROLL_SPEED: f32 = 100.0;

pub fn read_local_inputs(
    mut commands: Commands,
    keys: Res<Input<KeyCode>>,
    players: Query<&Player>,
    local_players: Option<Res<LocalPlayers>>,
    mut joystick: EventReader<VirtualJoystickEvent<String>>,
    interaction_query: Query<(&Interaction, &ButtonAction)>,//, Changed<Interaction>>,
) {
    let mut handles: Vec<usize> = Vec::new();
    if let Some(local_players) = &local_players {
        if local_players.0.is_empty() {
            handles.push(0);
        } else {
            for handle in &local_players.0 {
                handles.push(*handle);
            }
        }
    } else {
        handles.push(0);
    }
    let mut local_inputs = HashMap::new();
    for handle in &handles {
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
