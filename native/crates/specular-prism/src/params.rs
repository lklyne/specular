//! The Prism preset: every value the web build feeds its shaders, by name.
//!
//! Defaults are the values three uploaded for the Prism tab with every
//! control at its default (`reference/uniforms/f0240.json`). Colours are
//! linear, as the shaders take them.

use std::f32::consts::PI;

/// Every input of a Prism frame except the time.
#[derive(Debug, Clone, PartialEq)]
pub struct PrismParams {
    /// The mesh-gradient card inside the shells.
    pub gradient: Gradient,
    /// The inner shell, a tinted liquid.
    pub liquid: Shell,
    /// The outer shell, clear glass.
    pub glass: Shell,
    /// The glass's emissive rim.
    pub rim: Rim,
    /// The glass's stylised specular highlight.
    pub highlight: Highlight,
    /// The key light and the ambient term.
    pub light: Light,
    /// The baked environment's strength and turn.
    pub environment: Environment,
    /// The colour grade of the display pass.
    pub grade: Grade,
    /// Where the composite cuts the shape out of the scene's alpha.
    pub coverage: Coverage,
}

/// The gradient card.
#[derive(Debug, Clone, PartialEq)]
pub struct Gradient {
    /// The blob colours, linear RGB. The shader takes up to eight.
    pub colors: Vec<[f32; 3]>,
    /// Gradient seconds per second of time.
    pub speed: f32,
    /// How far the domain warp pushes the blobs.
    pub distortion: f32,
    /// The swirl around the card's centre.
    pub swirl: f32,
    /// The inverse-distance power each blob falls off with.
    pub falloff: f32,
    /// The speckle cell size, which also sets the grain's scale.
    pub speckle_size: f32,
    /// The seed of the grain and speckle hashes.
    pub grain_seed: f32,
    /// The share of cells that get a speckle.
    pub speckle_amount: f32,
    /// Posterize steps per channel.
    pub posterize_levels: f32,
    /// How much of the posterized colour is mixed in.
    pub posterize_amount: f32,
    /// The card's edge length in world units (the camera sees 1.4).
    pub size: f32,
    /// The superellipse exponent of the card's outline.
    pub squircleness: f32,
}

/// A transmission shell: a rounded box, refracting what is drawn behind it.
#[derive(Debug, Clone, PartialEq)]
pub struct Shell {
    /// Uniform scale of the unit shell (edge 1.2 world units).
    pub scale: f32,
    /// Base colour.
    pub color: [f32; 3],
    /// Surface roughness.
    pub roughness: f32,
    /// Index of refraction.
    pub ior: f32,
    /// Thickness for the refraction offset.
    pub thickness: f32,
    /// Spread of the per-channel refraction.
    pub dispersion: f32,
    /// Specular strength.
    pub specular_intensity: f32,
    /// Specular tint.
    pub specular_color: [f32; 3],
    /// Colour light takes on through the volume.
    pub attenuation_color: [f32; 3],
    /// Distance at which light has the attenuation colour. Infinite is none.
    pub attenuation_distance: f32,
    /// Scale of the animated distortion noise.
    pub distortion_scale: f32,
    /// Speed of the animated distortion noise.
    pub temporal_distortion: f32,
    /// Strength of the volume distortion.
    pub distortion: f32,
    /// Strength of the surface distortion.
    pub surface_distortion: f32,
    /// Extra per-channel offset.
    pub chromatic_aberration: f32,
    /// Blur along the refraction direction.
    pub anisotropic_blur: f32,
    /// Strength of the sample dither.
    pub dither_strength: f32,
    /// Size of the dither cells (the liquid's only).
    pub dither_scale: f32,
    /// Fade of the refraction towards the silhouette.
    pub edge_fade: f32,
    /// Scale of the rough-transmission blur.
    pub blur_scale: f32,
    /// How much the fallback colour shows where the backdrop is empty.
    pub backdrop_fallback: f32,
    /// What empty backdrop reads as.
    pub backdrop_fallback_color: [f32; 3],
    /// Depth offset of the backdrop lookup.
    pub backdrop_distance: f32,
    /// Transmission amount.
    pub transmission: f32,
}

/// The glass's rim emission: `pow(1 - n·v, power) * intensity`, pulsing.
#[derive(Debug, Clone, PartialEq)]
pub struct Rim {
    /// Emission colour.
    pub color: [f32; 3],
    /// Fresnel power.
    pub power: f32,
    /// Emission strength.
    pub intensity: f32,
    /// Pulse depth: the rim scales by `1 + pulse * sin(2t)`.
    pub pulse: f32,
}

/// The glass's stylised highlight.
#[derive(Debug, Clone, PartialEq)]
pub struct Highlight {
    /// Highlight colour.
    pub color: [f32; 3],
    /// Where the highlight's step starts.
    pub threshold: f32,
    /// Width of the step.
    pub softness: f32,
    /// Blinn-Phong exponent.
    pub shininess: f32,
    /// Strength.
    pub intensity: f32,
}

/// The key light, which eases from `start` to `target` over the first
/// seconds, and the ambient term.
#[derive(Debug, Clone, PartialEq)]
pub struct Light {
    /// Colour times intensity.
    pub color: [f32; 3],
    /// Ambient light.
    pub ambient: [f32; 3],
    /// World position at time 0.
    pub start: [f32; 3],
    /// World position it settles at.
    pub target: [f32; 3],
    /// Share of the distance left after each second.
    pub ease: f32,
}

/// The baked environment map's strength and rotation.
#[derive(Debug, Clone, PartialEq)]
pub struct Environment {
    /// Strength of the environment light.
    pub intensity: f32,
    /// Turn about Y, radians.
    pub rotation: f32,
}

/// The display pass's grade, then the paper colour behind it.
#[derive(Debug, Clone, PartialEq)]
pub struct Grade {
    /// Exposure multiplier.
    pub exposure: f32,
    /// Lift of the shadows.
    pub shadow_lift: f32,
    /// Gamma.
    pub gamma: f32,
    /// Contrast about 0.5.
    pub contrast: f32,
    /// Saturation.
    pub saturation: f32,
    /// Saturation of the dull colours.
    pub vibrance: f32,
    /// Warm (positive) or cool shift.
    pub temperature: f32,
    /// Magenta (positive) or green shift.
    pub tint: f32,
    /// Mix of the graded colour.
    pub amount: f32,
    /// The paper colour uncovered pixels flatten to.
    pub paper: [f32; 3],
}

/// The composite's alpha cut: `smoothstep(threshold, threshold + softness, a)`.
#[derive(Debug, Clone, PartialEq)]
pub struct Coverage {
    /// Lower edge.
    pub threshold: f32,
    /// Width of the edge.
    pub softness: f32,
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
        }
    }
}

impl PrismParams {
    /// The key light's world position `t` seconds in. The web build steps
    /// `p += (target - p) * (1 - ease^dt)` each frame, which is this at any
    /// frame rate.
    pub fn light_position(&self, t: f32) -> [f32; 3] {
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
    pub fn gradient_time(&self, t: f32) -> f32 {
        (f64::from(t) * f64::from(self.gradient.speed)) as f32
    }
}
