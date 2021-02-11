use std::unimplemented;

use anyhow::Result;
use log::debug;
use thiserror::Error;
use zydis::enums::Register;

use crate::{
    arch::Arch,
    aspace::AddressSpace,
    module::{Module, Permissions},
    VA,
};

pub mod mmu;
pub mod reg;

#[derive(Error, Debug)]
pub enum EmuError {
    #[error("invalid instruction: {0:#x}")]
    InvalidInstruction(VA),
}

#[derive(Clone)]
pub struct Emulator {
    mem: mmu::MMU,
    reg: reg::Registers,
    dis: zydis::Decoder,
}

impl Emulator {
    pub fn with_arch(arch: Arch) -> Emulator {
        let mut decoder = match arch {
            Arch::X64 => zydis::Decoder::new(zydis::MachineMode::LONG_64, zydis::AddressWidth::_64).unwrap(),
            Arch::X32 => zydis::Decoder::new(zydis::MachineMode::LEGACY_32, zydis::AddressWidth::_32).unwrap(),
        };

        // modes described here: https://github.com/zyantific/zydis/blob/5af06d64432aaa3f6af3cd3e120eefa061b790ab/include/Zydis/Decoder.h#L55
        // start with minimal until we need FULL
        decoder.enable_mode(zydis::DecoderMode::MINIMAL, false).unwrap();

        decoder.enable_mode(zydis::DecoderMode::KNC, false).unwrap();
        decoder.enable_mode(zydis::DecoderMode::MPX, false).unwrap();
        decoder.enable_mode(zydis::DecoderMode::CET, false).unwrap();
        decoder.enable_mode(zydis::DecoderMode::LZCNT, false).unwrap();
        decoder.enable_mode(zydis::DecoderMode::TZCNT, false).unwrap();
        decoder.enable_mode(zydis::DecoderMode::WBNOINVD, false).unwrap();
        decoder.enable_mode(zydis::DecoderMode::CLDEMOTE, false).unwrap();

        Emulator {
            mem: Default::default(),
            reg: Default::default(),
            dis: decoder,
        }
    }

    pub fn from_module(m: &Module) -> Emulator {
        let mut emu = Emulator::with_arch(m.arch);

        for section in m.sections.iter() {
            let mut page_addr = section.virtual_range.start;

            let section_size = section.virtual_range.end - section.virtual_range.start;
            emu.mem
                .mmap(
                    section.virtual_range.start,
                    crate::util::align(section_size, mmu::PAGE_SIZE as u64),
                    Permissions::W,
                )
                .unwrap();

            while page_addr < section.virtual_range.end {
                let mut page = [0u8; mmu::PAGE_SIZE];

                // AddressSpace currently allows non-page-aligned sizes.
                let page_data = if page_addr + mmu::PAGE_SIZE as u64 > section.virtual_range.end {
                    &mut page[..(section.virtual_range.end - page_addr) as usize]
                } else {
                    &mut page[..]
                };

                m.address_space.read_into(page_addr, page_data).unwrap();
                emu.mem.write_page(page_addr, &page[..]).unwrap();
                page_addr += mmu::PAGE_SIZE as u64;
            }

            emu.mem
                .mprotect(
                    section.virtual_range.start,
                    crate::util::align(section_size, mmu::PAGE_SIZE as u64),
                    section.permissions,
                )
                .unwrap();
        }

        emu
    }

    fn fetch(&mut self) -> Result<zydis::DecodedInstruction> {
        // TODO: 32 vs 64
        let pc = self.reg.rip;
        debug!("emu: fetch: {:#x}", pc);

        // TODO: segmentation.

        let mut buf = [0u8; 16];

        // TODO: perms for read should be X, not R.
        self.mem.read(pc, &mut buf[..])?;
        // TODO: if fail, callback.

        let insn = self.dis.decode(&buf[..]);

        if let Ok(Some(insn)) = insn {
            Ok(insn)
        } else {
            Err(EmuError::InvalidInstruction(pc).into())
        }
    }

