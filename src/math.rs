#![expect(unused)]
#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![allow(clippy::float_cmp)]
#![deny(clippy::alloc_instead_of_core)]
#![warn(missing_docs)]

use bevy::math::{DVec2, DVec3};
use std::f64::consts::{PI, TAU};

const GRAVITATIONAL_CONSTANT: f64 = 6.674e-11;

fn true_anomaly_to_eccentric_anomaly(true_anomaly: f64, eccentricity: f64) -> f64 {
    f64::atan2(
        (1.0 - eccentricity * eccentricity).sqrt() * true_anomaly.sin(),
        eccentricity + true_anomaly.cos(),
    )
}

fn mean_anomaly_to_eccentric_anomaly(mean_anomaly: f64, eccentricity: f64) -> f64 {
    let mean_anomaly_normalized = mean_anomaly.rem_euclid(TAU);
    let mut eccentric_anomaly = if eccentricity > 0.8 {
        PI
    } else {
        mean_anomaly_normalized
    };
    for _ in 0..100 {
        let error =
            eccentric_anomaly - eccentricity * eccentric_anomaly.sin() - mean_anomaly_normalized;
        if error.abs() < 1e-13 {
            break;
        }
        eccentric_anomaly -= error / (1.0 - eccentricity * eccentric_anomaly.cos());
    }
    eccentric_anomaly
}

fn eccentric_anomaly_to_true_anomaly(eccentric_anomaly: f64, eccentricity: f64) -> f64 {
    let half_eccentric_anomaly = eccentric_anomaly * 0.5;
    2.0 * f64::atan2(
        (1.0 + eccentricity).sqrt() * half_eccentric_anomaly.sin(),
        (1.0 - eccentricity).sqrt() * half_eccentric_anomaly.cos(),
    )
}

fn eccentric_anomaly_to_mean_anomaly(eccentric_anomaly: f64, eccentricity: f64) -> f64 {
    eccentric_anomaly - eccentricity * eccentric_anomaly.sin()
}

fn true_anomaly_to_mean_anomaly_elliptic(true_anomaly: f64, eccentricity: f64) -> f64 {
    let eccentric_anomaly = true_anomaly_to_eccentric_anomaly(true_anomaly, eccentricity);
    eccentric_anomaly_to_mean_anomaly(eccentric_anomaly, eccentricity)
}

fn true_anomaly_to_hyperbolic_anomaly(true_anomaly: f64, eccentricity: f64) -> f64 {
    let half_angle_factor = ((eccentricity - 1.0) / (eccentricity + 1.0)).sqrt();
    2.0 * (half_angle_factor * (true_anomaly / 2.0).tan()).atanh()
}

fn hyperbolic_anomaly_to_mean_anomaly(hyperbolic_anomaly: f64, eccentricity: f64) -> f64 {
    eccentricity * hyperbolic_anomaly.sinh() - hyperbolic_anomaly
}

fn true_anomaly_to_mean_anomaly_hyperbolic(true_anomaly: f64, eccentricity: f64) -> f64 {
    let hyperbolic_anomaly = true_anomaly_to_hyperbolic_anomaly(true_anomaly, eccentricity);
    hyperbolic_anomaly_to_mean_anomaly(hyperbolic_anomaly, eccentricity)
}

fn mean_anomaly_to_hyperbolic_anomaly(mean_anomaly: f64, eccentricity: f64) -> f64 {
    let mut hyperbolic_anomaly =
        mean_anomaly.signum() * (2.0 * mean_anomaly.abs() / eccentricity + 1.8).ln();
    for _ in 0..100 {
        let residual = eccentricity * hyperbolic_anomaly.sinh() - hyperbolic_anomaly - mean_anomaly;
        let derivative = eccentricity * hyperbolic_anomaly.cosh() - 1.0;
        let newton_step = residual / derivative;
        hyperbolic_anomaly -= newton_step;
        if newton_step.abs() < 1e-13 {
            break;
        }
    }
    hyperbolic_anomaly
}

fn hyperbolic_anomaly_to_true_anomaly(hyperbolic_anomaly: f64, eccentricity: f64) -> f64 {
    let half_angle_factor = ((eccentricity + 1.0) / (eccentricity - 1.0)).sqrt();
    2.0 * (half_angle_factor * (hyperbolic_anomaly / 2.0).tanh()).atan()
}

