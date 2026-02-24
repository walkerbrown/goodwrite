use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, LazyLock, Mutex},
};

use goodwrite_core::GlossaryFileData;

use super::{Alternative, Dictionary, data::DICTIONARY_TOML};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookupAlternative {
    pub word: String,
    pub pos: Option<String>,
    pub context: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DictionaryPosCandidate {
    pub lemma: String,
    pub pos: String,
    pub approved: bool,
}

#[derive(Debug)]
pub struct DictionaryLookup {
    approved: HashSet<String>,
    approved_forms: HashSet<String>,
    approved_pos: HashMap<String, HashSet<String>>,
    form_to_lemma: HashMap<String, String>,
    form_pos_candidates: HashMap<String, Vec<DictionaryPosCandidate>>,
    not_approved: HashMap<String, Vec<LookupAlternative>>,
    not_approved_by_pos: HashMap<(String, String), Vec<LookupAlternative>>,
    not_approved_pos: HashMap<String, HashSet<String>>,
}

impl DictionaryLookup {
    pub fn global() -> &'static Self {
        static INSTANCE: LazyLock<DictionaryLookup> = LazyLock::new(|| build_lookup(None));

        &INSTANCE
    }

    pub fn for_overlay(overlay: Option<&GlossaryFileData>) -> Arc<Self> {
        static BASE: LazyLock<Arc<DictionaryLookup>> =
            LazyLock::new(|| Arc::new(build_lookup(None)));
        static CACHE: LazyLock<Mutex<HashMap<String, Arc<DictionaryLookup>>>> =
            LazyLock::new(|| Mutex::new(HashMap::new()));

        let Some(overlay) = overlay else {
            return BASE.clone();
        };

        let signature = overlay_signature(overlay);
        if let Some(existing) = CACHE.lock().expect("overlay cache lock").get(&signature) {
            return existing.clone();
        }

        let lookup = Arc::new(build_lookup(Some(overlay)));
        CACHE
            .lock()
            .expect("overlay cache lock")
            .insert(signature, lookup.clone());
        lookup
    }

    pub fn validate_overlay_against_embedded(overlay: &GlossaryFileData) -> Result<(), String> {
        let base = DictionaryLookup::global();

        for entry in &overlay.approved {
            let word = entry.word.trim();
            if word.is_empty() {
                continue;
            }
            if base.is_known_word(word) {
                return Err(format!(
                    "glossary approved entry `{word}` conflicts with bundled STE dictionary"
                ));
            }

            for form in &entry.forms {
                let form = form.trim();
                if form.is_empty() {
                    continue;
                }
                if base.is_known_word(form) {
                    return Err(format!(
                        "glossary approved form `{form}` conflicts with bundled STE dictionary"
                    ));
                }
            }
        }

        for entry in &overlay.not_approved {
            let word = entry.word.trim();
            if word.is_empty() {
                continue;
            }
            if base.is_known_word(word) {
                return Err(format!(
                    "glossary not_approved entry `{word}` conflicts with bundled STE dictionary"
                ));
            }
        }

        Ok(())
    }

    pub fn is_approved(&self, word: &str) -> bool {
        self.approved.contains(&word.to_ascii_lowercase())
    }

    pub fn is_approved_form(&self, word: &str) -> bool {
        self.approved_forms.contains(&word.to_ascii_lowercase())
    }

    pub fn is_known_word(&self, word: &str) -> bool {
        let lower = word.to_ascii_lowercase();
        self.approved_forms.contains(&lower) || self.not_approved.contains_key(&lower)
    }

    pub fn lemma_for_form(&self, word: &str) -> Option<&str> {
        self.form_to_lemma
            .get(&word.to_ascii_lowercase())
            .map(String::as_str)
    }

    pub fn alternatives_for_word(&self, word: &str) -> Option<&[LookupAlternative]> {
        self.not_approved
            .get(&word.to_ascii_lowercase())
            .map(Vec::as_slice)
    }

    pub fn alternatives_for_word_by_pos(
        &self,
        word: &str,
        pos_tag: &str,
    ) -> Option<&[LookupAlternative]> {
        let key = (
            word.to_ascii_lowercase(),
            normalize_pos(&pos_tag.to_ascii_lowercase()),
        );
        self.not_approved_by_pos.get(&key).map(Vec::as_slice)
    }

    pub fn pos_candidates_for(&self, word: &str) -> Option<&[DictionaryPosCandidate]> {
        self.form_pos_candidates
            .get(&word.to_ascii_lowercase())
            .map(Vec::as_slice)
    }

    pub fn allowed_pos(&self, word: &str) -> Option<&HashSet<String>> {
        let lemma = self.lemma_for_form(word)?;
        self.approved_pos.get(lemma)
    }

    pub fn known_non_approved(&self, word: &str) -> bool {
        self.not_approved.contains_key(&word.to_ascii_lowercase())
    }

    pub fn known_non_approved_with_pos_tag(&self, word: &str, pos_tag: &str) -> bool {
        self.not_approved_by_pos.contains_key(&(
            word.to_ascii_lowercase(),
            normalize_pos(&pos_tag.to_ascii_lowercase()),
        ))
    }

    pub fn non_approved_pos_for_word(&self, word: &str) -> Option<&HashSet<String>> {
        self.not_approved_pos.get(&word.to_ascii_lowercase())
    }
}

