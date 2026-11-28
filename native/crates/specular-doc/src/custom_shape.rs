//! [`CustomShape`]: a shape whose silhouette a mod gives (ADR 0060). It is a
//! [`Shape`](crate::Shape) like any other, so it takes the shape tool's
//! colours, fill, border and label; only its outline is its own.
//!
//! The outline is SVG path data in the same 0 to 100 box the app's own
//! silhouettes are drawn in, stretched to the shape's rect. It is kept in
//! the `.canvas` file, so a reader without the mod still draws the shape.

use std::fmt;

use crate::JsonMap;

/// The most commands an outline may have.
const MOST_COMMANDS: usize = 4096;

/// One step of an outline, in the 0 to 100 box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutlineCommand {
    /// Starts a new sub-path.
    MoveTo([f64; 2]),
    /// A straight line.
    LineTo([f64; 2]),
    /// A quadratic Bézier.
    QuadTo {
        /// Its control point.
        control: [f64; 2],
        /// Where it ends.
        to: [f64; 2],
    },
    /// A cubic Bézier.
    CubicTo {
        /// The first control point.
        first: [f64; 2],
        /// The second control point.
        second: [f64; 2],
        /// Where it ends.
        to: [f64; 2],
    },
    /// Closes the sub-path.
    Close,
}

/// Why path data is not an outline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutlineError(String);

impl fmt::Display for OutlineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for OutlineError {}

/// A mod's shape: which one it is, the values it was made from, and its
/// outline.
#[derive(Debug, Clone, PartialEq)]
pub struct CustomShape {
    /// `<mod>/<shape>`, as the mod's `mod.json` names it.
    pub name: String,
    /// The values the mod made the outline from, which only it reads.
    pub params: JsonMap,
    path: String,
    outline: Vec<OutlineCommand>,
}

impl CustomShape {
    /// The shape `name` with `params` and the outline `path`, or why `path`
    /// is not one.
    pub fn new(name: impl Into<String>, params: JsonMap, path: &str) -> Result<Self, OutlineError> {
        Ok(Self {
            name: name.into(),
            params,
            outline: parse(path)?,
            path: path.trim().to_owned(),
        })
    }

    /// The outline as SVG path data.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The outline's steps, in the 0 to 100 box.
    pub fn outline(&self) -> &[OutlineCommand] {
        &self.outline
    }

    /// As the `.canvas` file keeps it: `{shape, params, path}`.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({ "shape": self.name, "params": self.params, "path": self.path })
    }

    /// From `{shape, params, path}`, or `None` when it is not that or its
    /// path is no outline.
    pub fn from_json(value: &serde_json::Value) -> Option<Self> {
        let name = value.get("shape")?.as_str()?;
        let params = (value.get("params").and_then(serde_json::Value::as_object))
            .cloned()
            .unwrap_or_default();
        Self::new(name, params, value.get("path")?.as_str()?).ok()
    }

    /// The mod that gives it: the part of its name before the `/`.
    pub fn mod_name(&self) -> &str {
        self.name
            .split_once('/')
            .map_or(&self.name, |(name, _)| name)
    }
}

/// Reads path data: `M`, `L`, `H`, `V`, `Q`, `C` and `Z`, absolute or, in
/// lower case, relative. Arcs and the smooth forms are refused: a curve is
/// written out with its control points.
fn parse(path: &str) -> Result<Vec<OutlineCommand>, OutlineError> {
    let mut reader = Reader { rest: path.trim() };
    let mut out = Vec::new();
    let mut at = [0.0, 0.0];
    let mut start = [0.0, 0.0];
    let mut letter: Option<char> = None;
    while !reader.done() {
        let command = match reader.letter() {
            Some(found) => found,
            // A number goes on with the last command; after a move it is a
            // line.
            None => match letter {
                Some('M') => 'L',
                Some('m') => 'l',
                Some(last) if !matches!(last, 'Z' | 'z') => last,
                _ => return Err(OutlineError("path data starts with M".to_owned())),
            },
        };
        if out.is_empty() && !matches!(command, 'M' | 'm') {
            return Err(OutlineError("path data starts with M".to_owned()));
        }
        let relative = command.is_ascii_lowercase();
        let base = if relative { at } else { [0.0, 0.0] };
        let point = |reader: &mut Reader<'_>| -> Result<[f64; 2], OutlineError> {
            let x = reader.number()?;
            let y = reader.number()?;
            Ok([base[0] + x, base[1] + y])
        };
        let step = match command.to_ascii_uppercase() {
            'M' => {
                at = point(&mut reader)?;
                start = at;
                OutlineCommand::MoveTo(at)
            }
            'L' => {
                at = point(&mut reader)?;
                OutlineCommand::LineTo(at)
            }
            'H' => {
                let x = reader.number()?;
                at = [if relative { at[0] + x } else { x }, at[1]];
                OutlineCommand::LineTo(at)
            }
            'V' => {
                let y = reader.number()?;
                at = [at[0], if relative { at[1] + y } else { y }];
                OutlineCommand::LineTo(at)
            }
            'Q' => {
                let control = point(&mut reader)?;
                at = point(&mut reader)?;
                OutlineCommand::QuadTo { control, to: at }
            }
            'C' => {
                let first = point(&mut reader)?;
                let second = point(&mut reader)?;
                at = point(&mut reader)?;
                OutlineCommand::CubicTo {
                    first,
                    second,
                    to: at,
                }
            }
            'Z' => {
                at = start;
                OutlineCommand::Close
            }
            other => {
                return Err(OutlineError(format!(
                    "`{other}` is not an outline command: use M, L, H, V, Q, C and Z"
                )));
            }
        };
        out.push(step);
        if out.len() > MOST_COMMANDS {
            return Err(OutlineError(format!(
                "an outline has at most {MOST_COMMANDS} commands"
            )));
        }
        letter = Some(command);
    }
    if out.len() < 2 {
        return Err(OutlineError(
            "an outline needs a move and a line".to_owned(),
        ));
    }
    Ok(out)
}

