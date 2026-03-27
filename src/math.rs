use bevy::math::DVec3;
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
    let mut target_eccentric_anomaly = if eccentricity > 0.8 {
        PI
    } else {
        mean_anomaly_normalized
    };
    let mut error = target_eccentric_anomaly
        - eccentricity * target_eccentric_anomaly.sin()
        - mean_anomaly_normalized;
    for _ in 0..100 {
        if error.abs() < 1e-13 {
            break;
        }
        target_eccentric_anomaly -= error / (1.0 - eccentricity * target_eccentric_anomaly.cos());
        error = target_eccentric_anomaly
            - eccentricity * target_eccentric_anomaly.sin()
            - mean_anomaly_normalized;
    }
    target_eccentric_anomaly
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
    let eccentricity_factor = ((eccentricity - 1.0) / (eccentricity + 1.0)).sqrt();
    2.0 * (eccentricity_factor * (true_anomaly / 2.0).tan()).atanh()
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
        let function_value =
            eccentricity * hyperbolic_anomaly.sinh() - hyperbolic_anomaly - mean_anomaly;
        let derivative_value = eccentricity * hyperbolic_anomaly.cosh() - 1.0;
        let delta_hyperbolic = function_value / derivative_value;
        hyperbolic_anomaly -= delta_hyperbolic;
        if delta_hyperbolic.abs() < 1e-13 {
            break;
        }
    }
    hyperbolic_anomaly
}

fn hyperbolic_anomaly_to_true_anomaly(hyperbolic_anomaly: f64, eccentricity: f64) -> f64 {
    let eccentricity_factor = ((eccentricity + 1.0) / (eccentricity - 1.0)).sqrt();
    2.0 * (eccentricity_factor * (hyperbolic_anomaly / 2.0).tanh()).atan()
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
    #[must_use]
    pub fn from_state(position: DVec3, velocity: DVec3, mu: f64, epoch: f64) -> Self {
        let (sma, ecc, inc, lan, aop, m0) = cartesian_to_kepler(position, velocity, mu);
        KeplerOrbit {
            standard_gravitational_parameter: mu,
            semi_major_axis: sma,
            eccentricity: ecc,
            inclination: inc,
            longitude_of_ascending_node: lan,
            argument_of_periapsis: aop,
            mean_anomaly_at_epoch: m0,
            epoch_time: epoch,
        }
    }

    #[must_use]
    pub fn state_at(&self, time: f64) -> (DVec3, DVec3) {
        kepler_to_cartesian(self, time)
    }

    #[must_use]
    pub fn periapsis_distance(&self) -> f64 {
        self.semi_major_axis * (1.0 - self.eccentricity)
    }

    #[must_use]
    pub fn apoapsis_distance(&self) -> Option<f64> {
        if self.eccentricity < 1.0 {
            Some(self.semi_major_axis * (1.0 + self.eccentricity))
        } else {
            None
        }
    }

    #[must_use]
    pub fn orbital_period(&self) -> Option<f64> {
        if self.eccentricity < 1.0 {
            Some(
                TAU * (self.semi_major_axis.powi(3) / self.standard_gravitational_parameter).sqrt(),
            )
        } else {
            None
        }
    }
}