fn build_lookup(overlay: Option<&GlossaryFileData>) -> DictionaryLookup {
    let parsed: Dictionary = toml::from_str(DICTIONARY_TOML)
        .unwrap_or_else(|error| panic!("dictionary.toml is valid: {error}"));

    let _ = parsed.notice.len();

    let mut approved = HashSet::new();
    let mut approved_forms = HashSet::new();
    let mut approved_pos: HashMap<String, HashSet<String>> = HashMap::new();
    let mut form_to_lemma = HashMap::new();
    let mut form_pos_candidates: HashMap<String, Vec<DictionaryPosCandidate>> = HashMap::new();
    let mut not_approved: HashMap<String, Vec<LookupAlternative>> = HashMap::new();
    let mut not_approved_by_pos: HashMap<(String, String), Vec<LookupAlternative>> = HashMap::new();
    let mut not_approved_pos: HashMap<String, HashSet<String>> = HashMap::new();

    for entry in parsed.approved {
        insert_approved_entry(
            &mut approved,
            &mut approved_forms,
            &mut approved_pos,
            &mut form_to_lemma,
            &mut form_pos_candidates,
            &entry.word,
            &entry.pos,
            &entry.forms,
        );
    }

    for entry in parsed.not_approved {
        let alternatives = entry
            .alternatives
            .into_iter()
            .map(|alternative| match alternative {
                Alternative::Word(word) => LookupAlternative {
                    word: word.to_ascii_lowercase(),
                    pos: None,
                    context: None,
                },
                Alternative::Detailed { word, pos, context } => LookupAlternative {
                    word: word.to_ascii_lowercase(),
                    pos: pos.map(|value| normalize_pos(&value)),
                    context,
                },
            })
            .collect::<Vec<_>>();

        insert_not_approved_entry(
            &mut not_approved,
            &mut not_approved_by_pos,
            &mut not_approved_pos,
            &mut form_pos_candidates,
            &entry.word,
            &entry.pos,
            &alternatives,
        );
    }

    if let Some(overlay) = overlay {
        for entry in &overlay.approved {
            insert_approved_entry(
                &mut approved,
                &mut approved_forms,
                &mut approved_pos,
                &mut form_to_lemma,
                &mut form_pos_candidates,
                &entry.word,
                &entry.pos,
                &entry.forms,
            );
        }

        for entry in &overlay.not_approved {
            let alternatives = entry
                .alternatives
                .iter()
                .map(|alternative| LookupAlternative {
                    word: alternative.word.to_ascii_lowercase(),
                    pos: alternative.pos.as_ref().map(|value| normalize_pos(value)),
                    context: alternative.context.clone(),
                })
                .collect::<Vec<_>>();
            insert_not_approved_entry(
                &mut not_approved,
                &mut not_approved_by_pos,
                &mut not_approved_pos,
                &mut form_pos_candidates,
                &entry.word,
                &entry.pos,
                &alternatives,
            );
        }
    }

    DictionaryLookup {
        approved,
        approved_forms,
        approved_pos,
        form_to_lemma,
        form_pos_candidates,
        not_approved,
        not_approved_by_pos,
        not_approved_pos,
    }
}