    fn read_register(&self, reg: Register) -> u64 {
        match reg {
            Register::RAX => self.reg.rax,
            Register::RBX => self.reg.rbx,
            Register::RCX => self.reg.rcx,
            Register::RDX => self.reg.rdx,
            Register::R8 => self.reg.r8,
            Register::R9 => self.reg.r9,
            Register::R10 => self.reg.r10,
            Register::R11 => self.reg.r11,
            Register::R12 => self.reg.r12,
            Register::R13 => self.reg.r13,
            Register::R14 => self.reg.r14,
            Register::R15 => self.reg.r15,
            Register::RSI => self.reg.rsi,
            Register::RDI => self.reg.rdi,
            Register::RSP => self.reg.rsp,
            Register::RBP => self.reg.rbp,
            _ => unimplemented!(),
        }
    }

    fn write_register(&mut self, reg: Register, size: u16, value: u64) {
        let reg = match reg {
            Register::RAX | Register::EAX | Register::AX | Register::AL => &mut self.reg.rax,
            Register::RBX | Register::EBX | Register::BX | Register::BL => &mut self.reg.rbx,
            Register::RCX | Register::ECX | Register::CX | Register::CL => &mut self.reg.rcx,
            Register::RDX | Register::EDX | Register::DX | Register::DL => &mut self.reg.rdx,
            Register::R8 | Register::R8D | Register::R8W | Register::R8B => &mut self.reg.r8,
            Register::R9 | Register::R9D | Register::R9W | Register::R9B => &mut self.reg.r9,
            Register::R10 | Register::R10D | Register::R10W | Register::R10B => &mut self.reg.r10,
            Register::R11 | Register::R11D | Register::R11W | Register::R11B => &mut self.reg.r11,
            Register::R12 | Register::R12D | Register::R12W | Register::R12B => &mut self.reg.r12,
            Register::R13 | Register::R13D | Register::R13W | Register::R13B => &mut self.reg.r13,
            Register::R14 | Register::R14D | Register::R14W | Register::R14B => &mut self.reg.r14,
            Register::R15 | Register::R15D | Register::R15W | Register::R15B => &mut self.reg.r15,
            Register::RSI | Register::ESI | Register::SI | Register::SIL => &mut self.reg.rsi,
            Register::RDI | Register::EDI | Register::DI | Register::DIL => &mut self.reg.rdi,
            Register::RSP | Register::ESP | Register::SP | Register::SPL => &mut self.reg.rsp,
            Register::RBP | Register::EBP | Register::BP | Register::BPL => &mut self.reg.rbp,
            Register::AH => unimplemented!(),
            Register::BH => unimplemented!(),
            Register::CH => unimplemented!(),
            Register::DH => unimplemented!(),
            _ => unimplemented!(),
        };

        match size {
            64 => {
                *reg = value;
            }
            32 => {
                *reg &= 0xFFFF_FFFF_0000_0000;
                *reg |= value & 0xFFFF_FFFF;
            }
            16 => {
                *reg &= 0xFFFF_FFFF_FFFF_0000;
                *reg |= value & 0xFFFF;
            }
            8 => {
                *reg &= 0xFFFF_FFFF_FFFF_FF00;
                *reg |= value & 0xFF;
            }
            _ => unimplemented!(),
        }
    }

