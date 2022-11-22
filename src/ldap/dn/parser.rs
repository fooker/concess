use nom::{IResult, Parser};
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{alpha1, alphanumeric1, char, digit1, multispace0, none_of, one_of};
use nom::combinator::{map, map_res, recognize};
use nom::error::{Error, ParseError};
use nom::multi::{count, fold_many0, many0, separated_list0, separated_list1};
use nom::sequence::{delimited, pair, preceded, separated_pair};

use crate::ldap::dn::AttributeName;

use super::{Attribute, DN, RDN};

fn ws<'a, F, O, E: ParseError<&'a str>>(inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
    where
        F: Fn(&'a str) -> IResult<&'a str, O, E> + 'a,
{
    return delimited(multispace0,
                     inner,
                     multispace0);
}

pub fn attribute_name(i: &str) -> IResult<&str, AttributeName, Error<&str>> {
    let string = map(recognize::<&str, _, _, _>(
        pair(
            alpha1,
            many0(alt((alphanumeric1, tag("-")))),
        )), |s| AttributeName::from(s.to_lowercase()),
    );

    let oid = map(separated_list1(char('.'),
                                  map_res(digit1, str::parse)),
                  |parts| AttributeName::OID(parts));

    return alt((
        string,
        oid
    ))(i);
}

pub fn attribute_value(i: &str) -> IResult<&str, String, Error<&str>> {
    fn escaped(i: &str) -> IResult<&str, core::primitive::char, Error<&str>> {
        return preceded(char('\\'), one_of("\\\",=\r+<>#; "))(i);
    }

    let simple = fold_many0(alt((
        none_of(",=+<>#;\\\""),
        escaped
    )), String::new, |mut acc, c| {
        acc.push(c);
        return acc;
    }).map(|s| s.trim().to_owned());

    let quoted = delimited(char('"'), fold_many0(alt((
        none_of("\\\""),
        escaped
    )), String::new, |mut acc, c| {
        acc.push(c);
        return acc;
    }), char('"'));

    let hexstr = map_res(preceded(char('#'), many0(
        map_res(recognize(count(one_of("0123456789abcdefABCDEF"), 2)), |s| u8::from_str_radix(s, 16)),
    )), String::from_utf8);

    return alt((hexstr, quoted, simple))(i);
}

pub fn attribute(i: &str) -> IResult<&str, Attribute, Error<&str>> {
    return map(separated_pair(ws(attribute_name), char('='), ws(attribute_value)),
               |(name, value)| Attribute {
                   name,
                   value: value.to_owned(),
               })(i);
}

pub fn rdn(i: &str) -> IResult<&str, RDN, Error<&str>> {
    return map(separated_list0(char('+'), attribute),
               |attributes| RDN { attributes })(i);
}

pub fn dn(i: &str) -> IResult<&str, DN, Error<&str>> {
    return map(separated_list0(alt((char(','), char(';'))), rdn),
               |components| DN { components })(i);
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_attribute_name() {
        assert_eq!(attribute_name("cn"), Ok(("", AttributeName::from("cn"))));
        assert_eq!(attribute_name("foo1-22x"), Ok(("", AttributeName::from("foo1-22x"))));
        assert_eq!(attribute_name("1"), Ok(("", AttributeName::from([1]))));
        assert_eq!(attribute_name("1.2.3.4"), Ok(("", AttributeName::from([1, 2, 3, 4]))));
    }

    #[test]
    fn test_parse_attribute_value() {
        assert_eq!(attribute_value("foo"), Ok(("", String::from("foo"))));
        assert_eq!(attribute_value("42"), Ok(("", String::from("42"))));
        assert_eq!(attribute_value("some spaces  in  between"), Ok(("", String::from("some spaces  in  between"))));
        assert_eq!(attribute_value("test\\+name@example.com"), Ok(("", String::from("test+name@example.com"))));
        assert_eq!(attribute_value(""), Ok(("", String::from(""))));

        assert_eq!(attribute_value("\"bar\""), Ok(("", String::from("bar"))));
        assert_eq!(attribute_value("\"  spaced out   \""), Ok(("", String::from("  spaced out   "))));
        assert_eq!(attribute_value("\"i'm so+very=special,too\""), Ok(("", String::from("i'm so+very=special,too"))));
        assert_eq!(attribute_value("\"\""), Ok(("", String::from(""))));

        assert_eq!(attribute_value("#68656c6c6f"), Ok(("", String::from("hello"))));
        assert_eq!(attribute_value("#68656c6c6f20f09f8c8e"), Ok(("", String::from("hello 🌎"))));
        assert_eq!(attribute_value("#"), Ok(("", String::from(""))));
    }

    #[test]
    fn test_parse_attribute() {
        assert_eq!(attribute("foo=bar"), Ok(("", Attribute { name: AttributeName::from("foo"), value: String::from("bar") })));
        assert_eq!(attribute("foo="), Ok(("", Attribute { name: AttributeName::from("foo"), value: String::from("") })));
        assert_eq!(attribute("foo=so much bar"), Ok(("", Attribute { name: AttributeName::from("foo"), value: String::from("so much bar") })));
    }

    #[test]
    fn test_parse_rdn() {
        assert_eq!(rdn("foo=bar"), Ok(("", RDN {
            attributes: vec![
                Attribute { name: AttributeName::from("foo"), value: String::from("bar") },
            ]
        })));

        assert_eq!(rdn("foo=bar+bar=baz+baz=foo"), Ok(("", RDN {
            attributes: vec![
                Attribute { name: AttributeName::from("foo"), value: String::from("bar") },
                Attribute { name: AttributeName::from("bar"), value: String::from("baz") },
                Attribute { name: AttributeName::from("baz"), value: String::from("foo") },
            ]
        })));
    }

    #[test]
    fn test_parse_dn() {
        assert_eq!(dn("cn=foo,cn=bar,ou=foobar+x=baz"), Ok(("", DN {
            components: vec![
                RDN {
                    attributes: vec![
                        Attribute { name: AttributeName::from("cn"), value: String::from("foo") },
                    ]
                },
                RDN {
                    attributes: vec![
                        Attribute { name: AttributeName::from("cn"), value: String::from("bar") },
                    ]
                },
                RDN {
                    attributes: vec![
                        Attribute { name: AttributeName::from("ou"), value: String::from("foobar") },
                        Attribute { name: AttributeName::from("x"), value: String::from("baz") },
                    ]
                },
            ]
        })));

        assert_eq!(dn("cn=foo,cn=bar;ou=foobar"), Ok(("", DN {
            components: vec![
                RDN {
                    attributes: vec![
                        Attribute { name: AttributeName::from("cn"), value: String::from("foo") },
                    ]
                },
                RDN {
                    attributes: vec![
                        Attribute { name: AttributeName::from("cn"), value: String::from("bar") },
                    ]
                },
                RDN {
                    attributes: vec![
                        Attribute { name: AttributeName::from("ou"), value: String::from("foobar") },
                    ]
                },
            ]
        })));
    }


    #[test]
    fn test_parsing() {
        assert_eq!(dn("cn=foo\\,bar,OU=FOO\\,bar , OU=foo\\;bar;OU=foo\\;bar ; ou=foo\\,,ou=foo\\,;ou=foo\\;;ou=foo\\,;ou=bar\\,"),
                   Ok(("", DN {
                       components: vec![
                           RDN {
                               attributes: vec![
                                   Attribute { name: AttributeName::from("cn"), value: String::from("foo,bar") },
                               ]
                           },
                           RDN {
                               attributes: vec![
                                   Attribute { name: AttributeName::from("ou"), value: String::from("FOO,bar") },
                               ]
                           },
                           RDN {
                               attributes: vec![
                                   Attribute { name: AttributeName::from("ou"), value: String::from("foo;bar") },
                               ]
                           },
                           RDN {
                               attributes: vec![
                                   Attribute { name: AttributeName::from("ou"), value: String::from("foo;bar") },
                               ]
                           },
                           RDN {
                               attributes: vec![
                                   Attribute { name: AttributeName::from("ou"), value: String::from("foo,") },
                               ]
                           },
                           RDN {
                               attributes: vec![
                                   Attribute { name: AttributeName::from("ou"), value: String::from("foo,") },
                               ]
                           },
                           RDN {
                               attributes: vec![
                                   Attribute { name: AttributeName::from("ou"), value: String::from("foo;") },
                               ]
                           },
                           RDN {
                               attributes: vec![
                                   Attribute { name: AttributeName::from("ou"), value: String::from("foo,") },
                               ]
                           },
                           RDN {
                               attributes: vec![
                                   Attribute { name: AttributeName::from("ou"), value: String::from("bar,") },
                               ]
                           },
                       ]
                   })));
    }
}