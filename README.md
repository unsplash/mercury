# Mercury

The guide of souls to the underworld [^1]. An alternative notification system to [Otto](https://github.com/unsplash/otto).

## Usage

Mercury is designed for easy plug and play in CI, which typically "knows" when something is deployed, broken, etc.

Run the following in a CI pipeline shell or script:

```sh
curl <host>/api/v1/slack -X POST \
  -d channel=playground \
  -d title=Mercury \
  -d desc="Running the example"
  -d link="https://github.com/unsplash/mercury"
```

### Security

Like Otto before it, Mercury is currently unauthenticated. Whilst this is the case, our Slack instance's peace is protected essentially only by obscurity. It is therefore recommended to avoid using Mercury in public repos.

## Contributing

Mercury is written in Rust. This offers a few benefits including:

- Outstanding tooling.
- A strong, expressive type system inspired by functional languages.
- More approachable to contributors outside of the Web team than something like Haskell.
- Acts as a test bed for Rust at Unsplash; timely with Fastly C@E in mind.

### Building

The Nix shell provides the necessary tooling to build with [Cargo](https://doc.rust-lang.org/stable/cargo/), which is recommended for development.

It's also possible to build directly with Nix. This can be leveraged to build a hermetic Docker image. Example:

```console
$ nix build ".#dockerImage" && ./result | podman load
$ podman run --rm -p 80 -e SLACK_TOKEN mercury
```

[^1]: https://en.wikipedia.org/wiki/Mercury_(mythology)
