package workspace

// TODO:
//   - AddressSpace interface
//   - higher level maps api
//     - track allocations
//     - snapshot, revert, commit
//  - then, forward-emulate one instruction (via code hook) to get next insn

import (
	"bytes"
	"encoding/binary"
	"errors"
	"fmt"
	"github.com/bnagy/gapstone"
	"io"
	"log"
)

func check(e error) {
	if e != nil {
		panic(e)
	}
}

const MAX_INSN_SIZE = 0x10

type Arch string
type Mode string

const ARCH_X86 Arch = "x86"
const MODE_32 Mode = "32"
const MODE_64 Mode = "64"

var GAPSTONE_ARCH_MAP = map[Arch]int{
	ARCH_X86: gapstone.CS_ARCH_X86,
}

var GAPSTONE_MODE_MAP = map[Mode]uint{
	MODE_32: gapstone.CS_MODE_32,
}

var InvalidArchError = errors.New("Invalid ARCH provided.")
var InvalidModeError = errors.New("Invalid MODE provided.")

type LoadedModule struct {
	Name        string
	BaseAddress VA
	EntryPoint  VA
}

func (m LoadedModule) VA(rva RVA) VA {
	return rva.VA(m.BaseAddress)
}

// note: relative to the module
func (m LoadedModule) MemRead(ws *Workspace, rva RVA, length uint64) ([]byte, error) {
	return ws.MemRead(m.VA(rva), length)
}

// note: relative to the module
func (m LoadedModule) MemReadPtr(ws *Workspace, rva RVA) (VA, error) {
	if ws.Mode == MODE_32 {
		var data uint32
		d, e := m.MemRead(ws, rva, 0x4)
		if e != nil {
			return 0, e
		}

		p := bytes.NewBuffer(d)
		binary.Read(p, binary.LittleEndian, &data)
		return VA(uint64(data)), nil
	} else if ws.Mode == MODE_64 {
		var data uint64
		d, e := m.MemRead(ws, rva, 0x8)
		if e != nil {
			return 0, e
		}

		p := bytes.NewBuffer(d)
		binary.Read(p, binary.LittleEndian, &data)
		return VA(uint64(data)), nil
	} else {
		return 0, InvalidModeError
	}
}

// note: relative to the module
func (m LoadedModule) MemWrite(ws *Workspace, rva RVA, data []byte) error {
	return ws.MemWrite(m.VA(rva), data)
}

type DisplayOptions struct {
	NumOpcodeBytes uint
}

type Workspace struct {
	// we cheat and use u as the address space
	as             AddressSpace
	Arch           Arch
	Mode           Mode
	loadedModules  []*LoadedModule
	memoryRegions  []MemoryRegion
	disassembler   gapstone.Engine
	displayOptions DisplayOptions
}

func New(arch Arch, mode Mode) (*Workspace, error) {
	if arch != ARCH_X86 {
		return nil, InvalidArchError
	}
	if !(mode == MODE_32 || mode == MODE_64) {
		return nil, InvalidModeError
	}

	as, e := NewSimpleAddressSpace()
	if e != nil {
		return nil, e
	}

	disassembler, e := gapstone.New(
		GAPSTONE_ARCH_MAP[arch],
		GAPSTONE_MODE_MAP[mode],
	)
	if e != nil {
		return nil, e
	}
	e = disassembler.SetOption(gapstone.CS_OPT_DETAIL, gapstone.CS_OPT_ON)
	check(e)
	if e != nil {
		return nil, e
	}

	return &Workspace{
		as:            as,
		Arch:          arch,
		Mode:          mode,
		loadedModules: make([]*LoadedModule, 0),
		memoryRegions: make([]MemoryRegion, 0),
		disassembler:  disassembler,
		displayOptions: DisplayOptions{
			NumOpcodeBytes: 8,
		},
	}, nil
}

func (ws *Workspace) Close() error {
	ws.disassembler.Close()
	return nil
}

/** (*Workspace) implements AddressSpace **/

