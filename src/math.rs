#![expect(unused)]

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
    let mut e = if eccentricity > 0.8 {
        PI
    } else {
        mean_anomaly_normalized
    };
    for _ in 0..100 {
        let error = e - eccentricity * e.sin() - mean_anomaly_normalized;
        if error.abs() < 1e-13 {
            break;
        }
        e -= error / (1.0 - eccentricity * e.cos());
    }
    e
}

fn eccentric_anomaly_to_true_anomaly(eccentric_anomaly: f64, eccentricity: f64) -> f64 {
    let half = eccentric_anomaly * 0.5;
    2.0 * f64::atan2(
        (1.0 + eccentricity).sqrt() * half.sin(),
        (1.0 - eccentricity).sqrt() * half.cos(),
    )
}

fn eccentric_anomaly_to_mean_anomaly(eccentric_anomaly: f64, eccentricity: f64) -> f64 {
    eccentric_anomaly - eccentricity * eccentric_anomaly.sin()
}

fn true_anomaly_to_mean_anomaly_elliptic(true_anomaly: f64, eccentricity: f64) -> f64 {
    let e = true_anomaly_to_eccentric_anomaly(true_anomaly, eccentricity);
    eccentric_anomaly_to_mean_anomaly(e, eccentricity)
}

fn true_anomaly_to_hyperbolic_anomaly(true_anomaly: f64, eccentricity: f64) -> f64 {
    let factor = ((eccentricity - 1.0) / (eccentricity + 1.0)).sqrt();
    2.0 * (factor * (true_anomaly / 2.0).tan()).atanh()
}

fn hyperbolic_anomaly_to_mean_anomaly(hyperbolic_anomaly: f64, eccentricity: f64) -> f64 {
    eccentricity * hyperbolic_anomaly.sinh() - hyperbolic_anomaly
}

fn true_anomaly_to_mean_anomaly_hyperbolic(true_anomaly: f64, eccentricity: f64) -> f64 {
    let h = true_anomaly_to_hyperbolic_anomaly(true_anomaly, eccentricity);
    hyperbolic_anomaly_to_mean_anomaly(h, eccentricity)
}

fn mean_anomaly_to_hyperbolic_anomaly(mean_anomaly: f64, eccentricity: f64) -> f64 {
    let mut h = mean_anomaly.signum() * (2.0 * mean_anomaly.abs() / eccentricity + 1.8).ln();
    for _ in 0..100 {
        let f = eccentricity * h.sinh() - h - mean_anomaly;
        let df = eccentricity * h.cosh() - 1.0;
        let delta = f / df;
        h -= delta;
        if delta.abs() < 1e-13 {
            break;
        }
    }
    h
}

