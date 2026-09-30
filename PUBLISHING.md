1. Check versions in `Cargo.toml`
1. Check changelog is up-to-date
1. Publish

    ```sh
    cargo login

    cd clear-config-derive
    cargo clean
    cargo publish

    cd ..
    cargo clean
    cargo publish
    ```

1. Tag git

    ```sh
    git tag v$VERSION
    git push --tags
    ```

1. Create a Github release
