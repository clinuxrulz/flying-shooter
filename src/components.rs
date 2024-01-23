use bevy::{prelude::*, utils::FixedState};
use std::hash::{BuildHasher, Hash, Hasher};

#[derive(Component, Clone, Copy)]
pub struct Player {
    pub handle: usize,
}

#[derive(Component, Clone, Copy)]
pub struct BulletReady(pub bool);

#[derive(Component)]
pub struct Bullet;

#[derive(Component, Clone, Copy)]
pub struct MoveDir(pub Vec2);

#[derive(Component, Clone, Copy)]
pub struct FaceDir(pub f32);

#[derive(Component, Clone, Copy)]
pub struct Velocity(pub Vec2);

#[derive(Component, Clone, Copy)]
pub struct Acceleration(pub Vec2);

pub fn checksum_face_dir(face_dir: &FaceDir) -> u64 {
    return face_dir.0.to_bits().into();
}

pub fn checksum_velocity(velocity: &Velocity) -> u64 {
    let mut hasher = FixedState.build_hasher();

    velocity.0.x.to_bits().hash(&mut hasher);
    velocity.0.y.to_bits().hash(&mut hasher);

    hasher.finish()
}

pub fn checksum_acceleration(acceleration: &Acceleration) -> u64 {
    let mut hasher = FixedState.build_hasher();

    acceleration.0.x.to_bits().hash(&mut hasher);
    acceleration.0.y.to_bits().hash(&mut hasher);

    hasher.finish()
}

pub fn checksum_transform(transform: &Transform) -> u64 {
    let mut hasher = FixedState.build_hasher();

    assert!(
        transform.translation.is_finite() && transform.rotation.is_finite(),
        "Hashing is not stable for NaN f32 values."
    );

    transform.translation.x.to_bits().hash(&mut hasher);
    transform.translation.y.to_bits().hash(&mut hasher);
    transform.translation.z.to_bits().hash(&mut hasher);

    transform.rotation.x.to_bits().hash(&mut hasher);
    transform.rotation.y.to_bits().hash(&mut hasher);
    transform.rotation.z.to_bits().hash(&mut hasher);
    transform.rotation.w.to_bits().hash(&mut hasher);

    // skip transform.scale as it's not used for gameplay

    hasher.finish()
}
