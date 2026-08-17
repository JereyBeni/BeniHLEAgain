/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Implementation of the GDB Remote Serial Protocol. This implements a server;
//! the client would be something like GDB or LLDB.
//!
//! Useful resources:
//! - Debugging with GDB, Appendix E: GDB Remote Serial Protocol
//! - The GDB source code:
//!   - include/gdb/signals.def for the meanings of signal numbers
//!   - gdb/arch/arm.h for ARM register numbers

use crate::cpu::{Cpu, CpuError};
use crate::mem::{GuestUSize, Mem, Ptr};
use std::fmt::Write as _;
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::net::TcpStream;
use std::time::Duration;

/// ARM architecture used by the guest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmArchitecture {
    Armv6,
    Armv7,
}

impl ArmArchitecture {
    /// Return the architecture name expected by GDB.
    fn gdb_name(self) -> &'static str {
        match self {
            Self::Armv6 => "armv6",
            Self::Armv7 => "armv7",
        }
    }
}

/// Generate the GDB target description XML.
fn target_xml(architecture: ArmArchitecture) -> String {
    format!(
        r#"
<target version="1.0">
    <architecture>{}</architecture>
    <osabi>Darwin</osabi>
</target>
"#,
        architecture.gdb_name()
    )
}

/// GDB Remote Serial Protocol handler, implementing a server.
pub struct GdbServer {
    reader: BufReader<TcpStream>,
    first_halt: bool,
    architecture: ArmArchitecture,
}

impl GdbServer {
    /// Create the handler from a TCP connection.
    ///
    /// Defaults to ARMv6 for backwards compatibility.
    pub fn new(connection: TcpStream) -> GdbServer {
        Self::new_with_architecture(connection, ArmArchitecture::Armv6)
    }

    /// Create the handler with an explicit ARM architecture.
    pub fn new_with_architecture(
        mut connection: TcpStream,
        architecture: ArmArchitecture,
    ) -> GdbServer {
        connection
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();

        connection
            .set_write_timeout(Some(Duration::from_secs(3)))
            .unwrap();

        let mut hello_byte = [0u8; 1];

        connection
            .read_exact(&mut hello_byte)
            .expect("Could not read greeting");

        assert!(hello_byte[0] == b'+');

        connection
            .write_all(b"+")
            .expect("Could not send greeting");

        GdbServer {
            reader: BufReader::with_capacity(4096, connection),
            first_halt: true,
            architecture,
        }
    }

    fn read_packet(&mut self) -> Option<String> {
        let buffer = match self.reader.fill_buf() {
            Ok(buffer) => buffer,

            Err(e) => match e.kind() {
                ErrorKind::BrokenPipe | ErrorKind::ConnectionReset => {
                    panic!("Lost connection to debugger: {}", e.kind());
                }

                _ => return None,
            },
        };

        if buffer.is_empty() {
            return None;
        }

        if buffer[0] == b'+' {
            self.reader.consume(1);
            log_dbg!("Got ACK");
            return None;
        }

        assert_eq!(buffer[0], b'$');

        let Some(body_end) = buffer.iter().position(|&c| c == b'#') else {
            assert!(buffer.len() != self.reader.capacity());
            log_dbg!("No packet end yet");
            return None;
        };

        let body = &buffer[1..body_end];

        let checksum1 = buffer.get((body_end + 1)..(body_end + 3))?;
        log_dbg!("Have full packet");

        let checksum1 = std::str::from_utf8(checksum1).unwrap();
        let checksum1 = u8::from_str_radix(checksum1, 16).unwrap();

        let checksum2 = body
            .iter()
            .fold(0u8, |a, &b| a.wrapping_add(b));

        assert_eq!(checksum1, checksum2);

        let body = String::from_utf8(body.to_vec()).unwrap();

        self.reader.consume(body_end + 3);

        log_dbg!("Got packet: {:?}", body);

        self.reader
            .get_mut()
            .write_all(b"+")
            .expect("Couldn't send ACK");

        Some(body)
    }

    fn send_packet(&mut self, body: &str) {
        let checksum = body
            .bytes()
            .fold(0u8, |a, b| a.wrapping_add(b));

        write!(
            self.reader.get_mut(),
            "${body}#{checksum:02x}"
        )
        .unwrap();

        log_dbg!("Sent packet: {:?}", body);
    }

