# Monoio Gateway

A high performance gateway based on [Monoio](http://github.com/bytedance/monoio).

## Installation

```shell
# clone this repo
git clone https://github.com/monoio-rs/monoio-gateway.git
# change work dir to executable crate
cd monoio-gateway/monoio-gateway
# install gateway to system wide
cargo install --path .
```

## Basic Usage

```shell
monoio-gateway --config path/to/config.json
```

## Configuration

The configuration of `monoio-gateway` is formed by a `json` file. 

### Configuration Option

#### Base

| field       | type      | description                     | required |
| ----------- | --------- | ------------------------------- | -------- |
| server_name | String    | server domain                   | true     |
| listen_port | [u16]     | port to bind, usually [80, 443] | true     |
| rules       | [Rules]   | proxy pass rules                | true     |
| tls         | TlsConfig | configuration for tls or acme   | false    |

#### Rules

| field      | type   | description                   | required |
| ---------- | ------ | ----------------------------- | -------- |
| path       | String | request path started with '/' | true     |
| proxy_pass | String | endpoint url                  | true     |

#### TlsConfig

| field       | type   | description                                                           | required |
| ----------- | ------ | --------------------------------------------------------------------- | -------- |
| mail        | String | email used to request SSL certificate(acme)                           | false    |
| chain       | String | pem file chained with root ca and server cert                         | false    |
| private_key | String | `pkcs8` encoded private key(start with `-----BEGIN PRIVATE KEY-----`) | false    |

Note: If defined `TlsConfig`, which means the server can also be served as `https`. Users should ensure one of the following parameters exist in `TlsConfig`, or `monoio-gateway` will fail to start:

- `mail`
  - if defined `mail`, the gateway will automatically request acme service (Let's Encrypt) to get a free certificate, download and deploy to runtime if there's no valid certificate. Users should ensure the corresponding dns record is pointed to current server.
- `chain`, `private_key`
  - the gateway will use certificates provided in config file, disable acme service for this `server_name`.  `mail` will be ignored and nullable.

### Example
an example configuration is shown below.

```json
{
  "configs": [
    {
      "server_name": "gateway.monoio.rs",
      "listen_port": [80, 443],
      "rules": [
        {
          "path": "/",
          "proxy_pass": {
            "uri": "https://www.google.com"
          }
        },
        {
          "path": "/apple_captive",
          "proxy_pass": {
            "uri": "http://captive.apple.com"
          }
        }
      ],
      "tls": {
        "mail": "me@monoio.rs"
      }
    }
  ]
}
```

