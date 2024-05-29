use flytrap::Region;

fn main() {
    for (code, details) in Region::all() {
        println!("{}\t{}", code, details.name);
    }
}
