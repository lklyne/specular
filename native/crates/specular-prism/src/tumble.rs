//! How the prism tumbles while its agent works: a damped spring pulls its
//! turn toward a target, and every second or two a burst moves the target
//! and kicks it, so it snaps round with an overshoot and settles.
//!
//! Every target is a turn that maps the cube onto itself (a quarter or half
//! turn about a face's axis, a third of a turn about a corner's), so
//! wherever it comes to rest it looks like the still avatar, and stopping
//! never needs a jump back.
//!
//! The motion steps at a fixed rate, so it is the same at any frame rate,
//! and the bursts come from a generator seeded by the thread, so each
//! thread tumbles its own way, the same way every time.

use std::f32::consts::{FRAC_PI_2, PI};
use std::time::Duration;

use crate::params::Pose;

/// One step of the motion.
const STEP: Duration = Duration::from_nanos(1_000_000_000 / 240);

/// The turn's spring: its natural frequency (rad/s) and damping ratio.
/// Under-damped enough to overshoot once and visibly settle in about a
/// second.
const TURN_FREQUENCY: f32 = 2.0 * PI * 1.1;
const TURN_DAMPING: f32 = 0.4;

/// The scale's spring, quicker than the turn's, so the pop is over before
/// the turn settles.
const SCALE_FREQUENCY: f32 = 2.0 * PI * 2.4;
const SCALE_DAMPING: f32 = 0.35;

/// Seconds from one burst to the next, drawn evenly between these.
const BURST_GAP: (f32, f32) = (0.9, 2.0);
/// Seconds from the agent starting to the first burst.
const FIRST_BURST: (f32, f32) = (0.1, 0.35);
/// The angular speed (rad/s) a burst adds about a random axis, on top of
/// the spring's pull toward the new target: the wobble off the target's
/// own axis.
const KICK: (f32, f32) = (2.0, 4.5);
/// The scale's speed (per second) a burst adds: a pop of about 5%.
const POP: (f32, f32) = (0.9, 1.3);

/// Below these it counts as at rest.
const REST_ANGLE: f32 = 0.002;
const REST_SPEED: f32 = 0.01;
const REST_SCALE: f32 = 0.001;

/// One avatar's tumble.
#[derive(Debug, Clone)]
pub struct Tumble {
    turn: Quat,
    /// Angular velocity, rad/s, about world axes.
    spin: [f32; 3],
    target: Quat,
    scale: f32,
    /// d(scale)/dt.
    grow: f32,
    /// Seconds of work left before the next burst.
    until_burst: f32,
    was_working: bool,
    /// Whether it has settled exactly on its target.
    resting: bool,
    /// Time not yet stepped.
    pending: Duration,
    random: Random,
}

impl Tumble {
    /// At rest, with bursts seeded by `seed`.
    pub fn new(seed: u64) -> Self {
        Self {
            turn: Quat::IDENTITY,
            spin: [0.0; 3],
            target: Quat::IDENTITY,
            scale: 1.0,
            grow: 0.0,
            until_burst: 0.0,
            was_working: false,
            resting: true,
            pending: Duration::ZERO,
            random: Random(seed),
        }
    }

    /// Moves it on by `elapsed`, bursting only while `working`.
    pub fn advance(&mut self, elapsed: Duration, working: bool) {
        self.pending += elapsed;
        while self.pending >= STEP {
            self.pending -= STEP;
            self.step(working);
        }
    }

    /// The pose to draw.
    pub fn pose(&self) -> Pose {
        Pose {
            rotation: self.turn.0,
            scale: self.scale,
        }
    }

    /// Whether it has come to rest, so drawing it again would show the
    /// same picture.
    pub fn at_rest(&self) -> bool {
        self.resting
    }

    /// One fixed step. Returns whether a burst started.
    fn step(&mut self, working: bool) -> bool {
        let dt = STEP.as_secs_f32();
        let mut burst = false;
        if working {
            if !self.was_working {
                self.until_burst = self.random.between(FIRST_BURST);
            }
            self.until_burst -= dt;
            if self.until_burst <= 0.0 {
                self.burst();
                burst = true;
            }
        }
        self.was_working = working;
        if self.resting {
            return burst;
        }

        let error = self.target.mul(self.turn.conjugate()).log();
        let (k, c) = spring(TURN_FREQUENCY, TURN_DAMPING);
        for (spin, error) in self.spin.iter_mut().zip(error) {
            *spin += (k * error - c * *spin) * dt;
        }
        self.turn = Quat::exp(self.spin.map(|w| w * dt))
            .mul(self.turn)
            .normalized();

        let (k, c) = spring(SCALE_FREQUENCY, SCALE_DAMPING);
        self.grow += (k * (1.0 - self.scale) - c * self.grow) * dt;
        self.scale += self.grow * dt;

        if !working && self.settling_done() {
            self.turn = self.target;
            self.spin = [0.0; 3];
            self.scale = 1.0;
            self.grow = 0.0;
            self.resting = true;
        }
        burst
    }