pub struct Matrix3x2 {
    pub e11: f64,
    pub e12: f64,
    pub e21: f64,
    pub e22: f64,
    pub e31: f64,
    pub e32: f64,
}

impl Matrix3x2 {
    /// The zero matrix.
    pub const ZERO: Self = Self {
        e11: 0.0,
        e12: 0.0,
        e21: 0.0,
        e22: 0.0,
        e31: 0.0,
        e32: 0.0,
    };

    /// The identity matrix.
    pub const IDENTITY: Self = Self {
        e11: 1.0,
        e22: 1.0,
        ..Self::ZERO
    };

    /// Computes a dot product between this matrix and a 2D vector.
    #[must_use]
    pub fn dot_vec(&self, vec: DVec2) -> DVec3 {
        DVec3::new(
            vec.x * self.e11 + vec.y * self.e12,
            vec.x * self.e21 + vec.y * self.e22,
            vec.x * self.e31 + vec.y * self.e32,
        )
    }
}

/// Inertial position and velocity at a point in time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StateVectors {
    pub position: DVec3,
    pub velocity: DVec3,
}

impl StateVectors {
    #[inline]
    #[must_use]
    pub fn new(position: DVec3, velocity: DVec3) -> Self {
        Self { position, velocity }
    }

    /// Convert to a [`KeplerOrbit`] around a body with gravitational parameter `mu` at `epoch`.
    #[must_use]
    pub fn to_orbit(self, mu: f64, epoch: f64) -> KeplerOrbit {
        KeplerOrbit::from_state(self, mu, epoch)
    }

    /// Specific orbital energy: ε = v²/2 − μ/r.
    /// Negative → elliptic, zero → parabolic, positive → hyperbolic.
    #[must_use]
    pub fn specific_energy(self, mu: f64) -> f64 {
        let radius = self.position.length();
        self.velocity.length_squared() * 0.5 - mu / radius
    }

    /// Specific angular momentum vector **h** = **r** × **v**.
    #[must_use]
    pub fn angular_momentum(self) -> DVec3 {
        self.position.cross(self.velocity)
    }

    /// Eccentricity vector, pointing toward periapsis with magnitude equal to eccentricity.
    #[must_use]
    pub fn eccentricity_vector(self, mu: f64) -> DVec3 {
        let angular_momentum = self.angular_momentum();
        self.velocity.cross(angular_momentum) / mu - self.position / self.position.length()
    }

    /// Scalar eccentricity.
    #[must_use]
    pub fn eccentricity(self, mu: f64) -> f64 {
        self.eccentricity_vector(mu).length()
    }

    /// Radial (along-**r**) and tangential speed components.
    #[must_use]
    pub fn radial_and_tangential_speed(self) -> (f64, f64) {
        let radial_unit = self.position / self.position.length();
        let radial_speed = self.velocity.dot(radial_unit);
        let tangential_speed = (self.velocity.length_squared() - radial_speed * radial_speed)
            .max(0.0)
            .sqrt();
        (radial_speed, tangential_speed)
    }

    /// Returns `true` if neither position nor velocity contains NaN or infinity.
    #[must_use]
    pub fn is_finite(self) -> bool {
        self.position.is_finite() && self.velocity.is_finite()
    }

    /// Translate both vectors by `offset` (e.g. to switch reference frames).
    #[must_use]
    pub fn translated(self, offset_position: DVec3, offset_velocity: DVec3) -> Self {
        Self {
            position: self.position + offset_position,
            velocity: self.velocity + offset_velocity,
        }
    }

    /// Express relative to a parent body's own state vectors.
    #[must_use]
    pub fn relative_to(self, parent: Self) -> Self {
        Self {
            position: self.position - parent.position,
            velocity: self.velocity - parent.velocity,
        }
    }
}

impl std::fmt::Display for StateVectors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "r=[{:.3e}, {:.3e}, {:.3e}] v=[{:.3e}, {:.3e}, {:.3e}]",
            self.position.x,
            self.position.y,
            self.position.z,
            self.velocity.x,
            self.velocity.y,
            self.velocity.z,
        )
    }
}

impl From<(DVec3, DVec3)> for StateVectors {
    fn from((position, velocity): (DVec3, DVec3)) -> Self {
        Self { position, velocity }
    }
}

