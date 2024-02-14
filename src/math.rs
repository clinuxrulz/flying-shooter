use bevy::math::Vec3;

const FINITE_CUBE_SIZE: f32 = 1024.0 * 4.0;

pub fn warp_infinite_space_into_finite_cube(p: Vec3) -> Vec3 {
    let mut x = p.x % FINITE_CUBE_SIZE;
    let mut y = p.y % FINITE_CUBE_SIZE;
    let mut z = p.z % FINITE_CUBE_SIZE;
    if x < 0.0 { x += FINITE_CUBE_SIZE; }
    if y < 0.0 { y += FINITE_CUBE_SIZE; }
    if z < 0.0 { z += FINITE_CUBE_SIZE; }
    return Vec3::new(x, y, z);
}

pub fn finite_cube_point_to_closest_visible_location(observer: Vec3, pt: Vec3) -> Vec3 {
    let mut closest = pt;
    let mut closest_dist = observer.distance_squared(pt);
    for x_offset in [-FINITE_CUBE_SIZE, 0.0f32, FINITE_CUBE_SIZE] {
        for y_offset in [-FINITE_CUBE_SIZE, 0.0f32, FINITE_CUBE_SIZE] {
            for z_offset in [-FINITE_CUBE_SIZE, 0.0f32, FINITE_CUBE_SIZE] {
                if x_offset == 0.0 && y_offset == 0.0 && z_offset == 0.0 {
                    continue;
                }
                let pt2 = pt + Vec3::new(x_offset, y_offset, z_offset);
                let dist = observer.distance_squared(pt2);
                if dist < closest_dist {
                    closest_dist = dist;
                    closest = pt2;
                }
            }
        }
    }
    return closest;
}