    fn settling_done(&self) -> bool {
        let angle = length(self.target.mul(self.turn.conjugate()).log());
        angle < REST_ANGLE
            && length(self.spin) < REST_SPEED
            && (self.scale - 1.0).abs() < REST_SCALE
            && self.grow.abs() < REST_SPEED
    }

    /// Moves the target by one of the cube's own turns and kicks it.
    fn burst(&mut self) {
        let symmetry = SYMMETRIES[self.random.below(SYMMETRIES.len())];
        let axis = self.random.direction();
        // The turn is about a screen axis (x tips it, z spins it in place),
        // and the cube at rest is square to the screen, so the new target
        // is still one of the cube's own turns.
        self.target = Quat::about(axis_of(symmetry.0, &mut self.random), symmetry.1)
            .mul(self.target)
            .normalized();
        let kick = self.random.between(KICK);
        for (spin, a) in self.spin.iter_mut().zip(axis) {
            *spin += a * kick;
        }
        self.grow += self.random.between(POP);
        self.until_burst = self.random.between(BURST_GAP);
        self.resting = false;
    }
}

/// The spring's stiffness and damping for a natural frequency and a
/// damping ratio.
fn spring(frequency: f32, damping: f32) -> (f32, f32) {
    (frequency * frequency, 2.0 * damping * frequency)
}

/// The kinds of turn that map a cube onto itself.
#[derive(Debug, Clone, Copy)]
enum Axis {
    /// Through the middle of two opposite faces.
    Face,
    /// Through two opposite corners.
    Corner,
}

/// The turns a burst picks from, with how far they go. Quarter turns come
/// twice, as they read best.
const SYMMETRIES: [(Axis, f32); 5] = [
    (Axis::Face, FRAC_PI_2),
    (Axis::Face, FRAC_PI_2),
    (Axis::Face, -FRAC_PI_2),
    (Axis::Face, PI),
    (Axis::Corner, 2.0 * PI / 3.0),
];

/// A random axis of `kind`, as the cube sits at rest.
fn axis_of(kind: Axis, random: &mut Random) -> [f32; 3] {
    match kind {
        Axis::Face => {
            let mut axis = [0.0; 3];
            axis[random.below(3)] = 1.0;
            axis
        }
        Axis::Corner => {
            let sign = |random: &mut Random| if random.below(2) == 0 { 1.0 } else { -1.0 };
            let d = 1.0 / 3.0_f32.sqrt();
            [sign(random) * d, sign(random) * d, sign(random) * d]
        }
    }
}

fn length(v: [f32; 3]) -> f32 {
    v.iter().map(|x| x * x).sum::<f32>().sqrt()
}

/// A unit quaternion `[x, y, z, w]`.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Quat([f32; 4]);

impl Quat {
    const IDENTITY: Self = Self([0.0, 0.0, 0.0, 1.0]);

    /// A turn of `angle` radians about the unit `axis`.
    fn about(axis: [f32; 3], angle: f32) -> Self {
        let (s, c) = (angle / 2.0).sin_cos();
        Self([axis[0] * s, axis[1] * s, axis[2] * s, c])
    }

    /// The turn of the rotation vector `v`: about `v`, by `|v|` radians.
    fn exp(v: [f32; 3]) -> Self {
        let angle = length(v);
        if angle < 1e-9 {
            return Self::IDENTITY;
        }
        Self::about(v.map(|x| x / angle), angle)
    }

    /// The rotation vector of the shorter way round to this turn.
    #[expect(clippy::many_single_char_names, reason = "the quaternion's own names")]
    fn log(self) -> [f32; 3] {
        let [x, y, z, w] = self.0;
        let (v, w) = if w < 0.0 {
            ([-x, -y, -z], -w)
        } else {
            ([x, y, z], w)
        };
        let s = length(v);
        if s < 1e-9 {
            return [0.0; 3];
        }
        let angle = 2.0 * s.atan2(w);
        v.map(|x| x / s * angle)
    }

