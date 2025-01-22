use serde::de::{self, Deserializer, IntoDeserializer as _, SeqAccess, Visitor};
use serde::ser::{SerializeSeq, Serializer};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Struct containing either a single item or a list of items of type T.
/// If a single item is present, `first` will contain it and `rest` will be empty.
/// If multiple items are present, `first` will contain the first item and `rest` will contain the rest.
/// IMPORTANT: this struct cannot be created with an empty vector.
/// OneOrMany objects can only be created using OneOrMany::from() or OneOrMany::try_from().
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct OneOrMany<T> {
    /// First item in the list.
    first: T,
    /// Rest of the items in the list.
    rest: Vec<T>,
}

/// Error type for when trying to create a OneOrMany object with an empty vector.
#[derive(Debug, thiserror::Error)]
#[error("Cannot create OneOrMany with an empty vector.")]
pub struct EmptyListError;

impl<T: Clone> OneOrMany<T> {
    /// Get the first item in the list.
    pub fn first(&self) -> T {
        self.first.clone()
    }

    /// Get the rest of the items in the list (excluding the first one).
    pub fn rest(&self) -> Vec<T> {
        self.rest.clone()
    }

    /// After `OneOrMany<T>` is created, add an item of type T to the `rest`.
    pub fn push(&mut self, item: T) {
        self.rest.push(item);
    }

    /// After `OneOrMany<T>` is created, insert an item of type T at an index.
    pub fn insert(&mut self, index: usize, item: T) {
        if index == 0 {
            let old_first = std::mem::replace(&mut self.first, item);
            self.rest.insert(0, old_first);
        } else {
            self.rest.insert(index - 1, item);
        }
    }

    /// Length of all items in `OneOrMany<T>`.
    pub fn len(&self) -> usize {
        1 + self.rest.len()
    }

    /// If `OneOrMany<T>` is empty. This will always be false because you cannot create an empty `OneOrMany<T>`.
    /// This method is required when the method `len` exists.
    pub fn is_empty(&self) -> bool {
        false
    }

    /// Create a OneOrMany object with a single item of any type.
    pub fn one(item: T) -> Self {
        OneOrMany {
            first: item,
            rest: vec![],
        }
    }

    /// Create a OneOrMany object with a vector of items of any type.
    pub fn many(items: Vec<T>) -> Result<Self, EmptyListError> {
        let mut iter = items.into_iter();
        Ok(OneOrMany {
            first: match iter.next() {
                Some(item) => item,
                None => return Err(EmptyListError),
            },
            rest: iter.collect(),
        })
    }

    /// Merge a list of OneOrMany items into a single OneOrMany item.
    pub fn merge(one_or_many_items: Vec<OneOrMany<T>>) -> Result<Self, EmptyListError> {
        let items = one_or_many_items
            .into_iter()
            .flat_map(|one_or_many| one_or_many.into_iter())
            .collect::<Vec<_>>();

        OneOrMany::many(items)
    }

    /// Specialized map function for OneOrMany objects.
    ///
    /// Since OneOrMany objects have *atleast* 1 item, using `.collect::<Vec<_>>()` and
    /// `OneOrMany::many()` is fallible resulting in unergonomic uses of `.expect` or `.unwrap`.
    /// This function bypasses those hurdles by directly constructing the `OneOrMany` struct.
    pub fn map<U, F: FnMut(T) -> U>(self, mut op: F) -> OneOrMany<U> {
        OneOrMany {
            first: op(self.first),
            rest: self.rest.into_iter().map(op).collect(),
        }
    }

    /// Specialized try map function for OneOrMany objects.
    ///
    /// Same as `OneOrMany::map` but fallible.
    pub fn try_map<U, E, F: FnMut(T) -> Result<U, E>>(self, mut op: F) -> Result<OneOrMany<U>, E> {
        Ok(OneOrMany {
            first: op(self.first)?,
            rest: self
                .rest
                .into_iter()
                .map(op)
                .collect::<Result<Vec<_>, E>>()?,
        })
    }

    pub fn iter(&self) -> Iter<T> {
        Iter {
            first: Some(&self.first),
            rest: self.rest.iter(),
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut {
            first: Some(&mut self.first),
            rest: self.rest.iter_mut(),
        }
    }
}

// ================================================================
// Implementations of Iterator for OneOrMany
//   - OneOrMany<T>::iter() -> iterate over references of T objects
//   - OneOrMany<T>::into_iter() -> iterate over owned T objects
//   - OneOrMany<T>::iter_mut() -> iterate over mutable references of T objects
// ================================================================

/// Struct returned by call to `OneOrMany::iter()`.
pub struct Iter<'a, T> {
    // References.
    first: Option<&'a T>,
    rest: std::slice::Iter<'a, T>,
}

/// Implement `Iterator` for `Iter<T>`.
/// The Item type of the `Iterator` trait is a reference of `T`.
impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(first) = self.first.take() {
            Some(first)
        } else {
            self.rest.next()
        }
    }
}

/// Struct returned by call to `OneOrMany::into_iter()`.
pub struct IntoIter<T> {
    // Owned.
    first: Option<T>,
    rest: std::vec::IntoIter<T>,
}

/// Implement `Iterator` for `IntoIter<T>`.
impl<T: Clone> IntoIterator for OneOrMany<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            first: Some(self.first),
            rest: self.rest.into_iter(),
        }
    }
}

