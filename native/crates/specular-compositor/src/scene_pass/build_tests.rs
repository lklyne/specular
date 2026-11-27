use glam::Vec2;
use specular_core::CssSize;
use specular_scene::FramePin;

use super::unstretched;

const LEFT_HANDLE: FramePin = FramePin {
    right: true,
    bottom: false,
};

/// A 200 x 100 rect laid out at 200 x 100 CSS pixels, showing a frame still
/// painted at `painted` while a handle drags it.
fn shown(painted: (u32, u32), pin: FramePin) -> (Vec2, Vec2, [f32; 4]) {
    let size = Vec2::new(200.0, 100.0);
    let painted = CssSize::new(painted.0, painted.1);
    let shown = unstretched(size, CssSize::new(200, 100), painted, pin);
    (shown.offset, shown.size, shown.uv)
}

#[test]
fn a_stale_frame_keeps_to_the_edge_the_resize_holds_still() {
    let cases = [
        // Grown from the right: the frame stays at the left, the strip past
        // it is bare.
        (
            (160, 100),
            FramePin::default(),
            (Vec2::ZERO, Vec2::new(160.0, 100.0), [0.0, 0.0, 1.0, 1.0]),
        ),
        // Grown from the left: the frame stays at the right edge, which did
        // not move, so its content does not slide toward the pointer.
        (
            (160, 100),
            LEFT_HANDLE,
            (
                Vec2::new(40.0, 0.0),
                Vec2::new(160.0, 100.0),
                [0.0, 0.0, 1.0, 1.0],
            ),
        ),
        // Shrunk from the left: the frame's right part shows, against the
        // right edge, and its left part, under the moving edge, is cut.
        (
            (250, 100),
            LEFT_HANDLE,
            (Vec2::ZERO, Vec2::new(200.0, 100.0), [0.2, 0.0, 0.8, 1.0]),
        ),
        // Shrunk from the right: the right part is cut, as a browser's is.
        (
            (250, 100),
            FramePin::default(),
            (Vec2::ZERO, Vec2::new(200.0, 100.0), [0.0, 0.0, 0.8, 1.0]),
        ),
    ];
    for (painted, pin, expected) in cases {
        assert_eq!(
            shown(painted, pin),
            expected,
            "painted at {painted:?}, {pin:?}"
        );
    }
}
