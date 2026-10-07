//! A deterministic canvas: 500 sticky notes, 200 freehand strokes, 100 arrows.

/// Font both candidates ask for, so the golden images compare like with like.
pub const FONT_FAMILY: &str = "Helvetica";
pub const FONT_SIZE: f32 = 14.0;
pub const LINE_HEIGHT: f32 = 18.0;
pub const NOTE_SIZE: [f32; 2] = [220.0, 160.0];
pub const NOTE_PADDING: f32 = 12.0;
pub const NOTE_RADIUS: f32 = 8.0;
pub const TEXT_COLOR: [u8; 3] = [30, 30, 36];
pub const BACKGROUND: [u8; 3] = [246, 245, 241];

const COLUMNS: usize = 25;
const ROWS: usize = 20;
const PITCH: [f32; 2] = [260.0, 200.0];
const STROKES: usize = 200;
const STROKE_POINTS: usize = 64;
const ARROWS: usize = 100;

const NOTE_COLORS: [[u8; 3]; 5] = [
    [255, 236, 153],
    [255, 201, 201],
    [178, 242, 187],
    [165, 216, 255],
    [238, 190, 250],
];
const INK_COLORS: [[u8; 3]; 4] = [[224, 49, 49], [25, 113, 194], [47, 158, 68], [52, 58, 64]];
const WORDS: [&str; 24] = [
    "canvas",
    "viewport",
    "breakpoint",
    "iterate",
    "spatial",
    "annotate",
    "wireframe",
    "layout",
    "the",
    "a",
    "of",
    "and",
    "handle",
    "resize",
    "arrow",
    "sticky",
    "note",
    "quick",
    "fix",
    "typography",
    "ligature",
    "affinity",
    "margin",
    "baseline",
];

pub struct Note {
    /// Top-left corner in canvas units.
    pub origin: [f32; 2],
    pub color: [u8; 3],
    pub text: String,
}

pub struct Stroke {
    pub points: Vec<[f32; 2]>,
    pub width: f32,
    pub color: [u8; 3],
}

/// A cubic from `p[0]` to `p[3]` with a triangular head at `p[3]`.
pub struct Arrow {
    pub p: [[f32; 2]; 4],
    pub width: f32,
    pub color: [u8; 3],
}

impl Arrow {
    /// The head as a triangle: tip first, then the two barbs.
    pub fn head(&self) -> [[f32; 2]; 3] {
        let [tip, from] = [self.p[3], self.p[2]];
        let (dx, dy) = (tip[0] - from[0], tip[1] - from[1]);
        let length = dx.hypot(dy).max(1e-3);
        let (ux, uy) = (dx / length, dy / length);
        let (size, half) = (self.width * 5.0, self.width * 2.5);
        let base = [tip[0] - ux * size, tip[1] - uy * size];
        [
            tip,
            [base[0] - uy * half, base[1] + ux * half],
            [base[0] + uy * half, base[1] - ux * half],
        ]
    }
}

/// Bounding box of a polyline as (`origin`, `size`), padded by `pad`.
pub fn bounds(points: &[[f32; 2]], pad: f32) -> ([f32; 2], [f32; 2]) {
    let mut min = [f32::MAX; 2];
    let mut max = [f32::MIN; 2];
    for point in points {
        min = [min[0].min(point[0]), min[1].min(point[1])];
        max = [max[0].max(point[0]), max[1].max(point[1])];
    }
    (
        [min[0] - pad, min[1] - pad],
        [max[0] - min[0] + 2.0 * pad, max[1] - min[1] + 2.0 * pad],
    )
}

pub struct World {
    pub notes: Vec<Note>,
    pub strokes: Vec<Stroke>,
    pub arrows: Vec<Arrow>,
    pub size: [f32; 2],
}

impl World {
    pub fn build() -> Self {
        let mut rng = Rng(0x5EED_CAFE_F00D_0001);
        let size = [COLUMNS as f32 * PITCH[0], ROWS as f32 * PITCH[1]];

        let notes: Vec<Note> = (0..COLUMNS * ROWS)
            .map(|i| {
                let words = 8 + rng.below(24);
                let text = (0..words)
                    .map(|_| WORDS[rng.below(WORDS.len())])
                    .collect::<Vec<_>>()
                    .join(" ");
                Note {
                    origin: [
                        (i % COLUMNS) as f32 * PITCH[0],
                        (i / COLUMNS) as f32 * PITCH[1],
                    ],
                    color: NOTE_COLORS[rng.below(NOTE_COLORS.len())],
                    text,
                }
            })
            .collect();

        let strokes = (0..STROKES)
            .map(|_| {
                let mut at = [rng.unit() * size[0], rng.unit() * size[1]];
                let mut heading = rng.unit() * std::f32::consts::TAU;
                let points = (0..STROKE_POINTS)
                    .map(|_| {
                        heading += (rng.unit() - 0.5) * 1.2;
                        at = [at[0] + heading.cos() * 9.0, at[1] + heading.sin() * 9.0];
                        at
                    })
                    .collect();
                Stroke {
                    points,
                    width: 2.0 + rng.unit() * 2.0,
                    color: INK_COLORS[rng.below(INK_COLORS.len())],
                }
            })
            .collect();

        let arrows = (0..ARROWS)
            .map(|_| {
                let from = rng.below(COLUMNS * ROWS);
                let column = (from % COLUMNS + 1 + rng.below(2)).min(COLUMNS - 1);
                let row = (from / COLUMNS + rng.below(3)).min(ROWS - 1);
                let start = notes[from].origin;
                let end = notes[row * COLUMNS + column].origin;
                let a = [start[0] + NOTE_SIZE[0], start[1] + NOTE_SIZE[1] * 0.5];
                let b = [end[0] - 4.0, end[1] + NOTE_SIZE[1] * 0.5];
                let reach = ((b[0] - a[0]).abs() * 0.5).max(30.0);
                Arrow {
                    p: [a, [a[0] + reach, a[1]], [b[0] - reach, b[1]], b],
                    width: 2.0,
                    color: INK_COLORS[rng.below(INK_COLORS.len())],
                }
            })
            .collect();

        Self {
            notes,
            strokes,
            arrows,
            size,
        }
    }

    /// Centre of a note near the middle of the grid, where the golden images look.
    pub fn focus(&self) -> [f32; 2] {
        let note = &self.notes[(ROWS / 2) * COLUMNS + COLUMNS / 2];
        [
            note.origin[0] + NOTE_SIZE[0] * 0.5,
            note.origin[1] + NOTE_SIZE[1] * 0.5,
        ]
    }
}

/// xorshift64*: enough randomness for a fixture, with no dependency.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }

    fn below(&mut self, bound: usize) -> usize {
        (self.next() % bound as u64) as usize
    }
}
