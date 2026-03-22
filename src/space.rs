use bevy::prelude::*;

use keplerian_sim::Orbit as KeplerOrbit;

#[derive(Component)]
pub struct Orbit(pub KeplerOrbit);

impl From<KeplerOrbit> for Orbit {
    fn from(value: KeplerOrbit) -> Self {
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
