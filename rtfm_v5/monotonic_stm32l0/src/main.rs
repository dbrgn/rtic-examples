#![no_main]
#![no_std]

mod monotonic_stm32l0;

use monotonic_stm32l0::{Duration, Instant, LinkedTim2Tim3};
use panic_rtt_target as _;
use rtic::app;
use rtt_target::{rprintln, rtt_init_print};
use stm32l0xx_hal::prelude::*;
use stm32l0xx_hal::{pac, rcc::Config, timer::LinkedTimerPair};

#[app(
    device = stm32l0xx_hal::pac,
    peripherals = true,
    monotonic = crate::monotonic_stm32l0::LinkedTim2Tim3,
)]
const APP: () = {
    #[init(spawn = [foo])]
    fn init(cx: init::Context) {
        // Initialize RTT
        rtt_init_print!();
        rprintln!("Init");

        // Get peripherals
        let dp: pac::Peripherals = cx.device;

        // Clock configuration. Use HSI at 16 MHz.
        rprintln!("Set up clock (16 MHz)");
        let mut rcc = dp.RCC.freeze(Config::hsi16());

        // Set up linked 32 bit timer with TIM2/TIM3.
        let linked_timer = LinkedTimerPair::tim2_tim3(dp.TIM2, dp.TIM3, &mut rcc);

        // Use linked timer as RTIC monotonic clock.
        rprintln!("Initialize monotonic timer (TIM2/TIM3)");
        LinkedTim2Tim3::initialize(linked_timer);

        rprintln!("init(baseline = {:?})", cx.start);

        // Spawn task "foo"
        cx.spawn.foo().unwrap();

        rprintln!("Init done!");
    }

    #[task(schedule = [foo])]
    fn foo(cx: foo::Context) {
        let now = Instant::now();
        rprintln!("foo(scheduled = {:?}, now = {:?})", cx.scheduled, now);
        cx.schedule
            .foo(cx.scheduled + Duration::from_cycles(6301))  // TODO: Does not work with values >6300
            .unwrap();
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        // The default implementation of #[idle] uses WFI to go to deep sleep.
        // Unfortunately this prevents RTT from working, so since this is a
        // debug-only example, we replace the WFI with a busy loop.
        loop {}
    }

    extern "C" {
        fn SPI1();
    }
};
