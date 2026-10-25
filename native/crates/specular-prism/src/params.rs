//! The Prism preset: every value the web build feeds its shaders, by name.
//!
//! Defaults are the values three uploaded for the Prism tab with every
//! control at its default (the capture's `uniforms/f0240.json`, ADR 0047).
//! Colours are linear, as the shaders take them.

use std::f32::consts::PI;

/// Every input of a Prism frame except the time.
#[derive(Debug, Clone, PartialEq)]
pub struct PrismParams {
    /// The mesh-gradient card inside the shells.
    pub(crate) gradient: Gradient,
    /// The inner shell, a tinted liquid.
    pub(crate) liquid: Shell,
    /// The outer shell, clear glass.
    pub(crate) glass: Shell,
    /// The glass's emissive rim.
    pub(crate) rim: Rim,
    /// The glass's stylised specular highlight.
    pub(crate) highlight: Highlight,
    /// The key light and the ambient term.
    pub(crate) light: Light,
    /// The baked environment's strength and turn.
    pub(crate) environment: Environment,
    /// The colour grade of the display pass.
    pub(crate) grade: Grade,
    /// Where the composite cuts the shape out of the scene's alpha.
    pub(crate) coverage: Coverage,
    /// How much of the world the frame shows.
    pub camera: Camera,
    /// How the shells are turned and scaled about their centre.
    pub pose: Pose,
}

/// The gradient card.
#[derive(Debug, Clone, PartialEq)]
pub struct Gradient {
    /// The blob colours, linear RGB. The shader takes up to eight.
    pub(crate) colors: Vec<[f32; 3]>,
    /// Gradient seconds per second of time.
    pub(crate) speed: f32,
    /// How far the domain warp pushes the blobs.
    pub(crate) distortion: f32,
    /// The swirl around the card's centre.
    pub(crate) swirl: f32,
    /// The inverse-distance power each blob falls off with.
    pub(crate) falloff: f32,
    /// The speckle cell size, which also sets the grain's scale.
    pub(crate) speckle_size: f32,
    /// The seed of the grain and speckle hashes.
    pub(crate) grain_seed: f32,
    /// The share of cells that get a speckle.
    pub(crate) speckle_amount: f32,
    /// Posterize steps per channel.
    pub(crate) posterize_levels: f32,
    /// How much of the posterized colour is mixed in.
    pub(crate) posterize_amount: f32,
    /// The card's edge length in world units (the camera sees 1.4).
    pub(crate) size: f32,
    /// The superellipse exponent of the card's outline.
    pub(crate) squircleness: f32,
}

/// A transmission shell: a rounded box, refracting what is drawn behind it.
#[derive(Debug, Clone, PartialEq)]
pub struct Shell {
    /// Uniform scale of the unit shell (edge 1.2 world units).
    pub(crate) scale: f32,
    /// Base colour.
    pub(crate) color: [f32; 3],
    /// Surface roughness.
    pub(crate) roughness: f32,
    /// Index of refraction.
    pub(crate) ior: f32,
    /// Thickness for the refraction offset.
    pub(crate) thickness: f32,
    /// Spread of the per-channel refraction.
    pub(crate) dispersion: f32,
    /// Specular strength.
    pub(crate) specular_intensity: f32,
    /// Specular tint.
    pub(crate) specular_color: [f32; 3],
    /// Colour light takes on through the volume.
    pub(crate) attenuation_color: [f32; 3],
    /// Distance at which light has the attenuation colour. Infinite is none.
    pub(crate) attenuation_distance: f32,
    /// Scale of the animated distortion noise.
    pub(crate) distortion_scale: f32,
    /// Speed of the animated distortion noise.
    pub(crate) temporal_distortion: f32,
    /// Strength of the volume distortion.
    pub(crate) distortion: f32,
    /// Strength of the surface distortion.
    pub(crate) surface_distortion: f32,
    /// Extra per-channel offset.
    pub(crate) chromatic_aberration: f32,
    /// Blur along the refraction direction.
    pub(crate) anisotropic_blur: f32,
    /// Strength of the sample dither.
    pub(crate) dither_strength: f32,
    /// Size of the dither cells (the liquid's only).
    pub(crate) dither_scale: f32,
    /// Fade of the refraction towards the silhouette.
    pub(crate) edge_fade: f32,
    /// Scale of the rough-transmission blur.
    pub(crate) blur_scale: f32,
    /// How much the fallback colour shows where the backdrop is empty.
    pub(crate) backdrop_fallback: f32,
    /// What empty backdrop reads as.
    pub(crate) backdrop_fallback_color: [f32; 3],
    /// Depth offset of the backdrop lookup.
    pub(crate) backdrop_distance: f32,
    /// Transmission amount.
    pub(crate) transmission: f32,
}

