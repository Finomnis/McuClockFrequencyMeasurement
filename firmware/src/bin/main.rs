#![no_main]
#![no_std]

use clock_frequency_measurement as _; // global logger + panicking-behavior + memory layout
use clock_frequency_measurement::hal;
use hal::rcc::{Enable, Reset};
use hal::stm32;

fn configure_core_timer(timer_core: stm32::TIM14, rcc: &mut hal::rcc::Rcc) -> stm32::TIM14 {
    stm32::TIM14::enable(rcc);
    stm32::TIM14::reset(rcc);
    timer_core.cr1.modify(|_, w| w.cen().clear_bit());
    timer_core.cnt.reset();
    let psc = 999; // Prescaler of 1000, exactly
    let arr = 0xffff; // Count to max
    timer_core.psc.write(|w| unsafe { w.psc().bits(psc) });
    timer_core.arr.write(|w| unsafe { w.bits(arr) });
    timer_core
        .cr1
        .modify(|_, w| w.cen().set_bit().urs().set_bit());

    timer_core
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
        core_time_prev_actual: u16,
        core_time_prev_expected: u16,
    }

    #[shared]
    struct SharedResources {
        timer_1hz: Timer<stm32::TIM16>,
        led: PB0<Output<PushPull>>,
        timer_core: stm32::TIM14,
    }

    #[init]
    fn init(ctx: init::Context) -> (SharedResources, LocalResources, init::Monotonics) {
        // enable dma clock during sleep, otherwise defmt doesn't work
        ctx.device.RCC.ahbenr.modify(|_, w| w.dmaen().set_bit());

        defmt::println!("Measuring clock frequency ...");

        // Initialize GPIOs
        let mut rcc = ctx.device.RCC.constrain();
        let gpiob = ctx.device.GPIOB.split(&mut rcc);
        let gpioa = ctx.device.GPIOA.split(&mut rcc);
        let led = gpiob.pb0.into_push_pull_output();
        let i2c_sda = gpioa.pa10.into_open_drain_output();
        let i2c_scl = gpioa.pa9.into_open_drain_output();

        // Initialize timers
        let mut timer_1hz = ctx.device.TIM16.timer(&mut rcc);
        timer_1hz.start(1.hz());
        timer_1hz.listen();

        // Configure TIM14 to run with clock/1000
        let timer_core = super::configure_core_timer(ctx.device.TIM14, &mut rcc);

        // Initialize I2C
        let mut i2c =
            ctx.device
                .I2C1
                .i2c(i2c_sda, i2c_scl, hal::i2c::Config::new(100.khz()), &mut rcc);

        (
            SharedResources {
                timer_core,
                timer_1hz,
                led,
            },
            LocalResources {
                core_time_prev_actual: 0,
                core_time_prev_expected: 0,
            },
            init::Monotonics(),
        )
    }

    #[task(binds = TIM16, shared = [led, timer_1hz, timer_core], local = [core_time_prev_expected])]
    fn measure_expected(mut ctx: measure_expected::Context) {
        ctx.shared.timer_1hz.lock(|timer| timer.clear_irq());
        ctx.shared.led.lock(|led| led.toggle().unwrap());

        let core_time = ctx
            .shared
            .timer_core
            .lock(|timer| timer.cnt.read().cnt().bits());

        let (core_time_diff, _) = core_time.overflowing_sub(*ctx.local.core_time_prev_expected);
        *ctx.local.core_time_prev_expected = core_time;

        defmt::println!(
            "Expected: {}.{:03} MHz",
            core_time_diff / 1000,
            core_time_diff % 1000
        );
    }

    #[task(binds = TIM14, shared = [led, timer_1hz, timer_core], local = [core_time_prev_actual])]
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
