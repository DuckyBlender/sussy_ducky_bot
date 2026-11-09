use std::collections::{HashMap, VecDeque};

use teloxide::types::{ChatId, FileId, MessageId};

#[derive(Clone)]
pub struct ImageRef {
    pub file_id: FileId,
    pub mime: String,
}

#[derive(Clone)]
pub struct Entry {
    pub from_bot: bool,
    pub text: String,
    pub image: Option<ImageRef>,
    pub parent: Option<MessageId>,
}

/// In-memory store of recent messages so whole reply chains can be rebuilt
/// (Telegram only includes the directly replied-to message). Oldest entries
/// are evicted once `capacity` is reached; nothing survives a restart.
pub struct History {
    entries: HashMap<(ChatId, MessageId), Entry>,
    order: VecDeque<(ChatId, MessageId)>,
    capacity: usize,
}

impl History {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: HashMap::new(),
            order: VecDeque::new(),
            capacity,
        }
    }

    pub fn contains(&self, chat: ChatId, id: MessageId) -> bool {
        self.entries.contains_key(&(chat, id))
    }

    pub fn insert(&mut self, chat: ChatId, id: MessageId, entry: Entry) {
        if self.entries.insert((chat, id), entry).is_none() {
            self.order.push_back((chat, id));
        }
        while self.order.len() > self.capacity {
            if let Some(old) = self.order.pop_front() {
                self.entries.remove(&old);
            }
        }
    }

    /// The reply chain ending at `id`, oldest first, at most `max` messages.
    pub fn chain(&self, chat: ChatId, id: MessageId, max: usize) -> Vec<Entry> {
        let mut chain = Vec::new();
        let mut next = Some(id);
        while let Some(id) = next {
            if chain.len() >= max {
                break;
            }
            let Some(entry) = self.entries.get(&(chat, id)) else {
                break;
            };
            chain.push(entry.clone());
            next = entry.parent;
        }
        chain.reverse();
        chain
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(text: &str, parent: Option<i32>) -> Entry {
        Entry {
            from_bot: false,
            text: text.into(),
            image: None,
            parent: parent.map(MessageId),
        }
    }

    #[test]
    fn walks_reply_chain() {
        let chat = ChatId(1);
        let mut h = History::new(100);
        h.insert(chat, MessageId(1), entry("a", None));
        h.insert(chat, MessageId(2), entry("b", Some(1)));
        h.insert(chat, MessageId(3), entry("unrelated", None));
        h.insert(chat, MessageId(4), entry("c", Some(2)));
        let texts: Vec<_> = h
            .chain(chat, MessageId(4), 10)
            .into_iter()
            .map(|e| e.text)
            .collect();
        assert_eq!(texts, ["a", "b", "c"]);
        assert_eq!(h.chain(chat, MessageId(4), 2).len(), 2);
    }

    #[test]
    fn evicts_oldest() {
        let chat = ChatId(1);
        let mut h = History::new(2);
        for i in 1..=3 {
            h.insert(chat, MessageId(i), entry("x", None));
        }
        assert!(!h.contains(chat, MessageId(1)));
        assert!(h.contains(chat, MessageId(3)));
    }
}
