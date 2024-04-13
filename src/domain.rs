//! src/domain.rs
use unicode_segmentation::UnicodeSegmentation;

// A _Tuple Struct_
#[derive(Debug)]
pub struct SubscriberName(String);

pub struct NewSubscriber {
    pub email: String,
    pub name: SubscriberName,
}

impl SubscriberName {
    /// Return an instance if inputs satisfy validation constraints.
    /// Else, panic!
    pub fn parse(s: String) -> Result<SubscriberName, String> {
        // `.trim()` returns a view over the input without trailing
        // whitespace-like characters.
        let is_empty_or_whitespace = s.trim().is_empty();

        let is_too_long = s.graphemes(true).count() > 256;
        let forbidden_characters = ['/', '(', ')', '"', '<', '>', '\\', ';', '{', '}'];
        let contains_forbidden_characters: bool =
            s.chars().any(|g| forbidden_characters.contains(&g));

        if is_empty_or_whitespace || is_too_long || contains_forbidden_characters {
            panic!("{} is not a valid subscriber name.", s)
        } else {
            Ok(Self(s))
        }
    }
    pub fn inner(self) -> String {
        // note that this method consumes `self`
        // you will no longer have `SubscriberName`
        self.0
    }
    /*
    pub fn inner_mut(&mut self) -> &mut str {
        // This may defeat the purpose of parsing
        &mut self.0
    }
    */
    /// Can probably remove this since we use AsRef trait
    pub fn inner_ref(&self) -> &str {
        // Shared reference to inner string.
        // it is **read-only** access
        // cannot compormise our invariants
        &self.0
    }
}

impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// --- Unit Tests --- //
#[cfg(test)]
mod tests {
    // use crate::domain::SubscriberName;
    use super::*;
    use claims::{assert_err, assert_ok};

    #[test]
    fn a_256_grapheme_long_name_is_valid() {
        let name: String = "Æ".repeat(256);
        assert_ok!(SubscriberName::parse(name));
    }

    #[test]
    fn name_longer_than_256_graphemes_is_rejected() {
        let name: String = "s".repeat(257);
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn whitespace_only_names_rejected() {
        let name = " ".to_string();
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn empty_string_is_rejected() {
        todo!();
    }
}
