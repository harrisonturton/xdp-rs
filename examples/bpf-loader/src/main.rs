/// load a BPF program into the XDP hook on a given interface
#[derive(argh::FromArgs, Debug)]
struct Args {
    /// BPF program object file
    #[argh(positional)]
    filepath: String,
    /// name of the program
    #[argh(positional)]
    program: String,
    /// network interface index
    #[argh(positional)]
    ifindex: u32,
}

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = argh::from_env::<Args>();

    println!(
        "Loading file \"{}\" with program \"{}\" onto ifindex {}",
        args.filepath, args.program, args.ifindex
    );

    let obj_buf = std::fs::read(args.filepath)?;
    let obj = bpf::Object::create(&obj_buf)?;
    obj.load()?;

    let prog = obj.find_program(&args.program)?;
    prog.attach_xdp(args.ifindex)?;

    println!("{} loaded. Ctrl+C to stop.", args.program);
    loop {}
}
