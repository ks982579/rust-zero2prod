// test/api/main.rs

mod change_password;
/** test/api/main.rs
* Structure api/ as we do a binary crate.
* Building tests now will create a single `api-<hash>` file.
*/
mod health_check;
mod helpers;
mod login;
mod newsletter;
mod subscriptions;
mod subscriptions_confirm;
