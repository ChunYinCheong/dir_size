use std::{env, fs, path::PathBuf, sync::mpsc::channel, time::Instant};

use anyhow::{Error, Ok, Result};
use clap::Parser;
use rayon::prelude::*;

#[derive(Debug, Clone)]
struct DirNode {
    pub path: PathBuf,
    pub size: u64,
    pub children: Vec<DirNode>,
}

/// Simple program to walk dir size
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Dir path
    #[arg(default_value = ".")]
    path: String,
}

fn main() -> Result<()> {
    let start = Instant::now();

    let args = Args::parse();
    println!("{args:?}");

    let mut path = env::current_dir()?;
    path.push(args.path);
    println!("path: {:?}:", path.canonicalize());

    // let result = v1(current_dir)?;
    let result = v2(path);

    println!("size: {:#?}", result.size);
    let mut children = result.children;
    children.sort_by_key(|n| n.size);
    children.reverse();
    for child in children {
        println!(
            "{:?} - {}",
            child.path.file_name().unwrap_or_default(),
            child.size
        )
    }

    let duration = start.elapsed();
    println!("Time elapsed is: {:?}", duration);
    Ok(())
}

fn v1(path: PathBuf) -> Result<DirNode> {
    let result = walk_v1(DirNode {
        path,
        size: 0,
        children: Default::default(),
    })?;

    Ok(result)
}

fn walk_v1(mut node: DirNode) -> Result<DirNode> {
    let mut size = 0;
    let mut children = vec![];

    for entry in fs::read_dir(&node.path)? {
        let entry = entry?;
        let path = entry.path();

        let metadata = fs::metadata(&path)?;
        if metadata.is_file() {
            size += metadata.len();
        }
        if metadata.is_dir() {
            let child = walk_v1(DirNode {
                path: path.clone(),
                size: 0,
                children: Default::default(),
            })?;
            size += child.size;
            children.push(child);
        }
    }
    node.size = size;
    node.children = children;
    Ok(node)
}

fn v2(path: PathBuf) -> DirNode {
    walk_v2(DirNode {
        path,
        size: 0,
        children: Default::default(),
    })
}

fn walk_v2(mut node: DirNode) -> DirNode {
    let (sender, receiver) = channel();
    let (sender2, receiver2) = channel();

    fs::read_dir(&node.path)
        .unwrap()
        .par_bridge()
        .for_each_with((sender, sender2), |(s1, s2), entry| {
            let entry = entry.unwrap();
            let path = entry.path();

            let metadata = fs::metadata(&path).unwrap();
            if metadata.is_file() {
                s1.send(metadata.len()).unwrap();
            }
            if metadata.is_dir() {
                let child = walk_v2(DirNode {
                    path,
                    size: 0,
                    children: Default::default(),
                });
                s1.send(child.size).unwrap();
                s2.send(child).unwrap();
            }
        });

    node.size = receiver.iter().sum();
    node.children = receiver2.iter().collect();
    node
}
