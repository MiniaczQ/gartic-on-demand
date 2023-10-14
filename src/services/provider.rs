pub trait Provider<T> {
    fn get(&self) -> T;
}

impl<T: Clone> Provider<T> for T {
    fn get(&self) -> T {
        self.clone()
    }
}
