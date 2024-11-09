#![no_std]
#![no_main]

use core::fmt::Write;
use defmt::unwrap;
use defmt_rtt as _; // global logger
use panic_probe as _;
// panic handler
use stm32wlxx_hal::{
    self as hal,
    chrono::{DateTime, Utc},
    embedded_hal::prelude::*,
    gpio::{pins, Output, PinState, PortA, PortB, PortC},
    info::{self},
    pac,
    rcc,
    //rng::{self, Rng},
    rtc::{Clk, Rtc},
    subghz::SubGhz,
    uart::{self, Uart1},
};
use rtic::app;
use rtic_monotonics::systick::prelude::*;
use heapless::String; // fixed capacity `std::Vec`

use bm_network::{
    bm_network_engine::BmNetworkEngine,
    bm_network_engine::BmEngineStatus,
};
mod at_cmd_handler;
use at_cmd_handler::{
    at_cmd::{
        self,
        AtCommand,
        AtCmdStr,
        MessageTuple,
    },
    at_cmd_resp::AtCmdResp,    
};
mod radio_control;
use radio_control::{
    radio_control::RadioState,
    radio_control::RadioControl,
    radio_rx_buffer::RadioRxBuffer,
};

systick_monotonic!(Mono, 1000);

#[app(device = stm32wlxx_hal::pac, peripherals = true, dispatchers = [USART1])]
mod app {
    use super::*;

    #[shared]
    struct Shared {
        uart1: Uart1<pins::B7, pins::B6>,
        rtc: Rtc,
        at_cmd_resp_inst: AtCmdResp,
        radio_inst: RadioControl,
        mesh_inst: BmNetworkEngine,
    }

    // Locals can only be used by 1 task
    #[local]
    struct Local {
        // LED task
        ring_indicator: Output<pins::C0>,
        tx_indicator: Output<pins::C1>,
        rx_indicator: Output<pins::B5>,

        // At Cmd task
        received_cmd: Option<(AtCommand,bool)>,
        resp_value: AtCmdStr,

        // Radio task
        buffer_available_to_parse: Option<RadioRxBuffer>,

        // mesh task
        outbound_buff_avail: bool,
        status: BmEngineStatus,
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
        dp.RCC.bdcr.modify(|_, w| w.lseon().on().lsesysen().enabled());
        while dp.RCC.bdcr.read().lserdy().is_not_ready() {}

        // Setup RTC to use LSE
        let mut rtc: Rtc = Rtc::new(dp.RTC, Clk::Lse, &mut dp.PWR, &mut dp.RCC);
        // Set date time        
        let timestamp_millis: i64 = 1729804645033;
        let secs: i64 = timestamp_millis / 1000;
        let nsec: u32 = unwrap!(u32::try_from(timestamp_millis % 1000).ok()) * 1_000_000;
        let date_time: DateTime<Utc> = unwrap!(DateTime::from_timestamp(secs, nsec));
        rtc.set_date_time(date_time.naive_utc());
        defmt::info!("RTC Init Complete");
        
        // Initialize the systick interrupt & obtain the token to prove that we did
        Mono::start(ctx.core.SYST, 48_000_000);
        
        // TBD how to use RNG in mesh. 
        // Need to add slop time on rebroadcasts?
        // Check airspace before TX
        //let rng: Rng = Rng::new(dp.RNG, rng::Clk::Msi, &mut dp.RCC);

        // Setup GPIO
        let gpioa: PortA = PortA::split(dp.GPIOA, &mut dp.RCC);
        let gpiob: PortB = PortB::split(dp.GPIOB, &mut dp.RCC);
        let gpioc: PortC = PortC::split(dp.GPIOC, &mut dp.RCC);

        // Setup LED's
        let mut led1: Output<pins::C0> = Output::default(gpioc.c0, cs);
        let mut led2: Output<pins::C1> = Output::default(gpioc.c1, cs);
        let mut led3: Output<pins::B5> = Output::default(gpiob.b5, cs);
        led1.set_level(PinState::High);        
        led2.set_level(PinState::High);
        led3.set_level(PinState::High);

        // Setup uart1
        let mut uart1: Uart1<pins::B7, pins::B6> =
            Uart1::new(dp.USART1, 115_200, uart::Clk::Hsi16, &mut dp.RCC)
                .enable_rx(gpiob.b7, cs)
                .enable_tx(gpiob.b6, cs);

