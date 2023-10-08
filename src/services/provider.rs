pub trait Provider<'a, T> {
    fn get(&'a self) -> T;
}
