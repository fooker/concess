use std::borrow::Cow;
use std::fmt;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::str::FromStr;

use itertools::{Itertools, Position};
use nom::Finish;
use serde::{Deserialize, Deserializer};

mod parser;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DN {
    components: Vec<RDN>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RDN {
    attributes: Vec<Attribute>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Attribute {
    name: AttributeName,
    value: String,
}

#[derive(Debug, Clone, Eq)]
pub enum AttributeName {
    String(Cow<'static, str>),
    OID(Vec<u16>),
}

impl PartialEq for AttributeName {
    fn eq(&self, other: &Self) -> bool {
        return match (self, other) {
            (Self::String(name), Self::String(other)) => name.to_ascii_lowercase().eq(&other.to_ascii_lowercase()),
            (Self::OID(oid), Self::OID(other)) => oid.eq(other),
            _ => false,
        };
    }
}

impl Hash for AttributeName {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::String(name) => name.to_ascii_lowercase().hash(state),
            Self::OID(oid) => oid.hash(state),
        }
    }
}

// Iterate through `iter` while it matches `prefix`; return `None` if `prefix`
// is not a prefix of `iter`, otherwise return `Some(iter_after_prefix)` giving
// `iter` after having exhausted `prefix`.
// Stolen from std::path
fn iter_after<'a, T, I, J>(mut iter: I, mut prefix: J) -> Option<I>
    where
        T: PartialEq + 'a,
        I: Iterator<Item=&'a T> + Clone,
        J: Iterator<Item=&'a T>,
{
    loop {
        let mut iter_next = iter.clone();
        match (iter_next.next(), prefix.next()) {
            (Some(ref x), Some(ref y)) if x == y => (),
            (Some(_), Some(_)) => return None,
            (Some(_), None) => return Some(iter),
            (None, None) => return Some(iter),
            (None, Some(_)) => return None,
        }
        iter = iter_next;
    }
}

impl DN {
    pub const ROOT: Self = Self { components: Vec::new() };

    pub fn iter(&self) -> impl Iterator<Item=&RDN> {
        return self.components.iter();
    }

    pub fn is_descendant_of(&self, parent: &DN) -> bool {
        return iter_after(self.components.iter().rev(), parent.components.iter().rev()).is_some();
    }

    pub fn is_ancestor_of(&self, child: &DN) -> bool {
        return child.is_descendant_of(self);
    }

    pub fn relative_to(&self, parent: &DN) -> Option<DN> {
        return iter_after(self.components.iter().rev(), parent.components.iter().rev())
            .map(|suffix| DN::from_iter(suffix.rev().cloned()));
    }

    pub fn join(&self, dn: impl Into<DN>) -> Self {
        let mut out = dn.into();
        out.components.extend(self.components.iter().cloned());
        return out;
    }

    pub fn parent(&self) -> Self {
        return Self {
            components: self.components.iter().skip(1).cloned().collect(),
        };
    }
}

impl RDN {
    pub fn iter(&self) -> impl Iterator<Item=&Attribute> {
        return self.attributes.iter();
    }
}

impl Attribute {
    pub fn name(&self) -> &AttributeName {
        return &self.name;
    }

    pub fn value(&self) -> &str {
        return &self.value;
    }
}

impl From<RDN> for DN {
    fn from(value: RDN) -> Self {
        return Self { components: vec![value] };
    }
}

impl From<Attribute> for RDN {
    fn from(value: Attribute) -> Self {
        return Self { attributes: vec![value] };
    }
}

impl<E> FromIterator<E> for DN
    where
        E: Into<RDN> {
    fn from_iter<T: IntoIterator<Item=E>>(iter: T) -> Self {
        return Self { components: iter.into_iter().map(Into::into).collect() };
    }
}

impl<E> FromIterator<E> for RDN
    where
        E: Into<Attribute> {
    fn from_iter<T: IntoIterator<Item=E>>(iter: T) -> Self {
        return Self { attributes: iter.into_iter().map(Into::into).collect() };
    }
}

impl<N, V> From<(N, V)> for DN
    where
        N: Into<AttributeName>,
        V: Into<String>,
{
    fn from((name, value): (N, V)) -> Self {
        return DN::from(RDN::from((name, value)));
    }
}

impl<N, V> From<(N, V)> for RDN
    where
        N: Into<AttributeName>,
        V: Into<String>,
{
    fn from((name, value): (N, V)) -> Self {
        return RDN::from(Attribute::from((name, value)));
    }
}

impl<N, V> From<(N, V)> for Attribute
    where
        N: Into<AttributeName>,
        V: Into<String>,
{
    fn from((name, value): (N, V)) -> Self {
        return Attribute {
            name: name.into(),
            value: value.into(),
        };
    }
}

impl const From<&'static str> for AttributeName {
    fn from(value: &'static str) -> Self {
        return Self::String(Cow::Borrowed(value));
    }
}