/// The glass's rim emission: `pow(1 - n·v, power) * intensity`, pulsing.
#[derive(Debug, Clone, PartialEq)]
pub struct Rim {
    /// Emission colour.
    pub(crate) color: [f32; 3],
    /// Fresnel power.
    pub(crate) power: f32,
    /// Emission strength.
    pub(crate) intensity: f32,
    /// Pulse depth: the rim scales by `1 + pulse * sin(2t)`.
    pub(crate) pulse: f32,
}

/// The glass's stylised highlight.
#[derive(Debug, Clone, PartialEq)]
pub struct Highlight {
    /// Highlight colour.
    pub(crate) color: [f32; 3],
    /// Where the highlight's step starts.
    pub(crate) threshold: f32,
    /// Width of the step.
    pub(crate) softness: f32,
    /// Blinn-Phong exponent.
    pub(crate) shininess: f32,
    /// Strength.
    pub(crate) intensity: f32,
}

/// The key light, which eases from `start` to `target` over the first
/// seconds, and the ambient term.
#[derive(Debug, Clone, PartialEq)]
pub struct Light {
    /// Colour times intensity.
    pub(crate) color: [f32; 3],
    /// Ambient light.
    pub(crate) ambient: [f32; 3],
    /// World position at time 0.
    pub(crate) start: [f32; 3],
    /// World position it settles at.
    pub(crate) target: [f32; 3],
    /// Share of the distance left after each second.
    pub(crate) ease: f32,
}

/// The baked environment map's strength and rotation.
#[derive(Debug, Clone, PartialEq)]
pub struct Environment {
    /// Strength of the environment light.
    pub(crate) intensity: f32,
    /// Turn about Y, radians.
    pub(crate) rotation: f32,
}

/// The display pass's grade, then the paper colour behind it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Grade {
    /// Exposure multiplier.
    pub(crate) exposure: f32,
    /// Lift of the shadows.
    pub(crate) shadow_lift: f32,
    /// Gamma.
    pub(crate) gamma: f32,
    /// Contrast about 0.5.
    pub(crate) contrast: f32,
    /// Saturation.
    pub(crate) saturation: f32,
    /// Saturation of the dull colours.
    pub(crate) vibrance: f32,
    /// Warm (positive) or cool shift.
    pub(crate) temperature: f32,
    /// Magenta (positive) or green shift.
    pub(crate) tint: f32,
    /// Mix of the graded colour.
    pub(crate) amount: f32,
    /// The paper colour uncovered pixels flatten to.
    pub(crate) paper: [f32; 3],
}

/// The composite's alpha cut: `smoothstep(threshold, threshold + softness, a)`.
#[derive(Debug, Clone, PartialEq)]
pub struct Coverage {
    /// Lower edge.
    pub(crate) threshold: f32,
    /// Width of the edge.
    pub(crate) softness: f32,
}

/// The orthographic camera looking down -Z at the shells.
#[derive(Debug, Clone, PartialEq)]
pub struct Camera {
    /// Half the frame's edge in world units. The web build shows 0.7, where
    /// the glass (half-edge 0.39) fills the middle 55%; about 0.4 fills the
    /// frame with the shape.
    pub half_extent: f32,
}

/// A turn and a uniform scale of the shells about their centre. The card
/// is drawn facing the camera whatever the pose (its shader is a
/// billboard), so a turn shows as the glass and liquid turning around it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose {
    /// A unit quaternion `[x, y, z, w]`.
    pub rotation: [f32; 4],
    /// A factor on both shells' and the card's scale.
    pub scale: f32,
}

impl Pose {
    /// No turn, no scale: the web build's pose.
    pub const IDENTITY: Self = Self {
        rotation: [0.0, 0.0, 0.0, 1.0],
        scale: 1.0,
    };

    /// The rotation as a 3x3 matrix, column-major.
    #[expect(clippy::many_single_char_names, reason = "the quaternion's own names")]
    pub(crate) fn matrix(&self) -> [[f32; 3]; 3] {
        let [x, y, z, w] = self.rotation;
        let n = (x * x + y * y + z * z + w * w)
            .sqrt()
            .max(f32::MIN_POSITIVE);
        let (x, y, z, w) = (x / n, y / n, z / n, w / n);
        [
            [
                1.0 - 2.0 * (y * y + z * z),
                2.0 * (x * y + z * w),
                2.0 * (x * z - y * w),
            ],
            [
                2.0 * (x * y - z * w),
                1.0 - 2.0 * (x * x + z * z),
                2.0 * (y * z + x * w),
            ],
            [
                2.0 * (x * z + y * w),
                2.0 * (y * z - x * w),
                1.0 - 2.0 * (x * x + y * y),
            ],
        ]
    }
}

