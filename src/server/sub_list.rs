use std::fmt::{Debug, Error as FmtError, Formatter};
use std::iter::Iterator;
use std::ops::FnMut;
use std::slice::{Iter, IterMut};

// 作为前缀树的缓存, 使用lru策略
#[derive(Debug)]
struct Level<T> {
    inner: Vec<T>,
}

impl<T> Level<T> {
    fn new() -> Self {
        Self { inner: Vec::new() }
    }

    fn insert(&mut self, value: T) {
        self.inner.insert(0, value);
    }

    fn search<F>(&mut self, condition: F) -> Option<&mut T>
    where
        F: FnMut(&T) -> bool,
    {
        self.inner.iter().position(condition).map(move |key| {
            let result: T = self.inner.remove(key);
            self.inner.insert(0, result);
            &mut self.inner[0]
        })
    }

    fn remove<F>(&mut self, condition: F)
    where
        F: FnMut(&T) -> bool,
    {
        if let Some(key) = self.inner.iter().position(condition) {
            self.inner.remove(key);
        }
    }

    fn iter(&self) -> Iter<T> {
        self.inner.iter()
    }

    fn iter_mut(&mut self) -> IterMut<T> {
        self.inner.iter_mut()
    }

    fn len(&self) -> usize {
        self.inner.len()
    }
}

#[test]
fn sublist_level() {
    let mut level = Level::new();
    level.insert(1);
    level.insert(2);
    level.insert(3);

    assert_eq!(level.search(|value| *value == 2), Some(&mut 2));
    level.remove(|value| *value == 1);
}

struct Entry<T>
where
    T: Debug,
{
    inner: Vec<T>,
    next_level: Level<(String, Entry<T>)>,
}

impl<T> Entry<T>
where
    T: Debug,
{
    fn new() -> Self {
        Self {
            inner: Vec::new(),
            next_level: Level::new(),
        }
    }

    fn search_mut_entry(&mut self, key: &str) -> Option<&mut Self> {
        self.next_level
            .search(|(k, _)| k == key)
            .map(|(_, entry)| &mut *entry)
    }

    fn subscribe(&mut self, list: &mut Vec<String>, subscription: T) {
        if list.is_empty() {
            self.inner.push(subscription);
        } else {
            let key: String = list.remove(0);

            match self.search_mut_entry(&key) {
                Some(entry) => entry.subscribe(list, subscription),
                None => {
                    let mut entry: Entry<T> = Self::new();
                    entry.subscribe(list, subscription);
                    self.next_level.insert((key, entry));
                }
            }
        }
    }

    fn get_subscribe_item(&mut self, list: &mut Vec<String>) -> Option<&mut Vec<T>> {
        if list.is_empty() {
            Some(&mut self.inner)
        } else {
            let key: String = list.remove(0);

            self.search_mut_entry(&key)
                .and_then(|entry| entry.get_subscribe_item(list))
        }
    }

    fn remove_subscription<F>(&mut self, remove_condition: &F)
    where
        F: Fn(&T) -> bool,
    {
        if let Some(position) = self.inner.iter().position(remove_condition) {
            self.inner.remove(position);
        }
        if self.next_level.len() > 0 {
            for (_, entry) in self.next_level.iter_mut() {
                entry.remove_subscription(remove_condition);
            }
        }
    }

    fn remove(&mut self, list: &mut Vec<String>) {
        match list.len() {
            0 => {}
            1 => {
                let key: String = list.remove(0);
                self.next_level.remove(|(k, _)| k == &key);
            }
            _ => {
                let key: String = list.remove(0);
                if let Some(entry) = self.search_mut_entry(&key) {
                    entry.remove(list);
                }
            }
        }
    }

    fn total(&self) -> usize {
        let mut sum: usize = 0;

        if self.next_level.len() == 0 {
            1
        } else {
            for item in self.next_level.iter() {
                sum += item.1.total();
            }

            sum
        }
    }
}

impl<T> Debug for Entry<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        f.debug_struct("Entry")
            .field("next_level", &self.next_level)
            .finish()
    }
}

#[test]
fn sub_entry() {
    let mut entry = Entry::new();

    for item in 0..3 {
        let mut list = vec![String::from("hellow"), String::from("world")];
        entry.subscribe(&mut list, item);
    }

    let mut list = vec![String::from("hellow"), String::from("world")];
    assert_eq!(
        entry.get_subscribe_item(&mut list),
        Some(&mut vec![0, 1, 2])
    );

    let fnc: fn(&usize) -> bool = |item| *item == 1usize;
    entry.remove_subscription(&fnc);

    let mut list = vec![String::from("hellow"), String::from("world")];
    assert_eq!(entry.get_subscribe_item(&mut list), Some(&mut vec![0, 2]));
}

// 用前缀树做的订阅列表
#[derive(Debug)]
pub(super) struct SubList<T>
where
    T: Clone + Debug,
{
    root: Entry<T>,
}

impl<T> SubList<T>
where
    T: Clone + Debug,
{
    pub(super) fn new() -> Self {
        Self { root: Entry::new() }
    }

    pub(super) fn subscribe(&mut self, sub: String, subscription: T) {
        self.root.subscribe(&mut Self::split(sub), subscription);
    }

    pub(super) fn get_subscribe_item(&mut self, sub: String) -> Option<&mut Vec<T>> {
        self.root.get_subscribe_item(&mut Self::split(sub))
    }

    pub(super) fn remove_subscription<F>(&mut self, remove_condition: F)
    where
        F: Fn(&T) -> bool,
    {
        self.root.remove_subscription(&remove_condition);
    }

    pub(super) fn remove(&mut self, sub: String) {
        self.root.remove(&mut Self::split(sub))
    }

    fn split(key: String) -> Vec<String> {
        key.split('.').map(|item| item.to_string()).collect()
    }

    pub fn total(&self) -> usize {
        self.root.total()
    }
}

#[test]
fn test_trie() {
    use futures::executor::block_on;
    let mut sublist: SubList<usize> = SubList::new();

    let mut sub = Vec::new();
    for item in 0..100usize {
        sub.push(item);
        sublist.subscribe(String::from("hello.world.fuck"), item);
    }

    assert_eq!(
        sublist.get_subscribe_item(String::from("hello.world.fuck")),
        Some(&mut sub)
    );

    sublist.remove_subscription(|item| *item == 50);
    sub.remove(50);

    assert_eq!(
        sublist.get_subscribe_item(String::from("hello.world.fuck")),
        Some(&mut sub)
    );
}
