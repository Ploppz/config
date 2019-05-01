use serde::de::DeserializeOwned;

pub trait ConfigType {
    fn set(&mut self, path: Path, value: String) -> Result<(), failure::Error>;
    fn is_leaf() -> bool {
        true
    }
    fn get_paths() -> Vec<Path> {
        Vec::new()
    }
}

macro_rules! basic_impl {
    ($ty:ty) => {
        impl $crate::ConfigType for $ty {
            fn set(&mut self, _path: Path, value: String) -> Result<(), failure::Error> {
                *self = ron::de::from_str(&value)?;
                Ok(())
            }
        }
    };
}
basic_impl!(i8);
basic_impl!(i16);
basic_impl!(i32);
basic_impl!(i64);

basic_impl!(u8);
basic_impl!(u16);
basic_impl!(u32);
basic_impl!(u64);

basic_impl!(f32);
basic_impl!(f64);

basic_impl!(bool);

impl ConfigType for String {
    fn set(&mut self, _path: Path, value: String) -> Result<(), failure::Error> {
        *self = value;
        Ok(())
    }
}

impl<X: DeserializeOwned, Y: DeserializeOwned> ConfigType for (X, Y) {
    fn set(&mut self, _path: Path, value: String) -> Result<(), failure::Error> {
        *self = ron::de::from_str(&value)?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct Path {
    nodes: Vec<String>,
}
impl Path {
    pub fn new(nodes: Vec<String>) -> Path {
        Path {
            nodes: nodes.into_iter().rev().collect(),
        }
    }
    pub fn pop_front(&mut self) -> Option<String> {
        self.nodes.pop()
    }
    pub fn push_front(&mut self, value: String) {
        self.nodes.push(value)
    }
    pub fn len(&self) -> usize {
        self.nodes.len()
    }
}

// Note: makes everything pub
#[macro_export]
macro_rules! is_string {
    { String } => {true};
    { $y:ty } => {false};
}
#[macro_export]
macro_rules! is_f32 {
    { f32 } => {true};
    { $y:ty } => {false};
}

#[macro_export]
macro_rules! get_paths_recurse {
    { $x:ident : $y:ty, $paths:ident } => {
        let field_name = stringify!($x).to_string();
            for mut path in <$y>::get_paths() {
                path.push_front(field_name.clone());
                $paths.push(path);
            }
    };
}

// NOTE: ignore the $(#[$($m:meta)*])* and corresponding $(#[$($m)*])* when reading. These are just
// to pass meta items to ALL struct definitions.
#[macro_export]
macro_rules! config {
    { $(#[$($m:meta)*])* struct $name:ident { $($t:tt)* } } => {
        $crate::config!{ @define $(#[$($m)*])* $($t)* }
        $crate::config!{ @make_struct $(#[$($m)*])* $name { $($t)* } }
    };

    // Make struct. Ignore substructures. These are already processesd somewhere else.
    { @make_struct $(#[$($m:meta)*])* $name:ident { $($x:ident : $y:ty $({ $($t:tt)* })* $(,)* )* } } => {
        $(#[$($m)*])*
        pub struct $name {
            $(pub $x: $y),*
        }
        impl $crate::ConfigType for $name {
            fn is_leaf() -> bool {false}
            fn set(&mut self, mut path: $crate::Path, value: String) -> Result<(), failure::Error> {
                use failure::bail;
                // TODO/NOTE: path could also easily be &mut if that performs better

                match path.pop_front() {
                    Some(field) => {
                        $(
                        if field == stringify!($x) {
                            self.$x.set(path.clone(), value.clone())?;
                        }
                        // TODO: else-if, and else: error
                        )*
                    },
                    None => bail!("Error in path"),
                }
                Ok(())
            }
            fn get_paths() -> Vec<$crate::Path> {
                let mut paths = Vec::new();
                $( {
                    get_paths_recurse!($x: $y, paths);
                } )*
                paths
            }
        }
    };

    // accept a sub-structure (and rest)
    { @define $(#[$($m:meta)*])* $x:ident: $y:ident { $($t:tt)* }, $($rest:tt)* } => {
        $crate::config!{$(#[$($m)*])* struct $y { $($t)* } }
        $crate::config!{@define $(#[$($m)*])* $($rest)*}
    };

    // The above rule, but just to accept ','
    { @define $(#[$($m:meta)*])* $x:ident: $y:ident { $($t:tt)* } $($rest:tt)* } => {
        $crate::config!{@define $(#[$($m)*])* $x: $y { $($t)* }, $($rest)* }
    };
    // fields
    { @define $(#[$($m:meta)*])* $x:ident: $y:ty, $($rest:tt)* } => {
        $crate::config!{@define $(#[$($m)*])* $($rest)*}
    };
    { @define $(#[$($m:meta)*])* $x:ident: $y:ty } => {
    };
    { @define $(#[$($m:meta)*])* } => {
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_config() {
        config![
            struct Empty {}
        ];
    }

    #[test]
    fn single_entry_allows_assignment_and_creation() {
        config![
            struct Single {
                entry: f32,
            }
        ];

        let mut x = Single { entry: 0.0 };
        x.entry = 1.0;
        assert_eq![1.0, x.entry];
    }

    #[test]
    fn single_entry_in_nested_structure() {
        config![
            struct Single {
                entry: Entry {
                    entry: Entry2 {
                        real_entry: f32,
                    },
                },
            }
        ];

        let mut x = Single {
            entry: Entry {
                entry: Entry2 { real_entry: 0.0 },
            },
        };
        x.entry.entry.real_entry = 1.0;
        assert_eq![1.0, x.entry.entry.real_entry];
    }
}
