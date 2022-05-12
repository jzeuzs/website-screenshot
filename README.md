# website-screenshot

[![Rust: 1.62+](https://img.shields.io/badge/rust-1.62+-93450a)](https://github.com/rust-lang/rust/milestone/93)
[![Continuous Delivery](https://github.com/devtomio/website-screenshot/actions/workflows/continuous-delivery.yml/badge.svg)](https://github.com/devtomio/website-screenshot/actions/workflows/continuous-delivery.yml)
[![Continuous Integration](https://github.com/devtomio/website-screenshot/actions/workflows/continuous-integration.yml/badge.svg)](https://github.com/devtomio/website-screenshot/actions/workflows/continuous-integration.yml)
[![Integration Tests](https://github.com/devtomio/website-screenshot/actions/workflows/integration-tests.yml/badge.svg)](https://github.com/devtomio/website-screenshot/actions/workflows/integration-tests.yml)

**📸 website screenshots as a service**

## Features

- 💫 powered by [Rust]
- 🚀 blazing fast
- 👮​​​‍‍​ built-in [ratelimiter](https://github.com/antifuchs/governor)
- 👜 built-in [storage providers](#storage-providers)
- 🛡️ built-in [authentication](#authentication)
- 🗼 configurable

## Deployment

### Prerequisites

- [Rust] nightly 1.62+ or greater
- [Redis] 6 or greater
- [Chrome] browser
- [Chromedriver] (must match with the version your [Chrome] browser)

### Environment Variables

- `PORT` - the port that the application will run (optional, defaults to `3000`)
- `REDIS_URL` - the address of your redis database (required)
- `FULLSCREEN_SCREENSHOT` - if set, it will screenshot the whole website (optional)
- `CHECK_IF_NSFW` - if set, it will check if the url is marked as NSFW (optional)
- `GOOGLE_CHROME_PATH` - the path to the chrome executable (optional)
- `CHROMEDRIVER_PATH` - the path to the chromedriver executable (optional)
- `DARK_MODE` - if set, it will take screenshots in dark mode, if the website supports it (optional)

### Railway

[![Deploy on Railway](https://railway.app/button.svg)](https://railway.app/new/template/3ZVgSw?referralCode=tomio)

#### Changing Providers

To change the provider in railway:

1. Open the repo that railway made for you.
2. Open the [Dockerfile](./Dockerfile)
3. Edit lines 39 and 44 with `fleet build --release --no-default-features --features <your_provider>`.
4. Deploy your app again.

### Pre-built Binaries

**⚠️- You can't change the provider when using the pre-built binaries.**

#### Linux

```sh
curl -fsSL https://raw.githubusercontent.com/devtomio/website-screenshot/main/install.sh
```

#### Other

See the [Releases page](https://github.com/devtomio/website-screenshot/releases) of this repository and download the appropriate binary.

### Docker

**⚠️- You can't change the provider when using the docker image.**

A dockerized version of this application is available [here](https://github.com/devtomio/website-screenshot/pkgs/container/website-screenshot).

### [Cargo](https://crates.io/crates/website-screenshot)

Installing `website-screenshot` as a crate.

```sh
cargo install website-screenshot
```

### Building from source

1. Clone this repository. e.g. `git clone https://github.com/devtomio/website-screenshot`
2. Build the binary `cargo build --release`

## Authentication

Authentication will be enabled if the `AUTH_TOKEN` variable is set.

It will check if the `Authorization` header sent by the user is equal to the `AUTH_TOKEN` that you set.

## Storage Providers

### Fs (Filesystem) Provider

_📝 - This is the default provider._

#### Build Command

```sh
cargo build --release
```

### [Cloudinary] Provider

_📝 - You must make an unsigned upload preset._

#### Environment Variables

- `CLOUDINARY_API_KEY` - your api key (required)
- `CLOUDINARY_UPLOAD_PRESET` - the name of your unsigned upload preset (required)
- `CLOUDINARY_CLOUD_NAME` - the cloud name that you set during registration (required)

#### Build Command

```sh
cargo build --release --no-default-features --features cloudinary_provider
```

### Amazon AWS [S3] Provider

_⚠️ - This is untested. If you encounter a bug please don't hesitate to open an issue._

#### Environment Variables

- `S3_BUCKET_NAME` - the name of your s3 bucket (required)
- `S3_REGION` - the region of your s3 bucket (required, only optional if using [self-hosted s3](#self-hosted-s3-storage))
- `S3_ACCESS_KEY` - your access key (required)
- `S3_SECRET_KEY` - your secret key (required)
- `S3_SECURITY_TOKEN` - your security token (optional)
- `S3_SESSION_TOKEN` - your session token (optional)

#### Self-hosted S3 Storage

To use a self-hosted S3 Storage, set the `S3_ENDPOINT_URL` environment variable to the endpoint url.

#### Build Command

```sh
cargo build --release --no-default-features --features s3_provider
```

### [Tixte] Provider

#### Environment Variables

- `TIXTE_UPLOAD_KEY` - your upload key, can be found on the integrations tab (required)
- `TIXTE_DOMAIN_CONFIG` - whether to use random domains or a specific domain (required, can only be `standard`, or `random`)
- `TIXTE_CUSTOM_DOMAIN` - the specific domain to use (only required when `TIXTE_DOMAIN_CONFIG` is set to `standard`)

#### Build Command

```sh
cargo build --release --no-default-features --features tixte_provider
```

### [Sled] Provider

#### Environment Variables

- `SLED_PATH` - the path to provide when opening the sled database (optional, defaults to `.website-screenshot`)

#### Build Command

```sh
cargo build --release --no-default-features --features sled_provider
```

## Endpoints

### **GET** `/`

Hello, world!

Example Response:

```text
Hello, world!
```

### **POST** `/screenshot`

Creates a screenshot.

#### Payload

- `url` - The url of the website. (string, required)
- `fullscreen` - If you want to take a fullscreen screenshot. (boolean, optional, overrides the `FULLSCREEN_SCREENSHOT` environment variable)
- `check_nsfw` - If you want to check if the url is marked as NSFW. (boolean, optional, overrides the `CHECK_IF_NSFW` environment variable)
- `dark_mode` - If you want to take a screenshot in dark mode if the website supports it. (boolean, optional, overrides the `DARK_MODE` environment variable)

Example Payload:

```json
{
  "url": "https://rust-lang.org"
}
```

Example Response

```json
{
  "slug": "abcdefghijk",
  "path": "/s/abcdefghijk",
  "metadata": {
    "url": "https://rust-lang.org",
    "fullscreen": false,
    "check_nsfw": false,
    "dark_mode": false
  }
}
```

### **GET** `/s/:slug`

Gets the screenshot that is corresponding to the slug.

Example URL

```text
http://localhost:3000/s/abcdefghijk
```

Example Response

![Rust Website](screenshots/abcdefghijk.png "Rust Website")

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.

## Contributors ✨

Thanks goes to these wonderful people ([emoji key](https://allcontributors.org/docs/en/emoji-key)):

<!-- ALL-CONTRIBUTORS-LIST:START - Do not remove or modify this section -->
<!-- prettier-ignore-start -->
<!-- markdownlint-disable -->
<table>
  <tr>
    <td align="center"><a href="https://tomio.codes/"><img src="https://avatars.githubusercontent.com/u/75403863?v=4?s=100" width="100px;" alt=""/><br /><sub><b>Tomio</b></sub></a><br /><a href="https://github.com/devtomio/website-screenshot/commits?author=devtomio" title="Code">💻</a> <a href="https://github.com/devtomio/website-screenshot/commits?author=devtomio" title="Documentation">📖</a> <a href="#infra-devtomio" title="Infrastructure (Hosting, Build-Tools, etc)">🚇</a> <a href="#maintenance-devtomio" title="Maintenance">🚧</a></td>
  </tr>
</table>

<!-- markdownlint-restore -->
<!-- prettier-ignore-end -->

<!-- ALL-CONTRIBUTORS-LIST:END -->

This project follows the [all-contributors](https://github.com/all-contributors/all-contributors) specification. Contributions of any kind welcome!

[Rust]: https://rust-lang.org
[Redis]: https://redis.io
[Chrome]: https://google.com/chrome
[Chromedriver]: https://chromedriver.chromium.org
[Cloudinary]: https://cloudinary.com
[S3]: https://aws.amazon.com/s3
[Tixte]: https://tixte.com
[Sled]: https://sled.rs