impl From<String> for AttributeName {
    fn from(value: String) -> Self {
        return Self::String(Cow::Owned(value));
    }
}

impl<const N: usize> From<[u16; N]> for AttributeName {
    fn from(value: [u16; N]) -> Self {
        return Self::OID(value.to_vec());
    }
}

impl FromStr for DN {
    type Err = nom::error::Error<String>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        return match parser::dn(s).finish() {
            Ok((_, dn)) => Ok(dn),
            Err(nom::error::Error { input, code }) => Err(nom::error::Error {
                input: input.to_owned(),
                code,
            }),
        };
    }
}

impl FromStr for RDN {
    type Err = nom::error::Error<String>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        return match parser::rdn(s).finish() {
            Ok((_, rdn)) => Ok(rdn),
            Err(nom::error::Error { input, code }) => Err(nom::error::Error {
                input: input.to_owned(),
                code,
            }),
        };
    }
}

impl FromStr for Attribute {
    type Err = nom::error::Error<String>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        return match parser::attribute(s).finish() {
            Ok((_, attribute)) => Ok(attribute),
            Err(nom::error::Error { input, code }) => Err(nom::error::Error {
                input: input.to_owned(),
                code,
            }),
        };
    }
}

impl FromStr for AttributeName {
    type Err = nom::error::Error<String>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        return match parser::attribute_name(s).finish() {
            Ok((_, attribute_name)) => Ok(attribute_name),
            Err(nom::error::Error { input, code }) => Err(nom::error::Error {
                input: input.to_owned(),
                code,
            }),
        };
    }
}

impl fmt::Display for DN {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut components = self.components.iter();
        if let Some(first) = components.next() {
            write!(f, "{}", first)?;
        }

        for component in components {
            write!(f, ",{}", component)?;
        }

        return Ok(());
    }
}

impl fmt::Display for RDN {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut attributes = self.attributes.iter();
        if let Some(first) = attributes.next() {
            write!(f, "{}", first)?;
        }

        for attribute in attributes {
            write!(f, "+{}", attribute)?;
        }

        return Ok(());
    }
}

impl fmt::Display for Attribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const ESCAPED: [char; 9] = ['#', ',', ';', '=', '+', '<', '>', '\"', '\\'];

        write!(f, "{}", self.name)?;
        write!(f, "=")?;

        for c in self.value.chars().with_position() {
            match c {
                Position::First(' ') |
                Position::Last(' ') |
                Position::Only(' ') => {
                    write!(f, "\\ ")?;
                }

                Position::First(c) |
                Position::Last(c) |
                Position::Middle(c) |
                Position::Only(c) => {
                    if c < ' ' {
                        write!(f, "\\{:02x}", c as u8)?;
                    } else if ESCAPED.contains(&c) {
                        write!(f, "\\{}", c)?;
                    } else {
                        write!(f, "{}", c)?;
                    }
                }
            }
        }

        return Ok(());
    }
}

impl fmt::Display for AttributeName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        return Ok(match self {
            AttributeName::String(name) => {
                write!(f, "{}", name)?;
            }
            AttributeName::OID(oid) => {
                write!(f, "{}", oid.iter()
                    .format_with(",", |elt, f| (f(elt))))?;
            }
        });
    }
}


