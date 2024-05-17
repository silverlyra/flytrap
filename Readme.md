flytrap
=======

[![Crates.io](https://img.shields.io/crates/v/flytrap?label=crate&labelColor=%23fdc452&color=gray)][crate]
[![CI](https://img.shields.io/github/actions/workflow/status/silverlyra/flytrap/ci.yml?label=%20&logo=github)][build]
[![docs.rs](https://img.shields.io/docsrs/flytrap)][docs]
[![MIT license](https://img.shields.io/crates/l/flytrap?color=3ae)][license]

Flytrap is a Rust [crate][] for reading the [Fly.io][] runtime [environment][].

[crate]: https://lib.rs/crates/flytrap
[Fly.io]: https://fly.io/
[environment]: https://fly.io/docs/reference/runtime-environment/
[build]: https://github.com/silverlyra/flytrap/actions/workflows/ci.yml?query=branch%3Amain
[docs]: https://docs.rs/flytrap
[license]: ./LICENSE

- Read Fly.io [environment variables][env-vars] like `$FLY_PUBLIC_IP` into a `struct`
- Query Fly.io [internal DNS][dns] addresses like `top3.nearest.of.<app>.internal`
- Query the Fly.io [machines API][]
- Parse Fly.io [request headers][] like [`Fly-Client-IP`][] (into an [`IpAddr`][ip])
- Turn Fly.io [region][regions] codes like `ord` into names like ”Chicago” and lat/long coordinates

[env-vars]: https://fly.io/docs/reference/runtime-environment/#environment-variables
[dns]: https://fly.io/docs/reference/private-networking/#fly-internal-addresses
[machines API]: https://fly.io/docs/machines/api/
[request headers]: https://fly.io/docs/reference/runtime-environment/#request-headers
[`Fly-Client-IP`]: https://docs.rs/flytrap/latest/flytrap/http/struct.FlyClientIp.html
[ip]: https://doc.rust-lang.org/std/net/enum.IpAddr.html
[regions]: https://fly.io/docs/reference/regions/

A [demo app][] is available at [**flytrap.fly.dev**](https://flytrap.fly.dev) which shows this crate’s capabilities.

[demo app]: https://github.com/silverlyra/flytrap/tree/main/demo

## Usage

Flytrap can be added to your project with cargo:

```sh
cargo add flytrap
```

Most of the crate’s [features](#features) are enabled by default, but at your
option, you can enable only what you need by setting `default-features = false`.

```sh
cargo add flytrap --no-default-features --features 'dns environment http serde'
```

### Placement

The [`Placement`][placement] type gives access to Fly.io runtime
[environment variables][env-vars] like `$FLY_PUBLIC_IP` and `$FLY_REGION`.

```rust
use flytrap::{Placement, Machine}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = Placement::current()?;

    println!("Fly.io app: {}", runtime.app);
    println!("    region: {}", runtime.location);

    if let Some(Machine{ id, memory: Some(memory), image: Some(image), .. }) = runtime.machine {
        println!("   machine: {id} ({memory} MB) running {image}");
    }

    if let Some(public_ip) = runtime.public_ip {
        println!(" public IP: {}", public_ip);
    }
    println!("private IP: {}", runtime.private_ip);

    Ok(())
}
```

[placement]: https://docs.rs/flytrap/latest/flytrap/struct.Placement.html

#### Regions

Flytrap models Fly.io [regions][] as an `enum`:

```rust
use flytrap::{City, Placement, Region};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = Placement::current()?;
    let region = runtime.region().unwrap_or(Region::Guadalajara);

    show(region);
    Ok(())
}

fn show(region: Region) {
    let City { name, country, geo } = region.city;
    println!("Running in {name} ({country}) @ {}, {}", geo.x(), geo.y());
}
```

Regions implement [`Ord`][ord], and sort geographically:

```rust
#[test]
fn longitude_latitude() {
    use flytrap::Region::*;

    let mut regions = [Bucharest, Chicago, HongKong, Johannesburg,
                       LosAngeles, Madrid, Santiago, Tokyo];
    regions.sort();

    assert_eq!(regions, [LosAngeles, Chicago, Santiago, Madrid,
                         Bucharest, Johannesburg, HongKong, Tokyo]);
}
```

[ord]: https://doc.rust-lang.org/std/cmp/trait.Ord.html

If Flytrap receives a region code it doesn’t recognize (i.e., a region which
didn’t exist when your version of Flytrap was built), the raw code like `ord`
will be stored as a [`RegionCode`][region-code].

(If you build Flytrap without the `regions` [feature](#features),
`flytrap::Region` simply becomes an alias for `String`.)

[region]: https://docs.rs/flytrap/latest/flytrap/enum.Region.html
[region-code]: https://docs.rs/flytrap/latest/flytrap/struct.RegionCode.html

### HTTP headers

The [`http`][http] module contains typed [`Header`][headers] implementations of
the HTTP [request headers][] added by Fly.io edge proxies, like
[`Fly-Client-IP`][client-ip].

[http]: https://docs.rs/flytrap/latest/flytrap/http/index.html
[client-ip]: https://docs.rs/flytrap/latest/flytrap/http/struct.FlyClientIp.html

```rust
use axum::{response::Html, TypedHeader};
use flytrap::http::{FlyClientIp, FlyRegion};

async fn ip(
    TypedHeader(ip): TypedHeader<FlyClientIp>,
    TypedHeader(edge): TypedHeader<FlyRegion>,
) -> Html<String> {
    Html(format!("Your IP: <code>{ip}</code> (via {edge})"))
}
```

### DNS queries

Create a [`Resolver`][resolver] in order to query the Fly.io [`.internal` DNS zone][dns].

```rust
use flytrap::Resolver;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let resolver = Resolver::new()?;

    // Discover all instances of the currently-running app
    let peers = resolver.current()?.peers().await?;

    for peer in peers {
        println!("peer {} in {} @ {}", peer.id, peer.location, peer.private_ip);
    }
}
```

[resolver]: https://docs.rs/flytrap/latest/flytrap/struct.Resolver.html

### Machines API requests

Create an [`api::Client`][API client] to send requests to the [machines API][].

> [!NOTE]  
> The `api` module is not built by default; the `api` [feature][Cargo features] must be enabled first.

```rust
use std::env;
use flytrap::api::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = env::var("FLY_API_TOKEN")?;
    let client = Client::new(token);

    // Discover other instances of the currently-running app
    let peers = client.peers().await?;

    for peer in peers {
        println!("peer {} in {} is {:?}", peer.name, peer.location, peer.state);
    }

    Ok(())
}
```

[API client]: https://docs.rs/flytrap/latest/flytrap/api/struct.Client.html

## Features

Flytrap’s compilation can be controlled through a number of [Cargo features][].

[Cargo features]: https://doc.rust-lang.org/cargo/reference/features.html

- **`api`**: Enable the [`api::Client`][API client] for the Fly.io [machines API][]
- **`dns`**: Enable [`Resolver`][resolver] for querying Fly.io [internal DNS][dns], via [`hickory-dns`][hickory] ⭐
- **`detect`**: Enable automatic [`Resolver`][resolver] setup for Wireguard VPN clients, via [`if-addrs`][if-addrs] ⭐️
- **`environment`**: Enable code which reads Fly.io environment variables like `$FLY_PUBLIC_IP` ⭐️
- **`http`**: Enable types for HTTP [`headers`][headers] like [`Fly-Client-IP`][client-ip] ⭐️
- **`nightly`**: Enable code which is only accepted by nightly Rust toolchains
- **`regions`**: Enable the [`Region`][region] type and [`RegionDetails`][region-details] structures ⭐️
- **`serde`**: Enable [Serde][serde] `#[derive(Deserialize, Serialize)]` on this crate’s types
- **`system-resolver`**: Enable the [`Resolver::system()`][system-resolver] constructor, which reads `/etc/resolv.conf`

_(Features marked with a ⭐️ are enabled by default.)_

[headers]: https://docs.rs/headers/latest/headers/trait.Header.html
[hickory]: https://lib.rs/crates/hickory-resolver
[if-addrs]: https://lib.rs/crates/if-addrs
[region-details]: https://docs.rs/flytrap/latest/flytrap/struct.RegionDetails.html
[serde]: https://serde.rs/
[system-resolver]: https://docs.rs/flytrap/latest/flytrap/struct.Resolver.html#method.system
