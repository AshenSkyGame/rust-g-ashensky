use once_cell::sync::Lazy;
use petrovich::{Case, Gender, firstname, lastname};
use std::collections::HashMap;
use std::fs;
use std::sync::Mutex;

static DICTIONARY: Lazy<Mutex<HashMap<String, HashMap<String, String>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

byond_fn! { fn sklonenie_load_dict(path) {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Some("ERROR: File not found".to_string()),
    };
    let dict: HashMap<String, HashMap<String, String>> = match serde_json::from_str(&content) {
        Ok(d) => d,
        Err(_) => return Some("ERROR: Invalid JSON".to_string()),
    };

    let mut lock = DICTIONARY.lock().unwrap();
    *lock = dict;
    Some("OK".to_string())
} }

byond_fn! { fn sklonenie_word(word, rule_str, gender_str, is_mob_str) {
    let rule = rule_str;
    let is_mob = is_mob_str == "1" || is_mob_str == "true";

    // 1. ALWAYS check the dictionary first, regardless of is_mob
    {
        let lock = DICTIONARY.lock().unwrap();
        if let Some(declined) = lock.get(word).and_then(|cases| cases.get(rule)) {
            return Some(declined.clone());
        }
    }

    // 2. If it's a mob and NOT in the dictionary, use Petrovich
    if is_mob {
        let gender = match gender_str {
            "male" => Gender::Male,
            "female" => Gender::Female,
            _ => Gender::Androgynous,
        };
        let case = match rule {
            "1" => Some(Case::Genitive),
            "2" => Some(Case::Dative),
            "3" => Some(Case::Accusative),
            "4" => Some(Case::Instrumental),
            "5" => Some(Case::Prepositional),
            _ => None,
        };

        if let Some(c) = case {
            let parts: Vec<&str> = word.split_whitespace().collect();
            if parts.len() == 1 {
                return Some(firstname(gender, parts[0], c));
            } else if parts.len() >= 2 {
                let first = firstname(gender, parts[0], c);
                let last = lastname(gender, parts[1], c);
                return Some(format!("{} {}", first, last));
            }
        }
    }

    // Fallback: return the original word if no rule found
    Some(word.to_string())
} }
