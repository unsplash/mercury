# Mercury

The guide of souls to the underworld [^1]. An alternative notification system to [Otto](https://github.com/unsplash/otto).

## API

Getting started with Mercury as a consumer.

### Direct Messaging

Mercury is designed for easy plug and play in CI, which typically "knows" when something is deployed, broken, etc.

```sh
curl https://mercury.proxy.unsplash.com/api/v1/slack -X POST \
    --oauth2-bearer <SLACK_TOKEN> \
    -d channel=playground \
    -d title=Mercury \
    -d desc="Running the example" \
    --data-urlencode link="https://github.com/unsplash/mercury?beware=url&encoding=!"
```

The token will be validated against the `$SLACK_TOKEN` found on startup.

### Heroku Webhooks

Additionally Mercury supports monitoring Heroku webhooks for rollbacks and environment variable changes. The webhook must be created manually with the URL target pointed at Mercury.

```console
$ heroku webhooks:add -l notify -i api:release -a <HEROKU_APP> -s <HEROKU_SECRET> -u https://mercury.proxy.unsplash.com/api/v1/heroku/hook?platform=slack&channel=playground
```

Webhooks will only successfully authenticate if the secret is the same on both sides. Mercury looks for the secret on startup at `$HEROKU_SECRET`. This feature, thus also this environment variable, is optional.

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

### Webhooks

To develop against Heroku's webhooks Heroku will need some way of reaching your local machine. A Nix shell named `webhooks` is included for this purpose, containing the Heroku CLI and [ngrok](https://ngrok.com), the generated URL from which can be passed along to Heroku.

## Hosting

Mercury is hosted on AWS. Infrastructure is defined in [api-ops](https://github.com/unsplash/api-ops).

We leverage Nix's reproducible builds to build a hermetic Docker image which can principally be deployed anywhere:

```console
$ nix build ".#dockerImage" && ./result | podman load
$ podman run -p 80 mercury
```

The server runs on `$PORT`, defaulting to port 80.

[^1]: https://en.wikipedia.org/wiki/Mercury_(mythology)
