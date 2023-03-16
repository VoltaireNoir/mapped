use std::{error::Error, time::Instant};

use mapped::{Processor, Threads, KNN};

#[test]
fn extreme() -> Result<(), Box<dyn Error>> {
    let i = Instant::now();
    Processor::new("./samples/11.jpg")
        //.strategy(Box::<KNN>::default())
        .threads(Threads::Extreme)
        .load()?
        .process()?;
    println!("Time elapsed for new API: {}", i.elapsed().as_secs_f64());
    Ok(())
}

#[test]
fn tracking() -> Result<(), Box<dyn Error>> {
    let i = Instant::now();
    let mut p = Processor::new("./samples/11.jpg")
        .threads(Threads::Extreme)
        .load()?;
    let mut track = p.gen_tracker();
    std::thread::spawn(move || {
        p.process();
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
    Ok(())
}

#[test]
fn old() -> Result<(), Box<dyn Error>> {
    let i = Instant::now();
    mapped::nordify(
        "./samples/11.jpg",
        Some("mapped.png"),
        &mapped::pallete::NORD,
        &mapped::Nearest,
    );
    println!("Time elapsed for old API: {}", i.elapsed().as_secs_f64());
    Ok(())
}

#[test]
fn ray() -> Result<(), Box<dyn Error>> {
    let i = Instant::now();
    Processor::new("./samples/11.jpg")
        .threads(Threads::Rayon)
        .load()?
        .process()?;
    println!("Time elapsed for ray API: {}", i.elapsed().as_secs_f64());
    Ok(())
}
