# virtual_gw

## Virtual Gateway

Literally, a virtual gateway is an application that acts as a gateway to your virtual private network.

Technically, it requires **mtls** as a method of authentication.
Any authenticated endpoint can access the **all** resources specified in the `client routes` .

## Platform

Linux Only.

## Build

```Bash
cargo build --release
```

## Run

```Bash
./target/release/virtual_gw -c [config.json]
```

## Config Examples

* As a server

```Json
{
    "server": true,
    "listen_ip": "0.0.0.0:443",
    "server_ip": "173.75.2.1",
    "client_ip": "172.25.20.0/24",
    "client_routes": [
        "172.25.20.0/24",
        "175.55.5.0/24"
    ],
    "key_file": "/path/to/your/server/key",
    "cert_file": "/path/to/your/server/cert",
    "ca_file": "/path/to/your/ca/cert"
}
```

* As a client

```Json
{
    "server": false,
    "server_ip": "127.0.0.1:443",
    "key_file": "/path/to/your/client/key",
    "cert_file": "/path/to/your/client/cert",
    "ca_file": "/path/to/your/ca/cert"
}
```