    pub fn step(&mut self) -> Result<()> {
        use zydis::enums::{Mnemonic::*, OperandType::*};

        debug!("emu: step: {:#x}", self.reg.rip);
        let insn = self.fetch()?;
        // TODO: handle invalid fetch
        // TODO: handle invalid instruction

        debug!("emu: step: {:#x}: {:#?}", self.reg.rip, insn.mnemonic);
        match insn.mnemonic {
            MOV => {
                //println!("{:#?}", insn);

                let dst = &insn.operands[0];
                let src = &insn.operands[1];

                let value = match src.ty {
                    IMMEDIATE => src.imm.value,
                    REGISTER => self.read_register(src.reg),
                    _ => unimplemented!(),
                };

                match dst.ty {
                    REGISTER => {
                        self.write_register(dst.reg, dst.size, value);
                    }
                    _ => unimplemented!(),
                }

                self.reg.rip += insn.length as u64;
            }
            _ => {
                self.reg.rip += insn.length as u64;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{arch::Arch, emu::*, test::*};

    use anyhow::Result;

    const BASE_ADDRESS: u64 = 0x1000;

    #[test]
    fn raw_create() -> Result<()> {
        //init_logging();

        let mut emu: Emulator = Emulator::with_arch(Arch::X64);

        emu.mem.mmap(BASE_ADDRESS, 0x1000, Permissions::RWX)?;

        // 0:  48 c7 c0 01 00 00 00    mov    rax,0x1
        let code = b"\x48\xC7\xC0\x01\x00\x00\x00";
        emu.mem.write(BASE_ADDRESS, &code[..])?;

        emu.reg.rip = BASE_ADDRESS;
        emu.step()?;

        assert_eq!(emu.reg.rip, BASE_ADDRESS + 0x7);
        assert_eq!(emu.reg.rax, 1);

        Ok(())
    }

    #[test]
    fn from_module() -> Result<()> {
        // 0:  48 c7 c0 01 00 00 00    mov    rax,0x1
        let m = load_shellcode64(&b"\x48\xC7\xC0\x01\x00\x00\x00"[..]);

        let mut emu = Emulator::from_module(&m);
        emu.reg.rip = m.address_space.base_address;
        emu.step()?;

        assert_eq!(emu.reg.rip, m.address_space.base_address + 0x7);
        assert_eq!(emu.reg.rax, 1);

        Ok(())
    }

    fn emu_from_sc(code: &[u8]) -> Emulator {
        let m = load_shellcode64(code);
        let mut emu = Emulator::from_module(&m);
        emu.reg.rip = m.address_space.base_address; // 0x1000

        emu.mem.mmap(0x5000, 0x2000, Permissions::RW).unwrap();
        emu.reg.rsp = 0x6000;
        emu.reg.rbp = 0x6000;

        emu
    }

    #[test]
    fn insn_mov_reg_imm() -> Result<()> {
        // 0:  48 c7 c0 01 00 00 00    mov    rax,0x1
        let mut emu = emu_from_sc(&b"\x48\xC7\xC0\x01\x00\x00\x00"[..]);
        emu.step()?;
        assert_eq!(emu.reg.rax, 1);

        // 0:  b8 01 00 00 00          mov    eax,0x1
        let mut emu = emu_from_sc(&b"\xB8\x01\x00\x00\x00"[..]);
        emu.reg.rax = 0xFFFF_FFFF_FFFF_FFFF;
        emu.step()?;
        assert_eq!(emu.reg.eax(), 1);
        assert_eq!(emu.reg.rax(), 0xFFFF_FFFF_0000_0001);

        // 0:  66 b8 01 00             mov    ax,0x1
        let mut emu = emu_from_sc(&b"\x66\xB8\x01\x00"[..]);
        emu.reg.rax = 0xFFFF_FFFF_FFFF_FFFF;
        emu.step()?;
        assert_eq!(emu.reg.ax(), 1);
        assert_eq!(emu.reg.eax(), 0xFFFF_0001);
        assert_eq!(emu.reg.rax(), 0xFFFF_FFFF_FFFF_0001);

        // 0:  b0 01                   mov    al,0x1
        let mut emu = emu_from_sc(&b"\xB0\x01"[..]);
        emu.reg.rax = 0xFFFF_FFFF_FFFF_FFFF;
        emu.step()?;
        assert_eq!(emu.reg.al(), 1);
        assert_eq!(emu.reg.ax(), 0xFF01);
        assert_eq!(emu.reg.eax(), 0xFFFF_FF01);
        assert_eq!(emu.reg.rax(), 0xFFFF_FFFF_FFFF_FF01);

        Ok(())
    }

    #[test]
    fn insn_mov_reg_reg() -> Result<()> {
        // 0:  48 89 c3                mov    rbx,rax
        let mut emu = emu_from_sc(&b"\x48\x89\xC3"[..]);
        emu.reg.rax = 1;
        emu.step()?;

        assert_eq!(emu.reg.rax, 1);
        assert_eq!(emu.reg.rbx, 1);

        Ok(())
    }
}