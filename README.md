# Tmp117

A no_std platform agnostic driver in rust  for the [TMP117](https://www.ti.com/product/TMP117) temperature sensor
using the [embedded-hal](https://github.com/rust-embedded/embedded-hal) and the [device-register](https://github.com/xgroleau/device-register) library.
A Sync and Async API is available, see the examples folder for more complete usage
The high level api always makes sure the device is in shutdownmode to save battery.
The low level api is always available if needed.

### Usage

```rust
// Pass the address of the tmp device
let tmp = Tmp117::<0x49, _, _, _>::new(spi);
let delay = Delay;
tmp.reset(delay).unwrap();

// Transition to continuous mode and shutdown after the closure
let mut tmp_cont = tmp.continuous(Default::default(), |t| {
// Get the value continuously in continuous mode
    for _ in 0..10 {
        /// Can transparently return error ehere
        let temp = tmp.wait_temp()?;
        info!("Temperature {}", temp);
    };
    Ok(())
}).unwrap();

```

### MSRV
Currently `1.75` and up is supported, but some previous nightly version may work

### License
Licensed under either of
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)

- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution
Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.


License: MIT OR Apache-2.0
