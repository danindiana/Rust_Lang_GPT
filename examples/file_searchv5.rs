use std::sync::{Arc, Mutex};
use std::sync::mpsc::Receiver;
use std::thread;
use std::fs::File;
use std::io::{BufRead, BufReader};

use clap::{Arg, App};
use trust_dns_resolver::Resolver;
use trust_dns_resolver::config::*;
use trust_dns_resolver::lookup::Lookup;

fn main() {
    let matches = App::new("DNS Enumeration Tool")
    .version("0.1")
    .author("Your Name")
    .about("Performs DNS Enumeration")
    .arg(Arg::new("domain")
        .short('d')
        .long("domain")
        .value_name("DOMAIN")
        .help("The target domain (e.g. example.com)")
        .required(true)
        .takes_value(true))
    .arg(Arg::new("wordlist")
        .short('w')
        .long("wordlist")
        .value_name("WORDLIST")
        .help("Path to the wordlist file")
        .required(true)
        .takes_value(true))
    .arg(Arg::new("lookup_txt")
        .short('t')
        .long("lookup_txt")
        .help("Whether to perform TXT record lookup"))
    .arg(Arg::new("show_cname")
        .short('c')
        .long("show_cname")
        .help("Whether to show CNAME records"))
    .get_matches();


    let domain = matches.value_of("domain").unwrap().to_string();
    let wordlist = matches.value_of("wordlist").unwrap().to_string();
    let lookup_txt = matches.is_present("lookup_txt");
    let show_cname = matches.is_present("show_cname");

    let (tx, rx) = std::sync::mpsc::channel();
    let rx = Arc::new(Mutex::new(rx));

    thread::spawn(move || worker(rx, domain, wordlist, lookup_txt, show_cname));

    println!("Suggested file name: dns_enum_results.txt");
}

fn worker(rx: Arc<Mutex<Receiver<String>>>, domain: String, wordlist: String, lookup_txt: bool, show_cname: bool) {
    let resolver = Resolver::new(ResolverConfig::default(), ResolverOpts::default()).unwrap();

    // Read the wordlist
    let file = File::open(wordlist).unwrap();
    let reader = BufReader::new(file);

    // Iterate through each line (word) in the wordlist
    for line in reader.lines() {
        let word = line.unwrap();
        let subdomain = format!("{}.{}", word, domain);

        // Perform DNS lookup
        println!("Trying: {}", subdomain);
        if let Ok(lookup) = resolver.lookup_ip(subdomain.as_str()) {
            println!("Found: {}", subdomain);
            for ip in lookup.iter() {
                println!("IP: {}", ip);
            }
        }

        // Optionally, perform TXT record lookup
        if lookup_txt {
            let fqdn = format!("{}.", subdomain);
            if let Ok(lookup) = resolver.lookup(fqdn.as_str(), trust_dns_proto::rr::RecordType::TXT) {
                for txt in lookup.iter() {
                    println!("TXT: {}", txt);
                }
            }
        }

        // Optionally, show CNAME records
        if show_cname {
            let fqdn = format!("{}.", subdomain);
            if let Ok(lookup) = resolver.lookup(fqdn.as_str(), trust_dns_proto::rr::RecordType::CNAME) {
                for cname in lookup.iter() {
                    println!("CNAME: {}", cname);
                }
            }
        }
    }
}
