use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::Path;

/// Distance indicating two characters are unreachable / unlinked in the keyboard graph.
pub const UNLINKED_DISTANCE: u32 = u32::MAX;

/// Normalized physical keyboard adjacency matrix and typo distance calculator.
#[derive(Debug, Clone)]
pub struct KeyDistanceMatrix {
    char_to_idx: HashMap<char, usize>,
    ascii_map: [Option<u16>; 128],
    thai_map: [Option<u16>; 128],
    num_nodes: usize,
    distances: Vec<u32>,
}

impl KeyDistanceMatrix {
    /// Embedded default physical keyboard adjacent pairs table.
    pub const DEFAULT_ADJACENT_PAIRS: &'static str =
        include_str!("../../../assets/adjacent-column.txt");

    /// Creates a new `KeyDistanceMatrix` using the embedded default adjacent pairs table.
    pub fn new() -> Self {
        Self::from_adjacent_pairs_str(Self::DEFAULT_ADJACENT_PAIRS)
    }

    /// Creates an empty `KeyDistanceMatrix` with no keys.
    pub fn empty() -> Self {
        Self {
            char_to_idx: HashMap::new(),
            ascii_map: [None; 128],
            thai_map: [None; 128],
            num_nodes: 0,
            distances: Vec::new(),
        }
    }

