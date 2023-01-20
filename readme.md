# concess

> :warning: **Early Development:** This is not production ready, yet. Do not use it for anything important.

## Introduction
`concess` is an over-simplified, featherweight, open-source and easy-to-use authentication and authorization server.
It mimics a variety of authentication protocols while managing users in a simple list of files.

All data used and processed by `concess` is stored in a single directory containing a human-readable YAML file for every user.

Currently, the following methods are supported:
- [x] LDAP
- [x] RADIUS
- [ ] webauthn
- [ ] OpenID connect
- [ ] OAuth2

## Building
`concess` is written in Rust, so a [Rust installation](https://www.rust-lang.org/) is required.
Currently, an unstable rust compiler is required to build the project.

To build:
```shell
$ git clone https://github.com/fooker/concess
$ cd concess
$ cargo build --release
$ ./target/release/concess --version
```

## Usage
Create a config file called `concess.yaml` by adapting the [Example](example/concess.yaml).

After that, create the `data` directory in the location specified in the config file.
In there, create users by creating a file per user in the `users` directory inside the `data` directory.
Each user entry must follow the naming scheme `NAME.yaml`, whereas the `NAME` is the username of the user.
See the [Examples](example/data/users/) again for inspiration and syntax.

For groups, each user can be assigned to an arbitrary number of groups.
There is no further configuration required for a group to exist - it will as long as there is at least a user in there. 