/// Path data read from the front.
struct Reader<'a> {
    rest: &'a str,
}

impl Reader<'_> {
    fn skip(&mut self) {
        self.rest = self
            .rest
            .trim_start_matches(|c: char| c.is_whitespace() || c == ',');
    }

    fn done(&mut self) -> bool {
        self.skip();
        self.rest.is_empty()
    }

    /// The command letter next, if a letter is next.
    fn letter(&mut self) -> Option<char> {
        self.skip();
        let next = self.rest.chars().next()?;
        next.is_ascii_alphabetic().then(|| {
            self.rest = &self.rest[1..];
            next
        })
    }

    fn number(&mut self) -> Result<f64, OutlineError> {
        self.skip();
        let bytes = self.rest.as_bytes();
        let mut end = 0;
        if matches!(bytes.first(), Some(b'+' | b'-')) {
            end += 1;
        }
        let mut dot = false;
        let mut exponent = false;
        while let Some(&byte) = bytes.get(end) {
            match byte {
                b'0'..=b'9' => end += 1,
                b'.' if !dot && !exponent => {
                    dot = true;
                    end += 1;
                }
                b'e' | b'E' if !exponent && end > 0 => {
                    exponent = true;
                    end += 1;
                    if matches!(bytes.get(end), Some(b'+' | b'-')) {
                        end += 1;
                    }
                }
                _ => break,
            }
        }
        let text = &self.rest[..end];
        let value: f64 = text
            .parse()
            .map_err(|_| OutlineError(format!("expected a number at `{}`", self.head())))?;
        if !value.is_finite() {
            return Err(OutlineError(format!("`{text}` is not a finite number")));
        }
        self.rest = &self.rest[end..];
        Ok(value)
    }

    fn head(&self) -> String {
        self.rest.chars().take(12).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_reads_absolute_and_relative_steps_and_runs_on_after_a_move() {
        let out = parse("M50,0 L100 50 l-50,50 H0 v-50 Z").expect("an outline");
        assert_eq!(
            out,
            vec![
                OutlineCommand::MoveTo([50.0, 0.0]),
                OutlineCommand::LineTo([100.0, 50.0]),
                OutlineCommand::LineTo([50.0, 100.0]),
                OutlineCommand::LineTo([0.0, 100.0]),
                OutlineCommand::LineTo([0.0, 50.0]),
                OutlineCommand::Close,
            ]
        );
        let implicit = parse("M0 0 10 10 20 0z").expect("an outline");
        assert_eq!(implicit[2], OutlineCommand::LineTo([20.0, 0.0]));
        let curve = parse("M0,0 C0,50 100,50 100,0 q-50,-40 -100,0").expect("an outline");
        assert_eq!(
            curve[2],
            OutlineCommand::QuadTo {
                control: [50.0, -40.0],
                to: [0.0, 0.0]
            }
        );
    }

    #[test]
    fn arcs_bad_numbers_and_a_missing_move_are_refused() {
        assert!(parse("M0,0 A10,10 0 0 1 20,20").is_err());
        assert!(parse("L10,10").is_err());
        assert!(parse("M0,0 Lx,10").is_err());
        assert!(parse("M0,0").is_err());
        assert!(parse("").is_err());
    }
}
