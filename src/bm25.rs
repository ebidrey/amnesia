use std::collections::HashMap;

use crate::model::Observation;

const K1: f64 = 1.2;
const B: f64 = 0.75;

const TITLE_WEIGHT: f64 = 2.0;
const CONTENT_WEIGHT: f64 = 1.0;
const TAGS_WEIGHT: f64 = 1.5;

struct Index {
    // weighted term frequency per document
    doc_wtf: Vec<HashMap<String, f64>>,
    // unweighted token count per document (used for length normalization)
    doc_len: Vec<usize>,
    // number of documents containing each term
    df: HashMap<String, usize>,
    avgdl: f64,
    n: usize,
}

impl Index {
    fn build(observations: &[Observation]) -> Self {
        let n = observations.len();
        let mut doc_wtf: Vec<HashMap<String, f64>> = Vec::with_capacity(n);
        let mut doc_len: Vec<usize> = Vec::with_capacity(n);
        let mut df: HashMap<String, usize> = HashMap::new();

        for obs in observations {
            let mut wtf: HashMap<String, f64> = HashMap::new();
            let mut len = 0usize;

            for tok in tokenize(&obs.title) {
                *wtf.entry(tok).or_insert(0.0) += TITLE_WEIGHT;
                len += 1;
            }
            for tok in tokenize(&obs.content) {
                *wtf.entry(tok).or_insert(0.0) += CONTENT_WEIGHT;
                len += 1;
            }
            for tag in &obs.tags {
                for tok in tokenize(tag) {
                    *wtf.entry(tok).or_insert(0.0) += TAGS_WEIGHT;
                    len += 1;
                }
            }

            for term in wtf.keys() {
                *df.entry(term.clone()).or_insert(0) += 1;
            }

            doc_wtf.push(wtf);
            doc_len.push(len);
        }

        let avgdl = if n == 0 {
            1.0
        } else {
            doc_len.iter().sum::<usize>() as f64 / n as f64
        };

        Self { doc_wtf, doc_len, df, avgdl, n }
    }

    fn score(&self, doc_idx: usize, query_terms: &[String]) -> f64 {
        let wtf = &self.doc_wtf[doc_idx];
        let len_norm = self.doc_len[doc_idx] as f64 / self.avgdl;

        query_terms
            .iter()
            .map(|term| {
                let tf = wtf.get(term).copied().unwrap_or(0.0);
                if tf == 0.0 {
                    return 0.0;
                }
                let df = self.df.get(term).copied().unwrap_or(0);
                let idf = idf(self.n, df);
                idf * (tf * (K1 + 1.0)) / (tf + K1 * (1.0 - B + B * len_norm))
            })
            .sum()
    }
}

/// Rank `observations` by BM25 relevance for `query`, returning at most `limit` results.
/// Observations with score 0 (no query term found) are excluded.
pub fn rank(observations: Vec<Observation>, query: &str, limit: usize) -> Vec<Observation> {
    let query_terms = tokenize(query);
    if query_terms.is_empty() {
        return observations.into_iter().take(limit).collect();
    }

    let index = Index::build(&observations);

    let mut scored: Vec<(f64, usize)> = (0..observations.len())
        .map(|i| (index.score(i, &query_terms), i))
        .filter(|(score, _)| *score > 0.0)
        .collect();

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(limit);

    scored.into_iter().map(|(_, i)| observations[i].clone()).collect()
}

fn idf(n: usize, df: usize) -> f64 {
    ((n as f64 - df as f64 + 0.5) / (df as f64 + 0.5) + 1.0).ln()
}

fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::OpType;

    fn obs(id: &str, title: &str, content: &str, tags: &[&str]) -> Observation {
        Observation {
            id: id.to_string(),
            timestamp: "2026-03-07T00:00:00Z".to_string(),
            agent: "test-agent".to_string(),
            op_type: OpType::Discovery,
            title: title.to_string(),
            content: content.to_string(),
            files: vec![],
            tags: tags.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn no_match_returns_empty() {
        let observations = vec![
            obs("01", "Redis caching strategy", "Use Redis for session storage", &[]),
        ];
        let result = rank(observations, "postgresql", 10);
        assert!(result.is_empty());
    }

    #[test]
    fn exact_title_match_is_included() {
        let observations = vec![
            obs("01", "Fixed N+1 query in Django", "Added select_related", &[]),
        ];
        let result = rank(observations, "django", 10);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "01");
    }

    #[test]
    fn title_match_ranked_above_content_only_match() {
        // "authentication" is in the title of "01" but only in content of "02"
        let observations = vec![
            obs("01", "Authentication via JWT", "Cookie-based session", &[]),
            obs("02", "Session storage design", "We use authentication tokens", &[]),
        ];
        let result = rank(observations, "authentication", 10);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "01"); // title match ranks first
    }

    #[test]
    fn tag_match_ranked_above_content_only_match() {
        // "postgresql" is a tag in "01" but only in content of "02"
        let observations = vec![
            obs("01", "Index strategy", "Use covering indexes", &["postgresql"]),
            obs("02", "Query optimization", "PostgreSQL slow query log analysis", &[]),
        ];
        let result = rank(observations, "postgresql", 10);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "01"); // tag weight 1.5 > content weight 1.0
    }

    #[test]
    fn results_sorted_by_score_descending() {
        let observations = vec![
            obs("01", "Unrelated topic", "nothing relevant here", &[]),
            obs("02", "JWT auth", "JWT authentication tokens", &["jwt"]),
            obs("03", "JWT authentication", "Use JWT for auth", &["jwt", "auth"]),
        ];
        let result = rank(observations, "jwt authentication", 10);
        // "03" has both terms in title + tags, should rank highest
        assert_eq!(result[0].id, "03");
    }

    #[test]
    fn limit_truncates_results() {
        let observations = vec![
            obs("01", "cache strategy", "redis cache", &[]),
            obs("02", "cache invalidation", "LRU cache policy", &[]),
            obs("03", "cache warming", "preload cache on startup", &[]),
        ];
        let result = rank(observations, "cache", 2);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn search_is_case_insensitive() {
        let observations = vec![
            obs("01", "PostgreSQL indexing", "B-tree indexes", &[]),
        ];
        let result = rank(observations, "POSTGRESQL", 10);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn empty_query_returns_all_up_to_limit() {
        let observations = vec![
            obs("01", "First", "content", &[]),
            obs("02", "Second", "content", &[]),
            obs("03", "Third", "content", &[]),
        ];
        let result = rank(observations, "", 2);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn multi_term_query_rewards_docs_with_both_terms() {
        let observations = vec![
            obs("01", "N+1 query bug", "Django ORM issue", &[]),           // has "django"
            obs("02", "Django authentication", "JWT auth setup", &[]),      // has "django"
            obs("03", "N+1 and Django ORM", "Fixed N+1 in Django views", &[]), // has both
        ];
        let result = rank(observations, "n+1 django", 10);
        // "03" has both terms — should rank first
        assert_eq!(result[0].id, "03");
    }

    #[test]
    fn rare_term_has_higher_idf() {
        // "unique" only in "01", "common" in all three
        // querying "unique" should rank "01" first with a higher score than "common"
        let observations = vec![
            obs("01", "unique term here", "common words", &[]),
            obs("02", "common words title", "common words content", &[]),
            obs("03", "common words again", "common words content", &[]),
        ];
        let result = rank(observations, "unique", 10);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "01");
    }
}
