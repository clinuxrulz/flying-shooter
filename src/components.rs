use bevy::{prelude::*, utils::FixedState};
use std::hash::{BuildHasher, Hash, Hasher};

#[derive(Component)]
pub struct Skybox;

#[derive(Component)]
pub struct AwaitingPlayersRoot;

#[derive(Component, Clone, Copy)]
pub struct Player {
    pub handle: usize,
}

#[derive(Component, Clone, Copy)]
pub struct FollowPlayer {
    pub target_player_handle: usize,
}

#[derive(Component, Clone, Copy)]
pub struct FollowBullet {
    pub index: usize,
}

#[derive(Component, Clone, Copy)]
pub struct BulletReady(pub bool);

#[derive(Component)]
pub struct Bullet;

#[derive(Component, Clone, Copy)]
pub struct BirthTime(pub f32);

#[derive(Component, Clone, Copy)]
pub struct Velocity(pub Vec3);

#[derive(Component, Clone, Copy)]
pub struct Speed(pub f32);

#[derive(Component, Clone, Copy)]
pub struct Acceleration(pub Vec3);

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