impl Default for Pose {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Default for PrismParams {
    #[expect(clippy::too_many_lines, reason = "one table of the preset's values")]
    fn default() -> Self {
        let shell = Shell {
            scale: 1.0,
            color: [1.0; 3],
            roughness: 0.0,
            ior: 1.5,
            thickness: 0.0,
            dispersion: 0.0,
            specular_intensity: 1.0,
            specular_color: [1.0; 3],
            attenuation_color: [1.0; 3],
            attenuation_distance: f32::INFINITY,
            distortion_scale: 0.0,
            temporal_distortion: 0.0,
            distortion: 0.0,
            surface_distortion: 0.0,
            chromatic_aberration: 0.0,
            anisotropic_blur: 0.0,
            dither_strength: 0.0,
            dither_scale: 0.0,
            edge_fade: 0.0,
            blur_scale: 1.0,
            backdrop_fallback: 1.0,
            backdrop_fallback_color: [0.0; 3],
            backdrop_distance: 0.0,
            transmission: 1.0,
        };
        Self {
            gradient: Gradient {
                colors: vec![
                    [0.814_846_6, 1.0, 0.0],
                    [0.274_677_3, 0.171_441_1, 1.0],
                    [1.0, 0.366_252_6, 0.0],
                    [1.0, 0.658_374_8, 0.0],
                    [1.0, 0.0, 0.545_724_5],
                ],
                speed: 1.65,
                distortion: 0.53,
                swirl: 0.81,
                falloff: 8.0,
                speckle_size: 0.9,
                grain_seed: 10.0,
                speckle_amount: 0.0,
                posterize_levels: 8.0,
                posterize_amount: 0.7,
                size: 0.5928,
                squircleness: 6.0,
            },
            liquid: Shell {
                scale: 0.55575,
                roughness: 0.36,
                ior: 1.49,
                thickness: 1.73,
                attenuation_color: [1.0, 0.693_871_7, 0.434_153_6],
                dither_strength: 0.5,
                dither_scale: 128.0,
                edge_fade: 0.1,
                backdrop_fallback_color: [0.814_846_6, 0.783_537_8, 0.715_693_5],
                ..shell.clone()
            },
            glass: Shell {
                scale: 0.65,
                color: [0.913_098_6; 3],
                roughness: 0.04,
                ior: 1.3,
                thickness: 2.69,
                dispersion: 4.0,
                specular_intensity: 0.55,
                distortion_scale: 5.3,
                temporal_distortion: 0.83,
                dither_strength: 2.0,
                edge_fade: 0.17,
                blur_scale: 8.0,
                backdrop_fallback_color: [0.783_537_8; 3],
                ..shell
            },
            rim: Rim {
                color: [1.0; 3],
                power: 8.0,
                intensity: 10.0,
                pulse: 0.15,
            },
            highlight: Highlight {
                color: [1.0, 0.584_078_4, 0.584_078_4],
                threshold: 0.45,
                softness: 0.6,
                shininess: 256.0,
                intensity: 3.15,
            },
            light: Light {
                color: [1.6; 3],
                ambient: [2.0; 3],
                start: [0.0, 0.0, 3.0],
                target: [2.4, 2.4, 0.4],
                ease: 0.2,
            },
            environment: Environment {
                intensity: 0.7,
                rotation: 0.75 * PI,
            },
            grade: Grade {
                exposure: 1.0,
                shadow_lift: 0.0,
                gamma: 1.0,
                contrast: 1.1,
                saturation: 1.2,
                vibrance: 0.1,
                temperature: 0.05,
                tint: 0.0,
                amount: 1.0,
                paper: [0.992_157, 0.913_726_8, 0.725_494],
            },
            coverage: Coverage {
                threshold: 0.55,
                softness: 0.04,
            },
            camera: Camera { half_extent: 0.7 },
            pose: Pose::IDENTITY,
        }
    }
}

impl PrismParams {
    /// The key light's world position `t` seconds in. The web build steps
    /// `p += (target - p) * (1 - ease^dt)` each frame, which is this at any
    /// frame rate.
    pub(crate) fn light_position(&self, t: f32) -> [f32; 3] {
        let left = f64::from(self.light.ease).powf(f64::from(t));
        let mut out = [0.0; 3];
        for (i, o) in out.iter_mut().enumerate() {
            let (s, e) = (
                f64::from(self.light.start[i]),
                f64::from(self.light.target[i]),
            );
            *o = (e + (s - e) * left) as f32;
        }
        out
    }

    /// The gradient's own clock `t` seconds in.
    pub(crate) fn gradient_time(&self, t: f32) -> f32 {
        (f64::from(t) * f64::from(self.gradient.speed)) as f32
    }
}
