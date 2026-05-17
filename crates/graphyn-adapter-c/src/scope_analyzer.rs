use std::collections::BTreeSet;

pub fn extract_member_accesses(source: &str, aliases: &[String]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for alias in aliases {
        for needle in [format!("{alias}->"), format!("{alias}.")] {
            for line in source.lines() {
                let mut rest = line;
                while let Some(idx) = rest.find(&needle) {
                    let after = &rest[idx + needle.len()..];
                    let prop: String = after
                        .chars()
                        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                        .collect();
                    if !prop.is_empty() {
                        out.insert(prop);
                    }
                    rest = after;
                }
            }
        }
    }
    out
}
