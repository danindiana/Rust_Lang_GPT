use std::fs::File;
use std::io::{BufReader, prelude::*};
use std::sync::{mpsc::{channel, Receiver}, Arc, Mutex};
use std::thread;
use clap::{App, Arg};
use trust_dns_resolver::config::*;
use trust_dns_resolver::Resolver;
use trust_dns_proto::rr::RecordType;

fn main() {
    let matches = App::new("dns_enum")
        .arg(Arg::new("consumers")
            .short('c')
            .long("consumers")
            .takes_value(true)
            .default_value("8"))
        .arg(Arg::new("domain")
            .short('d')
            .long("domain")
            .takes_value(true)
            .required(true))
        .arg(Arg::new("wordlist")
            .short('w')
            .long("wordlist")
            .takes_value(true)
            .default_value("names.txt"))
        .arg(Arg::new("lookup_a")
            .short('a')
            .long("lookup_a")
            .takes_value(false)
            .default_value("true"))
        .arg(Arg::new("lookup_txt")
            .short('t')
            .long("lookup_txt")
            .takes_value(false))
        .arg(Arg::new("show_cname")
            .short('s')
            .long("show_cname")
            .takes_value(false))
        .get_matches();

    let domain = matches.value_of("domain").unwrap().to_string();
    let wordlist = matches.value_of("wordlist").unwrap().to_string();
    let num_consumers: usize = matches.value_of("consumers").unwrap().parse().unwrap();
    let lookup_a = matches.is_present("lookup_a");
    let lookup_txt = matches.is_present("lookup_txt");
    let show_cname = matches.is_present("show_cname");

    let (tx, rx) = channel();
    let rx = Arc::new(Mutex::new(rx));

    for _ in 0..num_consumers {
        let rx = Arc::clone(&rx);
        let domain = domain.clone();
        thread::spawn(move || worker(rx, domain, lookup_a, lookup_txt, show_cname));
    
    }

    // Read subdomains from the wordlist file and send them to the channel
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

fn worker(rx: Arc<Mutex<Receiver<String>>>, domain: String, _lookup_a: bool, lookup_txt: bool, show_cname: bool) {
    let resolver = Resolver::new(ResolverConfig::default(), ResolverOpts::default()).unwrap();

    loop {
        let fqdn;
        {
            let rx = rx.lock().unwrap();
            if let Ok(subdomain) = rx.recv() {
                fqdn = format!("{}.{}", subdomain, domain);
            } else {
                break;
            }
        }

        if lookup_txt {
            if let Ok(lookup) = resolver.lookup(fqdn.clone(), RecordType::TXT) {
                let txt: Vec<String> = lookup.iter().map(|r| r.to_string()).collect();
                println!("TXT record: {} -> {:?}", fqdn, txt);
            }
        }

        if show_cname {
            if let Ok(lookup) = resolver.lookup(fqdn.clone(), RecordType::CNAME) {
                let cname: Vec<String> = lookup.iter().map(|r| r.to_string()).collect();
                println!("CNAME record: {} -> {:?}", fqdn, cname);
            }
        }
    }
}
