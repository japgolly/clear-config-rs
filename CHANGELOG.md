# 0.3.0

* Add `ConfigSource::env_file(filename, mandatory)` for loading `.env` files
* Display empty values as `""` in the config report

# 0.2.0

* Optional fields now display "None" as their default in the report
* Add `std::error::Error` impls for `ReadErrors` and `ErrorMsg`
* Add `std::fmt::Display` impl for `ErrorMsg`
* Remove `regex` dependency
* Add a `ConfigParser` for `std::path::PathBuf`
* Add a `ConfigParser` for `IpAddr`, `Ipv4Addr`, `Ipv6Addr`, `SocketAddr`

# 0.1.0

Initial release
