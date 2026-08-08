use crate::desktop::DesktopEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchResult {
    pub index: usize,
    score: usize,
}

pub fn filter(applications: &[DesktopEntry], query: &str) -> Vec<SearchResult> {
    let query = query.to_lowercase();
    let mut results = applications
        .iter()
        .enumerate()
        .filter_map(|(index, application)| {
            let name_score = fuzzy_score(&query, &application.name.to_lowercase());
            let generic_score = application
                .generic_name
                .as_deref()
                .map(str::to_lowercase)
                .and_then(|generic| fuzzy_score(&query, &generic));

            match (name_score, generic_score) {
                (Some(name_score), Some(generic_score)) => Some(name_score.max(generic_score)),
                (Some(score), None) | (None, Some(score)) => Some(score),
                (None, None) => None,
            }
            .map(|score| SearchResult { index, score })
        })
        .collect::<Vec<_>>();

    results.sort_unstable_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| {
                applications[left.index]
                    .name
                    .cmp(&applications[right.index].name)
            })
            .then_with(|| left.index.cmp(&right.index))
    });
    results
}

fn fuzzy_score(query: &str, candidate: &str) -> Option<usize> {
    const BASE_SCORE: i64 = 500;
    const CONTIGUOUS_BONUS: i64 = 12;
    const GAP_PENALTY: i64 = 2;
    const START_PENALTY: i64 = 10;

    if query.is_empty() {
        return Some(0);
    }

    let candidate_chars = candidate.chars().collect::<Vec<_>>();
    let query_chars = query.chars().collect::<Vec<_>>();
    if query_chars.len() > candidate_chars.len() {
        return None;
    }

    let mut previous_scores = vec![None; candidate_chars.len()];
    for (candidate_index, candidate_character) in candidate_chars.iter().enumerate() {
        if candidate_character == &query_chars[0] {
            previous_scores[candidate_index] = Some(-(candidate_index as i64) * START_PENALTY);
        }
    }

    for query_character in query_chars.iter().skip(1) {
        let mut current_scores = vec![None; candidate_chars.len()];
        let mut best_non_contiguous_base: Option<i64> = None;

        for (candidate_index, candidate_character) in candidate_chars.iter().enumerate() {
            if candidate_index >= 2 {
                if let Some(previous_score) = previous_scores[candidate_index - 2] {
                    let gap_base = previous_score + (candidate_index as i64 - 2) * GAP_PENALTY;
                    best_non_contiguous_base = match best_non_contiguous_base {
                        Some(best_score) => Some(best_score.max(gap_base)),
                        None => Some(gap_base),
                    };
                }
            }

            if candidate_character != query_character {
                continue;
            }

            let contiguous_score = (candidate_index > 0)
                .then(|| previous_scores[candidate_index - 1])
                .flatten()
                .map(|score| score + CONTIGUOUS_BONUS);
            let non_contiguous_score = best_non_contiguous_base
                .map(|score| score - (candidate_index as i64 - 1) * GAP_PENALTY);

            current_scores[candidate_index] = match (contiguous_score, non_contiguous_score) {
                (Some(contiguous), Some(non_contiguous)) => Some(contiguous.max(non_contiguous)),
                (Some(score), None) | (None, Some(score)) => Some(score),
                (None, None) => None,
            };
        }

        previous_scores = current_scores;
    }

    let best_match_score = previous_scores.into_iter().flatten().max()?;
    let prefix_bonus = usize::from(candidate.starts_with(query)).saturating_mul(1_000);
    let exact_bonus = usize::from(candidate == query).saturating_mul(1_000);
    let relevance_score = (BASE_SCORE + best_match_score).max(0) as usize;
    Some(prefix_bonus + exact_bonus + relevance_score)
}

#[cfg(test)]
mod tests {
    use super::{filter, fuzzy_score};
    use crate::desktop::DesktopEntry;

    fn application(name: &str) -> DesktopEntry {
        DesktopEntry {
            name: name.to_owned(),
            generic_name: None,
            exec: vec![name.to_owned()],
            working_dir: None,
            terminal: false,
        }
    }

    #[test]
    fn fuzzy_filter_matches_subsequences_and_ranks_prefixes() {
        let applications = vec![
            application("Firefox Developer Edition"),
            application("Files"),
        ];
        let results = filter(&applications, "fire");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].index, 0);
    }

    #[test]
    fn empty_query_returns_all_entries() {
        let applications = vec![application("B"), application("A")];
        let results = filter(&applications, "");

        assert_eq!(
            results
                .iter()
                .map(|result| result.index)
                .collect::<Vec<_>>(),
            vec![1, 0]
        );
    }

    #[test]
    fn fuzzy_score_prefers_a_later_contiguous_subsequence() {
        let later_contiguous = fuzzy_score("ab", "aab").expect("match expected");
        let greedy_match = fuzzy_score("ab", "acb").expect("match expected");

        assert!(later_contiguous > greedy_match);
    }

    #[test]
    fn equal_scores_are_tied_by_original_index() {
        let applications = vec![application("Same"), application("Same")];
        let results = filter(&applications, "same");

        assert_eq!(
            results
                .iter()
                .map(|result| result.index)
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
    }
}
