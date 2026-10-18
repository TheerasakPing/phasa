#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct TrieNode {
    is_terminal: bool,
    children: Vec<(char, usize)>,
}

/// A compact, memory-efficient trie storing UTF-8 Unicode characters.
///
/// Uses flat indexed node storage and sorted child vectors with binary search
/// to minimize memory overhead while maintaining sub-millisecond lookup times.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactTrie {
    nodes: Vec<TrieNode>,
    count: usize,
}

impl Default for CompactTrie {
    fn default() -> Self {
        Self::new()
    }
}

impl CompactTrie {
    /// Creates a new, empty `CompactTrie` initialized with a root node.
    pub fn new() -> Self {
        Self {
            nodes: vec![TrieNode::default()],
            count: 0,
        }
    }

    /// Creates a new `CompactTrie` with pre-allocated node capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        let mut nodes = Vec::with_capacity(capacity);
        nodes.push(TrieNode::default());
        Self { nodes, count: 0 }
    }

    /// Inserts a word into the trie.
    pub fn insert(&mut self, word: &str) {
        if word.is_empty() {
            return;
        }

        let mut curr = 0;
        for ch in word.chars() {
            match self.nodes[curr]
                .children
                .binary_search_by_key(&ch, |&(c, _)| c)
            {
                Ok(idx) => {
                    curr = self.nodes[curr].children[idx].1;
                }
                Err(idx) => {
                    let next_idx = self.nodes.len();
                    self.nodes.push(TrieNode::default());
                    self.nodes[curr].children.insert(idx, (ch, next_idx));
                    curr = next_idx;
                }
            }
        }

        if !self.nodes[curr].is_terminal {
            self.nodes[curr].is_terminal = true;
            self.count += 1;
        }
    }

    /// Returns `true` if the exact word exists in the trie.
    pub fn contains(&self, word: &str) -> bool {
        if word.is_empty() || self.nodes.is_empty() {
            return false;
        }

        let mut curr = 0;
        for ch in word.chars() {
            match self.nodes[curr]
                .children
                .binary_search_by_key(&ch, |&(c, _)| c)
            {
                Ok(idx) => {
                    curr = self.nodes[curr].children[idx].1;
                }
                Err(_) => return false,
            }
        }

        self.nodes[curr].is_terminal
    }

    /// Returns `true` if the given prefix exists along any path in the trie.
    pub fn is_prefix(&self, prefix: &str) -> bool {
        if prefix.is_empty() || self.is_empty() {
            return false;
        }

        let mut curr = 0;
        for ch in prefix.chars() {
            match self.nodes[curr]
                .children
                .binary_search_by_key(&ch, |&(c, _)| c)
            {
                Ok(idx) => {
                    curr = self.nodes[curr].children[idx].1;
                }
                Err(_) => return false,
            }
        }

        true
    }

    /// Returns the number of unique words stored in the trie.
    pub fn len(&self) -> usize {
        self.count
    }

    /// Returns `true` if the trie contains no words.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Shrinks internal vectors to fit memory snugly.
    pub fn shrink_to_fit(&mut self) {
        self.nodes.shrink_to_fit();
        for node in &mut self.nodes {
            node.children.shrink_to_fit();
        }
    }

    /// Populates a `CompactTrie` from a newline-separated wordlist string.
    pub fn from_wordlist_str(words: &str) -> Self {
        let mut trie = Self::new();
        for line in words.lines() {
            let trimmed = line.trim().trim_matches('\u{feff}');
            if !trimmed.is_empty() {
                trie.insert(trimmed);
            }
        }
        trie
    }
}