        // Setup sub GHz radio instance
        let mut radio_inst = RadioControl::new(
            SubGhz::new(dp.SPI3,  &mut dp.RCC),
            Output::default(gpiob.b0, cs), // TXCO Pwr
            Output::default(gpioa.a4, cs), // RF Ctrl 1
            Output::default(gpioa.a5, cs), // RF Ctrl 2
        );
        // Setup LoRa-E5 specific gpio's
        radio_inst.power_on();
        // Configure LoRa radio
        radio_inst.configure();
        defmt::info!("Radio Init Complete");
       
        // Setup AT command handler
        let at_cmd_resp_inst = AtCmdResp::new();
        defmt::info!("AT Command Init Complete");

        // Grab device number. Unique for each individual device.
        let devnum: u32 = info::Uid64::from_device().devnum();
        // Setup mesh stack
        let mesh_inst = BmNetworkEngine::new(Some(devnum));
        defmt::info!("Mesh Stack Init Complete");

        // Start software tasks
        led_task::spawn().unwrap();
        usart1_rx_task::spawn().unwrap();
        mesh_stack_task::spawn().unwrap();
        radio_health_task::spawn().unwrap();

        write_str_uart1(&mut uart1, "Enter Command: \n\r");
        defmt::info!("Startup Complete");        

        (
            Shared 
            {
                uart1,
                rtc,
                at_cmd_resp_inst,
                radio_inst,
                mesh_inst,
            }, 
            Local 
            {
                ring_indicator: led1,
                tx_indicator: led2,
                rx_indicator: led3,
                received_cmd: None,
                resp_value: AtCmdStr::new(),
                buffer_available_to_parse: None,
                outbound_buff_avail: false,
                status: BmEngineStatus::default(),
            }
        )
    }