    /// Communicates with the debugger.
    ///
    /// Returns true if the CPU should single-step and then resume debugging.
    /// Returns false if it should resume normal execution.
    #[must_use]
    pub fn wait_for_debugger(
        &mut self,
        stop_reason: Option<CpuError>,
        cpu: &mut Cpu,
        mem: &mut Mem,
    ) -> bool {
        echo!("Waiting for debugger to continue.");

        match stop_reason {
            None => {
                if self.first_halt {
                    self.first_halt = false;
                } else {
                    self.send_packet("S05");
                }
            }

            Some(CpuError::UndefinedInstruction)
            | Some(CpuError::Breakpoint) => {
                self.send_packet("S05");
            }

            Some(CpuError::MemoryError) => {
                self.send_packet("S0b");
            }
        }

        let do_step = loop {
            let Some(p) = self.read_packet() else {
                continue;
            };

            if p.is_empty() {
                continue;
            }

            match p.as_bytes()[0] {
                // Query for target halt reason.
                b'?' => {
                    assert!(stop_reason.is_none());
                    self.send_packet("S00");
                }

                // Read general registers.
                b'g' => {
                    let mut packet =
                        String::with_capacity(16 * 4 * 2);

                    for reg in cpu.regs() {
                        let reg =
                            u32::from_be_bytes(reg.to_le_bytes());

                        write!(packet, "{reg:08x}").unwrap();
                    }

                    self.send_packet(&packet);
                }

                // Write general registers.
                b'G' => {
                    let data = &p[1..];
                    let regs = cpu.regs_mut();

                    assert!(data.len() == regs.len() * 4 * 2);

                    for (i, reg) in regs.iter_mut().enumerate() {
                        let word =
                            &data[i * 4 * 2..][..4 * 2];

                        let word =
                            u32::from_str_radix(word, 16).unwrap();

                        let word =
                            u32::from_le_bytes(word.to_be_bytes());

                        *reg = word;
                    }

                    self.send_packet("OK");
                }

                // Read single register.
                b'p' => {
                    let num =
                        usize::from_str_radix(&p[1..], 16).unwrap();

                    let reg = if num < 16 {
                        Some(cpu.regs()[num])
                    } else if num == 25 {
                        Some(cpu.cpsr())
                    } else if (26..=57).contains(&num) {
                        None
                    } else if num == 58 {
                        Some(0u32)
                    } else if (16..=24).contains(&num) {
                        Some(0u32)
                    } else {
                        None
                    };

                    if (26..=57).contains(&num) {
                        self.send_packet(
                            "0000000000000000",
                        );
                    } else if let Some(reg) = reg {
                        let reg =
                            u32::from_be_bytes(reg.to_le_bytes());

                        self.send_packet(
                            &format!("{reg:08x}"),
                        );
                    } else {
                        self.send_packet("E00");
                    }
                }

                // Write single register.
                b'P' => {
                    let (num, word) =
                        p[1..].split_once('=').unwrap();

                    let num =
                        usize::from_str_radix(num, 16).unwrap();

                    if num < 16 {
                        let word =
                            u32::from_str_radix(word, 16).unwrap();

                        let word =
                            u32::from_le_bytes(word.to_be_bytes());

                        cpu.regs_mut()[num] = word;

                        self.send_packet("OK");
                    } else if num == 25 {
                        let word =
                            u32::from_str_radix(word, 16).unwrap();

                        let word =
                            u32::from_le_bytes(word.to_be_bytes());

                        cpu.set_cpsr(word);

                        self.send_packet("OK");
                    } else if (26..=57).contains(&num)
                        || num == 58
                        || (16..=24).contains(&num)
                    {
                        self.send_packet("OK");
                    } else {
                        self.send_packet("E00");
                    }
                }

                // Read memory.
                b'm' => {
                    let (addr, length) =
                        p[1..].split_once(',').unwrap();

                    let addr =
                        GuestUSize::from_str_radix(addr, 16)
                            .unwrap();

                    let length =
                        GuestUSize::from_str_radix(length, 16)
                            .unwrap();

                    let mut packet =
                        String::with_capacity(length as usize * 2);

                    match mem.get_bytes_fallible(
                        Ptr::from_bits(addr),
                        length,
                    ) {
                        Some(data) => {
                            for byte in data {
                                write!(
                                    packet,
                                    "{byte:02x}"
                                )
                                .unwrap();
                            }
                        }

                        None => {
                            write!(packet, "E00").unwrap();
                        }
                    }

                    self.send_packet(&packet);
                }

                // Write memory.
                b'M' => {
                    let (header, data) =
                        p[1..].split_once(':').unwrap();

                    let (addr, length) =
                        header.split_once(',').unwrap();

                    let addr =
                        GuestUSize::from_str_radix(addr, 16)
                            .unwrap();

                    let length =
                        GuestUSize::from_str_radix(length, 16)
                            .unwrap();

                    assert!(
                        data.len() == length as usize * 2
                    );

                    match mem.get_bytes_fallible_mut(
                        Ptr::from_bits(addr),
                        length,
                    ) {
                        Some(dest) => {
                            for i in 0..length as usize {
                                let byte =
                                    &data[i * 2..][..2];

                                let byte =
                                    u8::from_str_radix(
                                        byte,
                                        16,
                                    )
                                    .unwrap();

                                dest[i] = byte;
                            }

                            cpu.invalidate_cache_range(
                                addr,
                                length,
                            );

                            self.send_packet("OK");
                        }

                        None => {
                            self.send_packet("E00");
                        }
                    }
                }

                // Continue or Step.
                b'c' | b's' => {
                    let addr = &p[1..];

                    if !addr.is_empty() {
                        if let Ok(new_pc) =
                            u32::from_str_radix(addr, 16)
                        {
                            log!(
                                "GDB: Resume at address {:#x}",
                                new_pc
                            );

                            let func =
                                crate::abi::GuestFunction
                                    ::from_addr_with_thumb_bit(
                                        new_pc,
                                    );

                            cpu.branch(func);
                        } else {
                            log!(
                                "GDB: Could not parse resume address {:?}, ignoring",
                                addr
                            );
                        }
                    }

                    break p.as_bytes()[0] == b's';
                }

                // Continue/Step with signal.
                b'C' | b'S' => {
                    if let Some((_signal, addr)) =
                        p[1..].split_once(';')
                    {
                        if !addr.is_empty() {
                            if let Ok(new_pc) =
                                u32::from_str_radix(addr, 16)
                            {
                                log!(
                                    "GDB: Resume with signal at address {:#x}",
                                    new_pc
                                );

                                let func =
                                    crate::abi::GuestFunction
                                        ::from_addr_with_thumb_bit(
                                            new_pc,
                                        );

                                cpu.branch(func);
                            } else {
                                log!(
                                    "GDB: Could not parse resume address {:?}, ignoring",
                                    addr
                                );
                            }
                        }
                    }

                    break p.as_bytes()[0] == b'S';
                }

                // Kill.
                b'k' => {
                    panic!("Debugger requested kill.");
                }

                _ => {
                    if p == "qAttached" {
                        self.send_packet("0");
                    } else if p == "qSupported"
                        || p.starts_with("qSupported:")
                    {
                        self.send_packet(
                            "qXfer:features:read+",
                        );
                    } else if let Some(params) =
                        p.strip_prefix(
                            "qXfer:features:read:",
                        )
                    {
                        let (annex, params) =
                            params.split_once(':').unwrap();

                        let (offset, length) =
                            params.split_once(',').unwrap();

                        let offset =
                            usize::from_str_radix(
                                offset,
                                16,
                            )
                            .unwrap();

                        let length =
                            usize::from_str_radix(
                                length,
                                16,
                            )
                            .unwrap();

                        let xml =
                            target_xml(self.architecture);

                        let bytes = xml.as_bytes();

                        if annex == "target.xml"
                            && offset <= bytes.len()
                        {
                            let bytes =
                                &bytes[offset..];

                            let length_read =
                                length.min(bytes.len());

                            let mut packet =
                                String::with_capacity(
                                    1 + length_read,
                                );

                            if length_read < length {
                                packet.push('l');
                            } else {
                                packet.push('m');
                            }

                            packet.push_str(
                                std::str::from_utf8(
                                    &bytes[..length_read],
                                )
                                .unwrap(),
                            );

                            self.send_packet(&packet);
                        } else {
                            self.send_packet("E00");
                        }
                    } else {
                        log_dbg!("Unhandled packet.");
                        self.send_packet("");
                    }
                }
            }
        };

        if do_step {
            echo!(
                "Debugger requested step, resuming execution for one instruction only."
            );
        } else {
            echo!(
                "Debugger requested continue, resuming execution."
            );
        }

        do_step
    }
}
