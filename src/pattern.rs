use regress::{Matches, Regex};
use stable_pattern::{MatchIndices, MatchIndicesInternal, Pattern, SearchStep, Searcher};

pub struct RegexSearcher<'r, 't> {
    haystack: &'t str,
    it: Matches<'r, 't>,
    last_step_end: usize,
    next_match: Option<(usize, usize)>,
}

pub struct RegexPattern<'r>(pub &'r Regex);

impl<'r, 't> Pattern<'t> for RegexPattern<'r> {
    type Searcher = RegexSearcher<'r, 't>;

    fn into_searcher(self, haystack: &'t str) -> RegexSearcher<'r, 't> {
        RegexSearcher {
            haystack,
            it: self.0.find_iter(haystack),
            last_step_end: 0,
            next_match: None,
        }
    }
}

unsafe impl<'r, 't> Searcher<'t> for RegexSearcher<'r, 't> {
    #[inline]
    fn haystack(&self) -> &'t str {
        self.haystack
    }

    #[inline]
    fn next(&mut self) -> SearchStep {
        if let Some((s, e)) = self.next_match {
            self.next_match = None;
            self.last_step_end = e;

            return SearchStep::Match(s, e);
        }
        match self.it.next() {
            None => {
                if self.last_step_end < self.haystack().len() {
                    let last = self.last_step_end;
                    self.last_step_end = self.haystack().len();

                    SearchStep::Reject(last, self.haystack().len())
                } else {
                    SearchStep::Done
                }
            },
            Some(m) => {
                let (s, e) = (m.start(), m.end());

                if s == self.last_step_end {
                    self.last_step_end = e;
                    SearchStep::Match(s, e)
                } else {
                    self.next_match = Some((s, e));
                    let last = self.last_step_end;
                    self.last_step_end = s;
                    SearchStep::Reject(last, s)
                }
            },
        }
    }
}

pub fn replace<'a, P: Pattern<'a>>(text: &'a str, from: P, to: &str) -> String {
    let mut result = String::new();
    let mut last_end = 0;
    let match_indices: MatchIndices<P> =
        MatchIndices(MatchIndicesInternal(from.into_searcher(text)));

    for (start, part) in match_indices {
        result.push_str(unsafe { text.get_unchecked(last_end..start) });
        result.push_str(to);
        last_end = start + part.len();
    }

    result.push_str(unsafe { text.get_unchecked(last_end..text.len()) });
    result
}
