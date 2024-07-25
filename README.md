# Proxy Tools

Socks5 proxy tool, support Tcp / Udp proxy with auth or auth-free, base on quic protocol

Socks Request -- [Local SocksServer -- LocalClient] -- Server -- Remote


## Dependencies
```
sudo apt install protobuf-compiler
```

## Build
```
cargo build --release
```

## Usage
generate chacha20 encryption file:
```
cargo run --example gen_pass
```
copy output to vpn/fixtures/chacha20.txt (no new line for end)

server:
```
cd target/release
./quic-server --crypt-file ../../vpn/fixtures/chacha20.txt  --port 9527
```

client:
```
cd target/release
./quic-client\
    --crypt-file ../../vpn/fixtures/chacha20.txt\
    --server-url 127.0.0.1:9527\
    --tunnel-cnt 10\
    --port 9020\
     no-auth
```

### s2n-quic usage
里面写死了 `pem` 的相对路径，所以一定要在项目根目录下运行
```
cargo build
./target/debug/s2n-server
./target/debug/s2n-client  --server-url 127.0.0.1:9527 no-auth
```

## Debug Test
```
sudo apt install netcat
```

### Tcp test
start proxy server on local:
```
./server --port 9527
```

start proxy client on local:
```
./client --server-url 127.0.0.1:9527 --tunnel-cnt 1 no-auth
```

start tcp server use netcat
```
nc -l 1234
```

test tcp connect proxy socks5
```
nc -X 5 -x 127.0.0.1:9020 127.0.0.1 1234
```

Sending a message from one terminal in TCP server or client, and another terminal can receive it normally.


### Udp test
Attempting to request via UDP protocol to 8.8.8.8:53.

noted: This script may fail to run in China, but it should work fine on servers located in other countries.

```python
python3 test_udp_proxy.py  --proxy 127.0.0.1 --port 9020
```

## Structure

![proxy structure](./fixtures/proxy.png)