fn cartesian_to_kepler(
    position_vector: DVec3,
    velocity_vector: DVec3,
    mu: f64,
) -> (f64, f64, f64, f64, f64, f64) {
    let radius_magnitude = position_vector.length();
    let velocity_squared = velocity_vector.length_squared();

    let semi_major_axis = (2.0 / radius_magnitude - velocity_squared / mu).recip();

    let angular_momentum_vector = position_vector.cross(velocity_vector);
    let angular_momentum_magnitude = angular_momentum_vector.length();

    let eccentricity_vector =
        velocity_vector.cross(angular_momentum_vector) / mu - position_vector / radius_magnitude;
    let eccentricity = eccentricity_vector.length();

    let inclination = (angular_momentum_vector.z / angular_momentum_magnitude)
        .clamp(-1.0, 1.0)
        .acos();

    let node_vector = DVec3::new(-angular_momentum_vector.y, angular_momentum_vector.x, 0.0);
    let node_magnitude = node_vector.length();

    let longitude_of_ascending_node = if node_magnitude < 1e-10 {
        0.0
    } else {
        let cosine_lan = (node_vector.x / node_magnitude).clamp(-1.0, 1.0);
        if node_vector.y >= 0.0 {
            cosine_lan.acos()
        } else {
            TAU - cosine_lan.acos()
        }
    };

    let argument_of_periapsis = if node_magnitude < 1e-10 || eccentricity < 1e-10 {
        0.0
    } else {
        let cosine_aop = (node_vector.dot(eccentricity_vector) / (node_magnitude * eccentricity))
            .clamp(-1.0, 1.0);
        if eccentricity_vector.z >= 0.0 {
            cosine_aop.acos()
        } else {
            TAU - cosine_aop.acos()
        }
    };

    let true_anomaly = if eccentricity < 1e-10 {
        let cosine_u = (node_vector.dot(position_vector) / (node_magnitude * radius_magnitude))
            .clamp(-1.0, 1.0);
        if position_vector.z >= 0.0 {
            cosine_u.acos()
        } else {
            TAU - cosine_u.acos()
        }
    } else {
        let cosine_nu = (position_vector.dot(eccentricity_vector)
            / (radius_magnitude * eccentricity))
            .clamp(-1.0, 1.0);
        if position_vector.dot(velocity_vector) >= 0.0 {
            cosine_nu.acos()
        } else {
            TAU - cosine_nu.acos()
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

fn kepler_to_cartesian(orbit: &KeplerOrbit, time: f64) -> (DVec3, DVec3) {
    let mu = orbit.standard_gravitational_parameter;
    let semi_major_axis = orbit.semi_major_axis;
    let eccentricity = orbit.eccentricity;

    let mean_motion = (mu / semi_major_axis.abs().powi(3)).sqrt();
    let mean_anomaly_current =
        orbit.mean_anomaly_at_epoch + mean_motion * (time - orbit.epoch_time);

    let (orbital_x, orbital_y, orbital_velocity_x, orbital_velocity_y) = if eccentricity < 1.0 {
        let eccentric_anomaly =
            mean_anomaly_to_eccentric_anomaly(mean_anomaly_current, eccentricity);
        let true_anomaly = eccentric_anomaly_to_true_anomaly(eccentric_anomaly, eccentricity);
        let radius = semi_major_axis * (1.0 - eccentricity * eccentric_anomaly.cos());
        let velocity_scale = (mu * semi_major_axis).sqrt() / radius;
        (
            radius * true_anomaly.cos(),
            radius * true_anomaly.sin(),
            -velocity_scale * eccentric_anomaly.sin(),
            velocity_scale * (1.0 - eccentricity * eccentricity).sqrt() * eccentric_anomaly.cos(),
        )
    } else {
        let hyperbolic_anomaly =
            mean_anomaly_to_hyperbolic_anomaly(mean_anomaly_current, eccentricity);
        let true_anomaly = hyperbolic_anomaly_to_true_anomaly(hyperbolic_anomaly, eccentricity);
        let radius = semi_major_axis * (1.0 - eccentricity * hyperbolic_anomaly.cosh());
        let velocity_scale = (mu * semi_major_axis.abs()).sqrt() / radius;
        (
            radius * true_anomaly.cos(),
            radius * true_anomaly.sin(),
            -velocity_scale * hyperbolic_anomaly.sinh(),
            velocity_scale * (eccentricity * eccentricity - 1.0).sqrt() * hyperbolic_anomaly.cosh(),
        )
    };

    rotate_to_inertial(
        orbital_x,
        orbital_y,
        orbital_velocity_x,
        orbital_velocity_y,
        orbit.inclination,
        orbit.longitude_of_ascending_node,
        orbit.argument_of_periapsis,
    )
}

#[inline]
fn rotate_to_inertial(
    orbital_x: f64,
    orbital_y: f64,
    velocity_x: f64,
    velocity_y: f64,
    inclination: f64,
    lan: f64,
    aop: f64,
) -> (DVec3, DVec3) {
    let (sin_inclination, cos_inclination) = inclination.sin_cos();
    let (sin_aop, cos_aop) = aop.sin_cos();
    let (sin_lan, cos_lan) = lan.sin_cos();

    let column_x = DVec3::new(
        cos_aop * cos_lan - sin_aop * cos_inclination * sin_lan,
        cos_aop * sin_lan + sin_aop * cos_inclination * cos_lan,
        sin_aop * sin_inclination,
    );
    let column_y = DVec3::new(
        -(sin_aop * cos_lan + cos_aop * cos_inclination * sin_lan),
        -(sin_aop * sin_lan - cos_aop * cos_inclination * cos_lan),
        cos_aop * sin_inclination,
    );

    (
        column_x * orbital_x + column_y * orbital_y,
        column_x * velocity_x + column_y * velocity_y,
    )
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
}

fn bisection_search<F: Fn(f64) -> f64>(function: F, time_low: f64, time_high: f64) -> Option<f64> {
    const TOLERANCE: f64 = 0.5;
    const MAX_ITERATIONS: usize = 64;
    let (value_at_low, value_at_high) = (function(time_low), function(time_high));
    if value_at_low * value_at_high > 0.0 {
        return None;
    }
    let (mut low, mut high) = (time_low, time_high);
    let low_sign = value_at_low.signum();
    for _ in 0..MAX_ITERATIONS {
        if high - low < TOLERANCE {
            return Some(0.5 * (low + high));
        }
        let middle = 0.5 * (low + high);
        if function(middle).signum() == low_sign {
            low = middle;
        } else {
            high = middle;
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
    #[must_use]
    pub fn inertial_state_at(&self, bodies: &[Body], time: f64) -> (DVec3, DVec3) {
        let (relative_position, relative_velocity) = self.orbit.state_at(time);
        let primary_body = &bodies[self.primary_index];
        (
            relative_position + primary_body.position,
            relative_velocity + primary_body.velocity,
        )
    }
}

#[derive(Debug, Default)]
pub struct PatchedTrajectory {
    pub segments: Vec<OrbitSegment>,
}

/// # Panics
/// Panics if the trajectory segments list is empty after a push or if an index is out of bounds.
#[must_use]
pub fn build_trajectory(
    bodies: &[Body],
    initial_position: DVec3,
    initial_velocity: DVec3,
    start_time_input: f64,
    start_primary_index: usize,
    search_window: f64,
    max_patches: usize,
) -> PatchedTrajectory {
    let mut trajectory = PatchedTrajectory::default();
    let mut current_position = initial_position;
    let mut current_velocity = initial_velocity;
    let mut current_primary_index = start_primary_index;
    let mut current_time = start_time_input;

    for _ in 0..=max_patches {
        let primary_body = &bodies[current_primary_index];
        let orbit = KeplerOrbit::from_state(
            current_position - primary_body.position,
            current_velocity - primary_body.velocity,
            primary_body.mu(),
            current_time,
        );

        let (position_start, velocity_start) = orbit.state_at(current_time);
        if !position_start.is_finite() {
            break;
        }

        let end_search_time = current_time + search_window;
        let mut earliest_event: Option<(f64, usize)> = None;

        let will_exit_sphere =
            orbit.eccentricity >= 1.0 || position_start.dot(velocity_start) >= 0.0;
        if will_exit_sphere && primary_body.sphere_of_influence_radius.is_finite() {
            let sphere_radius = primary_body.sphere_of_influence_radius;
            if let Some(exit_time) = bisection_search(
                |time| orbit.state_at(time).0.length() - sphere_radius,
                current_time,
                end_search_time,
            ) {
                earliest_event = Some((
                    exit_time,
                    primary_body.parent_index.unwrap_or(current_primary_index),
                ));
            }
        }

        for (index, body) in bodies.iter().enumerate() {
            if body.parent_index != Some(current_primary_index) || index == current_primary_index {
                continue;
            }
            let child_relative_position = body.position - primary_body.position;
            let sphere_radius = body.sphere_of_influence_radius;

            if let Some(entry_time) = bisection_search(
                |time| (orbit.state_at(time).0 - child_relative_position).length() - sphere_radius,
                current_time,
                end_search_time,
            ) && (earliest_event.is_none() || entry_time < earliest_event.expect("Safe").0)
            {
                earliest_event = Some((entry_time, index));
            }
        }

        let (segment_end, next_primary_index) =
            earliest_event.unwrap_or((f64::INFINITY, current_primary_index));

        trajectory.segments.push(OrbitSegment {
            orbit,
            primary_index: current_primary_index,
            start_time: current_time,
            end_time: segment_end,
        });

        if segment_end.is_infinite() {
            break;
        }

        let last_segment = trajectory.segments.last().expect("Must exist");
        let (next_pos, next_vel) = last_segment.inertial_state_at(bodies, segment_end);
        current_position = next_pos;
        current_velocity = next_vel;
        current_time = segment_end;
        current_primary_index = next_primary_index;
    }

    trajectory
}
