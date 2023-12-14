/// Todo: Look at bevy people, there are smarter ways to do this,
/// also to distinguish between mut, non mut, record what types
/// have been extracted already, etc.
///
/// Todo: A derive macro on structs.
pub trait Extract<'a, T> {
    fn extract(&'a mut self) -> T;
}

impl<'a, T> Extract<'a, ()> for T {
    fn extract(&'a mut self) -> () {}
}

mod test {
    use super::Extract;

    struct State {
        num: u64,
        name: String,
    }

    impl<'a> Extract<'a, &'a mut u64> for State {
        fn extract(&'a mut self) -> &mut u64 {
            &mut self.num
        }
    }

    impl<'a> Extract<'a, &'a mut String> for State {
        fn extract(&'a mut self) -> &mut String {
            &mut self.name
        }
    }
}
