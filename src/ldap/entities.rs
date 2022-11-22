use std::ops::Deref;

use crate::database::{Group, User};

use super::dn::{AttributeName, DN};

pub trait Entity {
    /// The object classes of this entity
    const OBJECT_CLASSES: &'static [&'static str];

    /// Return all exposed attribute names
    const ATTRIBUTES: &'static [AttributeName];

    /// DN of entity relative to the global base DN
    fn dn(&self) -> DN;

    /// Get the values of the given attribute
    fn get(attribute: &AttributeName) -> Option<for<'a> fn(&'a Self) -> Vec<String>>;

    /// Checks whether the attribute is present or not
    fn has(attribute: &AttributeName) -> bool {
        return Self::get(attribute).is_some();
    }
}

const ATTR_OBJECT_CLASS: AttributeName = AttributeName::from("objectClass");
const ATTR_ENTRY_DN: AttributeName = AttributeName::from("entryDN");
const ATTR_CN: AttributeName = AttributeName::from("cn");
const ATTR_DISPLAY_NAME: AttributeName = AttributeName::from("displayName");
const ATTR_GIVEN_NAME: AttributeName = AttributeName::from("givenName");
const ATTR_SN: AttributeName = AttributeName::from("sn");
const ATTR_MAIL: AttributeName = AttributeName::from("mail");
const ATTR_MEMBER_OF: AttributeName = AttributeName::from("memberOf");
const ATTR_UNIQUE_MEMBERS: AttributeName = AttributeName::from("uniqueMembers");

pub struct WithBaseDN<'dn, T> {
    base_dn: &'dn DN,
    entity: T,
}

impl<'dn, T> WithBaseDN<'dn, T> {
    pub fn base_dn(&self) -> &'dn DN {
        return self.base_dn;
    }
}

impl<'dn, T> Deref for WithBaseDN<'dn, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        return &self.entity;
    }
}

impl User<'_, '_> {
    pub fn with_base_dn(self, base_dn: &DN) -> WithBaseDN<Self> {
        return WithBaseDN {
            base_dn,
            entity: self,
        };
    }
}

impl Entity for WithBaseDN<'_, User<'_, '_>> {
    const OBJECT_CLASSES: &'static [&'static str] = &[
        "inetOrgPerson",
        "organizationalPerson",
        "person",
        "top",
    ];

    const ATTRIBUTES: &'static [AttributeName] = &[
        ATTR_OBJECT_CLASS,
        ATTR_ENTRY_DN,
        ATTR_CN,
        ATTR_DISPLAY_NAME,
        ATTR_GIVEN_NAME,
        ATTR_SN,
        ATTR_MAIL,
        ATTR_MEMBER_OF,
    ];

    fn dn(&self) -> DN {
        return self.base_dn()
            .join(("ou", "users"))
            .join(("cn", self.name));
    }

    fn get(attribute: &AttributeName) -> Option<for<'a> fn(&'a Self) -> Vec<String>> {
        if attribute == &ATTR_OBJECT_CLASS {
            return Some(|_| Self::OBJECT_CLASSES.iter().map(ToString::to_string).collect());
        }

        if attribute == &ATTR_ENTRY_DN {
            return Some(|e| vec![e.dn().to_string()]);
        }

        if attribute == &ATTR_CN {
            return Some(|e| vec![e.name.to_string()]);
        }

        if attribute == &ATTR_DISPLAY_NAME {
            return Some(|e| vec![e.name.to_string()]);
        }

        if attribute == &ATTR_GIVEN_NAME {
            return Some(|e| vec![e.first_name.to_string()]);
        }

        if attribute == &ATTR_SN {
            return Some(|e| vec![e.last_name.to_string()]);
        }

        if attribute == &ATTR_MAIL {
            return Some(|e| vec![e.mail.to_string()]);
        }

        if attribute == &ATTR_MEMBER_OF {
            return Some(|e| e.groups()
                .map(|group| group.with_base_dn(e.base_dn()))
                .map(|group| group.dn().to_string())
                .collect());
        }

        return None;
    }
}

impl Group<'_, '_> {
    pub fn with_base_dn(self, base_dn: &DN) -> WithBaseDN<Self> {
        return WithBaseDN {
            base_dn,
            entity: self,
        };
    }
}

impl Entity for WithBaseDN<'_, Group<'_, '_>> {
    const OBJECT_CLASSES: &'static [&'static str] = &[
        "groupOfUniqueNames",
        "top"
    ];

    const ATTRIBUTES: &'static [AttributeName] = &[
        ATTR_OBJECT_CLASS,
        ATTR_ENTRY_DN,
        ATTR_CN,
        ATTR_UNIQUE_MEMBERS,
    ];

    fn dn(&self) -> DN {
        return self.base_dn
            .join(("ou", "groups"))
            .join(("cn", self.name));
    }

    fn get(attribute: &AttributeName) -> Option<for<'a> fn(&'a Self) -> Vec<String>> {
        if attribute == &ATTR_OBJECT_CLASS {
            return Some(|_| Self::OBJECT_CLASSES.iter().map(ToString::to_string).collect());
        }

        if attribute == &ATTR_ENTRY_DN {
            return Some(|e| vec![e.dn().to_string()]);
        }

        if attribute == &ATTR_CN {
            return Some(|e| vec![e.name.to_string()]);
        }

        if attribute == &ATTR_UNIQUE_MEMBERS {
            return Some(|e| e.members()
                .map(|user| user.with_base_dn(e.base_dn()))
                .map(|user| user.dn().to_string())
                .collect());
        }

        return None;
    }
}
