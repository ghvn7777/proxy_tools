# Changelog

All notable changes to this project will be documented in this file. See [conventional commits](https://www.conventionalcommits.org/) for commit guidelines.

---
## [0.5.2-support-quic](https://github.com/ghvn7777/proxy_tools/compare/v0.5.1..v0.5.2-support-quic) - 2024-07-22

### Bug Fixes

- client only connect local server bug - ([4644bcd](https://github.com/ghvn7777/proxy_tools/commit/4644bcdc70754145366560327a0e839db71774bc)) - Kaka

### Features

- add quic example - ([9017c70](https://github.com/ghvn7777/proxy_tools/commit/9017c70b2cbc04e031edb4eadd03afee84e54251)) - Kaka
- support quic - ([b80a407](https://github.com/ghvn7777/proxy_tools/commit/b80a407bbbda58bc54331c8165e0b51f0b16bfc5)) - Kaka

### Miscellaneous Chores

- changelog - ([f09194f](https://github.com/ghvn7777/proxy_tools/commit/f09194fb70c5e8e954e8ee7767575217b29c7f78)) - Kaka
- readme - ([b3e40b1](https://github.com/ghvn7777/proxy_tools/commit/b3e40b1ef8ed4437c30ea87b7f402a9ba653b957)) - Kaka
- readme use english - ([cf8663b](https://github.com/ghvn7777/proxy_tools/commit/cf8663becdd50a5a6388fbe4a612b23efc6a20c9)) - Kaka
- TextCrypt trait -> DataCrypt trait - ([86affce](https://github.com/ghvn7777/proxy_tools/commit/86affcee97e874ecc5bcf1f216deaffbe1310ecf)) - Kaka
- log level - ([9f53b12](https://github.com/ghvn7777/proxy_tools/commit/9f53b12bfbc7a2491cf4c59981ead4b9761e0305)) - Kaka
- rename some variable - ([d4767b3](https://github.com/ghvn7777/proxy_tools/commit/d4767b3dbba3629c4f3da67e63a75c82f32803fa)) - Kaka
- version - ([9ed53b4](https://github.com/ghvn7777/proxy_tools/commit/9ed53b4ce98f8fd54e1b092279fb4a582fdb9eb2)) - Kaka

### Refactoring

- add ReadStream / WriteStream / Proceesor / StreamSplit trait and refactor code - ([ed8f742](https://github.com/ghvn7777/proxy_tools/commit/ed8f742f89a07e214aecacba2c17603b84afc5f7)) - Kaka
- client tunnel code - ([0b0e9c3](https://github.com/ghvn7777/proxy_tools/commit/0b0e9c3849cc74f5cc50ba9eba9cd1a851cb9120)) - Kaka
- quic tunnel and change Crypt trait - ([48da56b](https://github.com/ghvn7777/proxy_tools/commit/48da56bc1be2a371da5fd797014e1010cc40525a)) - Kaka

---
## [0.5.1](https://github.com/ghvn7777/proxy_tools/compare/v0.5.0..v0.5.1) - 2024-07-20

### Features

- support chacha20 encryption - ([0dec96c](https://github.com/ghvn7777/proxy_tools/commit/0dec96cf80dce094d1612f298c69dff32dea3600)) - Kaka

### Miscellaneous Chores

- changelog - ([5d96ff8](https://github.com/ghvn7777/proxy_tools/commit/5d96ff898d2182f39c44b427e45cf195df588726)) - Kaka

---
## [0.5.0](https://github.com/ghvn7777/proxy_tools/compare/v0.4.0..v0.5.0) - 2024-07-20

### Features

- add docs - ([933df8e](https://github.com/ghvn7777/proxy_tools/commit/933df8e289ac04b2206c87653661ac6a69821172)) - Kaka
- add udp interface but not send - ([1ef4be9](https://github.com/ghvn7777/proxy_tools/commit/1ef4be962dcedb0e74f66c5bd30253f3aa5c636e)) - Kaka
- support udp but not test - ([9adbfce](https://github.com/ghvn7777/proxy_tools/commit/9adbfce03f14493cc3791fef7495465d55f8c7f5)) - Kaka
- udp read port timeout close connection - ([6f87ffc](https://github.com/ghvn7777/proxy_tools/commit/6f87ffc6ce4088dc95a7c4fedc9c9d0c6c5b91be)) - Kaka
- udp support complete - ([63be47b](https://github.com/ghvn7777/proxy_tools/commit/63be47b93e668368d1d2550d8e5a4456b5a6f8ec)) - Kaka

### Miscellaneous Chores

- change log - ([7a77325](https://github.com/ghvn7777/proxy_tools/commit/7a773258d5c122520597525b636bc0dd2fea96b9)) - Kaka
- readme - ([c8ece15](https://github.com/ghvn7777/proxy_tools/commit/c8ece155287e74743dd8fd8477cf8cde3c45811b)) - Kaka
- structure excalidraw - ([9d36ea6](https://github.com/ghvn7777/proxy_tools/commit/9d36ea6f65460c5fbcd735e4ad4f6906c36dccce)) - Kaka
- remove extra errors - ([4cf36ca](https://github.com/ghvn7777/proxy_tools/commit/4cf36ca3d702529991265f331529a28eb591ae77)) - Kaka
- write tunnel error log - ([9a2c2e6](https://github.com/ghvn7777/proxy_tools/commit/9a2c2e6bd4dcedfd683d8d749b58fef642a6416d)) - Kaka
- test udp success, but not stable - ([84ed3d1](https://github.com/ghvn7777/proxy_tools/commit/84ed3d1574a7368e6da51fe1917c8a399cc2e18e)) - Kaka
- readme - ([2ac159d](https://github.com/ghvn7777/proxy_tools/commit/2ac159d50f94bc3d26177ee5af1bce5e2d1e94f1)) - Kaka
- update version - ([94a42b1](https://github.com/ghvn7777/proxy_tools/commit/94a42b1766846f02477b97924d51e8f377b058cb)) - Kaka

---
## [0.4.0](https://github.com/ghvn7777/proxy_tools/compare/v0.3.2..v0.4.0) - 2024-07-18

### Bug Fixes

- socks return correct bind addr - ([d469649](https://github.com/ghvn7777/proxy_tools/commit/d469649f807aa40fadd4a8f997fa77de3a86242b)) - Kaka

### Features

- use self encode/decode algorithm - ([bd68c93](https://github.com/ghvn7777/proxy_tools/commit/bd68c93f5cd67f5dc346551ddbde1eb2857cd94b)) - Kaka
- work on linux - ([1350ce2](https://github.com/ghvn7777/proxy_tools/commit/1350ce2b36d44fe7714965ff74b2ba974e654d37)) - Kaka

### Miscellaneous Chores

- change log - ([648f249](https://github.com/ghvn7777/proxy_tools/commit/648f2494c4466aaa5e9d6a734a663762d6300f0a)) - Kaka
- fine tune param - ([169a83a](https://github.com/ghvn7777/proxy_tools/commit/169a83a6dcbc2512f79cde1af002660889b5bd04)) - Kaka
- log level - ([5c553a8](https://github.com/ghvn7777/proxy_tools/commit/5c553a8259609eaa367425e394e55492a92f1de9)) - Kaka
- use join!(..) instead tokio::select - ([cfd131d](https://github.com/ghvn7777/proxy_tools/commit/cfd131d61a4ec5314d75455843e6b892dde79c7e)) - Kaka
- add log debug - ([95cefad](https://github.com/ghvn7777/proxy_tools/commit/95cefade8f71a810fae582d96f1d094ceef610ae)) - Kaka
- remove network mod - ([6616004](https://github.com/ghvn7777/proxy_tools/commit/6616004bbeb7cdf1f064eb3690a8f3b3857b0b93)) - Kaka

### Other

- use box send data - ([33c287e](https://github.com/ghvn7777/proxy_tools/commit/33c287ea0b5ea218ca10e435ff73fd5df03f3fe5)) - Kaka

---
## [0.3.2] - 2024-07-17

### Bug Fixes

- server remote close port id bug - ([8ae30f8](https://github.com/ghvn7777/proxy_tools/commit/8ae30f8e45855f7261ec262ff0986ea15393ccde)) - Kaka

### Features

- socks5 auth - ([04aa052](https://github.com/ghvn7777/proxy_tools/commit/04aa052d3991a9ec68345d6c9852b358da05fbf1)) - Kaka
- support yamux - ([8003893](https://github.com/ghvn7777/proxy_tools/commit/80038939e137ea2506428b3598ffbd2e8d37f384)) - Kaka
- tcp proxy success (too many bug) - ([12a5d18](https://github.com/ghvn7777/proxy_tools/commit/12a5d181f3c4948c7f706e34a2c554d690f30cc7)) - Kaka
- split tcp stream to readstream and writestream - ([d516316](https://github.com/ghvn7777/proxy_tools/commit/d516316e7df6ab1443778b8eb9a16f769e78c9d5)) - Kaka
- send <client to socks tx> to client writer stream - ([8621fc1](https://github.com/ghvn7777/proxy_tools/commit/8621fc1e0066ca14cba0f186eaf0655e17aa5ab7)) - Kaka
- server tcp connect - ([22d3e72](https://github.com/ghvn7777/proxy_tools/commit/22d3e7244b0d08f151d0f0f14b76bc49b9c807d1)) - Kaka
- comple fist version - ([c5d8679](https://github.com/ghvn7777/proxy_tools/commit/c5d867988b085f6c15ada4b70983b692601af2b9)) - Kaka
- client retry client to server, server close port and add heart beat detect - ([e279223](https://github.com/ghvn7777/proxy_tools/commit/e27922309bbb13f7de5e01307b1e9a0e95666fd5)) - Kaka
- socks port auto close - ([834c110](https://github.com/ghvn7777/proxy_tools/commit/834c110fd9e4c30ecde9beeec79f5600b56b2a3b)) - Kaka
- socks communicate use SocksMsg - ([aca7173](https://github.com/ghvn7777/proxy_tools/commit/aca717387b86045b5255fef8d07be8f25e7a75e7)) - Kaka

### Miscellaneous Chores

- init project - ([392175d](https://github.com/ghvn7777/proxy_tools/commit/392175d7612ef38c16427e952c43c1dd92b5bfb9)) - Kaka
- add client tunne mod - ([baf30c2](https://github.com/ghvn7777/proxy_tools/commit/baf30c2ef58f160719531caf60320bd07771bd95)) - Kaka
- restructor service streams mod - ([368f1a8](https://github.com/ghvn7777/proxy_tools/commit/368f1a8ede0dc0fa75ed0ef36d918794cec41cb6)) - Kaka
- tunnel add <client tunnel> sub module - ([7e25826](https://github.com/ghvn7777/proxy_tools/commit/7e25826062838bcbfbc4d0409fe647e26aeda349)) - Kaka
- use match pattern instead some expect - ([f8277c0](https://github.com/ghvn7777/proxy_tools/commit/f8277c0a31e4770f398f18e736be911397245440)) - Kaka
- fine tune default parameter - ([7b688a5](https://github.com/ghvn7777/proxy_tools/commit/7b688a5f3cc20115e9400d1e80f6bf4c8b41de59)) - Kaka
- justfile - ([0c16747](https://github.com/ghvn7777/proxy_tools/commit/0c16747794a86fcdd94ba8ffa59c4542aacce85d)) - Kaka
- changelog - ([d13961c](https://github.com/ghvn7777/proxy_tools/commit/d13961c20c21335dcef9efb2d6d35fdf40659f57)) - Kaka

<!-- generated by git-cliff -->
