/// The observer pattern as a simple wrapper structure
///
/// Store a value and a set of subscriber functions. These functions hold no state, as they are
/// not closures, though they might access some global state, but this is generally discouraged.
/// Nor is it intended to ever store state in a closure-like manner. If you feel the need to store
/// state together with a function, please consider storing the state in the context object
/// instead.
pub struct Observe<T, C> {
    value: T,
    subscribers: Vec<Subscriber<C, T>>,
}

/// A subscriber
///
/// A subscriber takes as input a thing to modify `E` and the new value `T`.
/// IMPORTANT: Please ensure that subscribers are named functions, this ensures that they have
/// well-defined function pointers. A closure may have the exact same code but have a different
/// pointer in memory.
type Subscriber<C, T> = fn(&mut C, T);

impl<T: Default, E> Default for Observe<T, E> {
    /// Construct a default observer with no subscribers
    fn default() -> Self {
        Self {
            value: T::default(),
            subscribers: vec![],
        }
    }
}

impl<T: Clone + PartialEq, C> Observe<T, C> {
    /// Only call the subscribers if the value differs
    pub fn compare_and_set(&mut self, value: T, modifier: &mut C) {
        if self.value != value {
            self.set(value, modifier);
        }
    }

    /// For when you need to set some element but the subscribers depend on the parent element
    ///
    /// This version compares the new value to the previous value to ascertain whether or not to
    /// call the subscribers.
    pub fn dependency_compare_and_set(
        modifier: &mut C,
        getter: impl Fn(&mut C) -> &mut Self,
        value: T,
    ) {
        if getter(modifier).value != value {
            Self::dependency_set(modifier, getter, value);
        }
    }
}

impl<T: Clone, C> Observe<T, C> {
    /// For when you need to set some element but the subscribers depend on the parent element.
    ///
    /// This function will always call the subscribers, regardless of the value being new.
    pub fn dependency_set(modifier: &mut C, getter: impl Fn(&mut C) -> &mut Self, value: T) {
        getter(modifier).value = value.clone();
        let subscribers = getter(modifier).subscribers.clone();
        for sub in &subscribers {
            sub(modifier, value.clone());
        }
    }

    /// Create a new value without subscribers
    pub fn new(value: T) -> Self {
        Self {
            value,
            subscribers: vec![],
        }
    }

    /// Get the contained value. No direct access is given to ensure that the observers are always
    /// up to date.
    pub fn get(&self) -> &T {
        &self.value
    }

    /// Set the value to some other value and call all subscribers with the same top level
    /// structure.
    ///
    /// This function will always call the subscribers, regardless of the value being new.
    pub fn set(&mut self, value: T, modifier: &mut C) {
        self.value = value.clone();
        for sub in &self.subscribers {
            sub(modifier, value.clone());
        }
    }

    /// Find a subscriber in the subscriber list
    fn find_subscriber(&self, function: Subscriber<C, T>) -> Option<usize> {
        let mut index = None;
        for (idx, sub) in self.subscribers.iter().enumerate() {
            if function as *const u8 == *sub as *const u8 {
                index = Some(idx);
                break;
            }
        }
        index
    }

    /// Add a subscriber (NOTE: Should only use named functions)
    pub fn subscribe(&mut self, function: Subscriber<C, T>) {
        if self.find_subscriber(function).is_none() {
            self.subscribers.push(function);
        }
    }

    /// Remove a subscriber (NOTE: Only works with named functions)
    pub fn unsubscribe(&mut self, function: Subscriber<C, T>) {
        if let Some(idx) = self.find_subscriber(function) {
            self.subscribers.remove(idx);
        }
    }

