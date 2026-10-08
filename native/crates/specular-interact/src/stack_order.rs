//! The pure math of stack order (ADR 0014): moving a block of items in the
//! one front-to-back list, and keeping each group's members in one run.
//!
//! The list runs back to front, so the last slot is the frontmost.

use std::collections::HashSet;
use std::hash::Hash;

use specular_doc::{Document, ItemId};

use crate::scope::is_group;

/// The four stack-order verbs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Move {
    /// Past the next item in front that is not in the block.
    Forward,
    /// Past the next item behind that is not in the block.
    Backward,
    /// In front of everything.
    ToFront,
    /// Behind everything.
    ToBack,
}

/// A group and its direct members, for [`enforce_contiguity`].
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Run<T> {
    /// The group.
    pub(crate) group: T,
    /// Its direct children, groups included.
    pub(crate) children: Vec<T>,
}

/// `ids` that are in `order`, in the order's sequence.
fn block<T: Clone + Eq + Hash>(order: &[T], ids: &[T]) -> Vec<T> {
    let wanted: HashSet<&T> = ids.iter().collect();
    order
        .iter()
        .filter(|id| wanted.contains(id))
        .cloned()
        .collect()
}

/// `order` with `block` taken out.
fn without<T: Clone + Eq + Hash>(order: &[T], block: &[T]) -> Vec<T> {
    let taken: HashSet<&T> = block.iter().collect();
    order
        .iter()
        .filter(|id| !taken.contains(id))
        .cloned()
        .collect()
}

/// `order` after `how` moves `ids`, which keep their relative order. Ids
/// that are not in `order` are ignored; `order` comes back whole when none is.
pub(crate) fn apply<T: Clone + Eq + Hash>(order: &[T], ids: &[T], how: Move) -> Vec<T> {
    let block = block(order, ids);
    if block.is_empty() {
        return order.to_vec();
    }
    let rest = without(order, &block);
    let insert_at = match how {
        Move::ToFront => rest.len(),
        Move::ToBack => 0,
        Move::Forward => {
            // Just past the nearest item in front of the whole block that is
            // not in it, counted in `rest`.
            let Some(last) = order.iter().rposition(|id| block.contains(id)) else {
                return order.to_vec();
            };
            let Some(anchor) = order[last + 1..].iter().find(|id| !block.contains(id)) else {
                return order.to_vec();
            };
            rest.iter()
                .position(|id| id == anchor)
                .map_or(rest.len(), |at| at + 1)
        }
        Move::Backward => {
            let Some(first) = order.iter().position(|id| block.contains(id)) else {
                return order.to_vec();
            };
            let Some(anchor) = order[..first].iter().rev().find(|id| !block.contains(id)) else {
                return order.to_vec();
            };
            rest.iter().position(|id| id == anchor).unwrap_or(0)
        }
    };
    let mut next = rest;
    next.splice(insert_at..insert_at, block);
    next
}

/// Every descendant of `group`, children first. A parent cycle in a
/// hand-edited file ends at the first repeat.
fn descendants<'a, T: Eq + Hash>(group: &'a T, runs: &'a [Run<T>], seen: &mut HashSet<&'a T>) {
    if !seen.insert(group) {
        return;
    }
    if let Some(run) = runs.iter().find(|run| run.group == *group) {
        for child in &run.children {
            descendants(child, runs, seen);
        }
    }
}

/// `order` with each group's descendants and the group itself gathered into
/// one run. The run sits where its frontmost member was and the group's id
/// is its frontmost slot, so the group's stack position is its own. Members
/// keep their relative order.
pub(crate) fn enforce_contiguity<T: Clone + Eq + Hash>(order: &[T], runs: &[Run<T>]) -> Vec<T> {
    let mut next = order.to_vec();
    for run in runs {
        let mut family: HashSet<&T> = HashSet::new();
        for child in &run.children {
            descendants(child, runs, &mut family);
        }
        family.remove(&run.group);
        let mut ids: Vec<T> = family.into_iter().cloned().collect();
        ids.push(run.group.clone());
        let members = block(&next, &ids);
        if members.len() <= 1 {
            continue;
        }
        let Some(frontmost) = next.iter().rposition(|id| members.contains(id)) else {
            continue;
        };
        let rest = without(&next, &members);
        let at = (frontmost + 1)
            .saturating_sub(members.len())
            .min(rest.len());
        let children: Vec<&T> = members.iter().filter(|id| **id != run.group).collect();
        next = rest[..at]
            .iter()
            .chain(children)
            .chain(std::iter::once(&run.group))
            .chain(&rest[at..])
            .cloned()
            .collect();
    }
    next
}

/// Every group of `document` with its direct members, for
/// [`enforce_contiguity`].
pub(crate) fn document_runs(document: &Document) -> Vec<Run<ItemId>> {
    document
        .entities()
        .filter(|entity| is_group(entity))
        .map(|group| Run {
            group: ItemId::Entity(group.id.clone()),
            children: (document.children(&group.id))
                .map(|child| ItemId::Entity(child.id.clone()))
                .collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(text: &str) -> Vec<char> {
        text.chars().collect()
    }

    fn moved(order: &str, block: &str, how: Move) -> String {
        apply(&ids(order), &ids(block), how).into_iter().collect()
    }

    #[test]
    fn a_multi_selection_moves_forward_as_one_block() {
        assert_eq!(moved("abcd", "bc", Move::Forward), "adbc");

        {
            assert_eq!(moved("abcd", "bc", Move::Backward), "bcad");
        }

        {
            assert_eq!(moved("abcd", "bd", Move::ToFront), "acbd");
            assert_eq!(moved("abcd", "db", Move::ToFront), "acbd");
        }

        {
            assert_eq!(moved("abcd", "bd", Move::ToBack), "bdac");
        }

        {
            assert_eq!(moved("abcd", "d", Move::Forward), "abcd");
            assert_eq!(moved("abcd", "a", Move::Backward), "abcd");
            assert_eq!(moved("abcd", "cd", Move::Forward), "abcd");
            assert_eq!(moved("abcd", "ab", Move::Backward), "abcd");
        }

        {
            assert_eq!(moved("abcde", "ac", Move::Forward), "bdace");
        }

        {
            assert_eq!(moved("abc", "z", Move::ToFront), "abc");
        }
    }

    #[test]
    fn nested_group_runs_stay_contiguous_after_a_move() {
        let runs = [
            Run {
                group: 'O',
                children: ids("aI"),
            },
            Run {
                group: 'I',
                children: ids("b"),
            },
        ];
        let order = apply(&ids("aOxbIy"), &ids("O"), Move::Forward);
        let next: String = enforce_contiguity(&order, &runs).into_iter().collect();
        assert_eq!(next, "xabIOy");
    }
}
