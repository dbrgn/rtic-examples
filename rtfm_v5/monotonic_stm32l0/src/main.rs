#![no_main]
#![no_std]

//mod monotonic_stm32l0;

//use monotonic_stm32l0::*;
use panic_rtt_target as _;
use rtic::app;
use rtt_target::{rtt_init_print, rprintln};
use stm32l0xx_hal as _;

//#[app(device = stm32l0xx_hal::pac, peripherals = true, monotonic = crate::monotonic_nrf52::Tim1)]
#[app(device = stm32l0xx_hal::pac, peripherals = true)]
const APP: () = {
    #[init]
    fn init(cx: init::Context) {
        rtt_init_print!();
        rprintln!("Init");
    }

    extern "C" {
        fn SPI1();
    }
};
