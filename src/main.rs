mod crawler;
mod link_graph;

use log2::*;




fn main() {
    let _log2 = log2::open("log/log.txt")
        //.size(100*1024*1024)
        //.rotate(20)
        .tee(true)
        .module(true)
        .module_with_line(true)
        .module_filter(|module| module.contains(""))
        .compress(false)
        //.format(|record, tee| format!("[{}] [{}] {}\n", chrono::Local::now(), record.level(), record.args()))
        .start();
    info!("logging Initialized");

    let args: Vec<String> = std::env::args().collect();
    for arg in &args[..] {
        info!("arg: {}", arg);
    }

}

