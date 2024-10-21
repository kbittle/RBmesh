// Blinks the LED on the LoRa-E5_STM32WLE5JC_Module.

#![no_std]
#![no_main]

use core::fmt::Write;
use defmt::unwrap;
use defmt_rtt as _; // global logger
use panic_probe as _; use radio_control::RadioControl;
// panic handler
use stm32wlxx_hal::{
    self as hal,
    cortex_m::{self, delay::Delay},
    dma::{AllDma, Dma1Ch1, Dma2Ch1, Dma1Ch3, Dma2Ch6},
    embedded_hal::prelude::*,
    gpio::{pins, Output, PinState, PortA, PortB, PortC, Exti},
    info::{self, Package, Uid, Uid64},
    pac,
    rcc,
    rng::{self, Rng},
    subghz::{SubGhz},
    uart::{self, Uart1},
    util::new_delay,
};
use rtic::app;
use rtic_monotonics::systick::prelude::*;
use core::time::Duration;
use heapless::{Vec, String}; // fixed capacity `std::Vec`

mod at_command_handler;
mod radio_control;

systick_monotonic!(Mono, 1000);

fn write_str_uart1(uart1: &mut Uart1<pins::B7, pins::B6>, msg:&str)
{
    uart1.write_str(msg).unwrap();
}

fn write_slice_uart1(uart1: &mut Uart1<pins::B7, pins::B6>, msg:&[u8])
{
    uart1.write_str(core::str::from_utf8(msg).unwrap()).unwrap();
}

fn write_u8_uart1(uart1: &mut Uart1<pins::B7, pins::B6>, char:u8)
{
    uart1.write_char(char::from(char)).unwrap();
}

#[app(device = stm32wlxx_hal::pac, peripherals = true, dispatchers = [USART1])]
mod app {
    use super::*;

    #[shared]
    struct Shared {
        uart1: Uart1<pins::B7, pins::B6>,
        at_cmd_resp_inst: at_command_handler::AtCmdResp,
        radio_inst: radio_control::RadioControl,
    }

    // Locals can only be used by 1 task
    #[local]
    struct Local {
        led1: Output<pins::C0>,
        state: bool,
        
        resp_value: at_command_handler::AtCmdStr,
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local) {
        let mut dp: pac::Peripherals = ctx.device;

        // symptom of a version mismatch when using the RTIC alpha
        // see: https://github.com/rust-embedded/cortex-m/pull/350
        // replace with `ctx.cs` when cortex-m gets updated
        // KKB: This was reported in 2021, no fixed??
        let cs = unsafe { &hal::cortex_m::interrupt::CriticalSection::new() };
        unsafe {
            rcc::set_sysclk_msi(
                &mut dp.FLASH,
                &mut dp.PWR,
                &mut dp.RCC,
                rcc::MsiRange::Range48M,
                cs,
            )
        };

        // enable the HSI16 source clock
        dp.RCC.cr.modify(|_, w| w.hsion().set_bit());
        while dp.RCC.cr.read().hsirdy().is_not_ready() {}

        // start enabling the LSE clock before we need it
        unsafe { rcc::pulse_reset_backup_domain(&mut dp.RCC, &mut dp.PWR) };
        dp.PWR.cr1.modify(|_, w| w.dbp().enabled());
        // dp.RCC
        //     .bdcr
        //     .modify(|_, w| w.lseon().on().lsesysen().enabled());
        
        // Initialize the systick interrupt & obtain the token to prove that we did
        Mono::start(ctx.core.SYST, 48_000_000);
        
        let dma: AllDma = AllDma::split(dp.DMAMUX, dp.DMA1, dp.DMA2, &mut dp.RCC);

        // Setup GPIO
        let gpioa: PortA = PortA::split(dp.GPIOA, &mut dp.RCC);
        let gpiob: PortB = PortB::split(dp.GPIOB, &mut dp.RCC);
        let gpioc: PortC = PortC::split(dp.GPIOC, &mut dp.RCC);

        // Setup LED
        let mut led1: Output<pins::C0> = Output::default(gpioc.c0, cs);
        led1.set_level(PinState::Low);

        // Setup uart1
        let mut uart1: Uart1<pins::B7, pins::B6> =
            Uart1::new(dp.USART1, 115_200, uart::Clk::Hsi16, &mut dp.RCC)
                .enable_rx(gpiob.b7, cs)
                .enable_tx(gpiob.b6, cs);

        // Setup sub GHz radio instance
        let mut radio_inst = radio_control::RadioControl::new(
            SubGhz::new(dp.SPI3,  &mut dp.RCC),
            Output::default(gpiob.b0, cs),
            Output::default(gpioa.a4, cs),
            Output::default(gpioa.a5, cs),
        );
        // Setup LoRa-E5 specific gpio's
        radio_inst.power_on();
        // Configure LoRa radio
        radio_inst.configure();

        // Schedule the blinking task
        blinkTask::spawn().unwrap();
        usart1_rx_task::spawn().unwrap();
       
        let mut at_cmd_resp_inst = at_command_handler::AtCmdResp::new();
        at_cmd_resp_inst.add_at_cmd(at_command_handler::AtCommandSet::CmdAt, "", false, "");
        at_cmd_resp_inst.add_at_cmd(at_command_handler::AtCommandSet::CmdAtCsq, "+CSQ", false, "+CSQ:");
        at_cmd_resp_inst.add_at_cmd(at_command_handler::AtCommandSet::CmdAtGmr, "+GMR", false, "Version:");
        at_cmd_resp_inst.add_at_cmd(at_command_handler::AtCommandSet::CmdAtId, "+ID", true, "+ID:");
        at_cmd_resp_inst.add_at_cmd(at_command_handler::AtCommandSet::CmdList, "?", false, "");
        at_cmd_resp_inst.add_at_cmd(at_command_handler::AtCommandSet::CmdSendMessage, "+SEND", false, "+");
        at_cmd_resp_inst.add_at_cmd(at_command_handler::AtCommandSet::CmdRadioStatus, "+ST", false, "+");

        write_str_uart1(&mut uart1, "Enter Command: \n\r");
        defmt::info!("Startup Complete");

        (
            Shared 
            {
                uart1,
                at_cmd_resp_inst,
                radio_inst,
            }, 
            Local 
            {
                led1, 
                state: false,
                resp_value: at_command_handler::AtCmdStr::new(),
            }
        )
    }

