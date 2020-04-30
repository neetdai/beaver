use sequence_trie::SequenceTrie;
use std::iter::IntoIterator;
use tokio::sync::watch::{channel, Receiver, Sender};

#[derive(Debug)]
struct Entry<T>
where
    T: Clone + Default,
{
    sender: Sender<T>,
    receiver: Receiver<T>,
}

impl<T> Entry<T>
where
    T: Clone + Default,
{
    fn new() -> Self {
        let (sender, receiver) = channel(Default::default());
        Self { sender, receiver }
    }
}

#[derive(Debug)]
pub(super) struct SubList<T>
where
    T: Clone + Default,
{
    trie: SequenceTrie<String, Entry<T>>,
}

impl<T> SubList<T>
where
    T: Clone + Default,
{
    pub(super) fn new() -> Self {
        Self {
            trie: SequenceTrie::new(),
        }
    }

    pub(super) fn insert(&mut self, key: String) {
        let find_list: Vec<String> = Self::split(key.clone());
        let insert_list: Vec<String> = Self::split(key);
        if self
            .trie
            .get(find_list.into_iter().map(|item| item.as_ref()))
            .is_none()
        {
            self.trie.insert_owned(insert_list, Entry::new());
        }
    }

    // pub(super) fn subscribe(&mut self, key: String) -> {

    // }

    fn split(key: String) -> Vec<String> {
        key.split('.').map(|item| item.to_string()).collect()
    }
}

#[test]
fn test_trie() {
    let mut sublist: SubList<usize> = SubList::new();
    sublist.insert("hello.world.fuck".to_string());

    sublist.insert("hello.world.help".to_string());
}
