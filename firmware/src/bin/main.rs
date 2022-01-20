#![no_main]
#![no_std]

use clock_frequency_measurement as _; // global logger + panicking-behavior + memory layout
use clock_frequency_measurement::hal;
use hal::rcc::{Enable, Reset};
use hal::stm32;

fn configure_rel_timer(rel_timer: stm32::TIM14, rcc: &mut hal::rcc::Rcc) -> stm32::TIM14 {
    stm32::TIM14::enable(rcc);
    stm32::TIM14::reset(rcc);
    rel_timer.cr1.modify(|_, w| w.cen().clear_bit());
    rel_timer.cnt.reset();
    let psc = 999; // Prescaler of 1000, exactly
    let arr = 0xffff; // Count to max
    rel_timer.psc.write(|w| unsafe { w.psc().bits(psc) });
    rel_timer.arr.write(|w| unsafe { w.bits(arr) });
    rel_timer
        .cr1
        .modify(|_, w| w.cen().set_bit().urs().set_bit());

    rel_timer
}

#[rtic::app(device = clock_frequency_measurement::hal::stm32, peripherals = true)]
mod app {
    use super::hal;
    use hal::gpio::gpiob::PB0;
    use hal::gpio::{GpioExt, Output, PushPull};
    use hal::prelude::*;
    use hal::rcc::{Enable, RccExt, Reset};
    use hal::stm32;
    use hal::timer::{Timer, TimerExt};

    #[local]
    struct LocalResources {
        rel_time_prev_actual: u16,
        rel_time_prev_expected: u16,
    }

    #[shared]
    struct SharedResources {
        timer_1hz: Timer<stm32::TIM16>,
        led: PB0<Output<PushPull>>,
        rel_timer: stm32::TIM14,
    }

    #[init]
    fn init(ctx: init::Context) -> (SharedResources, LocalResources, init::Monotonics) {
        // enable dma clock during sleep, otherwise defmt doesn't work
        ctx.device.RCC.ahbenr.modify(|_, w| w.dmaen().set_bit());

        defmt::println!("Measuring clock frequency ...");

        // Initialize GPIOs
        let mut rcc = ctx.device.RCC.constrain();
        let gpiob = ctx.device.GPIOB.split(&mut rcc);
        let led = gpiob.pb0.into_push_pull_output();

        // Initialize timers
        let mut timer_1hz = ctx.device.TIM16.timer(&mut rcc);
        timer_1hz.start(1.hz());
        timer_1hz.listen();

        // Configure TIM14 to run with clock/1000
        let rel_timer = super::configure_rel_timer(ctx.device.TIM14, &mut rcc);

        (
            SharedResources {
                timer_1hz,
                led,
                rel_timer,
            },
            LocalResources {
                rel_time_prev_actual: 0,
                rel_time_prev_expected: 0,
            },
            init::Monotonics(),
        )
    }

    #[task(binds = TIM16, shared = [led, timer_1hz, rel_timer], local = [rel_time_prev_expected])]
    fn measure_expected(mut ctx: measure_expected::Context) {
        ctx.shared.timer_1hz.lock(|timer| timer.clear_irq());

        ctx.shared.led.lock(|led| led.toggle().unwrap());

        let rel_time = ctx
            .shared
            .rel_timer
            .lock(|timer| timer.cnt.read().cnt().bits());

        let rel_time_diff = rel_time
            .overflowing_sub(*ctx.local.rel_time_prev_expected)
            .0;
        *ctx.local.rel_time_prev_expected = rel_time;

        defmt::println!(
            "Expected: {}.{:03} MHz",
            rel_time_diff / 1000,
            rel_time_diff % 1000
        );
    }

    #[task(binds = TIM14, shared = [led, timer_1hz, rel_timer], local = [rel_time_prev_actual])]
    fn measure_actual(mut ctx: measure_actual::Context) {
        // ctx.shared.timer_1hz.lock(|timer| timer.clear_irq());

        // ctx.shared.led.lock(|led| led.toggle().unwrap());

        // let rel_time = ctx
        //     .shared
        //     .rel_timer
        //     .lock(|timer| timer.cnt.read().cnt().bits());

        // let rel_time_diff = rel_time.overflowing_sub(*ctx.local.rel_time_prev_actual).0;
        // *ctx.local.rel_time_prev_actual = rel_time;

        // defmt::println!(
        //     "Actual: {}.{:03} MHz",
        //     rel_time_diff / 1000,
        //     rel_time_diff % 1000
        // );
    }
}
