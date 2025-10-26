//! # Temporal-Compare
//!
//! Advanced temporal sequence comparison and pattern matching.
//!
//! ## Features
//! - Dynamic Time Warping (DTW)
//! - Longest Common Subsequence (LCS)
//! - Edit Distance (Levenshtein)
//! - Pattern matching and detection
//! - Efficient caching

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use thiserror::Error;
use dashmap::DashMap;
use lru::LruCache;
use std::sync::{Arc, Mutex};
use std::num::NonZeroUsize;

/// Errors that can occur during temporal comparison
#[derive(Debug, Error)]
pub enum TemporalError {
    #[error("Sequence too long: {0}")]
    SequenceTooLong(usize),

    #[error("Invalid algorithm: {0}")]
    InvalidAlgorithm(String),

    #[error("Cache error: {0}")]
    CacheError(String),
}

/// A temporal sequence element
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TemporalElement<T> {
    pub value: T,
    pub timestamp: u64,
}

/// A temporal sequence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sequence<T> {
    pub elements: Vec<TemporalElement<T>>,
}

impl<T> Sequence<T> {
    pub fn new() -> Self {
        Self { elements: Vec::new() }
    }

    pub fn push(&mut self, value: T, timestamp: u64) {
        self.elements.push(TemporalElement { value, timestamp });
    }

    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }
}

impl<T> Default for Sequence<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Comparison algorithm types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ComparisonAlgorithm {
    /// Dynamic Time Warping
    DTW,
    /// Longest Common Subsequence
    LCS,
    /// Edit Distance (Levenshtein)
    EditDistance,
    /// Euclidean distance
    Euclidean,
}

/// Result of a temporal comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonResult {
    pub distance: f64,
    pub algorithm: ComparisonAlgorithm,
    pub alignment: Option<Vec<(usize, usize)>>,
}

/// Statistics about cache performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub size: usize,
    pub capacity: usize,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        if self.hits + self.misses == 0 {
            0.0
        } else {
            self.hits as f64 / (self.hits + self.misses) as f64
        }
    }
}

/// Temporal comparator with caching
pub struct TemporalComparator<T> {
    cache: Arc<Mutex<LruCache<String, ComparisonResult>>>,
    cache_hits: Arc<DashMap<String, u64>>,
    cache_misses: Arc<DashMap<String, u64>>,
    max_sequence_length: usize,
}

