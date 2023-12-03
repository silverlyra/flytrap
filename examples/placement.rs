use flytrap::{Machine, Placement};

fn main() -> Result<(), flytrap::Error> {
    let runtime = Placement::current()?;

    println!("Fly.io app: {}", runtime.app);
    println!("    region: {}", runtime.location);

    if let Some(Machine {
        id,
        memory: Some(memory),
        image: Some(image),
        ..
    }) = runtime.machine
    {
        println!("   machine: {id} ({memory} MB) running {image}");
    }

    if let Some(public_ip) = runtime.public_ip {
        println!(" public IP: {}", public_ip);
    }
    println!("private IP: {}", runtime.private_ip);

    Ok(())
}
