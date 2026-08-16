use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{MAX_COLLECTION_ITEMS, SearchCategory, SearchResult, Validate, ValidationError};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchProvider {
    Applications,
    Files,
    Settings,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchProviderState {
    Pending,
    Streaming,
    Complete,
    Cancelled,
    TimedOut,
    Failed,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SearchQuery {
    pub generation: u64,
    pub text: String,
    pub max_results: usize,
    pub providers: Vec<SearchProvider>,
}

impl Validate for SearchQuery {
    fn validate(&self) -> Result<(), ValidationError> {
        if self.text.len() > crate::MAX_TEXT_BYTES {
            return Err(ValidationError::TextTooLong("search.query"));
        }
        if self.max_results == 0 || self.max_results > MAX_COLLECTION_ITEMS {
            return Err(ValidationError::OutOfRange("search.max_results"));
        }
        if self.providers.is_empty() || self.providers.len() > 3 {
            return Err(ValidationError::OutOfRange("search.providers"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SearchBatch {
    pub generation: u64,
    pub provider: SearchProvider,
    pub state: SearchProviderState,
    pub results: Vec<SearchResult>,
}

impl Validate for SearchBatch {
    fn validate(&self) -> Result<(), ValidationError> {
        if self.results.len() > MAX_COLLECTION_ITEMS {
            return Err(ValidationError::CollectionTooLarge("search.results"));
        }
        for result in &self.results {
            result.validate()?;
        }
        Ok(())
    }
}

pub fn rank_search_results(
    query: &str,
    results: &mut [SearchResult],
    recency: &BTreeMap<String, u16>,
) {
    let query = query.trim().to_lowercase();
    for result in results.iter_mut() {
        let title = result.title.to_lowercase();
        let base = if query.is_empty() {
            400
        } else if title.starts_with(&query) {
            900
        } else if title
            .split_whitespace()
            .any(|word| word.starts_with(&query))
        {
            750
        } else if title.contains(&query) {
            600
        } else {
            0
        };
        result.score_milli =
            (base + recency.get(&result.id).copied().unwrap_or(0).min(100)).min(1_000);
    }
    results.sort_by(|left, right| {
        right
            .score_milli
            .cmp(&left.score_milli)
            .then_with(|| category_order(&left.category).cmp(&category_order(&right.category)))
            .then_with(|| left.title.to_lowercase().cmp(&right.title.to_lowercase()))
            .then_with(|| left.id.cmp(&right.id))
    });
}

const fn category_order(category: &SearchCategory) -> u8 {
    match category {
        SearchCategory::Application => 0,
        SearchCategory::Setting => 1,
        SearchCategory::File => 2,
        SearchCategory::Command => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CommandDescriptor, CommandId, CommandRisk};

    fn result(id: &str, title: &str) -> SearchResult {
        SearchResult {
            id: id.into(),
            title: title.into(),
            subtitle: None,
            category: SearchCategory::Application,
            score_milli: 0,
            activation: CommandDescriptor {
                id: CommandId(format!("launch:{id}")),
                label: "Open".into(),
                enabled: true,
                risk: CommandRisk::Normal,
                children: Vec::new(),
            },
        }
    }

    #[test]
    fn prefix_word_substring_unicode_and_ties_are_deterministic() {
        let mut values = vec![
            result("substring", "Xterminal"),
            result("prefix", "Term app"),
            result("word", "Open term"),
        ];
        rank_search_results("TERM", &mut values, &BTreeMap::new());
        assert_eq!(
            values
                .iter()
                .map(|value| value.id.as_str())
                .collect::<Vec<_>>(),
            vec!["prefix", "word", "substring"]
        );
        let mut unicode = vec![result("u", "設定")];
        rank_search_results("設定", &mut unicode, &BTreeMap::new());
        assert_eq!(unicode[0].score_milli, 900);
    }
}