impl<T> TemporalComparator<T>
where
    T: Clone + PartialEq + fmt::Debug + Serialize,
{
    /// Create a new temporal comparator
    pub fn new(cache_size: usize, max_sequence_length: usize) -> Self {
        Self {
            cache: Arc::new(Mutex::new(LruCache::new(
                NonZeroUsize::new(cache_size).unwrap()
            ))),
            cache_hits: Arc::new(DashMap::new()),
            cache_misses: Arc::new(DashMap::new()),
            max_sequence_length,
        }
    }

    /// Compare two sequences using the specified algorithm
    pub fn compare(
        &self,
        seq1: &Sequence<T>,
        seq2: &Sequence<T>,
        algorithm: ComparisonAlgorithm,
    ) -> Result<ComparisonResult, TemporalError> {
        // Check sequence length
        if seq1.len() > self.max_sequence_length || seq2.len() > self.max_sequence_length {
            return Err(TemporalError::SequenceTooLong(
                seq1.len().max(seq2.len())
            ));
        }

        // Generate cache key
        let cache_key = self.cache_key(seq1, seq2, algorithm);

        // Check cache
        if let Ok(mut cache) = self.cache.lock() {
            if let Some(result) = cache.get(&cache_key) {
                self.record_cache_hit(&cache_key);
                return Ok(result.clone());
            }
        }

        self.record_cache_miss(&cache_key);

        // Compute comparison
        let result = match algorithm {
            ComparisonAlgorithm::DTW => self.dtw(seq1, seq2),
            ComparisonAlgorithm::LCS => self.lcs(seq1, seq2),
            ComparisonAlgorithm::EditDistance => self.edit_distance(seq1, seq2),
            ComparisonAlgorithm::Euclidean => self.euclidean(seq1, seq2),
        }?;

        // Store in cache
        if let Ok(mut cache) = self.cache.lock() {
            cache.put(cache_key, result.clone());
        }

        Ok(result)
    }

    /// Dynamic Time Warping implementation
    fn dtw(&self, seq1: &Sequence<T>, seq2: &Sequence<T>) -> Result<ComparisonResult, TemporalError> {
        let n = seq1.len();
        let m = seq2.len();

        if n == 0 || m == 0 {
            return Ok(ComparisonResult {
                distance: (n + m) as f64,
                algorithm: ComparisonAlgorithm::DTW,
                alignment: None,
            });
        }

        // Initialize DTW matrix
        let mut dtw = vec![vec![f64::INFINITY; m + 1]; n + 1];
        dtw[0][0] = 0.0;

        // Fill DTW matrix
        for i in 1..=n {
            for j in 1..=m {
                let cost = if seq1.elements[i-1].value == seq2.elements[j-1].value {
                    0.0
                } else {
                    1.0
                };

                dtw[i][j] = cost + dtw[i-1][j-1].min(dtw[i-1][j]).min(dtw[i][j-1]);
            }
        }

        // Backtrack for alignment
        let mut alignment = Vec::new();
        let (mut i, mut j) = (n, m);

        while i > 0 && j > 0 {
            alignment.push((i - 1, j - 1));

            let min_val = dtw[i-1][j-1].min(dtw[i-1][j]).min(dtw[i][j-1]);

            if dtw[i-1][j-1] == min_val {
                i -= 1;
                j -= 1;
            } else if dtw[i-1][j] == min_val {
                i -= 1;
            } else {
                j -= 1;
            }
        }

        alignment.reverse();

        Ok(ComparisonResult {
            distance: dtw[n][m],
            algorithm: ComparisonAlgorithm::DTW,
            alignment: Some(alignment),
        })
    }

    /// Longest Common Subsequence implementation
    fn lcs(&self, seq1: &Sequence<T>, seq2: &Sequence<T>) -> Result<ComparisonResult, TemporalError> {
        let n = seq1.len();
        let m = seq2.len();

        let mut dp = vec![vec![0; m + 1]; n + 1];

        for i in 1..=n {
            for j in 1..=m {
                if seq1.elements[i-1].value == seq2.elements[j-1].value {
                    dp[i][j] = dp[i-1][j-1] + 1;
                } else {
                    dp[i][j] = dp[i-1][j].max(dp[i][j-1]);
                }
            }
        }

        let lcs_length = dp[n][m];
        let distance = (n + m - 2 * lcs_length) as f64;

        Ok(ComparisonResult {
            distance,
            algorithm: ComparisonAlgorithm::LCS,
            alignment: None,
        })
    }

    /// Edit Distance (Levenshtein) implementation
    fn edit_distance(&self, seq1: &Sequence<T>, seq2: &Sequence<T>) -> Result<ComparisonResult, TemporalError> {
        let n = seq1.len();
        let m = seq2.len();

        let mut dp = vec![vec![0; m + 1]; n + 1];

        for i in 0..=n {
            dp[i][0] = i;
        }
        for j in 0..=m {
            dp[0][j] = j;
        }

        for i in 1..=n {
            for j in 1..=m {
                let cost = if seq1.elements[i-1].value == seq2.elements[j-1].value {
                    0
                } else {
                    1
                };

                dp[i][j] = (dp[i-1][j] + 1)
                    .min(dp[i][j-1] + 1)
                    .min(dp[i-1][j-1] + cost);
            }
        }

        Ok(ComparisonResult {
            distance: dp[n][m] as f64,
            algorithm: ComparisonAlgorithm::EditDistance,
            alignment: None,
        })
    }

    /// Euclidean distance (for numeric sequences)
    fn euclidean(&self, seq1: &Sequence<T>, seq2: &Sequence<T>) -> Result<ComparisonResult, TemporalError> {
        let n = seq1.len().min(seq2.len());
        let mut sum = 0.0;

        for i in 0..n {
            // Simplified: just count mismatches
            if seq1.elements[i].value != seq2.elements[i].value {
                sum += 1.0;
            }
        }

        Ok(ComparisonResult {
            distance: sum.sqrt(),
            algorithm: ComparisonAlgorithm::Euclidean,
            alignment: None,
        })
    }

    /// Generate cache key for a comparison
    fn cache_key(&self, seq1: &Sequence<T>, seq2: &Sequence<T>, algorithm: ComparisonAlgorithm) -> String {
        format!(
            "{:?}:{:?}:{:?}",
            seq1.elements.len(),
            seq2.elements.len(),
            algorithm
        )
    }

    fn record_cache_hit(&self, key: &str) {
        self.cache_hits.entry(key.to_string())
            .and_modify(|v| *v += 1)
            .or_insert(1);
    }

    fn record_cache_miss(&self, key: &str) {
        self.cache_misses.entry(key.to_string())
            .and_modify(|v| *v += 1)
            .or_insert(1);
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> CacheStats {
        let hits: u64 = self.cache_hits.iter().map(|r| *r.value()).sum();
        let misses: u64 = self.cache_misses.iter().map(|r| *r.value()).sum();

        let (size, capacity) = if let Ok(cache) = self.cache.lock() {
            (cache.len(), cache.cap().get())
        } else {
            (0, 0)
        };

        CacheStats {
            hits,
            misses,
            size,
            capacity,
        }
    }

    /// Clear the cache
    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.clear();
        }
        self.cache_hits.clear();
        self.cache_misses.clear();
    }
}