impl From<StateVectors> for (DVec3, DVec3) {
    fn from(state_vectors: StateVectors) -> (DVec3, DVec3) {
        (state_vectors.position, state_vectors.velocity)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct KeplerOrbit {
    pub standard_gravitational_parameter: f64,
    pub semi_major_axis: f64,
    pub eccentricity: f64,
    pub inclination: f64,
    pub longitude_of_ascending_node: f64,
    pub argument_of_periapsis: f64,
    pub mean_anomaly_at_epoch: f64,
    pub epoch_time: f64,
}

impl KeplerOrbit {
    /// Construct from inertial state vectors. `epoch` is the time those vectors are valid at.
    #[must_use]
    pub fn from_state(state: StateVectors, mu: f64, epoch: f64) -> Self {
        let (
            semi_major_axis,
            eccentricity,
            inclination,
            longitude_of_ascending_node,
            argument_of_periapsis,
            mean_anomaly_at_epoch,
        ) = cartesian_to_kepler(state, mu);
        Self {
            standard_gravitational_parameter: mu,
            semi_major_axis,
            eccentricity,
            inclination,
            longitude_of_ascending_node,
            argument_of_periapsis,
            mean_anomaly_at_epoch,
            epoch_time: epoch,
        }
    }

    /// Position and velocity in the reference frame of the primary body at `time`.
    #[must_use]
    pub fn state_at(&self, time: f64) -> StateVectors {
        kepler_to_cartesian(self, time)
    }

    /// Periapsis distance (always defined).
    #[must_use]
    pub fn periapsis_distance(&self) -> f64 {
        self.semi_major_axis * (1.0 - self.eccentricity)
    }

    /// Apoapsis distance — `None` for hyperbolic/parabolic orbits.
    #[must_use]
    pub fn apoapsis_distance(&self) -> Option<f64> {
        (self.eccentricity < 1.0).then_some(self.semi_major_axis * (1.0 + self.eccentricity))
    }

    /// Orbital period — `None` for hyperbolic/parabolic orbits.
    #[must_use]
    pub fn orbital_period(&self) -> Option<f64> {
        (self.eccentricity < 1.0).then(|| {
            TAU * (self.semi_major_axis.powi(3) / self.standard_gravitational_parameter).sqrt()
        })
    }

    /// Mean motion n = √(μ / |a|³), in rad/s.
    #[must_use]
    pub fn mean_motion(&self) -> f64 {
        (self.standard_gravitational_parameter / self.semi_major_axis.abs().powi(3)).sqrt()
    }

    /// Specific orbital energy ε = −μ / 2a  (elliptic → negative, hyperbolic → positive).
    #[must_use]
    pub fn specific_energy(&self) -> f64 {
        -self.standard_gravitational_parameter / (2.0 * self.semi_major_axis)
    }

    /// Speed at the given distance from the primary via the vis-viva equation.
    #[must_use]
    pub fn speed_at_distance(&self, radius: f64) -> f64 {
        let vis_viva_term = 2.0 / radius - 1.0 / self.semi_major_axis;
        (self.standard_gravitational_parameter * vis_viva_term)
            .max(0.0)
            .sqrt()
    }

    /// Whether this orbit will ever escape its primary (e ≥ 1).
    #[must_use]
    pub fn is_escape_trajectory(&self) -> bool {
        self.eccentricity >= 1.0
    }

    /// Re-epoch the orbit: same physical trajectory, new reference epoch.
    #[must_use]
    pub fn at_epoch(self, new_epoch: f64) -> Self {
        let mean_anomaly_delta = self.mean_motion() * (new_epoch - self.epoch_time);
        Self {
            mean_anomaly_at_epoch: self.mean_anomaly_at_epoch + mean_anomaly_delta,
            epoch_time: new_epoch,
            ..self
        }
    }
}

impl std::fmt::Display for KeplerOrbit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "KeplerOrbit {{ a={:.3e} m, e={:.6}, i={:.3}°, Ω={:.3}°, ω={:.3}°, M0={:.3}° }}",
            self.semi_major_axis,
            self.eccentricity,
            self.inclination.to_degrees(),
            self.longitude_of_ascending_node.to_degrees(),
            self.argument_of_periapsis.to_degrees(),
            self.mean_anomaly_at_epoch.to_degrees(),
        )
    }
}

