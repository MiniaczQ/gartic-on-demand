pub trait Provider<T> {
    fn get(&self) -> T;
}
