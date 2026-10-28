//! A layered (Sugiyama) layout with right-angle edges, for laying out
//! screens as a flow.
//!
//! Adapted from Pele's `src/layout/layered.ts`
//! (<https://github.com/obsidianmd/pele>, MIT, Copyright (c) 2026 kepano).
//! `orient` and `rank` follow it closely. Ordering, placement and routing
//! keep its shape (sweeps that keep the best order, ends spread along a
//! side, a track of its own for each turn across a gap) in fewer lines,
//! since a flow of screens is tens of nodes, not thousands, and has no
//! nested groups.
//!
//! Ranks run down the `y` axis; the caller turns the result for a flow
//! that runs across. What it aims for, most important first:
//!
//! 1. No line runs through a node, and no two lines touch or run along
//!    each other: the ends on one side have places of their own, and each
//!    turn across a gap has a track of its own.
//! 2. Lines cross as little as the order of the nodes allows.
//! 3. An edge is straight where it can be, else it turns once in each gap
//!    it crosses.
//! 4. Nodes keep the order they were given in, where nothing above says
//!    otherwise.

/// A node to place, measured in the layout's own axes: `w` across the
/// flow and `h` along it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Node {
    pub(crate) w: f64,
    pub(crate) h: f64,
}

/// An edge from `tail` to `head`, with the room its label needs along the
/// flow (0 for none).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Link {
    pub(crate) tail: usize,
    pub(crate) head: usize,
    pub(crate) label: f64,
}

/// The room the layout leaves, in canvas units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Spacing {
    /// Between two nodes of a rank.
    pub(crate) node: f64,
    /// The least gap between two ranks.
    pub(crate) rank: f64,
    /// Between two ends on one side of a node.
    pub(crate) port: f64,
    /// Between two tracks in a gap.
    pub(crate) track: f64,
    /// Kept clear between a rank and the nearest track, and either side of
    /// a label.
    pub(crate) margin: f64,
}

/// Where everything went. `centres[i]` is node `i`'s centre; `routes[i]`
/// is link `i`'s path from its tail to its head, `None` for a link from a
/// node to itself.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Layout {
    pub(crate) centres: Vec<(f64, f64)>,
    pub(crate) routes: Vec<Option<Vec<(f64, f64)>>>,
}

/// How many times the order of the ranks is swept, at most, and how many
/// sweeps in a row may find nothing better before it stops.
const MAX_SWEEPS: usize = 24;
const PATIENCE: usize = 4;
/// Placement passes: each pulls every node towards its neighbours.
const PLACE_PASSES: usize = 12;
/// How far out of line two points may be and still count as in line.
const IN_LINE: f64 = 0.5;

/// Lays out `nodes` and `links`.
pub(crate) fn layout(nodes: &[Node], links: &[Link], spacing: Spacing) -> Layout {
    let n = nodes.len();
    let reversed = orient(n, links);
    let rank = rank(n, links, &reversed);
    let mut graph = Graph::build(nodes, links, &reversed, &rank);
    graph.order();
    graph.place(spacing);
    graph.route(nodes, links, &reversed, spacing)
}

/// Which links run against the flow, so that what is left has no cycle: a
/// depth-first walk in the order given turns round each link back to a
/// node still on its path. Links from a node to itself are left alone.
fn orient(n: usize, links: &[Link]) -> Vec<bool> {
    let mut out: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (index, link) in links.iter().enumerate() {
        if link.tail != link.head {
            out[link.tail].push(index);
        }
    }
    // 0 unseen, 1 on the path, 2 done.
    let mut state = vec![0u8; n];
    let mut cursor = vec![0usize; n];
    let mut reversed = vec![false; links.len()];
    let mut stack = Vec::new();
    for root in 0..n {
        if state[root] != 0 {
            continue;
        }
        stack.push(root);
        state[root] = 1;
        while let Some(&v) = stack.last() {
            if let Some(&index) = out[v].get(cursor[v]) {
                cursor[v] += 1;
                let w = links[index].head;
                match state[w] {
                    1 => reversed[index] = true,
                    0 => {
                        state[w] = 1;
                        stack.push(w);
                    }
                    _ => {}
                }
            } else {
                state[v] = 2;
                stack.pop();
            }
        }
    }
    reversed
}

