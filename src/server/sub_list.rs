use async_spmc::{Receiver, Sender};
use std::fmt::{Debug, Error as FmtError, Formatter};
use std::iter::IntoIterator;
use std::iter::Iterator;
use std::ops::FnMut;

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

    fn search<F>(&mut self, mut condition: F) -> Option<&mut T>
    where
        F: FnMut(&T) -> bool,
    {
        self.inner
            .iter()
            .enumerate()
            .find_map(
                |(key, value)| {
                    if condition(value) {
                        Some(key)
                    } else {
                        None
                    }
                },
            )
            .map(move |key| {
                let result: T = self.inner.remove(key);
                self.inner.insert(0, result);
                &mut self.inner[0]
            })
    }

    fn remove<F>(&mut self, mut condition: F)
    where
        F: FnMut(&T) -> bool,
    {
        if let Some(key) =
            self.inner.iter().enumerate().find_map(
                |(key, value)| {
                    if condition(value) {
                        Some(key)
                    } else {
                        None
                    }
                },
            )
        {
            self.inner.remove(key);
        }
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

struct Entry<'a, T> {
    sender: Sender<T>,
    next_level: Level<(&'a str, Entry<'a, T>)>,
}

impl<'a, T> Entry<'a, T> {
    fn new() -> Self {
        Self {
            sender: Sender::new(),
            next_level: Level::new(),
        }
    }

    fn search_mut_entry(&mut self, key: &'a str) -> Option<&mut Self> {
        self.next_level.search(|(k, _)| {
            k == &key
        })
        .map(|(_, entry)| {
            &mut *entry
        })
    }

    fn subscribe(&mut self, list: &mut Vec<&'a str>) -> Receiver<T> {
        if list.is_empty() {
            self.sender.subscribe()
        } else {
            let key: & 'a str = list.remove(0);

            match self.search_mut_entry(&key) {
                Some(entry) => {
                    entry.subscribe(list)
                },
                None => {
                    let mut entry: Entry<T> = Self::new();
                    let recv: Receiver<T> = entry.subscribe(list);
                    self.next_level.insert((key, entry));
                    recv
                }
            }
        }
    }

    fn send(&mut self, list:&mut Vec<&'a str>, value: T) where T: Clone {
        if list.is_empty() {
            self.sender.send(value);
        } else {
            let key: &'a str = list.remove(0);

            if let Some(entry) = self.search_mut_entry(&key) {
                entry.send(list, value);
            }
        }
    }

    fn remove(&mut self, list: &mut Vec<&'a str>) {
        match list.len() {
            0 => {},
            1 => {
                let key: &'a str = list.remove(0);
                self.next_level.remove(|(k, _)| {
                    k == &key
                });
            },
            _ => {
                let key: &'a str = list.remove(0);
                if let Some(entry) = self.search_mut_entry(&key) {
                    entry.remove(list);
                }
            }
        }
    }
}

impl<'a, T> Debug for Entry<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        f.debug_struct("Entry")
            .field("next_level", &self.next_level)
            .finish()
    }
}

#[test]
fn sub_entry() {
    use futures::executor::block_on;
    let mut entry = Entry::new();
    let mut list = vec!["hellow", "world"];
    let recv = entry.subscribe(&mut list);

    let mut list = vec!["hellow", "world"];
    entry.send(&mut list, 3usize);
    entry.send(&mut vec!["hellow"], 4);

    let mut iter = block_on(recv.recv_iter());

    
    assert_eq!(iter.next(), Some(3));
    assert_eq!(iter.next(), None);
}

// 用前缀树做的订阅列表
#[derive(Debug)]
pub(super) struct SubList<'a, T>
where
    T: Clone,
{
    root: Entry<'a, T>,
}

impl<'a, T> SubList<'a, T>
where
    T: Clone,
{
    pub(super) fn new() -> Self {
        Self { root: Entry::new() }
    }

    pub(super) fn subscribe(&mut self, sub: &'a str) -> Receiver<T> {
        self.root.subscribe(&mut Self::split(sub))
    }

    pub(super) fn send(&mut self, sub: &'a str, value: T) {
        self.root.send(&mut Self::split(sub), value);
    }

    pub(super) fn remove(&mut self, sub: &'a str) {
        self.root.remove(&mut Self::split(sub))
    }

    fn split(key: &'a str) -> Vec<&'a str> {
        key.split('.').collect()
    }
}

#[test]
fn test_trie() {
    use futures::executor::block_on;
    let mut sublist: SubList<usize> = SubList::new();
    
    let recv = sublist.subscribe("hello.world.fuck");

    sublist.send("hello.world.fuck", 10);

    let mut iter = block_on(recv.recv_iter());
    assert_eq!(iter.next(), Some(10));

    sublist.remove("hello.world.fuck");
}
