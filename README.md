# Mercury

The guide of souls to the underworld [^1]. An alternative notification system to [Otto](https://github.com/unsplash/otto).

## Usage

Mercury is designed for easy plug and play in CI, which typically "knows" when something is deployed, broken, etc.

```sh
curl <HOST>/api/v1/slack -X POST \
    --oauth2-bearer <SLACK_TOKEN> \
    -d channel=playground \
    -d title=Mercury \
    -d desc="Running the example"
    -d link="https://github.com/unsplash/mercury"
```

## Contributing

Mercury is written in Rust. This offers a few benefits including:

- Outstanding first-party tooling and reproducible builds via Nix.
- A strong, expressive type system inspired by functional languages.
- More approachable to contributors outside of the Web team than something like Haskell.
- Acts as a test bed for Rust at Unsplash; timely with [Fastly C@E](https://developer.fastly.com/learning/compute/rust/) in mind.

The Nix shell provides the necessary tooling to build with [Cargo](https://doc.rust-lang.org/stable/cargo/), which is recommended for development:

```console
$ cargo run
$ cargo check
$ cargo test
$ cargo fmt
$ cargo clippy
```

## Hosting

Mercury is hosted on [Fly](https://fly.io) \*.

We can leverage Nix's reproducible builds to build a hermetic Docker image which can principally be deployed anywhere:

```console
$ nix build ".#dockerImage" && ./result | podman load
$ podman run -p 80 mercury
```

The server runs on `$PORT`, defaulting to port 80.

<sup>\* It's currently hosted on @samhh's personal Fly account, accessible at [mercury-test.fly.dev](https://mercury-test.fly.dev). This is temporary.</sup>

[^1]: https://en.wikipedia.org/wiki/Mercury_(mythology)