    /// Loads adjacent key pairs from an external file.
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let content = fs::read_to_string(path)?;
        Ok(Self::from_adjacent_pairs_str(&content))
    }

    /// Alias for `from_adjacent_pairs_str`.
    pub fn from_adjacent_pairs(pairs: &str) -> Self {
        Self::from_adjacent_pairs_str(pairs)
    }

    /// Builds the distance matrix from newline-separated adjacent key pair strings.
    ///
    /// Handles UTF-8 BOM, leading/trailing whitespace, and comments.
    /// Case-insensitive: uppercase letters map to lowercase equivalents.
    pub fn from_adjacent_pairs_str(pairs: &str) -> Self {
        let mut adj: HashMap<char, Vec<char>> = HashMap::new();

        for line in pairs.lines() {
            let trimmed = line.trim().trim_matches('\u{feff}');
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
                continue;
            }

            let chars: Vec<char> = trimmed.chars().collect();
            if chars.len() == 2 {
                let u = normalize_char(chars[0]);
                let v = normalize_char(chars[1]);
                if u != v {
                    adj.entry(u).or_default().push(v);
                    adj.entry(v).or_default().push(u);
                }
            }
        }

        let mut nodes: Vec<char> = adj.keys().copied().collect();
        nodes.sort_unstable();
        let num_nodes = nodes.len();

        let mut char_to_idx = HashMap::with_capacity(num_nodes);
        let mut ascii_map = [None; 128];
        let mut thai_map = [None; 128];

        for (i, &ch) in nodes.iter().enumerate() {
            char_to_idx.insert(ch, i);
            let codepoint = ch as u32;
            if codepoint < 128 {
                ascii_map[codepoint as usize] = Some(i as u16);
            } else if (0x0e00..=0x0e7f).contains(&codepoint) {
                thai_map[(codepoint - 0x0e00) as usize] = Some(i as u16);
            }
        }

        let mut distances = vec![UNLINKED_DISTANCE; num_nodes * num_nodes];
        for i in 0..num_nodes {
            distances[i * num_nodes + i] = 0;
        }

        // Multi-source BFS for all-pairs shortest paths
        for (start_idx, &start_char) in nodes.iter().enumerate() {
            let mut queue = VecDeque::new();
            queue.push_back((start_char, 0u32));

            while let Some((curr_char, dist)) = queue.pop_front() {
                if let Some(neighbors) = adj.get(&curr_char) {
                    for &nxt in neighbors {
                        if let Some(&nxt_idx) = char_to_idx.get(&nxt) {
                            let matrix_idx = start_idx * num_nodes + nxt_idx;
                            if distances[matrix_idx] == UNLINKED_DISTANCE {
                                distances[matrix_idx] = dist + 1;
                                queue.push_back((nxt, dist + 1));
                            }
                        }
                    }
                }
            }
        }

        Self {
            char_to_idx,
            ascii_map,
            thai_map,
            num_nodes,
            distances,
        }
    }

    #[inline]
    fn get_index(&self, c: char) -> Option<usize> {
        let cp = c as u32;
        if cp < 128 {
            self.ascii_map[cp as usize].map(|idx| idx as usize)
        } else if (0x0e00..=0x0e7f).contains(&cp) {
            self.thai_map[(cp - 0x0e00) as usize].map(|idx| idx as usize)
        } else {
            self.char_to_idx.get(&c).copied()
        }
    }

    /// Returns the shortest path graph distance between two characters.
    ///
    /// - 0 if `a` and `b` represent the same key (case-insensitive)
    /// - 1 if directly adjacent on the physical keyboard
    /// - 2+ for multi-hop shortest physical path
    /// - `UNLINKED_DISTANCE` (`u32::MAX`) if unlinked or not in the keymap
    #[inline]
    pub fn distance(&self, a: char, b: char) -> u32 {
        let na = normalize_char(a);
        let nb = normalize_char(b);
        if na == nb {
            return 0;
        }
        let idx_a = match self.get_index(na) {
            Some(i) => i,
            None => return UNLINKED_DISTANCE,
        };
        let idx_b = match self.get_index(nb) {
            Some(i) => i,
            None => return UNLINKED_DISTANCE,
        };
        self.distances[idx_a * self.num_nodes + idx_b]
    }

    /// Checks if two keys are directly adjacent on the physical keyboard.
    #[inline]
    pub fn are_adjacent(&self, a: char, b: char) -> bool {
        self.distance(a, b) == 1
    }

    /// Calculates weighted Levenshtein distance where:
    /// - identical characters cost 0
    /// - adjacent key substitutions cost 1
    /// - non-adjacent key substitutions cost 2
    /// - single character insertions / deletions cost 1
    pub fn edit_distance(&self, input: &str, candidate: &str) -> u32 {
        let s1: Vec<char> = input.chars().map(normalize_char).collect();
        let s2: Vec<char> = candidate.chars().map(normalize_char).collect();

        let m = s1.len();
        let n = s2.len();

        if m == 0 {
            return n as u32;
        }
        if n == 0 {
            return m as u32;
        }

        let mut prev = vec![0u32; n + 1];
        let mut curr = vec![0u32; n + 1];

        for (j, item) in prev.iter_mut().enumerate() {
            *item = j as u32;
        }

        for i in 1..=m {
            curr[0] = i as u32;
            let c1 = s1[i - 1];

            for j in 1..=n {
                let c2 = s2[j - 1];
                let sub_cost = if c1 == c2 {
                    0
                } else if self.are_adjacent(c1, c2) {
                    1
                } else {
                    2
                };

                let cost_sub = prev[j - 1] + sub_cost;
                let cost_del = prev[j] + 1;
                let cost_ins = curr[j - 1] + 1;

                curr[j] = cost_sub.min(cost_del).min(cost_ins);
            }

            std::mem::swap(&mut prev, &mut curr);
        }

        prev[n]
    }

    /// Fuzzy match comparing two strings allowing up to `max_cost` typo distance.
    ///
    /// Substitutions between adjacent keys cost 1, non-adjacent substitutions cost 2,
    /// and insertions / deletions cost 1.
    pub fn fuzzy_match(&self, input: &str, candidate: &str, max_cost: u32) -> bool {
        let len_diff = (input.chars().count() as isize - candidate.chars().count() as isize)
            .unsigned_abs() as u32;
        if len_diff > max_cost {
            return false;
        }
        self.edit_distance(input, candidate) <= max_cost
    }
}

impl Default for KeyDistanceMatrix {
    fn default() -> Self {
        Self::new()
    }
}

/// Normalizes a character to lowercase for physical key matching.
#[inline]
pub fn normalize_char(c: char) -> char {
    if c.is_ascii() {
        c.to_ascii_lowercase()
    } else {
        c.to_lowercase().next().unwrap_or(c)
    }
}