fn cartesian_to_kepler(state: StateVectors, mu: f64) -> (f64, f64, f64, f64, f64, f64) {
    let position_vector = state.position;
    let velocity_vector = state.velocity;

    let radius = position_vector.length();
    let velocity_squared = velocity_vector.length_squared();
    let semi_major_axis = (2.0 / radius - velocity_squared / mu).recip();

    let angular_momentum_vector = position_vector.cross(velocity_vector);
    let angular_momentum_magnitude = angular_momentum_vector.length();

    let eccentricity_vector =
        velocity_vector.cross(angular_momentum_vector) / mu - position_vector / radius;
    let eccentricity = eccentricity_vector.length();

    let inclination = (angular_momentum_vector.z / angular_momentum_magnitude)
        .clamp(-1.0, 1.0)
        .acos();

    let ascending_node_vector =
        DVec3::new(-angular_momentum_vector.y, angular_momentum_vector.x, 0.0);
    let ascending_node_magnitude = ascending_node_vector.length();

    let longitude_of_ascending_node = if ascending_node_magnitude < 1e-10 {
        0.0
    } else {
        let cos_longitude_of_ascending_node =
            (ascending_node_vector.x / ascending_node_magnitude).clamp(-1.0, 1.0);
        if ascending_node_vector.y >= 0.0 {
            cos_longitude_of_ascending_node.acos()
        } else {
            TAU - cos_longitude_of_ascending_node.acos()
        }
    };

    let argument_of_periapsis = if ascending_node_magnitude < 1e-10 || eccentricity < 1e-10 {
        0.0
    } else {
        let cos_argument_of_periapsis = (ascending_node_vector.dot(eccentricity_vector)
            / (ascending_node_magnitude * eccentricity))
            .clamp(-1.0, 1.0);
        if eccentricity_vector.z >= 0.0 {
            cos_argument_of_periapsis.acos()
        } else {
            TAU - cos_argument_of_periapsis.acos()
        }
    };

    let true_anomaly = if eccentricity < 1e-10 {
        let cos_argument_of_latitude = (ascending_node_vector.dot(position_vector)
            / (ascending_node_magnitude * radius))
            .clamp(-1.0, 1.0);
        if position_vector.z >= 0.0 {
            cos_argument_of_latitude.acos()
        } else {
            TAU - cos_argument_of_latitude.acos()
        }
    } else {
        let cos_true_anomaly =
            (position_vector.dot(eccentricity_vector) / (radius * eccentricity)).clamp(-1.0, 1.0);
        if position_vector.dot(velocity_vector) >= 0.0 {
            cos_true_anomaly.acos()
        } else {
            TAU - cos_true_anomaly.acos()
        }
    };

    let mean_anomaly = if eccentricity < 1.0 {
        true_anomaly_to_mean_anomaly_elliptic(true_anomaly, eccentricity)
    } else {
        true_anomaly_to_mean_anomaly_hyperbolic(true_anomaly, eccentricity)
    };

    (
        semi_major_axis,
        eccentricity,
        inclination,
        longitude_of_ascending_node,
        argument_of_periapsis,
        mean_anomaly,
    )
}

fn kepler_to_cartesian(orbit: &KeplerOrbit, time: f64) -> StateVectors {
    let gravitational_parameter = orbit.standard_gravitational_parameter;
    let semi_major_axis = orbit.semi_major_axis;
    let eccentricity = orbit.eccentricity;

    let mean_anomaly =
        orbit.mean_anomaly_at_epoch + orbit.mean_motion() * (time - orbit.epoch_time);

    let (perifocal_position, perifocal_velocity) = if eccentricity < 1.0 {
        let eccentric_anomaly = mean_anomaly_to_eccentric_anomaly(mean_anomaly, eccentricity);
        let (sin_eccentric_anomaly, cos_eccentric_anomaly) = eccentric_anomaly.sin_cos();
        let eccentricity_radial_factor = (1.0 - eccentricity * eccentricity).sqrt();
        let radius = semi_major_axis * (1.0 - eccentricity * cos_eccentric_anomaly);
        let velocity_scale = (gravitational_parameter * semi_major_axis).sqrt() / radius;
        (
            DVec2::new(
                semi_major_axis * (cos_eccentric_anomaly - eccentricity),
                semi_major_axis * eccentricity_radial_factor * sin_eccentric_anomaly,
            ),
            DVec2::new(
                -velocity_scale * sin_eccentric_anomaly,
                velocity_scale * eccentricity_radial_factor * cos_eccentric_anomaly,
            ),
        )
    } else {
        let hyperbolic_anomaly = mean_anomaly_to_hyperbolic_anomaly(mean_anomaly, eccentricity);
        let sinh_hyperbolic_anomaly = hyperbolic_anomaly.sinh();
        let cosh_hyperbolic_anomaly = hyperbolic_anomaly.cosh();
        let eccentricity_radial_factor = (eccentricity * eccentricity - 1.0).sqrt();
        let radius = semi_major_axis * (1.0 - eccentricity * cosh_hyperbolic_anomaly);
        let velocity_scale = (gravitational_parameter * semi_major_axis.abs()).sqrt() / radius;
        (
            DVec2::new(
                semi_major_axis * (cosh_hyperbolic_anomaly - eccentricity),
                -semi_major_axis * eccentricity_radial_factor * sinh_hyperbolic_anomaly,
            ),
            DVec2::new(
                -velocity_scale * sinh_hyperbolic_anomaly,
                velocity_scale * eccentricity_radial_factor * cosh_hyperbolic_anomaly,
            ),
        )
    };

    rotate_to_inertial(
        perifocal_position,
        perifocal_velocity,
        orbit.inclination,
        orbit.longitude_of_ascending_node,
        orbit.argument_of_periapsis,
    )
}