#[allow(clippy::too_many_arguments)]
fn insert_approved_entry(
    approved: &mut HashSet<String>,
    approved_forms: &mut HashSet<String>,
    approved_pos: &mut HashMap<String, HashSet<String>>,
    form_to_lemma: &mut HashMap<String, String>,
    form_pos_candidates: &mut HashMap<String, Vec<DictionaryPosCandidate>>,
    word: &str,
    pos: &str,
    forms: &[String],
) {
    let lemma = word.trim().to_ascii_lowercase();
    if lemma.is_empty() {
        return;
    }
    let normalized_pos = normalize_pos(pos);

    approved.insert(lemma.clone());
    approved_forms.insert(lemma.clone());
    form_to_lemma.insert(lemma.clone(), lemma.clone());
    approved_pos
        .entry(lemma.clone())
        .or_default()
        .insert(normalized_pos.clone());
    push_pos_candidate(
        form_pos_candidates,
        &lemma,
        DictionaryPosCandidate {
            lemma: lemma.clone(),
            pos: normalized_pos.clone(),
            approved: true,
        },
    );

    for form in forms {
        let form_lower = form.trim().to_ascii_lowercase();
        if form_lower.is_empty() {
            continue;
        }
        approved_forms.insert(form_lower.clone());
        form_to_lemma.insert(form_lower.clone(), lemma.clone());
        push_pos_candidate(
            form_pos_candidates,
            &form_lower,
            DictionaryPosCandidate {
                lemma: lemma.clone(),
                pos: normalized_pos.clone(),
                approved: true,
            },
        );
    }
}

fn insert_not_approved_entry(
    not_approved: &mut HashMap<String, Vec<LookupAlternative>>,
    not_approved_by_pos: &mut HashMap<(String, String), Vec<LookupAlternative>>,
    not_approved_pos: &mut HashMap<String, HashSet<String>>,
    form_pos_candidates: &mut HashMap<String, Vec<DictionaryPosCandidate>>,
    word: &str,
    pos: &str,
    alternatives: &[LookupAlternative],
) {
    let word = word.trim().to_ascii_lowercase();
    if word.is_empty() {
        return;
    }
    let pos = normalize_pos(pos);

    not_approved_pos
        .entry(word.clone())
        .or_default()
        .insert(pos.clone());

    let keyed = not_approved_by_pos
        .entry((word.clone(), pos.clone()))
        .or_default();
    for alternative in alternatives {
        push_unique_alternative(keyed, alternative.clone());
    }

    let merged = not_approved.entry(word.clone()).or_default();
    for alternative in alternatives {
        push_unique_alternative(merged, alternative.clone());
    }

    push_pos_candidate(
        form_pos_candidates,
        &word,
        DictionaryPosCandidate {
            lemma: word.clone(),
            pos,
            approved: false,
        },
    );
}

fn overlay_signature(overlay: &GlossaryFileData) -> String {
    let mut segments = Vec::new();

    for entry in &overlay.approved {
        let mut forms = entry
            .forms
            .iter()
            .map(|value| value.trim().to_ascii_lowercase())
            .collect::<Vec<_>>();
        forms.sort();
        segments.push(format!(
            "a:{}:{}:{}",
            entry.word.trim().to_ascii_lowercase(),
            entry.pos.trim().to_ascii_lowercase(),
            forms.join(",")
        ));
    }

    for entry in &overlay.not_approved {
        let mut alternatives = entry
            .alternatives
            .iter()
            .map(|alt| {
                format!(
                    "{}:{}:{}",
                    alt.word.trim().to_ascii_lowercase(),
                    alt.pos
                        .as_ref()
                        .map(|v| v.trim().to_ascii_lowercase())
                        .unwrap_or_default(),
                    alt.context
                        .as_ref()
                        .map(|v| v.trim().to_ascii_lowercase())
                        .unwrap_or_default()
                )
            })
            .collect::<Vec<_>>();
        alternatives.sort();
        segments.push(format!(
            "n:{}:{}:{}",
            entry.word.trim().to_ascii_lowercase(),
            entry.pos.trim().to_ascii_lowercase(),
            alternatives.join(",")
        ));
    }

    segments.sort();
    segments.join("|")
}

