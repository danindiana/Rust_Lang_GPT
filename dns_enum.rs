use clap::{Arg, App};
use std::fs::File;
use std::io::{self, BufReader, prelude::*};
use std::sync::mpsc::{channel, Sender};
use std::thread;
use trust_dns_resolver::config::*;
use trust_dns_resolver::Resolver;

fn main() {
    let matches = App::new("DNS Enumerator")
        .arg(Arg::with_name("consumers")
            .short("c")
            .long("consumers")
            .takes_value(true)
            .default_value("8")
            .help("Number of concurrent consumers."))
        .arg(Arg::with_name("domain")
            .short("d")
            .long("domain")
            .required(true)
            .takes_value(true)
            .help("Base domain to start enumeration from."))
        .arg(Arg::with_name("wordlist")
            .short("w")
            .long("wordlist")
            .takes_value(true)
            .default_value("names.txt")
            .help("Wordlist file to use for enumeration."))
        .arg(Arg::with_name("a")
            .short("a")
            .takes_value(false)
            .help("Lookup A records ( default true )"))
        .arg(Arg::with_name("txt")
            .short("txt")
            .takes_value(false)
            .help("Lookup TXT records ( default false )"))
        .arg(Arg::with_name("cname")
            .short("cname")
            .takes_value(false)
            .help("Show CNAME results ( default false )"))
        .get_matches();

    let consumers: usize = matches.value_of("consumers").unwrap().parse().expect("Consumers must be a number");
    let domain = matches.value_of("domain").unwrap().to_string();
    let wordlist = matches.value_of("wordlist").unwrap().to_string();
    let lookup_a = matches.is_present("a");
    let lookup_txt = matches.is_present("txt");
    let show_cname = matches.is_present("cname");

    let (tx, rx) = channel();

    for _ in 0..consumers {
        let rx = rx.clone();
        thread::spawn(move || worker(rx, domain.clone(), lookup_a, lookup_txt, show_cname));
    }

    if let Ok(file) = File::open(wordlist) {
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let subdomain = line.unwrap();
            tx.send(subdomain).unwrap();
        }
    } else {
        println!("Could not open wordlist file.");
        return;
    }

    println!("Suggested file name: dns_enum_results.txt");
}

fn worker(rx: std::sync::mpsc::Receiver<String>, domain: String, lookup_a: bool, lookup_txt: bool, show_cname: bool) {
    let resolver = Resolver::new(ResolverConfig::default(), ResolverOpts::default()).unwrap();
    while let Ok(subdomain) = rx.recv() {
        let fqdn = format!("{}.{}", subdomain, domain);

        if lookup_a {
            if let Ok(lookup) = resolver.lookup_ip(&fqdn) {
                let ips: Vec<String> = lookup.iter().map(|ip| ip.to_string()).collect();
                println!("A record: {} -> {:?}", fqdn, ips);
            }
        }

        if lookup_txt {
            if let Ok(lookup) = resolver.lookup_txt(&fqdn) {
                let txt: Vec<String> = lookup.iter().map(|txt| txt.to_string()).collect();
                println!("TXT record: {} -> {:?}", fqdn, txt);
            }
        }

        if show_cname {
            if let Ok(lookup) = resolver.lookup_cname(&fqdn) {
                let cname: Vec<String> = lookup.iter().map(|cname| cname.to_string()).collect();
                println!("CNAME record: {} -> {:?}", fqdn, cname);
            }
        }
    }
}