/// Implement `Iterator` for `IntoIter<T>`.
/// The Item type of the `Iterator` trait is an owned `T`.
impl<T: Clone> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(first) = self.first.take() {
            Some(first)
        } else {
            self.rest.next()
        }
    }
}

/// Struct returned by call to `OneOrMany::iter_mut()`.
pub struct IterMut<'a, T> {
    // Mutable references.
    first: Option<&'a mut T>,
    rest: std::slice::IterMut<'a, T>,
}

// Implement `Iterator` for `IterMut<T>`.
// The Item type of the `Iterator` trait is a mutable reference of `OneOrMany<T>`.
impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(first) = self.first.take() {
            Some(first)
        } else {
            self.rest.next()
        }
    }
}

impl<T: Clone> Serialize for OneOrMany<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for e in self.iter() {
            seq.serialize_element(e)?;
        }
        seq.end()
    }
}

impl<'de, T> Deserialize<'de> for OneOrMany<T>
where
    T: Deserialize<'de> + Clone,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct OneOrManyVisitor<T>(std::marker::PhantomData<T>);

        impl<'de, T> Visitor<'de> for OneOrManyVisitor<T>
        where
            T: Deserialize<'de> + Clone,
        {
            type Value = OneOrMany<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a sequence of at least one element or a single element")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let first = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let mut rest = Vec::new();
                while let Some(value) = seq.next_element()? {
                    rest.push(value);
                }
                Ok(OneOrMany { first, rest })
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let first: T = Deserialize::deserialize(value.into_deserializer())?;
                Ok(OneOrMany::one(first))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let first: T = Deserialize::deserialize(value.into_deserializer())?;
                Ok(OneOrMany::one(first))
            }

            fn visit_map<M>(self, map: M) -> Result<Self::Value, M::Error>
            where
                M: de::MapAccess<'de>,
            {
                let first: T =
                    Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))?;
                Ok(OneOrMany::one(first))
            }
        }

        deserializer.deserialize_any(OneOrManyVisitor(std::marker::PhantomData))
    }
}

#[cfg(test)]
mod test {
    use super::OneOrMany;

    #[test]
    fn test_single() {
        let one_or_many = OneOrMany::one("hello".to_string());

        assert_eq!(one_or_many.iter().count(), 1);

        one_or_many.iter().for_each(|i| {
            assert_eq!(i, "hello");
        });
    }

    #[test]
    fn test() {
        let one_or_many = OneOrMany::many(vec!["hello".to_string(), "word".to_string()]).unwrap();

        assert_eq!(one_or_many.iter().count(), 2);

        one_or_many.iter().enumerate().for_each(|(i, item)| {
            if i == 0 {
                assert_eq!(item, "hello");
            }
            if i == 1 {
                assert_eq!(item, "word");
            }
        });
    }

    #[test]
    fn test_one_or_many_into_iter_single() {
        let one_or_many = OneOrMany::one("hello".to_string());

        assert_eq!(one_or_many.clone().into_iter().count(), 1);

        one_or_many.into_iter().for_each(|i| {
            assert_eq!(i, "hello".to_string());
        });
    }

    #[test]
    fn test_one_or_many_into_iter() {
        let one_or_many = OneOrMany::many(vec!["hello".to_string(), "word".to_string()]).unwrap();

        assert_eq!(one_or_many.clone().into_iter().count(), 2);

        one_or_many.into_iter().enumerate().for_each(|(i, item)| {
            if i == 0 {
                assert_eq!(item, "hello".to_string());
            }
            if i == 1 {
                assert_eq!(item, "word".to_string());
            }
        });
    }

    #[test]
    fn test_one_or_many_merge() {
        let one_or_many_1 = OneOrMany::many(vec!["hello".to_string(), "word".to_string()]).unwrap();

        let one_or_many_2 = OneOrMany::one("sup".to_string());

        let merged = OneOrMany::merge(vec![one_or_many_1, one_or_many_2]).unwrap();

        assert_eq!(merged.iter().count(), 3);

        merged.iter().enumerate().for_each(|(i, item)| {
            if i == 0 {
                assert_eq!(item, "hello");
            }
            if i == 1 {
                assert_eq!(item, "word");
            }
            if i == 2 {
                assert_eq!(item, "sup");
            }
        });
    }

    #[test]
    fn test_mut_single() {
        let mut one_or_many = OneOrMany::one("hello".to_string());

        assert_eq!(one_or_many.iter_mut().count(), 1);

        one_or_many.iter_mut().for_each(|i| {
            assert_eq!(i, "hello");
        });
    }

    #[test]
    fn test_mut() {
        let mut one_or_many =
            OneOrMany::many(vec!["hello".to_string(), "word".to_string()]).unwrap();

        assert_eq!(one_or_many.iter_mut().count(), 2);

        one_or_many.iter_mut().enumerate().for_each(|(i, item)| {
            if i == 0 {
                item.push_str(" world");
                assert_eq!(item, "hello world");
            }
            if i == 1 {
                assert_eq!(item, "word");
            }
        });
    }

    #[test]
    fn test_one_or_many_error() {
        assert!(OneOrMany::<String>::many(vec![]).is_err())
    }

    #[test]
    fn test_len_single() {
        let one_or_many = OneOrMany::one("hello".to_string());

        assert_eq!(one_or_many.len(), 1);
    }

    #[test]
    fn test_len_many() {
        let one_or_many = OneOrMany::many(vec!["hello".to_string(), "word".to_string()]).unwrap();

        assert_eq!(one_or_many.len(), 2);
    }
}
