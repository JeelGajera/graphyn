//! Turning a tool's `kinds` argument into a traversal mask.
//!
//! Shared by every query tool so an agent gets one vocabulary and one error
//! message rather than three that drifted apart.

use graphyn_core::query::{self, RelationshipKindMask};

/// Documentation for the `kinds` argument, kept in one place because it is
/// repeated into three tool schemas and would otherwise diverge.
pub const KINDS_DOC: &str = "Optional: only follow these relationship kinds. \
One or more of: imports, calls, extends, implements, uses-type, \
accesses-property, re-exports, instantiates. Omit to follow every kind. \
Note that 'calls' and 'instantiates' are not currently emitted by any \
language adapter and will match nothing.";

/// Build a mask from tool arguments.
///
/// An unknown name is an error rather than something to ignore. Silently
/// dropping it would answer a narrower question than the caller asked and
/// present the result as if it were the answer to theirs — which, in a tool
/// used to decide whether a change is safe, is the worst available outcome.
pub fn mask_from_names(names: &Option<Vec<String>>) -> Result<RelationshipKindMask, String> {
    let Some(names) = names else {
        return Ok(RelationshipKindMask::all());
    };
    if names.is_empty() {
        return Ok(RelationshipKindMask::all());
    }

    let mut mask = RelationshipKindMask::none();
    for name in names {
        let kind = query::parse_kind(name).ok_or_else(|| {
            let known: Vec<&str> = query::ALL_KINDS.iter().map(query::kind_name).collect();
            format!(
                "unknown relationship kind '{name}'. Known kinds: {}",
                known.join(", ")
            )
        })?;
        mask = mask.with(kind);
    }
    Ok(mask)
}

/// A warning to prepend when a caller asked only for kinds nothing emits.
///
/// Without it the reply is an empty result that reads as "nothing depends on
/// this" — the exact false reassurance Graphyn exists to prevent.
pub fn unemitted_warning(mask: &RelationshipKindMask) -> Option<String> {
    if mask.is_all() {
        return None;
    }
    let requested = mask.kinds();
    if requested.is_empty() {
        return None;
    }

    let dead: Vec<&str> = requested
        .iter()
        .filter(|k| query::UNEMITTED_KINDS.contains(k))
        .map(query::kind_name)
        .collect();
    if dead.is_empty() {
        return None;
    }

    let all_dead = dead.len() == requested.len();
    Some(if all_dead {
        format!(
            "NOTE: every requested kind ({}) is unimplemented — no adapter emits it, \
             so this result is empty regardless of the code. This is not evidence \
             that nothing depends on the symbol.",
            dead.join(", ")
        )
    } else {
        format!(
            "NOTE: {} is unimplemented — no adapter emits it, so it contributed \
             nothing to this result.",
            dead.join(", ")
        )
    })
}
