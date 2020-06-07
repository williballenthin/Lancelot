use std::collections::VecDeque;

use anyhow::Result;
use fnv::FnvHashMap;
use log::debug;
use smallvec::SmallVec;

use crate::{analysis::dis, aspace::AddressSpace, loader::pe::PE, VA};

#[derive(Debug, Clone)]
pub struct BasicBlock {
    /// start VA of the basic block.
    pub addr: VA,

    /// length of the basic block in bytes.
    pub length: u64,

    /// VAs of start addresses of basic blocks that flow here.
    pub predecessors: SmallVec<[VA; 1]>,

    /// VAs of start addresses of basic blocks that flow from here.
    pub successors: SmallVec<[VA; 2]>,
}

pub struct CFG {
    // using FNV because the keys are small
    basic_blocks: FnvHashMap<VA, BasicBlock>,
}

/// Does the given instruction have a fallthrough flow?
pub fn does_insn_fallthrough(insn: &zydis::DecodedInstruction) -> bool {
    match insn.mnemonic {
        zydis::Mnemonic::JMP => false,
        zydis::Mnemonic::RET => false,
        zydis::Mnemonic::IRET => false,
        zydis::Mnemonic::IRETD => false,
        zydis::Mnemonic::IRETQ => false,
        // TODO: call may not fallthrough if function is noret.
        // will need another pass to clean this up.
        zydis::Mnemonic::CALL => true,
        _ => true,
    }
}

struct InstructionDescriptor {
    addr: VA,
    length: u64,
    does_fallthrough: bool,
    successors: SmallVec<[VA; 2]>,
}

pub fn build_cfg(pe: &PE, va: VA) -> Result<CFG> {
    let decoder = dis::get_disassembler(pe)?;
    let mut insn_buf = [0u8; 16];

    let mut queue: VecDeque<VA> = Default::default();
    queue.push_back(va);

    loop {
        let va = match queue.pop_back() {
            None => break,
            Some(va) => va,
        };

        // TODO: better error handling.
        pe.module.address_space.read_into(va, &mut insn_buf)?;

        // TODO: better error handling.
        if let Ok(Some(insn)) = decoder.decode(&insn_buf) {
            if does_insn_fallthrough(&insn) {
                queue.push_back(va + insn.length as u64);
            }
        }
    }

    Ok(CFG {
        basic_blocks: Default::default(),
    })
}
