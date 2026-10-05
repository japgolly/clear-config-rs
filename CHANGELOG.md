# 1.0.0

* Add a `ConfigParser` for `Vec`
* Support `key_prefix` on derivation fields
* Add `ctx.with_key_prefix`
* Accept `&str` instead of `&String` in many places
* Add `ConfigSource::memory`
* `ConfigSource::env_file`:
  * Accept `impl AsRef<Path>` as the filename
  * Parse `export KEY=VALUE`
  * Reject keys with spaces
  * Strip surrounding quotes from value
  * Accept comments after values
* Support `#[config(rename = "blah")]` on fields of derived `ConfigParser`s

# 0.3.0

* Add `ConfigSource::env_file(filename, mandatory)` for loading `.env` files
* Display empty values as `""` in the config report
* Obfuscate sensitive values in the config report (by default, any keys containing `SECRET` or `PASSWORD`)
* Add `ConfigContext.add_secret_key` and `ConfigContext.add_secret_keyword`
* Support `#[config(secret)]` in `ConfigReader` derivation

# 0.2.0

* Optional fields now display "None" as their default in the report
* Add `std::error::Error` impls for `ReadErrors` and `ErrorMsg`
* Add `std::fmt::Display` impl for `ErrorMsg`
* Remove `regex` dependency
* Add a `ConfigParser` for `std::path::PathBuf`
* Add a `ConfigParser` for `IpAddr`, `Ipv4Addr`, `Ipv6Addr`, `SocketAddr`

# 0.1.0

Initial release
