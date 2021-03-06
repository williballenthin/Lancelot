use criterion::{criterion_group, criterion_main, Criterion};

fn fetch_benchmark(c: &mut Criterion) {
    // 0:  48 c7 c0 01 00 00 00    mov    rax,0x1
    let mut emu = lancelot::test::emu_from_shellcode64(&b"\x48\xC7\xC0\x01\x00\x00\x00"[..]);
    emu.reg.rip = 0x0;

    c.bench_function("fetch", |b| {
        b.iter(|| {
            emu.mem.read_u128(criterion::black_box(0x0)).unwrap();
        })
    });

    c.bench_function("fetch and decode", |b| {
        b.iter(|| {
            emu.fetch().unwrap();
        })
    });
}

fn insn_benchmark(c: &mut Criterion) {
    c.bench_function("mov rax, 0x1", |b| {
        // 0:  48 c7 c0 01 00 00 00    mov    rax,0x1
        let mut emu = lancelot::test::emu_from_shellcode64(&b"\x48\xC7\xC0\x01\x00\x00\x00"[..]);
        b.iter(|| {
            emu.reg.rip = 0x0;
            emu.step().unwrap();
        })
    });

    c.bench_function("push 0x1", |b| {
        // 0:  6a 01                   push   0x1
        let mut emu = lancelot::test::emu_from_shellcode64(&b"\x6A\x01"[..]);
        b.iter(|| {
            emu.reg.rip = 0x0;
            emu.reg.rsp = 0x6000;
            emu.step().unwrap();
        })
    });

    c.bench_function("sub rax, rbx", |b| {
        // 0:  48 29 d8                sub    rax,rbx
        let mut emu = lancelot::test::emu_from_shellcode64(&b"\x48\x29\xD8"[..]);
        b.iter(|| {
            emu.reg.rip = 0x0;
            emu.reg.rax = 0x1;
            emu.reg.rbx = 0x1;
            emu.step().unwrap();
        })
    });

    c.bench_function("add rax, rbx", |b| {
        // 0:  48 01 d8                add    rax,rbx
        let mut emu = lancelot::test::emu_from_shellcode64(&b"\x48\x01\xD8"[..]);
        b.iter(|| {
            emu.reg.rip = 0x0;
            emu.reg.rax = 0x1;
            emu.reg.rbx = 0x1;
            emu.step().unwrap();
        })
    });
}

criterion_group!(fetch, fetch_benchmark);
criterion_group!(insn, insn_benchmark);
criterion_main!(fetch, insn);
