//! A uniform block laid out from a struct in the shader source.
//!
//! The shaders are three's generated WGSL, kept close to verbatim, so their
//! uniforms are named `nodeUniformN` and their order is whatever three chose.
//! Reading the layout from the source instead of mirroring each struct in
//! Rust keeps a regenerated shader from silently shifting every offset.

/// One member's place in the block.
#[derive(Debug, Clone)]
struct Member {
    name: String,
    offset: usize,
    ty: Ty,
}

/// The member types three emits in these shaders.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ty {
    F32,
    Vec2,
    Vec3,
    Vec4,
    Mat3,
    Mat4,
    Vec4Array(usize),
}

impl Ty {
    fn parse(text: &str) -> Option<Self> {
        let text: String = text.chars().filter(|c| !c.is_whitespace()).collect();
        Some(match text.as_str() {
            "f32" => Self::F32,
            "vec2<f32>" => Self::Vec2,
            "vec3<f32>" => Self::Vec3,
            "vec4<f32>" => Self::Vec4,
            "mat3x3<f32>" => Self::Mat3,
            "mat4x4<f32>" => Self::Mat4,
            other => {
                let count = other.strip_prefix("array<vec4<f32>,")?.strip_suffix('>')?;
                Self::Vec4Array(count.parse().ok()?)
            }
        })
    }

    /// WGSL's (align, size) for a uniform member.
    fn layout(self) -> (usize, usize) {
        match self {
            Self::F32 => (4, 4),
            Self::Vec2 => (8, 8),
            Self::Vec3 => (16, 12),
            Self::Vec4 => (16, 16),
            Self::Mat3 => (16, 48),
            Self::Mat4 => (16, 64),
            Self::Vec4Array(n) => (16, 16 * n),
        }
    }
}

/// A value for one member. Matrices are column-major, as WGSL holds them.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Value<'a> {
    F32(f32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    Mat3([[f32; 3]; 3]),
    Mat4([[f32; 4]; 4]),
    Vec4s(&'a [[f32; 4]]),
}

/// The bytes of one uniform struct, written member by member by name.
#[derive(Debug, Clone)]
pub(crate) struct Block {
    members: Vec<Member>,
    bytes: Vec<u8>,
}

impl Block {
    /// Lays out `struct_name` as declared in `source`. `None` if the struct
    /// is missing or holds a type this module does not know.
    pub(crate) fn from_wgsl(source: &str, struct_name: &str) -> Option<Self> {
        let head = format!("struct {struct_name} {{");
        let start = source.find(&head)? + head.len();
        let body = &source[start..start + source[start..].find("};")?];
        let mut members = Vec::new();
        let mut offset: usize = 0;
        let mut align_max: usize = 4;
        for line in split_members(body) {
            let (name, ty) = line.split_once(':')?;
            let ty = Ty::parse(ty)?;
            let (align, size) = ty.layout();
            offset = offset.next_multiple_of(align);
            align_max = align_max.max(align);
            members.push(Member {
                name: name.trim().to_owned(),
                offset,
                ty,
            });
            offset += size;
        }
        Some(Self {
            members,
            bytes: vec![0; offset.next_multiple_of(align_max)],
        })
    }

    /// A block with no members, as a stand-in for one that failed to parse.
    pub(crate) fn empty() -> Self {
        Self {
            members: Vec::new(),
            bytes: vec![0; 16],
        }
    }

    /// The packed bytes, ready for a uniform buffer.
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Writes `value` into the member `name`. A name the shader lacks or a
    /// value of the wrong shape is a bug in the caller; it is skipped in a
    /// release build.
    pub(crate) fn set(&mut self, name: &str, value: Value<'_>) {
        let Some(member) = self.members.iter().find(|m| m.name == name) else {
            debug_assert!(false, "the shader has no uniform {name}");
            return;
        };
        let mut floats: Vec<f32> = Vec::new();
        let fits = match (member.ty, value) {
            (Ty::F32, Value::F32(v)) => {
                floats.push(v);
                true
            }
            (Ty::Vec2, Value::Vec2(v)) => {
                floats.extend(v);
                true
            }
            (Ty::Vec3, Value::Vec3(v)) => {
                floats.extend(v);
                true
            }
            (Ty::Vec4, Value::Vec4(v)) => {
                floats.extend(v);
                true
            }
            (Ty::Mat3, Value::Mat3(m)) => {
                // Each column is padded to a vec4.
                for column in m {
                    floats.extend(column);
                    floats.push(0.0);
                }
                true
            }
            (Ty::Mat4, Value::Mat4(m)) => {
                floats.extend(m.iter().flatten());
                true
            }
            (Ty::Vec4Array(n), Value::Vec4s(items)) => {
                floats.extend(items.iter().take(n).flatten());
                true
            }
            _ => false,
        };
        debug_assert!(fits, "uniform {name} is a {:?}", member.ty);
        if !fits {
            return;
        }
        let at = member.offset;
        let src: &[u8] = bytemuck::cast_slice(&floats);
        self.bytes[at..at + src.len()].copy_from_slice(src);
    }
}

/// The member lines of a struct body, split on commas outside `<...>`.
fn split_members(body: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0_i32;
    let mut start = 0;
    for (i, c) in body.char_indices() {
        match c {
            '<' => depth += 1,
            '>' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(&body[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&body[start..]);
    parts
        .into_iter()
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packs_three_struct_with_vec3_and_mat3_padding() {
        let src = "struct objectStruct {\n\ta : vec3<f32>,\n\tb : f32,\n\tc : mat3x3<f32>,\n\td : f32\n};";
        let mut block = Block::from_wgsl(src, "objectStruct").unwrap();
        // a 0..12, b 12, c 16..64, d 64, size rounded to 80.
        assert_eq!(block.bytes().len(), 80);
        block.set("b", Value::F32(2.0));
        block.set(
            "c",
            Value::Mat3([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]]),
        );
        block.set("d", Value::F32(5.0));
        let floats: Vec<f32> = bytemuck::pod_collect_to_vec(block.bytes());
        assert_eq!(floats[3], 2.0);
        assert_eq!(&floats[4..12], &[1.0, 2.0, 3.0, 0.0, 4.0, 5.0, 6.0, 0.0]);
        assert_eq!(floats[16], 5.0);
    }
}
