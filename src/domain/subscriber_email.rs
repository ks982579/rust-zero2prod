use validator::ValidateEmail;

#[derive(Debug)]
pub struct SubscriberEmail(String);

impl ValidateEmail for SubscriberEmail {
    fn as_email_string(&self) -> Option<std::borrow::Cow<str>> {
        Some(std::borrow::Cow::Borrowed(&self.0))
    }
}

impl SubscriberEmail {
    pub fn parse(s: String) -> Result<SubscriberEmail, String> {
        let tmp: Self = SubscriberEmail(s);
        match tmp.validate_email() {
            true => Ok(tmp),
            false => Err(format!("{} is not a valid subscriber email.", tmp.0)),
        }
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use claims::assert_err;
    use fake::{faker::internet::en::SafeEmail, Fake};

    // Both `Clone` and `Debug` required by `quickcheck`
    #[derive(Debug, Clone)]
    struct ValidEmailFixture(pub String);

    impl quickcheck::Arbitrary for ValidEmailFixture {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            let email = SafeEmail().fake_with_rng(g);
            Self(email)
        }
    }

    #[test]
    fn empty_string_is_rejected() {
        let email = "".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let email = "ursula_at_example.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_missing_subject_is_rejected() {
        let email = "@example.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    // #[test]
    // fn valid_emails_are_parsed_successfully() {
    //     for _ in 0..10 {
    //         let email = SafeEmail().fake();
    //         claims::assert_ok!(SubscriberEmail::parse(email));
    //     }
    // }

    #[quickcheck_macros::quickcheck]
    fn valid_emails_are_parsed_successfully(valid_email: ValidEmailFixture) -> bool {
        SubscriberEmail::parse(valid_email.0).is_ok()
    }
}
