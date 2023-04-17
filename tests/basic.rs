use std::{error::Error, time::Instant};

use mapped::{ProcOptions, Threads, KNN};

#[test]
fn extreme() -> Result<(), Box<dyn Error>> {
    let i = Instant::now();
    ProcOptions::default()
        .threads(Threads::Extreme)
        .load("./samples/11.jpg")?
        .process()?;
    println!("Time elapsed for new API: {}", i.elapsed().as_secs_f64());
    Ok(())
}

#[test]
fn tracking() -> Result<(), Box<dyn Error>> {
    let i = Instant::now();
    let mut opts = ProcOptions::default();
    opts.threads(Threads::Extreme);
    let mut p = opts.load("./samples/11.jpg")?;
    let mut track = p.gen_tracker();
    std::thread::scope(|s| {
        s.spawn(move || {
            p.process().unwrap();
        });
        loop {
            if track.percentage() == 100.0 {
                break;
            }
        }
        println!(
            "Time elapsed for new API (with tracking): {}",
            i.elapsed().as_secs_f64()
        );
    });
    Ok(())
}

#[test]
fn ray() -> Result<(), Box<dyn Error>> {
    let i = Instant::now();
    ProcOptions::default()
        .threads(Threads::Rayon)
        .load("./samples/11.jpg")?
        .process()?;
    println!("Time elapsed for ray API: {}", i.elapsed().as_secs_f64());
    Ok(())
}
