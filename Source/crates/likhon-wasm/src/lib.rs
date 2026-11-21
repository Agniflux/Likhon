//! The engine in the browser (Web/Laravel/public/js/tools.js and later the typing boxes). Pure functions over text:
//! nothing is sent anywhere, there is no I/O.

use wasm_bindgen::prelude::*;

/// Bangla Unicode to Bijoy (the legacy ASCII encoding), as the desktop's legacy output does it.
#[wasm_bindgen]
pub fn unicode_to_bijoy(text: &str) -> String {
    likhon_engine::unicode_to_bijoy(text)
}

/// Bijoy text to Bangla Unicode.
#[wasm_bindgen]
pub fn bijoy_to_unicode(text: &str) -> String {
    likhon_engine::bijoy_to_unicode(text)
}

/// Bangla Unicode to Proshika (the Lipi fonts' legacy encoding), as the desktop's legacy output does it.
#[wasm_bindgen]
pub fn unicode_to_proshika(text: &str) -> String {
    likhon_engine::unicode_to_proshika(text)
}

/// Proshika text to Bangla Unicode.
#[wasm_bindgen]
pub fn proshika_to_unicode(text: &str) -> String {
    likhon_engine::proshika_to_unicode(text)
}

/// Phonetic (Latin letters) to Bangla.
#[wasm_bindgen]
pub fn phonetic(text: &str) -> String {
    likhon_engine::phonetic(text)
}

/// The phonetic keys that type a Bangla word (the Tools page's "How to type it"), or "" when there is no exact way.
#[wasm_bindgen]
pub fn how_to_type(word: &str) -> String {
    likhon_engine::tools::how_to_type(word).unwrap_or_default()
}

/// Khipro keys to Bangla, word by word (Space ends a word, as it does when typing).
#[wasm_bindgen]
pub fn khipro(text: &str) -> String {
    let im = likhon_engine::mit::khipro::classic();
    text.split(' ').map(|w| im.convert(w)).collect::<Vec<_>>().join(" ")
}
