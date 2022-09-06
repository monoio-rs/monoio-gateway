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

a simple gateway configuration is shown below.

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

### Configuration Option

To be done.