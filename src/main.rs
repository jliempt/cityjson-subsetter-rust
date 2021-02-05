#![allow(dead_code)]

// extern crate serde_json;

use std::fs::File;
use std::io::prelude::*;
use clap::{Arg, App};
use cityjson_cutter;
use serde_json;
use std::io::BufWriter;

fn main() -> std::io::Result<()> {
    let matches = App::new("My Super Program")
        .version("1.0")
        .author("Kevin K. <kbknapp@gmail.com>")
        .about("Does awesome things")
        .arg(Arg::new("path_in")
            .about("CityJSON input file path")
            .required(true)
            .index(1))
        .arg(Arg::new("subset")
            .about("Subset type (such as \"bbox\")")
            .required(true)
            .index(2))
         .arg(Arg::new("min_x")
            .about("Bounding box min_x")
            .required(true)
            .index(3))
         .arg(Arg::new("min_y")
            .about("Bounding box min_y")
            .required(true)
            .index(4))
        .arg(Arg::new("max_x")
            .about("Bounding box max_x")
            .required(true)
            .index(5))
        .arg(Arg::new("max_y")
            .about("Bounding box max_y")
            .required(true)
            .index(6))
        .arg(Arg::new("path_out")
            .about("CityJSON output file path")
            .required(true)
            .index(7))
        .get_matches();

    let mut path_in = "";
    let mut path_out = "";


    if let Some(p) = matches.value_of("path_in") {
        path_in = p;
    }

    if let Some(p) = matches.value_of("path_out") {
        path_out = p;
    }

    let mut bbox: [ f32; 4 ] = [ 0.0, 0.0, 0.0, 0.0 ];

    if let Some(x) = matches.value_of("min_x") {
        bbox[ 0 ] = x.parse::< f32 >().unwrap();
    }

    if let Some(x) = matches.value_of("min_y") {
        bbox[ 1 ] = x.parse::< f32 >().unwrap();
    }

    if let Some(x) = matches.value_of("max_x") {
        bbox[ 2 ] = x.parse::< f32 >().unwrap();
    }

    if let Some(x) = matches.value_of("max_y") {
        bbox[ 3 ] = x.parse::< f32 >().unwrap();
    }

    println!("{:?}", bbox);

    let mut file_in = File::open( path_in )?;
    let mut buf = Vec::new();
    file_in.read_to_end( &mut buf );

    let file_out = File::create( path_out )?;
    let out = cityjson_cutter::subset::get_subset_bbox( buf, &file_out, bbox );
    let bw = BufWriter::new( file_out );

    let res = serde_json::ser::to_writer( bw, &out );

    Ok(())
    
}