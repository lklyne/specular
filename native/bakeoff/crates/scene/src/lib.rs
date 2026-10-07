//! The workload both bake-off candidates draw, plus the headless plumbing they
//! share: a camera, an offscreen target with PNG readback, and frame timing.

pub mod camera;
pub mod gpu;
pub mod harness;
pub mod world;

pub use camera::Camera;
pub use gpu::Gpu;
pub use world::World;
