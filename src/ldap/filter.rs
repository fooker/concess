use anyhow::{anyhow, Result};
use ldap3_proto::{LdapFilter, LdapSearchScope};

use crate::ldap::dn::DN;

use super::dn::AttributeName;
use super::entities::Entity;

pub enum Filter {
    And(Vec<Filter>),
    Or(Vec<Filter>),
    Not(Box<Filter>),
    Equality(AttributeName, String),
    Present(AttributeName),
}

impl Filter {
    pub fn evaluate<E: Entity>(&self, entity: &E) -> bool {
        return match self {
            Filter::And(filters) => filters.iter()
                .all(|filter| filter.evaluate(entity)),

            Filter::Or(filters) => filters.iter()
                .any(|filter| filter.evaluate(entity)),

            Filter::Not(filter) => !filter.evaluate(entity),

            Filter::Equality(attribute, expected) => match E::get(attribute).map(|attribute| attribute(entity)) {
                Some(values) => values.iter().any(|value| value == expected),
                None => false,
            },

            Filter::Present(attribute) => E::has(&attribute),
        };
    }
}

impl TryFrom<&LdapFilter> for Filter {
    type Error = anyhow::Error;

    fn try_from(value: &LdapFilter) -> std::result::Result<Self, Self::Error> {
        return match value {
            LdapFilter::And(filters) => Ok(Self::And(filters.iter().map(Filter::try_from).collect::<Result<_>>()?)),
            LdapFilter::Or(filters) => Ok(Self::Or(filters.iter().map(Filter::try_from).collect::<Result<_>>()?)),
            LdapFilter::Not(filter) => Ok(Self::Not(Box::new(Filter::try_from(filter.as_ref())?))),
            LdapFilter::Equality(attribute, value) => Ok(Self::Equality(attribute.parse()?, value.to_string())),
            LdapFilter::Substring(_, _) => Err(anyhow!("Not supported")),
            LdapFilter::Present(attribute) => Ok(Self::Present(attribute.parse()?)),
        };
    }
}

pub struct Scope {
    pub base: DN,
    pub scope: LdapSearchScope,
}

impl Scope {
    pub fn matches<E: Entity>(&self, entity: &E) -> bool {
        return match self.scope {
            LdapSearchScope::Base => entity.dn() == self.base,
            LdapSearchScope::OneLevel => entity.dn().parent() == self.base,
            LdapSearchScope::Subtree => entity.dn().is_descendant_of(&self.base),
        };
    }

    pub fn is_root_dse(&self) -> bool {
        return self.base == DN::ROOT && self.scope == LdapSearchScope::Base;
    }
}