#[inline]
fn rotate_to_inertial(
    perifocal_position: DVec2,
    perifocal_velocity: DVec2,
    inclination: f64,
    longitude_of_ascending_node: f64,
    argument_of_periapsis: f64,
) -> StateVectors {
    let (sin_inclination, cos_inclination) = inclination.sin_cos();
    let (sin_argument_of_periapsis, cos_argument_of_periapsis) = argument_of_periapsis.sin_cos();
    let (sin_longitude_of_ascending_node, cos_longitude_of_ascending_node) =
        longitude_of_ascending_node.sin_cos();

    let perifocal_to_inertial = Matrix3x2 {
        e11: cos_argument_of_periapsis * cos_longitude_of_ascending_node
            - sin_argument_of_periapsis * cos_inclination * sin_longitude_of_ascending_node,
        e21: cos_argument_of_periapsis * sin_longitude_of_ascending_node
            + sin_argument_of_periapsis * cos_inclination * cos_longitude_of_ascending_node,
        e31: sin_argument_of_periapsis * sin_inclination,
        e12: -(sin_argument_of_periapsis * cos_longitude_of_ascending_node
            + cos_argument_of_periapsis * cos_inclination * sin_longitude_of_ascending_node),
        e22: -(sin_argument_of_periapsis * sin_longitude_of_ascending_node
            - cos_argument_of_periapsis * cos_inclination * cos_longitude_of_ascending_node),
        e32: cos_argument_of_periapsis * sin_inclination,
    };

    StateVectors {
        position: perifocal_to_inertial.dot_vec(perifocal_position),
        velocity: perifocal_to_inertial.dot_vec(perifocal_velocity),
    }
}

#[derive(Debug, Clone)]
pub struct Body {
    pub name: &'static str,
    pub mass: f64,
    pub position: DVec3,
    pub velocity: DVec3,
    pub sphere_of_influence_radius: f64,
    pub parent_index: Option<usize>,
}

impl Body {
    #[must_use]
    pub fn mu(&self) -> f64 {
        GRAVITATIONAL_CONSTANT * self.mass
    }

    #[must_use]
    pub fn state_vectors(&self) -> StateVectors {
        StateVectors::new(self.position, self.velocity)
    }
}

fn bisection_search<F: Fn(f64) -> f64>(
    signed_distance: F,
    lower_bound: f64,
    upper_bound: f64,
) -> Option<f64> {
    const TOLERANCE: f64 = 0.5;
    const MAX_ITERATIONS: usize = 64;
    let (value_at_lower, value_at_upper) =
        (signed_distance(lower_bound), signed_distance(upper_bound));
    if value_at_lower * value_at_upper > 0.0 {
        return None;
    }
    let (mut low, mut high) = (lower_bound, upper_bound);
    let lower_sign = value_at_lower.signum();
    for _ in 0..MAX_ITERATIONS {
        if high - low < TOLERANCE {
            break;
        }
        let midpoint = 0.5 * (low + high);
        if signed_distance(midpoint).signum() == lower_sign {
            low = midpoint;
        } else {
            high = midpoint;
        }
    }
    Some(0.5 * (low + high))
}

#[derive(Debug, Clone)]
pub struct OrbitSegment {
    pub orbit: KeplerOrbit,
    pub primary_index: usize,
    pub start_time: f64,
    pub end_time: f64,
}

