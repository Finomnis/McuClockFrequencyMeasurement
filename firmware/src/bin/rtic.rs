#![no_main]
#![no_std]

use clock_frequency_measurement as _; // global logger + panicking-behavior + memory layout
use clock_frequency_measurement::hal;

#[rtic::app(device = clock_frequency_measurement::hal::stm32, peripherals = true)]
mod app {
    use super::hal;
    use hal::gpio::gpiob::PB0;
    use hal::gpio::{GpioExt, Output, PushPull};
    use hal::prelude::*;
    use hal::rcc::RccExt;
    use hal::stm32;
    use hal::timer::{Timer, TimerExt};

    #[local]
    struct LocalResources {}

    #[shared]
    struct SharedResources {
        timer: Timer<stm32::TIM16>,
        led: PB0<Output<PushPull>>,
    }

    #[init]
    fn init(ctx: init::Context) -> (SharedResources, LocalResources, init::Monotonics) {
        // enable dma clock during sleep, otherwise defmt doesn't work
        ctx.device.RCC.ahbenr.modify(|_, w| w.dmaen().set_bit());

        defmt::println!("Rtic Blinky!");

        // Initialize GPIOs
        let mut rcc = ctx.device.RCC.constrain();
        let gpiob = ctx.device.GPIOB.split(&mut rcc);
        let led = gpiob.pb0.into_push_pull_output();

        // Initialize timers
        let mut timer = ctx.device.TIM16.timer(&mut rcc);
        timer.start(1.hz());
        timer.listen();

        (
            SharedResources { timer, led },
            LocalResources {},
            init::Monotonics(),
        )
    }

    #[task(binds = TIM16, shared = [led, timer])]
    fn timer_tick(mut ctx: timer_tick::Context) {
        ctx.shared.led.lock(|led| led.toggle().unwrap());
        ctx.shared.timer.lock(|timer| timer.clear_irq());
    }
}
