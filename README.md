# tmp117

A no_std platform agnostic driver in rust  for the [TMP117](https://www.ti.com/product/TMP117) temperature sensor
using the [embedded-hal](https://github.com/rust-embedded/embedded-hal) and the [device-register](https://github.com/xgroleau/device-register) library.
A Sync and Async API is available, see the examples folder for more complete usage
The library makes usage of the [typestate](https://docs.rust-embedded.org/book/static-guarantees/typestate-programming.html) pattern.
The low level api is always available if the typestate is too constraining

### Usage

```rust
// Pass the address of the tmp device
let tmp = Tmp117::<0x49, _, _, _>::new(spi);

// Transition to oneshot mode
let tmp_one = tmp.to_oneshot(Average::NoAverage).unwrap();
// Read and transition to shutdown since it's a one shot
let (temperature, tmp_shut) = tmp_one.wait_temp().unwrap();

// Transition to continuous mode
let mut tmp_cont = tmp_shut.to_continuous(Default::default()).unwrap();

// Get the value continuously in continuous mode
for _ in 0..10 {
    let temp = tmp_cont.wait_temp().unwrap();
    info!("Temperature {}", temp);
};

// Shutdown the device
let _  = tmp_cont.to_shutdown().unwrap();
```

### MSRV
Currently only rust `nightly-2022-09-29` and more is garanted to work with the library, but some previous version may work

### License
Licensed under either of
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)

- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution
Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

