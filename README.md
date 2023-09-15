# Mercury

The guide of souls to the underworld [^1].

Mercury is a secure, general-purpose notification system supporting arbitrary messaging to Slack as well as alerting in response to Heroku webhooks.

## API

Getting started with Mercury as a consumer.

### Direct Messaging

Mercury is designed for easy plug and play in CI, which can typically act as a source of truth.

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

Additionally Mercury supports monitoring Heroku webhooks for dyno crashes, rollbacks, and environment variable changes. The webhook must be created manually with the URL target pointed at Mercury.

```console
$ heroku webhooks:add -l notify -i dyno,api:release -a <HEROKU_APP> -s <HEROKU_SECRET> -u https://mercury.proxy.unsplash.com/api/v1/heroku/hook?platform=slack&channel=playground
```

Webhooks will only successfully authenticate if the secret is the same on both sides. Mercury looks for the secret on startup at `$HEROKU_SECRET`. This feature, thus also this environment variable, is optional.

## Contributing

Mercury is developed with Unsplash's particular needs in mind, however contributions are welcome!

Mercury is written in Rust. The Nix shell provides the necessary tooling to build with [Cargo](https://doc.rust-lang.org/stable/cargo/), which is recommended for development:

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

Unsplash's instance of Mercury is hosted on AWS. Infrastructure is defined in [api-ops](https://github.com/unsplash/api-ops).

We leverage Nix's reproducible builds to build a hermetic Docker image which can principally be deployed anywhere:

```console
$ nix build ".#dockerImage" && ./result | podman load
$ podman run -p 80 mercury
```

The server runs on `$PORT`, defaulting to port 80.

[^1]: https://en.wikipedia.org/wiki/Mercury_(mythology)
