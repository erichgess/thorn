//! Hosts braidc analysis data and compiler visibility data for use with
//! the Braid Viewer GUI. This service is started within or is manually pointed
//! to the output (target) directory of the Braid compiler and serves the
//! visibility data generated by the compiler for use by dev tools.  Critically
//! this service acts to convert and query that data into a form that is
//! useable by dev tools.

use std::path::PathBuf;

use clap::StructOpt;
use rocket::fairing::AdHoc;

use cors::CORS;
use sourcemap::SourceMap;
use trace::Trace;

use service::*;

mod cors;
mod graph;
mod sourcemap;
mod trace;
mod cli;
mod service;

#[macro_use]
extern crate rocket;

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![index, get_files, get_file, get_span])
        .attach(stage())
        .attach(CORS)
}

/// Configure the shared state of the trace data and source map
fn stage() -> AdHoc {
    AdHoc::on_ignite("Braid LS", move |rocket| async {
        // Load configuration from CLI
        let args = cli::Cli::parse();
        let target_dir = args.target().to_path_buf();

        // Load the compiler visibility data
        let trace_path = get_trace_file(target_dir.clone());
        let trace = Trace::load(trace_path).unwrap();

        let sourcemap_path = get_sourcemap_file(target_dir.clone());
        let sourcemap = SourceMap::load(sourcemap_path).unwrap();

        rocket
            .mount("/data", routes![get_data, get_graph])
            .manage(trace)
            .manage(sourcemap)
            .attach(CORS)
    })
}

/// Returns the path to the trace file
fn get_trace_file(dir: PathBuf) -> PathBuf {
    let mut pb = dir.to_path_buf();
    pb.push("trace.json");
    pb
}

/// Returns the path to the source map file
fn get_sourcemap_file(dir: PathBuf) -> PathBuf {
    let mut pb = dir.to_path_buf();
    pb.push("sourcemap.json");
    pb
}
