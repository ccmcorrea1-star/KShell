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
        right.score.cmp(&left.score).then_with(|| {
            applications[left.index]
                .name
                .cmp(&applications[right.index].name)
        })
    });
    results
}

fn fuzzy_score(query: &str, candidate: &str) -> Option<usize> {
    if query.is_empty() {
        return Some(0);
    }

    let candidate_chars = candidate.chars().collect::<Vec<_>>();
    let query_chars = query.chars().collect::<Vec<_>>();
    if query_chars.len() > candidate_chars.len() {
        return None;
    }

    let mut candidate_index = 0;
    let mut first_match = None;
    let mut previous_match = None;
    let mut gaps = 0;

    for query_character in query_chars {
        let relative_index = candidate_chars[candidate_index..]
            .iter()
            .position(|candidate_character| candidate_character == &query_character)?;

        let matched_index = candidate_index + relative_index;
        first_match.get_or_insert(matched_index);
        if let Some(previous_match) = previous_match {
            gaps += matched_index.saturating_sub(previous_match + 1);
        }
        previous_match = Some(matched_index);
        candidate_index = matched_index + 1;
    }

    let first_match = first_match?;
    let prefix_bonus = usize::from(candidate.starts_with(query)).saturating_mul(1_000);
    let exact_bonus = usize::from(candidate == query).saturating_mul(1_000);
    Some(prefix_bonus + exact_bonus + 500usize.saturating_sub(first_match * 10 + gaps * 2))
}

#[cfg(test)]
mod tests {
    use super::filter;
    use crate::desktop::DesktopEntry;

    fn application(name: &str) -> DesktopEntry {
        DesktopEntry {
            name: name.to_owned(),
            generic_name: None,
            icon: None,
            exec: vec![name.to_owned()],
            working_dir: None,
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
}
