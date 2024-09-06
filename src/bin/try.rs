use once_cell::sync::Lazy;
use rand::{Rng, SeedableRng};
use rand::{prelude::StdRng, distributions::Alphanumeric};


static CMD_END_MARKER: Lazy<String> = Lazy::new(|| {
    println!("INIT");
    let rng = StdRng::seed_from_u64(5);
    rng.sample_iter(Alphanumeric).map(|u| u as char).take(100).collect()
});


fn main() {
    for _i in 0..5 {
        println!("MAIN: {}", CMD_END_MARKER.clone());
    }

    std::thread::spawn(|| {
        for _i in 0..5 {
            println!("THREAD: {}", CMD_END_MARKER.clone());
        }
    })
    .join().unwrap();
}