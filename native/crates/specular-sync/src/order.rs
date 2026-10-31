//! Fractional order keys: strings that sort in stack order, so that a peer
//! can put an item between two others without renumbering the rest.
//!
//! Two peers that insert at the same place at the same time can mint the
//! same key. Items sort by key and then by id, so every peer still agrees on
//! one order, and the next [`assign`] gives the tied items distinct keys.

/// The digits, in ASCII order so that keys compare as plain strings.
const DIGITS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// A key that sorts after `before` and before `after`. `None` is the start
/// or the end of the stack. `before` must sort strictly before `after`.
pub(crate) fn between(before: Option<&str>, after: Option<&str>) -> String {
    debug_assert!(
        match (before, after) {
            (Some(before), Some(after)) => before < after,
            _ => true,
        },
        "keys out of order: {before:?} {after:?}"
    );
    midpoint(before.unwrap_or("").as_bytes(), after.map(str::as_bytes))
}

/// A string between `low` and `high` (`None` is past every key) that does not
/// end in the zero digit, so there is always room before it.
fn midpoint(low: &[u8], high: Option<&[u8]>) -> String {
    if let Some(high) = high {
        let shared = (0..high.len())
            .take_while(|&i| low.get(i).copied().unwrap_or(DIGITS[0]) == high[i])
            .count();
        if shared > 0 {
            let rest = midpoint(low.get(shared..).unwrap_or(&[]), Some(&high[shared..]));
            return format!("{}{rest}", ascii(&high[..shared]));
        }
    }
    let low_digit = low.first().map_or(0, |&byte| digit(byte));
    let high_digit = high
        .and_then(<[u8]>::first)
        .map_or(DIGITS.len(), |&byte| digit(byte));
    if high_digit - low_digit > 1 {
        return char::from(DIGITS[usize::midpoint(low_digit, high_digit)]).to_string();
    }
    match high {
        Some(high) if high.len() > 1 => ascii(&high[..1]),
        _ => {
            let rest = midpoint(low.get(1..).unwrap_or(&[]), None);
            format!("{}{rest}", char::from(DIGITS[low_digit]))
        }
    }
}

fn digit(byte: u8) -> usize {
    DIGITS.iter().position(|&d| d == byte).unwrap_or(0)
}

fn ascii(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// New keys for a list in its wanted order, given each item's current key
/// (`None` for an item that has none yet). Returns `(index, key)` for the
/// items whose key has to change, keeping as many current keys as it can so
/// that a reorder touches only the items that moved.
pub(crate) fn assign(current: &[Option<&str>]) -> Vec<(usize, String)> {
    let kept = longest_increasing(current);
    let mut changes = Vec::new();
    let mut previous: Option<String> = None;
    for (index, key) in current.iter().enumerate() {
        if kept[index] {
            previous = key.map(str::to_owned);
            continue;
        }
        let next = (index + 1..current.len())
            .find(|&i| kept[i])
            .and_then(|i| current[i]);
        let key = between(previous.as_deref(), next);
        previous = Some(key.clone());
        changes.push((index, key));
    }
    changes
}

/// Marks the longest run of keys, in list order, that strictly increase.
/// Those can stay; every other item gets a new key.
fn longest_increasing(keys: &[Option<&str>]) -> Vec<bool> {
    // `tails[k]` is the index of the smallest last key of a run of length k + 1.
    let mut tails: Vec<usize> = Vec::new();
    let mut previous: Vec<Option<usize>> = vec![None; keys.len()];
    for (index, key) in keys.iter().enumerate() {
        let Some(key) = key else { continue };
        let length = tails.partition_point(|&tail| keys[tail].is_some_and(|tail| tail < *key));
        previous[index] = length.checked_sub(1).map(|k| tails[k]);
        if length == tails.len() {
            tails.push(index);
        } else {
            tails[length] = index;
        }
    }
    let mut kept = vec![false; keys.len()];
    let mut next = tails.last().copied();
    while let Some(index) = next {
        kept[index] = true;
        next = previous[index];
    }
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_between_any_two_sort_between_them() {
        let cases = [
            (None, None),
            (None, Some("V")),
            (Some("V"), None),
            (Some("V"), Some("W")),
            (Some("V"), Some("V1")),
            (Some("Vz"), Some("W")),
            (Some("0001"), Some("001")),
            (None, Some("01")),
            (Some("zzz"), None),
        ];
        for (before, after) in cases {
            let key = between(before, after);
            assert!(
                before.is_none_or(|b| b < key.as_str()),
                "{before:?} < {key}"
            );
            assert!(after.is_none_or(|a| key.as_str() < a), "{key} < {after:?}");
            assert!(!key.ends_with('0'), "{key} ends in zero");
        }
    }

    #[test]
    fn a_reorder_rekeys_only_what_moved() {
        // a b c d e, with c moved to the front.
        let changes = assign(&[Some("c"), Some("a"), Some("b"), Some("d"), Some("e")]);
        assert_eq!(changes.len(), 1);
        let (index, key) = &changes[0];
        assert_eq!(*index, 0);
        assert!(key.as_str() < "a");

        // New items and tied keys get keys that fit between their neighbours.
        let current = [Some("a"), None, Some("b"), Some("b"), None];
        let mut keys: Vec<String> = current.iter().map(|k| k.unwrap_or("").to_owned()).collect();
        for (index, key) in assign(&current) {
            keys[index] = key;
        }
        assert!(keys.windows(2).all(|pair| pair[0] < pair[1]), "{keys:?}");
    }
}