fn normalize_pos(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "n" | "nn" | "noun" => "noun".to_string(),
        "v" | "vb" | "verb" => "verb".to_string(),
        "adj" | "jj" | "adjective" => "adjective".to_string(),
        "adv" | "rb" | "adverb" => "adverb".to_string(),
        "det" | "article" | "determiner" => "determiner".to_string(),
        "modal" | "md" => "modal".to_string(),
        "prep" | "preposition" => "preposition".to_string(),
        "conj" | "conjunction" => "conjunction".to_string(),
        "pron" | "pronoun" => "pronoun".to_string(),
        "tn" | "technical-noun" => "technical-noun".to_string(),
        "tv" | "technical-verb" => "technical-verb".to_string(),
        "participle" | "ptcp" => "participle".to_string(),
        "num" | "number" => "number".to_string(),
        other => other.to_string(),
    }
}

fn push_unique_alternative(target: &mut Vec<LookupAlternative>, value: LookupAlternative) {
    if target.iter().any(|existing| existing == &value) {
        return;
    }
    target.push(value);
}

fn push_pos_candidate(
    target: &mut HashMap<String, Vec<DictionaryPosCandidate>>,
    key: &str,
    candidate: DictionaryPosCandidate,
) {
    let bucket = target.entry(key.to_string()).or_default();
    if bucket.iter().any(|existing| existing == &candidate) {
        return;
    }
    bucket.push(candidate);
}

#[cfg(test)]
mod tests {
    use goodwrite_core::{
        GlossaryAlternative, GlossaryApprovedEntry, GlossaryFileData, GlossaryNotApprovedEntry,
    };

    use super::DictionaryLookup;

    #[test]
    fn rejects_overlay_when_approved_entry_redefines_embedded_word() {
        let overlay = GlossaryFileData {
            approved: vec![GlossaryApprovedEntry {
                word: "use".to_string(),
                pos: "verb".to_string(),
                forms: Vec::new(),
                approved_meaning: String::new(),
                goodwrite_example: String::new(),
                wrongwrite_example: String::new(),
            }],
            not_approved: Vec::new(),
        };

        let result = DictionaryLookup::validate_overlay_against_embedded(&overlay);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_overlay_when_approved_form_redefines_embedded_word() {
        let overlay = GlossaryFileData {
            approved: vec![GlossaryApprovedEntry {
                word: "telemeter".to_string(),
                pos: "verb".to_string(),
                forms: vec!["use".to_string()],
                approved_meaning: String::new(),
                goodwrite_example: String::new(),
                wrongwrite_example: String::new(),
            }],
            not_approved: Vec::new(),
        };

        let result = DictionaryLookup::validate_overlay_against_embedded(&overlay);
        assert!(result.is_err());
    }

    #[test]
    fn accepts_overlay_when_entries_do_not_conflict_with_embedded_dictionary() {
        let overlay = GlossaryFileData {
            approved: vec![GlossaryApprovedEntry {
                word: "telemeter".to_string(),
                pos: "verb".to_string(),
                forms: vec!["telemeters".to_string(), "telemetered".to_string()],
                approved_meaning: String::new(),
                goodwrite_example: String::new(),
                wrongwrite_example: String::new(),
            }],
            not_approved: vec![GlossaryNotApprovedEntry {
                word: "telemetering".to_string(),
                pos: "verb".to_string(),
                alternatives: vec![GlossaryAlternative {
                    word: "telemeter".to_string(),
                    pos: Some("verb".to_string()),
                    context: Some("present tense".to_string()),
                }],
                approved_meaning: String::new(),
                goodwrite_example: String::new(),
                wrongwrite_example: String::new(),
            }],
        };

        let result = DictionaryLookup::validate_overlay_against_embedded(&overlay);
        assert!(result.is_ok(), "{result:?}");
    }
}