func (ws *Workspace) MemRead(va VA, length uint64) ([]byte, error) {
	return ws.as.MemRead(va, length)
}

func (ws *Workspace) MemWrite(va VA, data []byte) error {
	return ws.as.MemWrite(va, data)
}

func (ws *Workspace) MemMap(va VA, length uint64, name string) error {
	return ws.as.MemMap(va, length, name)
}

func (ws *Workspace) MemUnmap(va VA, length uint64) error {
	return ws.as.MemUnmap(va, length)
}

func (ws *Workspace) GetMaps() ([]MemoryRegion, error) {
	return ws.as.GetMaps()
}

func (ws *Workspace) AddLoadedModule(mod *LoadedModule) error {
	ws.loadedModules = append(ws.loadedModules, mod)
	return nil
}

// TODO: remove this?
func (ws *Workspace) disassembleBytes(data []byte, address VA, w io.Writer) error {
	insns, e := ws.disassembler.Disasm([]byte(data), uint64(address), 0 /* all instructions */)
	check(e)

	w.Write([]byte(fmt.Sprintf("Disasm:\n")))
	for _, insn := range insns {
		w.Write([]byte(fmt.Sprintf("0x%x:\t%s\t\t%s\n", insn.Address, insn.Mnemonic, insn.OpStr)))
	}

	return nil
}

// TODO: remove this?
func (ws *Workspace) Disassemble(address VA, length uint64, w io.Writer) error {
	d, e := ws.MemRead(address, length)
	check(e)
	return ws.disassembleBytes(d, address, w)
}

// TODO: remove this?
var FailedToDisassembleInstruction = errors.New("Failed to disassemble an instruction")

// TODO: remove this?
func (ws *Workspace) DisassembleInstruction(address VA) (string, error) {
	d, e := ws.MemRead(address, uint64(MAX_INSN_SIZE))
	check(e)

	insns, e := ws.disassembler.Disasm(d, uint64(address), 1)
	check(e)

	for _, insn := range insns {
		// return the first one
		return fmt.Sprintf("0x%x: %s\t\t%s\n", insn.Address, insn.Mnemonic, insn.OpStr), nil
	}
	return "", FailedToDisassembleInstruction
}

func (ws *Workspace) GetInstructionLength(address VA) (uint64, error) {
	d, e := ws.MemRead(address, uint64(MAX_INSN_SIZE))
	check(e)

	insns, e := ws.disassembler.Disasm(d, uint64(address), 1)
	check(e)

	for _, insn := range insns {
		// return the first one
		return uint64(insn.Size), nil
	}
	return 0, FailedToDisassembleInstruction
}

func (ws Workspace) dumpMemoryRegions() error {
	log.Printf("=== memory map ===")
	for _, region := range ws.memoryRegions {
		log.Printf("  name: %s", region.Name)
		log.Printf("    address: %x", region.Address)
		log.Printf("    length: %x", region.Length)
	}
	return nil
}

func (ws *Workspace) GetEmulator() (*Emulator, error) {
	emu, e := newEmulator(ws)
	if e != nil {
		return nil, e
	}

	for _, region := range ws.memoryRegions {
		e := emu.MemMap(region.Address, region.Length, region.Name)
		check(e)
		if e != nil {
			emu.Close()
			return nil, e
		}

		d, e := ws.MemRead(region.Address, region.Length)
		check(e)
		if e != nil {
			emu.Close()
			return nil, e
		}

		e = emu.MemWrite(region.Address, d)
		check(e)
		if e != nil {
			emu.Close()
			return nil, e
		}
	}

	stackAddress := VA(0x69690000)
	stackSize := uint64(0x40000)
	e = emu.MemMap(VA(uint64(stackAddress)-(stackSize/2)), stackSize, "stack")
	check(e)

	emu.SetStackPointer(stackAddress)

	return emu, nil
}

func DoesInstructionHaveGroup(i gapstone.Instruction, group uint) bool {
	for _, group := range i.Groups {
		if group == group {
			return true
		}
	}
	return false
}
