use bevy::{math::DVec3, prelude::*};

use keplerian_sim::Orbit;

#[derive(Component)]
pub struct NewtonOrbit {
    position: DVec3,
    velocity: DVec3,
}

impl NewtonOrbit {
    pub fn new(pos: DVec3, vel: DVec3) -> Self {
        Self {
            position: pos,
            velocity: vel,
        }
    }
}

#[derive(Component)]
pub struct KeplerOrbit(pub Orbit);

impl From<Orbit> for KeplerOrbit {
    fn from(value: Orbit) -> Self {
        Self(value)
    }
}

#[derive(Component)]
pub struct CelestialBody {
    pub gravity: f64,
}

impl CelestialBody {
    pub fn new(gravity: f64) -> Self {
        Self { gravity }
    }
}