/// The ends of `link` as the layout runs it: from the lower rank to the
/// higher.
fn ends(link: &Link, reversed: bool) -> (usize, usize) {
    if reversed {
        (link.head, link.tail)
    } else {
        (link.tail, link.head)
    }
}

/// Each node's rank: the longest path to it, then each node moved towards
/// the side it has more links on, which keeps a loosely tied node beside
/// its neighbours. Ranks start at 0.
fn rank(count: usize, links: &[Link], reversed: &[bool]) -> Vec<usize> {
    let mut outs: Vec<Vec<usize>> = vec![Vec::new(); count];
    let mut ins: Vec<Vec<usize>> = vec![Vec::new(); count];
    for (index, link) in links.iter().enumerate() {
        if link.tail == link.head {
            continue;
        }
        let (a, b) = ends(link, reversed[index]);
        outs[a].push(b);
        ins[b].push(a);
    }
    let mut indegree: Vec<usize> = ins.iter().map(Vec::len).collect();
    let mut topo: Vec<usize> = (0..count).filter(|&v| indegree[v] == 0).collect();
    let mut ranks = vec![0i64; count];
    let mut next_up = 0;
    while next_up < topo.len() {
        let v = topo[next_up];
        next_up += 1;
        for &w in &outs[v] {
            ranks[w] = ranks[w].max(ranks[v] + 1);
            indegree[w] -= 1;
            if indegree[w] == 0 {
                topo.push(w);
            }
        }
    }
    for _ in 0..8 {
        let mut changed = false;
        for &v in topo.iter().rev() {
            let (arriving, leaving) = (ins[v].len(), outs[v].len());
            if arriving == 0 && leaving == 0 {
                continue;
            }
            let lo = ins[v].iter().map(|&u| ranks[u] + 1).max();
            let hi = outs[v].iter().map(|&w| ranks[w] - 1).min();
            let next = match (lo, hi) {
                (_, Some(hi)) if leaving > arriving => hi,
                (Some(lo), _) if arriving > leaving => lo,
                _ => ranks[v],
            };
            if next != ranks[v] {
                ranks[v] = next;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let least = ranks.iter().copied().min().unwrap_or(0);
    ranks.iter().map(|&rank| (rank - least) as usize).collect()
}

/// The graph that is ordered and placed: the nodes, and a dummy in every
/// rank a link passes, so every segment joins two neighbouring ranks.
struct Graph {
    /// Real nodes first, then dummies.
    width: Vec<f64>,
    real: usize,
    rank: Vec<usize>,
    /// Each link's chain from its upper end to its lower, `None` for a
    /// link from a node to itself.
    chains: Vec<Option<Vec<usize>>>,
    /// Neighbours in the rank above and below.
    up: Vec<Vec<usize>>,
    down: Vec<Vec<usize>>,
    /// The vertices of each rank in order, and each vertex's place in it.
    layers: Vec<Vec<usize>>,
    pos: Vec<usize>,
    /// Across-flow centres.
    x: Vec<f64>,
}

impl Graph {
    fn build(nodes: &[Node], links: &[Link], reversed: &[bool], rank: &[usize]) -> Self {
        let real = nodes.len();
        let mut width: Vec<f64> = nodes.iter().map(|node| node.w).collect();
        let mut ranks = rank.to_vec();
        let mut chains = Vec::with_capacity(links.len());
        for (index, link) in links.iter().enumerate() {
            if link.tail == link.head {
                chains.push(None);
                continue;
            }
            let (a, b) = ends(link, reversed[index]);
            let mut chain = vec![a];
            for r in rank[a] + 1..rank[b] {
                chain.push(width.len());
                width.push(0.0);
                ranks.push(r);
            }
            chain.push(b);
            chains.push(Some(chain));
        }
        let count = width.len();
        let mut up = vec![Vec::new(); count];
        let mut down = vec![Vec::new(); count];
        for chain in chains.iter().flatten() {
            for pair in chain.windows(2) {
                down[pair[0]].push(pair[1]);
                up[pair[1]].push(pair[0]);
            }
        }
        let depth = ranks.iter().copied().max().map_or(0, |most| most + 1);
        // Nodes in the order given, then each dummy after the vertex its
        // chain comes down from, so the first order already follows the
        // links.
        let mut layers = vec![Vec::new(); depth];
        for v in 0..real {
            layers[ranks[v]].push(v);
        }
        for v in real..count {
            layers[ranks[v]].push(v);
        }
        let mut graph = Self {
            width,
            real,
            rank: ranks,
            chains,
            up,
            down,
            layers,
            pos: vec![0; count],
            x: vec![0.0; count],
        };
        graph.number();
        graph.sweep(true);
        graph
    }

    fn is_real(&self, v: usize) -> bool {
        v < self.real
    }

    /// Sets `pos` from `layers`.
    fn number(&mut self) {
        for layer in &self.layers {
            for (place, &v) in layer.iter().enumerate() {
                self.pos[v] = place;
            }
        }
    }

    /// Crossings between rank `r` and the next.
    fn crossings_below(&self, r: usize) -> usize {
        let mut segments: Vec<(usize, usize)> = Vec::new();
        for &v in &self.layers[r] {
            for &w in &self.down[v] {
                segments.push((self.pos[v], self.pos[w]));
            }
        }
        let mut count = 0;
        for (i, a) in segments.iter().enumerate() {
            for b in &segments[i + 1..] {
                if (a.0 < b.0 && a.1 > b.1) || (a.0 > b.0 && a.1 < b.1) {
                    count += 1;
                }
            }
        }
        count
    }

    fn crossings(&self) -> usize {
        (0..self.layers.len().saturating_sub(1))
            .map(|r| self.crossings_below(r))
            .sum()
    }

    /// Sorts each rank by the mean place of its neighbours in the rank
    /// swept from. A vertex with none keeps its place.
    fn sweep(&mut self, downward: bool) {
        let depth = self.layers.len();
        let ranks: Vec<usize> = if downward {
            (1..depth).collect()
        } else {
            (0..depth.saturating_sub(1)).rev().collect()
        };
        for r in ranks {
            let mut keyed: Vec<(f64, usize, usize)> = (self.layers[r].iter())
                .map(|&v| {
                    let near = if downward { &self.up[v] } else { &self.down[v] };
                    let key = if near.is_empty() {
                        self.pos[v] as f64
                    } else {
                        near.iter().map(|&u| self.pos[u] as f64).sum::<f64>() / near.len() as f64
                    };
                    (key, self.pos[v], v)
                })
                .collect();
            keyed.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            self.layers[r] = keyed.into_iter().map(|(_, _, v)| v).collect();
            for (place, &v) in self.layers[r].iter().enumerate() {
                self.pos[v] = place;
            }
        }
    }

    /// Swaps neighbours in a rank while that lowers the crossings on
    /// either side of it.
    fn transpose(&mut self) {
        let depth = self.layers.len();
        let mut improved = true;
        let mut rounds = 0;
        while improved && rounds < 8 {
            improved = false;
            rounds += 1;
            for r in 0..depth {
                for i in 0..self.layers[r].len().saturating_sub(1) {
                    let before = self.crossings_near(r);
                    self.swap(r, i);
                    if self.crossings_near(r) < before {
                        improved = true;
                    } else {
                        self.swap(r, i);
                    }
                }
            }
        }
    }

    fn swap(&mut self, r: usize, i: usize) {
        self.layers[r].swap(i, i + 1);
        let (a, b) = (self.layers[r][i], self.layers[r][i + 1]);
        self.pos[a] = i;
        self.pos[b] = i + 1;
    }

    fn crossings_near(&self, r: usize) -> usize {
        let above = if r > 0 {
            self.crossings_below(r - 1)
        } else {
            0
        };
        let below = if r + 1 < self.layers.len() {
            self.crossings_below(r)
        } else {
            0
        };
        above + below
    }

    /// Orders every rank to cross as little as it can, keeping the best
    /// order any sweep found.
    fn order(&mut self) {
        let mut best = self.layers.clone();
        let mut fewest = self.crossings();
        let mut idle = 0;
        for index in 0..MAX_SWEEPS {
            if fewest == 0 || idle >= PATIENCE {
                break;
            }
            self.sweep(index % 2 == 1);
            self.transpose();
            let now = self.crossings();
            if now < fewest {
                fewest = now;
                best.clone_from(&self.layers);
                idle = 0;
            } else {
                idle += 1;
            }
        }
        self.layers = best;
        self.number();
    }

    /// The least distance between the centres of `a` and `b`, neighbours
    /// in a rank.
    fn gap(&self, a: usize, b: usize, spacing: Spacing) -> f64 {
        let room = match (self.is_real(a), self.is_real(b)) {
            (true, true) => spacing.node,
            (false, false) => spacing.port,
            _ => spacing.node / 2.0,
        };
        f64::midpoint(self.width[a], self.width[b]) + room
    }

    /// Places each rank across the flow: every vertex as near the mean of
    /// its neighbours as the order and the gaps allow.
    fn place(&mut self, spacing: Spacing) {
        for r in 0..self.layers.len() {
            let mut at = 0.0;
            for i in 0..self.layers[r].len() {
                let v = self.layers[r][i];
                if i > 0 {
                    at += self.gap(self.layers[r][i - 1], v, spacing);
                }
                self.x[v] = at;
            }
        }
        for pass in 0..PLACE_PASSES {
            let depth = self.layers.len();
            let ranks: Vec<usize> = if pass % 3 == 1 {
                (0..depth).rev().collect()
            } else {
                (0..depth).collect()
            };
            for r in ranks {
                let wanted: Vec<f64> = (self.layers[r].iter())
                    .map(|&v| {
                        let near: Vec<usize> = match pass % 3 {
                            0 => self.up[v].clone(),
                            1 => self.down[v].clone(),
                            _ => self.up[v].iter().chain(&self.down[v]).copied().collect(),
                        };
                        // A node pulls harder than a dummy, so screens
                        // linked to each other line up first.
                        let pull = |u: usize| if self.is_real(u) { 3.0 } else { 1.0 };
                        let weight: f64 = near.iter().map(|&u| pull(u)).sum();
                        if near.is_empty() {
                            self.x[v]
                        } else {
                            near.iter().map(|&u| self.x[u] * pull(u)).sum::<f64>() / weight
                        }
                    })
                    .collect();
                let gaps: Vec<f64> = (self.layers[r].windows(2))
                    .map(|pair| self.gap(pair[0], pair[1], spacing))
                    .collect();
                // A dummy pulls harder, so a long edge runs straight.
                let weights: Vec<f64> = (self.layers[r].iter())
                    .map(|&v| if self.is_real(v) { 1.0 } else { 2.0 })
                    .collect();
                let placed = nearest_in_order(&wanted, &gaps, &weights);
                for (&v, x) in self.layers[r].iter().zip(placed) {
                    self.x[v] = x;
                }
            }
        }
        let left = (0..self.x.len())
            .map(|v| self.x[v] - self.width[v] / 2.0)
            .fold(f64::INFINITY, f64::min);
        if left.is_finite() {
            // Every node's near side on a whole unit, so edges in line with
            // each other stay so once placed.
            for v in 0..self.x.len() {
                let half = self.width[v] / 2.0;
                self.x[v] = (self.x[v] - half - left).round() + half;
            }
        }
    }

    /// Lays the ranks along the flow, gives every end its place on its
    /// side and every turn its track, and returns the routes.
    fn route(&self, nodes: &[Node], links: &[Link], reversed: &[bool], spacing: Spacing) -> Layout {
        let depth = self.layers.len();
        let ports = self.ports(spacing);
        // The across-flow point each segment leaves from and arrives at.
        let segment_ends = |chain: &[usize], index: usize, link: usize| -> (f64, f64) {
            let (a, b) = (chain[index], chain[index + 1]);
            let from = if self.is_real(a) {
                ports.leaving[&(a, link)]
            } else {
                self.x[a]
            };
            let to = if self.is_real(b) {
                ports.arriving[&(b, link)]
            } else {
                self.x[b]
            };
            (from, to)
        };

        // The turns in each gap, and the label room each gap needs.
        let mut jogs: Vec<Vec<Jog>> = vec![Vec::new(); depth.saturating_sub(1)];
        let mut label_room = vec![0.0f64; depth.saturating_sub(1)];
        for (link, chain) in self.chains.iter().enumerate() {
            let Some(chain) = chain else { continue };
            for index in 0..chain.len() - 1 {
                let gap = self.rank[chain[index]];
                label_room[gap] = label_room[gap].max(links[link].label);
                let (from, to) = segment_ends(chain, index, link);
                if (from - to).abs() > IN_LINE {
                    jogs[gap].push(Jog {
                        link,
                        from,
                        to,
                        track: 0,
                    });
                }
            }
        }
        let mut tracks = vec![0usize; jogs.len()];
        for (gap, list) in jogs.iter_mut().enumerate() {
            tracks[gap] = assign_tracks(list, spacing.track);
        }

        // Ranks along the flow: each as deep as its deepest node, each gap
        // deep enough for its tracks and labels.
        let mut top = vec![0.0f64; depth];
        let mut thick = vec![0.0f64; depth];
        for v in 0..self.real {
            thick[self.rank[v]] = thick[self.rank[v]].max(nodes[v].h.ceil());
        }
        let mut gap_size = vec![0.0f64; depth.saturating_sub(1)];
        for gap in 0..gap_size.len() {
            let turns = tracks[gap];
            let track_run = turns.saturating_sub(1) as f64 * spacing.track;
            let label = label_room[gap];
            let need = if turns > 0 {
                // A label sits on one of the runs either side of the turns.
                2.0 * (label + spacing.margin) + track_run
            } else {
                label + 2.0 * spacing.margin
            };
            gap_size[gap] = spacing.rank.max(need).ceil();
        }
        for r in 1..depth {
            top[r] = top[r - 1] + thick[r - 1] + gap_size[r - 1];
        }
        // A node sits in the middle of its rank, on a whole unit.
        let node_top = |v: usize| {
            let r = self.rank[v];
            top[r] + ((thick[r] - nodes[v].h) / 2.0).floor()
        };
        let centre_y = |v: usize| node_top(v) + nodes[v].h / 2.0;
        let track_y = |gap: usize, track: usize| {
            let middle = top[gap] + thick[gap] + gap_size[gap] / 2.0;
            middle + (track as f64 - tracks[gap].saturating_sub(1) as f64 / 2.0) * spacing.track
        };
        let mut track_of = std::collections::HashMap::new();
        for (gap, list) in jogs.iter().enumerate() {
            for jog in list {
                track_of.insert((jog.link, gap), jog.track);
            }
        }

        let centres: Vec<(f64, f64)> = (0..self.real).map(|v| (self.x[v], centre_y(v))).collect();
        let routes = (self.chains.iter().enumerate())
            .map(|(link, chain)| {
                let chain = chain.as_ref()?;
                let (first, last) = (chain[0], chain[chain.len() - 1]);
                let mut points = vec![(
                    ports.leaving[&(first, link)],
                    node_top(first) + nodes[first].h,
                )];
                for index in 0..chain.len() - 1 {
                    let gap = self.rank[chain[index]];
                    let (from, to) = segment_ends(chain, index, link);
                    if let Some(&track) = track_of.get(&(link, gap)) {
                        let y = track_y(gap, track);
                        points.push((from, y));
                        points.push((to, y));
                    }
                }
                points.push((ports.arriving[&(last, link)], node_top(last)));
                let mut points = simplify(points);
                if reversed[link] {
                    points.reverse();
                }
                Some(points)
            })
            .collect();
        Layout { centres, routes }
    }

    /// Where each link meets its real ends: spread along the side in the
    /// order of where they lead, each alone on its side brought into line
    /// with its other end where both sides allow.
    fn ports(&self, spacing: Spacing) -> Ports {
        let mut ports = Ports::default();
        // Leaving and arriving ends of each real node, with the across-flow
        // place of the vertex each leads to.
        let mut leaving: Vec<Vec<(f64, usize)>> = vec![Vec::new(); self.real];
        let mut arriving: Vec<Vec<(f64, usize)>> = vec![Vec::new(); self.real];
        for (link, chain) in self.chains.iter().enumerate() {
            let Some(chain) = chain else { continue };
            let (first, last) = (chain[0], chain[chain.len() - 1]);
            leaving[first].push((self.x[chain[1]], link));
            arriving[last].push((self.x[chain[chain.len() - 2]], link));
        }
        let spread = |v: usize, ends: &mut Vec<(f64, usize)>| -> Vec<(usize, f64)> {
            ends.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            let count = ends.len();
            let pitch = port_pitch(self.width[v], count, spacing);
            (ends.iter().enumerate())
                .map(|(index, &(_, link))| {
                    let offset = (index as f64 - (count - 1) as f64 / 2.0) * pitch;
                    (link, self.x[v] + offset)
                })
                .collect()
        };
        for v in 0..self.real {
            for (link, x) in spread(v, &mut leaving[v]) {
                ports.leaving.insert((v, link), x);
            }
            for (link, x) in spread(v, &mut arriving[v]) {
                ports.arriving.insert((v, link), x);
            }
        }
        // Each end then slides along its side towards its edge's other end,
        // keeping the order and the pitch of the ends on that side, so an
        // edge runs straight wherever both its sides allow.
        let range = |v: usize| {
            let half = side_room(self.width[v], spacing) / 2.0;
            (self.x[v] - half, self.x[v] + half)
        };
        let other_end = |ports: &Ports, link: usize, leaves: bool| -> f64 {
            let Some(chain) = &self.chains[link] else {
                return 0.0;
            };
            if leaves {
                let next = chain[1];
                if self.is_real(next) {
                    ports.arriving[&(next, link)]
                } else {
                    self.x[next]
                }
            } else {
                let before = chain[chain.len() - 2];
                if self.is_real(before) {
                    ports.leaving[&(before, link)]
                } else {
                    self.x[before]
                }
            }
        };
        for _ in 0..4 {
            for (sides, leaves) in [(&leaving, true), (&arriving, false)] {
                for (v, ends) in sides.iter().enumerate() {
                    if ends.is_empty() {
                        continue;
                    }
                    let (low, high) = range(v);
                    let wanted: Vec<f64> = (ends.iter())
                        .map(|&(_, link)| other_end(&ports, link, leaves).clamp(low, high))
                        .collect();
                    let pitch = port_pitch(self.width[v], ends.len(), spacing);
                    let gaps = vec![pitch; ends.len() - 1];
                    let weights = vec![1.0; ends.len()];
                    let mut placed = nearest_in_order(&wanted, &gaps, &weights);
                    let (first, last) = (placed[0], placed[placed.len() - 1]);
                    let shift = if first < low {
                        low - first
                    } else if last > high {
                        high - last
                    } else {
                        0.0
                    };
                    for x in &mut placed {
                        *x += shift;
                    }
                    let side = if leaves {
                        &mut ports.leaving
                    } else {
                        &mut ports.arriving
                    };
                    for (&(_, link), x) in ends.iter().zip(placed) {
                        side.insert((v, link), x);
                    }
                }
            }
        }
        ports
    }
}

/// The room between neighbouring ends when `count` share a side of width
/// `width`: the pitch, or less when the side is too short for it.
fn port_pitch(width: f64, count: usize, spacing: Spacing) -> f64 {
    if count > 1 {
        spacing
            .port
            .min(side_room(width, spacing) / (count - 1) as f64)
    } else {
        0.0
    }
}

/// The part of a side of width `width` that ends may spread along.
fn side_room(width: f64, spacing: Spacing) -> f64 {
    (width - 2.0 * spacing.margin).max(0.0)
}

#[derive(Debug, Default)]
struct Ports {
    leaving: std::collections::HashMap<(usize, usize), f64>,
    arriving: std::collections::HashMap<(usize, usize), f64>,
}

/// A turn across a gap: the segment of `link` that comes down at `from`
/// and goes on down at `to`, across on `track`.
#[derive(Debug, Clone, Copy)]
struct Jog {
    link: usize,
    from: f64,
    to: f64,
    track: usize,
}

impl Jog {
    fn span(&self) -> (f64, f64) {
        (self.from.min(self.to), self.from.max(self.to))
    }
}

/// How many times turns `a` and `b` cross when `a` turns nearer the rank
/// above. Each is a line down to its track, across it, and on down.
fn crossings_if_above(a: &Jog, b: &Jog) -> usize {
    let (a_low, a_high) = a.span();
    let (b_low, b_high) = b.span();
    let inside = |x: f64, low: f64, high: f64| x > low && x < high;
    // b's line down to its track passes a's track; a's line on down from
    // its track passes b's track.
    usize::from(inside(b.from, a_low, a_high)) + usize::from(inside(a.to, b_low, b_high))
}

/// Gives each turn in a gap a track, nearest the rank above first, so they
/// cross as little as they can; turns whose runs across do not overlap
/// share one. Returns how many tracks the gap has.
fn assign_tracks(jogs: &mut [Jog], pitch: f64) -> usize {
    if jogs.is_empty() {
        return 0;
    }
    // A turn goes above another when that crosses less; the order is by
    // how many others each one would rather be above.
    let count = jogs.len();
    let mut wins = vec![0usize; count];
    for i in 0..count {
        for j in i + 1..count {
            let (above, below) = (
                crossings_if_above(&jogs[i], &jogs[j]),
                crossings_if_above(&jogs[j], &jogs[i]),
            );
            if above < below {
                wins[i] += 1;
            } else if below < above {
                wins[j] += 1;
            }
        }
    }
    let mut order: Vec<usize> = (0..count).collect();
    order.sort_by(|&a, &b| {
        (wins[b].cmp(&wins[a]))
            .then(jogs[a].from.total_cmp(&jogs[b].from))
            .then(jogs[a].link.cmp(&jogs[b].link))
    });
    let mut used = 0;
    for (place, &i) in order.iter().enumerate() {
        let (low, high) = jogs[i].span();
        let track = (order[..place].iter())
            .filter(|&&j| {
                let (other_low, other_high) = jogs[j].span();
                low < other_high + pitch && other_low < high + pitch
            })
            .map(|&j| jogs[j].track + 1)
            .max()
            .unwrap_or(0);
        jogs[i].track = track;
        used = used.max(track + 1);
    }
    used
}

/// The `x` nearest `wanted` (by weighted squares) that keeps each one at
/// least `gaps[i]` past the one before: pool-adjacent-violators on the
/// places with the gaps taken out.
fn nearest_in_order(wanted: &[f64], gaps: &[f64], weights: &[f64]) -> Vec<f64> {
    let mut offsets = vec![0.0; wanted.len()];
    for i in 1..wanted.len() {
        offsets[i] = offsets[i - 1] + gaps[i - 1];
    }
    // Blocks of (mean, weight, count) whose shifted places are equal.
    let mut blocks: Vec<(f64, f64, usize)> = Vec::new();
    for i in 0..wanted.len() {
        blocks.push((wanted[i] - offsets[i], weights[i], 1));
        while blocks.len() > 1 {
            let last = blocks[blocks.len() - 1];
            let before = blocks[blocks.len() - 2];
            if before.0 <= last.0 {
                break;
            }
            let weight = before.1 + last.1;
            let mean = (before.0 * before.1 + last.0 * last.1) / weight;
            blocks.pop();
            let end = blocks.len() - 1;
            blocks[end] = (mean, weight, before.2 + last.2);
        }
    }
    let mut placed = Vec::with_capacity(wanted.len());
    for (mean, _, count) in blocks {
        for _ in 0..count {
            placed.push(mean + offsets[placed.len()]);
        }
    }
    placed
}

/// `points` without repeats and without the middle of three in line.
fn simplify(points: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    let mut out: Vec<(f64, f64)> = Vec::with_capacity(points.len());
    for point in points {
        if let Some(&last) = out.last()
            && (last.0 - point.0).abs() < IN_LINE
            && (last.1 - point.1).abs() < IN_LINE
        {
            continue;
        }
        if out.len() >= 2 {
            let (a, b) = (out[out.len() - 2], out[out.len() - 1]);
            let vertical = (a.0 - b.0).abs() < IN_LINE && (b.0 - point.0).abs() < IN_LINE;
            let horizontal = (a.1 - b.1).abs() < IN_LINE && (b.1 - point.1).abs() < IN_LINE;
            if vertical || horizontal {
                out.pop();
            }
        }
        out.push(point);
    }
    out
}