    /// Count the amount of subscribers
    pub fn count_subscribers(&self) -> usize {
        self.subscribers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;

    #[quickcheck_macros::quickcheck]
    fn simple_set_and_get(value: i32) {
        let mut obs = Observe::<i32, ()>::new(0);
        obs.set(value, &mut ());
        assert_eq![value, *obs.get()];
    }

    #[quickcheck_macros::quickcheck]
    fn simple_set_and_get_with_subscriber(value: i32) {
        let mut obs = Observe::<i32, i64>::new(0);
        obs.subscribe(|ctx, value| *ctx = value as i64 + 1);
        let mut ctx = 0;
        obs.set(value, &mut ctx);
        assert_eq![value, *obs.get()];
        assert_eq![ctx, *obs.get() as i64 + 1];
    }

    #[quickcheck_macros::quickcheck]
    fn simple_set_and_get_with_subscriber_unsubscribe(value: i32, initial: i64) {
        let mut obs = Observe::<i32, i64>::new(0);
        fn subscriber(ctx: &mut i64, value: i32) {
            *ctx = value as i64 + 1;
        }
        obs.subscribe(subscriber);
        let mut ctx = initial;
        obs.unsubscribe(subscriber);
        obs.set(value, &mut ctx);
        assert_eq![value, *obs.get()];
        assert_eq![ctx, initial];
    }

    #[quickcheck_macros::quickcheck]
    fn setting_same_subscribers_runs_only_once(value: i32, mut count: u16) {
        count = count.max(1);

        let mut obs = Observe::<i32, i64>::new(0);
        fn subscriber(ctx: &mut i64, _: i32) {
            *ctx += 1;
        }
        for _ in 0..count {
            obs.subscribe(subscriber);
        }
        let mut ctx = 0;
        obs.set(value, &mut ctx);
        assert_eq![value, *obs.get()];
        assert_eq![1, ctx];
        assert_eq![1, obs.count_subscribers()];

        obs.unsubscribe(|_, _| {});
        assert_eq![1, obs.count_subscribers()];

        obs.unsubscribe(subscriber);
        assert_eq![0, obs.count_subscribers()];
    }

    #[quickcheck_macros::quickcheck]
    fn compare_and_setting_does_not_run_more_than_once(mut value: i32, mut count: u16) {
        value = value.max(1);
        count = count.max(1);

        let mut obs = Observe::<i32, i64>::new(0);
        obs.subscribe(|ctx, _| *ctx += 1);
        let mut ctx = 0;

        for _ in 0..count {
            obs.compare_and_set(value, &mut ctx);
        }
        assert_eq![1, ctx];
    }

    #[quickcheck_macros::quickcheck]
    fn setting_n_times_calls_subscriber_n_times(value: i32, count: u16) {
        let mut obs = Observe::<i32, i32>::new(0);
        let mut ctx = 0;
        obs.subscribe(|ctx, _| *ctx += 1);
        for _ in 0..count as u32 + 1 {
            obs.set(value, &mut ctx);
        }
        assert_eq![value, *obs.get()];
        assert_eq![count as i32 + 1, ctx];
    }

    #[quickcheck_macros::quickcheck]
    fn dependency_set_works(value: i32) {
        struct Main {
            obs: Observe<i32, Main>,
            other: f32,
        }

        let mut main = Main {
            obs: Observe::new(0),
            other: 0.0,
        };

        main.obs.subscribe(|ctx, value| ctx.other = value as f32);
        Observe::dependency_set(&mut main, |x| &mut x.obs, value);

        assert_eq![main.other, value as f32];
    }

    #[quickcheck_macros::quickcheck]
    fn dependency_set_and_copare_works(value: i32, mut count: u16) {
        count = count.max(1);

        struct Main {
            obs: Observe<i32, Main>,
            other: usize,
        }

        let mut main = Main {
            obs: Observe::new(0),
            other: 0,
        };

        main.obs.subscribe(|ctx, _| ctx.other += 1);
        for _ in 0..count {
            Observe::dependency_compare_and_set(&mut main, |x| &mut x.obs, value);
        }

        assert_eq![1, main.other];
    }

    #[bench]
    fn adding_and_removing_subscribers(b: &mut Bencher) {
        let mut obs = Observe::new(0);
        fn subscriber(_: &mut (), _: i32) {}
        b.iter(|| {
            obs.subscribe(subscriber);
            obs.unsubscribe(subscriber);
        });
    }
}
