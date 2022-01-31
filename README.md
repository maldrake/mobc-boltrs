# mobc-boltrs

This crate is no longer being maintained.  The mobc-boltrs crate is deprecated in favor of [mobc-bolt](https://crates.io/crates/mobc-bolt).

The remainder of the README file is preserved below, for historical purposes, but this crate should no longer be used.

# Usage

Include the following in `Cargo.toml` under the dependencies section:

```
mobc = "0.7.2"
mobc-boltrs = "0.2.2"
```

Then, in the project's source code, include something like the following:

```
let manager = BoltConnectionManager::new("localhost:7687", "localdomain", [V4_1, 0, 0, 0],
        HashMap::from_iter(vec![
            ("user_agent", "bolt-client/X.Y.Z"),
            ("scheme", "basic"),
            ("principal", "username"),
            ("credentials", "password"),
        ]),
    )
    .await?
let pool = Pool::builder().max_open(20).build(manager);
let client = pool.get().await?;
```

# Contributing

Contributions are very welcome. See the [contribution guidelines for this project](./CONTRIBUTING.md) for details.

# License

This project is available under the MIT license. See the [license file](./LICENSE.md) for the full text of the license.

This project reuses a substantial portion of the source code in [Luc Street](https://github.com/lucis-fluxum)'s [bb8-bolt](https://github.com/lucis-fluxum/bolt-rs/tree/master/bb8-bolt) crate. See the [license file](./LICENSE.md) for the MIT license statement related to the code reuse.