impl OrbitSegment {
    /// State in the inertial (solar-system) frame at `time`.
    #[must_use]
    pub fn inertial_state_at(&self, bodies: &[Body], time: f64) -> StateVectors {
        let relative_state = self.orbit.state_at(time);
        relative_state.translated(
            bodies[self.primary_index].position,
            bodies[self.primary_index].velocity,
        )
    }
}

#[derive(Debug, Default)]
pub struct PatchedTrajectory {
    pub segments: Vec<OrbitSegment>,
}

impl PatchedTrajectory {
    /// State in the inertial frame at `time`, using the correct segment.
    /// Returns `None` if `time` falls outside all segments.
    #[must_use]
    pub fn inertial_state_at(&self, bodies: &[Body], time: f64) -> Option<StateVectors> {
        let segment = self
            .segments
            .iter()
            .find(|segment| time >= segment.start_time && time <= segment.end_time)?;
        Some(segment.inertial_state_at(bodies, time))
    }

    /// The segment active at `time`, if any.
    #[must_use]
    pub fn segment_at(&self, time: f64) -> Option<&OrbitSegment> {
        self.segments
            .iter()
            .find(|segment| time >= segment.start_time && time <= segment.end_time)
    }

    /// Whether the trajectory is purely ballistic (no maneuver nodes).
    #[must_use]
    pub fn is_single_segment(&self) -> bool {
        self.segments.len() == 1
    }
}

/// Build a patched-conic trajectory by chilling sphere-of-influence transitions.
///
/// # Panics
/// Panics if `bodies` is empty or `start_primary_index` is out of bounds.
#[must_use]
pub fn build_trajectory(
    bodies: &[Body],
    initial_position: DVec3,
    initial_velocity: DVec3,
    start_time: f64,
    start_primary_index: usize,
    search_window: f64,
    max_patches: usize,
) -> PatchedTrajectory {
    let mut trajectory = PatchedTrajectory::default();
    let mut current_state = StateVectors::new(initial_position, initial_velocity);
    let mut current_primary = start_primary_index;
    let mut current_time = start_time;

    for _ in 0..=max_patches {
        let primary = &bodies[current_primary];
        let relative_state = current_state.relative_to(primary.state_vectors());
        let orbit = relative_state.to_orbit(primary.mu(), current_time);

        let start_state = orbit.state_at(current_time);
        if !start_state.is_finite() {
            break;
        }

        let search_end = current_time + search_window;
        let mut earliest_event: Option<(f64, usize)> = None;

        let on_outbound_trajectory =
            orbit.is_escape_trajectory() || start_state.position.dot(start_state.velocity) >= 0.0;
        if on_outbound_trajectory && primary.sphere_of_influence_radius.is_finite() {
            let sphere_of_influence_radius = primary.sphere_of_influence_radius;
            if let Some(transition_time) = bisection_search(
                |time| orbit.state_at(time).position.length() - sphere_of_influence_radius,
                current_time,
                search_end,
            ) {
                earliest_event = Some((
                    transition_time,
                    primary.parent_index.unwrap_or(current_primary),
                ));
            }
        }

        for (body_index, body) in bodies.iter().enumerate() {
            if body.parent_index != Some(current_primary) || body_index == current_primary {
                continue;
            }
            let child_relative_position = body.position - primary.position;
            let sphere_of_influence_radius = body.sphere_of_influence_radius;
            if let Some(transition_time) = bisection_search(
                |time| {
                    (orbit.state_at(time).position - child_relative_position).length()
                        - sphere_of_influence_radius
                },
                current_time,
                search_end,
            ) && earliest_event.is_none_or(|(earliest_transition_time, _)| {
                transition_time < earliest_transition_time
            }) {
                earliest_event = Some((transition_time, body_index));
            }
        }

        let (segment_end, next_primary) =
            earliest_event.unwrap_or((f64::INFINITY, current_primary));

        trajectory.segments.push(OrbitSegment {
            orbit,
            primary_index: current_primary,
            start_time: current_time,
            end_time: segment_end,
        });

        if segment_end.is_infinite() {
            break;
        }

        current_state = trajectory
            .segments
            .last()
            .expect("just pushed")
            .inertial_state_at(bodies, segment_end);
        current_time = segment_end;
        current_primary = next_primary;
    }

    trajectory
}