    // Blink task currently used as a visual watchdog
    #[task(
        local = [led1, state],
        priority = 2,
    )]
    async fn blinkTask(mut ctx: blinkTask::Context) {
        loop {

            if *ctx.local.state {
                ctx.local.led1.set_level_high();
                *ctx.local.state = false;
            } else {
                ctx.local.led1.set_level_low();
                *ctx.local.state = true;
            }
            Mono::delay(1000.millis()).await;
        }
    }

    // HAL crates does not support RX interrupts. So polling the peripheral instead.
    //
    // Attempted to make rx at cmd handling use fn callbacks to grab necessary data. The rust
    // language is very painful to deal with. Would have had to pass all struct instances into
    // at_command_handler.
    #[warn(unused_assignments)]
    #[task(
        shared = [uart1, at_cmd_resp_inst, radio_inst],
        local = [resp_value],
        priority = 2,
    )]
    async fn usart1_rx_task(mut ctx: usart1_rx_task::Context) {
        loop {
            ctx.shared.uart1.lock(|uart1| {
                if let Ok(received_byte) = uart1.read() {
                    ctx.shared.at_cmd_resp_inst.lock(|at_cmd_resp_inst| {
                        // Handle inbound character
                        let rx_cmd_enum = at_cmd_resp_inst.handle_command(received_byte);

                        // Handle parsed AT commands
                        match rx_cmd_enum {
                            at_command_handler::AtCommandSet::CmdNewLine => {                                
                                // Send character: >
                                write_str_uart1(uart1, "\n\r>");
                            }
                            at_command_handler::AtCommandSet::CmdAt => {                                
                                // Send generic AT response
                                write_slice_uart1(uart1, at_cmd_resp_inst.prepare_response(rx_cmd_enum, ctx.local.resp_value));
                            }
                            at_command_handler::AtCommandSet::CmdAtCsq => {
                                // Todo - get signal str from radio
                                ctx.shared.radio_inst.lock(|radio_inst| {
                                    *ctx.local.resp_value = radio_inst.check_signal_strength();
                                });

                                // Send AT response with value
                                write_slice_uart1(uart1, at_cmd_resp_inst.prepare_response(rx_cmd_enum, ctx.local.resp_value));
                            }
                            at_command_handler::AtCommandSet::CmdSendMessage => {
                                // Tell radio to TX
                                ctx.shared.radio_inst.lock(|radio_inst| {
                                    *ctx.local.resp_value = radio_inst.send_hello_world();
                                });

                                // Send AT response
                                write_slice_uart1(uart1, at_cmd_resp_inst.prepare_response(rx_cmd_enum, ctx.local.resp_value));
                            }
                            at_command_handler::AtCommandSet::CmdRadioStatus => {
                                ctx.shared.radio_inst.lock(|radio_inst| {
                                    *ctx.local.resp_value = radio_inst.get_status();
                                });

                                // Send AT response
                                write_slice_uart1(uart1, at_cmd_resp_inst.prepare_response(rx_cmd_enum, ctx.local.resp_value));
                            }
                            at_command_handler::AtCommandSet::CmdList => {
                                // Get list of commands from at_command_handler
                                *ctx.local.resp_value = at_cmd_resp_inst.get_available_cmds();
                                // Send generic AT response
                                write_slice_uart1(uart1, at_cmd_resp_inst.prepare_response(rx_cmd_enum, ctx.local.resp_value));
                            }
                            at_command_handler::AtCommandSet::CmdUnknown => {
                                // Dont do anything for unknowns
                            }
                            _ => {
                                defmt::warn!("usart1_rx_loop: Unhandled at command");
                            }
                        }
                    });

                    // Echo entered characters
                    write_u8_uart1(uart1, received_byte);
                };
            });
            
            Mono::delay(10.millis()).await;
        }
    }

    // Note: RADIO_IRQ_BUSY doesnt work with DMA version of subghz...
    #[task(
        binds = RADIO_IRQ_BUSY,
        shared = [radio_inst],
        priority = 2,
    )]
    fn radio_polling_task(mut ctx: radio_polling_task::Context)
    {
        ctx.shared.radio_inst.lock(|radio_inst| {
            radio_inst.locked_radio_irq_handler();
        });    
    }
}