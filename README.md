ogn-web-gateway
==============================================================================


[![Build Status](https://travis-ci.org/Turbo87/ogn-web-gateway.svg?branch=master)](https://travis-ci.org/Turbo87/ogn-web-gateway)

[OpenGliderNet] Web-Gateway

[OpenGliderNet]: http://wiki.glidernet.org/

This project contains a webserver that connects to the [OpenGliderNet],
saves the received records to a database for 24 hours and relays all data to
any connected WebSocket clients.

Docker Quickstart
------------------------------------------------------------------------------

With docker installed and this repo cloned, it's as easy as
`docker-compose up`. You can easily verify that it is up with:
```bash
curl http://0.0.0.0:8080/api/status
```

Installation & Usage
------------------------------------------------------------------------------

This project uses the in-memory database [Redis] as the data store for
the History API. Before attempting to install ogn-web-gateway make sure to
have a working Redis server running.

Next, you should clone this repository using [git]:

```bash
git clone https://github.com/Turbo87/ogn-web-gateway.git
```

Before continuing make sure to set the `REDIS_URL` environment variable so
that ogn-web-gateway known what Redis server it should try to connect to:

```bash
export REDIS_URL=redis://localhost
```

Finally we can use [cargo] to download all necessary dependencies, compile the
application and then run it:

```bash
cargo run --release
```

By default ogn-web-gateway does not produce any console output when running,
so don't be surprised. Once it is running you should be able to visit
<http://127.0.0.1:8080/api/status> to verify that everything runs correctly.

[Redis]: https://redis.io/
[git]: https://git-scm.com/
[cargo]: https://doc.rust-lang.org/cargo/

API Documentation
------------------------------------------------------------------------------

For the API documentation please have a look at the [`docs`](docs) folder.  

Contributing
------------------------------------------------------------------------------

See [`CONTRIBUTING.md`](CONTRIBUTING.md) for instructions on contributing
and how to use docker for a dev environment.

License
------------------------------------------------------------------------------

This project is licensed under either of

 - Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   <http://www.apache.org/licenses/LICENSE-2.0>)

 - MIT license ([LICENSE-MIT](LICENSE-MIT) or
   <http://opensource.org/licenses/MIT>)

at your option.
