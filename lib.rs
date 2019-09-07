#![feature(test)]
extern crate test;

pub use crate::observer::Observe;
pub use failure::bail;
use serde::de::DeserializeOwned;

pub mod observer;

pub trait ConfigType {
    fn check_set<'a>(
        path: impl Iterator<Item = &'a str>,
        value: &str,
    ) -> Result<(), failure::Error>;
    fn set<'a>(
        &mut self,
        path: impl Iterator<Item = &'a str>,
        value: &str,
    ) -> Result<(), failure::Error>;
    fn get_descendants() -> &'static [&'static str] {
        &[]
    }
}

macro_rules! basic_impl {
    ($ty:ty) => {
        impl $crate::ConfigType for $ty {
            fn check_set<'a>(
                mut path: impl Iterator<Item = &'a str>,
                value: &str,
            ) -> Result<(), failure::Error> {
                if path.next().is_none() {
                    ron::de::from_str::<Self>(value)?;
                    Ok(())
                } else {
                    $crate::bail!["Path too long"]
                }
            }
            fn set<'a>(
                &mut self,
                mut path: impl Iterator<Item = &'a str>,
                value: &str,
            ) -> Result<(), failure::Error> {
                if path.next().is_some() {
                    $crate::bail!["Path too long"];
                }
                *self = ron::de::from_str(value)?;
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
    fn check_set<'a>(
        mut path: impl Iterator<Item = &'a str>,
        _value: &str,
    ) -> Result<(), failure::Error> {
        if path.next().is_none() {
            Ok(())
        } else {
            bail!["Path too long"]
        }
    }
    fn set<'a>(
        &mut self,
        mut path: impl Iterator<Item = &'a str>,
        value: &str,
    ) -> Result<(), failure::Error> {
        if path.next().is_some() {
            bail!["Path too long"];
        }
        *self = ron::de::from_str(value)?;
        Ok(())
    }
}

impl<X: DeserializeOwned, Y: DeserializeOwned> ConfigType for (X, Y) {
    fn check_set<'a>(
        mut path: impl Iterator<Item = &'a str>,
        value: &str,
    ) -> Result<(), failure::Error> {
        if path.next().is_some() {
            bail!["Path too long"];
        }
        ron::de::from_str::<Self>(value)?;
        Ok(())
    }
    fn set<'a>(
        &mut self,
        mut path: impl Iterator<Item = &'a str>,
        value: &str,
    ) -> Result<(), failure::Error> {
        if path.next().is_some() {
            bail!["Path too long"];
        }
        *self = ron::de::from_str(value)?;
        Ok(())
    }
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
            fn check_set<'a>(mut path: impl Iterator<Item = &'a str>, value: &str) -> Result<(), failure::Error> {
                match path.next() {
                    $(
                    Some(stringify![$x]) => {
                        <$y>::check_set(path, value)
                    }
                    )*
                    Some(_) => {
                        $crate::bail!["Path is not found"]
                    }
                    None => {
                        $crate::bail!["Path is too short"]
                    }
                }
            }
            fn set<'a>(&mut self, mut path: impl Iterator<Item = &'a str>, value: &str) -> Result<(), failure::Error> {
                if let Some(item) = path.next() {
                    match item {
                        $(
                            stringify![$x] => {
                                self.$x.set(path, value)
                            }
                        )*
                        _ => {
                            $crate::bail!["Path not found"]
                        }
                    }
                } else {
                    $crate::bail!["Path too short"];
                }
            }
            fn get_descendants() -> &'static [&'static str] {
                &[$(stringify![$x]),*]
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
    { @define $(#[$($m:meta)*])* $x:ident: $y:ty } => {};
    { @define $(#[$($m:meta)*])* } => {};
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::iter::once;

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

    #[test]
    fn single_entry_get_paths() {
        config![
            #[derive(Default)]
            struct Single {
                entry: f32,
                kek: TopKek {
                    nice: i32
                }
            }
        ];

        let mut x = Single::default();
        assert![Single::check_set(once("entry"), "0.3").is_ok()];
        assert![Single::check_set(once("entry"), "string").is_err()];
        assert![Single::check_set("kek.nice".split('.'), "3").is_ok()];
        assert![Single::check_set(once("kek"), "123").is_err()];
        assert![Single::check_set("kek.nice".split('.'), "0.3").is_err()];

        x.set(once("entry"), "0.3");
        assert_eq![0.3, x.entry];

        x.set("kek.nice".split('.'), "1234");
        assert_eq![1234, x.kek.nice];
    }

    #[test]
    fn simple_split() {
        println!["{:?}", "lorem.ipsum.dolor".split('.').collect::<Vec<_>>()];
    }
}
