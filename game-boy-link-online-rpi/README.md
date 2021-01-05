# game-boy-link-online-rpi

## Setup

### Rust Target

* Raspberry Pi 0/1/2

  ```sh
  rustup target add arm-unknown-linux-gnueabihf
  ```

* Raspberry Pi 2

  ```sh
  rustup target add armv7-unknown-linux-gnueabihf
  ```

* Raspberry Pi 3/4

  ```sh
  rustup target add aarch64-unknown-linux-gnu
  ```

### Linker and Build Utilities

* Raspberry Pi 0/1/2

  ```sh
  apt install build-essential gcc-arm-linux-gnueabihf
  ```

* Raspberry Pi 3/4

  ```sh
  apt install build-essential gcc-aarch64-linux-gnu
  ```

## Build

* Raspberry Pi 0/1/2

  ```sh
  cargo build --target arm-unknown-linux-gnueabihf
  ```

* Raspberry Pi 2

  ```sh
  cargo build --target armv7-unknown-linux-gnueabihf
  ```

* Raspberry Pi 3/4

  ```sh
  cargo build --target aarch64-unknown-linux-gnu
  ```

### Building with `cross`

It is possible to build using `cross` as well, but there are some problems with
it. Since this crate relies on other crates in the workspace using `path`s,
`cross` must be used in the root of this repo. This is because `cross` utilizes
Docker / containers. Building this package can be done with the `--package`
flag.

Also, you may encounter an ICE error like the one reported here:
<https://github.com/rust-embedded/cross/issues/407>. To get around it, you can
disable `cargo` incremental builds like below.

```sh
export CARGO_INCREMENTAL=0;
cross build --package game-boy-link-online-rpi --target aarch64-unknown-linux-gnu;
```

## Running

### Environmental Variables

* `SCK_PIN`: Physical GPIO pin number used for SCK
* `SIN_PIN`: Physical GPIO pin number used for SIN
* `SOUT_PIN`: Physical GPIO pin number used for SOUT
* `SD_PIN`: Physical GPIO pin number used for SD
* `MODE`: Only supports `"printer"`

### Example

```sh
env MODE=printer SCK_PIN=3 SIN_PIN=5 SOUT_PIN=7 SD_PIN=8 ./game-boy-link-online-rpi
```