    /// `self` after `other`.
    fn mul(self, other: Self) -> Self {
        let [ax, ay, az, aw] = self.0;
        let [bx, by, bz, bw] = other.0;
        Self([
            aw * bx + ax * bw + ay * bz - az * by,
            aw * by - ax * bz + ay * bw + az * bx,
            aw * bz + ax * by - ay * bx + az * bw,
            aw * bw - ax * bx - ay * by - az * bz,
        ])
    }

    fn conjugate(self) -> Self {
        let [x, y, z, w] = self.0;
        Self([-x, -y, -z, w])
    }

    fn normalized(self) -> Self {
        let n = self.0.iter().map(|x| x * x).sum::<f32>().sqrt();
        Self(self.0.map(|x| x / n))
    }
}

/// `SplitMix64`: small, seedable and good enough for picking bursts.
#[derive(Debug, Clone)]
struct Random(u64);

impl Random {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// Evenly in `0..1`.
    fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }

    fn between(&mut self, (low, high): (f32, f32)) -> f32 {
        low + (high - low) * self.unit()
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    /// A unit vector, evenly over the sphere.
    fn direction(&mut self) -> [f32; 3] {
        let z = self.between((-1.0, 1.0));
        let angle = self.between((0.0, 2.0 * PI));
        let r = (1.0 - z * z).max(0.0).sqrt();
        [r * angle.cos(), r * angle.sin(), z]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(tumble: &mut Tumble, seconds: f32, frame: Duration, working: bool) {
        let mut left = Duration::from_secs_f32(seconds);
        while left >= frame {
            tumble.advance(frame, working);
            left -= frame;
        }
    }

    #[test]
    fn the_motion_is_the_same_at_any_frame_rate() {
        let (mut at_30, mut at_60, mut at_120) = (Tumble::new(7), Tumble::new(7), Tumble::new(7));
        run(&mut at_30, 4.0, Duration::from_micros(33_333), true);
        run(&mut at_60, 4.0, Duration::from_micros(16_666), true);
        run(&mut at_120, 4.0, Duration::from_micros(8_333), true);
        // The three runs stop a few hundred microseconds apart, inside one
        // step; what was stepped is identical.
        let poses = [at_30.pose(), at_60.pose(), at_120.pose()];
        assert_eq!(poses[0], poses[1]);
        assert_eq!(poses[1], poses[2]);
    }

    #[test]
    fn bursts_come_every_one_to_two_seconds_and_differ_by_seed() {
        let bursts = |seed| {
            let mut tumble = Tumble::new(seed);
            let mut at = Vec::new();
            for step in 0..240 * 20 {
                if tumble.step(true) {
                    at.push(step as f32 / 240.0);
                }
            }
            at
        };
        let a = bursts(1);
        assert_eq!(a, bursts(1), "a seed always gives the same bursts");
        assert_ne!(a, bursts(2), "another seed gives others");
        assert!(a[0] <= FIRST_BURST.1 + 0.01, "it starts at once: {a:?}");
        for gap in a.windows(2).map(|w| w[1] - w[0]) {
            assert!((0.89..=2.01).contains(&gap), "gap {gap} in {a:?}");
        }
    }

    #[test]
    fn it_moves_while_working_and_comes_to_rest_looking_still_after() {
        let mut tumble = Tumble::new(42);
        let mut widest = 0.0_f32;
        let mut largest = 1.0_f32;
        for _ in 0..240 * 6 {
            tumble.step(true);
            widest = widest.max(length(tumble.turn.log()));
            largest = largest.max(tumble.scale);
        }
        assert!(widest > 1.0, "it turns well away from rest: {widest}");
        assert!(largest < 1.08, "the pop stays small: {largest}");

        run(&mut tumble, 4.0, Duration::from_micros(16_666), false);
        assert!(tumble.at_rest());
        // At rest it is turned by one of the cube's own turns: every axis
        // lands on an axis.
        let m = tumble.pose().matrix();
        for value in m.iter().flatten() {
            let off = value.abs().min((value.abs() - 1.0).abs());
            assert!(off < 1e-3, "{m:?}");
        }
        assert_eq!(tumble.pose().scale, 1.0);
    }
}
