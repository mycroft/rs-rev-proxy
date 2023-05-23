# rs-rev-proxy

A small reverse proxy to serve S3 object, with cache and redirect features.

It is not recommended to use this, as I have no idea how I made it work.

The thing listens to web queries, will check for possible redirection rewrites, then check if target file in cache, and then will reverse-proxify the query and store the file in cache while returning it, all using streaming features.

It was built with Rust and Axum.

## Configuration

Set a `.env` file as following:

```sh
# AWS_ENDPOINT_URL=https://your-minio-server.mkz.me
AWS_SECRET_ACCESS_KEY=blah
AWS_ACCESS_KEY_ID=bluh
AWS_DEFAULT_REGION=us-east-1
RUST_LOG=debug,hyper=info,rusoto_core=info
```

## Compilation and launch

```sh
$ cargo build
...

$ cargo run
[2023-05-23T15:06:08Z INFO  rs_rev_proxy] Server listening on 0.0.0.0:8080
```

With in another term:

```sh
$ mkdir -p /tmp/cache
$ time curl -sq http://0:8080/blah2 | sha256sum
d2bafdef03246a64e3c58049c5ae188d8be9d74c6b23f05045637b07ad7167df  -

________________________________________________________
Executed in    1.83 secs      fish           external
   usr time  238.18 millis    1.81 millis  236.37 millis
   sys time  129.78 millis    5.90 millis  123.87 millis

$ sha256sum /tmp/cache/blah.pdf
d2bafdef03246a64e3c58049c5ae188d8be9d74c6b23f05045637b07ad7167df  /tmp/cache/blah.pdf

$ time curl -sq http://0:8080/blah2 | sha256sum
d2bafdef03246a64e3c58049c5ae188d8be9d74c6b23f05045637b07ad7167df  -

________________________________________________________
Executed in  104.06 millis    fish           external
   usr time   48.94 millis    0.00 millis   48.94 millis
   sys time   25.75 millis    2.50 millis   23.25 millis
```