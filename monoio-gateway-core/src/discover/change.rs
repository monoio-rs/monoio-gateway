pub enum DiscoverChange<Key, S> {
    Add(Key, S),
    Remove(Key, S),
}

