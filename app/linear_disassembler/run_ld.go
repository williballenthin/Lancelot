package main

import (
	"debug/pe"
	"fmt"
	"github.com/codegangsta/cli"
	"github.com/bnagy/gapstone"
	peloader "github.com/williballenthin/Lancelot/loader/pe"
	"github.com/williballenthin/Lancelot/utils"
	W "github.com/williballenthin/Lancelot/workspace"
	"github.com/williballenthin/Lancelot/workspace/dora/linear_disassembly"
	"log"
	"os"
	"strconv"
)

var inputFlag = cli.StringFlag{
	Name:  "input_file",
	Usage: "file to explore",
}

var fvaFlag = cli.StringFlag{
	Name:  "fva",
	Usage: "address of function to explore (hex)",
}

var verboseFlag = cli.BoolFlag{
	Name:  "verbose",
	Usage: "print debugging output",
}

func check(e error) {
	if e != nil {
		panic(e)
	}
}

func doit(path string, fva W.VA) error {
	f, e := pe.Open(path)
	check(e)

	ws, e := W.New(W.ARCH_X86, W.MODE_32)
	check(e)

	loader, e := peloader.New(path, f)
	check(e)

	_, e = loader.Load(ws)
	check(e)

	d, e := LinearDisassembly.New(ws)
	check(e)

        dis, e := ws.GetDisassembler()
	check(e)

	d.RegisterInstructionTraceHandler(func(va W.VA, insn gapstone.Instruction) error {
            s, _, e := LinearDisassembly.FormatAddressDisassembly(dis, ws, va, ws.DisplayOptions.NumOpcodeBytes)
            check(e)
            log.Printf(s)
	    return nil
	})

	d.RegisterInstructionTraceHandler(func(va W.VA, insn gapstone.Instruction) error {
            if W.DoesInstructionHaveGroup(insn, gapstone.X86_GRP_CALL) {
	        log.Printf("--> call")
	    }
	    return nil
	})

	d.RegisterJumpTraceHandler(func(va W.VA, insn gapstone.Instruction, jump LinearDisassembly.JumpTarget) error {
            log.Printf("0x%x --> 0x%x", uint64(va), uint64(jump.Va))
	    return nil
	})

	e = d.ExploreFunction(ws, fva)
	check(e)

	return nil
}

func main() {
	app := cli.NewApp()
	app.Version = "0.1"
	app.Name = "run_linear_disassembler"
	app.Usage = "Invoke linear disassembler."
	app.Flags = []cli.Flag{inputFlag, fvaFlag}
	app.Action = func(c *cli.Context) {
		if utils.CheckRequiredArgs(c, []cli.StringFlag{inputFlag, fvaFlag}) != nil {
			return
		}

		inputFile := c.String("input_file")
		if !utils.DoesPathExist(inputFile) {
			log.Printf("Error: file %s must exist", inputFile)
			return
		}

		iva, e := strconv.ParseUint(c.String("fva"), 0x10, 64)
		check(e)
		fva := W.VA(iva)
		check(doit(inputFile, fva))
	}
	fmt.Printf("%s\n", os.Args)
	app.Run(os.Args)
}