    // LED task will poll all other instances for data needed to iluminate the LED's
    #[task(
        shared = [radio_inst, mesh_inst],
        local = [ring_indicator, tx_indicator, rx_indicator],
        priority = 2,
    )]
    async fn led_task(mut ctx: led_task::Context) {
        loop {
            ctx.shared.mesh_inst.lock(|mesh_inst| {
                // Illuminate ring indicator when there is a message waiting for us
                let msg_cnt = mesh_inst.get_inbound_message_count() as u32;
                if msg_cnt > 0 {
                    ctx.local.ring_indicator.set_level_high();
                }
                else {
                    ctx.local.ring_indicator.set_level_low();
                }
            });

            ctx.shared.radio_inst.lock(|radio_inst| {
                // poll radio inst for tx/rx state
                // TODO - this may not work as we are only checking every 100ms
                //   and it could be blocked by IRQ handler??
                match radio_inst.current_state {
                    RadioState::Transmitting => {
                        ctx.local.tx_indicator.set_level_high();
                    }
                    RadioState::Receiving => {
                        ctx.local.rx_indicator.set_level_high();
                    }                    
                    _ => {
                        ctx.local.tx_indicator.set_level_low();
                        ctx.local.rx_indicator.set_level_low();
                    }
                }
            });
            Mono::delay(100.millis()).await;           
        }
    }

    // RB Mesh main task
    //
    // Responsible for grabbing incoming packets from the radio buffer and processing them 
    // in the mesh stack. Run the mesh engine. If the mesh stack has any packets ready to 
    // send, push them to the radio task.
    //
    #[task(
        shared = [uart1, rtc, radio_inst, mesh_inst],
        local = [buffer_available_to_parse, outbound_buff_avail, status],
        priority = 2,
    )]
    async fn mesh_stack_task(mut ctx: mesh_stack_task::Context) {
        loop {
            // Pop packet buffer off rx_buffer and free lock before processing
            ctx.shared.radio_inst.lock(|radio_inst| {
                if radio_inst.rx_buffer.len() > 0 {
                    defmt::info!("mesh_stack_task: rx_buffer_len={}", radio_inst.rx_buffer.len());
                    *ctx.local.buffer_available_to_parse = radio_inst.rx_buffer.pop();                    
                }
                else {
                    *ctx.local.buffer_available_to_parse = None;
                }
            });

            // Push inbound packets to mesh stack
            (
                &mut ctx.shared.mesh_inst,
                &mut ctx.shared.rtc
            ).lock(|mesh_inst, rtc| {
                if let Some(buffer_to_parse) = ctx.local.buffer_available_to_parse {
                    let current_millis: i64 = unwrap!(rtc.date_time()).and_utc().timestamp_millis();
                    mesh_inst.process_packet(buffer_to_parse.length.into(), 
                        &mut buffer_to_parse.buffer,
                        current_millis,
                        buffer_to_parse.rssi);
                }
            });
            
            (
                &mut ctx.shared.mesh_inst,
                &mut ctx.shared.rtc,
                &mut ctx.shared.uart1
            ).lock(|mesh_inst, rtc, uart1| {
                // Run mesh engine
                let current_millis: i64 = unwrap!(rtc.date_time()).and_utc().timestamp_millis();
                let new_status = mesh_inst.run_engine(current_millis);
                if *ctx.local.status != new_status {
                    *ctx.local.status = new_status;

                    // Update UI when we have updates from mesh engine
                    match *ctx.local.status {
                        BmEngineStatus::PerformingNetworkDiscovery => {
                            write_str_uart1(uart1, "\n\r+Searching for route");
                        }
                        BmEngineStatus::RouteFound => {
                            write_str_uart1(uart1, "\n\r+Found route");
                        }
                        BmEngineStatus::SendingPayload => {
                            write_str_uart1(uart1, "\n\r+Sending payload");
                        }
                        BmEngineStatus::AckReceieved => {
                            write_str_uart1(uart1, "\n\r+Ack Received");
                        }
                        BmEngineStatus::ErrorNoRoute => {
                            write_str_uart1(uart1, "\n\r+Error no route");
                        }
                        BmEngineStatus::ErrorNoAck => {
                            write_str_uart1(uart1, "\n\r+Error no ack");
                        }
                        BmEngineStatus::Complete => {
                            write_str_uart1(uart1, "\n\rOk");
                        }
                        _ => { }
                    }
                }
            });                

            // Peek at outbound queue of mesh stack
            ctx.shared.mesh_inst.lock(|mesh_inst| {
                *ctx.local.outbound_buff_avail = mesh_inst.get_next_outbound_packet().is_some();
            });

            // If we have a packet to send in the mesh stack
            if *ctx.local.outbound_buff_avail {
                // Check and wait until the radio is free
                let mut current_state:RadioState = RadioState::default(); 
                ctx.shared.radio_inst.lock(|radio_inst| {
                    current_state = radio_inst.current_state.clone(); 
                });
                while current_state != RadioState::Idle {
                    Mono::delay(100.millis()).await;
                    ctx.shared.radio_inst.lock(|radio_inst| {
                        current_state = radio_inst.current_state.clone(); 
                    });
                }

                // Prepare and send packet to radio
                ctx.shared.mesh_inst.lock(|mesh_inst| {
                    if let Some(outbound_packet) = mesh_inst.get_next_outbound_packet() {
                        if let Some(outbound_packet_bytes) = outbound_packet.clone().to_bytes() {
                            let length_to_send = outbound_packet_bytes.len().try_into().unwrap();
                            defmt::info!("mesh_task: initiate tx len={}", length_to_send);

                            // Initiate TX
                            ctx.shared.radio_inst.lock(|radio_inst| {
                                unwrap!(radio_inst.send_packet( length_to_send, outbound_packet_bytes.as_slice()));
                            });
                        }
                    }
                });

                // Delay until complete
                let mut cont = true;
                while cont {                    
                    ctx.shared.radio_inst.lock(|radio_inst| {
                        current_state = radio_inst.current_state.clone(); 
                    });
                    match current_state {
                        // Wait for radio to finish TX, success or failure
                        // Maybe later handle radio failures seperately??
                        RadioState::Idle | 
                        RadioState::Failure => {
                            cont = false;
                            defmt::info!("mesh_task: RadioState = {}", current_state);

                            // Update packet tx time and tx count
                            (
                                &mut ctx.shared.mesh_inst,
                                &mut ctx.shared.rtc
                            ).lock(|mesh_inst, rtc| {
                                let time_millis = unwrap!(rtc.date_time()).and_utc().timestamp_millis();
                                mesh_inst.set_next_outbound_complete(time_millis);                               
                            });
                        }
                        _ => { }
                    }
                    Mono::delay(50.millis()).await;
                }
            }                

            Mono::delay(100.millis()).await;
        }
    }

    // HAL crates does not support RX interrupts. So polling the peripheral instead.
    //
    // Attempted to make rx at cmd handling use fn callbacks to grab necessary data. The rust
    // language is very painful to deal with. Would have had to pass all struct instances into
    // at_command_handler.
    #[task(
        shared = [uart1, rtc, at_cmd_resp_inst, radio_inst, mesh_inst],
        local = [received_cmd, resp_value],
        priority = 2,
    )]
    async fn usart1_rx_task(mut ctx: usart1_rx_task::Context) {
        loop {
            // Read byte and free up uart1 locks
            (
                &mut ctx.shared.uart1,
                &mut ctx.shared.at_cmd_resp_inst,
            ).lock(|uart1, at_cmd_resp_inst| {
                if let Ok(in_char) = uart1.read() {
                    if let Some((rx_cmd_enum, print_help)) = at_cmd_resp_inst.handle_rx_char(in_char) {
                        defmt::info!("usart1_rx_task: cmd={}, help={}", rx_cmd_enum, print_help);

                        // Current mechanism to print help
                        if print_help {
                            defmt::info!("handle_command: print_help");
                            write_slice_uart1(uart1, at_cmd_resp_inst.prepare_help_str(rx_cmd_enum));
                            return
                        }
                    
                        // Handle parsed AT commands
                        match rx_cmd_enum {                                   
                            AtCommand::At => {
                                // Send generic AT response
                                write_slice_uart1(uart1, 
                                    at_cmd_resp_inst.prepare_response(rx_cmd_enum, "")
                                );                       
                            }
                            AtCommand::AtCsq => {
                                (
                                    &mut ctx.shared.radio_inst
                                ).lock(|radio_inst| {
                                    // Send AT response with value
                                    write_slice_uart1(uart1, 
                                        at_cmd_resp_inst.prepare_response(
                                            rx_cmd_enum, 
                                            // Query instantaneous rssi
                                            radio_inst.check_signal_strength().trim()
                                        )
                                    );
                                });                            
                            }
                            AtCommand::AtGmr => {
                                // Send ID response
                                write_slice_uart1(uart1, 
                                    at_cmd_resp_inst.prepare_response(
                                        rx_cmd_enum, 
                                        "1.0.0.0",
                                    )
                                );
                            }
                            AtCommand::AtId => {
                                (
                                    &mut ctx.shared.mesh_inst
                                ).lock(|mesh_inst| {
                                    // Convert u32 ID to String<>
                                    let str_resp: String<10> = String::try_from(mesh_inst.table.get_local_network_id().unwrap()).unwrap();
                        
                                    // Send ID response
                                    write_slice_uart1(uart1, 
                                        at_cmd_resp_inst.prepare_response(
                                            rx_cmd_enum, 
                                            str_resp.trim(),
                                        )
                                    );
                                });                            
                            }
                            AtCommand::AtMsgReceiveCnt => {
                                ctx.shared.mesh_inst.lock(|mesh_inst| {
                                    // Convert usize count to String<>
                                    let msg_cnt = mesh_inst.get_inbound_message_count() as u32;
                                    let str_resp: String<10> = String::try_from(msg_cnt).unwrap();
                        
                                    defmt::info!("AtMsgReceiveCnt: cnt:{}", msg_cnt);

                                    // Send msg count response
                                    write_slice_uart1(uart1, 
                                        at_cmd_resp_inst.prepare_response(
                                            rx_cmd_enum, 
                                            str_resp.trim(),
                                        )
                                    );
                                });
                            }
                            AtCommand::AtMsgReceive => {
                                ctx.shared.mesh_inst.lock(|mesh_inst| {
                                    if let Some(in_msg) = mesh_inst.get_inbound_message() {
                                        defmt::info!("AtMsgReceive: in_msg:{}", defmt::Display2Format(&in_msg));
                                    }
                                    else {
                                        defmt::info!("AtMsgReceive: No msg available");
                                        write_str_uart1(uart1, "\n\r0 Messages\n\r>");
                                    }
                                });
                            }
                            AtCommand::AtMsgSend => {
                                // Parse argument String buffer into tuple
                                let msg_cmd: Option<MessageTuple> = at_cmd::cmd_arg_into_msg(at_cmd_resp_inst.get_cmd_arg());

                                if let Some((network_id, ack_required, ttl, payload)) = msg_cmd {
                                    defmt::info!("AtMsgSend: id:{} ack:{} ttl:{} payload_len:{}", 
                                        network_id, ack_required, ttl, payload.len());

                                    // Load new packet into engine
                                    ctx.shared.mesh_inst.lock(|mesh_inst| {
                                        mesh_inst.initiate_packet_transfer(network_id, ack_required, ttl, payload);
                                    });
                    
                                    // Do not print Ok response here. Mesh engine state machine will drive UI responses 
                                }
                                else {
                                    defmt::error!("AtMsgSend: Invalid command format");
                                    write_str_uart1(uart1, "\n\rCmd Error\n\r>");
                                }                           
                            }
                            AtCommand::TestMessage => {
                                (
                                    &mut ctx.shared.radio_inst
                                ).lock(|radio_inst| {
                                    // Send AT response
                                    write_slice_uart1(uart1, 
                                        at_cmd_resp_inst.prepare_response(
                                            rx_cmd_enum, 
                                            // Tell radio to TX
                                            radio_inst.send_test_message().trim()
                                        )
                                    );
                                });                            
                            }
                            AtCommand::RoutingTable => {
                                (
                                    &mut ctx.shared.mesh_inst,
                                ).lock(|mesh_inst| {
                                    // Prints out 1 line per node in table.
                                    // Note: May need to refactor this when we support more nodes.
                                    let num_nodes = mesh_inst.table.get_num_nodes();
                                    if num_nodes > 0 {
                                        let mut output_str: String<100> = String::new();
                                        for node_idx in 0..num_nodes {
                                            if let Some(node_data) = mesh_inst.table.get_node_by_idx(node_idx) {
                                                // Write struct to String. Formatter is implemented in node file
                                                write!(&mut output_str, "\n\r{}", node_data).unwrap();
                                                write_str_uart1(uart1, output_str.as_str());
                                            }
                                        }
                                    }
                                    else {
                                        write_str_uart1(uart1, "\n\r0 Nodes");
                                    }
                                    write_str_uart1(uart1, "\n\rOk\n\r>");
                                });
                            }
                            AtCommand::RadioStatus => {
                                (
                                    &mut ctx.shared.radio_inst
                                ).lock(|radio_inst| {
                                    // Print radio status response
                                    write_slice_uart1(uart1, 
                                        at_cmd_resp_inst.prepare_response(
                                            rx_cmd_enum, 
                                            radio_inst.get_status().trim()
                                        )
                                    );
                                });
                            }                
                            AtCommand::AtList => {
                                // Get list of commands from at_command_handler
                                let mut cmd_list: AtCmdStr = AtCmdStr::new();
                                AtCommand::get_available_cmds(&mut cmd_list);
                                write_slice_uart1(uart1, 
                                    at_cmd_resp_inst.prepare_response(
                                        rx_cmd_enum, 
                                        cmd_list.trim()
                                    )
                                );
                            }
                            AtCommand::NewLine => {
                                // Send character: >
                                write_str_uart1(uart1, "\n\r>");
                            }
                            AtCommand::Unknown => {
                                write_str_uart1(uart1, "\n\rCmd Error\n\r>");
                            }
                            _ => {
                                defmt::warn!("usart1_rx_loop: Unhandled at command");
                            }                    
                        }
                    }

                    // Echo entered characters
                    write_u8_uart1(uart1, in_char);                    
                }                
            });            
            
            Mono::delay(10.millis()).await;
        }
    }

    // Interrupt handler for internal radio irq
    // Note: RADIO_IRQ_BUSY doesnt work with DMA version of subghz...
    #[task(
        binds = RADIO_IRQ_BUSY,
        shared = [rtc, radio_inst],
        priority = 2,
    )]
    fn radio_irq(mut ctx: radio_irq::Context)
    {
        (
            &mut ctx.shared.rtc,
            &mut ctx.shared.radio_inst
        ).lock(|rtc, radio_inst| {
            let current_millis: i64 = unwrap!(rtc.date_time()).and_utc().timestamp_millis();
            radio_inst.locked_radio_irq_handler(current_millis);
        });    
    }

    // Task to cover periodic maintenance of radio.
    #[task(
        shared = [rtc, radio_inst],
        priority = 2,
    )]
    async fn radio_health_task(mut ctx: radio_health_task::Context) {
        (
            &mut ctx.shared.rtc,
            &mut ctx.shared.radio_inst
        ).lock(|rtc, radio_inst| {
            let current_millis: i64 = unwrap!(rtc.date_time()).and_utc().timestamp_millis();
            radio_inst.locked_radio_cycle_checks(current_millis);
        });
    }
}


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

