//! The preset's values.

use std::f32::consts::PI;

use super::{
    Camera, Coverage, Environment, Grade, Gradient, Highlight, Light, Pose, PrismParams, Rim, Shell,
};

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