impl PartialEq<str> for AttributeName {
    fn eq(&self, other: &str) -> bool {
        return match self {
            AttributeName::String(name) => name == other,
            AttributeName::OID(_) => false,
        };
    }
}

impl<'de> Deserialize<'de> for DN {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de> {
        let s = Deserialize::deserialize(deserializer)?;
        return Self::from_str(s)
            .map_err(serde::de::Error::custom);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use super::parser::*;

    #[test]
    fn test_join() {
        assert_eq!(DN::from_iter([("dc", "example"), ("dc", "com")]).join(("ou", "test")),
                   DN::from_iter([("ou", "test"), ("dc", "example"), ("dc", "com")]));
    }

    #[test]
    fn test_descendant() {
        assert!(DN::from_iter([("ou", "test"), ("dc", "example"), ("dc", "com")]).is_descendant_of(&DN::from_iter([("ou", "test"), ("dc", "example"), ("dc", "com")])));
        assert!(DN::from_iter([("ou", "test"), ("dc", "example"), ("dc", "com")]).is_descendant_of(&DN::from_iter([("dc", "example"), ("dc", "com")])));
        assert!(DN::from_iter([("ou", "test"), ("dc", "example"), ("dc", "com")]).is_descendant_of(&DN::from_iter([("dc", "com")])));
        assert!(DN::from_iter([("ou", "test"), ("dc", "example"), ("dc", "com")]).is_descendant_of(&DN::ROOT));


        assert!(!DN::from_iter([("ou", "test"), ("dc", "example"), ("dc", "com")]).is_descendant_of(&DN::from_iter([("dc", "broken"), ("dc", "com")])));
        assert!(!DN::from_iter([("ou", "test"), ("dc", "example"), ("dc", "com")]).is_descendant_of(&DN::from_iter([("dc", "com"), ("ex", "suffix")])));
    }

    #[test]
    fn test_ancestor() {
        assert!(DN::from_iter([("ou", "test"), ("dc", "example"), ("dc", "com")]).is_ancestor_of(&DN::from_iter([("ou", "test"), ("dc", "example"), ("dc", "com")])));
        assert!(DN::from_iter([("dc", "example"), ("dc", "com")]).is_ancestor_of(&DN::from_iter([("ou", "test"), ("dc", "example"), ("dc", "com")])));
        assert!(DN::from_iter([("dc", "com")]).is_ancestor_of(&DN::from_iter([("ou", "test"), ("dc", "example"), ("dc", "com")])));
        assert!(DN::ROOT.is_ancestor_of(&DN::from_iter([("ou", "test"), ("dc", "example"), ("dc", "com")])));


        assert!(!DN::from_iter([("dc", "broken"), ("dc", "com")]).is_ancestor_of(&DN::from_iter([("ou", "test"), ("dc", "example"), ("dc", "com")])));
        assert!(!DN::from_iter([("dc", "com"), ("ex", "suffix")]).is_ancestor_of(&DN::from_iter([("ou", "test"), ("dc", "example"), ("dc", "com")])));
    }

    #[test]
    fn test_relative_to() {
        assert_eq!(DN::from_iter([("cn", "myself"), ("ou", "test"), ("dc", "example"), ("dc", "com")]).relative_to(&DN::from_iter([("dc", "example"), ("dc", "com")])),
                   Some(DN::from_iter([("cn", "myself"), ("ou", "test")])));

        assert_eq!(DN::from_iter([("cn", "myself"), ("ou", "test"), ("dc", "example"), ("dc", "com")]).relative_to(&DN::from_iter([("ou", "broken"), ("dc", "example"), ("dc", "com")])),
                   None);
    }

    #[test]
    fn test_parent() {
        assert_eq!(DN::from_iter([("cn", "myself"), ("ou", "test"), ("dc", "example"), ("dc", "com")]).parent(),
                   DN::from_iter([("ou", "test"),  ("dc", "example"), ("dc", "com")]));

        assert_eq!(DN::from_iter([("dc", "com")]).parent(),
                   DN::ROOT);

        assert_eq!(DN::ROOT.parent(),
                   DN::ROOT);
    }
}