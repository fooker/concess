use argon2::Argon2;
use password_hash::{PasswordHashString, PasswordVerifier};

use crate::Database;

#[derive(Clone)]
pub struct User<'db, 'data> {
    pub name: &'data str,

    pub password: &'data PasswordHashString,

    pub first_name: &'data str,
    pub last_name: &'data str,

    pub mail: &'data str,

    pub groups: &'data Vec<String>,

    pub(super) database: &'db Database,
}

impl<'db, 'data> User<'db, 'data> {
    pub fn groups(&'db self) -> impl Iterator<Item=Group<'db, '_>> + 'db {
        return self.groups.iter()
            .map(|group| Group {
                name: &group,
                database: self.database,
            });
    }

    pub fn verify_password(&self, password: &[u8]) -> bool {
        return Argon2::default()
            .verify_password(password, &self.password.password_hash())
            .is_ok();
    }
}

#[derive(Clone)]
pub struct Group<'db, 'data> {
    pub name: &'data str,

    pub(super) database: &'db Database,
}

impl<'db, 'data> Group<'db, 'data> {
    pub fn members(&'db self) -> impl Iterator<Item=User<'db, '_>> + 'db {
        return self.database.users.iter()
            .filter(|user| user.groups.iter().any(|group| group == self.name))
            .map(|user| User {
                name: &user.name,
                password: &user.password,
                first_name: &user.first_name,
                last_name: &user.last_name,
                mail: &user.mail,
                groups: &user.groups,
                database: self.database,
            });
    }
}