impl<T> Default for TemporalComparator<T>
where
    T: Clone + PartialEq + fmt::Debug + Serialize,
{
    fn default() -> Self {
        Self::new(1000, 10000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sequence_creation() {
        let mut seq: Sequence<i32> = Sequence::new();
        seq.push(1, 100);
        seq.push(2, 200);

        assert_eq!(seq.len(), 2);
        assert!(!seq.is_empty());
    }

    #[test]
    fn test_dtw() {
        let comparator = TemporalComparator::new(100, 1000);

        let mut seq1: Sequence<i32> = Sequence::new();
        seq1.push(1, 100);
        seq1.push(2, 200);
        seq1.push(3, 300);

        let mut seq2: Sequence<i32> = Sequence::new();
        seq2.push(1, 100);
        seq2.push(2, 200);
        seq2.push(3, 300);

        let result = comparator.compare(&seq1, &seq2, ComparisonAlgorithm::DTW).unwrap();
        assert_eq!(result.distance, 0.0);
    }

    #[test]
    fn test_edit_distance() {
        let comparator = TemporalComparator::new(100, 1000);

        let mut seq1: Sequence<char> = Sequence::new();
        seq1.push('k', 1);
        seq1.push('i', 2);
        seq1.push('t', 3);
        seq1.push('t', 4);
        seq1.push('e', 5);
        seq1.push('n', 6);

        let mut seq2: Sequence<char> = Sequence::new();
        seq2.push('s', 1);
        seq2.push('i', 2);
        seq2.push('t', 3);
        seq2.push('t', 4);
        seq2.push('i', 5);
        seq2.push('n', 6);
        seq2.push('g', 7);

        let result = comparator.compare(&seq1, &seq2, ComparisonAlgorithm::EditDistance).unwrap();
        assert_eq!(result.distance, 3.0);
    }

    #[test]
    fn test_lcs() {
        let comparator = TemporalComparator::new(100, 1000);

        let mut seq1: Sequence<i32> = Sequence::new();
        seq1.push(1, 1);
        seq1.push(2, 2);
        seq1.push(3, 3);
        seq1.push(4, 4);

        let mut seq2: Sequence<i32> = Sequence::new();
        seq2.push(1, 1);
        seq2.push(3, 3);
        seq2.push(4, 4);

        let result = comparator.compare(&seq1, &seq2, ComparisonAlgorithm::LCS).unwrap();
        assert_eq!(result.distance, 1.0);
    }

    #[test]
    fn test_cache() {
        let comparator = TemporalComparator::new(100, 1000);

        let mut seq1: Sequence<i32> = Sequence::new();
        seq1.push(1, 1);
        seq1.push(2, 2);

        let mut seq2: Sequence<i32> = Sequence::new();
        seq2.push(1, 1);
        seq2.push(2, 2);

        // First comparison - cache miss
        comparator.compare(&seq1, &seq2, ComparisonAlgorithm::DTW).unwrap();

        // Second comparison - cache hit
        comparator.compare(&seq1, &seq2, ComparisonAlgorithm::DTW).unwrap();

        let stats = comparator.cache_stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }
}
