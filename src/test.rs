use core::time;
use notify::{RecursiveMode, Result, Watcher};
use std::{path::Path, thread};
#[test]
#[ignore = "reason"]
fn test() {
    for i in 0..3 {
        thread::spawn(move || loop {
            println!("thread {}", i);
            thread::sleep(time::Duration::from_secs(1));
        });
    }
    thread::park();
}
#[test]
fn main() -> Result<()> {
    thread::spawn(move || {
        // Automatically select the best implementation for your platform.
        let mut watcher = notify::recommended_watcher(|res| match res {
            Ok(event) => println!("event: {:?}", event),
            Err(e) => println!("watch error: {:?}", e),
        })
        .unwrap();

        // Add a path to be watched. All files and directories at that path and
        // below will be monitored for changes.
        watcher
            .watch(
                Path::new("/tmp/filegirl/dir"),
                RecursiveMode::Recursive,
            )
            .unwrap();
        thread::park();
    });

    thread::park();
    Ok(())
}
#[test]
#[ignore = "reason"]
fn demo() -> Result<()> {
    // Automatically select the best implementation for your platform.
    let mut watcher = notify::recommended_watcher(|res| {
        match res {
           Ok(event) => println!("event: {:?}", event),
           Err(e) => println!("watch error: {:?}", e),
        }
    })?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(Path::new("/tmp/filegirl/dir"), RecursiveMode::Recursive)?;
    thread::park();
    Ok(())
}