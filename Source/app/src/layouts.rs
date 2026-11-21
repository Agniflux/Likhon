//! The layouts built into the exe: Probhat and Jatiya, JSON layout files (shared/schemas/layout.schema.json) converted
//! from Avro Keyboard's MPL-2.0 files (assets/layouts/avro/, SOURCE.md there). Phonetic is not a file; it is the
//! engine's own. Every other layout is added from the library (owner, 2026-10-23): JSON layouts become files in the
//! data folder's layouts/ (sideload.rs), and input methods such as Khipro become layouts/<id>.mim, typed by the
//! engine's m17n interpreter (crates/likhon-engine/src/mit).

use likhon_engine::fixed::Layout;

/// Layout ids in the order the top bar and Settings list them (phonetic first, then these).
pub const BUILT_IN: &[(&str, &str)] = &[
    ("probhat", include_str!("../../assets/layouts/avro/probhat.json")),
    ("jatiya", include_str!("../../assets/layouts/avro/jatiya.json")),
];

/// An input method added from the library (layouts/<id>.mim), parsed once per id.
pub fn input_method(id: &str) -> Option<std::sync::Arc<likhon_engine::mit::khipro::InputMethod>> {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    static CACHE: Mutex<Option<HashMap<String, Arc<likhon_engine::mit::khipro::InputMethod>>>> = Mutex::new(None);
    if !id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-') || id.is_empty() {
        return None;
    }
    let mut cache = CACHE.lock().ok()?;
    let map = cache.get_or_insert_with(HashMap::new);
    if let Some(im) = map.get(id) {
        return Some(im.clone());
    }
    let text = std::fs::read_to_string(crate::settings::dir().0.join("layouts").join(format!("{id}.mim"))).ok()?;
    let im = Arc::new(likhon_engine::mit::khipro::InputMethod::parse(&text));
    map.insert(id.to_string(), im.clone());
    Some(im)
}

/// Layouts added from the library or a file, in name order: (id, English name, Bangla name).
pub fn added() -> Vec<(String, String, Option<String>)> {
    let mut out: Vec<(String, String, Option<String>)> = crate::sideload::list("layouts")
        .into_iter()
        .map(|(id, v)| (id.clone(), v["name"]["en"].as_str().unwrap_or(&id).to_string(), v["name"]["bn"].as_str().map(String::from)))
        .collect();
    out.extend(crate::sideload::input_methods());
    out.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));
    out
}

/// The layout for an id (built in, or added from a file), or None for phonetic, an input method or an unknown id (the
/// agent then types phonetic, or the input method).
pub fn load(id: &str) -> Option<Layout> {
    BUILT_IN.iter().find(|(i, _)| *i == id).and_then(|(_, text)| Layout::from_json(text).ok()).or_else(|| crate::sideload::layout(id))
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_built_in_layout_loads() {
        for (id, _) in super::BUILT_IN {
            assert_eq!(super::load(id).map(|l| l.id), Some(id.to_string()));
        }
        assert!(super::load("phonetic").is_none());
    }
}