fn hyperbolic_anomaly_to_true_anomaly(hyperbolic_anomaly: f64, eccentricity: f64) -> f64 {
    let factor = ((eccentricity + 1.0) / (eccentricity - 1.0)).sqrt();
    2.0 * (factor * (hyperbolic_anomaly / 2.0).tanh()).atan()
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
        let r = self.position.length();
        self.velocity.length_squared() * 0.5 - mu / r
    }

    /// Specific angular momentum vector **h** = **r** × **v**.
    #[must_use]
    pub fn angular_momentum(self) -> DVec3 {
        self.position.cross(self.velocity)
    }

    /// Eccentricity vector, pointing toward periapsis with magnitude equal to eccentricity.
    #[must_use]
    pub fn eccentricity_vector(self, mu: f64) -> DVec3 {
        let h = self.angular_momentum();
        self.velocity.cross(h) / mu - self.position / self.position.length()
    }

    /// Scalar eccentricity.
    #[must_use]
    pub fn eccentricity(self, mu: f64) -> f64 {
        self.eccentricity_vector(mu).length()
    }

    /// Radial (along-**r**) and tangential speed components.
    #[must_use]
    pub fn radial_and_tangential_speed(self) -> (f64, f64) {
        let r_hat = self.position / self.position.length();
        let v_r = self.velocity.dot(r_hat);
        let v_t = (self.velocity.length_squared() - v_r * v_r).max(0.0).sqrt();
        (v_r, v_t)
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
    fn from(sv: StateVectors) -> (DVec3, DVec3) {
        (sv.position, sv.velocity)
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
        let (sma, ecc, inc, lan, aop, m0) = cartesian_to_kepler(state, mu);
        Self {
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
        let term = 2.0 / radius - 1.0 / self.semi_major_axis;
        (self.standard_gravitational_parameter * term)
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
        let delta_m = self.mean_motion() * (new_epoch - self.epoch_time);
        Self {
            mean_anomaly_at_epoch: self.mean_anomaly_at_epoch + delta_m,
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
    let StateVectors {
        position: r_vec,
        velocity: v_vec,
    } = state;

    let r = r_vec.length();
    let v_sq = v_vec.length_squared();
    let semi_major_axis = (2.0 / r - v_sq / mu).recip();

    let h_vec = r_vec.cross(v_vec);
    let h = h_vec.length();

    let ecc_vec = v_vec.cross(h_vec) / mu - r_vec / r;
    let eccentricity = ecc_vec.length();

    let inclination = (h_vec.z / h).clamp(-1.0, 1.0).acos();

    let node_vec = DVec3::new(-h_vec.y, h_vec.x, 0.0);
    let node_mag = node_vec.length();

    let longitude_of_ascending_node = if node_mag < 1e-10 {
        0.0
    } else {
        let cos_lan = (node_vec.x / node_mag).clamp(-1.0, 1.0);
        if node_vec.y >= 0.0 {
            cos_lan.acos()
        } else {
            TAU - cos_lan.acos()
        }
    };

    let argument_of_periapsis = if node_mag < 1e-10 || eccentricity < 1e-10 {
        0.0
    } else {
        let cos_aop = (node_vec.dot(ecc_vec) / (node_mag * eccentricity)).clamp(-1.0, 1.0);
        if ecc_vec.z >= 0.0 {
            cos_aop.acos()
        } else {
            TAU - cos_aop.acos()
        }
    };

    let true_anomaly = if eccentricity < 1e-10 {
        let cos_u = (node_vec.dot(r_vec) / (node_mag * r)).clamp(-1.0, 1.0);
        if r_vec.z >= 0.0 {
            cos_u.acos()
        } else {
            TAU - cos_u.acos()
        }
    } else {
        let cos_nu = (r_vec.dot(ecc_vec) / (r * eccentricity)).clamp(-1.0, 1.0);
        if r_vec.dot(v_vec) >= 0.0 {
            cos_nu.acos()
        } else {
            TAU - cos_nu.acos()
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
    let mu = orbit.standard_gravitational_parameter;
    let a = orbit.semi_major_axis;
    let ecc = orbit.eccentricity;

    let mean_anomaly =
        orbit.mean_anomaly_at_epoch + orbit.mean_motion() * (time - orbit.epoch_time);

    use bevy::math::DVec2;
    let (orbital_pos, orbital_vel) = if ecc < 1.0 {
        let e_anom = mean_anomaly_to_eccentric_anomaly(mean_anomaly, ecc);
        let (sin_e, cos_e) = e_anom.sin_cos();
        let ecc_factor = (1.0 - ecc * ecc).sqrt();
        let r = a * (1.0 - ecc * cos_e);
        let v_scale = (mu * a).sqrt() / r;
        (
            DVec2::new(a * (cos_e - ecc), a * ecc_factor * sin_e),
            DVec2::new(-v_scale * sin_e, v_scale * ecc_factor * cos_e),
        )
    } else {
        let h_anom = mean_anomaly_to_hyperbolic_anomaly(mean_anomaly, ecc);
        let sinh_h = h_anom.sinh();
        let cosh_h = h_anom.cosh();
        let ecc_factor = (ecc * ecc - 1.0).sqrt();
        let r = a * (1.0 - ecc * cosh_h);
        let v_scale = (mu * a.abs()).sqrt() / r;
        (
            DVec2::new(a * (cosh_h - ecc), -a * ecc_factor * sinh_h),
            DVec2::new(-v_scale * sinh_h, v_scale * ecc_factor * cosh_h),
        )
    };

    rotate_to_inertial(
        orbital_pos,
        orbital_vel,
        orbit.inclination,
        orbit.longitude_of_ascending_node,
        orbit.argument_of_periapsis,
    )
}

#[inline]
fn rotate_to_inertial(
    orbital_pos: bevy::math::DVec2,
    orbital_vel: bevy::math::DVec2,
    inclination: f64,
    lan: f64,
    aop: f64,
) -> StateVectors {
    let (sin_inc, cos_inc) = inclination.sin_cos();
    let (sin_aop, cos_aop) = aop.sin_cos();
    let (sin_lan, cos_lan) = lan.sin_cos();

    let col_x = DVec3::new(
        cos_aop * cos_lan - sin_aop * cos_inc * sin_lan,
        cos_aop * sin_lan + sin_aop * cos_inc * cos_lan,
        sin_aop * sin_inc,
    );
    let col_y = DVec3::new(
        -(sin_aop * cos_lan + cos_aop * cos_inc * sin_lan),
        -(sin_aop * sin_lan - cos_aop * cos_inc * cos_lan),
        cos_aop * sin_inc,
    );

    StateVectors {
        position: col_x * orbital_pos.x + col_y * orbital_pos.y,
        velocity: col_x * orbital_vel.x + col_y * orbital_vel.y,
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

// ─── Patched conics ───────────────────────────────────────────────────────────

fn bisection_search<F: Fn(f64) -> f64>(f: F, t_low: f64, t_high: f64) -> Option<f64> {
    const TOLERANCE: f64 = 0.5;
    const MAX_ITERATIONS: usize = 64;
    let (f_low, f_high) = (f(t_low), f(t_high));
    if f_low * f_high > 0.0 {
        return None;
    }
    let (mut low, mut high) = (t_low, t_high);
    let low_sign = f_low.signum();
    for _ in 0..MAX_ITERATIONS {
        if high - low < TOLERANCE {
            break;
        }
        let mid = 0.5 * (low + high);
        if f(mid).signum() == low_sign {
            low = mid;
        } else {
            high = mid;
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
        let relative = self.orbit.state_at(time);
        relative.translated(
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
            .find(|s| time >= s.start_time && time <= s.end_time)?;
        Some(segment.inertial_state_at(bodies, time))
    }

    /// The segment active at `time`, if any.
    #[must_use]
    pub fn segment_at(&self, time: f64) -> Option<&OrbitSegment> {
        self.segments
            .iter()
            .find(|s| time >= s.start_time && time <= s.end_time)
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

        let start_sv = orbit.state_at(current_time);
        if !start_sv.is_finite() {
            break;
        }

        let search_end = current_time + search_window;
        let mut earliest_event: Option<(f64, usize)> = None;

        // Check for SOI exit
        let will_exit =
            orbit.is_escape_trajectory() || start_sv.position.dot(start_sv.velocity) >= 0.0;
        if will_exit && primary.sphere_of_influence_radius.is_finite() {
            let soi_r = primary.sphere_of_influence_radius;
            if let Some(t) = bisection_search(
                |t| orbit.state_at(t).position.length() - soi_r,
                current_time,
                search_end,
            ) {
                earliest_event = Some((t, primary.parent_index.unwrap_or(current_primary)));
            }
        }

        // Check for SOI entry into a child body
        for (index, body) in bodies.iter().enumerate() {
            if body.parent_index != Some(current_primary) || index == current_primary {
                continue;
            }
            let child_rel_pos = body.position - primary.position;
            let soi_r = body.sphere_of_influence_radius;
            if let Some(t) = bisection_search(
                |t| (orbit.state_at(t).position - child_rel_pos).length() - soi_r,
                current_time,
                search_end,
            ) && earliest_event.is_none_or(|(et, _)| t < et)
            {
                earliest_event = Some((t, index));
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
