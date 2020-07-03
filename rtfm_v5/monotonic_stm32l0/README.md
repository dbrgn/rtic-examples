# STM32L0 Monotonic

In this example we show the use of a custom `rtic::Monotonic` implementation
which uses two linked 16 bit timers of the `STM32L0` MCU to form a single 32
bit monotonic timer.

## Flashing

First, adjust your MCU in the `features` list of the `stm32l0xx-hal` in
`Cargo.toml`:

    stm32l0xx-hal = { version = "...", features = ["rt", "mcu-STM32L071KBTx"] }

Install cargo-embed:

    cargo install cargo-embed

Then:

    cargo embed --release --chip <chip>

Example:

    cargo embed --release --chip stm32l071kbtx
