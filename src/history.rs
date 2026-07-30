#[derive(Debug, Eq, PartialEq)]
pub struct History {
    total_words: u64,
    last_transcript: String,
}

impl History {
    pub fn new(total_words: u64) -> Self {
        Self {
            total_words,
            last_transcript: String::new(),
        }
    }

    pub fn total_words(&self) -> u64 {
        self.total_words
    }

    pub fn last_transcript(&self) -> &str {
        &self.last_transcript
    }

    pub fn record_success(&mut self, transcript: String) {
        let word_count = transcript.split_whitespace().count() as u64;
        if word_count == 0 {
            return;
        }
        self.total_words = self.total_words.saturating_add(word_count);
        self.last_transcript = transcript;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Settings, SettingsStore};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn successful_transcripts_add_whitespace_delimited_words_once() {
        let mut history = History::new(7);

        history.record_success(" one\ttwo\nthree  ".into());

        assert_eq!(history.total_words(), 10);
        assert_eq!(history.last_transcript(), " one\ttwo\nthree  ");
    }

    #[test]
    fn a_new_process_retains_only_the_persisted_total() {
        let mut first_process = History::new(4);
        first_process.record_success("five six".into());
        let path = std::env::temp_dir().join(format!(
            "echo-history-test-{}-{}.json",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let store = SettingsStore::at(path.clone());
        let settings = Settings {
            total_words: first_process.total_words(),
            ..Settings::default()
        };
        store.save(&settings).expect("word total persists");

        let restarted_process = History::new(store.load().expect("settings reload").total_words);

        assert_eq!(restarted_process.total_words(), 6);
        assert_eq!(restarted_process.last_transcript(), "");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn empty_results_do_not_change_history() {
        let mut history = History::new(9);

        history.record_success(" \n\t ".into());

        assert_eq!(history.total_words(), 9);
        assert_eq!(history.last_transcript(), "");
    }
}
